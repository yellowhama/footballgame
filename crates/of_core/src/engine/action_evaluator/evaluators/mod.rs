//! Action Evaluators
//!
//! FIX_2601/0108: 각 액션별 UAE 6요소 평가

mod shoot;
mod pass;
mod dribble;
mod common;

pub use shoot::ShootEvaluator;
pub use pass::PassEvaluator;
pub use dribble::DribbleEvaluator;
pub use common::{EvalContext, ActionEvaluator};

use crate::engine::action_evaluator::types::{Action, ActionScore};

/// 액션 평가 레지스트리
pub struct EvaluatorRegistry;

impl EvaluatorRegistry {
    /// 액션에 대한 점수 계산
    pub fn evaluate(action: &Action, ctx: &EvalContext) -> ActionScore {
        match action {
            Action::Shoot => ShootEvaluator::evaluate(ctx),
            Action::Pass { target_id } => PassEvaluator::evaluate(ctx, *target_id),
            Action::ThroughBall { target_id } => PassEvaluator::evaluate_through_ball(ctx, *target_id),
            Action::Dribble { direction } => DribbleEvaluator::evaluate(ctx, *direction),
            Action::Cross { target_zone } => evaluate_cross(ctx, *target_zone),
            Action::Hold => evaluate_hold(ctx),
            Action::Header { is_shot } => evaluate_header(ctx, *is_shot),
            Action::Clear => evaluate_clear(ctx),

            // Off-Ball 액션들
            Action::RunIntoSpace { target } => evaluate_run(ctx, *target),
            Action::Support { position } => evaluate_support(ctx, *position),
            Action::Overlap => evaluate_overlap(ctx),
            Action::HoldPosition => evaluate_hold_position(ctx),

            // 수비 액션들
            Action::Press => evaluate_press(ctx),
            Action::Tackle => evaluate_tackle(ctx),
            Action::Jockey => evaluate_jockey(ctx),
            Action::Mark { target_id } => evaluate_mark(ctx, *target_id),
            Action::Cover { zone } => evaluate_cover(ctx, *zone),
            Action::Intercept { lane } => evaluate_intercept(ctx, lane),
            Action::BlockLane { lane } => evaluate_block_lane(ctx, lane),

            // 전환 액션들
            Action::CounterPress => evaluate_counter_press(ctx),
            Action::Delay => evaluate_delay(ctx),
            Action::CoverEmergency { zone } => evaluate_cover_emergency(ctx, *zone),
            Action::FirstPassForward { target_id } => PassEvaluator::evaluate(ctx, *target_id),
            Action::Carry { direction } => DribbleEvaluator::evaluate(ctx, *direction),
            Action::RunSupport { target_space } => evaluate_run(ctx, *target_space),

            // FIX_2601/0113 Phase 2: 새 액션들
            Action::DrawFoul => evaluate_draw_foul(ctx),
            Action::RecoveryRun { target } => evaluate_recovery_run(ctx, *target),
        }
    }
}

// === 간단한 평가 함수들 (전체 구현은 개별 파일로 분리 가능) ===

use crate::engine::action_evaluator::types::{CrossZone, PassLane, PlayerId, Position, Vec2, Zone};

fn evaluate_cross(ctx: &EvalContext, _target_zone: CrossZone) -> ActionScore {
    let crossing = ctx.crossing / 100.0;
    let technique = ctx.technique / 100.0;
    let vision = ctx.vision / 100.0;

    ActionScore {
        distance: if ctx.in_crossing_zone { 1.0 } else { 0.5 },
        safety: if ctx.cross_lane_clear { 0.7 } else { 0.3 }
            * if ctx.local_pressure < 0.5 { 1.0 } else { 0.6 },
        readiness: crossing * 0.5 + technique * 0.3 + vision * 0.2,
        progression: ctx.best_header_target_xg,
        space: ctx.box_target_space_score,
        tactical: if ctx.has_aerial_threat { 0.6 } else { 0.3 },
    }
}

fn evaluate_hold(ctx: &EvalContext) -> ActionScore {
    // FIX_2601/0109: Hold 점수 대폭 하향
    // Hold는 Dribble로 통합되므로 거의 선택되지 않아야 함
    let strength = ctx.strength / 100.0;
    let composure = ctx.composure / 100.0;

    ActionScore {
        distance: 0.1,  // Hold는 진전이 아님
        safety: 0.3,    // 고정 낮은 값
        readiness: strength * 0.2 + composure * 0.2,
        progression: 0.0,  // 전진 없음
        space: 0.2,
        tactical: if ctx.is_target_man { 0.4 } else { 0.1 },
    }
}

fn evaluate_header(ctx: &EvalContext, is_shot: bool) -> ActionScore {
    let heading = ctx.heading / 100.0;
    let jumping = ctx.jumping / 100.0;
    let strength = ctx.strength / 100.0;

    ActionScore {
        distance: if ctx.dist_to_ball < 1.0 { 1.0 } else { 0.5 },
        safety: ctx.aerial_duel_advantage * 0.5 + 0.3,
        readiness: heading * 0.5 + jumping * 0.3 + strength * 0.2,
        progression: if is_shot { ctx.header_xg } else { 0.3 },
        space: 0.5,
        tactical: if ctx.is_set_piece { 0.6 } else { 0.3 },
    }
}

fn evaluate_clear(ctx: &EvalContext) -> ActionScore {
    let heading = ctx.heading / 100.0;
    let strength = ctx.strength / 100.0;
    let composure = ctx.composure / 100.0;

    ActionScore {
        distance: if ctx.dist_to_ball < 1.0 { 1.0 } else { 0.5 },
        safety: if ctx.clear_direction_safe { 0.6 } else { 0.3 }
            + if ctx.not_own_goal_risk { 0.3 } else { 0.0 },
        readiness: heading * 0.4 + strength * 0.4 + composure * 0.2,
        progression: ctx.xg_reduction_from_clear,
        space: 1.0,
        tactical: if ctx.is_last_ditch { 0.8 } else { 0.4 },
    }
}

fn evaluate_run(ctx: &EvalContext, target: Position) -> ActionScore {
    let pace = ctx.pace / 100.0;
    let off_the_ball = ctx.off_the_ball / 100.0;
    let anticipation = ctx.anticipation / 100.0;
    let stamina_pct = ctx.stamina_pct;

    let run_distance = ((target.x - ctx.player_x).powi(2) + (target.y - ctx.player_y).powi(2)).sqrt();

    ActionScore {
        distance: match run_distance {
            d if d < 5.0 => 0.6,
            d if d < 15.0 => 1.0,
            d if d < 30.0 => 0.8,
            _ => 0.4,
        },
        safety: if ctx.not_leaving_hole { 0.5 } else { 0.2 }
            + stamina_pct * 0.3
            + if ctx.can_recover_if_turnover { 0.2 } else { 0.0 },
        readiness: pace * 0.4 + off_the_ball * 0.4 + anticipation * 0.2,
        progression: ctx.xg_at_target,
        space: ctx.space_at_target,
        tactical: if ctx.is_behind_defense { 0.5 } else { 0.2 }
            + if ctx.creates_overload { 0.3 } else { 0.0 },
    }
}

fn evaluate_support(ctx: &EvalContext, _position: Position) -> ActionScore {
    let off_the_ball = ctx.off_the_ball / 100.0;
    let teamwork = ctx.teamwork / 100.0;
    let positioning = ctx.positioning / 100.0;

    ActionScore {
        distance: 0.8,
        safety: if ctx.provides_passing_option { 0.5 } else { 0.3 }
            + if ctx.not_blocking_space { 0.3 } else { 0.0 },
        readiness: off_the_ball * 0.4 + teamwork * 0.3 + positioning * 0.3,
        progression: ctx.xg_if_receives,
        space: ctx.space_at_support_position,
        tactical: if ctx.creates_triangle { 0.4 } else { 0.2 },
    }
}

fn evaluate_overlap(_ctx: &EvalContext) -> ActionScore {
    // 간단한 기본 구현
    ActionScore {
        distance: 0.8,
        safety: 0.5,
        readiness: 0.7,
        progression: 0.6,
        space: 0.7,
        tactical: 0.6,
    }
}

fn evaluate_hold_position(_ctx: &EvalContext) -> ActionScore {
    ActionScore {
        distance: 1.0,
        safety: 0.8,
        readiness: 1.0,
        progression: 0.3,
        space: 0.5,
        tactical: 0.5,
    }
}

fn evaluate_press(ctx: &EvalContext) -> ActionScore {
    let aggression = ctx.aggression / 100.0;
    let work_rate = ctx.work_rate / 100.0;
    let pace = ctx.pace / 100.0;
    let anticipation = ctx.anticipation / 100.0;
    let stamina_pct = ctx.stamina_pct;

    ActionScore {
        distance: match ctx.dist_to_ball_carrier {
            d if d < 5.0 => 0.9,
            d if d < 15.0 => 1.0,
            _ => 0.3,
        },
        safety: if ctx.has_cover_behind { 0.4 } else { 0.2 }
            + (1.0 - ctx.overcommit_risk) * 0.3
            + stamina_pct * 0.3,
        readiness: aggression * 0.3 + work_rate * 0.3 + pace * 0.2 + anticipation * 0.2,
        progression: ctx.tackle_success_probability,
        space: ctx.pass_options_blocked_ratio,
        tactical: if ctx.press_trigger_met { 0.5 } else { 0.2 }
            + if ctx.team_is_pressing { 0.3 } else { 0.0 },
    }
}

fn evaluate_tackle(ctx: &EvalContext) -> ActionScore {
    let tackling = ctx.tackling / 100.0;
    let strength = ctx.strength / 100.0;

    ActionScore {
        distance: match ctx.dist_to_ball {
            d if d < 1.0 => 1.0,
            d if d < 2.0 => 0.8,
            _ => 0.3,
        },
        safety: (1.0 - ctx.foul_probability) * 0.4
            + (1.0 - ctx.beaten_if_miss_probability) * 0.6,
        readiness: tackling * 0.5 + strength * 0.3 + ctx.timing_quality * 0.2,
        progression: ctx.ball_recovery_value,
        space: ctx.space_after_tackle,
        tactical: (1.0 - if ctx.is_last_man { 0.5 } else { 0.0 })
            * (1.0 - if ctx.in_own_box { 0.3 } else { 0.0 }),
    }
}

fn evaluate_jockey(ctx: &EvalContext) -> ActionScore {
    let positioning = ctx.positioning / 100.0;
    let anticipation = ctx.anticipation / 100.0;

    ActionScore {
        distance: match ctx.dist_to_ball_carrier {
            d if d < 3.0 => 1.0,
            d if d < 5.0 => 0.8,
            _ => 0.4,
        },
        safety: 0.8, // 조키는 안전한 수비
        readiness: positioning * 0.5 + anticipation * 0.5,
        progression: 0.3,
        space: 0.5,
        tactical: 0.6,
    }
}

fn evaluate_mark(ctx: &EvalContext, _target_id: PlayerId) -> ActionScore {
    let marking = ctx.marking / 100.0;
    let positioning = ctx.positioning / 100.0;
    let concentration = ctx.concentration / 100.0;

    ActionScore {
        distance: 0.8, // 마킹 거리는 외부에서 계산
        safety: if ctx.can_see_ball { 0.3 } else { 0.1 }
            + (1.0 - ctx.ball_watching_risk) * 0.3
            + if ctx.cover_available { 0.4 } else { 0.2 },
        readiness: marking * 0.4 + positioning * 0.3 + concentration * 0.3,
        progression: ctx.pass_option_denied_value,
        space: ctx.secondary_cover_area,
        tactical: if ctx.matches_team_marking_style { 0.6 } else { 0.3 },
    }
}

fn evaluate_cover(ctx: &EvalContext, _zone: Zone) -> ActionScore {
    let positioning = ctx.positioning / 100.0;
    let anticipation = ctx.anticipation / 100.0;
    let pace = ctx.pace / 100.0;

    ActionScore {
        distance: 0.8,
        safety: if ctx.covers_dangerous_space { 0.5 } else { 0.2 }
            + if ctx.maintains_line { 0.3 } else { 0.1 },
        readiness: positioning * 0.4 + anticipation * 0.3 + pace * 0.3,
        progression: ctx.xg_reduction_from_cover,
        space: ctx.area_protected_size,
        tactical: if ctx.is_covering_teammate { 0.5 } else { 0.2 }
            + if ctx.blocks_passing_lane { 0.3 } else { 0.0 },
    }
}

fn evaluate_intercept(ctx: &EvalContext, _lane: &PassLane) -> ActionScore {
    let anticipation = ctx.anticipation / 100.0;
    let positioning = ctx.positioning / 100.0;
    let pace = ctx.pace / 100.0;
    let decisions = ctx.decisions / 100.0;

    ActionScore {
        distance: 0.7,
        safety: ctx.intercept_success_probability * 0.6
            + (1.0 - ctx.out_of_position_if_miss) * 0.4,
        readiness: anticipation * 0.4 + positioning * 0.3 + pace * 0.2 + decisions * 0.1,
        progression: ctx.ball_recovery_value,
        space: ctx.space_after_intercept,
        tactical: if ctx.triggers_counter { 0.5 } else { 0.2 }
            + if ctx.high_value_interception { 0.3 } else { 0.0 },
    }
}

fn evaluate_block_lane(_ctx: &EvalContext, _lane: &PassLane) -> ActionScore {
    ActionScore {
        distance: 0.8,
        safety: 0.7,
        readiness: 0.7,
        progression: 0.4,
        space: 0.5,
        tactical: 0.6,
    }
}

fn evaluate_counter_press(ctx: &EvalContext) -> ActionScore {
    let work_rate = ctx.work_rate / 100.0;
    let aggression = ctx.aggression / 100.0;
    let stamina_pct = ctx.stamina_pct;

    ActionScore {
        distance: 0.8,
        safety: 0.5,
        readiness: work_rate * 0.4 + aggression * 0.3 + stamina_pct * 0.3,
        progression: 0.6, // 역압박 성공 시 높은 가치
        space: 0.5,
        tactical: 0.8, // 전술적으로 중요
    }
}

fn evaluate_delay(ctx: &EvalContext) -> ActionScore {
    let positioning = ctx.positioning / 100.0;

    ActionScore {
        distance: 0.7,
        safety: 0.8, // 지연은 안전한 선택
        readiness: positioning * 0.6 + 0.4,
        progression: 0.3,
        space: 0.5,
        tactical: 0.6,
    }
}

fn evaluate_cover_emergency(_ctx: &EvalContext, _zone: Zone) -> ActionScore {
    ActionScore {
        distance: 0.7,
        safety: 0.9, // 긴급 커버는 안전 최우선
        readiness: 0.8,
        progression: 0.5,
        space: 0.6,
        tactical: 0.8,
    }
}

// ============================================================================
// FIX_2601/0113 Phase 2: 새 Evaluator 함수들
// ============================================================================

/// 파울 유도 평가
/// - 목적: 세트피스 획득, 시간 끌기
/// - 위치에 따라 progression 다름 (페널티박스 근처 = 높음)
fn evaluate_draw_foul(ctx: &EvalContext) -> ActionScore {
    let technique = ctx.technique / 100.0;
    let flair = ctx.flair / 100.0;
    let composure = ctx.composure / 100.0;

    ActionScore {
        // 거리: 좋은 위치에서 파울 유도 시 높은 가치
        distance: if ctx.dist_to_goal < 25.0 {
            0.7
        } else if ctx.dist_to_goal < 40.0 {
            0.5
        } else {
            0.3
        },

        // 안전성: 파울 성공 시 볼 유지 보장
        safety: 0.7,

        // 준비: 기술/플레어/침착함 기반
        readiness: technique * 0.4 + flair * 0.4 + composure * 0.2,

        // 진전: 위치에 따른 세트피스 가치
        progression: if ctx.in_own_box {
            0.0 // 자책골 위험 존에서는 절대 안됨
        } else if ctx.dist_to_goal < 20.0 {
            0.6 // 페널티박스 근처 - 프리킥/페널티 기회
        } else if ctx.dist_to_goal < 35.0 {
            0.4 // 프리킥 위험 존
        } else {
            0.2
        },

        // 공간: N/A (고정)
        space: 0.5,

        // 전술: 압박 상황에서 시간 끌기 효과
        tactical: if ctx.local_pressure > 0.7 { 0.5 } else { 0.3 },
    }
}

/// 수비적 복귀 (역습 저지) 평가
/// - 목적: 역습 당할 때 수비 위치로 복귀
/// - 라스트맨/체력/속도가 중요
fn evaluate_recovery_run(ctx: &EvalContext, target: Position) -> ActionScore {
    let pace = ctx.pace / 100.0;
    let stamina_pct = ctx.stamina_pct;
    let work_rate = ctx.work_rate / 100.0;

    // 복귀 거리 계산
    let recovery_distance =
        ((target.x - ctx.player_x).powi(2) + (target.y - ctx.player_y).powi(2)).sqrt();

    ActionScore {
        // 거리: 가까울수록 좋음
        distance: match recovery_distance {
            d if d < 10.0 => 0.9,
            d if d < 20.0 => 0.7,
            d if d < 35.0 => 0.5,
            _ => 0.3,
        },

        // 안전성: 체력/커버 상황
        safety: stamina_pct * 0.5
            + if ctx.has_cover_behind { 0.3 } else { 0.0 }
            + if ctx.cover_available { 0.2 } else { 0.0 },

        // 준비: 속도/워크레이트/체력
        readiness: pace * 0.4 + work_rate * 0.4 + stamina_pct * 0.2,

        // 진전: 역습 저지 가치 (항상 높음)
        progression: 0.6,

        // 공간: 커버해야 할 공간
        space: if ctx.covers_dangerous_space { 0.7 } else { 0.4 },

        // 전술: 라스트맨이면 더 중요
        tactical: if ctx.is_last_man { 0.8 } else { 0.5 },
    }
}
