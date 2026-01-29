// 컨디션 시스템 - 5단계 관리
use rand::Rng;
use serde::{Deserialize, Serialize};

/// 5단계 컨디션 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Condition {
    /// 절호조 - 150% 효율
    PerfectForm,
    /// 호조 - 125% 효율
    GoodForm,
    /// 보통 - 100% 효율
    #[default]
    Normal,
    /// 부진 - 75% 효율
    PoorForm,
    /// 절부진 - 50% 효율
    TerribleForm,
}

impl Condition {
    /// 훈련 효율 배수 반환
    pub fn efficiency_multiplier(&self) -> f32 {
        match self {
            Condition::PerfectForm => 1.5,
            Condition::GoodForm => 1.25,
            Condition::Normal => 1.0,
            Condition::PoorForm => 0.75,
            Condition::TerribleForm => 0.5,
        }
    }

    /// 디스플레이용 텍스트
    pub fn display_text(&self) -> &'static str {
        match self {
            Condition::PerfectForm => "절호조 ⭐⭐⭐",
            Condition::GoodForm => "호조 ⭐⭐",
            Condition::Normal => "보통 ⭐",
            Condition::PoorForm => "부진 ⚠️",
            Condition::TerribleForm => "절부진 ❌",
        }
    }

    /// 이모지만 반환
    pub fn emoji(&self) -> &'static str {
        match self {
            Condition::PerfectForm => "⭐⭐⭐",
            Condition::GoodForm => "⭐⭐",
            Condition::Normal => "⭐",
            Condition::PoorForm => "⚠️",
            Condition::TerribleForm => "❌",
        }
    }

    /// 체력과 최근 훈련 기록을 기반으로 컨디션 계산
    pub fn calculate(
        current_stamina: u8,
        consecutive_training_days: u8,
        consecutive_rest_days: u8,
        rng: &mut impl Rng,
    ) -> Self {
        // 기본 확률 설정
        let base_roll = rng.gen_range(0.0..1.0);

        // 체력 기반 보정
        let stamina_bonus = match current_stamina {
            80..=100 => 0.1,  // 체력 좋으면 보너스
            60..=79 => 0.0,   // 보통
            40..=59 => -0.05, // 약간 페널티
            20..=39 => -0.1,  // 페널티
            _ => -0.2,        // 큰 페널티
        };

        // 연속 훈련/휴식 보정
        let training_penalty = if consecutive_training_days >= 5 {
            -0.15 // 과도한 훈련
        } else if consecutive_training_days >= 3 {
            -0.05 // 약간 누적
        } else {
            0.0
        };

        let rest_bonus = if consecutive_rest_days >= 3 {
            0.15 // 충분한 휴식
        } else if consecutive_rest_days >= 2 {
            0.05 // 약간 휴식
        } else {
            0.0
        };

        let final_roll = base_roll + stamina_bonus + training_penalty + rest_bonus;

        // 확률에 따른 컨디션 결정
        match final_roll {
            x if x >= 0.95 => Condition::PerfectForm, // 5% (조정후)
            x if x >= 0.80 => Condition::GoodForm,    // 15%
            x if x >= 0.30 => Condition::Normal,      // 50%
            x if x >= 0.10 => Condition::PoorForm,    // 20%
            _ => Condition::TerribleForm,             // 10%
        }
    }

    /// 컨디션 개선 시도 (휴식이나 특별 관리)
    pub fn try_improve(&self, rng: &mut impl Rng) -> Self {
        let improvement_chance = rng.gen_range(0.0..1.0);

        match self {
            Condition::TerribleForm => {
                if improvement_chance > 0.6 {
                    Condition::PoorForm
                } else {
                    *self
                }
            }
            Condition::PoorForm => {
                if improvement_chance > 0.5 {
                    Condition::Normal
                } else if improvement_chance < 0.1 {
                    Condition::TerribleForm
                } else {
                    *self
                }
            }
            Condition::Normal => {
                if improvement_chance > 0.7 {
                    Condition::GoodForm
                } else if improvement_chance < 0.1 {
                    Condition::PoorForm
                } else {
                    *self
                }
            }
            Condition::GoodForm => {
                if improvement_chance > 0.9 {
                    Condition::PerfectForm
                } else if improvement_chance < 0.2 {
                    Condition::Normal
                } else {
                    *self
                }
            }
            Condition::PerfectForm => {
                if improvement_chance < 0.3 {
                    Condition::GoodForm
                } else {
                    *self
                }
            }
        }
    }

    /// FIX_2601/0123: Condition improvement with determination bonus
    ///
    /// High determination players have better chance of recovering from poor form.
    /// - determination 100: +15% improvement chance
    /// - determination 50: +0% (baseline)
    /// - determination 20: -6% improvement chance
    pub fn try_improve_with_determination(&self, determination: u8, rng: &mut impl Rng) -> Self {
        // Calculate determination bonus: -0.06 to +0.15
        let det_bonus = (determination as f32 - 50.0) / 100.0 * 0.3;
        let improvement_chance = rng.gen_range(0.0..1.0) + det_bonus;

        match self {
            Condition::TerribleForm => {
                if improvement_chance > 0.6 {
                    Condition::PoorForm
                } else {
                    *self
                }
            }
            Condition::PoorForm => {
                if improvement_chance > 0.5 {
                    Condition::Normal
                } else if improvement_chance < 0.1 - det_bonus {
                    // Harder to get worse with high determination
                    Condition::TerribleForm
                } else {
                    *self
                }
            }
            Condition::Normal => {
                if improvement_chance > 0.7 {
                    Condition::GoodForm
                } else if improvement_chance < 0.1 - det_bonus {
                    Condition::PoorForm
                } else {
                    *self
                }
            }
            Condition::GoodForm => {
                if improvement_chance > 0.9 {
                    Condition::PerfectForm
                } else if improvement_chance < 0.2 - det_bonus {
                    Condition::Normal
                } else {
                    *self
                }
            }
            Condition::PerfectForm => {
                if improvement_chance < 0.3 - det_bonus {
                    // High determination helps maintain peak form
                    Condition::GoodForm
                } else {
                    *self
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efficiency_multipliers() {
        assert_eq!(Condition::PerfectForm.efficiency_multiplier(), 1.5);
        assert_eq!(Condition::Normal.efficiency_multiplier(), 1.0);
        assert_eq!(Condition::TerribleForm.efficiency_multiplier(), 0.5);
    }

    #[test]
    fn test_condition_calculation() {
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let mut rng = ChaCha8Rng::seed_from_u64(42);

        // 높은 체력, 적당한 훈련
        let condition = Condition::calculate(90, 2, 0, &mut rng);
        assert!(matches!(condition, Condition::Normal | Condition::GoodForm));

        // 낮은 체력, 과도한 훈련 - 확률적이므로 Normal도 허용
        let condition = Condition::calculate(20, 6, 0, &mut rng);
        assert!(matches!(
            condition,
            Condition::Normal | Condition::PoorForm | Condition::TerribleForm
        ));

        // 충분한 휴식 후
        let condition = Condition::calculate(80, 0, 4, &mut rng);
        assert!(matches!(
            condition,
            Condition::Normal | Condition::GoodForm | Condition::PerfectForm
        ));
    }
}
