//! Defense Intent System (P16 Phase 1)
//!
//! Attack과 대칭적인 Defense Intent 체계 구축
//! - Contain: 지연/봉쇄 (Jockey, DelayPass, CoverSpace, BlockLane)
//! - Press: 압박/차단 (ClosingDown, InterceptAttempt, ForceTouchline, TrackRunner)
//! - Challenge: 도전/탈취 (StandingTackle, SlidingTackle, ShoulderCharge, PokeAway)
//!
//! ## Gate 구조에서의 역할
//! - Gate A: Mindset 기반 Intent 후보 필터링
//! - Gate B: Utility 기반 Intent 선택
//! - Gate C: Intent → Technique 매핑 (ActionModel)

// ============================================================================
// Defense Intent Enums
// ============================================================================

/// Defense Intent Trinity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefenseIntent {
    /// 지연/봉쇄: 시간을 벌고 공격 진행을 늦춤
    Contain(ContainTechnique),
    /// 압박/차단: 공격수에게 압박을 가해 실수 유도
    Press(PressTechnique),
    /// 도전/탈취: 직접적인 볼 탈취 시도
    Challenge(ChallengeTechnique),
}

impl Default for DefenseIntent {
    fn default() -> Self {
        Self::Contain(ContainTechnique::Jockey)
    }
}

// ============================================================================
// Technique Enums
// ============================================================================

/// Contain Technique (지연/봉쇄)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainTechnique {
    /// 1:1 마크하며 거리 유지
    #[default]
    Jockey,
    /// 패스 레인 차단
    DelayPass,
    /// 공간 커버
    CoverSpace,
    /// 슛/패스 레인 차단
    BlockLane,
}

/// Press Technique (압박/차단)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PressTechnique {
    /// 빠른 접근
    #[default]
    ClosingDown,
    /// 패스 차단 시도
    InterceptAttempt,
    /// 사이드로 밀기
    ForceTouchline,
    /// 러너 추적
    TrackRunner,
}

/// Challenge Technique (도전/탈취)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChallengeTechnique {
    /// 안전한 태클
    #[default]
    StandingTackle,
    /// 위험하지만 범위 넓음
    SlidingTackle,
    /// 피지컬 대결
    ShoulderCharge,
    /// 빠른 볼 터치 시도
    PokeAway,
}

// ============================================================================
// Defense Context
// ============================================================================

/// 수비 상황 컨텍스트
#[derive(Debug, Clone, Default)]
pub struct DefenseContext {
    /// 볼 캐리어까지의 거리 (미터)
    pub distance_to_ball_carrier: f32,
    /// 선수 스태미나 (0.0-1.0) - 0108: Stamina-Aware Defense
    pub stamina_percent: f32,
    /// 1:1 상황 여부
    pub is_one_on_one: bool,
    /// 위험한 패스 레인 존재
    pub dangerous_pass_lane: Option<usize>,
    /// 위험한 공간 존재
    pub dangerous_space: Option<(f32, f32)>,
    /// 패스 진행 중 여부
    pub pass_in_progress: bool,
    /// 위험한 러너 (오프더볼 움직임)
    pub dangerous_runner: Option<usize>,
    /// 터치라인 근처 여부
    pub near_touchline: bool,
    /// 공격수가 빠르게 지나가려 함
    pub attacker_is_passing_by: bool,
    /// 페널티박스 내 여부
    pub in_penalty_box: bool,
    /// 어깨 대결 가능
    pub shoulder_to_shoulder_possible: bool,
    /// 볼이 루즈한 상태
    pub ball_loose: bool,
    /// 역습 상황
    pub is_counter_attack: bool,
    /// 수적 우위 (양수면 우위, 음수면 열세)
    pub numerical_advantage: i8,
    /// 위험 지역 (Red/Yellow/Green)
    pub danger_zone: DangerZone,
}

/// 위험 지역 분류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DangerZone {
    /// 골 전방 박스 (매우 위험)
    Red,
    /// 페널티박스 주변
    Yellow,
    /// 안전 지역
    #[default]
    Green,
}

// ============================================================================
// Intent Selection
// ============================================================================

/// 스태미나 고갈 임계값 (0108: Open-Football Integration)
/// 이 값 이하면 고강도 수비(Press, Challenge)를 저강도(Contain)로 전환
const STAMINA_EXHAUSTED_THRESHOLD: f32 = 0.30;

/// 거리 기반 기본 Intent 결정
pub fn base_intent_by_distance(distance_m: f32) -> DefenseIntent {
    if distance_m > 8.0 {
        DefenseIntent::Contain(ContainTechnique::Jockey)
    } else if distance_m > 3.0 {
        DefenseIntent::Press(PressTechnique::ClosingDown)
    } else {
        DefenseIntent::Challenge(ChallengeTechnique::StandingTackle)
    }
}

/// Defense Intent 선택 (Main Entry Point)
pub fn select_defense_intent(
    ctx: &DefenseContext,
    attributes: &DefenderAttributes,
) -> DefenseIntent {
    // 0. 스태미나 체크 (0108: Stamina-Aware Defense)
    // 스태미나가 30% 이하면 고강도 수비를 저강도로 전환
    if ctx.stamina_percent > 0.0 && ctx.stamina_percent < STAMINA_EXHAUSTED_THRESHOLD {
        // 지친 선수는 Contain만 가능 (에너지 보존)
        return DefenseIntent::Contain(ContainTechnique::DelayPass);
    }

    // 1. 거리 기반 기본 Intent
    let base_intent = base_intent_by_distance(ctx.distance_to_ball_carrier);

    // 2. 상황 기반 수정
    let situational_intent = adjust_for_situation(base_intent, ctx);

    // 3. 속성 기반 최종 결정
    finalize_intent(situational_intent, ctx, attributes)
}

/// 상황에 따른 Intent 수정
fn adjust_for_situation(base: DefenseIntent, ctx: &DefenseContext) -> DefenseIntent {
    // 역습 상황: Contain 선호
    if ctx.is_counter_attack {
        if let DefenseIntent::Challenge(_) = base {
            return DefenseIntent::Contain(ContainTechnique::Jockey);
        }
    }

    // 수적 열세: Contain 선호
    if ctx.numerical_advantage < 0 {
        if let DefenseIntent::Challenge(_) = base {
            return DefenseIntent::Contain(ContainTechnique::CoverSpace);
        }
    }

    // 페널티박스 내: Challenge 선호 (but careful)
    if ctx.in_penalty_box && ctx.distance_to_ball_carrier < 3.0 {
        return DefenseIntent::Challenge(ChallengeTechnique::StandingTackle);
    }

    // 터치라인 근처: Press로 밀어붙이기
    if ctx.near_touchline && ctx.distance_to_ball_carrier < 6.0 {
        return DefenseIntent::Press(PressTechnique::ForceTouchline);
    }

    // 패스 진행 중: Intercept 시도
    if ctx.pass_in_progress {
        return DefenseIntent::Press(PressTechnique::InterceptAttempt);
    }

    base
}

/// 속성 기반 최종 Intent 결정
fn finalize_intent(
    base: DefenseIntent,
    ctx: &DefenseContext,
    attrs: &DefenderAttributes,
) -> DefenseIntent {
    match base {
        DefenseIntent::Contain(tech) => {
            DefenseIntent::Contain(select_contain_technique(ctx, attrs, tech))
        }
        DefenseIntent::Press(tech) => {
            DefenseIntent::Press(select_press_technique(ctx, attrs, tech))
        }
        DefenseIntent::Challenge(tech) => {
            DefenseIntent::Challenge(select_challenge_technique(ctx, attrs, tech))
        }
    }
}

// ============================================================================
// Technique Selection
// ============================================================================

/// Contain Technique 선택
fn select_contain_technique(
    ctx: &DefenseContext,
    _attrs: &DefenderAttributes,
    default: ContainTechnique,
) -> ContainTechnique {
    // 1:1 상황
    if ctx.is_one_on_one {
        return ContainTechnique::Jockey;
    }

    // 패스 레인 위협
    if ctx.dangerous_pass_lane.is_some() {
        return ContainTechnique::BlockLane;
    }

    // 공간 위협
    if ctx.dangerous_space.is_some() {
        return ContainTechnique::CoverSpace;
    }

    default
}

/// Press Technique 선택
fn select_press_technique(
    ctx: &DefenseContext,
    _attrs: &DefenderAttributes,
    default: PressTechnique,
) -> PressTechnique {
    // 패스 진행 중
    if ctx.pass_in_progress {
        return PressTechnique::InterceptAttempt;
    }

    // 러너 추적 필요
    if ctx.dangerous_runner.is_some() {
        return PressTechnique::TrackRunner;
    }

    // 사이드라인 근처
    if ctx.near_touchline {
        return PressTechnique::ForceTouchline;
    }

    default
}

/// Challenge Technique 선택
fn select_challenge_technique(
    ctx: &DefenseContext,
    attrs: &DefenderAttributes,
    default: ChallengeTechnique,
) -> ChallengeTechnique {
    // 슬라이딩 태클 조건
    if ctx.attacker_is_passing_by
        && attrs.tackling > 70.0
        && attrs.aggression > 65.0
        && !ctx.in_penalty_box
    {
        return ChallengeTechnique::SlidingTackle;
    }

    // 피지컬 대결 조건
    if attrs.strength > 75.0 && ctx.shoulder_to_shoulder_possible {
        return ChallengeTechnique::ShoulderCharge;
    }

    // 빠른 찔러넣기
    if ctx.ball_loose && attrs.reactions > 70.0 {
        return ChallengeTechnique::PokeAway;
    }

    default
}

// ============================================================================
// Defender Attributes (Snapshot)
// ============================================================================

/// 수비수 속성 스냅샷
#[derive(Debug, Clone, Copy, Default)]
pub struct DefenderAttributes {
    // Primary
    pub tackling: f32,
    pub positioning: f32,
    pub marking: f32,

    // Physical
    pub pace: f32,
    pub strength: f32,
    pub stamina: f32,
    pub agility: f32,
    pub balance: f32,

    // Mental
    pub anticipation: f32,
    pub concentration: f32,
    pub aggression: f32,
    pub bravery: f32,
    pub work_rate: f32,
    pub reactions: f32,

    // Technical
    pub interceptions: f32,
}

impl DefenderAttributes {
    /// Overall 능력치에서 기본 속성 생성
    pub fn from_overall(overall: u8) -> Self {
        let base = overall as f32;
        Self {
            tackling: base,
            positioning: base,
            marking: base,
            pace: base,
            strength: base,
            stamina: base,
            agility: base,
            balance: base,
            anticipation: base,
            concentration: base,
            aggression: base * 0.8, // 기본적으로 약간 낮게
            bravery: base,
            work_rate: base,
            reactions: base,
            interceptions: base,
        }
    }

    /// Create from Player.attributes (when available)
    /// Converts FM attributes (0-100 scale) to DefenderAttributes (f32)
    pub fn from_player_attributes(attrs: &crate::models::player::PlayerAttributes) -> Self {
        Self {
            tackling: attrs.tackling as f32,
            positioning: attrs.positioning as f32,
            marking: attrs.marking as f32,
            pace: attrs.pace as f32,
            strength: attrs.strength as f32,
            stamina: attrs.stamina as f32,
            agility: attrs.agility as f32,
            balance: attrs.balance as f32,
            anticipation: attrs.anticipation as f32,
            concentration: attrs.concentration as f32,
            aggression: attrs.aggression as f32,
            bravery: attrs.bravery as f32,
            work_rate: attrs.work_rate as f32,
            reactions: attrs.anticipation as f32, // Proxy (not in FM 36 attributes)
            interceptions: ((attrs.anticipation + attrs.positioning) / 2) as f32, // Composite
        }
    }
}

// ============================================================================
// Defense Score Calculation
// ============================================================================

/// Intent별 수비 점수 계산
pub fn calculate_defense_score(intent: DefenseIntent, attrs: &DefenderAttributes) -> f32 {
    match intent {
        DefenseIntent::Contain(_) => {
            attrs.positioning * 0.35
                + attrs.anticipation * 0.30
                + attrs.concentration * 0.20
                + attrs.agility * 0.15
        }
        DefenseIntent::Press(_) => {
            attrs.work_rate * 0.30
                + attrs.aggression * 0.25
                + attrs.pace * 0.25
                + attrs.stamina * 0.20
        }
        DefenseIntent::Challenge(_) => {
            attrs.tackling * 0.35
                + attrs.strength * 0.25
                + attrs.bravery * 0.20
                + attrs.reactions * 0.20
        }
    }
}

/// Technique별 수정자 계산
pub fn calculate_technique_modifier(intent: DefenseIntent, attrs: &DefenderAttributes) -> f32 {
    match intent {
        // Contain
        DefenseIntent::Contain(ContainTechnique::Jockey) => {
            (attrs.agility * 0.4 + attrs.balance * 0.3 + attrs.reactions * 0.3) / 100.0
        }
        DefenseIntent::Contain(ContainTechnique::BlockLane) => {
            (attrs.positioning * 0.5 + attrs.anticipation * 0.3 + attrs.reactions * 0.2) / 100.0
        }
        DefenseIntent::Contain(ContainTechnique::CoverSpace) => {
            (attrs.positioning * 0.4 + attrs.anticipation * 0.4 + attrs.pace * 0.2) / 100.0
        }
        DefenseIntent::Contain(ContainTechnique::DelayPass) => {
            (attrs.positioning * 0.4 + attrs.anticipation * 0.3 + attrs.work_rate * 0.3) / 100.0
        }

        // Press
        DefenseIntent::Press(PressTechnique::ClosingDown) => {
            (attrs.pace * 0.4 + attrs.agility * 0.3 + attrs.work_rate * 0.3) / 100.0
        }
        DefenseIntent::Press(PressTechnique::InterceptAttempt) => {
            (attrs.interceptions * 0.5 + attrs.anticipation * 0.3 + attrs.reactions * 0.2) / 100.0
        }
        DefenseIntent::Press(PressTechnique::ForceTouchline) => {
            (attrs.positioning * 0.4 + attrs.work_rate * 0.3 + attrs.pace * 0.3) / 100.0
        }
        DefenseIntent::Press(PressTechnique::TrackRunner) => {
            (attrs.pace * 0.4 + attrs.stamina * 0.3 + attrs.marking * 0.3) / 100.0
        }

        // Challenge
        DefenseIntent::Challenge(ChallengeTechnique::StandingTackle) => {
            (attrs.tackling * 0.5 + attrs.balance * 0.25 + attrs.strength * 0.25) / 100.0
        }
        DefenseIntent::Challenge(ChallengeTechnique::SlidingTackle) => {
            (attrs.tackling * 0.4 + attrs.aggression * 0.3 + attrs.bravery * 0.3) / 100.0
        }
        DefenseIntent::Challenge(ChallengeTechnique::ShoulderCharge) => {
            (attrs.strength * 0.5 + attrs.balance * 0.25 + attrs.aggression * 0.25) / 100.0
        }
        DefenseIntent::Challenge(ChallengeTechnique::PokeAway) => {
            (attrs.reactions * 0.5 + attrs.tackling * 0.3 + attrs.anticipation * 0.2) / 100.0
        }
    }
}

// ============================================================================
// Foul & Card System
// ============================================================================

/// 파울 확률 계산 (Challenge)
pub fn calculate_foul_probability(
    challenge: ChallengeTechnique,
    attrs: &DefenderAttributes,
    defense_won: bool,
) -> f32 {
    let base_foul_rate = match challenge {
        ChallengeTechnique::StandingTackle => 0.10,
        ChallengeTechnique::SlidingTackle => 0.25,
        ChallengeTechnique::ShoulderCharge => 0.15,
        ChallengeTechnique::PokeAway => 0.05,
    };

    // 실패 시 파울 확률 증가
    let outcome_mod = if defense_won { 0.5 } else { 1.5 };

    // 속성 기반 수정 (태클 실력 좋으면 파울 감소)
    let skill_mod = 1.0 - (attrs.tackling / 200.0);

    (base_foul_rate * outcome_mod * skill_mod).clamp(0.0, 1.0)
}

/// 파울 확률 계산 (Press)
/// FIX_2601/0109: Press 상황에서도 파울 발생 가능 (실제 축구에서 압박 중 파울 빈번)
pub fn calculate_press_foul_probability(
    press: PressTechnique,
    attrs: &DefenderAttributes,
    defense_won: bool,
) -> f32 {
    // Press는 Challenge보다 기본 파울 확률 낮음
    let base_foul_rate = match press {
        PressTechnique::ClosingDown => 0.08,      // 빠른 접근 시 충돌
        PressTechnique::ForceTouchline => 0.10,   // 밀어붙이기
        PressTechnique::InterceptAttempt => 0.05, // 인터셉트 시도
        PressTechnique::TrackRunner => 0.06,      // 추적 중 충돌
    };

    // 실패 시 파울 확률 증가 (Challenge보다 낮은 증가율)
    let outcome_mod = if defense_won { 0.6 } else { 1.3 };

    // aggression 기반 수정 (높은 aggression = 더 많은 파울)
    let aggression_mod = 1.0 + (attrs.aggression / 100.0) * 0.3;

    // 태클 실력으로 약간 감소
    let skill_mod = 1.0 - (attrs.tackling / 300.0);

    (base_foul_rate * outcome_mod * aggression_mod * skill_mod).clamp(0.0, 0.35)
}

/// 카드 확률 계산 (Press)
/// Press 파울은 Challenge보다 카드 확률 낮음
pub fn calculate_press_card_probability(
    press: PressTechnique,
    attrs: &DefenderAttributes,
    in_penalty_box: bool,
) -> (f32, f32) {
    let (base_yellow, base_red) = match press {
        PressTechnique::ClosingDown => (0.08, 0.005),
        PressTechnique::ForceTouchline => (0.12, 0.01),
        PressTechnique::InterceptAttempt => (0.05, 0.002),
        PressTechnique::TrackRunner => (0.06, 0.005),
    };

    let mut yellow: f32 = base_yellow;
    let mut red: f32 = base_red;

    // 페널티박스 내
    if in_penalty_box {
        yellow += 0.08;
    }

    // 과격한 플레이
    if attrs.aggression > 80.0 {
        yellow += 0.08;
        red += 0.02;
    }

    (yellow.min(0.5_f32), red.min(0.15_f32))
}

/// 카드 확률 계산 (yellow, red)
pub fn calculate_card_probability(
    challenge: ChallengeTechnique,
    attrs: &DefenderAttributes,
    in_penalty_box: bool,
    from_behind: bool,
    denied_goal_opportunity: bool,
) -> (f32, f32) {
    let (base_yellow, base_red) = match challenge {
        ChallengeTechnique::StandingTackle => (0.15, 0.02),
        ChallengeTechnique::SlidingTackle => (0.30, 0.05),
        ChallengeTechnique::ShoulderCharge => (0.10, 0.01),
        ChallengeTechnique::PokeAway => (0.05, 0.00),
    };

    let mut yellow: f32 = base_yellow;
    let mut red: f32 = base_red;

    // 득점 기회 저지 (DOGSO)
    if denied_goal_opportunity {
        yellow += 0.3;
        red += 0.15;
    }

    // 페널티박스 내
    if in_penalty_box {
        yellow += 0.1;
    }

    // 뒤에서 태클
    if from_behind {
        yellow += 0.2;
        red += 0.10;
    }

    // 과격한 플레이
    if attrs.aggression > 85.0 {
        yellow += 0.1;
        red += 0.05;
    }

    (yellow.min(1.0_f32), red.min(1.0_f32))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_attrs() -> DefenderAttributes {
        DefenderAttributes::from_overall(70)
    }

    #[test]
    fn test_base_intent_by_distance() {
        // 멀면 Contain
        let intent = base_intent_by_distance(12.0);
        assert!(matches!(intent, DefenseIntent::Contain(_)));

        // 중거리면 Press
        let intent = base_intent_by_distance(5.0);
        assert!(matches!(intent, DefenseIntent::Press(_)));

        // 가까우면 Challenge
        let intent = base_intent_by_distance(2.0);
        assert!(matches!(intent, DefenseIntent::Challenge(_)));
    }

    #[test]
    fn test_select_defense_intent() {
        let attrs = default_attrs();

        // 기본 상황
        let ctx = DefenseContext { distance_to_ball_carrier: 10.0, ..Default::default() };
        let intent = select_defense_intent(&ctx, &attrs);
        assert!(matches!(intent, DefenseIntent::Contain(_)));

        // 가까운 상황
        let ctx = DefenseContext { distance_to_ball_carrier: 2.0, ..Default::default() };
        let intent = select_defense_intent(&ctx, &attrs);
        assert!(matches!(intent, DefenseIntent::Challenge(_)));
    }

    #[test]
    fn test_counter_attack_forces_contain() {
        let attrs = default_attrs();

        let ctx = DefenseContext {
            distance_to_ball_carrier: 2.0, // 원래는 Challenge
            is_counter_attack: true,       // 역습 상황
            ..Default::default()
        };

        let intent = select_defense_intent(&ctx, &attrs);
        assert!(
            matches!(intent, DefenseIntent::Contain(_)),
            "Counter attack should force Contain, got {:?}",
            intent
        );
    }

    #[test]
    fn test_numerical_disadvantage_forces_contain() {
        let attrs = default_attrs();

        let ctx = DefenseContext {
            distance_to_ball_carrier: 2.0,
            numerical_advantage: -1, // 수적 열세
            ..Default::default()
        };

        let intent = select_defense_intent(&ctx, &attrs);
        assert!(
            matches!(intent, DefenseIntent::Contain(_)),
            "Numerical disadvantage should force Contain, got {:?}",
            intent
        );
    }

    #[test]
    fn test_touchline_forces_press() {
        let attrs = default_attrs();

        let ctx = DefenseContext {
            distance_to_ball_carrier: 5.0,
            near_touchline: true,
            ..Default::default()
        };

        let intent = select_defense_intent(&ctx, &attrs);
        assert!(
            matches!(intent, DefenseIntent::Press(PressTechnique::ForceTouchline)),
            "Near touchline should use ForceTouchline, got {:?}",
            intent
        );
    }

    #[test]
    fn test_sliding_tackle_selection() {
        let mut attrs = default_attrs();
        attrs.tackling = 80.0;
        attrs.aggression = 70.0;

        let ctx = DefenseContext {
            distance_to_ball_carrier: 1.5,
            attacker_is_passing_by: true,
            in_penalty_box: false,
            ..Default::default()
        };

        let intent = select_defense_intent(&ctx, &attrs);
        assert!(
            matches!(intent, DefenseIntent::Challenge(ChallengeTechnique::SlidingTackle)),
            "Should select sliding tackle, got {:?}",
            intent
        );
    }

    #[test]
    fn test_defense_score_calculation() {
        let attrs = DefenderAttributes {
            positioning: 80.0,
            anticipation: 75.0,
            concentration: 70.0,
            agility: 65.0,
            ..Default::default()
        };

        let score =
            calculate_defense_score(DefenseIntent::Contain(ContainTechnique::Jockey), &attrs);

        // 0.35*80 + 0.30*75 + 0.20*70 + 0.15*65 = 28 + 22.5 + 14 + 9.75 = 74.25
        assert!((score - 74.25).abs() < 0.1, "Score was {}", score);
    }

    #[test]
    fn test_foul_probability() {
        let attrs = DefenderAttributes { tackling: 70.0, ..Default::default() };

        // 슬라이딩 태클 실패 시 높은 파울 확률
        let prob_fail =
            calculate_foul_probability(ChallengeTechnique::SlidingTackle, &attrs, false);

        // 슬라이딩 태클 성공 시 낮은 파울 확률
        let prob_success =
            calculate_foul_probability(ChallengeTechnique::SlidingTackle, &attrs, true);

        assert!(prob_fail > prob_success, "Failed tackle should have higher foul prob");
        assert!(prob_fail > 0.2, "Failed sliding tackle should have high foul prob");
    }

    #[test]
    fn test_card_probability_dogso() {
        let attrs = default_attrs();

        let (yellow_normal, red_normal) = calculate_card_probability(
            ChallengeTechnique::SlidingTackle,
            &attrs,
            false,
            false,
            false,
        );

        let (yellow_dogso, red_dogso) = calculate_card_probability(
            ChallengeTechnique::SlidingTackle,
            &attrs,
            false,
            false,
            true, // DOGSO
        );

        assert!(yellow_dogso > yellow_normal, "DOGSO should increase yellow card prob");
        assert!(red_dogso > red_normal, "DOGSO should increase red card prob");
    }

    #[test]
    fn test_technique_modifier_range() {
        let attrs = DefenderAttributes::from_overall(80);

        // 모든 technique modifier가 0~1 범위인지 확인
        let techniques = [
            DefenseIntent::Contain(ContainTechnique::Jockey),
            DefenseIntent::Contain(ContainTechnique::BlockLane),
            DefenseIntent::Press(PressTechnique::ClosingDown),
            DefenseIntent::Press(PressTechnique::InterceptAttempt),
            DefenseIntent::Challenge(ChallengeTechnique::StandingTackle),
            DefenseIntent::Challenge(ChallengeTechnique::SlidingTackle),
        ];

        for tech in techniques {
            let modifier = calculate_technique_modifier(tech, &attrs);
            assert!(
                (0.0..=1.0).contains(&modifier),
                "Modifier for {:?} was {}, should be 0~1",
                tech,
                modifier
            );
        }
    }

    // ============================================================================
    // 0108: Stamina-Aware Defense Tests
    // ============================================================================

    #[test]
    fn test_stamina_exhausted_forces_contain() {
        let attrs = default_attrs();

        // 스태미나 25% - 지친 상태 (원래 Challenge 거리)
        let ctx = DefenseContext {
            distance_to_ball_carrier: 2.0, // 원래는 Challenge
            stamina_percent: 0.25,         // 30% 이하
            ..Default::default()
        };

        let intent = select_defense_intent(&ctx, &attrs);
        assert!(
            matches!(intent, DefenseIntent::Contain(ContainTechnique::DelayPass)),
            "Exhausted player should use Contain(DelayPass), got {:?}",
            intent
        );
    }

    #[test]
    fn test_stamina_normal_allows_challenge() {
        let attrs = default_attrs();

        // 스태미나 50% - 정상 상태
        let ctx = DefenseContext {
            distance_to_ball_carrier: 2.0, // Challenge 거리
            stamina_percent: 0.50,         // 30% 이상
            ..Default::default()
        };

        let intent = select_defense_intent(&ctx, &attrs);
        assert!(
            matches!(intent, DefenseIntent::Challenge(_)),
            "Normal stamina player should be able to Challenge, got {:?}",
            intent
        );
    }

    #[test]
    fn test_stamina_threshold_boundary() {
        let attrs = default_attrs();

        // 정확히 30% - 경계값 (아직 정상)
        let ctx_at_threshold = DefenseContext {
            distance_to_ball_carrier: 2.0,
            stamina_percent: 0.30,
            ..Default::default()
        };
        let intent = select_defense_intent(&ctx_at_threshold, &attrs);
        assert!(
            matches!(intent, DefenseIntent::Challenge(_)),
            "At threshold (30%) should still allow Challenge, got {:?}",
            intent
        );

        // 29% - 임계값 이하
        let ctx_below = DefenseContext {
            distance_to_ball_carrier: 2.0,
            stamina_percent: 0.29,
            ..Default::default()
        };
        let intent = select_defense_intent(&ctx_below, &attrs);
        assert!(
            matches!(intent, DefenseIntent::Contain(_)),
            "Below threshold (29%) should force Contain, got {:?}",
            intent
        );
    }

    #[test]
    fn test_stamina_zero_uses_default_behavior() {
        let attrs = default_attrs();

        // stamina_percent = 0.0 (기본값, 정보 없음) - 기존 로직 유지
        let ctx = DefenseContext {
            distance_to_ball_carrier: 2.0,
            stamina_percent: 0.0, // Default (no stamina info)
            ..Default::default()
        };

        let intent = select_defense_intent(&ctx, &attrs);
        assert!(
            matches!(intent, DefenseIntent::Challenge(_)),
            "Zero stamina (default) should use normal behavior, got {:?}",
            intent
        );
    }
}
