//! Action Set Builder
//!
//! FIX_2601/0108: 상태별 후보 액션 생성
//! 상태에 따라 후보 액션 풀이 완전히 달라짐

use super::state::{PlayerPhaseState, RoleTag};
use super::types::{Action, CrossZone, PassLane, PlayerId, Position, Vec2, Zone};
use crate::engine::physics_constants::field;

/// 액션 셋 빌더에 필요한 컨텍스트
pub struct ActionSetContext {
    // 선수 위치
    pub player_x: f32,
    pub player_y: f32,

    // 피치 정보
    pub attacks_right: bool,

    // 슛 관련
    pub in_shooting_zone: bool,
    pub has_clear_shot: bool,

    // 크로스 관련
    pub in_crossing_zone: bool,

    // 패스 타겟들
    pub pass_targets: Vec<PlayerId>,
    pub through_ball_targets: Vec<PlayerId>,
    /// FIX_2601/1128: 상호 패스 대상 (최근에 나에게 패스한 선수들)
    pub reciprocal_targets: Vec<PlayerId>,

    // 수비 관련
    pub dist_to_ball_carrier: f32,
    pub marking_target: Option<PlayerId>,
    pub most_dangerous_lane: Option<PassLane>,
    pub most_exposed_zone: Option<Zone>,

    // 역습 관련
    pub has_runner_ahead: bool,
    pub best_counter_target: Option<PlayerId>,
    pub counter_space: Option<Position>,

    // 현재 상태
    pub in_own_third: bool,
    pub under_pressure: bool,
}

impl Default for ActionSetContext {
    fn default() -> Self {
        Self {
            player_x: field::CENTER_X,
            player_y: field::CENTER_Y,
            attacks_right: true,
            in_shooting_zone: false,
            has_clear_shot: false,
            in_crossing_zone: false,
            pass_targets: vec![],
            through_ball_targets: vec![],
            reciprocal_targets: vec![],
            dist_to_ball_carrier: 50.0,
            marking_target: None,
            most_dangerous_lane: None,
            most_exposed_zone: None,
            has_runner_ahead: false,
            best_counter_target: None,
            counter_space: None,
            in_own_third: false,
            under_pressure: false,
        }
    }
}

/// 액션 셋 빌더
pub struct ActionSetBuilder;

impl ActionSetBuilder {
    /// 상태별 후보 액션 생성
    pub fn for_state(
        state: PlayerPhaseState,
        role: RoleTag,
        ctx: &ActionSetContext,
    ) -> Vec<Action> {
        match state {
            PlayerPhaseState::OnBall => Self::on_ball_actions(role, ctx),
            PlayerPhaseState::ReadyToReceive => Self::ready_to_receive_actions(ctx),
            PlayerPhaseState::OffBallAttack => Self::off_ball_attack_actions(role, ctx),
            PlayerPhaseState::DefendBallCarrier => Self::defend_ball_carrier_actions(ctx),
            PlayerPhaseState::DefendOffBallTarget => Self::defend_off_ball_actions(ctx),
            PlayerPhaseState::DefensiveShape => Self::defensive_shape_actions(role, ctx),
            PlayerPhaseState::TransitionLoss => Self::transition_loss_actions(ctx),
            PlayerPhaseState::TransitionWin => Self::transition_win_actions(role, ctx),
        }
    }

    /// OnBall 상태 액션
    /// FIX_2601/0109: Hold 제거, Dribble 통합 (캐리+돌파+홀드)
    fn on_ball_actions(role: RoleTag, ctx: &ActionSetContext) -> Vec<Action> {
        let mut actions = vec![];

        // 1. Shoot - 항상 후보에 추가 (거리로 점수 결정)
        actions.push(Action::Shoot);

        // 2. 패스 옵션
        for &target_id in &ctx.pass_targets {
            actions.push(Action::Pass { target_id });
        }

        // 3. Dribble - 기본 On-Ball 액션 (캐리+돌파+홀드 통합)
        // 전진 방향
        let forward_dir = if ctx.attacks_right {
            Vec2::new(1.0, 0.0)
        } else {
            Vec2::new(-1.0, 0.0)
        };
        actions.push(Action::Dribble { direction: forward_dir });

        // 측면 드리블 옵션 (사이드 탈출)
        if ctx.player_y < 30.0 {
            // 왼쪽에 있으면 오른쪽으로
            actions.push(Action::Dribble { direction: Vec2::new(forward_dir.x * 0.7, 0.7) });
        } else if ctx.player_y > 38.0 {
            // 오른쪽에 있으면 왼쪽으로
            actions.push(Action::Dribble { direction: Vec2::new(forward_dir.x * 0.7, -0.7) });
        }

        // 4. 역할별 추가
        match role {
            RoleTag::Creator => {
                // Creator는 크로스/스루볼 우선
                if ctx.in_crossing_zone {
                    actions.push(Action::Cross {
                        target_zone: CrossZone::FarPost,
                    });
                    actions.push(Action::Cross {
                        target_zone: CrossZone::NearPost,
                    });
                }
                for &target_id in &ctx.through_ball_targets {
                    actions.push(Action::ThroughBall { target_id });
                }
            }
            _ => {}
        }

        // 5. 위험 상황: 클리어
        if ctx.in_own_third && ctx.under_pressure {
            actions.push(Action::Clear);
        }

        actions
    }

    /// ReadyToReceive 상태 액션
    fn ready_to_receive_actions(ctx: &ActionSetContext) -> Vec<Action> {
        let mut actions = vec![];

        // 현재 위치 유지 (받을 준비)
        actions.push(Action::HoldPosition);

        // 약간 움직여서 더 좋은 각도 만들기
        actions.push(Action::Support {
            position: Position::new(ctx.player_x, ctx.player_y),
        });

        actions
    }

    /// OffBallAttack 상태 액션
    fn off_ball_attack_actions(role: RoleTag, ctx: &ActionSetContext) -> Vec<Action> {
        let mut actions = vec![];

        // 서포트 움직임
        actions.push(Action::Support {
            position: Position::new(ctx.player_x + 5.0, ctx.player_y),
        });

        match role {
            RoleTag::Runner => {
                // 공간 침투
                let target = if ctx.attacks_right {
                    Position::new(ctx.player_x + 15.0, ctx.player_y)
                } else {
                    Position::new(ctx.player_x - 15.0, ctx.player_y)
                };
                actions.push(Action::RunIntoSpace { target });
            }
            RoleTag::Outlet => {
                // 탈출구 역할
                actions.push(Action::HoldPosition);
            }
            _ => {}
        }

        // 오버래핑 런 (풀백/윙어)
        actions.push(Action::Overlap);

        actions
    }

    /// DefendBallCarrier 상태 액션
    fn defend_ball_carrier_actions(ctx: &ActionSetContext) -> Vec<Action> {
        let mut actions = vec![];

        let dist = ctx.dist_to_ball_carrier;

        if dist < 2.0 {
            actions.push(Action::Tackle);
        }
        if dist < 5.0 {
            actions.push(Action::Jockey);
        }
        if dist < 15.0 {
            actions.push(Action::Press);
        }

        // 패스레인 차단
        if let Some(lane) = ctx.most_dangerous_lane {
            actions.push(Action::BlockLane { lane });
        }

        actions
    }

    /// DefendOffBallTarget 상태 액션
    fn defend_off_ball_actions(ctx: &ActionSetContext) -> Vec<Action> {
        let mut actions = vec![];

        // 마킹
        if let Some(target_id) = ctx.marking_target {
            actions.push(Action::Mark { target_id });
        }

        // 인터셉트
        if let Some(lane) = ctx.most_dangerous_lane {
            actions.push(Action::Intercept { lane });
            actions.push(Action::BlockLane { lane });
        }

        actions
    }

    /// DefensiveShape 상태 액션
    fn defensive_shape_actions(role: RoleTag, ctx: &ActionSetContext) -> Vec<Action> {
        let mut actions = vec![];

        // 형태 유지
        actions.push(Action::HoldPosition);

        match role {
            RoleTag::Anchor => {
                // 커버
                if let Some(zone) = ctx.most_exposed_zone {
                    actions.push(Action::Cover { zone });
                }
            }
            RoleTag::Outlet => {
                // 역습 대기 (위치 유지)
                actions.push(Action::HoldPosition);
            }
            _ => {}
        }

        actions
    }

    /// TransitionLoss 상태 액션
    fn transition_loss_actions(ctx: &ActionSetContext) -> Vec<Action> {
        let mut actions = vec![];

        // 역압박
        actions.push(Action::CounterPress);

        // 지연
        actions.push(Action::Delay);

        // 긴급 커버
        if let Some(zone) = ctx.most_exposed_zone {
            actions.push(Action::CoverEmergency { zone });
        }

        actions
    }

    /// TransitionWin 상태 액션
    fn transition_win_actions(role: RoleTag, ctx: &ActionSetContext) -> Vec<Action> {
        let mut actions = vec![];

        // 역습 첫 패스
        if ctx.has_runner_ahead {
            if let Some(target_id) = ctx.best_counter_target {
                actions.push(Action::FirstPassForward { target_id });
            }
        }

        // 캐리 (역습 드리블)
        let direction = if ctx.attacks_right {
            Vec2::new(1.0, 0.0)
        } else {
            Vec2::new(-1.0, 0.0)
        };
        actions.push(Action::Carry { direction });

        // 역습 서포트 런
        if matches!(role, RoleTag::Runner) {
            if let Some(target_space) = ctx.counter_space {
                actions.push(Action::RunSupport { target_space });
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_on_ball_actions() {
        let ctx = ActionSetContext {
            in_shooting_zone: true,
            has_clear_shot: true,
            pass_targets: vec![PlayerId::new(2), PlayerId::new(3)],
            ..Default::default()
        };

        let actions = ActionSetBuilder::for_state(
            PlayerPhaseState::OnBall,
            RoleTag::Finisher,
            &ctx,
        );

        // Finisher는 슛이 첫 번째
        assert!(matches!(actions.first(), Some(Action::Shoot)));

        // 패스 옵션도 포함
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Pass { .. })));
    }

    #[test]
    fn test_defend_ball_carrier_actions() {
        let ctx = ActionSetContext {
            dist_to_ball_carrier: 3.0,
            most_dangerous_lane: Some(PassLane {
                from: (50.0, field::CENTER_Y),
                to: (70.0, field::CENTER_Y),
            }),
            ..Default::default()
        };

        let actions = ActionSetBuilder::for_state(
            PlayerPhaseState::DefendBallCarrier,
            RoleTag::Presser,
            &ctx,
        );

        // 3m 거리: Jockey, Press 포함
        assert!(actions.iter().any(|a| matches!(a, Action::Jockey)));
        assert!(actions.iter().any(|a| matches!(a, Action::Press)));

        // Tackle은 2m 이상이라 없음
        assert!(!actions.iter().any(|a| matches!(a, Action::Tackle)));
    }

    #[test]
    fn test_transition_win_actions() {
        let ctx = ActionSetContext {
            has_runner_ahead: true,
            best_counter_target: Some(PlayerId::new(9)),
            counter_space: Some(Position::new(80.0, field::CENTER_Y)),
            attacks_right: true,
            ..Default::default()
        };

        let actions = ActionSetBuilder::for_state(
            PlayerPhaseState::TransitionWin,
            RoleTag::Runner,
            &ctx,
        );

        // 역습 첫 패스 포함
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::FirstPassForward { .. })));

        // 역습 서포트 런 포함 (Runner 역할)
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::RunSupport { .. })));
    }
}
