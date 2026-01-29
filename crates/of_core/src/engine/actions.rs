//! Action execution logic and result structures
//!
//! This module provides pure functions for resolving player actions.
//! All randomness is passed in as parameters via Roll structs.

use super::physics_constants::skills;
use super::probability;

// ============================================================================
// Shot Action
// ============================================================================

/// 슈팅 액션 컨텍스트
#[derive(Debug, Clone)]
pub struct ShotContext {
    pub shooter_idx: usize,
    pub shooter_name: String,
    pub is_home: bool,
    pub player_pos: (f32, f32),
    pub distance_m: f32,

    // 슈터 스탯
    pub finishing: f32,
    pub composure: f32,
    pub technique: f32,
    pub long_shots: f32,
    pub strength: f32,

    // 컨텍스트
    pub positional_modifier: f32,
    pub under_pressure: bool,
    pub ball_height: f32,

    // Gold Traits
    pub has_cannon: bool,
    pub has_sniper: bool,
    pub has_lob_master: bool,
    pub has_acrobat: bool,

    // GK 정보
    pub gk_idx: usize,
    pub gk_reflexes: f32,
    pub gk_positioning: f32,
    pub gk_handling: f32,
    pub gk_diving: f32,
}

/// 슈팅에 필요한 랜덤 값들
#[derive(Debug, Clone)]
pub struct ShotRolls {
    pub power_variance: f32,
    pub accuracy_variance: f32,
    pub on_target_roll: f32,
    pub goal_roll: f32,
    pub save_roll: f32,
}

/// 슈팅 결과
#[derive(Debug, Clone)]
pub struct ShotResult {
    pub shot_power: f32,
    pub accuracy: f32,
    pub xg: f32,
    pub on_target: bool,
    pub is_goal: bool,
    pub is_save: bool,
    pub shot_position: (f32, f32, f32),
}

/// 슈팅 결과 계산 (순수 함수)
pub fn resolve_shot(ctx: &ShotContext, rolls: &ShotRolls) -> ShotResult {
    // 파워 계산
    let shot_power = probability::shot_power(
        ctx.long_shots,
        ctx.strength,
        ctx.technique,
        ctx.distance_m,
        rolls.power_variance,
    );

    // 정확도 계산
    let accuracy = probability::shot_accuracy(
        ctx.finishing,
        ctx.composure,
        ctx.technique,
        ctx.positional_modifier,
        shot_power,
        ctx.under_pressure,
        rolls.accuracy_variance,
    );

    // xG 계산
    let xg = probability::xg_skill_based(ctx.distance_m, accuracy);

    // FIX_2601/0114: 거리 기반 on_target 패널티
    // NOTE: 이 로직은 phase_action 경로에서만 사용됨
    // tick_based 엔진은 action_queue.rs의 OutcomeSet을 사용함
    let on_target_multiplier = if ctx.distance_m > 16.0 {
        let penalty = ((ctx.distance_m - 16.0) / 19.0).min(1.0) * 0.50;
        0.85 - penalty
    } else {
        0.95
    };
    let on_target = rolls.on_target_roll < accuracy * on_target_multiplier;

    // 골 확률 결정
    let mut goal_prob = xg;

    // Gold Trait 효과
    if ctx.distance_m >= 30.0 && ctx.has_cannon {
        goal_prob = 0.35;
    }
    if accuracy >= 0.85 && ctx.has_sniper {
        goal_prob = 0.50;
    }
    if ctx.has_lob_master && ctx.ball_height > 2.0 {
        goal_prob = (goal_prob * 1.5).min(0.7);
    }
    if ctx.has_acrobat && ctx.ball_height > 1.0 {
        goal_prob = (goal_prob * 1.3).min(0.7);
    }

    // GK 세이브 확률
    let shot_height = match ctx.ball_height {
        h if h < 0.5 => 0.5,
        h if h < 1.5 => 1.2,
        h if h < 2.2 => 2.0,
        _ => 2.5,
    };

    let save_prob = probability::gk_save_probability(
        ctx.gk_reflexes,
        ctx.gk_positioning,
        ctx.gk_handling,
        ctx.gk_diving,
        shot_height,
        shot_power,
        ctx.distance_m,
    );

    let is_goal = on_target && rolls.goal_roll < goal_prob;
    let is_save = on_target && !is_goal && rolls.save_roll < save_prob * 0.7;

    ShotResult {
        shot_power,
        accuracy,
        xg,
        on_target,
        is_goal,
        is_save,
        shot_position: (ctx.player_pos.0, ctx.player_pos.1, ctx.ball_height),
    }
}

// ============================================================================
// Tackle Action
// ============================================================================

/// 태클 액션 컨텍스트
/// match_sim.rs의 calculate_tackle_success 로직과 동기화됨 (2025-12-06)
#[derive(Debug, Clone)]
pub struct TackleContext {
    pub tackler_idx: usize,
    pub tackler_name: String,
    pub ball_holder_idx: usize,
    pub ball_holder_name: String,
    pub is_home: bool,

    // 태클러 스탯 (match_sim.rs와 동기화)
    pub tackling: f32,
    pub anticipation: f32,
    pub aggression: f32,
    pub positioning: f32,

    // 볼 홀더 스탯
    pub dribbling: f32,
    pub balance: f32,

    // PlayerInstructions 결과 (match_sim.rs에서 계산해서 전달)
    pub instruction_modifier: f32, // 0.8 ~ 1.1 범위

    // Gold Traits (match_sim.rs와 동기화)
    pub has_vacuum: bool, // Gold Vacuum: 태클 성공률 극대화
    pub has_wall: bool,   // Gold Wall: 블록 성공률 극대화
    pub has_reader: bool, // Gold Reader: 패스 경로 예측
    pub has_bully: bool,  // Gold Bully: 물리적 지배력

    // A13: PerfectTackle 스킬 (파울 0%)
    pub has_perfect_tackle: bool,
}

/// 태클에 필요한 랜덤 값들
#[derive(Debug, Clone)]
pub struct TackleRolls {
    pub success_roll: f32,
    pub foul_roll: f32,
    pub card_roll: f32,
    pub injury_roll: f32,
}

/// 태클 결과
#[derive(Debug, Clone)]
pub struct TackleResult {
    pub success: bool,
    pub foul: bool,
    pub yellow_card: bool,
    pub red_card: bool,
    pub injury: bool,
}

/// 태클 결과 계산 (순수 함수)
/// match_sim.rs의 calculate_tackle_success + execute_tackle_action 로직 통합
pub fn resolve_tackle(ctx: &TackleContext, rolls: &TackleRolls) -> TackleResult {
    // ============================================
    // 1. 기본 태클 성공률 계산 (match_sim.rs 로직)
    // ============================================
    let tackling = skills::normalize(ctx.tackling);
    let anticipation = skills::normalize(ctx.anticipation);
    let positioning = skills::normalize(ctx.positioning);
    let aggression = skills::normalize(ctx.aggression);

    let dribbling = skills::normalize(ctx.dribbling);
    let balance = skills::normalize(ctx.balance);

    let tackle_skill = tackling * 0.4 + anticipation * 0.3 + positioning * 0.2 + aggression * 0.1;
    let evade_skill = dribbling * 0.6 + balance * 0.4;

    // PlayerInstructions 보정 적용
    let mut success =
        ((tackle_skill - evade_skill * 0.5 + 0.5) * ctx.instruction_modifier).clamp(0.2, 0.9);

    // ============================================
    // 2. Gold Trait 효과 적용
    // ============================================

    // Gold Vacuum: 태클 성공률 극대화 (최소 85%)
    if ctx.has_vacuum {
        success = success.max(0.85);
    }

    // Gold Wall: 블록 성공률 극대화 (최소 80%)
    if ctx.has_wall {
        success = success.max(0.80);
    }

    // Gold Reader: 패스 경로 예측으로 태클 보너스 (+10%)
    if ctx.has_reader {
        success = (success + 0.10).min(0.90);
    }

    // Gold Bully: 물리적 지배력으로 태클 보너스 (+8%)
    if ctx.has_bully {
        success = (success + 0.08).min(0.92);
    }

    // ============================================
    // 3. 성공/실패 판정
    // ============================================
    let is_success = rolls.success_roll < success;

    // ============================================
    // 4. 파울 판정
    // FIX_2601/0109: 파울 확률 증가 (벤치마크: 경기당 22~28회)
    // ============================================
    let foul_chance = if ctx.has_perfect_tackle {
        // A13: PerfectTackle - 파울 확률 0%
        0.0
    } else if is_success {
        // 성공 시에도 파울 가능 (강한 태클, 늦은 태클 등)
        // FIX_2601/0109: 5% → 12% + aggression 보너스
        0.12 + aggression * 0.08
    } else {
        // 실패 시 파울 가능성 높음
        // FIX_2601/0109: 기본 20% + aggression 보너스
        0.20 + aggression * 0.25
    };
    let is_foul = rolls.foul_roll < foul_chance;

    // 카드 판정
    // FIX_2601/0109: 카드 확률도 약간 조정
    let yellow_card = is_foul && rolls.card_roll < 0.25;
    let red_card = is_foul && ctx.aggression > 15.0 && rolls.card_roll < 0.04;

    // 부상 판정
    let injury = is_foul && rolls.injury_roll < 0.02;

    TackleResult { success: is_success, foul: is_foul, yellow_card, red_card, injury }
}

// ============================================================================
// Dribble Action
// ============================================================================

/// 드리블 액션 컨텍스트
/// match_sim.rs의 calculate_dribble_success 로직과 동기화됨 (2025-12-06)
#[derive(Debug, Clone)]
pub struct DribbleContext {
    pub dribbler_idx: usize,
    pub dribbler_name: String,
    pub is_home: bool,
    pub current_pos: (f32, f32),
    pub target_pos: (f32, f32),

    // 드리블러 스탯 (match_sim.rs와 동기화)
    pub dribbling: f32,
    pub agility: f32,
    pub balance: f32,
    pub pace: f32,

    // 수비수 스탯 (match_sim.rs와 동기화: marking 사용)
    pub defender_idx: Option<usize>,
    pub defender_marking: Option<f32>,
    pub defender_positioning: Option<f32>,
    pub defender_pace: Option<f32>,

    // PlayerInstructions 결과 (match_sim.rs에서 계산해서 전달)
    pub offensive_modifier: f32, // 0.9 ~ 1.05 범위
    pub defensive_modifier: f32, // 0.9 ~ 1.05 범위

    // A13: SpeedDemon 스킬
    pub has_speed_demon: bool,

    // 드리블러 Gold Traits
    pub has_speedster: bool,  // Gold Speedster: 속도 경합 시 거의 확실한 돌파
    pub has_technician: bool, // Gold Technician: 1:1 돌파 시 수비수 거의 확실히 제침
    pub has_tank: bool,       // Gold Tank: 태클 당해도 버티는 확률 극대화

    // 수비수 Gold Traits
    pub defender_has_shadow: bool, // Gold Shadow: 드리블러에게 바짝 붙어 밀착
    pub defender_has_bully: bool,  // Gold Bully: 물리적 지배력으로 드리블러 압박
}

/// 드리블에 필요한 랜덤 값들
#[derive(Debug, Clone)]
pub struct DribbleRolls {
    pub success_roll: f32,
}

/// 드리블 결과
#[derive(Debug, Clone)]
pub struct DribbleResult {
    pub success: bool,
    pub tackled: bool,
    pub new_position: (f32, f32),
}

/// 드리블 성공 확률만 계산 (롤 없이)
/// ActionOptions 계산 등에서 확률만 필요할 때 사용
pub fn dribble_success_probability(ctx: &DribbleContext) -> f32 {
    // 수비수가 없으면 높은 성공률
    if ctx.defender_idx.is_none() {
        return 0.95;
    }

    let dribbling = skills::normalize(ctx.dribbling);
    let agility = skills::normalize(ctx.agility);
    let balance = skills::normalize(ctx.balance);
    let pace = skills::normalize(ctx.pace);

    let marking = skills::normalize(ctx.defender_marking.unwrap_or(10.0));
    let positioning = skills::normalize(ctx.defender_positioning.unwrap_or(10.0));
    let defender_pace = skills::normalize(ctx.defender_pace.unwrap_or(10.0));

    // A13: SpeedDemon
    if ctx.has_speed_demon {
        let pace_diff = pace - defender_pace;
        if pace_diff >= 0.25 {
            return 0.95;
        }
    }

    let dribble_skill = dribbling * 0.4 + agility * 0.25 + balance * 0.2 + pace * 0.15;
    let defend_skill = marking * 0.6 + positioning * 0.4;

    let off_mod = ctx.offensive_modifier;
    let def_mod = ctx.defensive_modifier;

    let mut success =
        (dribble_skill * off_mod - defend_skill * def_mod * 0.6 + 0.5).clamp(0.15, 0.85);

    // Gold Traits
    if ctx.has_speedster && pace > defender_pace {
        success = 0.90;
    }
    if ctx.has_technician {
        success = 0.85;
    }
    if ctx.has_tank {
        success = success.max(0.75);
    }
    if ctx.defender_has_shadow {
        success = success.min(0.50);
    }
    if ctx.defender_has_bully {
        success = (success - 0.15).max(0.15);
    }

    success
}

/// 드리블 결과 계산 (순수 함수)
/// match_sim.rs의 calculate_dribble_success 로직과 동기화됨 (2025-12-06)
pub fn resolve_dribble(ctx: &DribbleContext, rolls: &DribbleRolls) -> DribbleResult {
    // 수비수가 없으면 높은 성공률
    if ctx.defender_idx.is_none() {
        return DribbleResult { success: true, tackled: false, new_position: ctx.target_pos };
    }

    // ============================================
    // 1. 스탯 정규화
    // ============================================
    let dribbling = skills::normalize(ctx.dribbling);
    let agility = skills::normalize(ctx.agility);
    let balance = skills::normalize(ctx.balance);
    let pace = skills::normalize(ctx.pace);

    let marking = skills::normalize(ctx.defender_marking.unwrap_or(10.0));
    let positioning = skills::normalize(ctx.defender_positioning.unwrap_or(10.0));
    let defender_pace = skills::normalize(ctx.defender_pace.unwrap_or(10.0));

    // ============================================
    // 2. A13: SpeedDemon 스킬 - 속도 경합 시 거의 승리
    // ============================================
    if ctx.has_speed_demon {
        let pace_diff = pace - defender_pace;
        if pace_diff >= 0.25 {
            // 5/20 = 0.25 (normalized)
            return DribbleResult { success: true, tackled: false, new_position: ctx.target_pos };
        }
    }

    // ============================================
    // 3. 기본 스킬 계산
    // ============================================
    let dribble_skill = dribbling * 0.4 + agility * 0.25 + balance * 0.2 + pace * 0.15;
    let defend_skill = marking * 0.6 + positioning * 0.4;

    // PlayerInstructions 보정 적용
    let off_mod = ctx.offensive_modifier; // 0.9 ~ 1.05
    let def_mod = ctx.defensive_modifier; // 0.9 ~ 1.05

    let mut success =
        (dribble_skill * off_mod - defend_skill * def_mod * 0.6 + 0.5).clamp(0.15, 0.85);

    // ============================================
    // 4. Gold Trait 효과 적용
    // ============================================

    // Gold Speedster: 속도 경합 시 거의 확실한 돌파 (pace 차이 있을 때)
    if ctx.has_speedster && pace > defender_pace {
        success = 0.90; // trait_balance.speedster_success_rate
    }

    // Gold Technician: 1:1 돌파 시 수비수 거의 확실히 제침
    if ctx.has_technician {
        success = 0.85; // trait_balance.technician_success_rate
    }

    // Gold Tank: 태클 당해도 버티는 확률 극대화
    if ctx.has_tank {
        success = success.max(0.75); // trait_balance.tank_min_success_rate
    }

    // Gold Shadow (수비수): 드리블러에게 바짝 붙어 밀착
    if ctx.defender_has_shadow {
        success = success.min(0.50); // trait_balance.shadow_opponent_dribble_cap
    }

    // Gold Bully (수비수): 물리적 지배력으로 드리블러 압박
    if ctx.defender_has_bully {
        success = (success - 0.15).max(0.15); // trait_balance.bully_opponent_dribble_penalty
    }

    // ============================================
    // 5. 성공/실패 판정
    // ============================================
    let is_success = rolls.success_roll < success;

    let new_position = if is_success { ctx.target_pos } else { ctx.current_pos };

    DribbleResult {
        success: is_success,
        tackled: !is_success, // 수비수가 있는 상태에서 실패 = 태클당함
        new_position,
    }
}

// ============================================================================
// Aerial Duel
// ============================================================================

/// 공중전 컨텍스트
#[derive(Debug, Clone)]
pub struct AerialDuelContext {
    pub attacker_idx: usize,
    pub attacker_name: String,
    pub attacker_heading: f32,
    pub attacker_jumping: f32,
    pub attacker_strength: f32,
    pub attacker_bravery: f32,
    pub has_airraid: bool,
    pub has_bully: bool, // Gold Bully trait

    pub defenders: Vec<AerialDefender>,
}

/// 공중전 수비수 정보
#[derive(Debug, Clone)]
pub struct AerialDefender {
    pub idx: usize,
    pub name: String,
    pub heading: f32,
    pub jumping: f32,
    pub strength: f32,
    pub bravery: f32,
    pub has_airraid: bool, // Gold AirRaid trait
    pub has_bully: bool,   // Gold Bully trait
    pub position: (f32, f32), // FIX_2601/0116: Position for tie-breaking
}

/// 공중전 결과
#[derive(Debug, Clone)]
pub struct AerialDuelResult {
    pub winner_idx: usize,
    pub winner_name: String,
    pub attacker_won: bool,
}

/// Gold trait 보너스 상수
const AIR_RAID_AERIAL_BONUS: f32 = 0.25;
const BULLY_AERIAL_BONUS: f32 = 0.15; // balance.bully_tackle_bonus와 동기화 필요

/// 공중전 결과 계산 (순수 함수)
/// match_sim.rs의 resolve_aerial_duel과 동등한 기능
pub fn resolve_aerial_duel(ctx: &AerialDuelContext, roll: f32) -> AerialDuelResult {
    // 공격자 기본 강도 계산
    let mut attacker_strength = probability::aerial_strength(
        ctx.attacker_heading,
        ctx.attacker_jumping,
        ctx.attacker_strength,
        ctx.attacker_bravery,
    );

    // 공격자 Gold Traits 적용
    if ctx.has_airraid {
        attacker_strength *= 1.0 + AIR_RAID_AERIAL_BONUS;
    }
    if ctx.has_bully {
        attacker_strength *= 1.0 + BULLY_AERIAL_BONUS;
    }

    // 가장 강한 수비수 찾기 (Gold Traits 포함)
    let best_defender = ctx
        .defenders
        .iter()
        .map(|d| {
            let mut def_str =
                probability::aerial_strength(d.heading, d.jumping, d.strength, d.bravery);
            if d.has_airraid {
                def_str *= 1.0 + AIR_RAID_AERIAL_BONUS;
            }
            if d.has_bully {
                def_str *= 1.0 + BULLY_AERIAL_BONUS;
            }
            (d, def_str)
        })
        .max_by(|a, b| {
            match a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal) {
                std::cmp::Ordering::Equal => {
                    // FIX_2601/0116: Use position-based tie-breaker to avoid index bias
                    crate::engine::match_sim::deterministic_tie_hash(
                        a.0.idx, a.0.position,
                        b.0.idx, b.0.position,
                    )
                }
                other => other,
            }
        });

    match best_defender {
        Some((defender, def_strength)) => {
            let total = attacker_strength + def_strength;
            if total == 0.0 {
                return AerialDuelResult {
                    winner_idx: ctx.attacker_idx,
                    winner_name: ctx.attacker_name.clone(),
                    attacker_won: true,
                };
            }

            let win_chance = attacker_strength / total;
            let attacker_won = roll < win_chance;

            if attacker_won {
                AerialDuelResult {
                    winner_idx: ctx.attacker_idx,
                    winner_name: ctx.attacker_name.clone(),
                    attacker_won: true,
                }
            } else {
                AerialDuelResult {
                    winner_idx: defender.idx,
                    winner_name: defender.name.clone(),
                    attacker_won: false,
                }
            }
        }
        None => AerialDuelResult {
            winner_idx: ctx.attacker_idx,
            winner_name: ctx.attacker_name.clone(),
            attacker_won: true,
        },
    }
}

// ============================================================================
// Deception Action (A13)
// ============================================================================

/// 기만 액션 컨텍스트
#[derive(Debug, Clone)]
pub struct DeceptionContext {
    pub attacker_idx: usize,
    pub defender_idx: usize,
    pub action_type: String, // "feint", "skill_move", "body_feint"

    // 공격자 스탯
    pub flair: f32,
    pub technique: f32,
    pub dribbling: f32,
    pub agility: f32,
    pub balance: f32,
    pub composure: f32,

    // 수비자 스탯
    pub anticipation: f32,
    pub concentration: f32,
    pub positioning: f32,
    pub decisions: f32,
    pub defender_agility: f32,
}

/// 기만 액션 결과
#[derive(Debug, Clone)]
pub struct DeceptionResult {
    pub success: bool,
    pub margin: f32, // 승패 차이 (양수=공격자 유리)
}

/// 기만 액션 결과 계산 (순수 함수)
pub fn resolve_deception(ctx: &DeceptionContext, roll: f32) -> DeceptionResult {
    let att_score = probability::attacker_deception_score(
        ctx.flair,
        ctx.technique,
        ctx.dribbling,
        ctx.agility,
        ctx.balance,
        ctx.composure,
        &ctx.action_type,
    );

    let def_score = probability::defender_reaction_score(
        ctx.anticipation,
        ctx.concentration,
        ctx.positioning,
        ctx.decisions,
        ctx.defender_agility,
        &ctx.action_type,
    );

    let win_chance = att_score / (att_score + def_score);
    let success = roll < win_chance;
    let margin = att_score - def_score;

    DeceptionResult { success, margin }
}

// ============================================================================
// Pass Action
// ============================================================================

/// 패스 타겟 정보
#[derive(Debug, Clone)]
pub struct PassTargetInfo {
    pub idx: usize,
    pub name: String,
    pub position: (f32, f32),
    pub off_the_ball: f32,
    pub anticipation: f32,
    pub first_touch: f32,
}

/// 패스 액션 컨텍스트
#[derive(Debug, Clone)]
pub struct PassContext {
    pub passer_idx: usize,
    pub passer_name: String,
    pub is_home: bool,
    pub is_long_pass: bool,
    pub passer_pos: (f32, f32),

    pub passing: f32,
    pub vision: f32,
    pub technique: f32,
    pub positional_modifier: f32,

    pub target: PassTargetInfo,
    pub opponent_positions: Vec<(f32, f32)>,

    // Gold Traits
    pub has_architect: bool,
    pub has_maestro: bool, // Gold Maestro trait
}

/// 패스에 필요한 랜덤 값들
#[derive(Debug, Clone)]
pub struct PassRolls {
    pub success_roll: f32,
    pub intercept_roll: f32,
}

/// 패스 결과
#[derive(Debug, Clone)]
pub struct PassResult {
    pub success: bool,
    pub intercepted: bool,
    pub interceptor_idx: Option<usize>,
    pub target_idx: usize,
    pub target_name: String,
}

/// 패스 결과 계산 (순수 함수)
///
/// TODO(FIX_2601/D-4): Migrate to FieldBoard + attribute_calc::pressure_penalty()
/// when PassContext gains access to FieldBoard or PressureLevel.
#[allow(deprecated)] // probability::pressure_penalty - see D-4 migration plan
pub fn resolve_pass(ctx: &PassContext, rolls: &PassRolls, distance_m: f32) -> PassResult {
    let pressure = probability::pressure_penalty(ctx.passer_pos, &ctx.opponent_positions);
    let interception_risk = probability::interception_risk(
        ctx.passer_pos,
        ctx.target.position,
        &ctx.opponent_positions,
    );

    let mut success_rate = probability::pass_success(
        distance_m,
        ctx.passing,
        ctx.vision,
        ctx.technique,
        ctx.positional_modifier,
        pressure,
        interception_risk,
        ctx.is_home,
    );

    // Gold Trait 효과
    if ctx.has_architect && ctx.is_long_pass {
        success_rate = (success_rate + 0.15).min(0.95);
    }
    if ctx.has_maestro {
        success_rate = (success_rate + 0.10).min(0.95);
    }

    let success = rolls.success_roll < success_rate;

    // 인터셉트 체크 (패스 실패 시)
    let intercepted = !success && rolls.intercept_roll < interception_risk * 2.0;

    PassResult {
        success,
        intercepted,
        interceptor_idx: if intercepted { Some(0) } else { None }, // 실제 인터셉터는 호출자가 결정
        target_idx: ctx.target.idx,
        target_name: ctx.target.name.clone(),
    }
}

// ============================================================================
// Cross Action
// ============================================================================

/// 크로스 액션 컨텍스트
#[derive(Debug, Clone)]
pub struct CrossContext {
    pub crosser_idx: usize,
    pub crosser_name: String,
    pub is_home: bool,
    pub crosser_pos: (f32, f32),

    pub crossing: f32,
    pub technique: f32,
    pub vision: f32,
    pub distance_m: f32,

    pub targets: Vec<CrossTarget>,

    // Gold Traits
    pub has_crosser: bool,
}

/// 크로스 타겟 정보
#[derive(Debug, Clone)]
pub struct CrossTarget {
    pub idx: usize,
    pub name: String,
    pub heading: f32,
    pub jumping: f32,
    pub position: (f32, f32),
    pub zone_weight: f32,
}

/// 크로스에 필요한 랜덤 값들
#[derive(Debug, Clone)]
pub struct CrossRolls {
    pub accuracy_roll: f32,
    pub header_roll: f32,
}

/// 크로스 결과
#[derive(Debug, Clone)]
pub struct CrossResult {
    pub successful_delivery: bool,
    pub target_reached: Option<usize>,
    pub target_name: Option<String>,
}

/// 크로스 결과 계산 (순수 함수)
pub fn resolve_cross(ctx: &CrossContext, rolls: &CrossRolls) -> CrossResult {
    let mut accuracy =
        probability::cross_accuracy(ctx.crossing, ctx.technique, ctx.vision, ctx.distance_m);

    if ctx.has_crosser {
        accuracy = (accuracy + 0.15).min(0.90);
    }

    let successful_delivery = rolls.accuracy_roll < accuracy;

    if !successful_delivery || ctx.targets.is_empty() {
        return CrossResult { successful_delivery: false, target_reached: None, target_name: None };
    }

    // 최적 타겟 선택
    let best_target = ctx.targets.iter().max_by(|a, b| {
        let score_a = (skills::normalize(a.heading) * 0.6 + skills::normalize(a.jumping) * 0.4)
            * a.zone_weight;
        let score_b = (skills::normalize(b.heading) * 0.6 + skills::normalize(b.jumping) * 0.4)
            * b.zone_weight;
        match score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal) {
            std::cmp::Ordering::Equal => {
                // FIX_2601/0116: Use position-based tie-breaker to avoid index bias
                crate::engine::match_sim::deterministic_tie_hash(
                    a.idx, a.position,
                    b.idx, b.position,
                )
            }
            other => other,
        }
    });

    if let Some(target) = best_target {
        CrossResult {
            successful_delivery: true,
            target_reached: Some(target.idx),
            target_name: Some(target.name.clone()),
        }
    } else {
        CrossResult { successful_delivery: false, target_reached: None, target_name: None }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shot_on_target() {
        let ctx = ShotContext {
            shooter_idx: 9,
            shooter_name: "Striker".to_string(),
            is_home: true,
            player_pos: (0.5, 0.85),
            distance_m: 15.0,
            finishing: 16.0,
            composure: 15.0,
            technique: 14.0,
            long_shots: 12.0,
            strength: 13.0,
            positional_modifier: 0.7,
            under_pressure: false,
            ball_height: 0.3,
            has_cannon: false,
            has_sniper: false,
            has_lob_master: false,
            has_acrobat: false,
            gk_idx: 11,
            gk_reflexes: 14.0,
            gk_positioning: 13.0,
            gk_handling: 12.0,
            gk_diving: 13.0,
        };

        let rolls = ShotRolls {
            power_variance: 0.0,
            accuracy_variance: 0.0,
            on_target_roll: 0.2, // Low = on target
            goal_roll: 0.1,
            save_roll: 0.5,
        };

        let result = resolve_shot(&ctx, &rolls);
        assert!(result.on_target, "Shot should be on target");
        assert!(result.xg > 0.1, "xG should be reasonable: {}", result.xg);
    }

    #[test]
    fn test_shot_goal_with_gold_cannon() {
        // Test Gold Cannon trait: Long range shots have better goal chance
        let ctx = ShotContext {
            shooter_idx: 9,
            shooter_name: "Striker".to_string(),
            is_home: true,
            player_pos: (0.5, 0.5),
            distance_m: 35.0, // Long range (>= 30m triggers Cannon)
            finishing: 14.0,
            composure: 13.0,
            technique: 14.0,
            long_shots: 17.0, // Good long shots
            strength: 15.0,
            positional_modifier: 0.6,
            under_pressure: false,
            ball_height: 0.3,
            has_cannon: true, // Gold Cannon
            has_sniper: false,
            has_lob_master: false,
            has_acrobat: false,
            gk_idx: 11,
            gk_reflexes: 14.0,
            gk_positioning: 13.0,
            gk_handling: 12.0,
            gk_diving: 13.0,
        };

        let rolls = ShotRolls {
            power_variance: 0.0,
            accuracy_variance: 0.0,
            on_target_roll: 0.1, // On target
            goal_roll: 0.25,     // Should score with 35% chance from Cannon
            save_roll: 0.5,
        };

        let result = resolve_shot(&ctx, &rolls);
        // Gold Cannon at 35m gives goal_prob = 0.35
        // roll 0.25 < 0.35 = goal
        assert!(result.on_target, "Shot should be on target");
        assert!(result.is_goal, "Cannon should score long range");
    }

    #[test]
    fn test_tackle_success() {
        let ctx = TackleContext {
            tackler_idx: 4,
            tackler_name: "Defender".to_string(),
            ball_holder_idx: 9,
            ball_holder_name: "Striker".to_string(),
            is_home: true,
            tackling: 16.0,
            anticipation: 15.0,
            aggression: 13.0,
            positioning: 14.0,
            dribbling: 14.0,
            balance: 13.0,
            instruction_modifier: 1.0,
            has_vacuum: false,
            has_wall: false,
            has_reader: false,
            has_bully: false,
            has_perfect_tackle: false,
        };

        let rolls = TackleRolls {
            success_roll: 0.3, // Low = success
            foul_roll: 0.9,
            card_roll: 0.9,
            injury_roll: 0.99,
        };

        let result = resolve_tackle(&ctx, &rolls);
        assert!(result.success, "Tackle should succeed");
        assert!(!result.foul, "No foul expected");
    }

    #[test]
    fn test_tackle_foul() {
        let ctx = TackleContext {
            tackler_idx: 4,
            tackler_name: "Defender".to_string(),
            ball_holder_idx: 9,
            ball_holder_name: "Striker".to_string(),
            is_home: true,
            tackling: 10.0,
            anticipation: 10.0,
            aggression: 17.0, // High aggression
            positioning: 11.0,
            dribbling: 16.0,
            balance: 15.0,
            instruction_modifier: 1.0,
            has_vacuum: false,
            has_wall: false,
            has_reader: false,
            has_bully: false,
            has_perfect_tackle: false,
        };

        let rolls = TackleRolls {
            success_roll: 0.9, // High = failure
            foul_roll: 0.1,    // Low = foul
            card_roll: 0.2,    // Could be yellow
            injury_roll: 0.99,
        };

        let result = resolve_tackle(&ctx, &rolls);
        assert!(!result.success, "Tackle should fail");
        assert!(result.foul, "Foul expected");
    }

    #[test]
    fn test_dribble_success() {
        let ctx = DribbleContext {
            dribbler_idx: 10,
            dribbler_name: "Winger".to_string(),
            is_home: true,
            current_pos: (0.7, 0.5),
            target_pos: (0.8, 0.6),
            dribbling: 17.0,
            agility: 16.0,
            balance: 15.0,
            pace: 16.0,
            defender_idx: Some(4),
            defender_marking: Some(13.0),
            defender_pace: Some(12.0),
            defender_positioning: Some(13.0),
            offensive_modifier: 1.0,
            defensive_modifier: 1.0,
            has_speed_demon: false,
            has_speedster: false,
            has_technician: false,
            has_tank: false,
            defender_has_shadow: false,
            defender_has_bully: false,
        };

        let rolls = DribbleRolls { success_roll: 0.3 };

        let result = resolve_dribble(&ctx, &rolls);
        assert!(result.success, "Skilled dribbler should succeed");
        assert_eq!(result.new_position, ctx.target_pos);
    }

    #[test]
    fn test_dribble_with_technician() {
        // Gold Technician: 1:1 돌파 시 수비수 거의 확실히 제침 (85% 성공률)
        let ctx = DribbleContext {
            dribbler_idx: 10,
            dribbler_name: "Winger".to_string(),
            is_home: true,
            current_pos: (0.7, 0.5),
            target_pos: (0.8, 0.6),
            dribbling: 15.0,
            agility: 14.0,
            balance: 13.0,
            pace: 14.0,
            defender_idx: Some(4),
            defender_marking: Some(14.0),
            defender_pace: Some(14.0),
            defender_positioning: Some(14.0),
            offensive_modifier: 1.0,
            defensive_modifier: 1.0,
            has_speed_demon: false,
            has_speedster: false,
            has_technician: true, // Gold Technician
            has_tank: false,
            defender_has_shadow: false,
            defender_has_bully: false,
        };

        let rolls = DribbleRolls { success_roll: 0.5 };

        let result = resolve_dribble(&ctx, &rolls);
        // Gold Technician gives 85% success rate, 0.5 < 0.85 = success
        assert!(result.success, "Gold Technician should help");
    }

    #[test]
    fn test_dribble_speed_demon() {
        // A13: SpeedDemon - pace 차이 5+ (0.25 normalized)면 자동 성공
        let ctx = DribbleContext {
            dribbler_idx: 10,
            dribbler_name: "Winger".to_string(),
            is_home: true,
            current_pos: (0.7, 0.5),
            target_pos: (0.8, 0.6),
            dribbling: 12.0, // 낮은 드리블
            agility: 12.0,
            balance: 12.0,
            pace: 18.0, // 높은 속도
            defender_idx: Some(4),
            defender_marking: Some(16.0), // 높은 마킹
            defender_pace: Some(12.0),    // 낮은 속도 (차이 6)
            defender_positioning: Some(15.0),
            offensive_modifier: 1.0,
            defensive_modifier: 1.0,
            has_speed_demon: true, // SpeedDemon 스킬!
            has_speedster: false,
            has_technician: false,
            has_tank: false,
            defender_has_shadow: false,
            defender_has_bully: false,
        };

        let rolls = DribbleRolls { success_roll: 0.99 }; // 아무리 높아도

        let result = resolve_dribble(&ctx, &rolls);
        // SpeedDemon + pace 차이 6 (0.3 >= 0.25) = 자동 성공
        assert!(result.success, "SpeedDemon with pace advantage should auto-succeed");
    }

    #[test]
    fn test_dribble_shadow_counter() {
        // Gold Shadow (수비수): 드리블 성공률 최대 50%로 제한
        let ctx = DribbleContext {
            dribbler_idx: 10,
            dribbler_name: "Winger".to_string(),
            is_home: true,
            current_pos: (0.7, 0.5),
            target_pos: (0.8, 0.6),
            dribbling: 18.0, // 매우 높은 드리블
            agility: 17.0,
            balance: 16.0,
            pace: 16.0,
            defender_idx: Some(4),
            defender_marking: Some(10.0), // 낮은 마킹
            defender_pace: Some(10.0),
            defender_positioning: Some(10.0),
            offensive_modifier: 1.0,
            defensive_modifier: 1.0,
            has_speed_demon: false,
            has_speedster: false,
            has_technician: false,
            has_tank: false,
            defender_has_shadow: true, // Gold Shadow!
            defender_has_bully: false,
        };

        let rolls = DribbleRolls { success_roll: 0.6 }; // 60%

        let result = resolve_dribble(&ctx, &rolls);
        // Shadow caps at 50%, 0.6 > 0.5 = fail
        assert!(!result.success, "Gold Shadow should cap dribble success");
        assert!(result.tackled, "Failed dribble with defender = tackled");
    }

    #[test]
    fn test_aerial_duel_attacker_wins() {
        let ctx = AerialDuelContext {
            attacker_idx: 9,
            attacker_name: "Striker".to_string(),
            attacker_heading: 17.0,
            attacker_jumping: 16.0,
            attacker_strength: 15.0,
            attacker_bravery: 14.0,
            has_airraid: false,
            has_bully: false,
            defenders: vec![AerialDefender {
                idx: 4,
                name: "CB".to_string(),
                heading: 13.0,
                jumping: 12.0,
                strength: 14.0,
                bravery: 13.0,
                has_airraid: false,
                has_bully: false,
                position: (0.5, 0.5),
            }],
        };

        let result = resolve_aerial_duel(&ctx, 0.3); // Favor attacker
        assert!(result.attacker_won, "Strong header should win");
        assert_eq!(result.winner_idx, 9);
    }

    #[test]
    fn test_aerial_duel_defender_wins() {
        let ctx = AerialDuelContext {
            attacker_idx: 9,
            attacker_name: "Striker".to_string(),
            attacker_heading: 12.0,
            attacker_jumping: 11.0,
            attacker_strength: 12.0,
            attacker_bravery: 11.0,
            has_airraid: false,
            has_bully: false,
            defenders: vec![AerialDefender {
                idx: 4,
                name: "CB".to_string(),
                heading: 17.0,
                jumping: 16.0,
                strength: 16.0,
                bravery: 15.0,
                has_airraid: false,
                has_bully: false,
                position: (0.5, 0.5),
            }],
        };

        let result = resolve_aerial_duel(&ctx, 0.7); // Favor defender
        assert!(!result.attacker_won, "Strong CB should win");
        assert_eq!(result.winner_idx, 4);
    }

    #[test]
    fn test_aerial_duel_airraid_bonus() {
        // 공격자 AirRaid가 있으면 약한 스탯으로도 이길 수 있음
        let ctx = AerialDuelContext {
            attacker_idx: 9,
            attacker_name: "Striker".to_string(),
            attacker_heading: 12.0,
            attacker_jumping: 12.0,
            attacker_strength: 12.0,
            attacker_bravery: 12.0,
            has_airraid: true, // +25% 보너스
            has_bully: false,
            defenders: vec![AerialDefender {
                idx: 4,
                name: "CB".to_string(),
                heading: 14.0,
                jumping: 14.0,
                strength: 14.0,
                bravery: 14.0,
                has_airraid: false,
                has_bully: false,
                position: (0.5, 0.5),
            }],
        };

        let result = resolve_aerial_duel(&ctx, 0.4);
        assert!(result.attacker_won, "AirRaid bonus should help win");
    }

    #[test]
    fn test_deception_success() {
        let ctx = DeceptionContext {
            attacker_idx: 10,
            defender_idx: 3,
            action_type: "skill_move".to_string(),
            flair: 17.0,
            technique: 16.0,
            dribbling: 17.0,
            agility: 16.0,
            balance: 15.0,
            composure: 14.0,
            anticipation: 13.0,
            concentration: 12.0,
            positioning: 13.0,
            decisions: 12.0,
            defender_agility: 12.0,
        };

        let result = resolve_deception(&ctx, 0.3);
        assert!(result.success, "Skilled attacker should deceive");
        assert!(result.margin > 0.0, "Margin should favor attacker");
    }

    #[test]
    fn test_pass_success() {
        let ctx = PassContext {
            passer_idx: 6,
            passer_name: "CM".to_string(),
            is_home: true,
            is_long_pass: false,
            passer_pos: (0.5, 0.5),
            passing: 16.0,
            vision: 15.0,
            technique: 14.0,
            positional_modifier: 0.7,
            target: PassTargetInfo {
                idx: 9,
                name: "ST".to_string(),
                position: (0.5, 0.7),
                off_the_ball: 14.0,
                anticipation: 13.0,
                first_touch: 14.0,
            },
            opponent_positions: vec![(0.6, 0.6)],
            has_architect: false,
            has_maestro: false,
        };

        let rolls = PassRolls { success_roll: 0.3, intercept_roll: 0.9 };

        let result = resolve_pass(&ctx, &rolls, 20.0);
        assert!(result.success, "Good passer should succeed");
        assert!(!result.intercepted);
    }

    #[test]
    fn test_cross_success() {
        let ctx = CrossContext {
            crosser_idx: 7,
            crosser_name: "RW".to_string(),
            is_home: true,
            crosser_pos: (0.9, 0.8),
            crossing: 16.0,
            technique: 15.0,
            vision: 14.0,
            distance_m: 25.0,
            targets: vec![CrossTarget {
                idx: 9,
                name: "ST".to_string(),
                heading: 15.0,
                jumping: 14.0,
                position: (0.5, 0.9),
                zone_weight: 1.0,
            }],
            has_crosser: false,
        };

        let rolls = CrossRolls { accuracy_roll: 0.3, header_roll: 0.5 };

        let result = resolve_cross(&ctx, &rolls);
        assert!(result.successful_delivery, "Good crosser should deliver");
        assert!(result.target_reached.is_some());
    }
}
