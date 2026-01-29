//! Defender-specific behavior handler
//!
//! Handles states like: HoldingLine, Covering, MarkingTight, Pressing,
//! TrackingBack, Clearing, Heading, Tackling, OffsideTrap, PushingUp, Intercepting

use crate::engine::position_behavior::{PositionContext, PositionStateHandler};
use crate::engine::position_substates::{DefenderSubState, PositionSubState};

/// Handler for defender positions (CB, LB, RB, LWB, RWB)
pub struct DefenderHandler;

impl PositionStateHandler for DefenderHandler {
    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        use DefenderSubState::*;

        let ds = match ctx.current_substate {
            PositionSubState::Defender(ds) => ds,
            _ => return Some(self.default_substate()),
        };

        // Opponent with ball nearby + close enough → Tackling
        if ctx.opponent_with_ball_nearby && ctx.ball_distance < 3.0 && ds != Tackling {
            return Some(PositionSubState::Defender(Tackling));
        }

        // Aerial ball incoming → Heading
        if ctx.aerial_ball_incoming && ctx.ball_distance < 10.0 && ds != Heading {
            return Some(PositionSubState::Defender(Heading));
        }

        // Ball in own box + high danger → Clearing
        if ctx.ball_in_own_box && ctx.high_danger && ctx.player_has_ball {
            return Some(PositionSubState::Defender(Clearing));
        }

        // P3.6: Ball in own defensive corner → Clearing (escape corner stuck)
        // Corner zones: within 10m of corners (0,0), (0,68), (105,0), (105,68)
        // Only trigger for own defensive corners (based on is_home)
        if ctx.player_has_ball {
            let ball_pos = ctx.ball_position;
            let in_corner = (ball_pos.0 < 10.0 || ball_pos.0 > 95.0)
                && (ball_pos.1 < 10.0 || ball_pos.1 > 58.0);
            // Check if it's our defensive corner (home defends left, away defends right)
            let in_own_corner = if ctx.attacks_right {
                ball_pos.0 < 10.0 && (ball_pos.1 < 10.0 || ball_pos.1 > 58.0)
            } else {
                ball_pos.0 > 95.0 && (ball_pos.1 < 10.0 || ball_pos.1 > 58.0)
            };
            if in_corner && in_own_corner {
                return Some(PositionSubState::Defender(Clearing));
            }
        }

        // Team has ball + high line instruction → PushingUp
        if ctx.team_has_ball && ctx.instruction_high_line && ds == HoldingLine {
            return Some(PositionSubState::Defender(PushingUp));
        }

        // Lost possession + out of position → TrackingBack
        if !ctx.team_has_ball && !ctx.at_defensive_position && ctx.high_danger {
            return Some(PositionSubState::Defender(TrackingBack));
        }

        // Phase 2: Refactored Pressing Transition
        // HoldingLine/Covering pressing triggers live in their state handlers.
        if matches!(ds, HoldingLine) {
            use crate::engine::behaviors::defender_holding_line::DefenderHoldingLineState;
            use crate::engine::behaviors::traits::StateBehavior;
            if let Some(new_state) = DefenderHoldingLineState.try_fast_transition(ctx) {
                return Some(new_state);
            }
        }
        if matches!(ds, Covering) {
            use crate::engine::behaviors::defender_covering::DefenderCoveringState;
            use crate::engine::behaviors::traits::StateBehavior;
            if let Some(new_state) = DefenderCoveringState.try_fast_transition(ctx) {
                return Some(new_state);
            }
        }



        // Pass interception opportunity
        if ctx.ball_distance < 5.0 && !ctx.team_has_ball && !ctx.opponent_with_ball_nearby {
            return Some(PositionSubState::Defender(Intercepting));
        }

        // Default: return to holding line when recovered
        if ctx.at_defensive_position
            && !ctx.opponent_with_ball_nearby
            && matches!(ds, TrackingBack | Pressing | Tackling)
        {
            return Some(PositionSubState::Defender(HoldingLine));
        }

        None
    }

    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        use DefenderSubState::*;

        let ds = match ctx.current_substate {
            PositionSubState::Defender(ds) => ds,
            _ => return (0.0, 0.0),
        };

        match ds {
            HoldingLine => {
                // Delegate to StateBehavior implementation
                use crate::engine::behaviors::defender_holding_line::DefenderHoldingLineState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderHoldingLineState.calculate_velocity(ctx)
            }
            Covering => {
                // Delegate to StateBehavior implementation
                use crate::engine::behaviors::defender_covering::DefenderCoveringState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderCoveringState.calculate_velocity(ctx)
            }
            MarkingTight => {
                // PoC: Delegate to Trait-based State Object
                // This demonstrates the new architecture where logic is encapsulated per state
                use crate::engine::behaviors::traits::StateBehavior;
                use crate::engine::behaviors::defender_marking::DefenderMarkingState;
                DefenderMarkingState.calculate_velocity(ctx)
            }
            Pressing => {
                // Phase 2: Delegate to DefenderPressingState
                use crate::engine::behaviors::traits::StateBehavior;
                use crate::engine::behaviors::defender_pressing::DefenderPressingState;
                DefenderPressingState.calculate_velocity(ctx)
            }
            TrackingBack => {
                use crate::engine::behaviors::defender_tracking_back::DefenderTrackingBackState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderTrackingBackState.calculate_velocity(ctx)
            }

            PushingUp => {
                use crate::engine::behaviors::defender_pushing_up::DefenderPushingUpState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderPushingUpState.calculate_velocity(ctx)
            }
            Tackling => {
                use crate::engine::behaviors::defender_states::DefenderTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderTacklingState.calculate_velocity(ctx)
            }
            SlidingTackle => {
                use crate::engine::behaviors::defender_states::DefenderSlidingTackleState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderSlidingTackleState.calculate_velocity(ctx)
            }
            Heading => {
                use crate::engine::behaviors::defender_states::DefenderHeadingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderHeadingState.calculate_velocity(ctx)
            }
            Clearing => {
                use crate::engine::behaviors::defender_states::DefenderClearingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderClearingState.calculate_velocity(ctx)
            }
            Blocking => {
                use crate::engine::behaviors::defender_blocking::DefenderBlockingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderBlockingState.calculate_velocity(ctx)
            }
            OffsideTrap => {
                use crate::engine::behaviors::defender_offside_trap::DefenderOffsideTrapState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderOffsideTrapState.calculate_velocity(ctx)
            }
            Intercepting => {
                use crate::engine::behaviors::defender_intercepting::DefenderInterceptingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderInterceptingState.calculate_velocity(ctx)
            }
            Dribbling => {
                use crate::engine::behaviors::defender_states::DefenderDribblingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderDribblingState.calculate_velocity(ctx)
            }
            Passing => {
                use crate::engine::behaviors::defender_states::DefenderPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderPassingState.calculate_velocity(ctx)
            }
            Running => {
                use crate::engine::behaviors::defender_states::DefenderRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderRunningState.calculate_velocity(ctx)
            }
            Returning => {
                use crate::engine::behaviors::defender_states::DefenderReturningState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderReturningState.calculate_velocity(ctx)
            }
            Resting => {
                use crate::engine::behaviors::defender_states::DefenderRestingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderRestingState.calculate_velocity(ctx)
            }
            Standing => {
                use crate::engine::behaviors::defender_states::DefenderStandingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderStandingState.calculate_velocity(ctx)
            }
            Walking => {
                use crate::engine::behaviors::defender_states::DefenderWalkingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderWalkingState.calculate_velocity(ctx)
            }
            Jogging => {
                use crate::engine::behaviors::defender_states::DefenderJoggingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderJoggingState.calculate_velocity(ctx)
            }
        }
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        use DefenderSubState::*;

        let ds = match ctx.current_substate {
            PositionSubState::Defender(ds) => ds,
            _ => return false,
        };

        match ds {
            Tackling => {
                use crate::engine::behaviors::defender_states::DefenderTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderTacklingState.should_timeout(ctx)
            }
            SlidingTackle => {
                use crate::engine::behaviors::defender_states::DefenderSlidingTackleState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderSlidingTackleState.should_timeout(ctx)
            }
            Pressing => {
                use crate::engine::behaviors::traits::StateBehavior;
                use crate::engine::behaviors::defender_pressing::DefenderPressingState;
                DefenderPressingState.should_timeout(ctx)
            }
            TrackingBack => {
                use crate::engine::behaviors::defender_tracking_back::DefenderTrackingBackState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderTrackingBackState.should_timeout(ctx)
            }
            Heading => {
                use crate::engine::behaviors::defender_states::DefenderHeadingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderHeadingState.should_timeout(ctx)
            }
            Clearing => {
                use crate::engine::behaviors::defender_states::DefenderClearingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderClearingState.should_timeout(ctx)
            }
            Blocking => {
                use crate::engine::behaviors::defender_blocking::DefenderBlockingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderBlockingState.should_timeout(ctx)
            }
            OffsideTrap => {
                use crate::engine::behaviors::defender_offside_trap::DefenderOffsideTrapState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderOffsideTrapState.should_timeout(ctx)
            }
            Intercepting => {
                use crate::engine::behaviors::defender_intercepting::DefenderInterceptingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderInterceptingState.should_timeout(ctx)
            }
            PushingUp => {
                use crate::engine::behaviors::defender_pushing_up::DefenderPushingUpState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderPushingUpState.should_timeout(ctx)
            }
            Dribbling => {
                use crate::engine::behaviors::defender_states::DefenderDribblingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderDribblingState.should_timeout(ctx)
            }
            Passing => {
                use crate::engine::behaviors::defender_states::DefenderPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderPassingState.should_timeout(ctx)
            }
            Running => {
                use crate::engine::behaviors::defender_states::DefenderRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderRunningState.should_timeout(ctx)
            }
            Returning => {
                use crate::engine::behaviors::defender_states::DefenderReturningState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderReturningState.should_timeout(ctx)
            }
            Resting => {
                use crate::engine::behaviors::defender_states::DefenderRestingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderRestingState.should_timeout(ctx)
            }
            // P4.4: Standing exits when ball approaches or pressure builds
            Standing => {
                use crate::engine::behaviors::defender_states::DefenderStandingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderStandingState.should_timeout(ctx)
            }
            // P4.4: Walking exits when action is needed
            Walking => {
                use crate::engine::behaviors::defender_states::DefenderWalkingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderWalkingState.should_timeout(ctx)
            }
            // P5.1: Jogging exits when closer action needed
            Jogging => {
                use crate::engine::behaviors::defender_states::DefenderJoggingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderJoggingState.should_timeout(ctx)
            }
            HoldingLine | Covering | MarkingTight => false,
        }
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        use DefenderSubState::*;

        let ds = match ctx.current_substate {
            PositionSubState::Defender(ds) => ds,
            _ => return self.default_substate(),
        };

        match ds {
            Tackling => {
                use crate::engine::behaviors::defender_states::DefenderTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderTacklingState.timeout_transition(ctx)
            }
            SlidingTackle => {
                use crate::engine::behaviors::defender_states::DefenderSlidingTackleState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderSlidingTackleState.timeout_transition(ctx)
            }
            Pressing => {
                use crate::engine::behaviors::defender_pressing::DefenderPressingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderPressingState.timeout_transition(ctx)
            }
            Heading => {
                use crate::engine::behaviors::defender_states::DefenderHeadingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderHeadingState.timeout_transition(ctx)
            }
            Clearing => {
                use crate::engine::behaviors::defender_states::DefenderClearingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderClearingState.timeout_transition(ctx)
            }
            Blocking => {
                use crate::engine::behaviors::defender_blocking::DefenderBlockingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderBlockingState.timeout_transition(ctx)
            }
            OffsideTrap => {
                use crate::engine::behaviors::defender_offside_trap::DefenderOffsideTrapState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderOffsideTrapState.timeout_transition(ctx)
            }
            Intercepting => {
                use crate::engine::behaviors::defender_intercepting::DefenderInterceptingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderInterceptingState.timeout_transition(ctx)
            }
            TrackingBack => {
                use crate::engine::behaviors::defender_tracking_back::DefenderTrackingBackState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderTrackingBackState.timeout_transition(ctx)
            }
            Returning => {
                use crate::engine::behaviors::defender_states::DefenderReturningState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderReturningState.timeout_transition(ctx)
            }
            PushingUp => {
                use crate::engine::behaviors::defender_pushing_up::DefenderPushingUpState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderPushingUpState.timeout_transition(ctx)
            }
            Dribbling => {
                use crate::engine::behaviors::defender_states::DefenderDribblingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderDribblingState.timeout_transition(ctx)
            }
            Passing => {
                use crate::engine::behaviors::defender_states::DefenderPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderPassingState.timeout_transition(ctx)
            }
            Running => {
                use crate::engine::behaviors::defender_states::DefenderRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderRunningState.timeout_transition(ctx)
            }
            Resting => {
                use crate::engine::behaviors::defender_states::DefenderRestingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderRestingState.timeout_transition(ctx)
            }
            // P4.4/P5.1: Standing/Walking/Jogging → return to active defense
            Standing => {
                use crate::engine::behaviors::defender_states::DefenderStandingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderStandingState.timeout_transition(ctx)
            }
            Walking => {
                use crate::engine::behaviors::defender_states::DefenderWalkingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderWalkingState.timeout_transition(ctx)
            }
            Jogging => {
                use crate::engine::behaviors::defender_states::DefenderJoggingState;
                use crate::engine::behaviors::traits::StateBehavior;
                DefenderJoggingState.timeout_transition(ctx)
            }
            HoldingLine | Covering | MarkingTight => self.default_substate(),
        }
    }

    fn default_substate(&self) -> PositionSubState {
        PositionSubState::defender_default()
    }

    /// FIX_2512 Phase 13: Slow evaluation for defenders
    ///
    /// Defenders use slow evaluation for:
    /// - Pass target selection (when building from the back)
    /// - Pressing intensity (joining the press vs holding line)
    fn process_slow(
        &self,
        ctx: &crate::engine::slow_evaluation::SlowContext,
    ) -> crate::engine::slow_evaluation::SlowEvaluationResult {
        use crate::engine::slow_evaluation::{
            PassTargetEvaluator, PressingEvaluator, SlowEvaluationResult,
        };

        let mut result = SlowEvaluationResult::default();

        // 1. Pass target evaluation (if has ball - building from back)
        if ctx.core.player_has_ball {
            let targets = PassTargetEvaluator::evaluate_targets(ctx);
            result.best_pass_target = targets.first().copied();
        }

        // 2. Pressing evaluation (if opponent has ball)
        // Defenders only press when ball is in their zone
        if !ctx.core.team_has_ball {
            result.pressing_intensity = PressingEvaluator::evaluate_pressing(ctx);

            // Defenders have higher threshold - only press when really needed
            if result.pressing_intensity > 0.85 {
                result.suggested_state =
                    Some(PositionSubState::Defender(DefenderSubState::Pressing));
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
    fn test_tackling_transition() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::HoldingLine);
        ctx.opponent_with_ball_nearby = true;
        ctx.ball_distance = 2.0;

        let result = handler.try_fast_transition(&ctx);
        assert_eq!(
            result,
            Some(PositionSubState::Defender(DefenderSubState::Tackling))
        );
    }

    #[test]
    fn test_tracking_back_transition() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::HoldingLine);
        ctx.team_has_ball = false;
        ctx.at_defensive_position = false;
        ctx.high_danger = true;

        let result = handler.try_fast_transition(&ctx);
        assert_eq!(
            result,
            Some(PositionSubState::Defender(DefenderSubState::TrackingBack))
        );
    }

    #[test]
    fn test_tackling_timeout() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::Tackling);
        ctx.in_substate_ticks = 21;
        ctx.at_defensive_position = true;

        assert!(handler.should_timeout(&ctx));
        assert_eq!(
            handler.timeout_transition(&ctx),
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        );
    }

    // P4.3: Velocity tests for defender states

    #[test]
    fn test_velocity_holding_line() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::HoldingLine);
        ctx.player_position = (50.0, 30.0);
        ctx.defensive_line_y = 35.0; // Line is ahead

        let vel = handler.calculate_velocity(&ctx);
        // Should move toward defensive line (positive y)
        assert!(vel.1 > 0.0, "Should move toward defensive line");
        assert!(vel.1.abs() <= 2.0, "Velocity should be clamped");
    }

    #[test]
    fn test_velocity_tracking_back() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::TrackingBack);
        ctx.player_position = (50.0, 50.0);
        ctx.own_goal = (0.0, field::CENTER_Y);

        let vel = handler.calculate_velocity(&ctx);
        let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
        // TrackingBack should be fast (sprint toward goal)
        assert!(speed > 5.0, "TrackingBack should be sprint speed: {}", speed);
    }

    #[test]
    fn test_velocity_pressing() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::Pressing);
        ctx.player_position = (40.0, 30.0);
        ctx.ball_position = (50.0, 35.0);

        let vel = handler.calculate_velocity(&ctx);
        // Should move toward ball
        assert!(vel.0 > 0.0, "Should move toward ball x");
        assert!(vel.1 > 0.0, "Should move toward ball y");
    }

    #[test]
    fn test_velocity_resting_is_zero() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::Resting);

        let vel = handler.calculate_velocity(&ctx);
        assert_eq!(vel, (0.0, 0.0), "Resting should have zero velocity");
    }

    #[test]
    fn test_velocity_clearing_is_stationary() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::Clearing);

        let vel = handler.calculate_velocity(&ctx);
        assert_eq!(vel, (0.0, 0.0), "Clearing should be stationary");
    }

    #[test]
    fn test_velocity_passing_is_stationary() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::Passing);

        let vel = handler.calculate_velocity(&ctx);
        assert_eq!(vel, (0.0, 0.0), "Passing should be stationary");
    }

    #[test]
    fn test_timeout_transition_to_covering() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::Pressing);
        ctx.in_substate_ticks = 61;
        ctx.at_defensive_position = false;
        ctx.high_danger = false;
        ctx.ball_distance = 20.0;  // Ball far enough to trigger Covering (not MarkingTight)

        assert!(handler.should_timeout(&ctx));
        assert_eq!(
            handler.timeout_transition(&ctx),
            PositionSubState::Defender(DefenderSubState::Covering)
        );
    }

    #[test]
    fn test_resting_no_timeout() {
        let handler = DefenderHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Defender(DefenderSubState::Resting);
        ctx.in_substate_ticks = 1000; // Long duration

        // Resting should not timeout (stamina-based exit)
        assert!(!handler.should_timeout(&ctx));
    }
}
