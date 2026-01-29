//! EV Decision Configuration (P12)

use serde::{Deserialize, Serialize};

/// EV 기반 의사결정 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionConfig {
    // === Reward Values ===
    /// 골 성공 시 reward (기본: 1.0)
    pub goal_reward: f32,
    /// 찬스 생성 reward (기본: 0.4)
    pub chance_creation_reward: f32,
    /// 드리블 성공 후 위협 증가분 (기본: 0.15)
    pub dribble_threat_gain: f32,

    // === Cost Values ===
    /// 공격권 상실 기본 비용 (기본: 0.25)
    pub loss_of_possession_base_cost: f32,
    /// 역습 위험 가중치 (기본: 0.15)
    pub counterattack_risk_weight: f32,
    /// 백패스 실패 추가 비용 (기본: 0.15)
    pub backward_pass_fail_penalty: f32,

    // === Thresholds ===
    /// 최소 액션 EV (이하면 Hold) (기본: -0.3)
    pub min_action_ev_threshold: f32,
    /// 슛 최소 거리 (m) (기본: 5.0)
    pub min_shot_distance: f32,
    /// 슛 최대 거리 (m) (기본: 35.0)
    pub max_shot_distance: f32,

    // === Pass EV Factors ===
    /// 패스 거리별 성공률 감소 계수 (기본: 0.01)
    pub pass_distance_decay: f32,
    /// 인터셉트 위험 가중치 (기본: 0.2)
    pub interception_risk_weight: f32,

    // === Feature Flags ===
    /// EV 기반 의사결정 사용 여부 (기본: true)
    pub use_ev_decision: bool,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            goal_reward: 1.0,
            chance_creation_reward: 0.4,
            dribble_threat_gain: 0.15,

            loss_of_possession_base_cost: 0.25,
            counterattack_risk_weight: 0.15,
            backward_pass_fail_penalty: 0.15,

            min_action_ev_threshold: -0.3,
            min_shot_distance: 5.0,
            max_shot_distance: 35.0,

            pass_distance_decay: 0.01,
            interception_risk_weight: 0.2,

            use_ev_decision: true,
        }
    }
}
