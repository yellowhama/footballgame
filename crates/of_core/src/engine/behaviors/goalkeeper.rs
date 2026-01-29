//! Goalkeeper-specific behavior handler
//!
//! Handles states like: Attentive, Positioning, PreparingForSave, Diving,
//! Catching, Punching, Sweeping, Distributing, HoldingBall, ReturningToGoal

use crate::engine::position_behavior::{PositionContext, PositionStateHandler};
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

/// Handler for goalkeeper position
pub struct GoalkeeperHandler;

impl PositionStateHandler for GoalkeeperHandler {
    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        use GoalkeeperSubState::*;

        let gs = match ctx.current_substate {
            PositionSubState::Goalkeeper(gs) => gs,
            _ => return Some(self.default_substate()),
        };

        match gs {
            Attentive => {
                use crate::engine::behaviors::goalkeeper_attentive::GoalkeeperAttentiveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperAttentiveState.try_fast_transition(ctx)
            }
            Positioning => {
                use crate::engine::behaviors::goalkeeper_positioning::GoalkeeperPositioningState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPositioningState.try_fast_transition(ctx)
            }
            PreparingForSave => {
                use crate::engine::behaviors::goalkeeper_preparing_for_save::GoalkeeperPreparingForSaveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPreparingForSaveState.try_fast_transition(ctx)
            }
            Jumping => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperJumpingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperJumpingState.try_fast_transition(ctx)
            }
            Diving => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperDivingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperDivingState.try_fast_transition(ctx)
            }
            Catching => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperCatchingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperCatchingState.try_fast_transition(ctx)
            }
            Punching => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperPunchingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPunchingState.try_fast_transition(ctx)
            }
            Sweeping => {
                use crate::engine::behaviors::goalkeeper_sweeping::GoalkeeperSweepingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperSweepingState.try_fast_transition(ctx)
            }
            ComingOut => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperComingOutState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperComingOutState.try_fast_transition(ctx)
            }
            Distributing => {
                use crate::engine::behaviors::goalkeeper_distributing::GoalkeeperDistributingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperDistributingState.try_fast_transition(ctx)
            }
            Throwing => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperThrowingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperThrowingState.try_fast_transition(ctx)
            }
            Kicking => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperKickingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperKickingState.try_fast_transition(ctx)
            }
            PickingUpBall => {
                use crate::engine::behaviors::goalkeeper_picking_up_ball::GoalkeeperPickingUpBallState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPickingUpBallState.try_fast_transition(ctx)
            }
            HoldingBall => {
                use crate::engine::behaviors::goalkeeper_holding_ball::GoalkeeperHoldingBallState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperHoldingBallState.try_fast_transition(ctx)
            }
            PenaltySave => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperPenaltySaveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPenaltySaveState.try_fast_transition(ctx)
            }
            ReturningToGoal => {
                use crate::engine::behaviors::goalkeeper_returning_to_goal::GoalkeeperReturningToGoalState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperReturningToGoalState.try_fast_transition(ctx)
            }
            Tackling => {
                use crate::engine::behaviors::goalkeeper_tackling::GoalkeeperTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperTacklingState.try_fast_transition(ctx)
            }
            UnderPressure => {
                use crate::engine::behaviors::goalkeeper_under_pressure::GoalkeeperUnderPressureState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperUnderPressureState.try_fast_transition(ctx)
            }
            Shooting => {
                use crate::engine::behaviors::goalkeeper_shooting::GoalkeeperShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperShootingState.try_fast_transition(ctx)
            }
            Running => {
                use crate::engine::behaviors::goalkeeper_running::GoalkeeperRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperRunningState.try_fast_transition(ctx)
            }
            Passing => {
                use crate::engine::behaviors::goalkeeper_passing::GoalkeeperPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPassingState.try_fast_transition(ctx)
            }
        }
    }

    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        use GoalkeeperSubState::*;

        let gs = match ctx.current_substate {
            PositionSubState::Goalkeeper(gs) => gs,
            _ => return (0.0, 0.0),
        };

        match gs {
            Positioning => {
                use crate::engine::behaviors::goalkeeper_positioning::GoalkeeperPositioningState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPositioningState.calculate_velocity(ctx)
            }
            Sweeping => {
                use crate::engine::behaviors::goalkeeper_sweeping::GoalkeeperSweepingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperSweepingState.calculate_velocity(ctx)
            }
            ComingOut => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperComingOutState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperComingOutState.calculate_velocity(ctx)
            }
            ReturningToGoal => {
                use crate::engine::behaviors::goalkeeper_returning_to_goal::GoalkeeperReturningToGoalState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperReturningToGoalState.calculate_velocity(ctx)
            }
            PreparingForSave => {
                use crate::engine::behaviors::goalkeeper_preparing_for_save::GoalkeeperPreparingForSaveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPreparingForSaveState.calculate_velocity(ctx)
            }
            PenaltySave => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperPenaltySaveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPenaltySaveState.calculate_velocity(ctx)
            }
            HoldingBall | PickingUpBall => {
                use crate::engine::behaviors::goalkeeper_holding_ball::GoalkeeperHoldingBallState;
                use crate::engine::behaviors::traits::StateBehavior;
                match gs {
                    PickingUpBall => {
                        use crate::engine::behaviors::goalkeeper_picking_up_ball::GoalkeeperPickingUpBallState;
                        GoalkeeperPickingUpBallState.calculate_velocity(ctx)
                    }
                    _ => GoalkeeperHoldingBallState.calculate_velocity(ctx),
                }
            }
            Distributing => {
                use crate::engine::behaviors::goalkeeper_distributing::GoalkeeperDistributingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperDistributingState.calculate_velocity(ctx)
            }
            Throwing => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperThrowingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperThrowingState.calculate_velocity(ctx)
            }
            Kicking => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperKickingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperKickingState.calculate_velocity(ctx)
            }
            Diving => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperDivingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperDivingState.calculate_velocity(ctx)
            }
            Catching => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperCatchingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperCatchingState.calculate_velocity(ctx)
            }
            Punching => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperPunchingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPunchingState.calculate_velocity(ctx)
            }
            Jumping => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperJumpingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperJumpingState.calculate_velocity(ctx)
            }
            Attentive => {
                use crate::engine::behaviors::goalkeeper_attentive::GoalkeeperAttentiveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperAttentiveState.calculate_velocity(ctx)
            }
            Tackling => {
                use crate::engine::behaviors::goalkeeper_tackling::GoalkeeperTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperTacklingState.calculate_velocity(ctx)
            }
            UnderPressure => {
                use crate::engine::behaviors::goalkeeper_under_pressure::GoalkeeperUnderPressureState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperUnderPressureState.calculate_velocity(ctx)
            }
            Shooting => {
                use crate::engine::behaviors::goalkeeper_shooting::GoalkeeperShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperShootingState.calculate_velocity(ctx)
            }
            Running => {
                use crate::engine::behaviors::goalkeeper_running::GoalkeeperRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperRunningState.calculate_velocity(ctx)
            }
            Passing => {
                use crate::engine::behaviors::goalkeeper_passing::GoalkeeperPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPassingState.calculate_velocity(ctx)
            }
        }
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        use GoalkeeperSubState::*;

        let gs = match ctx.current_substate {
            PositionSubState::Goalkeeper(gs) => gs,
            _ => return false,
        };

        match gs {
            // 6-second rule: must distribute after 120 ticks (6s at 20Hz)
            HoldingBall => {
                use crate::engine::behaviors::goalkeeper_holding_ball::GoalkeeperHoldingBallState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperHoldingBallState.should_timeout(ctx)
            }
            // PreparingForSave timeout if shot doesn't come
            PreparingForSave => {
                use crate::engine::behaviors::goalkeeper_preparing_for_save::GoalkeeperPreparingForSaveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPreparingForSaveState.should_timeout(ctx)
            }
            // PenaltySave timeout if shot doesn't come
            PenaltySave => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperPenaltySaveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPenaltySaveState.should_timeout(ctx)
            }
            // Sweeping ends when ball is far or caught
            Sweeping => {
                use crate::engine::behaviors::goalkeeper_sweeping::GoalkeeperSweepingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperSweepingState.should_timeout(ctx)
            }
            // ComingOut ends when ball is far or caught
            ComingOut => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperComingOutState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperComingOutState.should_timeout(ctx)
            }
            // Diving is a brief action
            Diving => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperDivingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperDivingState.should_timeout(ctx)
            }
            // Catching is a brief action
            Catching => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperCatchingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperCatchingState.should_timeout(ctx)
            }
            // Punching is a brief action
            Punching => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperPunchingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPunchingState.should_timeout(ctx)
            }
            // Jumping is a brief action
            Jumping => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperJumpingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperJumpingState.should_timeout(ctx)
            }
            // Distributing timeout
            Distributing => {
                use crate::engine::behaviors::goalkeeper_distributing::GoalkeeperDistributingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperDistributingState.should_timeout(ctx)
            }
            // Throwing timeout
            Throwing => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperThrowingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperThrowingState.should_timeout(ctx)
            }
            // Kicking timeout
            Kicking => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperKickingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperKickingState.should_timeout(ctx)
            }
            // PickingUpBall is quick action
            PickingUpBall => {
                use crate::engine::behaviors::goalkeeper_picking_up_ball::GoalkeeperPickingUpBallState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPickingUpBallState.should_timeout(ctx)
            }
            // ReturningToGoal ends when near goal
            ReturningToGoal => {
                use crate::engine::behaviors::goalkeeper_returning_to_goal::GoalkeeperReturningToGoalState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperReturningToGoalState.should_timeout(ctx)
            }
            // Positioning has no timeout
            Positioning => {
                use crate::engine::behaviors::goalkeeper_positioning::GoalkeeperPositioningState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPositioningState.should_timeout(ctx)
            }
            Attentive => {
                use crate::engine::behaviors::goalkeeper_attentive::GoalkeeperAttentiveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperAttentiveState.should_timeout(ctx)
            }
            // Tackling is brief
            Tackling => {
                use crate::engine::behaviors::goalkeeper_tackling::GoalkeeperTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperTacklingState.should_timeout(ctx)
            }
            // UnderPressure resolves when pressure drops or time expires
            UnderPressure => {
                use crate::engine::behaviors::goalkeeper_under_pressure::GoalkeeperUnderPressureState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperUnderPressureState.should_timeout(ctx)
            }
            // Shooting is brief
            Shooting => {
                use crate::engine::behaviors::goalkeeper_shooting::GoalkeeperShootingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperShootingState.should_timeout(ctx)
            }
            // Running timeout
            Running => {
                use crate::engine::behaviors::goalkeeper_running::GoalkeeperRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperRunningState.should_timeout(ctx)
            }
            // Passing is brief
            Passing => {
                use crate::engine::behaviors::goalkeeper_passing::GoalkeeperPassingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPassingState.should_timeout(ctx)
            }
        }
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        use GoalkeeperSubState::*;

        let gs = match ctx.current_substate {
            PositionSubState::Goalkeeper(gs) => gs,
            _ => return self.default_substate(),
        };

        match gs {
            HoldingBall => {
                use crate::engine::behaviors::goalkeeper_holding_ball::GoalkeeperHoldingBallState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperHoldingBallState.timeout_transition(ctx)
            }
            PreparingForSave => {
                use crate::engine::behaviors::goalkeeper_preparing_for_save::GoalkeeperPreparingForSaveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPreparingForSaveState.timeout_transition(ctx)
            }
            PenaltySave => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperPenaltySaveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPenaltySaveState.timeout_transition(ctx)
            }
            Sweeping => {
                use crate::engine::behaviors::goalkeeper_sweeping::GoalkeeperSweepingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperSweepingState.timeout_transition(ctx)
            }
            ComingOut => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperComingOutState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperComingOutState.timeout_transition(ctx)
            }
            Diving => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperDivingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperDivingState.timeout_transition(ctx)
            }
            Catching => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperCatchingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperCatchingState.timeout_transition(ctx)
            }
            Punching => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperPunchingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPunchingState.timeout_transition(ctx)
            }
            Jumping => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperJumpingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperJumpingState.timeout_transition(ctx)
            }
            PickingUpBall => {
                use crate::engine::behaviors::goalkeeper_picking_up_ball::GoalkeeperPickingUpBallState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPickingUpBallState.timeout_transition(ctx)
            }
            Distributing => {
                use crate::engine::behaviors::goalkeeper_distributing::GoalkeeperDistributingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperDistributingState.timeout_transition(ctx)
            }
            Throwing => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperThrowingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperThrowingState.timeout_transition(ctx)
            }
            Kicking => {
                use crate::engine::behaviors::goalkeeper_action_states::GoalkeeperKickingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperKickingState.timeout_transition(ctx)
            }
            ReturningToGoal => {
                use crate::engine::behaviors::goalkeeper_returning_to_goal::GoalkeeperReturningToGoalState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperReturningToGoalState.timeout_transition(ctx)
            }
            Positioning => {
                use crate::engine::behaviors::goalkeeper_positioning::GoalkeeperPositioningState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperPositioningState.timeout_transition(ctx)
            }
            Attentive => {
                use crate::engine::behaviors::goalkeeper_attentive::GoalkeeperAttentiveState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperAttentiveState.timeout_transition(ctx)
            }
            Tackling => {
                use crate::engine::behaviors::goalkeeper_tackling::GoalkeeperTacklingState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperTacklingState.timeout_transition(ctx)
            }
            UnderPressure => {
                use crate::engine::behaviors::goalkeeper_under_pressure::GoalkeeperUnderPressureState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperUnderPressureState.timeout_transition(ctx)
            }
            Shooting | Passing => {
                match gs {
                    Shooting => {
                        use crate::engine::behaviors::goalkeeper_shooting::GoalkeeperShootingState;
                        use crate::engine::behaviors::traits::StateBehavior;
                        GoalkeeperShootingState.timeout_transition(ctx)
                    }
                    _ => {
                        use crate::engine::behaviors::goalkeeper_passing::GoalkeeperPassingState;
                        use crate::engine::behaviors::traits::StateBehavior;
                        GoalkeeperPassingState.timeout_transition(ctx)
                    }
                }
            }
            Running => {
                use crate::engine::behaviors::goalkeeper_running::GoalkeeperRunningState;
                use crate::engine::behaviors::traits::StateBehavior;
                GoalkeeperRunningState.timeout_transition(ctx)
            }
        }
    }

    fn default_substate(&self) -> PositionSubState {
        PositionSubState::goalkeeper_default()
    }

    /// FIX_2512 Phase 13: Slow evaluation for goalkeepers
    ///
    /// Goalkeepers use slow evaluation for:
    /// - Pass target selection (when distributing)
    fn process_slow(
        &self,
        ctx: &crate::engine::slow_evaluation::SlowContext,
    ) -> crate::engine::slow_evaluation::SlowEvaluationResult {
        use crate::engine::slow_evaluation::{PassTargetEvaluator, SlowEvaluationResult};

        let mut result = SlowEvaluationResult::default();

        // Only evaluate pass targets when holding the ball
        if ctx.core.player_has_ball {
            let targets = PassTargetEvaluator::evaluate_targets(ctx);
            result.best_pass_target = targets.first().copied();
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn test_shot_incoming_transition() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive);
        ctx.shot_incoming = true;

        let result = handler.try_fast_transition(&ctx);
        assert_eq!(
            result,
            Some(PositionSubState::Goalkeeper(
                GoalkeeperSubState::PreparingForSave
            ))
        );
    }

    #[test]
    fn test_ball_caught_transition() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive);
        ctx.player_has_ball = true;

        let result = handler.try_fast_transition(&ctx);
        assert_eq!(
            result,
            Some(PositionSubState::Goalkeeper(GoalkeeperSubState::HoldingBall))
        );
    }

    #[test]
    fn test_holding_ball_timeout() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::HoldingBall);
        ctx.in_substate_ticks = 121;

        assert!(handler.should_timeout(&ctx));
        assert_eq!(
            handler.timeout_transition(&ctx),
            PositionSubState::Goalkeeper(GoalkeeperSubState::Distributing)
        );
    }

    // P4.3: Velocity tests for goalkeeper states

    #[test]
    fn test_velocity_attentive_positioning() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive);
        ctx.player_position = (5.0, 30.0);
        ctx.ball_position = (40.0, 40.0);
        ctx.own_goal = (0.0, field::CENTER_Y);

        let vel = handler.calculate_velocity(&ctx);
        // Attentive GK should position based on ball angle
        // Speed should be moderate
        let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
        assert!(speed <= 3.0, "Attentive positioning should be moderate speed: {}", speed);
    }

    #[test]
    fn test_velocity_diving() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::Diving);
        ctx.player_position = (3.0, field::CENTER_Y);
        ctx.ball_position = (5.0, 38.0);

        let vel = handler.calculate_velocity(&ctx);
        // Diving is an action state with no velocity modifier (controlled by animation)
        assert_eq!(vel, (0.0, 0.0), "Diving is action state with no velocity");
    }

    #[test]
    fn test_velocity_sweeping() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::Sweeping);
        ctx.player_position = (8.0, field::CENTER_Y);
        ctx.ball_position = (25.0, field::CENTER_Y);

        let vel = handler.calculate_velocity(&ctx);
        let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
        // Sweeping should be sprint speed
        assert!(speed > 6.0, "Sweeping should be sprint speed: {}", speed);
    }

    #[test]
    fn test_velocity_holding_ball_stationary() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::HoldingBall);

        let vel = handler.calculate_velocity(&ctx);
        assert_eq!(vel, (0.0, 0.0), "HoldingBall should be stationary");
    }

    #[test]
    fn test_velocity_distributing_forward_movement() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::Distributing);

        let vel = handler.calculate_velocity(&ctx);
        // Forward movement in X direction (attacks_right=true by default)
        assert_eq!(vel, (1.0, 0.0), "Distributing has forward movement in X");
    }

    #[test]
    fn test_velocity_kicking_forward_movement() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::Kicking);

        let vel = handler.calculate_velocity(&ctx);
        // Forward movement in X direction (attacks_right=true by default)
        assert_eq!(vel, (1.0, 0.0), "Kicking has forward movement in X");
    }

    #[test]
    fn test_velocity_throwing_forward_movement() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::Throwing);

        let vel = handler.calculate_velocity(&ctx);
        // Forward movement in X direction (attacks_right=true by default)
        assert_eq!(vel, (1.0, 0.0), "Throwing has forward movement in X");
    }

    #[test]
    fn test_velocity_returning_to_goal() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::ReturningToGoal);
        ctx.player_position = (20.0, 40.0);
        ctx.goal_position = (0.0, field::CENTER_Y);  // Handler uses goal_position

        let vel = handler.calculate_velocity(&ctx);
        // Should move toward goal at moderate speed
        assert!(vel.0 < 0.0, "Should move toward goal (negative x)");
        let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
        assert!(speed > 3.0, "ReturningToGoal should be moderate speed: {}", speed);
    }

    #[test]
    fn test_diving_timeout() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::Diving);
        ctx.in_substate_ticks = 21;  // Diving times out after > 20 ticks

        assert!(handler.should_timeout(&ctx), "Diving should timeout after 20 ticks");
    }

    #[test]
    fn test_preparing_for_save_transition() {
        let handler = GoalkeeperHandler;
        let mut ctx = PositionContext::default();
        ctx.current_substate = PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive);
        ctx.shot_incoming = true;
        ctx.ball_position = (15.0, 36.0);
        ctx.player_position = (3.0, field::CENTER_Y);

        let result = handler.try_fast_transition(&ctx);
        assert!(result.is_some(), "Should transition when shot incoming");
    }
}
