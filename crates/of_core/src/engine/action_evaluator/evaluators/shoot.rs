//! Shoot Evaluator
//!
//! FIX_2601/0108: 슛 액션에 대한 UAE 6요소 평가

use super::common::{ActionEvaluator, EvalContext};
use crate::engine::action_evaluator::types::ActionScore;

/// 슛 평가자
pub struct ShootEvaluator;

impl ActionEvaluator for ShootEvaluator {
    fn evaluate(ctx: &EvalContext) -> ActionScore {
        // 능력치 정규화 (0-100 → 0.0-1.0)
        let finishing = ctx.finishing / 100.0;
        let long_shots = ctx.long_shots / 100.0;
        let composure = ctx.composure / 100.0;
        let technique = ctx.technique / 100.0;

        // 거리 기반 가중 (가까울수록 finishing, 멀수록 long_shots)
        let dist_factor = (ctx.dist_to_goal / 30.0).clamp(0.0, 1.0);
        let shot_skill = finishing * (1.0 - dist_factor) + long_shots * dist_factor;

        // FIX_2601/0109: 슈팅존 보너스 (패스보다 슛 선호)
        let shooting_zone_bonus = if ctx.dist_to_goal < 20.0 {
            0.3  // 20m 이내: 큰 보너스
        } else if ctx.dist_to_goal < 30.0 {
            0.15  // 30m 이내: 작은 보너스
        } else {
            0.0
        };

        ActionScore {
            // Distance: 거리 정상 범위 (2~25m 최적)
            distance: match ctx.dist_to_goal {
                d if d < 2.0 => 0.7, // 너무 가까움 (0.6→0.7)
                d if d < 8.0 => 1.0, // 최적
                d if d < 16.0 => 0.95, // (0.9→0.95)
                d if d < 25.0 => 0.8, // (0.7→0.8)
                d if d < 30.0 => 0.5, // (0.4→0.5)
                _ => 0.1, // 30m+
            },

            // Safety: 실패 시 역습 위험도 + 슈팅존 보너스
            safety: (if ctx.shot_lane_clear { 0.6 } else { 0.4 }
                * if ctx.local_pressure < 0.5 { 1.0 } else { 0.8 }
                + if ctx.is_one_on_one { 0.2 } else { 0.0 })
                + shooting_zone_bonus,

            // Readiness: 선수 슛 능력
            readiness: shot_skill * 0.5 + composure * 0.3 + technique * 0.2,

            // Progression: xG 기반 + 거리 보너스 (핵심!)
            progression: (ctx.xg + shooting_zone_bonus).clamp(0.0, 1.0),

            // Space: 슛 공간 확보
            space: if ctx.shot_lane_clear { 0.8 } else { 0.4 }
                * (1.0 - ctx.local_pressure * 0.5).max(0.3),

            // Tactical: 슛 상황 적합성
            tactical: if ctx.in_shooting_zone { 0.7 } else { 0.2 }
                + if ctx.is_one_on_one { 0.2 } else { 0.0 }
                + if ctx.shot_angle > 0.3 { 0.1 } else { 0.0 },
        }
    }
}

impl ShootEvaluator {
    /// 전체 평가 (ActionEvaluator trait 위임)
    pub fn evaluate(ctx: &EvalContext) -> ActionScore {
        <Self as ActionEvaluator>::evaluate(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_close_range_shot() {
        let ctx = EvalContext {
            dist_to_goal: 6.0,
            xg: 0.35,
            finishing: 85.0,
            long_shots: 70.0,
            composure: 80.0,
            technique: 75.0,
            shot_lane_clear: true,
            local_pressure: 0.3,
            in_shooting_zone: true,
            is_one_on_one: false,
            shot_angle: 0.5,
            ..Default::default()
        };

        let score = ShootEvaluator::evaluate(&ctx);

        // 가까운 거리 → distance 높음
        assert!(score.distance > 0.9);

        // xG + 슈팅존 보너스 기반 progression (6m < 20m → +0.3 bonus)
        // progression = (0.35 + 0.3).clamp = 0.65
        assert!(score.progression > 0.6);

        // 좋은 조건 → readiness 높음
        assert!(score.readiness > 0.7);
    }

    #[test]
    fn test_long_range_shot() {
        let ctx = EvalContext {
            dist_to_goal: 28.0,
            xg: 0.05,
            finishing: 70.0,
            long_shots: 85.0,
            composure: 75.0,
            technique: 80.0,
            shot_lane_clear: true,
            local_pressure: 0.2,
            in_shooting_zone: false,
            is_one_on_one: false,
            shot_angle: 0.4,
            ..Default::default()
        };

        let score = ShootEvaluator::evaluate(&ctx);

        // 먼 거리 → distance 낮음 (28m → 0.5)
        assert!(score.distance <= 0.5);

        // xG + 슈팅존 보너스 (28m < 30m → +0.15 bonus)
        // progression = (0.05 + 0.15).clamp = 0.20
        assert!(score.progression < 0.25);
    }

    #[test]
    fn test_pressure_reduces_safety() {
        let low_pressure = EvalContext {
            local_pressure: 0.2,
            shot_lane_clear: true,
            ..Default::default()
        };

        let high_pressure = EvalContext {
            local_pressure: 0.8,
            shot_lane_clear: false,
            ..Default::default()
        };

        let low_score = ShootEvaluator::evaluate(&low_pressure);
        let high_score = ShootEvaluator::evaluate(&high_pressure);

        assert!(low_score.safety > high_score.safety);
    }
}
