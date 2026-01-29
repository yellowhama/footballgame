// 훈련 효과 적용 시스템
use crate::models::player::PlayerAttributes;
use crate::player::types::CorePlayer;
use crate::training::condition::Condition;
use crate::training::types::{
    CoachBonusLog, Injury, InjurySeverity, InjuryType, TrainingResult, TrainingSession,
};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// FIX_2601/0123: Calculate determination bonus for training
///
/// Combines skill determination (Mental attribute) and personality determination
/// to provide a training efficiency multiplier.
///
/// # Returns
/// A multiplier between 0.85 and 1.25:
/// - determination 20 (low): 0.85 (-15%)
/// - determination 50 (average): 1.05 (+5%)
/// - determination 80 (high): 1.17 (+17%)
/// - determination 100 (elite): 1.25 (+25%)
pub fn calculate_determination_bonus(skill_det: u8, personality_det: u8) -> f32 {
    // Combine skill and personality determination (skill weighs more)
    let combined = (skill_det as f32 * 0.7 + personality_det as f32 * 0.3) / 100.0;
    // Map to 0.85-1.25 range
    0.85 + (combined * 0.4)
}

/// 훈련 효과 엔진
pub struct TrainingEffectEngine;

impl TrainingEffectEngine {
    /// 훈련 세션 실행 및 효과 적용
    pub fn execute_training(
        player: &mut CorePlayer,
        session: &TrainingSession,
        condition: Condition,
        seed: u64,
        coach_bonus_log: Vec<CoachBonusLog>,
    ) -> TrainingResult {
        Self::execute_training_with_date(player, session, condition, seed, coach_bonus_log, "")
    }

    /// 훈련 세션 실행 (날짜 포함)
    pub fn execute_training_with_date(
        player: &mut CorePlayer,
        session: &TrainingSession,
        condition: Condition,
        seed: u64,
        coach_bonus_log: Vec<CoachBonusLog>,
        current_date: &str,
    ) -> TrainingResult {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        // 실제 효과 계산
        let base_effect = session.calculate_effect(condition, player.ca as u16, player.pa as u16);

        // FIX_2601/0123: Apply determination bonus from both skill and personality
        let skill_determination = player.detailed_stats.determination;
        let personality_determination = player.personality.determination;
        let determination_bonus = calculate_determination_bonus(skill_determination, personality_determination);

        // FIX_2601/0123: Also apply personality's training efficiency multiplier
        let personality_bonus = player.personality.training_efficiency_multiplier();

        // Final effect = base × determination × personality
        let actual_effect = base_effect * determination_bonus * personality_bonus;

        let ca_before = player.ca;

        // 영향받는 속성들과 가중치 가져오기
        let affected_attributes = session.target.affected_attributes();

        // 속성별 상승치 계산 및 적용
        let mut improved_attributes = Vec::new();

        for (attr_name, weight) in affected_attributes {
            // 기본 성장치 계산
            let base_growth = actual_effect * weight;

            // 랜덤 변동 (±20%)
            let random_factor = rng.gen_range(0.8..=1.2);
            let final_growth = base_growth * random_factor;

            if final_growth >= 0.1 {
                // 실제 속성 증가 시도
                if let Ok(growth_amount) =
                    apply_attribute_increase(player, attr_name, final_growth, &mut rng)
                {
                    if growth_amount > 0.0 {
                        improved_attributes.push((attr_name.to_string(), growth_amount));
                    }
                }
            }
        }

        if !improved_attributes.is_empty() {
            // Keep CA/hexagon stats in sync after attribute changes.
            player.recalculate_all();
        }

        let ca_change = player.ca as f32 - ca_before as f32;

        // 부상 체크 (현재 stamina 속성 사용 + injury_proneness)
        let current_stamina = player.detailed_stats.stamina;
        let base_injury_check = check_injury(&mut rng, current_stamina);

        // injury_proneness 적용 (추가 확률)
        let injury_occurred =
            base_injury_check || (rng.gen::<f32>() < player.injury_proneness * 0.5);

        // 부상 발생시 실제 부상 생성 및 적용
        if injury_occurred && !player.is_injured() {
            let injury = generate_injury(&mut rng, &session.intensity, current_date);
            player.set_current_injury(injury);
        }

        // 메시지 생성
        let message =
            generate_training_message(session, condition, &improved_attributes, injury_occurred);

        TrainingResult {
            session: session.clone(),
            actual_effect,
            improved_attributes,
            ca_change,
            injury_occurred,
            message,
            coach_bonus_log,
        }
    }
}

/// 속성 증가 적용
fn apply_attribute_increase(
    player: &mut CorePlayer,
    attr_name: &str,
    growth: f32,
    rng: &mut ChaCha8Rng,
) -> Result<f32, String> {
    // 현재 속성값 가져오기
    let current_value = get_attribute_value(&player.detailed_stats, attr_name)?;

    // PA 한계 체크
    if player.ca >= player.pa {
        // PA 도달시 최소 성장만
        if rng.gen::<f32>() > 0.1 {
            return Ok(0.0);
        }
    }

    // 속성 한계 (100) 체크
    if current_value >= 95 {
        // 95 이상에서는 성장 매우 어려움
        if rng.gen::<f32>() > 0.05 {
            return Ok(0.0);
        }
    }

    // 실제 증가량 계산
    let increase = if growth >= 1.0 {
        growth.floor()
    } else if rng.gen::<f32>() < growth {
        1.0
    } else {
        0.0
    };

    if increase > 0.0 {
        // 속성 업데이트
        set_attribute_value(
            &mut player.detailed_stats,
            attr_name,
            (current_value + increase as u8).min(100),
        )?;
    }

    Ok(increase)
}

/// 속성값 읽기
fn get_attribute_value(attrs: &PlayerAttributes, name: &str) -> Result<u8, String> {
    match name {
        // Technical (14) - OpenFootball original
        "corners" => Ok(attrs.corners),
        "crossing" => Ok(attrs.crossing),
        "dribbling" => Ok(attrs.dribbling),
        "finishing" => Ok(attrs.finishing),
        "first_touch" => Ok(attrs.first_touch),
        "free_kicks" => Ok(attrs.free_kicks),
        "heading" => Ok(attrs.heading),
        "long_shots" => Ok(attrs.long_shots),
        "long_throws" => Ok(attrs.long_throws),
        "marking" => Ok(attrs.marking),
        "passing" => Ok(attrs.passing),
        "penalty_taking" => Ok(attrs.penalty_taking),
        "tackling" => Ok(attrs.tackling),
        "technique" => Ok(attrs.technique),

        // Mental (14) - OpenFootball original
        "aggression" => Ok(attrs.aggression),
        "anticipation" => Ok(attrs.anticipation),
        "bravery" => Ok(attrs.bravery),
        "composure" => Ok(attrs.composure),
        "concentration" => Ok(attrs.concentration),
        "decisions" => Ok(attrs.decisions),
        "determination" => Ok(attrs.determination),
        "flair" => Ok(attrs.flair),
        "leadership" => Ok(attrs.leadership),
        "off_the_ball" => Ok(attrs.off_the_ball),
        "positioning" => Ok(attrs.positioning),
        "teamwork" => Ok(attrs.teamwork),
        "vision" => Ok(attrs.vision),
        "work_rate" => Ok(attrs.work_rate),

        // Physical (8) - OpenFootball original
        "acceleration" => Ok(attrs.acceleration),
        "agility" => Ok(attrs.agility),
        "balance" => Ok(attrs.balance),
        "jumping" => Ok(attrs.jumping),
        "natural_fitness" => Ok(attrs.natural_fitness),
        "pace" => Ok(attrs.pace),
        "stamina" => Ok(attrs.stamina),
        "strength" => Ok(attrs.strength),

        _ => Err(format!("Unknown attribute: {}", name)),
    }
}

/// 속성값 설정
fn set_attribute_value(attrs: &mut PlayerAttributes, name: &str, value: u8) -> Result<(), String> {
    match name {
        // Technical (14) - OpenFootball original
        "corners" => attrs.corners = value,
        "crossing" => attrs.crossing = value,
        "dribbling" => attrs.dribbling = value,
        "finishing" => attrs.finishing = value,
        "first_touch" => attrs.first_touch = value,
        "free_kicks" => attrs.free_kicks = value,
        "heading" => attrs.heading = value,
        "long_shots" => attrs.long_shots = value,
        "long_throws" => attrs.long_throws = value,
        "marking" => attrs.marking = value,
        "passing" => attrs.passing = value,
        "penalty_taking" => attrs.penalty_taking = value,
        "tackling" => attrs.tackling = value,
        "technique" => attrs.technique = value,

        // Mental (14) - OpenFootball original
        "aggression" => attrs.aggression = value,
        "anticipation" => attrs.anticipation = value,
        "bravery" => attrs.bravery = value,
        "composure" => attrs.composure = value,
        "concentration" => attrs.concentration = value,
        "decisions" => attrs.decisions = value,
        "determination" => attrs.determination = value,
        "flair" => attrs.flair = value,
        "leadership" => attrs.leadership = value,
        "off_the_ball" => attrs.off_the_ball = value,
        "positioning" => attrs.positioning = value,
        "teamwork" => attrs.teamwork = value,
        "vision" => attrs.vision = value,
        "work_rate" => attrs.work_rate = value,

        // Physical (8) - OpenFootball original
        "acceleration" => attrs.acceleration = value,
        "agility" => attrs.agility = value,
        "balance" => attrs.balance = value,
        "jumping" => attrs.jumping = value,
        "natural_fitness" => attrs.natural_fitness = value,
        "pace" => attrs.pace = value,
        "stamina" => attrs.stamina = value,
        "strength" => attrs.strength = value,

        _ => return Err(format!("Unknown attribute: {}", name)),
    }
    Ok(())
}

/// 부상 체크
fn check_injury(rng: &mut ChaCha8Rng, current_stamina: u8) -> bool {
    let injury_risk = match current_stamina {
        40..=100 => 0.01, // 1%
        30..=39 => 0.05,  // 5%
        20..=29 => 0.15,  // 15%
        10..=19 => 0.30,  // 30%
        5..=9 => 0.50,    // 50%
        _ => 0.80,        // 80%
    };

    rng.gen::<f32>() < injury_risk
}

/// 부상 생성 (훈련 중 발생한 부상)
fn generate_injury(
    rng: &mut ChaCha8Rng,
    intensity: &crate::training::stamina::TrainingIntensity,
    current_date: &str,
) -> Injury {
    // 강도에 따른 심각도 결정
    let severity = match intensity {
        crate::training::stamina::TrainingIntensity::Light => {
            if rng.gen::<f32>() < 0.9 {
                InjurySeverity::Minor
            } else {
                InjurySeverity::Moderate
            }
        }
        crate::training::stamina::TrainingIntensity::Normal => {
            let roll = rng.gen::<f32>();
            if roll < 0.6 {
                InjurySeverity::Minor
            } else if roll < 0.9 {
                InjurySeverity::Moderate
            } else {
                InjurySeverity::Serious
            }
        }
        crate::training::stamina::TrainingIntensity::Intensive => {
            let roll = rng.gen::<f32>();
            if roll < 0.3 {
                InjurySeverity::Minor
            } else if roll < 0.7 {
                InjurySeverity::Moderate
            } else if roll < 0.95 {
                InjurySeverity::Serious
            } else {
                InjurySeverity::Severe
            }
        }
        crate::training::stamina::TrainingIntensity::Rest => {
            // 휴식 중에는 경미한 부상만
            InjurySeverity::Minor
        }
    };

    // 부상 유형 결정
    let injury_type = match rng.gen_range(0..5) {
        0 => InjuryType::Muscle,
        1 => InjuryType::Ligament,
        2 => InjuryType::Fatigue,
        3 => InjuryType::Bruise,
        _ => InjuryType::Muscle, // 기본
    };

    // 회복 기간 결정
    let (min_days, max_days) = severity.recovery_range();
    let recovery_days = rng.gen_range(min_days..=max_days);

    // 영향받는 속성
    let affected_attributes =
        injury_type.affected_attributes().iter().map(|s| s.to_string()).collect();

    Injury {
        injury_type,
        severity,
        affected_attributes,
        recovery_days_total: recovery_days,
        recovery_days_remaining: recovery_days,
        occurred_date: current_date.to_string(),
    }
}

/// 훈련 결과 메시지 생성
fn generate_training_message(
    session: &TrainingSession,
    condition: Condition,
    improved_attrs: &[(String, f32)],
    injury_occurred: bool,
) -> String {
    if injury_occurred {
        return format!(
            "⚠️ 훈련 중 부상 발생! {} 훈련을 중단합니다.",
            session.target.display_name()
        );
    }

    if improved_attrs.is_empty() {
        return format!(
            "😔 {} 상태에서 {} 훈련을 했지만 눈에 띄는 성장이 없었습니다.",
            condition.display_text(),
            session.target.display_name()
        );
    }

    let top_attrs: Vec<String> = improved_attrs
        .iter()
        .take(3)
        .map(|(name, growth)| format!("{} +{:.1}", name, growth))
        .collect();

    format!(
        "✨ {} 상태에서 {} 훈련 완료! 성장: {}",
        condition.emoji(),
        session.target.display_name(),
        top_attrs.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::stamina::TrainingIntensity;
    use crate::training::types::{TrainingTarget, TrainingType};

    #[test]
    fn test_training_effect_application() {
        // 테스트용 선수 생성
        let mut player = CorePlayer {
            id: "test-player".to_string(),
            name: "테스트".to_string(),
            age_months: 16.0 * 12.0, // 16세
            ca: 80,
            pa: 120,
            position: crate::models::player::Position::CM,
            detailed_stats: PlayerAttributes::default(),
            hexagon_stats: crate::HexagonCalculator::calculate_all(
                &PlayerAttributes::default(),
                crate::models::player::Position::CM,
            ),
            growth_profile: crate::player::types::GrowthProfile::default(),
            personality: crate::player::personality::PersonAttributes::default(),
            special_abilities: crate::special_ability::SpecialAbilityCollection::new(),
            instructions: Default::default(), // Added missing field
            current_injury: None,
            injury_history: Vec::new(),
            injury_proneness: 0.3,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            career_stats: crate::player::PlayerCareerStats::new(),
        };

        // 훈련 세션 생성
        let session = TrainingSession::new(
            TrainingType::Individual,
            TrainingTarget::Pace,
            TrainingIntensity::Normal,
        );

        // 훈련 실행
        let result = TrainingEffectEngine::execute_training(
            &mut player,
            &session,
            Condition::Normal,
            42,
            Vec::new(),
        );

        // 결과 검증
        assert!(result.actual_effect > 0.0);
        assert!(!result.injury_occurred); // 체력 80에서는 부상 확률 낮음
    }

    #[test]
    fn test_injury_risk() {
        let mut rng = ChaCha8Rng::seed_from_u64(12345);

        // 체력 5에서 테스트 (50% 부상 확률, 5..=9 범위)
        let mut injury_count = 0;
        for _ in 0..100 {
            if check_injury(&mut rng, 5) {
                injury_count += 1;
            }
        }
        assert!(injury_count > 35); // 대략 50% 근처

        // 체력 100에서 테스트 (1% 부상 확률)
        let mut injury_count = 0;
        for _ in 0..1000 {
            if check_injury(&mut rng, 100) {
                injury_count += 1;
            }
        }
        assert!(injury_count < 30); // 대략 1% 근처
    }
}
