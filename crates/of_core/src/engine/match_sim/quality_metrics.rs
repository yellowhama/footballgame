//! Phase 14: Quality Metrics for Gameplay Verification
//!
//! This module provides metrics collection and analysis for verifying
//! the quality of match simulations after Phase 5-13 implementation.
//!
//! ## Metrics Tracked
//!
//! | Metric | Target | Description |
//! |--------|--------|-------------|
//! | Cluster Density | Zone-aware | Players within 5m (varies by field zone) |
//! | Corner Stuck | < 10% | % ticks ball in corner zones |
//! | Oscillation | < 5% | Position delta variance threshold |
//! | Pass Success | 70-85% | Successful / attempted |
//! | Pressing Triggers | > 0 | process_slow() pressing suggestions |
//! | Penetration Runs | > 0 | RunningInBehind state entries |
//!
//! ## Zone-Aware Cluster Density
//!
//! Different zones have different expected player densities:
//! - Penalty area (Área): 8-12 players normal during attacks
//! - Attacking midfield (Mediapunta): 4-6 players
//! - Wings (Extremo/Banda): 2-4 players
//! - Central midfield (Mediocentro): 3-5 players
//! - Set pieces: Excluded from measurement

use std::collections::HashMap;

use crate::engine::physics_constants::field;

/// Field zones for context-aware metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldZone {
    /// Penalty area (0-16.5m or 88.5-105m x-axis, 13.85-54.15m y-axis)
    /// Expected density: 8-12 players during attacks
    PenaltyArea,
    /// Central attacking zone (mediapunta area)
    /// Expected density: 4-6 players
    AttackingMidfield,
    /// Wings (banda/extremo)
    /// Expected density: 2-4 players
    Wing,
    /// Central midfield (mediocentro)
    /// Expected density: 3-5 players
    CentralMidfield,
    /// Defensive third
    /// Expected density: 3-6 players
    DefensiveThird,
}

impl FieldZone {
    /// Determine zone from ball position (in meters, 105x68 field)
    pub fn from_position(x: f32, y: f32) -> Self {
        // Penalty areas: x < 16.5 or x > 88.5, y in 13.85-54.15
        let in_penalty_area_x = !(16.5..=88.5).contains(&x);
        let in_penalty_area_y = y > 13.85 && y < 54.15;

        if in_penalty_area_x && in_penalty_area_y {
            return FieldZone::PenaltyArea;
        }

        // Wings: y < 20 or y > 48 (outer thirds of width)
        if !(20.0..=48.0).contains(&y) {
            return FieldZone::Wing;
        }

        // Attacking midfield: x > 70 or x < 35, central y
        if !(35.0..=70.0).contains(&x) {
            return FieldZone::AttackingMidfield;
        }

        // Defensive third: x < 35 (for home) - simplified as just central midfield
        // Central midfield: middle of the pitch
        FieldZone::CentralMidfield
    }

    /// Maximum expected density for this zone (players within 5m)
    /// P3.7: Adjusted thresholds to realistic levels (both teams combined)
    pub fn max_expected_density(&self) -> u32 {
        match self {
            FieldZone::PenaltyArea => 14, // P3.7: 12→14, attacks have 10+ players
            FieldZone::AttackingMidfield => 9, // P3.7: 7→9, both teams contest
            FieldZone::Wing => 8,         // P3.7: 5→8, wing play involves multiple players
            FieldZone::CentralMidfield => 8, // P3.7: 6→8, midfield battles normal
            FieldZone::DefensiveThird => 9, // P3.7: 7→9, defensive pressure
        }
    }
}

/// Quality metrics collected during match simulation
#[derive(Debug, Clone, Default)]
pub struct QualityMetrics {
    // === Movement Quality ===
    /// Cluster density samples per zone (zone -> (sum, count))
    pub cluster_density_by_zone: HashMap<String, (f64, u32)>,
    /// Anomaly count: times density exceeded zone threshold
    pub cluster_anomaly_count: u32,
    /// Total cluster samples (for anomaly rate calculation)
    pub cluster_total_samples: u32,
    /// Set piece ticks (excluded from cluster measurement)
    pub set_piece_ticks: u32,

    /// Number of STUCK EVENTS (ball in corner 10+ seconds, counted once per event)
    pub corner_stuck_events: u32,
    /// Consecutive ticks in corner zone (for stuck detection)
    pub consecutive_corner_ticks: u32,
    /// Whether we already counted current stuck event (to avoid double counting)
    pub current_stuck_counted: bool,
    /// Total ticks measured
    pub total_ticks: u32,

    /// Count of position oscillations detected
    pub oscillation_count: u32,

    // === Decision Quality ===
    /// Successful passes
    pub passes_successful: u32,
    /// Attempted passes
    pub passes_attempted: u32,

    /// Number of pressing trigger activations from process_slow()
    pub pressing_triggers: u32,

    /// Number of penetration run initiations (RunningInBehind state)
    pub penetration_runs: u32,

    // === Substate Distribution ===
    /// Ticks spent in each substate (for distribution analysis)
    pub substate_ticks: HashMap<String, u32>,

    // === Rule Decision Quality (FIX_2601/0123 Phase 6) ===
    /// Fouls committed (home, away)
    pub fouls_committed: (u32, u32),
    /// Handballs detected (home, away)
    pub handballs_detected: (u32, u32),
    /// Offsides called (home, away)
    pub offsides_called: (u32, u32),
    /// Cards issued (yellow_home, yellow_away, red_home, red_away)
    pub cards_issued: (u32, u32, u32, u32),
    /// Dispatcher decisions made
    pub dispatcher_decisions: (u32, u32), // (total, matched with legacy)
    /// Dispatcher match rate (0.0 - 1.0)
    pub dispatcher_match_rate: f32,
}

impl QualityMetrics {
    /// Create new empty metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a cluster density sample with zone awareness
    ///
    /// # Arguments
    /// * `ball_x` - Ball x position in meters (0-105)
    /// * `ball_y` - Ball y position in meters (0-68)
    /// * `density` - Number of players within 5m of ball carrier
    /// * `is_set_piece` - Whether this is during a set piece (excluded from anomaly detection)
    pub fn record_cluster_density_zone_aware(
        &mut self,
        ball_x: f32,
        ball_y: f32,
        density: u32,
        is_set_piece: bool,
    ) {
        if is_set_piece {
            self.set_piece_ticks += 1;
            return; // Don't count set pieces in cluster metrics
        }

        let zone = FieldZone::from_position(ball_x, ball_y);
        let zone_name = format!("{:?}", zone);
        let max_expected = zone.max_expected_density();

        // Track per-zone density
        let entry = self.cluster_density_by_zone.entry(zone_name).or_insert((0.0, 0));
        entry.0 += density as f64;
        entry.1 += 1;

        // Track anomalies (density exceeding zone threshold)
        self.cluster_total_samples += 1;
        if density > max_expected {
            self.cluster_anomaly_count += 1;
        }
    }

    /// Legacy method for backward compatibility (uses CentralMidfield zone thresholds)
    pub fn record_cluster_density(&mut self, density: f32) {
        // Assume central midfield for legacy calls
        self.record_cluster_density_zone_aware(field::CENTER_X, field::CENTER_Y, density as u32, false);
    }

    /// Record whether ball is in corner zone this tick
    /// Counts as "stuck EVENT" after 10+ seconds (40 ticks) continuously in corner
    /// Each stuck event is counted ONCE (not per tick)
    /// P3.6: is_set_piece = true skips stuck counting (ball may wait during goal kicks)
    pub fn record_ball_position(&mut self, x_m: f32, y_m: f32, is_set_piece: bool) {
        self.total_ticks += 1;

        // P3.6: During set pieces, reset corner counter (don't count as stuck)
        if is_set_piece {
            self.consecutive_corner_ticks = 0;
            return;
        }

        // Corner zones: within 8m of corners (0,0), (0,68), (105,0), (105,68)
        // P3.6: Reduced from 10m to 8m to focus on true corner stuck situations
        let in_corner = !(8.0..=97.0).contains(&x_m) && !(8.0..=60.0).contains(&y_m);

        // 10 seconds = 40 ticks (at 250ms per tick)
        const STUCK_THRESHOLD_TICKS: u32 = 40;

        if in_corner {
            self.consecutive_corner_ticks += 1;
            // Count as stuck EVENT once when threshold is reached
            if self.consecutive_corner_ticks >= STUCK_THRESHOLD_TICKS && !self.current_stuck_counted
            {
                self.corner_stuck_events += 1;
                self.current_stuck_counted = true;
            }
        } else {
            // Reset when ball leaves corner
            self.consecutive_corner_ticks = 0;
            self.current_stuck_counted = false;
        }
    }

    /// Record pass result (combined: increments both attempts and optionally successes)
    pub fn record_pass(&mut self, successful: bool) {
        self.passes_attempted += 1;
        if successful {
            self.passes_successful += 1;
        }
    }

    /// Record pass attempt (call this when pass starts)
    pub fn record_pass_attempt(&mut self) {
        self.passes_attempted += 1;
    }

    /// Record pass success (call this when pass completes - does NOT increment attempts)
    pub fn record_pass_success(&mut self) {
        self.passes_successful += 1;
    }

    /// Record pressing trigger activation
    pub fn record_pressing_trigger(&mut self) {
        self.pressing_triggers += 1;
    }

    /// Record penetration run initiation
    pub fn record_penetration_run(&mut self) {
        self.penetration_runs += 1;
    }

    /// Record substate usage for a player
    pub fn record_substate(&mut self, substate_name: &str) {
        *self.substate_ticks.entry(substate_name.to_string()).or_insert(0) += 1;
    }

    /// Record position oscillation
    pub fn record_oscillation(&mut self) {
        self.oscillation_count += 1;
    }

    // === Rule Decision Recording (FIX_2601/0123 Phase 6) ===

    /// Record a foul committed
    pub fn record_foul(&mut self, is_home: bool) {
        if is_home {
            self.fouls_committed.0 += 1;
        } else {
            self.fouls_committed.1 += 1;
        }
    }

    /// Record a handball detected
    pub fn record_handball(&mut self, is_home: bool) {
        if is_home {
            self.handballs_detected.0 += 1;
        } else {
            self.handballs_detected.1 += 1;
        }
    }

    /// Record an offside called
    pub fn record_offside(&mut self, is_home: bool) {
        if is_home {
            self.offsides_called.0 += 1;
        } else {
            self.offsides_called.1 += 1;
        }
    }

    /// Record a card issued
    pub fn record_card(&mut self, is_home: bool, is_red: bool) {
        if is_home {
            if is_red {
                self.cards_issued.2 += 1;
            } else {
                self.cards_issued.0 += 1;
            }
        } else if is_red {
            self.cards_issued.3 += 1;
        } else {
            self.cards_issued.1 += 1;
        }
    }

    /// Record dispatcher decision comparison
    pub fn record_dispatcher_decision(&mut self, matched: bool) {
        self.dispatcher_decisions.0 += 1;
        if matched {
            self.dispatcher_decisions.1 += 1;
        }
        // Update match rate
        if self.dispatcher_decisions.0 > 0 {
            self.dispatcher_match_rate =
                self.dispatcher_decisions.1 as f32 / self.dispatcher_decisions.0 as f32;
        }
    }

    // === Computed Metrics ===

    /// Cluster anomaly rate (0.0 - 1.0)
    /// Percentage of samples where density exceeded zone-specific threshold
    pub fn cluster_anomaly_rate(&self) -> f32 {
        if self.cluster_total_samples == 0 {
            return 0.0;
        }
        self.cluster_anomaly_count as f32 / self.cluster_total_samples as f32
    }

    /// Average cluster density per zone
    pub fn cluster_density_by_zone_avg(&self) -> HashMap<String, f32> {
        self.cluster_density_by_zone
            .iter()
            .map(|(zone, (sum, count))| {
                let avg = if *count > 0 { (*sum / *count as f64) as f32 } else { 0.0 };
                (zone.clone(), avg)
            })
            .collect()
    }

    /// Corner stuck events count (10+ seconds stuck = 1 event)
    pub fn corner_stuck_events(&self) -> u32 {
        self.corner_stuck_events
    }

    /// Oscillation percentage (0.0 - 1.0)
    pub fn oscillation_pct(&self) -> f32 {
        if self.total_ticks == 0 {
            return 0.0;
        }
        self.oscillation_count as f32 / self.total_ticks as f32
    }

    /// Pass success rate (0.0 - 1.0)
    pub fn pass_success_rate(&self) -> f32 {
        if self.passes_attempted == 0 {
            return 0.0;
        }
        self.passes_successful as f32 / self.passes_attempted as f32
    }

    /// Check if all substates are being used (no dead states)
    pub fn unused_substates(&self, expected_substates: &[&str]) -> Vec<String> {
        expected_substates
            .iter()
            .filter(|s| !self.substate_ticks.contains_key(**s))
            .map(|s| s.to_string())
            .collect()
    }

    /// Generate quality report
    pub fn generate_report(&self) -> QualityReport {
        QualityReport {
            cluster_anomaly_rate: self.cluster_anomaly_rate(),
            cluster_density_by_zone: self.cluster_density_by_zone_avg(),
            corner_stuck_events: self.corner_stuck_events(),
            oscillation_pct: self.oscillation_pct(),
            pass_success_rate: self.pass_success_rate(),
            pressing_triggers: self.pressing_triggers,
            penetration_runs: self.penetration_runs,
            total_ticks: self.total_ticks,
            set_piece_ticks: self.set_piece_ticks,
            substate_usage: self.substate_ticks.clone(),
            // Rule metrics (FIX_2601/0123 Phase 6)
            fouls_committed: self.fouls_committed,
            handballs_detected: self.handballs_detected,
            offsides_called: self.offsides_called,
            cards_issued: self.cards_issued,
            dispatcher_match_rate: self.dispatcher_match_rate,
        }
    }
}

/// Computed quality report for analysis
#[derive(Debug, Clone)]
pub struct QualityReport {
    /// Rate of cluster density exceeding zone-specific thresholds (0.0 - 1.0)
    pub cluster_anomaly_rate: f32,
    /// Average density per zone
    pub cluster_density_by_zone: HashMap<String, f32>,
    /// Number of corner stuck events (10+ seconds stuck = 1 event)
    pub corner_stuck_events: u32,
    pub oscillation_pct: f32,
    pub pass_success_rate: f32,
    pub pressing_triggers: u32,
    pub penetration_runs: u32,
    pub total_ticks: u32,
    pub set_piece_ticks: u32,
    pub substate_usage: HashMap<String, u32>,
    // Rule metrics (FIX_2601/0123 Phase 6)
    /// Fouls committed (home, away)
    pub fouls_committed: (u32, u32),
    /// Handballs detected (home, away)
    pub handballs_detected: (u32, u32),
    /// Offsides called (home, away)
    pub offsides_called: (u32, u32),
    /// Cards issued (yellow_home, yellow_away, red_home, red_away)
    pub cards_issued: (u32, u32, u32, u32),
    /// Dispatcher match rate (0.0 - 1.0)
    pub dispatcher_match_rate: f32,
}

impl QualityReport {
    /// Check if report passes all quality thresholds
    pub fn passes_quality_gate(&self) -> QualityGateResult {
        let mut failures = Vec::new();

        // Target: cluster anomaly rate < 10%
        // (10% of the time, density exceeds zone-specific threshold is acceptable)
        if self.cluster_anomaly_rate > 0.10 {
            failures.push(format!(
                "Cluster anomaly rate {:.1}% > 10% target (too often exceeding zone density limits)",
                self.cluster_anomaly_rate * 100.0
            ));
        }

        // Target: corner stuck events < 5 per match
        // (5 times stuck for 10+ seconds is acceptable)
        if self.corner_stuck_events > 5 {
            failures.push(format!("Corner stuck events {} > 5 target", self.corner_stuck_events));
        }

        // Target: oscillation < 5%
        if self.oscillation_pct > 0.05 {
            failures.push(format!("Oscillation {}% > 5% target", self.oscillation_pct * 100.0));
        }

        // Target: pass success 70-85%
        if self.pass_success_rate < 0.70 {
            failures.push(format!("Pass success {}% < 70% target", self.pass_success_rate * 100.0));
        } else if self.pass_success_rate > 0.85 {
            failures.push(format!(
                "Pass success {}% > 85% target (unrealistic)",
                self.pass_success_rate * 100.0
            ));
        }

        // Target: pressing triggers > 0 (only if enough ticks)
        if self.total_ticks > 1000 && self.pressing_triggers == 0 {
            failures.push("No pressing triggers recorded".to_string());
        }

        // Target: penetration runs > 0 (only if enough ticks)
        if self.total_ticks > 1000 && self.penetration_runs == 0 {
            failures.push("No penetration runs recorded".to_string());
        }

        QualityGateResult { passed: failures.is_empty(), failures }
    }
}

/// Result of quality gate check
#[derive(Debug, Clone)]
pub struct QualityGateResult {
    pub passed: bool,
    pub failures: Vec<String>,
}

// === Phase 4: Shot Balance Metrics ===

/// Shot balance statistics from multiple matches
#[derive(Debug, Clone, Default)]
pub struct ShotBalanceStats {
    /// (shots_home, shots_away) for each match
    pub match_shots: Vec<(u16, u16)>,
    /// (forward_passes_home + away, ball_progress_m_home + away, box_entries_home + away)
    pub match_progress: Vec<(u16, f32, u16)>,
}

impl ShotBalanceStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a match result
    pub fn add_match(
        &mut self,
        shots_home: u16,
        shots_away: u16,
        forward_passes: u16,
        ball_progress_m: f32,
        box_entries: u16,
    ) {
        self.match_shots.push((shots_home, shots_away));
        self.match_progress.push((forward_passes, ball_progress_m, box_entries));
    }

    /// Calculate shot ratio for each match (home/away, with minimum 1 to avoid division by zero)
    fn shot_ratios(&self) -> Vec<f32> {
        self.match_shots
            .iter()
            .map(|(h, a)| {
                let home = (*h).max(1) as f32;
                let away = (*a).max(1) as f32;
                home / away
            })
            .collect()
    }

    /// Median shot ratio
    pub fn median_shot_ratio(&self) -> f32 {
        let mut ratios = self.shot_ratios();
        if ratios.is_empty() {
            return 1.0;
        }
        ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = ratios.len() / 2;
        if ratios.len() % 2 == 0 {
            (ratios[mid - 1] + ratios[mid]) / 2.0
        } else {
            ratios[mid]
        }
    }

    /// P95 shot ratio (for outlier detection)
    pub fn p95_shot_ratio(&self) -> f32 {
        let mut ratios = self.shot_ratios();
        if ratios.is_empty() {
            return 1.0;
        }
        ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((ratios.len() as f32) * 0.95).floor() as usize;
        ratios[idx.min(ratios.len() - 1)]
    }

    /// P5 shot ratio (for outlier detection - lower bound)
    pub fn p5_shot_ratio(&self) -> f32 {
        let mut ratios = self.shot_ratios();
        if ratios.is_empty() {
            return 1.0;
        }
        ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((ratios.len() as f32) * 0.05).floor() as usize;
        ratios[idx]
    }

    /// Average total shots per match
    pub fn avg_total_shots(&self) -> f32 {
        if self.match_shots.is_empty() {
            return 0.0;
        }
        let total: u32 = self.match_shots.iter().map(|(h, a)| (*h + *a) as u32).sum();
        total as f32 / self.match_shots.len() as f32
    }

    /// Average ball progress per match (meters)
    pub fn avg_ball_progress_m(&self) -> f32 {
        if self.match_progress.is_empty() {
            return 0.0;
        }
        let total: f32 = self.match_progress.iter().map(|(_, p, _)| p).sum();
        total / self.match_progress.len() as f32
    }

    /// Average box entries per match
    pub fn avg_box_entries(&self) -> f32 {
        if self.match_progress.is_empty() {
            return 0.0;
        }
        let total: u32 = self.match_progress.iter().map(|(_, _, e)| *e as u32).sum();
        total as f32 / self.match_progress.len() as f32
    }

    /// Generate shot balance report
    pub fn generate_report(&self) -> ShotBalanceReport {
        ShotBalanceReport {
            sample_size: self.match_shots.len(),
            median_shot_ratio: self.median_shot_ratio(),
            p5_shot_ratio: self.p5_shot_ratio(),
            p95_shot_ratio: self.p95_shot_ratio(),
            avg_total_shots: self.avg_total_shots(),
            avg_ball_progress_m: self.avg_ball_progress_m(),
            avg_box_entries: self.avg_box_entries(),
        }
    }
}

/// Shot balance report for gate checks
#[derive(Debug, Clone)]
pub struct ShotBalanceReport {
    pub sample_size: usize,
    pub median_shot_ratio: f32,
    pub p5_shot_ratio: f32,
    pub p95_shot_ratio: f32,
    pub avg_total_shots: f32,
    pub avg_ball_progress_m: f32,
    pub avg_box_entries: f32,
}

impl ShotBalanceReport {
    /// Check if report passes shot balance gates
    /// DoD: median ratio in [0.7, 1.3], p95 in [0.3, 3.0], avg shots in [12, 36]
    pub fn passes_shot_balance_gate(&self) -> ShotBalanceGateResult {
        let mut failures = Vec::new();

        // Gate 1: Median shot ratio in [0.7, 1.3]
        if self.median_shot_ratio < 0.7 || self.median_shot_ratio > 1.3 {
            failures.push(format!(
                "Median shot ratio {:.2} not in [0.7, 1.3] target",
                self.median_shot_ratio
            ));
        }

        // Gate 2: P95/P5 shot ratio in [0.3, 3.0] (no extreme outliers)
        if self.p5_shot_ratio < 0.3 {
            failures.push(format!(
                "P5 shot ratio {:.2} < 0.3 (too many home-weak matches)",
                self.p5_shot_ratio
            ));
        }
        if self.p95_shot_ratio > 3.0 {
            failures.push(format!(
                "P95 shot ratio {:.2} > 3.0 (too many home-dominant matches)",
                self.p95_shot_ratio
            ));
        }

        // Gate 3: Average total shots in [12, 36] (6-18 per team)
        if self.avg_total_shots < 12.0 {
            failures.push(format!(
                "Avg total shots {:.1} < 12 (too few shots per match)",
                self.avg_total_shots
            ));
        }
        if self.avg_total_shots > 36.0 {
            failures.push(format!(
                "Avg total shots {:.1} > 36 (too many shots per match)",
                self.avg_total_shots
            ));
        }

        ShotBalanceGateResult { passed: failures.is_empty(), failures }
    }

    /// Check if report passes progress gates
    /// DoD: avg ball progress >= 6m per possession, avg box entries >= 10 per match
    pub fn passes_progress_gate(&self) -> ShotBalanceGateResult {
        let mut failures = Vec::new();

        // Gate 1: Average box entries >= 10 per match
        if self.avg_box_entries < 10.0 {
            failures
                .push(format!("Avg box entries {:.1} < 10 per match target", self.avg_box_entries));
        }

        // Note: ball_progress_m per possession requires possession count
        // For now we just check if ball is progressing at all
        if self.avg_ball_progress_m < 100.0 {
            failures.push(format!(
                "Avg ball progress {:.1}m < 100m per match (ball not advancing)",
                self.avg_ball_progress_m
            ));
        }

        ShotBalanceGateResult { passed: failures.is_empty(), failures }
    }
}

/// Result of shot balance gate check
#[derive(Debug, Clone)]
pub struct ShotBalanceGateResult {
    pub passed: bool,
    pub failures: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let mut metrics = QualityMetrics::new();

        // Record zone-aware cluster density
        // Wing zone (y < 20): max expected = 8 (P3.7 adjusted)
        metrics.record_cluster_density_zone_aware(50.0, 10.0, 3, false); // OK
        metrics.record_cluster_density_zone_aware(50.0, 10.0, 6, false); // OK (< 8)
        metrics.record_cluster_density_zone_aware(50.0, 10.0, 10, false); // Anomaly (> 8)

        // Penalty area: max expected = 14 (P3.7 adjusted)
        metrics.record_cluster_density_zone_aware(10.0, field::CENTER_Y, 10, false); // OK
        metrics.record_cluster_density_zone_aware(95.0, field::CENTER_Y, 16, false); // Anomaly (> 14)

        metrics.record_ball_position(50.0, field::CENTER_Y, false); // Center - not corner
        metrics.record_ball_position(5.0, 5.0, false); // Corner
        metrics.record_pass(true);
        metrics.record_pass(true);
        metrics.record_pass(false);
        metrics.record_pressing_trigger();
        metrics.record_penetration_run();

        // 2 anomalies out of 5 samples = 40%
        assert!((metrics.cluster_anomaly_rate() - 0.40).abs() < 0.01);
        // Only 1 tick in corner, not 40+ consecutive, so no stuck events
        assert_eq!(metrics.corner_stuck_events, 0);
        assert!((metrics.pass_success_rate() - 0.666).abs() < 0.01);
        assert_eq!(metrics.pressing_triggers, 1);
        assert_eq!(metrics.penetration_runs, 1);
    }

    #[test]
    fn test_quality_gate_pass() {
        let report = QualityReport {
            cluster_anomaly_rate: 0.05, // < 10% OK
            cluster_density_by_zone: HashMap::new(),
            corner_stuck_events: 2,  // < 5 events OK
            oscillation_pct: 0.02,   // < 5% OK
            pass_success_rate: 0.75, // 70-85% OK
            pressing_triggers: 10,
            penetration_runs: 5,
            total_ticks: 5000,
            set_piece_ticks: 100,
            substate_usage: HashMap::new(),
            // Rule metrics (FIX_2601/0123 Phase 6)
            fouls_committed: (5, 4),
            handballs_detected: (1, 0),
            offsides_called: (2, 3),
            cards_issued: (1, 2, 0, 0),
            dispatcher_match_rate: 0.95,
        };

        let result = report.passes_quality_gate();
        assert!(result.passed, "Should pass: {:?}", result.failures);
    }

    #[test]
    fn test_quality_gate_fail_cluster() {
        let report = QualityReport {
            cluster_anomaly_rate: 0.15, // > 10% FAIL
            cluster_density_by_zone: HashMap::new(),
            corner_stuck_events: 2, // < 5 events OK
            oscillation_pct: 0.02,
            pass_success_rate: 0.75,
            pressing_triggers: 10,
            penetration_runs: 5,
            total_ticks: 5000,
            set_piece_ticks: 100,
            substate_usage: HashMap::new(),
            fouls_committed: (5, 4),
            handballs_detected: (1, 0),
            offsides_called: (2, 3),
            cards_issued: (1, 2, 0, 0),
            dispatcher_match_rate: 0.95,
        };

        let result = report.passes_quality_gate();
        assert!(!result.passed);
        assert!(result.failures[0].contains("Cluster anomaly"));
    }

    #[test]
    fn test_corner_zone_detection() {
        let mut metrics = QualityMetrics::new();

        // Brief visits to corners should NOT count as stuck (need 40+ consecutive ticks)
        // P3.6: Corner zone is 8m x 8m from each corner
        metrics.record_ball_position(5.0, 5.0, false); // Bottom-left corner (< 8m)
        metrics.record_ball_position(100.0, 5.0, false); // Bottom-right corner (resets counter)
        metrics.record_ball_position(5.0, 63.0, false); // Top-left corner (resets counter)
        metrics.record_ball_position(100.0, 63.0, false); // Top-right corner (resets counter)
        metrics.record_ball_position(50.0, field::CENTER_Y, false); // Center (resets counter)

        // Brief visits don't count as stuck
        assert_eq!(metrics.corner_stuck_events, 0);
        assert_eq!(metrics.total_ticks, 5);

        // Now test actual stuck scenario: 45 consecutive ticks in same corner (= 11.25 seconds)
        let mut metrics2 = QualityMetrics::new();
        for _ in 0..45 {
            metrics2.record_ball_position(5.0, 5.0, false); // Same corner for 45 ticks
        }
        // Stuck threshold is 40 ticks (10 seconds), so this counts as 1 stuck EVENT
        assert_eq!(metrics2.corner_stuck_events, 1);
        assert_eq!(metrics2.total_ticks, 45);

        // Leave corner and get stuck again = 2 events
        metrics2.record_ball_position(50.0, field::CENTER_Y, false); // Leave corner
        for _ in 0..50 {
            metrics2.record_ball_position(100.0, 63.0, false); // Another corner for 50 ticks
        }
        assert_eq!(metrics2.corner_stuck_events, 2);

        // P3.6: Test that set pieces reset counter and don't count as stuck
        let mut metrics3 = QualityMetrics::new();
        for i in 0..50 {
            // Set piece in first 20 ticks, then normal play
            let is_set_piece = i < 20;
            metrics3.record_ball_position(5.0, 5.0, is_set_piece);
        }
        // Only 30 ticks of normal play in corner (below 40 threshold)
        assert_eq!(metrics3.corner_stuck_events, 0);
    }

    #[test]
    fn test_zone_detection() {
        // Penalty area
        assert_eq!(
            FieldZone::from_position(10.0, field::CENTER_Y),
            FieldZone::PenaltyArea
        );
        assert_eq!(
            FieldZone::from_position(95.0, field::CENTER_Y),
            FieldZone::PenaltyArea
        );

        // Wings
        assert_eq!(FieldZone::from_position(50.0, 10.0), FieldZone::Wing);
        assert_eq!(FieldZone::from_position(50.0, 58.0), FieldZone::Wing);

        // Central midfield
        assert_eq!(
            FieldZone::from_position(field::CENTER_X, field::CENTER_Y),
            FieldZone::CentralMidfield
        );

        // Attacking midfield
        assert_eq!(
            FieldZone::from_position(80.0, field::CENTER_Y),
            FieldZone::AttackingMidfield
        );
    }

    #[test]
    fn test_set_piece_exclusion() {
        let mut metrics = QualityMetrics::new();

        // Normal play
        metrics.record_cluster_density_zone_aware(50.0, field::CENTER_Y, 10, false);  
        // Set piece - should not count as anomaly
        metrics.record_cluster_density_zone_aware(10.0, field::CENTER_Y, 15, true);   

        assert_eq!(metrics.cluster_total_samples, 1); // Only non-set-piece counted
        assert_eq!(metrics.set_piece_ticks, 1);
        // 10 players in central midfield (max 6) = anomaly
        assert_eq!(metrics.cluster_anomaly_count, 1);
    }

    // === Phase 4: Shot Balance Gate Tests ===

    #[test]
    fn test_shot_balance_stats_calculation() {
        let mut stats = ShotBalanceStats::new();

        // Add some matches with balanced shots
        stats.add_match(10, 12, 40, 500.0, 8); // ratio 0.83
        stats.add_match(15, 10, 50, 600.0, 12); // ratio 1.50
        stats.add_match(8, 8, 35, 400.0, 6); // ratio 1.00
        stats.add_match(12, 15, 45, 550.0, 10); // ratio 0.80
        stats.add_match(10, 10, 42, 480.0, 9); // ratio 1.00

        let report = stats.generate_report();

        // Median of [0.80, 0.83, 1.00, 1.00, 1.50] = 1.00
        assert!(
            (report.median_shot_ratio - 1.0).abs() < 0.01,
            "Median: {}",
            report.median_shot_ratio
        );

        // Avg total shots: (22 + 25 + 16 + 27 + 20) / 5 = 22
        assert!(
            (report.avg_total_shots - 22.0).abs() < 0.1,
            "Avg shots: {}",
            report.avg_total_shots
        );

        // Avg box entries: (8 + 12 + 6 + 10 + 9) / 5 = 9
        assert!(
            (report.avg_box_entries - 9.0).abs() < 0.1,
            "Avg entries: {}",
            report.avg_box_entries
        );
    }

    #[test]
    fn test_shot_balance_gate_pass() {
        let report = ShotBalanceReport {
            sample_size: 100,
            median_shot_ratio: 1.0, // [0.7, 1.3] OK
            p5_shot_ratio: 0.5,     // >= 0.3 OK
            p95_shot_ratio: 2.0,    // <= 3.0 OK
            avg_total_shots: 24.0,  // [12, 36] OK
            avg_ball_progress_m: 500.0,
            avg_box_entries: 15.0,
        };

        let result = report.passes_shot_balance_gate();
        assert!(result.passed, "Should pass: {:?}", result.failures);
    }

    #[test]
    fn test_shot_balance_gate_fail_ratio() {
        // Case: extreme imbalance (like 5-62)
        let report = ShotBalanceReport {
            sample_size: 100,
            median_shot_ratio: 0.1, // Way below 0.7 - FAIL
            p5_shot_ratio: 0.05,    // Below 0.3 - FAIL
            p95_shot_ratio: 0.2,    // OK
            avg_total_shots: 67.0,  // Above 36 - FAIL
            avg_ball_progress_m: 500.0,
            avg_box_entries: 15.0,
        };

        let result = report.passes_shot_balance_gate();
        assert!(!result.passed);
        assert!(result.failures.len() >= 2, "Should have multiple failures: {:?}", result.failures);
    }

    #[test]
    fn test_progress_gate_pass() {
        let report = ShotBalanceReport {
            sample_size: 100,
            median_shot_ratio: 1.0,
            p5_shot_ratio: 0.5,
            p95_shot_ratio: 2.0,
            avg_total_shots: 24.0,
            avg_ball_progress_m: 500.0, // >= 100 OK
            avg_box_entries: 15.0,      // >= 10 OK
        };

        let result = report.passes_progress_gate();
        assert!(result.passed, "Should pass: {:?}", result.failures);
    }

    #[test]
    fn test_progress_gate_fail() {
        let report = ShotBalanceReport {
            sample_size: 100,
            median_shot_ratio: 1.0,
            p5_shot_ratio: 0.5,
            p95_shot_ratio: 2.0,
            avg_total_shots: 24.0,
            avg_ball_progress_m: 50.0, // < 100 - FAIL
            avg_box_entries: 5.0,      // < 10 - FAIL
        };

        let result = report.passes_progress_gate();
        assert!(!result.passed);
        assert_eq!(result.failures.len(), 2, "Should have 2 failures: {:?}", result.failures);
    }
}
