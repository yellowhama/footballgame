//! Probability calculation utilities for match simulation
//!
//! All functions are pure - they take stats as input and return probabilities.
//! This allows easy unit testing without needing a full MatchEngine.

use super::coordinates;
use super::physics_constants::{self, home_advantage, skills, zones};
use crate::models::player::Position;

// ============================================================================
// Data Structures
// ============================================================================

/// 포지션별 스탯 묶음
#[derive(Debug, Clone, Default)]
pub struct PositionalStats {
    pub finishing: f32,
    pub composure: f32,
    pub long_shots: f32,
    pub technique: f32,
    pub passing: f32,
    pub vision: f32,
    pub marking: f32,
    pub positioning: f32,
    pub heading: f32,
    pub tackling: f32,
    pub pace: f32,
    pub anticipation: f32,
}

// ============================================================================
// Shooting Probability Functions
// ============================================================================

/// 슈팅 확률 계산 (순수 함수)
/// 거리와 각도 기반 기본 확률 반환
pub fn shooting_probability(player_pos: (f32, f32), is_home: bool) -> f32 {
    let goal_y = if is_home { 1.0 } else { 0.0 };
    let goal_pos = (0.5, goal_y);

    let dist_to_goal =
        ((player_pos.0 - goal_pos.0).powi(2) + (player_pos.1 - goal_pos.1).powi(2)).sqrt();

    let base_prob = if dist_to_goal < 0.2 {
        0.4
    } else if dist_to_goal < 0.35 {
        0.25
    } else if dist_to_goal < 0.5 {
        0.1
    } else {
        0.02
    };

    let angle_factor = 1.0 - (player_pos.0 - 0.5).abs();
    base_prob * angle_factor
}

/// 슈팅 파워 계산 (스킬 기반)
#[inline]
pub fn shot_power(
    long_shots: f32,
    strength: f32,
    technique: f32,
    distance_m: f32,
    variance: f32,
) -> f32 {
    let ls = skills::normalize(long_shots);
    let str_n = skills::normalize(strength);
    let tech = skills::normalize(technique);

    let base_power = physics_constants::shot::BASE_POWER_MPS;
    let max_additional = physics_constants::shot::MAX_ADDITIONAL_POWER_MPS;
    let distance_factor = (distance_m / zones::LONG_RANGE_M).min(1.0);
    let skill_power = ls * 0.4 + str_n * 0.4 + tech * 0.2;

    let power =
        base_power + max_additional * (distance_factor * 0.5 + skill_power * 0.5 + variance);
    power.clamp(physics_constants::shot::MIN_POWER_MPS, physics_constants::shot::MAX_POWER_MPS)
}

/// 슈팅 정확도 계산
#[inline]
pub fn shot_accuracy(
    finishing: f32,
    composure: f32,
    technique: f32,
    positional_modifier: f32,
    shot_power: f32,
    under_pressure: bool,
    variance: f32,
) -> f32 {
    let fin = skills::normalize(finishing);
    let comp = skills::normalize(composure);
    let tech = skills::normalize(technique);

    let base_skill = fin * 0.5 + comp * 0.3 + tech * 0.2;
    let skill_accuracy = base_skill * 0.7 + positional_modifier * 0.3;

    let power_penalty = ((shot_power - 20.0) / 12.0).clamp(0.0, 0.3);
    let pressure_penalty = if under_pressure { 0.15 * (1.0 - comp) } else { 0.0 };

    (skill_accuracy - power_penalty - pressure_penalty + variance).clamp(0.1, 0.95)
}

/// xG 계산 (거리 + 정확도 기반)
/// Goal Realism v5.1 (2026-01-07) - FIX_2601/0108
///
/// 목표: 평균 ~3.5골/경기, 범위 1-7골, 다양성 확보
/// - 0-0: 드묾 (~5%)
/// - 1-3골: 흔함 (~40%)
/// - 4-5골: 보통 (~35%)
/// - 6-7골: 드묾 (~15%)
/// - 8+골: 매우 드묾 (~5%)
#[inline]
pub fn xg_skill_based(distance_m: f32, accuracy: f32) -> f32 {
    // FIX_2601/0109: xG v8 - 벤치마크 목표 (2.7-3.0골/경기)
    // v7 대비 15% 감소 (iter3: 3.35골 → 목표 2.85)
    let base_xg = if distance_m < zones::VERY_CLOSE_M {
        0.78 // 6야드 박스 (< 10m) - 78% (was 90%)
    } else if distance_m < zones::CLOSE_M {
        0.43 // 페널티 박스 (< 16.5m) - 43% (was 50%)
    } else if distance_m < zones::MID_RANGE_M {
        0.17 // 박스 경계 (< 25m) - 17% (was 20%)
    } else if distance_m < zones::LONG_RANGE_M {
        0.085 // 미드레인지 (< 35m) - 8.5% (was 10%)
    } else {
        0.035 // 롱샷 (> 35m) - 3.5% (was 4%)
    };

    // 정확도 반영 - accuracy 영향 유지
    let xg = base_xg * (0.55 + accuracy * 0.45);
    xg.clamp(0.01, 0.92)
}

// ============================================================================
// Pass Probability Functions
// ============================================================================

/// 패스 성공률 계산 (순수 함수)
#[inline]
pub fn pass_success(
    distance_m: f32,
    passing: f32,
    vision: f32,
    technique: f32,
    positional_modifier: f32,
    pressure_penalty: f32,
    interception_risk: f32,
    is_home: bool,
) -> f32 {
    // 거리 기반 점수
    let distance_score = if distance_m < physics_constants::pass::VERY_SHORT_M {
        0.6 + 0.3 * (distance_m / physics_constants::pass::VERY_SHORT_M)
    } else if distance_m < physics_constants::pass::SHORT_M {
        0.9 - 0.1 * (1.0 - (distance_m - physics_constants::pass::VERY_SHORT_M) / 15.0)
    } else if distance_m < physics_constants::pass::OPTIMAL_MAX_M {
        1.0
    } else if distance_m < physics_constants::pass::LONG_M {
        0.9 - 0.4 * ((distance_m - physics_constants::pass::OPTIMAL_MAX_M) / 40.0)
    } else {
        0.3
    };

    // 스킬 팩터
    let pass = skills::normalize(passing);
    let vis = skills::normalize(vision);
    let tech = skills::normalize(technique);
    let base_skill = pass * 0.5 + vis * 0.3 + tech * 0.2;
    let skill_factor = base_skill * 0.7 + positional_modifier * 0.3;

    let base_success = distance_score * (0.6 + skill_factor * 0.4);
    let success_rate = base_success - pressure_penalty - interception_risk;

    let success_rate = if is_home {
        (success_rate + home_advantage::PASS_SUCCESS_BONUS).min(0.95)
    } else {
        success_rate
    };

    success_rate.clamp(0.15, 0.95)
}

/// 압박 패널티 계산 (Legacy O(n) implementation)
///
/// # Deprecated
/// Use `attribute_calc::pressure_penalty()` with `FieldBoard::local_pressure()` instead.
/// The new approach is O(1) and integrates with player attributes (composure, decisions).
///
/// Migration path:
/// ```ignore
/// // Old:
/// let penalty = probability::pressure_penalty(pos, &opponents);
///
/// // New:
/// let pressure_level = ctx.get_local_pressure_level(player_idx);
/// let penalty = attribute_calc::pressure_penalty(pressure_level, composure, decisions);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use FieldBoard + attribute_calc::pressure_penalty() for O(1) lookup. See FIX_2601/0106 D-4."
)]
pub fn pressure_penalty(player_pos: (f32, f32), opponent_positions: &[(f32, f32)]) -> f32 {
    let mut penalty: f32 = 0.0;

    for opp_pos in opponent_positions {
        let dist = coordinates::distance_between_m(player_pos, *opp_pos);

        if dist < physics_constants::pressure::VERY_CLOSE_M {
            penalty += 0.15;
        } else if dist < physics_constants::pressure::CLOSE_M {
            penalty += 0.05;
        }
    }

    penalty.min(physics_constants::pressure::MAX_PENALTY)
}

/// 인터셉트 위험도 계산
pub fn interception_risk(
    from_pos: (f32, f32),
    to_pos: (f32, f32),
    opponent_positions: &[(f32, f32)],
) -> f32 {
    let mut risk = 0.0;

    for opp_pos in opponent_positions {
        let to_pass_target = (to_pos.0 - from_pos.0, to_pos.1 - from_pos.1);
        let to_opponent = (opp_pos.0 - from_pos.0, opp_pos.1 - from_pos.1);

        let dot = to_pass_target.0 * to_opponent.0 + to_pass_target.1 * to_opponent.1;
        if dot > 0.0 {
            let dist_to_opp = coordinates::distance_between_m(from_pos, *opp_pos);
            let dist_to_target = coordinates::distance_between_m(from_pos, to_pos);

            if dist_to_opp < dist_to_target {
                risk += 0.1 * (1.0 - dist_to_opp / dist_to_target);
            }
        }
    }

    risk.min(0.25)
}

/// 공과의 거리 기반 참여 가중치
pub fn involvement_weight(player_pos: (f32, f32), ball_pos: (f32, f32)) -> f32 {
    let distance =
        ((player_pos.0 - ball_pos.0).powi(2) + (player_pos.1 - ball_pos.1).powi(2)).sqrt();

    if distance < 0.1 {
        1.0
    } else if distance < 0.2 {
        0.7
    } else if distance < 0.3 {
        0.4
    } else {
        0.1
    }
}

// ============================================================================
// Tackle/Dribble Probability Functions
// ============================================================================

/// 태클 성공률 계산
pub fn tackle_success(
    tackler_tackling: f32,
    tackler_aggression: f32,
    tackler_pace: f32,
    tackler_anticipation: f32,
    ball_holder_dribbling: f32,
    ball_holder_agility: f32,
    ball_holder_balance: f32,
    has_gold_sweeper: bool,
    has_gold_tank: bool,
) -> f32 {
    let tack = skills::normalize(tackler_tackling);
    let agg = skills::normalize(tackler_aggression);
    let pace = skills::normalize(tackler_pace);
    let ant = skills::normalize(tackler_anticipation);

    let drib = skills::normalize(ball_holder_dribbling);
    let agi = skills::normalize(ball_holder_agility);
    let bal = skills::normalize(ball_holder_balance);

    let tackle_score = tack * 0.4 + agg * 0.2 + pace * 0.2 + ant * 0.2;
    let evasion_score = drib * 0.4 + agi * 0.3 + bal * 0.3;

    let mut base_success = (tackle_score - evasion_score * 0.5 + 0.5).clamp(0.2, 0.8);

    if has_gold_sweeper {
        base_success = (base_success + 0.15).min(0.90);
    }
    if has_gold_tank {
        base_success *= 0.7;
    }

    base_success
}

/// 드리블 성공률 계산
pub fn dribble_success(
    dribbler_dribbling: f32,
    dribbler_agility: f32,
    dribbler_balance: f32,
    dribbler_pace: f32,
    dribbler_flair: f32,
    defender_tackling: f32,
    defender_pace: f32,
    defender_positioning: f32,
    has_gold_dribbler: bool,
) -> f32 {
    let drib = skills::normalize(dribbler_dribbling);
    let agi = skills::normalize(dribbler_agility);
    let bal = skills::normalize(dribbler_balance);
    let pace = skills::normalize(dribbler_pace);
    let flair = skills::normalize(dribbler_flair);

    let def_tack = skills::normalize(defender_tackling);
    let def_pace = skills::normalize(defender_pace);
    let def_pos = skills::normalize(defender_positioning);

    let attack_score = drib * 0.35 + agi * 0.2 + bal * 0.15 + pace * 0.15 + flair * 0.15;
    let defense_score = def_tack * 0.4 + def_pace * 0.3 + def_pos * 0.3;

    let mut base_success = (attack_score - defense_score * 0.4 + 0.4).clamp(0.15, 0.85);

    if has_gold_dribbler {
        base_success = (base_success + 0.20).min(0.95);
    }

    base_success
}

// ============================================================================
// Header/Aerial Probability Functions
// ============================================================================

/// 헤딩 성공률 계산 - match_sim.rs와 동일
pub fn header_success(
    heading: f32,
    jumping: f32,
    positioning: f32,
    strength: f32,
    contest: bool,
) -> f32 {
    let head = skills::normalize(heading);
    let jump = skills::normalize(jumping);
    let pos = skills::normalize(positioning);
    let str_n = skills::normalize(strength);

    let base_success = head * 0.4 + jump * 0.25 + pos * 0.2 + str_n * 0.15;

    if contest {
        // 경합 상황: 피지컬이 중요
        (base_success * 0.85).clamp(0.15, 0.80)
    } else {
        // 프리헤더: 기술이 더 중요
        (base_success * 1.1).clamp(0.30, 0.90)
    }
}

/// 공중전 강도 계산 - match_sim.rs와 동일
pub fn aerial_strength(heading: f32, jumping: f32, strength: f32, aggression: f32) -> f32 {
    let head = skills::normalize(heading);
    let jump = skills::normalize(jumping);
    let str_n = skills::normalize(strength);
    let agg = skills::normalize(aggression);

    head * 0.35 + jump * 0.30 + str_n * 0.25 + agg * 0.10
}

// ============================================================================
// GK Probability Functions
// ============================================================================

/// GK 세이브 확률 계산
///
/// FIX_2601/0109: 통합 함수 wrapper로 변경
/// 기존 시그니처 유지하면서 내부적으로 통합 함수 사용
pub fn gk_save_probability(
    reflexes: f32,
    positioning: f32,
    handling: f32,
    diving: f32,
    shot_height: f32,
    shot_power: f32,
    distance_m: f32,
) -> f32 {
    use crate::engine::match_sim::attribute_calc::calculate_gk_save_prob_unified;

    // shot_power를 m/s로 변환 (0-100 스케일 → 15-35 m/s)
    let shot_speed_mps = 15.0 + (shot_power / 100.0) * 20.0;

    calculate_gk_save_prob_unified(
        reflexes,
        positioning,
        handling,
        diving,
        distance_m,
        shot_speed_mps,
        shot_height,
        false, // is_one_on_one (이 컨텍스트에서는 알 수 없음)
    )
}

// ============================================================================
// Position-based Skill Rating Functions
// ============================================================================

/// 포지션별 스킬 레이팅 계산
pub fn positional_skill_rating(
    position: &Position,
    action_type: &str,
    stats: &PositionalStats,
) -> f32 {
    match action_type {
        "shooting" => positional_shooting_rating(position, stats),
        "passing" => positional_passing_rating(position, stats),
        "defending" => positional_defending_rating(position, stats),
        _ => 0.5,
    }
}

fn positional_shooting_rating(position: &Position, stats: &PositionalStats) -> f32 {
    let fin = skills::normalize(stats.finishing);
    let comp = skills::normalize(stats.composure);
    let ls = skills::normalize(stats.long_shots);
    let tech = skills::normalize(stats.technique);

    match position {
        Position::ST | Position::CF | Position::FW => fin * 0.6 + comp * 0.4,
        Position::CAM | Position::LW | Position::RW => fin * 0.5 + ls * 0.5,
        Position::CM | Position::CDM => ls * 0.7 + tech * 0.3,
        _ => fin,
    }
}

fn positional_passing_rating(position: &Position, stats: &PositionalStats) -> f32 {
    let pass = skills::normalize(stats.passing);
    let vis = skills::normalize(stats.vision);
    let tech = skills::normalize(stats.technique);

    match position {
        Position::CM | Position::CAM | Position::CDM => pass * 0.4 + vis * 0.4 + tech * 0.2,
        Position::CB | Position::DF => pass * 0.6 + vis * 0.4,
        _ => pass * 0.5 + vis * 0.5,
    }
}

fn positional_defending_rating(position: &Position, stats: &PositionalStats) -> f32 {
    let mark = skills::normalize(stats.marking);
    let pos = skills::normalize(stats.positioning);
    let head = skills::normalize(stats.heading);
    let tack = skills::normalize(stats.tackling);
    let pace = skills::normalize(stats.pace);
    let ant = skills::normalize(stats.anticipation);

    match position {
        Position::CB | Position::DF => mark * 0.4 + pos * 0.35 + head * 0.25,
        Position::LB | Position::RB | Position::LWB | Position::RWB => {
            tack * 0.4 + pace * 0.3 + pos * 0.3
        }
        Position::CDM => tack * 0.35 + ant * 0.35 + pos * 0.3,
        _ => tack,
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// 오프사이드 라인 계산 (x좌표 = 길이 방향)
/// - pos.0 (x): 0 = 홈 골라인, 1 = 어웨이 골라인
/// - 오프사이드 라인 = 수비팀의 두 번째 마지막 수비수 위치
/// - 홈팀 수비 시: x 오름차순 정렬 후 2번째 (골키퍼 다음으로 뒤에 있는 수비수)
/// - 어웨이팀 수비 시: x 내림차순 정렬 후 2번째
pub fn offside_line(defender_positions: &[(f32, f32)], defending_team_is_home: bool) -> f32 {
    let mut x_positions: Vec<f32> = defender_positions.iter().map(|p| p.0).collect();

    if defending_team_is_home {
        // 홈팀 수비: x=0이 자기 골, 오름차순 정렬 (골키퍼가 가장 작은 x)
        x_positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        // 어웨이팀 수비: x=1이 자기 골, 내림차순 정렬 (골키퍼가 가장 큰 x)
        x_positions.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    }

    // 2번째 수비수 위치 (index 1), 없으면 중앙
    x_positions.get(1).copied().unwrap_or(0.5)
}

/// 돌파 확률 계산
pub fn break_chance(
    attacker_pace: f32,
    attacker_acceleration: f32,
    attacker_off_the_ball: f32,
    defender_pace: f32,
    defender_positioning: f32,
    defender_anticipation: f32,
) -> f32 {
    let att_pace = skills::normalize(attacker_pace);
    let att_acc = skills::normalize(attacker_acceleration);
    let att_otb = skills::normalize(attacker_off_the_ball);
    let def_pace = skills::normalize(defender_pace);
    let def_pos = skills::normalize(defender_positioning);
    let def_ant = skills::normalize(defender_anticipation);

    let attack_score = att_pace * 0.4 + att_acc * 0.35 + att_otb * 0.25;
    let defense_score = def_pace * 0.35 + def_pos * 0.35 + def_ant * 0.30;

    ((attack_score - defense_score) * 0.5 + 0.3).clamp(0.05, 0.6)
}

/// 크로스 정확도 계산
/// match_sim.rs 표준: crossing * 0.5 + vision * 0.3 + technique * 0.2
pub fn cross_accuracy(crossing: f32, technique: f32, vision: f32, distance_m: f32) -> f32 {
    let cross = skills::normalize(crossing);
    let tech = skills::normalize(technique);
    let vis = skills::normalize(vision);

    let base = cross * 0.5 + vis * 0.3 + tech * 0.2;
    let distance_penalty = (distance_m / 40.0).clamp(0.0, 0.3);

    (base - distance_penalty).clamp(0.2, 0.85)
}

/// 볼 컨트롤 성공률 계산
pub fn ball_control_success(
    first_touch: f32,
    technique: f32,
    composure: f32,
    under_pressure: bool,
) -> f32 {
    let ft = skills::normalize(first_touch);
    let tech = skills::normalize(technique);
    let comp = skills::normalize(composure);

    let base = ft * 0.5 + tech * 0.3 + comp * 0.2;
    let pressure_penalty = if under_pressure { 0.15 } else { 0.0 };

    (base - pressure_penalty).clamp(0.3, 0.98)
}

/// 인터셉트 확률 계산 (패스 경로 기반)
pub fn intercept_chance(
    anticipation: f32,
    positioning: f32,
    pace: f32,
    distance_to_path: f32,
    max_intercept_range: f32,
) -> f32 {
    if distance_to_path > max_intercept_range {
        return 0.0;
    }

    let ant = skills::normalize(anticipation);
    let pos = skills::normalize(positioning);
    let spd = skills::normalize(pace);

    let distance_factor = 1.0 - (distance_to_path / max_intercept_range);
    let base_chance = ant * 0.4 + pos * 0.3 + spd * 0.3;

    (base_chance * distance_factor).clamp(0.0, 0.7)
}

// ============================================================================
// Pass Factor Functions (6-factor system)
// ============================================================================

/// 패스 거리 팩터
pub fn pass_distance_factor(distance_m: f32) -> f32 {
    if distance_m < 10.0 {
        1.0
    } else if distance_m < 20.0 {
        0.95 - (distance_m - 10.0) * 0.01
    } else if distance_m < 35.0 {
        0.85 - (distance_m - 20.0) * 0.015
    } else {
        (0.62 - (distance_m - 35.0) * 0.02).max(0.3)
    }
}

/// 패스 안전도 팩터
pub fn pass_safety_factor(
    to_pos: (f32, f32),
    opponent_positions: &[(f32, f32)],
    min_safe_distance_m: f32,
) -> f32 {
    let mut closest_opponent_dist = f32::MAX;

    for opp_pos in opponent_positions {
        let dist = coordinates::distance_between_m(to_pos, *opp_pos);
        if dist < closest_opponent_dist {
            closest_opponent_dist = dist;
        }
    }

    if closest_opponent_dist < min_safe_distance_m {
        0.3 + (closest_opponent_dist / min_safe_distance_m) * 0.4
    } else {
        0.7 + ((closest_opponent_dist - min_safe_distance_m) / 10.0).min(0.3)
    }
}

/// 패스 준비도 팩터 (수신자의 준비 상태)
pub fn pass_readiness_factor(off_the_ball: f32, anticipation: f32, is_facing_ball: bool) -> f32 {
    let otb = skills::normalize(off_the_ball);
    let ant = skills::normalize(anticipation);

    let base = otb * 0.6 + ant * 0.4;
    if is_facing_ball {
        (base + 0.1).min(1.0)
    } else {
        base * 0.8
    }
}

/// 패스 진행 팩터 (공격 방향으로의 진행도)
pub fn pass_progression_factor(from_pos: (f32, f32), to_pos: (f32, f32), is_home: bool) -> f32 {
    let progress = if is_home {
        to_pos.1 - from_pos.1 // 홈팀은 y 증가가 진행
    } else {
        from_pos.1 - to_pos.1 // 어웨이팀은 y 감소가 진행
    };

    if progress > 0.2 {
        1.0
    } else if progress > 0.0 {
        0.8 + progress
    } else if progress > -0.1 {
        0.7
    } else {
        0.5
    }
}

/// 패스 공간 팩터 (수신 위치의 공간)
pub fn pass_space_factor(to_pos: (f32, f32), teammate_positions: &[(f32, f32)]) -> f32 {
    let mut nearby_teammates = 0;

    for tm_pos in teammate_positions {
        let dist = ((to_pos.0 - tm_pos.0).powi(2) + (to_pos.1 - tm_pos.1).powi(2)).sqrt();
        if dist < 0.15 {
            nearby_teammates += 1;
        }
    }

    match nearby_teammates {
        0 => 1.0, // 고립 - 좋은 공간
        1 => 0.9,
        2 => 0.7,
        _ => 0.5, // 혼잡
    }
}

/// 패스 전술 팩터 (전술 지시 기반)
pub fn pass_tactical_factor(receiver_role: &str, team_style: &str) -> f32 {
    match (receiver_role, team_style) {
        ("forward", "attacking") => 1.1,
        ("midfielder", "possession") => 1.05,
        ("defender", "defensive") => 1.0,
        ("forward", "defensive") => 0.85,
        _ => 1.0,
    }
}

// ============================================================================
// Deception Action Functions (A13)
// ============================================================================

/// 공격자 기만 점수 계산
pub fn attacker_deception_score(
    flair: f32,
    technique: f32,
    dribbling: f32,
    agility: f32,
    balance: f32,
    composure: f32,
    action_type: &str,
) -> f32 {
    let fl = skills::normalize(flair);
    let tech = skills::normalize(technique);
    let drib = skills::normalize(dribbling);
    let agi = skills::normalize(agility);
    let bal = skills::normalize(balance);
    let comp = skills::normalize(composure);

    match action_type {
        "feint" => fl * 0.35 + tech * 0.25 + agi * 0.25 + bal * 0.15,
        "skill_move" => fl * 0.4 + drib * 0.3 + tech * 0.2 + agi * 0.1,
        "body_feint" => agi * 0.35 + bal * 0.30 + fl * 0.20 + comp * 0.15,
        _ => (fl + tech + drib) / 3.0,
    }
}

/// 수비자 반응 점수 계산
pub fn defender_reaction_score(
    anticipation: f32,
    concentration: f32,
    positioning: f32,
    decisions: f32,
    agility: f32,
    action_type: &str,
) -> f32 {
    let ant = skills::normalize(anticipation);
    let conc = skills::normalize(concentration);
    let pos = skills::normalize(positioning);
    let dec = skills::normalize(decisions);
    let agi = skills::normalize(agility);

    match action_type {
        "feint" => ant * 0.35 + conc * 0.30 + pos * 0.20 + dec * 0.15,
        "skill_move" => ant * 0.30 + agi * 0.30 + conc * 0.25 + pos * 0.15,
        "body_feint" => conc * 0.35 + ant * 0.30 + agi * 0.20 + dec * 0.15,
        _ => (ant + conc + pos) / 3.0,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shooting_probability_close_range() {
        let prob = shooting_probability((0.5, 0.9), true);
        assert!(prob > 0.3, "Close range should have high probability: {}", prob);
    }

    #[test]
    fn test_shooting_probability_long_range() {
        let prob = shooting_probability((0.5, 0.3), true);
        assert!(prob < 0.1, "Long range should have low probability: {}", prob);
    }

    #[test]
    fn test_shot_power_calculation() {
        let power = shot_power(15.0, 14.0, 13.0, 20.0, 0.0);
        assert!((15.0..=35.0).contains(&power), "Power should be in range: {}", power);
    }

    #[test]
    fn test_shot_accuracy_calculation() {
        let accuracy = shot_accuracy(16.0, 15.0, 14.0, 0.7, 25.0, false, 0.0);
        assert!((0.1..=0.95).contains(&accuracy), "Accuracy should be in range: {}", accuracy);
    }

    #[test]
    fn test_xg_penalty_area() {
        // FIX_2601/0109: xG v8 (benchmark target: 2.7-3.0 goals/match)
        // 10m = CLOSE_M zone (10.0 < 16.5), accuracy 0.8 → base 0.43 * (0.55 + 0.8*0.45) ≈ 0.3913
        let xg = xg_skill_based(10.0, 0.8);
        assert!(xg > 0.30, "Penalty area xG should be meaningful: {}", xg);
        assert!(xg < 0.45, "Penalty area xG should be realistic: {}", xg);
    }

    #[test]
    fn test_xg_long_range() {
        // FIX_2601/0108: Goal Realism v5.1
        // 40m = long shot (> 35m), accuracy 0.5 → base 0.015 * (0.55 + 0.5*0.45) ≈ 0.012
        let xg = xg_skill_based(40.0, 0.5);
        assert!(xg < 0.03, "Long range xG should be very low: {}", xg);
    }

    #[test]
    #[allow(deprecated)] // Testing legacy function
    fn test_pressure_penalty() {
        let opponents = vec![(0.5, 0.5), (0.52, 0.52)];
        let penalty = pressure_penalty((0.5, 0.5), &opponents);
        assert!(penalty > 0.1, "Close opponents should add pressure: {}", penalty);
    }

    #[test]
    fn test_interception_risk() {
        let opponents = vec![(0.55, 0.55)];
        let risk = interception_risk((0.5, 0.5), (0.6, 0.6), &opponents);
        assert!(risk > 0.0, "Opponent in path should add risk: {}", risk);
    }

    #[test]
    fn test_tackle_success_balanced() {
        let success = tackle_success(
            12.0, 10.0, 11.0, 12.0, // tackler (average stats)
            16.0, 17.0, 15.0, // ball holder (skilled dribbler)
            false, false,
        );
        assert!((0.2..=0.8).contains(&success), "Balanced tackle: {}", success);
    }

    #[test]
    fn test_tackle_success_gold_sweeper() {
        let without_gold = tackle_success(14.0, 12.0, 13.0, 14.0, 14.0, 15.0, 13.0, false, false);
        let with_gold = tackle_success(14.0, 12.0, 13.0, 14.0, 14.0, 15.0, 13.0, true, false);
        assert!(with_gold > without_gold, "Gold Sweeper should increase success");
    }

    #[test]
    fn test_dribble_success() {
        let success = dribble_success(
            16.0, 15.0, 14.0, 14.0, 15.0, // dribbler
            13.0, 12.0, 13.0, // defender
            false,
        );
        assert!((0.15..=0.85).contains(&success), "Dribble success: {}", success);
    }

    #[test]
    fn test_header_success_contest() {
        // header_success(heading, jumping, positioning, strength, contest)
        let free = header_success(15.0, 14.0, 12.0, 13.0, false);
        let contested = header_success(15.0, 14.0, 12.0, 13.0, true);
        assert!(free > contested, "Free header should be easier");
    }

    #[test]
    fn test_gk_save_probability() {
        let save = gk_save_probability(15.0, 14.0, 13.0, 14.0, 1.0, 25.0, 15.0);
        assert!((0.05..=0.85).contains(&save), "GK save prob: {}", save);
    }

    #[test]
    fn test_pass_distance_factor() {
        let short = pass_distance_factor(5.0);
        let long = pass_distance_factor(40.0);
        assert!(short > long, "Short pass should have better factor");
    }

    #[test]
    fn test_pass_progression_factor() {
        let forward = pass_progression_factor((0.5, 0.3), (0.5, 0.7), true);
        let backward = pass_progression_factor((0.5, 0.7), (0.5, 0.3), true);
        assert!(forward > backward, "Forward pass should have better factor");
    }

    #[test]
    fn test_break_chance() {
        let chance = break_chance(16.0, 15.0, 14.0, 12.0, 13.0, 12.0);
        assert!(chance > 0.3, "Fast attacker should have good break chance: {}", chance);
    }

    #[test]
    fn test_attacker_deception_score() {
        let score = attacker_deception_score(17.0, 15.0, 16.0, 15.0, 14.0, 13.0, "skill_move");
        assert!(score > 0.5, "Skilled player should have high deception: {}", score);
    }

    #[test]
    fn test_defender_reaction_score() {
        let score = defender_reaction_score(15.0, 16.0, 14.0, 14.0, 13.0, "feint");
        assert!(score > 0.4, "Alert defender should react well: {}", score);
    }
}
