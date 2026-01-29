//! Defender Marking State Logic
//!
//! Handles tight marking behavior.

use crate::engine::behaviors::traits::StateBehavior;
use crate::engine::position_behavior::{direction_to, PositionContext};
use crate::engine::position_substates::{DefenderSubState, PositionSubState};

pub struct DefenderMarkingState;

impl StateBehavior for DefenderMarkingState {
    fn calculate_velocity(&self, ctx: &PositionContext) -> (f32, f32) {
        // Phase 2: Use BehaviorLevers for dynamic marking distance/tightness
        let target_dist = ctx.levers.marking_distance.max(0.5); // Minimum 0.5m
        
        if ctx.ball_distance > target_dist {
            // We are too far, move closer
            let dir = direction_to(ctx.player_position, ctx.ball_position);
            
            // Stickiness determines how aggressively we close the gap
            // Base speed 3.0 (from old logic) * stickiness factor (0.5 ~ 1.5)
            // If stickiness is low, we are sluggish. If high, we are snappy.
            let stickiness_factor = 0.5 + ctx.levers.marking_stickiness; 
            let speed = 3.0 * stickiness_factor;
            
            (dir.0 * speed, dir.1 * speed)
        } else {
            // Inside marking distance
            // If stickiness is high, we might micro-adjust to stay exactly at distance
            // For now, hold position
            (0.0, 0.0)
        }
    }

    fn try_fast_transition(&self, ctx: &PositionContext) -> Option<PositionSubState> {
        // Phase 2: Use press_trigger_distance to decide when to switch to Pressing
        
        // 1. If ball comes within press trigger range, switch to Pressing
        // (Note: Pressing behavior will handle the actual sprint/tackle-seek)
        if ctx.ball_distance < ctx.levers.press_trigger_distance {
             // Note: Defender::Tackling is the old state.
             // We map Pressing logic to 'Tackling' enum for now, or 'Pressing' if available?
             // PositionSubState::Defender enum has: Returning, HoldingLine, Covering, Marking, Tackling, Pressing
             // Let's check the enum definition. Assuming 'Pressing' exists or mapping to Tackling.
             // The file imports PositionSubState::Defender(DefenderSubState::Tackling) in original code.
             // I will use Tackling if Pressing doesn't exist yet, or check enum.
             // User plan said: "Switch to Pressing".
             return Some(PositionSubState::Defender(DefenderSubState::Pressing));
        }

        // 2. If ball gets too far away (lost mark), switch to Covering
        // Use a hardcoded or lever-based "lose mark" distance?
        // Let's use 15.0m as before or maybe 2x marking distance?
        if ctx.ball_distance > 15.0 {
             return Some(PositionSubState::Defender(DefenderSubState::Covering));
        }

        None
    }

    fn should_timeout(&self, _ctx: &PositionContext) -> bool {
        false // Marking is a sustainable state until conditions change
    }

    fn timeout_transition(&self, _ctx: &PositionContext) -> PositionSubState {
        PositionSubState::Defender(DefenderSubState::HoldingLine)
    }
}
