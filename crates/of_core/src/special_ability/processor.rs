use crate::special_ability::{
    AbilityActivationContext, AbilityCombinationEngine, AbilityEffectCalculator, CombinationResult,
    PlayerContext, SkillEffects, SpecialAbility, SpecialAbilityCollection,
};
use serde::{Deserialize, Serialize};

/// 🎮 특수능력 시스템 메인 프로세서
/// 게임 루프와 연동되는 핵심 컨트롤러
pub struct SpecialAbilityProcessor;

impl SpecialAbilityProcessor {
    /// 게임 틱마다 호출되는 메인 처리 함수
    /// 1. 자동 조합 확인 및 실행
    /// 2. 스킬 효과 계산
    /// 3. 상황별 능력 활성화
    pub fn process_abilities(
        collection: &mut SpecialAbilityCollection,
        player_context: &PlayerContext,
        activation_context: &AbilityActivationContext,
    ) -> ProcessingResult {
        let mut result = ProcessingResult::new();

        // 1. 자동 조합 시스템 실행
        let combination_results =
            AbilityCombinationEngine::process_automatic_combinations(collection, player_context);

        if !combination_results.is_empty() {
            result.combinations = combination_results;
            result.has_combinations = true;
        }

        // 2. 현재 보유 능력들의 종합 효과 계산
        result.skill_effects =
            AbilityEffectCalculator::calculate_combined_effects(&collection.abilities);

        // 3. 상황별 특수 효과 적용
        Self::apply_situational_effects(&mut result, collection, activation_context);

        result
    }

    /// 상황별 특수 효과 적용
    /// ClutchPlayer, PressureHandler 등의 상황 의존적 능력
    fn apply_situational_effects(
        result: &mut ProcessingResult,
        collection: &SpecialAbilityCollection,
        context: &AbilityActivationContext,
    ) {
        for ability in &collection.abilities {
            if !ability.is_positive() {
                continue;
            }

            match ability.ability_type {
                crate::special_ability::SpecialAbilityType::ClutchPlayer => {
                    if context.is_clutch_time() {
                        Self::apply_clutch_bonus(&mut result.skill_effects, ability);
                    }
                }

                crate::special_ability::SpecialAbilityType::PressureHandler => {
                    if context.is_high_pressure() {
                        Self::apply_pressure_resistance(&mut result.skill_effects, ability);
                    }
                }

                crate::special_ability::SpecialAbilityType::EnduranceKing => {
                    if context.is_fatigued() {
                        Self::apply_endurance_boost(&mut result.skill_effects, ability);
                    }
                }

                _ => {} // 다른 능력들은 기본 효과만
            }
        }
    }

    /// 클러치 상황 보너스 적용
    fn apply_clutch_bonus(effects: &mut SkillEffects, ability: &SpecialAbility) {
        let multiplier = ability.effect_strength() * 0.2; // 20% 추가 보너스
        effects.composure *= 1.0 + multiplier;
        effects.determination *= 1.0 + multiplier;
        effects.decisions *= 1.0 + multiplier;
    }

    /// 압박 저항력 적용
    fn apply_pressure_resistance(effects: &mut SkillEffects, ability: &SpecialAbility) {
        let multiplier = ability.effect_strength() * 0.15; // 15% 압박 저항
        effects.composure *= 1.0 + multiplier;
        effects.concentration *= 1.0 + multiplier;
        effects.first_touch *= 1.0 + multiplier;
    }

    /// 지구력 부스트 적용
    fn apply_endurance_boost(effects: &mut SkillEffects, ability: &SpecialAbility) {
        let multiplier = ability.effect_strength() * 0.1; // 10% 피로 저항
        effects.stamina *= 1.0 + multiplier;
        effects.work_rate *= 1.0 + multiplier;
    }

    /// OpenFootball PlayerSkills에 효과 적용
    /// 실제 게임에서 사용할 때 호출되는 함수
    pub fn apply_effects_to_openfootball_skills(
        base_skills: &mut OpenFootballSkills,
        effects: &SkillEffects,
    ) {
        // Technical Skills
        base_skills.technical.corners += effects.corners;
        base_skills.technical.crossing += effects.crossing;
        base_skills.technical.dribbling += effects.dribbling;
        base_skills.technical.finishing += effects.finishing;
        base_skills.technical.first_touch += effects.first_touch;
        base_skills.technical.free_kicks += effects.free_kicks;
        base_skills.technical.heading += effects.heading;
        base_skills.technical.long_shots += effects.long_shots;
        base_skills.technical.long_throws += effects.long_throws;
        base_skills.technical.marking += effects.marking;
        base_skills.technical.passing += effects.passing;
        base_skills.technical.penalty_taking += effects.penalty_taking;
        base_skills.technical.tackling += effects.tackling;
        base_skills.technical.technique += effects.technique;

        // Mental Skills
        base_skills.mental.aggression += effects.aggression;
        base_skills.mental.anticipation += effects.anticipation;
        base_skills.mental.bravery += effects.bravery;
        base_skills.mental.composure += effects.composure;
        base_skills.mental.concentration += effects.concentration;
        base_skills.mental.decisions += effects.decisions;
        base_skills.mental.determination += effects.determination;
        base_skills.mental.flair += effects.flair;
        base_skills.mental.leadership += effects.leadership;
        base_skills.mental.off_the_ball += effects.off_the_ball;
        base_skills.mental.positioning += effects.positioning;
        base_skills.mental.teamwork += effects.teamwork;
        base_skills.mental.vision += effects.vision;
        base_skills.mental.work_rate += effects.work_rate;

        // Physical Skills
        base_skills.physical.acceleration += effects.acceleration;
        base_skills.physical.agility += effects.agility;
        base_skills.physical.balance += effects.balance;
        base_skills.physical.jumping += effects.jumping;
        base_skills.physical.natural_fitness += effects.natural_fitness;
        base_skills.physical.pace += effects.pace;
        base_skills.physical.stamina += effects.stamina;
        base_skills.physical.strength += effects.strength;

        // 값 범위 제한 (0.0 ~ 20.0)
        Self::clamp_skills(base_skills);
    }

    /// 스킬 값 범위 제한 (OpenFootball 호환성)
    fn clamp_skills(skills: &mut OpenFootballSkills) {
        // Technical
        skills.technical.corners = skills.technical.corners.clamp(0.0, 20.0);
        skills.technical.crossing = skills.technical.crossing.clamp(0.0, 20.0);
        skills.technical.dribbling = skills.technical.dribbling.clamp(0.0, 20.0);
        skills.technical.finishing = skills.technical.finishing.clamp(0.0, 20.0);
        skills.technical.first_touch = skills.technical.first_touch.clamp(0.0, 20.0);
        skills.technical.free_kicks = skills.technical.free_kicks.clamp(0.0, 20.0);
        skills.technical.heading = skills.technical.heading.clamp(0.0, 20.0);
        skills.technical.long_shots = skills.technical.long_shots.clamp(0.0, 20.0);
        skills.technical.long_throws = skills.technical.long_throws.clamp(0.0, 20.0);
        skills.technical.marking = skills.technical.marking.clamp(0.0, 20.0);
        skills.technical.passing = skills.technical.passing.clamp(0.0, 20.0);
        skills.technical.penalty_taking = skills.technical.penalty_taking.clamp(0.0, 20.0);
        skills.technical.tackling = skills.technical.tackling.clamp(0.0, 20.0);
        skills.technical.technique = skills.technical.technique.clamp(0.0, 20.0);

        // Mental
        skills.mental.aggression = skills.mental.aggression.clamp(0.0, 20.0);
        skills.mental.anticipation = skills.mental.anticipation.clamp(0.0, 20.0);
        skills.mental.bravery = skills.mental.bravery.clamp(0.0, 20.0);
        skills.mental.composure = skills.mental.composure.clamp(0.0, 20.0);
        skills.mental.concentration = skills.mental.concentration.clamp(0.0, 20.0);
        skills.mental.decisions = skills.mental.decisions.clamp(0.0, 20.0);
        skills.mental.determination = skills.mental.determination.clamp(0.0, 20.0);
        skills.mental.flair = skills.mental.flair.clamp(0.0, 20.0);
        skills.mental.leadership = skills.mental.leadership.clamp(0.0, 20.0);
        skills.mental.off_the_ball = skills.mental.off_the_ball.clamp(0.0, 20.0);
        skills.mental.positioning = skills.mental.positioning.clamp(0.0, 20.0);
        skills.mental.teamwork = skills.mental.teamwork.clamp(0.0, 20.0);
        skills.mental.vision = skills.mental.vision.clamp(0.0, 20.0);
        skills.mental.work_rate = skills.mental.work_rate.clamp(0.0, 20.0);

        // Physical
        skills.physical.acceleration = skills.physical.acceleration.clamp(0.0, 20.0);
        skills.physical.agility = skills.physical.agility.clamp(0.0, 20.0);
        skills.physical.balance = skills.physical.balance.clamp(0.0, 20.0);
        skills.physical.jumping = skills.physical.jumping.clamp(0.0, 20.0);
        skills.physical.natural_fitness = skills.physical.natural_fitness.clamp(0.0, 20.0);
        skills.physical.pace = skills.physical.pace.clamp(0.0, 20.0);
        skills.physical.stamina = skills.physical.stamina.clamp(0.0, 20.0);
        skills.physical.strength = skills.physical.strength.clamp(0.0, 20.0);
    }
}

/// 처리 결과
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub skill_effects: SkillEffects,
    pub combinations: Vec<CombinationResult>,
    pub has_combinations: bool,
    pub situational_bonuses: Vec<String>,
}

impl ProcessingResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// 사용자에게 보여줄 메시지들
    pub fn get_messages(&self) -> Vec<String> {
        let mut messages = Vec::new();

        // 조합 메시지
        for combination in &self.combinations {
            messages.push(combination.success_message());
        }

        // 상황별 보너스 메시지
        messages.extend(self.situational_bonuses.clone());

        messages
    }
}

/// OpenFootball PlayerSkills와 호환되는 구조체
/// third_party/open-football/src/core/src/club/player/skills.rs와 동일한 구조
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenFootballSkills {
    pub technical: OpenFootballTechnical,
    pub mental: OpenFootballMental,
    pub physical: OpenFootballPhysical,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenFootballTechnical {
    pub corners: f32,
    pub crossing: f32,
    pub dribbling: f32,
    pub finishing: f32,
    pub first_touch: f32,
    pub free_kicks: f32,
    pub heading: f32,
    pub long_shots: f32,
    pub long_throws: f32,
    pub marking: f32,
    pub passing: f32,
    pub penalty_taking: f32,
    pub tackling: f32,
    pub technique: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenFootballMental {
    pub aggression: f32,
    pub anticipation: f32,
    pub bravery: f32,
    pub composure: f32,
    pub concentration: f32,
    pub decisions: f32,
    pub determination: f32,
    pub flair: f32,
    pub leadership: f32,
    pub off_the_ball: f32,
    pub positioning: f32,
    pub teamwork: f32,
    pub vision: f32,
    pub work_rate: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenFootballPhysical {
    pub acceleration: f32,
    pub agility: f32,
    pub balance: f32,
    pub jumping: f32,
    pub natural_fitness: f32,
    pub pace: f32,
    pub stamina: f32,
    pub strength: f32,
    pub match_readiness: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::special_ability::{AbilityTier, SpecialAbilityType};

    #[test]
    fn test_ability_processing() {
        let mut collection = SpecialAbilityCollection::new();
        collection.add_ability(SpecialAbility::new(
            SpecialAbilityType::DribblingMaster,
            AbilityTier::Gold,
        ));

        let player_context = PlayerContext::default();
        let activation_context = AbilityActivationContext {
            match_minute: 85,
            score_difference: 1,
            pressure_level: 0.8,
            fatigue_level: 0.5,
            is_crucial_moment: true,
            team_morale: 0.7,
        };

        let result = SpecialAbilityProcessor::process_abilities(
            &mut collection,
            &player_context,
            &activation_context,
        );

        // 드리블링 효과가 적용되었는지 확인
        assert!(result.skill_effects.dribbling > 0.0);
        assert!(result.skill_effects.technique > 0.0);
    }

    #[test]
    fn test_openfootball_skills_application() {
        let mut of_skills = OpenFootballSkills::default();
        of_skills.technical.dribbling = 10.0;

        let mut effects = SkillEffects::zero();
        effects.dribbling = 5.0;

        SpecialAbilityProcessor::apply_effects_to_openfootball_skills(&mut of_skills, &effects);

        assert_eq!(of_skills.technical.dribbling, 15.0);
    }

    #[test]
    fn test_skill_clamping() {
        let mut of_skills = OpenFootballSkills::default();
        of_skills.technical.dribbling = 25.0; // 범위 초과

        SpecialAbilityProcessor::clamp_skills(&mut of_skills);

        assert_eq!(of_skills.technical.dribbling, 20.0); // 최대값으로 제한
    }
}
