//! Match Analysis - Automated Tactical Pattern Detection
//!
//! This module provides post-match analysis by detecting tactical patterns
//! from completed matches using existing MatchEvent and PositionData.
//!
//! ## Pattern Types
//! 1. **Possession Shifts**: Significant possession changes over time (≥15% change in 10-min windows)
//! 2. **Danger Timeline**: High-xG moments throughout match (xG ≥ 0.15)
//! 3. **Attack Zone Distribution**: Analysis of attack origins (9-zone grid)
//! 4. **Pressure Patterns**: High/low pressure periods by field thirds
//!
//! ## Design Philosophy
//! - **Memory Efficient**: Event-based reconstruction, no storage during simulation
//! - **Performance**: Single-pass algorithms, <50ms for 200-event match
//! - **Actionable**: Focus on tactical insights, not raw statistics
//!
//! ## Usage
//! ```rust,no_run
//! use of_core::engine::match_analysis::analyze_match;
//! use of_core::models::MatchResult;
//!
//! let match_result = MatchResult::default();
//! let report = analyze_match(&match_result);
//! println!("Found {} possession shifts", report.possession_shifts.len());
//! ```

use super::physics_constants::field;
use super::dsa_summary;
use super::interpretation_v1;
use crate::models::{EventType, MatchEvent, MatchResult};
use serde::{Deserialize, Serialize};

/// UI/Analytics meaning tier for high-xG moments.
///
/// Contract: UI must not derive highlight thresholds from raw xG values.
/// Rust assigns a deterministic tier here and UI renders by tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DangerMomentTier {
    Normal,
    High,
}

impl Default for DangerMomentTier {
    fn default() -> Self {
        Self::Normal
    }
}

impl DangerMomentTier {
    /// Deterministic tiering for post-match UI.
    ///
    /// Thresholds are defined here (SSOT) to prevent UI drift.
    pub fn from_xg_value(xg_value: f32) -> Self {
        // NOTE: Keep aligned with FIX_2601/0124 UI anti-pattern rule:
        // UI must not hardcode `xg >= 0.25`.
        if xg_value >= 0.25 {
            Self::High
        } else {
            Self::Normal
        }
    }
}

/// UI/Analytics meaning tier for possession shifts.
///
/// Contract: magnitude values are in percentage points (0..100),
/// and UI must not guess units or thresholds from raw numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PossessionShiftTier {
    Significant,
    Major,
    Extreme,
}

impl Default for PossessionShiftTier {
    fn default() -> Self {
        Self::Significant
    }
}

impl PossessionShiftTier {
    /// Deterministic tiering for post-match UI.
    ///
    /// Uses the same base significance threshold as detection (15pp),
    /// then splits higher magnitudes for stable visualization semantics.
    pub fn from_magnitude_pp(magnitude_pp: f32) -> Self {
        if magnitude_pp >= 35.0 {
            Self::Extreme
        } else if magnitude_pp >= 25.0 {
            Self::Major
        } else {
            Self::Significant
        }
    }
}

/// Complete match analysis report with all detected patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchAnalysisReport {
    /// Match duration in minutes (typically 90)
    pub duration_minutes: u8,
    /// Significant possession changes over time
    pub possession_shifts: Vec<PossessionShift>,
    /// High-xG moments throughout match
    pub danger_timeline: Vec<DangerMoment>,
    /// Attack zone distribution analysis
    pub attack_zones: AttackZoneAnalysis,
    /// Pressure patterns by field thirds
    pub pressure_patterns: Vec<PressurePeriod>,
    /// DSA v1.1 authoritative telemetry summary (derived from `position_data`) 
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub dsa_summary: Option<dsa_summary::DsaSummary>,
    /// Interpretation layer v1 (Replay/Analytics meaning layer, post-match).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub interpretation_v1: Option<interpretation_v1::MatchInterpretationV1>,
    /// Report generation timestamp
    pub generated_at_ms: u64,
}

/// Significant possession change detected in a time window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PossessionShift {
    /// Window start minute
    pub start_minute: u8,
    /// Window end minute
    pub end_minute: u8,
    /// Initial possession percentage for home team
    pub from_possession_home: f32,
    /// Final possession percentage for home team
    pub to_possession_home: f32,
    /// Absolute change magnitude
    ///
    /// Units: percentage points (0..100), e.g. 15.0 means 15%.
    pub magnitude: f32,
    /// Meaning tier for UI rendering (SSOT; UI must not threshold on magnitude).
    #[serde(default)]
    pub tier: PossessionShiftTier,
    /// Human-readable description
    pub description: String,
}

/// High-xG moment (chance created)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DangerMoment {
    /// Minute when chance occurred
    pub minute: u8,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Expected goals value (≥ 0.15)
    pub xg_value: f32,
    /// Meaning tier for UI rendering (SSOT; UI must not threshold on xG).
    #[serde(default)]
    pub tier: DangerMomentTier,
    /// Event type (Shot, ShotOnTarget, Goal)
    pub event_type: String,
    /// Player who created the chance
    pub player_name: String,
    /// True if home team, false if away
    pub is_home: bool,
    /// Ball position in meters (x, y)
    pub position: (f32, f32),
    /// Human-readable description
    pub description: String,
}

/// Attack zone distribution across 9 field zones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackZoneAnalysis {
    /// Individual zone statistics
    pub zones: Vec<AttackZone>,
    /// Name of zone with most attacks
    pub dominant_zone: String,
    /// Total attacks analyzed
    pub total_attacks: u32,
}

/// Single attack zone statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackZone {
    /// Zone name (e.g., "Left Final Third")
    pub zone_name: String,
    /// Number of attacks from this zone
    pub attack_count: u32,
    /// Percentage of total attacks
    pub percentage: f32,
    /// Zone center position in meters (for visualization)
    pub center_position: (f32, f32),
}

/// Pressure pattern detected in a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressurePeriod {
    /// Period start minute
    pub start_minute: u8,
    /// Period end minute
    pub end_minute: u8,
    /// Field third where pressure occurred (Defensive, Middle, Final)
    pub field_third: String,
    /// Team applying pressure (home/away)
    pub pressing_team: String,
    /// Intensity level (High, Normal, Low)
    pub intensity: String,
    /// Number of pressure events (tackles + fouls)
    pub event_count: u32,
    /// Human-readable description
    pub description: String,
}

/// Main analysis function - entry point for all pattern detection
pub fn analyze_match(result: &MatchResult) -> MatchAnalysisReport {
    let duration_minutes: u8 = 90; // Standard match duration
    let dsa = dsa_summary::analyze_dsa_summary(result, duration_minutes);
    MatchAnalysisReport {
        duration_minutes,
        possession_shifts: detect_possession_shifts(&result.events),
        danger_timeline: detect_danger_moments(&result.events),
        attack_zones: analyze_attack_zones(&result.events),
        pressure_patterns: detect_pressure_patterns(&result.events),
        dsa_summary: dsa.clone(),
        interpretation_v1: Some(interpretation_v1::build_interpretation_v1(
            result,
            dsa.as_ref(),
        )),
        generated_at_ms: current_timestamp_ms(),
    }
}

/// Detect significant possession changes over time
///
/// Algorithm:
/// 1. Divide match into 10-minute windows
/// 2. Count ball-touch events per team (passes, shots, tackles won)
/// 3. Calculate possession ratio: home_touches / (home + away)
/// 4. Compare consecutive windows
/// 5. If |delta| ≥ 15% → record as PossessionShift
fn detect_possession_shifts(events: &[MatchEvent]) -> Vec<PossessionShift> {
    const WINDOW_SIZE_MINUTES: u8 = 10;
    const SIGNIFICANCE_THRESHOLD: f32 = 15.0; // 15% change
    const MIN_EVENTS_PER_WINDOW: u32 = 5; // Minimum touches per team

    let mut shifts = Vec::new();
    let mut current_minute = 0;

    while current_minute + WINDOW_SIZE_MINUTES <= 90 {
        let window_start = current_minute;
        let window_end = current_minute + WINDOW_SIZE_MINUTES;

        // Count ball-touch events in current window
        let (home_touches, away_touches) = count_ball_touches(events, window_start, window_end);

        // Skip if too few events (not statistically significant)
        if home_touches < MIN_EVENTS_PER_WINDOW && away_touches < MIN_EVENTS_PER_WINDOW {
            current_minute += WINDOW_SIZE_MINUTES;
            continue;
        }

        // Calculate possession percentage
        let total_touches = home_touches + away_touches;
        let possession_home = if total_touches > 0 {
            (home_touches as f32 / total_touches as f32) * 100.0
        } else {
            50.0 // Default to 50-50 if no touches
        };

        // Compare with next window
        let next_window_end = window_end + WINDOW_SIZE_MINUTES;
        if next_window_end <= 90 {
            let (next_home, next_away) = count_ball_touches(events, window_end, next_window_end);
            let next_total = next_home + next_away;

            if next_total >= MIN_EVENTS_PER_WINDOW {
                let next_possession_home = (next_home as f32 / next_total as f32) * 100.0;
                let magnitude = (next_possession_home - possession_home).abs();

                if magnitude >= SIGNIFICANCE_THRESHOLD {
                    shifts.push(PossessionShift {
                        start_minute: window_start,
                        end_minute: next_window_end,
                        from_possession_home: possession_home,
                        to_possession_home: next_possession_home,
                        magnitude,
                        tier: PossessionShiftTier::from_magnitude_pp(magnitude),
                        description: format!(
                            "Possession {} from {:.0}% to {:.0}% (Minutes {}-{})",
                            if next_possession_home > possession_home {
                                "increased"
                            } else {
                                "dropped"
                            },
                            possession_home,
                            next_possession_home,
                            window_start,
                            next_window_end
                        ),
                    });
                }
            }
        }

        current_minute += WINDOW_SIZE_MINUTES;
    }

    shifts
}

/// Count ball-touch events (passes, shots, tackles won) in a time window
fn count_ball_touches(events: &[MatchEvent], start_minute: u8, end_minute: u8) -> (u32, u32) {
    let mut home_touches = 0;
    let mut away_touches = 0;

    for event in events {
        if event.minute >= start_minute && event.minute < end_minute {
            let is_touch = matches!(
                event.event_type,
                EventType::Pass
                    | EventType::Shot
                    | EventType::ShotOnTarget
                    | EventType::ShotOffTarget
                    | EventType::Tackle
                    | EventType::Dribble
            );

            if is_touch {
                if event.is_home_team {
                    home_touches += 1;
                } else {
                    away_touches += 1;
                }
            }
        }
    }

    (home_touches, away_touches)
}

/// Detect high-xG moments throughout the match
///
/// Algorithm:
/// 1. Filter events where event_type in [Shot, ShotOnTarget, Goal]
/// 2. Extract xG value from event details
/// 3. If xG ≥ 0.15 → record as DangerMoment
/// 4. Sort by xG value (descending)
/// 5. Return top 10 moments
fn detect_danger_moments(events: &[MatchEvent]) -> Vec<DangerMoment> {
    const XG_THRESHOLD: f32 = 0.15; // High-quality chances only
    const MAX_MOMENTS: usize = 10; // Top 10 moments

    let mut moments: Vec<DangerMoment> = events
        .iter()
        .filter_map(|event| {
            // Only consider shot-related events
            let is_shot_event = matches!(
                event.event_type,
                EventType::Shot | EventType::ShotOnTarget | EventType::Goal
            );

            if !is_shot_event {
                return None;
            }

            // Extract xG value
            let xg_value = event.details.as_ref().and_then(|d| d.xg_value).unwrap_or(0.0);

            // Filter by threshold
            if xg_value < XG_THRESHOLD {
                return None;
            }

            // Extract position (convert normalized to meters: 0-1 → 0-105, 0-68)
            let position = event
                .details
                .as_ref()
                .and_then(|d| d.ball_position)
                .map(|(x, y, _z)| (x * field::LENGTH_M, y * field::WIDTH_M))
                .unwrap_or((field::LENGTH_M / 2.0, field::WIDTH_M / 2.0)); // Default to center

            // C7: Use track_id for player identity
            let player_name = event
                .player_track_id
                .map(|id| format!("Player #{}", id))
                .unwrap_or_else(|| "Unknown".to_string());

            Some(DangerMoment {
                minute: event.minute,
                timestamp_ms: event.timestamp_ms.unwrap_or((event.minute as u64) * 60_000),
                xg_value,
                tier: DangerMomentTier::from_xg_value(xg_value),
                event_type: format!("{:?}", event.event_type),
                player_name: player_name.clone(),
                is_home: event.is_home_team,
                position,
                description: format!(
                    "High chance (xG: {:.2}) - {} at {}'",
                    xg_value, player_name, event.minute
                ),
            })
        })
        .collect();

    // Sort by xG (highest first)
    moments
        .sort_by(|a, b| b.xg_value.partial_cmp(&a.xg_value).unwrap_or(std::cmp::Ordering::Equal));

    // Take top N
    moments.truncate(MAX_MOMENTS);
    moments
}

/// Analyze attack zone distribution across 9 field zones
///
/// Zone Grid (3 lanes × 3 thirds):
/// ```text
///         Left (0-35m)  |  Center (35-70m)  |  Right (70-105m)
/// Final   LF (0.67-1.0) |  CF (0.67-1.0)    |  RF (0.67-1.0)
/// Middle  LM (0.33-0.67)|  CM (0.33-0.67)   |  RM (0.33-0.67)
/// Def     LD (0.0-0.33) |  CD (0.0-0.33)    |  RD (0.0-0.33)
/// ```
fn analyze_attack_zones(events: &[MatchEvent]) -> AttackZoneAnalysis {
    // Initialize zone counters
    let zone_names = [
        "Left Defensive",
        "Center Defensive",
        "Right Defensive",
        "Left Middle",
        "Center Middle",
        "Right Middle",
        "Left Final",
        "Center Final",
        "Right Final",
    ];

    let mut zone_counts = [0u32; 9];

    // Count shots by zone
    for event in events {
        if matches!(event.event_type, EventType::Shot | EventType::ShotOnTarget | EventType::Goal) {
            if let Some(ref details) = event.details {
                if let Some((x, y, _z)) = details.ball_position {
                    // Normalize for team direction (home attacks toward 1.0)
                    let norm_y = if event.is_home_team { y } else { 1.0 - y };

                    let zone_idx = categorize_zone_index(x, norm_y);
                    zone_counts[zone_idx] += 1;
                }
            }
        }
    }

    // Calculate percentages and create zone objects
    let total_attacks: u32 = zone_counts.iter().sum();
    let zones: Vec<AttackZone> = zone_names
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            let count = zone_counts[idx];
            let percentage =
                if total_attacks > 0 { (count as f32 / total_attacks as f32) * 100.0 } else { 0.0 };

            AttackZone {
                zone_name: name.to_string(),
                attack_count: count,
                percentage,
                center_position: get_zone_center(idx),
            }
        })
        .collect();

    // Find dominant zone
    let dominant_idx = zone_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, &count)| count)
        .map(|(idx, _)| idx)
        .unwrap_or(4); // Default to center middle

    AttackZoneAnalysis { zones, dominant_zone: zone_names[dominant_idx].to_string(), total_attacks }
}

/// Categorize position into zone index (0-8)
fn categorize_zone_index(x: f32, y: f32) -> usize {
    // Determine lane (0=Left, 1=Center, 2=Right)
    let lane = if x < 0.333 {
        0
    } else if x < 0.667 {
        1
    } else {
        2
    };

    // Determine third (0=Defensive, 1=Middle, 2=Final)
    let third = if y < 0.333 {
        0
    } else if y < 0.667 {
        1
    } else {
        2
    };

    third * 3 + lane
}

/// Get zone center position in meters
fn get_zone_center(zone_idx: usize) -> (f32, f32) {
    let third = zone_idx / 3;
    let lane = zone_idx % 3;

    let x = match lane {
        0 => 17.5, // Left: 0-35m, center at 17.5
        1 => field::CENTER_X, // Center: 35-70m, center at field center
        _ => 87.5, // Right: 70-105m, center at 87.5
    };

    let y = match third {
        0 => 11.33, // Defensive: 0-34m, center at 11.33
        1 => field::CENTER_Y,  // Middle: 34-68m, center at field center
        _ => 56.67, // Final: 68-102m, center at 56.67
    };

    (x, y)
}

/// Detect high/low pressure periods in field thirds
///
/// Algorithm:
/// 1. Divide match into 15-minute periods
/// 2. For each third (Defensive, Middle, Final):
///    - Count tackles + fouls
///    - Calculate event density: events / duration
/// 3. Classify: High (≥ 1.5 events/min), Normal, Low (≤ 0.5)
/// 4. Determine pressing team (more tackles in opponent's third)
fn detect_pressure_patterns(events: &[MatchEvent]) -> Vec<PressurePeriod> {
    const PERIOD_SIZE_MINUTES: u8 = 15;
    const HIGH_PRESSURE_THRESHOLD: f32 = 1.5; // events per minute
    const LOW_PRESSURE_THRESHOLD: f32 = 0.5;

    let mut patterns = Vec::new();
    let mut current_minute = 0;

    while current_minute + PERIOD_SIZE_MINUTES <= 90 {
        let period_start = current_minute;
        let period_end = current_minute + PERIOD_SIZE_MINUTES;

        // Count pressure events by third
        let (def_home, def_away, mid_home, mid_away, fin_home, fin_away) =
            count_pressure_events_by_third(events, period_start, period_end);

        // Analyze each third
        for (third_name, home_count, away_count) in [
            ("Defensive", def_home, def_away),
            ("Middle", mid_home, mid_away),
            ("Final", fin_home, fin_away),
        ] {
            let total_events = home_count + away_count;
            let event_density = total_events as f32 / PERIOD_SIZE_MINUTES as f32;

            let intensity = if event_density >= HIGH_PRESSURE_THRESHOLD {
                "High"
            } else if event_density <= LOW_PRESSURE_THRESHOLD {
                "Low"
            } else {
                "Normal"
            };

            // Determine pressing team (more tackles in opponent's third)
            let pressing_team = if home_count > away_count {
                "home"
            } else if away_count > home_count {
                "away"
            } else {
                "neutral"
            };

            // Only record non-normal patterns
            if intensity != "Normal" {
                patterns.push(PressurePeriod {
                    start_minute: period_start,
                    end_minute: period_end,
                    field_third: third_name.to_string(),
                    pressing_team: pressing_team.to_string(),
                    intensity: intensity.to_string(),
                    event_count: total_events,
                    description: format!(
                        "{} pressure in {} third by {} ({}-{} minutes)",
                        intensity,
                        third_name.to_lowercase(),
                        pressing_team,
                        period_start,
                        period_end
                    ),
                });
            }
        }

        current_minute += PERIOD_SIZE_MINUTES;
    }

    patterns
}

/// Count pressure events (tackles + fouls) by field third
/// Returns: (def_home, def_away, mid_home, mid_away, fin_home, fin_away)
fn count_pressure_events_by_third(
    events: &[MatchEvent],
    start_minute: u8,
    end_minute: u8,
) -> (u32, u32, u32, u32, u32, u32) {
    let mut def_home = 0;
    let mut def_away = 0;
    let mut mid_home = 0;
    let mut mid_away = 0;
    let mut fin_home = 0;
    let mut fin_away = 0;

    for event in events {
        if event.minute >= start_minute && event.minute < end_minute {
            let is_pressure = matches!(event.event_type, EventType::Tackle | EventType::Foul);

            if is_pressure {
                if let Some(ref details) = event.details {
                    if let Some((_x, y, _z)) = details.ball_position {
                        // Normalize for team direction
                        let norm_y = if event.is_home_team { y } else { 1.0 - y };

                        // Determine third
                        if norm_y < 0.333 {
                            // Defensive third
                            if event.is_home_team {
                                def_home += 1;
                            } else {
                                def_away += 1;
                            }
                        } else if norm_y < 0.667 {
                            // Middle third
                            if event.is_home_team {
                                mid_home += 1;
                            } else {
                                mid_away += 1;
                            }
                        } else {
                            // Final third
                            if event.is_home_team {
                                fin_home += 1;
                            } else {
                                fin_away += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    (def_home, def_away, mid_home, mid_away, fin_home, fin_away)
}

/// Get current timestamp in milliseconds (for report generation)
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EventDetails, EventType, MatchEvent, MatchResult, Statistics};

    /// Helper: Create mock MatchResult with given events
    fn create_mock_match_result(events: Vec<MatchEvent>) -> MatchResult {
        MatchResult {
            schema_version: 1,
            coord_contract_version: crate::engine::coordinate_contract::COORD_CONTRACT_VERSION,
            coord_system: crate::engine::coordinate_contract::COORD_SYSTEM_METERS_V2.to_string(),
            ssot_proof: crate::fix01::SsotProof::default(),
            determinism: Default::default(),
            score_home: 0,
            score_away: 0,
            events,
            statistics: Statistics::default(),
            position_data: None,
            replay_events: None,
            viewer_events: None,
            home_team: None,
            away_team: None,
            match_setup: None,
            debug_info: None,
            summary: None,
            board_summary: None,
            penalty_shootout: None,
            best_moments: None,
            shot_opp_telemetry: None,
        }
    }

    /// Helper: Create pass event
    fn create_pass_event(minute: u8, is_home: bool) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: Some((minute as u64) * 60_000),
            event_type: EventType::Pass,
            is_home_team: is_home,
            player_track_id: Some(if is_home { minute % 11 } else { 11 + (minute % 11) }),
            target_track_id: None,
            details: Some(EventDetails {
                ball_position: Some((0.5, 0.5, 0.0)),
                ..Default::default()
            }),
        }
    }

    /// Helper: Create shot event with xG
    fn create_shot_event(minute: u8, is_home: bool, xg: f32, pos: (f32, f32, f32)) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: Some((minute as u64) * 60_000),
            event_type: EventType::Shot,
            is_home_team: is_home,
            player_track_id: Some(if is_home { minute % 11 } else { 11 + (minute % 11) }),
            target_track_id: None,
            details: Some(EventDetails {
                xg_value: Some(xg),
                ball_position: Some(pos),
                ..Default::default()
            }),
        }
    }

    /// Helper: Create tackle event at position
    fn create_tackle_event(minute: u8, is_home: bool, pos: (f32, f32, f32)) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: Some((minute as u64) * 60_000),
            event_type: EventType::Tackle,
            is_home_team: is_home,
            player_track_id: Some(if is_home { minute % 11 } else { 11 + (minute % 11) }),
            target_track_id: None,
            details: Some(EventDetails { ball_position: Some(pos), ..Default::default() }),
        }
    }

    #[test]
    fn test_possession_shift_detection() {
        // Create events simulating possession shift
        let mut events = Vec::new();

        // Minutes 0-9: Home dominates (15 home touches vs 5 away touches = 75% home)
        for i in 0..15 {
            events.push(create_pass_event(i % 10, true)); // Spread across minutes 0-9
        }
        for i in 0..5 {
            events.push(create_pass_event(i % 10, false)); // Spread across minutes 0-4
        }

        // Minutes 10-19: Away dominates (5 home touches vs 15 away touches = 25% home)
        for i in 0..5 {
            events.push(create_pass_event(10 + (i % 10), true)); // Minutes 10-14
        }
        for i in 0..15 {
            events.push(create_pass_event(10 + (i % 10), false)); // Minutes 10-19
        }

        let shifts = detect_possession_shifts(&events);

        // Should detect one significant shift
        assert!(!shifts.is_empty(), "Should detect at least one possession shift");

        let first_shift = &shifts[0];
        assert_eq!(first_shift.start_minute, 0);
        assert_eq!(first_shift.end_minute, 20);
        assert!(
            first_shift.from_possession_home > 50.0,
            "Home should start with >50% possession (got {}%)",
            first_shift.from_possession_home
        );
        assert!(
            first_shift.to_possession_home < 50.0,
            "Home should end with <50% possession (got {}%)",
            first_shift.to_possession_home
        );
        assert!(
            first_shift.magnitude >= 15.0,
            "Magnitude should be >= 15% (got {}%)",
            first_shift.magnitude
        );
        assert_eq!(
            first_shift.tier,
            PossessionShiftTier::Extreme,
            "Tier should be derived in Rust (SSOT) and reflect large magnitude shifts"
        );
    }

    #[test]
    fn test_possession_shift_no_change() {
        // Create events with consistent possession (no shifts)
        let mut events = Vec::new();

        // Minutes 0-30: Consistent 50-50 possession
        for minute in 0..30 {
            events.push(create_pass_event(minute, true));
            events.push(create_pass_event(minute, false));
        }

        let shifts = detect_possession_shifts(&events);

        // Should not detect any significant shifts
        assert!(shifts.is_empty(), "Should not detect shifts with consistent possession");
    }

    #[test]
    fn test_danger_moments_filtering() {
        // Create shots with varying xG values
        let events = vec![
            create_shot_event(10, true, 0.05, (0.8, 0.5, 0.0)), // Low xG - should be filtered
            create_shot_event(20, true, 0.20, (0.85, 0.45, 0.0)), // High xG - should be included
            create_shot_event(30, false, 0.30, (0.15, 0.55, 0.0)), // High xG - should be included
            create_shot_event(40, true, 0.10, (0.7, 0.5, 0.0)), // Low xG - should be filtered
            create_shot_event(50, false, 0.25, (0.2, 0.5, 0.0)), // High xG - should be included
        ];

        let moments = detect_danger_moments(&events);

        // Should only include shots with xG >= 0.15
        assert_eq!(moments.len(), 3, "Should have exactly 3 high-xG moments");

        // Should be sorted by xG (descending)
        assert!(moments[0].xg_value >= moments[1].xg_value);
        assert!(moments[1].xg_value >= moments[2].xg_value);

        // Highest should be 0.30
        assert_eq!(moments[0].xg_value, 0.30);
        assert_eq!(moments[0].minute, 30);
        assert_eq!(
            moments[0].tier,
            DangerMomentTier::High,
            "Tier should be derived in Rust (SSOT) for high-xG moments"
        );
        assert_eq!(
            moments[2].xg_value,
            0.20,
            "Lowest included moment should still be included by XG_THRESHOLD"
        );
        assert_eq!(
            moments[2].tier,
            DangerMomentTier::Normal,
            "Tier should not be decided in UI via xg thresholds"
        );
    }

    #[test]
    fn test_danger_moments_max_limit() {
        // Create 15 high-xG shots
        let mut events = Vec::new();
        for i in 0..15 {
            events.push(create_shot_event(i, true, 0.20 + (i as f32 * 0.01), (0.8, 0.5, 0.0)));
        }

        let moments = detect_danger_moments(&events);

        // Should cap at 10 moments
        assert!(moments.len() <= 10, "Should not exceed 10 moments");
    }

    #[test]
    fn test_zone_categorization() {
        // Test all 9 zones with edge cases

        // Left Defensive (0-0.333, 0-0.333)
        assert_eq!(categorize_zone_index(0.1, 0.1), 0);

        // Center Defensive (0.333-0.667, 0-0.333)
        assert_eq!(categorize_zone_index(0.5, 0.2), 1);

        // Right Defensive (0.667-1.0, 0-0.333)
        assert_eq!(categorize_zone_index(0.8, 0.3), 2);

        // Left Middle (0-0.333, 0.333-0.667)
        assert_eq!(categorize_zone_index(0.2, 0.5), 3);

        // Center Middle (0.333-0.667, 0.333-0.667)
        assert_eq!(categorize_zone_index(0.5, 0.5), 4);

        // Right Middle (0.667-1.0, 0.333-0.667)
        assert_eq!(categorize_zone_index(0.9, 0.6), 5);

        // Left Final (0-0.333, 0.667-1.0)
        assert_eq!(categorize_zone_index(0.1, 0.8), 6);

        // Center Final (0.333-0.667, 0.667-1.0)
        assert_eq!(categorize_zone_index(0.5, 0.9), 7);

        // Right Final (0.667-1.0, 0.667-1.0)
        assert_eq!(categorize_zone_index(0.95, 0.95), 8);

        // Test boundaries
        assert_eq!(categorize_zone_index(0.333, 0.333), 4); // Should be center middle
        assert_eq!(categorize_zone_index(0.667, 0.667), 8); // Should be right final
    }

    #[test]
    fn test_attack_zone_analysis() {
        // Create shots distributed across zones
        let events = vec![
            // 3 shots from Left Final (zone 6)
            create_shot_event(10, true, 0.20, (0.1, 0.8, 0.0)),
            create_shot_event(15, true, 0.25, (0.2, 0.9, 0.0)),
            create_shot_event(20, true, 0.18, (0.15, 0.85, 0.0)),
            // 2 shots from Center Final (zone 7)
            create_shot_event(30, true, 0.30, (0.5, 0.9, 0.0)),
            create_shot_event(35, true, 0.28, (0.45, 0.85, 0.0)),
            // 1 shot from Right Middle (zone 5)
            create_shot_event(40, true, 0.15, (0.8, 0.5, 0.0)),
        ];

        let analysis = analyze_attack_zones(&events);

        // Should have 9 zones total
        assert_eq!(analysis.zones.len(), 9);

        // Total attacks should be 6
        assert_eq!(analysis.total_attacks, 6);

        // Left Final should have 50% (3 out of 6)
        let left_final = analysis.zones.iter().find(|z| z.zone_name == "Left Final").unwrap();
        assert_eq!(left_final.attack_count, 3);
        assert!((left_final.percentage - 50.0).abs() < 0.1);

        // Dominant zone should be Left Final
        assert_eq!(analysis.dominant_zone, "Left Final");
    }

    #[test]
    fn test_attack_zone_empty() {
        // No shots
        let events = vec![create_pass_event(10, true), create_pass_event(20, false)];

        let analysis = analyze_attack_zones(&events);

        // Should have 9 zones with 0 attacks
        assert_eq!(analysis.zones.len(), 9);
        assert_eq!(analysis.total_attacks, 0);

        for zone in &analysis.zones {
            assert_eq!(zone.attack_count, 0);
            assert_eq!(zone.percentage, 0.0);
        }
    }

    #[test]
    fn test_pressure_pattern_detection() {
        // Create high-pressure period in final third
        let mut events = Vec::new();

        // Minutes 0-15: 25 tackles in final third (high pressure)
        for i in 0..25 {
            let minute = (i % 15) as u8;
            events.push(create_tackle_event(minute, true, (0.5, 0.8, 0.0))); // Final third
        }

        // Minutes 15-30: 5 tackles in defensive third (low pressure)
        for i in 0..5 {
            let minute = 15 + (i % 15) as u8;
            events.push(create_tackle_event(minute, true, (0.5, 0.2, 0.0))); // Defensive third
        }

        let patterns = detect_pressure_patterns(&events);

        // Should detect at least one pressure pattern
        assert!(!patterns.is_empty(), "Should detect at least one pressure pattern");

        // Find high pressure pattern
        let high_pressure = patterns.iter().find(|p| p.intensity == "High");
        assert!(high_pressure.is_some(), "Should detect high pressure pattern");

        if let Some(pattern) = high_pressure {
            assert_eq!(pattern.field_third, "Final");
            assert!(pattern.event_count >= 20);
        }

        // Find low pressure pattern
        let low_pressure = patterns.iter().find(|p| p.intensity == "Low");
        assert!(low_pressure.is_some(), "Should detect low pressure pattern");
    }

    #[test]
    fn test_pressure_pattern_intensity_thresholds() {
        // Test boundary conditions for intensity classification

        // High pressure: >= 1.5 events/min = >= 22.5 events in 15 minutes
        let mut high_events = Vec::new();
        for i in 0..23 {
            high_events.push(create_tackle_event((i % 15) as u8, true, (0.5, 0.8, 0.0)));
        }
        let high_patterns = detect_pressure_patterns(&high_events);
        let has_high = high_patterns.iter().any(|p| p.intensity == "High");
        assert!(has_high, "Should classify as high pressure");

        // Low pressure: <= 0.5 events/min = <= 7.5 events in 15 minutes
        let mut low_events = Vec::new();
        for i in 0..7 {
            low_events.push(create_tackle_event((i % 15) as u8, true, (0.5, 0.8, 0.0)));
        }
        let low_patterns = detect_pressure_patterns(&low_events);
        let has_low = low_patterns.iter().any(|p| p.intensity == "Low");
        assert!(has_low, "Should classify as low pressure");
    }

    #[test]
    fn test_full_match_analysis() {
        // Create realistic match with all event types
        let mut events = Vec::new();

        // Add possession events
        for minute in 0..90 {
            if minute < 45 {
                // First half: Home dominates
                events.push(create_pass_event(minute, true));
                if minute % 3 == 0 {
                    events.push(create_pass_event(minute, false));
                }
            } else {
                // Second half: Away dominates
                events.push(create_pass_event(minute, false));
                if minute % 3 == 0 {
                    events.push(create_pass_event(minute, true));
                }
            }
        }

        // Add high-xG shots
        events.push(create_shot_event(15, true, 0.25, (0.8, 0.5, 0.0)));
        events.push(create_shot_event(30, false, 0.30, (0.2, 0.5, 0.0)));
        events.push(create_shot_event(60, true, 0.20, (0.85, 0.45, 0.0)));

        // Add pressure events
        for i in 0..20 {
            events.push(create_tackle_event((i % 15) as u8, true, (0.5, 0.8, 0.0)));
        }

        let result = create_mock_match_result(events);
        let report = analyze_match(&result);

        // Verify structure
        assert_eq!(report.duration_minutes, 90);
        assert!(!report.possession_shifts.is_empty(), "Should have possession shifts");
        assert!(!report.danger_timeline.is_empty(), "Should have danger moments");
        assert_eq!(report.attack_zones.zones.len(), 9, "Should have 9 zones");
        assert_eq!(report.attack_zones.total_attacks, 3, "Should have 3 shots");
        assert!(report.generated_at_ms > 0, "Should have valid timestamp");
    }

    #[test]
    fn test_edge_case_low_event_match() {
        // Match with very few events (< 50)
        let events = vec![
            create_pass_event(10, true),
            create_pass_event(20, false),
            create_shot_event(30, true, 0.20, (0.8, 0.5, 0.0)),
        ];

        let result = create_mock_match_result(events);
        let report = analyze_match(&result);

        // Should not crash and return valid structure
        assert_eq!(report.duration_minutes, 90);
        assert_eq!(report.attack_zones.zones.len(), 9);
        // Possession shifts might be empty due to low event count
        // This is expected behavior (graceful degradation)
    }

    #[test]
    fn test_edge_case_one_sided_match() {
        // Match with 95% possession to one team
        let mut events = Vec::new();
        for minute in 0..90 {
            // 19 home touches per minute
            for _ in 0..19 {
                events.push(create_pass_event(minute, true));
            }
            // 1 away touch per minute
            events.push(create_pass_event(minute, false));
        }

        let shifts = detect_possession_shifts(&events);

        // Should not detect any shifts (possession remains constant)
        assert!(shifts.is_empty(), "Should not detect shifts in one-sided match");
    }

    #[test]
    fn test_edge_case_missing_xg_value() {
        // Shot event without xG value
        let mut event = create_shot_event(20, true, 0.0, (0.8, 0.5, 0.0));
        if let Some(ref mut details) = event.details {
            details.xg_value = None; // Remove xG value
        }

        let events = vec![event];
        let moments = detect_danger_moments(&events);

        // Should gracefully handle missing xG (default to 0.0, filtered out)
        assert!(moments.is_empty(), "Should filter out shots with no xG value");
    }

    #[test]
    fn test_edge_case_missing_ball_position() {
        // Shot event without position
        let mut event = create_shot_event(20, true, 0.25, (0.0, 0.0, 0.0));
        if let Some(ref mut details) = event.details {
            details.ball_position = None; // Remove position
        }

        let events = vec![event];
        let moments = detect_danger_moments(&events);

        // Should use default center position
        assert_eq!(moments.len(), 1);
        assert_eq!(moments[0].position, (field::CENTER_X, field::CENTER_Y), "Should default to center");
    }

    #[test]
    fn test_zone_center_positions() {
        // Verify zone center positions are correct
        assert_eq!(get_zone_center(0), (17.5, 11.33)); // Left Defensive
        assert_eq!(get_zone_center(4), (field::CENTER_X, field::CENTER_Y)); // Center Middle
        assert_eq!(get_zone_center(8), (87.5, 56.67)); // Right Final
    }
}
