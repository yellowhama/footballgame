// 전술 카드 시스템
use super::card::{CardRarity, CardType};
use serde::{Deserialize, Serialize};

/// 전술 스타일 (match_engine의 TacticalStyle과 연동)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TacticalStyle {
    Defensive,     // 수비적
    Balanced,      // 균형
    Attacking,     // 공격적
    CounterAttack, // 역습
    Possession,    // 점유율
    Pressing,      // 압박
    DirectPlay,    // 직접 플레이
    WingPlay,      // 측면 공격
}

impl TacticalStyle {
    /// 전술 스타일 아이콘
    pub fn icon(&self) -> &'static str {
        match self {
            TacticalStyle::Defensive => "🛡️",
            TacticalStyle::Balanced => "⚖️",
            TacticalStyle::Attacking => "⚔️",
            TacticalStyle::CounterAttack => "🎯",
            TacticalStyle::Possession => "⚽",
            TacticalStyle::Pressing => "🏃",
            TacticalStyle::DirectPlay => "⬆️",
            TacticalStyle::WingPlay => "➡️",
        }
    }

    /// 전술 설명
    pub fn description(&self) -> &'static str {
        match self {
            TacticalStyle::Defensive => "수비 중심, 안정적인 경기 운영",
            TacticalStyle::Balanced => "공수 균형, 유연한 전환",
            TacticalStyle::Attacking => "공격 중심, 적극적인 전진",
            TacticalStyle::CounterAttack => "빠른 역습, 효율적인 공격",
            TacticalStyle::Possession => "점유율 중시, 패스 위주",
            TacticalStyle::Pressing => "전방 압박, 높은 수비라인",
            TacticalStyle::DirectPlay => "다이렉트 패스, 빠른 전개",
            TacticalStyle::WingPlay => "측면 활용, 크로스 중심",
        }
    }

    /// 전술별 기본 보너스
    pub fn base_bonuses(&self) -> Vec<(String, f32)> {
        match self {
            TacticalStyle::Defensive => {
                vec![("defense".to_string(), 0.2), ("stamina_save".to_string(), 0.1)]
            }
            TacticalStyle::Attacking => {
                vec![("offense".to_string(), 0.2), ("scoring_chance".to_string(), 0.15)]
            }
            TacticalStyle::Possession => {
                vec![("pass_accuracy".to_string(), 0.15), ("ball_retention".to_string(), 0.2)]
            }
            TacticalStyle::CounterAttack => {
                vec![("counter_speed".to_string(), 0.25), ("transition".to_string(), 0.15)]
            }
            TacticalStyle::Pressing => {
                vec![("ball_recovery".to_string(), 0.2), ("opponent_mistakes".to_string(), 0.15)]
            }
            TacticalStyle::Balanced => {
                vec![("adaptability".to_string(), 0.1), ("consistency".to_string(), 0.1)]
            }
            TacticalStyle::DirectPlay => {
                vec![("attack_speed".to_string(), 0.2), ("long_pass".to_string(), 0.15)]
            }
            TacticalStyle::WingPlay => {
                vec![("wing_attack".to_string(), 0.2), ("cross_accuracy".to_string(), 0.15)]
            }
        }
    }
}

/// 전술 카드
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticsCard {
    /// 카드 고유 ID
    pub id: String,
    /// 카드 이름
    pub name: String,
    /// 레어도
    pub rarity: CardRarity,
    /// 카드 타입 (항상 Tactics)
    pub card_type: CardType,
    /// 전술 스타일
    pub tactical_style: TacticalStyle,
    /// 현재 레벨 (1-10)
    pub level: u8,
    /// 현재 경험치
    pub experience: u32,
    /// 사용 횟수
    pub use_count: u32,
    /// 카드 설명
    pub description: String,
    /// 특수 효과 (옵션)
    pub special_effect: Option<TacticalEffect>,
    /// 콤보 가능한 전술들
    pub combo_tactics: Vec<TacticalStyle>,
}

/// 전술 특수 효과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalEffect {
    pub name: String,
    pub description: String,
    pub effect_type: TacticalEffectType,
    pub value: f32,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TacticalEffectType {
    FormationBonus,     // 포메이션 보너스
    WeatherAdaptation,  // 날씨 적응
    HomefieldAdvantage, // 홈 어드밴티지
    MomentumShift,      // 모멘텀 변화
    ClutchPerformance,  // 중요 순간 성능
    SetPieceBonus,      // 세트피스 보너스
    SubstitutionBoost,  // 교체 선수 부스트
    ComebackBonus,      // 역전 보너스
}

impl TacticsCard {
    /// 새 전술 카드 생성
    pub fn new(
        id: String,
        name: String,
        rarity: CardRarity,
        tactical_style: TacticalStyle,
        description: String,
    ) -> Self {
        Self {
            id,
            name,
            rarity,
            card_type: CardType::Tactics,
            tactical_style,
            level: 1,
            experience: 0,
            use_count: 0,
            description,
            special_effect: None,
            combo_tactics: Vec::new(),
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

    /// 전술 콤보 체크
    pub fn has_combo_with(&self, other_style: TacticalStyle) -> bool {
        self.combo_tactics.contains(&other_style)
    }

    /// 전술 효과 적용 가능 여부
    pub fn can_apply_effect(&self, condition: &str) -> bool {
        if let Some(effect) = &self.special_effect {
            if let Some(effect_condition) = &effect.condition {
                return effect_condition == condition;
            }
            true // 조건이 없으면 항상 적용
        } else {
            false
        }
    }
}

/// 전술 콤보 시스템
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalCombo {
    pub name: String,
    pub required_tactics: Vec<TacticalStyle>,
    pub bonus_type: String,
    pub bonus_value: f32,
    pub description: String,
}

impl TacticalCombo {
    /// 콤보 확인
    pub fn is_active(&self, active_tactics: &[TacticalStyle]) -> bool {
        self.required_tactics.iter().all(|req| active_tactics.contains(req))
    }
}

/// 사전 정의된 전술 콤보들
pub fn get_predefined_combos() -> Vec<TacticalCombo> {
    vec![
        TacticalCombo {
            name: "Total Football".to_string(),
            required_tactics: vec![TacticalStyle::Possession, TacticalStyle::Pressing],
            bonus_type: "team_coordination".to_string(),
            bonus_value: 0.25,
            description: "점유율과 압박을 조합한 토탈 풋볼".to_string(),
        },
        TacticalCombo {
            name: "Park the Bus".to_string(),
            required_tactics: vec![TacticalStyle::Defensive, TacticalStyle::CounterAttack],
            bonus_type: "defensive_stability".to_string(),
            bonus_value: 0.3,
            description: "극수비 후 역습 전술".to_string(),
        },
        TacticalCombo {
            name: "Gegenpress".to_string(),
            required_tactics: vec![TacticalStyle::Pressing, TacticalStyle::DirectPlay],
            bonus_type: "transition_speed".to_string(),
            bonus_value: 0.25,
            description: "강한 압박과 빠른 전개".to_string(),
        },
        TacticalCombo {
            name: "Tiki-Taka".to_string(),
            required_tactics: vec![TacticalStyle::Possession, TacticalStyle::Attacking],
            bonus_type: "passing_network".to_string(),
            bonus_value: 0.3,
            description: "짧은 패스 위주의 공격적 점유".to_string(),
        },
        TacticalCombo {
            name: "Wing Overload".to_string(),
            required_tactics: vec![TacticalStyle::WingPlay, TacticalStyle::Attacking],
            bonus_type: "wing_dominance".to_string(),
            bonus_value: 0.25,
            description: "측면 집중 공격".to_string(),
        },
    ]
}

/// 전술 카드 생성용 템플릿
pub fn get_tactics_templates() -> Vec<(String, TacticalStyle, CardRarity, String)> {
    vec![
        // 1성 전술들
        (
            "기본 수비".to_string(),
            TacticalStyle::Defensive,
            CardRarity::One,
            "기초적인 수비 전술".to_string(),
        ),
        (
            "기본 공격".to_string(),
            TacticalStyle::Attacking,
            CardRarity::One,
            "기초적인 공격 전술".to_string(),
        ),
        // 2성 전술들
        (
            "안정적 수비".to_string(),
            TacticalStyle::Defensive,
            CardRarity::Two,
            "개선된 수비 조직력".to_string(),
        ),
        (
            "빠른 역습".to_string(),
            TacticalStyle::CounterAttack,
            CardRarity::Two,
            "효율적인 역습 전술".to_string(),
        ),
        // 3성 전술들
        (
            "볼 점유".to_string(),
            TacticalStyle::Possession,
            CardRarity::Three,
            "패스 중심의 점유 축구".to_string(),
        ),
        (
            "전방 압박".to_string(),
            TacticalStyle::Pressing,
            CardRarity::Three,
            "적극적인 전방 압박".to_string(),
        ),
        // 4성 전술들
        (
            "티키타카".to_string(),
            TacticalStyle::Possession,
            CardRarity::Four,
            "바르셀로나식 짧은 패스 축구".to_string(),
        ),
        (
            "게겐프레싱".to_string(),
            TacticalStyle::Pressing,
            CardRarity::Four,
            "클롭식 강력한 압박 축구".to_string(),
        ),
        // 5성 전술들
        (
            "토탈 풋볼".to_string(),
            TacticalStyle::Balanced,
            CardRarity::Five,
            "크루이프의 혁명적 전술".to_string(),
        ),
        (
            "카테나치오".to_string(),
            TacticalStyle::Defensive,
            CardRarity::Five,
            "이탈리아 전통의 철벽 수비".to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tactics_card_creation() {
        let card = TacticsCard::new(
            "tactics_001".to_string(),
            "티키타카".to_string(),
            CardRarity::Four,
            TacticalStyle::Possession,
            "짧은 패스 위주의 점유 축구".to_string(),
        );

        assert_eq!(card.card_type, CardType::Tactics);
        assert_eq!(card.level, 1);
        assert_eq!(card.tactical_style, TacticalStyle::Possession);
    }

    #[test]
    fn test_tactical_combo() {
        let combo = TacticalCombo {
            name: "Total Football".to_string(),
            required_tactics: vec![TacticalStyle::Possession, TacticalStyle::Pressing],
            bonus_type: "team_coordination".to_string(),
            bonus_value: 0.25,
            description: "점유율과 압박을 조합한 토탈 풋볼".to_string(),
        };

        let active_tactics =
            vec![TacticalStyle::Possession, TacticalStyle::Pressing, TacticalStyle::Attacking];
        assert!(combo.is_active(&active_tactics));

        let inactive_tactics = vec![TacticalStyle::Defensive, TacticalStyle::CounterAttack];
        assert!(!combo.is_active(&inactive_tactics));
    }

    #[test]
    fn test_level_progression() {
        let mut card = TacticsCard::new(
            "tactics_002".to_string(),
            "Test Tactics".to_string(),
            CardRarity::Three,
            TacticalStyle::Balanced,
            "Test description".to_string(),
        );

        // 경험치 추가 테스트
        card.add_experience(100); // 레벨 2로 상승
        assert_eq!(card.level, 2);
        assert_eq!(card.experience, 0);

        card.add_experience(250); // 레벨 3로 상승, 50 남음
        assert_eq!(card.level, 3);
        assert_eq!(card.experience, 50);
    }
}
