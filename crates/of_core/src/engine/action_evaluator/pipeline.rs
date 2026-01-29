//! Decision Pipeline
//!
//! FIX_2601/0108: UAE 통합 파이프라인
//!
//! 파이프라인 순서:
//! 1. 상태 분류 (PlayerPhaseState)
//! 2. 후보 액션 생성 (ActionSetBuilder)
//! 3. Hard Gate 필터링
//! 4. UAE 6요소 평가
//! 5. 가중치 적용
//! 6. 팀 조율 (충돌 페널티)
//! 7. 최종 선택

use super::action_set::{ActionSetBuilder, ActionSetContext};
use super::evaluators::{EvalContext, EvaluatorRegistry};
use super::hard_gate::{filter_by_hard_gate, HardGateContext};
use super::state::{PlayerPhaseState, RoleTag, StateContext};
use super::team_coord::TeamCoordinator;
use super::types::{Action, ActionWeights, PlayerId, ScoredAction};
use super::weights::WeightCalculator;
use crate::engine::behavior_intent::BehaviorIntent;

/// 파이프라인 설정
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// 최소 점수 (이 이하는 선택 불가)
    pub min_score_threshold: f32,

    /// Top-N 중 랜덤 선택 (결정론적이면 1)
    pub top_n_selection: usize,

    /// 디버그 로깅 활성화
    pub debug_logging: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            min_score_threshold: 0.1,
            top_n_selection: 1,
            debug_logging: false,
        }
    }
}

/// 파이프라인 결과
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// 선택된 액션
    pub selected: Option<ScoredAction>,

    /// 모든 평가된 액션 (디버깅용)
    pub all_scored: Vec<ScoredAction>,

    /// 분류된 상태
    pub state: PlayerPhaseState,

    /// 역할 태그
    pub role: RoleTag,

    /// Hard Gate로 필터된 액션 수
    pub filtered_count: usize,

    /// 선택된 BehaviorIntent (selected.behavior_intent 편의 접근)
    pub behavior_intent: Option<BehaviorIntent>,
}

/// 통합 결정 파이프라인
pub struct DecisionPipeline {
    config: PipelineConfig,
}

impl DecisionPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    /// 파이프라인 실행
    pub fn execute(
        &self,
        player_id: PlayerId,
        state_ctx: &StateContext,
        action_ctx: &ActionSetContext,
        eval_ctx: &EvalContext,
        hard_gate_ctx: &HardGateContext,
        team_coord: &mut TeamCoordinator,
        position: &str,
        traits: &[&str],
        mentality: &str,
        passing_style: &str,
        tempo: &str,
    ) -> PipelineResult {
        // 1. 상태 분류
        let state = PlayerPhaseState::classify(state_ctx);
        // TODO: is_pressing should come from team tactical state
        let is_pressing = matches!(mentality, "Attacking" | "VeryAttacking");
        let role = RoleTag::from_position_and_state(position, state, is_pressing);

        // 2. 후보 액션 생성
        let mut candidates = ActionSetBuilder::for_state(state, role, action_ctx);
        let original_count = candidates.len();

        // 3. Hard Gate 필터링
        filter_by_hard_gate(&mut candidates, hard_gate_ctx);
        let filtered_count = original_count - candidates.len();

        // 4. 가중치 계산
        let weights =
            WeightCalculator::calculate(position, traits, mentality, passing_style, tempo, state);

        // 5. UAE 평가 + 점수 계산 + BehaviorIntent 파생
        let mut scored: Vec<ScoredAction> = candidates
            .into_iter()
            .map(|action| {
                let score = EvaluatorRegistry::evaluate(&action, eval_ctx);
                // 컨텍스트 기반 세분화된 BehaviorIntent 파생
                let behavior_intent = BehaviorIntent::from_action_with_context(&action, eval_ctx);
                ScoredAction::with_intent(action, score, &weights, behavior_intent)
            })
            .collect();

        // 5b. 패스 전방/후방/reciprocity 보정 (FIX_2601/0109, FIX_2601/1128)
        // player_id 기준: 0-10 홈, 11-21 어웨이
        // 낮은 idx = 수비수, 높은 idx = 공격수
        let player_num = player_id.0 as i32;
        let is_home = player_num < 11;
        for sa in &mut scored {
            if let Action::Pass { target_id } = &sa.action {
                let target_num = target_id.0 as i32;
                // 같은 팀 내에서 앞/뒤 비교
                let target_slot = if is_home { target_num } else { target_num - 11 };
                let player_slot = if is_home { player_num } else { player_num - 11 };

                // FIX_2601/1128: 전방 패스 보너스 대폭 축소 (0.12 → 0.04)
                // 후방 패스 페널티 제거
                if target_slot > player_slot {
                    sa.weighted_total += 0.04;  // 전방 패스 보너스 (축소)
                }
                // 후방 패스 페널티 제거됨

                // FIX_2601/1128: Reciprocity 보너스 추가
                // 최근에 나에게 패스한 선수에게 패스하면 보너스
                if action_ctx.reciprocal_targets.contains(target_id) {
                    sa.weighted_total += 0.20;  // 상호 패스 보너스
                }
            }
        }

        // 6. 팀 조율 (충돌 페널티)
        for sa in &mut scored {
            team_coord.apply_conflict_penalty(sa);
        }

        // 7. 정렬 (높은 점수 순)
        scored.sort_by(|a, b| {
            b.weighted_total
                .partial_cmp(&a.weighted_total)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 8. 슈팅 점수 조정 (FIX_2601/0109)
        // - 8m 이하: 무조건 슛
        // - 1v1 or clear shot: + 보너스
        // - 수비수 앞에 있으면: - 페널티
        let dist = eval_ctx.dist_to_goal;
        let force_shoot = dist < 8.0 && eval_ctx.in_shooting_zone;

        if !force_shoot {
            for sa in &mut scored {
                if matches!(sa.action, Action::Shoot) {
                    // 슈팅존 보너스 (거리 기반)
                    if dist < 16.5 {
                        sa.weighted_total += 0.15;  // Box 내
                    } else if dist < 25.0 {
                        sa.weighted_total += 0.05;   // Arc
                    }

                    // 1v1 또는 클리어샷 보너스
                    if eval_ctx.is_one_on_one {
                        sa.weighted_total += 0.2;
                    }
                    if eval_ctx.shot_lane_clear {
                        sa.weighted_total += 0.05;
                    }

                    // 수비수 페널티 (앞에 있는 수비수 수)
                    let defenders = eval_ctx.defenders_ahead;
                    if defenders >= 1 {
                        sa.weighted_total -= 0.08 * defenders as f32;  // 1명당 -0.08
                    }

                    // 거리 페널티 (25m 이상)
                    if dist > 25.0 {
                        sa.weighted_total -= 0.15;
                    }
                }
            }

            // 재정렬
            scored.sort_by(|a, b| {
                b.weighted_total
                    .partial_cmp(&a.weighted_total)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // 9. 최소 점수 필터
        scored.retain(|sa| sa.weighted_total >= self.config.min_score_threshold);

        // 10. 선택
        let selected = if force_shoot {
            // 8m 이내: 무조건 슛
            let shoot_action = scored.iter().find(|sa| matches!(sa.action, Action::Shoot)).cloned();
            if let Some(ref sa) = shoot_action {
                team_coord.claim(&sa.action, player_id);
            }
            shoot_action
        } else if scored.is_empty() {
            None
        } else {
            // Top-N 중 첫 번째 (결정론적)
            let idx = 0.min(self.config.top_n_selection.saturating_sub(1));
            let sa = scored.get(idx).cloned();

            // 선택된 액션 예약
            if let Some(ref selected) = sa {
                team_coord.claim(&selected.action, player_id);
            }

            sa
        };

        // BehaviorIntent 추출 (선택된 액션에서)
        let behavior_intent = selected.as_ref().map(|sa| sa.behavior_intent);

        PipelineResult {
            selected,
            all_scored: scored,
            state,
            role,
            filtered_count,
            behavior_intent,
        }
    }
}

/// 간단한 파이프라인 (기본 설정)
pub fn run_pipeline(
    player_id: PlayerId,
    state_ctx: &StateContext,
    action_ctx: &ActionSetContext,
    eval_ctx: &EvalContext,
    hard_gate_ctx: &HardGateContext,
    team_coord: &mut TeamCoordinator,
    position: &str,
) -> PipelineResult {
    let pipeline = DecisionPipeline::new(PipelineConfig::default());
    pipeline.execute(
        player_id,
        state_ctx,
        action_ctx,
        eval_ctx,
        hard_gate_ctx,
        team_coord,
        position,
        &[],        // no traits
        "Balanced", // mentality
        "Mixed",    // passing style
        "Normal",   // tempo
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::action_evaluator::types::Position;

    fn default_state_ctx() -> StateContext {
        StateContext {
            team_has_ball: true,
            i_have_ball: true,
            dist_to_ball: 0.0,
            possession_changed_tick: 0,
            current_tick: 100,
            marking_assignment: None,
            pass_lane_clear: true,
            body_facing_ball: true,
            dist_to_ball_carrier: 0.0,
            assigned_to_ball_carrier: false,
            closest_to_ball_carrier: false,
        }
    }

    fn default_action_ctx() -> ActionSetContext {
        ActionSetContext {
            in_shooting_zone: true,
            has_clear_shot: true,
            pass_targets: vec![PlayerId::new(7), PlayerId::new(11)],
            attacks_right: true,
            ..Default::default()
        }
    }

    fn default_eval_ctx() -> EvalContext {
        EvalContext {
            xg: 0.25,
            dist_to_goal: 12.0,
            finishing: 82.0,
            long_shots: 75.0,
            composure: 78.0,
            technique: 80.0,
            passing: 76.0,
            vision: 74.0,
            dribbling: 79.0,
            agility: 81.0,
            pace: 83.0,
            flair: 72.0,
            balance: 77.0,
            shot_lane_clear: true,
            local_pressure: 0.3,
            in_shooting_zone: true,
            ..Default::default()
        }
    }

    fn default_hard_gate_ctx() -> HardGateContext {
        HardGateContext::default()
    }

    #[test]
    fn test_on_ball_pipeline() {
        let player_id = PlayerId::new(10);
        let state_ctx = default_state_ctx();
        let action_ctx = default_action_ctx();
        let eval_ctx = default_eval_ctx();
        let hard_gate_ctx = default_hard_gate_ctx();
        let mut team_coord = TeamCoordinator::new();

        let result = run_pipeline(
            player_id,
            &state_ctx,
            &action_ctx,
            &eval_ctx,
            &hard_gate_ctx,
            &mut team_coord,
            "ST",
        );

        // OnBall 상태
        assert_eq!(result.state, PlayerPhaseState::OnBall);

        // Finisher 역할
        assert_eq!(result.role, RoleTag::Finisher);

        // 액션 선택됨
        assert!(result.selected.is_some());

        // 여러 액션 평가됨
        assert!(result.all_scored.len() > 1);
    }

    #[test]
    fn test_defensive_pipeline() {
        let player_id = PlayerId::new(4);
        let state_ctx = StateContext {
            team_has_ball: false,
            i_have_ball: false,
            dist_to_ball: 8.0,
            possession_changed_tick: 0,
            current_tick: 100,
            marking_assignment: Some(PlayerId::new(9)),
            pass_lane_clear: false,
            body_facing_ball: true,
            dist_to_ball_carrier: 8.0,
            assigned_to_ball_carrier: true,
            closest_to_ball_carrier: true,
        };
        let action_ctx = ActionSetContext {
            dist_to_ball_carrier: 8.0,
            marking_target: Some(PlayerId::new(9)),
            ..Default::default()
        };
        let eval_ctx = EvalContext {
            tackling: 85.0,
            marking: 82.0,
            positioning: 80.0,
            anticipation: 78.0,
            aggression: 75.0,
            work_rate: 80.0,
            dist_to_ball_carrier: 8.0,
            ..Default::default()
        };
        let hard_gate_ctx = default_hard_gate_ctx();
        let mut team_coord = TeamCoordinator::new();

        let result = run_pipeline(
            player_id,
            &state_ctx,
            &action_ctx,
            &eval_ctx,
            &hard_gate_ctx,
            &mut team_coord,
            "CB",
        );

        // 수비 상태
        assert!(matches!(
            result.state,
            PlayerPhaseState::DefendBallCarrier | PlayerPhaseState::DefendOffBallTarget
        ));

        // 액션 선택됨
        assert!(result.selected.is_some());
    }

    #[test]
    fn test_team_coordination() {
        let state_ctx = StateContext {
            team_has_ball: false,
            i_have_ball: false,
            dist_to_ball: 15.0,
            possession_changed_tick: 0,
            current_tick: 100,
            marking_assignment: Some(PlayerId::new(9)),
            pass_lane_clear: false,
            body_facing_ball: true,
            dist_to_ball_carrier: 15.0,
            assigned_to_ball_carrier: false,
            closest_to_ball_carrier: false,
        };
        let action_ctx = ActionSetContext {
            dist_to_ball_carrier: 15.0,
            marking_target: Some(PlayerId::new(9)),
            ..Default::default()
        };
        let eval_ctx = EvalContext {
            tackling: 80.0,
            marking: 78.0,
            positioning: 76.0,
            ..Default::default()
        };
        let hard_gate_ctx = default_hard_gate_ctx();
        let mut team_coord = TeamCoordinator::new();

        // 첫 번째 선수가 마킹
        let result1 = run_pipeline(
            PlayerId::new(4),
            &state_ctx,
            &action_ctx,
            &eval_ctx,
            &hard_gate_ctx,
            &mut team_coord,
            "CB",
        );

        // 마킹 예약됨
        assert!(team_coord.is_target_claimed(PlayerId::new(9)));

        // 두 번째 선수 (같은 타겟)
        let result2 = run_pipeline(
            PlayerId::new(5),
            &state_ctx,
            &action_ctx,
            &eval_ctx,
            &hard_gate_ctx,
            &mut team_coord,
            "CB",
        );

        // 둘 다 액션 있음
        assert!(result1.selected.is_some());
        assert!(result2.selected.is_some());

        // 두 번째 선수는 Mark가 아닌 다른 액션 선택해야 함 (페널티로 인해)
        // (또는 Mark 점수가 낮아져야 함)
    }
}
