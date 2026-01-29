//! # Engine Configuration Module (P10-13 Phase 6)
//!
//! 모든 튜닝 상수를 중앙에서 관리하는 설정 시스템.
//!
//! ## 목적
//! - 튜닝 지옥(Tuning Hell) 방지
//! - 밸런스 조정 용이
//! - 프리셋 지원 (Realistic, Arcade, Simulation)
//!
//! ## 사용법
//! ```rust
//! use of_core::engine::config::EngineConfig;
//!
//! let config = EngineConfig::default();
//! let arcade = EngineConfig::arcade();
//! ```

mod audacity_config;
mod decision_config;
mod execution_config;
mod stamina_config;
mod thresholds_config;

pub use audacity_config::AudacityConfig;
pub use decision_config::DecisionConfig;
pub use execution_config::ExecutionConfig;
pub use stamina_config::StaminaConfig;
pub use thresholds_config::{
    DefenseThresholds, DuelThresholds, PhysicsThresholds, RuleThresholds, ThresholdsConfig,
};

use serde::{Deserialize, Serialize};

/// P12+P13+P14 전체 설정
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EngineConfig {
    /// EV Decision (P12) 설정
    pub decision: DecisionConfig,
    /// Execution Error (P13) 설정
    pub execution: ExecutionConfig,
    /// Audacity (P14) 설정
    pub audacity: AudacityConfig,
    /// Stamina 시스템 설정
    pub stamina: StaminaConfig,
    /// Rule thresholds (FIX_2601/0123 Phase 6)
    #[serde(default)]
    pub thresholds: ThresholdsConfig,
}

impl EngineConfig {
    /// 현실적인 시뮬레이션 (기본)
    pub fn realistic() -> Self {
        Self::default()
    }

    /// 아케이드 스타일 (더 공격적, 더 많은 골)
    pub fn arcade() -> Self {
        let mut cfg = Self::default();
        cfg.decision.goal_reward = 1.5;
        cfg.decision.loss_of_possession_base_cost = 0.1;
        cfg.audacity.glory_bonus_scale = 1.2;
        cfg.audacity.risk_dampen_max = 0.9;
        cfg.execution.shot_angle_sigma = 2.0; // 더 정확한 슛
        cfg.thresholds = ThresholdsConfig::arcade(); // FIX_2601/0123 Phase 6
        cfg
    }

    /// 시뮬레이션 스타일 (더 현실적, 더 적은 골)
    pub fn simulation() -> Self {
        let mut cfg = Self::default();
        cfg.decision.goal_reward = 0.8;
        cfg.decision.loss_of_possession_base_cost = 0.35;
        cfg.audacity.glory_bonus_scale = 0.5;
        cfg.execution.shot_angle_sigma = 14.0;
        cfg.execution.pressure_factor_weight = 0.9;
        cfg.thresholds = ThresholdsConfig::simulation(); // FIX_2601/0123 Phase 6
        cfg
    }

    /// 테스트용 (오차 최소화)
    pub fn deterministic() -> Self {
        let mut cfg = Self::default();
        cfg.execution.shot_angle_sigma = 0.001;
        cfg.execution.shot_power_sigma = 0.001;
        cfg.execution.pass_angle_sigma = 0.001;
        cfg.audacity.use_audacity = false;
        cfg
    }
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = EngineConfig::default();
        assert!((cfg.decision.goal_reward - 1.0).abs() < 0.01);
        assert!(cfg.audacity.use_audacity);
        assert!((cfg.stamina.base_decay_rate - 0.00002).abs() < 0.00001);
    }

    #[test]
    fn test_arcade_has_more_goals() {
        let realistic = EngineConfig::realistic();
        let arcade = EngineConfig::arcade();

        // Arcade should have higher goal reward
        assert!(arcade.decision.goal_reward > realistic.decision.goal_reward);
        // Lower loss cost
        assert!(
            arcade.decision.loss_of_possession_base_cost
                < realistic.decision.loss_of_possession_base_cost
        );
        // More accurate shots
        assert!(arcade.execution.shot_angle_sigma < realistic.execution.shot_angle_sigma);
    }

    #[test]
    fn test_simulation_more_realistic() {
        let realistic = EngineConfig::realistic();
        let simulation = EngineConfig::simulation();

        // Simulation should have lower goal reward
        assert!(simulation.decision.goal_reward < realistic.decision.goal_reward);
        // Higher loss cost
        assert!(
            simulation.decision.loss_of_possession_base_cost
                > realistic.decision.loss_of_possession_base_cost
        );
        // Less accurate shots
        assert!(simulation.execution.shot_angle_sigma > realistic.execution.shot_angle_sigma);
    }

    #[test]
    fn test_deterministic_minimal_error() {
        let det = EngineConfig::deterministic();
        assert!(det.execution.shot_angle_sigma < 0.01);
        assert!(!det.audacity.use_audacity);
    }

    #[test]
    fn test_config_serialization() {
        let cfg = EngineConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: EngineConfig = serde_json::from_str(&json).unwrap();
        assert!((parsed.decision.goal_reward - cfg.decision.goal_reward).abs() < 0.001);
    }
}
