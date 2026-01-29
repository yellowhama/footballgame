//! Player State Machine
//!
//! P7 Spec Section 3: 선수의 현재 상태 (행동 가능 여부 결정)
//!
//! ## 상태 전이 규칙
//! ```text
//! Idle → Moving/Sprinting/InAction
//! InAction → Recovering/Staggered/Idle
//! Recovering → Idle
//! Staggered → Recovering → Idle
//! Cooldown → Idle (시간 경과 후)
//! ```

use serde::{Deserialize, Serialize};
// P0: Core types moved to action_queue
use super::action_queue::PhaseActionType;

/// 선수의 현재 상태
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum PlayerState {
    /// 자유 상태 (어떤 행동이든 가능)
    #[default]
    Idle,

    /// 이동 중
    Moving { target: (f32, f32) },

    /// 스프린트 중
    Sprinting { target: (f32, f32) },

    /// 액션 실행 중 (태클, 패스, 슈팅 등)
    InAction { action_id: u64 },

    /// 넘어짐/충돌 후 회복 중
    Recovering { remaining_ticks: u8 },

    /// 비틀거림 (균형 잃음)
    Staggered { remaining_ticks: u8 },

    /// 쿨다운 (특정 행동 불가)
    Cooldown { action_type: PhaseActionType, remaining_ticks: u8 },

    /// 부상
    Injured,

    /// 퇴장 (레드카드)
    SentOff,
}

impl PlayerState {
    /// 새 액션을 시작할 수 있는지
    pub fn can_start_action(&self) -> bool {
        matches!(self, PlayerState::Idle | PlayerState::Moving { .. })
    }

    /// 이동할 수 있는지
    pub fn can_move(&self) -> bool {
        matches!(
            self,
            PlayerState::Idle
                | PlayerState::Moving { .. }
                | PlayerState::Sprinting { .. }
                | PlayerState::Cooldown { .. }
        )
    }

    /// 스프린트할 수 있는지
    pub fn can_sprint(&self) -> bool {
        matches!(self, PlayerState::Idle | PlayerState::Moving { .. })
    }

    /// 태클할 수 있는지 (쿨다운 체크 포함)
    pub fn can_tackle(&self) -> bool {
        match self {
            PlayerState::Idle | PlayerState::Moving { .. } => true,
            PlayerState::Cooldown { action_type, .. } => {
                // 태클 쿨다운 중이면 태클 불가
                *action_type != PhaseActionType::Tackle
            }
            _ => false,
        }
    }

    /// 패스할 수 있는지
    pub fn can_pass(&self) -> bool {
        matches!(self, PlayerState::Idle | PlayerState::Moving { .. })
    }

    /// 슛할 수 있는지
    pub fn can_shoot(&self) -> bool {
        matches!(self, PlayerState::Idle | PlayerState::Moving { .. })
    }

    /// 드리블할 수 있는지
    pub fn can_dribble(&self) -> bool {
        matches!(self, PlayerState::Idle | PlayerState::Moving { .. })
    }

    /// 활성 상태인지 (자유롭게 움직일 수 없는 상태)
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            PlayerState::InAction { .. }
                | PlayerState::Recovering { .. }
                | PlayerState::Staggered { .. }
                | PlayerState::Injured
                | PlayerState::SentOff
        )
    }

    /// 충돌 시 밀릴 수 있는지 (InAction 상태에서는 밀리지 않음)
    pub fn can_be_pushed(&self) -> bool {
        !matches!(self, PlayerState::InAction { .. } | PlayerState::Injured | PlayerState::SentOff)
    }

    /// 매 틱 상태 업데이트
    ///
    /// remaining_ticks를 감소시키고, 0이 되면 다음 상태로 전환
    pub fn tick_update(&mut self) {
        match self {
            PlayerState::Recovering { remaining_ticks } => {
                if *remaining_ticks > 0 {
                    *remaining_ticks -= 1;
                }
                if *remaining_ticks == 0 {
                    *self = PlayerState::Idle;
                }
            }

            PlayerState::Staggered { remaining_ticks } => {
                if *remaining_ticks > 0 {
                    *remaining_ticks -= 1;
                }
                if *remaining_ticks == 0 {
                    // Staggered → Recovering (4틱 추가 회복)
                    *self = PlayerState::Recovering { remaining_ticks: 4 };
                }
            }

            PlayerState::Cooldown { remaining_ticks, .. } => {
                if *remaining_ticks > 0 {
                    *remaining_ticks -= 1;
                }
                if *remaining_ticks == 0 {
                    *self = PlayerState::Idle;
                }
            }

            // 다른 상태는 틱 업데이트에서 자동 전환 없음
            _ => {}
        }
    }

    /// InAction 상태로 전환
    pub fn start_action(&mut self, action_id: u64) {
        *self = PlayerState::InAction { action_id };
    }

    /// 액션 완료 후 Recovering 상태로 전환
    pub fn finish_action_recovering(&mut self, recovery_ticks: u8) {
        *self = PlayerState::Recovering { remaining_ticks: recovery_ticks };
    }

    /// 액션 완료 후 Cooldown 상태로 전환
    pub fn finish_action_cooldown(&mut self, action_type: PhaseActionType, cooldown_ticks: u8) {
        *self = PlayerState::Cooldown { action_type, remaining_ticks: cooldown_ticks };
    }

    /// 액션 완료 후 바로 Idle로 전환
    pub fn finish_action_idle(&mut self) {
        *self = PlayerState::Idle;
    }

    /// Staggered 상태로 전환 (충돌, 태클 당함 등)
    pub fn stagger(&mut self, ticks: u8) {
        *self = PlayerState::Staggered { remaining_ticks: ticks };
    }

    /// 이동 시작
    pub fn start_moving(&mut self, target: (f32, f32)) {
        *self = PlayerState::Moving { target };
    }

    /// 스프린트 시작
    pub fn start_sprinting(&mut self, target: (f32, f32)) {
        *self = PlayerState::Sprinting { target };
    }

    /// 이동 완료
    pub fn stop_moving(&mut self) {
        if matches!(self, PlayerState::Moving { .. } | PlayerState::Sprinting { .. }) {
            *self = PlayerState::Idle;
        }
    }

    /// 남은 틱 수 반환 (해당하는 상태만)
    pub fn remaining_ticks(&self) -> Option<u8> {
        match self {
            PlayerState::Recovering { remaining_ticks } => Some(*remaining_ticks),
            PlayerState::Staggered { remaining_ticks } => Some(*remaining_ticks),
            PlayerState::Cooldown { remaining_ticks, .. } => Some(*remaining_ticks),
            _ => None,
        }
    }

    /// 상태 이름 문자열 반환
    pub fn name(&self) -> &'static str {
        match self {
            PlayerState::Idle => "Idle",
            PlayerState::Moving { .. } => "Moving",
            PlayerState::Sprinting { .. } => "Sprinting",
            PlayerState::InAction { .. } => "InAction",
            PlayerState::Recovering { .. } => "Recovering",
            PlayerState::Staggered { .. } => "Staggered",
            PlayerState::Cooldown { .. } => "Cooldown",
            PlayerState::Injured => "Injured",
            PlayerState::SentOff => "SentOff",
        }
    }
}

/// 선수 상태 배열 (22명)
pub type PlayerStates = [PlayerState; 22];

/// 기본 선수 상태 배열 생성
pub fn default_player_states() -> PlayerStates {
    [PlayerState::Idle; 22]
}

/// 모든 선수의 상태 틱 업데이트
pub fn tick_update_all(states: &mut PlayerStates) {
    for state in states.iter_mut() {
        state.tick_update();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn test_default_state() {
        let state = PlayerState::default();
        assert_eq!(state, PlayerState::Idle);
    }

    #[test]
    fn test_can_start_action() {
        assert!(PlayerState::Idle.can_start_action());
        assert!(PlayerState::Moving { target: (0.0, 0.0) }.can_start_action());
        assert!(!PlayerState::InAction { action_id: 1 }.can_start_action());
        assert!(!PlayerState::Recovering { remaining_ticks: 5 }.can_start_action());
    }

    #[test]
    fn test_can_move() {
        assert!(PlayerState::Idle.can_move());
        assert!(PlayerState::Moving { target: (0.0, 0.0) }.can_move());
        assert!(PlayerState::Cooldown {
            action_type: PhaseActionType::Tackle,
            remaining_ticks: 10
        }
        .can_move());
        assert!(!PlayerState::Recovering { remaining_ticks: 5 }.can_move());
        assert!(!PlayerState::InAction { action_id: 1 }.can_move());
    }

    #[test]
    fn test_can_tackle_cooldown() {
        assert!(PlayerState::Idle.can_tackle());

        // 태클 쿨다운 중이면 태클 불가
        assert!(!PlayerState::Cooldown {
            action_type: PhaseActionType::Tackle,
            remaining_ticks: 10
        }
        .can_tackle());

        // 패스 쿨다운 중이면 태클 가능
        assert!(PlayerState::Cooldown { action_type: PhaseActionType::Pass, remaining_ticks: 10 }
            .can_tackle());
    }

    #[test]
    fn test_tick_update_recovering() {
        let mut state = PlayerState::Recovering { remaining_ticks: 3 };

        state.tick_update();
        assert_eq!(state, PlayerState::Recovering { remaining_ticks: 2 });

        state.tick_update();
        assert_eq!(state, PlayerState::Recovering { remaining_ticks: 1 });

        state.tick_update();
        assert_eq!(state, PlayerState::Idle);
    }

    #[test]
    fn test_tick_update_staggered() {
        let mut state = PlayerState::Staggered { remaining_ticks: 2 };

        state.tick_update();
        assert_eq!(state, PlayerState::Staggered { remaining_ticks: 1 });

        state.tick_update();
        // Staggered → Recovering
        assert_eq!(state, PlayerState::Recovering { remaining_ticks: 4 });
    }

    #[test]
    fn test_tick_update_cooldown() {
        let mut state =
            PlayerState::Cooldown { action_type: PhaseActionType::Tackle, remaining_ticks: 2 };

        state.tick_update();
        assert!(matches!(state, PlayerState::Cooldown { remaining_ticks: 1, .. }));

        state.tick_update();
        assert_eq!(state, PlayerState::Idle);
    }

    #[test]
    fn test_action_lifecycle() {
        let mut state = PlayerState::Idle;

        // 액션 시작
        state.start_action(123);
        assert_eq!(state, PlayerState::InAction { action_id: 123 });

        // 액션 완료 → Recovering
        state.finish_action_recovering(8);
        assert_eq!(state, PlayerState::Recovering { remaining_ticks: 8 });

        // 8틱 후 Idle
        for _ in 0..8 {
            state.tick_update();
        }
        assert_eq!(state, PlayerState::Idle);
    }

    #[test]
    fn test_stagger() {
        let mut state = PlayerState::Moving { target: (50.0, field::CENTER_Y) };

        state.stagger(4);
        assert_eq!(state, PlayerState::Staggered { remaining_ticks: 4 });

        // 4틱 Staggered + 4틱 Recovering
        for _ in 0..4 {
            state.tick_update();
        }
        assert!(matches!(state, PlayerState::Recovering { .. }));

        for _ in 0..4 {
            state.tick_update();
        }
        assert_eq!(state, PlayerState::Idle);
    }

    #[test]
    fn test_default_player_states() {
        let states = default_player_states();
        assert_eq!(states.len(), 22);
        for state in states.iter() {
            assert_eq!(*state, PlayerState::Idle);
        }
    }

    #[test]
    fn test_tick_update_all() {
        let mut states = default_player_states();
        states[0] = PlayerState::Recovering { remaining_ticks: 2 };
        states[5] =
            PlayerState::Cooldown { action_type: PhaseActionType::Tackle, remaining_ticks: 1 };

        tick_update_all(&mut states);

        assert_eq!(states[0], PlayerState::Recovering { remaining_ticks: 1 });
        assert_eq!(states[5], PlayerState::Idle);
    }
}
