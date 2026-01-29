//! Set piece execution logic (penalties, free kicks, corners)
//!
//! This module provides pure functions for calculating set piece outcomes.
//! All randomness is passed in as parameters, allowing deterministic unit testing.

use super::physics_constants::skills;

// ============================================================================
// Penalty Kick
// ============================================================================

/// 페널티킥 실행 컨텍스트
#[derive(Debug, Clone)]
pub struct PenaltyContext {
    pub taker_idx: usize,
    pub taker_name: String,
    pub penalty_taking: f32,
    pub composure: f32,
    pub finishing: f32,
    pub gk_agility: f32,
    pub gk_anticipation: f32,
    pub has_deadball_gold: bool,
}

/// 페널티킥 결과
#[derive(Debug, Clone)]
pub enum PenaltyResult {
    Goal { scorer: String },
    Save { gk_name: String },
    Miss,
}

/// 페널티킥 성공률 계산
pub fn penalty_success_rate(ctx: &PenaltyContext) -> f32 {
    let penalty_skill = skills::normalize(ctx.penalty_taking);
    let composure = skills::normalize(ctx.composure);
    let finishing = skills::normalize(ctx.finishing);

    let base_success = 0.75 + (penalty_skill * 0.05) + (composure * 0.03) + (finishing * 0.02);

    if ctx.has_deadball_gold {
        (base_success * 1.15).min(0.95)
    } else {
        base_success.min(0.90)
    }
}

/// GK 페널티 세이브 확률
pub fn penalty_save_rate(gk_agility: f32, gk_anticipation: f32) -> f32 {
    let agi = skills::normalize(gk_agility);
    let ant = skills::normalize(gk_anticipation);
    (agi * 0.4 + ant * 0.4) * 0.25 // 최대 20%
}

/// 페널티킥 결과 결정 (rng 값을 파라미터로 받음)
pub fn resolve_penalty(ctx: &PenaltyContext, gk_name: String, roll: f32) -> PenaltyResult {
    let success_rate = penalty_success_rate(ctx);
    let save_prob = penalty_save_rate(ctx.gk_agility, ctx.gk_anticipation);

    if roll < success_rate * (1.0 - save_prob) {
        PenaltyResult::Goal { scorer: ctx.taker_name.clone() }
    } else if roll < success_rate {
        PenaltyResult::Save { gk_name }
    } else {
        PenaltyResult::Miss
    }
}

// ============================================================================
// Free Kick
// ============================================================================

/// 프리킥 실행 컨텍스트
#[derive(Debug, Clone)]
pub struct FreeKickContext {
    pub position: (f32, f32),
    pub is_home_attacking: bool,
    pub taker_idx: usize,
    pub taker_name: String,
    pub freekick_skill: f32,
    pub technique: f32,
    pub long_shots: f32,
    pub has_deadball_gold: bool,
}

/// 프리킥 결과
#[derive(Debug, Clone)]
pub enum FreeKickResult {
    DirectGoal { scorer: String },
    PassToTeammate { target_idx: usize },
    DefenseCleared,
    Continued,
}

/// 직접 프리킥 골 확률
pub fn direct_freekick_goal_chance(ctx: &FreeKickContext) -> f32 {
    use crate::engine::coordinates;
    let distance_to_goal = coordinates::distance_to_goal_m(ctx.position, ctx.is_home_attacking);

    if distance_to_goal > 30.0 {
        return 0.0; // 직접 슛 불가 거리
    }

    let freekick = skills::normalize(ctx.freekick_skill);
    let curve = skills::normalize(ctx.technique);
    let power = skills::normalize(ctx.long_shots);

    let distance_factor = 1.0 - (distance_to_goal / 40.0);
    let base_chance = freekick * 0.4 + curve * 0.35 + power * 0.25;
    let goal_chance = base_chance * distance_factor * 0.15;

    if ctx.has_deadball_gold {
        (goal_chance * 2.0).min(0.25)
    } else {
        goal_chance
    }
}

/// 프리킥 결과 결정
pub fn resolve_freekick(
    ctx: &FreeKickContext,
    nearest_attacker: Option<usize>,
    goal_roll: f32,
    outcome_roll: f32,
) -> FreeKickResult {
    let goal_chance = direct_freekick_goal_chance(ctx);

    if goal_roll < goal_chance {
        return FreeKickResult::DirectGoal { scorer: ctx.taker_name.clone() };
    }

    // 크로스/패스로 전환
    if outcome_roll < 0.3 {
        if let Some(target) = nearest_attacker {
            return FreeKickResult::PassToTeammate { target_idx: target };
        }
    } else if outcome_roll < 0.5 {
        return FreeKickResult::DefenseCleared;
    }

    FreeKickResult::Continued
}

// ============================================================================
// Corner Kick
// ============================================================================

/// 코너킥 실행 컨텍스트
#[derive(Debug, Clone)]
pub struct CornerContext {
    pub is_home_attacking: bool,
    pub taker_idx: usize,
    pub taker_name: String,
    pub crossing: f32,
    pub technique: f32,
    pub vision: f32,
    pub has_crosser_gold: bool,
    pub header_targets: Vec<HeaderTarget>,
}

/// 헤딩 타겟 정보
#[derive(Debug, Clone)]
pub struct HeaderTarget {
    pub idx: usize,
    pub name: String,
    pub heading: f32,
    pub jumping: f32,
    pub has_airraid_gold: bool,
}

/// 코너킥 결과
#[derive(Debug, Clone)]
pub enum CornerResult {
    HeaderGoal { scorer: String },
    HeaderShot { shooter: String, on_target: bool, xg: f32 },
    CrossFailed,
    NoTarget,
}

/// 코너킥 크로스 정확도 계산
pub fn corner_cross_accuracy(ctx: &CornerContext) -> f32 {
    let crossing = skills::normalize(ctx.crossing);
    let curve = skills::normalize(ctx.technique);
    let vision = skills::normalize(ctx.vision);

    let base = crossing * 0.5 + curve * 0.3 + vision * 0.2;

    if ctx.has_crosser_gold {
        (base + 0.2).min(0.95)
    } else {
        base
    }
}

/// 헤딩 골 확률 계산
pub fn header_goal_chance(target: &HeaderTarget) -> f32 {
    let heading = skills::normalize(target.heading);
    let jumping = skills::normalize(target.jumping);

    let header_skill = heading * 0.6 + jumping * 0.4;
    let header_skill =
        if target.has_airraid_gold { (header_skill + 0.25).min(0.95) } else { header_skill };

    header_skill * 0.25 // 기본 10-20% 골 확률
}

/// 코너킥 결과 결정
pub fn resolve_corner(
    ctx: &CornerContext,
    cross_roll: f32,
    goal_roll: f32,
    on_target_roll: f32,
) -> CornerResult {
    if ctx.header_targets.is_empty() {
        return CornerResult::NoTarget;
    }

    let cross_accuracy = corner_cross_accuracy(ctx);

    if cross_roll >= cross_accuracy {
        return CornerResult::CrossFailed;
    }

    // 크로스 성공 - 최적 헤딩 타겟 선택
    let best_target = find_best_header_target(&ctx.header_targets);

    if let Some(target) = best_target {
        let goal_chance = header_goal_chance(target);

        if goal_roll < goal_chance {
            return CornerResult::HeaderGoal { scorer: target.name.clone() };
        }

        // 골 실패 - 슛 시도
        let header_skill =
            skills::normalize(target.heading) * 0.6 + skills::normalize(target.jumping) * 0.4;
        let on_target = on_target_roll < 0.4;
        let xg = header_skill * 0.15;

        return CornerResult::HeaderShot { shooter: target.name.clone(), on_target, xg };
    }

    CornerResult::CrossFailed
}

/// 최적의 헤딩 타겟 찾기
pub fn find_best_header_target(targets: &[HeaderTarget]) -> Option<&HeaderTarget> {
    targets.iter().max_by(|a, b| {
        let score_a = a.heading * 0.6 + a.jumping * 0.4;
        let score_b = b.heading * 0.6 + b.jumping * 0.4;
        score_a.partial_cmp(&score_b).unwrap()
    })
}

// ============================================================================
// Utility Functions
// ============================================================================

/// 가장 가까운 팀메이트 찾기
pub fn find_nearest_player(
    from_pos: (f32, f32),
    positions: &[(usize, (f32, f32))], // (idx, pos)
) -> Option<usize> {
    positions
        .iter()
        .min_by(|a, b| {
            let dist_a = (a.1 .0 - from_pos.0).powi(2) + (a.1 .1 - from_pos.1).powi(2);
            let dist_b = (b.1 .0 - from_pos.0).powi(2) + (b.1 .1 - from_pos.1).powi(2);
            dist_a.partial_cmp(&dist_b).unwrap()
        })
        .map(|(idx, _)| *idx)
}

/// 공이 아웃된 상황 판단
#[derive(Debug, Clone)]
pub enum OutOfBoundsResult {
    Corner { attacking_home: bool },
    GoalKick { gk_idx: usize, position: (f32, f32) },
    ThrowIn { throwing_home: bool, position: (f32, f32) },
    NotOut,
}

/// 공 아웃 체크 (순수 함수)
///
/// FIX_2601/0109: 경계값 수정 (0.01/0.99 → 0.0/1.0)
/// FIFA Laws: "whole ball crosses" - 공 전체가 라인을 완전히 넘어야 아웃
/// 기존 0.01/0.99는 ~1.05m 안쪽에서 아웃 판정되어 규정 위반
///
/// FIX_2601/0116: 하프타임 이후 공격 방향이 바뀌므로, 골라인 기준(코너/골킥) 판정도
/// "누가 어느 쪽 골을 수비하는지"를 반영해야 한다.
///
/// - `home_attacks_right=true`  => 홈은 오른쪽(1.0)으로 공격, 왼쪽(0.0)을 수비
/// - `home_attacks_right=false` => 홈은 왼쪽(0.0)으로 공격, 오른쪽(1.0)을 수비
pub fn check_ball_out(
    ball_pos: (f32, f32),
    last_touch_home: Option<bool>,
    home_attacks_right: bool,
) -> OutOfBoundsResult {
    // 골라인 체크 (x <= 0 또는 x >= 1) - 정규화 좌표
    if ball_pos.0 <= 0.0 || ball_pos.0 >= 1.0 {
        let is_right_side = ball_pos.0 >= 1.0;

        // 골 영역 체크 (y = 0.43 ~ 0.57 정도가 골대)
        let is_goal_area = ball_pos.1 >= 0.43 && ball_pos.1 <= 0.57;

        if !is_goal_area {
            let last_touch_home = last_touch_home.unwrap_or(false);

            // 홈의 수비 방향(골) 결정: 공격 방향의 반대
            let home_defends_right = !home_attacks_right;

            // 공이 나간 골라인(좌/우)에 따라 수비 팀이 홈인지 결정
            let defending_home = if is_right_side { home_defends_right } else { !home_defends_right };

            // 코너킥: 수비팀이 마지막 터치
            if last_touch_home == defending_home {
                return OutOfBoundsResult::Corner { attacking_home: !defending_home };
            }

            // 골킥: 공격팀이 마지막 터치 -> 수비팀이 골킥
            let gk_idx = if defending_home { 0 } else { 11 };
            let position = if is_right_side { (0.95, 0.5) } else { (0.05, 0.5) };
            return OutOfBoundsResult::GoalKick { gk_idx, position };
        }
    }

    // 터치라인 체크 (y <= 0 또는 y >= 1) - 스로인
    if ball_pos.1 <= 0.0 || ball_pos.1 >= 1.0 {
        let last_touch_home = last_touch_home.unwrap_or(false);
        let throwing_home = !last_touch_home;

        let position = (ball_pos.0.clamp(0.05, 0.95), if ball_pos.1 <= 0.0 { 0.05 } else { 0.95 });

        return OutOfBoundsResult::ThrowIn { throwing_home, position };
    }

    OutOfBoundsResult::NotOut
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_penalty_success_rate_normal() {
        let ctx = PenaltyContext {
            taker_idx: 9,
            taker_name: "Striker".to_string(),
            penalty_taking: 15.0,
            composure: 14.0,
            finishing: 16.0,
            gk_agility: 14.0,
            gk_anticipation: 13.0,
            has_deadball_gold: false,
        };

        let rate = penalty_success_rate(&ctx);
        assert!(rate > 0.75 && rate <= 0.90, "Normal penalty rate: {}", rate);
    }

    #[test]
    fn test_penalty_success_rate_gold() {
        let ctx = PenaltyContext {
            taker_idx: 9,
            taker_name: "Specialist".to_string(),
            penalty_taking: 18.0,
            composure: 16.0,
            finishing: 17.0,
            gk_agility: 14.0,
            gk_anticipation: 13.0,
            has_deadball_gold: true,
        };

        let rate = penalty_success_rate(&ctx);
        assert!(rate > 0.85, "Gold trait penalty rate: {}", rate);
    }

    #[test]
    fn test_penalty_resolve_goal() {
        let ctx = PenaltyContext {
            taker_idx: 9,
            taker_name: "Striker".to_string(),
            penalty_taking: 16.0,
            composure: 15.0,
            finishing: 16.0,
            gk_agility: 12.0,
            gk_anticipation: 12.0,
            has_deadball_gold: false,
        };

        let result = resolve_penalty(&ctx, "GK".to_string(), 0.3); // Low roll = success
        assert!(matches!(result, PenaltyResult::Goal { .. }));
    }

    #[test]
    fn test_penalty_resolve_save() {
        let ctx = PenaltyContext {
            taker_idx: 9,
            taker_name: "Striker".to_string(),
            penalty_taking: 12.0,
            composure: 12.0,
            finishing: 12.0,
            gk_agility: 17.0,
            gk_anticipation: 16.0,
            has_deadball_gold: false,
        };

        // Roll between success*no_save and success
        let result = resolve_penalty(&ctx, "GK".to_string(), 0.72);
        assert!(matches!(result, PenaltyResult::Save { .. }));
    }

    #[test]
    fn test_penalty_resolve_miss() {
        let ctx = PenaltyContext {
            taker_idx: 9,
            taker_name: "Striker".to_string(),
            penalty_taking: 10.0,
            composure: 10.0,
            finishing: 10.0,
            gk_agility: 14.0,
            gk_anticipation: 14.0,
            has_deadball_gold: false,
        };

        let result = resolve_penalty(&ctx, "GK".to_string(), 0.95); // High roll = miss
        assert!(matches!(result, PenaltyResult::Miss));
    }

    #[test]
    fn test_direct_freekick_close() {
        // Normalized coords: (width, length)
        // Close freekick: length=0.85 = near opponent goal
        let ctx = FreeKickContext {
            position: (0.5, 0.85), // Close to goal (length=0.85)
            is_home_attacking: true,
            taker_idx: 7,
            taker_name: "Midfielder".to_string(),
            freekick_skill: 16.0,
            technique: 15.0,
            long_shots: 14.0,
            has_deadball_gold: false,
        };

        let chance = direct_freekick_goal_chance(&ctx);
        assert!(chance > 0.0, "Close freekick should have chance: {}", chance);
    }

    #[test]
    fn test_direct_freekick_far() {
        // Normalized coords: (width, length)
        // Far freekick: length=0.5 = midfield (too far for direct shot)
        let ctx = FreeKickContext {
            position: (0.5, 0.5), // Too far (midfield)
            is_home_attacking: true,
            taker_idx: 7,
            taker_name: "Midfielder".to_string(),
            freekick_skill: 16.0,
            technique: 15.0,
            long_shots: 14.0,
            has_deadball_gold: false,
        };

        let chance = direct_freekick_goal_chance(&ctx);
        assert!(chance == 0.0, "Far freekick should have 0 chance: {}", chance);
    }

    #[test]
    fn test_corner_cross_accuracy() {
        let ctx = CornerContext {
            is_home_attacking: true,
            taker_idx: 3,
            taker_name: "Winger".to_string(),
            crossing: 16.0,
            technique: 14.0,
            vision: 13.0,
            has_crosser_gold: false,
            header_targets: vec![],
        };

        let accuracy = corner_cross_accuracy(&ctx);
        assert!(accuracy > 0.3 && accuracy < 0.8, "Cross accuracy: {}", accuracy);
    }

    #[test]
    fn test_corner_cross_accuracy_gold() {
        let ctx_normal = CornerContext {
            is_home_attacking: true,
            taker_idx: 3,
            taker_name: "Winger".to_string(),
            crossing: 15.0,
            technique: 14.0,
            vision: 13.0,
            has_crosser_gold: false,
            header_targets: vec![],
        };

        let ctx_gold = CornerContext { has_crosser_gold: true, ..ctx_normal.clone() };

        let normal = corner_cross_accuracy(&ctx_normal);
        let gold = corner_cross_accuracy(&ctx_gold);
        assert!(gold > normal, "Gold should increase accuracy");
    }

    #[test]
    fn test_header_goal_chance() {
        let target = HeaderTarget {
            idx: 4,
            name: "Defender".to_string(),
            heading: 16.0,
            jumping: 15.0,
            has_airraid_gold: false,
        };

        let chance = header_goal_chance(&target);
        assert!(chance > 0.05 && chance < 0.25, "Header goal chance: {}", chance);
    }

    #[test]
    fn test_header_goal_chance_gold() {
        let target_normal = HeaderTarget {
            idx: 4,
            name: "Defender".to_string(),
            heading: 15.0,
            jumping: 14.0,
            has_airraid_gold: false,
        };

        let target_gold = HeaderTarget { has_airraid_gold: true, ..target_normal.clone() };

        let normal = header_goal_chance(&target_normal);
        let gold = header_goal_chance(&target_gold);
        assert!(gold > normal, "Gold AirRaid should increase chance");
    }

    #[test]
    fn test_find_best_header_target() {
        let targets = vec![
            HeaderTarget {
                idx: 2,
                name: "CB1".to_string(),
                heading: 14.0,
                jumping: 13.0,
                has_airraid_gold: false,
            },
            HeaderTarget {
                idx: 4,
                name: "CB2".to_string(),
                heading: 17.0,
                jumping: 16.0,
                has_airraid_gold: false,
            },
            HeaderTarget {
                idx: 9,
                name: "ST".to_string(),
                heading: 12.0,
                jumping: 11.0,
                has_airraid_gold: false,
            },
        ];

        let best = find_best_header_target(&targets);
        assert!(best.is_some());
        assert_eq!(best.unwrap().idx, 4, "CB2 should be best header");
    }

    #[test]
    fn test_corner_result_goal() {
        let ctx = CornerContext {
            is_home_attacking: true,
            taker_idx: 3,
            taker_name: "Winger".to_string(),
            crossing: 18.0,
            technique: 16.0,
            vision: 15.0,
            has_crosser_gold: false,
            header_targets: vec![HeaderTarget {
                idx: 4,
                name: "CB".to_string(),
                heading: 18.0,
                jumping: 17.0,
                has_airraid_gold: true,
            }],
        };

        // Good cross (low roll) + goal (low roll)
        let result = resolve_corner(&ctx, 0.1, 0.05, 0.5);
        assert!(matches!(result, CornerResult::HeaderGoal { .. }));
    }

    #[test]
    fn test_corner_result_cross_failed() {
        let ctx = CornerContext {
            is_home_attacking: true,
            taker_idx: 3,
            taker_name: "Winger".to_string(),
            crossing: 10.0,
            technique: 10.0,
            vision: 10.0,
            has_crosser_gold: false,
            header_targets: vec![HeaderTarget {
                idx: 4,
                name: "CB".to_string(),
                heading: 15.0,
                jumping: 14.0,
                has_airraid_gold: false,
            }],
        };

        // Bad cross (high roll)
        let result = resolve_corner(&ctx, 0.95, 0.5, 0.5);
        assert!(matches!(result, CornerResult::CrossFailed));
    }

    #[test]
    fn test_ball_out_corner() {
        // FIX_2601/0109: 경계값 수정 (0.01/0.99 → 0.0/1.0)
        // 공이 라인을 완전히 넘어야 아웃 (FIFA: "whole ball crosses")
        let result = check_ball_out((1.0, 0.8), Some(false), true); // Away touches, right side, at boundary
        assert!(matches!(result, OutOfBoundsResult::Corner { attacking_home: true }));
    }

    #[test]
    fn test_ball_out_goal_kick() {
        // FIX_2601/0109: 경계값 수정
        let result = check_ball_out((0.0, 0.8), Some(false), true); // Away touches, left side, at boundary
        assert!(matches!(result, OutOfBoundsResult::GoalKick { gk_idx: 0, .. }));
    }

    #[test]
    fn test_ball_out_throw_in() {
        // FIX_2601/0109: 경계값 수정
        let result = check_ball_out((0.5, 1.0), Some(true), true); // Home touches, top, at boundary
        assert!(matches!(result, OutOfBoundsResult::ThrowIn { throwing_home: false, .. }));
    }

    #[test]
    fn test_ball_not_out() {
        let result = check_ball_out((0.5, 0.5), None, true);
        assert!(matches!(result, OutOfBoundsResult::NotOut));
    }

    #[test]
    fn test_ball_out_goal_line_direction_flips_at_halftime() {
        // Same world position, same last touch -> different restart based on half direction.
        //
        // First half: home attacks RIGHT => home defends LEFT.
        // - Right goal line is defended by AWAY, so away last-touch => corner for HOME.
        let first_half = check_ball_out((1.0, 0.8), Some(false), true);
        assert!(matches!(first_half, OutOfBoundsResult::Corner { attacking_home: true }));

        // Second half: home attacks LEFT => home defends RIGHT.
        // - Right goal line is defended by HOME, so away last-touch => goal kick for HOME.
        let second_half = check_ball_out((1.0, 0.8), Some(false), false);
        assert!(matches!(second_half, OutOfBoundsResult::GoalKick { gk_idx: 0, .. }));
    }

    #[test]
    fn test_find_nearest_player() {
        let positions = vec![(1, (0.3, 0.3)), (2, (0.6, 0.6)), (3, (0.51, 0.51))];

        let nearest = find_nearest_player((0.5, 0.5), &positions);
        assert_eq!(nearest, Some(3), "Player 3 should be nearest");
    }
}
