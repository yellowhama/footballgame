//! Shot Opportunity Telemetry System
//!
//! This module tracks shot opportunities for bias detection and analysis.
//! Records "all decision frames where shot was among Top-K candidates",
//! not just frames where a shot was actually taken.
//!
//! ## Key Parameters
//! - K = 5: Top-K candidates to consider
//! - T_abs = 0.15: Absolute utility threshold
//! - T_rel = 0.70: Relative utility ratio (vs top candidate)
//!
//! ## 4-Table Analysis
//! - Zone Table: Shot rate by 20-zone positional play system
//! - Timing Table: Shot rate by match minute buckets
//! - Constraint Table: Shot rate by defensive pressure (nearby opponents)
//! - Funnel Table: Shot rate by xG bucket

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::decision_topology::{CandidateAction, DecisionContext};
use super::utility::UtilityResult;
use crate::calibration::zone::PosPlayZoneId;
use crate::engine::tactical_context::TeamSide;

// ============================================================================
// Constants
// ============================================================================

/// Top-K candidates to consider
pub const SHOT_OPP_TOP_K: usize = 5;

/// Absolute utility threshold for shot consideration
pub const SHOT_OPP_T_ABS: f32 = 0.15;

/// Relative utility ratio (shot_utility / top_utility must be >= this)
pub const SHOT_OPP_T_REL: f32 = 0.70;

// ============================================================================
// Data Structures
// ============================================================================

/// Simple action category for telemetry recording
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionCategory {
    Shot,
    Pass,
    Dribble,
    Hold,
    Defense,
    Other,
}

/// Kickoff phase for bias analysis
/// Helps identify if bias is concentrated in specific game phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum KickoffPhase {
    /// Game start (first ~200 ticks, ~50 seconds)
    GameStart,
    /// After a goal (restart)
    AfterGoal,
    /// After halftime
    AfterHalftime,
    /// Normal play
    #[default]
    Normal,
}

/// Single shot opportunity frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShotOpportunityFrame {
    /// Current simulation tick
    pub tick: u32,
    /// Player track_id (0-21)
    pub player_track_id: u8,
    /// Team side (Home/Away)
    pub team_side: TeamSide,
    /// 20-zone positional play zone (1-20)
    pub zone: u8,
    /// Distance to goal in meters
    pub distance_to_goal: f32,
    /// Expected goals value
    pub xg: f32,
    /// Shot action utility value
    pub shot_utility: f32,
    /// Top candidate utility value
    pub top_utility: f32,
    /// Action that was actually chosen (simple category)
    pub action_chosen: ActionCategory,
    /// Whether shot was taken
    pub shot_taken: bool,
    /// Valid pass targets (track_ids, sorted for determinism)
    pub valid_targets: Vec<u8>,
    /// Number of opponents within 8m
    pub nearby_opponents_8m: u8,
    /// Distance to goalkeeper in meters
    pub goalkeeper_dist: f32,

    // === FIX_2601/0120: Bias analysis fields ===

    /// Owner's body direction alignment with goal: cos(angle_diff)
    /// 1.0 = facing goal directly, 0.0 = perpendicular, -1.0 = facing away
    pub owner_body_dir_to_goal_cos: f32,

    /// Team attack direction (true = attacking right/toward x=105)
    pub team_attacks_right: bool,

    /// Current kickoff phase (for identifying phase-specific bias)
    pub kickoff_phase: KickoffPhase,
}

/// Aggregated statistics per team
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShotOppAgg {
    /// Total shot opportunities recorded
    pub total_opportunities: u32,
    /// Shots actually taken
    pub shots_taken: u32,
    /// By zone: zone_index → (opportunities, shots_taken)
    pub by_zone: HashMap<u8, (u32, u32)>,
    /// By distance bucket: bucket → (opportunities, shots_taken)
    /// Buckets: 0 = 0-5m, 1 = 5-10m, 2 = 10-15m, 3 = 15-20m, 4 = 20-25m, 5 = 25m+
    pub by_distance_bucket: HashMap<u8, (u32, u32)>,
    /// By nearby opponents: count → (opportunities, shots_taken)
    pub by_nearby_opponents: HashMap<u8, (u32, u32)>,
    /// By xG bucket: bucket → (opportunities, shots_taken)
    /// Buckets: 0 = 0-0.05, 1 = 0.05-0.10, 2 = 0.10-0.20, 3 = 0.20-0.40, 4 = 0.40+
    pub by_xg_bucket: HashMap<u8, (u32, u32)>,
    /// By minute bucket: bucket → (opportunities, shots_taken)
    /// Buckets: 0 = 0-15, 1 = 15-30, 2 = 30-45, 3 = 45-60, 4 = 60-75, 5 = 75-90
    pub by_minute_bucket: HashMap<u8, (u32, u32)>,
}

impl ShotOppAgg {
    fn record(&mut self, frame: &ShotOpportunityFrame, minute: u8) {
        self.total_opportunities += 1;
        if frame.shot_taken {
            self.shots_taken += 1;
        }

        // By zone
        let zone_entry = self.by_zone.entry(frame.zone).or_insert((0, 0));
        zone_entry.0 += 1;
        if frame.shot_taken {
            zone_entry.1 += 1;
        }

        // By distance bucket
        let dist_bucket = distance_to_bucket(frame.distance_to_goal);
        let dist_entry = self.by_distance_bucket.entry(dist_bucket).or_insert((0, 0));
        dist_entry.0 += 1;
        if frame.shot_taken {
            dist_entry.1 += 1;
        }

        // By nearby opponents
        let opp_entry = self.by_nearby_opponents.entry(frame.nearby_opponents_8m).or_insert((0, 0));
        opp_entry.0 += 1;
        if frame.shot_taken {
            opp_entry.1 += 1;
        }

        // By xG bucket
        let xg_bucket = xg_to_bucket(frame.xg);
        let xg_entry = self.by_xg_bucket.entry(xg_bucket).or_insert((0, 0));
        xg_entry.0 += 1;
        if frame.shot_taken {
            xg_entry.1 += 1;
        }

        // By minute bucket
        let min_bucket = minute_to_bucket(minute);
        let min_entry = self.by_minute_bucket.entry(min_bucket).or_insert((0, 0));
        min_entry.0 += 1;
        if frame.shot_taken {
            min_entry.1 += 1;
        }
    }
}

/// Full shot opportunity telemetry for a match
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShotOppTelemetry {
    /// Individual frames (all recorded opportunities)
    pub frames: Vec<ShotOpportunityFrame>,
    /// Aggregated stats for home team
    pub home_agg: ShotOppAgg,
    /// Aggregated stats for away team
    pub away_agg: ShotOppAgg,
}

impl ShotOppTelemetry {
    /// Create new empty telemetry
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a shot opportunity frame
    pub fn record_frame(&mut self, frame: ShotOpportunityFrame, minute: u8) {
        match frame.team_side {
            TeamSide::Home => self.home_agg.record(&frame, minute),
            TeamSide::Away => self.away_agg.record(&frame, minute),
        }
        self.frames.push(frame);
    }

    /// Check if telemetry is enabled via environment variable
    pub fn is_enabled() -> bool {
        std::env::var("OF_DEBUG_SHOT_OPP")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
    }

    // ========================================================================
    // 4-Table Output
    // ========================================================================

    /// Print Zone Table (shot rate by 20-zone)
    pub fn print_zone_table(&self) {
        println!("=== Zone Table (20-zone) ===");
        println!("{:>6} | {:>6} | {:>6} | {:>13} | {:>5} | {:>6}",
                 "Zone", "Home", "Away", "Opportunities", "Shots", "Rate");
        println!("{}", "-".repeat(60));

        for zone_idx in 0..20u8 {
            let zone = zone_idx + 1; // 1-indexed for display
            let (h_opp, h_shot) = self.home_agg.by_zone.get(&zone).unwrap_or(&(0, 0));
            let (a_opp, a_shot) = self.away_agg.by_zone.get(&zone).unwrap_or(&(0, 0));
            let total_opp = h_opp + a_opp;
            let total_shot = h_shot + a_shot;
            let rate = if total_opp > 0 {
                total_shot as f32 / total_opp as f32
            } else {
                0.0
            };

            if total_opp > 0 {
                let zone_name = PosPlayZoneId::from_index(zone_idx as usize)
                    .map(|z| z.as_str())
                    .unwrap_or("???");
                println!("{:>6} | {:>6} | {:>6} | {:>13} | {:>5} | {:>6.2}",
                         zone_name, h_opp, a_opp, total_opp, total_shot, rate);
            }
        }
    }

    /// Print Timing Table (shot rate by minute bucket)
    pub fn print_timing_table(&self) {
        println!("\n=== Timing Table (by minute) ===");
        println!("{:>10} | {:>13} | {:>5} | {:>6}", "Minutes", "Opportunities", "Shots", "Rate");
        println!("{}", "-".repeat(45));

        let bucket_names = ["0-15", "15-30", "30-45", "45-60", "60-75", "75-90"];
        for (bucket, name) in bucket_names.iter().enumerate() {
            let bucket = bucket as u8;
            let (h_opp, h_shot) = self.home_agg.by_minute_bucket.get(&bucket).unwrap_or(&(0, 0));
            let (a_opp, a_shot) = self.away_agg.by_minute_bucket.get(&bucket).unwrap_or(&(0, 0));
            let total_opp = h_opp + a_opp;
            let total_shot = h_shot + a_shot;
            let rate = if total_opp > 0 {
                total_shot as f32 / total_opp as f32
            } else {
                0.0
            };
            println!("{:>10} | {:>13} | {:>5} | {:>6.2}", name, total_opp, total_shot, rate);
        }
    }

    /// Print Constraint Table (shot rate by nearby opponents)
    pub fn print_constraint_table(&self) {
        println!("\n=== Constraint Table (by nearby opponents) ===");
        println!("{:>12} | {:>13} | {:>5} | {:>6}", "Opponents", "Opportunities", "Shots", "Rate");
        println!("{}", "-".repeat(45));

        for opp_count in 0..=6u8 {
            let (h_opp, h_shot) = self.home_agg.by_nearby_opponents.get(&opp_count).unwrap_or(&(0, 0));
            let (a_opp, a_shot) = self.away_agg.by_nearby_opponents.get(&opp_count).unwrap_or(&(0, 0));
            let total_opp = h_opp + a_opp;
            let total_shot = h_shot + a_shot;
            if total_opp > 0 {
                let rate = total_shot as f32 / total_opp as f32;
                let label = if opp_count >= 6 { "6+".to_string() } else { opp_count.to_string() };
                println!("{:>12} | {:>13} | {:>5} | {:>6.2}", label, total_opp, total_shot, rate);
            }
        }
    }

    /// Print Funnel Table (shot rate by xG bucket)
    pub fn print_funnel_table(&self) {
        println!("\n=== Funnel Table (by xG) ===");
        println!("{:>12} | {:>13} | {:>5} | {:>6}", "xG Range", "Opportunities", "Shots", "Rate");
        println!("{}", "-".repeat(45));

        let bucket_names = ["0-0.05", "0.05-0.10", "0.10-0.20", "0.20-0.40", "0.40+"];
        for (bucket, name) in bucket_names.iter().enumerate() {
            let bucket = bucket as u8;
            let (h_opp, h_shot) = self.home_agg.by_xg_bucket.get(&bucket).unwrap_or(&(0, 0));
            let (a_opp, a_shot) = self.away_agg.by_xg_bucket.get(&bucket).unwrap_or(&(0, 0));
            let total_opp = h_opp + a_opp;
            let total_shot = h_shot + a_shot;
            let rate = if total_opp > 0 {
                total_shot as f32 / total_opp as f32
            } else {
                0.0
            };
            println!("{:>12} | {:>13} | {:>5} | {:>6.2}", name, total_opp, total_shot, rate);
        }
    }

    /// Print all 4 tables
    pub fn print_all_tables(&self) {
        println!("\n========== SHOT OPPORTUNITY TELEMETRY ==========\n");
        println!("Total frames: {}", self.frames.len());
        println!("Home: {} opportunities, {} shots ({:.1}%)",
                 self.home_agg.total_opportunities,
                 self.home_agg.shots_taken,
                 if self.home_agg.total_opportunities > 0 {
                     100.0 * self.home_agg.shots_taken as f32 / self.home_agg.total_opportunities as f32
                 } else { 0.0 });
        println!("Away: {} opportunities, {} shots ({:.1}%)",
                 self.away_agg.total_opportunities,
                 self.away_agg.shots_taken,
                 if self.away_agg.total_opportunities > 0 {
                     100.0 * self.away_agg.shots_taken as f32 / self.away_agg.total_opportunities as f32
                 } else { 0.0 });

        self.print_zone_table();
        self.print_timing_table();
        self.print_constraint_table();
        self.print_funnel_table();

        println!("\n================================================\n");
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert distance to bucket (0 = 0-5m, 1 = 5-10m, etc.)
fn distance_to_bucket(dist_m: f32) -> u8 {
    match dist_m {
        d if d < 5.0 => 0,
        d if d < 10.0 => 1,
        d if d < 15.0 => 2,
        d if d < 20.0 => 3,
        d if d < 25.0 => 4,
        _ => 5,
    }
}

/// Convert xG to bucket
fn xg_to_bucket(xg: f32) -> u8 {
    match xg {
        x if x < 0.05 => 0,
        x if x < 0.10 => 1,
        x if x < 0.20 => 2,
        x if x < 0.40 => 3,
        _ => 4,
    }
}

/// Convert minute to bucket (0 = 0-15, 1 = 15-30, etc.)
fn minute_to_bucket(minute: u8) -> u8 {
    (minute / 15).min(5)
}

/// Check if a CandidateAction is a shot type
pub fn is_shot_candidate(action: &CandidateAction) -> bool {
    matches!(
        action,
        CandidateAction::ShootNormal
            | CandidateAction::ShootFinesse
            | CandidateAction::ShootPower
            | CandidateAction::ShootChip
            | CandidateAction::Header
    )
}

/// Convert CandidateAction to ActionCategory for recording
pub fn candidate_to_action_category(action: &CandidateAction) -> ActionCategory {
    match action {
        CandidateAction::ShootNormal
        | CandidateAction::ShootFinesse
        | CandidateAction::ShootPower
        | CandidateAction::ShootChip
        | CandidateAction::Header => ActionCategory::Shot,
        CandidateAction::SafePass
        | CandidateAction::ProgressivePass
        | CandidateAction::SwitchPlay
        | CandidateAction::ThroughBall
        | CandidateAction::Cross
        | CandidateAction::ClearBall => ActionCategory::Pass,
        CandidateAction::CarryBall | CandidateAction::TakeOn | CandidateAction::OneTwo => {
            ActionCategory::Dribble
        }
        CandidateAction::Hold
        | CandidateAction::ShieldBall
        | CandidateAction::HoldUpPlay
        | CandidateAction::DrawFoul => ActionCategory::Hold,
        CandidateAction::Jockey
        | CandidateAction::DelayPass
        | CandidateAction::CoverSpace
        | CandidateAction::BlockLane
        | CandidateAction::ClosingDown
        | CandidateAction::InterceptAttempt
        | CandidateAction::ForceTouchline
        | CandidateAction::TrackRunner
        | CandidateAction::StandingTackle
        | CandidateAction::SlidingTackle
        | CandidateAction::ShoulderCharge
        | CandidateAction::PokeAway => ActionCategory::Defense,
        _ => ActionCategory::Other,
    }
}

/// Check shot opportunity and create frame if criteria met
///
/// Returns Some(ShotOpportunityFrame) if shot was in top-K and meets thresholds.
///
/// # Additional Parameters for Bias Analysis (FIX_2601/0120)
/// - `body_dir`: Player's body direction as unit vector (dx, dy)
/// - `attacks_right`: Team's attack direction (true = toward x=105)
/// - `kickoff_phase`: Current game phase for phase-specific bias detection
pub fn check_shot_opportunity(
    tick: u32,
    player_track_id: u8,
    team_side: TeamSide,
    zone: PosPlayZoneId,
    decision_ctx: &DecisionContext,
    candidates: &[(CandidateAction, UtilityResult)],
    chosen_action: &CandidateAction,
    valid_pass_targets: &[u8],
    goalkeeper_dist: f32,
    // FIX_2601/0120: Bias analysis parameters
    body_dir: (f32, f32),
    attacks_right: bool,
    kickoff_phase: KickoffPhase,
) -> Option<ShotOpportunityFrame> {
    // 1. Find shot candidate in results
    let shot_result = candidates
        .iter()
        .find(|(action, _)| is_shot_candidate(action));

    let (_, shot_util) = shot_result?;

    // 2. Calculate Top-K
    let mut sorted: Vec<_> = candidates.iter().collect();
    sorted.sort_by(|a, b| {
        b.1.utility
            .partial_cmp(&a.1.utility)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_k: Vec<_> = sorted.into_iter().take(SHOT_OPP_TOP_K).collect();

    // 3. Check if shot is in Top-K
    let shot_in_top_k = top_k
        .iter()
        .any(|(action, _)| is_shot_candidate(action));
    if !shot_in_top_k {
        return None;
    }

    // 4. Check thresholds
    let top_utility = top_k.first().map(|(_, u)| u.utility).unwrap_or(0.0);

    // T_abs check
    if shot_util.utility < SHOT_OPP_T_ABS {
        return None;
    }

    // T_rel check (avoid division by zero)
    if top_utility > 0.0 && shot_util.utility < top_utility * SHOT_OPP_T_REL {
        return None;
    }

    // 5. Build frame
    let mut valid_targets: Vec<u8> = valid_pass_targets.to_vec();
    valid_targets.sort(); // Deterministic ordering

    let shot_taken = is_shot_candidate(chosen_action);

    // FIX_2601/0120: Calculate body direction alignment with goal
    // goal_dir = (+1, 0) if attacks_right, (-1, 0) if attacks_left
    // cos(angle) = dot(body_dir, goal_dir) = body_dir.0 * goal_dir.0
    let goal_dir_x = if attacks_right { 1.0_f32 } else { -1.0_f32 };
    let owner_body_dir_to_goal_cos = body_dir.0 * goal_dir_x;

    Some(ShotOpportunityFrame {
        tick,
        player_track_id,
        team_side,
        zone: zone.to_index() as u8 + 1, // 1-indexed for display
        distance_to_goal: decision_ctx.distance_to_goal,
        xg: decision_ctx.xg,
        shot_utility: shot_util.utility,
        top_utility,
        action_chosen: candidate_to_action_category(chosen_action),
        shot_taken,
        valid_targets,
        nearby_opponents_8m: decision_ctx.nearby_opponents_8m,
        goalkeeper_dist,
        // FIX_2601/0120: Bias analysis fields
        owner_body_dir_to_goal_cos,
        team_attacks_right: attacks_right,
        kickoff_phase,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_to_bucket() {
        assert_eq!(distance_to_bucket(3.0), 0);
        assert_eq!(distance_to_bucket(7.0), 1);
        assert_eq!(distance_to_bucket(12.0), 2);
        assert_eq!(distance_to_bucket(18.0), 3);
        assert_eq!(distance_to_bucket(23.0), 4);
        assert_eq!(distance_to_bucket(30.0), 5);
    }

    #[test]
    fn test_xg_to_bucket() {
        assert_eq!(xg_to_bucket(0.02), 0);
        assert_eq!(xg_to_bucket(0.07), 1);
        assert_eq!(xg_to_bucket(0.15), 2);
        assert_eq!(xg_to_bucket(0.30), 3);
        assert_eq!(xg_to_bucket(0.50), 4);
    }

    #[test]
    fn test_minute_to_bucket() {
        assert_eq!(minute_to_bucket(10), 0);
        assert_eq!(minute_to_bucket(20), 1);
        assert_eq!(minute_to_bucket(40), 2);
        assert_eq!(minute_to_bucket(55), 3);
        assert_eq!(minute_to_bucket(70), 4);
        assert_eq!(minute_to_bucket(85), 5);
    }

    #[test]
    fn test_is_shot_candidate() {
        assert!(is_shot_candidate(&CandidateAction::ShootNormal));
        assert!(is_shot_candidate(&CandidateAction::ShootFinesse));
        assert!(is_shot_candidate(&CandidateAction::ShootPower));
        assert!(is_shot_candidate(&CandidateAction::ShootChip));
        assert!(is_shot_candidate(&CandidateAction::Header));
        assert!(!is_shot_candidate(&CandidateAction::SafePass));
        assert!(!is_shot_candidate(&CandidateAction::TakeOn));
    }

    #[test]
    fn test_telemetry_enabled() {
        // By default should be disabled
        std::env::remove_var("OF_DEBUG_SHOT_OPP");
        assert!(!ShotOppTelemetry::is_enabled());

        // Enable via env var
        std::env::set_var("OF_DEBUG_SHOT_OPP", "1");
        assert!(ShotOppTelemetry::is_enabled());

        // Cleanup
        std::env::remove_var("OF_DEBUG_SHOT_OPP");
    }

    #[test]
    fn test_agg_record() {
        let mut agg = ShotOppAgg::default();

        let frame = ShotOpportunityFrame {
            tick: 100,
            player_track_id: 9,
            team_side: TeamSide::Home,
            zone: 17, // CBox
            distance_to_goal: 12.0,
            xg: 0.15,
            shot_utility: 0.5,
            top_utility: 0.6,
            action_chosen: ActionCategory::Shot,
            shot_taken: true,
            valid_targets: vec![7, 8, 10],
            nearby_opponents_8m: 2,
            goalkeeper_dist: 8.0,
            // FIX_2601/0120: Bias analysis fields
            owner_body_dir_to_goal_cos: 0.95, // Facing goal
            team_attacks_right: true,
            kickoff_phase: KickoffPhase::Normal,
        };

        agg.record(&frame, 35);

        assert_eq!(agg.total_opportunities, 1);
        assert_eq!(agg.shots_taken, 1);
        assert_eq!(agg.by_zone.get(&17), Some(&(1, 1)));
        assert_eq!(agg.by_distance_bucket.get(&2), Some(&(1, 1))); // 10-15m
        assert_eq!(agg.by_nearby_opponents.get(&2), Some(&(1, 1)));
        assert_eq!(agg.by_xg_bucket.get(&2), Some(&(1, 1))); // 0.10-0.20
        assert_eq!(agg.by_minute_bucket.get(&2), Some(&(1, 1))); // 30-45
    }

    #[test]
    fn test_body_dir_to_goal_cos_calculation() {
        // Test: Home player (attacks_right=true) facing right (1.0, 0.0) -> cos = 1.0
        let goal_dir_x = 1.0_f32; // attacks_right
        let body_dir = (1.0_f32, 0.0_f32);
        let cos = body_dir.0 * goal_dir_x;
        assert!((cos - 1.0).abs() < 0.001, "Facing goal should have cos=1.0");

        // Test: Home player facing away (-1.0, 0.0) -> cos = -1.0
        let body_dir = (-1.0_f32, 0.0_f32);
        let cos = body_dir.0 * goal_dir_x;
        assert!((cos - (-1.0)).abs() < 0.001, "Facing away should have cos=-1.0");

        // Test: Away player (attacks_right=false) facing left (-1.0, 0.0) -> cos = 1.0
        let goal_dir_x = -1.0_f32; // attacks_left
        let body_dir = (-1.0_f32, 0.0_f32);
        let cos = body_dir.0 * goal_dir_x;
        assert!((cos - 1.0).abs() < 0.001, "Away facing goal should have cos=1.0");

        // Test: Away player facing right (1.0, 0.0) -> cos = -1.0 (facing away from goal)
        let body_dir = (1.0_f32, 0.0_f32);
        let cos = body_dir.0 * goal_dir_x;
        assert!((cos - (-1.0)).abs() < 0.001, "Away facing wrong way should have cos=-1.0");
    }
}
