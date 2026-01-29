//! Forward-specific behavior handler
//!
//! Handles states like: RunningInBehind, HoldingUpPlay, Finishing,
//! ReceivingCross, Pressing, CreatingSpace, OffsideTrapBreaking, Assisting

use crate::engine::position_behavior::{PositionContext, PositionStateHandler};
use crate::engine::position_substates::{ForwardSubState, PositionSubState};

/// Handler for forward positions (LW, RW, CF, ST)
pub struct ForwardHandler;

impl PositionStateHandler for ForwardHandler {
    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        use ForwardSubState::*;

        let fs = match ctx.current_substate {
            PositionSubState::Forward(fs) => fs,
            _ => return Some(self.default_substate()),
        };

        // Has ball near goal → Finishing
        if ctx.player_has_ball && ctx.in_scoring_position && fs != Finishing {
            return Some(PositionSubState::Forward(Finishing));
        }

        // Has ball, back to goal → HoldingUpPlay
        if ctx.player_has_ball && ctx.back_to_goal && !ctx.in_scoring_position {
            return Some(PositionSubState::Forward(HoldingUpPlay));
        }

        // Cross incoming + in box → ReceivingCross
        if ctx.cross_incoming && ctx.in_box && fs != ReceivingCross {
            return Some(PositionSubState::Forward(ReceivingCross));
        }

        // Can break offside + team attacking → RunningInBehind
        // Phase 2: Use Levers for run trigger
        if ctx.can_break_offside 
            && ctx.team_attacking 
            && !ctx.player_has_ball 
            && fs != RunningInBehind
            && ctx.goal_distance < ctx.levers.run_trigger_distance
        {
            return Some(PositionSubState::Forward(RunningInBehind));
        }

        // Ball coming to me + suitable position → prepare to receive
        if ctx.ball_coming_to_me && !ctx.player_has_ball && ctx.back_to_goal {
            return Some(PositionSubState::Forward(HoldingUpPlay));
        }

        // Not team's ball + in pressing zone → Pressing
        if !ctx.team_has_ball && ctx.in_pressing_zone && !matches!(fs, Pressing) {
            return Some(PositionSubState::Forward(Pressing));
        }

        // Has ball, not in scoring position, teammates making runs → Assisting
        if ctx.player_has_ball && !ctx.in_scoring_position && !ctx.back_to_goal {
            return Some(PositionSubState::Forward(Assisting));
        }

        // Team has ball, not pressing, need to create space
        if ctx.team_has_ball && !ctx.player_has_ball && ctx.crowded_area {
            return Some(PositionSubState::Forward(CreatingSpace));
        }

        None
    }

    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        use ForwardSubState::*;

        let fs = match ctx.current_substate {
            PositionSubState::Forward(fs) => fs,
            _ => return (0.0, 0.0),
        };

        match fs {
            RunningInBehind => {
                // PoC: Delegate to Trait-based State Object
                use crate::engine::behaviors::traits::StateBehavior;
                use crate::engine::behaviors::forward_run_in_behind::ForwardRunInBehindState;
                ForwardRunInBehindState.calculate_velocity(ctx)
            }
            HoldingUpPlay => {
                use crate::engine::behaviors::forward_holding_up_play::ForwardHoldingUpPlayState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardHoldingUpPlayState.calculate_velocity(ctx)
            }
            Finishing => {
                use crate::engine::behaviors::forward_states::ForwardFinishingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardFinishingState.calculate_velocity(ctx)
            }
            Shooting => {
                use crate::engine::behaviors::forward_states::ForwardShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardShootingState.calculate_velocity(ctx)
            }
            ReceivingCross => {
                use crate::engine::behaviors::forward_receiving_cross::ForwardReceivingCrossState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardReceivingCrossState.calculate_velocity(ctx)
            }
            Pressing => {
                use crate::engine::behaviors::forward_pressing::ForwardPressingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardPressingState.calculate_velocity(ctx)
            }
            CreatingSpace => {
                // Delegate to Trait-based State Object
                use crate::engine::behaviors::forward_creating_space::ForwardCreatingSpaceState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardCreatingSpaceState.calculate_velocity(ctx)
            }
            OffsideTrapBreaking => {
                use crate::engine::behaviors::forward_offside_trap_breaking::ForwardOffsideTrapBreakingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardOffsideTrapBreakingState.calculate_velocity(ctx)
            }
            Assisting => {
                use crate::engine::behaviors::forward_assisting::ForwardAssistingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardAssistingState.calculate_velocity(ctx)
            }
            Dribbling => {
                use crate::engine::behaviors::forward_states::ForwardDribblingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardDribblingState.calculate_velocity(ctx)
            }
            Heading => {
                use crate::engine::behaviors::forward_states::ForwardHeadingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardHeadingState.calculate_velocity(ctx)
            }
            Intercepting => {
                use crate::engine::behaviors::forward_states::ForwardInterceptingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardInterceptingState.calculate_velocity(ctx)
            }
            Tackling => {
                use crate::engine::behaviors::forward_states::ForwardTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardTacklingState.calculate_velocity(ctx)
            }
            Running => {
                use crate::engine::behaviors::forward_states::ForwardRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardRunningState.calculate_velocity(ctx)
            }
            Returning => {
                use crate::engine::behaviors::forward_states::ForwardReturningState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardReturningState.calculate_velocity(ctx)
            }
            Passing => {
                use crate::engine::behaviors::forward_states::ForwardPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardPassingState.calculate_velocity(ctx)
            }
            Resting => {
                use crate::engine::behaviors::forward_states::ForwardRestingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardRestingState.calculate_velocity(ctx)
            }
            Standing => {
                use crate::engine::behaviors::forward_states::ForwardStandingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardStandingState.calculate_velocity(ctx)
            }
            Walking => {
                use crate::engine::behaviors::forward_states::ForwardWalkingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardWalkingState.calculate_velocity(ctx)
            }
            Jogging => {
                use crate::engine::behaviors::forward_states::ForwardJoggingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardJoggingState.calculate_velocity(ctx)
            }
        }
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        use ForwardSubState::*;

        let fs = match ctx.current_substate {
            PositionSubState::Forward(fs) => fs,
            _ => return false,
        };

        match fs {
            RunningInBehind => ctx.in_substate_ticks > 40 || ctx.offside_detected,
            HoldingUpPlay => {
                use crate::engine::behaviors::forward_holding_up_play::ForwardHoldingUpPlayState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardHoldingUpPlayState.should_timeout(ctx)
            }
            Finishing => {
                use crate::engine::behaviors::forward_states::ForwardFinishingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardFinishingState.should_timeout(ctx)
            }
            Shooting => {
                use crate::engine::behaviors::forward_states::ForwardShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardShootingState.should_timeout(ctx)
            }
            ReceivingCross => {
                use crate::engine::behaviors::forward_receiving_cross::ForwardReceivingCrossState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardReceivingCrossState.should_timeout(ctx)
            }
            Pressing => {
                use crate::engine::behaviors::forward_pressing::ForwardPressingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardPressingState.should_timeout(ctx)
            }
            CreatingSpace => ctx.in_substate_ticks > 40,
            OffsideTrapBreaking => {
                use crate::engine::behaviors::forward_offside_trap_breaking::ForwardOffsideTrapBreakingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardOffsideTrapBreakingState.should_timeout(ctx)
            }
            Assisting => {
                use crate::engine::behaviors::forward_assisting::ForwardAssistingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardAssistingState.should_timeout(ctx)
            }
            Dribbling => {
                use crate::engine::behaviors::forward_states::ForwardDribblingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardDribblingState.should_timeout(ctx)
            }
            Heading => {
                use crate::engine::behaviors::forward_states::ForwardHeadingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardHeadingState.should_timeout(ctx)
            }
            Intercepting => {
                use crate::engine::behaviors::forward_states::ForwardInterceptingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardInterceptingState.should_timeout(ctx)
            }
            Tackling => {
                use crate::engine::behaviors::forward_states::ForwardTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardTacklingState.should_timeout(ctx)
            }
            Running => {
                use crate::engine::behaviors::forward_states::ForwardRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardRunningState.should_timeout(ctx)
            }
            Returning => {
                use crate::engine::behaviors::forward_states::ForwardReturningState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardReturningState.should_timeout(ctx)
            }
            Passing => {
                use crate::engine::behaviors::forward_states::ForwardPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardPassingState.should_timeout(ctx)
            }
            Resting => {
                use crate::engine::behaviors::forward_states::ForwardRestingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardRestingState.should_timeout(ctx)
            }
            // P4.4: Standing exits when ball approaches or opportunity arises
            Standing => {
                use crate::engine::behaviors::forward_states::ForwardStandingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardStandingState.should_timeout(ctx)
            }
            // P4.4: Walking exits when action is needed
            Walking => {
                use crate::engine::behaviors::forward_states::ForwardWalkingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardWalkingState.should_timeout(ctx)
            }
            // P5.1: Jogging exits when closer action needed
            Jogging => {
                use crate::engine::behaviors::forward_states::ForwardJoggingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardJoggingState.should_timeout(ctx)
            }
        }
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        use ForwardSubState::*;

        let fs = match ctx.current_substate {
            PositionSubState::Forward(fs) => fs,
            _ => return self.default_substate(),
        };

        match fs {
            RunningInBehind => {
                if ctx.offside_detected {
                    // Got caught offside, drop back
                    PositionSubState::Forward(CreatingSpace)
                } else {
                    // Run completed without receiving, create space
                    PositionSubState::Forward(CreatingSpace)
                }
            }
            HoldingUpPlay => {
                use crate::engine::behaviors::forward_holding_up_play::ForwardHoldingUpPlayState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardHoldingUpPlayState.timeout_transition(ctx)
            }
            Finishing => {
                use crate::engine::behaviors::forward_states::ForwardFinishingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardFinishingState.timeout_transition(ctx)
            }
            Shooting => {
                use crate::engine::behaviors::forward_states::ForwardShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardShootingState.timeout_transition(ctx)
            }
            Dribbling => {
                use crate::engine::behaviors::forward_states::ForwardDribblingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardDribblingState.timeout_transition(ctx)
            }
            Passing => {
                use crate::engine::behaviors::forward_states::ForwardPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardPassingState.timeout_transition(ctx)
            }
            ReceivingCross => {
                use crate::engine::behaviors::forward_receiving_cross::ForwardReceivingCrossState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardReceivingCrossState.timeout_transition(ctx)
            }
            Heading => {
                use crate::engine::behaviors::forward_states::ForwardHeadingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardHeadingState.timeout_transition(ctx)
            }
            Pressing => {
                use crate::engine::behaviors::forward_pressing::ForwardPressingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardPressingState.timeout_transition(ctx)
            }
            Tackling => {
                use crate::engine::behaviors::forward_states::ForwardTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardTacklingState.timeout_transition(ctx)
            }
            Intercepting => {
                use crate::engine::behaviors::forward_states::ForwardInterceptingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardInterceptingState.timeout_transition(ctx)
            }
            CreatingSpace => {
                if ctx.team_attacking && ctx.can_break_offside {
                    PositionSubState::Forward(RunningInBehind)
                } else if ctx.team_has_ball {
                    PositionSubState::Forward(CreatingSpace)
                } else {
                    PositionSubState::Forward(Pressing)
                }
            }
            Running => {
                use crate::engine::behaviors::forward_states::ForwardRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardRunningState.timeout_transition(ctx)
            }
            Returning => {
                use crate::engine::behaviors::forward_states::ForwardReturningState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardReturningState.timeout_transition(ctx)
            }
            OffsideTrapBreaking => {
                use crate::engine::behaviors::forward_offside_trap_breaking::ForwardOffsideTrapBreakingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardOffsideTrapBreakingState.timeout_transition(ctx)
            }
            Resting => {
                use crate::engine::behaviors::forward_states::ForwardRestingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardRestingState.timeout_transition(ctx)
            }
            // P4.4/P5.1: Standing/Walking/Jogging → return to active attacking
            Standing => {
                use crate::engine::behaviors::forward_states::ForwardStandingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardStandingState.timeout_transition(ctx)
            }
            Walking => {
                use crate::engine::behaviors::forward_states::ForwardWalkingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardWalkingState.timeout_transition(ctx)
            }
            Jogging => {
                use crate::engine::behaviors::forward_states::ForwardJoggingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardJoggingState.timeout_transition(ctx)
            }
            Assisting => {
                use crate::engine::behaviors::forward_assisting::ForwardAssistingState;
                use crate::engine::behaviors::traits::StateBehavior;
                ForwardAssistingState.timeout_transition(ctx)
            }
        }
    }

    fn default_substate(&self) -> PositionSubState {
        PositionSubState::forward_default()
    }

    /// FIX_2512 Phase 13: Slow evaluation for forwards
    ///
    /// Forwards use slow evaluation for:
    /// - Pass target selection when holding the ball
    /// - Penetration timing (key for RunningInBehind)
    /// - Pressing intensity from the front
    fn process_slow(
        &self,
        ctx: &crate::engine::slow_evaluation::SlowContext,
    ) -> crate::engine::slow_evaluation::SlowEvaluationResult {
        use crate::engine::slow_evaluation::{
            PassTargetEvaluator, PenetrationEvaluator, PressingEvaluator, SlowEvaluationResult,
            ZoneEscapeEvaluator,
        };

        let mut result = SlowEvaluationResult::default();

        // 1. Pass target evaluation (if has ball)
        if ctx.core.player_has_ball {
            let targets = PassTargetEvaluator::evaluate_targets(ctx);
            result.best_pass_target = targets.first().copied();
        }

        // 2. Penetration evaluation (if team has ball, not me)
        // This is THE key evaluation for forwards
        if ctx.core.team_has_ball && !ctx.core.player_has_ball {
            let (should, score) = PenetrationEvaluator::should_penetrate(ctx);
            result.should_penetrate = should;
            result.penetration_score = score;

            if should {
                result.suggested_state =
                    Some(PositionSubState::Forward(ForwardSubState::RunningInBehind));
            } else if ZoneEscapeEvaluator::should_create_space(ctx) {
                // P3.1: Zone escape when crowded and not penetrating
                result.suggested_state =
                    Some(PositionSubState::Forward(ForwardSubState::CreatingSpace));
            }
        }

        // 3. Pressing evaluation (if opponent has ball)
        // Forwards press from the front to start counter-press
        if !ctx.core.team_has_ball {
            result.pressing_intensity = PressingEvaluator::evaluate_pressing(ctx);

            if result.pressing_intensity > 0.7 {
                result.suggested_state =
                    Some(PositionSubState::Forward(ForwardSubState::Pressing));
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn test_finishing_transition() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::CreatingSpace);
        ctx.player_has_ball = true;
        ctx.in_scoring_position = true;

        let result = handler.try_fast_transition(&ctx);
        assert_eq!(
            result,
            Some(PositionSubState::Forward(ForwardSubState::Finishing))
        );
    }

    #[test]
    fn test_running_in_behind_transition() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::CreatingSpace);
        ctx.can_break_offside = true;
        ctx.team_attacking = true;
        ctx.player_has_ball = false;

        let result = handler.try_fast_transition(&ctx);
        assert_eq!(
            result,
            Some(PositionSubState::Forward(ForwardSubState::RunningInBehind))
        );
    }

    #[test]
    fn test_cross_receiving_transition() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::CreatingSpace);
        ctx.cross_incoming = true;
        ctx.in_box = true;

        let result = handler.try_fast_transition(&ctx);
        assert_eq!(
            result,
            Some(PositionSubState::Forward(ForwardSubState::ReceivingCross))
        );
    }

    #[test]
    fn test_pressing_timeout() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::Pressing);
        ctx.in_substate_ticks = 61;
        ctx.team_has_ball = true;

        assert!(handler.should_timeout(&ctx));
        assert_eq!(
            handler.timeout_transition(&ctx),
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        );
    }

    // P4.3: Velocity tests for forward states

    #[test]
    fn test_velocity_running_in_behind() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::RunningInBehind);
        ctx.player_position = (50.0, 70.0);

        let vel = handler.calculate_velocity(&ctx);
        let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
        // RunningInBehind should be max sprint
        assert!(speed > 7.0, "RunningInBehind should be max sprint speed: {}", speed);
    }

    #[test]
    fn test_velocity_pressing() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::Pressing);
        ctx.player_position = (40.0, 80.0);
        ctx.ball_position = (50.0, 75.0);

        let vel = handler.calculate_velocity(&ctx);
        // Should move toward ball
        assert!(vel.0 > 0.0, "Should move toward ball x");
    }

    #[test]
    fn test_velocity_holding_up_play() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::HoldingUpPlay);

        let vel = handler.calculate_velocity(&ctx);
        // Should shield ball with slight backward lean
        assert!(vel.1 < 0.0, "HoldingUpPlay should have slight backward movement");
        assert!(vel.1.abs() < 1.0, "Movement should be minimal");
    }

    #[test]
    fn test_velocity_finishing_near_ball() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::Finishing);
        ctx.player_position = (90.0, field::CENTER_Y);
        ctx.ball_position = (90.5, field::CENTER_Y);
        ctx.ball_distance = 0.5;

        let vel = handler.calculate_velocity(&ctx);
        // Near ball - should be stationary for shot
        assert_eq!(vel, (0.0, 0.0), "Finishing near ball should be stationary");
    }

    #[test]
    fn test_velocity_resting_is_zero() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::Resting);

        let vel = handler.calculate_velocity(&ctx);
        assert_eq!(vel, (0.0, 0.0), "Resting should have zero velocity");
    }

    #[test]
    fn test_velocity_passing_stationary() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::Passing);

        let vel = handler.calculate_velocity(&ctx);
        assert_eq!(vel, (0.0, 0.0), "Passing should be stationary");
    }

    #[test]
    fn test_velocity_assisting_stationary() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::Assisting);

        let vel = handler.calculate_velocity(&ctx);
        assert_eq!(vel, (0.0, 0.0), "Assisting should be stationary (looking for pass)");
    }

    #[test]
    fn test_resting_no_timeout() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::Resting);
        ctx.in_substate_ticks = 1000;

        assert!(!handler.should_timeout(&ctx), "Resting should not timeout");
    }

    #[test]
    fn test_timeout_transition_to_creating_space() {
        let handler = ForwardHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Forward(ForwardSubState::Pressing);
        ctx.in_substate_ticks = 61;
        ctx.team_has_ball = false;

        assert!(handler.should_timeout(&ctx));
        assert_eq!(
            handler.timeout_transition(&ctx),
            PositionSubState::Forward(ForwardSubState::CreatingSpace)
        );
    }
}
