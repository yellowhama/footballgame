//! Stamina Configuration (P13 Prerequisite)

use serde::{Deserialize, Serialize};

/// Stamina 시스템 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaminaConfig {
    // === Decay Rates ===
    /// 기본 감소율 (per tick) (기본: 0.00002)
    pub base_decay_rate: f32,
    /// 스프린트 감소율 (per tick) (기본: 0.0004)
    pub sprint_decay_rate: f32,

    // === Action Costs ===
    /// 슛 stamina 비용 (기본: 0.008)
    pub shot_stamina_cost: f32,
    /// 태클 stamina 비용 (기본: 0.012)
    pub tackle_stamina_cost: f32,
    /// 헤더 stamina 비용 (기본: 0.010)
    pub header_stamina_cost: f32,
    /// 드리블 stamina 비용 (기본: 0.003)
    pub dribble_stamina_cost: f32,
    /// 패스 stamina 비용 (기본: 0.001)
    pub pass_stamina_cost: f32,
    /// 인터셉트 stamina 비용 (기본: 0.006)
    pub intercept_stamina_cost: f32,
    /// 세이브 stamina 비용 (기본: 0.015)
    pub save_stamina_cost: f32,

    // === Stamina Attribute Impact ===
    /// 높은 stamina 능력치가 감소율에 미치는 영향 (기본: 0.5)
    pub stamina_attr_impact: f32,

    // === Sprint Detection ===
    /// 스프린트 판정 속도 비율 (max_speed의 몇 %) (기본: 0.8)
    pub sprint_speed_threshold: f32,
}

impl Default for StaminaConfig {
    fn default() -> Self {
        Self {
            base_decay_rate: 0.00002,
            sprint_decay_rate: 0.0004,

            shot_stamina_cost: 0.008,
            tackle_stamina_cost: 0.012,
            header_stamina_cost: 0.010,
            dribble_stamina_cost: 0.003,
            pass_stamina_cost: 0.001,
            intercept_stamina_cost: 0.006,
            save_stamina_cost: 0.015,

            stamina_attr_impact: 0.5,
            sprint_speed_threshold: 0.8,
        }
    }
}
