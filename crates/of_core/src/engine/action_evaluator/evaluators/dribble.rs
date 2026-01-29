//! Dribble Evaluator
//!
//! FIX_2601/0108: 드리블 액션에 대한 UAE 6요소 평가

use super::common::{ActionEvaluator, EvalContext};
use crate::engine::action_evaluator::types::{ActionScore, Vec2};

/// 드리블 평가자
pub struct DribbleEvaluator;

impl DribbleEvaluator {
    /// 드리블 평가
    pub fn evaluate(ctx: &EvalContext, _direction: Vec2) -> ActionScore {
        let dribbling = ctx.dribbling / 100.0;
        let agility = ctx.agility / 100.0;
        let pace = ctx.pace / 100.0;
        let flair = ctx.flair / 100.0;
        let composure = ctx.composure / 100.0;
        let balance = ctx.balance / 100.0;

        ActionScore {
            // Distance: 앞에 공간이 있는지
            distance: match ctx.space_ahead {
                s if s > 0.8 => 1.0,
                s if s > 0.5 => 0.85,
                s if s > 0.3 => 0.6,
                _ => 0.3,
            },

            // Safety: 실패 시 역습 위험
            safety: ctx.dribble_success_probability * 0.5
                + if ctx.has_outlet { 0.25 } else { 0.0 }
                + if !ctx.beaten_if_fail { 0.25 } else { 0.0 },

            // Readiness: 드리블 능력
            readiness: dribbling * 0.35
                + agility * 0.25
                + pace * 0.15
                + flair * 0.10
                + balance * 0.10
                + composure * 0.05,

            // Progression: xG 증가량
            progression: ctx.xg_gain_from_carry.clamp(0.0, 1.0),

            // Space: 현재 + 목표 공간
            space: ctx.space_ahead * 0.7
                + (1.0 - (ctx.defenders_ahead as f32 * 0.2).min(0.8)) * 0.3,

            // Tactical: 드리블 상황 적합성
            tactical: if ctx.closest_defender_dist > 3.0 { 0.3 } else { 0.1 }
                + if ctx.has_outlet { 0.2 } else { 0.0 }
                + if ctx.defenders_ahead <= 1 { 0.2 } else { 0.0 }
                + if ctx.local_pressure < 0.4 { 0.2 } else { 0.0 },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_space_dribble() {
        let ctx = EvalContext {
            space_ahead: 0.9,
            dribble_success_probability: 0.8,
            has_outlet: true,
            beaten_if_fail: false,
            dribbling: 85.0,
            agility: 88.0,
            pace: 90.0,
            flair: 82.0,
            balance: 78.0,
            composure: 75.0,
            xg_gain_from_carry: 0.15,
            defenders_ahead: 0,
            closest_defender_dist: 8.0,
            local_pressure: 0.2,
            ..Default::default()
        };

        let score = DribbleEvaluator::evaluate(&ctx, Vec2::new(1.0, 0.0));

        // 열린 공간 → distance/space 높음
        assert!(score.distance > 0.9);
        assert!(score.space > 0.8);

        // 안전한 상황 → safety 높음
        assert!(score.safety > 0.8);
    }

    #[test]
    fn test_crowded_dribble() {
        let ctx = EvalContext {
            space_ahead: 0.2,
            dribble_success_probability: 0.4,
            has_outlet: false,
            beaten_if_fail: true,
            dribbling: 75.0,
            agility: 72.0,
            pace: 78.0,
            flair: 65.0,
            balance: 70.0,
            composure: 68.0,
            xg_gain_from_carry: 0.02,
            defenders_ahead: 3,
            closest_defender_dist: 1.5,
            local_pressure: 0.7,
            ..Default::default()
        };

        let score = DribbleEvaluator::evaluate(&ctx, Vec2::new(1.0, 0.0));

        // 막힌 상황 → distance/space 낮음
        assert!(score.distance < 0.4);
        assert!(score.space < 0.4);

        // 위험한 상황 → safety 낮음
        assert!(score.safety < 0.3);
    }

    #[test]
    fn test_skill_affects_readiness() {
        let low_skill = EvalContext {
            dribbling: 50.0,
            agility: 55.0,
            pace: 60.0,
            flair: 45.0,
            balance: 50.0,
            composure: 55.0,
            ..Default::default()
        };

        let high_skill = EvalContext {
            dribbling: 92.0,
            agility: 90.0,
            pace: 88.0,
            flair: 85.0,
            balance: 82.0,
            composure: 80.0,
            ..Default::default()
        };

        let low_score = DribbleEvaluator::evaluate(&low_skill, Vec2::new(1.0, 0.0));
        let high_score = DribbleEvaluator::evaluate(&high_skill, Vec2::new(1.0, 0.0));

        // 스킬이 높으면 readiness가 높음
        assert!(high_score.readiness > low_score.readiness);
        assert!(high_score.readiness > 0.8);
        assert!(low_score.readiness < 0.6);
    }
}
