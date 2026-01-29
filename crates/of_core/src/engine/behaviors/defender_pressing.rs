use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{DefenderSubState, PositionSubState};

pub struct DefenderPressingState;

impl StateBehavior for DefenderPressingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        // Simple pressing logic: Sprint towards ball carrier
        let dir = direction_to(ctx.player_position, ctx.ball_position);
        
        // P2: Uses sprint speed (typically higher than jog)
        // Can be modulated by fatigue or aggregation prevention later.
        let speed = 5.0; 
        
        (dir.0 * speed, dir.1 * speed)
    }

    fn try_fast_transition(&self, _ctx: &PositionContext) -> Option<PositionSubState> {
        // Pressing takes priority until timeout or tackle.
        // Fast transitions out of pressing are usually to Tackling.
        
        // Logic moved to DefenderHandler::try_fast_transition
        // But if we want self-contained logic:
        /*
        if ctx.ball_distance < 2.0 {
            return Some(PositionSubState::Defender(DefenderSubState::Tackling));
        }
        */
        None
    }

    fn should_timeout(&self, ctx: &PositionContext) -> bool {
        // P2: Timeout based on Levers
        
        // 1. Give up if ball moves too far away (failed press)
        if ctx.ball_distance > ctx.levers.press_giveup_distance {
            return true;
        }

        // 2. Give up if pressed for too long (stamina conservation)
        if ctx.in_substate_ticks > ctx.levers.max_press_ticks {
            return true;
        }
        
        false
    }

    fn timeout_transition(&self, ctx: &PositionContext) -> PositionSubState {
        // Return to marking/covering based on ball position
        if ctx.ball_distance < 15.0 {
            PositionSubState::Defender(DefenderSubState::MarkingTight)
        } else {
            PositionSubState::Defender(DefenderSubState::Covering)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::behaviors::levers::BehaviorLevers;

    #[test]
    fn test_pressing_velocity() {
        let state = DefenderPressingState;
        let mut ctx = PositionContext::default();
        ctx.player_position = (50.0, 50.0);
        ctx.ball_position = (60.0, 60.0); // Northeast

        let (vx, vy) = state.calculate_velocity(&ctx);
        assert!(vx > 0.0);
        assert!(vy > 0.0);
    }

    #[test]
    fn test_timeout_distance() {
        let state = DefenderPressingState;
        let mut ctx = PositionContext::default();
        ctx.levers = BehaviorLevers {
            press_giveup_distance: 20.0,
            ..Default::default()
        };
        ctx.ball_distance = 21.0; // Consistently far

        assert!(state.should_timeout(&ctx));
    }

    #[test]
    fn test_timeout_duration() {
        let state = DefenderPressingState;
        let mut ctx = PositionContext::default();
        ctx.levers = BehaviorLevers {
            max_press_ticks: 60,
            press_giveup_distance: 100.0, // Far enough
            ..Default::default()
        };
        ctx.in_substate_ticks = 61;

        assert!(state.should_timeout(&ctx));
    }
}
