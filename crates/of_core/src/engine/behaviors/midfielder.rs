//! Midfielder-specific behavior handler
//!
//! Handles states like: Distributing, SwitchingPlay, CreatingSpace, AttackSupporting,
//! HoldingPossession, Pressing, TrackingRunner, Shooting, Recycling, Intercepting

use crate::engine::position_behavior::{PositionContext, PositionStateHandler};
use crate::engine::position_substates::{MidfielderSubState, PositionSubState};

/// Handler for midfielder positions (CDM, CM, CAM, LM, RM)
pub struct MidfielderHandler;

impl PositionStateHandler for MidfielderHandler {
    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        use MidfielderSubState::*;

        let ms = match ctx.current_substate {
            PositionSubState::Midfielder(ms) => ms,
            _ => return Some(self.default_substate()),
        };

        // Has ball → Distributing (unless already in on-ball state)
        if ctx.player_has_ball && !matches!(ms, Distributing | Shooting | HoldingPossession) {
            return Some(PositionSubState::Midfielder(Distributing));
        }

        // Shooting opportunity (edge of box, clear sight)
        if ctx.player_has_ball && ctx.goal_distance < 25.0 && ctx.clear_shot && ms != Shooting {
            return Some(PositionSubState::Midfielder(Shooting));
        }

        // Under pressure with ball → HoldingPossession
        if ctx.player_has_ball && ctx.local_pressure > 0.6 && ms == Distributing {
            return Some(PositionSubState::Midfielder(HoldingPossession));
        }

        // Runner to track (defensive duty)
        if !ctx.team_has_ball && ctx.runner_to_track.is_some() && ms != TrackingRunner {
            return Some(PositionSubState::Midfielder(TrackingRunner));
        }

        // Crowded area + team attacking → CreateSpace
        if ctx.team_has_ball && !ctx.player_has_ball && ctx.crowded_area && ms != CreatingSpace {
            return Some(PositionSubState::Midfielder(CreatingSpace));
        }

        // Ball loose nearby → Intercepting
        if ctx.ball_distance < 5.0 && !ctx.team_has_ball && !ctx.opponent_with_ball_nearby {
            return Some(PositionSubState::Midfielder(Intercepting));
        }

        // Team attacking, not with ball → AttackSupporting
        if ctx.team_has_ball
            && !ctx.player_has_ball
            && !ctx.crowded_area
            && matches!(ms, Distributing | Recycling)
        {
            return Some(PositionSubState::Midfielder(AttackSupporting));
        }

        // Opponent has ball nearby → Pressing
        if ctx.opponent_with_ball_nearby
            && ctx.ball_distance < 10.0
            && !matches!(ms, Pressing | TrackingRunner)
        {
            return Some(PositionSubState::Midfielder(Pressing));
        }

        None
    }

    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        use MidfielderSubState::*;

        let ms = match ctx.current_substate {
            PositionSubState::Midfielder(ms) => ms,
            _ => return (0.0, 0.0),
        };

        match ms {
            SwitchingPlay => {
                use crate::engine::behaviors::midfielder_states::MidfielderSwitchingPlayState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderSwitchingPlayState.calculate_velocity(ctx)
            }
            CreatingSpace => {
                use crate::engine::behaviors::midfielder_creating_space::MidfielderCreatingSpaceState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderCreatingSpaceState.calculate_velocity(ctx)
            }
            AttackSupporting => {
                use crate::engine::behaviors::midfielder_attack_supporting::MidfielderAttackSupportingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderAttackSupportingState.calculate_velocity(ctx)
            }
            TrackingRunner => {
                use crate::engine::behaviors::midfielder_tracking_runner::MidfielderTrackingRunnerState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderTrackingRunnerState.calculate_velocity(ctx)
            }
            Pressing => {
                use crate::engine::behaviors::midfielder_pressing::MidfielderPressingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderPressingState.calculate_velocity(ctx)
            }
            Recycling => {
                use crate::engine::behaviors::midfielder_states::MidfielderRecyclingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderRecyclingState.calculate_velocity(ctx)
            }
            Intercepting => {
                use crate::engine::behaviors::midfielder_states::MidfielderInterceptingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderInterceptingState.calculate_velocity(ctx)
            }
            Dribbling => {
                use crate::engine::behaviors::midfielder_states::MidfielderDribblingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderDribblingState.calculate_velocity(ctx)
            }
            Crossing => {
                use crate::engine::behaviors::midfielder_states::MidfielderCrossingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderCrossingState.calculate_velocity(ctx)
            }
            DistanceShooting => {
                use crate::engine::behaviors::midfielder_states::MidfielderDistanceShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderDistanceShootingState.calculate_velocity(ctx)
            }
            Tackling => {
                use crate::engine::behaviors::midfielder_states::MidfielderTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderTacklingState.calculate_velocity(ctx)
            }
            Running => {
                use crate::engine::behaviors::midfielder_states::MidfielderRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderRunningState.calculate_velocity(ctx)
            }
            Returning => {
                use crate::engine::behaviors::midfielder_states::MidfielderReturningState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderReturningState.calculate_velocity(ctx)
            }
            Passing => {
                // Delegate to Trait-based State Object
                use crate::engine::behaviors::midfielder_passing::MidfielderPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderPassingState.calculate_velocity(ctx)
            }
            Distributing => {
                use crate::engine::behaviors::midfielder_states::MidfielderDistributingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderDistributingState.calculate_velocity(ctx)
            }
            HoldingPossession => {
                use crate::engine::behaviors::midfielder_states::MidfielderHoldingPossessionState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderHoldingPossessionState.calculate_velocity(ctx)
            }
            Shooting => {
                use crate::engine::behaviors::midfielder_states::MidfielderShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderShootingState.calculate_velocity(ctx)
            }
            Resting => {
                use crate::engine::behaviors::midfielder_states::MidfielderRestingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderRestingState.calculate_velocity(ctx)
            }
            Standing => {
                use crate::engine::behaviors::midfielder_states::MidfielderStandingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderStandingState.calculate_velocity(ctx)
            }
            Walking => {
                use crate::engine::behaviors::midfielder_states::MidfielderWalkingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderWalkingState.calculate_velocity(ctx)
            }
            Jogging => {
                use crate::engine::behaviors::midfielder_states::MidfielderJoggingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderJoggingState.calculate_velocity(ctx)
            }
        }
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        use MidfielderSubState::*;

        let ms = match ctx.current_substate {
            PositionSubState::Midfielder(ms) => ms,
            _ => return false,
        };

        match ms {
            CreatingSpace => {
                use crate::engine::behaviors::midfielder_creating_space::MidfielderCreatingSpaceState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderCreatingSpaceState.should_timeout(ctx)
            }
            Pressing => {
                use crate::engine::behaviors::midfielder_pressing::MidfielderPressingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderPressingState.should_timeout(ctx)
            }
            TrackingRunner => {
                use crate::engine::behaviors::midfielder_tracking_runner::MidfielderTrackingRunnerState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderTrackingRunnerState.should_timeout(ctx)
            }
            Intercepting => {
                use crate::engine::behaviors::midfielder_states::MidfielderInterceptingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderInterceptingState.should_timeout(ctx)
            }
            Shooting => {
                use crate::engine::behaviors::midfielder_states::MidfielderShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderShootingState.should_timeout(ctx)
            }
            DistanceShooting => {
                use crate::engine::behaviors::midfielder_states::MidfielderDistanceShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderDistanceShootingState.should_timeout(ctx)
            }
            HoldingPossession => {
                use crate::engine::behaviors::midfielder_states::MidfielderHoldingPossessionState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderHoldingPossessionState.should_timeout(ctx)
            }
            SwitchingPlay => {
                use crate::engine::behaviors::midfielder_states::MidfielderSwitchingPlayState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderSwitchingPlayState.should_timeout(ctx)
            }
            Recycling => {
                use crate::engine::behaviors::midfielder_states::MidfielderRecyclingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderRecyclingState.should_timeout(ctx)
            }
            Dribbling => {
                use crate::engine::behaviors::midfielder_states::MidfielderDribblingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderDribblingState.should_timeout(ctx)
            }
            Crossing => {
                use crate::engine::behaviors::midfielder_states::MidfielderCrossingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderCrossingState.should_timeout(ctx)
            }
            Tackling => {
                use crate::engine::behaviors::midfielder_states::MidfielderTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderTacklingState.should_timeout(ctx)
            }
            Running => {
                use crate::engine::behaviors::midfielder_states::MidfielderRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderRunningState.should_timeout(ctx)
            }
            Returning => {
                use crate::engine::behaviors::midfielder_states::MidfielderReturningState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderReturningState.should_timeout(ctx)
            }
            Passing => {
                use crate::engine::behaviors::midfielder_passing::MidfielderPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderPassingState.should_timeout(ctx)
            }
            Resting => {
                use crate::engine::behaviors::midfielder_states::MidfielderRestingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderRestingState.should_timeout(ctx)
            }
            // P4.4: Standing exits when ball approaches or pressure builds
            Standing => {
                use crate::engine::behaviors::midfielder_states::MidfielderStandingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderStandingState.should_timeout(ctx)
            }
            // P4.4: Walking exits when action is needed
            Walking => {
                use crate::engine::behaviors::midfielder_states::MidfielderWalkingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderWalkingState.should_timeout(ctx)
            }
            // P5.1: Jogging exits when closer action needed
            Jogging => {
                use crate::engine::behaviors::midfielder_states::MidfielderJoggingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderJoggingState.should_timeout(ctx)
            }
            AttackSupporting => {
                use crate::engine::behaviors::midfielder_attack_supporting::MidfielderAttackSupportingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderAttackSupportingState.should_timeout(ctx)
            }
            Distributing => {
                use crate::engine::behaviors::midfielder_states::MidfielderDistributingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderDistributingState.should_timeout(ctx)
            }
        }
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        use MidfielderSubState::*;

        let ms = match ctx.current_substate {
            PositionSubState::Midfielder(ms) => ms,
            _ => return self.default_substate(),
        };

        match ms {
            SwitchingPlay => {
                use crate::engine::behaviors::midfielder_states::MidfielderSwitchingPlayState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderSwitchingPlayState.timeout_transition(ctx)
            }
            Recycling => {
                use crate::engine::behaviors::midfielder_states::MidfielderRecyclingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderRecyclingState.timeout_transition(ctx)
            }
            CreatingSpace => {
                use crate::engine::behaviors::midfielder_creating_space::MidfielderCreatingSpaceState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderCreatingSpaceState.timeout_transition(ctx)
            }
            Pressing => {
                use crate::engine::behaviors::midfielder_pressing::MidfielderPressingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderPressingState.timeout_transition(ctx)
            }
            Intercepting => {
                use crate::engine::behaviors::midfielder_states::MidfielderInterceptingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderInterceptingState.timeout_transition(ctx)
            }
            Tackling => {
                use crate::engine::behaviors::midfielder_states::MidfielderTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderTacklingState.timeout_transition(ctx)
            }
            TrackingRunner => {
                use crate::engine::behaviors::midfielder_tracking_runner::MidfielderTrackingRunnerState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderTrackingRunnerState.timeout_transition(ctx)
            }
            Shooting => {
                use crate::engine::behaviors::midfielder_states::MidfielderShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderShootingState.timeout_transition(ctx)
            }
            DistanceShooting => {
                use crate::engine::behaviors::midfielder_states::MidfielderDistanceShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderDistanceShootingState.timeout_transition(ctx)
            }
            HoldingPossession => {
                use crate::engine::behaviors::midfielder_states::MidfielderHoldingPossessionState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderHoldingPossessionState.timeout_transition(ctx)
            }
            Passing => {
                use crate::engine::behaviors::midfielder_passing::MidfielderPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderPassingState.timeout_transition(ctx)
            }
            Dribbling => {
                use crate::engine::behaviors::midfielder_states::MidfielderDribblingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderDribblingState.timeout_transition(ctx)
            }
            Crossing => {
                use crate::engine::behaviors::midfielder_states::MidfielderCrossingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderCrossingState.timeout_transition(ctx)
            }
            Running => {
                use crate::engine::behaviors::midfielder_states::MidfielderRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderRunningState.timeout_transition(ctx)
            }
            Returning => {
                use crate::engine::behaviors::midfielder_states::MidfielderReturningState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderReturningState.timeout_transition(ctx)
            }
            Resting => {
                use crate::engine::behaviors::midfielder_states::MidfielderRestingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderRestingState.timeout_transition(ctx)
            }
            // P4.4/P5.1: Standing/Walking/Jogging → return to active play
            Standing => {
                use crate::engine::behaviors::midfielder_states::MidfielderStandingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderStandingState.timeout_transition(ctx)
            }
            Walking => {
                use crate::engine::behaviors::midfielder_states::MidfielderWalkingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderWalkingState.timeout_transition(ctx)
            }
            Jogging => {
                use crate::engine::behaviors::midfielder_states::MidfielderJoggingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderJoggingState.timeout_transition(ctx)
            }
            AttackSupporting => {
                use crate::engine::behaviors::midfielder_attack_supporting::MidfielderAttackSupportingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderAttackSupportingState.timeout_transition(ctx)
            }
            Distributing => {
                use crate::engine::behaviors::midfielder_states::MidfielderDistributingState;
                use crate::engine::behaviors::traits::StateBehavior;
                MidfielderDistributingState.timeout_transition(ctx)
            }
        }
    }

    fn default_substate(&self) -> PositionSubState {
        PositionSubState::midfielder_default()
    }

    /// FIX_2512 Phase 13: Slow evaluation for midfielders
    ///
    /// Midfielders use slow evaluation for:
    /// - Pass target selection (key role: distribution)
    /// - Pressing intensity (supporting forwards)
    /// - Space creation timing
    fn process_slow(
        &self,
        ctx: &crate::engine::slow_evaluation::SlowContext,
    ) -> crate::engine::slow_evaluation::SlowEvaluationResult {
        use crate::engine::slow_evaluation::{
            PassTargetEvaluator, PressingEvaluator, SlowEvaluationResult, ZoneEscapeEvaluator,
        };

        let mut result = SlowEvaluationResult::default();

        // 1. Pass target evaluation (primary role for midfielders)
        if ctx.core.player_has_ball {
            let targets = PassTargetEvaluator::evaluate_targets(ctx);
            result.best_pass_target = targets.first().copied();
        }

        // 2. Pressing evaluation (if opponent has ball)
        // Midfielders support the press from behind
        if !ctx.core.team_has_ball {
            result.pressing_intensity = PressingEvaluator::evaluate_pressing(ctx);

            // Only trigger pressing state if intensity is high
            if result.pressing_intensity > 0.75 {
                result.suggested_state =
                    Some(PositionSubState::Midfielder(MidfielderSubState::Pressing));
            }
        }

        // 3. Zone escape (P3.1): Use ZoneEscapeEvaluator for zone-aware crowding
        // ZoneEscapeEvaluator already checks team_has_ball && !player_has_ball internally
        if ZoneEscapeEvaluator::should_create_space(ctx) {
            result.suggested_state =
                Some(PositionSubState::Midfielder(MidfielderSubState::CreatingSpace));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn test_has_ball_transition() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::AttackSupporting);
        ctx.player_has_ball = true;

        let result = handler.try_fast_transition(&ctx);
        assert_eq!(
            result,
            Some(PositionSubState::Midfielder(
                MidfielderSubState::Distributing
            ))
        );
    }

    #[test]
    fn test_shooting_opportunity() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::Distributing);
        ctx.player_has_ball = true;
        ctx.goal_distance = 20.0;
        ctx.clear_shot = true;

        let result = handler.try_fast_transition(&ctx);
        assert_eq!(
            result,
            Some(PositionSubState::Midfielder(MidfielderSubState::Shooting))
        );
    }

    #[test]
    fn test_creating_space_timeout() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::CreatingSpace);
        ctx.in_substate_ticks = 41;
        ctx.team_has_ball = true;

        assert!(handler.should_timeout(&ctx));
        assert_eq!(
            handler.timeout_transition(&ctx),
            PositionSubState::Midfielder(MidfielderSubState::AttackSupporting)
        );
    }

    // P4.3: Velocity tests for midfielder states

    #[test]
    fn test_velocity_pressing() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::Pressing);
        ctx.player_position = (40.0, 30.0);
        ctx.ball_position = (50.0, 40.0);

        let vel = handler.calculate_velocity(&ctx);
        let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
        // Pressing should be fast
        assert!(speed > 4.0, "Pressing should be high speed: {}", speed);
    }

    #[test]
    fn test_velocity_creating_space() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::CreatingSpace);
        ctx.least_crowded_direction = (0.7, 0.7);

        let vel = handler.calculate_velocity(&ctx);
        // Should move toward least crowded direction
        assert!(vel.0 > 0.0, "Should move in least crowded x direction");
        assert!(vel.1 > 0.0, "Should move in least crowded y direction");
    }

    #[test]
    fn test_velocity_distributing_stationary() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::Distributing);

        let vel = handler.calculate_velocity(&ctx);
        assert_eq!(vel, (0.0, 0.0), "Distributing should be stationary");
    }

    #[test]
    fn test_velocity_resting_is_zero() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::Resting);

        let vel = handler.calculate_velocity(&ctx);
        assert_eq!(vel, (0.0, 0.0), "Resting should have zero velocity");
    }

    #[test]
    fn test_velocity_passing_stationary() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::Passing);

        let vel = handler.calculate_velocity(&ctx);
        assert_eq!(vel, (0.0, 0.0), "Passing should be stationary");
    }

    #[test]
    fn test_velocity_recycling_backward() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::Recycling);
        ctx.player_position = (50.0, 50.0);
        ctx.own_goal = (0.0, field::CENTER_Y);

        let vel = handler.calculate_velocity(&ctx);
        // Should move toward own goal (backward)
        assert!(vel.0 < 0.0, "Recycling should move backward (toward own goal)");
    }

    #[test]
    fn test_resting_no_timeout() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::Resting);
        ctx.in_substate_ticks = 1000;

        assert!(!handler.should_timeout(&ctx), "Resting should not timeout");
    }

    #[test]
    fn test_timeout_transition_to_distributing() {
        let handler = MidfielderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Midfielder(MidfielderSubState::Pressing);
        ctx.in_substate_ticks = 61;
        ctx.team_has_ball = false;

        assert!(handler.should_timeout(&ctx));
        assert_eq!(
            handler.timeout_transition(&ctx),
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        );
    }
}
