// 코치 카드 데이터 구조
use serde::{Deserialize, Serialize};

/// 카드 레어도 (⭐1~5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardRarity {
    One = 1,   // ⭐ 일반 (50%)
    Two = 2,   // ⭐⭐ 고급 (25%)
    Three = 3, // ⭐⭐⭐ 희귀 (15%)
    Four = 4,  // ⭐⭐⭐⭐ 영웅 (7%)
    Five = 5,  // ⭐⭐⭐⭐⭐ 전설 (3%)
}

impl CardRarity {
    /// 레어도별 기본 보너스 효과
    pub fn base_bonus(&self) -> f32 {
        match self {
            CardRarity::One => 1.10,   // 10% 보너스
            CardRarity::Two => 1.15,   // 15% 보너스
            CardRarity::Three => 1.20, // 20% 보너스
            CardRarity::Four => 1.30,  // 30% 보너스
            CardRarity::Five => 1.50,  // 50% 보너스
        }
    }

    /// 레어도 이모지
    pub fn emoji(&self) -> &'static str {
        match self {
            CardRarity::One => "⭐",
            CardRarity::Two => "⭐⭐",
            CardRarity::Three => "⭐⭐⭐",
            CardRarity::Four => "⭐⭐⭐⭐",
            CardRarity::Five => "⭐⭐⭐⭐⭐",
        }
    }

    /// 레어도 색상 (터미널용)
    pub fn color(&self) -> &'static str {
        match self {
            CardRarity::One => "gray",
            CardRarity::Two => "green",
            CardRarity::Three => "blue",
            CardRarity::Four => "purple",
            CardRarity::Five => "orange",
        }
    }
}

/// 카드 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardType {
    Manager, // 감독
    Coach,   // 코치
    Tactics, // 전술
}

/// 전문 분야
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Specialty {
    Speed,     // 스피드 (PACE 계열)
    Power,     // 피지컬 (POWER 계열)
    Technical, // 기술 (TECHNICAL 계열)
    Mental,    // 정신력 (MENTAL 계열)
    Balanced,  // 밸런스 (모든 영역)
}

impl Specialty {
    /// 훈련 타겟과 매칭 확인
    pub fn matches_training(&self, training: &crate::training::TrainingTarget) -> bool {
        use crate::training::TrainingTarget;
        match (self, training) {
            (Specialty::Speed, TrainingTarget::Pace) => true,
            (Specialty::Power, TrainingTarget::Power | TrainingTarget::Endurance) => true,
            (Specialty::Technical, TrainingTarget::Technical | TrainingTarget::Passing) => true,
            (Specialty::Mental, TrainingTarget::Mental) => true,
            (Specialty::Balanced, _) => true, // 밸런스는 모든 훈련과 매칭
            _ => false,
        }
    }

    /// 전문 분야 아이콘
    pub fn icon(&self) -> &'static str {
        match self {
            Specialty::Speed => "🏃",
            Specialty::Power => "💪",
            Specialty::Technical => "⚽",
            Specialty::Mental => "🧠",
            Specialty::Balanced => "⚖️",
        }
    }
}

/// 코치 카드
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachCard {
    /// 카드 고유 ID
    pub id: String,
    /// 카드 이름
    pub name: String,
    /// 레어도
    pub rarity: CardRarity,
    /// 카드 타입
    pub card_type: CardType,
    /// 전문 분야
    pub specialty: Specialty,
    /// 현재 레벨 (1-10)
    pub level: u8,
    /// 현재 경험치
    pub experience: u32,
    /// 사용 횟수
    pub use_count: u32,
    /// 카드 설명
    pub description: String,
    /// 특수 능력 (옵션)
    pub special_ability: Option<SpecialAbility>,
}

/// 특수 능력
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialAbility {
    pub name: String,
    pub description: String,
    pub effect_type: EffectType,
    pub value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectType {
    InjuryPrevention,     // 부상 방지
    GrowthBoost,          // 성장 가속
    StaminaRecovery,      // 체력 회복
    ConditionImprovement, // 컨디션 개선
    ExperienceBonus,      // 경험치 보너스
}

impl CoachCard {
    /// 새 카드 생성
    pub fn new(
        id: String,
        name: String,
        rarity: CardRarity,
        card_type: CardType,
        specialty: Specialty,
        description: String,
    ) -> Self {
        Self {
            id,
            name,
            rarity,
            card_type,
            specialty,
            level: 1,
            experience: 0,
            use_count: 0,
            description,
            special_ability: None,
        }
    }

    /// 현재 보너스 계산 (레벨 포함)
    pub fn current_bonus(&self) -> f32 {
        let base = self.rarity.base_bonus();
        let level_bonus = 1.0 + (0.05 * (self.level - 1) as f32); // 레벨당 5% 증가
        base * level_bonus
    }

    /// 경험치 추가
    pub fn add_experience(&mut self, exp: u32) {
        if self.level >= 10 {
            return; // 최대 레벨
        }

        self.experience += exp;
        self.check_level_up();
    }

    /// 레벨업 체크
    pub fn check_level_up(&mut self) {
        while self.level < 10 {
            let required = self.required_experience();
            if self.experience >= required {
                self.experience -= required;
                self.level += 1;
            } else {
                break;
            }
        }
    }

    /// 필요 경험치
    pub fn required_experience(&self) -> u32 {
        100 * self.level as u32
    }

    /// 레벨업 진행률
    pub fn level_progress(&self) -> f32 {
        if self.level >= 10 {
            return 1.0;
        }
        self.experience as f32 / self.required_experience() as f32
    }

    /// 카드 사용 기록
    pub fn record_use(&mut self) {
        self.use_count += 1;
        self.add_experience(10); // 사용당 10 경험치
    }

    /// 카드 표시 문자열
    pub fn display(&self) -> String {
        format!(
            "{} {} {} Lv.{} [{}]",
            self.rarity.emoji(),
            self.name,
            self.specialty.icon(),
            self.level,
            match self.card_type {
                CardType::Manager => "감독",
                CardType::Coach => "코치",
                CardType::Tactics => "전술",
            }
        )
    }
}

/// 기본 카드 생성 (빈 슬롯용)
pub fn create_default_coach(specialty: Specialty) -> CoachCard {
    CoachCard::new(
        format!("default_{:?}", specialty).to_lowercase(),
        "기본 코치".to_string(),
        CardRarity::One,
        CardType::Coach,
        specialty,
        "기본적인 훈련 지원을 제공합니다.".to_string(),
    )
}

pub fn create_default_manager() -> CoachCard {
    CoachCard::new(
        "default_manager".to_string(),
        "기본 감독".to_string(),
        CardRarity::One,
        CardType::Manager,
        Specialty::Balanced,
        "기본적인 팀 관리를 제공합니다.".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_creation() {
        let card = CoachCard::new(
            "test_001".to_string(),
            "테스트 코치".to_string(),
            CardRarity::Three,
            CardType::Coach,
            Specialty::Speed,
            "빠른 선수 육성 전문".to_string(),
        );

        assert_eq!(card.level, 1);
        assert_eq!(card.rarity, CardRarity::Three);
        assert_eq!(card.current_bonus(), 1.20); // ⭐3 기본 보너스
    }

    #[test]
    fn test_level_up() {
        let mut card = create_default_coach(Specialty::Power);
        card.add_experience(100); // 레벨 2 필요 경험치

        assert_eq!(card.level, 2);
        assert_eq!(card.experience, 0);
        assert_eq!(card.current_bonus(), 1.10 * 1.05); // 레벨 2 보너스
    }

    #[test]
    fn test_specialty_matching() {
        use crate::training::TrainingTarget;

        assert!(Specialty::Speed.matches_training(&TrainingTarget::Pace));
        assert!(Specialty::Power.matches_training(&TrainingTarget::Power));
        assert!(Specialty::Balanced.matches_training(&TrainingTarget::Technical));
        assert!(!Specialty::Speed.matches_training(&TrainingTarget::Mental));
    }
}
