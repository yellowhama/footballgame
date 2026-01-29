//! Attribute-Based Calculation Module
//!
//! 하드코딩된 확률/수치를 "Context × Attribute" 기반으로 계산.
//! 모든 EV 관련 확률 계산은 이 모듈을 통해 능력치와 연동됨.
//!
//! ## 핵심 원칙
//! 1. **하드코딩 금지**: 모든 확률/보너스는 능력치 기반
//! 2. **Context 반영**: 거리, 압박, 위치 등 상황이 능력치 효과에 영향
//! 3. **투명한 공식**: 각 계산의 의미가 명확해야 함
//!
//! ## 능력치 정규화 (FIX_2601/0106 P3-11)
//! 통합 API: `crate::engine::physics_constants::attribute`
//! - FM 스타일: 1-20 → `attribute::from_fm()`
//! - OFB 스타일: 0-100 → `attribute::from_100()` 또는 `attr01()`

use super::calculations::PressureLevel;
use crate::engine::weights::{StackRule, WeightComposer};
use crate::models::Position;

// FIX_2601/0106 P3-11: Re-export from unified module
pub use crate::engine::physics_constants::attribute;

// ============================================================================
// 능력치 정규화 헬퍼 (Backward Compatibility)
// ============================================================================

/// 0-100 능력치를 0.0-1.0으로 정규화
///
/// Equivalent to `attribute::from_100(value)`
///
/// FIX_2601/0106 P3-11: Delegates to unified attribute module
#[inline]
pub fn attr01(value: f32) -> f32 {
    attribute::from_100(value)
}

/// 두 능력치의 가중 평균 (a*w + b*(1-w))
#[inline]
pub fn weighted_avg(a: f32, b: f32, weight_a: f32) -> f32 {
    a * weight_a + b * (1.0 - weight_a)
}

/// 세 능력치의 가중 평균
///
/// # Panics (debug only)
/// `wa + wb > 1.0`이면 debug_assert 실패 (c의 가중치가 음수가 됨)
#[inline]
pub fn weighted_avg3(a: f32, b: f32, c: f32, wa: f32, wb: f32) -> f32 {
    debug_assert!(
        wa + wb <= 1.0 + f32::EPSILON,
        "weighted_avg3: weights must sum to <= 1.0, got wa={}, wb={}, sum={}",
        wa,
        wb,
        wa + wb
    );
    a * wa + b * wb + c * (1.0 - wa - wb)
}

// ============================================================================
// 패스 성공 확률 계산
// ============================================================================

/// 패스 거리 factor 계산
///
/// 능력치 기반: Vision + Passing이 높을수록 장거리 패스 정확도 유지
///
/// # Arguments
/// * `distance_m` - 패스 거리 (미터)
/// * `passing` - 패스 능력치 (0-100)
/// * `vision` - 시야 능력치 (0-100)
///
/// # Returns
/// 거리 factor (0.3 ~ 0.95)
pub fn pass_distance_factor(distance_m: f32, passing: f32, vision: f32) -> f32 {
    let p = attr01(passing);
    let v = attr01(vision);

    // 종합 패스 능력 (Passing 60%, Vision 40%)
    let pass_skill = p * 0.6 + v * 0.4;

    // 기본 거리 감쇠 커브
    // - 10m 이하: 거의 영향 없음
    // - 10-30m: 선형 감소
    // - 30m 이상: 급격히 감소
    let base_factor = if distance_m < 10.0 {
        0.95
    } else if distance_m < 30.0 {
        // 10m에서 0.95, 30m에서 ~0.55 (낮은 능력치 기준)
        let decay = (distance_m - 10.0) / 20.0; // 0.0 ~ 1.0
        0.95 - decay * 0.40
    } else {
        // 30m 이상: 급격히 감소
        let extra = (distance_m - 30.0) / 20.0; // 30m에서 0, 50m에서 1
        (0.55 - extra * 0.25).max(0.30)
    };

    // 능력치 보정: 높은 능력치일수록 거리 패널티 감소
    // skill=1.0이면 패널티의 50%만 적용, skill=0.0이면 100% 적용
    let penalty_reduction = pass_skill * 0.5;
    let penalty = 0.95 - base_factor;
    let adjusted_factor = 0.95 - penalty * (1.0 - penalty_reduction);

    adjusted_factor.clamp(0.30, 0.95)
}

// FIX_2601/0113: Vision-Based Pass Range System

/// Calculate maximum effective pass range based on vision and position
///
/// Vision determines how far a player can effectively see and execute passes.
/// Different positions have different range multipliers based on their typical roles.
///
/// # Arguments
/// * `vision` - Vision attribute (0-100)
/// * `position` - Player's position
///
/// # Returns
/// Maximum pass range in meters (15m - 75m depending on vision and position)
pub fn calculate_max_pass_range(vision: f32, position: &Position) -> f32 {
    let base_multiplier = match position {
        Position::GK => 1.0,                                            // GK: shorter range
        Position::CB | Position::LB | Position::RB | Position::DF => 1.2,  // Defenders: moderate
        Position::CM | Position::CDM | Position::CAM | Position::MF => 1.5, // Midfielders: longest
        Position::LM | Position::RM => 1.35,                            // Wide midfielders
        Position::ST | Position::CF | Position::LW | Position::RW | Position::FW => 1.2, // Forwards
        Position::LWB | Position::RWB => 1.3,                           // Wing-backs
    };
    // vision (0-100) → range (15m - 55m base, multiplied by position)
    15.0 + (vision / 100.0) * 40.0 * base_multiplier
}

/// Calculate non-linear distance difficulty factor (v2)
///
/// FIX_2601/0113: Exponential decay near range limits for more realistic
/// long pass difficulty. Replaces linear decay from v1.
///
/// # Arguments
/// * `distance_m` - Pass distance in meters
/// * `max_range` - Maximum effective range (from calculate_max_pass_range)
/// * `passing` - Passing attribute (0-100)
/// * `vision` - Vision attribute (0-100)
///
/// # Returns
/// Distance factor (0.25-0.98): 1.0 at optimal range, decreasing exponentially beyond
pub fn pass_distance_factor_v2(
    distance_m: f32,
    max_range: f32,
    passing: f32,
    vision: f32,
) -> f32 {
    let ratio = (distance_m / max_range).clamp(0.0, 1.5);
    let skill = attr01(passing) * 0.6 + attr01(vision) * 0.4;

    // Non-linear distance decay
    let base_factor = if ratio < 0.25 {
        // Very short: slight penalty for overly simple passes
        0.92 + ratio * 0.32 // 0.92-1.0
    } else if ratio < 0.50 {
        // Optimal range: no penalty
        1.00
    } else if ratio < 0.70 {
        // Medium distance: gradual decay
        1.00 - (ratio - 0.50) * 0.50 // 1.0-0.90
    } else if ratio < 0.90 {
        // Long distance: faster decay
        0.90 - (ratio - 0.70) * 1.00 // 0.90-0.70
    } else if ratio < 1.00 {
        // Near max range: exponential decay
        let over = ratio - 0.90;
        0.70 - over.powf(1.5) * 2.50 // 0.70-0.45
    } else {
        // Beyond max range: severe penalty
        0.30
    };

    // Skill adjustment: reduces penalty by up to 35%
    let penalty = 1.0 - base_factor;
    let penalty_reduction = skill.powf(1.2) * 0.35;
    let adjusted = 1.0 - penalty * (1.0 - penalty_reduction);

    adjusted.clamp(0.25, 0.98)
}

/// 압박 저항 계산
///
/// 능력치 기반: Composure + Decisions가 높을수록 압박 상황에서 안정적
///
/// # Arguments
/// * `pressure_level` - 압박 수준
/// * `composure` - 침착성 (0-100)
/// * `decisions` - 판단력 (0-100)
///
/// # Returns
/// 압박 페널티 (0.0 ~ 0.5)
pub fn pressure_penalty(pressure_level: PressureLevel, composure: f32, decisions: f32) -> f32 {
    let c = attr01(composure);
    let d = attr01(decisions);

    // 압박 저항력 (Composure 70%, Decisions 30%)
    let resistance = c * 0.7 + d * 0.3;

    // 기본 압박 페널티 (능력치 없을 때)
    let base_penalty = match pressure_level {
        PressureLevel::None => 0.0,
        PressureLevel::Light => 0.15,
        PressureLevel::Moderate => 0.30,
        PressureLevel::Heavy => 0.45,
        PressureLevel::Extreme => 0.60,
    };

    // 능력치가 높을수록 페널티 감소 (최대 60% 감소)
    let penalty = base_penalty * (1.0 - resistance * 0.6);

    penalty.clamp(0.0, 0.50)
}

/// 패스 능력 기반 성공률 계산 (Composer 기반)
///
/// # Arguments
/// * `passing` - 패스 능력치 (0-100)
/// * `technique` - 테크닉 (0-100)
///
/// # Returns
/// WeightComposer (Base: 0.6 ~ 0.95)
pub fn pass_skill_composer(passing: f32, technique: f32) -> WeightComposer {
    let p = attr01(passing);
    let t = attr01(technique);

    // 기본 성공률: 0.6 + (능력치 보너스)
    // 최고 능력치(1.0)일 때 0.95, 최저(0.0)일 때 0.60
    let skill = p * 0.7 + t * 0.3;
    let base = 0.60 + skill * 0.35;

    let mut composer = WeightComposer::new();
    composer.add("Base", base, StackRule::AddLn);
    composer
}

/// Legacy wrapper for pass_skill_factor
pub fn pass_skill_factor(passing: f32, technique: f32) -> f32 {
    pass_skill_composer(passing, technique).compose()
}

// ============================================================================
// 드리블 성공 확률 계산
// ============================================================================

/// 드리블 기본 성공률 계산
///
/// # Arguments
/// * `dribbling` - 드리블 (0-100)
/// * `agility` - 민첩성 (0-100)
/// * `balance` - 밸런스 (0-100)
///
/// # Returns
/// 기본 성공률 (0.35 ~ 0.80)
pub fn dribble_base_success(dribbling: f32, agility: f32, balance: f32) -> f32 {
    let d = attr01(dribbling);
    let a = attr01(agility);
    let b = attr01(balance);

    // 종합 드리블 능력 (Dribbling 50%, Agility 30%, Balance 20%)
    let skill = d * 0.5 + a * 0.3 + b * 0.2;

    // 기본 성공률: 0.35 + 능력치 보너스
    0.35 + skill * 0.45
}

/// 드리블 압박 저항
///
/// # Arguments
/// * `pressure_level` - 압박 수준
/// * `composure` - 침착성 (0-100)
/// * `strength` - 피지컬 강함 (0-100)
///
/// # Returns
/// 압박 페널티 (0.0 ~ 0.55)
pub fn dribble_pressure_penalty(
    pressure_level: PressureLevel,
    composure: f32,
    strength: f32,
) -> f32 {
    let c = attr01(composure);
    let s = attr01(strength);

    // 드리블 압박 저항 (Composure 60%, Strength 40%)
    let resistance = c * 0.6 + s * 0.4;

    // 기본 페널티 (드리블은 패스보다 압박 영향 큼)
    let base_penalty = match pressure_level {
        PressureLevel::None => 0.0,
        PressureLevel::Light => 0.12,
        PressureLevel::Moderate => 0.25,
        PressureLevel::Heavy => 0.40,
        PressureLevel::Extreme => 0.55,
    };

    // 능력치 보정 (최대 50% 감소)
    base_penalty * (1.0 - resistance * 0.5)
}

// ============================================================================
// 드리블 Zone-based Risk 계산 (FIX_2601/0106)
// ============================================================================

/// 드리블 존 위험도 페널티 계산
///
/// 수비 지역에서 드리블 실패 시 위험이 크므로 성공률 낮춤
/// open-football 참고: 수비에서 리스크 인지
///
/// # Arguments
/// * `normalized_x` - 자기 골대(0.0) ~ 상대 골대(1.0) 정규화 좌표
///
/// # Returns
/// 존 페널티 (0.0 ~ 0.25) - 수비 third에서 최대 25% 페널티
pub fn dribble_zone_penalty(normalized_x: f32) -> f32 {
    // normalized_x: 0.0 = 자기 골대, 1.0 = 상대 골대 (공격 방향 기준)
    // Defensive third: 0.0 ~ 0.33 (최대 페널티)
    // Middle third: 0.33 ~ 0.66 (약간 페널티)
    // Attacking third: 0.66 ~ 1.0 (페널티 없음)

    // FIX_2601/0106 P2-9: Clamp to valid range to prevent invalid penalties
    let normalized_x = normalized_x.clamp(0.0, 1.0);

    if normalized_x >= 0.66 {
        // Attacking third - 드리블 OK
        0.0
    } else if normalized_x >= 0.33 {
        // Middle third - 약간의 페널티
        let progress = (0.66 - normalized_x) / 0.33; // 0.0 ~ 1.0
        progress * 0.10 // 최대 10% 페널티
    } else {
        // Defensive third - 높은 페널티
        let depth = (0.33 - normalized_x) / 0.33; // 0.0 ~ 1.0
        0.10 + depth * 0.15 // 10% ~ 25% 페널티
    }
}

/// 드리블 vs 수비수 성공률 계산
///
/// open-football 스타일: success_chance = 0.5 + skill_diff * 0.3
/// 공격자와 수비자의 능력치 차이 기반
///
/// # Arguments
/// * `dribbler_score` - 드리블러 능력 점수 (0.0 ~ 1.0)
/// * `defender_score` - 수비수 능력 점수 (0.0 ~ 1.0)
/// * `distance_m` - 수비수와의 거리 (미터)
///
/// # Returns
/// 성공 확률 (0.20 ~ 0.85)
pub fn dribble_vs_defender_success(
    dribbler_score: f32,
    defender_score: f32,
    distance_m: f32,
) -> f32 {
    // open-football 스타일 공식
    let skill_diff = dribbler_score - defender_score;
    let base = 0.50 + skill_diff * 0.30;

    // 거리 보정: 수비수가 멀면 드리블 성공 확률 증가
    let distance_bonus = if distance_m > 5.0 {
        0.15 // 5m 이상이면 안전
    } else if distance_m > 3.0 {
        0.08 // 3-5m 중간
    } else if distance_m > 1.5 {
        0.0 // 1.5-3m 기본
    } else {
        -0.10 // 1.5m 미만 - 위험
    };

    (base + distance_bonus).clamp(0.20, 0.85)
}

/// 드리블 공격자 점수 계산
///
/// # Arguments
/// * `dribbling` - 드리블 (0-100)
/// * `agility` - 민첩성 (0-100)
/// * `balance` - 밸런스 (0-100)
/// * `pace` - 속도 (0-100)
///
/// # Returns
/// 공격 점수 (0.0 ~ 1.0)
pub fn dribble_attack_score(dribbling: f32, agility: f32, balance: f32, pace: f32) -> f32 {
    let d = attr01(dribbling);
    let a = attr01(agility);
    let b = attr01(balance);
    let p = attr01(pace);

    // Dribbling(45%) + Agility(25%) + Pace(15%) + Balance(15%)
    d * 0.45 + a * 0.25 + p * 0.15 + b * 0.15
}

/// 드리블 수비자 점수 계산
///
/// # Arguments
/// * `tackling` - 태클 (0-100)
/// * `positioning` - 포지셔닝 (0-100)
/// * `pace` - 속도 (0-100)
/// * `anticipation` - 예측력 (0-100)
///
/// # Returns
/// 수비 점수 (0.0 ~ 1.0)
pub fn dribble_defense_score(tackling: f32, positioning: f32, pace: f32, anticipation: f32) -> f32 {
    let t = attr01(tackling);
    let pos = attr01(positioning);
    let p = attr01(pace);
    let ant = attr01(anticipation);

    // Tackling(35%) + Positioning(25%) + Pace(20%) + Anticipation(20%)
    t * 0.35 + pos * 0.25 + p * 0.20 + ant * 0.20
}

// ============================================================================
// TakeOn(1:1 돌파) 성공 확률 계산
// ============================================================================

/// TakeOn 공격 점수 계산
///
/// # Arguments
/// * `dribbling` - 드리블 (0-100)
/// * `agility` - 민첩성 (0-100)
/// * `pace` - 속도 (0-100)
/// * `flair` - 기교 (0-100)
/// * `composure` - 침착성 (0-100)
///
/// # Returns
/// 공격 점수 (0.0 ~ 1.0)
pub fn takeon_attack_score(
    dribbling: f32,
    agility: f32,
    pace: f32,
    flair: f32,
    composure: f32,
) -> f32 {
    let d = attr01(dribbling);
    let a = attr01(agility);
    let p = attr01(pace);
    let f = attr01(flair);
    let c = attr01(composure);

    // 가중치: Dribbling(35%) + Agility(25%) + Flair(20%) + Pace(10%) + Composure(10%)
    d * 0.35 + a * 0.25 + f * 0.20 + p * 0.10 + c * 0.10
}

/// TakeOn 수비 점수 계산
///
/// # Arguments
/// * `tackling` - 태클 (0-100)
/// * `positioning` - 포지셔닝 (0-100)
/// * `anticipation` - 예측력 (0-100)
/// * `aggression` - 적극성 (0-100)
///
/// # Returns
/// 수비 점수 (0.0 ~ 1.0)
pub fn takeon_defense_score(
    tackling: f32,
    positioning: f32,
    anticipation: f32,
    aggression: f32,
) -> f32 {
    let t = attr01(tackling);
    let pos = attr01(positioning);
    let ant = attr01(anticipation);
    let agg = attr01(aggression);

    // 가중치: Tackling(40%) + Positioning(25%) + Anticipation(20%) + Aggression(15%)
    // Aggression은 양면: 높으면 커밋하기 쉬워 속임에 취약
    t * 0.40 + pos * 0.25 + ant * 0.20 + agg * 0.15
}

/// TakeOn 성공 확률 계산
///
/// # Arguments
/// * `attack_score` - 공격 점수 (0.0 ~ 1.0)
/// * `defense_score` - 수비 점수 (0.0 ~ 1.0)
/// * `defender_aggression` - 수비수 적극성 (0-100) - 높으면 속임에 취약
///
/// # Returns
/// 성공 확률 (0.15 ~ 0.70)
pub fn takeon_success_prob(attack_score: f32, defense_score: f32, defender_aggression: f32) -> f32 {
    let agg = attr01(defender_aggression);

    // 기본 성공률: 공격점수 vs 수비점수 차이 기반
    // 동등(차이=0)이면 0.45, 공격유리(+0.3)면 ~0.60, 수비유리(-0.3)면 ~0.30
    let diff = attack_score - defense_score;
    let base = 0.45 + diff * 0.50;

    // Aggression 보정: 적극적인 수비수는 속임에 취약
    // High aggression(1.0) → +0.08, Low(0.0) → 0
    let aggression_bonus = agg * 0.08;

    (base + aggression_bonus).clamp(0.15, 0.70)
}

// ============================================================================
// 슛 관련 계산
// ============================================================================

/// 슛 정확도 계산 (Composer 기반)
///
/// # Arguments
/// * `finishing` - 마무리 (0-100)
/// * `technique` - 테크닉 (0-100)
/// * `composure` - 침착성 (0-100)
/// * `pressure` - 압박 수치 (0.0 ~ 1.0)
///
/// # Returns
/// WeightComposer (Base: 0.10 ~ 0.95)
pub fn shot_accuracy_composer(
    finishing: f32,
    technique: f32,
    composure: f32,
    pressure: f32,
) -> WeightComposer {
    let f = attr01(finishing);
    let t = attr01(technique);
    let c = attr01(composure);

    // 기본 정확도: Finishing(60%) + Technique(25%) + Composure(15%)
    let base_accuracy = f * 0.60 + t * 0.25 + c * 0.15;

    let mut composer = WeightComposer::new();
    composer.add("Base", base_accuracy, StackRule::AddLn);

    // 압박 패널티: 압박이 높을수록 정확도 감소
    // Composure가 높으면 압박 영향 감소
    let pressure_effect = pressure * (1.0 - c * 0.5);
    let pressure_factor = 1.0 - pressure_effect * 0.40; // 최대 40% 감소

    composer.add("Pressure", pressure_factor, StackRule::AddLn);

    composer
}

/// Legacy wrapper for shot_accuracy
pub fn shot_accuracy(finishing: f32, technique: f32, composure: f32, pressure: f32) -> f32 {
    shot_accuracy_composer(finishing, technique, composure, pressure).compose().clamp(0.10, 0.95)
}

/// 장거리슛 보정
///
/// # Arguments
/// * `distance_m` - 슛 거리 (미터)
/// * `long_shots` - 장거리슛 (0-100)
/// * `shot_power` - 슛 파워 (0-100)
///
/// # Returns
/// 장거리 보정 factor (0.0 ~ 1.0, 1.0이면 패널티 없음)
pub fn long_shot_factor(distance_m: f32, long_shots: f32, shot_power: f32) -> f32 {
    if distance_m < 16.5 {
        // 페널티 박스 내: 패널티 없음
        return 1.0;
    }

    let ls = attr01(long_shots);
    let sp = attr01(shot_power);

    // 장거리슛 능력 (Long Shots 70%, Shot Power 30%)
    let long_skill = ls * 0.7 + sp * 0.3;

    // 거리 패널티: 16.5m 이후 급격히 증가
    let extra_dist = (distance_m - 16.5) / 20.0; // 16.5m에서 0, 36.5m에서 1
    let base_penalty = (extra_dist * 0.50).min(0.60); // 최대 60% 패널티

    // 능력치가 높으면 패널티 감소 (최대 70% 감소)
    let adjusted_penalty = base_penalty * (1.0 - long_skill * 0.70);

    (1.0 - adjusted_penalty).clamp(0.40, 1.0)
}

// ============================================================================
// 인터셉트 위험 계산
// ============================================================================

/// 패스 경로 상 인터셉트 위험 계산
///
/// # Arguments
/// * `base_risk` - 위치 기반 기본 위험 (0.0 ~ 0.5)
/// * `passer_vision` - 패서의 시야 (0-100)
/// * `pass_disguise` - 패스 숨기기 능력 (Technique으로 대체)
///
/// # Returns
/// 조정된 인터셉트 위험 (0.0 ~ 0.5)
pub fn intercept_risk_adjusted(base_risk: f32, vision: f32, technique: f32) -> f32 {
    let v = attr01(vision);
    let t = attr01(technique);

    // 높은 Vision/Technique은 인터셉트 위험 감소
    let disguise_skill = v * 0.6 + t * 0.4;

    // 최대 40% 위험 감소
    base_risk * (1.0 - disguise_skill * 0.4)
}

// ============================================================================
// 골키퍼 Urgency 계산 (open-football 참고, FIX_2601/0108)
// ============================================================================

/// 골키퍼 세이브 긴급도 계산 (open-football 스타일)
///
/// 공의 거리와 속도 기반으로 세이브 긴급도 결정.
/// 높은 urgency = 빠른 다이빙, 더 공격적인 세이브 시도
///
/// # Arguments
/// * `ball_distance_m` - 공과 골대 사이 거리 (미터)
/// * `ball_speed_toward_goal` - 골대 방향 공 속도 (m/s, 양수 = 골대로 향함)
///
/// # Returns
/// Urgency 값 (1.0 ~ 2.0)
/// - 1.0: 평상시, 여유 있는 상황
/// - 2.0: 최대 긴급, 즉시 반응 필요
pub fn gk_save_urgency(ball_distance_m: f32, ball_speed_toward_goal: f32) -> f32 {
    // open-football 공식:
    // urgency = (1.0 - distance/100) * (1.0 + velocity/10)
    // 하지만 우리는 Coord10 기준으로 조정 (100m -> 105m 필드)

    // 거리 factor: 가까울수록 높음 (0.0 ~ 1.0)
    let distance_factor = (1.0 - ball_distance_m / 50.0).clamp(0.0, 1.0);

    // 속도 factor: 빠를수록 높음 (1.0 ~ 2.0)
    let speed_factor = (1.0 + ball_speed_toward_goal.max(0.0) / 15.0).clamp(1.0, 2.0);

    // 최종 urgency: 거리 × 속도 조합
    let urgency = distance_factor * speed_factor;

    urgency.clamp(1.0, 2.0)
}

/// 골키퍼 다이빙 속도 계산
///
/// Urgency와 능력치 기반 다이빙 속도 결정
///
/// # Arguments
/// * `acceleration` - 가속력 (0-100)
/// * `agility` - 민첩성 (0-100)
/// * `urgency` - 세이브 긴급도 (1.0 ~ 2.0)
///
/// # Returns
/// 다이빙 속도 factor (0.5 ~ 2.0)
pub fn gk_dive_speed(acceleration: f32, agility: f32, urgency: f32) -> f32 {
    let acc = attr01(acceleration);
    let agi = attr01(agility);

    // open-football: (acceleration + agility) * 0.2 * urgency
    // 우리 스케일로 조정
    let base_speed = (acc + agi) * 0.5; // 0.0 ~ 1.0

    base_speed * urgency
}

/// 골키퍼 반응 시간 보너스 계산
///
/// 높은 urgency 상황에서 반사신경에 따른 추가 보너스
///
/// # Arguments
/// * `reflexes` - 반사신경 (0-100, anticipation 매핑)
/// * `composure` - 침착함 (0-100)
/// * `urgency` - 세이브 긴급도 (1.0 ~ 2.0)
///
/// # Returns
/// 반응 보너스 (0.0 ~ 0.25) - 세이브 확률에 추가
pub fn gk_reaction_bonus(reflexes: f32, composure: f32, urgency: f32) -> f32 {
    let ref_n = attr01(reflexes);
    let comp = attr01(composure);

    // 높은 urgency에서 reflexes가 더 중요
    // 낮은 urgency에서는 composure로 안정적 세이브
    let urgency_factor = (urgency - 1.0).clamp(0.0, 1.0); // 0.0 ~ 1.0

    let high_urgency_skill = ref_n;
    let low_urgency_skill = comp * 0.5 + ref_n * 0.5;

    let skill = urgency_factor * high_urgency_skill + (1.0 - urgency_factor) * low_urgency_skill;

    // 최대 25% 보너스
    skill * 0.25
}

/// 골키퍼 점프 도달 거리 계산
///
/// urgency에 따른 도달 가능 거리 (다이빙/점프)
///
/// # Arguments
/// * `jumping` - 점프력 (0-100)
/// * `height_cm` - 키 (cm)
/// * `urgency` - 세이브 긴급도 (1.0 ~ 2.0)
///
/// # Returns
/// 도달 거리 (미터) - 기본 포지션에서 손이 닿는 거리
pub fn gk_reach_distance(jumping: f32, height_cm: f32, urgency: f32) -> f32 {
    let jump = attr01(jumping);

    // 기본 도달 거리: 키의 절반 + 점프력 보너스
    let height_m = height_cm / 100.0;
    let base_reach = height_m * 0.5; // 기본 0.9m (180cm GK)

    // 점프력 추가 도달: 최대 0.8m
    let jump_bonus = jump * 0.8;

    // urgency가 높으면 풀 다이빙 (추가 0.3m)
    let dive_bonus = if urgency > 1.5 { 0.3 } else { 0.0 };

    base_reach + jump_bonus + dive_bonus
}

/// 골키퍼 예측 위치 계산을 위한 time-to-goal
///
/// 공이 골대에 도달하는 예상 시간 (초)
///
/// # Arguments
/// * `ball_distance_m` - 공과 골대 거리 (미터)
/// * `ball_speed_mps` - 공 속도 (m/s)
///
/// # Returns
/// 도달 시간 (초), 0.1초 최소
pub fn gk_time_to_goal(ball_distance_m: f32, ball_speed_mps: f32) -> f32 {
    if ball_speed_mps <= 0.1 {
        return 5.0; // 거의 정지 상태
    }

    (ball_distance_m / ball_speed_mps).clamp(0.1, 5.0)
}

/// 골키퍼 세이브 확률 urgency 보정
///
/// 높은 urgency에서 반사신경 위주, 낮은 urgency에서 포지셔닝 위주
///
/// # Arguments
/// * `base_save_prob` - 기본 세이브 확률
/// * `reflexes` - 반사신경 (0-100)
/// * `positioning` - 포지셔닝 (0-100)
/// * `urgency` - 세이브 긴급도 (1.0 ~ 2.0)
///
/// # Returns
/// urgency 조정된 세이브 확률
pub fn gk_save_prob_with_urgency(
    base_save_prob: f32,
    reflexes: f32,
    positioning: f32,
    urgency: f32,
) -> f32 {
    let ref_n = attr01(reflexes);
    let pos_n = attr01(positioning);

    // urgency가 높을수록 reflexes 가중치 증가
    let urgency_factor = ((urgency - 1.0) / 1.0).clamp(0.0, 1.0); // 0.0 ~ 1.0

    // 평상시: positioning 60%, reflexes 40%
    // 긴급시: positioning 30%, reflexes 70%
    let pos_weight = 0.6 - urgency_factor * 0.3; // 0.6 → 0.3
    let ref_weight = 1.0 - pos_weight; // 0.4 → 0.7

    let skill_factor = pos_n * pos_weight + ref_n * ref_weight;

    // 기본 확률에 스킬 보정 적용 (±20%)
    let adjustment = (skill_factor - 0.5) * 0.4; // -0.2 ~ +0.2

    (base_save_prob + adjustment).clamp(0.05, 0.95)
}

// ============================================================================
// GK Save Probability Unified (FIX_2601/0109)
// ============================================================================

/// 통합 GK 세이브 확률 계산 (Single Source of Truth)
///
/// 모든 GK 세이브 관련 로직을 하나로 통합:
/// - Urgency 기반 동적 가중치
/// - 높이별 세이브 난이도
/// - 거리/속도 보정
/// - 1v1 상황 페널티
///
/// # Arguments
/// * `reflexes` - 반사신경 (0-100)
/// * `positioning` - 포지셔닝 (0-100)
/// * `handling` - 핸들링 (0-100)
/// * `diving` - 다이빙 (0-100)
/// * `distance_m` - 슈터와 골대 사이 거리 (미터)
/// * `shot_speed_mps` - 슛 속도 (m/s)
/// * `shot_height_m` - 슛 높이 (미터, 0=그라운더)
/// * `is_one_on_one` - 1v1 상황 여부
///
/// # Returns
/// 최종 세이브 확률 (0.05 ~ 0.85)
pub fn calculate_gk_save_prob_unified(
    reflexes: f32,
    positioning: f32,
    handling: f32,
    diving: f32,
    distance_m: f32,
    shot_speed_mps: f32,
    shot_height_m: f32,
    is_one_on_one: bool,
) -> f32 {
    // 1. Urgency 계산 (거리 + 속도 기반)
    let urgency = gk_save_urgency(distance_m, shot_speed_mps);

    // 2. 동적 스킬 가중치 (urgency 기반)
    let urgency_factor = ((urgency - 1.0) / 1.0).clamp(0.0, 1.0);
    let ref_weight = 0.4 + urgency_factor * 0.3; // 0.4 → 0.7
    let pos_weight = 0.6 - urgency_factor * 0.3; // 0.6 → 0.3

    // 3. 정규화된 스킬 값
    let ref_n = attr01(reflexes);
    let pos_n = attr01(positioning);
    let dive_n = attr01(diving);
    let hand_n = attr01(handling);

    // 4. 기본 세이브 확률 (reflexes + positioning 가중치 적용)
    let base_save = ref_n * ref_weight + pos_n * pos_weight;

    // 5. 높이별 보정
    let height_factor = if shot_height_m < 0.5 {
        // Ground shot: diving 중요
        0.7 + dive_n * 0.3
    } else if shot_height_m < 1.5 {
        // Mid height: handling 중요
        0.8 + hand_n * 0.2
    } else if shot_height_m < 2.2 {
        // High shot: diving 중요
        0.6 + dive_n * 0.4
    } else {
        // Very high (near crossbar): 어려움
        0.4 + dive_n * 0.3
    };

    // 6. 파워 factor (빠른 슛은 막기 어려움)
    // FIX_2601/0106 D-1: Multiplicative factors (항상 양수 유지)
    // 0 m/s → 1.0, 35 m/s → 0.6 (linear interpolation)
    let power_factor = 1.0 - (shot_speed_mps / 35.0).clamp(0.0, 0.4);

    // 7. 거리 factor (먼 슛은 준비 시간 있음)
    // 0m → 1.0, 40m+ → 1.15
    let distance_factor = 1.0 + (distance_m / 40.0).clamp(0.0, 0.15);

    // 8. 1v1 factor (GK 불리)
    // 1v1 → 0.85, otherwise → 1.0
    let one_on_one_factor = if is_one_on_one { 0.85 } else { 1.0 };

    // 9. 최종 확률 계산 (multiplicative: 모든 factor가 양수이므로 음수 중간값 불가)
    let final_prob = base_save * height_factor * power_factor * distance_factor * one_on_one_factor;

    final_prob.clamp(0.05, 0.85)
}

// ============================================================================
// FM Meta Attribute Helpers (FIX_2601/0107)
// ============================================================================

/// Concentration 기반 실수 확률 modifier
///
/// FM에서 Concentration(+12 points)은 Tier 2 메타 속성.
/// 낮은 concentration = 실수 확률 증가
/// 피로도가 높을수록 concentration 영향력 증가
///
/// # Arguments
/// * `concentration` - 집중력 (0-100)
/// * `fatigue` - 피로도 (0.0~1.0, 1.0 = 완전 피로)
///
/// # Returns
/// 실수 확률 modifier (0.0~0.15)
/// - 높은 concentration + 낮은 fatigue → ~0.01 (실수 거의 없음)
/// - 낮은 concentration + 높은 fatigue → ~0.15 (실수 확률 15%)
pub fn concentration_error_modifier(concentration: f32, fatigue: f32) -> f32 {
    let c = attr01(concentration);
    let f = fatigue.clamp(0.0, 1.0);

    // 기본 실수 확률: 집중력이 낮을수록 증가
    let base_error = (1.0 - c) * 0.10; // 0% ~ 10%

    // 피로도 증폭: 피로할수록 집중력 영향 커짐
    let fatigue_multiplier = 1.0 + f * 0.5; // 1.0 ~ 1.5

    (base_error * fatigue_multiplier).clamp(0.0, 0.15)
}

/// Teamwork 기반 패스 수신 보너스
///
/// FM 실험: Teamwork=1이면 패스 수신 실패 다수
/// 높은 teamwork = 동료와 연계 좋음 = 수신 성공률 증가
///
/// # Arguments
/// * `teamwork` - 팀워크 (0-100)
///
/// # Returns
/// 수신 보너스 (0.0 ~ 0.10)
pub fn teamwork_reception_bonus(teamwork: f32) -> f32 {
    let t = attr01(teamwork);
    t * 0.10 // 최대 10% 보너스
}

/// Pace + Acceleration 복합 보너스
///
/// FM에서 Pace(+21), Acceleration(+20)이 최상위 메타.
/// 1:1 상황, 슈팅, 드리블 등에서 물리적 우위 제공
///
/// # Arguments
/// * `pace` - 속도 (0-100)
/// * `acceleration` - 가속력 (0-100)
/// * `situation` - "shooting" | "dribbling" | "tackling"
///
/// # Returns
/// 보너스 (0.0 ~ 0.15)
pub fn pace_acceleration_bonus(pace: f32, acceleration: f32, situation: &str) -> f32 {
    let p = attr01(pace);
    let a = attr01(acceleration);

    // 상황별 가중치
    let (pace_w, accel_w) = match situation {
        "shooting" => (0.6, 0.4),  // 슈팅: 좋은 각도 확보
        "dribbling" => (0.4, 0.6), // 드리블: 순간 가속 중요
        "tackling" => (0.5, 0.5),  // 태클: 둘 다 균형
        "movement" => (0.7, 0.3),  // 이동: 최고 속도 위주
        _ => (0.5, 0.5),
    };

    let physical = p * pace_w + a * accel_w;
    physical * 0.15 // 최대 15% 보너스
}

/// FM 메타 기반 슈팅 성공률 (enhanced version)
///
/// 기존: finishing(60%) + technique(25%) + composure(15%)
/// FM 메타: finishing(35%) + composure(25%) + technique(15%)
///         + pace(10%) + balance(8%) + concentration(7%)
///
/// # Arguments
/// * `finishing` - 마무리 (0-100)
/// * `technique` - 테크닉 (0-100)
/// * `composure` - 침착성 (0-100)
/// * `pace` - 속도 (0-100)
/// * `balance` - 밸런스 (0-100)
/// * `concentration` - 집중력 (0-100)
/// * `pressure` - 압박 수치 (0.0 ~ 1.0)
///
/// # Returns
/// 슈팅 정확도 (0.10 ~ 0.95)
pub fn shot_accuracy_fm_meta(
    finishing: f32,
    technique: f32,
    composure: f32,
    pace: f32,
    balance: f32,
    concentration: f32,
    pressure: f32,
) -> f32 {
    let f = attr01(finishing);
    let t = attr01(technique);
    let c = attr01(composure);
    let p = attr01(pace);
    let b = attr01(balance);
    let conc = attr01(concentration);

    // FM 메타 기반 가중치
    // finishing(35%) + composure(25%) + technique(15%) + pace(10%) + balance(8%) + concentration(7%)
    let base_accuracy = f * 0.35 + c * 0.25 + t * 0.15 + p * 0.10 + b * 0.08 + conc * 0.07;

    // 압박 패널티: 압박이 높을수록 정확도 감소
    // Composure가 높으면 압박 영향 감소
    let pressure_effect = pressure * (1.0 - c * 0.5);
    let pressure_factor = 1.0 - pressure_effect * 0.40; // 최대 40% 감소

    (base_accuracy * pressure_factor).clamp(0.10, 0.95)
}

/// FM 메타 기반 드리블 공격 점수 (enhanced version)
///
/// 기존: dribbling(45%) + agility(25%) + pace(15%) + balance(15%)
/// FM 메타: dribbling(30%) + agility(20%) + pace(15%) + acceleration(15%)
///         + balance(10%) + flair(5%) + anticipation(5%)
///
/// # Returns
/// 공격 점수 (0.0 ~ 1.0)
pub fn dribble_attack_score_fm_meta(
    dribbling: f32,
    agility: f32,
    pace: f32,
    acceleration: f32,
    balance: f32,
    flair: f32,
    anticipation: f32,
) -> f32 {
    let d = attr01(dribbling);
    let a = attr01(agility);
    let p = attr01(pace);
    let acc = attr01(acceleration);
    let b = attr01(balance);
    let fl = attr01(flair);
    let ant = attr01(anticipation);

    // FM 메타 기반 가중치
    // dribbling(30%) + agility(20%) + pace(15%) + acceleration(15%) + balance(10%) + flair(5%) + anticipation(5%)
    d * 0.30 + a * 0.20 + p * 0.15 + acc * 0.15 + b * 0.10 + fl * 0.05 + ant * 0.05
}

/// FM 메타 기반 패스 성공률 (enhanced version)
///
/// 기존: passing(70%) + technique(30%)
/// FM 메타: passing(40%) + vision(20%) + technique(15%)
///         + decisions(10%) + anticipation(8%) + concentration(7%)
///
/// # Returns
/// 패스 기본 성공률 (0.50 ~ 0.95)
pub fn pass_skill_fm_meta(
    passing: f32,
    vision: f32,
    technique: f32,
    decisions: f32,
    anticipation: f32,
    concentration: f32,
) -> f32 {
    let ps = attr01(passing);
    let v = attr01(vision);
    let t = attr01(technique);
    let d = attr01(decisions);
    let ant = attr01(anticipation);
    let conc = attr01(concentration);

    // FM 메타 기반 가중치
    // passing(40%) + vision(20%) + technique(15%) + decisions(10%) + anticipation(8%) + concentration(7%)
    let skill = ps * 0.40 + v * 0.20 + t * 0.15 + d * 0.10 + ant * 0.08 + conc * 0.07;

    // 기본 성공률: 0.50 + (능력치 보너스)
    0.50 + skill * 0.45
}

/// FM 메타 기반 태클 성공률 (FIX_2601/0107 v2)
///
/// open-football 참고: tackling(40%) + aggression(20%) + bravery(10%) + strength(20%) + agility(10%)
/// 하이브리드 방식: tackling(30%) + strength(15%) + aggression(15%) + anticipation(15%)
///                + pace(10%) + bravery(10%) + concentration(5%)
///
/// # Arguments
/// * `tackling` - 태클 기술 (0-100)
/// * `anticipation` - 예측력 (0-100)
/// * `pace` - 속도 (0-100)
/// * `strength` - 피지컬 (0-100)
/// * `aggression` - 공격성 (0-100) ← open-football
/// * `bravery` - 용감함 (0-100) ← open-football
/// * `concentration` - 집중력 (0-100)
///
/// # Returns
/// 태클 기본 성공률 (0.0 ~ 1.0)
pub fn tackle_skill_fm_meta(
    tackling: f32,
    anticipation: f32,
    pace: f32,
    strength: f32,
    aggression: f32,
    bravery: f32,
    concentration: f32,
) -> f32 {
    let tck = attr01(tackling);
    let ant = attr01(anticipation);
    let p = attr01(pace);
    let str = attr01(strength);
    let agg = attr01(aggression);
    let brv = attr01(bravery);
    let conc = attr01(concentration);

    // 하이브리드 가중치 (open-football 참고)
    // tackling(30%) + strength(15%) + aggression(15%) + anticipation(15%) + pace(10%) + bravery(10%) + concentration(5%)
    tck * 0.30 + str * 0.15 + agg * 0.15 + ant * 0.15 + p * 0.10 + brv * 0.10 + conc * 0.05
}

/// FM 메타 기반 헤딩 성공률 (FIX_2601/0107)
///
/// FM 메타 분석 결과:
/// - Heading(40%): 헤딩 기술 자체
/// - Jumping(25%): 공중볼 경합 능력
/// - Anticipation(15%): 공 궤적 예측
/// - Strength(10%): 피지컬 경합
/// - Balance(10%): 착지 후 안정성
pub fn header_skill_fm_meta(
    heading: f32,
    jumping: f32,
    anticipation: f32,
    strength: f32,
    balance: f32,
) -> f32 {
    let h = attr01(heading);
    let j = attr01(jumping);
    let a = attr01(anticipation);
    let s = attr01(strength);
    let b = attr01(balance);

    h * 0.40 + j * 0.25 + a * 0.15 + s * 0.10 + b * 0.10
}

/// FM 메타 기반 인터셉트 성공률 (FIX_2601/0107)
///
/// FM 메타 분석 결과:
/// - Anticipation(35%): 패스 경로 예측 (최중요)
/// - Positioning(25%): 인터셉트 위치 선점
/// - Pace(20%): 공으로의 접근 속도
/// - Concentration(10%): 집중력 유지
/// - Decisions(10%): 판단력 (언제 인터셉트할지)
pub fn intercept_skill_fm_meta(
    anticipation: f32,
    positioning: f32,
    pace: f32,
    concentration: f32,
    decisions: f32,
) -> f32 {
    let ant = attr01(anticipation);
    let pos = attr01(positioning);
    let p = attr01(pace);
    let conc = attr01(concentration);
    let dec = attr01(decisions);

    ant * 0.35 + pos * 0.25 + p * 0.20 + conc * 0.10 + dec * 0.10
}

/// 드리블 방어 점수 (방어자 관점) (FIX_2601/0107)
///
/// open-football 분석: 방어자 입장에서 드리블러 저지 능력
/// - Tackling(25%): 태클 기술
/// - Positioning(20%): 수비 위치 선정
/// - Anticipation(20%): 드리블러 움직임 예측
/// - Pace(15%): 추적 속도
/// - Strength(10%): 피지컬 경합
/// - Concentration(10%): 집중력 유지
///
/// # Returns
/// 방어 점수 (0.0 ~ 1.0)
pub fn dribble_defend_score_fm_meta(
    tackling: f32,
    positioning: f32,
    anticipation: f32,
    pace: f32,
    strength: f32,
    concentration: f32,
) -> f32 {
    let tck = attr01(tackling);
    let pos = attr01(positioning);
    let ant = attr01(anticipation);
    let p = attr01(pace);
    let str = attr01(strength);
    let conc = attr01(concentration);

    tck * 0.25 + pos * 0.20 + ant * 0.20 + p * 0.15 + str * 0.10 + conc * 0.10
}

/// GK 세이브 능력 (FIX_2601/0107)
///
/// open-football 참고: diving urgency + prediction 기반
/// - Reflexes(35%): 반사 신경 (슛 반응)
/// - Handling(25%): 공 처리 능력
/// - Positioning(25%): GK 위치 선정
/// - Diving(15%): 다이빙 범위/능력
///
/// # Returns
/// 세이브 능력 점수 (0.0 ~ 1.0)
pub fn gk_save_skill_fm_meta(
    reflexes: f32,
    handling: f32,
    positioning: f32,
    diving: f32,
) -> f32 {
    let ref_val = attr01(reflexes);
    let hnd = attr01(handling);
    let pos = attr01(positioning);
    let div = attr01(diving);

    ref_val * 0.35 + hnd * 0.25 + pos * 0.25 + div * 0.15
}

/// GK 돌진 판단 능력 (FIX_2601/0107)
///
/// open-football 참고: rushing 판단 및 실행
/// - Pace(30%): 돌진 속도
/// - Anticipation(25%): 상황 예측
/// - Bravery(25%): 돌진 용기
/// - Decisions(20%): 돌진 타이밍 판단
///
/// # Returns
/// 돌진 능력 점수 (0.0 ~ 1.0)
pub fn gk_rushing_skill_fm_meta(
    pace: f32,
    anticipation: f32,
    bravery: f32,
    decisions: f32,
) -> f32 {
    let p = attr01(pace);
    let ant = attr01(anticipation);
    let brv = attr01(bravery);
    let dec = attr01(decisions);

    p * 0.30 + ant * 0.25 + brv * 0.25 + dec * 0.20
}

// ============================================================================
// 테스트
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attr01() {
        assert_eq!(attr01(0.0), 0.0);
        assert_eq!(attr01(50.0), 0.5);
        assert_eq!(attr01(100.0), 1.0);
        assert_eq!(attr01(150.0), 1.0); // clamped
    }

    #[test]
    fn test_pass_distance_factor() {
        // 짧은 거리는 능력치 상관없이 높음
        assert!(pass_distance_factor(5.0, 50.0, 50.0) > 0.90);

        // 같은 거리에서 높은 능력치가 더 높은 factor
        let low_skill = pass_distance_factor(25.0, 40.0, 40.0);
        let high_skill = pass_distance_factor(25.0, 80.0, 80.0);
        assert!(high_skill > low_skill, "high: {}, low: {}", high_skill, low_skill);

        // 아주 먼 거리는 낮음
        assert!(pass_distance_factor(50.0, 50.0, 50.0) < 0.50);
    }

    #[test]
    fn test_pressure_penalty() {
        // 압박 없으면 페널티 없음
        assert_eq!(pressure_penalty(PressureLevel::None, 50.0, 50.0), 0.0);

        // 높은 Composure는 페널티 감소
        let low_composure = pressure_penalty(PressureLevel::Heavy, 30.0, 30.0);
        let high_composure = pressure_penalty(PressureLevel::Heavy, 90.0, 90.0);
        assert!(high_composure < low_composure);

        // Extreme 압박은 항상 어느 정도 페널티
        assert!(pressure_penalty(PressureLevel::Extreme, 100.0, 100.0) > 0.15);
    }

    #[test]
    fn test_dribble_base_success() {
        // 최저 능력치
        let low = dribble_base_success(20.0, 20.0, 20.0);
        assert!((0.35..=0.50).contains(&low));

        // 최고 능력치
        let high = dribble_base_success(90.0, 90.0, 90.0);
        assert!((0.70..=0.80).contains(&high));

        // 중간
        let mid = dribble_base_success(50.0, 50.0, 50.0);
        assert!(mid > low && mid < high);
    }

    #[test]
    fn test_takeon_success_prob() {
        // 공격 우세
        let attack_wins = takeon_success_prob(0.8, 0.4, 50.0);
        assert!(attack_wins > 0.55);

        // 수비 우세
        let defense_wins = takeon_success_prob(0.4, 0.8, 50.0);
        assert!(defense_wins < 0.35);

        // 동등
        let equal = takeon_success_prob(0.5, 0.5, 50.0);
        assert!(equal > 0.40 && equal < 0.55);

        // 적극적인 수비수 상대로 약간 유리
        let vs_aggressive = takeon_success_prob(0.5, 0.5, 90.0);
        let vs_passive = takeon_success_prob(0.5, 0.5, 20.0);
        assert!(vs_aggressive > vs_passive);
    }

    #[test]
    fn test_shot_accuracy() {
        // 압박 없을 때
        let no_pressure = shot_accuracy(70.0, 70.0, 70.0, 0.0);
        assert!(no_pressure > 0.60);

        // 압박 있을 때
        let with_pressure = shot_accuracy(70.0, 70.0, 70.0, 0.8);
        assert!(with_pressure < no_pressure);

        // 높은 Composure는 압박 영향 감소
        let high_composure = shot_accuracy(70.0, 70.0, 95.0, 0.8);
        let low_composure = shot_accuracy(70.0, 70.0, 30.0, 0.8);
        assert!(high_composure > low_composure);
    }

    #[test]
    fn test_long_shot_factor() {
        // 박스 내: 패널티 없음
        assert_eq!(long_shot_factor(10.0, 50.0, 50.0), 1.0);

        // 장거리: 패널티 있음
        let close = long_shot_factor(20.0, 50.0, 50.0);
        let far = long_shot_factor(35.0, 50.0, 50.0);
        assert!(close > far);

        // 높은 Long Shots는 패널티 감소
        let low_skill = long_shot_factor(30.0, 30.0, 30.0);
        let high_skill = long_shot_factor(30.0, 90.0, 90.0);
        assert!(high_skill > low_skill);
    }

    #[test]
    fn test_dribble_zone_penalty() {
        // Attacking third (0.66~1.0): 페널티 없음
        assert_eq!(dribble_zone_penalty(1.0), 0.0);
        assert_eq!(dribble_zone_penalty(0.8), 0.0);
        assert_eq!(dribble_zone_penalty(0.66), 0.0);

        // Middle third (0.33~0.66): 약간 페널티
        let mid_penalty = dribble_zone_penalty(0.5);
        assert!(mid_penalty > 0.0 && mid_penalty <= 0.10);

        // Defensive third (0.0~0.33): 높은 페널티
        let def_penalty = dribble_zone_penalty(0.15);
        assert!((0.10..=0.25).contains(&def_penalty));

        // 자기 골대 앞: 최대 페널티
        let max_penalty = dribble_zone_penalty(0.0);
        assert!(max_penalty >= 0.20);
    }

    #[test]
    fn test_dribble_vs_defender_success() {
        // 공격자 우세
        let attack_wins = dribble_vs_defender_success(0.8, 0.4, 3.0);
        assert!(attack_wins > 0.55);

        // 수비자 우세
        let defense_wins = dribble_vs_defender_success(0.4, 0.8, 3.0);
        assert!(defense_wins < 0.45);

        // 동등 능력, 거리 보정
        let close = dribble_vs_defender_success(0.5, 0.5, 1.0); // 가까움
        let far = dribble_vs_defender_success(0.5, 0.5, 6.0); // 멀음
        assert!(far > close);
    }

    #[test]
    fn test_dribble_attack_defense_score() {
        // 높은 능력치
        let high_attack = dribble_attack_score(90.0, 85.0, 80.0, 85.0);
        assert!(high_attack > 0.80);

        let low_attack = dribble_attack_score(40.0, 40.0, 40.0, 40.0);
        assert!(low_attack < 0.50);

        // 수비 점수
        let high_defense = dribble_defense_score(90.0, 85.0, 80.0, 85.0);
        assert!(high_defense > 0.80);

        let low_defense = dribble_defense_score(40.0, 40.0, 40.0, 40.0);
        assert!(low_defense < 0.50);
    }

    // ===== GK Urgency Tests (FIX_2601/0108) =====

    #[test]
    fn test_gk_save_urgency() {
        // 멀고 느린 공: 낮은 urgency
        let low_urgency = gk_save_urgency(40.0, 5.0);
        assert!((1.0..1.3).contains(&low_urgency));

        // 가깝고 빠른 공: 높은 urgency
        let high_urgency = gk_save_urgency(10.0, 25.0);
        assert!(high_urgency > 1.5);

        // 매우 가깝고 빠른 공: 최대 urgency
        let max_urgency = gk_save_urgency(5.0, 30.0);
        assert!(max_urgency >= 1.8);

        // 공이 멀리 있으면 urgency 낮음
        assert!(gk_save_urgency(50.0, 20.0) < gk_save_urgency(10.0, 20.0));
    }

    #[test]
    fn test_gk_dive_speed() {
        // 높은 능력치 + 높은 urgency
        let fast = gk_dive_speed(90.0, 85.0, 2.0);
        assert!(fast > 1.5);

        // 낮은 능력치 + 낮은 urgency
        let slow = gk_dive_speed(40.0, 40.0, 1.0);
        assert!(slow < 0.5);

        // urgency가 높으면 더 빠름
        let low_u = gk_dive_speed(70.0, 70.0, 1.0);
        let high_u = gk_dive_speed(70.0, 70.0, 2.0);
        assert!(high_u > low_u);
    }

    #[test]
    fn test_gk_reaction_bonus() {
        // 높은 reflexes + 높은 urgency
        let high_bonus = gk_reaction_bonus(90.0, 50.0, 2.0);
        assert!(high_bonus > 0.20);

        // 낮은 능력치
        let low_bonus = gk_reaction_bonus(30.0, 30.0, 1.5);
        assert!(low_bonus < 0.10);

        // 높은 urgency에서 reflexes가 더 중요
        let high_ref = gk_reaction_bonus(90.0, 30.0, 2.0);
        let high_comp = gk_reaction_bonus(30.0, 90.0, 2.0);
        assert!(high_ref > high_comp);
    }

    #[test]
    fn test_gk_reach_distance() {
        // 큰 키 + 높은 점프력 + 높은 urgency
        let far_reach = gk_reach_distance(90.0, 195.0, 2.0);
        assert!(far_reach > 1.5);

        // 작은 키 + 낮은 점프력
        let short_reach = gk_reach_distance(40.0, 175.0, 1.0);
        assert!(short_reach < 1.3);

        // urgency > 1.5 면 다이빙 보너스
        let no_dive = gk_reach_distance(70.0, 185.0, 1.3);
        let with_dive = gk_reach_distance(70.0, 185.0, 1.8);
        assert!(with_dive > no_dive);
    }

    #[test]
    fn test_gk_time_to_goal() {
        // 빠른 슛
        let fast = gk_time_to_goal(15.0, 30.0);
        assert!(fast < 1.0);

        // 느린 슛
        let slow = gk_time_to_goal(20.0, 10.0);
        assert!(slow > 1.5);

        // 정지된 공
        let stopped = gk_time_to_goal(10.0, 0.0);
        assert_eq!(stopped, 5.0);
    }

    #[test]
    fn test_gk_save_prob_with_urgency() {
        let base_prob = 0.5;

        // 높은 urgency: reflexes 중요
        let high_ref_high_u = gk_save_prob_with_urgency(base_prob, 90.0, 50.0, 2.0);
        let low_ref_high_u = gk_save_prob_with_urgency(base_prob, 50.0, 90.0, 2.0);
        assert!(high_ref_high_u > low_ref_high_u);

        // 낮은 urgency: positioning 중요
        let high_pos_low_u = gk_save_prob_with_urgency(base_prob, 50.0, 90.0, 1.0);
        let low_pos_low_u = gk_save_prob_with_urgency(base_prob, 90.0, 50.0, 1.0);
        assert!(high_pos_low_u > low_pos_low_u);

        // 확률 범위 체크
        assert!((0.05..=0.95).contains(&high_ref_high_u));
    }

    #[test]
    fn test_calculate_gk_save_prob_unified() {
        // 1. 기본 케이스: 평균 GK, 중거리 슛
        let avg = calculate_gk_save_prob_unified(
            50.0, 50.0, 50.0, 50.0,  // skills
            20.0,  // distance
            25.0,  // speed
            1.0,   // height
            false, // not 1v1
        );
        assert!(avg > 0.2 && avg < 0.6, "avg: {}", avg);

        // 2. 엘리트 GK vs 평균 GK
        let elite = calculate_gk_save_prob_unified(
            90.0, 85.0, 80.0, 88.0, // elite skills
            20.0, 25.0, 1.0, false,
        );
        assert!(elite > avg, "elite {} > avg {}", elite, avg);

        // 3. 근거리 강슛 (urgency 높음)
        let close_power = calculate_gk_save_prob_unified(
            80.0, 80.0, 80.0, 80.0, 8.0,  // close
            32.0, // fast
            0.3,  // ground
            false,
        );
        // 근거리 강슛도 좋은 GK는 50% 정도 막을 수 있음
        assert!(close_power > 0.3 && close_power < 0.7, "close_power: {}", close_power);

        // 4. 원거리 슛 (시간 있음)
        let long_range = calculate_gk_save_prob_unified(
            80.0, 80.0, 80.0, 80.0, 30.0, // far
            20.0, // slower
            1.2, false,
        );
        assert!(long_range > close_power, "long {} > close {}", long_range, close_power);

        // 5. 1v1 페널티
        let one_on_one = calculate_gk_save_prob_unified(
            80.0, 80.0, 80.0, 80.0, 15.0, 22.0, 0.8, true, // is_one_on_one
        );
        let normal = calculate_gk_save_prob_unified(80.0, 80.0, 80.0, 80.0, 15.0, 22.0, 0.8, false);
        assert!(one_on_one < normal, "1v1 {} < normal {}", one_on_one, normal);

        // 6. 범위 확인
        assert!((0.05..=0.85).contains(&close_power));
        assert!((0.05..=0.85).contains(&long_range));
    }

    // ===== FM Meta Attribute Tests (FIX_2601/0107) =====

    #[test]
    fn test_concentration_error_modifier() {
        // 높은 집중력 + 낮은 피로 → 낮은 실수 확률
        let focused = concentration_error_modifier(90.0, 0.1);
        assert!(focused < 0.02, "focused: {}", focused);

        // 낮은 집중력 + 높은 피로 → 높은 실수 확률
        let unfocused = concentration_error_modifier(30.0, 0.8);
        assert!(unfocused > 0.08, "unfocused: {}", unfocused);

        // 최저 집중력 + 최대 피로 → 최대 실수 확률
        let worst = concentration_error_modifier(0.0, 1.0);
        assert!((0.14..=0.15).contains(&worst), "worst: {}", worst);

        // 최고 집중력 → 피로와 상관없이 낮은 실수 확률
        let elite_tired = concentration_error_modifier(100.0, 1.0);
        assert!(elite_tired < 0.01, "elite_tired: {}", elite_tired);
    }

    #[test]
    fn test_teamwork_reception_bonus() {
        // 낮은 팀워크 → 낮은 보너스
        let selfish = teamwork_reception_bonus(20.0);
        assert!(selfish < 0.03, "selfish: {}", selfish);

        // 높은 팀워크 → 높은 보너스
        let team_player = teamwork_reception_bonus(90.0);
        assert!(team_player > 0.08, "team_player: {}", team_player);

        // 최대 팀워크 → 최대 보너스 (10%)
        let max_teamwork = teamwork_reception_bonus(100.0);
        assert!((0.09..=0.10).contains(&max_teamwork), "max: {}", max_teamwork);
    }

    #[test]
    fn test_pace_acceleration_bonus() {
        // 슈팅: pace 가중치 높음
        let shooting_fast = pace_acceleration_bonus(90.0, 70.0, "shooting");
        let shooting_accel = pace_acceleration_bonus(70.0, 90.0, "shooting");
        assert!(shooting_fast > shooting_accel, "shooting: pace {} > accel {}", shooting_fast, shooting_accel);

        // 드리블: acceleration 가중치 높음
        let dribble_fast = pace_acceleration_bonus(90.0, 70.0, "dribbling");
        let dribble_accel = pace_acceleration_bonus(70.0, 90.0, "dribbling");
        assert!(dribble_accel > dribble_fast, "dribbling: accel {} > pace {}", dribble_accel, dribble_fast);

        // 태클: 균형
        let tackle_fast = pace_acceleration_bonus(90.0, 70.0, "tackling");
        let tackle_accel = pace_acceleration_bonus(70.0, 90.0, "tackling");
        assert!((tackle_fast - tackle_accel).abs() < 0.01, "tackling: {} ~= {}", tackle_fast, tackle_accel);

        // 최대 보너스 확인 (15%)
        let max_bonus = pace_acceleration_bonus(100.0, 100.0, "shooting");
        assert!((0.14..=0.15).contains(&max_bonus), "max: {}", max_bonus);
    }

    #[test]
    fn test_shot_accuracy_fm_meta() {
        // 압박 없을 때 - 엘리트 스트라이커
        let elite_no_pressure = shot_accuracy_fm_meta(90.0, 85.0, 88.0, 80.0, 82.0, 85.0, 0.0);
        assert!(elite_no_pressure > 0.75, "elite: {}", elite_no_pressure);

        // 압박 있을 때
        let elite_with_pressure = shot_accuracy_fm_meta(90.0, 85.0, 88.0, 80.0, 82.0, 85.0, 0.8);
        assert!(elite_with_pressure < elite_no_pressure, "pressure effect");
        assert!(elite_with_pressure > 0.55, "elite under pressure: {}", elite_with_pressure);

        // 평균 스트라이커
        let avg = shot_accuracy_fm_meta(60.0, 55.0, 58.0, 60.0, 55.0, 55.0, 0.3);
        assert!(avg > 0.40 && avg < 0.65, "avg: {}", avg);

        // 범위 확인
        let min = shot_accuracy_fm_meta(20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 1.0);
        let max = shot_accuracy_fm_meta(99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 0.0);
        assert!(min >= 0.10, "min: {}", min);
        assert!(max <= 0.95, "max: {}", max);
    }

    #[test]
    fn test_dribble_attack_score_fm_meta() {
        // 엘리트 드리블러 (Messi/Neymar type)
        let elite = dribble_attack_score_fm_meta(95.0, 92.0, 85.0, 90.0, 88.0, 90.0, 85.0);
        assert!(elite > 0.85, "elite: {}", elite);

        // 평균 선수
        let avg = dribble_attack_score_fm_meta(55.0, 55.0, 55.0, 55.0, 55.0, 55.0, 55.0);
        assert!(avg > 0.45 && avg < 0.65, "avg: {}", avg);

        // 순수 스피드 선수 vs 기술 선수
        let speed = dribble_attack_score_fm_meta(60.0, 60.0, 90.0, 92.0, 55.0, 50.0, 50.0);
        let technical = dribble_attack_score_fm_meta(90.0, 85.0, 50.0, 50.0, 80.0, 90.0, 85.0);
        // FM 메타에서 속도형도 기술형과 경쟁 가능해야 함
        assert!((speed - technical).abs() < 0.15, "speed {} vs tech {}", speed, technical);
    }

    #[test]
    fn test_pass_skill_fm_meta() {
        // 엘리트 플레이메이커
        let elite = pass_skill_fm_meta(90.0, 88.0, 85.0, 87.0, 85.0, 82.0);
        assert!(elite > 0.85, "elite: {}", elite);

        // 수비수 (낮은 패스 능력)
        let defender = pass_skill_fm_meta(55.0, 50.0, 55.0, 55.0, 60.0, 60.0);
        assert!(defender > 0.65 && defender < 0.80, "defender: {}", defender);

        // 범위 확인
        let min = pass_skill_fm_meta(20.0, 20.0, 20.0, 20.0, 20.0, 20.0);
        let max = pass_skill_fm_meta(99.0, 99.0, 99.0, 99.0, 99.0, 99.0);
        assert!(min >= 0.50, "min: {}", min);
        assert!(max <= 0.95, "max: {}", max);
    }

    #[test]
    fn test_tackle_skill_fm_meta() {
        // 엘리트 수비수 (tackling, anticipation, pace, strength, aggression, bravery, concentration)
        let elite = tackle_skill_fm_meta(90.0, 88.0, 75.0, 85.0, 85.0, 80.0, 82.0);
        assert!(elite > 0.75, "elite: {}", elite);

        // 공격적인 수비수 (높은 aggression/bravery)
        let aggressive_def = tackle_skill_fm_meta(75.0, 70.0, 70.0, 80.0, 92.0, 88.0, 70.0);
        let calm_def = tackle_skill_fm_meta(75.0, 70.0, 70.0, 80.0, 50.0, 50.0, 70.0);
        // 공격적인 수비수가 더 높은 점수
        assert!(aggressive_def > calm_def, "aggressive {} vs calm {}", aggressive_def, calm_def);

        // 범위 확인
        let min = tackle_skill_fm_meta(20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0);
        let max = tackle_skill_fm_meta(99.0, 99.0, 99.0, 99.0, 99.0, 99.0, 99.0);
        assert!(min >= 0.15, "min: {}", min);
        assert!(max <= 1.0, "max: {}", max);
    }

    #[test]
    fn test_dribble_defend_score_fm_meta() {
        // 엘리트 수비수 (tackling, positioning, anticipation, pace, strength, concentration)
        let elite = dribble_defend_score_fm_meta(90.0, 88.0, 85.0, 75.0, 80.0, 82.0);
        assert!(elite > 0.75, "elite: {}", elite);

        // 범위 확인
        let min = dribble_defend_score_fm_meta(20.0, 20.0, 20.0, 20.0, 20.0, 20.0);
        let max = dribble_defend_score_fm_meta(99.0, 99.0, 99.0, 99.0, 99.0, 99.0);
        assert!(min >= 0.15, "min: {}", min);
        assert!(max <= 1.0, "max: {}", max);
    }

    #[test]
    fn test_gk_save_skill_fm_meta() {
        // 엘리트 GK (reflexes, handling, positioning, diving)
        let elite = gk_save_skill_fm_meta(90.0, 85.0, 88.0, 85.0);
        assert!(elite > 0.80, "elite: {}", elite);

        // 범위 확인
        let min = gk_save_skill_fm_meta(20.0, 20.0, 20.0, 20.0);
        let max = gk_save_skill_fm_meta(99.0, 99.0, 99.0, 99.0);
        assert!(min >= 0.15, "min: {}", min);
        assert!(max <= 1.0, "max: {}", max);
    }

    #[test]
    fn test_gk_rushing_skill_fm_meta() {
        // 용감한 GK (pace, anticipation, bravery, decisions)
        let brave = gk_rushing_skill_fm_meta(75.0, 80.0, 92.0, 78.0);
        let timid = gk_rushing_skill_fm_meta(75.0, 80.0, 40.0, 78.0);
        // 용감한 GK가 더 높은 돌진 점수
        assert!(brave > timid, "brave {} vs timid {}", brave, timid);

        // 범위 확인
        let min = gk_rushing_skill_fm_meta(20.0, 20.0, 20.0, 20.0);
        let max = gk_rushing_skill_fm_meta(99.0, 99.0, 99.0, 99.0);
        assert!(min >= 0.15, "min: {}", min);
        assert!(max <= 1.0, "max: {}", max);
    }
}
