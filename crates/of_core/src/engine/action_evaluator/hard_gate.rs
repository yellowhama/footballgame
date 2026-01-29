//! Hard Gate Filter
//!
//! FIX_2601/0108: UAE 점수 계산 전에 불가능한 액션을 제거
//!
//! 핵심 원칙: Hard Gate 실패 = 후보에서 제거 (Safety=0이 아님)

use super::types::{Action, PlayerId};
use crate::engine::physics_constants::field;

/// Hard Gate 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardGate {
    /// 오프사이드 위치
    Offside,

    /// GK에게 백패스 (팀원이 찬 공)
    BackpassToGK,

    /// 피치 밖으로 나감
    OutOfBounds,

    /// 선수 사용 불가 (부상/퇴장)
    PlayerUnavailable,

    /// 핸드볼 (GK가 페널티 에어리어 밖에서 공 핸들링)
    /// NOTE: 현재 액션 세트(Shoot/Pass/Dribble 등)는 발 액션만 포함
    /// GK 전용 액션(Catch/Throw) 추가 시 활성화 예정
    Handball,

    /// 이미 같은 타겟을 다른 선수가 담당 (TeamCoord에서 사용)
    AlreadyClaimed,
}

impl HardGate {
    /// 액션에 대해 Hard Gate 체크
    /// 통과하면 None, 실패하면 Some(HardGate)
    pub fn check(action: &Action, ctx: &HardGateContext) -> Option<HardGate> {
        // 1. 선수 상태 체크 (모든 액션에 적용)
        if ctx.player_injured || ctx.player_sent_off {
            return Some(HardGate::PlayerUnavailable);
        }

        match action {
            // 오프사이드 체크 (Run, ThroughBall)
            Action::RunIntoSpace { target } => {
                if ctx.is_offside_position(target.x, target.y) {
                    return Some(HardGate::Offside);
                }
            }
            Action::ThroughBall { target_id } => {
                if ctx.is_target_offside(*target_id) {
                    return Some(HardGate::Offside);
                }
            }

            // 백패스 체크
            Action::Pass { target_id } => {
                if ctx.is_target_gk(*target_id) && ctx.ball_was_kicked_by_teammate {
                    return Some(HardGate::BackpassToGK);
                }
            }

            // 드리블 방향이 피치 밖
            Action::Dribble { direction } => {
                let new_x = ctx.player_x + direction.x * 5.0; // 5m 전진 가정
                let new_y = ctx.player_y + direction.y * 5.0;
                if new_x < 0.0
                    || new_x > field::LENGTH_M
                    || new_y < 0.0
                    || new_y > field::WIDTH_M
                {
                    return Some(HardGate::OutOfBounds);
                }
            }

            // 런 목표가 피치 밖
            Action::RunSupport { target_space } => {
                if target_space.x < 0.0
                    || target_space.x > field::LENGTH_M
                    || target_space.y < 0.0
                    || target_space.y > field::WIDTH_M
                {
                    return Some(HardGate::OutOfBounds);
                }
            }

            // 다른 액션들은 기본적으로 통과
            _ => {}
        }

        None
    }

    /// 에러 메시지
    pub fn message(&self) -> &'static str {
        match self {
            HardGate::Offside => "Offside position",
            HardGate::BackpassToGK => "Backpass to GK not allowed",
            HardGate::OutOfBounds => "Target is out of bounds",
            HardGate::PlayerUnavailable => "Player is injured or sent off",
            HardGate::Handball => "Handball",
            HardGate::AlreadyClaimed => "Target already claimed by teammate",
        }
    }
}

/// Hard Gate 체크에 필요한 컨텍스트
#[derive(Debug, Clone)]
pub struct HardGateContext {
    // 선수 상태
    pub player_injured: bool,
    pub player_sent_off: bool,
    pub player_x: f32,
    pub player_y: f32,

    // 공 상태
    pub ball_was_kicked_by_teammate: bool,

    // 팀 정보
    pub is_home: bool,
    pub attacks_right: bool,

    // 오프사이드 라인 (수비수 위치 기반)
    pub offside_line_x: f32,

    // GK 정보
    pub gk_id: PlayerId,

    // 타겟 선수 위치 (ID -> 위치 매핑용 클로저는 외부에서)
    pub target_positions: Vec<(PlayerId, f32, f32)>, // (id, x, y)
}

impl HardGateContext {
    /// 해당 위치가 오프사이드인지 확인
    pub fn is_offside_position(&self, x: f32, _y: f32) -> bool {
        if self.attacks_right {
            x > self.offside_line_x
        } else {
            x < self.offside_line_x
        }
    }

    /// 타겟 선수가 오프사이드인지 확인
    pub fn is_target_offside(&self, target_id: PlayerId) -> bool {
        if let Some((_, x, _)) = self.target_positions.iter().find(|(id, _, _)| *id == target_id) {
            self.is_offside_position(*x, 0.0)
        } else {
            false
        }
    }

    /// 타겟이 GK인지 확인
    pub fn is_target_gk(&self, target_id: PlayerId) -> bool {
        target_id == self.gk_id
    }
}

impl Default for HardGateContext {
    fn default() -> Self {
        Self {
            player_injured: false,
            player_sent_off: false,
            player_x: field::CENTER_X,
            player_y: field::CENTER_Y,
            ball_was_kicked_by_teammate: false,
            is_home: true,
            attacks_right: true,
            offside_line_x: 90.0,
            gk_id: PlayerId::default(),
            target_positions: vec![],
        }
    }
}

/// 액션 목록에서 Hard Gate 실패한 것들 필터링
pub fn filter_by_hard_gate(actions: &mut Vec<Action>, ctx: &HardGateContext) {
    actions.retain(|action| HardGate::check(action, ctx).is_none());
}

/// 액션 목록에서 Hard Gate 결과와 함께 반환
pub fn check_all_hard_gates(
    actions: &[Action],
    ctx: &HardGateContext,
) -> Vec<(Action, Option<HardGate>)> {
    actions
        .iter()
        .map(|action| (action.clone(), HardGate::check(action, ctx)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::action_evaluator::types::{Position, Vec2};

    #[test]
    fn test_offside_check() {
        let ctx = HardGateContext {
            attacks_right: true,
            offside_line_x: 80.0,
            ..Default::default()
        };

        // 오프사이드 위치
        let action = Action::RunIntoSpace {
            target: Position::new(85.0, field::CENTER_Y),
        };
        assert_eq!(HardGate::check(&action, &ctx), Some(HardGate::Offside));

        // 온사이드 위치
        let action = Action::RunIntoSpace {
            target: Position::new(75.0, field::CENTER_Y),
        };
        assert_eq!(HardGate::check(&action, &ctx), None);
    }

    #[test]
    fn test_backpass_check() {
        let gk_id = PlayerId::new(1);
        let ctx = HardGateContext {
            gk_id,
            ball_was_kicked_by_teammate: true,
            target_positions: vec![(gk_id, 5.0, field::CENTER_Y)],
            ..Default::default()
        };

        // GK에게 백패스 (불가)
        let action = Action::Pass { target_id: gk_id };
        assert_eq!(HardGate::check(&action, &ctx), Some(HardGate::BackpassToGK));

        // 다른 선수에게 패스 (허용)
        let other_id = PlayerId::new(2);
        let action = Action::Pass { target_id: other_id };
        assert_eq!(HardGate::check(&action, &ctx), None);
    }

    #[test]
    fn test_out_of_bounds() {
        let ctx = HardGateContext {
            player_x: 102.0, // 102 + 5 = 107 > 105
            player_y: field::CENTER_Y,
            ..Default::default()
        };

        // 피치 밖으로 드리블
        let action = Action::Dribble {
            direction: Vec2::new(1.0, 0.0),
        };
        assert_eq!(HardGate::check(&action, &ctx), Some(HardGate::OutOfBounds));
    }

    #[test]
    fn test_player_unavailable() {
        let ctx = HardGateContext {
            player_injured: true,
            ..Default::default()
        };

        let action = Action::Shoot;
        assert_eq!(
            HardGate::check(&action, &ctx),
            Some(HardGate::PlayerUnavailable)
        );
    }
}
