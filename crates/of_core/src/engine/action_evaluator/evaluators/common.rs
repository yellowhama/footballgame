//! Common Evaluator Components
//!
//! FIX_2601/0108: EvalContext and ActionEvaluator trait

use crate::engine::action_evaluator::types::ActionScore;

/// 액션 평가에 필요한 모든 컨텍스트
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    // === 선수 위치/상태 ===
    pub player_x: f32,
    pub player_y: f32,
    pub dist_to_goal: f32,
    pub dist_to_ball: f32,
    pub dist_to_ball_carrier: f32,
    pub stamina_pct: f32,

    // === 선수 능력치 (0-100) ===
    pub finishing: f32,
    pub long_shots: f32,
    pub composure: f32,
    pub technique: f32,
    pub passing: f32,
    pub vision: f32,
    pub crossing: f32,
    pub dribbling: f32,
    pub flair: f32,
    pub agility: f32,
    pub pace: f32,
    pub acceleration: f32,
    pub strength: f32,
    pub balance: f32,
    pub heading: f32,
    pub jumping: f32,
    pub tackling: f32,
    pub marking: f32,
    pub positioning: f32,
    pub anticipation: f32,
    pub decisions: f32,
    pub concentration: f32,
    pub aggression: f32,
    pub work_rate: f32,
    pub teamwork: f32,
    pub off_the_ball: f32,

    // === 슛 관련 ===
    pub xg: f32,
    pub shot_angle: f32,
    pub gk_dist: f32,
    pub shot_lane_clear: bool,
    pub is_one_on_one: bool,
    pub in_shooting_zone: bool,
    pub local_pressure: f32,

    // === 패스 관련 ===
    pub receiver_freedom: f32,
    pub receiver_dist: f32,
    pub line_break_value: f32,
    pub receiver_xg_if_receives: f32,
    pub pass_lane_clear: bool,
    pub receiver_is_forward: bool,
    pub receiver_has_space: f32,
    pub pass_interceptor_count: u32,
    /// FIX_2601/1128: 상호 패스 대상 여부 (최근에 이 선수에게 패스받았으면 true)
    pub is_reciprocal_target: bool,

    // === 드리블 관련 ===
    pub space_ahead: f32,
    pub xg_gain_from_carry: f32,
    pub defenders_ahead: u32,
    pub has_outlet: bool,
    pub dribble_success_probability: f32,
    pub beaten_if_fail: bool,
    pub closest_defender_dist: f32,

    // === 크로스 관련 ===
    pub in_crossing_zone: bool,
    pub cross_lane_clear: bool,
    pub best_header_target_xg: f32,
    pub box_target_space_score: f32,
    pub has_aerial_threat: bool,

    // === 홀드/클리어 관련 ===
    pub can_shield_ball: bool,
    pub nearby_opponents: u32,
    pub teammates_advancing_ratio: f32,
    pub is_target_man: bool,
    pub clear_direction_safe: bool,
    pub not_own_goal_risk: bool,
    pub xg_reduction_from_clear: f32,
    pub is_last_ditch: bool,

    // === 헤더 관련 ===
    pub aerial_duel_advantage: f32,
    pub header_xg: f32,
    pub is_set_piece: bool,

    // === 런/서포트 관련 ===
    pub xg_at_target: f32,
    pub space_at_target: f32,
    pub is_behind_defense: bool,
    pub creates_overload: bool,
    pub not_leaving_hole: bool,
    pub can_recover_if_turnover: bool,
    pub provides_passing_option: bool,
    pub not_blocking_space: bool,
    pub xg_if_receives: f32,
    pub space_at_support_position: f32,
    pub creates_triangle: bool,

    // === 수비 관련 ===
    pub has_cover_behind: bool,
    pub overcommit_risk: f32,
    pub tackle_success_probability: f32,
    pub pass_options_blocked_ratio: f32,
    pub press_trigger_met: bool,
    pub team_is_pressing: bool,
    pub foul_probability: f32,
    pub beaten_if_miss_probability: f32,
    pub timing_quality: f32,
    pub ball_recovery_value: f32,
    pub space_after_tackle: f32,
    pub is_last_man: bool,
    pub in_own_box: bool,
    pub can_see_ball: bool,
    pub ball_watching_risk: f32,
    pub cover_available: bool,
    pub pass_option_denied_value: f32,
    pub secondary_cover_area: f32,
    pub matches_team_marking_style: bool,
    pub covers_dangerous_space: bool,
    pub maintains_line: bool,
    pub xg_reduction_from_cover: f32,
    pub area_protected_size: f32,
    pub is_covering_teammate: bool,
    pub blocks_passing_lane: bool,
    pub intercept_success_probability: f32,
    pub out_of_position_if_miss: f32,
    pub space_after_intercept: f32,
    pub triggers_counter: bool,
    pub high_value_interception: bool,

    // === 공격 방향 ===
    pub attacks_right: bool,
}

impl EvalContext {
    /// 빈 컨텍스트 생성
    pub fn new() -> Self {
        Self::default()
    }

    /// 슛 평가용 컨텍스트 빌더
    pub fn for_shooting(
        xg: f32,
        finishing: f32,
        long_shots: f32,
        composure: f32,
        dist_to_goal: f32,
    ) -> Self {
        Self {
            xg,
            finishing,
            long_shots,
            composure,
            dist_to_goal,
            in_shooting_zone: dist_to_goal < 25.0,
            shot_lane_clear: true,
            ..Default::default()
        }
    }

    /// 패스 평가용 컨텍스트 빌더
    pub fn for_passing(
        passing: f32,
        vision: f32,
        technique: f32,
        receiver_freedom: f32,
        receiver_dist: f32,
    ) -> Self {
        Self {
            passing,
            vision,
            technique,
            receiver_freedom,
            receiver_dist,
            pass_lane_clear: true,
            ..Default::default()
        }
    }
}

/// 액션 평가자 트레이트
pub trait ActionEvaluator {
    /// 컨텍스트에서 ActionScore 계산
    fn evaluate(ctx: &EvalContext) -> ActionScore;
}
