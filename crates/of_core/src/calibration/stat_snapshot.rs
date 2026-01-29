//! Match Statistics Snapshot
//!
//! Collects per-match statistics for calibration.
//!
//! FIX_2601/0113: Added 20-zone tactical metrics tracking.
//! FIX_2601/NEW_FUNC: Added per-player pass statistics for Gini analysis.

use super::zone::{ZoneId, PosPlayZoneId};
use crate::analysis::metrics::gini::GiniMetrics;
use crate::models::{MatchResult, MatchEvent, EventType};

/// Per-team match statistics snapshot
#[derive(Debug, Clone, Default)]
pub struct MatchStatSnapshot {
    // Metadata
    pub team_id: u32,
    pub match_id: u64,

    // Defensive Actions
    pub tackles: u32,
    pub tackle_successes: u32,
    pub interceptions: u32,
    pub clearances: u32,
    pub press_events: u32,
    pub blocks: u32,

    // Passes
    pub pass_attempts: u32,
    pub pass_successes: u32,
    pub progressive_passes: u32,
    pub key_passes: u32,
    pub crosses: u32,
    pub cross_successes: u32,
    pub long_passes: u32,
    pub long_pass_successes: u32,
    pub backward_passes: u32,
    pub lateral_passes: u32,

    // Shots
    pub shot_attempts: u32,
    pub shots_on_target: u32,
    pub goals: u32,
    pub xg_total: f32,

    // Zone Distribution - Layer 1: 6-zone (indexed by ZoneId)
    pub touches_by_zone: [u32; 6],
    pub shots_by_zone: [u32; 6],
    pub xg_by_zone: [f32; 6],
    pub passes_from_zone: [u32; 6],
    pub passes_to_zone: [u32; 6],

    // Zone Distribution - Layer 2: 20-zone tactical (indexed by PosPlayZoneId)
    pub touches_by_posplay_zone: [u32; 20],
    pub passes_from_posplay_zone: [u32; 20],
    pub passes_to_posplay_zone: [u32; 20],

    // Halfspace pass tracking (for entry rate calculation)
    pub halfspace_entry_passes: u32,
    pub shots_from_halfspace: u32,

    // FIX_2601/NEW_FUNC: Per-player statistics for Gini analysis
    pub player_stats: PlayerPassStats,
}

// ============================================================================
// FIX_2601/NEW_FUNC: Per-Player Statistics for Gini Analysis
// ============================================================================

/// Per-player pass and touch statistics.
///
/// Used for calculating Gini coefficients to detect concentration/monopoly patterns.
#[derive(Debug, Clone)]
pub struct PlayerPassStats {
    /// Passes sent per player [11]
    pub passes_sent: [u32; 11],
    /// Passes received per player [11]
    pub passes_received: [u32; 11],
    /// Total touches per player [11]
    pub touches: [u32; 11],
    /// Progressive passes sent per player [11]
    pub progressive_sent: [u32; 11],
    /// Key passes sent per player [11]
    pub key_passes: [u32; 11],
}

impl Default for PlayerPassStats {
    fn default() -> Self {
        Self {
            passes_sent: [0; 11],
            passes_received: [0; 11],
            touches: [0; 11],
            progressive_sent: [0; 11],
            key_passes: [0; 11],
        }
    }
}

impl PlayerPassStats {
    /// Calculate Gini metrics from player statistics.
    pub fn gini_metrics(&self) -> GiniMetrics {
        GiniMetrics::from_player_stats(
            &self.touches,
            &self.passes_sent,
            &self.passes_received,
            &self.progressive_sent,
        )
    }

    /// Record a pass from one player to another.
    pub fn record_pass(&mut self, passer_idx: usize, receiver_idx: usize, is_progressive: bool, is_key: bool) {
        if passer_idx < 11 {
            self.passes_sent[passer_idx] += 1;
            if is_progressive {
                self.progressive_sent[passer_idx] += 1;
            }
            if is_key {
                self.key_passes[passer_idx] += 1;
            }
        }
        if receiver_idx < 11 {
            self.passes_received[receiver_idx] += 1;
        }
    }

    /// Record a touch by a player.
    pub fn record_touch(&mut self, player_idx: usize) {
        if player_idx < 11 {
            self.touches[player_idx] += 1;
        }
    }

    /// Get the player with the most touches.
    pub fn most_touches_player(&self) -> (usize, u32) {
        self.touches.iter()
            .enumerate()
            .max_by_key(|&(_, count)| count)
            .map(|(idx, &count)| (idx, count))
            .unwrap_or((0, 0))
    }

    /// Get the player who sends the most passes.
    pub fn most_passes_sent_player(&self) -> (usize, u32) {
        self.passes_sent.iter()
            .enumerate()
            .max_by_key(|&(_, count)| count)
            .map(|(idx, &count)| (idx, count))
            .unwrap_or((0, 0))
    }

    /// Get the player who receives the most passes.
    pub fn most_passes_received_player(&self) -> (usize, u32) {
        self.passes_received.iter()
            .enumerate()
            .max_by_key(|&(_, count)| count)
            .map(|(idx, &count)| (idx, count))
            .unwrap_or((0, 0))
    }

    /// Total touches across all players.
    pub fn total_touches(&self) -> u32 {
        self.touches.iter().sum()
    }

    /// Total passes sent.
    pub fn total_passes(&self) -> u32 {
        self.passes_sent.iter().sum()
    }
}

// ============================================================================
// FIX_2601/0113: Tactical Metrics Structs
// ============================================================================

/// Half-space tactical metrics
#[derive(Debug, Clone, Default)]
pub struct HalfSpaceMetrics {
    /// Half-space touch rate (total HS touches / total touches)
    pub touch_rate: f32,
    /// Half-space entry pass rate (HS entry passes / total passes)
    pub entry_pass_rate: f32,
    /// Shot from half-space rate (HS shots / total shots)
    pub shot_from_hs_rate: f32,
}

/// Lane occupancy metrics (5 lanes: LW, LHS, C, RHS, RW)
#[derive(Debug, Clone)]
pub struct LaneOccupancy {
    /// Touch share per lane [LW, LHS, C, RHS, RW]
    pub touch_by_lane: [f32; 5],
    /// Entropy (0 = single lane, 1 = uniform)
    pub entropy: f32,
    /// Balance (1 = symmetric, 0 = one-sided)
    pub balance: f32,
}

impl Default for LaneOccupancy {
    fn default() -> Self {
        Self {
            touch_by_lane: [0.2; 5],
            entropy: 1.0,
            balance: 1.0,
        }
    }
}

/// Zone progression metrics (advancement through quarters)
#[derive(Debug, Clone, Default)]
pub struct ZoneProgression {
    /// DEF → MID progression rate
    pub def_to_mid_rate: f32,
    /// MID → FIN progression rate
    pub mid_to_fin_rate: f32,
    /// FIN → BOX progression rate
    pub fin_to_box_rate: f32,
    /// Direct DEF → BOX rate (long ball)
    pub direct_to_box_rate: f32,
}

impl MatchStatSnapshot {
    /// Create a new empty snapshot
    pub fn new(team_id: u32, match_id: u64) -> Self {
        Self {
            team_id,
            match_id,
            ..Default::default()
        }
    }

    /// Record a touch in a zone (6-zone only)
    pub fn record_touch(&mut self, zone: ZoneId) {
        self.touches_by_zone[zone.index()] += 1;
    }

    /// Record a touch with both 6-zone and 20-zone tracking
    pub fn record_touch_posplay(&mut self, zone6: ZoneId, zone20: PosPlayZoneId) {
        self.touches_by_zone[zone6.index()] += 1;
        self.touches_by_posplay_zone[zone20.to_index()] += 1;
    }

    /// Record a pass attempt
    pub fn record_pass(
        &mut self,
        success: bool,
        is_progressive: bool,
        is_key: bool,
        is_cross: bool,
        is_long: bool,
        is_backward: bool,
        from_zone: ZoneId,
        to_zone: ZoneId,
    ) {
        self.pass_attempts += 1;
        self.passes_from_zone[from_zone.index()] += 1;
        self.passes_to_zone[to_zone.index()] += 1;

        if success {
            self.pass_successes += 1;
        }

        if is_progressive {
            self.progressive_passes += 1;
        }
        if is_key {
            self.key_passes += 1;
        }
        if is_cross {
            self.crosses += 1;
            if success {
                self.cross_successes += 1;
            }
        }
        if is_long {
            self.long_passes += 1;
            if success {
                self.long_pass_successes += 1;
            }
        }
        if is_backward {
            self.backward_passes += 1;
        }
        if !is_progressive && !is_backward && !is_long {
            self.lateral_passes += 1;
        }
    }

    /// Record a pass with 20-zone tracking
    pub fn record_pass_posplay(
        &mut self,
        success: bool,
        is_progressive: bool,
        is_key: bool,
        is_cross: bool,
        is_long: bool,
        is_backward: bool,
        from_zone6: ZoneId,
        to_zone6: ZoneId,
        from_zone20: PosPlayZoneId,
        to_zone20: PosPlayZoneId,
    ) {
        // Record 6-zone stats
        self.record_pass(
            success, is_progressive, is_key, is_cross, is_long, is_backward,
            from_zone6, to_zone6
        );

        // Record 20-zone stats
        self.passes_from_posplay_zone[from_zone20.to_index()] += 1;
        self.passes_to_posplay_zone[to_zone20.to_index()] += 1;

        // Track halfspace entry passes (wide → halfspace)
        if !from_zone20.is_halfspace() && to_zone20.is_halfspace() {
            self.halfspace_entry_passes += 1;
        }
    }

    /// Record a pass with full tracking (zones + player indices).
    ///
    /// FIX_2601/NEW_FUNC: Extended to track per-player statistics for Gini analysis.
    pub fn record_pass_full(
        &mut self,
        passer_idx: usize,
        receiver_idx: usize,
        success: bool,
        is_progressive: bool,
        is_key: bool,
        is_cross: bool,
        is_long: bool,
        is_backward: bool,
        from_zone6: ZoneId,
        to_zone6: ZoneId,
        from_zone20: PosPlayZoneId,
        to_zone20: PosPlayZoneId,
    ) {
        // Record zone-based stats
        self.record_pass_posplay(
            success, is_progressive, is_key, is_cross, is_long, is_backward,
            from_zone6, to_zone6, from_zone20, to_zone20
        );

        // Record player-based stats for Gini analysis
        self.player_stats.record_pass(passer_idx, receiver_idx, is_progressive, is_key);
    }

    /// Record a touch with player index for Gini analysis.
    ///
    /// FIX_2601/NEW_FUNC: Extended to track per-player touches.
    pub fn record_touch_with_player(&mut self, player_idx: usize, zone6: ZoneId, zone20: PosPlayZoneId) {
        self.record_touch_posplay(zone6, zone20);
        self.player_stats.record_touch(player_idx);
    }

    /// Get Gini metrics for this team's pass distribution.
    ///
    /// FIX_2601/NEW_FUNC: Returns Gini coefficients for inequality analysis.
    pub fn gini_metrics(&self) -> GiniMetrics {
        self.player_stats.gini_metrics()
    }

    /// Record a shot (6-zone only)
    pub fn record_shot(&mut self, on_target: bool, is_goal: bool, xg: f32, zone: ZoneId) {
        self.shot_attempts += 1;
        self.shots_by_zone[zone.index()] += 1;
        self.xg_total += xg;
        self.xg_by_zone[zone.index()] += xg;

        if on_target {
            self.shots_on_target += 1;
        }
        if is_goal {
            self.goals += 1;
        }
    }

    /// Record a shot with 20-zone tracking
    pub fn record_shot_posplay(
        &mut self,
        on_target: bool,
        is_goal: bool,
        xg: f32,
        zone6: ZoneId,
        zone20: PosPlayZoneId,
    ) {
        self.record_shot(on_target, is_goal, xg, zone6);

        // Track shots from halfspace
        if zone20.is_halfspace() {
            self.shots_from_halfspace += 1;
        }
    }

    /// Record a tackle
    pub fn record_tackle(&mut self, success: bool) {
        self.tackles += 1;
        if success {
            self.tackle_successes += 1;
        }
    }

    /// Record an interception
    pub fn record_interception(&mut self) {
        self.interceptions += 1;
    }

    /// Record a clearance
    pub fn record_clearance(&mut self) {
        self.clearances += 1;
    }

    /// Record a press event
    pub fn record_press(&mut self) {
        self.press_events += 1;
    }

    /// Record a block
    pub fn record_block(&mut self) {
        self.blocks += 1;
    }

    // === Computed Statistics ===

    /// Pass success rate (0.0 - 1.0)
    pub fn pass_success_rate(&self) -> f32 {
        if self.pass_attempts == 0 {
            return 0.0;
        }
        self.pass_successes as f32 / self.pass_attempts as f32
    }

    /// Progressive pass share
    pub fn progressive_share(&self) -> f32 {
        if self.pass_attempts == 0 {
            return 0.0;
        }
        self.progressive_passes as f32 / self.pass_attempts as f32
    }

    /// Key pass share
    pub fn key_pass_share(&self) -> f32 {
        if self.pass_attempts == 0 {
            return 0.0;
        }
        self.key_passes as f32 / self.pass_attempts as f32
    }

    /// Cross share
    pub fn cross_share(&self) -> f32 {
        if self.pass_attempts == 0 {
            return 0.0;
        }
        self.crosses as f32 / self.pass_attempts as f32
    }

    /// Long pass share
    pub fn long_pass_share(&self) -> f32 {
        if self.pass_attempts == 0 {
            return 0.0;
        }
        self.long_passes as f32 / self.pass_attempts as f32
    }

    /// Shots on target rate
    pub fn on_target_rate(&self) -> f32 {
        if self.shot_attempts == 0 {
            return 0.0;
        }
        self.shots_on_target as f32 / self.shot_attempts as f32
    }

    /// Goal conversion rate
    pub fn conversion_rate(&self) -> f32 {
        if self.shot_attempts == 0 {
            return 0.0;
        }
        self.goals as f32 / self.shot_attempts as f32
    }

    /// Touch distribution by zone (normalized)
    pub fn touch_distribution(&self) -> [f32; 6] {
        let total: u32 = self.touches_by_zone.iter().sum();
        if total == 0 {
            return [0.0; 6];
        }
        let mut dist = [0.0; 6];
        for (i, &count) in self.touches_by_zone.iter().enumerate() {
            dist[i] = count as f32 / total as f32;
        }
        dist
    }

    /// xG distribution by zone (normalized)
    pub fn xg_distribution(&self) -> [f32; 6] {
        if self.xg_total <= 0.0 {
            return [0.0; 6];
        }
        let mut dist = [0.0; 6];
        for (i, &xg) in self.xg_by_zone.iter().enumerate() {
            dist[i] = xg / self.xg_total;
        }
        dist
    }

    // ========================================================================
    // FIX_2601/0113: Tactical Metrics (20-zone based)
    // ========================================================================

    /// Calculate half-space tactical metrics
    pub fn halfspace_metrics(&self) -> HalfSpaceMetrics {
        // Half-space zone indices: LHS(1,6,11,16), RHS(3,8,13,18)
        const HS_INDICES: [usize; 8] = [1, 3, 6, 8, 11, 13, 16, 18];

        let hs_touches: u32 = HS_INDICES.iter()
            .map(|&i| self.touches_by_posplay_zone[i])
            .sum();
        let total_touches: u32 = self.touches_by_posplay_zone.iter().sum();

        let touch_rate = if total_touches > 0 {
            hs_touches as f32 / total_touches as f32
        } else {
            0.0
        };

        let entry_pass_rate = if self.pass_attempts > 0 {
            self.halfspace_entry_passes as f32 / self.pass_attempts as f32
        } else {
            0.0
        };

        let shot_from_hs_rate = if self.shot_attempts > 0 {
            self.shots_from_halfspace as f32 / self.shot_attempts as f32
        } else {
            0.0
        };

        HalfSpaceMetrics {
            touch_rate,
            entry_pass_rate,
            shot_from_hs_rate,
        }
    }

    /// Calculate lane occupancy metrics
    pub fn lane_occupancy(&self) -> LaneOccupancy {
        // Lane groupings (4 quarters per lane)
        const LANE_INDICES: [[usize; 4]; 5] = [
            [0, 5, 10, 15],   // LW
            [1, 6, 11, 16],   // LHS
            [2, 7, 12, 17],   // C
            [3, 8, 13, 18],   // RHS
            [4, 9, 14, 19],   // RW
        ];

        let mut lane_touches = [0u32; 5];
        for (lane_idx, indices) in LANE_INDICES.iter().enumerate() {
            for &zone_idx in indices {
                lane_touches[lane_idx] += self.touches_by_posplay_zone[zone_idx];
            }
        }

        let total: u32 = lane_touches.iter().sum();
        let touch_by_lane = if total > 0 {
            [
                lane_touches[0] as f32 / total as f32,
                lane_touches[1] as f32 / total as f32,
                lane_touches[2] as f32 / total as f32,
                lane_touches[3] as f32 / total as f32,
                lane_touches[4] as f32 / total as f32,
            ]
        } else {
            [0.2; 5]
        };

        // Calculate entropy: -sum(p * log2(p)) / log2(5)
        let entropy = Self::calculate_entropy(&touch_by_lane);

        // Calculate balance: 1 - |left_sum - right_sum|
        let left_sum = touch_by_lane[0] + touch_by_lane[1];
        let right_sum = touch_by_lane[3] + touch_by_lane[4];
        let balance = 1.0 - (left_sum - right_sum).abs();

        LaneOccupancy {
            touch_by_lane,
            entropy,
            balance,
        }
    }

    /// Calculate Shannon entropy (normalized to 0-1)
    fn calculate_entropy(shares: &[f32; 5]) -> f32 {
        let mut entropy = 0.0f32;
        for &share in shares {
            if share > 0.0 {
                entropy -= share * share.log2();
            }
        }
        // Normalize by max entropy (log2(5) ≈ 2.322)
        entropy / 5.0f32.log2()
    }

    /// Calculate zone progression metrics
    ///
    /// Phase 6.1: Updated to use passes_from_posplay_zone (pass origin data)
    /// since passes_to_posplay_zone (pass destination) is not available from events.
    pub fn zone_progression(&self) -> ZoneProgression {
        // Quarter groupings
        const DEF_INDICES: [usize; 5] = [0, 1, 2, 3, 4];      // DEF quarter
        const MID_INDICES: [usize; 5] = [5, 6, 7, 8, 9];      // MID quarter
        const FIN_INDICES: [usize; 5] = [10, 11, 12, 13, 14]; // FIN quarter
        const BOX_INDICES: [usize; 5] = [15, 16, 17, 18, 19]; // BOX quarter

        let passes_from_def: u32 = DEF_INDICES.iter()
            .map(|&i| self.passes_from_posplay_zone[i])
            .sum();
        let passes_from_mid: u32 = MID_INDICES.iter()
            .map(|&i| self.passes_from_posplay_zone[i])
            .sum();
        let passes_from_fin: u32 = FIN_INDICES.iter()
            .map(|&i| self.passes_from_posplay_zone[i])
            .sum();
        let passes_from_box: u32 = BOX_INDICES.iter()
            .map(|&i| self.passes_from_posplay_zone[i])
            .sum();

        let total_passes_from_zones = passes_from_def + passes_from_mid + passes_from_fin + passes_from_box;

        // Phase 6.1: Calculate progression rates based on pass origin zones
        // def_to_mid_rate: ratio of passes from MID or beyond (non-defensive passes)
        // This indicates how much the team has progressed beyond the defensive third
        let def_to_mid_rate = if total_passes_from_zones > 0 {
            (passes_from_mid + passes_from_fin + passes_from_box) as f32 / total_passes_from_zones as f32
        } else {
            0.0
        };

        // mid_to_fin_rate: ratio of passes from FIN or BOX (attacking third passes)
        let mid_to_fin_rate = if total_passes_from_zones > 0 {
            (passes_from_fin + passes_from_box) as f32 / total_passes_from_zones as f32
        } else {
            0.0
        };

        // fin_to_box_rate: ratio of passes from BOX (final third passes)
        let fin_to_box_rate = if total_passes_from_zones > 0 {
            passes_from_box as f32 / total_passes_from_zones as f32
        } else {
            0.0
        };

        // Direct to box (long balls from DEF) - estimate based on long passes
        let direct_to_box_rate = if passes_from_def > 0 && self.pass_attempts > 0 {
            // Rough estimate: long passes that end in box
            (self.long_passes as f32 * 0.3 / self.pass_attempts as f32).min(0.2)
        } else {
            0.0
        };

        ZoneProgression {
            def_to_mid_rate,
            mid_to_fin_rate,
            fin_to_box_rate,
            direct_to_box_rate,
        }
    }

    /// 20-zone touch distribution (normalized)
    pub fn touch_distribution_20zone(&self) -> [f32; 20] {
        let total: u32 = self.touches_by_posplay_zone.iter().sum();
        if total == 0 {
            return [0.0; 20];
        }
        let mut dist = [0.0; 20];
        for (i, &count) in self.touches_by_posplay_zone.iter().enumerate() {
            dist[i] = count as f32 / total as f32;
        }
        dist
    }

    // =========================================================================
    // FIX_2601/Phase6: from_match_result() - Convert MatchResult to snapshot
    // =========================================================================

    /// Create a MatchStatSnapshot from a MatchResult.
    ///
    /// This converts simulation output to calibration-ready format.
    /// Player stats and zone data are extracted from events where available.
    ///
    /// # Arguments
    /// * `result` - The match result from simulation
    /// * `is_home` - True for home team, false for away team
    pub fn from_match_result(result: &MatchResult, is_home: bool) -> Self {
        let stats = &result.statistics;
        let mut snapshot = Self::default();

        // Set team identifier
        snapshot.team_id = if is_home { 0 } else { 1 };

        // =====================================================================
        // Basic statistics from Statistics struct
        // =====================================================================
        if is_home {
            snapshot.shot_attempts = stats.shots_home as u32;
            snapshot.shots_on_target = stats.shots_on_target_home as u32;
            snapshot.goals = result.score_home as u32;
            snapshot.xg_total = stats.xg_home;
            snapshot.pass_attempts = stats.pass_attempts_home as u32;
            snapshot.pass_successes = stats.passes_home as u32;
            snapshot.tackles = stats.tackles_home as u32;
            snapshot.crosses = stats.crosses_home as u32;
        } else {
            snapshot.shot_attempts = stats.shots_away as u32;
            snapshot.shots_on_target = stats.shots_on_target_away as u32;
            snapshot.goals = result.score_away as u32;
            snapshot.xg_total = stats.xg_away;
            snapshot.pass_attempts = stats.pass_attempts_away as u32;
            snapshot.pass_successes = stats.passes_away as u32;
            snapshot.tackles = stats.tackles_away as u32;
            snapshot.crosses = stats.crosses_away as u32;
        }

        // =====================================================================
        // Extract player stats and zone data from events
        // =====================================================================
        Self::extract_from_events(&mut snapshot, &result.events, is_home);

        // =====================================================================
        // Synthesize zone distribution if not enough event data
        // =====================================================================
        if snapshot.touches_by_posplay_zone.iter().sum::<u32>() == 0 {
            snapshot.synthesize_zone_distribution();
        }

        // =====================================================================
        // Ensure minimum player touches for Gini calculation
        // =====================================================================
        if snapshot.player_stats.total_touches() == 0 {
            snapshot.synthesize_player_touches();
        }

        snapshot
    }

    /// Extract player stats and zone data from match events.
    fn extract_from_events(snapshot: &mut Self, events: &[MatchEvent], is_home: bool) {
        for event in events {
            // Only process events for this team
            if event.is_home_team != is_home {
                continue;
            }

            // Track player actions based on event type
            match event.event_type {
                EventType::Pass => {
                    // Record pass with player IDs
                    if let Some(passer_id) = event.player_track_id {
                        let passer_idx = Self::track_id_to_player_idx(passer_id, is_home);
                        let receiver_idx = event.target_track_id
                            .map(|t| Self::track_id_to_player_idx(t, is_home))
                            .unwrap_or(passer_idx); // fallback to passer if no receiver

                        // Determine if progressive (simplified: forward half of field)
                        let is_progressive = event.details.as_ref()
                            .and_then(|d| d.ball_position)
                            .map(|(x, _, _)| x > 0.5)
                            .unwrap_or(false);

                        snapshot.player_stats.record_pass(passer_idx, receiver_idx, is_progressive, false);
                        snapshot.player_stats.record_touch(passer_idx);

                        // Record zone from ball position (for touches and pass origin)
                        if let Some(pos) = event.details.as_ref().and_then(|d| d.ball_position) {
                            let zone20 = Self::position_to_zone20(pos.0, pos.1, is_home);
                            snapshot.touches_by_posplay_zone[zone20] += 1;
                            // Phase 6.1: Also record passes from this zone
                            snapshot.passes_from_posplay_zone[zone20] += 1;
                        }
                    }
                }
                EventType::Shot | EventType::ShotOnTarget | EventType::ShotOffTarget => {
                    if let Some(shooter_id) = event.player_track_id {
                        let idx = Self::track_id_to_player_idx(shooter_id, is_home);
                        snapshot.player_stats.record_touch(idx);

                        // Record zone from ball position
                        if let Some(pos) = event.details.as_ref().and_then(|d| d.ball_position) {
                            let zone20 = Self::position_to_zone20(pos.0, pos.1, is_home);
                            snapshot.touches_by_posplay_zone[zone20] += 1;

                            // Track halfspace shots
                            if Self::is_halfspace_zone(zone20) {
                                snapshot.shots_from_halfspace += 1;
                            }
                        }
                    }
                }
                EventType::Dribble => {
                    if let Some(player_id) = event.player_track_id {
                        let idx = Self::track_id_to_player_idx(player_id, is_home);
                        snapshot.player_stats.record_touch(idx);

                        if let Some(pos) = event.details.as_ref().and_then(|d| d.ball_position) {
                            let zone20 = Self::position_to_zone20(pos.0, pos.1, is_home);
                            snapshot.touches_by_posplay_zone[zone20] += 1;
                        }
                    }
                }
                EventType::Tackle => {
                    if let Some(player_id) = event.player_track_id {
                        let idx = Self::track_id_to_player_idx(player_id, is_home);
                        snapshot.player_stats.record_touch(idx);
                        snapshot.tackle_successes += 1;
                    }
                }
                EventType::Corner | EventType::Freekick | EventType::ThrowIn => {
                    if let Some(player_id) = event.player_track_id {
                        let idx = Self::track_id_to_player_idx(player_id, is_home);
                        snapshot.player_stats.record_touch(idx);
                    }
                }
                _ => {}
            }
        }
    }

    /// Convert track_id (0-21) to player index (0-10).
    fn track_id_to_player_idx(track_id: u8, is_home: bool) -> usize {
        if is_home {
            (track_id as usize).min(10)
        } else {
            ((track_id as usize).saturating_sub(11)).min(10)
        }
    }

    /// Convert Coord10 position to 20-zone index.
    /// Zones are 5 lanes x 4 quarters (DEF, MID, FIN, BOX).
    ///
    /// FIX_2601/0113: ball_position은 이제 Coord10 단위 (0-1050, 0-680)
    /// Phase 6.1: Added is_home parameter to flip x-axis for away team
    /// FIX_2601/0113-B: y축도 뒤집어서 Away팀의 좌/우를 올바르게 계산
    fn position_to_zone20(x: f32, y: f32, is_home: bool) -> usize {
        // FIX_2601/0113: Coord10 → normalized 변환
        let x_norm = x / 1050.0;
        let y_norm = y / 680.0;

        // Flip both axes for away team (they attack toward x=0, and view field from opposite side)
        let x_adj = if is_home { x_norm } else { 1.0 - x_norm };
        let y_adj = if is_home { y_norm } else { 1.0 - y_norm };

        // Quarter (0=DEF, 1=MID, 2=FIN, 3=BOX)
        let quarter = ((x_adj * 4.0).floor() as usize).min(3);
        // Lane (0=LW, 1=LHS, 2=C, 3=RHS, 4=RW)
        let lane = ((y_adj * 5.0).floor() as usize).min(4);
        quarter * 5 + lane
    }

    /// Check if a zone20 index is in a halfspace.
    fn is_halfspace_zone(zone20: usize) -> bool {
        // Halfspace lanes are LHS (1) and RHS (3)
        let lane = zone20 % 5;
        lane == 1 || lane == 3
    }

    /// Synthesize reasonable zone distribution when event data is sparse.
    fn synthesize_zone_distribution(&mut self) {
        // Use typical football touch distribution:
        // - More touches in midfield
        // - Some concentration in center lane
        // - Reasonable spread across zones
        let pass_count = self.pass_attempts.max(100);

        // Base touches per zone
        for quarter in 0..4 {
            for lane in 0..5 {
                let zone = quarter * 5 + lane;

                // Weight by quarter (midfield heaviest)
                let quarter_weight = match quarter {
                    0 => 0.20, // DEF
                    1 => 0.35, // MID
                    2 => 0.30, // FIN
                    3 => 0.15, // BOX
                    _ => 0.0,
                };

                // Weight by lane (center heaviest)
                let lane_weight = match lane {
                    0 | 4 => 0.15, // Wings
                    1 | 3 => 0.22, // Halfspaces
                    2 => 0.26,     // Center
                    _ => 0.0,
                };

                let touches = ((pass_count as f32) * quarter_weight * lane_weight * 5.0) as u32;
                self.touches_by_posplay_zone[zone] = touches.max(1);
            }
        }
    }

    /// Synthesize player touches when event data is sparse.
    fn synthesize_player_touches(&mut self) {
        let total_touches = self.pass_attempts.max(100);

        // Distribute touches with realistic position weighting
        // Midfielders get more, GK gets less
        let weights: [f32; 11] = [
            0.03,  // GK
            0.08, 0.08, 0.08, 0.08,  // Defenders
            0.12, 0.14, 0.12,         // Midfielders
            0.10, 0.10, 0.07,         // Attackers
        ];

        for (idx, &weight) in weights.iter().enumerate() {
            let touches = ((total_touches as f32) * weight) as u32;
            for _ in 0..touches {
                self.player_stats.record_touch(idx);
            }
        }

        // Add some passes between players
        let pass_count = (total_touches / 2) as usize;
        for i in 0..pass_count {
            let passer = (i % 10) + 1;  // Outfield players (1-10)
            let receiver = ((i + 3) % 10) + 1;  // Different player
            self.player_stats.record_pass(passer, receiver, i % 5 == 0, i % 20 == 0);
        }
    }
}

/// Combined snapshot for both teams
#[derive(Debug, Clone)]
pub struct MatchSnapshot {
    pub match_id: u64,
    pub home: MatchStatSnapshot,
    pub away: MatchStatSnapshot,
}

impl MatchSnapshot {
    pub fn new(match_id: u64) -> Self {
        Self {
            match_id,
            home: MatchStatSnapshot::new(0, match_id),
            away: MatchStatSnapshot::new(1, match_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pass_success_rate() {
        let mut snapshot = MatchStatSnapshot::default();
        snapshot.pass_attempts = 100;
        snapshot.pass_successes = 83;

        assert!((snapshot.pass_success_rate() - 0.83).abs() < 0.001);
    }

    #[test]
    fn test_progressive_share() {
        let mut snapshot = MatchStatSnapshot::default();
        snapshot.pass_attempts = 100;
        snapshot.progressive_passes = 18;

        assert!((snapshot.progressive_share() - 0.18).abs() < 0.001);
    }

    #[test]
    fn test_touch_distribution() {
        let mut snapshot = MatchStatSnapshot::default();
        snapshot.touches_by_zone = [10, 20, 10, 15, 30, 15]; // sum = 100

        let dist = snapshot.touch_distribution();
        assert!((dist[0] - 0.10).abs() < 0.001);
        assert!((dist[4] - 0.30).abs() < 0.001);
    }

    // =========================================================================
    // FIX_2601/0113: Tactical Metrics Tests
    // =========================================================================

    #[test]
    fn test_halfspace_metrics_calculation() {
        let mut snapshot = MatchStatSnapshot::default();

        // Set up 20-zone touches: HS indices are 1,3,6,8,11,13,16,18
        // Each HS zone gets 5 touches (8 zones * 5 = 40 total HS)
        // Each non-HS zone gets 5 touches (12 zones * 5 = 60 total non-HS)
        // Total = 100 touches, 40 in HS -> 40% touch rate
        snapshot.touches_by_posplay_zone = [
            5, 5, 5, 5, 5,   // DEF: all 5 each (10 in HS: idx 1,3)
            5, 5, 5, 5, 5,   // MID: all 5 each (10 in HS: idx 6,8)
            5, 5, 5, 5, 5,   // FIN: all 5 each (10 in HS: idx 11,13)
            5, 5, 5, 5, 5,   // BOX: all 5 each (10 in HS: idx 16,18)
        ];
        // Total: 100 touches, 40 in HS -> 40% touch rate

        let hs = snapshot.halfspace_metrics();
        assert!((hs.touch_rate - 0.40).abs() < 0.01,
            "Expected HS touch_rate ~0.40, got {}", hs.touch_rate);
    }

    #[test]
    fn test_halfspace_entry_pass_rate() {
        let mut snapshot = MatchStatSnapshot::default();
        snapshot.pass_attempts = 100;
        snapshot.halfspace_entry_passes = 15;
        snapshot.shot_attempts = 10;
        snapshot.shots_from_halfspace = 3;

        let hs = snapshot.halfspace_metrics();
        assert!((hs.entry_pass_rate - 0.15).abs() < 0.001);
        assert!((hs.shot_from_hs_rate - 0.30).abs() < 0.001);
    }

    #[test]
    fn test_lane_occupancy_uniform() {
        let mut snapshot = MatchStatSnapshot::default();

        // Uniform distribution across lanes (each lane gets 20 touches)
        snapshot.touches_by_posplay_zone = [
            5, 5, 5, 5, 5,   // DEF
            5, 5, 5, 5, 5,   // MID
            5, 5, 5, 5, 5,   // FIN
            5, 5, 5, 5, 5,   // BOX
        ];

        let lane = snapshot.lane_occupancy();

        // All lanes should have 0.2 share
        for (i, share) in lane.touch_by_lane.iter().enumerate() {
            assert!((share - 0.2).abs() < 0.001, "Lane {} expected 0.2, got {}", i, share);
        }

        // Entropy should be close to 1.0 (uniform)
        assert!(lane.entropy > 0.95, "Expected entropy > 0.95, got {}", lane.entropy);

        // Balance should be close to 1.0 (symmetric)
        assert!(lane.balance > 0.95, "Expected balance > 0.95, got {}", lane.balance);
    }

    #[test]
    fn test_lane_occupancy_skewed() {
        let mut snapshot = MatchStatSnapshot::default();

        // All touches on left side (LW + LHS)
        snapshot.touches_by_posplay_zone = [
            25, 25, 0, 0, 0,   // DEF
            25, 25, 0, 0, 0,   // MID
            0, 0, 0, 0, 0,     // FIN
            0, 0, 0, 0, 0,     // BOX
        ];

        let lane = snapshot.lane_occupancy();

        // Left lanes should dominate
        assert!(lane.touch_by_lane[0] > 0.4); // LW
        assert!(lane.touch_by_lane[1] > 0.4); // LHS

        // Entropy should be low (concentrated)
        assert!(lane.entropy < 0.8, "Expected entropy < 0.8 for skewed, got {}", lane.entropy);

        // Balance should be low (one-sided)
        assert!(lane.balance < 0.5, "Expected balance < 0.5 for skewed, got {}", lane.balance);
    }

    #[test]
    fn test_zone_progression() {
        let mut snapshot = MatchStatSnapshot::default();
        snapshot.pass_attempts = 100;

        // Phase 6.1: zone_progression uses passes_from_posplay_zone (origin-only)
        // because pass destination zone data isn't available from events.
        //
        // DEF passes
        snapshot.passes_from_posplay_zone[0] = 10; // LW_DEF
        snapshot.passes_from_posplay_zone[2] = 10; // C_DEF

        // MID passes
        snapshot.passes_from_posplay_zone[6] = 10; // LHS_MID

        // FIN passes
        snapshot.passes_from_posplay_zone[11] = 10; // LHS_FIN

        let prog = snapshot.zone_progression();

        // Should have progression rates > 0
        assert!(prog.def_to_mid_rate > 0.0, "Expected def_to_mid > 0");
        assert!(prog.mid_to_fin_rate > 0.0, "Expected mid_to_fin > 0");
    }

    #[test]
    fn test_touch_distribution_20zone() {
        let mut snapshot = MatchStatSnapshot::default();

        // Set up some touches
        snapshot.touches_by_posplay_zone[0] = 50;  // Half in LW_DEF
        snapshot.touches_by_posplay_zone[7] = 50;  // Half in C_MID

        let dist = snapshot.touch_distribution_20zone();

        assert!((dist[0] - 0.5).abs() < 0.001, "Expected 0.5 in zone 0");
        assert!((dist[7] - 0.5).abs() < 0.001, "Expected 0.5 in zone 7");
        assert!((dist[1]).abs() < 0.001, "Expected 0.0 in zone 1");
    }

    // =========================================================================
    // FIX_2601/NEW_FUNC: PlayerPassStats and Gini Tests
    // =========================================================================

    #[test]
    fn test_player_pass_stats_record() {
        let mut stats = PlayerPassStats::default();

        // Player 5 passes to player 9
        stats.record_pass(5, 9, true, false); // progressive, not key
        stats.record_pass(5, 9, false, true); // not progressive, key
        stats.record_pass(7, 5, false, false); // neither

        assert_eq!(stats.passes_sent[5], 2);
        assert_eq!(stats.passes_sent[7], 1);
        assert_eq!(stats.passes_received[9], 2);
        assert_eq!(stats.passes_received[5], 1);
        assert_eq!(stats.progressive_sent[5], 1);
        assert_eq!(stats.key_passes[5], 1);
    }

    #[test]
    fn test_player_pass_stats_touches() {
        let mut stats = PlayerPassStats::default();

        stats.record_touch(0);
        stats.record_touch(0);
        stats.record_touch(5);
        stats.record_touch(10);

        assert_eq!(stats.touches[0], 2);
        assert_eq!(stats.touches[5], 1);
        assert_eq!(stats.touches[10], 1);
        assert_eq!(stats.total_touches(), 4);
    }

    #[test]
    fn test_player_pass_stats_most_active() {
        let mut stats = PlayerPassStats::default();

        // Player 5 has most touches
        for _ in 0..20 {
            stats.record_touch(5);
        }
        for _ in 0..10 {
            stats.record_touch(7);
        }

        let (most_touch_idx, most_touch_count) = stats.most_touches_player();
        assert_eq!(most_touch_idx, 5);
        assert_eq!(most_touch_count, 20);
    }

    #[test]
    fn test_player_pass_stats_gini_uniform() {
        let mut stats = PlayerPassStats::default();

        // Uniform distribution - each player gets 10 touches
        for player in 0..11 {
            for _ in 0..10 {
                stats.record_touch(player);
            }
        }

        let gini = stats.gini_metrics();
        assert!(gini.touch_gini < 0.05, "Uniform touches should have low Gini: {}", gini.touch_gini);
    }

    #[test]
    fn test_player_pass_stats_gini_monopoly() {
        let mut stats = PlayerPassStats::default();

        // Monopoly - one player has all touches
        for _ in 0..100 {
            stats.record_touch(5);
        }

        let gini = stats.gini_metrics();
        assert!(gini.touch_gini > 0.85, "Monopoly should have high Gini: {}", gini.touch_gini);
    }

    #[test]
    fn test_snapshot_gini_metrics() {
        let mut snapshot = MatchStatSnapshot::default();

        // Record some passes with player indices
        for _ in 0..30 {
            snapshot.player_stats.record_pass(5, 9, true, false);
        }
        for _ in 0..20 {
            snapshot.player_stats.record_pass(7, 5, false, false);
        }
        for _ in 0..10 {
            snapshot.player_stats.record_pass(3, 7, false, false);
        }

        let gini = snapshot.gini_metrics();

        // Should have moderate concentration
        assert!(gini.pass_sent_gini > 0.2, "Expected some concentration in pass sending");
        assert!(gini.pass_recv_gini > 0.2, "Expected some concentration in pass receiving");
    }

    // =========================================================================
    // FIX_2601/Phase6: from_match_result() Tests
    // =========================================================================

    #[test]
    fn test_from_match_result_basic_stats() {
        use crate::models::{MatchResult, Statistics};

        let mut result = MatchResult::new();
        result.score_home = 2;
        result.score_away = 1;
        result.statistics = Statistics {
            shots_home: 12,
            shots_away: 8,
            shots_on_target_home: 5,
            shots_on_target_away: 3,
            xg_home: 1.5,
            xg_away: 0.8,
            pass_attempts_home: 450,
            pass_attempts_away: 380,
            passes_home: 400,
            passes_away: 340,
            tackles_home: 15,
            tackles_away: 18,
            ..Default::default()
        };

        let home_snap = MatchStatSnapshot::from_match_result(&result, true);
        let away_snap = MatchStatSnapshot::from_match_result(&result, false);

        // Home team stats
        assert_eq!(home_snap.shot_attempts, 12);
        assert_eq!(home_snap.shots_on_target, 5);
        assert_eq!(home_snap.goals, 2);
        assert!((home_snap.xg_total - 1.5).abs() < 0.01);
        assert_eq!(home_snap.pass_attempts, 450);
        assert_eq!(home_snap.pass_successes, 400);
        assert_eq!(home_snap.tackles, 15);

        // Away team stats
        assert_eq!(away_snap.shot_attempts, 8);
        assert_eq!(away_snap.shots_on_target, 3);
        assert_eq!(away_snap.goals, 1);
        assert!((away_snap.xg_total - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_from_match_result_synthesizes_zone_data() {
        use crate::models::{MatchResult, Statistics};

        let mut result = MatchResult::new();
        result.statistics.pass_attempts_home = 400;

        let snap = MatchStatSnapshot::from_match_result(&result, true);

        // Should have synthesized zone data
        let total_touches: u32 = snap.touches_by_posplay_zone.iter().sum();
        assert!(total_touches > 0, "Should have synthesized zone touches");

        // Check lane distribution
        let lane = snap.lane_occupancy();
        assert!(lane.entropy > 0.5, "Should have reasonable lane entropy");
    }

    #[test]
    fn test_from_match_result_synthesizes_player_touches() {
        use crate::models::{MatchResult, Statistics};

        let mut result = MatchResult::new();
        result.statistics.pass_attempts_home = 400;

        let snap = MatchStatSnapshot::from_match_result(&result, true);

        // Should have synthesized player touches
        assert!(snap.player_stats.total_touches() > 0, "Should have synthesized touches");

        // Check Gini is reasonable
        let gini = snap.gini_metrics();
        assert!(gini.touch_gini < 0.5, "Synthesized touches should not be monopolistic: {}", gini.touch_gini);
    }

    #[test]
    fn test_position_to_zone20() {
        // Test zone mapping
        // FIX_2601/0113: ball_position은 이제 Coord10 단위 (0-1050, 0-680)

        // Home team orientation
        // Center-ish (x=525, y=340 in Coord10) should be quarter=2, lane=C
        let zone = MatchStatSnapshot::position_to_zone20(525.0, 340.0, true);
        assert_eq!(zone / 5, 2); // FIN quarter (0.5 * 4 = 2)
        assert_eq!(zone % 5, 2); // C lane (0.5 * 5 = 2)

        // Defensive left wing (x=105, y=68 in Coord10 = 0.1, 0.1 normalized)
        let zone = MatchStatSnapshot::position_to_zone20(105.0, 68.0, true);
        assert_eq!(zone / 5, 0); // DEF quarter
        assert_eq!(zone % 5, 0); // LW lane

        // Away team orientation flips both axes (x_adj = 1 - x, y_adj = 1 - y)
        // FIX_2601/0113-B: y축도 뒤집어서 Away팀의 좌/우를 올바르게 계산
        let zone = MatchStatSnapshot::position_to_zone20(105.0, 68.0, false);
        assert_eq!(zone / 5, 3); // BOX quarter (x_adj=0.9 -> 3)
        assert_eq!(zone % 5, 4); // RW lane (y_adj=0.9 -> 4)
    }

    #[test]
    fn test_is_halfspace_zone() {
        // LHS and RHS lanes are halfspaces
        assert!(MatchStatSnapshot::is_halfspace_zone(1));  // DEF_LHS
        assert!(MatchStatSnapshot::is_halfspace_zone(3));  // DEF_RHS
        assert!(MatchStatSnapshot::is_halfspace_zone(6));  // MID_LHS
        assert!(MatchStatSnapshot::is_halfspace_zone(18)); // BOX_RHS

        // Other lanes are not halfspaces
        assert!(!MatchStatSnapshot::is_halfspace_zone(0));  // DEF_LW
        assert!(!MatchStatSnapshot::is_halfspace_zone(2));  // DEF_C
        assert!(!MatchStatSnapshot::is_halfspace_zone(4));  // DEF_RW
    }
}
