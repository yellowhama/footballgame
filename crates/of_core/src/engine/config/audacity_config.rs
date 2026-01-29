//! Audacity Configuration (P14)

use serde::{Deserialize, Serialize};

/// Audacity 보정 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudacityConfig {
    // === Glory Bonus ===
    /// Glory bonus 스케일 (기본: 0.8)
    pub glory_bonus_scale: f32,
    /// High reward 판정 기준 (기본: 0.7)
    pub high_reward_threshold: f32,
    /// Low probability 판정 기준 (기본: 0.35)
    pub low_prob_threshold: f32,

    // === Risk Dampening ===
    /// Risk dampening 최대치 (기본: 0.7)
    pub risk_dampen_max: f32,
    /// Audacity → Risk dampen 가중치 (기본: 0.4)
    pub audacity_risk_dampen_weight: f32,
    /// Desperation → Risk dampen 가중치 (기본: 0.3)
    pub desperation_risk_dampen_weight: f32,

    // === Desperation Calculation ===
    /// 지고 있을 때 가중치 (기본: 0.7)
    pub losing_weight: f32,
    /// 후반 가중치 (기본: 0.3)
    pub late_game_weight: f32,
    /// 후반 시작 분 (기본: 70.0)
    pub late_game_start_minute: f32,

    // === Audacity Calculation ===
    /// Aggression → Audacity 가중치 (기본: 0.7)
    pub aggression_weight: f32,
    /// (1 - Decisions) → Audacity 가중치 (기본: 0.3)
    pub anti_decisions_weight: f32,

    // === Blending (alpha) ===
    /// Flair → alpha 가중치 (기본: 0.5)
    pub flair_alpha_weight: f32,
    /// Audacity → alpha 가중치 (기본: 0.3)
    pub audacity_alpha_weight: f32,
    /// Desperation → alpha 가중치 (기본: 0.2)
    pub desperation_alpha_weight: f32,
    /// alpha 최소값 (기본: 0.1)
    pub alpha_min: f32,
    /// alpha 최대값 (기본: 0.9)
    pub alpha_max: f32,

    // === Feature Flag ===
    /// Audacity 보정 사용 여부 (기본: true)
    pub use_audacity: bool,
}

impl Default for AudacityConfig {
    fn default() -> Self {
        Self {
            glory_bonus_scale: 0.8,
            high_reward_threshold: 0.7,
            low_prob_threshold: 0.35,

            risk_dampen_max: 0.7,
            audacity_risk_dampen_weight: 0.4,
            desperation_risk_dampen_weight: 0.3,

            losing_weight: 0.7,
            late_game_weight: 0.3,
            late_game_start_minute: 70.0,

            aggression_weight: 0.7,
            anti_decisions_weight: 0.3,

            flair_alpha_weight: 0.5,
            audacity_alpha_weight: 0.3,
            desperation_alpha_weight: 0.2,
            alpha_min: 0.1,
            alpha_max: 0.9,

            use_audacity: true,
        }
    }
}
