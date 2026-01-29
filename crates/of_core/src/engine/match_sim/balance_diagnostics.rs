//! Balance Diagnostics for FIX_2512_1230
//!
//! Tracks per-team statistics to identify the source of ball equilibrium asymmetry.
//!
//! Key metrics:
//! - Ball X position over time (histogram by third)
//! - Pass direction (forward vs backward) by team
//! - Interception count by team
//! - Possession time by field third

use crate::engine::coordinates::x_to_team_view_m;
use crate::engine::positioning_engine::PositioningRole;
use crate::engine::team_phase::TeamPhase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::pitch_zone::{zone_of_position, PitchZone};
use crate::engine::physics_constants::field;

/// Diagnostic data collector for balance analysis
#[derive(Debug, Default)]
pub struct BalanceDiagnostics {
    /// Ball position samples (X coordinate in meters)
    ball_x_samples: Vec<f32>,
    /// Ball position samples split by half (meters)
    ball_x_sum_first_half: f64,
    ball_x_sum_second_half: f64,
    ball_x_samples_first_half: u32,
    ball_x_samples_second_half: u32,
    /// Ball position samples in possession team view (mirrored by attack direction)
    ball_x_possession_view_sum: f64,
    ball_x_possession_view_samples: u32,
    /// Ball position samples by possessing team (world X, meters)
    home_possession_ball_x_sum: f64,
    away_possession_ball_x_sum: f64,
    home_possession_ball_x_samples: u32,
    away_possession_ball_x_samples: u32,
    /// Ball position samples by possessing team (team-view X, meters)
    home_possession_ball_x_team_view_sum: f64,
    away_possession_ball_x_team_view_sum: f64,
    home_possession_ball_x_team_view_samples: u32,
    away_possession_ball_x_team_view_samples: u32,

    /// Pass attempts by team (home_forward, home_backward, away_forward, away_backward)
    home_forward_passes: u32,
    home_backward_passes: u32,
    away_forward_passes: u32,
    away_backward_passes: u32,
    /// Pass progression sums (meters, team-view)
    home_pass_progress_sum: f64,
    away_pass_progress_sum: f64,
    home_pass_progress_samples: u32,
    away_pass_progress_samples: u32,
    home_forward_progress_sum: f64,
    away_forward_progress_sum: f64,
    home_forward_progress_samples: u32,
    away_forward_progress_samples: u32,
    /// Max forward pass option distance (meters)
    home_max_forward_option_sum: f64,
    away_max_forward_option_sum: f64,
    home_max_forward_option_samples: u32,
    away_max_forward_option_samples: u32,
    /// Pass target distance histogram (5m buckets)
    home_pass_distance_histogram: HashMap<u32, u32>,
    away_pass_distance_histogram: HashMap<u32, u32>,
    /// Pass zone transitions (team-view)
    home_zone_transitions: HashMap<(PitchZone, PitchZone), u32>,
    away_zone_transitions: HashMap<(PitchZone, PitchZone), u32>,

    /// Role target X averages (team-view, meters)
    home_support_target_x_sum: f64,
    home_support_target_x_samples: u32,
    home_recycle_target_x_sum: f64,
    home_recycle_target_x_samples: u32,
    home_stretch_target_x_sum: f64,
    home_stretch_target_x_samples: u32,
    home_penetrate_target_x_sum: f64,
    home_penetrate_target_x_samples: u32,
    away_support_target_x_sum: f64,
    away_support_target_x_samples: u32,
    away_recycle_target_x_sum: f64,
    away_recycle_target_x_samples: u32,
    away_stretch_target_x_sum: f64,
    away_stretch_target_x_samples: u32,
    away_penetrate_target_x_sum: f64,
    away_penetrate_target_x_samples: u32,

    /// Role input snapshots (team-view, meters)
    home_snapshot_ball_x_sum: f64,
    home_snapshot_ball_x_samples: u32,
    away_snapshot_ball_x_sum: f64,
    away_snapshot_ball_x_samples: u32,
    home_snapshot_offside_line_sum: f64,
    home_snapshot_offside_line_samples: u32,
    away_snapshot_offside_line_sum: f64,
    away_snapshot_offside_line_samples: u32,
    home_snapshot_phase_counts: [u32; 4],
    away_snapshot_phase_counts: [u32; 4],

    /// Players ahead of the ball (outfield only)
    home_ahead_of_ball_count_sum: u32,
    home_ahead_of_ball_total_sum: u32,
    away_ahead_of_ball_count_sum: u32,
    away_ahead_of_ball_total_sum: u32,

    /// Penetrate role count during attacking phase (outfield only)
    home_attack_penetrate_count_sum: u32,
    home_attack_penetrate_total_sum: u32,
    home_attack_penetrate_samples: u32,
    away_attack_penetrate_count_sum: u32,
    away_attack_penetrate_total_sum: u32,
    away_attack_penetrate_samples: u32,
    /// Penetrate objective count during attacking phase (outfield only)
    home_attack_objective_penetrate_count_sum: u32,
    home_attack_objective_penetrate_total_sum: u32,
    home_attack_objective_penetrate_samples: u32,
    away_attack_objective_penetrate_count_sum: u32,
    away_attack_objective_penetrate_total_sum: u32,
    away_attack_objective_penetrate_samples: u32,

    /// Interceptions by team
    home_interceptions: u32,
    away_interceptions: u32,

    /// Possession ticks by field third (0=defensive, 1=middle, 2=attacking)
    home_possession_by_third: [u32; 3],
    away_possession_by_third: [u32; 3],

    /// Shots by team
    home_shots: u32,
    away_shots: u32,

    /// Goals by team
    home_goals: u32,
    away_goals: u32,
    /// GK distribution metrics
    home_gk_distributions: u32,
    away_gk_distributions: u32,
    home_gk_distribution_long: u32,
    away_gk_distribution_long: u32,
    home_gk_distribution_forward: u32,
    away_gk_distribution_forward: u32,
    home_gk_distribution_distance_sum: f64,
    away_gk_distribution_distance_sum: f64,
    home_gk_distribution_target_x_sum: f64,
    away_gk_distribution_target_x_sum: f64,
    home_gk_goal_kick_count: u32,
    away_gk_goal_kick_count: u32,
    home_gk_goal_kick_target_x_sum: f64,
    away_gk_goal_kick_target_x_sum: f64,
    home_gk_open_play_count: u32,
    away_gk_open_play_count: u32,
    home_gk_open_play_target_x_sum: f64,
    away_gk_open_play_target_x_sum: f64,

    /// Pressing intensity samples (sum of per-player intensities)
    home_pressing_intensity_sum: f64,
    away_pressing_intensity_sum: f64,
    pressing_intensity_samples: u32,

    /// Sample interval (every N ticks)
    sample_interval: u32,
    tick_counter: u32,

    // FIX_2512_1231: Player position tracking
    /// Sum of HOME outfield player X positions
    home_player_x_sum: f64,
    /// Sum of AWAY outfield player X positions
    away_player_x_sum: f64,
    /// Sum of HOME outfield player X positions (team-view, always attacking right)
    home_player_x_team_view_sum: f64,
    /// Sum of AWAY outfield player X positions (team-view, always attacking right)
    away_player_x_team_view_sum: f64,
    /// Number of player position samples
    player_position_samples: u64,

    // FIX_2512_1231 v4: Set piece counters to diagnose shot sources
    /// Corners awarded by team (attacking corner)
    home_corners: u32,
    away_corners: u32,
    /// Free kicks by team
    home_free_kicks: u32,
    away_free_kicks: u32,
    /// Penalties by team
    home_penalties: u32,
    away_penalties: u32,
    /// Shots from set pieces (subset of total shots)
    home_set_piece_shots: u32,
    away_set_piece_shots: u32,

    // FIX_2512_1231 v4b: Shot outcome breakdown
    home_shots_woodwork: u32,
    away_shots_woodwork: u32,
    home_shots_goal: u32,
    away_shots_goal: u32,
    home_shots_saved: u32,
    away_shots_saved: u32,
    home_shots_blocked: u32,
    away_shots_blocked: u32,
    home_shots_off_target: u32,
    away_shots_off_target: u32,

    // FIX_2512_0101: Role distribution tracking for asymmetry diagnosis
    /// Presser count when in Defense phase AND ball in opponent territory
    home_defense_deep_presser_count: u32,
    home_defense_deep_presser_samples: u32,
    away_defense_deep_presser_count: u32,
    away_defense_deep_presser_samples: u32,
    /// Marker count when in Defense phase AND ball in opponent territory
    home_defense_deep_marker_count: u32,
    home_defense_deep_marker_samples: u32,
    away_defense_deep_marker_count: u32,
    away_defense_deep_marker_samples: u32,
    /// Presser target X (team-view) during Defense phase
    home_presser_target_x_sum: f64,
    home_presser_target_x_samples: u32,
    away_presser_target_x_sum: f64,
    away_presser_target_x_samples: u32,
    /// Marker target X (team-view) during Defense phase
    home_marker_target_x_sum: f64,
    home_marker_target_x_samples: u32,
    away_marker_target_x_sum: f64,
    away_marker_target_x_samples: u32,
    /// Phase transition counts (possession changes)
    home_to_attack_transitions: u32,
    away_to_attack_transitions: u32,
    /// Ticks spent in each phase before transition
    home_defense_ticks_before_attack_sum: u64,
    home_defense_ticks_before_attack_samples: u32,
    away_defense_ticks_before_attack_sum: u64,
    away_defense_ticks_before_attack_samples: u32,
    /// Last phase for transition tracking
    last_home_phase: Option<TeamPhase>,
    last_away_phase: Option<TeamPhase>,
    /// Tick counter for phase duration
    home_phase_tick_counter: u64,
    away_phase_tick_counter: u64,
}

impl BalanceDiagnostics {
    pub fn new() -> Self {
        Self {
            sample_interval: 10, // Sample every 10 ticks (2.5 seconds at 250ms/tick)
            ..Default::default()
        }
    }

    /// Record ball position and possession
    pub fn record_tick(
        &mut self,
        ball_x: f32,
        owner_idx: Option<usize>,
        home_attacks_right: bool,
        minute: u8,
        pressing_intensity: &[f32; 22],
    ) {
        self.tick_counter += 1;

        // Sample ball position periodically
        if self.tick_counter % self.sample_interval == 0 {
            self.ball_x_samples.push(ball_x);
            if minute < 45 {
                self.ball_x_sum_first_half += ball_x as f64;
                self.ball_x_samples_first_half += 1;
            } else {
                self.ball_x_sum_second_half += ball_x as f64;
                self.ball_x_samples_second_half += 1;
            }
            if let Some(idx) = owner_idx {
                let is_home = idx < 11;
                let attacks_right = if is_home { home_attacks_right } else { !home_attacks_right };
                let ball_x_team_view = x_to_team_view_m(ball_x, attacks_right);
                self.ball_x_possession_view_sum += ball_x_team_view as f64;
                self.ball_x_possession_view_samples += 1;
                if is_home {
                    self.home_possession_ball_x_sum += ball_x as f64;
                    self.home_possession_ball_x_samples += 1;
                    self.home_possession_ball_x_team_view_sum += ball_x_team_view as f64;
                    self.home_possession_ball_x_team_view_samples += 1;
                } else {
                    self.away_possession_ball_x_sum += ball_x as f64;
                    self.away_possession_ball_x_samples += 1;
                    self.away_possession_ball_x_team_view_sum += ball_x_team_view as f64;
                    self.away_possession_ball_x_team_view_samples += 1;
                }
            }
            self.record_pressing_intensity_sample(pressing_intensity);
        }

        // Record possession by third
        if let Some(idx) = owner_idx {
            let is_home = idx < 11;
            let third = self.get_third_for_team(ball_x, is_home, home_attacks_right);

            if is_home {
                self.home_possession_by_third[third] += 1;
            } else {
                self.away_possession_by_third[third] += 1;
            }
        }
    }

    fn record_pressing_intensity_sample(&mut self, pressing_intensity: &[f32; 22]) {
        let home_sum: f32 = pressing_intensity[..11].iter().sum();
        let away_sum: f32 = pressing_intensity[11..].iter().sum();
        self.home_pressing_intensity_sum += home_sum as f64;
        self.away_pressing_intensity_sum += away_sum as f64;
        self.pressing_intensity_samples += 1;
    }

    /// Record GK distribution details (goal kicks + open play passes).
    pub fn record_gk_distribution(
        &mut self,
        is_home: bool,
        from_pos: (f32, f32),
        target_pos: (f32, f32),
        attacks_right: bool,
        is_long: bool,
        is_goal_kick: bool,
    ) {
        let dx = target_pos.0 - from_pos.0;
        let dy = target_pos.1 - from_pos.1;
        let distance = (dx * dx + dy * dy).sqrt() as f64;
        // TeamView: forward = target_x > from_x (always attacking toward higher X)
        let from_tv = x_to_team_view_m(from_pos.0, attacks_right);
        let target_tv = x_to_team_view_m(target_pos.0, attacks_right);
        let is_forward = target_tv > from_tv;
        let target_x_team_view = target_tv as f64;

        if is_home {
            self.home_gk_distributions += 1;
            if is_long {
                self.home_gk_distribution_long += 1;
            }
            if is_forward {
                self.home_gk_distribution_forward += 1;
            }
            self.home_gk_distribution_distance_sum += distance;
            self.home_gk_distribution_target_x_sum += target_x_team_view;
            if is_goal_kick {
                self.home_gk_goal_kick_count += 1;
                self.home_gk_goal_kick_target_x_sum += target_x_team_view;
            } else {
                self.home_gk_open_play_count += 1;
                self.home_gk_open_play_target_x_sum += target_x_team_view;
            }
        } else {
            self.away_gk_distributions += 1;
            if is_long {
                self.away_gk_distribution_long += 1;
            }
            if is_forward {
                self.away_gk_distribution_forward += 1;
            }
            self.away_gk_distribution_distance_sum += distance;
            self.away_gk_distribution_target_x_sum += target_x_team_view;
            if is_goal_kick {
                self.away_gk_goal_kick_count += 1;
                self.away_gk_goal_kick_target_x_sum += target_x_team_view;
            } else {
                self.away_gk_open_play_count += 1;
                self.away_gk_open_play_target_x_sum += target_x_team_view;
            }
        }
    }

    /// Record a pass attempt
    pub fn record_pass(
        &mut self,
        passer_idx: usize,
        is_forward: bool,
        progress_m: f32,
        pass_distance_m: f32,
        max_forward_option_m: f32,
    ) {
        const PASS_DISTANCE_BUCKET_M: f32 = 5.0;
        const PASS_DISTANCE_MAX_BUCKET: u32 = 120;

        let is_home = passer_idx < 11;
        if is_home {
            if is_forward {
                self.home_forward_passes += 1;
            } else {
                self.home_backward_passes += 1;
            }
            self.home_pass_progress_sum += progress_m as f64;
            self.home_pass_progress_samples += 1;
            if is_forward {
                self.home_forward_progress_sum += progress_m as f64;
                self.home_forward_progress_samples += 1;
            }
            self.home_max_forward_option_sum += max_forward_option_m as f64;
            self.home_max_forward_option_samples += 1;
            let bucket = ((pass_distance_m / PASS_DISTANCE_BUCKET_M).floor() as u32 * 5)
                .min(PASS_DISTANCE_MAX_BUCKET);
            *self.home_pass_distance_histogram.entry(bucket).or_insert(0) += 1;
        } else {
            if is_forward {
                self.away_forward_passes += 1;
            } else {
                self.away_backward_passes += 1;
            }
            self.away_pass_progress_sum += progress_m as f64;
            self.away_pass_progress_samples += 1;
            if is_forward {
                self.away_forward_progress_sum += progress_m as f64;
                self.away_forward_progress_samples += 1;
            }
            self.away_max_forward_option_sum += max_forward_option_m as f64;
            self.away_max_forward_option_samples += 1;
            let bucket = ((pass_distance_m / PASS_DISTANCE_BUCKET_M).floor() as u32 * 5)
                .min(PASS_DISTANCE_MAX_BUCKET);
            *self.away_pass_distance_histogram.entry(bucket).or_insert(0) += 1;
        }
    }

    /// Record a pass zone transition in team view.
    pub fn record_zone_transition(
        &mut self,
        is_home: bool,
        from_pos_m: (f32, f32),
        to_pos_m: (f32, f32),
        attacks_right: bool,
    ) {
        let from_zone = zone_of_position(from_pos_m.0, from_pos_m.1, attacks_right);
        let to_zone = zone_of_position(to_pos_m.0, to_pos_m.1, attacks_right);
        let transitions =
            if is_home { &mut self.home_zone_transitions } else { &mut self.away_zone_transitions };
        *transitions.entry((from_zone, to_zone)).or_insert(0) += 1;
    }

    /// Record an interception
    pub fn record_interception(&mut self, interceptor_idx: usize) {
        if interceptor_idx < 11 {
            self.home_interceptions += 1;
        } else {
            self.away_interceptions += 1;
        }
    }

    /// Record a shot
    pub fn record_shot(&mut self, shooter_idx: usize) {
        if shooter_idx < 11 {
            self.home_shots += 1;
        } else {
            self.away_shots += 1;
        }
    }

    /// Record a goal
    pub fn record_goal(&mut self, is_home_goal: bool) {
        if is_home_goal {
            self.home_goals += 1;
        } else {
            self.away_goals += 1;
        }
    }

    /// FIX_2512_1231 v4: Record a corner kick awarded
    pub fn record_corner(&mut self, is_home_attacking: bool) {
        if is_home_attacking {
            self.home_corners += 1;
        } else {
            self.away_corners += 1;
        }
    }

    /// FIX_2512_1231 v4: Record a free kick awarded
    pub fn record_free_kick(&mut self, is_home_attacking: bool) {
        if is_home_attacking {
            self.home_free_kicks += 1;
        } else {
            self.away_free_kicks += 1;
        }
    }

    /// FIX_2512_1231 v4: Record a penalty awarded
    pub fn record_penalty(&mut self, is_home_attacking: bool) {
        if is_home_attacking {
            self.home_penalties += 1;
        } else {
            self.away_penalties += 1;
        }
    }

    /// FIX_2512_1231 v4: Record a shot from set piece
    pub fn record_set_piece_shot(&mut self, shooter_idx: usize) {
        if shooter_idx < 11 {
            self.home_set_piece_shots += 1;
        } else {
            self.away_set_piece_shots += 1;
        }
    }

    /// FIX_2512_1231 v4b: Record shot outcome by type
    pub fn record_shot_woodwork(&mut self, shooter_idx: usize) {
        if shooter_idx < 11 {
            self.home_shots_woodwork += 1;
        } else {
            self.away_shots_woodwork += 1;
        }
    }

    pub fn record_shot_goal(&mut self, shooter_idx: usize) {
        if shooter_idx < 11 {
            self.home_shots_goal += 1;
        } else {
            self.away_shots_goal += 1;
        }
    }

    pub fn record_shot_saved(&mut self, shooter_idx: usize) {
        if shooter_idx < 11 {
            self.home_shots_saved += 1;
        } else {
            self.away_shots_saved += 1;
        }
    }

    pub fn record_shot_blocked(&mut self, shooter_idx: usize) {
        if shooter_idx < 11 {
            self.home_shots_blocked += 1;
        } else {
            self.away_shots_blocked += 1;
        }
    }

    pub fn record_shot_off_target(&mut self, shooter_idx: usize) {
        if shooter_idx < 11 {
            self.home_shots_off_target += 1;
        } else {
            self.away_shots_off_target += 1;
        }
    }

    /// FIX_2512_1231: Record player positions for distribution analysis
    pub fn record_player_positions(&mut self, positions: &[(f32, f32)], home_attacks_right: bool) {
        if positions.len() < 22 {
            return;
        }
        // HOME outfield players (indices 1-10, skip GK at 0) - TeamView X
        let home_team_view_x_sum: f64 = (1..11)
            .map(|i| x_to_team_view_m(positions[i].0, home_attacks_right) as f64)
            .sum();
        // HOME outfield players - world X
        let home_x_sum: f64 = (1..11).map(|i| positions[i].0 as f64).sum();
        // AWAY outfield players (indices 12-21, skip GK at 11) - TeamView X
        let away_attacks_right = !home_attacks_right;
        let away_team_view_x_sum: f64 = (12..22)
            .map(|i| x_to_team_view_m(positions[i].0, away_attacks_right) as f64)
            .sum();
        // AWAY outfield players (indices 12-21, skip GK at 11)
        let away_x_sum: f64 = (12..22).map(|i| positions[i].0 as f64).sum();

        self.home_player_x_sum += home_x_sum;
        self.away_player_x_sum += away_x_sum;
        self.home_player_x_team_view_sum += home_team_view_x_sum;
        self.away_player_x_team_view_sum += away_team_view_x_sum;
        self.player_position_samples += 1;
    }

    fn phase_index(phase: TeamPhase) -> usize {
        match phase {
            TeamPhase::Attack => 0,
            TeamPhase::Defense => 1,
            TeamPhase::TransitionAttack => 2,
            TeamPhase::TransitionDefense => 3,
        }
    }

    /// Record positioning role targets and ahead-of-ball ratios (outfield only).
    pub fn record_positioning_snapshot(
        &mut self,
        home_attacks_right: bool,
        ball_x: f32,
        home_offside_line: f32,
        away_offside_line: f32,
        home_phase: TeamPhase,
        away_phase: TeamPhase,
        home_roles: &[(PositioningRole, (f32, f32))],
        away_roles: &[(PositioningRole, (f32, f32))],
        home_positions: &[(f32, f32)],
        away_positions: &[(f32, f32)],
    ) {
        let home_ball_x_team_view = x_to_team_view_m(ball_x, home_attacks_right);
        let away_attacks_right = !home_attacks_right;
        let away_ball_x_team_view = x_to_team_view_m(ball_x, away_attacks_right);
        let home_offside_team_view = x_to_team_view_m(home_offside_line, home_attacks_right);
        let away_offside_team_view = x_to_team_view_m(away_offside_line, away_attacks_right);

        self.home_snapshot_ball_x_sum += home_ball_x_team_view as f64;
        self.home_snapshot_ball_x_samples += 1;
        self.away_snapshot_ball_x_sum += away_ball_x_team_view as f64;
        self.away_snapshot_ball_x_samples += 1;
        self.home_snapshot_offside_line_sum += home_offside_team_view as f64;
        self.home_snapshot_offside_line_samples += 1;
        self.away_snapshot_offside_line_sum += away_offside_team_view as f64;
        self.away_snapshot_offside_line_samples += 1;
        self.home_snapshot_phase_counts[Self::phase_index(home_phase)] += 1;
        self.away_snapshot_phase_counts[Self::phase_index(away_phase)] += 1;

        self.record_role_target_x(home_roles, home_attacks_right, true);
        self.record_role_target_x(away_roles, !home_attacks_right, false);

        self.record_ahead_of_ball(home_positions, ball_x, home_attacks_right, true);
        self.record_ahead_of_ball(away_positions, ball_x, !home_attacks_right, false);

        self.record_attack_penetrate(home_roles, home_phase, true);
        self.record_attack_penetrate(away_roles, away_phase, false);

        // FIX_2512_0101: Track defense deep roles
        if matches!(home_phase, TeamPhase::Defense | TeamPhase::TransitionDefense) {
            self.record_defense_deep_roles(home_roles, true, ball_x, home_attacks_right);
        }
        if matches!(away_phase, TeamPhase::Defense | TeamPhase::TransitionDefense) {
            self.record_defense_deep_roles(away_roles, false, ball_x, !home_attacks_right);
        }

        // FIX_2512_0101: Track phase transitions
        self.record_phase_transition(home_phase, away_phase);
    }

    fn record_role_target_x(
        &mut self,
        roles: &[(PositioningRole, (f32, f32))],
        attacks_right: bool,
        is_home: bool,
    ) {
        for (role, target) in roles {
            let target_x = x_to_team_view_m(target.0, attacks_right);
            match role {
                PositioningRole::Support => {
                    if is_home {
                        self.home_support_target_x_sum += target_x as f64;
                        self.home_support_target_x_samples += 1;
                    } else {
                        self.away_support_target_x_sum += target_x as f64;
                        self.away_support_target_x_samples += 1;
                    }
                }
                PositioningRole::Recycle => {
                    if is_home {
                        self.home_recycle_target_x_sum += target_x as f64;
                        self.home_recycle_target_x_samples += 1;
                    } else {
                        self.away_recycle_target_x_sum += target_x as f64;
                        self.away_recycle_target_x_samples += 1;
                    }
                }
                PositioningRole::Stretch => {
                    if is_home {
                        self.home_stretch_target_x_sum += target_x as f64;
                        self.home_stretch_target_x_samples += 1;
                    } else {
                        self.away_stretch_target_x_sum += target_x as f64;
                        self.away_stretch_target_x_samples += 1;
                    }
                }
                PositioningRole::Penetrate => {
                    if is_home {
                        self.home_penetrate_target_x_sum += target_x as f64;
                        self.home_penetrate_target_x_samples += 1;
                    } else {
                        self.away_penetrate_target_x_sum += target_x as f64;
                        self.away_penetrate_target_x_samples += 1;
                    }
                }
                // FIX_2512_0101: Track Presser/Marker target X
                PositioningRole::Presser => {
                    if is_home {
                        self.home_presser_target_x_sum += target_x as f64;
                        self.home_presser_target_x_samples += 1;
                    } else {
                        self.away_presser_target_x_sum += target_x as f64;
                        self.away_presser_target_x_samples += 1;
                    }
                }
                PositioningRole::Marker => {
                    if is_home {
                        self.home_marker_target_x_sum += target_x as f64;
                        self.home_marker_target_x_samples += 1;
                    } else {
                        self.away_marker_target_x_sum += target_x as f64;
                        self.away_marker_target_x_samples += 1;
                    }
                }
                _ => {}
            }
        }
    }

    /// FIX_2512_0101: Record defense deep role distribution
    /// Called when team is in Defense phase AND ball is deep in defender's own territory
    pub fn record_defense_deep_roles(
        &mut self,
        roles: &[(PositioningRole, (f32, f32))],
        is_home: bool,
        ball_x: f32,
        attacks_right: bool,
    ) {
        // Check if ball is in "deep" territory for defender (near defender's own goal)
        // TeamView: own goal at X=0, so "deep" means ball_tv < 35
        let ball_tv = x_to_team_view_m(ball_x, attacks_right);
        let ball_in_defender_deep = ball_tv < 35.0;

        if !ball_in_defender_deep {
            return;
        }

        let presser_count =
            roles.iter().filter(|(r, _)| matches!(r, PositioningRole::Presser)).count() as u32;
        let marker_count =
            roles.iter().filter(|(r, _)| matches!(r, PositioningRole::Marker)).count() as u32;

        if is_home {
            self.home_defense_deep_presser_count += presser_count;
            self.home_defense_deep_presser_samples += 1;
            self.home_defense_deep_marker_count += marker_count;
            self.home_defense_deep_marker_samples += 1;
        } else {
            self.away_defense_deep_presser_count += presser_count;
            self.away_defense_deep_presser_samples += 1;
            self.away_defense_deep_marker_count += marker_count;
            self.away_defense_deep_marker_samples += 1;
        }
    }

    /// FIX_2512_0101: Track phase transitions
    pub fn record_phase_transition(&mut self, home_phase: TeamPhase, away_phase: TeamPhase) {
        // Track HOME phase transitions
        if let Some(last) = self.last_home_phase {
            if !last.is_attacking() && home_phase.is_attacking() {
                // Defense → Attack transition
                self.home_to_attack_transitions += 1;
                self.home_defense_ticks_before_attack_sum += self.home_phase_tick_counter;
                self.home_defense_ticks_before_attack_samples += 1;
            }
            if last != home_phase {
                self.home_phase_tick_counter = 0;
            }
        }
        self.last_home_phase = Some(home_phase);
        self.home_phase_tick_counter += 1;

        // Track AWAY phase transitions
        if let Some(last) = self.last_away_phase {
            if !last.is_attacking() && away_phase.is_attacking() {
                // Defense → Attack transition
                self.away_to_attack_transitions += 1;
                self.away_defense_ticks_before_attack_sum += self.away_phase_tick_counter;
                self.away_defense_ticks_before_attack_samples += 1;
            }
            if last != away_phase {
                self.away_phase_tick_counter = 0;
            }
        }
        self.last_away_phase = Some(away_phase);
        self.away_phase_tick_counter += 1;
    }

    fn record_ahead_of_ball(
        &mut self,
        positions: &[(f32, f32)],
        ball_x: f32,
        attacks_right: bool,
        is_home: bool,
    ) {
        if positions.is_empty() {
            return;
        }
        // TeamView: ahead of ball = player_x_tv > ball_x_tv
        let ball_tv = x_to_team_view_m(ball_x, attacks_right);
        let ahead_count = positions
            .iter()
            .filter(|(x, _)| x_to_team_view_m(*x, attacks_right) > ball_tv)
            .count() as u32;
        let total = positions.len() as u32;
        if is_home {
            self.home_ahead_of_ball_count_sum += ahead_count;
            self.home_ahead_of_ball_total_sum += total;
        } else {
            self.away_ahead_of_ball_count_sum += ahead_count;
            self.away_ahead_of_ball_total_sum += total;
        }
    }

    fn record_attack_penetrate(
        &mut self,
        roles: &[(PositioningRole, (f32, f32))],
        phase: TeamPhase,
        is_home: bool,
    ) {
        if !phase.is_attacking() || roles.is_empty() {
            return;
        }
        let penetrate_count =
            roles.iter().filter(|(role, _)| matches!(role, PositioningRole::Penetrate)).count()
                as u32;
        let total = roles.len() as u32;
        if is_home {
            self.home_attack_penetrate_count_sum += penetrate_count;
            self.home_attack_penetrate_total_sum += total;
            self.home_attack_penetrate_samples += 1;
        } else {
            self.away_attack_penetrate_count_sum += penetrate_count;
            self.away_attack_penetrate_total_sum += total;
            self.away_attack_penetrate_samples += 1;
        }
    }

    pub fn record_attack_objective_penetrate(
        &mut self,
        home_penetrate_count: u32,
        home_total: u32,
        away_penetrate_count: u32,
        away_total: u32,
    ) {
        if home_total > 0 {
            self.home_attack_objective_penetrate_count_sum += home_penetrate_count;
            self.home_attack_objective_penetrate_total_sum += home_total;
            self.home_attack_objective_penetrate_samples += 1;
        }
        if away_total > 0 {
            self.away_attack_objective_penetrate_count_sum += away_penetrate_count;
            self.away_attack_objective_penetrate_total_sum += away_total;
            self.away_attack_objective_penetrate_samples += 1;
        }
    }

    /// Get field third (0=defensive, 1=middle, 2=attacking) for a team
    fn get_third_for_team(&self, ball_x: f32, is_home: bool, home_attacks_right: bool) -> usize {
        let attacks_right = if is_home { home_attacks_right } else { !home_attacks_right };
        // TeamView: x=0 is own goal, x=105 is opponent goal
        let ball_tv = x_to_team_view_m(ball_x, attacks_right);
        if ball_tv < 35.0 {
            0 // Defensive third
        } else if ball_tv < 70.0 {
            1 // Middle third
        } else {
            2 // Attacking third
        }
    }

    /// Generate a summary report
    pub fn generate_report(&self) -> DiagnosticReport {
        // Calculate ball X statistics
        let avg_ball_x = if self.ball_x_samples.is_empty() {
            field::CENTER_X
        } else {
            self.ball_x_samples.iter().sum::<f32>() / self.ball_x_samples.len() as f32
        };
        let avg_ball_x_possession_view = if self.ball_x_possession_view_samples > 0 {
            Some(
                (self.ball_x_possession_view_sum / self.ball_x_possession_view_samples as f64)
                    as f32,
            )
        } else {
            None
        };
        let avg_home_possession_ball_x = if self.home_possession_ball_x_samples > 0 {
            Some(
                (self.home_possession_ball_x_sum / self.home_possession_ball_x_samples as f64)
                    as f32,
            )
        } else {
            None
        };
        let avg_away_possession_ball_x = if self.away_possession_ball_x_samples > 0 {
            Some(
                (self.away_possession_ball_x_sum / self.away_possession_ball_x_samples as f64)
                    as f32,
            )
        } else {
            None
        };

        // Calculate ball position histogram (by 10m buckets)
        let mut ball_x_histogram: HashMap<u32, u32> = HashMap::new();
        for &x in &self.ball_x_samples {
            let bucket = (x / 10.0) as u32 * 10;
            *ball_x_histogram.entry(bucket).or_insert(0) += 1;
        }

        // Calculate pass forward ratios
        let home_total_passes = self.home_forward_passes + self.home_backward_passes;
        let away_total_passes = self.away_forward_passes + self.away_backward_passes;

        let home_forward_ratio = if home_total_passes > 0 {
            self.home_forward_passes as f32 / home_total_passes as f32
        } else {
            0.0
        };

        let away_forward_ratio = if away_total_passes > 0 {
            self.away_forward_passes as f32 / away_total_passes as f32
        } else {
            0.0
        };

        let avg_home_pass_progress_m = if self.home_pass_progress_samples > 0 {
            Some((self.home_pass_progress_sum / self.home_pass_progress_samples as f64) as f32)
        } else {
            None
        };
        let avg_away_pass_progress_m = if self.away_pass_progress_samples > 0 {
            Some((self.away_pass_progress_sum / self.away_pass_progress_samples as f64) as f32)
        } else {
            None
        };
        let avg_home_forward_progress_m = if self.home_forward_progress_samples > 0 {
            Some(
                (self.home_forward_progress_sum / self.home_forward_progress_samples as f64) as f32,
            )
        } else {
            None
        };
        let avg_away_forward_progress_m = if self.away_forward_progress_samples > 0 {
            Some(
                (self.away_forward_progress_sum / self.away_forward_progress_samples as f64) as f32,
            )
        } else {
            None
        };
        let avg_home_max_forward_option_m = if self.home_max_forward_option_samples > 0 {
            Some(
                (self.home_max_forward_option_sum / self.home_max_forward_option_samples as f64)
                    as f32,
            )
        } else {
            None
        };
        let avg_away_max_forward_option_m = if self.away_max_forward_option_samples > 0 {
            Some(
                (self.away_max_forward_option_sum / self.away_max_forward_option_samples as f64)
                    as f32,
            )
        } else {
            None
        };

        // Calculate possession percentages by third
        let home_total_poss: u32 = self.home_possession_by_third.iter().sum();
        let away_total_poss: u32 = self.away_possession_by_third.iter().sum();

        let home_attacking_third_pct = if home_total_poss > 0 {
            self.home_possession_by_third[2] as f32 / home_total_poss as f32 * 100.0
        } else {
            0.0
        };

        let away_attacking_third_pct = if away_total_poss > 0 {
            self.away_possession_by_third[2] as f32 / away_total_poss as f32 * 100.0
        } else {
            0.0
        };

        let avg_pressing_intensity_home = if self.pressing_intensity_samples > 0 {
            Some(
                (self.home_pressing_intensity_sum / (self.pressing_intensity_samples as f64 * 11.0))
                    as f32,
            )
        } else {
            None
        };
        let avg_pressing_intensity_away = if self.pressing_intensity_samples > 0 {
            Some(
                (self.away_pressing_intensity_sum / (self.pressing_intensity_samples as f64 * 11.0))
                    as f32,
            )
        } else {
            None
        };

        let avg_home_support_target_x_team_view_m = if self.home_support_target_x_samples > 0 {
            Some(
                (self.home_support_target_x_sum / self.home_support_target_x_samples as f64) as f32,
            )
        } else {
            None
        };
        let avg_away_support_target_x_team_view_m = if self.away_support_target_x_samples > 0 {
            Some(
                (self.away_support_target_x_sum / self.away_support_target_x_samples as f64) as f32,
            )
        } else {
            None
        };
        let avg_home_recycle_target_x_team_view_m = if self.home_recycle_target_x_samples > 0 {
            Some(
                (self.home_recycle_target_x_sum / self.home_recycle_target_x_samples as f64) as f32,
            )
        } else {
            None
        };
        let avg_away_recycle_target_x_team_view_m = if self.away_recycle_target_x_samples > 0 {
            Some(
                (self.away_recycle_target_x_sum / self.away_recycle_target_x_samples as f64) as f32,
            )
        } else {
            None
        };
        let avg_home_stretch_target_x_team_view_m = if self.home_stretch_target_x_samples > 0 {
            Some(
                (self.home_stretch_target_x_sum / self.home_stretch_target_x_samples as f64) as f32,
            )
        } else {
            None
        };
        let avg_away_stretch_target_x_team_view_m = if self.away_stretch_target_x_samples > 0 {
            Some(
                (self.away_stretch_target_x_sum / self.away_stretch_target_x_samples as f64) as f32,
            )
        } else {
            None
        };
        let avg_home_penetrate_target_x_team_view_m = if self.home_penetrate_target_x_samples > 0 {
            Some(
                (self.home_penetrate_target_x_sum / self.home_penetrate_target_x_samples as f64)
                    as f32,
            )
        } else {
            None
        };
        let avg_away_penetrate_target_x_team_view_m = if self.away_penetrate_target_x_samples > 0 {
            Some(
                (self.away_penetrate_target_x_sum / self.away_penetrate_target_x_samples as f64)
                    as f32,
            )
        } else {
            None
        };

        let avg_home_snapshot_ball_x_team_view_m = if self.home_snapshot_ball_x_samples > 0 {
            Some((self.home_snapshot_ball_x_sum / self.home_snapshot_ball_x_samples as f64) as f32)
        } else {
            None
        };
        let avg_away_snapshot_ball_x_team_view_m = if self.away_snapshot_ball_x_samples > 0 {
            Some((self.away_snapshot_ball_x_sum / self.away_snapshot_ball_x_samples as f64) as f32)
        } else {
            None
        };
        let avg_home_snapshot_offside_line_team_view_m = if self.home_snapshot_offside_line_samples
            > 0
        {
            Some(
                (self.home_snapshot_offside_line_sum
                    / self.home_snapshot_offside_line_samples as f64) as f32,
            )
        } else {
            None
        };
        let avg_away_snapshot_offside_line_team_view_m = if self.away_snapshot_offside_line_samples
            > 0
        {
            Some(
                (self.away_snapshot_offside_line_sum
                    / self.away_snapshot_offside_line_samples as f64) as f32,
            )
        } else {
            None
        };

        let home_ahead_of_ball_ratio = if self.home_ahead_of_ball_total_sum > 0 {
            Some(
                self.home_ahead_of_ball_count_sum as f32 / self.home_ahead_of_ball_total_sum as f32,
            )
        } else {
            None
        };
        let away_ahead_of_ball_ratio = if self.away_ahead_of_ball_total_sum > 0 {
            Some(
                self.away_ahead_of_ball_count_sum as f32 / self.away_ahead_of_ball_total_sum as f32,
            )
        } else {
            None
        };

        let home_attack_penetrate_avg_count = if self.home_attack_penetrate_samples > 0 {
            Some(
                self.home_attack_penetrate_count_sum as f32
                    / self.home_attack_penetrate_samples as f32,
            )
        } else {
            None
        };
        let away_attack_penetrate_avg_count = if self.away_attack_penetrate_samples > 0 {
            Some(
                self.away_attack_penetrate_count_sum as f32
                    / self.away_attack_penetrate_samples as f32,
            )
        } else {
            None
        };
        let home_attack_penetrate_ratio = if self.home_attack_penetrate_total_sum > 0 {
            Some(
                self.home_attack_penetrate_count_sum as f32
                    / self.home_attack_penetrate_total_sum as f32,
            )
        } else {
            None
        };
        let away_attack_penetrate_ratio = if self.away_attack_penetrate_total_sum > 0 {
            Some(
                self.away_attack_penetrate_count_sum as f32
                    / self.away_attack_penetrate_total_sum as f32,
            )
        } else {
            None
        };
        let home_attack_objective_penetrate_avg_count =
            if self.home_attack_objective_penetrate_samples > 0 {
                Some(
                    self.home_attack_objective_penetrate_count_sum as f32
                        / self.home_attack_objective_penetrate_samples as f32,
                )
            } else {
                None
            };
        let away_attack_objective_penetrate_avg_count =
            if self.away_attack_objective_penetrate_samples > 0 {
                Some(
                    self.away_attack_objective_penetrate_count_sum as f32
                        / self.away_attack_objective_penetrate_samples as f32,
                )
            } else {
                None
            };
        let home_attack_objective_penetrate_ratio =
            if self.home_attack_objective_penetrate_total_sum > 0 {
                Some(
                    self.home_attack_objective_penetrate_count_sum as f32
                        / self.home_attack_objective_penetrate_total_sum as f32,
                )
            } else {
                None
            };
        let away_attack_objective_penetrate_ratio =
            if self.away_attack_objective_penetrate_total_sum > 0 {
                Some(
                    self.away_attack_objective_penetrate_count_sum as f32
                        / self.away_attack_objective_penetrate_total_sum as f32,
                )
            } else {
                None
            };

        let home_gk_distribution_count = self.home_gk_distributions;
        let away_gk_distribution_count = self.away_gk_distributions;
        let home_gk_long_ratio = if home_gk_distribution_count > 0 {
            Some(self.home_gk_distribution_long as f32 / home_gk_distribution_count as f32)
        } else {
            None
        };
        let away_gk_long_ratio = if away_gk_distribution_count > 0 {
            Some(self.away_gk_distribution_long as f32 / away_gk_distribution_count as f32)
        } else {
            None
        };
        let home_gk_forward_ratio = if home_gk_distribution_count > 0 {
            Some(self.home_gk_distribution_forward as f32 / home_gk_distribution_count as f32)
        } else {
            None
        };
        let away_gk_forward_ratio = if away_gk_distribution_count > 0 {
            Some(self.away_gk_distribution_forward as f32 / away_gk_distribution_count as f32)
        } else {
            None
        };
        let home_gk_avg_distance_m = if home_gk_distribution_count > 0 {
            Some(
                (self.home_gk_distribution_distance_sum / home_gk_distribution_count as f64) as f32,
            )
        } else {
            None
        };
        let away_gk_avg_distance_m = if away_gk_distribution_count > 0 {
            Some(
                (self.away_gk_distribution_distance_sum / away_gk_distribution_count as f64) as f32,
            )
        } else {
            None
        };
        let home_gk_avg_target_x_team_view_m = if home_gk_distribution_count > 0 {
            Some(
                (self.home_gk_distribution_target_x_sum / home_gk_distribution_count as f64) as f32,
            )
        } else {
            None
        };
        let away_gk_avg_target_x_team_view_m = if away_gk_distribution_count > 0 {
            Some(
                (self.away_gk_distribution_target_x_sum / away_gk_distribution_count as f64) as f32,
            )
        } else {
            None
        };
        let home_gk_goal_kick_avg_target_x_team_view_m = if self.home_gk_goal_kick_count > 0 {
            Some((self.home_gk_goal_kick_target_x_sum / self.home_gk_goal_kick_count as f64) as f32)
        } else {
            None
        };
        let away_gk_goal_kick_avg_target_x_team_view_m = if self.away_gk_goal_kick_count > 0 {
            Some((self.away_gk_goal_kick_target_x_sum / self.away_gk_goal_kick_count as f64) as f32)
        } else {
            None
        };
        let home_gk_open_play_avg_target_x_team_view_m = if self.home_gk_open_play_count > 0 {
            Some((self.home_gk_open_play_target_x_sum / self.home_gk_open_play_count as f64) as f32)
        } else {
            None
        };
        let away_gk_open_play_avg_target_x_team_view_m = if self.away_gk_open_play_count > 0 {
            Some((self.away_gk_open_play_target_x_sum / self.away_gk_open_play_count as f64) as f32)
        } else {
            None
        };

        DiagnosticReport {
            avg_ball_x,
            avg_ball_x_possession_view,
            avg_home_possession_ball_x,
            avg_away_possession_ball_x,
            ball_x_histogram,
            home_forward_passes: self.home_forward_passes,
            home_backward_passes: self.home_backward_passes,
            away_forward_passes: self.away_forward_passes,
            away_backward_passes: self.away_backward_passes,
            home_forward_ratio,
            away_forward_ratio,
            avg_home_pass_progress_m,
            avg_away_pass_progress_m,
            avg_home_forward_progress_m,
            avg_away_forward_progress_m,
            avg_home_max_forward_option_m,
            avg_away_max_forward_option_m,
            home_pass_distance_histogram: self.home_pass_distance_histogram.clone(),
            away_pass_distance_histogram: self.away_pass_distance_histogram.clone(),
            home_zone_transitions: self.home_zone_transitions.clone(),
            away_zone_transitions: self.away_zone_transitions.clone(),
            home_interceptions: self.home_interceptions,
            away_interceptions: self.away_interceptions,
            home_attacking_third_pct,
            away_attacking_third_pct,
            avg_pressing_intensity_home,
            avg_pressing_intensity_away,
            avg_home_support_target_x_team_view_m,
            avg_away_support_target_x_team_view_m,
            avg_home_recycle_target_x_team_view_m,
            avg_away_recycle_target_x_team_view_m,
            avg_home_stretch_target_x_team_view_m,
            avg_away_stretch_target_x_team_view_m,
            avg_home_penetrate_target_x_team_view_m,
            avg_away_penetrate_target_x_team_view_m,
            avg_home_snapshot_ball_x_team_view_m,
            avg_away_snapshot_ball_x_team_view_m,
            avg_home_snapshot_offside_line_team_view_m,
            avg_away_snapshot_offside_line_team_view_m,
            home_snapshot_phase_counts: self.home_snapshot_phase_counts,
            away_snapshot_phase_counts: self.away_snapshot_phase_counts,
            home_ahead_of_ball_ratio,
            away_ahead_of_ball_ratio,
            home_attack_penetrate_avg_count,
            away_attack_penetrate_avg_count,
            home_attack_penetrate_ratio,
            away_attack_penetrate_ratio,
            home_attack_objective_penetrate_avg_count,
            away_attack_objective_penetrate_avg_count,
            home_attack_objective_penetrate_ratio,
            away_attack_objective_penetrate_ratio,
            home_gk_distribution_count,
            away_gk_distribution_count,
            home_gk_long_ratio,
            away_gk_long_ratio,
            home_gk_forward_ratio,
            away_gk_forward_ratio,
            home_gk_avg_distance_m,
            away_gk_avg_distance_m,
            home_gk_avg_target_x_team_view_m,
            away_gk_avg_target_x_team_view_m,
            home_gk_goal_kick_count: self.home_gk_goal_kick_count,
            away_gk_goal_kick_count: self.away_gk_goal_kick_count,
            home_gk_open_play_count: self.home_gk_open_play_count,
            away_gk_open_play_count: self.away_gk_open_play_count,
            home_gk_goal_kick_avg_target_x_team_view_m,
            away_gk_goal_kick_avg_target_x_team_view_m,
            home_gk_open_play_avg_target_x_team_view_m,
            away_gk_open_play_avg_target_x_team_view_m,
            home_shots: self.home_shots,
            away_shots: self.away_shots,
            home_goals: self.home_goals,
            away_goals: self.away_goals,
        }
    }

    /// Phase 0: Minimal diagnostics summary (single-run)
    pub fn print_phase0_summary(&self) {
        let avg_home_world = if self.home_possession_ball_x_samples > 0 {
            Some(
                (self.home_possession_ball_x_sum / self.home_possession_ball_x_samples as f64)
                    as f32,
            )
        } else {
            None
        };
        let avg_away_world = if self.away_possession_ball_x_samples > 0 {
            Some(
                (self.away_possession_ball_x_sum / self.away_possession_ball_x_samples as f64)
                    as f32,
            )
        } else {
            None
        };
        let avg_home_team = if self.home_possession_ball_x_team_view_samples > 0 {
            Some(
                (self.home_possession_ball_x_team_view_sum
                    / self.home_possession_ball_x_team_view_samples as f64) as f32,
            )
        } else {
            None
        };
        let avg_away_team = if self.away_possession_ball_x_team_view_samples > 0 {
            Some(
                (self.away_possession_ball_x_team_view_sum
                    / self.away_possession_ball_x_team_view_samples as f64) as f32,
            )
        } else {
            None
        };

        let home_pass_progress = if self.home_pass_progress_samples > 0 {
            Some((self.home_pass_progress_sum / self.home_pass_progress_samples as f64) as f32)
        } else {
            None
        };
        let away_pass_progress = if self.away_pass_progress_samples > 0 {
            Some((self.away_pass_progress_sum / self.away_pass_progress_samples as f64) as f32)
        } else {
            None
        };

        let home_total_passes = self.home_forward_passes + self.home_backward_passes;
        let away_total_passes = self.away_forward_passes + self.away_backward_passes;
        let home_forward_ratio = if home_total_passes > 0 {
            Some(self.home_forward_passes as f32 / home_total_passes as f32)
        } else {
            None
        };
        let away_forward_ratio = if away_total_passes > 0 {
            Some(self.away_forward_passes as f32 / away_total_passes as f32)
        } else {
            None
        };

        let home_max_forward = if self.home_max_forward_option_samples > 0 {
            Some(
                (self.home_max_forward_option_sum / self.home_max_forward_option_samples as f64)
                    as f32,
            )
        } else {
            None
        };
        let away_max_forward = if self.away_max_forward_option_samples > 0 {
            Some(
                (self.away_max_forward_option_sum / self.away_max_forward_option_samples as f64)
                    as f32,
            )
        } else {
            None
        };

        let fmt1 = |val: Option<f32>| match val {
            Some(v) => format!("{:.1}", v),
            None => "n/a".to_string(),
        };
        let fmt2 = |val: Option<f32>| match val {
            Some(v) => format!("{:.2}", v),
            None => "n/a".to_string(),
        };

        println!(
            "[PHASE0] home avg_x_world={} avg_x_team={} pass_prog={} fwd_ratio={} max_fwd_opt={}",
            fmt1(avg_home_world),
            fmt1(avg_home_team),
            fmt1(home_pass_progress),
            fmt2(home_forward_ratio),
            fmt1(home_max_forward)
        );
        println!(
            "[PHASE0] away avg_x_world={} avg_x_team={} pass_prog={} fwd_ratio={} max_fwd_opt={}",
            fmt1(avg_away_world),
            fmt1(avg_away_team),
            fmt1(away_pass_progress),
            fmt2(away_forward_ratio),
            fmt1(away_max_forward)
        );

        let home_bins = Self::phase0_pass_distance_bins(&self.home_pass_distance_histogram);
        let away_bins = Self::phase0_pass_distance_bins(&self.away_pass_distance_histogram);
        println!("[PHASE0] home pass_dist_bins={}", Self::format_phase0_bins(home_bins));
        println!("[PHASE0] away pass_dist_bins={}", Self::format_phase0_bins(away_bins));
        #[cfg(debug_assertions)]
        if let Some((count, avg, max)) = crate::engine::action_queue::debug_through_height_stats() {
            println!("[PHASE0] through_height samples={} avg={:.2} max={:.2}", count, avg, max);
        } else {
            println!("[PHASE0] through_height samples=0 avg=n/a max=n/a");
        }
    }

    fn phase0_pass_distance_bins(hist: &HashMap<u32, u32>) -> [u32; 6] {
        let mut bins = [0_u32; 6];
        for (&bucket, &count) in hist {
            if bucket < 5 {
                bins[0] += count;
            } else if bucket < 10 {
                bins[1] += count;
            } else if bucket < 15 {
                bins[2] += count;
            } else if bucket < 20 {
                bins[3] += count;
            } else if bucket < 30 {
                bins[4] += count;
            } else {
                bins[5] += count;
            }
        }
        bins
    }

    fn format_phase0_bins(bins: [u32; 6]) -> String {
        format!(
            "0-5:{} 5-10:{} 10-15:{} 15-20:{} 20-30:{} 30+:{}",
            bins[0], bins[1], bins[2], bins[3], bins[4], bins[5]
        )
    }

    /// Print a formatted report to stdout
    pub fn print_report(&self) {
        let report = self.generate_report();

        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║           BALANCE DIAGNOSTICS REPORT (FIX_2512_1230)        ║");
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ BALL POSITION                                              ║");
        println!(
            "║   Average X: {:.1}m (center={:.1}m)                         ║",
            report.avg_ball_x,
            field::CENTER_X
        );
        if self.ball_x_samples_first_half > 0 {
            let avg_first_half = self.ball_x_sum_first_half / self.ball_x_samples_first_half as f64;
            println!(
                "║   Avg X 1st half: {:.1}m                                 ║",
                avg_first_half
            );
        }
        if self.ball_x_samples_second_half > 0 {
            let avg_second_half =
                self.ball_x_sum_second_half / self.ball_x_samples_second_half as f64;
            println!(
                "║   Avg X 2nd half: {:.1}m                                 ║",
                avg_second_half
            );
        }
        if let Some(avg_ball_x_possession_view) = report.avg_ball_x_possession_view {
            println!(
                "║   Avg X team-view (possession): {:.1}m                   ║",
                avg_ball_x_possession_view
            );
        }
        println!("║   Histogram:                                               ║");
        for bucket in (0..=100).step_by(10) {
            let count = report.ball_x_histogram.get(&bucket).unwrap_or(&0);
            let bar_len = (*count as usize).min(30);
            let bar: String = "█".repeat(bar_len);
            println!(
                "║   {:3}-{:3}m: {:4} {}                                    ║",
                bucket,
                bucket + 10,
                count,
                bar
            );
        }
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ PASS DIRECTION                                             ║");
        println!(
            "║   HOME: {} fwd / {} bwd ({:.1}% forward)                    ║",
            report.home_forward_passes,
            report.home_backward_passes,
            report.home_forward_ratio * 100.0
        );
        println!(
            "║   AWAY: {} fwd / {} bwd ({:.1}% forward)                    ║",
            report.away_forward_passes,
            report.away_backward_passes,
            report.away_forward_ratio * 100.0
        );
        if let (Some(home_avg), Some(away_avg)) =
            (report.avg_home_pass_progress_m, report.avg_away_pass_progress_m)
        {
            println!(
                "║   Avg progress: HOME {:.1}m | AWAY {:.1}m                 ║",
                home_avg, away_avg
            );
        }
        if let (Some(home_avg), Some(away_avg)) =
            (report.avg_home_forward_progress_m, report.avg_away_forward_progress_m)
        {
            println!(
                "║   Avg fwd prog: HOME {:.1}m | AWAY {:.1}m                 ║",
                home_avg, away_avg
            );
        }
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ POSSESSION BALL X (TEAM)                                  ║");
        if let (Some(home_avg), Some(away_avg)) =
            (report.avg_home_possession_ball_x, report.avg_away_possession_ball_x)
        {
            println!(
                "║   HOME avg X: {:.1}m | AWAY avg X: {:.1}m                ║",
                home_avg, away_avg
            );
        } else {
            println!("║   Possession samples: none                               ║");
        }
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ PASS OPTIONS (max fwd option)                             ║");
        if let (Some(home_avg), Some(away_avg)) =
            (report.avg_home_max_forward_option_m, report.avg_away_max_forward_option_m)
        {
            println!(
                "║   HOME avg max fwd: {:.1}m | AWAY avg max fwd: {:.1}m     ║",
                home_avg, away_avg
            );
        } else {
            println!("║   Pass option samples: none                              ║");
        }
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ PASS TARGET DISTANCE (5m bins)                            ║");
        let print_pass_hist = |label: &str, hist: &HashMap<u32, u32>| {
            let mut has_samples = false;
            for bucket in (0..=120).step_by(5) {
                let count = hist.get(&bucket).copied().unwrap_or(0);
                if count > 0 {
                    has_samples = true;
                    println!(
                        "║   {} {:3}-{:3}m: {:4}                                   ║",
                        label,
                        bucket,
                        bucket + 5,
                        count
                    );
                }
            }
            if !has_samples {
                println!("║   {}: no samples                                        ║", label);
            }
        };
        print_pass_hist("HOME", &report.home_pass_distance_histogram);
        print_pass_hist("AWAY", &report.away_pass_distance_histogram);
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ PASS ZONE TRANSITIONS (top 5)                             ║");
        let print_zone_transitions =
            |label: &str, transitions: &HashMap<(PitchZone, PitchZone), u32>| {
                let mut items: Vec<_> = transitions.iter().collect();
                items.sort_by(|a, b| b.1.cmp(a.1));
                if items.is_empty() {
                    println!("║   {}: no samples                                         ║", label);
                    return;
                }
                for ((from_zone, to_zone), count) in items.into_iter().take(5) {
                    println!(
                        "║   {} {:<18} -> {:<18}: {:4}                        ║",
                        label,
                        format!("{:?}", from_zone),
                        format!("{:?}", to_zone),
                        count
                    );
                }
            };
        print_zone_transitions("HOME", &report.home_zone_transitions);
        print_zone_transitions("AWAY", &report.away_zone_transitions);
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ PASS ZONE COUNTS (from, top 5)                            ║");
        let print_zone_counts =
            |label: &str, transitions: &HashMap<(PitchZone, PitchZone), u32>| {
                let mut counts: HashMap<PitchZone, u32> = HashMap::new();
                for ((from_zone, _to_zone), count) in transitions {
                    *counts.entry(*from_zone).or_insert(0) += count;
                }
                let mut items: Vec<_> = counts.iter().collect();
                items.sort_by(|a, b| b.1.cmp(a.1));
                if items.is_empty() {
                    println!("║   {}: no samples                                     ║", label);
                    return;
                }
                for (zone, count) in items.into_iter().take(5) {
                    println!(
                        "║   {} {:<18}: {:4}                              ║",
                        label,
                        format!("{:?}", zone),
                        count
                    );
                }
            };
        print_zone_counts("HOME", &report.home_zone_transitions);
        print_zone_counts("AWAY", &report.away_zone_transitions);
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ ROLE TARGET X (TEAM-VIEW)                                 ║");
        let print_role_target = |label: &str, home: Option<f32>, away: Option<f32>| {
            if let (Some(home_val), Some(away_val)) = (home, away) {
                println!(
                    "║   {:<9} HOME {:>5.1}m | AWAY {:>5.1}m                 ║",
                    label, home_val, away_val
                );
            } else {
                println!("║   {:<9} no samples                                     ║", label);
            }
        };
        print_role_target(
            "Support",
            report.avg_home_support_target_x_team_view_m,
            report.avg_away_support_target_x_team_view_m,
        );
        print_role_target(
            "Recycle",
            report.avg_home_recycle_target_x_team_view_m,
            report.avg_away_recycle_target_x_team_view_m,
        );
        print_role_target(
            "Stretch",
            report.avg_home_stretch_target_x_team_view_m,
            report.avg_away_stretch_target_x_team_view_m,
        );
        print_role_target(
            "Penetrate",
            report.avg_home_penetrate_target_x_team_view_m,
            report.avg_away_penetrate_target_x_team_view_m,
        );
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ ROLE INPUTS (TEAM-VIEW)                                   ║");
        if let (Some(home_ball), Some(away_ball)) = (
            report.avg_home_snapshot_ball_x_team_view_m,
            report.avg_away_snapshot_ball_x_team_view_m,
        ) {
            println!(
                "║   Ball X avg: HOME {:>5.1}m | AWAY {:>5.1}m              ║",
                home_ball, away_ball
            );
        } else {
            println!("║   Ball X avg: no samples                                ║");
        }
        if let (Some(home_offside), Some(away_offside)) = (
            report.avg_home_snapshot_offside_line_team_view_m,
            report.avg_away_snapshot_offside_line_team_view_m,
        ) {
            println!(
                "║   Offside avg: HOME {:>5.1}m | AWAY {:>5.1}m             ║",
                home_offside, away_offside
            );
        } else {
            println!("║   Offside avg: no samples                               ║");
        }
        let [h_a, h_d, h_ta, h_td] = report.home_snapshot_phase_counts;
        let [a_a, a_d, a_ta, a_td] = report.away_snapshot_phase_counts;
        println!("║   Phase HOME A/D/TA/TD: {:>4}/{:>4}/{:>4}/{:>4}       ║", h_a, h_d, h_ta, h_td);
        println!("║   Phase AWAY A/D/TA/TD: {:>4}/{:>4}/{:>4}/{:>4}       ║", a_a, a_d, a_ta, a_td);
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ AHEAD OF BALL (OUTFIELD)                                  ║");
        if let (Some(home_ratio), Some(away_ratio)) =
            (report.home_ahead_of_ball_ratio, report.away_ahead_of_ball_ratio)
        {
            println!(
                "║   HOME ahead: {:.1}% | AWAY ahead: {:.1}%               ║",
                home_ratio * 100.0,
                away_ratio * 100.0
            );
        } else {
            println!("║   Ahead-of-ball samples: none                           ║");
        }
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ PENETRATE IN ATTACK PHASE (OUTFIELD)                      ║");
        if let (Some(home_avg), Some(home_ratio)) =
            (report.home_attack_penetrate_avg_count, report.home_attack_penetrate_ratio)
        {
            println!(
                "║   HOME avg: {:.1} | ratio: {:.1}%                       ║",
                home_avg,
                home_ratio * 100.0
            );
        } else {
            println!("║   HOME: no attacking samples                             ║");
        }
        if let (Some(away_avg), Some(away_ratio)) =
            (report.away_attack_penetrate_avg_count, report.away_attack_penetrate_ratio)
        {
            println!(
                "║   AWAY avg: {:.1} | ratio: {:.1}%                       ║",
                away_avg,
                away_ratio * 100.0
            );
        } else {
            println!("║   AWAY: no attacking samples                             ║");
        }
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ OBJECTIVE PENETRATE IN ATTACK PHASE (OUTFIELD)            ║");
        if let (Some(home_avg), Some(home_ratio)) = (
            report.home_attack_objective_penetrate_avg_count,
            report.home_attack_objective_penetrate_ratio,
        ) {
            println!(
                "║   HOME avg: {:.1} | ratio: {:.1}%                       ║",
                home_avg,
                home_ratio * 100.0
            );
        } else {
            println!("║   HOME: no attacking samples                             ║");
        }
        if let (Some(away_avg), Some(away_ratio)) = (
            report.away_attack_objective_penetrate_avg_count,
            report.away_attack_objective_penetrate_ratio,
        ) {
            println!(
                "║   AWAY avg: {:.1} | ratio: {:.1}%                       ║",
                away_avg,
                away_ratio * 100.0
            );
        } else {
            println!("║   AWAY: no attacking samples                             ║");
        }
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ GK DISTRIBUTION                                           ║");
        if report.home_gk_distribution_count > 0 {
            let long_pct = report.home_gk_long_ratio.unwrap_or(0.0) * 100.0;
            let forward_pct = report.home_gk_forward_ratio.unwrap_or(0.0) * 100.0;
            let avg_dist = report.home_gk_avg_distance_m.unwrap_or(0.0);
            let avg_target_x = report.home_gk_avg_target_x_team_view_m.unwrap_or(0.0);
            println!(
                "║   HOME: {} total | {:.1}% long | {:.1}% forward            ║",
                report.home_gk_distribution_count, long_pct, forward_pct
            );
            println!(
                "║         avg dist {:.1}m | avg tgt X tv {:.1}m              ║",
                avg_dist, avg_target_x
            );
            if report.home_gk_goal_kick_count > 0 {
                let avg_target = report.home_gk_goal_kick_avg_target_x_team_view_m.unwrap_or(0.0);
                println!(
                    "║         goal kicks {:3} | avg tgt X tv {:.1}m             ║",
                    report.home_gk_goal_kick_count, avg_target
                );
            }
            if report.home_gk_open_play_count > 0 {
                let avg_target = report.home_gk_open_play_avg_target_x_team_view_m.unwrap_or(0.0);
                println!(
                    "║         open play {:3} | avg tgt X tv {:.1}m              ║",
                    report.home_gk_open_play_count, avg_target
                );
            }
        } else {
            println!("║   HOME: no GK distributions                               ║");
        }
        if report.away_gk_distribution_count > 0 {
            let long_pct = report.away_gk_long_ratio.unwrap_or(0.0) * 100.0;
            let forward_pct = report.away_gk_forward_ratio.unwrap_or(0.0) * 100.0;
            let avg_dist = report.away_gk_avg_distance_m.unwrap_or(0.0);
            let avg_target_x = report.away_gk_avg_target_x_team_view_m.unwrap_or(0.0);
            println!(
                "║   AWAY: {} total | {:.1}% long | {:.1}% forward            ║",
                report.away_gk_distribution_count, long_pct, forward_pct
            );
            println!(
                "║         avg dist {:.1}m | avg tgt X tv {:.1}m              ║",
                avg_dist, avg_target_x
            );
            if report.away_gk_goal_kick_count > 0 {
                let avg_target = report.away_gk_goal_kick_avg_target_x_team_view_m.unwrap_or(0.0);
                println!(
                    "║         goal kicks {:3} | avg tgt X tv {:.1}m             ║",
                    report.away_gk_goal_kick_count, avg_target
                );
            }
            if report.away_gk_open_play_count > 0 {
                let avg_target = report.away_gk_open_play_avg_target_x_team_view_m.unwrap_or(0.0);
                println!(
                    "║         open play {:3} | avg tgt X tv {:.1}m              ║",
                    report.away_gk_open_play_count, avg_target
                );
            }
        } else {
            println!("║   AWAY: no GK distributions                               ║");
        }
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ PRESSING INTENSITY                                        ║");
        if let (Some(home_avg), Some(away_avg)) =
            (report.avg_pressing_intensity_home, report.avg_pressing_intensity_away)
        {
            println!("║   HOME avg intensity: {:.2}                               ║", home_avg);
            println!("║   AWAY avg intensity: {:.2}                               ║", away_avg);
        } else {
            println!("║   Pressing intensity samples: none                        ║");
        }
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ INTERCEPTIONS                                              ║");
        println!(
            "║   HOME intercepts: {}                                      ║",
            report.home_interceptions
        );
        println!(
            "║   AWAY intercepts: {}                                      ║",
            report.away_interceptions
        );
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ ATTACKING THIRD POSSESSION                                 ║");
        println!(
            "║   HOME in attacking third: {:.1}%                          ║",
            report.home_attacking_third_pct
        );
        println!(
            "║   AWAY in attacking third: {:.1}%                          ║",
            report.away_attacking_third_pct
        );
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ SET PIECES (FIX_2512_1231 v4)                              ║");
        println!(
            "║   Corners: HOME {} - {} AWAY                               ║",
            self.home_corners, self.away_corners
        );
        println!(
            "║   Free Kicks: HOME {} - {} AWAY                            ║",
            self.home_free_kicks, self.away_free_kicks
        );
        println!(
            "║   Penalties: HOME {} - {} AWAY                             ║",
            self.home_penalties, self.away_penalties
        );
        let _total_set_pieces_home = self.home_corners + self.home_free_kicks + self.home_penalties;
        let _total_set_pieces_away = self.away_corners + self.away_free_kicks + self.away_penalties;
        println!(
            "║   Set piece shots: HOME {} - {} AWAY                       ║",
            self.home_set_piece_shots, self.away_set_piece_shots
        );
        let open_play_shots_home = self.home_shots.saturating_sub(self.home_set_piece_shots);
        let open_play_shots_away = self.away_shots.saturating_sub(self.away_set_piece_shots);
        println!(
            "║   Open play shots: HOME {} - {} AWAY                       ║",
            open_play_shots_home, open_play_shots_away
        );
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ SHOT OUTCOMES (FIX_2512_1231 v4b)                          ║");
        println!(
            "║   Woodwork: HOME {} - {} AWAY                              ║",
            self.home_shots_woodwork, self.away_shots_woodwork
        );
        println!(
            "║   Goals: HOME {} - {} AWAY                                 ║",
            self.home_shots_goal, self.away_shots_goal
        );
        println!(
            "║   Saved: HOME {} - {} AWAY                                 ║",
            self.home_shots_saved, self.away_shots_saved
        );
        println!(
            "║   Blocked: HOME {} - {} AWAY                               ║",
            self.home_shots_blocked, self.away_shots_blocked
        );
        println!(
            "║   Off-target: HOME {} - {} AWAY                            ║",
            self.home_shots_off_target, self.away_shots_off_target
        );
        let tracked_home = self.home_shots_woodwork
            + self.home_shots_goal
            + self.home_shots_saved
            + self.home_shots_blocked
            + self.home_shots_off_target;
        let tracked_away = self.away_shots_woodwork
            + self.away_shots_goal
            + self.away_shots_saved
            + self.away_shots_blocked
            + self.away_shots_off_target;
        let untracked_home = self.home_shots.saturating_sub(tracked_home);
        let untracked_away = self.away_shots.saturating_sub(tracked_away);
        println!(
            "║   Untracked: HOME {} - {} AWAY                             ║",
            untracked_home, untracked_away
        );
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ OUTCOME                                                    ║");
        println!(
            "║   Shots: HOME {} - {} AWAY                                 ║",
            report.home_shots, report.away_shots
        );
        println!(
            "║   Goals: HOME {} - {} AWAY                                 ║",
            report.home_goals, report.away_goals
        );
        // FIX_2512_1231: Add player position averages
        if self.player_position_samples > 0 {
            let samples = self.player_position_samples as f64 * 10.0;
            let home_avg = self.home_player_x_sum / samples;
            let away_avg = self.away_player_x_sum / samples;
            let home_team_view_avg = self.home_player_x_team_view_sum / samples;
            let away_team_view_avg = self.away_player_x_team_view_sum / samples;
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║ PLAYER POSITIONS (FIX_2512_1231)                           ║");
            println!("║   HOME world avg X: {:.1}m                                 ║", home_avg);
            println!("║   AWAY world avg X: {:.1}m                                 ║", away_avg);
            println!("║   ---                                                       ║");
            println!(
                "║   HOME team-view avg X (attack-right): {:.1}m              ║",
                home_team_view_avg
            );
            println!(
                "║   AWAY team-view avg X (attack-right): {:.1}m              ║",
                away_team_view_avg
            );
            println!(
                "║   Team-view diff: {:.1}m (expect ~0m if symmetric)         ║",
                (home_team_view_avg - away_team_view_avg).abs()
            );
        }

        // FIX_2512_0101: Role distribution and phase transition diagnostics
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ ROLE DISTRIBUTION (FIX_2512_0101)                          ║");

        // Presser/Marker target X
        let home_presser_avg = if self.home_presser_target_x_samples > 0 {
            self.home_presser_target_x_sum / self.home_presser_target_x_samples as f64
        } else {
            0.0
        };
        let away_presser_avg = if self.away_presser_target_x_samples > 0 {
            self.away_presser_target_x_sum / self.away_presser_target_x_samples as f64
        } else {
            0.0
        };
        let home_marker_avg = if self.home_marker_target_x_samples > 0 {
            self.home_marker_target_x_sum / self.home_marker_target_x_samples as f64
        } else {
            0.0
        };
        let away_marker_avg = if self.away_marker_target_x_samples > 0 {
            self.away_marker_target_x_sum / self.away_marker_target_x_samples as f64
        } else {
            0.0
        };

        println!("║   Presser target X (team-view, avg):                       ║");
        println!(
            "║     HOME: {:.1}m  AWAY: {:.1}m  (diff: {:.1}m)            ║",
            home_presser_avg,
            away_presser_avg,
            (home_presser_avg - away_presser_avg).abs()
        );
        println!("║   Marker target X (team-view, avg):                        ║");
        println!(
            "║     HOME: {:.1}m  AWAY: {:.1}m  (diff: {:.1}m)            ║",
            home_marker_avg,
            away_marker_avg,
            (home_marker_avg - away_marker_avg).abs()
        );

        // Defense deep role counts (when defender is pressed deep in own territory)
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ DEFENSE DEEP ROLES (ball near defender's goal)             ║");
        let home_deep_presser_avg = if self.home_defense_deep_presser_samples > 0 {
            self.home_defense_deep_presser_count as f64
                / self.home_defense_deep_presser_samples as f64
        } else {
            0.0
        };
        let away_deep_presser_avg = if self.away_defense_deep_presser_samples > 0 {
            self.away_defense_deep_presser_count as f64
                / self.away_defense_deep_presser_samples as f64
        } else {
            0.0
        };
        let home_deep_marker_avg = if self.home_defense_deep_marker_samples > 0 {
            self.home_defense_deep_marker_count as f64
                / self.home_defense_deep_marker_samples as f64
        } else {
            0.0
        };
        let away_deep_marker_avg = if self.away_defense_deep_marker_samples > 0 {
            self.away_defense_deep_marker_count as f64
                / self.away_defense_deep_marker_samples as f64
        } else {
            0.0
        };

        println!("║   Pressers (avg count per sample):                         ║");
        println!(
            "║     HOME: {:.2}  AWAY: {:.2}  (diff: {:.2})               ║",
            home_deep_presser_avg,
            away_deep_presser_avg,
            (home_deep_presser_avg - away_deep_presser_avg).abs()
        );
        println!("║   Markers (avg count per sample):                          ║");
        println!(
            "║     HOME: {:.2}  AWAY: {:.2}  (diff: {:.2})               ║",
            home_deep_marker_avg,
            away_deep_marker_avg,
            (home_deep_marker_avg - away_deep_marker_avg).abs()
        );
        println!(
            "║   Samples: HOME {} - {} AWAY                               ║",
            self.home_defense_deep_presser_samples, self.away_defense_deep_presser_samples
        );

        // Phase transitions
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ PHASE TRANSITIONS                                          ║");
        println!("║   Defense→Attack transitions:                              ║");
        println!(
            "║     HOME: {}  AWAY: {}                                    ║",
            self.home_to_attack_transitions, self.away_to_attack_transitions
        );
        let home_avg_ticks = if self.home_defense_ticks_before_attack_samples > 0 {
            self.home_defense_ticks_before_attack_sum as f64
                / self.home_defense_ticks_before_attack_samples as f64
        } else {
            0.0
        };
        let away_avg_ticks = if self.away_defense_ticks_before_attack_samples > 0 {
            self.away_defense_ticks_before_attack_sum as f64
                / self.away_defense_ticks_before_attack_samples as f64
        } else {
            0.0
        };
        println!("║   Avg ticks in Defense before Attack:                      ║");
        println!(
            "║     HOME: {:.1}  AWAY: {:.1}  (diff: {:.1})               ║",
            home_avg_ticks,
            away_avg_ticks,
            (home_avg_ticks - away_avg_ticks).abs()
        );
        println!("║   (Lower = faster to attack, advantage for pressing)       ║");

        println!("╚════════════════════════════════════════════════════════════╝");
    }
}

/// Structured diagnostic report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub avg_ball_x: f32,
    pub avg_ball_x_possession_view: Option<f32>,
    pub avg_home_possession_ball_x: Option<f32>,
    pub avg_away_possession_ball_x: Option<f32>,
    pub ball_x_histogram: HashMap<u32, u32>,
    pub home_forward_passes: u32,
    pub home_backward_passes: u32,
    pub away_forward_passes: u32,
    pub away_backward_passes: u32,
    pub home_forward_ratio: f32,
    pub away_forward_ratio: f32,
    pub avg_home_pass_progress_m: Option<f32>,
    pub avg_away_pass_progress_m: Option<f32>,
    pub avg_home_forward_progress_m: Option<f32>,
    pub avg_away_forward_progress_m: Option<f32>,
    pub avg_home_max_forward_option_m: Option<f32>,
    pub avg_away_max_forward_option_m: Option<f32>,
    pub home_pass_distance_histogram: HashMap<u32, u32>,
    pub away_pass_distance_histogram: HashMap<u32, u32>,
    #[serde(
        serialize_with = "serialize_zone_transitions",
        deserialize_with = "deserialize_zone_transitions"
    )]
    pub home_zone_transitions: HashMap<(PitchZone, PitchZone), u32>,
    #[serde(
        serialize_with = "serialize_zone_transitions",
        deserialize_with = "deserialize_zone_transitions"
    )]
    pub away_zone_transitions: HashMap<(PitchZone, PitchZone), u32>,
    pub home_interceptions: u32,
    pub away_interceptions: u32,
    pub home_attacking_third_pct: f32,
    pub away_attacking_third_pct: f32,
    pub avg_pressing_intensity_home: Option<f32>,
    pub avg_pressing_intensity_away: Option<f32>,
    pub avg_home_support_target_x_team_view_m: Option<f32>,
    pub avg_away_support_target_x_team_view_m: Option<f32>,
    pub avg_home_recycle_target_x_team_view_m: Option<f32>,
    pub avg_away_recycle_target_x_team_view_m: Option<f32>,
    pub avg_home_stretch_target_x_team_view_m: Option<f32>,
    pub avg_away_stretch_target_x_team_view_m: Option<f32>,
    pub avg_home_penetrate_target_x_team_view_m: Option<f32>,
    pub avg_away_penetrate_target_x_team_view_m: Option<f32>,
    pub avg_home_snapshot_ball_x_team_view_m: Option<f32>,
    pub avg_away_snapshot_ball_x_team_view_m: Option<f32>,
    pub avg_home_snapshot_offside_line_team_view_m: Option<f32>,
    pub avg_away_snapshot_offside_line_team_view_m: Option<f32>,
    pub home_snapshot_phase_counts: [u32; 4],
    pub away_snapshot_phase_counts: [u32; 4],
    pub home_ahead_of_ball_ratio: Option<f32>,
    pub away_ahead_of_ball_ratio: Option<f32>,
    pub home_attack_penetrate_avg_count: Option<f32>,
    pub away_attack_penetrate_avg_count: Option<f32>,
    pub home_attack_penetrate_ratio: Option<f32>,
    pub away_attack_penetrate_ratio: Option<f32>,
    pub home_attack_objective_penetrate_avg_count: Option<f32>,
    pub away_attack_objective_penetrate_avg_count: Option<f32>,
    pub home_attack_objective_penetrate_ratio: Option<f32>,
    pub away_attack_objective_penetrate_ratio: Option<f32>,
    pub home_gk_distribution_count: u32,
    pub away_gk_distribution_count: u32,
    pub home_gk_long_ratio: Option<f32>,
    pub away_gk_long_ratio: Option<f32>,
    pub home_gk_forward_ratio: Option<f32>,
    pub away_gk_forward_ratio: Option<f32>,
    pub home_gk_avg_distance_m: Option<f32>,
    pub away_gk_avg_distance_m: Option<f32>,
    pub home_gk_avg_target_x_team_view_m: Option<f32>,
    pub away_gk_avg_target_x_team_view_m: Option<f32>,
    pub home_gk_goal_kick_count: u32,
    pub away_gk_goal_kick_count: u32,
    pub home_gk_open_play_count: u32,
    pub away_gk_open_play_count: u32,
    pub home_gk_goal_kick_avg_target_x_team_view_m: Option<f32>,
    pub away_gk_goal_kick_avg_target_x_team_view_m: Option<f32>,
    pub home_gk_open_play_avg_target_x_team_view_m: Option<f32>,
    pub away_gk_open_play_avg_target_x_team_view_m: Option<f32>,
    pub home_shots: u32,
    pub away_shots: u32,
    pub home_goals: u32,
    pub away_goals: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ZoneTransitionDump {
    from: String,
    to: String,
    count: u32,
}

fn serialize_zone_transitions<S>(
    transitions: &HashMap<(PitchZone, PitchZone), u32>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut items: Vec<ZoneTransitionDump> = transitions
        .iter()
        .map(|((from, to), count)| ZoneTransitionDump {
            from: from.as_str().to_string(),
            to: to.as_str().to_string(),
            count: *count,
        })
        .collect();
    items.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
    items.serialize(serializer)
}

fn deserialize_zone_transitions<'de, D>(
    deserializer: D,
) -> Result<HashMap<(PitchZone, PitchZone), u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let items = Vec::<ZoneTransitionDump>::deserialize(deserializer)?;
    let mut map = HashMap::new();
    for item in items {
        let from = PitchZone::from_str(&item.from)
            .ok_or_else(|| serde::de::Error::custom(format!("Unknown PitchZone: {}", item.from)))?;
        let to = PitchZone::from_str(&item.to)
            .ok_or_else(|| serde::de::Error::custom(format!("Unknown PitchZone: {}", item.to)))?;
        map.insert((from, to), item.count);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_basic() {
        let mut diag = BalanceDiagnostics::new();

        // Simulate some game activity (HOME attacks right = true)
        for i in 0..100 {
            diag.record_tick(20.0 + (i as f32 * 0.5), Some(5), true, 0, &[0.0; 22]);
            // HOME possession
        }

        diag.record_pass(5, true, 6.0, 12.0, 8.0); // HOME forward pass
        diag.record_pass(5, false, -4.0, 6.0, 3.0); // HOME backward pass
        diag.record_pass(15, true, 5.0, 15.0, 10.0); // AWAY forward pass
        diag.record_interception(12); // AWAY intercepts
        diag.record_shot(15);
        diag.record_goal(false); // AWAY goal

        let report = diag.generate_report();

        assert!(report.avg_ball_x > 20.0 && report.avg_ball_x < 70.0);
        assert!(report.avg_ball_x_possession_view.is_some());
        assert_eq!(report.home_forward_passes, 1);
        assert_eq!(report.home_backward_passes, 1);
        assert_eq!(report.away_forward_passes, 1);
        assert_eq!(report.away_interceptions, 1);
        assert_eq!(report.away_shots, 1);
        assert_eq!(report.away_goals, 1);
    }
}
