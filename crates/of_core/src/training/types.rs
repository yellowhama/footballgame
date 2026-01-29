// 훈련 시스템 기본 타입 정의
use serde::{Deserialize, Serialize};

/// 훈련 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrainingType {
    /// 팀 훈련 (감독 지정)
    Team,
    /// 개인 훈련 (선수 선택)
    Individual,
    /// 특별 훈련 (이벤트)
    Special,
}

/// 훈련 대상 능력치 (6각형)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrainingTarget {
    /// PACE - 속도, 가속력
    Pace,
    /// POWER - 근력, 점프력
    Power,
    /// TECHNICAL - 기술, 볼컨트롤
    Technical,
    /// SHOOTING - 슈팅, 마무리
    Shooting,
    /// PASSING - 패스, 시야
    Passing,
    /// DEFENDING - 수비, 태클
    Defending,
    /// MENTAL - 정신력, 집중력 (추가)
    Mental,
    /// ENDURANCE - 지구력, 체력 (추가)
    Endurance,
    /// BALANCED - 균형잡힌 성장 (팀훈련용)
    Balanced,
}

impl TrainingTarget {
    /// 디스플레이 이름
    pub fn display_name(&self) -> &'static str {
        match self {
            TrainingTarget::Pace => "스피드 훈련",
            TrainingTarget::Power => "근력 훈련",
            TrainingTarget::Technical => "기술 훈련",
            TrainingTarget::Shooting => "슈팅 훈련",
            TrainingTarget::Passing => "패스 훈련",
            TrainingTarget::Defending => "수비 훈련",
            TrainingTarget::Mental => "정신력 훈련",
            TrainingTarget::Endurance => "지구력 훈련",
            TrainingTarget::Balanced => "종합 훈련",
        }
    }

    /// 아이콘/이모지
    pub fn icon(&self) -> &'static str {
        match self {
            TrainingTarget::Pace => "🏃",
            TrainingTarget::Power => "💪",
            TrainingTarget::Technical => "⚡",
            TrainingTarget::Shooting => "⚽",
            TrainingTarget::Passing => "🎯",
            TrainingTarget::Defending => "🛡️",
            TrainingTarget::Mental => "🧠",
            TrainingTarget::Endurance => "❤️",
            TrainingTarget::Balanced => "⚖️",
        }
    }

    /// 영향받는 속성들 반환 (속성명, 가중치)
    pub fn affected_attributes(&self) -> Vec<(&'static str, f32)> {
        match self {
            TrainingTarget::Pace => {
                vec![("pace", 1.5), ("acceleration", 1.5), ("agility", 1.0), ("balance", 0.5)]
            }
            TrainingTarget::Power => vec![
                ("strength", 1.5),
                ("jumping", 1.5),
                ("heading", 1.0),
                ("natural_fitness", 0.5),
            ],
            TrainingTarget::Technical => {
                vec![("technique", 1.5), ("first_touch", 1.5), ("dribbling", 1.2), ("flair", 0.5)]
            }
            TrainingTarget::Shooting => vec![
                ("finishing", 1.5),
                ("long_shots", 1.2),
                ("composure", 0.8),
                ("technique", 0.5),
            ],
            TrainingTarget::Passing => vec![
                ("passing", 1.5),
                ("vision", 1.5),
                ("crossing", 1.0),
                ("technique", 0.8),
                ("teamwork", 0.5),
            ],
            TrainingTarget::Defending => vec![
                ("tackling", 1.5),
                ("marking", 1.5),
                ("positioning", 1.2),
                ("concentration", 1.0),
                ("aggression", 0.5),
            ],
            TrainingTarget::Mental => vec![
                ("concentration", 1.2),
                ("composure", 1.2),
                ("decisions", 1.2),
                ("anticipation", 1.0),
                ("leadership", 0.8),
                ("determination", 0.8),
            ],
            TrainingTarget::Endurance => vec![
                ("stamina", 1.5),
                ("natural_fitness", 1.5),
                ("work_rate", 1.0),
                ("balance", 0.5),
            ],
            TrainingTarget::Balanced => vec![
                ("technique", 0.8),
                ("passing", 0.8),
                ("tackling", 0.8),
                ("pace", 0.6),
                ("strength", 0.6),
                ("stamina", 0.6),
                ("concentration", 0.6),
                ("positioning", 0.6),
            ],
        }
    }
}

/// 코치 보너스 로그 (UI/리포트 용도)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachBonusLog {
    pub source: String,
    pub bonus_multiplier: f32,
    pub reason: String,
}

/// 훈련 세션
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSession {
    /// 훈련 타입
    pub training_type: TrainingType,
    /// 목표 능력치
    pub target: TrainingTarget,
    /// 훈련 강도
    pub intensity: crate::training::stamina::TrainingIntensity,
    /// 체력 소모량
    pub stamina_cost: u8,
    /// 기본 효과
    pub base_effect: f32,
    /// 코치 보정
    pub coach_bonus: f32,
}

impl TrainingSession {
    /// 새 훈련 세션 생성
    pub fn new(
        training_type: TrainingType,
        target: TrainingTarget,
        intensity: crate::training::stamina::TrainingIntensity,
    ) -> Self {
        let stamina_cost = intensity.stamina_cost();
        let base_effect = match training_type {
            TrainingType::Team => 0.8,       // 팀훈련은 효율 낮지만 안정적
            TrainingType::Individual => 1.2, // 개인훈련은 효율 높음
            TrainingType::Special => 1.5,    // 특별훈련은 매우 효율적
        };

        Self {
            training_type,
            target,
            intensity,
            stamina_cost,
            base_effect,
            coach_bonus: 1.0, // 기본값
        }
    }

    /// 실제 효과 계산 (컨디션, 체력 등 고려)
    pub fn calculate_effect(
        &self,
        condition: crate::training::condition::Condition,
        current_ca: u16,
        pa: u16,
    ) -> f32 {
        // 기본 효과
        let mut effect = self.base_effect;

        // 강도 보정
        effect *= self.intensity.effect_multiplier();

        // 컨디션 보정
        effect *= condition.efficiency_multiplier();

        // 코치 보정
        effect *= self.coach_bonus;

        // PA 한계 보정 (PA에 가까울수록 성장 둔화)
        let growth_factor = if current_ca >= pa {
            0.1 // PA 도달시 최소 성장
        } else {
            let ratio = current_ca as f32 / pa as f32;
            // 0% -> 100%, 100% -> 10% 성장률
            1.0 - (ratio * 0.9)
        };
        effect *= growth_factor;

        effect
    }
}

/// 훈련 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingResult {
    /// 수행한 훈련
    pub session: TrainingSession,
    /// 실제 효과
    pub actual_effect: f32,
    /// 상승한 속성들
    pub improved_attributes: Vec<(String, f32)>,
    /// CA 변화량
    pub ca_change: f32,
    /// 부상 발생 여부
    pub injury_occurred: bool,
    /// 피드백 메시지
    pub message: String,
    /// 코치 카드별 보너스 로그
    pub coach_bonus_log: Vec<CoachBonusLog>,
}

/// 부상 유형
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InjuryType {
    /// 근육 부상
    Muscle,
    /// 인대 부상
    Ligament,
    /// 골절
    Bone,
    /// 과로
    Fatigue,
    /// 타박상
    Bruise,
}

impl InjuryType {
    pub fn display_name(&self) -> &'static str {
        match self {
            InjuryType::Muscle => "근육 부상",
            InjuryType::Ligament => "인대 부상",
            InjuryType::Bone => "골절",
            InjuryType::Fatigue => "과로",
            InjuryType::Bruise => "타박상",
        }
    }

    /// 해당 부상이 영향을 미치는 속성들
    pub fn affected_attributes(&self) -> Vec<&'static str> {
        match self {
            InjuryType::Muscle => vec!["pace", "acceleration", "strength"],
            InjuryType::Ligament => vec!["agility", "balance"],
            InjuryType::Bone => vec!["jumping", "strength"],
            InjuryType::Fatigue => vec!["stamina", "work_rate"],
            InjuryType::Bruise => vec![],
        }
    }
}

/// 부상 심각도
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InjurySeverity {
    /// 경미: 1-3일
    Minor,
    /// 중간: 4-7일
    Moderate,
    /// 심각: 8-21일
    Serious,
    /// 중증: 22-60일
    Severe,
}

impl InjurySeverity {
    /// 회복 기간 범위 (최소, 최대)
    pub fn recovery_range(&self) -> (u8, u8) {
        match self {
            InjurySeverity::Minor => (1, 3),
            InjurySeverity::Moderate => (4, 7),
            InjurySeverity::Serious => (8, 21),
            InjurySeverity::Severe => (22, 60),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            InjurySeverity::Minor => "경미",
            InjurySeverity::Moderate => "중간",
            InjurySeverity::Serious => "심각",
            InjurySeverity::Severe => "중증",
        }
    }
}

/// 부상 정보
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Injury {
    /// 부상 유형
    pub injury_type: InjuryType,
    /// 심각도
    pub severity: InjurySeverity,
    /// 영향 받는 속성
    pub affected_attributes: Vec<String>,
    /// 총 회복 기간 (일)
    pub recovery_days_total: u8,
    /// 남은 회복 기간 (일)
    pub recovery_days_remaining: u8,
    /// 발생 날짜
    pub occurred_date: String,
}

impl Injury {
    /// 회복 진행률 (0.0 ~ 1.0)
    pub fn recovery_progress(&self) -> f32 {
        if self.recovery_days_total == 0 {
            return 1.0;
        }
        1.0 - (self.recovery_days_remaining as f32 / self.recovery_days_total as f32)
    }

    /// 회복 완료 여부
    pub fn is_recovered(&self) -> bool {
        self.recovery_days_remaining == 0
    }

    /// 하루 회복 진행
    pub fn advance_recovery(&mut self) -> bool {
        if self.recovery_days_remaining > 0 {
            self.recovery_days_remaining -= 1;
        }
        self.is_recovered()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_target_attributes() {
        let pace_attrs = TrainingTarget::Pace.affected_attributes();
        assert!(pace_attrs.iter().any(|(name, _)| *name == "pace"));
        assert!(pace_attrs.iter().any(|(name, _)| *name == "acceleration"));

        let balanced_attrs = TrainingTarget::Balanced.affected_attributes();
        assert!(balanced_attrs.len() >= 6); // 균형잡힌 훈련은 여러 속성
    }

    #[test]
    fn test_training_session_effect() {
        use crate::training::condition::Condition;
        use crate::training::stamina::TrainingIntensity;

        let session = TrainingSession::new(
            TrainingType::Individual,
            TrainingTarget::Pace,
            TrainingIntensity::Normal,
        );

        // 정상 컨디션, CA 80, PA 120
        let effect = session.calculate_effect(Condition::Normal, 80, 120);
        assert!(effect > 0.0 && effect <= 1.2);

        // 절호조 컨디션
        let effect_perfect = session.calculate_effect(Condition::PerfectForm, 80, 120);
        assert!(effect_perfect > effect); // 컨디션 좋으면 효과 증가

        // PA 도달 상황
        let effect_pa_reached = session.calculate_effect(Condition::Normal, 120, 120);
        assert!(effect_pa_reached < 0.2); // PA 도달시 성장 둔화
    }
}
