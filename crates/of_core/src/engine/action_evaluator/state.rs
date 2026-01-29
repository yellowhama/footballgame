//! Player Phase State System
//!
//! FIX_2601/0108: 상태 기반 의사결정 시스템
//! 상태에 따라 후보 액션 풀 자체가 달라짐

use super::types::PlayerId;

/// 선수의 현재 게임 상황 상태 (8개)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PlayerPhaseState {
    // === 공격 (팀이 볼 소유) ===
    /// 내가 공을 가지고 있음
    OnBall,

    /// 공을 받을 준비 (패스 레인 + 몸 방향 OK)
    ReadyToReceive,

    /// 공은 없지만 공격 전개 중 (일반 오프볼)
    OffBallAttack,

    // === 수비 (팀이 볼 미소유) ===
    /// 볼 캐리어를 직접 압박/견제 중
    DefendBallCarrier,

    /// 공 없는 상대를 마킹/차단 중
    DefendOffBallTarget,

    /// 수비 형태 유지/커버 (공격 대기 포함)
    DefensiveShape,

    // === 전환 ===
    /// 공을 막 잃음 (0~2초) - 즉시 역압박/지연
    TransitionLoss,

    /// 공을 막 얻음 (0~2초) - 즉시 역습/전개
    TransitionWin,
}

impl PlayerPhaseState {
    /// 상태 분류
    pub fn classify(ctx: &StateContext) -> Self {
        // 1. 전환 체크 (우선순위 최고)
        if ctx.possession_changed_within_ticks(8) {
            // 약 2초 (250ms per tick)
            if ctx.team_has_ball {
                return Self::TransitionWin;
            } else {
                return Self::TransitionLoss;
            }
        }

        // 2. 팀 점유 상태
        if ctx.team_has_ball {
            // 공격 상태
            if ctx.i_have_ball {
                return Self::OnBall;
            }
            if ctx.can_receive_pass() {
                return Self::ReadyToReceive;
            }
            return Self::OffBallAttack;
        } else {
            // 수비 상태
            if ctx.am_defending_ball_carrier() {
                return Self::DefendBallCarrier;
            }
            if ctx.have_marking_target() {
                return Self::DefendOffBallTarget;
            }
            return Self::DefensiveShape;
        }
    }

    /// Off-Ball 상태인지 확인
    pub fn is_off_ball(&self) -> bool {
        !matches!(self, Self::OnBall)
    }

    /// 수비 상태인지 확인
    pub fn is_defensive(&self) -> bool {
        matches!(
            self,
            Self::DefendBallCarrier
                | Self::DefendOffBallTarget
                | Self::DefensiveShape
                | Self::TransitionLoss
        )
    }

    /// 전환 상태인지 확인
    pub fn is_transition(&self) -> bool {
        matches!(self, Self::TransitionLoss | Self::TransitionWin)
    }
}

/// 상태 내에서의 세부 역할
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleTag {
    // === 공격 역할 ===
    /// 마무리 담당 (슛 우선)
    Finisher,

    /// 찬스 메이커 (패스/크로스 우선)
    Creator,

    /// 공간 침투
    Runner,

    /// 탈출구 (수비 형태에서 공격 전환 대기)
    Outlet,

    // === 수비 역할 ===
    /// 압박 담당
    Presser,

    /// 마킹 담당
    Marker,

    /// 앵커/커버 담당
    Anchor,

    /// 스위퍼/라인 뒤 커버
    Sweeper,
}

impl RoleTag {
    /// 포지션과 전술에서 역할 결정
    pub fn from_position_and_state(
        position: &str,
        state: PlayerPhaseState,
        is_pressing: bool,
    ) -> Self {
        match state {
            PlayerPhaseState::OnBall => {
                // On-ball일 때 포지션 기반 역할
                match position {
                    "ST" | "CF" => Self::Finisher,
                    "AM" | "W" | "WM" => Self::Creator,
                    _ => Self::Creator, // 기본값
                }
            }
            PlayerPhaseState::ReadyToReceive | PlayerPhaseState::OffBallAttack => {
                match position {
                    "ST" | "CF" => Self::Runner,
                    "AM" | "W" => Self::Runner,
                    "CM" | "DM" => Self::Outlet,
                    "CB" | "FB" => Self::Outlet,
                    _ => Self::Runner,
                }
            }
            PlayerPhaseState::TransitionWin => {
                match position {
                    "ST" | "CF" | "W" => Self::Runner,
                    _ => Self::Outlet,
                }
            }
            PlayerPhaseState::DefendBallCarrier => {
                if is_pressing {
                    Self::Presser
                } else {
                    Self::Marker
                }
            }
            PlayerPhaseState::DefendOffBallTarget => Self::Marker,
            PlayerPhaseState::DefensiveShape => {
                match position {
                    "CB" | "DM" => Self::Anchor,
                    "GK" => Self::Sweeper,
                    "ST" | "CF" => Self::Outlet, // 공격수는 역습 대기
                    _ => Self::Anchor,
                }
            }
            PlayerPhaseState::TransitionLoss => {
                if is_pressing {
                    Self::Presser
                } else {
                    Self::Anchor
                }
            }
        }
    }
}

/// 상태 분류에 필요한 컨텍스트
#[derive(Debug, Clone)]
pub struct StateContext {
    /// 우리 팀이 볼 소유 중
    pub team_has_ball: bool,

    /// 내가 공을 가지고 있음
    pub i_have_ball: bool,

    /// 공까지 거리 (m)
    pub dist_to_ball: f32,

    /// 점유 변화 틱
    pub possession_changed_tick: u64,

    /// 현재 틱
    pub current_tick: u64,

    /// 마킹 할당된 상대 선수
    pub marking_assignment: Option<PlayerId>,

    /// 패스 레인이 클리어한가
    pub pass_lane_clear: bool,

    /// 몸이 공 방향을 향하고 있는가
    pub body_facing_ball: bool,

    /// 볼캐리어까지 거리 (수비 시)
    pub dist_to_ball_carrier: f32,

    /// 볼캐리어 수비 담당 여부
    pub assigned_to_ball_carrier: bool,

    /// 팀 내 볼캐리어까지 가장 가까운가
    pub closest_to_ball_carrier: bool,
}

impl StateContext {
    /// 점유 변화가 지정된 틱 이내에 일어났는지
    pub fn possession_changed_within_ticks(&self, ticks: u64) -> bool {
        self.current_tick.saturating_sub(self.possession_changed_tick) < ticks
    }

    /// 패스를 받을 수 있는 상태인지
    pub fn can_receive_pass(&self) -> bool {
        self.pass_lane_clear && self.body_facing_ball && self.dist_to_ball < 30.0
    }

    /// 볼캐리어를 수비 중인지
    pub fn am_defending_ball_carrier(&self) -> bool {
        self.assigned_to_ball_carrier
            || (self.closest_to_ball_carrier && self.dist_to_ball_carrier < 10.0)
    }

    /// 마킹 타겟이 있는지
    pub fn have_marking_target(&self) -> bool {
        self.marking_assignment.is_some()
    }
}

impl Default for StateContext {
    fn default() -> Self {
        Self {
            team_has_ball: false,
            i_have_ball: false,
            dist_to_ball: 50.0,
            possession_changed_tick: 0,
            current_tick: 0,
            marking_assignment: None,
            pass_lane_clear: false,
            body_facing_ball: false,
            dist_to_ball_carrier: 50.0,
            assigned_to_ball_carrier: false,
            closest_to_ball_carrier: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_on_ball_classification() {
        let ctx = StateContext {
            team_has_ball: true,
            i_have_ball: true,
            possession_changed_tick: 0,
            current_tick: 100, // Not in transition window
            ..Default::default()
        };
        assert_eq!(PlayerPhaseState::classify(&ctx), PlayerPhaseState::OnBall);
    }

    #[test]
    fn test_transition_win_classification() {
        let ctx = StateContext {
            team_has_ball: true,
            i_have_ball: false,
            possession_changed_tick: 100,
            current_tick: 102, // 2틱 전에 점유 변화
            ..Default::default()
        };
        assert_eq!(
            PlayerPhaseState::classify(&ctx),
            PlayerPhaseState::TransitionWin
        );
    }

    #[test]
    fn test_defensive_shape_classification() {
        let ctx = StateContext {
            team_has_ball: false,
            i_have_ball: false,
            possession_changed_tick: 0,
            current_tick: 100, // 오래 전에 점유 변화
            marking_assignment: None,
            assigned_to_ball_carrier: false,
            closest_to_ball_carrier: false,
            ..Default::default()
        };
        assert_eq!(
            PlayerPhaseState::classify(&ctx),
            PlayerPhaseState::DefensiveShape
        );
    }
}
