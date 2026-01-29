//! Elastic Band Theory: 상대 좌표 포지셔닝 시스템
//!
//! 포메이션은 "지도상의 좌표"가 아니라, 선수들끼리의 "상대적 거리(약속)".
//! 선수에게는 보이지 않는 '앵커(닻)'와 '고무줄'이 달려있습니다.
//!
//! ## 핵심 개념
//! - **팀 기준점 (Team Center)**: 팀 전체의 중심점 (공의 위치와 수비 라인 높이에 따라 결정)
//! - **포메이션 오프셋 (Formation Offset)**: 팀 기준점으로부터 "내가 얼마나 떨어져 있어야 하는가"
//! - **복귀 본능 (Elasticity)**: 상황이 바뀌면 고무줄이 당겨지듯 다시 앵커 포인트 근처로 복귀
//!
//! ## 라인 시스템 (Rail System)
//! 선수 개개인이 각자의 X좌표(깊이)를 계산하면 라인이 삐뚤빼뚤해짐.
//! → "라인(Line)이 먼저 위치를 잡고, 선수는 그 라인 위에 서야" 합니다.

use serde::{Deserialize, Serialize};
use super::sort_keys::compare_score_desc_stable;

// ============================================================================
// Constants
// ============================================================================

/// 팀 슬라이드 상수
pub mod constants {
    use crate::engine::physics_constants::field;

    /// 공 따라가는 비율 (좌우)
    pub const BALL_FOLLOW_RATIO_Y: f32 = 0.6;

    /// 공격 시 공 뒤 기본 거리 (m)
    pub const ATTACKING_BEHIND_BALL: f32 = 15.0;

    /// 수비 라인 기본 높이 비율 (0.0 = 우리골대, 1.0 = 상대골대)
    pub const DEFAULT_DEFENSIVE_LINE: f32 = 0.35;

    /// 미드필더 라인 오프셋 (수비 라인 앞)
    pub const MIDFIELD_LINE_OFFSET: f32 = 15.0;

    /// 공격 라인 오프셋 (미드필더 라인 앞)
    pub const ATTACK_LINE_OFFSET: f32 = 20.0;

    /// 자석 효과 최대 거리 (m)
    pub const MAGNET_MAX_PULL: f32 = 3.0;

    /// 자석 감지 범위 (m)
    pub const MAGNET_ZONE_RADIUS: f32 = 10.0;

    /// X축 (깊이) 이동 제한 비율 (라인 파괴 방지)
    pub const X_AXIS_DAMPING: f32 = 0.3;

    /// Y축 (좌우) 이동 비율
    pub const Y_AXIS_FREEDOM: f32 = 1.0;

    /// 고무줄 복귀 속도 (m/tick)
    pub const ELASTIC_SNAP_SPEED: f32 = 1.5;

    /// 고무줄 최대 늘어남 (m) - 이 거리 이상 벌어지면 강제 복귀
    pub const ELASTIC_MAX_STRETCH: f32 = 8.0;

    /// 필드 길이 (m)
    pub const FIELD_LENGTH: f32 = field::LENGTH_M;

    /// 필드 폭 (m)
    pub const FIELD_WIDTH: f32 = field::WIDTH_M;

    /// 필드 중앙 X (m)
    pub const CENTER_X: f32 = field::CENTER_X;

    /// 필드 중앙 Y (m)
    pub const CENTER_Y: f32 = field::CENTER_Y;
}

pub use constants::*;

// ============================================================================
// Types
// ============================================================================

/// 선수 역할 (포지션 라인 결정용)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PositionLine {
    /// 골키퍼
    Goalkeeper,
    /// 수비 라인
    Defender,
    /// 미드필더 라인
    #[default]
    Midfielder,
    /// 공격 라인
    Forward,
}

/// 팀 포지셔닝 상태
#[derive(Debug, Clone, Default)]
pub struct TeamPositioningState {
    /// 팀 기준점 (미터)
    pub team_center: (f32, f32),

    /// 수비 라인 X 좌표 (공유 - 레일)
    pub defensive_line_x: f32,

    /// 미드필더 라인 X 좌표 (공유 - 레일)
    pub midfield_line_x: f32,

    /// 공격 라인 X 좌표 (공유 - 레일)
    pub attack_line_x: f32,

    /// 팀 좌우 쏠림 (Y offset)
    pub team_shift_y: f32,

    /// 공 소유 여부
    pub has_possession: bool,

    /// 압박 강도 (0.0 ~ 1.0)
    pub press_intensity: f32,
}

/// 전술 설정 (Elastic Band에 필요한 부분만)
#[derive(Debug, Clone)]
pub struct ElasticTactics {
    /// 수비 라인 높이 (0.0 = 매우 깊음, 1.0 = 매우 높음)
    pub defensive_line_height: f32,

    /// 컴팩트니스 (0.0 = 넓게, 1.0 = 좁게)
    pub compactness: f32,

    /// 좌우 너비 (0.0 = 좁게, 1.0 = 넓게)
    pub width: f32,
}

impl Default for ElasticTactics {
    fn default() -> Self {
        Self { defensive_line_height: 0.5, compactness: 0.5, width: 0.5 }
    }
}

/// 포메이션 오프셋 (팀 기준점으로부터의 상대 위치)
#[derive(Debug, Clone, Copy, Default)]
pub struct FormationOffset {
    /// X 오프셋 (앞/뒤)
    pub x: f32,
    /// Y 오프셋 (좌/우)
    pub y: f32,
    /// 역할 라인
    pub line: PositionLine,
}

/// 상대 선수 위협 정보
#[derive(Debug, Clone, Copy)]
pub struct ThreatInfo {
    /// 상대 선수 인덱스
    pub opponent_idx: usize,
    /// 상대 선수 위치
    pub position: (f32, f32),
    /// 위협 점수 (0.0 ~ 1.0)
    pub threat_score: f32,
}

// ============================================================================
// Phase 1: Team Shape & Opponent Shape (0113)
// ============================================================================

/// 실시간 팀 형태 (Open-Football 참고)
///
/// 전술 설정값이 아니라 실제 선수 위치에서 계산된 "현재 상태"
#[derive(Debug, Clone, Default)]
pub struct TeamShape {
    /// 좌우 퍼짐 (m) - 가장 왼쪽~오른쪽 선수 거리
    pub width: f32,
    /// 앞뒤 퍼짐 (m) - 가장 뒤~앞 선수 거리
    pub depth: f32,
    /// 밀집도 (0.0 ~ 1.0) - 높을수록 뭉쳐있음
    pub compactness: f32,
    /// 팀 중심점 (m)
    pub center: (f32, f32),
}

/// 상대 형태 분석 결과 (Open-Football 참고)
///
/// 공격수/미드필더가 상대 형태를 읽고 무브먼트 결정에 활용
#[derive(Debug, Clone, Default)]
pub struct OpponentShape {
    /// 높은 라인인가? (수비라인 X > 필드 40%)
    pub high_line: bool,
    /// 중앙 밀집인가? (중앙 20m 내 6명+)
    pub compact_central: bool,
    /// 좁게 서있는가? (width < 30m)
    pub narrow: bool,
    /// 수비 라인 X 위치 (m) - 뒤에서 4번째 선수
    pub defensive_line_x: f32,
}

/// 수비 라인 응집력 (Phase 2)
#[derive(Debug, Clone, Default)]
pub struct DefensiveLineCohesion {
    /// 현재 라인 X 위치 (수비수 평균)
    pub line_x: f32,
    /// 가장 멀리 떨어진 수비수 거리 (m)
    pub max_deviation: f32,
    /// 응집력 충족 여부 (편차 <= MAX_DEVIATION)
    pub is_cohesive: bool,
}

/// 수비 라인 응집력 허용 최대 편차 (m)
pub const MAX_DEFENSIVE_LINE_DEVIATION: f32 = 8.0;

// ============================================================================
// Phase 3: Movement Patterns (0113)
// ============================================================================

/// 공격수 무브먼트 패턴 (Open-Football 참고)
///
/// 상대 형태(OpponentShape)를 읽고 적절한 패턴 선택
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ForwardMovement {
    /// 직선 침투 (라인 뒤로) - high_line 상대에 효과적
    DirectRun,
    /// 대각선 런 (수비 사이) - compact_central 상대에 효과적
    DiagonalRun,
    /// 채널 런 (수비수 사이 공간)
    ChannelRun,
    /// 측면으로 빠짐 (공간 창출) - narrow 상대에 효과적
    DriftWide,
    /// 내려와서 받기 (기본)
    #[default]
    CheckToFeet,
    /// 공 반대편으로 이동 (수비 늘리기)
    OppositeMovement,
}

/// 미드필더 무브먼트 패턴
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MidfielderMovement {
    /// 직선 이동 (기본)
    #[default]
    Direct,
    /// 곡선 이동 (마커 따돌리기)
    Curved,
    /// 뒤로 빠져서 받기
    CheckToReceive,
    /// 공 이동 반대로
    OppositeRun,
    /// 3자 런 (2차 패스 받기)
    ThirdManRun,
}

// ============================================================================
// Phase 4: Trap Readiness (0113)
// ============================================================================

/// 오프사이드 트랩 준비 상태 (Open-Football 참고)
///
/// 단순 on/off가 아니라, 팀 능력과 라인 응집력 기반으로 트랩 가능 여부 판단
#[derive(Debug, Clone)]
pub struct OffsideTrapReadiness {
    /// 트랩 실행 가능 여부
    pub can_execute: bool,
    /// 응집력 점수 (0.0 ~ 1.0) - 높을수록 트랩 성공 확률 증가
    pub cohesion_score: f32,
    /// 불가 사유 (Ready가 아니면 트랩 불가)
    pub reason: TrapBlockReason,
}

impl Default for OffsideTrapReadiness {
    fn default() -> Self {
        Self {
            can_execute: false,
            cohesion_score: 0.0,
            reason: TrapBlockReason::TacticsDisabled,
        }
    }
}

/// 트랩 불가 사유
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrapBlockReason {
    /// 트랩 실행 준비됨
    Ready,
    /// 전술 설정에서 비활성화
    #[default]
    TacticsDisabled,
    /// 팀 팀워크 낮음 (< 65)
    LowTeamwork,
    /// 팀 경험 낮음 (< 60)
    LowExperience,
    /// 수비 라인 편차 큼 (> 8m)
    LineTooSpread,
}

/// 트랩 평가에 필요한 최소 팀워크
pub const MIN_TRAP_TEAMWORK: f32 = 65.0;
/// 트랩 평가에 필요한 최소 경험
pub const MIN_TRAP_EXPERIENCE: f32 = 60.0;

// ============================================================================
// Core Functions
// ============================================================================

/// 팀 기준점 계산
///
/// 공격 시: 공 뒤 10~20m 지점을 중심으로 밀고 올라감
/// 수비 시: 공과 골대 사이 적절한 지점
///
/// # Arguments
/// * `attacks_right` - true: 오른쪽(x=105) 공격, false: 왼쪽(x=0) 공격
///   FIX_2601/0109: 하프타임 전환 반영을 위해 is_home 대신 attacks_right 사용
pub fn calculate_team_center(
    ball_pos: (f32, f32),
    has_possession: bool,
    tactics: &ElasticTactics,
    attacks_right: bool,
) -> (f32, f32) {
    let own_goal_x = if attacks_right { 0.0 } else { FIELD_LENGTH };
    let attack_dir = if attacks_right { 1.0 } else { -1.0 };

    // 1. 세로 위치 (X축): 수비 라인 조절
    let team_x = if has_possession {
        // 공격 시: 공 뒤 15m 지점을 중심으로 (조절 가능)
        let behind_distance = ATTACKING_BEHIND_BALL * (1.0 - tactics.compactness * 0.3);
        ball_pos.0 - attack_dir * behind_distance
    } else {
        // 수비 시: 공과 골대 사이
        // defensive_line_height가 높을수록 공 쪽으로 올라감
        let base_ratio = DEFAULT_DEFENSIVE_LINE + tactics.defensive_line_height * 0.3;

        // 공 위치와 골대 사이를 보간

        own_goal_x + (ball_pos.0 - own_goal_x) * base_ratio
    };

    // 2. 가로 위치 (Y축): 좌우 쏠림
    // 공 위치의 60%만큼만 따라감 (너무 쏠리면 반대편 비어버림)
    let center_y = FIELD_WIDTH / 2.0;
    let ball_offset_y = ball_pos.1 - center_y;
    let team_y = center_y + ball_offset_y * BALL_FOLLOW_RATIO_Y;

    (team_x, team_y)
}

/// 라인 높이 계산 (레일 시스템)
///
/// 각 라인의 X 좌표를 계산하여 선수들이 같은 라인에 서도록 함
///
/// # Arguments
/// * `attacks_right` - true: 오른쪽(x=105) 공격, false: 왼쪽(x=0) 공격
///   FIX_2601/0109: 하프타임 전환 반영
pub fn calculate_line_heights(
    team_center: (f32, f32),
    tactics: &ElasticTactics,
    has_possession: bool,
    attacks_right: bool,
) -> (f32, f32, f32) {
    let attack_dir = if attacks_right { 1.0 } else { -1.0 };

    // 컴팩트니스에 따른 라인 간격 조절
    let line_spacing = 1.0 - tactics.compactness * 0.4; // 0.6 ~ 1.0

    // 공격/수비에 따른 라인 간격 조절
    let possession_modifier = if has_possession { 1.2 } else { 0.8 };

    let defensive_line_x = team_center.0;
    let midfield_line_x =
        team_center.0 + attack_dir * MIDFIELD_LINE_OFFSET * line_spacing * possession_modifier;
    let attack_line_x =
        midfield_line_x + attack_dir * ATTACK_LINE_OFFSET * line_spacing * possession_modifier;

    (defensive_line_x, midfield_line_x, attack_line_x)
}

/// 팀 좌우 쏠림 계산
///
/// 공이 좌측에 있으면 팀 전체가 좌측으로 이동
pub fn calculate_team_shift(ball_pos: (f32, f32), tactics: &ElasticTactics) -> f32 {
    let center_y = FIELD_WIDTH / 2.0;
    let ball_offset = ball_pos.1 - center_y;

    // 전술 너비에 따라 쏠림 정도 조절
    // width가 높으면 덜 쏠림 (넓게 유지)
    let shift_ratio = BALL_FOLLOW_RATIO_Y * (1.0 - tactics.width * 0.3);

    ball_offset * shift_ratio
}

/// 커버 쉬프트 계산 (자석 로직)
///
/// 내 구역에 있는 위협적인 상대에게 살짝 끌려감
/// 단, 라인을 깨지 않는 선에서만
pub fn calculate_cover_shift(
    player_pos: (f32, f32),
    threats: &[ThreatInfo],
    _player_line: PositionLine,
) -> (f32, f32) {
    if threats.is_empty() {
        return (0.0, 0.0);
    }

    // 내 구역 내 가장 위협적인 상대 찾기
    let mut best_threat: Option<&ThreatInfo> = None;
    let mut best_score = 0.0;

    for threat in threats {
        let dist = distance(player_pos, threat.position);
        if dist < MAGNET_ZONE_RADIUS && threat.threat_score > best_score {
            best_threat = Some(threat);
            best_score = threat.threat_score;
        }
    }

    let Some(target) = best_threat else {
        return (0.0, 0.0);
    };

    // 방향 계산
    let dx = target.position.0 - player_pos.0;
    let dy = target.position.1 - player_pos.1;
    let dist = distance(player_pos, target.position);

    if dist < 0.1 {
        return (0.0, 0.0);
    }

    // 당김 강도 (거리가 가까울수록 강하게, 최대 3m)
    let pull_strength = ((MAGNET_ZONE_RADIUS - dist) / MAGNET_ZONE_RADIUS * MAGNET_MAX_PULL)
        .clamp(0.0, MAGNET_MAX_PULL);

    let norm_dx = dx / dist;
    let norm_dy = dy / dist;

    // X축(깊이)은 제한, Y축(좌우)은 자유롭게
    let shift_x = norm_dx * pull_strength * X_AXIS_DAMPING;
    let shift_y = norm_dy * pull_strength * Y_AXIS_FREEDOM;

    (shift_x, shift_y)
}

/// 내 구역 내 위협 탐지
pub fn find_threats_in_zone(
    player_pos: (f32, f32),
    zone_radius: f32,
    opponents: &[(f32, f32)],
    ball_pos: (f32, f32),
    goal_pos: (f32, f32),
) -> Vec<ThreatInfo> {
    let mut threats = Vec::new();

    for (idx, &opp_pos) in opponents.iter().enumerate() {
        let dist = distance(player_pos, opp_pos);

        if dist < zone_radius {
            // 위협 점수 계산
            // - 골대에 가까울수록 위험
            // - 공에 가까울수록 위험
            let dist_to_goal = distance(opp_pos, goal_pos);
            let dist_to_ball = distance(opp_pos, ball_pos);

            // 정규화된 위협 점수
            let goal_threat = 1.0 - (dist_to_goal / FIELD_LENGTH).min(1.0);
            let ball_proximity = 1.0 - (dist_to_ball / 30.0).min(1.0);

            let threat_score = (goal_threat * 0.6 + ball_proximity * 0.4).clamp(0.0, 1.0);

            threats.push(ThreatInfo { opponent_idx: idx, position: opp_pos, threat_score });
        }
    }

    // 위협 점수로 정렬 (높은 순)
    // FIX_2601/0123 PR #9-2: Stable tie-break using opponent_idx
    threats.sort_by(|a, b| {
        compare_score_desc_stable(
            a.threat_score,
            a.opponent_idx,
            b.threat_score,
            b.opponent_idx,
        )
    });

    threats
}

/// 역할 오프셋 계산 (공격 가담, 수비 가담 등)
///
/// # Arguments
/// * `attacks_right` - true: 오른쪽(x=105) 공격, false: 왼쪽(x=0) 공격
///   FIX_2601/0109: 하프타임 전환 반영
pub fn calculate_role_offset(
    role_instruction: RoleInstruction,
    has_possession: bool,
    attacks_right: bool,
) -> (f32, f32) {
    let attack_dir = if attacks_right { 1.0 } else { -1.0 };

    match role_instruction {
        RoleInstruction::Default => (0.0, 0.0),

        RoleInstruction::GetForward => {
            // 공격 가담: 앞으로 15-20m
            if has_possession {
                (attack_dir * 18.0, 0.0)
            } else {
                (0.0, 0.0)
            }
        }

        RoleInstruction::StayBack => {
            // 수비 유지: 뒤로 10m
            (-attack_dir * 10.0, 0.0)
        }

        RoleInstruction::CutInside => {
            // 안쪽으로 컷인
            let center_y = FIELD_WIDTH / 2.0;
            (attack_dir * 5.0, (center_y - CENTER_Y).signum() * -8.0)
        }

        RoleInstruction::HugTouchline => {
            // 터치라인 붙어서 넓이 확보
            (0.0, 0.0) // Y 오프셋은 포메이션에서 이미 반영
        }
    }
}

/// 선수 역할 지시
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoleInstruction {
    #[default]
    Default,
    /// 공격 가담
    GetForward,
    /// 수비 유지
    StayBack,
    /// 안쪽으로 컷인
    CutInside,
    /// 터치라인 유지
    HugTouchline,
}

/// 고무줄 스냅백 계산
///
/// 선수가 앵커에서 너무 멀어지면 강제로 복귀
pub fn calculate_elastic_snapback(
    current_pos: (f32, f32),
    anchor_pos: (f32, f32),
    max_stretch: f32,
) -> Option<(f32, f32)> {
    let dist = distance(current_pos, anchor_pos);

    if dist <= max_stretch {
        return None; // 스냅백 불필요
    }

    // 앵커 방향으로 복귀
    let dx = anchor_pos.0 - current_pos.0;
    let dy = anchor_pos.1 - current_pos.1;

    // 복귀 속도는 늘어난 정도에 비례
    let overshoot = dist - max_stretch;
    let snap_strength = (overshoot / 5.0).min(1.0) * ELASTIC_SNAP_SPEED;

    let norm_dx = dx / dist;
    let norm_dy = dy / dist;

    Some((norm_dx * snap_strength, norm_dy * snap_strength))
}

/// 최종 선수 목표 위치 계산
///
/// Step 1. 레일 깔기 (라인 높이 결정)
/// Step 2. 구슬 배치 (포메이션 오프셋 적용) + P0-W2: Width scaling
/// Step 3. 자석 효과 (커버 쉬프트)
/// Step 4. 역할 오프셋
/// Step 5. 필드 경계 클램핑
///
/// # Arguments
/// * `attacks_right` - true: 오른쪽(x=105) 공격, false: 왼쪽(x=0) 공격
///   FIX_2601/0109: 하프타임 전환 반영
pub fn calculate_player_target(
    team_state: &TeamPositioningState,
    formation_offset: FormationOffset,
    threats: &[ThreatInfo],
    role_instruction: RoleInstruction,
    tactics: &ElasticTactics,
    attacks_right: bool,
) -> (f32, f32) {
    // Step 1 & 2: 라인 + 포메이션 오프셋
    let line_x = match formation_offset.line {
        PositionLine::Goalkeeper => {
            // GK는 특수 처리: 자기 골대 근처
            if attacks_right {
                5.0 // 오른쪽 공격 → 왼쪽 골대 수비
            } else {
                FIELD_LENGTH - 5.0 // 왼쪽 공격 → 오른쪽 골대 수비
            }
        }
        PositionLine::Defender => team_state.defensive_line_x,
        PositionLine::Midfielder => team_state.midfield_line_x,
        PositionLine::Forward => team_state.attack_line_x,
    };

    let base_x = line_x + formation_offset.x;

    // P0-W2: Apply width scaling to formation offset
    // VeryNarrow (width=0.0) → width_factor=0.75 → 75% spacing
    // Normal (width=0.5) → width_factor=1.0 → 100% spacing
    // VeryWide (width=1.0) → width_factor=1.25 → 125% spacing
    let width_factor = 0.75 + (tactics.width * 0.5);
    let scaled_offset_y = formation_offset.y * width_factor;

    let base_y = team_state.team_center.1 + scaled_offset_y + team_state.team_shift_y;

    // Step 3: 자석 효과
    let cover_shift = calculate_cover_shift((base_x, base_y), threats, formation_offset.line);

    // Step 4: 역할 오프셋
    let role_offset = calculate_role_offset(role_instruction, team_state.has_possession, attacks_right);

    // 합산
    let target_x = base_x + cover_shift.0 + role_offset.0;
    let target_y = base_y + cover_shift.1 + role_offset.1;

    // Step 5: 필드 경계 클램핑
    let clamped_x = target_x.clamp(2.0, FIELD_LENGTH - 2.0);
    let clamped_y = target_y.clamp(2.0, FIELD_WIDTH - 2.0);

    (clamped_x, clamped_y)
}

/// 팀 포지셔닝 상태 업데이트
///
/// # Arguments
/// * `attacks_right` - true: 오른쪽(x=105) 공격, false: 왼쪽(x=0) 공격
///   FIX_2601/0109: 하프타임 전환 반영
pub fn update_team_positioning_state(
    state: &mut TeamPositioningState,
    ball_pos: (f32, f32),
    has_possession: bool,
    tactics: &ElasticTactics,
    attacks_right: bool,
) {
    state.has_possession = has_possession;

    // 팀 기준점 계산
    state.team_center = calculate_team_center(ball_pos, has_possession, tactics, attacks_right);

    // 라인 높이 계산
    let (def_x, mid_x, att_x) =
        calculate_line_heights(state.team_center, tactics, has_possession, attacks_right);
    state.defensive_line_x = def_x;
    state.midfield_line_x = mid_x;
    state.attack_line_x = att_x;

    // 팀 쏠림 계산
    state.team_shift_y = calculate_team_shift(ball_pos, tactics);
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 두 점 사이의 거리
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

// ============================================================================
// P0-1: TeamInstructions Integration
// ============================================================================

/// Convert TeamInstructions to ElasticTactics
///
/// Maps user-facing tactical settings to internal elastic band parameters.
/// This is the primary integration point for making team tactics affect positioning.
pub fn team_instructions_to_elastic_tactics(
    instructions: &crate::tactics::team_instructions::TeamInstructions,
) -> ElasticTactics {
    // Defensive line height: VeryDeep=-2 to VeryHigh=+2 → 0.0 to 1.0
    let line_numeric = instructions.defensive_line.to_numeric() as f32; // -2..+2
    let defensive_line_height = ((line_numeric + 2.0) / 4.0).clamp(0.0, 1.0); // 0.0..1.0

    // Team width: VeryNarrow=-2 to VeryWide=+2 → 0.0 to 1.0
    let width_numeric = instructions.team_width.to_numeric() as f32; // -2..+2
    let width = ((width_numeric + 2.0) / 4.0).clamp(0.0, 1.0); // 0.0..1.0

    // Compactness: inverse of width (narrow = compact)
    // VeryWide → low compactness, VeryNarrow → high compactness
    let compactness = 1.0 - width;

    ElasticTactics { defensive_line_height, compactness, width }
}

/// Apply pressing intensity modifier to TeamPositioningState
///
/// Higher pressing → higher press_intensity value in state
pub fn apply_pressing_intensity(
    state: &mut TeamPositioningState,
    instructions: &crate::tactics::team_instructions::TeamInstructions,
) {
    // VeryLow=-2 to VeryHigh=+2 → 0.0 to 1.0
    let pressing_numeric = instructions.pressing_intensity.to_numeric() as f32;
    state.press_intensity = ((pressing_numeric + 2.0) / 4.0).clamp(0.0, 1.0);
}

// ============================================================================
// Phase 1: Team Shape & Opponent Shape Functions (0113)
// ============================================================================

/// 팀 형태 계산 (GK 제외된 필드 플레이어 위치 기반)
///
/// # Arguments
/// * `positions` - 필드 플레이어 위치 (GK 제외, 10명)
///
/// # Returns
/// * `TeamShape` - 실시간 팀 형태 (width, depth, compactness, center)
pub fn calculate_team_shape(positions: &[(f32, f32)]) -> TeamShape {
    if positions.is_empty() {
        return TeamShape::default();
    }

    // 경계 계산
    let (min_x, max_x) = positions
        .iter()
        .map(|p| p.0)
        .fold((f32::MAX, f32::MIN), |(min, max), x| (min.min(x), max.max(x)));
    let (min_y, max_y) = positions
        .iter()
        .map(|p| p.1)
        .fold((f32::MAX, f32::MIN), |(min, max), y| (min.min(y), max.max(y)));

    let width = max_y - min_y;
    let depth = max_x - min_x;
    let area = (width * depth).max(1.0);

    // 밀집도 계산
    // - 10명이 30x40m (1200m²)에 있으면: 10 * 50 / 1200 ≈ 0.42
    // - 10명이 20x20m (400m²)에 있으면: 10 * 50 / 400 = 1.25 → clamp → 1.0
    // - 10명이 40x50m (2000m²)에 있으면: 10 * 50 / 2000 = 0.25
    let compactness = (positions.len() as f32 * 50.0 / area).clamp(0.0, 1.0);

    // 중심점 계산
    let center_x = positions.iter().map(|p| p.0).sum::<f32>() / positions.len() as f32;
    let center_y = positions.iter().map(|p| p.1).sum::<f32>() / positions.len() as f32;

    TeamShape {
        width,
        depth,
        compactness,
        center: (center_x, center_y),
    }
}

/// 상대 형태 분석
///
/// 공격수/미드필더가 상대 수비 형태를 읽고 무브먼트 결정에 활용
///
/// # Arguments
/// * `positions` - 상대 필드 플레이어 위치 (GK 제외, 10명)
/// * `is_attacking_right` - true면 오른쪽으로 공격 (홈팀), false면 왼쪽으로 공격
///
/// # Returns
/// * `OpponentShape` - 상대 형태 분석 결과
pub fn analyze_opponent_shape(positions: &[(f32, f32)], is_attacking_right: bool) -> OpponentShape {
    if positions.len() < 4 {
        return OpponentShape::default();
    }

    // 수비 라인 X 계산: 뒤에서 4번째 선수 (GK 제외하면 뒤에서 3-4번째가 수비 라인)
    // FIX_2601/0123 PR #9-2: Use total_cmp for NaN-safe ordering
    let mut x_positions: Vec<f32> = positions.iter().map(|p| p.0).collect();
    x_positions.sort_by(|a, b| {
        if is_attacking_right {
            // 왼쪽으로 공격받는 팀 = 작은 X가 뒤
            a.total_cmp(b)
        } else {
            // 오른쪽으로 공격받는 팀 = 큰 X가 뒤
            b.total_cmp(a)
        }
    });
    // 뒤에서 4번째 (인덱스 3)
    let defensive_line_x = x_positions.get(3).copied().unwrap_or(30.0);

    // 높은 라인 판단: 필드 40% 이상으로 올라왔는가?
    // is_attacking_right=true: 상대는 왼쪽 골대 수비 → 라인 X > 42m면 높음
    // is_attacking_right=false: 상대는 오른쪽 골대 수비 → 라인 X < 63m면 높음
    let high_line = if is_attacking_right {
        defensive_line_x > 42.0 // 105m * 0.4
    } else {
        defensive_line_x < 63.0 // 105m * 0.6
    };

    // 중앙 밀집: 중앙 Y ± 10m 내 선수 6명 이상
    let center_y = CENTER_Y;
    let central_count = positions.iter().filter(|p| (p.1 - center_y).abs() < 10.0).count();
    let compact_central = central_count >= 6;

    // 좁음: 팀 width < 30m
    let shape = calculate_team_shape(positions);
    let narrow = shape.width < 30.0;

    OpponentShape {
        high_line,
        compact_central,
        narrow,
        defensive_line_x,
    }
}

/// 수비 라인 응집력 계산
///
/// 수비수들이 같은 라인에 잘 서있는지 평가
///
/// # Arguments
/// * `defender_positions` - 수비수 위치 (보통 4명)
///
/// # Returns
/// * `DefensiveLineCohesion` - 라인 응집력 결과
pub fn calculate_line_cohesion(defender_positions: &[(f32, f32)]) -> DefensiveLineCohesion {
    if defender_positions.is_empty() {
        return DefensiveLineCohesion::default();
    }

    // 수비수 평균 X 위치 = 라인 위치
    let line_x =
        defender_positions.iter().map(|p| p.0).sum::<f32>() / defender_positions.len() as f32;

    // 최대 편차: 가장 멀리 떨어진 수비수
    let max_deviation = defender_positions
        .iter()
        .map(|p| (p.0 - line_x).abs())
        .fold(0.0f32, |max, d| max.max(d));

    // 응집력 충족: 편차 <= 8m
    let is_cohesive = max_deviation <= MAX_DEFENSIVE_LINE_DEVIATION;

    DefensiveLineCohesion {
        line_x,
        max_deviation,
        is_cohesive,
    }
}

// ============================================================================
// Phase 3: Movement Pattern Selection Functions (0113)
// ============================================================================

/// 공격수 무브먼트 패턴 선택
///
/// 상대 형태(OpponentShape)와 선수 능력치를 기반으로 적절한 무브먼트 선택
///
/// # Arguments
/// * `opponent_shape` - 상대 형태 분석 결과
/// * `off_the_ball` - 선수 off_the_ball 능력 (0-20)
/// * `anticipation` - 선수 anticipation 능력 (0-20)
///
/// # Returns
/// * `ForwardMovement` - 선택된 무브먼트 패턴
pub fn select_forward_movement(
    opponent_shape: &OpponentShape,
    off_the_ball: u8,
    anticipation: u8,
) -> ForwardMovement {
    // 중앙 밀집 → 대각선/채널 런으로 수비 사이 침투
    if opponent_shape.compact_central {
        if off_the_ball > 14 {
            return ForwardMovement::DiagonalRun;
        } else {
            return ForwardMovement::ChannelRun;
        }
    }

    // 높은 라인 → 직선 침투로 라인 뒤 공간 공략
    if opponent_shape.high_line {
        if anticipation > 13 {
            return ForwardMovement::DirectRun;
        } else {
            return ForwardMovement::ChannelRun;
        }
    }

    // 좁게 서있음 → 측면으로 빠져서 공간 활용
    if opponent_shape.narrow {
        return ForwardMovement::DriftWide;
    }

    // 기본: 내려와서 받기
    ForwardMovement::CheckToFeet
}

/// 미드필더 무브먼트 패턴 선택
///
/// # Arguments
/// * `opponent_shape` - 상대 형태 분석 결과
/// * `off_the_ball` - 선수 off_the_ball 능력 (0-20)
/// * `has_possession` - 우리 팀 공 소유 여부
///
/// # Returns
/// * `MidfielderMovement` - 선택된 무브먼트 패턴
pub fn select_midfielder_movement(
    opponent_shape: &OpponentShape,
    off_the_ball: u8,
    has_possession: bool,
) -> MidfielderMovement {
    // 공격 시
    if has_possession {
        // 높은 라인 상대 → 3자 런으로 2차 패스 받기
        if opponent_shape.high_line && off_the_ball > 13 {
            return MidfielderMovement::ThirdManRun;
        }

        // 중앙 밀집 → 곡선 이동으로 마커 따돌리기
        if opponent_shape.compact_central {
            return MidfielderMovement::Curved;
        }

        // 기본: 직선 이동
        return MidfielderMovement::Direct;
    }

    // 수비 시 → 뒤로 빠져서 패스 옵션 제공
    MidfielderMovement::CheckToReceive
}

/// 무브먼트 패턴을 위치 오프셋으로 변환
///
/// # Arguments
/// * `movement` - 공격수 무브먼트 패턴
/// * `attacks_right` - true: 오른쪽(x=105) 공격, false: 왼쪽(x=0) 공격
///   FIX_2601/0109: 하프타임 전환 반영
/// * `current_y` - 현재 Y 위치 (측면 판단용)
///
/// # Returns
/// * `(f32, f32)` - (X 오프셋, Y 오프셋) in meters
pub fn forward_movement_to_offset(
    movement: ForwardMovement,
    attacks_right: bool,
    current_y: f32,
) -> (f32, f32) {
    let attack_dir = if attacks_right { 1.0 } else { -1.0 };
    let center_y = CENTER_Y;

    match movement {
        ForwardMovement::DirectRun => {
            // 직선 침투: 앞으로 15m
            (attack_dir * 15.0, 0.0)
        }
        ForwardMovement::DiagonalRun => {
            // 대각선 런: 앞으로 12m + 중앙 반대쪽으로 8m
            let y_dir = if current_y > center_y { -1.0 } else { 1.0 };
            (attack_dir * 12.0, y_dir * 8.0)
        }
        ForwardMovement::ChannelRun => {
            // 채널 런: 앞으로 10m (수비 사이)
            (attack_dir * 10.0, 0.0)
        }
        ForwardMovement::DriftWide => {
            // 측면 빠짐: 앞으로 5m + 측면으로 12m
            let y_dir = if current_y > center_y { 1.0 } else { -1.0 };
            (attack_dir * 5.0, y_dir * 12.0)
        }
        ForwardMovement::CheckToFeet => {
            // 내려와서 받기: 뒤로 8m
            (-attack_dir * 8.0, 0.0)
        }
        ForwardMovement::OppositeMovement => {
            // 공 반대편: 반대쪽으로 15m
            let y_dir = if current_y > center_y { -1.0 } else { 1.0 };
            (0.0, y_dir * 15.0)
        }
    }
}

/// 미드필더 무브먼트 패턴을 위치 오프셋으로 변환
///
/// # Arguments
/// * `movement` - 미드필더 무브먼트 패턴
/// * `attacks_right` - true: 오른쪽(x=105) 공격, false: 왼쪽(x=0) 공격
///   FIX_2601/0109: 하프타임 전환 반영
/// * `current_y` - 현재 Y 위치
///
/// # Returns
/// * `(f32, f32)` - (X 오프셋, Y 오프셋) in meters
pub fn midfielder_movement_to_offset(
    movement: MidfielderMovement,
    attacks_right: bool,
    current_y: f32,
) -> (f32, f32) {
    let attack_dir = if attacks_right { 1.0 } else { -1.0 };
    let center_y = FIELD_WIDTH / 2.0;

    match movement {
        MidfielderMovement::Direct => {
            // 직선: 앞으로 8m
            (attack_dir * 8.0, 0.0)
        }
        MidfielderMovement::Curved => {
            // 곡선: 앞으로 6m + 측면으로 5m
            let y_dir = if current_y > center_y { -1.0 } else { 1.0 };
            (attack_dir * 6.0, y_dir * 5.0)
        }
        MidfielderMovement::CheckToReceive => {
            // 뒤로 빠짐: 뒤로 6m
            (-attack_dir * 6.0, 0.0)
        }
        MidfielderMovement::OppositeRun => {
            // 반대편: 반대쪽으로 10m
            let y_dir = if current_y > center_y { -1.0 } else { 1.0 };
            (0.0, y_dir * 10.0)
        }
        MidfielderMovement::ThirdManRun => {
            // 3자 런: 앞으로 12m (패스 이후 공간으로)
            (attack_dir * 12.0, 0.0)
        }
    }
}

// ============================================================================
// Phase 4: Trap Readiness Functions (0113)
// ============================================================================

/// 오프사이드 트랩 준비 상태 평가
///
/// Open-Football 참고: 단순 on/off가 아니라 팀 능력과 라인 응집력 기반으로 평가
///
/// # Arguments
/// * `avg_teamwork` - 수비수 평균 팀워크 (0-20 → 0-100 스케일)
/// * `avg_experience` - 수비수 평균 경험 (0-20 → 0-100 스케일)
/// * `line_cohesion` - 수비 라인 응집력 상태
/// * `use_offside_trap` - 전술에서 오프사이드 트랩 활성화 여부
///
/// # Returns
/// * `OffsideTrapReadiness` - 트랩 준비 상태
///
/// # Example
/// ```ignore
/// let cohesion = calculate_line_cohesion(&defender_positions);
/// let readiness = evaluate_trap_readiness(72.0, 68.0, &cohesion, true);
/// if readiness.can_execute {
///     // 트랩 실행 가능
/// }
/// ```
pub fn evaluate_trap_readiness(
    avg_teamwork: f32,
    avg_experience: f32,
    line_cohesion: &DefensiveLineCohesion,
    use_offside_trap: bool,
) -> OffsideTrapReadiness {
    // 1. 전술에서 비활성화 → 즉시 반환
    if !use_offside_trap {
        return OffsideTrapReadiness {
            can_execute: false,
            cohesion_score: 0.0,
            reason: TrapBlockReason::TacticsDisabled,
        };
    }

    // 2. 팀워크 부족 (< 65)
    if avg_teamwork < MIN_TRAP_TEAMWORK {
        return OffsideTrapReadiness {
            can_execute: false,
            cohesion_score: avg_teamwork / 100.0,
            reason: TrapBlockReason::LowTeamwork,
        };
    }

    // 3. 경험 부족 (< 60)
    if avg_experience < MIN_TRAP_EXPERIENCE {
        return OffsideTrapReadiness {
            can_execute: false,
            cohesion_score: avg_experience / 100.0,
            reason: TrapBlockReason::LowExperience,
        };
    }

    // 4. 라인 응집력 부족 (편차 > 8m)
    if !line_cohesion.is_cohesive {
        // 편차가 클수록 점수 낮음: 8m=1.0, 15m=0.53, 23m=0.0
        let deviation_penalty = (line_cohesion.max_deviation / 15.0).min(1.0);
        return OffsideTrapReadiness {
            can_execute: false,
            cohesion_score: 1.0 - deviation_penalty,
            reason: TrapBlockReason::LineTooSpread,
        };
    }

    // 5. 모든 조건 충족 → 트랩 가능
    // 응집력 점수 = 팀워크 기여 * 라인 일관성
    // 팀워크 80/100 + 라인 편차 4m → 0.8 * (1 - 4/15) ≈ 0.59
    let teamwork_factor = avg_teamwork / 100.0;
    let line_factor = 1.0 - (line_cohesion.max_deviation / 15.0).min(1.0);
    let cohesion_score = (teamwork_factor * line_factor).clamp(0.0, 1.0);

    OffsideTrapReadiness {
        can_execute: true,
        cohesion_score,
        reason: TrapBlockReason::Ready,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_tactics() -> ElasticTactics {
        ElasticTactics::default()
    }

    #[test]
    fn test_team_center_attacking() {
        let ball_pos = (60.0, CENTER_Y);
        let tactics = default_tactics();

        let center = calculate_team_center(ball_pos, true, &tactics, true);

        // 공격 시 공 뒤에 위치해야 함
        assert!(center.0 < ball_pos.0, "Team center should be behind ball when attacking");
        // Y축은 공 따라감
        assert!((center.1 - CENTER_Y).abs() < 1.0, "Team center Y should follow ball");
    }

    #[test]
    fn test_team_center_defending() {
        let ball_pos = (70.0, 50.0);
        let tactics = default_tactics();

        let center = calculate_team_center(ball_pos, false, &tactics, true);

        // 수비 시 공과 골대 사이
        assert!(center.0 < ball_pos.0, "Team center should be between ball and goal");
        assert!(center.0 > 0.0, "Team center should not be in goal");

        // Y축은 공 따라가되 덜 쏠림
        let ball_offset = ball_pos.1 - CENTER_Y;
        let center_offset = center.1 - CENTER_Y;
        assert!(
            center_offset.abs() < ball_offset.abs(),
            "Team should not fully follow ball sideways"
        );
    }

    #[test]
    fn test_line_heights_order() {
        let team_center = (40.0, CENTER_Y);
        let tactics = default_tactics();

        let (def_x, mid_x, att_x) = calculate_line_heights(team_center, &tactics, true, true);

        // 홈팀 공격 시: 수비 < 미드 < 공격 순서
        assert!(def_x < mid_x, "Defense should be behind midfield");
        assert!(mid_x < att_x, "Midfield should be behind attack");
    }

    #[test]
    fn test_team_shift_follows_ball() {
        let tactics = default_tactics();

        // 공이 왼쪽에 있을 때
        let shift_left = calculate_team_shift((50.0, 20.0), &tactics);
        // 공이 오른쪽에 있을 때
        let shift_right = calculate_team_shift((50.0, 48.0), &tactics);

        assert!(shift_left < 0.0, "Shift should be negative when ball is on left");
        assert!(shift_right > 0.0, "Shift should be positive when ball is on right");
    }

    #[test]
    fn test_cover_shift_magnet_effect() {
        let player_pos = (30.0, CENTER_Y);
        let threats =
            vec![ThreatInfo { opponent_idx: 0, position: (32.0, 38.0), threat_score: 0.8 }];

        let shift = calculate_cover_shift(player_pos, &threats, PositionLine::Defender);

        // 위협 쪽으로 끌려가야 함
        assert!(shift.1 > 0.0, "Should be pulled toward threat on right");
        // X축은 제한되어야 함
        assert!(shift.0.abs() < shift.1.abs() * 0.5, "X movement should be dampened");
    }

    #[test]
    fn test_cover_shift_no_threat() {
        let player_pos = (30.0, CENTER_Y);
        let threats: Vec<ThreatInfo> = vec![];

        let shift = calculate_cover_shift(player_pos, &threats, PositionLine::Defender);

        assert_eq!(shift, (0.0, 0.0), "No shift when no threats");
    }

    #[test]
    fn test_elastic_snapback_within_range() {
        let current = (50.0, CENTER_Y);
        let anchor = (52.0, 35.0);

        let snap = calculate_elastic_snapback(current, anchor, ELASTIC_MAX_STRETCH);

        assert!(snap.is_none(), "No snapback needed within max stretch");
    }

    #[test]
    fn test_elastic_snapback_overstretched() {
        let current = (50.0, CENTER_Y);
        let anchor = (65.0, CENTER_Y); // 15m away (> 8m max stretch)

        let snap = calculate_elastic_snapback(current, anchor, ELASTIC_MAX_STRETCH);

        assert!(snap.is_some(), "Snapback needed when overstretched");
        let (dx, _dy) = snap.unwrap();
        assert!(dx > 0.0, "Should snap toward anchor");
    }

    #[test]
    fn test_find_threats_in_zone() {
        let player_pos = (30.0, CENTER_Y);
        let opponents = vec![
            (32.0, 36.0), // 가까움
            (50.0, CENTER_Y), // 멀음
            (28.0, 32.0), // 가까움
        ];
        let ball_pos = (35.0, CENTER_Y);
        let goal_pos = (0.0, CENTER_Y);

        let threats = find_threats_in_zone(player_pos, 10.0, &opponents, ball_pos, goal_pos);

        assert_eq!(threats.len(), 2, "Should find 2 threats within zone");
    }

    #[test]
    fn test_role_offset_get_forward() {
        let offset = calculate_role_offset(RoleInstruction::GetForward, true, true);

        assert!(offset.0 > 0.0, "Get forward should push player forward");
    }

    #[test]
    fn test_role_offset_stay_back() {
        let offset = calculate_role_offset(RoleInstruction::StayBack, true, true);

        assert!(offset.0 < 0.0, "Stay back should pull player backward");
    }

    #[test]
    fn test_calculate_player_target_basic() {
        let mut state = TeamPositioningState::default();
        state.team_center = (40.0, CENTER_Y);
        state.defensive_line_x = 30.0;
        state.midfield_line_x = 45.0;
        state.attack_line_x = 65.0;
        state.has_possession = true;

        let offset = FormationOffset { x: 0.0, y: 10.0, line: PositionLine::Midfielder };

        let target = calculate_player_target(
            &state,
            offset,
            &[],
            RoleInstruction::Default,
            &ElasticTactics::default(),
            true,
        );

        // 미드필더 라인 + Y 오프셋
        assert!((target.0 - 45.0).abs() < 1.0, "Should be on midfield line");
        assert!((target.1 - 44.0).abs() < 1.0, "Should have Y offset");
    }

    #[test]
    fn test_field_boundary_clamping() {
        let mut state = TeamPositioningState::default();
        state.team_center = (40.0, 5.0); // 왼쪽 끝에 가까움
        state.defensive_line_x = 30.0;
        state.midfield_line_x = 45.0;
        state.attack_line_x = 65.0;
        state.team_shift_y = -30.0; // 극단적인 쏠림

        let offset = FormationOffset {
            x: 0.0,
            y: -20.0, // 왼쪽으로 더 밀기
            line: PositionLine::Midfielder,
        };

        let target = calculate_player_target(
            &state,
            offset,
            &[],
            RoleInstruction::Default,
            &ElasticTactics::default(),
            true,
        );

        assert!(target.1 >= 2.0, "Y should be clamped to field boundary");
    }

    #[test]
    fn test_update_team_positioning_state() {
        let mut state = TeamPositioningState::default();
        let tactics = default_tactics();
        let ball_pos = (60.0, 40.0);

        update_team_positioning_state(&mut state, ball_pos, true, &tactics, true);

        assert!(state.has_possession);
        assert!(state.team_center.0 > 0.0 && state.team_center.0 < FIELD_LENGTH);
        assert!(state.defensive_line_x < state.midfield_line_x);
    }

    // ========================================================================
    // Phase 1 Tests: TeamShape & OpponentShape (0113)
    // ========================================================================

    #[test]
    fn test_team_shape_compact() {
        // 10명이 20x20m에 뭉쳐있는 경우 → 높은 밀집도
        let positions: Vec<(f32, f32)> = (0..10)
            .map(|i| (30.0 + (i % 5) as f32 * 4.0, 24.0 + (i / 5) as f32 * 10.0))
            .collect();

        let shape = calculate_team_shape(&positions);

        assert!(shape.width < 25.0, "Width should be narrow: {}", shape.width);
        assert!(shape.depth < 20.0, "Depth should be short: {}", shape.depth);
        assert!(shape.compactness > 0.5, "Compactness should be high: {}", shape.compactness);
    }

    #[test]
    fn test_team_shape_spread() {
        // 10명이 45x55m에 퍼져있는 경우 → 낮은 밀집도
        let positions: Vec<(f32, f32)> = (0..10)
            .map(|i| (15.0 + (i % 5) as f32 * 11.0, 7.0 + (i / 5) as f32 * 27.0))
            .collect();
        // width = 27m (7 to 34), depth = 44m (15 to 59)
        // area = 27 * 44 = 1188m², compactness = 10 * 50 / 1188 ≈ 0.42

        let shape = calculate_team_shape(&positions);

        assert!(shape.width > 20.0, "Width should be wide: {}", shape.width);
        assert!(shape.depth > 30.0, "Depth should be long: {}", shape.depth);
        assert!(shape.compactness < 0.5, "Compactness should be low: {}", shape.compactness);
    }

    #[test]
    fn test_team_shape_empty() {
        let positions: Vec<(f32, f32)> = vec![];
        let shape = calculate_team_shape(&positions);

        assert_eq!(shape.width, 0.0);
        assert_eq!(shape.depth, 0.0);
        assert_eq!(shape.compactness, 0.0);
    }

    #[test]
    fn test_opponent_shape_high_line() {
        // 상대가 높은 라인 (수비 라인 X > 42m)
        // is_attacking_right=true: 홈팀이 오른쪽 공격 → 상대(어웨이)는 왼쪽 골대 수비
        let positions: Vec<(f32, f32)> = vec![
            (50.0, 20.0), // 수비수들이 하프라인 넘어 올라옴
            (48.0, 30.0),
            (52.0, 38.0),
            (47.0, 48.0),
            (55.0, 25.0), // 미드필더
            (58.0, CENTER_Y),
            (56.0, 43.0),
            (65.0, 28.0), // 공격수
            (68.0, CENTER_Y),
            (66.0, 40.0),
        ];

        let shape = analyze_opponent_shape(&positions, true);

        assert!(shape.high_line, "Should detect high line");
        assert!(shape.defensive_line_x > 42.0, "Defensive line should be high: {}", shape.defensive_line_x);
    }

    #[test]
    fn test_opponent_shape_deep_line() {
        // 상대가 낮은 라인 (수비 라인 X < 30m)
        let positions: Vec<(f32, f32)> = vec![
            (25.0, 20.0), // 수비수들이 깊이 내려감
            (22.0, 30.0),
            (28.0, 38.0),
            (24.0, 48.0),
            (35.0, 25.0), // 미드필더
            (38.0, CENTER_Y),
            (36.0, 43.0),
            (45.0, 28.0), // 공격수
            (48.0, CENTER_Y),
            (46.0, 40.0),
        ];

        let shape = analyze_opponent_shape(&positions, true);

        assert!(!shape.high_line, "Should detect deep line");
        assert!(shape.defensive_line_x < 30.0, "Defensive line should be deep: {}", shape.defensive_line_x);
    }

    #[test]
    fn test_opponent_shape_compact_central() {
        // 중앙에 6명 이상 밀집 (Y: 24~44)
        let positions: Vec<(f32, f32)> = vec![
            (30.0, 30.0),
            (32.0, 32.0),
            (28.0, CENTER_Y),
            (35.0, 36.0),
            (33.0, 38.0),
            (31.0, 40.0), // 여기까지 6명 중앙
            (40.0, 10.0), // 측면
            (42.0, 58.0),
            (50.0, CENTER_Y),
            (55.0, CENTER_Y),
        ];

        let shape = analyze_opponent_shape(&positions, true);

        assert!(shape.compact_central, "Should detect compact central");
    }

    #[test]
    fn test_opponent_shape_narrow() {
        // 좁게 서있음 (width < 30m)
        let positions: Vec<(f32, f32)> = vec![
            (30.0, 24.0),
            (32.0, 28.0),
            (28.0, 32.0),
            (35.0, 36.0),
            (33.0, 40.0),
            (31.0, 44.0),
            (40.0, 26.0),
            (42.0, 42.0),
            (50.0, CENTER_Y),
            (55.0, CENTER_Y),
        ];

        let shape = analyze_opponent_shape(&positions, true);

        assert!(shape.narrow, "Should detect narrow formation");
    }

    #[test]
    fn test_line_cohesion_good() {
        // 수비수 4명이 같은 라인에 잘 서있음 (편차 < 8m)
        let defenders = vec![
            (30.0, 15.0),
            (32.0, 28.0),
            (29.0, 40.0),
            (31.0, 53.0),
        ];

        let cohesion = calculate_line_cohesion(&defenders);

        assert!(cohesion.is_cohesive, "Should be cohesive");
        assert!(cohesion.max_deviation < 8.0, "Max deviation should be low: {}", cohesion.max_deviation);
        assert!((cohesion.line_x - 30.5).abs() < 1.0, "Line should be around 30.5m");
    }

    #[test]
    fn test_line_cohesion_bad() {
        // 수비수 1명이 라인에서 벗어남 (편차 > 8m)
        let defenders = vec![
            (30.0, 15.0),
            (32.0, 28.0),
            (45.0, 40.0), // 15m 앞에 있음!
            (31.0, 53.0),
        ];

        let cohesion = calculate_line_cohesion(&defenders);

        assert!(!cohesion.is_cohesive, "Should not be cohesive");
        assert!(cohesion.max_deviation > 8.0, "Max deviation should be high: {}", cohesion.max_deviation);
    }

    #[test]
    fn test_line_cohesion_empty() {
        let defenders: Vec<(f32, f32)> = vec![];
        let cohesion = calculate_line_cohesion(&defenders);

        assert_eq!(cohesion.line_x, 0.0);
        assert_eq!(cohesion.max_deviation, 0.0);
        assert!(!cohesion.is_cohesive);
    }

    // ========================================================================
    // Phase 3 Tests: Movement Patterns (0113)
    // ========================================================================

    #[test]
    fn test_forward_movement_compact_central() {
        // 중앙 밀집 상대 → 대각선 런 (off_the_ball > 14)
        let shape = OpponentShape {
            high_line: false,
            compact_central: true,
            narrow: false,
            defensive_line_x: 35.0,
        };

        let movement = select_forward_movement(&shape, 15, 12);
        assert_eq!(movement, ForwardMovement::DiagonalRun);

        // off_the_ball <= 14 → 채널 런
        let movement2 = select_forward_movement(&shape, 12, 12);
        assert_eq!(movement2, ForwardMovement::ChannelRun);
    }

    #[test]
    fn test_forward_movement_high_line() {
        // 높은 라인 상대 → 직선 침투 (anticipation > 13)
        let shape = OpponentShape {
            high_line: true,
            compact_central: false,
            narrow: false,
            defensive_line_x: 50.0,
        };

        let movement = select_forward_movement(&shape, 12, 15);
        assert_eq!(movement, ForwardMovement::DirectRun);

        // anticipation <= 13 → 채널 런
        let movement2 = select_forward_movement(&shape, 12, 10);
        assert_eq!(movement2, ForwardMovement::ChannelRun);
    }

    #[test]
    fn test_forward_movement_narrow() {
        // 좁은 상대 → 측면 빠짐
        let shape = OpponentShape {
            high_line: false,
            compact_central: false,
            narrow: true,
            defensive_line_x: 35.0,
        };

        let movement = select_forward_movement(&shape, 12, 12);
        assert_eq!(movement, ForwardMovement::DriftWide);
    }

    #[test]
    fn test_forward_movement_default() {
        // 기본 상황 → 내려와서 받기
        let shape = OpponentShape::default();

        let movement = select_forward_movement(&shape, 12, 12);
        assert_eq!(movement, ForwardMovement::CheckToFeet);
    }

    #[test]
    fn test_midfielder_movement_attacking() {
        // 공격 시 + 높은 라인 + 능력 좋음 → 3자 런
        let shape = OpponentShape {
            high_line: true,
            compact_central: false,
            narrow: false,
            defensive_line_x: 50.0,
        };

        let movement = select_midfielder_movement(&shape, 15, true);
        assert_eq!(movement, MidfielderMovement::ThirdManRun);
    }

    #[test]
    fn test_midfielder_movement_compact() {
        // 공격 시 + 중앙 밀집 → 곡선 이동
        let shape = OpponentShape {
            high_line: false,
            compact_central: true,
            narrow: false,
            defensive_line_x: 35.0,
        };

        let movement = select_midfielder_movement(&shape, 12, true);
        assert_eq!(movement, MidfielderMovement::Curved);
    }

    #[test]
    fn test_midfielder_movement_defending() {
        // 수비 시 → 뒤로 빠짐
        let shape = OpponentShape::default();

        let movement = select_midfielder_movement(&shape, 15, false);
        assert_eq!(movement, MidfielderMovement::CheckToReceive);
    }

    #[test]
    fn test_forward_movement_to_offset_direct_run() {
        // 홈팀 직선 침투 → 앞으로 15m
        let offset = forward_movement_to_offset(ForwardMovement::DirectRun, true, CENTER_Y);
        assert!((offset.0 - 15.0).abs() < 0.1, "X offset should be 15m: {}", offset.0);
        assert!((offset.1).abs() < 0.1, "Y offset should be 0: {}", offset.1);

        // 어웨이팀 → 반대 방향
        let offset_away = forward_movement_to_offset(ForwardMovement::DirectRun, false, CENTER_Y);
        assert!((offset_away.0 + 15.0).abs() < 0.1, "Away X should be -15m");
    }

    #[test]
    fn test_forward_movement_to_offset_diagonal() {
        // 대각선 런 (현재 Y > 중앙) → 중앙 쪽으로 (Y 감소)
        let offset = forward_movement_to_offset(ForwardMovement::DiagonalRun, true, 50.0);
        assert!(offset.0 > 0.0, "Should move forward");
        assert!(offset.1 < 0.0, "Should move toward center (negative Y)");

        // 현재 Y < 중앙 → 중앙 쪽으로 (Y 증가)
        let offset2 = forward_movement_to_offset(ForwardMovement::DiagonalRun, true, 20.0);
        assert!(offset2.1 > 0.0, "Should move toward center (positive Y)");
    }

    #[test]
    fn test_midfielder_movement_to_offset() {
        // 직선 이동
        let offset = midfielder_movement_to_offset(MidfielderMovement::Direct, true, CENTER_Y);
        assert!((offset.0 - 8.0).abs() < 0.1, "X offset should be 8m");

        // 뒤로 빠짐
        let offset2 = midfielder_movement_to_offset(MidfielderMovement::CheckToReceive, true, CENTER_Y);
        assert!((offset2.0 + 6.0).abs() < 0.1, "X offset should be -6m");
    }

    // ========================================================================
    // Phase 4 Tests: Trap Readiness (0113)
    // ========================================================================

    #[test]
    fn test_trap_readiness_tactics_disabled() {
        // 전술에서 비활성화 → 트랩 불가
        let cohesion = DefensiveLineCohesion {
            line_x: 30.0,
            max_deviation: 3.0,
            is_cohesive: true,
        };

        let readiness = evaluate_trap_readiness(80.0, 75.0, &cohesion, false);

        assert!(!readiness.can_execute);
        assert_eq!(readiness.reason, TrapBlockReason::TacticsDisabled);
        assert_eq!(readiness.cohesion_score, 0.0);
    }

    #[test]
    fn test_trap_readiness_low_teamwork() {
        // 팀워크 부족 (< 65)
        let cohesion = DefensiveLineCohesion {
            line_x: 30.0,
            max_deviation: 3.0,
            is_cohesive: true,
        };

        let readiness = evaluate_trap_readiness(60.0, 75.0, &cohesion, true);

        assert!(!readiness.can_execute);
        assert_eq!(readiness.reason, TrapBlockReason::LowTeamwork);
        assert!((readiness.cohesion_score - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_trap_readiness_low_experience() {
        // 경험 부족 (< 60)
        let cohesion = DefensiveLineCohesion {
            line_x: 30.0,
            max_deviation: 3.0,
            is_cohesive: true,
        };

        let readiness = evaluate_trap_readiness(70.0, 55.0, &cohesion, true);

        assert!(!readiness.can_execute);
        assert_eq!(readiness.reason, TrapBlockReason::LowExperience);
        assert!((readiness.cohesion_score - 0.55).abs() < 0.01);
    }

    #[test]
    fn test_trap_readiness_line_spread() {
        // 라인 편차 큼 (> 8m)
        let cohesion = DefensiveLineCohesion {
            line_x: 30.0,
            max_deviation: 12.0,
            is_cohesive: false,
        };

        let readiness = evaluate_trap_readiness(80.0, 75.0, &cohesion, true);

        assert!(!readiness.can_execute);
        assert_eq!(readiness.reason, TrapBlockReason::LineTooSpread);
        // cohesion_score = 1.0 - 12.0/15.0 = 0.2
        assert!((readiness.cohesion_score - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_trap_readiness_all_conditions_met() {
        // 모든 조건 충족
        let cohesion = DefensiveLineCohesion {
            line_x: 30.0,
            max_deviation: 4.0,
            is_cohesive: true,
        };

        let readiness = evaluate_trap_readiness(80.0, 75.0, &cohesion, true);

        assert!(readiness.can_execute);
        assert_eq!(readiness.reason, TrapBlockReason::Ready);
        // cohesion_score = 0.8 * (1.0 - 4.0/15.0) = 0.8 * 0.733 ≈ 0.587
        assert!(readiness.cohesion_score > 0.5 && readiness.cohesion_score < 0.7,
            "Cohesion score should be around 0.58: {}", readiness.cohesion_score);
    }

    #[test]
    fn test_trap_readiness_perfect_conditions() {
        // 완벽한 조건: 팀워크 100, 경험 100, 편차 0
        let cohesion = DefensiveLineCohesion {
            line_x: 30.0,
            max_deviation: 0.0,
            is_cohesive: true,
        };

        let readiness = evaluate_trap_readiness(100.0, 100.0, &cohesion, true);

        assert!(readiness.can_execute);
        assert_eq!(readiness.reason, TrapBlockReason::Ready);
        // cohesion_score = 1.0 * 1.0 = 1.0
        assert!((readiness.cohesion_score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_trap_readiness_boundary_teamwork() {
        // 경계값: 팀워크 정확히 65
        let cohesion = DefensiveLineCohesion {
            line_x: 30.0,
            max_deviation: 3.0,
            is_cohesive: true,
        };

        let readiness = evaluate_trap_readiness(65.0, 70.0, &cohesion, true);

        // 65.0 >= 65.0 → 통과
        assert!(readiness.can_execute);
        assert_eq!(readiness.reason, TrapBlockReason::Ready);
    }

    #[test]
    fn test_trap_readiness_boundary_experience() {
        // 경계값: 경험 정확히 60
        let cohesion = DefensiveLineCohesion {
            line_x: 30.0,
            max_deviation: 3.0,
            is_cohesive: true,
        };

        let readiness = evaluate_trap_readiness(70.0, 60.0, &cohesion, true);

        // 60.0 >= 60.0 → 통과
        assert!(readiness.can_execute);
        assert_eq!(readiness.reason, TrapBlockReason::Ready);
    }
}
