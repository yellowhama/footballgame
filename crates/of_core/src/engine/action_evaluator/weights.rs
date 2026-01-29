//! Weight Calculator
//!
//! FIX_2601/0108: 포지션/성향/전술에 따른 가중치 계산
//! Multiplicative 방식 사용 (additive 아님)

use super::state::PlayerPhaseState;
use super::types::{ActionWeights, WeightMultiplier};

/// 포지션 문자열 (lifetime 유연하게)
pub type PositionStr<'a> = &'a str;

/// 가중치 계산기
pub struct WeightCalculator;

impl WeightCalculator {
    /// 포지션별 기본 가중치
    pub fn for_position(position: &str) -> ActionWeights {
        match position {
            "GK" => ActionWeights {
                distance: 0.15,
                safety: 0.40, // 안전 최우선
                readiness: 0.20,
                progression: 0.05,
                space: 0.10,
                tactical: 0.10,
            },
            "CB" => ActionWeights {
                distance: 0.20,
                safety: 0.32, // 안전 중시
                readiness: 0.15,
                progression: 0.13,
                space: 0.10,
                tactical: 0.10,
            },
            "FB" | "LB" | "RB" | "WB" => ActionWeights {
                distance: 0.20,
                safety: 0.25,
                readiness: 0.15,
                progression: 0.20,
                space: 0.10,
                tactical: 0.10,
            },
            "DM" | "CDM" => ActionWeights {
                distance: 0.20,
                safety: 0.28, // 안전+전술
                readiness: 0.15,
                progression: 0.17,
                space: 0.10,
                tactical: 0.10,
            },
            "CM" => ActionWeights {
                distance: 0.20,
                safety: 0.25,
                readiness: 0.15,
                progression: 0.20,
                space: 0.10,
                tactical: 0.10,
            },
            "AM" | "CAM" => ActionWeights {
                distance: 0.18,
                safety: 0.20,
                readiness: 0.15,
                progression: 0.27, // 전진 중시
                space: 0.10,
                tactical: 0.10,
            },
            "W" | "LW" | "RW" | "LM" | "RM" => ActionWeights {
                distance: 0.18,
                safety: 0.18,
                readiness: 0.15,
                progression: 0.24,
                space: 0.15, // 공간 중시
                tactical: 0.10,
            },
            "ST" | "CF" => ActionWeights {
                distance: 0.18,
                safety: 0.15,
                readiness: 0.15,
                progression: 0.30, // 골 최우선
                space: 0.12,
                tactical: 0.10,
            },
            _ => ActionWeights::default(),
        }
    }

    /// 성향별 배율
    pub fn trait_multiplier(trait_name: &str) -> WeightMultiplier {
        match trait_name {
            // 패스 성향
            "PlaysRiskyPasses" => WeightMultiplier {
                safety: 0.85,
                progression: 1.15,
                ..Default::default()
            },
            "TriesKillerBalls" => WeightMultiplier {
                safety: 0.80,
                tactical: 1.20,
                ..Default::default()
            },
            "PlaysItSafe" => WeightMultiplier {
                safety: 1.20,
                progression: 0.90,
                ..Default::default()
            },

            // 슈팅 성향
            "ShootsFromDistance" => WeightMultiplier {
                distance: 1.20, // 거리 허용 범위 확대
                ..Default::default()
            },
            "PlacesShots" => WeightMultiplier {
                readiness: 1.10,
                ..Default::default()
            },
            "ShootsWithPower" => WeightMultiplier {
                safety: 0.95,
                ..Default::default()
            },

            // 드리블 성향
            "TriesToBeatDefender" => WeightMultiplier {
                safety: 0.85,
                progression: 1.15,
                ..Default::default()
            },
            "RunsWithBallOften" => WeightMultiplier {
                space: 1.15,
                tactical: 1.10,
                ..Default::default()
            },
            "CutsInsideFromWing" => WeightMultiplier {
                tactical: 1.15,
                ..Default::default()
            },

            // 움직임 성향
            "StaysBack" => WeightMultiplier {
                progression: 0.70,
                safety: 1.30,
                ..Default::default()
            },
            "GetsForward" => WeightMultiplier {
                progression: 1.20,
                safety: 0.90,
                ..Default::default()
            },
            "TriesToBeatOffsideTrap" => WeightMultiplier {
                progression: 1.25,
                ..Default::default()
            },
            "MakesForwardRuns" => WeightMultiplier {
                progression: 1.15,
                ..Default::default()
            },

            // 수비 성향
            "DivesIntoTackles" => WeightMultiplier {
                safety: 0.75,
                ..Default::default()
            },
            "StayOnFeet" => WeightMultiplier {
                safety: 1.15,
                ..Default::default()
            },
            "MarksOpponentTightly" => WeightMultiplier {
                distance: 0.90, // 더 가까이 마킹
                ..Default::default()
            },

            // 기타
            "HoldsUpBall" => WeightMultiplier {
                space: 1.15,
                progression: 0.90,
                ..Default::default()
            },
            "DictatesTempo" => WeightMultiplier {
                tactical: 1.20,
                ..Default::default()
            },

            _ => WeightMultiplier::default(),
        }
    }

    /// 멘탈리티별 배율
    pub fn mentality_multiplier(mentality: &str) -> WeightMultiplier {
        match mentality {
            "VeryDefensive" => WeightMultiplier {
                safety: 1.25,
                progression: 0.80,
                ..Default::default()
            },
            "Defensive" => WeightMultiplier {
                safety: 1.15,
                progression: 0.90,
                ..Default::default()
            },
            "Balanced" => WeightMultiplier::default(),
            "Attacking" => WeightMultiplier {
                safety: 0.90,
                progression: 1.15,
                ..Default::default()
            },
            "VeryAttacking" => WeightMultiplier {
                safety: 0.80,
                progression: 1.25,
                ..Default::default()
            },
            _ => WeightMultiplier::default(),
        }
    }

    /// 패스 스타일별 배율
    pub fn passing_style_multiplier(style: &str) -> WeightMultiplier {
        match style {
            "Direct" => WeightMultiplier {
                progression: 1.15,
                ..Default::default()
            },
            "Short" => WeightMultiplier {
                safety: 1.15,
                ..Default::default()
            },
            _ => WeightMultiplier::default(),
        }
    }

    /// 템포별 배율
    pub fn tempo_multiplier(tempo: &str) -> WeightMultiplier {
        match tempo {
            "Fast" => WeightMultiplier {
                progression: 1.10,
                ..Default::default()
            },
            "Slow" => WeightMultiplier {
                safety: 1.10,
                ..Default::default()
            },
            _ => WeightMultiplier::default(),
        }
    }

    /// 상태별 추가 배율
    pub fn state_multiplier(state: PlayerPhaseState) -> WeightMultiplier {
        match state {
            PlayerPhaseState::TransitionWin => WeightMultiplier {
                progression: 1.30, // 역습 시 전진 우선
                safety: 0.85,
                ..Default::default()
            },
            PlayerPhaseState::TransitionLoss => WeightMultiplier {
                safety: 1.20, // 역압박 시 안전 우선
                progression: 0.80,
                ..Default::default()
            },
            PlayerPhaseState::DefensiveShape => WeightMultiplier {
                safety: 1.15,
                tactical: 1.10, // 형태 유지 중요
                ..Default::default()
            },
            _ => WeightMultiplier::default(),
        }
    }

    /// 최종 가중치 계산
    ///
    /// 1. 포지션 기본값
    /// 2. 성향 배율 적용 (multiplicative)
    /// 3. 전술 배율 적용 (multiplicative)
    /// 4. 상태 배율 적용 (multiplicative)
    /// 5. 클램프 [0.05, 0.60]
    pub fn calculate(
        position: &str,
        traits: &[&str],
        mentality: &str,
        passing_style: &str,
        tempo: &str,
        state: PlayerPhaseState,
    ) -> ActionWeights {
        // 1. 포지션 기본값
        let mut weights = Self::for_position(position);

        // 2. 성향 배율 (순차 적용)
        for trait_name in traits {
            weights.apply_multiplier(Self::trait_multiplier(trait_name));
        }

        // 3. 전술 배율
        weights.apply_multiplier(Self::mentality_multiplier(mentality));
        weights.apply_multiplier(Self::passing_style_multiplier(passing_style));
        weights.apply_multiplier(Self::tempo_multiplier(tempo));

        // 4. 상태 배율
        weights.apply_multiplier(Self::state_multiplier(state));

        // 5. 클램프
        weights.clamp_all(0.05, 0.60);

        weights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_weights() {
        let gk = WeightCalculator::for_position("GK");
        let st = WeightCalculator::for_position("ST");

        // GK는 safety가 높음
        assert!(gk.safety > st.safety);

        // ST는 progression이 높음
        assert!(st.progression > gk.progression);
    }

    #[test]
    fn test_trait_multiplier() {
        let mut weights = ActionWeights::default();
        let original_safety = weights.safety;

        weights.apply_multiplier(WeightCalculator::trait_multiplier("PlaysItSafe"));

        // PlaysItSafe: safety가 증가
        assert!(weights.safety > original_safety);
    }

    #[test]
    fn test_full_calculation() {
        let weights = WeightCalculator::calculate(
            "ST",
            &["ShootsFromDistance"],
            "Attacking",
            "Direct",
            "Fast",
            PlayerPhaseState::OnBall,
        );

        // ST + Attacking + Direct + Fast → progression이 매우 높아야 함
        assert!(weights.progression > 0.30);

        // 클램프 확인
        assert!(weights.safety >= 0.05);
        assert!(weights.safety <= 0.60);
    }
}
