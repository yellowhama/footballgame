//! 6-Factor Pass System (Open Football Style)
//!
//! This module contains pass-related logic for MatchEngine:
//! - 6-Factor pass evaluation system
//! - Pass target selection
//! - Pass execution with Gold Traits
//! - FIX_2601/0115: Vision/range gate, Role-specific priorities, Lane-clear status
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::attribute_calc::{calculate_max_pass_range, pass_distance_factor_v2};
use super::buildup_phase::BuildupPhase;
use super::pitch_zone::zone_of_position;
use super::zone_transition::ZoneTransitionStyle;
use super::MatchEngine;
use crate::engine::body_blocking::{self, find_interceptors};
use crate::engine::coordinates;
use crate::engine::physics_constants::{self, field, home_advantage, skills};
use crate::engine::types::Coord10;
use crate::models::trait_system::TraitId;
use crate::models::{MatchEvent, Position};
use crate::tactics::{BuildUpStyle, TeamTempo};
use rand::Rng;
use serde::{Deserialize, Serialize};

// ===========================================
// FIX_2601/0115: Pass Evaluator Logging
// ===========================================

/// Pass score components for logging and analysis
/// Each field represents one factor's contribution (0.0-1.0 normalized)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassScoreComponents {
    /// Distance factor (15% weight): optimal 10-25m
    pub distance: f32,
    /// Safety factor (15% weight): interception risk
    pub safety: f32,
    /// Readiness factor (10% weight): receiver marking
    pub readiness: f32,
    /// Progression factor (40% weight): forward pass preference
    pub progression: f32,
    /// Circulation factor: lateral/back pass bonus
    pub circulation: f32,
    /// Space factor (10% weight): space around receiver
    pub space: f32,
    /// Tactical factor (10% weight): tactical advantage
    pub tactical: f32,
    /// Role-specific bonus total
    pub role_bonus: f32,
    /// Lane status: "Clear" / "Contested(X.XX)" / "Blocked"
    pub lane_status: String,
}

/// Complete pass evaluation log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassEvaluationLog {
    /// Current tick when evaluation occurred
    pub tick: u32,
    /// Passer index (0-21)
    pub passer_id: u32,
    /// Target index (0-21)
    pub target_id: u32,
    /// Component scores breakdown
    pub components: PassScoreComponents,
    /// Final weighted score
    pub final_score: f32,
    /// Whether this target was selected
    pub selected: bool,
}

// ===========================================
// FIX_2601/0115: Pass Failure Branching
// ===========================================

/// Pass outcome with detailed failure types (open-football style)
/// Instead of binary success/failure, provides specific failure reasons
#[derive(Debug, Clone)]
pub enum PassFailureOutcome {
    /// Pass was intercepted by opponent
    Intercepted {
        interceptor_idx: usize,
        interception_position_m: (f32, f32),
    },
    /// Receiver failed to control the ball (first touch error)
    Miscontrolled {
        receiver_idx: usize,
        loose_ball_position_m: (f32, f32),
    },
    /// Passer was under pressure and failed to execute
    PressureForced {
        pressure_source_idx: usize,
    },
    /// Ball went out of bounds
    OutOfBounds {
        exit_position_m: (f32, f32),
    },
}

/// Complete pass outcome (success or specific failure)
#[derive(Debug, Clone)]
pub enum DetailedPassOutcome {
    /// Pass succeeded - receiver gets the ball
    Success {
        receiver_idx: usize,
    },
    /// Pass failed with specific reason
    Failed(PassFailureOutcome),
}

/// Context for pass failure resolution
pub struct PassFailureContext {
    /// Highest intercept probability from any opponent
    pub max_intercept_prob: f32,
    /// Index of best potential interceptor
    pub best_interceptor_idx: Option<usize>,
    /// Position where interception would occur
    pub intercept_position_m: Option<(f32, f32)>,
    /// Pressure level on receiver (0.0-1.0)
    pub receiver_pressure: f32,
    /// Receiver's first touch ability (1-20)
    pub receiver_first_touch: f32,
    /// Primary pressure source on passer
    pub passer_pressure_source_idx: Option<usize>,
    /// Passer pressure level (0.0-1.0)
    pub passer_pressure: f32,
}

// ===========================================
// FIX_2601/0115: Lane-Clear Status
// ===========================================

/// Lane clear status for pass evaluation (open-football style)
/// Maps our interceptor-based safety to a discrete lane status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LaneClearStatus {
    /// No interceptors - completely safe pass lane
    Clear,
    /// Some interception risk but passable
    Contested { risk: f32 },
    /// High interception risk - lane effectively blocked
    Blocked,
}

impl LaneClearStatus {
    /// Penalty to apply to pass score based on lane status
    pub fn score_penalty(&self) -> f32 {
        match self {
            LaneClearStatus::Clear => 0.0,
            LaneClearStatus::Contested { risk } => risk * 0.3,
            LaneClearStatus::Blocked => 0.5,
        }
    }
}

// ===========================================
// FIX_2601/0113: Pass Risk Type System
// ===========================================

/// Pass type categorization based on distance for risk assessment
/// FIX_2601/0113: Different pass types have different base risks
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PassRiskType {
    /// Short pass (<15m) - very low risk
    Short,
    /// Medium pass (15-30m) - low risk
    Medium,
    /// Long pass (30-45m) - moderate risk
    Long,
    /// Very long pass (>45m) - high risk
    VeryLong,
    /// Cross - high risk (aerial ball)
    Cross,
}

impl PassRiskType {
    /// Classify pass type based on distance and cross flag
    pub fn from_distance(distance_m: f32, is_cross: bool) -> Self {
        if is_cross {
            return Self::Cross;
        }
        match distance_m {
            d if d < 15.0 => Self::Short,
            d if d < 30.0 => Self::Medium,
            d if d < 45.0 => Self::Long,
            _ => Self::VeryLong,
        }
    }

    /// Base risk for this pass type (0.0-1.0)
    pub fn base_risk(&self) -> f32 {
        match self {
            Self::Short => 0.03,
            Self::Medium => 0.08,
            Self::Long => 0.18,
            Self::VeryLong => 0.30,
            Self::Cross => 0.25,
        }
    }

    /// Get skill-adjusted risk considering Long Passing and Crossing attributes
    pub fn adjusted_risk(&self, long_passing: f32, crossing: f32) -> f32 {
        let base = self.base_risk();
        match self {
            Self::Long | Self::VeryLong => {
                // Long Passing reduces risk by up to 30%
                base * (1.0 - (long_passing / 100.0) * 0.30)
            }
            Self::Cross => {
                // Crossing reduces risk by up to 40%
                base * (1.0 - (crossing / 100.0) * 0.40)
            }
            _ => base,
        }
    }

    /// Get success rate modifier (1.0 - adjusted_risk)
    pub fn success_modifier(&self, long_passing: f32, crossing: f32) -> f32 {
        1.0 - self.adjusted_risk(long_passing, crossing)
    }
}

// ===========================================
// FIX_2601/0115: Role-Specific Pass Bonus
// ===========================================

/// Role-specific pass evaluation bonuses (open-football style)
/// Forwards prioritize line breaks and shot setups
/// Defenders prioritize safety
#[derive(Debug, Clone, Copy)]
pub struct RolePassBonus {
    /// Bonus for passes that break the defensive line
    pub line_break_bonus: f32,
    /// Bonus for passes to shooting positions
    pub shot_setup_bonus: f32,
    /// Multiplier for safety factor weight
    pub safety_multiplier: f32,
    /// Multiplier for progression factor weight
    pub progression_multiplier: f32,
    /// Bonus for spread/switch passes
    pub spread_bonus: f32,
}

impl Default for RolePassBonus {
    fn default() -> Self {
        Self {
            line_break_bonus: 0.10,
            shot_setup_bonus: 0.10,
            safety_multiplier: 1.0,
            progression_multiplier: 1.0,
            spread_bonus: 0.05,
        }
    }
}

impl RolePassBonus {
    /// Get role-specific bonus based on player position
    pub fn for_position(pos: &crate::models::Position) -> Self {
        use crate::models::Position;
        match pos {
            // Forwards: Line breaks + shot setup, lower safety concern
            Position::ST | Position::CF | Position::LW | Position::RW | Position::FW => Self {
                line_break_bonus: 0.30,
                shot_setup_bonus: 0.35,
                safety_multiplier: 0.7,
                progression_multiplier: 1.2,
                spread_bonus: 0.0,
            },
            // Attacking midfielders: Balanced but favor progression
            Position::CAM | Position::LM | Position::RM => Self {
                line_break_bonus: 0.15,
                shot_setup_bonus: 0.15,
                safety_multiplier: 0.9,
                progression_multiplier: 1.1,
                spread_bonus: 0.15,
            },
            // Central midfielders: Balanced with spread bonus
            Position::CM | Position::MF => Self {
                line_break_bonus: 0.10,
                shot_setup_bonus: 0.10,
                safety_multiplier: 1.0,
                progression_multiplier: 1.0,
                spread_bonus: 0.20,
            },
            // Defensive midfielders: Safety first
            Position::CDM => Self {
                line_break_bonus: 0.05,
                shot_setup_bonus: 0.05,
                safety_multiplier: 1.3,
                progression_multiplier: 0.9,
                spread_bonus: 0.15,
            },
            // Defenders: Safety first, spread bonus
            Position::CB
            | Position::LB
            | Position::RB
            | Position::LWB
            | Position::RWB
            | Position::DF => Self {
                line_break_bonus: 0.0,
                shot_setup_bonus: 0.0,
                safety_multiplier: 1.5,
                progression_multiplier: 0.8,
                spread_bonus: 0.10,
            },
            // Goalkeeper: Maximum safety
            Position::GK => Self {
                line_break_bonus: 0.0,
                shot_setup_bonus: 0.0,
                safety_multiplier: 2.0,
                progression_multiplier: 0.5,
                spread_bonus: 0.0,
            },
        }
    }
}

// ===========================================
// FIX_2601/0112: Trajectory-Based Pass Risk (GRF Integration)
// ===========================================

/// Calculate trajectory-based pass risk by sampling points along the pass path.
///
/// This approach is inspired by Google Research Football's bot.py:
/// - Sample 10 points along the pass trajectory
/// - Find the minimum clearance to any opponent at each point
/// - Return risk based on the "choke point" (minimum clearance)
///
/// Returns: Risk value 0.0 (safe) to 1.0 (dangerous)
pub fn calculate_trajectory_risk(
    passer_pos_m: (f32, f32),
    receiver_pos_m: (f32, f32),
    opponent_positions_m: &[(f32, f32)],
) -> f32 {
    const SAMPLES: usize = 10;
    const MAX_INTERCEPT_RANGE: f32 = 8.0; // meters - beyond this is considered safe

    if opponent_positions_m.is_empty() {
        return 0.0;
    }

    let mut min_clearance = f32::MAX;

    // Sample 10 points along the trajectory (excluding start, including end)
    for i in 1..=SAMPLES {
        let t = i as f32 / SAMPLES as f32;
        let point_x = passer_pos_m.0 + t * (receiver_pos_m.0 - passer_pos_m.0);
        let point_y = passer_pos_m.1 + t * (receiver_pos_m.1 - passer_pos_m.1);

        // Find minimum distance to any opponent at this point
        for opp in opponent_positions_m {
            let dx = point_x - opp.0;
            let dy = point_y - opp.1;
            let dist = (dx * dx + dy * dy).sqrt();
            min_clearance = min_clearance.min(dist);
        }
    }

    // Convert clearance to risk: closer = higher risk
    // Risk = 1.0 when clearance = 0, Risk = 0.0 when clearance >= MAX_INTERCEPT_RANGE
    1.0 - (min_clearance / MAX_INTERCEPT_RANGE).clamp(0.0, 1.0)
}

/// Calculate trajectory risk with opponent velocity prediction.
///
/// Enhanced version that considers opponent movement:
/// - Predicts where opponents will be when the ball reaches each point
/// - Uses pass speed and opponent speeds for time-based prediction
///
/// Returns: Risk value 0.0 (safe) to 1.0 (dangerous)
#[allow(dead_code)]
pub fn calculate_trajectory_risk_with_velocity(
    passer_pos_m: (f32, f32),
    receiver_pos_m: (f32, f32),
    opponent_positions_m: &[(f32, f32)],
    opponent_velocities: &[(f32, f32)],
    pass_speed_mps: f32,
) -> f32 {
    const SAMPLES: usize = 10;
    const MAX_INTERCEPT_RANGE: f32 = 6.0; // Tighter range for velocity prediction

    if opponent_positions_m.is_empty() {
        return 0.0;
    }

    let pass_dist = {
        let dx = receiver_pos_m.0 - passer_pos_m.0;
        let dy = receiver_pos_m.1 - passer_pos_m.1;
        (dx * dx + dy * dy).sqrt()
    };

    if pass_dist < 0.1 {
        return 0.0;
    }

    let total_flight_time = pass_dist / pass_speed_mps;
    let mut min_clearance = f32::MAX;

    for i in 1..=SAMPLES {
        let t = i as f32 / SAMPLES as f32;
        let time_at_point = total_flight_time * t;

        let point_x = passer_pos_m.0 + t * (receiver_pos_m.0 - passer_pos_m.0);
        let point_y = passer_pos_m.1 + t * (receiver_pos_m.1 - passer_pos_m.1);

        // Predict opponent positions at this time
        for (idx, opp) in opponent_positions_m.iter().enumerate() {
            let (vx, vy) = opponent_velocities.get(idx).copied().unwrap_or((0.0, 0.0));

            // Clamp prediction time (opponent can only move so far)
            let pred_time = time_at_point.min(1.5);
            let pred_x = opp.0 + vx * pred_time;
            let pred_y = opp.1 + vy * pred_time;

            let dx = point_x - pred_x;
            let dy = point_y - pred_y;
            let dist = (dx * dx + dy * dy).sqrt();
            min_clearance = min_clearance.min(dist);
        }
    }

    1.0 - (min_clearance / MAX_INTERCEPT_RANGE).clamp(0.0, 1.0)
}

impl MatchEngine {
    // ===========================================
    // FIX_2601/0115: Vision Range Gate
    // ===========================================

    /// Calculate vision range for a player based on vision attribute and position
    /// Open-football style: mental.vision * role_multiplier
    /// Returns distance in meters
    ///
    /// FIX_2601/0113: Relaxed multipliers to achieve ~10% Long Pass Share
    pub(crate) fn calculate_vision_range(&self, player_idx: usize) -> f32 {
        let vision = self.get_player_vision(player_idx);
        let position = self.get_match_player(player_idx).position;

        // Role-based multiplier (open-football style)
        let multiplier = if position.is_forward() {
            15.0 // Forwards: focused forward vision
        } else if position.is_midfielder() {
            20.0 // Midfielders: widest vision range
        } else if position.is_defender() {
            15.0 // Defenders: focused but safer
        } else {
            17.5 // Default (GK, etc.)
        };

        // FIX: Vision is on 0-100 scale, not 1-20
        // vision (0-100) * multiplier * 0.00675 = range in meters
        // Target: ~10% Long Pass (30m+) for balanced teams
        // e.g., vision 60 * 20 * 0.00675 = 8.1, then +26.5 base = 34.6m (midfielder)
        // e.g., vision 60 * 15 * 0.00675 = 6.075, then +26.5 base = 32.6m (forward/defender)
        26.5 + vision * multiplier * 0.00675
    }

    /// Check if a target is within passer's vision range
    pub(crate) fn is_within_vision_range(&self, passer_idx: usize, target_idx: usize) -> bool {
        let vision_range = self.calculate_vision_range(passer_idx);
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let target_pos = self.get_player_position_by_index(target_idx);
        let distance = passer_pos.distance_to_m(&target_pos);
        distance <= vision_range
    }

    // ===========================================
    // FIX_2601/0115: Lane-Clear Status Check
    // ===========================================

    /// Calculate lane clear status between two positions
    /// Uses our interceptor model and maps to discrete status
    pub(crate) fn calculate_lane_status(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> LaneClearStatus {
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        // Convert to meters
        let passer_pos_m = passer_pos.to_meters();
        let receiver_pos_m = receiver_pos.to_meters();

        // Get all player positions and speeds
        let player_positions_m: Vec<(f32, f32)> =
            self.player_positions.iter().map(|p| p.to_meters()).collect();

        let player_speeds: Vec<f32> = (0..22)
            .map(|idx| {
                let pace = self.get_player_pace(idx);
                4.0 + (pace / 20.0) * 4.0 // pace (1-20) -> speed (4-8 m/s)
            })
            .collect();

        // Calculate pass speed based on distance
        let pass_distance_m = body_blocking::distance(passer_pos_m, receiver_pos_m);
        let pass_speed = 15.0 + (pass_distance_m / 50.0) * 10.0; // 15-25 m/s

        // Find interceptors
        let interceptors = find_interceptors(
            passer_pos_m,
            receiver_pos_m,
            pass_speed,
            &player_positions_m,
            &player_speeds,
            opponent_range,
        );

        if interceptors.is_empty() {
            return LaneClearStatus::Clear;
        }

        // Get highest interception probability
        let max_intercept_prob = interceptors
            .iter()
            .map(|i| i.intercept_probability)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        if max_intercept_prob > 0.7 {
            LaneClearStatus::Blocked
        } else if max_intercept_prob < 0.1 {
            LaneClearStatus::Clear
        } else {
            LaneClearStatus::Contested {
                risk: max_intercept_prob,
            }
        }
    }

    // ===========================================
    // FIX_2601/0115: Pass Failure Resolution
    // ===========================================

    /// Build context for pass failure resolution
    pub(crate) fn build_pass_failure_context(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> PassFailureContext {
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        let passer_pos_m = passer_pos.to_meters();
        let receiver_pos_m = receiver_pos.to_meters();

        // Get all player positions and speeds
        let player_positions_m: Vec<(f32, f32)> =
            self.player_positions.iter().map(|p| p.to_meters()).collect();

        let player_speeds: Vec<f32> = (0..22)
            .map(|idx| {
                let pace = self.get_player_pace(idx);
                4.0 + (pace / 20.0) * 4.0
            })
            .collect();

        // Calculate pass speed
        let pass_distance_m = body_blocking::distance(passer_pos_m, receiver_pos_m);
        let pass_speed = 15.0 + (pass_distance_m / 50.0) * 10.0;

        // Find interceptors
        let interceptors = find_interceptors(
            passer_pos_m,
            receiver_pos_m,
            pass_speed,
            &player_positions_m,
            &player_speeds,
            opponent_range.clone(),
        );

        // Get best interceptor
        let (max_intercept_prob, best_interceptor_idx, intercept_position_m) =
            if let Some(best) = interceptors.first() {
                (
                    best.intercept_probability,
                    Some(best.player_idx),
                    Some(best.intercept_point),
                )
            } else {
                (0.0, None, None)
            };

        // Calculate receiver pressure
        let opponent_positions_m: Vec<(f32, f32)> = opponent_range
            .clone()
            .filter_map(|idx| player_positions_m.get(idx).copied())
            .collect();
        let receiver_pressure =
            Self::calculate_pressure_factor_at(receiver_pos_m, &opponent_positions_m, 8.0);

        // Calculate passer pressure
        let passer_pressure =
            Self::calculate_pressure_factor_at(passer_pos_m, &opponent_positions_m, 10.0);

        // Find closest opponent to passer (pressure source)
        // FIX_2601/0115: Position-neutral tie-breaker (replaces Y-bias from 0110)
        let passer_pressure_source_idx = opponent_range
            .map(|idx| {
                let opp_pos = self.get_player_position_by_index(idx).to_meters();
                let dist = body_blocking::distance(passer_pos_m, opp_pos);
                (idx, dist, opp_pos) // Include full position for tie-breaking
            })
            .min_by(|a, b| {
                match a.1.partial_cmp(&b.1) {
                    Some(std::cmp::Ordering::Equal) | None => {
                        // FIX_2601/0115: Deterministic hash tie-breaker (no position bias)
                        super::deterministic_tie_hash(a.0, a.2, b.0, b.2)
                    }
                    Some(ord) => ord,
                }
            })
            .filter(|(_, dist, _)| *dist < 10.0)
            .map(|(idx, _, _)| idx);

        // Get receiver's first touch
        let receiver_first_touch = self.get_player_first_touch(receiver_idx);

        PassFailureContext {
            max_intercept_prob,
            best_interceptor_idx,
            intercept_position_m,
            receiver_pressure,
            receiver_first_touch,
            passer_pressure_source_idx,
            passer_pressure,
        }
    }

    /// Resolve pass outcome with detailed failure branching
    /// FIX_2601/0115: Instead of binary success/fail, returns specific failure type
    pub(crate) fn resolve_detailed_pass_outcome(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
        base_success_rate: f32,
        rng: &mut impl rand::Rng,
    ) -> DetailedPassOutcome {
        let roll: f32 = rng.gen();

        // Success case
        if roll < base_success_rate {
            return DetailedPassOutcome::Success {
                receiver_idx,
            };
        }

        // Failure - determine type
        let ctx = self.build_pass_failure_context(passer_idx, receiver_idx, is_home);

        // Calculate relative probabilities for each failure type
        // Higher intercept prob -> more likely interception
        // Higher passer pressure -> more likely pressure forced
        // Lower first touch -> more likely miscontrol
        let intercept_weight = ctx.max_intercept_prob;
        let pressure_weight = ctx.passer_pressure * 0.4;
        let miscontrol_weight = (1.0 - ctx.receiver_first_touch / 20.0) * 0.3;
        let out_of_bounds_weight = 0.1; // Base chance for going out

        let total_weight = intercept_weight + pressure_weight + miscontrol_weight + out_of_bounds_weight;

        // Normalize and roll
        let failure_roll: f32 = rng.gen();
        let intercept_threshold = intercept_weight / total_weight;
        let pressure_threshold = intercept_threshold + pressure_weight / total_weight;
        let miscontrol_threshold = pressure_threshold + miscontrol_weight / total_weight;

        let failure = if failure_roll < intercept_threshold {
            // Interception
            if let (Some(idx), Some(pos)) = (ctx.best_interceptor_idx, ctx.intercept_position_m) {
                PassFailureOutcome::Intercepted {
                    interceptor_idx: idx,
                    interception_position_m: pos,
                }
            } else {
                // Fallback: nearest opponent
                let opponent_range = if is_home { 11..22 } else { 0..11 };
                let fallback_idx = opponent_range.start;
                let fallback_pos = self.get_player_position_by_index(fallback_idx).to_meters();
                PassFailureOutcome::Intercepted {
                    interceptor_idx: fallback_idx,
                    interception_position_m: fallback_pos,
                }
            }
        } else if failure_roll < pressure_threshold {
            // Pressure forced turnover
            if let Some(idx) = ctx.passer_pressure_source_idx {
                PassFailureOutcome::PressureForced {
                    pressure_source_idx: idx,
                }
            } else {
                // Fallback to interception
                let opponent_range = if is_home { 11..22 } else { 0..11 };
                let fallback_idx = opponent_range.start;
                PassFailureOutcome::PressureForced {
                    pressure_source_idx: fallback_idx,
                }
            }
        } else if failure_roll < miscontrol_threshold {
            // Miscontrol by receiver
            let receiver_pos = self.get_player_position_by_index(receiver_idx).to_meters();
            // Loose ball position offset by 2-5 meters in random direction
            let angle: f32 = rng.gen::<f32>() * std::f32::consts::PI * 2.0;
            let dist: f32 = 2.0 + rng.gen::<f32>() * 3.0;
            let loose_pos = (
                receiver_pos.0 + angle.cos() * dist,
                receiver_pos.1 + angle.sin() * dist,
            );
            PassFailureOutcome::Miscontrolled {
                receiver_idx,
                loose_ball_position_m: loose_pos,
            }
        } else {
            // Out of bounds
            let receiver_pos = self.get_player_position_by_index(receiver_idx).to_meters();
            // Exit near the edge
            let exit_pos = (
                receiver_pos.0.clamp(0.0, field::LENGTH_M),
                if receiver_pos.1 < physics_constants::field::CENTER_Y {
                    0.0
                } else {
                    physics_constants::field::WIDTH_M
                },
            );
            PassFailureOutcome::OutOfBounds {
                exit_position_m: exit_pos,
            }
        };

        DetailedPassOutcome::Failed(failure)
    }

    // ===========================================
    // 6-Factor Pass System (Open Football Style)
    // ===========================================

    /// Update possession tracking (decision-tick granularity).
    pub(crate) fn update_possession_clock(&mut self) {
        let current_owner = self.ball.current_owner;
        if current_owner != self.possession_owner_idx {
            self.possession_owner_idx = current_owner;
            self.possession_owner_since_tick = self.current_tick;
        }
    }

    /// Current possession duration in ticks for a given player.
    pub(crate) fn possession_duration_ticks(&self, player_idx: usize) -> u64 {
        if self.possession_owner_idx == Some(player_idx) {
            self.current_tick.saturating_sub(self.possession_owner_since_tick)
        } else {
            0
        }
    }

    /// Current possession duration in milliseconds for a given player.
    pub(crate) fn possession_duration_ms(&self, player_idx: usize) -> u64 {
        self.possession_duration_ticks(player_idx) * 250
    }

    /// 1. Distance Factor (15%) - Pass distance evaluation
    pub(crate) fn calculate_pass_distance_factor(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
    ) -> f32 {
        use physics_constants::pass;

        // FIX_2601: Use Coord10::distance_to_m() directly
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let distance_m = passer_pos.distance_to_m(&receiver_pos);

        // Optimal distance: 10-25m
        if distance_m < pass::VERY_SHORT_M {
            0.7 + (distance_m / pass::VERY_SHORT_M) * 0.3 // Too short
        } else if distance_m <= pass::SHORT_M {
            1.0 // Optimal
        } else if distance_m <= pass::OPTIMAL_MAX_M {
            1.0 - (distance_m - pass::SHORT_M) / (pass::OPTIMAL_MAX_M - pass::SHORT_M) * 0.3
        } else {
            0.4 // Too long
        }
    }

    /// 2. Safety Factor (15%) - Interception risk
    /// P7 Body Blocking: Uses find_interceptors for realistic interception calculation
    /// FIX_2601/0112: Now includes trajectory-based risk analysis (GRF integration)
    pub(crate) fn calculate_pass_safety_factor(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> f32 {
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        // FIX_2601: Coord10 → meters conversion
        let passer_pos_m = passer_pos.to_meters();
        let receiver_pos_m = receiver_pos.to_meters();

        // Get all player positions in meters (FIX_2601: Coord10::to_meters())
        let player_positions_m: Vec<(f32, f32)> =
            self.player_positions.iter().map(|p| p.to_meters()).collect();

        // Get player speeds (approximate from pace stat, default 5.0 m/s)
        let player_speeds: Vec<f32> = (0..22)
            .map(|idx| {
                let pace = self.get_player_pace(idx);
                // Convert pace (1-20) to speed (4-8 m/s)
                4.0 + (pace / 20.0) * 4.0
            })
            .collect();

        // Calculate pass speed based on distance
        let pass_distance_m = body_blocking::distance(passer_pos_m, receiver_pos_m);
        let pass_speed = 15.0 + (pass_distance_m / 50.0) * 10.0; // 15-25 m/s based on distance

        let opponent_range_for_interceptors = opponent_range.clone();

        // Use find_interceptors for realistic interception calculation
        let interceptors = find_interceptors(
            passer_pos_m,
            receiver_pos_m,
            pass_speed,
            &player_positions_m,
            &player_speeds,
            opponent_range_for_interceptors,
        );

        // Calculate safety factor based on interceptor count and probabilities
        let opponent_positions_m: Vec<(f32, f32)> = opponent_range
            .clone()
            .filter_map(|idx| player_positions_m.get(idx).copied())
            .collect();

        let passer_pressure =
            Self::calculate_pressure_factor_at(passer_pos_m, &opponent_positions_m, 10.0);
        let receiver_pressure =
            Self::calculate_pressure_factor_at(receiver_pos_m, &opponent_positions_m, 8.0);
        let angle_factor = self.calculate_pass_angle_factor(passer_idx, passer_pos_m, receiver_pos_m);

        let mut safety = if interceptors.is_empty() {
            1.0 // Completely safe
        } else {
            // Consider highest probability interceptor
            let best_intercept_prob = interceptors[0].intercept_probability;

            // Also consider number of potential interceptors
            let count_penalty = match interceptors.len() {
                1 => 0.0,
                2 => 0.1,
                _ => 0.2,
            };

            // Safety = 1.0 - intercept_probability - count_penalty
            (1.0 - best_intercept_prob - count_penalty).clamp(0.2, 1.0)
        };

        // FIX_2601/0112: Apply trajectory-based risk (GRF integration)
        // Trajectory risk analyzes the entire pass path, not just the endpoints
        let trajectory_risk = calculate_trajectory_risk(
            passer_pos_m,
            receiver_pos_m,
            &opponent_positions_m,
        );
        // Blend trajectory risk with interceptor-based safety (30% weight for trajectory)
        safety = (safety * 0.7 + (1.0 - trajectory_risk) * 0.3).clamp(0.2, 1.0);

        // Passer/receiver pressure + pass angle adjustment (open-football style)
        let pressure_penalty = passer_pressure * 0.2 + receiver_pressure * 0.15;
        safety = (safety - pressure_penalty).clamp(0.0, 1.0);
        safety *= angle_factor;

        // High-pass trap penalty proxy: long passes to pressured receivers are riskier
        let trap_penalty = if pass_distance_m > 25.0 { receiver_pressure * 0.12 } else { 0.0 };
        (safety - trap_penalty).clamp(0.2, 1.0)
    }

    /// Pressure factor based on opponents within radius (0.0 ~ 1.0)
    fn calculate_pressure_factor_at(
        position: (f32, f32),
        opponent_positions: &[(f32, f32)],
        radius_m: f32,
    ) -> f32 {
        let mut nearby = 0;
        for opp in opponent_positions {
            let dx = opp.0 - position.0;
            let dy = opp.1 - position.1;
            if dx * dx + dy * dy <= radius_m * radius_m {
                nearby += 1;
            }
        }

        match nearby {
            0 => 0.0,
            1 => 0.3,
            2 => 0.6,
            3 => 0.85,
            _ => 1.0,
        }
    }

    /// Pass angle factor based on passer movement direction
    fn calculate_pass_angle_factor(
        &self,
        passer_idx: usize,
        passer_pos_m: (f32, f32),
        receiver_pos_m: (f32, f32),
    ) -> f32 {
        let (vx, vy) = self.player_velocities[passer_idx];
        let vel_len_sq = vx * vx + vy * vy;
        let dx = receiver_pos_m.0 - passer_pos_m.0;
        let dy = receiver_pos_m.1 - passer_pos_m.1;
        let pass_len_sq = dx * dx + dy * dy;

        if vel_len_sq < 0.01 || pass_len_sq < 0.01 {
            return 1.0;
        }

        let vel_len = vel_len_sq.sqrt();
        let pass_len = pass_len_sq.sqrt();
        let dot = (vx * dx + vy * dy) / (vel_len * pass_len);

        if dot > 0.5 {
            1.0
        } else if dot > -0.5 {
            0.85
        } else {
            0.7
        }
    }

    /// 3. Readiness Factor (10%) - Receiver readiness
    pub(crate) fn calculate_pass_readiness_factor(
        &self,
        receiver_idx: usize,
        is_home: bool,
    ) -> f32 {
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        // Check if receiver is being marked
        let mut closest_marker_dist = f32::MAX;

        for opp_idx in opponent_range {
            // FIX_2601: Use Coord10::distance_to_m()
            let opp_pos = self.get_player_position_by_index(opp_idx);
            let dist = receiver_pos.distance_to_m(&opp_pos);
            if dist < closest_marker_dist {
                closest_marker_dist = dist;
            }
        }

        // Readiness based on marker distance
        use physics_constants::action_thresholds;
        if closest_marker_dist > action_thresholds::MARKER_FREE_M {
            1.0 // Completely free
        } else if closest_marker_dist > action_thresholds::MARKER_SPACE_M {
            0.8 // Some space
        } else if closest_marker_dist > action_thresholds::MARKER_TIGHT_M {
            0.5 // Being marked
        } else {
            0.2 // Tight marking
        }
    }

    /// Pass progression delta (normalized length, team view)
    /// FIX_2601/0117: Made pub(crate) for debug access
    pub(crate) fn calculate_pass_progression_delta(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> f32 {
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);

        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);

        // 좌표계: pos.1 = length (골 방향)
        // attacks_right=true advances when receiver length > passer length
        // attacks_right=false advances when receiver length < passer length
        if attacks_right {
            coordinates::norm_length(receiver_pos.to_normalized_legacy())
                - coordinates::norm_length(passer_pos.to_normalized_legacy())
        } else {
            coordinates::norm_length(passer_pos.to_normalized_legacy())
                - coordinates::norm_length(receiver_pos.to_normalized_legacy())
        }
    }

    /// 4. Progression Factor (40%) - Forward pass preference
    /// FIX_2601/0105: Increased score differentiation to strongly favor forward passes
    pub(crate) fn calculate_pass_progression_factor(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> f32 {
        let progression =
            self.calculate_pass_progression_delta(passer_idx, receiver_idx, is_home);

        // FIX_2601/1128: INVERTED pass progression scoring for forward_pass_rate 22% target
        // Previous: forward=0.9, back=0.35 (2.6:1 ratio forward) → forward_rate=48%
        // New: forward=0.4, back=0.85 (2.1:1 ratio backward) → target forward_rate=22%
        if progression > 0.15 {
            0.40 // Big progression - penalize heavily
        } else if progression > 0.05 {
            0.50 // Moderate progression - penalize
        } else if progression > -0.05 {
            0.70 // Lateral pass - favor
        } else {
            0.85 // Back pass - strongly favor
        }
    }

    /// 5. Space Factor (10%) - Space around receiver
    pub(crate) fn calculate_pass_space_factor(&self, receiver_idx: usize, is_home: bool) -> f32 {
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        // Count opponents within 10m radius
        let mut opponents_nearby = 0;

        for opp_idx in opponent_range {
            // FIX_2601: Use Coord10::distance_to_m()
            let opp_pos = self.get_player_position_by_index(opp_idx);
            let dist = receiver_pos.distance_to_m(&opp_pos);
            if dist < 10.0 {
                opponents_nearby += 1;
            }
        }

        match opponents_nearby {
            0 => 1.0,
            1 => 0.7,
            2 => 0.4,
            _ => 0.2,
        }
    }

    /// 6. Tactical Factor (10%) - Tactical advantage
    pub(crate) fn calculate_pass_tactical_factor(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> f32 {
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let receiver_pos_m = receiver_pos.to_meters();
        let passer_pos_m = self.get_player_position_by_index(passer_idx).to_meters();

        // FIX_2601/0105: Use attacks_right for direction (considers halftime swap)
        let attacks_right = self.attacks_right(is_home);
        // Distance to goal
        let distance_to_goal =
            coordinates::distance_to_goal_m(receiver_pos.to_normalized_legacy(), attacks_right);

        // Danger zone (near penalty box)
        let in_danger_zone = distance_to_goal < 25.0;

        // Good shooting angle
        let good_angle = self.has_good_shooting_angle(receiver_idx);

        let mut score = if in_danger_zone && good_angle {
            1.0
        } else if in_danger_zone {
            0.7
        } else if distance_to_goal < 40.0 {
            0.5
        } else {
            0.3
        };

        let numerical_superiority =
            self.calculate_numerical_superiority(receiver_pos_m, is_home, 20.0);
        score += numerical_superiority * 0.2;

        if self.is_switch_of_play(passer_pos_m, receiver_pos_m) {
            score += 0.2;
        }
        if self.breaks_defensive_lines(passer_pos_m, receiver_pos_m, is_home) {
            score += 0.3;
        }

        score += self.position_specific_pass_bonus(receiver_idx, receiver_pos, is_home);

        score.clamp(0.0, 1.0)
    }

    fn calculate_numerical_superiority(
        &self,
        position_m: (f32, f32),
        is_home: bool,
        radius_m: f32,
    ) -> f32 {
        let teammate_range = if is_home { 0..11 } else { 11..22 };
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        let mut teammates = 0;
        let mut opponents = 0;

        for idx in teammate_range {
            let pos = self.get_player_position_by_index(idx).to_meters();
            let dx = pos.0 - position_m.0;
            let dy = pos.1 - position_m.1;
            if dx * dx + dy * dy <= radius_m * radius_m {
                teammates += 1;
            }
        }

        for idx in opponent_range {
            let pos = self.get_player_position_by_index(idx).to_meters();
            let dx = pos.0 - position_m.0;
            let dy = pos.1 - position_m.1;
            if dx * dx + dy * dy <= radius_m * radius_m {
                opponents += 1;
            }
        }

        let diff = teammates as i32 - opponents as i32;
        match diff {
            d if d >= 2 => 1.0,
            1 => 0.7,
            0 => 0.5,
            -1 => 0.3,
            _ => 0.0,
        }
    }

    fn is_switch_of_play(&self, passer_pos_m: (f32, f32), receiver_pos_m: (f32, f32)) -> bool {
        let width_diff = (passer_pos_m.1 - receiver_pos_m.1).abs();
        width_diff > field::WIDTH_M * 0.4
    }

    /// FIX_2601/1128: Check if there's a recent pass between two players (bidirectional)
    /// Returns true if either (from → to) or (to → from) exists in recent_pass_pairs
    /// This allows for complete reciprocal patterns (A↔B)
    /// FIX_2601/1130: Reduced window from 40 to 5 to focus on immediate return passes
    pub(super) fn has_recent_pass_from(&self, from_idx: usize, to_idx: usize) -> bool {
        let from = from_idx as u8;
        let to = to_idx as u8;
        // FIX_2601/1130: Check only the most recent 5 passes for immediate reciprocity
        // This encourages quick return passes rather than distant historical patterns
        self.recent_pass_pairs.iter().rev().take(5).any(|&(p, r)| {
            (p == from && r == to) || (p == to && r == from)
        })
    }

    /// FIX_2601/1128: Check if target recently received a pass (for diversity bonus)
    /// Returns true if target appears as receiver in recent_pass_pairs (last 5 passes)
    pub(super) fn is_recent_pass_receiver(&self, target_idx: usize) -> bool {
        let target = target_idx as u8;
        // Check only the most recent 5 passes for diversity
        self.recent_pass_pairs
            .iter()
            .rev()
            .take(5)
            .any(|&(_, r)| r == target)
    }

    fn breaks_defensive_lines(
        &self,
        passer_pos_m: (f32, f32),
        receiver_pos_m: (f32, f32),
        is_home: bool,
    ) -> bool {
        let opponent_range = if is_home { 11..22 } else { 0..11 };
        let min_x = passer_pos_m.0.min(receiver_pos_m.0);
        let max_x = passer_pos_m.0.max(receiver_pos_m.0);
        let mut opponents_between = 0;

        for idx in opponent_range {
            let opp_pos = self.get_player_position_by_index(idx).to_meters();
            if opp_pos.0 <= min_x || opp_pos.0 >= max_x {
                continue;
            }
            let dist_to_line =
                body_blocking::point_to_line_distance(opp_pos, passer_pos_m, receiver_pos_m);
            if dist_to_line < 3.0 {
                opponents_between += 1;
            }
        }

        opponents_between >= 2
    }

    fn position_specific_pass_bonus(
        &self,
        receiver_idx: usize,
        receiver_pos: Coord10,
        is_home: bool,
    ) -> f32 {
        let pos = self.get_match_player(receiver_idx).position;
        let receiver_pos_norm = receiver_pos.to_normalized_legacy();
        let receiver_pos_m = receiver_pos.to_meters();
        let attacks_right = self.attacks_right(is_home);

        if pos.is_forward() {
            if coordinates::is_in_attacking_third(receiver_pos_norm, attacks_right) {
                return 0.15;
            }
            return 0.05;
        }

        if matches!(pos, Position::CAM | Position::CM | Position::CDM) {
            return self.calculate_pass_space_factor(receiver_idx, is_home) * 0.1;
        }

        if matches!(
            pos,
            Position::LM | Position::RM | Position::LW | Position::RW | Position::LWB | Position::RWB
        ) {
            let is_wide =
                receiver_pos_m.1 < field::WIDTH_M * 0.2 || receiver_pos_m.1 > field::WIDTH_M * 0.8;
            let space = self.calculate_pass_space_factor(receiver_idx, is_home);
            if is_wide && space > 0.7 {
                return 0.1;
            }
        }

        0.0
    }

    /// 6-Factor combined pass score
    ///
    /// FIX_2601/0106 P3: Now applies BuildupPhase-based weight adjustments
    /// - OwnThird: Higher safety weight (1.5x), lower progression (0.5x)
    /// - MiddleThird: Balanced weights (1.0x)
    /// - FinalThird: Lower safety (0.7x), higher progression (1.5x)
    ///
    /// FIX_2601/0115: Added role-specific bonuses and lane-clear status
    pub(crate) fn calculate_pass_score_6factor(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> f32 {
        let distance = self.calculate_pass_distance_factor(passer_idx, receiver_idx);
        let safety = self.calculate_pass_safety_factor(passer_idx, receiver_idx, is_home);
        let readiness = self.calculate_pass_readiness_factor(receiver_idx, is_home);
        let progression = self.calculate_pass_progression_factor(passer_idx, receiver_idx, is_home);
        let progression_delta =
            self.calculate_pass_progression_delta(passer_idx, receiver_idx, is_home);
        // FIX_2601/1128: Increased circulation values to encourage backward/lateral passes
        // Old (0110): forward=0.0, sideways=0.5, backward=0.6
        // New (1128): forward=0.0, sideways=0.6, backward=0.75
        // This helps improve reciprocity (A→B→A) and density (varied pass targets)
        let circulation = if progression_delta > 0.05 {
            0.0
        } else if progression_delta >= -0.05 {
            0.6  // sideways: increased from 0.5
        } else {
            0.75 // backward: increased from 0.6
        };
        let space = self.calculate_pass_space_factor(receiver_idx, is_home);
        let tactical = self.calculate_pass_tactical_factor(passer_idx, receiver_idx, is_home);

        // FIX_2601/0115: Get lane-clear status
        let lane_status = self.calculate_lane_status(passer_idx, receiver_idx, is_home);

        // FIX_2601/0106 P3: Determine buildup phase from ball position
        let ball_pos_m = self.ball.position.to_meters();
        let attacks_right = self.attacks_right(is_home);
        let phase = BuildupPhase::from_ball_position(ball_pos_m.0, attacks_right);

        let instructions = if is_home { &self.home_instructions } else { &self.away_instructions };

        // FIX_2601/0115: Get role-specific bonuses for passer
        let passer_position = self.get_match_player(passer_idx).position;
        let role_bonus = RolePassBonus::for_position(&passer_position);

        // FIX_2601/0110: Reduced Short style circ_mul from 1.3 to 1.0 to prevent
        // backward passes from being too attractive relative to forward passes.
        let (style_prog_mul, style_circ_mul) = match instructions.build_up_style {
            BuildUpStyle::Short => (0.8, 1.0),
            BuildUpStyle::Mixed => (1.0, 1.0),
            BuildUpStyle::Direct => (1.3, 0.6),
        };

        let (base_tempo_prog, base_tempo_circ) = match instructions.team_tempo {
            TeamTempo::VerySlow => (0.85, 1.2),
            TeamTempo::Slow => (0.9, 1.1),
            TeamTempo::Normal => (1.0, 1.0),
            TeamTempo::Fast => (1.1, 0.9),
            TeamTempo::VeryFast => (1.2, 0.8),
        };

        // DPER Framework: Apply experimental tempo bias
        // tempo_bias: -1.0 (slower) to +1.0 (faster)
        let exp_tempo_bias = self.exp_tempo_bias();
        let tempo_prog_mul = base_tempo_prog * (1.0 + exp_tempo_bias * 0.2);
        let tempo_circ_mul = base_tempo_circ * (1.0 - exp_tempo_bias * 0.2);

        // Base weights
        // FIX_2601/0113: Adjusted for realistic long pass distribution
        // FIX_2601/1128: Further adjusted to reduce forward pass bias (38% → 22% target)
        // Progression: 30% → 20% (further reduce over-progression bias)
        // Circulation: 13% → 20% (boost backward/lateral pass value)
        const BASE_DISTANCE_W: f32 = 0.15;
        const BASE_SAFETY_W: f32 = 0.22;
        const BASE_READINESS_W: f32 = 0.10;
        const BASE_PROGRESSION_W: f32 = 0.20;
        const BASE_CIRCULATION_W: f32 = 0.20;
        const BASE_SPACE_W: f32 = 0.10;
        const BASE_TACTICAL_W: f32 = 0.10;

        // FIX_2601/0115: Apply role-specific weight adjustments
        let safety_w = (BASE_SAFETY_W * phase.safety_multiplier() * role_bonus.safety_multiplier)
            .clamp(0.05, 0.40);
        // FIX_2601/0119: Kickoff pass dampening - TESTED AND REJECTED
        // Tested: boost forward pass weight by 15%, reduce backward by 10% for first 20 ticks
        // Result: Caused regression (0.89 → 0.65 ratio) - increased turnover risk hurt Home more
        // Keeping baseline weights without kickoff phase adjustment

        // FIX_2601/1128: Apply AttackSubPhase multipliers
        // These are the key weights that shift balance between forward and backward passes
        // Circulation phase: forward=0.25, backward=2.0 (strongly favor circulation)
        // Progression phase: forward=1.1, backward=0.9 (slightly favor forward)
        let phase_state = if is_home { &self.home_phase_state } else { &self.away_phase_state };
        let subphase_forward_mul = phase_state.forward_pass_weight();
        let subphase_circ_mul = phase_state.circulation_pass_weight();


        let progression_w = (BASE_PROGRESSION_W
            * phase.progression_multiplier()
            * role_bonus.progression_multiplier
            * style_prog_mul
            * tempo_prog_mul
            * subphase_forward_mul)  // FIX_2601/1128: Apply sub-phase forward weight
            .clamp(0.05, 0.60);  // Lower floor to allow stronger circulation bias
        let circulation_w = BASE_CIRCULATION_W * style_circ_mul * tempo_circ_mul * subphase_circ_mul;

        // Renormalize weights to sum to 1.0
        let total_w = BASE_DISTANCE_W
            + safety_w
            + BASE_READINESS_W
            + progression_w
            + circulation_w
            + BASE_SPACE_W
            + BASE_TACTICAL_W;
        // FIX_2601/0106: Guard against division by zero (defensive coding)
        let norm = if total_w > 0.0 { 1.0 / total_w } else { 1.0 };

        let mut score = distance * BASE_DISTANCE_W * norm
            + safety * safety_w * norm
            + readiness * BASE_READINESS_W * norm
            + progression * progression_w * norm
            + circulation * circulation_w * norm
            + space * BASE_SPACE_W * norm
            + tactical * BASE_TACTICAL_W * norm;

        // FIX_2601/0115: Apply lane-clear penalty
        score -= lane_status.score_penalty();

        // OpenFootManager zone transition bias (team view)
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let passer_pos_m = passer_pos.to_meters();
        let receiver_pos_m = receiver_pos.to_meters();
        // FIX_2601: Use proper attack direction (accounts for halftime)
        let attacks_right = self.attacks_right(is_home);
        let from_zone = zone_of_position(passer_pos_m.0, passer_pos_m.1, attacks_right);
        let to_zone = zone_of_position(receiver_pos_m.0, receiver_pos_m.1, attacks_right);
        let style = ZoneTransitionStyle::from_instructions(instructions);
        let zone_factor = super::zone_transition::pass_factor(style, from_zone, to_zone);
        score = (score * zone_factor).clamp(0.0, 1.0);

        // FIX_2601/0115: Apply role-specific bonuses
        // Check if pass breaks defensive line
        if self.breaks_defensive_lines(passer_pos_m, receiver_pos_m, is_home) {
            score += role_bonus.line_break_bonus;
        }

        // Check if receiver is in good shooting position
        let receiver_distance_to_goal =
            coordinates::distance_to_goal_m(receiver_pos.to_normalized_legacy(), attacks_right);
        let receiver_has_good_angle = self.has_good_shooting_angle(receiver_idx);
        if receiver_distance_to_goal < 25.0 && receiver_has_good_angle {
            score += role_bonus.shot_setup_bonus;
        }

        // Check if this is a spread/switch pass
        if self.is_switch_of_play(passer_pos_m, receiver_pos_m) {
            score += role_bonus.spread_bonus;
        }

        // FIX_2601/0123: Reciprocity - no bonus or penalty (neutral)
        // The reciprocity injection in tick_based.rs already handles this
        // Setting bonus/penalty here causes imbalance
        const RECIPROCITY_BONUS: f32 = 0.0;
        if self.has_recent_pass_from(receiver_idx, passer_idx) {
            score += RECIPROCITY_BONUS;
        }

        // FIX_2601/1128: Direct sub-phase based pass direction adjustment
        // This is the PRIMARY mechanism for encouraging backward passes in Circulation mode
        // Applied after all other bonuses to have maximum effect
        use crate::engine::team_phase::AttackSubPhase;
        let is_backward = progression_delta < -0.05;
        let is_forward = progression_delta > 0.05;

        // FIX_2601/1128: Very strong direction bonuses for forward_pass_rate 22% target
        match phase_state.attack_sub_phase {
            AttackSubPhase::Circulation => {
                // In Circulation: VERY strongly favor backward/lateral, heavily penalize forward
                if is_backward {
                    score += 0.50; // Much bigger bonus for backward passes (was 0.35)
                } else if is_forward {
                    score -= 0.40; // Much stronger penalty for forward passes (was 0.25)
                }
            }
            AttackSubPhase::Progression => {
                // In Progression: neutral (remove forward preference to reduce forward_pass_rate)
                // Previously favored forward, now neutral
                if is_backward {
                    score += 0.05; // Small bonus for backward even in Progression
                }
            }
            AttackSubPhase::Finalization => {
                // In Finalization: no direction preference
            }
        }

        // FIX_2601/0123: Diversity bonus REMOVED to reduce density
        // Previously encouraged passing to different teammates (increased density)
        // Now set to 0.0 to help reduce density (0.62→0.45-0.50 target)
        const DIVERSITY_BONUS: f32 = 0.0;
        if !self.is_recent_pass_receiver(receiver_idx) {
            score += DIVERSITY_BONUS;
        }

        // FIX_2601/1128: Safe option floor - backward/lateral passes get minimum score
        // Ensures safe options remain viable even when forward options are preferred
        const SAFE_OPTION_FLOOR: f32 = 0.35;
        if progression_delta < 0.0 && score < SAFE_OPTION_FLOOR {
            score = SAFE_OPTION_FLOOR;
        }

        score = score.clamp(0.0, 1.0);

        // B1: Hero Gravity - Pass Priority Boost
        // "When in doubt, pass to the hero" - 1.3x multiplier for hero
        if self.is_user_player(receiver_idx, is_home) {
            score *= physics_constants::hero_gravity::PASS_PRIORITY_MULTIPLIER;
        }

        score
    }

    /// Calculate pass score with full component logging
    /// FIX_2601/0115: Returns score and log entry for analysis/replay
    #[allow(dead_code)]
    pub(crate) fn calculate_pass_score_with_log(
        &self,
        passer_idx: usize,
        receiver_idx: usize,
        is_home: bool,
    ) -> (f32, PassEvaluationLog) {
        // Calculate all components
        let distance = self.calculate_pass_distance_factor(passer_idx, receiver_idx);
        let safety = self.calculate_pass_safety_factor(passer_idx, receiver_idx, is_home);
        let readiness = self.calculate_pass_readiness_factor(receiver_idx, is_home);
        let progression = self.calculate_pass_progression_factor(passer_idx, receiver_idx, is_home);
        let progression_delta =
            self.calculate_pass_progression_delta(passer_idx, receiver_idx, is_home);
        // FIX_2601/1128: Increased circulation values to encourage backward/lateral passes
        // Old (0110): forward=0.0, sideways=0.5, backward=0.6
        // New (1128): forward=0.0, sideways=0.6, backward=0.75
        // This helps improve reciprocity (A→B→A) and density (varied pass targets)
        let circulation = if progression_delta > 0.05 {
            0.0
        } else if progression_delta >= -0.05 {
            0.6  // sideways: increased from 0.5
        } else {
            0.75 // backward: increased from 0.6
        };
        let space = self.calculate_pass_space_factor(receiver_idx, is_home);
        let tactical = self.calculate_pass_tactical_factor(passer_idx, receiver_idx, is_home);
        let lane_status = self.calculate_lane_status(passer_idx, receiver_idx, is_home);

        // Get role bonus
        let passer_position = self.get_match_player(passer_idx).position;
        let role_bonus_struct = RolePassBonus::for_position(&passer_position);
        let passer_pos = self.get_player_position_by_index(passer_idx);
        let receiver_pos = self.get_player_position_by_index(receiver_idx);
        let passer_pos_m = passer_pos.to_meters();
        let receiver_pos_m = receiver_pos.to_meters();
        let attacks_right = self.attacks_right(is_home);

        // Calculate role-specific bonuses
        let mut role_bonus_total = 0.0;
        if self.breaks_defensive_lines(passer_pos_m, receiver_pos_m, is_home) {
            role_bonus_total += role_bonus_struct.line_break_bonus;
        }
        let receiver_distance_to_goal =
            coordinates::distance_to_goal_m(receiver_pos.to_normalized_legacy(), attacks_right);
        if receiver_distance_to_goal < 25.0 && self.has_good_shooting_angle(receiver_idx) {
            role_bonus_total += role_bonus_struct.shot_setup_bonus;
        }
        if self.is_switch_of_play(passer_pos_m, receiver_pos_m) {
            role_bonus_total += role_bonus_struct.spread_bonus;
        }

        // Build lane status string
        let lane_status_str = match lane_status {
            LaneClearStatus::Clear => "Clear".to_string(),
            LaneClearStatus::Contested { risk } => format!("Contested({:.2})", risk),
            LaneClearStatus::Blocked => "Blocked".to_string(),
        };

        // Create log entry
        let components = PassScoreComponents {
            distance,
            safety,
            readiness,
            progression,
            circulation,
            space,
            tactical,
            role_bonus: role_bonus_total,
            lane_status: lane_status_str,
        };

        // Calculate final score using the existing function
        let final_score = self.calculate_pass_score_6factor(passer_idx, receiver_idx, is_home);

        let log = PassEvaluationLog {
            tick: self.current_tick as u32,
            passer_id: passer_idx as u32,
            target_id: receiver_idx as u32,
            components,
            final_score,
            selected: false, // Will be set by caller
        };

        (final_score, log)
    }

    /// Tactical improvement threshold for pass target selection.
    pub(crate) fn pass_tactical_improvement_threshold(&self, passer_idx: usize) -> f32 {
        const BASE_THRESHOLD: f32 = 0.05;
        const MAX_HOLD_MS: f32 = 5000.0;

        let hold_ms = self.possession_duration_ms(passer_idx) as f32;
        let hold_norm = (hold_ms / MAX_HOLD_MS).clamp(0.0, 1.0);
        let long_possession_factor = hold_norm * hold_norm;

        (BASE_THRESHOLD * (1.0 - long_possession_factor)).clamp(0.0, BASE_THRESHOLD)
    }

    // =========================================================================
    // FIX_2601/0109: Switch of Play Target Selection (Open-Football Integration)
    // =========================================================================

    /// Find optimal switch of play target
    ///
    /// Open-Football integration: Switch of play should target:
    /// 1. Players on the opposite side of the field (>40% width distance)
    /// 2. Players with more space around them (fewer nearby opponents)
    ///
    /// Returns (player_idx, position, space_score) of best switch target
    pub(crate) fn find_switch_play_target(
        &self,
        passer_idx: usize,
        is_home: bool,
    ) -> Option<(usize, (f32, f32), f32)> {
        const BASE_MIN_SWITCH_WIDTH: f32 = 0.4; // Minimum 40% field width for switch
        const SPACE_RADIUS_M: f32 = 10.0; // Radius to count nearby opponents

        // DPER Framework: Apply experimental width bias
        // width_bias: -1.0 (narrower) to +1.0 (wider)
        let exp_width_bias = self.exp_width_bias();
        // Positive bias = lower threshold (easier to trigger wide play)
        // Negative bias = higher threshold (harder to trigger wide play)
        let min_switch_width = (BASE_MIN_SWITCH_WIDTH - exp_width_bias * 0.15).clamp(0.2, 0.6);

        let passer_pos = self.get_player_position_by_index(passer_idx);
        let passer_pos_norm = passer_pos.to_normalized_legacy();
        let passer_x = crate::engine::coordinates::norm_width(passer_pos_norm);

        let teammate_range = if is_home { 1..11 } else { 12..22 }; // Exclude GK
        let opponent_range = if is_home { 11..22 } else { 0..11 };

        // Collect opponents positions in meters for space calculation
        let opponent_positions_m: Vec<(f32, f32)> = opponent_range
            .clone()
            .map(|idx| self.get_player_position_by_index(idx).to_meters())
            .collect();

        // Find valid switch targets on opposite side
        let mut candidates: Vec<(usize, (f32, f32), f32)> = Vec::new();

        for teammate_idx in teammate_range {
            if teammate_idx == passer_idx {
                continue;
            }

            let teammate_pos = self.get_player_position_by_index(teammate_idx);
            let teammate_pos_norm = teammate_pos.to_normalized_legacy();
            let teammate_x = crate::engine::coordinates::norm_width(teammate_pos_norm);

            // Check if on opposite side (minimum width distance)
            let width_distance = (teammate_x - passer_x).abs();
            if width_distance < min_switch_width {
                continue;
            }

            // Calculate space around receiver
            let teammate_pos_m = teammate_pos.to_meters();
            let opponents_nearby = opponent_positions_m
                .iter()
                .filter(|opp| {
                    let dx = opp.0 - teammate_pos_m.0;
                    let dy = opp.1 - teammate_pos_m.1;
                    (dx * dx + dy * dy).sqrt() < SPACE_RADIUS_M
                })
                .count();

            // Space score: fewer opponents = higher score
            let space_score = match opponents_nearby {
                0 => 1.0,
                1 => 0.7,
                2 => 0.4,
                _ => 0.2,
            };

            // Combined score: prioritize width distance + space
            // DPER Framework: width_bias affects prioritization
            let width_weight = (0.6 + exp_width_bias * 0.2).clamp(0.3, 0.8);
            let space_weight = 1.0 - width_weight;
            let combined_score = width_distance * width_weight + space_score * space_weight;

            candidates.push((teammate_idx, teammate_pos_norm, combined_score));
        }

        // Return best candidate (highest combined score)
        candidates
            .into_iter()
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// 6-Factor based best pass target selection
    pub(crate) fn find_best_pass_target(&self, passer_idx: usize, is_home: bool) -> Option<usize> {
        let valid_targets = self.find_valid_pass_targets(passer_idx, is_home);

        if valid_targets.is_empty() {
            return None;
        }

        let tactical_threshold = self.pass_tactical_improvement_threshold(passer_idx);
        let passer_tactical = self.calculate_pass_tactical_factor(passer_idx, passer_idx, is_home);

        let mut best_target = None;
        let mut best_score = 0.0;
        let mut fallback_target = valid_targets[0];
        let mut fallback_score = 0.0;

        // FIX_2601/1128: Track best safe option separately for probabilistic injection
        let mut best_safe_target = None;
        let mut best_safe_score = 0.0;

        for &target_idx in &valid_targets {
            let score = self.calculate_pass_score_6factor(passer_idx, target_idx, is_home);
            if score > fallback_score {
                fallback_score = score;
                fallback_target = target_idx;
            }

            // FIX_2601/1128: Check if this is a backward/lateral pass (safe option)
            let progression_delta = self.calculate_pass_progression_delta(passer_idx, target_idx, is_home);
            let is_safe_option = progression_delta < 0.0;

            // FIX_2601/1128: Track best safe option
            if is_safe_option && score > best_safe_score {
                best_safe_score = score;
                best_safe_target = Some(target_idx);
            }

            // FIX_2601/0123: Removed reciprocity skip - we no longer want to encourage A→B→A patterns
            // Previously: let has_reciprocity = self.has_recent_pass_from(target_idx, passer_idx);
            // Now only safe options skip tactical threshold (not reciprocal passes)
            let skip_tactical = is_safe_option;

            if !skip_tactical {
                let target_tactical =
                    self.calculate_pass_tactical_factor(passer_idx, target_idx, is_home);
                if target_tactical < passer_tactical + tactical_threshold {
                    continue;
                }
            }

            if score > best_score {
                best_score = score;
                best_target = Some(target_idx);
            }
        }

        // FIX_2601/1128: Probabilistic safe option injection
        // 50% of the time, choose a safe option if available (was 35%)
        // This directly increases backward/lateral pass rate for reciprocity/density
        const SAFE_OPTION_PROBABILITY: f32 = 0.50;
        if let Some(safe_target) = best_safe_target {
            // Use deterministic "random" based on passer and tick
            let pseudo_random = ((passer_idx * 31 + self.current_tick as usize * 17) % 100) as f32 / 100.0;
            if pseudo_random < SAFE_OPTION_PROBABILITY {
                return Some(safe_target);
            }
        }

        Some(best_target.unwrap_or(fallback_target))
    }

    /// Execute pass action (6-Factor system)
    pub(crate) fn execute_pass_action(&mut self, from_idx: usize, is_home: bool, is_long: bool) {
        // FIX_2601/0117: Debug pass selection in first minute
        let debug_early = self.minute == 0 && std::env::var("DEBUG_PASS_0117").is_ok();

        // 6-Factor based best pass target selection
        let target_idx_opt = if is_long {
            // FIX_2601/0110: Use is_home (not attacks_right) for teammate range selection
            let valid_targets = self.find_valid_pass_targets(from_idx, is_home);
            let forward_targets: Vec<usize> = valid_targets
                .iter()
                .copied()
                .filter(|&i| {
                    let slot = if is_home { i } else { i - 11 };
                    slot >= 7
                })
                .collect();

            if !forward_targets.is_empty() {
                let mut best = forward_targets[0];
                let mut best_score = 0.0;
                for &t in &forward_targets {
                    let score = self.calculate_pass_score_6factor(from_idx, t, is_home);
                    if score > best_score {
                        best_score = score;
                        best = t;
                    }
                }
                Some(best)
            } else {
                self.select_pass_target(from_idx, is_home, true)
            }
        } else {
            self.find_best_pass_target(from_idx, is_home)
                .or_else(|| self.select_pass_target(from_idx, is_home, false))
        };

        let target_idx = match target_idx_opt {
            Some(idx) => idx,
            None => {
                self.assign_possession_to_nearest_defender(is_home);
                return;
            }
        };

        // FIX_2601/0117: Debug pass selection
        if debug_early {
            let from_pos = self.get_player_position_by_index(from_idx).to_meters();
            let target_pos = self.get_player_position_by_index(target_idx).to_meters();
            let team = if is_home { "HOME" } else { "AWAY" };
            let attacks_right = self.attacks_right(is_home);
            let delta_x = target_pos.0 - from_pos.0;
            let is_forward = if attacks_right { delta_x > 2.0 } else { delta_x < -2.0 };
            let dir = if is_forward { "FORWARD" } else { "BACKWARD" };
            let prog_delta = self.calculate_pass_progression_delta(from_idx, target_idx, is_home);
            println!(
                "[PASS_DBG] {} idx={} at X={:.1}m → idx={} at X={:.1}m [{}] attacks_right={} delta_x={:+.1} prog_delta={:+.3}",
                team, from_idx, from_pos.0, target_idx, target_pos.0, dir, attacks_right, delta_x, prog_delta
            );
        }

        // FIX_2601/0119: Track pass direction by slot group
        if std::env::var("DEBUG_SLOT_BIAS").is_ok() {
            let from_pos = self.get_player_position_by_index(from_idx);
            let target_pos = self.get_player_position_by_index(target_idx);
            let attacks_right = self.attacks_right(is_home);
            let prog_delta = self.calculate_pass_progression_delta(from_idx, target_idx, is_home);
            let slot_group = if from_idx < 11 { "0-10" } else { "11-21" };
            let direction = if prog_delta > 0.05 { "forward" } else if prog_delta < -0.05 { "backward" } else { "lateral" };
            let from_x = from_pos.to_meters().0;
            let target_x = target_pos.to_meters().0;
            eprintln!(
                "[PASS_DIR] tick={} slot_group={} dir={} from_x={:.1} to_x={:.1} prog_delta={:+.3} attacks_right={}",
                self.current_tick, slot_group, direction, from_x, target_x, prog_delta, attacks_right
            );
        }

        // 6-Factor based pass score
        let pass_score = self.calculate_pass_score_6factor(from_idx, target_idx, is_home);

        // Skill correction
        let passing = skills::normalize(self.get_player_passing(from_idx));
        let vision = skills::normalize(self.get_player_vision(from_idx));
        let technique = skills::normalize(self.get_player_technique(from_idx));
        let skill_factor = passing * 0.5 + vision * 0.3 + technique * 0.2;

        // Final success rate (6-Factor 60% + Skill 40%)
        let mut success_rate = (pass_score * 0.6 + skill_factor * 0.4).clamp(0.15, 0.95);

        // FIX_2601/0113: Apply PassRiskType modifier based on distance
        let from_pos = self.get_player_position_by_index(from_idx);
        let target_pos = self.get_player_position_by_index(target_idx);
        let pass_distance_m = from_pos.distance_to(&target_pos) as f32 / 10.0;
        let pass_risk_type = PassRiskType::from_distance(pass_distance_m, false);
        // Use passing attribute for long_passing (attributes don't separate these)
        let passer_passing = self.get_player_passing(from_idx) as f32;
        let passer_crossing = self.get_player_crossing(from_idx) as f32;
        let risk_modifier = pass_risk_type.success_modifier(passer_passing, passer_crossing);
        success_rate *= risk_modifier;

        // Gold Traits application
        if is_long && self.player_has_gold_trait(from_idx, TraitId::Architect) {
            success_rate = (success_rate + 0.15).min(0.95); // Long pass +15%
        }
        if self.player_has_gold_trait(from_idx, TraitId::Maestro) {
            success_rate = (success_rate + 0.10).min(0.95); // Maestro +10%
        }

        // Home advantage
        let final_rate = if is_home {
            (success_rate + home_advantage::PASS_SUCCESS_BONUS).min(0.95)
        } else {
            success_rate
        };

        let success = self.rng.gen::<f32>() < final_rate;

        // Emit pass event (for event tracking/replay)
        // C6: Use from_idx directly as track_id
        self.emit_event(
            MatchEvent::pass(self.minute, self.current_timestamp_ms(), is_home, from_idx, {
                let (x_m, y_m) = self.ball.position.to_meters();
                let h_m = self.ball.height as f32 / 10.0;
                (x_m / field::LENGTH_M, y_m / field::WIDTH_M, h_m)
            })
            .with_target_track_id(Some(target_idx)),
        );

        if success {
            // P7-OFFSIDE-02: Check offside BEFORE delivering the ball
            // FIX_2601/0105: Use attacks_right for direction
            // Phase 2: Use check_offside_detailed() to get OffsideDetails for "Why?" button
            if let Some(offside_details) = self.check_offside_detailed(from_idx, target_idx, is_home)
            {
                // Offside! Ball goes to defending team
                let receiver_pos = self.get_player_position_by_index(target_idx);

                // Record offside event with details
                // C6: Use target_idx directly as track_id
                self.emit_event(
                    MatchEvent::offside(self.minute, self.current_timestamp_ms(), is_home, target_idx)
                        .with_offside_details(offside_details),
                );

                // Record replay event
                if let Some(recorder) = self.replay_recorder.as_mut() {
                    let t_seconds = self.current_tick as f64 * 0.5;
                    let team_id = if is_home { 0 } else { 1 };
                    let player_id = target_idx as u32;
                    let receiver_pos_m = receiver_pos.to_meters();
                    recorder.record_offside(
                        t_seconds,
                        team_id,
                        player_id,
                        crate::replay::types::MeterPos {
                            x: receiver_pos_m.0 as f64,
                            y: receiver_pos_m.1 as f64,
                        },
                    );
                }

                // FIX_2601/0112: Statistics updated via events in stats.update_from_events()
                // Only update internal counters here
                if is_home {
                    self.offside_count_home += 1;
                } else {
                    self.offside_count_away += 1;
                }

                // Offside restart: indirect free kick to defending team
                self.apply_offside_restart(is_home, receiver_pos);
                return;
            }

            // Phase 3: Start ball flight to target (only if position tracking enabled)
            if self.track_positions {
                let from_pos = self.get_player_position_by_index(from_idx);
                let to_pos = self.get_player_position_by_index(target_idx);

                // FIX_2601: Convert to normalized for distance calculation and height profile
                let from_pos_norm = from_pos.to_normalized_legacy();
                let to_pos_norm = to_pos.to_normalized_legacy();

                // Calculate flight speed based on distance and pass type (in normalized units)
                let distance = coordinates::distance_norm(from_pos_norm, to_pos_norm);
                let base_speed = if is_long { 1.5 } else { 2.5 }; // Long pass slower
                let flight_speed = base_speed / distance.max(0.1); // Speed in progress/sec

                // A4: Apply height_profile based on pass situation
                // to_pos is already Coord10, use it directly
                self.ball.height_profile = self.determine_pass_height_profile(
                    from_pos_norm,
                    to_pos_norm,
                    distance,
                    self.ball.height as f32 / 10.0,
                    is_home,
                );

                self.ball.start_flight(to_pos, flight_speed, Some(target_idx));
            } else {
                // FIX_2601/1120: Update ball position to target's position to prevent teleportation
                // Short distance pass - instant transfer
                self.ball.current_owner = Some(target_idx);
                self.ball.position = self.player_positions[target_idx];
            }
        } else {
            self.assign_possession_to_nearest_defender(is_home);
        }
    }
}
