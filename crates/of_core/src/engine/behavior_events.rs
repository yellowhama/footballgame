use crate::engine::action_detail::{ActionDetail, ActionTarget, DribbleStyle, PassType, ShotType};
use crate::engine::action_queue::{ActionQueue, ActionType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BehaviorEvent {
    RequestPass {
        passer_idx: usize,
        target_idx: usize,
        is_long: bool,
        is_through: bool,
        priority: u8,
    },
    RequestClear {
        clearer_idx: usize,
        target_idx: usize,
        power: f32,
        priority: u8,
    },
    RequestTackle {
        tackler_idx: usize,
        target_idx: usize,
        priority: u8,
    },
    RequestIntercept {
        player_idx: usize,
        ball_position: (f32, f32),
        priority: u8,
    },
    RequestShot {
        shooter_idx: usize,
        target: (f32, f32),
        power: f32,
        shot_type: ShotType,
        priority: u8,
    },
    RequestDribble {
        dribbler_idx: usize,
        direction: (f32, f32),
        aggressive: bool,
        style: DribbleStyle,
        priority: u8,
    },
}

pub struct BehaviorEventDispatcher;

impl BehaviorEventDispatcher {
    pub fn dispatch(
        queue: &mut ActionQueue,
        event: BehaviorEvent,
        execute_tick: u64,
    ) -> Option<u64> {
        match event {
            BehaviorEvent::RequestPass {
                passer_idx,
                target_idx,
                is_long,
                is_through,
                priority,
            } => Some(queue.schedule_new(
                execute_tick,
                ActionType::Pass {
                    target_idx,
                    is_long,
                    is_through,
                    intended_target_pos: None,
                    intended_passer_pos: None, // FIX_2601/0123: Added missing field
                },
                passer_idx,
                priority,
            )),
            BehaviorEvent::RequestClear {
                clearer_idx,
                target_idx,
                power,
                priority,
            } => {
                let detail = ActionDetail::for_pass(
                    PassType::Clear,
                    ActionTarget::Player(target_idx),
                    power,
                    0.0,
                );
                Some(queue.schedule_new_with_detail(
                    execute_tick,
                    ActionType::Pass {
                        target_idx,
                        is_long: true,
                        is_through: false,
                        intended_target_pos: None,
                        intended_passer_pos: None, // FIX_2601/0123: Added missing field
                    },
                    clearer_idx,
                    priority,
                    detail,
                ))
            }
            BehaviorEvent::RequestTackle { tackler_idx, target_idx, priority } => {
                Some(queue.schedule_new(
                    execute_tick,
                    ActionType::Tackle { target_idx },
                    tackler_idx,
                    priority,
                ))
            }
            BehaviorEvent::RequestIntercept {
                player_idx,
                ball_position,
                priority,
            } => Some(queue.schedule_new(
                execute_tick,
                ActionType::Intercept { ball_position },
                player_idx,
                priority,
            )),
            BehaviorEvent::RequestShot {
                shooter_idx,
                target,
                power,
                shot_type,
                priority,
            } => {
                let detail = ActionDetail::for_shot(
                    shot_type,
                    ActionTarget::GoalMouth(target.0, target.1),
                    power,
                    0.0,
                );
                Some(queue.schedule_new_with_detail(
                    execute_tick,
                    ActionType::Shot { power, target },
                    shooter_idx,
                    priority,
                    detail,
                ))
            }
            BehaviorEvent::RequestDribble {
                dribbler_idx,
                direction,
                aggressive,
                style,
                priority,
            } => {
                let detail = ActionDetail::for_dribble(style, None, aggressive);
                Some(queue.schedule_new_with_detail(
                    execute_tick,
                    ActionType::Dribble { direction, aggressive },
                    dribbler_idx,
                    priority,
                    detail,
                ))
            }
        }
    }
}
