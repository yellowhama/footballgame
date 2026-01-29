// 시너지 효과 시스템
use super::card::CoachCard;
use serde::{Deserialize, Serialize};

/// 시너지 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SynergyType {
    /// 같은 전문분야 3장 이상
    SpecialtyFocus,
    /// 모든 전문분야 보유 (다양성)
    Diversity,
    /// 레어도 합계 기준
    RarityBonus,
    /// 특정 코치 조합
    SpecificCombo(Vec<String>),
    /// 레벨 합계 기준
    ExperiencedTeam,
    /// 신규 코치 육성
    RookieDevelopment,
}

/// 시너지 효과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynergyEffect {
    /// 시너지 이름
    pub name: String,
    /// 시너지 타입
    pub synergy_type: SynergyType,
    /// 발동 조건 설명
    pub condition: String,
    /// 보너스 종류
    pub bonus_type: BonusType,
    /// 보너스 수치
    pub bonus_value: f32,
    /// 시너지 설명
    pub description: String,
    /// 활성화 여부
    pub is_active: bool,
}

/// 보너스 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BonusType {
    /// 훈련 효과 증가
    TrainingEfficiency,
    /// 체력 소모 감소
    StaminaCost,
    /// 부상 위험 감소
    InjuryPrevention,
    /// 경험치 획득 증가
    ExperienceGain,
    /// 컨디션 유지
    ConditionStability,
    /// CA 성장 가속
    GrowthRate,
}

/// 시너지 계산기
pub struct SynergyCalculator;

impl SynergyCalculator {
    /// 덱의 모든 시너지 효과 계산
    pub fn calculate_all_synergies(
        manager: &Option<CoachCard>,
        coaches: &[Option<CoachCard>],
    ) -> Vec<SynergyEffect> {
        let mut synergies = Vec::new();

        // 전문분야 집중 시너지
        if let Some(effect) = Self::check_specialty_focus(manager, coaches) {
            synergies.push(effect);
        }

        // 다양성 시너지
        if let Some(effect) = Self::check_diversity(manager, coaches) {
            synergies.push(effect);
        }

        // 레어도 시너지
        if let Some(effect) = Self::check_rarity_bonus(manager, coaches) {
            synergies.push(effect);
        }

        // 경험 많은 팀 시너지
        if let Some(effect) = Self::check_experienced_team(manager, coaches) {
            synergies.push(effect);
        }

        // 신규 코치 육성 시너지
        if let Some(effect) = Self::check_rookie_development(manager, coaches) {
            synergies.push(effect);
        }

        synergies
    }

    /// 전문분야 집중 시너지 체크
    fn check_specialty_focus(
        manager: &Option<CoachCard>,
        coaches: &[Option<CoachCard>],
    ) -> Option<SynergyEffect> {
        let mut specialty_count = std::collections::HashMap::new();

        // 감독 전문분야 카운트
        if let Some(ref m) = manager {
            *specialty_count.entry(m.specialty).or_insert(0) += 1;
        }

        // 코치 전문분야 카운트
        for c in coaches.iter().flatten() {
            *specialty_count.entry(c.specialty).or_insert(0) += 1;
        }

        // 3장 이상 같은 전문분야 확인
        for (specialty, count) in specialty_count {
            if count >= 3 {
                return Some(SynergyEffect {
                    name: format!("{:?} 전문가 팀", specialty),
                    synergy_type: SynergyType::SpecialtyFocus,
                    condition: format!("{:?} 전문 카드 3장 이상", specialty),
                    bonus_type: BonusType::TrainingEfficiency,
                    bonus_value: 0.15, // 15% 보너스
                    description: format!("{:?} 훈련 시 추가 15% 효과 증가", specialty),
                    is_active: true,
                });
            }
        }

        None
    }

    /// 다양성 시너지 체크
    fn check_diversity(
        manager: &Option<CoachCard>,
        coaches: &[Option<CoachCard>],
    ) -> Option<SynergyEffect> {
        let mut specialties = std::collections::HashSet::new();

        if let Some(ref m) = manager {
            specialties.insert(m.specialty);
        }

        for c in coaches.iter().flatten() {
            specialties.insert(c.specialty);
        }

        // 4가지 이상 다른 전문분야
        if specialties.len() >= 4 {
            return Some(SynergyEffect {
                name: "다재다능한 스태프".to_string(),
                synergy_type: SynergyType::Diversity,
                condition: "4가지 이상 다른 전문분야 보유".to_string(),
                bonus_type: BonusType::GrowthRate,
                bonus_value: 0.10,
                description: "모든 능력치 성장률 10% 증가".to_string(),
                is_active: true,
            });
        }

        None
    }

    /// 레어도 보너스 체크
    fn check_rarity_bonus(
        manager: &Option<CoachCard>,
        coaches: &[Option<CoachCard>],
    ) -> Option<SynergyEffect> {
        let mut total_rarity = 0u8;
        let mut card_count = 0;

        if let Some(ref m) = manager {
            total_rarity += m.rarity as u8;
            card_count += 1;
        }

        for c in coaches.iter().flatten() {
            total_rarity += c.rarity as u8;
            card_count += 1;
        }

        // 평균 레어도 3.5 이상
        if card_count > 0 && (total_rarity as f32 / card_count as f32) >= 3.5 {
            return Some(SynergyEffect {
                name: "올스타 스태프".to_string(),
                synergy_type: SynergyType::RarityBonus,
                condition: "평균 레어도 ⭐⭐⭐+ 이상".to_string(),
                bonus_type: BonusType::ExperienceGain,
                bonus_value: 0.25,
                description: "카드 경험치 획득량 25% 증가".to_string(),
                is_active: true,
            });
        }

        None
    }

    /// 경험 많은 팀 시너지
    fn check_experienced_team(
        manager: &Option<CoachCard>,
        coaches: &[Option<CoachCard>],
    ) -> Option<SynergyEffect> {
        let mut total_level = 0u32;
        let mut card_count = 0;

        if let Some(ref m) = manager {
            total_level += m.level as u32;
            card_count += 1;
        }

        for c in coaches.iter().flatten() {
            total_level += c.level as u32;
            card_count += 1;
        }

        // 평균 레벨 5 이상
        if card_count > 0 && (total_level as f32 / card_count as f32) >= 5.0 {
            return Some(SynergyEffect {
                name: "베테랑 스태프".to_string(),
                synergy_type: SynergyType::ExperiencedTeam,
                condition: "평균 카드 레벨 5 이상".to_string(),
                bonus_type: BonusType::InjuryPrevention,
                bonus_value: 0.30,
                description: "부상 위험 30% 감소".to_string(),
                is_active: true,
            });
        }

        None
    }

    /// 신규 코치 육성 시너지
    fn check_rookie_development(
        manager: &Option<CoachCard>,
        coaches: &[Option<CoachCard>],
    ) -> Option<SynergyEffect> {
        let mut low_level_count = 0;
        let mut high_level_exists = false;

        // 높은 레벨 감독 확인
        if let Some(ref m) = manager {
            if m.level >= 7 {
                high_level_exists = true;
            }
        }

        // 낮은 레벨 코치 카운트
        for c in coaches.iter().flatten() {
            if c.level <= 3 {
                low_level_count += 1;
            }
        }

        // 고레벨 감독 + 저레벨 코치 2명 이상
        if high_level_exists && low_level_count >= 2 {
            return Some(SynergyEffect {
                name: "신인 육성 프로그램".to_string(),
                synergy_type: SynergyType::RookieDevelopment,
                condition: "Lv7+ 감독과 Lv3 이하 코치 2명".to_string(),
                bonus_type: BonusType::ExperienceGain,
                bonus_value: 0.50,
                description: "저레벨 카드 경험치 50% 추가".to_string(),
                is_active: true,
            });
        }

        None
    }

    /// 특정 콤보 시너지 체크 (확장용)
    pub fn check_specific_combos(
        cards: &[CoachCard],
        combo_db: &ComboDatabase,
    ) -> Vec<SynergyEffect> {
        let mut found_combos = Vec::new();

        for combo in &combo_db.combos {
            if combo.check_cards(cards) {
                found_combos.push(combo.create_effect());
            }
        }

        found_combos
    }
}

/// 특정 콤보 데이터베이스
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboDatabase {
    pub combos: Vec<SpecificCombo>,
}

/// 특정 콤보 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificCombo {
    pub name: String,
    pub required_card_ids: Vec<String>,
    pub bonus_type: BonusType,
    pub bonus_value: f32,
    pub description: String,
}

impl SpecificCombo {
    /// 카드들이 콤보 조건을 만족하는지 체크
    pub fn check_cards(&self, cards: &[CoachCard]) -> bool {
        let card_ids: std::collections::HashSet<_> = cards.iter().map(|c| c.id.clone()).collect();

        self.required_card_ids.iter().all(|id| card_ids.contains(id))
    }

    /// 시너지 효과 생성
    pub fn create_effect(&self) -> SynergyEffect {
        SynergyEffect {
            name: self.name.clone(),
            synergy_type: SynergyType::SpecificCombo(self.required_card_ids.clone()),
            condition: format!("특정 카드 조합: {:?}", self.required_card_ids),
            bonus_type: self.bonus_type.clone(),
            bonus_value: self.bonus_value,
            description: self.description.clone(),
            is_active: true,
        }
    }
}

impl Default for ComboDatabase {
    fn default() -> Self {
        Self {
            combos: vec![
                // 예시 콤보들
                SpecificCombo {
                    name: "황금 트리오".to_string(),
                    required_card_ids: vec![
                        "legendary_coach_001".to_string(),
                        "legendary_coach_002".to_string(),
                        "legendary_coach_003".to_string(),
                    ],
                    bonus_type: BonusType::TrainingEfficiency,
                    bonus_value: 0.30,
                    description: "전설의 코치 3인방이 모이면 훈련 효과 30% 증가".to_string(),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::card::{CardRarity, Specialty};
    use super::*;

    #[test]
    fn test_specialty_focus_synergy() {
        let manager = Some(CoachCard::new(
            "m1".to_string(),
            "스피드 감독".to_string(),
            CardRarity::Three,
            super::super::card::CardType::Manager,
            Specialty::Speed,
            "".to_string(),
        ));

        let coaches = vec![
            Some(CoachCard::new(
                "c1".to_string(),
                "스피드 코치1".to_string(),
                CardRarity::Two,
                super::super::card::CardType::Coach,
                Specialty::Speed,
                "".to_string(),
            )),
            Some(CoachCard::new(
                "c2".to_string(),
                "스피드 코치2".to_string(),
                CardRarity::Two,
                super::super::card::CardType::Coach,
                Specialty::Speed,
                "".to_string(),
            )),
            None,
        ];

        let synergies = SynergyCalculator::calculate_all_synergies(&manager, &coaches);

        // Speed 전문가 3명이므로 시너지 발동
        assert!(!synergies.is_empty());
        assert!(synergies.iter().any(|s| matches!(s.synergy_type, SynergyType::SpecialtyFocus)));
    }
}
