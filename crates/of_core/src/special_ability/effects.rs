use crate::special_ability::{SpecialAbility, SpecialAbilityType};
use serde::{Deserialize, Serialize};

/// 🎯 OpenFootball PlayerSkills 효과 구조체
/// third_party/open-football/src/core/src/club/player/skills.rs 연동
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillEffects {
    // Technical Skills 14개 효과
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

    // Mental Skills 14개 효과
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

    // Physical Skills 8개 효과
    pub acceleration: f32,
    pub agility: f32,
    pub balance: f32,
    pub jumping: f32,
    pub natural_fitness: f32,
    pub pace: f32,
    pub stamina: f32,
    pub strength: f32,
}

impl SkillEffects {
    /// 모든 효과를 0으로 초기화
    pub fn zero() -> Self {
        Self::default()
    }

    /// 효과 합산 (다른 SkillEffects와 합치기)
    pub fn add(&mut self, other: &SkillEffects) {
        self.corners += other.corners;
        self.crossing += other.crossing;
        self.dribbling += other.dribbling;
        self.finishing += other.finishing;
        self.first_touch += other.first_touch;
        self.free_kicks += other.free_kicks;
        self.heading += other.heading;
        self.long_shots += other.long_shots;
        self.long_throws += other.long_throws;
        self.marking += other.marking;
        self.passing += other.passing;
        self.penalty_taking += other.penalty_taking;
        self.tackling += other.tackling;
        self.technique += other.technique;

        self.aggression += other.aggression;
        self.anticipation += other.anticipation;
        self.bravery += other.bravery;
        self.composure += other.composure;
        self.concentration += other.concentration;
        self.decisions += other.decisions;
        self.determination += other.determination;
        self.flair += other.flair;
        self.leadership += other.leadership;
        self.off_the_ball += other.off_the_ball;
        self.positioning += other.positioning;
        self.teamwork += other.teamwork;
        self.vision += other.vision;
        self.work_rate += other.work_rate;

        self.acceleration += other.acceleration;
        self.agility += other.agility;
        self.balance += other.balance;
        self.jumping += other.jumping;
        self.natural_fitness += other.natural_fitness;
        self.pace += other.pace;
        self.stamina += other.stamina;
        self.strength += other.strength;
    }

    /// 배율 적용 (티어 효과 반영)
    pub fn multiply(&mut self, multiplier: f32) {
        self.corners *= multiplier;
        self.crossing *= multiplier;
        self.dribbling *= multiplier;
        self.finishing *= multiplier;
        self.first_touch *= multiplier;
        self.free_kicks *= multiplier;
        self.heading *= multiplier;
        self.long_shots *= multiplier;
        self.long_throws *= multiplier;
        self.marking *= multiplier;
        self.passing *= multiplier;
        self.penalty_taking *= multiplier;
        self.tackling *= multiplier;
        self.technique *= multiplier;

        self.aggression *= multiplier;
        self.anticipation *= multiplier;
        self.bravery *= multiplier;
        self.composure *= multiplier;
        self.concentration *= multiplier;
        self.decisions *= multiplier;
        self.determination *= multiplier;
        self.flair *= multiplier;
        self.leadership *= multiplier;
        self.off_the_ball *= multiplier;
        self.positioning *= multiplier;
        self.teamwork *= multiplier;
        self.vision *= multiplier;
        self.work_rate *= multiplier;

        self.acceleration *= multiplier;
        self.agility *= multiplier;
        self.balance *= multiplier;
        self.jumping *= multiplier;
        self.natural_fitness *= multiplier;
        self.pace *= multiplier;
        self.stamina *= multiplier;
        self.strength *= multiplier;
    }
}

/// 🌟 특수능력별 효과 정의
/// 각 특수능력이 OpenFootball Skills에 미치는 영향
pub struct AbilityEffectCalculator;

impl AbilityEffectCalculator {
    /// 특수능력에서 스킬 효과 계산
    pub fn calculate_effects(ability: &SpecialAbility) -> SkillEffects {
        let mut effects = Self::get_base_effects(ability.ability_type);

        // 티어별 배율 적용
        let multiplier = ability.tier.effect_multiplier();
        effects.multiply(multiplier);

        effects
    }

    /// 특수능력 타입별 기본 효과 정의
    fn get_base_effects(ability_type: SpecialAbilityType) -> SkillEffects {
        let mut effects = SkillEffects::zero();

        match ability_type {
            // 🔥 Technical 계열
            SpecialAbilityType::DribblingMaster => {
                effects.dribbling = 4.0;
                effects.first_touch = 3.0;
                effects.technique = 2.0;
                effects.agility = 2.0;
                effects.balance = 1.5;
            }

            SpecialAbilityType::PassingGenius => {
                effects.passing = 5.0;
                effects.vision = 4.0;
                effects.technique = 2.0;
                effects.composure = 2.0;
                effects.decisions = 1.5;
            }

            SpecialAbilityType::ShootingStar => {
                effects.finishing = 5.0;
                effects.long_shots = 4.0;
                effects.technique = 2.0;
                effects.composure = 2.0;
                effects.off_the_ball = 1.5;
            }

            SpecialAbilityType::SetPieceSpecialist => {
                effects.free_kicks = 5.0;
                effects.corners = 4.0;
                effects.penalty_taking = 3.0;
                effects.technique = 2.0;
                effects.composure = 2.0;
            }

            // 🧠 Mental 계열
            SpecialAbilityType::CaptainMaterial => {
                effects.leadership = 5.0;
                effects.determination = 3.0;
                effects.teamwork = 3.0;
                effects.composure = 2.0;
                effects.bravery = 2.0;
            }

            SpecialAbilityType::ClutchPlayer => {
                effects.composure = 5.0;
                effects.determination = 4.0;
                effects.decisions = 3.0;
                effects.concentration = 2.0;
                effects.bravery = 2.0;
            }

            SpecialAbilityType::TeamPlayer => {
                effects.teamwork = 5.0;
                effects.passing = 2.0;
                effects.work_rate = 3.0;
                effects.positioning = 2.0;
                effects.decisions = 1.5;
            }

            SpecialAbilityType::PressureHandler => {
                effects.composure = 4.0;
                effects.concentration = 4.0;
                effects.decisions = 3.0;
                effects.bravery = 2.0;
                effects.determination = 2.0;
            }

            // 💪 Physical 계열
            SpecialAbilityType::SpeedDemon => {
                effects.pace = 5.0;
                effects.acceleration = 4.0;
                effects.agility = 2.0;
                effects.natural_fitness = 2.0;
                effects.off_the_ball = 1.5;
            }

            SpecialAbilityType::EnduranceKing => {
                effects.stamina = 6.0;
                effects.natural_fitness = 4.0;
                effects.work_rate = 3.0;
                effects.determination = 2.0;
            }

            SpecialAbilityType::PowerHouse => {
                effects.strength = 5.0;
                effects.jumping = 3.0;
                effects.heading = 3.0;
                effects.tackling = 2.0;
                effects.bravery = 2.0;
            }

            SpecialAbilityType::AgilityMaster => {
                effects.agility = 5.0;
                effects.balance = 4.0;
                effects.dribbling = 2.0;
                effects.first_touch = 2.0;
                effects.acceleration = 1.5;
            }
        }

        effects
    }

    /// 여러 특수능력의 종합 효과 계산
    pub fn calculate_combined_effects(abilities: &[SpecialAbility]) -> SkillEffects {
        let mut total_effects = SkillEffects::zero();

        for ability in abilities {
            let ability_effects = Self::calculate_effects(ability);
            total_effects.add(&ability_effects);
        }

        // 🎯 시너지 효과 적용 (같은 카테고리 능력이 많을수록 보너스)
        Self::apply_synergy_bonuses(&mut total_effects, abilities);

        total_effects
    }

    /// 시너지 보너스 적용
    fn apply_synergy_bonuses(effects: &mut SkillEffects, abilities: &[SpecialAbility]) {
        use crate::special_ability::AbilityCategory;

        // 카테고리별 개수 계산
        let mut technical_count = 0;
        let mut mental_count = 0;
        let mut physical_count = 0;

        for ability in abilities {
            if !ability.is_positive() {
                continue; // 부정적 능력은 시너지 제외
            }

            match ability.ability_type.category() {
                AbilityCategory::Technical => technical_count += 1,
                AbilityCategory::Mental => mental_count += 1,
                AbilityCategory::Physical => physical_count += 1,
            }
        }

        // 시너지 보너스 적용 (2개 이상부터)
        if technical_count >= 2 {
            let bonus = (technical_count as f32 - 1.0) * 0.1;
            effects.technique *= 1.0 + bonus;
            effects.dribbling *= 1.0 + bonus;
            effects.passing *= 1.0 + bonus;
        }

        if mental_count >= 2 {
            let bonus = (mental_count as f32 - 1.0) * 0.1;
            effects.composure *= 1.0 + bonus;
            effects.decisions *= 1.0 + bonus;
            effects.concentration *= 1.0 + bonus;
        }

        if physical_count >= 2 {
            let bonus = (physical_count as f32 - 1.0) * 0.1;
            effects.stamina *= 1.0 + bonus;
            effects.natural_fitness *= 1.0 + bonus;
            effects.strength *= 1.0 + bonus;
        }
    }
}

/// 🎮 게임 상황별 특수능력 활성화 컨텍스트
/// OpenFootball StateProcessing에서 사용될 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityActivationContext {
    pub match_minute: u32,
    pub score_difference: i32,
    pub pressure_level: f32,     // 상대 선수 압박 수준
    pub fatigue_level: f32,      // 피로도
    pub is_crucial_moment: bool, // 중요한 순간 여부
    pub team_morale: f32,        // 팀 사기
}

impl AbilityActivationContext {
    /// 클러치 상황 판단
    pub fn is_clutch_time(&self) -> bool {
        self.match_minute > 80
            || (self.match_minute > 40 && self.match_minute < 50)
            || self.score_difference.abs() <= 1
    }

    /// 고압박 상황 판단
    pub fn is_high_pressure(&self) -> bool {
        self.pressure_level > 0.7
    }

    /// 고피로 상황 판단
    pub fn is_fatigued(&self) -> bool {
        self.fatigue_level > 0.8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::special_ability::{AbilityTier, SpecialAbility, SpecialAbilityType};

    #[test]
    fn test_dribbling_master_effects() {
        let ability = SpecialAbility::new(SpecialAbilityType::DribblingMaster, AbilityTier::Gold);
        let effects = AbilityEffectCalculator::calculate_effects(&ability);

        // Gold 티어 배율(6.0) 적용된 드리블 효과 확인
        assert_eq!(effects.dribbling, 4.0 * 6.0); // 24.0
        assert_eq!(effects.first_touch, 3.0 * 6.0); // 18.0
        assert!(effects.technique > 0.0);
    }

    #[test]
    fn test_combined_effects() {
        let abilities = vec![
            SpecialAbility::new(SpecialAbilityType::DribblingMaster, AbilityTier::Silver),
            SpecialAbility::new(SpecialAbilityType::PassingGenius, AbilityTier::Bronze),
        ];

        let effects = AbilityEffectCalculator::calculate_combined_effects(&abilities);

        // 두 능력의 효과가 합산되어야 함
        assert!(effects.dribbling > 0.0);
        assert!(effects.passing > 0.0);
        assert!(effects.technique > 0.0); // 공통 효과
    }

    #[test]
    fn test_activation_context() {
        let context = AbilityActivationContext {
            match_minute: 85,
            score_difference: 1,
            pressure_level: 0.8,
            fatigue_level: 0.9,
            is_crucial_moment: true,
            team_morale: 0.7,
        };

        assert!(context.is_clutch_time());
        assert!(context.is_high_pressure());
        assert!(context.is_fatigued());
    }
}
