use crate::special_ability::{
    AbilityTier, CombinationRecord, CombinationType, SpecialAbility, SpecialAbilityCollection,
    SpecialAbilityType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 🎯 파워프로 스타일 자동 조합 시스템
/// Bronze 2개 → Silver (자동)
/// Silver 2개 + 조건 → Gold (자동)
/// Gold 2개 + 조건 → Diamond (조건부)
/// Diamond 3개 + 완벽조건 → Legend (극한조건)
pub struct AbilityCombinationEngine;

impl AbilityCombinationEngine {
    /// 자동 조합 가능한 능력 탐지 및 실행
    /// 게임 루프에서 매번 호출되어야 함
    pub fn process_automatic_combinations(
        collection: &mut SpecialAbilityCollection,
        player_context: &PlayerContext,
    ) -> Vec<CombinationResult> {
        let mut results = Vec::new();

        // 1. Bronze → Silver 자동 조합 (무조건)
        results.extend(Self::try_bronze_to_silver(collection));

        // 2. Silver → Gold 자동 조합 (조건부)
        results.extend(Self::try_silver_to_gold(collection, player_context));

        // 3. Gold → Diamond 조합 (엄격한 조건)
        results.extend(Self::try_gold_to_diamond(collection, player_context));

        // 4. Diamond → Legend 조합 (극한 조건)
        results.extend(Self::try_diamond_to_legend(collection, player_context));

        // 5. 부정적 조합 (자동 벌칙)
        results.extend(Self::try_negative_combinations(collection));

        results
    }

    /// Bronze → Silver 자동 조합
    /// 같은 카테고리 Bronze 2개 → 랜덤 Silver 1개
    fn try_bronze_to_silver(collection: &mut SpecialAbilityCollection) -> Vec<CombinationResult> {
        let mut results = Vec::new();

        // Bronze 능력들을 카테고리별로 그룹화
        let bronze_abilities: Vec<&SpecialAbility> = collection
            .abilities
            .iter()
            .filter(|a| a.tier == AbilityTier::Bronze && a.is_positive())
            .collect();

        let mut category_counts = HashMap::new();
        for ability in &bronze_abilities {
            let category = ability.ability_type.category();
            *category_counts.entry(category).or_insert(0) += 1;
        }

        // 2개 이상 있는 카테고리에서 조합 실행
        for (category, count) in category_counts {
            if count >= 2 {
                let combo_result = Self::execute_bronze_to_silver_combination(collection, category);
                if let Some(result) = combo_result {
                    results.push(result);
                }
            }
        }

        results
    }

    /// Bronze → Silver 조합 실행
    fn execute_bronze_to_silver_combination(
        collection: &mut SpecialAbilityCollection,
        category: crate::special_ability::AbilityCategory,
    ) -> Option<CombinationResult> {
        // 해당 카테고리의 Bronze 능력 2개 선택
        let bronze_indices: Vec<usize> = collection
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, a)| a.tier == AbilityTier::Bronze && a.ability_type.category() == category)
            .map(|(i, _)| i)
            .take(2)
            .collect();

        if bronze_indices.len() < 2 {
            return None;
        }

        // 소재 능력들 제거 (역순으로 제거해야 인덱스 꼬이지 않음)
        let mut input_abilities = Vec::new();
        for &index in bronze_indices.iter().rev() {
            input_abilities.push(collection.abilities.remove(index));
        }
        input_abilities.reverse(); // 원래 순서로 복원

        // 결과 능력 생성 (카테고리에 맞는 Silver 능력 랜덤 선택)
        let silver_ability_type = Self::random_ability_from_category(category);
        let result_ability = SpecialAbility::new(silver_ability_type, AbilityTier::Silver);

        // 컬렉션에 추가
        collection.add_ability(result_ability.clone());

        // 기록 저장
        let record = CombinationRecord {
            date: chrono::Utc::now().naive_utc(),
            input_abilities: input_abilities.clone(),
            output_ability: result_ability.clone(),
            combination_type: CombinationType::Automatic,
        };
        collection.combination_history.push(record);

        Some(CombinationResult {
            input_abilities,
            output_ability: result_ability,
            combination_type: CombinationType::Automatic,
            message: "특별한 재능이 각성했습니다!".to_string(),
            success: true,
        })
    }

    /// Silver → Gold 자동 조합 (조건부)
    fn try_silver_to_gold(
        collection: &mut SpecialAbilityCollection,
        context: &PlayerContext,
    ) -> Vec<CombinationResult> {
        let results = Vec::new();

        // Silver 능력 2개 + 추가 조건 확인
        let silver_abilities: Vec<&SpecialAbility> = collection
            .abilities
            .iter()
            .filter(|a| a.tier == AbilityTier::Silver && a.is_positive())
            .collect();

        if silver_abilities.len() >= 2 && Self::check_gold_conditions(context) {
            // 조합 실행 로직은 Bronze → Silver와 유사하지만 조건 체크 추가
            // 구현 생략 (실제로는 위와 동일한 패턴으로 구현)
        }

        results
    }

    /// Gold → Diamond 조합 (엄격한 조건)
    fn try_gold_to_diamond(
        _collection: &mut SpecialAbilityCollection,
        context: &PlayerContext,
    ) -> Vec<CombinationResult> {
        let results = Vec::new();

        if Self::check_diamond_conditions(context) {
            // Diamond 조합 로직
        }

        results
    }

    /// Diamond → Legend 조합 (극한 조건)
    fn try_diamond_to_legend(
        _collection: &mut SpecialAbilityCollection,
        context: &PlayerContext,
    ) -> Vec<CombinationResult> {
        let results = Vec::new();

        if Self::check_legend_conditions(context) {
            // Legend 조합 로직 (가장 어려운 조건)
        }

        results
    }

    /// 부정적 조합 (자동 벌칙)
    /// Red 2개 → Poison (막을 수 없음)
    fn try_negative_combinations(
        collection: &mut SpecialAbilityCollection,
    ) -> Vec<CombinationResult> {
        let results = Vec::new();

        let red_abilities: Vec<&SpecialAbility> =
            collection.abilities.iter().filter(|a| a.tier == AbilityTier::Red).collect();

        if red_abilities.len() >= 2 {
            // 부정적 조합 실행 (경고 메시지와 함께)
            // "나쁜 습관이 굳어지고 있습니다..."
        }

        results
    }

    /// 카테고리별 랜덤 능력 선택
    fn random_ability_from_category(
        category: crate::special_ability::AbilityCategory,
    ) -> SpecialAbilityType {
        use crate::special_ability::AbilityCategory;
        use rand::prelude::SliceRandom;

        let abilities = match category {
            AbilityCategory::Technical => vec![
                SpecialAbilityType::DribblingMaster,
                SpecialAbilityType::PassingGenius,
                SpecialAbilityType::ShootingStar,
                SpecialAbilityType::SetPieceSpecialist,
            ],
            AbilityCategory::Mental => vec![
                SpecialAbilityType::CaptainMaterial,
                SpecialAbilityType::ClutchPlayer,
                SpecialAbilityType::TeamPlayer,
                SpecialAbilityType::PressureHandler,
            ],
            AbilityCategory::Physical => vec![
                SpecialAbilityType::SpeedDemon,
                SpecialAbilityType::EnduranceKing,
                SpecialAbilityType::PowerHouse,
                SpecialAbilityType::AgilityMaster,
            ],
        };

        *abilities.choose(&mut rand::thread_rng()).unwrap()
    }

    /// Gold 조합 조건 확인
    fn check_gold_conditions(context: &PlayerContext) -> bool {
        context.current_ability >= 80
            && context.games_played >= 20
            && context.training_consistency > 0.8
    }

    /// Diamond 조합 조건 확인 (매우 엄격)
    fn check_diamond_conditions(context: &PlayerContext) -> bool {
        context.current_ability >= 90
            && context.is_team_captain
            && context.major_titles > 0
            && context.perfect_games >= 3
    }

    /// Legend 조합 조건 확인 (극한 조건)
    fn check_legend_conditions(context: &PlayerContext) -> bool {
        context.current_ability >= 95
            && context.is_national_team_player
            && context.major_titles >= 2
            && context.perfect_season
            && context.all_relationships_maxed
    }
}

/// 특수능력 조합 관련 컨텍스트
/// 조합 조건 판단에 사용되는 선수 상태 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerContext {
    pub current_ability: u8,           // 현재 능력치
    pub potential_ability: u8,         // 잠재 능력치
    pub games_played: u32,             // 경기 수
    pub training_consistency: f32,     // 훈련 일관성 (0.0-1.0)
    pub is_team_captain: bool,         // 주장 여부
    pub is_national_team_player: bool, // 국대 선수 여부
    pub major_titles: u32,             // 주요 대회 우승 횟수
    pub perfect_games: u32,            // 완벽한 경기 횟수
    pub perfect_season: bool,          // 완벽한 시즌 달성 여부
    pub all_relationships_maxed: bool, // 모든 관계도 최대치 달성
}

impl Default for PlayerContext {
    fn default() -> Self {
        Self {
            current_ability: 50,
            potential_ability: 80,
            games_played: 0,
            training_consistency: 0.5,
            is_team_captain: false,
            is_national_team_player: false,
            major_titles: 0,
            perfect_games: 0,
            perfect_season: false,
            all_relationships_maxed: false,
        }
    }
}

/// 조합 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinationResult {
    pub input_abilities: Vec<SpecialAbility>,
    pub output_ability: SpecialAbility,
    pub combination_type: CombinationType,
    pub message: String, // 플레이어에게 보여줄 메시지
    pub success: bool,
}

impl CombinationResult {
    /// 조합 성공 메시지 생성
    pub fn success_message(&self) -> String {
        match self.output_ability.tier {
            AbilityTier::Silver => "✨ 특별한 재능이 각성했습니다!".to_string(),
            AbilityTier::Gold => "🌟 전설적인 각성이 일어났습니다!".to_string(),
            AbilityTier::Diamond => "💎 신의 영역에 발을 들였습니다!".to_string(),
            AbilityTier::Legend => "🌈 축구의 신으로 각성했습니다!".to_string(),
            AbilityTier::Poison => "☠️ 나쁜 습관이 굳어져버렸습니다...".to_string(),
            _ => "⚡ 능력이 변화했습니다.".to_string(),
        }
    }

    /// 조합 효과 설명
    pub fn effect_description(&self) -> String {
        format!(
            "{} → {} ({:?})",
            self.input_abilities
                .iter()
                .map(|a| a.ability_type.name())
                .collect::<Vec<_>>()
                .join(" + "),
            self.output_ability.ability_type.name(),
            self.output_ability.tier
        )
    }
}

/// 🎯 조합 룰 정의
/// 파워프로 스타일 정확한 조합 공식
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinationRule {
    pub input_types: Vec<SpecialAbilityType>,
    pub input_tier: AbilityTier,
    pub output_type: SpecialAbilityType,
    pub output_tier: AbilityTier,
    pub required_conditions: CombinationConditions,
    pub probability: f32, // 조합 성공 확률
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinationConditions {
    pub min_current_ability: u8,
    pub min_games_played: u32,
    pub requires_captain: bool,
    pub requires_national_team: bool,
    pub min_training_consistency: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::special_ability::SpecialAbilityCollection;

    #[test]
    fn test_bronze_to_silver_combination() {
        let mut collection = SpecialAbilityCollection::new();

        // Bronze 능력 2개 추가
        collection.add_ability(SpecialAbility::new(
            SpecialAbilityType::DribblingMaster,
            AbilityTier::Bronze,
        ));
        collection.add_ability(SpecialAbility::new(
            SpecialAbilityType::PassingGenius,
            AbilityTier::Bronze,
        ));

        let context = PlayerContext::default();
        let results =
            AbilityCombinationEngine::process_automatic_combinations(&mut collection, &context);

        // Silver 조합이 발생했는지 확인
        assert!(!results.is_empty());
        if let Some(result) = results.first() {
            assert_eq!(result.output_ability.tier, AbilityTier::Silver);
            assert_eq!(result.input_abilities.len(), 2);
        }

        // Bronze 능력이 사라지고 Silver 능력이 추가되었는지 확인
        let silver_count =
            collection.abilities.iter().filter(|a| a.tier == AbilityTier::Silver).count();
        assert_eq!(silver_count, 1);
    }

    #[test]
    fn test_player_context_conditions() {
        let context = PlayerContext {
            current_ability: 85,
            games_played: 25,
            training_consistency: 0.9,
            ..Default::default()
        };

        assert!(AbilityCombinationEngine::check_gold_conditions(&context));
    }

    #[test]
    fn test_combination_result_messages() {
        let result = CombinationResult {
            input_abilities: vec![
                SpecialAbility::new(SpecialAbilityType::DribblingMaster, AbilityTier::Bronze),
                SpecialAbility::new(SpecialAbilityType::PassingGenius, AbilityTier::Bronze),
            ],
            output_ability: SpecialAbility::new(
                SpecialAbilityType::ShootingStar,
                AbilityTier::Gold,
            ),
            combination_type: CombinationType::Automatic,
            message: "Test".to_string(),
            success: true,
        };

        assert!(result.success_message().contains("전설적인 각성"));
        assert!(result.effect_description().contains("슈팅 스타"));
    }
}
