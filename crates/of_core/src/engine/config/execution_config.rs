//! Execution Error Configuration (P13)

use serde::{Deserialize, Serialize};

/// 실행 오차 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    // === Base Sigma Values (per action) ===
    /// 슛 각도 오차 sigma (도) (기본: 3.0)
    pub shot_angle_sigma: f32,
    /// 슛 파워 오차 sigma (기본: 0.08)
    pub shot_power_sigma: f32,

    /// 패스 각도 오차 sigma (도) (기본: 2.5)
    pub pass_angle_sigma: f32,
    /// 패스 파워 오차 sigma (기본: 0.06)
    pub pass_power_sigma: f32,

    /// 크로스 각도 오차 sigma (도) (기본: 4.0)
    pub cross_angle_sigma: f32,
    /// 크로스 파워 오차 sigma (기본: 0.10)
    pub cross_power_sigma: f32,

    /// 퍼스트 터치 각도 오차 sigma (도) (기본: 2.0)
    pub first_touch_angle_sigma: f32,
    /// 퍼스트 터치 거리 오차 sigma (m) (기본: 0.15)
    pub first_touch_dist_sigma: f32,

    /// 세이브 각도 오차 sigma (도) (기본: 2.5)
    pub save_angle_sigma: f32,
    /// 세이브 거리 오차 sigma (기본: 0.08)
    pub save_dist_sigma: f32,

    // === Factor Weights ===
    /// 능력치 부족 → 오차 증가 가중치 (기본: 0.8)
    pub skill_factor_weight: f32,
    /// 압박 → 오차 증가 가중치 (기본: 0.6)
    pub pressure_factor_weight: f32,
    /// 피로 → 오차 증가 가중치 (기본: 0.4)
    pub fatigue_factor_weight: f32,
    /// 약발 → 오차 증가 가중치 (기본: 0.5)
    pub weak_foot_factor_weight: f32,
    /// 침착함 → 오차 감소 가중치 (기본: 0.4)
    pub calm_factor_weight: f32,

    // === First Touch Thresholds (meters) ===
    /// Perfect touch 최대 거리 (기본: 0.3)
    pub perfect_touch_dist: f32,
    /// Good touch 최대 거리 (기본: 0.8)
    pub good_touch_dist: f32,
    /// Heavy touch 최대 거리 (기본: 1.5)
    pub heavy_touch_dist: f32,
    // >= heavy_touch_dist → Loose ball
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            shot_angle_sigma: 3.0,
            shot_power_sigma: 0.08,

            pass_angle_sigma: 2.5,
            pass_power_sigma: 0.06,

            cross_angle_sigma: 4.0,
            cross_power_sigma: 0.10,

            first_touch_angle_sigma: 2.0,
            first_touch_dist_sigma: 0.15,

            save_angle_sigma: 2.5,
            save_dist_sigma: 0.08,

            skill_factor_weight: 0.8,
            pressure_factor_weight: 0.6,
            fatigue_factor_weight: 0.4,
            weak_foot_factor_weight: 0.5,
            calm_factor_weight: 0.4,

            perfect_touch_dist: 0.3,
            good_touch_dist: 0.8,
            heavy_touch_dist: 1.5,
        }
    }
}
