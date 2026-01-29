//! # Advanced QA Metrics
//!
//! FIX_2601/1125: High-level QA metrics for match realism analysis.
//!
//! ## Metrics Implemented
//!
//! - **Line Spacing**: Team compactness (DEF-MID-FWD line gaps)
//! - **Passing Network**: Pass distribution analysis (Gini, density, reciprocity)
//! - **PPDA**: Passes Per Defensive Action (pressing intensity)
//!
//! ## Usage
//!
//! ```ignore
//! use of_core::analysis::qa::advanced_metrics::*;
//!
//! let advanced = compute_advanced_metrics(&result, &setup);
//! println!("Team length: {:.1}m", advanced.line_spacing.home.df_mean);
//! println!("PPDA: {:.1}", advanced.ppda.home.ppda);
//! ```

use crate::models::events::{EventType, MatchEvent};
use crate::models::match_result::MatchPositionData;
use crate::models::match_setup::{MatchSetup, TeamSide};
use crate::models::player::Position;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;

// ============================================================================
// LineRole Enum
// ============================================================================

/// Simplified line role for tactical analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LineRole {
    /// Goalkeeper
    GK,
    /// Defensive line (CB, LB, RB, LWB, RWB)
    DEF,
    /// Midfield line (CDM, CM, CAM, LM, RM)
    MID,
    /// Forward line (LW, RW, CF, ST)
    FWD,
}

impl LineRole {
    /// Convert from Position to LineRole.
    pub fn from_position(pos: &Position) -> Self {
        if pos.is_goalkeeper() {
            LineRole::GK
        } else if pos.is_defender() {
            LineRole::DEF
        } else if pos.is_midfielder() {
            LineRole::MID
        } else {
            LineRole::FWD
        }
    }
}

// ============================================================================
// Configuration Structs
// ============================================================================

/// Configuration for line spacing computation.
#[derive(Debug, Clone)]
pub struct LineSpacingConfig {
    /// Sample interval in milliseconds (default: 250ms)
    pub sample_interval_ms: u64,
    /// Compact threshold in meters (df < this → compact)
    pub compact_threshold_m: f32,
    /// Stretch threshold in meters (df > this → stretched)
    pub stretch_threshold_m: f32,
}

impl Default for LineSpacingConfig {
    fn default() -> Self {
        Self {
            sample_interval_ms: 250,
            compact_threshold_m: 28.0,
            stretch_threshold_m: 40.0,
        }
    }
}

/// Configuration for passing network computation.
#[derive(Debug, Clone)]
pub struct PassNetworkConfig {
    /// Time delta for receiver inference (default: 500ms)
    pub receiver_delta_ms: u64,
    /// Maximum distance for receiver inference in meters
    pub receiver_max_dist_m: f32,
    /// Forward pass threshold in meters (x + this = forward)
    pub forward_threshold_m: f32,
    /// FIX_2601/1130: Use target_track_id (intended receiver) instead of position-based inference.
    /// When true, uses event.target_track_id for accurate reciprocity/density calculation.
    /// When false, uses legacy position-based inference (ball position at t+500ms).
    pub use_intended_receiver: bool,
}

impl Default for PassNetworkConfig {
    fn default() -> Self {
        Self {
            receiver_delta_ms: 500,
            receiver_max_dist_m: 25.0,
            forward_threshold_m: 7.0,
            use_intended_receiver: true, // FIX_2601/1130: Use intended receiver by default
        }
    }
}

/// Configuration for PPDA computation.
#[derive(Debug, Clone)]
pub struct PpdaConfig {
    /// Defensive zone threshold (x <= this is def zone for team attacking right)
    pub def_zone_x_m: f32,
    /// High regain threshold (x >= this is high press zone)
    pub high_regain_x_m: f32,
}

impl Default for PpdaConfig {
    fn default() -> Self {
        Self {
            def_zone_x_m: 63.0,  // ~60% of pitch
            high_regain_x_m: 70.0,
        }
    }
}

// ============================================================================
// Summary Structs
// ============================================================================

/// Line spacing summary for one team.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LineSpacingSummary {
    /// DEF-MID gap mean (meters)
    pub dm_mean: f32,
    /// MID-FWD gap mean (meters)
    pub mf_mean: f32,
    /// Team length (DEF-FWD) mean (meters)
    pub df_mean: f32,
    /// Team length standard deviation
    pub df_std: f32,
    /// 10th percentile of team length
    pub df_p10: f32,
    /// 50th percentile (median) of team length
    pub df_p50: f32,
    /// 90th percentile of team length
    pub df_p90: f32,
    /// Compact rate (df < 28m)
    pub compact_rate: f32,
    /// Stretch rate (df > 40m)
    pub stretch_rate: f32,
    /// Number of valid samples
    pub sample_count: u32,
}

/// Passing network summary for one team.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PassNetworkSummary {
    /// Gini coefficient of pass involvement (0=equal, 1=monopoly)
    pub gini_involvement: f32,
    /// Network centralization (max share - avg share)
    pub centralization: f32,
    /// Network density (unique edges / max possible edges)
    pub density: f32,
    /// Reciprocity (bidirectional pairs / total pairs)
    pub reciprocity: f32,
    /// Forward pass rate (passes going forward by threshold)
    pub forward_pass_rate: f32,
    /// Total passes analyzed
    pub total_passes: u32,
    /// Passes with inferred receiver
    pub passes_with_receiver: u32,
}

/// PPDA summary for one team.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PpdaSummary {
    /// PPDA value (passes_allowed / def_actions)
    pub ppda: f32,
    /// Opponent passes allowed in defensive zone
    pub passes_allowed: u32,
    /// Defensive actions (tackles + fouls) in defensive zone
    pub def_actions: u32,
    /// High regain rate (regains in opponent's half)
    pub high_regain_rate: f32,
    /// Total regains
    pub total_regains: u32,
}

/// Combined advanced metrics for both teams.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QaAdvancedMetrics {
    /// Line spacing metrics
    pub line_spacing: TeamMetrics<LineSpacingSummary>,
    /// Passing network metrics
    pub pass_network: TeamMetrics<PassNetworkSummary>,
    /// PPDA metrics
    pub ppda: TeamMetrics<PpdaSummary>,
}

/// Generic wrapper for home/away team metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamMetrics<T: Default> {
    pub home: T,
    pub away: T,
}

// ============================================================================
// Line Spacing Computation
// ============================================================================

/// Compute line spacing metrics for a team.
///
/// # Arguments
/// * `position_data` - Match position data
/// * `team_indices` - Range of player indices (0..11 for home, 11..22 for away)
/// * `attacks_right` - Whether this team attacks towards x=105
/// * `role_map` - LineRole for each player slot (0-10)
/// * `cfg` - Configuration parameters
///
/// # Returns
/// Line spacing summary
pub fn compute_line_spacing(
    position_data: &MatchPositionData,
    team_indices: Range<usize>,
    attacks_right: bool,
    role_map: &[LineRole; 11],
    cfg: &LineSpacingConfig,
) -> LineSpacingSummary {
    let mut df_samples: Vec<f32> = Vec::new();
    let mut dm_samples: Vec<f32> = Vec::new();
    let mut mf_samples: Vec<f32> = Vec::new();

    // Find min/max timestamps
    let mut min_ts = u64::MAX;
    let mut max_ts = 0u64;

    for idx in team_indices.clone() {
        if let Some(first) = position_data.players[idx].first() {
            min_ts = min_ts.min(first.timestamp);
        }
        if let Some(last) = position_data.players[idx].last() {
            max_ts = max_ts.max(last.timestamp);
        }
    }

    if min_ts >= max_ts {
        return LineSpacingSummary::default();
    }

    // Sample at intervals
    let mut ts = min_ts;
    while ts <= max_ts {
        // Collect x-positions by role
        let mut def_x: Vec<f32> = Vec::new();
        let mut mid_x: Vec<f32> = Vec::new();
        let mut fwd_x: Vec<f32> = Vec::new();

        for (slot, idx) in team_indices.clone().enumerate() {
            if let Some(pos) = interpolate_position(&position_data.players[idx], ts) {
                // Convert to team coordinate (attacking team sees own goal at 0)
                let x = if attacks_right { pos.0 } else { 105.0 - pos.0 };

                match role_map[slot] {
                    LineRole::GK => {} // Skip GK
                    LineRole::DEF => def_x.push(x),
                    LineRole::MID => mid_x.push(x),
                    LineRole::FWD => fwd_x.push(x),
                }
            }
        }

        // Calculate line medians
        if !def_x.is_empty() && !mid_x.is_empty() && !fwd_x.is_empty() {
            let def_median = median(&mut def_x);
            let mid_median = median(&mut mid_x);
            let fwd_median = median(&mut fwd_x);

            let dm = mid_median - def_median;
            let mf = fwd_median - mid_median;
            let df = fwd_median - def_median;

            if dm >= 0.0 && mf >= 0.0 && df >= 0.0 {
                dm_samples.push(dm);
                mf_samples.push(mf);
                df_samples.push(df);
            }
        }

        ts += cfg.sample_interval_ms;
    }

    if df_samples.is_empty() {
        return LineSpacingSummary::default();
    }

    let sample_count = df_samples.len() as u32;

    // Compute statistics
    let dm_mean = mean(&dm_samples);
    let mf_mean = mean(&mf_samples);
    let df_mean = mean(&df_samples);
    let df_std = std_dev(&df_samples, df_mean);

    // Percentiles
    df_samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let df_p10 = percentile(&df_samples, 0.10);
    let df_p50 = percentile(&df_samples, 0.50);
    let df_p90 = percentile(&df_samples, 0.90);

    // Compact/stretch rates
    let compact_count = df_samples.iter().filter(|&&x| x < cfg.compact_threshold_m).count();
    let stretch_count = df_samples.iter().filter(|&&x| x > cfg.stretch_threshold_m).count();
    let compact_rate = compact_count as f32 / df_samples.len() as f32;
    let stretch_rate = stretch_count as f32 / df_samples.len() as f32;

    LineSpacingSummary {
        dm_mean,
        mf_mean,
        df_mean,
        df_std,
        df_p10,
        df_p50,
        df_p90,
        compact_rate,
        stretch_rate,
        sample_count,
    }
}

// ============================================================================
// Passing Network Computation
// ============================================================================

/// Compute passing network metrics for a team.
///
/// # Arguments
/// * `position_data` - Match position data (for receiver inference)
/// * `events` - All match events
/// * `team_indices` - Range of player indices (0..11 for home, 11..22 for away)
/// * `attacks_right` - Whether this team attacks towards x=105
/// * `cfg` - Configuration parameters
///
/// # Returns
/// Passing network summary
pub fn compute_pass_network(
    position_data: &MatchPositionData,
    events: &[MatchEvent],
    team_indices: Range<usize>,
    attacks_right: bool,
    cfg: &PassNetworkConfig,
) -> PassNetworkSummary {
    let is_home = team_indices.start == 0;

    // Filter pass events for this team
    let pass_events: Vec<&MatchEvent> = events
        .iter()
        .filter(|e| e.event_type == EventType::Pass && e.is_home_team == is_home)
        .collect();

    if pass_events.is_empty() {
        return PassNetworkSummary::default();
    }

    // Build edge counts: (passer_slot, receiver_slot) -> count
    let mut edges: HashMap<(u8, u8), u32> = HashMap::new();
    let mut pass_counts: [u32; 11] = [0; 11];
    let mut passes_with_receiver = 0u32;
    let mut forward_passes = 0u32;
    let total_passes = pass_events.len() as u32;

    for event in &pass_events {
        let passer_track_id = match event.player_track_id {
            Some(id) => id as usize,
            None => continue,
        };

        if !team_indices.contains(&passer_track_id) {
            continue;
        }

        let passer_slot = TeamSide::team_slot(passer_track_id);
        pass_counts[passer_slot as usize] += 1;

        // FIX_2601/1129: Get passer position from intended_passer_pos (selection time)
        // Fallback to ball_position if not available
        let passer_x = event
            .details
            .as_ref()
            .and_then(|d| d.intended_passer_pos.map(|(x, _)| x / 10.0))  // Coord10 to meters
            .or_else(|| {
                event.details.as_ref()
                    .and_then(|d| d.ball_position)
                    .map(|(x, _, _)| x / 10.0)  // Coord10 to meters
            })
            .unwrap_or(52.5);

        // FIX_2601/1130: Determine receiver using intended target or position-based fallback
        let receiver_slot = if cfg.use_intended_receiver {
            // Try intended receiver (target_track_id) first
            get_intended_receiver(event, passer_track_id, team_indices.clone())
                .or_else(|| {
                    // Fallback to position-based inference if target_track_id not available
                    event.timestamp_ms.and_then(|ts| infer_pass_receiver(
                        ts,
                        position_data,
                        team_indices.clone(),
                        cfg.receiver_delta_ms,
                        cfg.receiver_max_dist_m,
                        passer_track_id,
                    ))
                })
        } else {
            // Legacy: always use position-based inference
            event.timestamp_ms.and_then(|ts| infer_pass_receiver(
                ts,
                position_data,
                team_indices.clone(),
                cfg.receiver_delta_ms,
                cfg.receiver_max_dist_m,
                passer_track_id,
            ))
        };

        if let Some(receiver_slot) = receiver_slot {
            passes_with_receiver += 1;
            *edges.entry((passer_slot, receiver_slot)).or_insert(0) += 1;

            // FIX_2601/1128: Check if forward pass using intended_target_pos when available
            // Use intended position (selection-time) for accurate forward rate calculation
            let intended_receiver_x = event
                .details
                .as_ref()
                .and_then(|d| d.intended_target_pos)
                .map(|(x, _)| x / 10.0);  // Coord10 to meters

            // DEBUG: Track intended_target_pos availability
            static INTENDED_SOME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            static INTENDED_NONE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            if intended_receiver_x.is_some() {
                INTENDED_SOME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            } else {
                INTENDED_NONE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            let total = INTENDED_SOME.load(std::sync::atomic::Ordering::Relaxed)
                + INTENDED_NONE.load(std::sync::atomic::Ordering::Relaxed);
            if total > 0 && total % 1000 == 0 {
                eprintln!(
                    "[ADV_METRICS INTENDED] Total: {} | Some: {} ({:.1}%) | None: {} ({:.1}%)",
                    total,
                    INTENDED_SOME.load(std::sync::atomic::Ordering::Relaxed),
                    100.0 * INTENDED_SOME.load(std::sync::atomic::Ordering::Relaxed) as f64 / total as f64,
                    INTENDED_NONE.load(std::sync::atomic::Ordering::Relaxed),
                    100.0 * INTENDED_NONE.load(std::sync::atomic::Ordering::Relaxed) as f64 / total as f64
                );
            }

            let receiver_x_m = if let Some(intended_x) = intended_receiver_x {
                // Use intended position (selection-time)
                Some(intended_x)
            } else {
                // Fallback: interpolate position at ts + delta
                let receiver_track_id = if is_home {
                    receiver_slot as usize
                } else {
                    11 + receiver_slot as usize
                };
                // Use event timestamp for position interpolation
                event.timestamp_ms.and_then(|ts| {
                    interpolate_position(
                        &position_data.players[receiver_track_id],
                        ts + cfg.receiver_delta_ms,
                    )
                    .map(|(x, _)| x)
                })
            };

            // FIX_2601/0123: Use is_forward_pass flag when available (computed at decision time)
            // This avoids halftime direction issues since attacks_right changes but was
            // hardcoded to true in compute_advanced_metrics
            let is_forward = event
                .details
                .as_ref()
                .and_then(|d| d.is_forward_pass);

            // DEBUG: Track is_forward_pass flag usage
            use std::sync::atomic::{AtomicU64, Ordering};
            static FLAG_SOME_TRUE: AtomicU64 = AtomicU64::new(0);
            static FLAG_SOME_FALSE: AtomicU64 = AtomicU64::new(0);
            static FLAG_NONE: AtomicU64 = AtomicU64::new(0);
            match is_forward {
                Some(true) => { FLAG_SOME_TRUE.fetch_add(1, Ordering::Relaxed); }
                Some(false) => { FLAG_SOME_FALSE.fetch_add(1, Ordering::Relaxed); }
                None => { FLAG_NONE.fetch_add(1, Ordering::Relaxed); }
            }
            let total = FLAG_SOME_TRUE.load(Ordering::Relaxed) + FLAG_SOME_FALSE.load(Ordering::Relaxed) + FLAG_NONE.load(Ordering::Relaxed);
            if total > 0 && total % 1000 == 0 {
                eprintln!(
                    "[QA_IS_FORWARD] Total: {} | Some(true): {} ({:.1}%) | Some(false): {} ({:.1}%) | None: {} ({:.1}%)",
                    total,
                    FLAG_SOME_TRUE.load(Ordering::Relaxed),
                    100.0 * FLAG_SOME_TRUE.load(Ordering::Relaxed) as f64 / total as f64,
                    FLAG_SOME_FALSE.load(Ordering::Relaxed),
                    100.0 * FLAG_SOME_FALSE.load(Ordering::Relaxed) as f64 / total as f64,
                    FLAG_NONE.load(Ordering::Relaxed),
                    100.0 * FLAG_NONE.load(Ordering::Relaxed) as f64 / total as f64
                );
            }

            if let Some(forward) = is_forward {
                // Use pre-computed flag (correct direction at decision time)
                if forward {
                    forward_passes += 1;
                }
            } else if let Some(receiver_x) = receiver_x_m {
                // Fallback: compute using attacks_right (may be incorrect for 2nd half)
                let dx = if attacks_right {
                    receiver_x - passer_x
                } else {
                    passer_x - receiver_x
                };

                if dx >= cfg.forward_threshold_m {
                    forward_passes += 1;
                }
            }
        }
    }

    // Calculate network metrics
    let total_involvement: u32 = pass_counts.iter().sum();
    let gini_involvement = if total_involvement > 0 {
        gini_coefficient(&pass_counts.iter().map(|&x| x as f32).collect::<Vec<_>>())
    } else {
        0.0
    };

    // Centralization: max share - average share
    let max_share = pass_counts.iter().max().copied().unwrap_or(0) as f32 / total_involvement.max(1) as f32;
    let avg_share = 1.0 / 11.0;
    let centralization = (max_share - avg_share).max(0.0);

    // Density: unique edges / max possible edges (11 * 10 = 110)
    let unique_edges = edges.len() as f32;
    let max_edges = 11.0 * 10.0;
    let density = unique_edges / max_edges;

    // Reciprocity: bidirectional pairs / total pairs
    let mut bidirectional_pairs = 0u32;
    let mut total_pairs = 0u32;
    for &(a, b) in edges.keys() {
        if a < b {
            total_pairs += 1;
            if edges.contains_key(&(b, a)) {
                bidirectional_pairs += 1;
            }
        }
    }
    let reciprocity = if total_pairs > 0 {
        bidirectional_pairs as f32 / total_pairs as f32
    } else {
        0.0
    };

    // Forward pass rate
    let forward_pass_rate = if passes_with_receiver > 0 {
        forward_passes as f32 / passes_with_receiver as f32
    } else {
        0.0
    };

    PassNetworkSummary {
        gini_involvement,
        centralization,
        density,
        reciprocity,
        forward_pass_rate,
        total_passes,
        passes_with_receiver,
    }
}

/// Infer pass receiver from position data.
///
/// Looks at ball position at t+delta_ms and finds the closest teammate.
fn infer_pass_receiver(
    pass_timestamp_ms: u64,
    position_data: &MatchPositionData,
    team_indices: Range<usize>,
    delta_ms: u64,
    max_dist_m: f32,
    passer_track_id: usize,
) -> Option<u8> {
    let target_ts = pass_timestamp_ms + delta_ms;

    // Get ball position at target time
    let ball_pos = interpolate_position(&position_data.ball, target_ts)?;

    // Find closest teammate (excluding passer)
    let mut best_slot: Option<u8> = None;
    let mut best_dist = max_dist_m;

    for (slot, track_id) in team_indices.enumerate() {
        if track_id == passer_track_id {
            continue;
        }

        if let Some(player_pos) = interpolate_position(&position_data.players[track_id], target_ts) {
            let dist = ((player_pos.0 - ball_pos.0).powi(2) + (player_pos.1 - ball_pos.1).powi(2)).sqrt();
            if dist < best_dist {
                best_dist = dist;
                best_slot = Some(slot as u8);
            }
        }
    }

    best_slot
}

/// FIX_2601/1130: Extract intended receiver slot from event's target_track_id.
///
/// Uses the pass selection's intended receiver (target_track_id) instead of
/// position-based inference for accurate reciprocity/density calculation.
///
/// # Arguments
/// * `event` - The pass event
/// * `passer_track_id` - Track ID of the passer (to exclude self-passes)
/// * `team_indices` - Range of track IDs for this team (0..11 or 11..22)
///
/// # Returns
/// Some(slot) if target_track_id is valid and on the same team, None otherwise
fn get_intended_receiver(
    event: &MatchEvent,
    passer_track_id: usize,
    team_indices: Range<usize>,
) -> Option<u8> {
    let target_id = event.target_track_id? as usize;

    // Validate: must be on the same team
    if !team_indices.contains(&target_id) {
        return None;
    }

    // Exclude self-passes
    if target_id == passer_track_id {
        return None;
    }

    // Convert track_id to slot (0-10)
    // Home team: track_id 0-10 → slot 0-10
    // Away team: track_id 11-21 → slot 0-10
    let slot = if target_id < 11 {
        target_id as u8
    } else {
        (target_id - 11) as u8
    };

    Some(slot)
}

// ============================================================================
// PPDA Computation
// ============================================================================

/// Compute PPDA metrics for a team (as the defending team).
///
/// PPDA measures pressing intensity: lower = more intense pressing.
///
/// # Arguments
/// * `events` - All match events
/// * `position_data` - Optional position data (for high regain calculation)
/// * `defending_team_is_home` - Whether the defending team is home
/// * `attacks_right` - Whether the defending team attacks towards x=105
/// * `cfg` - Configuration parameters
///
/// # Returns
/// PPDA summary for the defending team
pub fn compute_ppda(
    events: &[MatchEvent],
    _position_data: Option<&MatchPositionData>,
    defending_team_is_home: bool,
    attacks_right: bool,
    cfg: &PpdaConfig,
) -> PpdaSummary {
    let opponent_is_home = !defending_team_is_home;

    // Count opponent passes in defensive zone (opponent's attacking zone)
    // Defensive zone for team attacking right: x <= def_zone_x_m (opponent's half)
    let mut passes_allowed = 0u32;

    for event in events {
        if event.event_type == EventType::Pass && event.is_home_team == opponent_is_home {
            if let Some(ref details) = event.details {
                if let Some((x, _, _)) = details.ball_position {
                    let x_m = x / 10.0; // Coord10 to meters
                    // For team attacking right, def zone is x <= threshold
                    // For team attacking left, def zone is x >= (105 - threshold)
                    let in_def_zone = if attacks_right {
                        x_m <= cfg.def_zone_x_m
                    } else {
                        x_m >= (105.0 - cfg.def_zone_x_m)
                    };

                    if in_def_zone {
                        passes_allowed += 1;
                    }
                }
            }
        }
    }

    // Count defensive actions (tackles + fouls) in defensive zone
    let mut def_actions = 0u32;
    let mut total_regains = 0u32;
    let mut high_regains = 0u32;

    for event in events {
        let is_def_action = matches!(event.event_type, EventType::Tackle | EventType::Foul);

        if is_def_action && event.is_home_team == defending_team_is_home {
            if let Some(ref details) = event.details {
                if let Some((x, _, _)) = details.ball_position {
                    let x_m = x / 10.0;

                    let in_def_zone = if attacks_right {
                        x_m <= cfg.def_zone_x_m
                    } else {
                        x_m >= (105.0 - cfg.def_zone_x_m)
                    };

                    if in_def_zone {
                        def_actions += 1;
                    }

                    // Count as regain (tackle = possession win)
                    if event.event_type == EventType::Tackle {
                        total_regains += 1;

                        // High regain: in opponent's half
                        let in_high_zone = if attacks_right {
                            x_m >= cfg.high_regain_x_m
                        } else {
                            x_m <= (105.0 - cfg.high_regain_x_m)
                        };

                        if in_high_zone {
                            high_regains += 1;
                        }
                    }
                }
            }
        }
    }

    // Calculate PPDA
    let ppda = if def_actions > 0 {
        passes_allowed as f32 / def_actions as f32
    } else {
        f32::MAX // No defensive actions = infinite PPDA
    };

    // High regain rate
    let high_regain_rate = if total_regains > 0 {
        high_regains as f32 / total_regains as f32
    } else {
        0.0
    };

    PpdaSummary {
        ppda,
        passes_allowed,
        def_actions,
        high_regain_rate,
        total_regains,
    }
}

// ============================================================================
// Integration Function
// ============================================================================

/// Compute all advanced metrics for a match.
///
/// # Arguments
/// * `position_data` - Match position data
/// * `events` - Match events
/// * `setup` - Match setup (for player positions)
///
/// # Returns
/// Combined advanced metrics for both teams
pub fn compute_advanced_metrics(
    position_data: &MatchPositionData,
    events: &[MatchEvent],
    setup: &MatchSetup,
) -> QaAdvancedMetrics {
    // Build role maps from setup
    let home_roles = build_role_map(&setup.home);
    let away_roles = build_role_map(&setup.away);

    let line_cfg = LineSpacingConfig::default();
    let pass_cfg = PassNetworkConfig::default();
    let ppda_cfg = PpdaConfig::default();

    // Home team attacks right in first half (standard convention)
    let home_attacks_right = true;

    // Compute line spacing
    let home_line = compute_line_spacing(
        position_data,
        0..11,
        home_attacks_right,
        &home_roles,
        &line_cfg,
    );
    let away_line = compute_line_spacing(
        position_data,
        11..22,
        !home_attacks_right,
        &away_roles,
        &line_cfg,
    );

    // Compute passing network
    let home_network = compute_pass_network(
        position_data,
        events,
        0..11,
        home_attacks_right,
        &pass_cfg,
    );
    let away_network = compute_pass_network(
        position_data,
        events,
        11..22,
        !home_attacks_right,
        &pass_cfg,
    );

    // Compute PPDA (defending team perspective)
    let home_ppda = compute_ppda(
        events,
        Some(position_data),
        true,  // home is defending
        home_attacks_right,
        &ppda_cfg,
    );
    let away_ppda = compute_ppda(
        events,
        Some(position_data),
        false, // away is defending
        !home_attacks_right,
        &ppda_cfg,
    );

    QaAdvancedMetrics {
        line_spacing: TeamMetrics {
            home: home_line,
            away: away_line,
        },
        pass_network: TeamMetrics {
            home: home_network,
            away: away_network,
        },
        ppda: TeamMetrics {
            home: home_ppda,
            away: away_ppda,
        },
    }
}

/// Build role map from team setup.
fn build_role_map(team: &crate::models::match_setup::TeamSetup) -> [LineRole; 11] {
    let mut roles = [LineRole::MID; 11]; // Default to MID
    for (i, player) in team.starters.iter().enumerate() {
        if i < 11 {
            roles[i] = LineRole::from_position(&player.position);
        }
    }
    roles
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Interpolate position at a given timestamp.
fn interpolate_position(
    positions: &[crate::models::match_result::PositionDataItem],
    timestamp: u64,
) -> Option<(f32, f32)> {
    if positions.is_empty() {
        return None;
    }

    // Binary search for closest timestamp
    let idx = positions.partition_point(|p| p.timestamp < timestamp);

    if idx == 0 {
        return Some(positions[0].position);
    }
    if idx >= positions.len() {
        return Some(positions.last()?.position);
    }

    // Linear interpolation
    let p0 = &positions[idx - 1];
    let p1 = &positions[idx];

    if p0.timestamp == p1.timestamp {
        return Some(p0.position);
    }

    let t = (timestamp - p0.timestamp) as f32 / (p1.timestamp - p0.timestamp) as f32;
    let x = p0.position.0 + t * (p1.position.0 - p0.position.0);
    let y = p0.position.1 + t * (p1.position.1 - p0.position.1);

    Some((x, y))
}

/// Calculate mean of a slice.
fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

/// Calculate standard deviation.
fn std_dev(values: &[f32], mean: f32) -> f32 {
    if values.len() < 2 {
        return 0.0;
    }
    let variance: f32 = values.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / values.len() as f32;
    variance.sqrt()
}

/// Calculate median (mutates input for sorting).
fn median(values: &mut [f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

/// Calculate percentile (assumes sorted input).
fn percentile(sorted_values: &[f32], p: f32) -> f32 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let idx = ((sorted_values.len() - 1) as f32 * p).round() as usize;
    sorted_values[idx.min(sorted_values.len() - 1)]
}

/// Calculate Gini coefficient.
fn gini_coefficient(values: &[f32]) -> f32 {
    if values.is_empty() || values.iter().all(|&x| x == 0.0) {
        return 0.0;
    }

    let mut sorted: Vec<f32> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len() as f32;
    let total: f32 = sorted.iter().sum();

    if total == 0.0 {
        return 0.0;
    }

    let mut gini_sum = 0.0f32;

    for (i, &val) in sorted.iter().enumerate() {
        // Gini formula: sum of (2*i - n - 1) * x_i
        gini_sum += (2.0 * (i as f32 + 1.0) - n - 1.0) * val;
    }

    gini_sum / (n * total)
}

// ============================================================================
// Baseline Reference Data
// ============================================================================

/// Baseline reference values from real match data (EPL 2023/24).
/// Source: qa_baseline.json "advanced" section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedBaseline {
    /// Line spacing baseline
    pub line_spacing: LineSpacingBaseline,
    /// Pass network baseline
    pub pass_network: PassNetworkBaseline,
    /// PPDA baseline
    pub ppda: PpdaBaseline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineSpacingBaseline {
    pub df_mean_m: f32,
    pub df_std_m: f32,
    pub compact_rate: f32,
    pub stretch_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassNetworkBaseline {
    pub gini_involvement: f32,
    pub density: f32,
    pub reciprocity: f32,
    pub forward_pass_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PpdaBaseline {
    pub ppda_mean: f32,
    pub ppda_std: f32,
    pub high_regain_rate: f32,
}

impl Default for AdvancedBaseline {
    fn default() -> Self {
        // Default values from EPL 2023/24 aggregate
        Self {
            line_spacing: LineSpacingBaseline {
                df_mean_m: 32.0,
                df_std_m: 5.5,
                compact_rate: 0.25,
                stretch_rate: 0.15,
            },
            pass_network: PassNetworkBaseline {
                gini_involvement: 0.35,
                density: 0.40,
                reciprocity: 0.55,
                forward_pass_rate: 0.22,
            },
            ppda: PpdaBaseline {
                ppda_mean: 10.5,
                ppda_std: 3.0,
                high_regain_rate: 0.25,
            },
        }
    }
}

// ============================================================================
// Scorecard Types
// ============================================================================

/// QA Scorecard - aggregated quality score for a batch of matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaScorecard {
    /// Number of matches analyzed
    pub runs: u32,
    /// League baseline used
    pub league_baseline: String,
    /// Overall score (0-100)
    pub overall_score: f32,
    /// Subscores by category
    pub subscores: QaSubscores,
    /// Alerts (warnings and failures)
    pub alerts: Vec<QaAlert>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QaSubscores {
    /// Line spacing realism (0-100)
    pub line_spacing: f32,
    /// Passing network realism (0-100)
    pub passing_network: f32,
    /// PPDA/pressing realism (0-100)
    pub ppda_pressing: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaAlert {
    /// Alert level
    pub level: AlertLevel,
    /// Metric name
    pub metric: String,
    /// Alert message
    pub msg: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    Warn,
    Fail,
}

// ============================================================================
// Batch Aggregation
// ============================================================================

/// Aggregated metrics from multiple match runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    /// Number of matches aggregated
    pub run_count: u32,

    // Line spacing aggregates (both teams combined)
    pub df_mean_avg: f32,
    pub df_mean_std: f32,
    pub df_std_avg: f32,
    pub compact_rate_avg: f32,
    pub stretch_rate_avg: f32,

    // Pass network aggregates
    pub gini_involvement_avg: f32,
    pub density_avg: f32,
    pub reciprocity_avg: f32,
    pub forward_pass_rate_avg: f32,

    // PPDA aggregates
    pub ppda_avg: f32,
    pub ppda_std: f32,
    pub high_regain_rate_avg: f32,
}

/// Aggregate metrics from multiple match runs.
///
/// # Arguments
/// * `metrics` - Vector of per-match advanced metrics
///
/// # Returns
/// Aggregated metrics with mean/std across all matches
pub fn aggregate_runs(metrics: &[QaAdvancedMetrics]) -> AggregatedMetrics {
    if metrics.is_empty() {
        return AggregatedMetrics::default();
    }

    let run_count = metrics.len() as u32;

    // Collect per-match values (both teams averaged)
    let mut df_means: Vec<f32> = Vec::with_capacity(metrics.len());
    let mut df_stds: Vec<f32> = Vec::with_capacity(metrics.len());
    let mut compact_rates: Vec<f32> = Vec::with_capacity(metrics.len());
    let mut stretch_rates: Vec<f32> = Vec::with_capacity(metrics.len());

    let mut gini_vals: Vec<f32> = Vec::with_capacity(metrics.len());
    let mut density_vals: Vec<f32> = Vec::with_capacity(metrics.len());
    let mut reciprocity_vals: Vec<f32> = Vec::with_capacity(metrics.len());
    let mut forward_rates: Vec<f32> = Vec::with_capacity(metrics.len());

    let mut ppda_vals: Vec<f32> = Vec::with_capacity(metrics.len());
    let mut high_regain_vals: Vec<f32> = Vec::with_capacity(metrics.len());

    for m in metrics {
        // Line spacing (average both teams)
        let home_ls = &m.line_spacing.home;
        let away_ls = &m.line_spacing.away;
        if home_ls.sample_count > 0 && away_ls.sample_count > 0 {
            df_means.push((home_ls.df_mean + away_ls.df_mean) / 2.0);
            df_stds.push((home_ls.df_std + away_ls.df_std) / 2.0);
            compact_rates.push((home_ls.compact_rate + away_ls.compact_rate) / 2.0);
            stretch_rates.push((home_ls.stretch_rate + away_ls.stretch_rate) / 2.0);
        }

        // Pass network (average both teams)
        let home_pn = &m.pass_network.home;
        let away_pn = &m.pass_network.away;
        if home_pn.total_passes > 0 && away_pn.total_passes > 0 {
            gini_vals.push((home_pn.gini_involvement + away_pn.gini_involvement) / 2.0);
            density_vals.push((home_pn.density + away_pn.density) / 2.0);
            reciprocity_vals.push((home_pn.reciprocity + away_pn.reciprocity) / 2.0);
            forward_rates.push((home_pn.forward_pass_rate + away_pn.forward_pass_rate) / 2.0);
        }

        // PPDA (average both teams, exclude infinite values)
        let home_ppda = &m.ppda.home;
        let away_ppda = &m.ppda.away;
        if home_ppda.ppda < f32::MAX && away_ppda.ppda < f32::MAX {
            ppda_vals.push((home_ppda.ppda + away_ppda.ppda) / 2.0);
        }
        if home_ppda.total_regains > 0 && away_ppda.total_regains > 0 {
            high_regain_vals.push((home_ppda.high_regain_rate + away_ppda.high_regain_rate) / 2.0);
        }
    }

    let df_mean_avg = mean(&df_means);
    let df_mean_std = std_dev(&df_means, df_mean_avg);

    let ppda_avg = mean(&ppda_vals);
    let ppda_std = std_dev(&ppda_vals, ppda_avg);

    AggregatedMetrics {
        run_count,
        df_mean_avg,
        df_mean_std,
        df_std_avg: mean(&df_stds),
        compact_rate_avg: mean(&compact_rates),
        stretch_rate_avg: mean(&stretch_rates),
        gini_involvement_avg: mean(&gini_vals),
        density_avg: mean(&density_vals),
        reciprocity_avg: mean(&reciprocity_vals),
        forward_pass_rate_avg: mean(&forward_rates),
        ppda_avg,
        ppda_std,
        high_regain_rate_avg: mean(&high_regain_vals),
    }
}

// ============================================================================
// Scoring Against Baseline
// ============================================================================

/// Score configuration for z-score based scoring.
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    /// Z-score multiplier for penalty (default: 25)
    pub z_penalty_multiplier: f32,
    /// Maximum score (default: 100)
    pub max_score: f32,
    /// Minimum score (default: 0)
    pub min_score: f32,
    /// Warn threshold (z-score, default: 1.5)
    pub warn_z_threshold: f32,
    /// Fail threshold (z-score, default: 2.5)
    pub fail_z_threshold: f32,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            z_penalty_multiplier: 25.0,
            max_score: 100.0,
            min_score: 0.0,
            warn_z_threshold: 1.5,
            fail_z_threshold: 2.5,
        }
    }
}

/// Score aggregated metrics against baseline.
///
/// Uses z-score based scoring:
/// - `|z| = 0` → 100 points
/// - `|z| = 1` → 75 points
/// - `|z| = 2` → 50 points
/// - `|z| = 4` → 0 points
///
/// # Arguments
/// * `aggregated` - Aggregated metrics from batch runs
/// * `baseline` - Baseline reference values
/// * `cfg` - Scoring configuration
///
/// # Returns
/// QaScorecard with scores and alerts
pub fn score_against_baseline(
    aggregated: &AggregatedMetrics,
    baseline: &AdvancedBaseline,
    cfg: &ScoringConfig,
) -> QaScorecard {
    let mut alerts = Vec::new();

    // === Line Spacing Score ===
    let line_scores = [
        score_metric(
            aggregated.df_mean_avg,
            baseline.line_spacing.df_mean_m,
            baseline.line_spacing.df_std_m,
            "line.df_mean",
            cfg,
            &mut alerts,
        ),
        score_metric(
            aggregated.compact_rate_avg,
            baseline.line_spacing.compact_rate,
            0.10, // assumed std
            "line.compact_rate",
            cfg,
            &mut alerts,
        ),
        score_metric(
            aggregated.stretch_rate_avg,
            baseline.line_spacing.stretch_rate,
            0.08, // assumed std
            "line.stretch_rate",
            cfg,
            &mut alerts,
        ),
    ];
    let line_spacing_score = mean(&line_scores);

    // === Pass Network Score ===
    let network_scores = [
        score_metric(
            aggregated.gini_involvement_avg,
            baseline.pass_network.gini_involvement,
            0.10,
            "net.gini_involvement",
            cfg,
            &mut alerts,
        ),
        score_metric(
            aggregated.density_avg,
            baseline.pass_network.density,
            0.12,
            "net.density",
            cfg,
            &mut alerts,
        ),
        score_metric(
            aggregated.reciprocity_avg,
            baseline.pass_network.reciprocity,
            0.15,
            "net.reciprocity",
            cfg,
            &mut alerts,
        ),
        score_metric(
            aggregated.forward_pass_rate_avg,
            baseline.pass_network.forward_pass_rate,
            0.08,
            "net.forward_pass_rate",
            cfg,
            &mut alerts,
        ),
    ];
    let passing_network_score = mean(&network_scores);

    // === PPDA Score ===
    let ppda_scores = [
        score_metric(
            aggregated.ppda_avg,
            baseline.ppda.ppda_mean,
            baseline.ppda.ppda_std,
            "ppda.ppda",
            cfg,
            &mut alerts,
        ),
        score_metric(
            aggregated.high_regain_rate_avg,
            baseline.ppda.high_regain_rate,
            0.10,
            "ppda.high_regain_rate",
            cfg,
            &mut alerts,
        ),
    ];
    let ppda_pressing_score = mean(&ppda_scores);

    // === Overall Score ===
    // Weighted average: line 40%, network 30%, ppda 30%
    let overall_score = line_spacing_score * 0.40
        + passing_network_score * 0.30
        + ppda_pressing_score * 0.30;

    QaScorecard {
        runs: aggregated.run_count,
        league_baseline: "EPL_2023_24".to_string(),
        overall_score,
        subscores: QaSubscores {
            line_spacing: line_spacing_score,
            passing_network: passing_network_score,
            ppda_pressing: ppda_pressing_score,
        },
        alerts,
    }
}

/// Score a single metric against baseline using z-score.
fn score_metric(
    value: f32,
    baseline_mean: f32,
    baseline_std: f32,
    metric_name: &str,
    cfg: &ScoringConfig,
    alerts: &mut Vec<QaAlert>,
) -> f32 {
    // Calculate z-score
    let z = if baseline_std > 0.001 {
        (value - baseline_mean).abs() / baseline_std
    } else {
        (value - baseline_mean).abs() * 10.0 // Fallback for zero std
    };

    // Score: 100 - 25 * |z|, clamped to [0, 100]
    let score = (cfg.max_score - cfg.z_penalty_multiplier * z)
        .clamp(cfg.min_score, cfg.max_score);

    // Generate alerts
    if z >= cfg.fail_z_threshold {
        alerts.push(QaAlert {
            level: AlertLevel::Fail,
            metric: metric_name.to_string(),
            msg: format!(
                "{} = {:.2} (z={:.2}, baseline={:.2}±{:.2})",
                metric_name, value, z, baseline_mean, baseline_std
            ),
        });
    } else if z >= cfg.warn_z_threshold {
        alerts.push(QaAlert {
            level: AlertLevel::Warn,
            metric: metric_name.to_string(),
            msg: format!(
                "{} = {:.2} (z={:.2}, baseline={:.2}±{:.2})",
                metric_name, value, z, baseline_mean, baseline_std
            ),
        });
    }

    score
}

/// Generate scorecard from a batch of match results.
///
/// Convenience function that combines aggregate_runs and score_against_baseline.
pub fn generate_scorecard(metrics: &[QaAdvancedMetrics]) -> QaScorecard {
    let aggregated = aggregate_runs(metrics);
    let baseline = AdvancedBaseline::default();
    let cfg = ScoringConfig::default();
    score_against_baseline(&aggregated, &baseline, &cfg)
}

// ============================================================================
// QA Grade System (FIX_2601/1127)
// ============================================================================

/// QA grade thresholds for overall score interpretation.
///
/// FIX_2601/1127: Standardized grade boundaries:
/// - FAIL:     < 55
/// - MARGINAL: 55-70
/// - PASS:     70-85
/// - GREAT:    >= 85
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QaGrade {
    /// Score < 55: Engine does not produce realistic football
    Fail,
    /// Score 55-70: Engine needs tuning
    Marginal,
    /// Score 70-85: Engine produces acceptable football
    Pass,
    /// Score >= 85: Engine produces high-quality realistic football
    Great,
}

impl QaGrade {
    /// Grade thresholds
    pub const FAIL_THRESHOLD: f32 = 55.0;
    pub const MARGINAL_THRESHOLD: f32 = 70.0;
    pub const PASS_THRESHOLD: f32 = 85.0;

    /// Get grade from score
    pub fn from_score(score: f32) -> Self {
        if score < Self::FAIL_THRESHOLD {
            QaGrade::Fail
        } else if score < Self::MARGINAL_THRESHOLD {
            QaGrade::Marginal
        } else if score < Self::PASS_THRESHOLD {
            QaGrade::Pass
        } else {
            QaGrade::Great
        }
    }

    /// Get display label
    pub fn label(&self) -> &'static str {
        match self {
            QaGrade::Fail => "FAIL",
            QaGrade::Marginal => "MARGINAL",
            QaGrade::Pass => "PASS",
            QaGrade::Great => "GREAT",
        }
    }

    /// Get emoji indicator
    pub fn emoji(&self) -> &'static str {
        match self {
            QaGrade::Fail => "❌",
            QaGrade::Marginal => "⚠️",
            QaGrade::Pass => "✅",
            QaGrade::Great => "🌟",
        }
    }

    /// Check if grade is acceptable (PASS or better)
    pub fn is_acceptable(&self) -> bool {
        matches!(self, QaGrade::Pass | QaGrade::Great)
    }

    /// Check if grade passes CI gate (MARGINAL or better)
    pub fn passes_ci_gate(&self) -> bool {
        !matches!(self, QaGrade::Fail)
    }
}

impl std::fmt::Display for QaGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.emoji(), self.label())
    }
}

/// Get the QA grade for a scorecard
pub fn get_scorecard_grade(scorecard: &QaScorecard) -> QaGrade {
    QaGrade::from_score(scorecard.overall_score)
}

/// CI gate check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGateResult {
    /// Overall grade
    pub grade: QaGrade,
    /// Whether the gate passed
    pub passed: bool,
    /// Reason for failure (if any)
    pub failure_reason: Option<String>,
    /// Metrics that need attention
    pub attention_metrics: Vec<String>,
}

/// Check CI gate conditions for a scorecard.
///
/// FIX_2601/1127: CI gate passes if:
/// - Overall grade is MARGINAL or better (score >= 55)
/// - No FAIL-level alerts on critical metrics
pub fn check_ci_gate(scorecard: &QaScorecard) -> CiGateResult {
    let grade = get_scorecard_grade(scorecard);
    let passed = grade.passes_ci_gate();

    let failure_reason = if !passed {
        Some(format!(
            "Overall score {:.1} is below MARGINAL threshold (55)",
            scorecard.overall_score
        ))
    } else {
        None
    };

    // Collect metrics that need attention (WARN or FAIL alerts)
    let attention_metrics: Vec<String> = scorecard
        .alerts
        .iter()
        .map(|a| a.metric.clone())
        .collect();

    CiGateResult {
        grade,
        passed,
        failure_reason,
        attention_metrics,
    }
}

// ============================================================================
// Kickoff Window Configuration
// ============================================================================

/// Extended configuration with kickoff window exclusion.
///
/// FIX_2601/1120: GameStart 구간 (첫 ~50초) 91:1 편향 발견.
/// 이 구간을 분석에서 제외할 수 있는 옵션 제공.
#[derive(Debug, Clone)]
pub struct AdvancedMetricsConfig {
    pub line_spacing: LineSpacingConfig,
    pub pass_network: PassNetworkConfig,
    pub ppda: PpdaConfig,
    /// Exclude first N milliseconds after kickoff (default: 0 = no exclusion)
    /// Set to 10000 (10 seconds) to exclude GameStart bias window
    pub exclude_kickoff_window_ms: u64,
}

impl Default for AdvancedMetricsConfig {
    fn default() -> Self {
        Self {
            line_spacing: LineSpacingConfig::default(),
            pass_network: PassNetworkConfig::default(),
            ppda: PpdaConfig::default(),
            exclude_kickoff_window_ms: 0, // No exclusion by default
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::match_result::{MatchPositionData, PositionDataItem};

    #[test]
    fn test_line_role_from_position() {
        assert_eq!(LineRole::from_position(&Position::GK), LineRole::GK);
        assert_eq!(LineRole::from_position(&Position::CB), LineRole::DEF);
        assert_eq!(LineRole::from_position(&Position::LB), LineRole::DEF);
        assert_eq!(LineRole::from_position(&Position::CM), LineRole::MID);
        assert_eq!(LineRole::from_position(&Position::CAM), LineRole::MID);
        assert_eq!(LineRole::from_position(&Position::ST), LineRole::FWD);
        assert_eq!(LineRole::from_position(&Position::LW), LineRole::FWD);
    }

    #[test]
    fn test_gini_coefficient_equal() {
        // All equal values → Gini = 0
        let values = vec![10.0, 10.0, 10.0, 10.0];
        let gini = gini_coefficient(&values);
        assert!(gini.abs() < 0.01, "Equal distribution should have Gini ≈ 0, got {}", gini);
    }

    #[test]
    fn test_gini_coefficient_unequal() {
        // One person has everything → Gini ≈ 1
        let values = vec![0.0, 0.0, 0.0, 100.0];
        let gini = gini_coefficient(&values);
        assert!(gini > 0.7, "Monopoly should have high Gini, got {}", gini);
    }

    #[test]
    fn test_median() {
        let mut odd = vec![1.0, 3.0, 2.0, 5.0, 4.0];
        assert_eq!(median(&mut odd), 3.0);

        let mut even = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(median(&mut even), 2.5);
    }

    #[test]
    fn test_percentile() {
        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert_eq!(percentile(&sorted, 0.0), 1.0);
        assert_eq!(percentile(&sorted, 0.5), 6.0);
        assert_eq!(percentile(&sorted, 1.0), 10.0);
    }

    #[test]
    fn test_interpolate_position() {
        let positions = vec![
            PositionDataItem::new(0, (0.0, 0.0)),
            PositionDataItem::new(1000, (10.0, 10.0)),
        ];

        // Exact match
        let p0 = interpolate_position(&positions, 0).unwrap();
        assert_eq!(p0, (0.0, 0.0));

        // Midpoint
        let p500 = interpolate_position(&positions, 500).unwrap();
        assert!((p500.0 - 5.0).abs() < 0.1);
        assert!((p500.1 - 5.0).abs() < 0.1);

        // End
        let p1000 = interpolate_position(&positions, 1000).unwrap();
        assert_eq!(p1000, (10.0, 10.0));
    }

    #[test]
    fn test_line_spacing_empty_data() {
        let position_data = MatchPositionData::new();
        let roles = [LineRole::GK, LineRole::DEF, LineRole::DEF, LineRole::DEF, LineRole::DEF,
                     LineRole::MID, LineRole::MID, LineRole::MID,
                     LineRole::FWD, LineRole::FWD, LineRole::FWD];
        let cfg = LineSpacingConfig::default();

        let result = compute_line_spacing(&position_data, 0..11, true, &roles, &cfg);
        assert_eq!(result.sample_count, 0);
    }

    #[test]
    fn test_pass_network_empty_events() {
        let position_data = MatchPositionData::new();
        let events: Vec<MatchEvent> = Vec::new();
        let cfg = PassNetworkConfig::default();

        let result = compute_pass_network(&position_data, &events, 0..11, true, &cfg);
        assert_eq!(result.total_passes, 0);
    }

    #[test]
    fn test_ppda_calculation() {
        // Create test events
        let mut events = Vec::new();

        // 10 opponent passes in defensive zone
        for i in 0..10 {
            events.push(MatchEvent::pass(
                i as u8,
                i as u64 * 1000,
                false, // opponent (away)
                15,    // track_id
                (300.0, 340.0, 0.0), // x=30m, in def zone
            ));
        }

        // 2 tackles by defending team
        events.push(MatchEvent::tackle(10, 10000, true, 5, (300.0, 340.0, 0.0)));
        events.push(MatchEvent::tackle(11, 11000, true, 6, (300.0, 340.0, 0.0)));

        let cfg = PpdaConfig::default();
        let result = compute_ppda(&events, None, true, true, &cfg);

        assert_eq!(result.passes_allowed, 10);
        assert_eq!(result.def_actions, 2);
        assert!((result.ppda - 5.0).abs() < 0.1, "PPDA should be 10/2=5, got {}", result.ppda);
    }

    #[test]
    fn test_aggregate_runs_empty() {
        let metrics: Vec<QaAdvancedMetrics> = Vec::new();
        let result = aggregate_runs(&metrics);
        assert_eq!(result.run_count, 0);
        assert_eq!(result.df_mean_avg, 0.0);
    }

    #[test]
    fn test_aggregate_runs_single() {
        let mut metrics = Vec::new();

        // Create a single match metric
        let mut m = QaAdvancedMetrics::default();
        m.line_spacing.home.df_mean = 30.0;
        m.line_spacing.home.sample_count = 100;
        m.line_spacing.away.df_mean = 34.0;
        m.line_spacing.away.sample_count = 100;

        m.pass_network.home.total_passes = 200;
        m.pass_network.home.gini_involvement = 0.30;
        m.pass_network.away.total_passes = 180;
        m.pass_network.away.gini_involvement = 0.35;

        m.ppda.home.ppda = 8.0;
        m.ppda.away.ppda = 12.0;

        metrics.push(m);

        let result = aggregate_runs(&metrics);
        assert_eq!(result.run_count, 1);
        assert!((result.df_mean_avg - 32.0).abs() < 0.1, "Expected avg 32.0, got {}", result.df_mean_avg);
        assert!((result.gini_involvement_avg - 0.325).abs() < 0.01);
        assert!((result.ppda_avg - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_score_against_baseline() {
        let mut aggregated = AggregatedMetrics::default();
        aggregated.run_count = 100;
        aggregated.df_mean_avg = 32.0; // Exactly on baseline
        aggregated.compact_rate_avg = 0.25;
        aggregated.stretch_rate_avg = 0.15;
        aggregated.gini_involvement_avg = 0.35;
        aggregated.density_avg = 0.40;
        aggregated.reciprocity_avg = 0.55;
        aggregated.forward_pass_rate_avg = 0.22;
        aggregated.ppda_avg = 10.5;
        aggregated.high_regain_rate_avg = 0.25;

        let baseline = AdvancedBaseline::default();
        let cfg = ScoringConfig::default();

        let scorecard = score_against_baseline(&aggregated, &baseline, &cfg);

        // All metrics on baseline → should score ~100
        assert!(scorecard.overall_score > 95.0, "Perfect match should score high, got {}", scorecard.overall_score);
        assert!(scorecard.alerts.is_empty(), "No alerts expected for perfect match");
    }

    #[test]
    fn test_score_against_baseline_with_deviation() {
        let mut aggregated = AggregatedMetrics::default();
        aggregated.run_count = 100;
        aggregated.df_mean_avg = 45.0; // 2+ std above baseline (32 + 2*5.5 = 43)
        aggregated.compact_rate_avg = 0.25;
        aggregated.stretch_rate_avg = 0.15;
        aggregated.gini_involvement_avg = 0.35;
        aggregated.density_avg = 0.40;
        aggregated.reciprocity_avg = 0.55;
        aggregated.forward_pass_rate_avg = 0.22;
        aggregated.ppda_avg = 10.5;
        aggregated.high_regain_rate_avg = 0.25;

        let baseline = AdvancedBaseline::default();
        let cfg = ScoringConfig::default();

        let scorecard = score_against_baseline(&aggregated, &baseline, &cfg);

        // df_mean is ~2.4 std above baseline → should trigger warn or fail
        assert!(scorecard.alerts.len() >= 1, "Expected alert for df_mean deviation");
        assert!(
            scorecard.alerts.iter().any(|a| a.metric == "line.df_mean"),
            "Expected df_mean alert"
        );
    }

    #[test]
    fn test_generate_scorecard() {
        let metrics: Vec<QaAdvancedMetrics> = Vec::new();
        let scorecard = generate_scorecard(&metrics);
        assert_eq!(scorecard.runs, 0);
    }

    #[test]
    fn test_z_score_calculation() {
        let cfg = ScoringConfig::default();
        let mut alerts = Vec::new();

        // z = 0 → score = 100
        let score0 = score_metric(10.0, 10.0, 2.0, "test", &cfg, &mut alerts);
        assert!((score0 - 100.0).abs() < 0.1);

        // z = 1 → score = 75
        let score1 = score_metric(12.0, 10.0, 2.0, "test", &cfg, &mut alerts);
        assert!((score1 - 75.0).abs() < 0.1);

        // z = 2 → score = 50
        let score2 = score_metric(14.0, 10.0, 2.0, "test", &cfg, &mut alerts);
        assert!((score2 - 50.0).abs() < 0.1);

        // z = 4 → score = 0
        let score4 = score_metric(18.0, 10.0, 2.0, "test", &cfg, &mut alerts);
        assert!(score4 < 1.0);
    }
}
