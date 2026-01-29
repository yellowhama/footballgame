//! Player Objective System
//!
//! 선수 개인의 현재 목표를 정의합니다.
//! TeamPhase에 따라 각 선수에게 적절한 목표가 할당됩니다.
//!
//! ## 공격 목표 (Attack / TransitionAttack)
//! - CreateChance: 찬스 메이킹 (스루패스, 크로스)
//! - RetainPossession: 점유 유지 (안전한 패스)
//! - Support: 패스 옵션 제공 (근거리 지원)
//! - Penetrate: 침투 런 (공간 파고들기)
//! - StretchWidth: 넓이 확보 (측면 벌리기)
//! - Recycle: 볼 순환 (템포 조절)
//!
//! ## 수비 목표 (Defense / TransitionDefense)
//! - RecoverBall: 공 탈취 시도 (압박, 태클)
//! - Delay: 지연 플레이 (시간 벌기)
//! - ProtectZone: 공간 보호 (존 마크)
//! - MarkOpponent: 대인 마크
//! - MaintainShape: 대형 유지 (라인 정렬)
//! - TrackRunner: 침투자 추적

use serde::{Deserialize, Serialize};

use super::positioning::PositionKey;
use super::team_phase::TeamPhase;
use crate::engine::debug_flags::match_debug_enabled;

/// 선수 개인의 현재 목표
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PlayerObjective {
    // ===== 공격 목표 =====
    /// 찬스 메이킹 - 스루패스, 크로스, 킬패스 시도
    CreateChance,
    /// 점유 유지 - 안전한 패스로 공 보유
    #[default]
    RetainPossession,
    /// 패스 옵션 제공 - 볼 소유자 근처에서 지원
    Support,
    /// 침투 런 - 수비 라인 뒤 공간으로 달리기
    Penetrate,
    /// 넓이 확보 - 측면으로 벌려서 공간 생성
    StretchWidth,
    /// 볼 순환 - 후방/측면으로 공 돌리기
    Recycle,

    // ===== 수비 목표 =====
    /// 공 탈취 시도 - 적극적 압박, 태클
    RecoverBall,
    /// 지연 플레이 - 상대 진행 늦추기, 시간 벌기
    Delay,
    /// 공간 보호 - 위험 구역 커버
    ProtectZone,
    /// 대인 마크 - 특정 상대 밀착 마크
    MarkOpponent,
    /// 대형 유지 - 라인 정렬, 간격 유지
    MaintainShape,
    /// 침투자 추적 - 달려나가는 상대 따라가기
    TrackRunner,
}

impl PlayerObjective {
    /// 이 목표가 공격적인 목표인지
    pub fn is_offensive(&self) -> bool {
        matches!(
            self,
            PlayerObjective::CreateChance
                | PlayerObjective::RetainPossession
                | PlayerObjective::Support
                | PlayerObjective::Penetrate
                | PlayerObjective::StretchWidth
                | PlayerObjective::Recycle
        )
    }

    /// 이 목표가 적극적인 행동을 요구하는지 (vs 위치 유지)
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            PlayerObjective::CreateChance
                | PlayerObjective::Penetrate
                | PlayerObjective::RecoverBall
                | PlayerObjective::TrackRunner
        )
    }

    /// 이 목표의 위험 허용 수준 (0.0 = 안전 우선, 1.0 = 위험 감수)
    pub fn risk_tolerance(&self) -> f32 {
        match self {
            PlayerObjective::CreateChance => 0.8,
            PlayerObjective::Penetrate => 0.9,
            PlayerObjective::RecoverBall => 0.7,
            PlayerObjective::StretchWidth => 0.5,
            PlayerObjective::Support => 0.3,
            PlayerObjective::RetainPossession => 0.2,
            PlayerObjective::Recycle => 0.1,
            PlayerObjective::Delay => 0.3,
            PlayerObjective::ProtectZone => 0.2,
            PlayerObjective::MarkOpponent => 0.4,
            PlayerObjective::MaintainShape => 0.1,
            PlayerObjective::TrackRunner => 0.5,
        }
    }
}

/// 선수 목표 할당 컨텍스트
#[derive(Debug, Clone)]
pub struct ObjectiveContext {
    /// 팀 페이즈
    pub team_phase: TeamPhase,
    /// 이 선수가 공을 소유하고 있는지
    pub has_ball: bool,
    /// 선수 포지션
    pub position_key: PositionKey,
    /// 볼 소유자와의 거리 (미터)
    pub distance_to_ball: f32,
    /// 상대 골대와의 거리 (미터)
    pub distance_to_opponent_goal: f32,
    /// 가장 가까운 상대와의 거리 (미터)
    pub nearest_opponent_distance: f32,
    /// 팀 내 순위 (공에 가까운 순, 0 = 가장 가까움)
    pub proximity_rank: usize,
}

/// 선수에게 목표 할당
pub fn assign_objective(ctx: &ObjectiveContext) -> PlayerObjective {
    // 볼 소유자는 특별 처리
    if ctx.has_ball {
        return assign_ball_holder_objective(ctx);
    }

    let obj = match ctx.team_phase {
        TeamPhase::Attack => assign_attack_objective(ctx),
        TeamPhase::TransitionAttack => assign_transition_attack_objective(ctx),
        TeamPhase::Defense => assign_defense_objective(ctx),
        TeamPhase::TransitionDefense => assign_transition_defense_objective(ctx),
    };

    // FIX_2601/0105: Debug output for attacking phases
    static DEBUG_ATK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if ctx.team_phase.is_attacking() && DEBUG_ATK.load(std::sync::atomic::Ordering::Relaxed) < 10 {
        DEBUG_ATK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        #[cfg(debug_assertions)]
        if match_debug_enabled() {
            println!(
                "[DEBUG-0105-OBJ-ATK] phase={:?} pos={:?} is_fwd={} obj={:?}",
                ctx.team_phase,
                ctx.position_key,
                is_forward_position(&ctx.position_key),
                obj
            );
        }
    }

    obj
}

/// 볼 소유자 목표 할당
fn assign_ball_holder_objective(ctx: &ObjectiveContext) -> PlayerObjective {
    // 골대 가까이 + 공격 페이즈 → 찬스 메이킹
    if ctx.distance_to_opponent_goal < 25.0 && ctx.team_phase.is_attacking() {
        return PlayerObjective::CreateChance;
    }

    // 압박 받는 중 → 점유 유지 (안전 패스)
    if ctx.nearest_opponent_distance < 3.0 {
        return PlayerObjective::RetainPossession;
    }

    // 그 외 → 기본적으로 점유 유지
    PlayerObjective::RetainPossession
}

/// 공격 페이즈 목표 할당
///
/// FIX_2601/0105: Forward positions (ST, CF, LW, RW) always get Penetrate
/// during Attack phase, regardless of distance to goal. This ensures the
/// team pushes forward when attacking, not just when already near goal.
fn assign_attack_objective(ctx: &ObjectiveContext) -> PlayerObjective {
    // FIX_2601/0105: Forward positions ALWAYS penetrate during attack
    // This is the key fix - forwards must push toward opponent goal
    if is_forward_position(&ctx.position_key) {
        return PlayerObjective::Penetrate;
    }

    // 공에 가장 가까운 2명 → 지원
    if ctx.proximity_rank < 2 {
        return PlayerObjective::Support;
    }

    // 공격 진영 (골대 25m 이내) → 침투 또는 찬스 메이킹
    if ctx.distance_to_opponent_goal < 25.0 {
        if ctx.proximity_rank < 4 {
            return PlayerObjective::Penetrate;
        }
        return PlayerObjective::StretchWidth;
    }

    // 측면 선수 → 넓이 확보
    if is_wide_position(&ctx.position_key) {
        return PlayerObjective::StretchWidth;
    }

    // 미드필더 → 지원 또는 순환
    if ctx.distance_to_ball < 20.0 {
        PlayerObjective::Support
    } else {
        PlayerObjective::Recycle
    }
}

/// 전환 공격 목표 할당
///
/// FIX_2601/0105: Forward positions always penetrate on counter-attack
fn assign_transition_attack_objective(ctx: &ObjectiveContext) -> PlayerObjective {
    // FIX_2601/0105: Forward positions ALWAYS penetrate on transition
    if is_forward_position(&ctx.position_key) {
        return PlayerObjective::Penetrate;
    }

    // 역습 시 → 침투 우선 (거리 제한 완화)
    if ctx.distance_to_opponent_goal < 50.0 && ctx.proximity_rank < 3 {
        return PlayerObjective::Penetrate;
    }

    // 공에 가까운 선수 → 지원
    if ctx.proximity_rank < 2 {
        return PlayerObjective::Support;
    }

    // 나머지 → 전진하며 지원
    PlayerObjective::Support
}

/// 수비 페이즈 목표 할당
fn assign_defense_objective(ctx: &ObjectiveContext) -> PlayerObjective {
    // 공에 가장 가까운 선수 → 압박/탈취
    if ctx.proximity_rank == 0 {
        return PlayerObjective::RecoverBall;
    }

    // 2-3번째 가까운 선수 → 커버/마크
    if ctx.proximity_rank < 3 {
        return PlayerObjective::MarkOpponent;
    }

    // 수비수 → 대형 유지
    if is_defender_position(&ctx.position_key) {
        return PlayerObjective::MaintainShape;
    }

    // 나머지 → 공간 보호
    PlayerObjective::ProtectZone
}

/// 전환 수비 목표 할당
fn assign_transition_defense_objective(ctx: &ObjectiveContext) -> PlayerObjective {
    // 공에 가장 가까운 선수 → 지연 (시간 벌기)
    if ctx.proximity_rank == 0 {
        return PlayerObjective::Delay;
    }

    // 2번째 가까운 선수 → 탈취 시도
    if ctx.proximity_rank == 1 {
        return PlayerObjective::RecoverBall;
    }

    // 수비수 → 빠른 복귀 (대형 유지)
    if is_defender_position(&ctx.position_key) {
        return PlayerObjective::MaintainShape;
    }

    // 나머지 → 침투자 추적 또는 복귀
    if ctx.distance_to_ball > 30.0 {
        PlayerObjective::TrackRunner
    } else {
        PlayerObjective::ProtectZone
    }
}

/// 측면 포지션인지 확인
fn is_wide_position(pos: &PositionKey) -> bool {
    matches!(
        pos,
        PositionKey::LW
            | PositionKey::RW
            | PositionKey::LM
            | PositionKey::RM
            | PositionKey::LWB
            | PositionKey::RWB
            | PositionKey::LB
            | PositionKey::RB
    )
}

/// 수비 포지션인지 확인
fn is_defender_position(pos: &PositionKey) -> bool {
    matches!(
        pos,
        PositionKey::GK
            | PositionKey::CB
            | PositionKey::LCB
            | PositionKey::RCB
            | PositionKey::LB
            | PositionKey::RB
            | PositionKey::LWB
            | PositionKey::RWB
    )
}

/// 공격 포지션인지 확인
/// FIX_2601/0105: Forward positions should always penetrate during attack
/// Also includes attacking midfielders (LM, RM) as they often make runs
fn is_forward_position(pos: &PositionKey) -> bool {
    matches!(
        pos,
        PositionKey::ST
            | PositionKey::CF
            | PositionKey::LW
            | PositionKey::RW
            | PositionKey::LF  // FIX_2601/0105: Added
            | PositionKey::RF  // FIX_2601/0105: Added
            | PositionKey::CAM
            | PositionKey::LM  // FIX_2601/0105: Attacking mids
            | PositionKey::RM // FIX_2601/0105: Attacking mids
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ball_holder_near_goal() {
        let ctx = ObjectiveContext {
            team_phase: TeamPhase::Attack,
            has_ball: true,
            position_key: PositionKey::ST,
            distance_to_ball: 0.0,
            distance_to_opponent_goal: 15.0,
            nearest_opponent_distance: 5.0,
            proximity_rank: 0,
        };

        let obj = assign_objective(&ctx);
        assert_eq!(obj, PlayerObjective::CreateChance);
    }

    #[test]
    fn test_ball_holder_under_pressure() {
        let ctx = ObjectiveContext {
            team_phase: TeamPhase::Attack,
            has_ball: true,
            position_key: PositionKey::CM,
            distance_to_ball: 0.0,
            distance_to_opponent_goal: 50.0,
            nearest_opponent_distance: 2.0, // 압박 받는 중
            proximity_rank: 0,
        };

        let obj = assign_objective(&ctx);
        assert_eq!(obj, PlayerObjective::RetainPossession);
    }

    #[test]
    fn test_attack_phase_support() {
        let ctx = ObjectiveContext {
            team_phase: TeamPhase::Attack,
            has_ball: false,
            position_key: PositionKey::CM,
            distance_to_ball: 10.0,
            distance_to_opponent_goal: 45.0,
            nearest_opponent_distance: 8.0,
            proximity_rank: 1, // 공에 두 번째로 가까움
        };

        let obj = assign_objective(&ctx);
        assert_eq!(obj, PlayerObjective::Support);
    }

    #[test]
    fn test_defense_phase_recover() {
        let ctx = ObjectiveContext {
            team_phase: TeamPhase::Defense,
            has_ball: false,
            position_key: PositionKey::CM,
            distance_to_ball: 5.0,
            distance_to_opponent_goal: 60.0,
            nearest_opponent_distance: 3.0,
            proximity_rank: 0, // 공에 가장 가까움
        };

        let obj = assign_objective(&ctx);
        assert_eq!(obj, PlayerObjective::RecoverBall);
    }

    #[test]
    fn test_transition_defense_delay() {
        let ctx = ObjectiveContext {
            team_phase: TeamPhase::TransitionDefense,
            has_ball: false,
            position_key: PositionKey::CAM,
            distance_to_ball: 8.0,
            distance_to_opponent_goal: 55.0,
            nearest_opponent_distance: 2.0,
            proximity_rank: 0,
        };

        let obj = assign_objective(&ctx);
        assert_eq!(obj, PlayerObjective::Delay);
    }

    #[test]
    fn test_objective_is_offensive() {
        assert!(PlayerObjective::CreateChance.is_offensive());
        assert!(PlayerObjective::Support.is_offensive());
        assert!(!PlayerObjective::RecoverBall.is_offensive());
        assert!(!PlayerObjective::MaintainShape.is_offensive());
    }

    #[test]
    fn test_objective_risk_tolerance() {
        assert!(
            PlayerObjective::Penetrate.risk_tolerance()
                > PlayerObjective::RetainPossession.risk_tolerance()
        );
        assert!(
            PlayerObjective::RecoverBall.risk_tolerance()
                > PlayerObjective::MaintainShape.risk_tolerance()
        );
    }
}
