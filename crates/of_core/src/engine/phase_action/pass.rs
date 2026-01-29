//! Pass Action FSM
//!
//! P7 Spec Section 2.4: Pass Phase with Windup, InFlight, Arrival
//! P7-PASS-TUNE: Intent → Technique → Physics 시스템
//!
//! ## 핵심 원칙: Intent → Technique → Physics
//! ```text
//! 상황 인식 → Intent 결정 (왜?) → Technique 선택 (어떻게?) → Physics 실행
//! ```
//!
//! ## Intent (5가지)
//! - Retain: 안전 유지 (뒤/옆)
//! - Progress: 전진 전개 (라인 타기)
//! - Penetrate: 침투 (뒷공간)
//! - Switch: 전환 (반대)
//! - Escape: 탈압박 (세이프티)
//!
//! ## Technique (6가지)
//! - Ground: 땅볼 패스
//! - Driven: 강한 땅볼
//! - Lofted: 로빙
//! - Cross: 크로스
//! - Through: 스루패스
//! - Clear: 세이프티 클리어
//!
//! ## Pass Phase Flow
//! ```text
//! Windup (2 ticks) → Kick (1 tick) → InFlight (N ticks) → Arrival
//! ```

use super::action_common::{
    clamp01, lerp as common_lerp, skill01, weighted_choice_index, ActionModel,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
// P0: Core types moved to action_queue
use super::super::action_queue::{
    BallIntentKind, BallTrajectoryIntent, CurveDirection, HeightClass, PassType, SpeedClass,
};
use super::ball_physics::HeightCurve;
use super::duration::*;
use crate::models::player::PlayerAttributes;

// ============================================================================
// P7-PASS-TUNE: Intent → Technique System
// ============================================================================

/// 패스 의도 (Intent) - "왜" 패스하는가?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PassIntent {
    /// 안전 유지 (뒤/옆 패스)
    /// - 점유율 유지, 시간 벌기
    Retain,

    /// 전진 전개 (라인 타기)
    /// - 앞으로 전개하지만 위험하지 않게
    Progress,

    /// 침투 (뒷공간)
    /// - 수비 라인 뒤로 찌르는 패스
    Penetrate,

    /// 전환 (반대 전개)
    /// - 왼쪽 ↔ 오른쪽 사이드 체인지
    Switch,

    /// 탈압박 (세이프티)
    /// - 위험 회피, 클리어
    Escape,
}

/// 패스 기술 (Technique) - "어떻게" 패스하는가?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PassTechnique {
    /// 땅볼 패스 (짧~중거리)
    Ground,

    /// 강한 땅볼 (중~장거리, 속도↑)
    Driven,

    /// 로빙 (공중, 뒤 공간)
    Lofted,

    /// 크로스 (측면 → 박스)
    Cross,

    /// 스루패스 (리드/침투 전용)
    Through,

    /// 세이프티 클리어 (위험 회피)
    Clear,
}

/// 패스 물리 파라미터
#[derive(Debug, Clone, Copy)]
pub struct PassPhysicsParams {
    /// 공의 진행 속도 (m/s)
    pub speed: f32,
    /// 공이 발에서 떨어지는 거리 (m)
    pub ball_distance: f32,
    /// 공중 높이 (0=땅볼, 1=최대 로브)
    pub height: f32,
    /// 리드 거리 (받는 지점 앞쪽으로 찌르는 거리)
    pub lead_distance: f32,
    /// 실패/차단 리스크 (0~1)
    pub risk: f32,
    /// 액션 지속시간 (틱)
    pub duration_ticks: u8,
}

impl PassIntent {
    /// Intent → 허용 Technique 후보군
    pub fn available_techniques(&self) -> &'static [PassTechnique] {
        match self {
            Self::Retain => &[PassTechnique::Ground, PassTechnique::Driven],
            Self::Progress => {
                &[PassTechnique::Ground, PassTechnique::Driven, PassTechnique::Lofted]
            }
            Self::Penetrate => {
                &[PassTechnique::Through, PassTechnique::Lofted, PassTechnique::Driven]
            }
            Self::Switch => &[PassTechnique::Lofted, PassTechnique::Driven],
            Self::Escape => &[PassTechnique::Clear, PassTechnique::Ground],
        }
    }

    /// Intent 이름
    pub fn name(&self) -> &'static str {
        match self {
            Self::Retain => "Retain",
            Self::Progress => "Progress",
            Self::Penetrate => "Penetrate",
            Self::Switch => "Switch",
            Self::Escape => "Escape",
        }
    }
}

impl PassTechnique {
    /// Technique → Physics 파라미터 테이블
    pub fn physics_params(&self) -> PassPhysicsParams {
        match self {
            Self::Ground => PassPhysicsParams {
                speed: 11.0,
                ball_distance: 0.35,
                height: 0.0,
                lead_distance: 0.5,
                risk: 0.20,
                duration_ticks: 14,
            },
            Self::Driven => PassPhysicsParams {
                speed: 15.0,
                ball_distance: 0.40,
                height: 0.0,
                lead_distance: 0.8,
                risk: 0.35,
                duration_ticks: 12,
            },
            Self::Lofted => PassPhysicsParams {
                speed: 12.5,
                ball_distance: 0.55,
                height: 0.70,
                lead_distance: 1.0,
                risk: 0.45,
                duration_ticks: 20,
            },
            Self::Cross => PassPhysicsParams {
                speed: 13.5,
                ball_distance: 0.60,
                height: 0.85,
                lead_distance: 0.9,
                risk: 0.55,
                duration_ticks: 22,
            },
            Self::Through => PassPhysicsParams {
                speed: 14.0,
                ball_distance: 0.45,
                height: 0.10,
                lead_distance: 2.0,
                risk: 0.60,
                duration_ticks: 16,
            },
            Self::Clear => PassPhysicsParams {
                speed: 17.0,
                ball_distance: 0.70,
                height: 0.90,
                lead_distance: 0.5,
                risk: 0.25,
                duration_ticks: 18,
            },
        }
    }

    /// Technique 이름
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ground => "Ground",
            Self::Driven => "Driven",
            Self::Lofted => "Lofted",
            Self::Cross => "Cross",
            Self::Through => "Through",
            Self::Clear => "Clear",
        }
    }

    /// 기본 성공 확률 (0~1)
    pub fn base_success_rate(&self) -> f32 {
        match self {
            Self::Ground => 0.85,
            Self::Driven => 0.75,
            Self::Lofted => 0.70,
            Self::Cross => 0.60,
            Self::Through => 0.55,
            Self::Clear => 0.90,
        }
    }

    /// PassType으로 변환 (기존 시스템 호환)
    pub fn to_pass_type(&self) -> PassType {
        match self {
            Self::Ground => PassType::Ground,
            Self::Driven => PassType::Ground, // Driven은 Ground의 강화 버전
            Self::Lofted => PassType::Lofted,
            Self::Cross => PassType::Cross,
            Self::Through => PassType::ThroughBall,
            Self::Clear => PassType::Lofted, // Clear는 Lofted의 안전 버전
        }
    }
}

// ============================================================================
// P7-PASS-TUNE: Technique Selection & Resolve
// ============================================================================

/// 패스 기술 선택 결과
#[derive(Debug, Clone, Copy)]
pub struct PassTechniqueSelection {
    pub technique: PassTechnique,
    pub success_chance: f32,
}

/// 패스 컨텍스트 (기술 선택에 사용)
#[derive(Debug, Clone, Copy, Default)]
pub struct PassContext {
    /// 거리 (0=가까운 짧패스, 1=긴 패스)
    pub distance_norm: f32,
    /// 타겟 열림 정도 (1=잘 열림)
    pub target_openness: f32,
    /// 침투 라인 존재 (1=뒷공간 있음)
    pub through_lane: f32,
    /// 크로스각 (1=측면 크로스각 좋음)
    pub crossing_angle: f32,
}

/// 패스 스킬 (기술 선택에 사용)
#[derive(Debug, Clone, Copy, Default)]
pub struct PassSkills {
    pub passing: u8,
    pub technique: u8,
    pub vision: u8,
    pub decisions: u8,
    pub composure: u8,
}

/// FM 메타 기반 패서 스킬 (FIX_2601/0107)
///
/// FM 메타 분석 결과 반영:
/// - Anticipation: +14 points (Tier 2)
/// - Concentration: +12 points (Tier 2)
/// - Teamwork: 패스 수신에 중요 (FM 실험 결과)
#[derive(Debug, Clone, Copy)]
pub struct PassSkillsFM {
    /// 기본 스킬
    pub base: PassSkills,
    /// 예측력 (0-99)
    pub anticipation: u8,
    /// 집중력 (0-99)
    pub concentration: u8,
}

impl PassSkillsFM {
    /// 기본 PassSkills로 변환 (backward compatibility)
    pub fn as_base(&self) -> &PassSkills {
        &self.base
    }
}

impl PassSkills {
    /// PlayerAttributes에서 생성
    pub fn from_attributes(attrs: &PlayerAttributes) -> Self {
        Self {
            passing: attrs.passing,
            technique: attrs.technique,
            vision: attrs.vision,
            decisions: attrs.decisions,
            composure: attrs.composure,
        }
    }
}

/// 능력치 기반 기술 선택 (Intent → Technique)
pub fn choose_pass_technique(
    intent: PassIntent,
    ctx: &PassContext,
    skills: &PassSkills,
    pressure: f32,
) -> PassTechniqueSelection {
    let pressure = pressure.clamp(0.0, 1.0);
    let candidates = intent.available_techniques();

    if candidates.is_empty() {
        return PassTechniqueSelection { technique: PassTechnique::Ground, success_chance: 0.5 };
    }

    // 스킬 정규화 (0~1)
    let pass_n = skills.passing as f32 / 100.0;
    let tech_n = skills.technique as f32 / 100.0;
    let vis_n = skills.vision as f32 / 100.0;
    let dec_n = skills.decisions as f32 / 100.0;
    let comp_n = skills.composure as f32 / 100.0;

    let mut best_technique = candidates[0];
    let mut best_score = 0.0f32;

    for &tech in candidates {
        let mut score = 1.0;

        // 스킬 적합도 (0.6~1.6)
        let skill_fit = match tech {
            PassTechnique::Ground => 0.55 * pass_n + 0.25 * dec_n + 0.20 * comp_n,
            PassTechnique::Driven => 0.55 * pass_n + 0.35 * tech_n + 0.10 * dec_n,
            PassTechnique::Lofted => 0.45 * pass_n + 0.35 * tech_n + 0.20 * vis_n,
            PassTechnique::Cross => 0.35 * pass_n + 0.45 * tech_n + 0.20 * vis_n,
            PassTechnique::Through => 0.35 * pass_n + 0.25 * tech_n + 0.40 * vis_n,
            PassTechnique::Clear => 0.20 * pass_n + 0.10 * vis_n + 0.70 * dec_n,
        };
        score *= 0.6 + 1.0 * skill_fit.clamp(0.0, 1.0);

        // 상황 적합도
        let d = ctx.distance_norm;
        let open = ctx.target_openness;
        let lane = ctx.through_lane;
        let cross = ctx.crossing_angle;

        score *= match tech {
            PassTechnique::Ground => lerp(1.3, 0.8, d),
            PassTechnique::Driven => lerp(0.9, 1.3, d),
            PassTechnique::Lofted => lerp(0.8, 1.2, d) * lerp(0.9, 1.1, 1.0 - open),
            PassTechnique::Cross => lerp(0.7, 1.4, cross) * lerp(0.8, 1.2, d),
            PassTechnique::Through => lerp(0.6, 1.5, lane) * lerp(0.9, 1.1, d),
            PassTechnique::Clear => lerp(1.0, 1.4, pressure) * lerp(1.0, 1.2, 1.0 - open),
        };

        // PassType risk bias: high-pass traps are harder, long/through get mild bonuses
        let mut type_risk = 1.0;
        if matches!(tech, PassTechnique::Lofted | PassTechnique::Cross) {
            let trap_penalty = lerp(1.0, 0.75, (1.0 - open).clamp(0.0, 1.0));
            let short_penalty = if d < 0.25 { lerp(0.6, 1.0, d / 0.25) } else { 1.0 };
            type_risk *= trap_penalty * short_penalty;
            // FIX_2601/0117: Lofted 장거리 보너스 (d > 0.5 = 25m 이상)
            if matches!(tech, PassTechnique::Lofted) && d > 0.5 {
                let long_bonus = 1.0 + (d - 0.5) * 0.4; // 최대 1.2x at d=1.0
                type_risk *= long_bonus;
            }
        }

        if matches!(tech, PassTechnique::Through) {
            let lane_bonus = lerp(1.0, 1.15, lane);
            let long_bonus = lerp(1.0, 1.10, d);
            type_risk *= lane_bonus * long_bonus;
        } else if matches!(tech, PassTechnique::Driven) {
            let long_bonus = lerp(1.0, 1.08, d);
            type_risk *= long_bonus;
        }

        score *= type_risk;

        // 압박 패널티 (복잡한 기술일수록 불리)
        let complexity = match tech {
            PassTechnique::Ground => 0.15,
            PassTechnique::Driven => 0.30,
            PassTechnique::Lofted => 0.45,
            PassTechnique::Cross => 0.55,
            PassTechnique::Through => 0.60,
            PassTechnique::Clear => 0.10,
        };
        let press_resist = 0.45 * comp_n + 0.35 * dec_n + 0.20 * tech_n;
        let eff_pressure = (pressure * (1.0 + complexity)).max(0.0);
        score *= lerp(1.0, 0.45, (eff_pressure * (1.0 - press_resist)).clamp(0.0, 1.0));

        // Debug logging for technique selection
        #[cfg(debug_assertions)]
        if crate::engine::debug_flags::action_debug_enabled() {
            eprintln!(
                "[DEBUG TECH] {:?}: score={:.3} (skill={:.2} ctx={:.2} risk={:.2} press={:.2}) d={:.2} open={:.2}",
                tech, score, skill_fit,
                match tech {
                    PassTechnique::Ground => lerp(1.3, 0.8, d),
                    PassTechnique::Driven => lerp(0.9, 1.3, d),
                    PassTechnique::Lofted => lerp(0.8, 1.2, d) * lerp(0.9, 1.1, 1.0 - open),
                    _ => 1.0,
                },
                type_risk,
                lerp(1.0, 0.45, (eff_pressure * (1.0 - press_resist)).clamp(0.0, 1.0)),
                d, open
            );
        }

        if score > best_score {
            best_score = score;
            best_technique = tech;
        }
    }

    // Debug: Log final selection
    #[cfg(debug_assertions)]
    if crate::engine::debug_flags::action_debug_enabled() {
        eprintln!(
            "[DEBUG TECH] SELECTED: {:?} (score={:.3}) intent={:?}",
            best_technique, best_score, intent
        );
    }

    // 성공 확률 계산
    let base_success = best_technique.base_success_rate();
    let skill_bonus = (pass_n + tech_n + vis_n) / 3.0 * 0.15;
    let pressure_penalty = pressure * (1.0 - comp_n * 0.5) * 0.2;
    let success_chance = (base_success + skill_bonus - pressure_penalty).clamp(0.1, 0.95);

    PassTechniqueSelection { technique: best_technique, success_chance }
}

/// 패스 성공 확률 계산
pub fn pass_base_success_prob(technique: PassTechnique, skills: &PassSkills, pressure: f32) -> f32 {
    let pressure = pressure.clamp(0.0, 1.0);

    let pass_n = skills.passing as f32 / 100.0;
    let tech_n = skills.technique as f32 / 100.0;
    let vis_n = skills.vision as f32 / 100.0;
    let dec_n = skills.decisions as f32 / 100.0;
    let comp_n = skills.composure as f32 / 100.0;

    // 기술별 난이도
    let difficulty = match technique {
        PassTechnique::Ground => 0.25,
        PassTechnique::Driven => 0.35,
        PassTechnique::Lofted => 0.45,
        PassTechnique::Cross => 0.55,
        PassTechnique::Through => 0.60,
        PassTechnique::Clear => 0.20,
    };

    // 스킬 합성
    let skill = match technique {
        PassTechnique::Ground => 0.55 * pass_n + 0.20 * dec_n + 0.25 * comp_n,
        PassTechnique::Driven => 0.50 * pass_n + 0.35 * tech_n + 0.15 * dec_n,
        PassTechnique::Lofted => 0.45 * pass_n + 0.35 * tech_n + 0.20 * vis_n,
        PassTechnique::Cross => 0.35 * pass_n + 0.45 * tech_n + 0.20 * vis_n,
        PassTechnique::Through => 0.35 * pass_n + 0.25 * tech_n + 0.40 * vis_n,
        PassTechnique::Clear => 0.35 * dec_n + 0.25 * comp_n + 0.40 * pass_n,
    };

    // 압박 저항
    let press_resist = 0.45 * comp_n + 0.35 * dec_n + 0.20 * tech_n;

    let mut p = 0.55 + 0.60 * (skill - difficulty);
    p -= pressure * lerp(0.22, 0.08, press_resist);

    p.clamp(0.02, 0.98)
}

/// FM 메타 기반 패스 성공 확률 계산 (FIX_2601/0107)
///
/// FM-Arena 테스트 결과 반영:
/// - passing(40%) + vision(20%) + technique(15%)
/// - decisions(10%) + anticipation(8%) + concentration(7%)
///
/// 기존 함수와의 차이:
/// - 전통적 passing 가중치 감소 (55% → 40%)
/// - 정신적 속성(anticipation, concentration) 추가
/// - 압박 저항에 concentration 영향
pub fn pass_base_success_prob_fm_meta(
    technique: PassTechnique,
    skills: &PassSkillsFM,
    pressure: f32,
) -> f32 {
    use crate::engine::match_sim::attribute_calc::pass_skill_fm_meta;

    let pressure = pressure.clamp(0.0, 1.0);

    // FM 메타 공식으로 기본 스킬 계산
    let fm_skill = pass_skill_fm_meta(
        skills.base.passing as f32,
        skills.base.vision as f32,
        skills.base.technique as f32,
        skills.base.decisions as f32,
        skills.anticipation as f32,
        skills.concentration as f32,
    );

    // 기술별 난이도 보정 (원래 로직 유지)
    let difficulty_mult = match technique {
        PassTechnique::Ground => 1.0,
        PassTechnique::Driven => 0.95,
        PassTechnique::Lofted => 0.90,
        PassTechnique::Cross => 0.85,
        PassTechnique::Through => 0.80,
        PassTechnique::Clear => 1.0,
    };

    // 압박 저항 (FM 메타: composure + concentration 중요)
    let comp_n = skills.base.composure as f32 / 100.0;
    let conc_n = skills.concentration as f32 / 100.0;
    let dec_n = skills.base.decisions as f32 / 100.0;
    let press_resist = 0.40 * comp_n + 0.35 * conc_n + 0.25 * dec_n;

    // 기본 성공률에 난이도와 압박 적용
    let mut p = fm_skill * difficulty_mult;
    p -= pressure * lerp(0.25, 0.08, press_resist);

    p.clamp(0.02, 0.98)
}

/// 패스 수신 성공률에 teamwork 보너스 적용 (FIX_2601/0107)
///
/// FM 실험 결과: Teamwork=1이면 패스 수신 대폭 감소
///
/// # Arguments
/// * `base_reception_prob` - 기본 수신 확률
/// * `receiver_teamwork` - 수신자 팀워크 (0-99)
///
/// # Returns
/// teamwork 보정된 수신 확률
pub fn apply_teamwork_reception_bonus(base_reception_prob: f32, receiver_teamwork: u8) -> f32 {
    use crate::engine::match_sim::attribute_calc::teamwork_reception_bonus;

    let bonus = teamwork_reception_bonus(receiver_teamwork as f32);
    (base_reception_prob + bonus).clamp(0.0, 1.0)
}

/// 패스 실행 오차 계산 (방향, 속도, 높이 시그마)
pub fn pass_execution_error(
    technique: PassTechnique,
    skills: &PassSkills,
    pressure: f32,
) -> (f32, f32, f32) {
    let pressure = pressure.clamp(0.0, 1.0);

    let pass_n = skills.passing as f32 / 100.0;
    let tech_n = skills.technique as f32 / 100.0;
    let comp_n = skills.composure as f32 / 100.0;
    let dec_n = skills.decisions as f32 / 100.0;

    let control = 0.45 * pass_n + 0.30 * tech_n + 0.25 * comp_n;
    let calm = 0.60 * comp_n + 0.40 * dec_n;

    // 기본 오차 (방향 라디안, 속도/높이 비율)
    let (base_dir, base_speed, base_h) = match technique {
        PassTechnique::Ground => (0.06, 0.05, 0.00),
        PassTechnique::Driven => (0.08, 0.07, 0.00),
        PassTechnique::Lofted => (0.10, 0.08, 0.10),
        PassTechnique::Cross => (0.12, 0.08, 0.14),
        PassTechnique::Through => (0.13, 0.09, 0.06),
        PassTechnique::Clear => (0.09, 0.10, 0.16),
    };

    // 스킬이 높을수록 오차 감소
    let skill_factor = lerp(1.35, 0.55, control.clamp(0.0, 1.0));

    // 압박이 높을수록 오차 증가 (침착할수록 덜)
    let press_factor = 1.0 + pressure * lerp(0.90, 0.35, calm.clamp(0.0, 1.0));

    (
        base_dir * skill_factor * press_factor,
        base_speed * skill_factor * press_factor,
        base_h * skill_factor * press_factor,
    )
}

/// 선형 보간
#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// ============================================================================
// Pass Constants
// ============================================================================

/// 트랩 성공 범위 (m)
pub const TRAP_SUCCESS_RANGE: f32 = 1.5;

/// 인터셉트 범위 (m)
pub const INTERCEPT_RANGE: f32 = 2.0;

// ============================================================================
// PassPhase Enum
// ============================================================================

/// 패스 Phase (FSM 상태)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PassPhase {
    /// 패스 준비 (방향 보기, 체중 이동)
    Windup { remaining_ticks: u8 },

    /// 킥 동작 (발이 공에 닿는 순간)
    Kick,

    /// 공 비행 중 (Ball FSM이 처리)
    InFlight { arrival_tick: u64 },

    /// 도착 (트랩/인터셉트 판정)
    Arrival,

    /// 완료
    Finished,
}

impl PassPhase {
    /// Phase 이름
    pub fn name(&self) -> &'static str {
        match self {
            PassPhase::Windup { .. } => "Windup",
            PassPhase::Kick => "Kick",
            PassPhase::InFlight { .. } => "InFlight",
            PassPhase::Arrival => "Arrival",
            PassPhase::Finished => "Finished",
        }
    }

    /// 활성 상태인지
    pub fn is_active(&self) -> bool {
        !matches!(self, PassPhase::Finished)
    }
}

// ============================================================================
// PassResult
// ============================================================================

/// 패스 결과
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PassResult {
    /// 패스 계속 진행 중
    Continue,
    /// 패스 시작됨
    Started { passer_idx: usize, receiver_idx: usize, arrival_tick: u64, ball_speed: f32 },
    /// 트랩 성공
    TrapSuccess { receiver_idx: usize },
    /// 트랩 실패 (루즈볼)
    TrapFailed { receiver_idx: usize, ball_position: (f32, f32) },
    /// 인터셉트 성공
    InterceptSuccess { interceptor_idx: usize },
    /// 패스 미스 (리시버가 범위 밖)
    Missed { passer_idx: usize, intended_receiver_idx: usize },
}

// ============================================================================
// PassAction Struct
// ============================================================================

/// 실행 중인 패스 액션
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassAction {
    /// 액션 ID
    pub id: u64,

    /// 패스 Phase
    pub phase: PassPhase,

    /// 패스하는 선수 인덱스
    pub passer_idx: usize,

    /// 패서 팀 ID
    pub passer_team: u32,

    /// 패스 받을 선수 인덱스
    pub receiver_idx: usize,

    /// 패스 종류
    pub pass_type: PassType,

    /// 목표 위치 (receiver 위치 또는 공간)
    pub target_pos: (f32, f32),

    /// 패스 속도 (m/s)
    pub pass_speed: f32,

    /// 커브 팩터 (-0.35 ~ 0.35)
    pub curve_factor: f32,

    /// 시작 틱
    pub start_tick: u64,

    /// 도착 예정 틱 (InFlight 시 설정)
    arrival_tick: u64,

    /// Viewer 이벤트 (Contact Frame에서 생성, 외부에서 수집)
    #[serde(skip)]
    pending_viewer_event: Option<BallTrajectoryIntent>,

    // ========== ActionModel Integration ==========
    /// 패스 의도 (왜 패스하는가)
    pub intent: PassIntent,

    /// 선택된 기술 (어떻게 패스하는가)
    pub technique: PassTechnique,

    // ========== P17 Phase 4: 스킬 필드 ==========
    /// 패스 정확도 (0-99)
    pub passing: u8,
    /// 시야/판단력 (0-99)
    pub vision: u8,
    /// 커브 능력 (0-99)
    pub curve_skill: u8,
}

impl PassAction {
    /// 새 패스 액션 생성
    pub fn new(
        id: u64,
        passer_idx: usize,
        passer_team: u32,
        receiver_idx: usize,
        pass_type: PassType,
        target_pos: (f32, f32),
        start_tick: u64,
    ) -> Self {
        // 레거시 호환: PassType에서 기본 Intent/Technique 추론
        let (intent, technique) = Self::infer_intent_technique_from_pass_type(pass_type);
        Self {
            id,
            phase: PassPhase::Windup { remaining_ticks: pass_type.windup_ticks() },
            passer_idx,
            passer_team,
            receiver_idx,
            pass_type,
            target_pos,
            pass_speed: pass_type.base_speed(),
            curve_factor: 0.0,
            start_tick,
            arrival_tick: 0,
            pending_viewer_event: None,
            intent,
            technique,
            // P17: 스킬 필드 기본값
            passing: 0,
            vision: 0,
            curve_skill: 0,
        }
    }

    /// Intent와 Technique를 지정하여 패스 액션 생성 (ActionModel 통합용)
    pub fn new_with_intent(
        id: u64,
        passer_idx: usize,
        passer_team: u32,
        receiver_idx: usize,
        intent: PassIntent,
        technique: PassTechnique,
        target_pos: (f32, f32),
        start_tick: u64,
    ) -> Self {
        let pass_type = technique.to_pass_type();
        let physics = technique.physics_params();
        Self {
            id,
            phase: PassPhase::Windup {
                remaining_ticks: physics.duration_ticks / 3, // Windup = 1/3 of duration
            },
            passer_idx,
            passer_team,
            receiver_idx,
            pass_type,
            target_pos,
            pass_speed: physics.speed,
            curve_factor: 0.0,
            start_tick,
            arrival_tick: 0,
            pending_viewer_event: None,
            intent,
            technique,
            passing: 0,
            vision: 0,
            curve_skill: 0,
        }
    }

    /// PassType에서 Intent/Technique 추론 (레거시 호환)
    fn infer_intent_technique_from_pass_type(pass_type: PassType) -> (PassIntent, PassTechnique) {
        match pass_type {
            PassType::Ground => (PassIntent::Retain, PassTechnique::Ground),
            PassType::Lofted => (PassIntent::Switch, PassTechnique::Lofted),
            PassType::ThroughBall => (PassIntent::Penetrate, PassTechnique::Through),
            PassType::Cross => (PassIntent::Penetrate, PassTechnique::Cross),
            PassType::BackPass => (PassIntent::Retain, PassTechnique::Ground),
        }
    }

    /// P17 Phase 4: 능력치와 함께 패스 액션 생성
    pub fn new_with_attrs(
        id: u64,
        passer_idx: usize,
        passer_team: u32,
        receiver_idx: usize,
        pass_type: PassType,
        target_pos: (f32, f32),
        start_tick: u64,
        attrs: &PlayerAttributes,
    ) -> Self {
        let mut action =
            Self::new(id, passer_idx, passer_team, receiver_idx, pass_type, target_pos, start_tick);
        action.passing = attrs.passing;
        action.vision = attrs.vision;
        action.curve_skill = attrs.technique; // technique가 커브 능력과 가장 유사
        action
    }

    /// 커브 설정
    pub fn with_curve(mut self, factor: f32) -> Self {
        self.curve_factor = factor.clamp(-0.35, 0.35);
        self
    }

    /// 속도 조정
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.pass_speed = speed.max(5.0); // 최소 5m/s
        self
    }

    /// 패스 완료 여부
    pub fn is_finished(&self) -> bool {
        matches!(self.phase, PassPhase::Finished)
    }

    /// 높이 곡선 가져오기
    pub fn height_curve(&self) -> HeightCurve {
        HeightCurve::from_pass_type(self.pass_type)
    }

    /// Viewer 이벤트 수집 (Contact Frame에서 생성된 이벤트를 꺼냄)
    pub fn take_viewer_event(&mut self) -> Option<BallTrajectoryIntent> {
        self.pending_viewer_event.take()
    }

    // ========================================================================
    // Tick Update
    // ========================================================================

    /// 매 틱 업데이트
    ///
    /// Parameters:
    /// - current_tick: 현재 틱
    /// - passer_pos: 패서 위치
    /// - ball_pos: 공 위치
    /// - receiver_pos: 리시버 위치
    ///
    /// Returns: (PassResult, target_direction)
    pub fn update_tick(
        &mut self,
        current_tick: u64,
        passer_pos: (f32, f32),
        ball_pos: (f32, f32),
        receiver_pos: (f32, f32),
    ) -> (PassResult, Option<(f32, f32)>) {
        // Contact Frame 감지를 위해 이전 phase 저장
        let prev_phase = std::mem::discriminant(&self.phase);

        let result = match self.phase {
            PassPhase::Windup { remaining_ticks } => self.tick_windup(remaining_ticks, passer_pos),
            PassPhase::Kick => self.tick_kick(current_tick, ball_pos),
            PassPhase::InFlight { arrival_tick } => self.tick_in_flight(current_tick, arrival_tick),
            PassPhase::Arrival => self.tick_arrival(ball_pos, receiver_pos),
            PassPhase::Finished => (PassResult::Continue, None),
        };

        // Contact Frame: Windup → Kick 전환 시 Viewer 이벤트 생성
        let curr_phase = std::mem::discriminant(&self.phase);
        if prev_phase != curr_phase && matches!(self.phase, PassPhase::Kick) {
            let event = self.emit_kick_event(
                current_tick,
                passer_pos,
                self.passer_idx as u32,
                Some(self.receiver_idx as u32),
            );
            self.pending_viewer_event = Some(event);
        }

        result
    }

    /// Windup Phase - 패스 준비
    fn tick_windup(
        &mut self,
        remaining: u8,
        passer_pos: (f32, f32),
    ) -> (PassResult, Option<(f32, f32)>) {
        // 패스 방향으로 몸 돌리기
        let dir = normalize((self.target_pos.0 - passer_pos.0, self.target_pos.1 - passer_pos.1));

        let new_remaining = remaining.saturating_sub(1);

        if new_remaining == 0 {
            self.phase = PassPhase::Kick;
        } else {
            self.phase = PassPhase::Windup { remaining_ticks: new_remaining };
        }

        (PassResult::Continue, Some(dir))
    }

    /// Kick Phase - 실제 킥 동작
    fn tick_kick(
        &mut self,
        current_tick: u64,
        ball_pos: (f32, f32),
    ) -> (PassResult, Option<(f32, f32)>) {
        // 비행 시간 계산
        let distance = distance_calc(ball_pos, self.target_pos);
        let flight_ticks = if self.pass_speed > 0.0 {
            (distance / self.pass_speed * TICKS_PER_SECOND as f32).ceil() as u64
        } else {
            1
        };

        let arrival_tick = current_tick + flight_ticks.max(1);
        self.arrival_tick = arrival_tick;

        // InFlight Phase로 전환
        self.phase = PassPhase::InFlight { arrival_tick };

        let result = PassResult::Started {
            passer_idx: self.passer_idx,
            receiver_idx: self.receiver_idx,
            arrival_tick,
            ball_speed: self.pass_speed,
        };

        (result, None)
    }

    /// InFlight Phase - 공 비행 중
    fn tick_in_flight(
        &mut self,
        current_tick: u64,
        arrival_tick: u64,
    ) -> (PassResult, Option<(f32, f32)>) {
        // 도착 틱 확인
        if current_tick >= arrival_tick {
            self.phase = PassPhase::Arrival;
        }

        (PassResult::Continue, None)
    }

    /// Arrival Phase - 트랩 판정
    fn tick_arrival(
        &mut self,
        ball_pos: (f32, f32),
        receiver_pos: (f32, f32),
    ) -> (PassResult, Option<(f32, f32)>) {
        let dist = distance_calc(receiver_pos, ball_pos);

        self.phase = PassPhase::Finished;

        // 트랩 범위 내인지
        if dist < TRAP_SUCCESS_RANGE {
            (PassResult::TrapSuccess { receiver_idx: self.receiver_idx }, None)
        } else if dist < TRAP_SUCCESS_RANGE * 2.0 {
            // 트랩 실패 (공이 가까이 왔지만 못 잡음)
            (
                PassResult::TrapFailed { receiver_idx: self.receiver_idx, ball_position: ball_pos },
                None,
            )
        } else {
            // 리시버가 범위 밖 → 미스패스
            (
                PassResult::Missed {
                    passer_idx: self.passer_idx,
                    intended_receiver_idx: self.receiver_idx,
                },
                None,
            )
        }
    }

    /// 인터셉트 체크 (외부에서 호출)
    pub fn check_intercept_possible(&self, ball_pos: (f32, f32), opponent_pos: (f32, f32)) -> bool {
        if !matches!(self.phase, PassPhase::InFlight { .. }) {
            return false;
        }

        let dist = distance_calc(opponent_pos, ball_pos);
        dist < INTERCEPT_RANGE
    }

    /// 인터셉트 처리
    pub fn handle_intercept(&mut self, interceptor_idx: usize) -> PassResult {
        self.phase = PassPhase::Finished;
        PassResult::InterceptSuccess { interceptor_idx }
    }

    // ========================================================================
    // Viewer Event Generation (P7 Section 14)
    // ========================================================================

    /// Kick Phase 시점에 호출하여 BallTrajectoryIntent 생성
    ///
    /// # Arguments
    /// * `now_tick` - 현재 틱
    /// * `passer_pos` - 패서 위치 (2D)
    /// * `passer_track_id` - 패서 트랙 ID
    /// * `target_track_id` - 리시버 트랙 ID (있는 경우)
    ///
    /// # Returns
    /// BallTrajectoryIntent for Viewer
    pub fn emit_kick_event(
        &self,
        now_tick: u64,
        passer_pos: (f32, f32),
        passer_track_id: u32,
        target_track_id: Option<u32>,
    ) -> BallTrajectoryIntent {
        let t_ms = BallTrajectoryIntent::tick_to_ms(now_tick);

        // pass_type → intent.kind + height_class 매핑
        let (kind, height_class) = self.pass_type_to_intent_kind();

        // curve_factor 변환: 내부 ±0.35 → Viewer용 ±1.0
        let normalized_curve = self.normalize_curve_factor();
        let curve = CurveDirection::from_factor(normalized_curve);
        let curve_amount = normalized_curve.abs();

        // speed_class 결정 (pass_speed vs v_ref 비교)
        let v_ref = kind.v_ref();
        let speed_class = self.determine_speed_class(v_ref);

        // travel_ms 계산
        let travel_ms = BallTrajectoryIntent::calculate_travel_ms(
            passer_pos,
            self.target_pos,
            kind,
            speed_class,
        );

        // contact_offset_ms 계산 (스프라이트 FPS 기반)
        let contact_offset_ms = BallTrajectoryIntent::contact_offset_for_kind(kind);

        BallTrajectoryIntent {
            t_ms,
            contact_offset_ms,
            actor_track_id: passer_track_id,
            target_track_id,
            kind,
            from: passer_pos,
            to: self.target_pos,
            travel_ms,
            speed_class,
            height_class,
            curve,
            curve_amount,
            outcome: "pending", // 초기 상태
            actor_pos: Some(passer_pos),
        }
    }

    /// lock_ms 계산 (Viewer 애니메이션 락 시간)
    ///
    /// lock_ms = (windup_ticks + kick_ticks + recovery_ticks) * 250
    pub fn calculate_lock_ms(&self) -> u32 {
        let windup = self.pass_type.windup_ticks() as u32;
        let kick = 1u32; // Kick Phase는 항상 1 tick
        let recovery = self.pass_type.recovery_ticks() as u32;
        (windup + kick + recovery) * 250
    }

    /// pass_type을 BallIntentKind + HeightClass로 변환
    fn pass_type_to_intent_kind(&self) -> (BallIntentKind, HeightClass) {
        match self.pass_type {
            PassType::Ground => (BallIntentKind::Pass, HeightClass::Ground),
            PassType::BackPass => (BallIntentKind::Pass, HeightClass::Ground),
            PassType::Lofted => (BallIntentKind::Pass, HeightClass::High),
            PassType::ThroughBall => (BallIntentKind::Through, HeightClass::Ground),
            PassType::Cross => (BallIntentKind::Cross, HeightClass::Low),
        }
    }

    /// curve_factor를 Viewer 표준 스케일로 정규화
    ///
    /// 내부: -0.35 ~ +0.35
    /// Viewer: -1.0 ~ +1.0
    fn normalize_curve_factor(&self) -> f32 {
        (self.curve_factor / 0.35).clamp(-1.0, 1.0)
    }

    /// 실제 속도와 v_ref 비교하여 SpeedClass 결정
    fn determine_speed_class(&self, v_ref: f32) -> SpeedClass {
        let ratio = self.pass_speed / v_ref;
        if ratio < 0.85 {
            SpeedClass::Slow
        } else if ratio > 1.15 {
            SpeedClass::Fast
        } else {
            SpeedClass::Normal
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 두 점 사이 거리
#[inline]
fn distance_calc(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

/// 벡터 정규화
#[inline]
fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len > 0.001 {
        (v.0 / len, v.1 / len)
    } else {
        (1.0, 0.0)
    }
}

/// 패스 난이도 계산
pub fn calculate_pass_difficulty(
    passer_pos: (f32, f32),
    target_pos: (f32, f32),
    defender_positions: &[(f32, f32)],
    pass_type: PassType,
) -> f32 {
    let distance = distance_calc(passer_pos, target_pos);

    // 거리 난이도 (0~1)
    let distance_factor = (distance / 50.0).clamp(0.0, 1.0);

    // 수비 압박 난이도
    let mut pressure = 0.0;
    for def_pos in defender_positions {
        let dist_to_line = point_to_line_distance(*def_pos, passer_pos, target_pos);
        if dist_to_line < 3.0 {
            pressure += (3.0 - dist_to_line) / 3.0;
        }
    }
    let pressure_factor = (pressure / 2.0).clamp(0.0, 1.0);

    // 패스 타입 난이도
    let type_factor = match pass_type {
        PassType::BackPass => 0.1,
        PassType::Ground => 0.3,
        PassType::Lofted => 0.5,
        PassType::ThroughBall => 0.7,
        PassType::Cross => 0.6,
    };

    // 종합 난이도 (0~1)
    (distance_factor * 0.3 + pressure_factor * 0.4 + type_factor * 0.3).clamp(0.0, 1.0)
}

/// 점과 선 사이의 거리
fn point_to_line_distance(point: (f32, f32), line_start: (f32, f32), line_end: (f32, f32)) -> f32 {
    let dx = line_end.0 - line_start.0;
    let dy = line_end.1 - line_start.1;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 0.001 {
        return distance_calc(point, line_start);
    }

    let t = ((point.0 - line_start.0) * dx + (point.1 - line_start.1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let closest = (line_start.0 + t * dx, line_start.1 + t * dy);

    distance_calc(point, closest)
}

/// 패스 시도 가능 여부
pub fn can_start_pass(has_ball: bool, can_start_action: bool) -> bool {
    has_ball && can_start_action
}

// ============================================================================
// ActionModel Impl (공통 trait)
// ============================================================================

/// Zero-sized model type for trait impl
pub struct PassModel;

impl ActionModel for PassModel {
    type Intent = PassIntent;
    type Technique = PassTechnique;
    type PhysicsParams = PassPhysicsParams;
    type Context = PassContext;
    type Skills = PassSkills;

    #[inline]
    fn available_techniques(intent: Self::Intent) -> &'static [Self::Technique] {
        intent.available_techniques()
    }

    #[inline]
    fn physics_params(technique: Self::Technique) -> Self::PhysicsParams {
        technique.physics_params()
    }

    fn choose_technique<R: Rng + ?Sized>(
        intent: Self::Intent,
        context: Self::Context,
        skills: Self::Skills,
        pressure: f32,
        rng: &mut R,
    ) -> Self::Technique {
        let pressure = clamp01(pressure);
        let d = clamp01(context.distance_norm);
        let open = clamp01(context.target_openness);
        let lane = clamp01(context.through_lane);
        let cross = clamp01(context.crossing_angle);

        let pass = skill01(skills.passing);
        let tech = skill01(skills.technique);
        let vis = skill01(skills.vision);
        let dec = skill01(skills.decisions);
        let comp = skill01(skills.composure);

        let candidates = intent.available_techniques();
        let mut weights: Vec<f32> = Vec::with_capacity(candidates.len());

        for &t in candidates {
            let mut w = 1.0;

            let skill_fit = match t {
                PassTechnique::Ground => 0.55 * pass + 0.25 * dec + 0.20 * comp,
                PassTechnique::Driven => 0.55 * pass + 0.35 * tech + 0.10 * dec,
                PassTechnique::Lofted => 0.45 * pass + 0.35 * tech + 0.20 * vis,
                PassTechnique::Cross => 0.35 * pass + 0.45 * tech + 0.20 * vis,
                PassTechnique::Through => 0.35 * pass + 0.25 * tech + 0.40 * vis,
                PassTechnique::Clear => 0.20 * pass + 0.10 * vis + 0.70 * dec,
            };
            w *= common_lerp(0.6, 1.6, clamp01(skill_fit));

            w *= match t {
                PassTechnique::Ground => common_lerp(1.3, 0.8, d),
                PassTechnique::Driven => common_lerp(0.9, 1.3, d),
                PassTechnique::Lofted => {
                    common_lerp(0.8, 1.2, d) * common_lerp(0.9, 1.1, 1.0 - open)
                }
                PassTechnique::Cross => common_lerp(0.7, 1.4, cross) * common_lerp(0.8, 1.2, d),
                PassTechnique::Through => common_lerp(0.6, 1.5, lane) * common_lerp(0.9, 1.1, d),
                PassTechnique::Clear => {
                    common_lerp(1.0, 1.4, pressure) * common_lerp(1.0, 1.2, 1.0 - open)
                }
            };

            let complexity = match t {
                PassTechnique::Ground => 0.15,
                PassTechnique::Driven => 0.30,
                PassTechnique::Lofted => 0.45,
                PassTechnique::Cross => 0.55,
                PassTechnique::Through => 0.60,
                PassTechnique::Clear => 0.10,
            };

            let pressure_resist = 0.45 * comp + 0.35 * dec + 0.20 * tech;
            let eff_pressure = (pressure * (1.0 + complexity)).max(0.0);
            w *= common_lerp(1.0, 0.45, clamp01(eff_pressure * (1.0 - pressure_resist)));

            weights.push(w.max(0.001));
        }

        let idx = weighted_choice_index(&weights, rng);
        candidates[idx]
    }

    fn base_success_prob(technique: Self::Technique, skills: Self::Skills, pressure: f32) -> f32 {
        let pressure = clamp01(pressure);

        let pass = skill01(skills.passing);
        let tech = skill01(skills.technique);
        let vis = skill01(skills.vision);
        let dec = skill01(skills.decisions);
        let comp = skill01(skills.composure);

        let difficulty = match technique {
            PassTechnique::Ground => 0.25,
            PassTechnique::Driven => 0.35,
            PassTechnique::Lofted => 0.45,
            PassTechnique::Cross => 0.55,
            PassTechnique::Through => 0.60,
            PassTechnique::Clear => 0.20,
        };

        let skill = match technique {
            PassTechnique::Ground => 0.55 * pass + 0.20 * dec + 0.25 * comp,
            PassTechnique::Driven => 0.50 * pass + 0.35 * tech + 0.15 * dec,
            PassTechnique::Lofted => 0.45 * pass + 0.35 * tech + 0.20 * vis,
            PassTechnique::Cross => 0.35 * pass + 0.45 * tech + 0.20 * vis,
            PassTechnique::Through => 0.35 * pass + 0.25 * tech + 0.40 * vis,
            PassTechnique::Clear => 0.35 * dec + 0.25 * comp + 0.40 * pass,
        };

        let press_resist = 0.45 * comp + 0.35 * dec + 0.20 * tech;

        let mut p = 0.55 + 0.60 * (skill - difficulty);
        p -= pressure * common_lerp(0.22, 0.08, press_resist);
        p.clamp(0.02, 0.98)
    }

    fn execution_error(
        technique: Self::Technique,
        skills: Self::Skills,
        pressure: f32,
    ) -> (f32, f32, f32) {
        let pressure = clamp01(pressure);

        let pass = skill01(skills.passing);
        let tech = skill01(skills.technique);
        let dec = skill01(skills.decisions);
        let comp = skill01(skills.composure);

        let control = 0.45 * pass + 0.30 * tech + 0.25 * comp;
        let calm = 0.60 * comp + 0.40 * dec;

        let (base_dir, base_speed, base_h) = match technique {
            PassTechnique::Ground => (0.06, 0.05, 0.00),
            PassTechnique::Driven => (0.08, 0.07, 0.00),
            PassTechnique::Lofted => (0.10, 0.08, 0.10),
            PassTechnique::Cross => (0.12, 0.08, 0.14),
            PassTechnique::Through => (0.13, 0.09, 0.06),
            PassTechnique::Clear => (0.09, 0.10, 0.16),
        };

        let skill_factor = common_lerp(1.35, 0.55, clamp01(control));
        let press_factor = 1.0 + pressure * common_lerp(0.90, 0.35, clamp01(calm));

        (
            base_dir * skill_factor * press_factor,
            base_speed * skill_factor * press_factor,
            base_h * skill_factor * press_factor,
        )
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;
    const CY: f32 = field::CENTER_Y;

    #[test]
    fn test_pass_action_creation() {
        let action = PassAction::new(1, 5, 0, 8, PassType::Ground, (60.0, CY), 100);

        assert_eq!(action.id, 1);
        assert_eq!(action.passer_idx, 5);
        assert_eq!(action.receiver_idx, 8);
        assert!(matches!(action.phase, PassPhase::Windup { .. }));
    }

    #[test]
    fn test_pass_windup_to_kick() {
        let mut action = PassAction::new(1, 5, 0, 8, PassType::Ground, (60.0, CY), 0);

        let passer_pos = (50.0, CY);
        let ball_pos = (50.0, CY);
        let receiver_pos = (60.0, CY);

        // Windup ticks (Ground = 1)
        action.update_tick(0, passer_pos, ball_pos, receiver_pos);
        assert!(matches!(action.phase, PassPhase::Kick));
    }

    #[test]
    fn test_pass_kick_starts_flight() {
        let mut action = PassAction::new(1, 5, 0, 8, PassType::Ground, (60.0, CY), 0);

        // Skip to Kick
        action.phase = PassPhase::Kick;

        let passer_pos = (50.0, CY);
        let ball_pos = (50.0, CY);
        let receiver_pos = (60.0, CY);

        let (result, _) = action.update_tick(0, passer_pos, ball_pos, receiver_pos);

        assert!(matches!(result, PassResult::Started { .. }));
        assert!(matches!(action.phase, PassPhase::InFlight { .. }));
    }

    #[test]
    fn test_pass_arrival_trap_success() {
        let mut action = PassAction::new(1, 5, 0, 8, PassType::Ground, (60.0, CY), 0);

        // Skip to Arrival
        action.phase = PassPhase::Arrival;

        let passer_pos = (50.0, CY);
        let ball_pos = (60.0, CY);
        let receiver_pos = (60.5, CY); // 리시버가 공 근처

        let (result, _) = action.update_tick(10, passer_pos, ball_pos, receiver_pos);

        assert!(matches!(result, PassResult::TrapSuccess { .. }));
        assert!(action.is_finished());
    }

    #[test]
    fn test_pass_arrival_missed() {
        let mut action = PassAction::new(1, 5, 0, 8, PassType::Ground, (60.0, CY), 0);

        // Skip to Arrival
        action.phase = PassPhase::Arrival;

        let passer_pos = (50.0, CY);
        let ball_pos = (60.0, CY);
        let receiver_pos = (70.0, CY); // 리시버가 너무 멀리

        let (result, _) = action.update_tick(10, passer_pos, ball_pos, receiver_pos);

        assert!(matches!(result, PassResult::Missed { .. }));
    }

    #[test]
    fn test_pass_type_speeds() {
        assert!(PassType::ThroughBall.base_speed() > PassType::BackPass.base_speed());
        // Lofted 패스는 높이 올라가서 느리게 도착, 하지만 base_speed는 더 빠름 (거리 감안)
        assert!(PassType::Lofted.base_speed() > PassType::Ground.base_speed());
    }

    #[test]
    fn test_check_intercept_possible() {
        let mut action = PassAction::new(1, 5, 0, 8, PassType::Ground, (60.0, CY), 0);
        action.phase = PassPhase::InFlight { arrival_tick: 10 };

        let ball_pos = (55.0, CY);
        let opponent_near = (55.5, CY);
        let opponent_far = (65.0, CY);

        assert!(action.check_intercept_possible(ball_pos, opponent_near));
        assert!(!action.check_intercept_possible(ball_pos, opponent_far));
    }

    #[test]
    fn test_pass_difficulty() {
        let passer_pos = (50.0, CY);
        let short_target = (55.0, CY);
        let long_target = (80.0, CY);

        let short_diff = calculate_pass_difficulty(passer_pos, short_target, &[], PassType::Ground);
        let long_diff = calculate_pass_difficulty(passer_pos, long_target, &[], PassType::Ground);

        assert!(long_diff > short_diff);
    }

    #[test]
    fn test_point_to_line_distance() {
        let line_start = (0.0, 0.0);
        let line_end = (10.0, 0.0);

        // 선 위의 점
        let on_line = point_to_line_distance((5.0, 0.0), line_start, line_end);
        assert!(on_line < 0.01);

        // 선에서 5m 떨어진 점
        let off_line = point_to_line_distance((5.0, 5.0), line_start, line_end);
        assert!((off_line - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_phase_names() {
        assert_eq!(PassPhase::Windup { remaining_ticks: 2 }.name(), "Windup");
        assert_eq!(PassPhase::Kick.name(), "Kick");
        assert_eq!(PassPhase::InFlight { arrival_tick: 10 }.name(), "InFlight");
        assert_eq!(PassPhase::Arrival.name(), "Arrival");
        assert_eq!(PassPhase::Finished.name(), "Finished");
    }

    #[test]
    fn test_with_curve() {
        let action = PassAction::new(1, 5, 0, 8, PassType::Cross, (60.0, CY), 0).with_curve(0.5);

        assert_eq!(action.curve_factor, 0.35); // Clamped
    }

    #[test]
    fn test_can_start_pass() {
        assert!(can_start_pass(true, true));
        assert!(!can_start_pass(false, true));
        assert!(!can_start_pass(true, false));
    }

    // ========================================================================
    // Viewer Event Tests (P7 Section 14)
    // ========================================================================

    #[test]
    fn test_emit_kick_event_ground_pass() {
        let action = PassAction::new(1, 5, 0, 8, PassType::Ground, (60.0, CY), 0);

        let event = action.emit_kick_event(100, (50.0, CY), 5, Some(8));      

        assert_eq!(event.t_ms, 25000); // 100 * 250ms
        assert_eq!(event.actor_track_id, 5);
        assert_eq!(event.target_track_id, Some(8));
        assert_eq!(event.kind, BallIntentKind::Pass);
        assert_eq!(event.height_class, HeightClass::Ground);
        assert_eq!(event.from, (50.0, CY));
        assert_eq!(event.to, (60.0, CY));
        assert_eq!(event.outcome, "pending");
    }

    #[test]
    fn test_emit_kick_event_through_ball() {
        let action = PassAction::new(1, 5, 0, 8, PassType::ThroughBall, (70.0, CY), 0);

        let event = action.emit_kick_event(50, (50.0, CY), 5, Some(8));       

        assert_eq!(event.kind, BallIntentKind::Through);
        assert_eq!(event.height_class, HeightClass::Ground);
    }

    #[test]
    fn test_emit_kick_event_cross() {
        let action = PassAction::new(1, 5, 0, 8, PassType::Cross, (80.0, CY), 0).with_curve(0.2);

        let event = action.emit_kick_event(50, (70.0, field::WIDTH_M), 5, Some(9));

        assert_eq!(event.kind, BallIntentKind::Cross);
        assert_eq!(event.height_class, HeightClass::Low);
        // curve_factor 0.2 / 0.35 ≈ 0.57, > 0.05 → Out
        assert_eq!(event.curve, CurveDirection::Out);
        assert!(event.curve_amount > 0.5);
    }

    #[test]
    fn test_calculate_lock_ms() {
        // Ground: windup=1, kick=1, recovery=1 → (1+1+1)*250 = 750ms
        let ground_action = PassAction::new(1, 5, 0, 8, PassType::Ground, (60.0, CY), 0);
        assert_eq!(ground_action.calculate_lock_ms(), 750);

        // Cross: windup=3, kick=1, recovery=2 → (3+1+2)*250 = 1500ms
        let cross_action = PassAction::new(2, 5, 0, 8, PassType::Cross, (80.0, CY), 0);
        assert_eq!(cross_action.calculate_lock_ms(), 1500);
    }

    #[test]
    fn test_curve_factor_normalization() {
        // Max positive curve
        let action = PassAction::new(1, 5, 0, 8, PassType::Cross, (60.0, CY), 0).with_curve(0.35);
        let event = action.emit_kick_event(0, (50.0, CY), 5, None);        
        assert!((event.curve_amount - 1.0).abs() < 0.01);
        assert_eq!(event.curve, CurveDirection::Out);

        // Max negative curve
        let action2 =
            PassAction::new(2, 5, 0, 8, PassType::Cross, (60.0, CY), 0).with_curve(-0.35);
        let event2 = action2.emit_kick_event(0, (50.0, CY), 5, None);      
        assert!((event2.curve_amount - 1.0).abs() < 0.01);
        assert_eq!(event2.curve, CurveDirection::In);

        // No curve
        let action3 = PassAction::new(3, 5, 0, 8, PassType::Ground, (60.0, CY), 0);
        let event3 = action3.emit_kick_event(0, (50.0, CY), 5, None);      
        assert_eq!(event3.curve, CurveDirection::None);
    }

    #[test]
    fn test_speed_class_determination() {
        // Slow pass (< 0.85 * v_ref)
        let mut action = PassAction::new(1, 5, 0, 8, PassType::Ground, (60.0, CY), 0);
        action.pass_speed = 9.0; // 9 / 12 = 0.75 < 0.85
        let event = action.emit_kick_event(0, (50.0, CY), 5, None);        
        assert_eq!(event.speed_class, SpeedClass::Slow);

        // Normal pass (0.85-1.15 * v_ref)
        action.pass_speed = 12.0; // 12 / 12 = 1.0
        let event = action.emit_kick_event(0, (50.0, CY), 5, None);        
        assert_eq!(event.speed_class, SpeedClass::Normal);

        // Fast pass (> 1.15 * v_ref)
        action.pass_speed = 15.0; // 15 / 12 = 1.25 > 1.15
        let event = action.emit_kick_event(0, (50.0, CY), 5, None);        
        assert_eq!(event.speed_class, SpeedClass::Fast);
    }
}
