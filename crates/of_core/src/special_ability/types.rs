use serde::{Deserialize, Serialize};

/// 🌟 특수능력 7단계 티어 시스템
/// 파워풀 프로야구 스타일 등급 체계
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityTier {
    // 긍정적 능력 (5단계)
    Bronze,  // 🟤 동특 - 아주 조금 향상 (+1~2)
    Silver,  // ⚪ 은특 - 조금 향상 (+3~4)
    Gold,    // 🟡 금특 - 꽤 향상 (+5~7)
    Diamond, // 💎 다이아특 - 많이 향상 (+8~10)
    Legend,  // 🌈 레전드특 - 엄청나게 향상 (+12~15)

    // 부정적 능력 (2단계)
    Red,    // 🔴 적특 - 조금 감소 (-1~2)
    Poison, // 🟣 독특 - 많이 감소 (-5~7)
}

impl AbilityTier {
    /// 티어별 효과 배율 반환
    pub fn effect_multiplier(&self) -> f32 {
        match self {
            AbilityTier::Bronze => 1.5,
            AbilityTier::Silver => 3.5,
            AbilityTier::Gold => 6.0,
            AbilityTier::Diamond => 9.0,
            AbilityTier::Legend => 13.5,
            AbilityTier::Red => -1.5,
            AbilityTier::Poison => -6.0,
        }
    }

    /// 희귀도 순위 (1이 가장 희귀)
    pub fn rarity_rank(&self) -> u8 {
        match self {
            AbilityTier::Legend => 1,
            AbilityTier::Diamond => 2,
            AbilityTier::Gold => 3,
            AbilityTier::Silver => 4,
            AbilityTier::Bronze => 5,
            AbilityTier::Poison => 6,
            AbilityTier::Red => 7,
        }
    }
}

/// 🎯 12개 특수능력 체계
/// Technical, Mental, Physical 각 4개씩
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecialAbilityType {
    // Technical 계열 (4개) - 기술적 특성
    DribblingMaster,    // 드리블 마스터
    PassingGenius,      // 패스 천재
    ShootingStar,       // 슈팅 스타
    SetPieceSpecialist, // 세트피스 전문가

    // Mental 계열 (4개) - 정신적 특성
    CaptainMaterial, // 주장감
    ClutchPlayer,    // 클러치 플레이어
    TeamPlayer,      // 팀 플레이어
    PressureHandler, // 압박 관리자

    // Physical 계열 (4개) - 신체적 특성
    SpeedDemon,    // 스피드 악마
    EnduranceKing, // 체력왕
    PowerHouse,    // 파워하우스
    AgilityMaster, // 민첩성 마스터
}

impl SpecialAbilityType {
    /// 능력 카테고리 반환
    pub fn category(&self) -> AbilityCategory {
        match self {
            SpecialAbilityType::DribblingMaster
            | SpecialAbilityType::PassingGenius
            | SpecialAbilityType::ShootingStar
            | SpecialAbilityType::SetPieceSpecialist => AbilityCategory::Technical,

            SpecialAbilityType::CaptainMaterial
            | SpecialAbilityType::ClutchPlayer
            | SpecialAbilityType::TeamPlayer
            | SpecialAbilityType::PressureHandler => AbilityCategory::Mental,

            SpecialAbilityType::SpeedDemon
            | SpecialAbilityType::EnduranceKing
            | SpecialAbilityType::PowerHouse
            | SpecialAbilityType::AgilityMaster => AbilityCategory::Physical,
        }
    }

    /// 능력 이름 (한국어)
    pub fn name(&self) -> &'static str {
        match self {
            SpecialAbilityType::DribblingMaster => "드리블 마스터",
            SpecialAbilityType::PassingGenius => "패스 천재",
            SpecialAbilityType::ShootingStar => "슈팅 스타",
            SpecialAbilityType::SetPieceSpecialist => "세트피스 전문가",
            SpecialAbilityType::CaptainMaterial => "주장감",
            SpecialAbilityType::ClutchPlayer => "클러치 플레이어",
            SpecialAbilityType::TeamPlayer => "팀 플레이어",
            SpecialAbilityType::PressureHandler => "압박 관리자",
            SpecialAbilityType::SpeedDemon => "스피드 악마",
            SpecialAbilityType::EnduranceKing => "체력왕",
            SpecialAbilityType::PowerHouse => "파워하우스",
            SpecialAbilityType::AgilityMaster => "민첩성 마스터",
        }
    }
}

/// 능력 카테고리
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityCategory {
    Technical, // 기술
    Mental,    // 정신
    Physical,  // 체력
}

/// 🌟 특수능력 구조체 (핵심)
/// OpenFootball PlayerSkills에 직접 영향을 주는 시스템
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecialAbility {
    pub ability_type: SpecialAbilityType,
    pub tier: AbilityTier,
    pub acquired_date: Option<chrono::NaiveDateTime>,
    pub activation_count: u32,
}

impl SpecialAbility {
    /// 새로운 특수능력 생성
    pub fn new(ability_type: SpecialAbilityType, tier: AbilityTier) -> Self {
        Self {
            ability_type,
            tier,
            acquired_date: Some(chrono::Utc::now().naive_utc()),
            activation_count: 0,
        }
    }

    /// 능력 고유 ID 생성
    pub fn id(&self) -> String {
        format!("{:?}_{:?}", self.ability_type, self.tier)
    }

    /// 효과 배율 계산
    pub fn effect_strength(&self) -> f32 {
        self.tier.effect_multiplier()
    }

    /// 긍정적/부정적 능력 구분
    pub fn is_positive(&self) -> bool {
        !matches!(self.tier, AbilityTier::Red | AbilityTier::Poison)
    }

    /// 조합 가능 여부 확인
    pub fn can_combine(&self) -> bool {
        self.tier != AbilityTier::Legend && self.tier != AbilityTier::Poison
    }
}

/// 선수의 특수능력 컬렉션
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SpecialAbilityCollection {
    pub abilities: Vec<SpecialAbility>,
    pub combination_history: Vec<CombinationRecord>,
}

impl SpecialAbilityCollection {
    /// 새로운 컬렉션 생성
    pub fn new() -> Self {
        Self { abilities: Vec::new(), combination_history: Vec::new() }
    }

    /// 특수능력 추가
    pub fn add_ability(&mut self, ability: SpecialAbility) {
        self.abilities.push(ability);
    }

    /// 특정 타입의 능력 보유 확인
    pub fn has_ability(&self, ability_type: SpecialAbilityType) -> bool {
        self.abilities.iter().any(|a| a.ability_type == ability_type)
    }

    /// 특정 타입+티어의 능력 보유 확인
    pub fn has_exact_ability(&self, ability_type: SpecialAbilityType, tier: AbilityTier) -> bool {
        self.abilities.iter().any(|a| a.ability_type == ability_type && a.tier == tier)
    }

    /// 카테고리별 능력 개수 반환
    pub fn count_by_category(&self, category: AbilityCategory) -> usize {
        self.abilities.iter().filter(|a| a.ability_type.category() == category).count()
    }

    /// 긍정적 능력만 반환
    pub fn positive_abilities(&self) -> Vec<&SpecialAbility> {
        self.abilities.iter().filter(|a| a.is_positive()).collect()
    }

    /// 부정적 능력만 반환
    pub fn negative_abilities(&self) -> Vec<&SpecialAbility> {
        self.abilities.iter().filter(|a| !a.is_positive()).collect()
    }
}

/// 조합 이력 기록
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CombinationRecord {
    pub date: chrono::NaiveDateTime,
    pub input_abilities: Vec<SpecialAbility>,
    pub output_ability: SpecialAbility,
    pub combination_type: CombinationType,
}

/// 조합 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombinationType {
    Automatic,  // 자동 조합 (트리거)
    Manual,     // 수동 조합 (사용자 선택)
    Punishment, // 부정적 자동 조합
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ability_tier_multipliers() {
        assert_eq!(AbilityTier::Bronze.effect_multiplier(), 1.5);
        assert_eq!(AbilityTier::Legend.effect_multiplier(), 13.5);
        assert_eq!(AbilityTier::Red.effect_multiplier(), -1.5);
    }

    #[test]
    fn test_special_ability_creation() {
        let ability = SpecialAbility::new(SpecialAbilityType::DribblingMaster, AbilityTier::Gold);
        assert_eq!(ability.ability_type, SpecialAbilityType::DribblingMaster);
        assert_eq!(ability.tier, AbilityTier::Gold);
        assert!(ability.is_positive());
        assert!(ability.can_combine());
    }

    #[test]
    fn test_ability_collection() {
        let mut collection = SpecialAbilityCollection::new();

        let dribbling =
            SpecialAbility::new(SpecialAbilityType::DribblingMaster, AbilityTier::Silver);
        let passing = SpecialAbility::new(SpecialAbilityType::PassingGenius, AbilityTier::Bronze);

        collection.add_ability(dribbling);
        collection.add_ability(passing);

        assert_eq!(collection.count_by_category(AbilityCategory::Technical), 2);
        assert_eq!(collection.positive_abilities().len(), 2);
        assert_eq!(collection.negative_abilities().len(), 0);
    }
}
