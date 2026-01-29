//! Attack-Defense Duel Resolution (P16 Phase 3)
//!
//! 공격 Intent vs 수비 Intent 대결 해결
//! - Base probability matrix
//! - Attribute-based modifiers
//! - Context modifiers
//! - Foul/Card integration

use rand::Rng;

use super::decision_topology::{
    BeatStyle, ProgressStyle, ProtectStyle, ScoreStyle, SelectedIntent,
};
use super::defense_intent::{
    calculate_card_probability, calculate_foul_probability, calculate_press_card_probability,
    calculate_press_foul_probability, ChallengeTechnique, DefenderAttributes, DefenseIntent,
    PressTechnique,
};
use super::rules::{
    check_duel_foul_wrapper, ContactEvent, DefenseIntentInfo, DispatcherContext, LegacyDuelFoulResult,
    RuleCheckMode, RuleDecision, RuleDispatcher, RuleTeamId, TechniqueType,
};
use crate::engine::types::Coord10;

// ============================================================================
// Duel Outcome
// ============================================================================

/// 대결 결과
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuelOutcome {
    /// 공격 성공 (볼 유지/돌파)
    AttackWins,
    /// 수비 성공 (볼 탈취/차단)
    DefenseWins,
    /// 50/50 상황 (볼 루즈)
    Contested,
}

/// 대결 결과 상세
#[derive(Debug, Clone)]
pub struct DuelResult {
    /// 결과 유형
    pub outcome: DuelOutcome,
    /// 볼 소유자 (0: 공격팀, 1: 수비팀, None: 루즈볼)
    pub ball_winner: Option<u8>,
    /// 파울 정보
    pub foul: Option<FoulInfo>,
    /// 부상 정보 (향후 확장)
    pub injury: Option<InjuryInfo>,
    /// 확률 정보 (디버그/리플레이용)
    pub probabilities: DuelProbabilities,
}

/// 파울 정보
#[derive(Debug, Clone, Copy)]
pub struct FoulInfo {
    /// 파울 발생 여부
    pub occurred: bool,
    /// 옐로 카드
    pub yellow_card: bool,
    /// 레드 카드
    pub red_card: bool,
    /// 페널티 킥 (페널티박스 내 파울)
    pub penalty_kick: bool,
    /// 프리킥 위치 (피치 비율 0~1)
    pub free_kick_position: Option<f32>,
}

/// 부상 정보 (향후 확장)
#[derive(Debug, Clone, Copy)]
pub struct InjuryInfo {
    pub severity: InjurySeverity,
    pub affected_player: u8, // 0: attacker, 1: defender
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjurySeverity {
    Minor,
    Moderate,
    Severe,
}

/// 대결 확률 정보 (디버그용)
#[derive(Debug, Clone, Copy, Default)]
pub struct DuelProbabilities {
    pub attack_base: f32,
    pub defense_base: f32,
    pub contested_base: f32,
    pub attack_final: f32,
    pub defense_final: f32,
    pub contested_final: f32,
    pub attack_score: f32,
    pub defense_score: f32,
}

// ============================================================================
// Attacker Attributes
// ============================================================================

/// 공격수 속성 스냅샷
#[derive(Debug, Clone, Copy, Default)]
pub struct AttackerAttributes {
    // Technical
    pub dribbling: f32,
    pub ball_control: f32,
    pub finishing: f32,
    pub passing: f32,

    // Physical
    pub pace: f32,
    pub strength: f32,
    pub agility: f32,
    pub balance: f32,

    // Mental
    pub composure: f32,
    pub vision: f32,
    pub flair: f32,
    pub off_the_ball: f32,
}

impl AttackerAttributes {
    /// Overall 능력치에서 기본 속성 생성
    pub fn from_overall(overall: u8) -> Self {
        let base = overall as f32;
        Self {
            dribbling: base,
            ball_control: base,
            finishing: base,
            passing: base,
            pace: base,
            strength: base,
            agility: base,
            balance: base,
            composure: base,
            vision: base,
            flair: base,
            off_the_ball: base,
        }
    }

    /// Create from Player.attributes (when available)
    /// Converts FM attributes (0-100 scale) to AttackerAttributes (f32)
    pub fn from_player_attributes(attrs: &crate::models::player::PlayerAttributes) -> Self {
        Self {
            dribbling: attrs.dribbling as f32,
            ball_control: ((attrs.first_touch + attrs.technique) / 2) as f32, // Composite
            finishing: attrs.finishing as f32,
            passing: attrs.passing as f32,
            pace: attrs.pace as f32,
            strength: attrs.strength as f32,
            agility: attrs.agility as f32,
            balance: attrs.balance as f32,
            composure: attrs.composure as f32,
            vision: attrs.vision as f32,
            flair: attrs.flair as f32,
            off_the_ball: attrs.off_the_ball as f32,
        }
    }
}

// ============================================================================
// Duel Context
// ============================================================================

/// 대결 컨텍스트
#[derive(Debug, Clone, Default)]
pub struct DuelContext {
    /// 페널티박스 내 여부
    pub in_penalty_box: bool,
    /// 뒤에서 태클 여부
    pub tackle_from_behind: bool,
    /// 득점 기회 저지 (DOGSO)
    pub denied_goal_opportunity: bool,
    /// 피치 위치 (0: 수비진영, 1: 공격진영)
    pub pitch_position: f32,
    /// 수비수가 도움을 받을 수 있는지
    pub defensive_cover: bool,
    /// 공격수 속도 (m/s)
    pub attacker_speed: f32,
    /// 각도 우위 (공격 기준, -1 ~ 1)
    pub angle_advantage: f32,
    /// 볼이 공중에 있는지
    pub ball_in_air: bool,
    /// 습한 피치 여부
    pub wet_pitch: bool,
}

// ============================================================================
// Base Probability Matrix
// ============================================================================

/// Attack Intent vs Defense Intent 기본 확률 매트릭스
/// Returns: (attack_prob, defense_prob, contested_prob)
fn get_base_probabilities(
    attack_intent: &SelectedIntent,
    defense_intent: &DefenseIntent,
) -> (f32, f32, f32) {
    match (attack_intent, defense_intent) {
        // ========================================
        // Protect vs Defense
        // ========================================
        (SelectedIntent::Protect(ProtectStyle::Shield), DefenseIntent::Contain(_)) => {
            (0.75, 0.10, 0.15)
        }
        (SelectedIntent::Protect(ProtectStyle::Shield), DefenseIntent::Press(_)) => {
            (0.55, 0.25, 0.20)
        }
        (SelectedIntent::Protect(ProtectStyle::Shield), DefenseIntent::Challenge(_)) => {
            (0.35, 0.40, 0.25)
        }

        (SelectedIntent::Protect(ProtectStyle::HoldUp), DefenseIntent::Contain(_)) => {
            (0.70, 0.10, 0.20)
        }
        (SelectedIntent::Protect(ProtectStyle::HoldUp), DefenseIntent::Press(_)) => {
            (0.50, 0.25, 0.25)
        }
        (SelectedIntent::Protect(ProtectStyle::HoldUp), DefenseIntent::Challenge(_)) => {
            (0.30, 0.35, 0.35)
        }

        (SelectedIntent::Protect(ProtectStyle::DrawFoul), DefenseIntent::Contain(_)) => {
            (0.30, 0.20, 0.50)
        } // 높은 Contested = 애매한 상황
        (SelectedIntent::Protect(ProtectStyle::DrawFoul), DefenseIntent::Press(_)) => {
            (0.35, 0.30, 0.35)
        }
        (SelectedIntent::Protect(ProtectStyle::DrawFoul), DefenseIntent::Challenge(_)) => {
            (0.40, 0.35, 0.25)
        } // Challenge 시 파울 유도 쉬움

        // ========================================
        // Progress vs Defense
        // ========================================
        (SelectedIntent::Progress(ProgressStyle::Safe), DefenseIntent::Contain(_)) => {
            (0.80, 0.08, 0.12)
        }
        (SelectedIntent::Progress(ProgressStyle::Safe), DefenseIntent::Press(_)) => {
            (0.60, 0.25, 0.15)
        }
        (SelectedIntent::Progress(ProgressStyle::Safe), DefenseIntent::Challenge(_)) => {
            (0.50, 0.30, 0.20)
        }

        (SelectedIntent::Progress(ProgressStyle::Progressive), DefenseIntent::Contain(_)) => {
            (0.65, 0.15, 0.20)
        }
        (SelectedIntent::Progress(ProgressStyle::Progressive), DefenseIntent::Press(_)) => {
            (0.45, 0.30, 0.25)
        }
        (SelectedIntent::Progress(ProgressStyle::Progressive), DefenseIntent::Challenge(_)) => {
            (0.40, 0.35, 0.25)
        }

        (SelectedIntent::Progress(ProgressStyle::Switch), DefenseIntent::Contain(_)) => {
            (0.75, 0.10, 0.15)
        }
        (SelectedIntent::Progress(ProgressStyle::Switch), DefenseIntent::Press(_)) => {
            (0.55, 0.25, 0.20)
        }
        (SelectedIntent::Progress(ProgressStyle::Switch), DefenseIntent::Challenge(_)) => {
            (0.45, 0.30, 0.25)
        }

        (SelectedIntent::Progress(ProgressStyle::Carry), DefenseIntent::Contain(_)) => {
            (0.70, 0.12, 0.18)
        }
        (SelectedIntent::Progress(ProgressStyle::Carry), DefenseIntent::Press(_)) => {
            (0.50, 0.28, 0.22)
        }
        (SelectedIntent::Progress(ProgressStyle::Carry), DefenseIntent::Challenge(_)) => {
            (0.40, 0.38, 0.22)
        }

        // ========================================
        // Beat vs Defense
        // ========================================
        (SelectedIntent::Beat(BeatStyle::TakeOn), DefenseIntent::Contain(_)) => (0.75, 0.10, 0.15), // Contain은 TakeOn에 약함
        (SelectedIntent::Beat(BeatStyle::TakeOn), DefenseIntent::Press(_)) => (0.55, 0.25, 0.20),
        (SelectedIntent::Beat(BeatStyle::TakeOn), DefenseIntent::Challenge(_)) => {
            (0.40, 0.40, 0.20)
        } // Challenge는 50/50

        (SelectedIntent::Beat(BeatStyle::OneTwo), DefenseIntent::Contain(_)) => (0.70, 0.12, 0.18),
        (SelectedIntent::Beat(BeatStyle::OneTwo), DefenseIntent::Press(_)) => (0.50, 0.30, 0.20),
        (SelectedIntent::Beat(BeatStyle::OneTwo), DefenseIntent::Challenge(_)) => {
            (0.45, 0.35, 0.20)
        }

        (SelectedIntent::Beat(BeatStyle::Through), DefenseIntent::Contain(_)) => (0.55, 0.20, 0.25), // Through ball은 리스크 있음
        (SelectedIntent::Beat(BeatStyle::Through), DefenseIntent::Press(_)) => (0.40, 0.35, 0.25), // Press가 Through ball 차단에 효과적
        (SelectedIntent::Beat(BeatStyle::Through), DefenseIntent::Challenge(_)) => {
            (0.45, 0.35, 0.20)
        }

        // ========================================
        // Score vs Defense (슛 시도)
        // ========================================
        (SelectedIntent::Score(_), DefenseIntent::Contain(_)) => (0.70, 0.15, 0.15), // 슛은 대부분 성공 (GK가 막음)
        (SelectedIntent::Score(_), DefenseIntent::Press(_)) => (0.55, 0.25, 0.20),   // 압박 시 방해
        (SelectedIntent::Score(_), DefenseIntent::Challenge(_)) => (0.45, 0.35, 0.20), // 블록 가능성

        // ========================================
        // Defense Intents (수비수가 공격하는 경우는 없음, fallback)
        // ========================================
        (SelectedIntent::Contain(_), _)
        | (SelectedIntent::Press(_), _)
        | (SelectedIntent::Challenge(_), _) => (0.50, 0.30, 0.20),

        // ========================================
        // Transition & Special
        // ========================================
        (SelectedIntent::CounterAttack, DefenseIntent::Contain(_)) => (0.65, 0.15, 0.20),
        (SelectedIntent::CounterAttack, DefenseIntent::Press(_)) => (0.50, 0.30, 0.20),
        (SelectedIntent::CounterAttack, DefenseIntent::Challenge(_)) => (0.45, 0.35, 0.20),

        (SelectedIntent::Recovery, _) => (0.30, 0.50, 0.20), // Recovery는 수비 유리

        (SelectedIntent::Clear, _) => (0.75, 0.15, 0.10), // Clear는 대부분 성공

        (SelectedIntent::Wait, _) => (0.40, 0.40, 0.20), // 대기는 50/50
    }
}

// ============================================================================
// Attribute Score Calculation
// ============================================================================

/// 공격 대결 점수 계산 (Intent 기반)
fn calculate_attack_duel_score(
    attack_intent: &SelectedIntent,
    attacker: &AttackerAttributes,
) -> f32 {
    match attack_intent {
        SelectedIntent::Protect(ProtectStyle::Shield) => {
            attacker.strength * 0.40 + attacker.balance * 0.30 + attacker.ball_control * 0.30
        }
        SelectedIntent::Protect(ProtectStyle::HoldUp) => {
            attacker.strength * 0.30 + attacker.ball_control * 0.35 + attacker.composure * 0.35
        }
        SelectedIntent::Protect(ProtectStyle::DrawFoul) => {
            attacker.flair * 0.40 + attacker.agility * 0.30 + attacker.balance * 0.30
        }

        SelectedIntent::Progress(ProgressStyle::Safe) => {
            attacker.passing * 0.50 + attacker.vision * 0.30 + attacker.composure * 0.20
        }
        SelectedIntent::Progress(ProgressStyle::Progressive) => {
            attacker.passing * 0.40 + attacker.vision * 0.35 + attacker.ball_control * 0.25
        }
        SelectedIntent::Progress(ProgressStyle::Switch) => {
            attacker.passing * 0.50 + attacker.vision * 0.40 + attacker.composure * 0.10
        }
        SelectedIntent::Progress(ProgressStyle::Carry) => {
            attacker.dribbling * 0.35 + attacker.ball_control * 0.35 + attacker.pace * 0.30
        }

        SelectedIntent::Beat(BeatStyle::TakeOn) => {
            attacker.dribbling * 0.40 + attacker.agility * 0.30 + attacker.flair * 0.30
        }
        SelectedIntent::Beat(BeatStyle::OneTwo) => {
            attacker.passing * 0.35 + attacker.off_the_ball * 0.35 + attacker.vision * 0.30
        }
        SelectedIntent::Beat(BeatStyle::Through) => {
            attacker.passing * 0.40 + attacker.vision * 0.40 + attacker.composure * 0.20
        }

        SelectedIntent::Score(ScoreStyle::Normal) => {
            attacker.finishing * 0.50 + attacker.composure * 0.30 + attacker.ball_control * 0.20
        }
        SelectedIntent::Score(ScoreStyle::Finesse) => {
            attacker.finishing * 0.40 + attacker.flair * 0.35 + attacker.composure * 0.25
        }
        SelectedIntent::Score(ScoreStyle::Power) => {
            attacker.finishing * 0.40 + attacker.strength * 0.35 + attacker.balance * 0.25
        }
        SelectedIntent::Score(ScoreStyle::Chip) => {
            attacker.finishing * 0.30 + attacker.flair * 0.40 + attacker.composure * 0.30
        }
        SelectedIntent::Score(ScoreStyle::Header) => {
            attacker.finishing * 0.35 + attacker.strength * 0.35 + attacker.balance * 0.30
        }

        SelectedIntent::CounterAttack => {
            attacker.pace * 0.40 + attacker.off_the_ball * 0.35 + attacker.composure * 0.25
        }

        // 수비 Intent (공격자가 선택 안함, fallback)
        _ => attacker.ball_control * 0.50 + attacker.composure * 0.50,
    }
}

/// 수비 대결 점수 계산 (Intent 기반)
fn calculate_defense_duel_score(
    defense_intent: &DefenseIntent,
    defender: &DefenderAttributes,
) -> f32 {
    match defense_intent {
        DefenseIntent::Contain(_) => {
            defender.positioning * 0.35
                + defender.anticipation * 0.30
                + defender.concentration * 0.20
                + defender.agility * 0.15
        }
        DefenseIntent::Press(_) => {
            defender.work_rate * 0.30
                + defender.aggression * 0.25
                + defender.pace * 0.25
                + defender.stamina * 0.20
        }
        DefenseIntent::Challenge(_) => {
            defender.tackling * 0.35
                + defender.strength * 0.25
                + defender.bravery * 0.20
                + defender.reactions * 0.20
        }
    }
}

// ============================================================================
// Context Modifiers
// ============================================================================

/// 컨텍스트 기반 수정자 계산 (공격 기준, 양수면 공격 유리)
fn calculate_context_modifier(ctx: &DuelContext) -> f32 {
    let mut modifier = 0.0;

    // 피치 위치 (공격 진영일수록 공격 유리)
    modifier += (ctx.pitch_position - 0.5) * 0.1;

    // 각도 우위
    modifier += ctx.angle_advantage * 0.1;

    // 속도 보너스
    if ctx.attacker_speed > 7.0 {
        modifier += 0.05;
    }

    // 수비 커버 있으면 수비 유리
    if ctx.defensive_cover {
        modifier -= 0.1;
    }

    // 공중볼은 다른 역학
    if ctx.ball_in_air {
        modifier -= 0.05; // 약간 수비 유리
    }

    // 습한 피치는 불확실성 증가 (여기서는 약간 수비 유리)
    if ctx.wet_pitch {
        modifier -= 0.03;
    }

    modifier.clamp(-0.25, 0.25)
}

// ============================================================================
// Main Duel Resolution
// ============================================================================

/// Attack Intent vs Defense Intent 대결 해결
pub fn resolve_attack_defense_duel(
    attack_intent: &SelectedIntent,
    defense_intent: &DefenseIntent,
    attacker: &AttackerAttributes,
    defender: &DefenderAttributes,
    ctx: &DuelContext,
    rng: &mut impl Rng,
) -> DuelResult {
    // 1. 기본 확률 가져오기
    let (attack_base, defense_base, contested_base) =
        get_base_probabilities(attack_intent, defense_intent);

    // 2. 속성 점수 계산
    let attack_score = calculate_attack_duel_score(attack_intent, attacker);
    let defense_score = calculate_defense_duel_score(defense_intent, defender);

    // 점수 차이를 확률 수정자로 변환 (-30 ~ +30 범위를 -0.15 ~ +0.15로)
    let score_diff = (attack_score - defense_score) / 200.0;

    // 3. 컨텍스트 수정자
    let context_mod = calculate_context_modifier(ctx);

    // 4. 최종 확률 계산 (정규화)
    let attack_raw = (attack_base + score_diff * 0.3 + context_mod).max(0.05);
    let defense_raw = (defense_base - score_diff * 0.3 - context_mod).max(0.05);
    let contested_raw = contested_base.max(0.05);

    let total = attack_raw + defense_raw + contested_raw;
    let attack_prob = (attack_raw / total).clamp(0.05, 0.90);
    let defense_prob = (defense_raw / total).clamp(0.05, 0.90);
    let contested_prob = 1.0 - attack_prob - defense_prob;

    // 5. 결과 결정
    let roll = rng.gen::<f32>();
    let outcome = if roll < attack_prob {
        DuelOutcome::AttackWins
    } else if roll < attack_prob + defense_prob {
        DuelOutcome::DefenseWins
    } else {
        DuelOutcome::Contested
    };

    // 6. 볼 소유자 결정
    let ball_winner = match outcome {
        DuelOutcome::AttackWins => Some(0),  // 공격팀
        DuelOutcome::DefenseWins => Some(1), // 수비팀
        DuelOutcome::Contested => None,      // 루즈볼
    };

    // 7. 파울 체크 (Challenge만)
    let foul = check_for_foul(defense_intent, &outcome, defender, ctx, rng);

    // 8. 결과 반환
    DuelResult {
        outcome,
        ball_winner,
        foul,
        injury: None, // 향후 확장
        probabilities: DuelProbabilities {
            attack_base,
            defense_base,
            contested_base,
            attack_final: attack_prob,
            defense_final: defense_prob,
            contested_final: contested_prob,
            attack_score,
            defense_score,
        },
    }
}

/// 파울 체크
/// FIX_2601/0109: Press에서도 파울 발생 가능 (실제 축구에서 압박 중 파울 빈번)
fn check_for_foul(
    defense_intent: &DefenseIntent,
    outcome: &DuelOutcome,
    defender: &DefenderAttributes,
    ctx: &DuelContext,
    rng: &mut impl Rng,
) -> Option<FoulInfo> {
    let defense_won = matches!(outcome, DuelOutcome::DefenseWins);

    match defense_intent {
        // Challenge: 기존 로직 (높은 파울 확률)
        DefenseIntent::Challenge(tech) => {
            check_challenge_foul(*tech, defense_won, defender, ctx, rng)
        }
        // Press: 신규 로직 (낮은 파울 확률, FIX_2601/0109)
        DefenseIntent::Press(tech) => check_press_foul(*tech, defense_won, defender, ctx, rng),
        // Contain: 파울 없음 (거리 유지, 접촉 없음)
        DefenseIntent::Contain(_) => None,
    }
}

/// Challenge 파울 체크 (기존 로직)
fn check_challenge_foul(
    technique: super::defense_intent::ChallengeTechnique,
    defense_won: bool,
    defender: &DefenderAttributes,
    ctx: &DuelContext,
    rng: &mut impl Rng,
) -> Option<FoulInfo> {
    // 파울 확률
    let foul_prob = calculate_foul_probability(technique, defender, defense_won);

    if rng.gen::<f32>() >= foul_prob {
        return None;
    }

    // 카드 확률
    let (yellow_prob, red_prob) = calculate_card_probability(
        technique,
        defender,
        ctx.in_penalty_box,
        ctx.tackle_from_behind,
        ctx.denied_goal_opportunity,
    );

    let yellow_card = rng.gen::<f32>() < yellow_prob;
    let red_card = rng.gen::<f32>() < red_prob;

    Some(FoulInfo {
        occurred: true,
        yellow_card,
        red_card,
        penalty_kick: ctx.in_penalty_box,
        free_kick_position: if ctx.in_penalty_box { None } else { Some(ctx.pitch_position) },
    })
}

/// Press 파울 체크 (FIX_2601/0109 신규)
fn check_press_foul(
    technique: PressTechnique,
    defense_won: bool,
    defender: &DefenderAttributes,
    ctx: &DuelContext,
    rng: &mut impl Rng,
) -> Option<FoulInfo> {
    // 파울 확률 (Press는 Challenge보다 낮음)
    let foul_prob = calculate_press_foul_probability(technique, defender, defense_won);

    if rng.gen::<f32>() >= foul_prob {
        return None;
    }

    // 카드 확률 (Press는 Challenge보다 낮음)
    let (yellow_prob, red_prob) =
        calculate_press_card_probability(technique, defender, ctx.in_penalty_box);

    let yellow_card = rng.gen::<f32>() < yellow_prob;
    let red_card = rng.gen::<f32>() < red_prob;

    Some(FoulInfo {
        occurred: true,
        yellow_card,
        red_card,
        penalty_kick: ctx.in_penalty_box,
        free_kick_position: if ctx.in_penalty_box { None } else { Some(ctx.pitch_position) },
    })
}

// ============================================================================
// FIX_2601/0123 Phase 6: Dispatcher Integration
// ============================================================================

/// Convert ChallengeTechnique to TechniqueType
fn to_rule_technique(challenge: ChallengeTechnique) -> TechniqueType {
    match challenge {
        ChallengeTechnique::StandingTackle => TechniqueType::StandingTackle,
        ChallengeTechnique::SlidingTackle => TechniqueType::SlidingTackle,
        ChallengeTechnique::ShoulderCharge => TechniqueType::ShoulderCharge,
        ChallengeTechnique::PokeAway => TechniqueType::PokeAway,
    }
}

/// Convert PressTechnique to TechniqueType
fn press_to_rule_technique(press: PressTechnique) -> TechniqueType {
    match press {
        PressTechnique::ClosingDown => TechniqueType::ClosingDown,
        PressTechnique::InterceptAttempt => TechniqueType::InterceptAttempt,
        PressTechnique::ForceTouchline => TechniqueType::ForceTouchline,
        PressTechnique::TrackRunner => TechniqueType::TrackRunner,
    }
}

/// Check for foul with dispatcher integration
///
/// This function is the integration point between duel.rs and the RuleDispatcher.
/// It supports A/B comparison mode and can optionally use the dispatcher for decisions.
///
/// # Arguments
/// * `defense_intent` - Defense intent from duel resolution
/// * `outcome` - Duel outcome
/// * `defender` - Defender attributes
/// * `defender_idx` - Defender player index (0-21)
/// * `attacker_idx` - Attacker player index (0-21)
/// * `ctx` - Duel context
/// * `position` - Contact position (Coord10)
/// * `dispatcher` - Optional RuleDispatcher for A/B comparison
/// * `mode` - Rule check mode
/// * `rng` - RNG for legacy calculation
///
/// # Returns
/// FoulInfo if a foul occurred, None otherwise
pub fn check_for_foul_with_dispatcher(
    defense_intent: &DefenseIntent,
    outcome: &DuelOutcome,
    defender: &DefenderAttributes,
    defender_idx: usize,
    attacker_idx: usize,
    ctx: &DuelContext,
    position: Coord10,
    dispatcher: Option<&mut RuleDispatcher>,
    mode: RuleCheckMode,
    rng: &mut impl Rng,
) -> Option<FoulInfo> {
    let defense_won = matches!(outcome, DuelOutcome::DefenseWins);

    // First, get the legacy result
    let legacy_foul = match defense_intent {
        DefenseIntent::Challenge(tech) => {
            check_challenge_foul(*tech, defense_won, defender, ctx, rng)
        }
        DefenseIntent::Press(tech) => {
            check_press_foul(*tech, defense_won, defender, ctx, rng)
        }
        DefenseIntent::Contain(_) => None,
    };

    // If no dispatcher, just return legacy result
    let dispatcher = match dispatcher {
        Some(d) => d,
        None => return legacy_foul,
    };

    // Create intent info for dispatcher
    let (intent_type, technique) = match defense_intent {
        DefenseIntent::Challenge(tech) => {
            (super::rules::DefenseIntentType::Challenge, to_rule_technique(*tech))
        }
        DefenseIntent::Press(tech) => {
            (super::rules::DefenseIntentType::Press, press_to_rule_technique(*tech))
        }
        DefenseIntent::Contain(_) => {
            // Contain doesn't cause fouls
            return legacy_foul;
        }
    };

    let intent_info = DefenseIntentInfo {
        intent_type,
        technique,
        defense_won,
        defender_tackling: defender.tackling,
        defender_aggression: defender.aggression,
    };

    // Create contact event
    let contact_event = ContactEvent {
        tackler_idx: defender_idx,
        ball_carrier_idx: attacker_idx,
        position: position.clone(),
        contact_angle: 0.0, // Not tracked in duel system
        ball_won_first: defense_won,
        intensity: defender.aggression / 100.0,
    };

    // Create legacy result for comparison
    let legacy_result = LegacyDuelFoulResult {
        occurred: legacy_foul.as_ref().map_or(false, |f| f.occurred),
        yellow_card: legacy_foul.as_ref().map_or(false, |f| f.yellow_card),
        red_card: legacy_foul.as_ref().map_or(false, |f| f.red_card),
        penalty_kick: legacy_foul.as_ref().map_or(false, |f| f.penalty_kick),
        free_kick_position: legacy_foul.as_ref().and_then(|f| f.free_kick_position),
        defender_idx,
        attacker_idx,
        position: position.clone(),
    };

    let last_touch_team = RuleTeamId::from_player_index(attacker_idx);
    let rng_roll = rng.gen::<f32>();

    // Call the wrapper for A/B comparison
    let decision = check_duel_foul_wrapper(
        mode,
        dispatcher,
        &legacy_result,
        &intent_info,
        &contact_event,
        &position,
        last_touch_team,
        rng_roll,
    );

    // If dispatcher applies, convert RuleDecision back to FoulInfo
    if mode.dispatcher_applies() {
        match decision {
            Some(RuleDecision::Foul { foul_type, card, .. }) => {
                let is_penalty = matches!(foul_type, super::rules::FoulType::Penalty);
                Some(FoulInfo {
                    occurred: true,
                    yellow_card: matches!(card, Some(super::rules::Card::Yellow)),
                    red_card: matches!(card, Some(super::rules::Card::Red)),
                    penalty_kick: is_penalty,
                    free_kick_position: if is_penalty { None } else { Some(ctx.pitch_position) },
                })
            }
            _ => None,
        }
    } else {
        legacy_foul
    }
}

// ============================================================================
// Simplified Duel (Overall 기반)
// ============================================================================

/// 간단한 대결 해결 (Overall 능력치만 사용)
pub fn resolve_duel_simple(
    attack_intent: &SelectedIntent,
    defense_intent: &DefenseIntent,
    attacker_overall: u8,
    defender_overall: u8,
    in_penalty_box: bool,
    rng: &mut impl Rng,
) -> DuelResult {
    let attacker = AttackerAttributes::from_overall(attacker_overall);
    let defender = DefenderAttributes::from_overall(defender_overall);
    let ctx = DuelContext { in_penalty_box, pitch_position: 0.5, ..Default::default() };

    resolve_attack_defense_duel(attack_intent, defense_intent, &attacker, &defender, &ctx, rng)
}

// ============================================================================
// FIX_2601/0102: Tackle Score Calculation
// ============================================================================

/// 태클 점수 계산 (0.0 ~ 1.0)
///
/// Open-Football 방식의 볼 탈취 능력 점수.
/// 루즈볼 경쟁 시 더 높은 점수의 선수가 소유권 획득.
///
/// ## 가중치
/// - tackling: 40% (핵심 태클 능력)
/// - aggression: 20% (공격적 볼 쟁탈)
/// - strength: 20% (피지컬 우위)
/// - bravery: 10% (과감한 도전)
/// - agility: 10% (민첩한 반응)
///
/// ## Example
/// ```ignore
/// let score = calculate_tackle_score(attrs);
/// // 80 tackling, 70 aggression, 75 strength, 60 bravery, 70 agility
/// // = 0.8*0.4 + 0.7*0.2 + 0.75*0.2 + 0.6*0.1 + 0.7*0.1
/// // = 0.32 + 0.14 + 0.15 + 0.06 + 0.07 = 0.74
/// ```
pub fn calculate_tackle_score(attrs: &crate::models::player::PlayerAttributes) -> f32 {
    let tackling = attrs.tackling as f32 / 100.0;
    let aggression = attrs.aggression as f32 / 100.0;
    let strength = attrs.strength as f32 / 100.0;
    let bravery = attrs.bravery as f32 / 100.0;
    let agility = attrs.agility as f32 / 100.0;

    tackling * 0.4 + aggression * 0.2 + strength * 0.2 + bravery * 0.1 + agility * 0.1
}

/// DefenderAttributes에서 태클 점수 계산
///
/// DefenderAttributes는 이미 0.0~1.0 정규화된 값을 가짐.
pub fn calculate_tackle_score_from_defender(attrs: &DefenderAttributes) -> f32 {
    attrs.tackling * 0.4
        + attrs.aggression * 0.2
        + attrs.strength * 0.2
        + attrs.bravery * 0.1
        + attrs.agility * 0.1
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::defense_intent::ChallengeTechnique;
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn default_attacker() -> AttackerAttributes {
        AttackerAttributes::from_overall(75)
    }

    fn default_defender() -> DefenderAttributes {
        DefenderAttributes::from_overall(75)
    }

    fn default_ctx() -> DuelContext {
        DuelContext::default()
    }

    #[test]
    fn test_base_probabilities_sum_to_one() {
        let attack_intents = [
            SelectedIntent::Protect(ProtectStyle::Shield),
            SelectedIntent::Progress(ProgressStyle::Safe),
            SelectedIntent::Beat(BeatStyle::TakeOn),
            SelectedIntent::Score(ScoreStyle::Normal),
        ];

        let defense_intents = [
            DefenseIntent::Contain(super::super::defense_intent::ContainTechnique::Jockey),
            DefenseIntent::Press(super::super::defense_intent::PressTechnique::ClosingDown),
            DefenseIntent::Challenge(ChallengeTechnique::StandingTackle),
        ];

        for attack in &attack_intents {
            for defense in &defense_intents {
                let (a, d, c) = get_base_probabilities(attack, defense);
                let sum = a + d + c;
                assert!(
                    (sum - 1.0).abs() < 0.01,
                    "Probabilities for {:?} vs {:?} sum to {}, not 1.0",
                    attack,
                    defense,
                    sum
                );
            }
        }
    }

    #[test]
    fn test_resolve_duel_produces_valid_outcome() {
        let mut rng = test_rng();
        let attacker = default_attacker();
        let defender = default_defender();
        let ctx = default_ctx();

        for _ in 0..100 {
            let result = resolve_attack_defense_duel(
                &SelectedIntent::Beat(BeatStyle::TakeOn),
                &DefenseIntent::Challenge(ChallengeTechnique::StandingTackle),
                &attacker,
                &defender,
                &ctx,
                &mut rng,
            );

            // 결과가 유효한지 확인
            assert!(matches!(
                result.outcome,
                DuelOutcome::AttackWins | DuelOutcome::DefenseWins | DuelOutcome::Contested
            ));

            // 확률이 유효 범위인지
            assert!(result.probabilities.attack_final >= 0.0);
            assert!(result.probabilities.defense_final >= 0.0);
            assert!(result.probabilities.contested_final >= 0.0);
        }
    }

    #[test]
    fn test_protect_vs_challenge_probabilities() {
        let (attack, defense, _contested) = get_base_probabilities(
            &SelectedIntent::Protect(ProtectStyle::Shield),
            &DefenseIntent::Challenge(ChallengeTechnique::StandingTackle),
        );

        // Shield vs Challenge: 수비가 유리해야 함
        assert!(
            defense >= attack,
            "Challenge should beat Shield: attack={}, defense={}",
            attack,
            defense
        );
    }

    #[test]
    fn test_takeon_vs_contain_probabilities() {
        let (attack, defense, _) = get_base_probabilities(
            &SelectedIntent::Beat(BeatStyle::TakeOn),
            &DefenseIntent::Contain(super::super::defense_intent::ContainTechnique::Jockey),
        );

        // TakeOn vs Contain: 공격이 유리해야 함
        assert!(
            attack > defense,
            "TakeOn should beat Contain: attack={}, defense={}",
            attack,
            defense
        );
    }

    #[test]
    fn test_high_attacker_skill_increases_win_rate() {
        let mut rng = test_rng();

        let low_skill = AttackerAttributes::from_overall(50);
        let high_skill = AttackerAttributes::from_overall(90);
        let defender = default_defender();
        let ctx = default_ctx();

        let attack_intent = SelectedIntent::Beat(BeatStyle::TakeOn);
        let defense_intent = DefenseIntent::Challenge(ChallengeTechnique::StandingTackle);

        let mut low_wins = 0;
        let mut high_wins = 0;
        let trials = 1000;

        for _ in 0..trials {
            let result_low = resolve_attack_defense_duel(
                &attack_intent,
                &defense_intent,
                &low_skill,
                &defender,
                &ctx,
                &mut rng,
            );
            if matches!(result_low.outcome, DuelOutcome::AttackWins) {
                low_wins += 1;
            }

            let result_high = resolve_attack_defense_duel(
                &attack_intent,
                &defense_intent,
                &high_skill,
                &defender,
                &ctx,
                &mut rng,
            );
            if matches!(result_high.outcome, DuelOutcome::AttackWins) {
                high_wins += 1;
            }
        }

        assert!(
            high_wins > low_wins,
            "High skill attacker should win more: high={}, low={}",
            high_wins,
            low_wins
        );
    }

    #[test]
    fn test_foul_occurs_on_challenge() {
        let mut rng = test_rng();
        let attacker = default_attacker();
        let defender = default_defender();
        let ctx = default_ctx();

        let mut foul_count = 0;
        let trials = 1000;

        for _ in 0..trials {
            let result = resolve_attack_defense_duel(
                &SelectedIntent::Beat(BeatStyle::TakeOn),
                &DefenseIntent::Challenge(ChallengeTechnique::SlidingTackle),
                &attacker,
                &defender,
                &ctx,
                &mut rng,
            );

            if result.foul.is_some() {
                foul_count += 1;
            }
        }

        // 슬라이딩 태클은 파울이 발생해야 함 (기본 25%)
        assert!(
            foul_count > 100 && foul_count < 500,
            "Sliding tackle should produce fouls: {}/{}",
            foul_count,
            trials
        );
    }

    #[test]
    fn test_penalty_kick_on_box_foul() {
        let mut rng = test_rng();
        let attacker = default_attacker();
        let defender = default_defender();
        let ctx = DuelContext { in_penalty_box: true, ..default_ctx() };

        let mut pk_count = 0;
        let trials = 1000;

        for _ in 0..trials {
            let result = resolve_attack_defense_duel(
                &SelectedIntent::Beat(BeatStyle::TakeOn),
                &DefenseIntent::Challenge(ChallengeTechnique::SlidingTackle),
                &attacker,
                &defender,
                &ctx,
                &mut rng,
            );

            if let Some(foul) = result.foul {
                if foul.penalty_kick {
                    pk_count += 1;
                }
            }
        }

        // 페널티박스 내 파울은 PK가 되어야 함
        assert!(pk_count > 50, "Box fouls should result in PKs: {}/{}", pk_count, trials);
    }

    #[test]
    fn test_context_modifier_range() {
        let ctx = DuelContext {
            pitch_position: 1.0,  // 최대 공격 위치
            angle_advantage: 1.0, // 최대 각도 우위
            attacker_speed: 10.0, // 빠름
            defensive_cover: false,
            ball_in_air: false,
            wet_pitch: false,
            ..default_ctx()
        };

        let modifier = calculate_context_modifier(&ctx);
        assert!(modifier <= 0.25, "Context modifier should be clamped: {}", modifier);

        let ctx_bad = DuelContext {
            pitch_position: 0.0,
            angle_advantage: -1.0,
            defensive_cover: true,
            ball_in_air: true,
            wet_pitch: true,
            ..default_ctx()
        };

        let modifier_bad = calculate_context_modifier(&ctx_bad);
        assert!(modifier_bad >= -0.25, "Context modifier should be clamped: {}", modifier_bad);
    }

    #[test]
    fn test_simple_duel() {
        let mut rng = test_rng();

        let result = resolve_duel_simple(
            &SelectedIntent::Score(ScoreStyle::Normal),
            &DefenseIntent::Challenge(ChallengeTechnique::StandingTackle),
            80, // attacker overall
            75, // defender overall
            false,
            &mut rng,
        );

        // 결과가 유효한지 확인
        assert!(matches!(
            result.outcome,
            DuelOutcome::AttackWins | DuelOutcome::DefenseWins | DuelOutcome::Contested
        ));
    }

    #[test]
    fn test_ball_winner_assignment() {
        let mut rng = test_rng();
        let attacker = default_attacker();
        let defender = default_defender();
        let ctx = default_ctx();

        for _ in 0..100 {
            let result = resolve_attack_defense_duel(
                &SelectedIntent::Progress(ProgressStyle::Carry),
                &DefenseIntent::Press(super::super::defense_intent::PressTechnique::ClosingDown),
                &attacker,
                &defender,
                &ctx,
                &mut rng,
            );

            match result.outcome {
                DuelOutcome::AttackWins => {
                    assert_eq!(
                        result.ball_winner,
                        Some(0),
                        "Attack win should give ball to attacker"
                    );
                }
                DuelOutcome::DefenseWins => {
                    assert_eq!(
                        result.ball_winner,
                        Some(1),
                        "Defense win should give ball to defender"
                    );
                }
                DuelOutcome::Contested => {
                    assert_eq!(result.ball_winner, None, "Contested should be loose ball");
                }
            }
        }
    }

    // ========== FIX_2601/0102: Tackle Score Tests ==========

    #[test]
    fn test_tackle_score_calculation() {
        use crate::models::player::PlayerAttributes;

        // 완벽한 태클러: 모든 관련 스탯 100
        let perfect = PlayerAttributes {
            tackling: 100,
            aggression: 100,
            strength: 100,
            bravery: 100,
            agility: 100,
            ..Default::default()
        };
        let score = super::calculate_tackle_score(&perfect);
        assert!((score - 1.0).abs() < 0.01, "Perfect tackling score should be 1.0, got {}", score);

        // 평균 선수: 모든 관련 스탯 50
        let average = PlayerAttributes {
            tackling: 50,
            aggression: 50,
            strength: 50,
            bravery: 50,
            agility: 50,
            ..Default::default()
        };
        let score = super::calculate_tackle_score(&average);
        assert!((score - 0.5).abs() < 0.01, "Average tackling score should be 0.5, got {}", score);

        // 가중치 검증: tackling이 40%로 가장 중요
        let high_tackling = PlayerAttributes {
            tackling: 100,
            aggression: 0,
            strength: 0,
            bravery: 0,
            agility: 0,
            ..Default::default()
        };
        let score = super::calculate_tackle_score(&high_tackling);
        assert!((score - 0.4).abs() < 0.01, "Tackling-only score should be 0.4, got {}", score);
    }

    #[test]
    fn test_tackle_score_from_defender_attrs() {
        let defender = DefenderAttributes {
            tackling: 0.8,
            aggression: 0.7,
            strength: 0.75,
            bravery: 0.6,
            agility: 0.7,
            ..Default::default()
        };

        let score = super::calculate_tackle_score_from_defender(&defender);
        // 0.8*0.4 + 0.7*0.2 + 0.75*0.2 + 0.6*0.1 + 0.7*0.1
        // = 0.32 + 0.14 + 0.15 + 0.06 + 0.07 = 0.74
        assert!((score - 0.74).abs() < 0.01, "Score should be 0.74, got {}", score);
    }
}
