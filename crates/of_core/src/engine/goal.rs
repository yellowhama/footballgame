//! Goal Contract - The Football World Rules (축구 세계의 헌법)
//!
//! P0: 엔진이 "축구 규칙"을 알고 플레이하게 만든다
//!
//! ## 핵심 규칙
//! 1. 우리팀이 공을 가지고 있다 → 공격 → 상대방 골대에 공을 넣어야 점수
//! 2. 상대팀이 공을 가지고 있다 → 수비 → 우리 골대에 공이 들어가는걸 막아야
//! 3. 경기 끝났을때 점수가 높은 팀이 이긴다
//!
//! ## Goal 소유권
//! - Home 골대 (x=0) = Home 팀이 지키는 골대
//! - Away 골대 (x=105) = Away 팀이 지키는 골대
//! - Home 팀이 공격할 골대 = Away 골대 (x=105)
//! - Away 팀이 공격할 골대 = Home 골대 (x=0)

use super::coordinates::MeterPos;
use super::physics_constants::{field, goal as goal_const};
use super::tactical_context::TeamSide;

/// 골대 정의 - 축구 세계의 절대 규칙
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Goal {
    /// 이 골대를 지키는 팀 (소유자)
    pub owner: TeamSide,
    /// 골대 중심 위치 (미터 좌표)
    pub center: MeterPos,
    /// 골대 너비 (7.32m)
    pub width: f32,
    /// 골대 높이 (2.44m)
    pub height: f32,
}

impl Goal {
    /// FIFA 표준 골대 너비
    pub const WIDTH: f32 = goal_const::WIDTH_M;
    /// FIFA 표준 골대 높이
    pub const HEIGHT: f32 = goal_const::HEIGHT_M;

    /// Home 팀이 지키는 골대 (x=0, 왼쪽)
    pub fn home_goal() -> Self {
        Self {
            owner: TeamSide::Home,
            center: (0.0, field::CENTER_Y),
            width: Self::WIDTH,
            height: Self::HEIGHT,
        }
    }

    /// Away 팀이 지키는 골대 (x=105, 오른쪽)
    pub fn away_goal() -> Self {
        Self {
            owner: TeamSide::Away,
            center: (field::LENGTH_M, field::CENTER_Y),
            width: Self::WIDTH,
            height: Self::HEIGHT,
        }
    }

    /// 골대 Y 범위 (중심 ± width/2)
    pub fn y_range(&self) -> (f32, f32) {
        let half_width = self.width / 2.0;
        (self.center.1 - half_width, self.center.1 + half_width)
    }

    /// 골대 안에 있는지 체크 (x, y, height 모두 고려)
    ///
    /// FIX_2601/0109: GOAL_LINE_EPSILON 수정 (1.5m → 0.15m)
    /// FIFA Laws: "whole ball crosses the goal line" - 공 전체가 라인을 넘어야 골
    /// 공 반지름 ≈ 0.11m이므로, 0.15m (반지름 + 허용오차)로 설정
    pub fn contains(&self, ball_pos: MeterPos, ball_height: f32) -> bool {
        /// 골라인 통과 판정을 위한 최소 거리 (미터)
        /// FIFA 규정: 공 전체가 라인을 완전히 넘어야 골
        /// 공 반지름(0.11m) + 허용오차(0.04m) = 0.15m
        const GOAL_LINE_EPSILON: f32 = 0.15;

        let (y_min, y_max) = self.y_range();

        // Y 범위 체크
        let y_in_range = ball_pos.1 >= y_min && ball_pos.1 <= y_max;

        // 높이 체크 (0 ~ 2.44m)
        let height_in_range = ball_height >= 0.0 && ball_height <= self.height;

        // 골라인 통과 체크 (x 좌표) - EPSILON 적용
        // 공이 골라인을 "확실히" 넘어야 골로 인정
        let crossed_line = match self.owner {
            TeamSide::Home => ball_pos.0 < -GOAL_LINE_EPSILON,
            TeamSide::Away => ball_pos.0 > field::LENGTH_M + GOAL_LINE_EPSILON,
        };

        crossed_line && y_in_range && height_in_range
    }

    /// 위치에서 이 골대까지의 거리
    pub fn distance_from(&self, pos: MeterPos) -> f32 {
        let dx = self.center.0 - pos.0;
        let dy = self.center.1 - pos.1;
        (dx * dx + dy * dy).sqrt()
    }

    /// 위치에서 이 골대로의 각도 (라디안)
    pub fn angle_from(&self, pos: MeterPos) -> f32 {
        let dx = self.center.0 - pos.0;
        let dy = self.center.1 - pos.1;
        dy.atan2(dx)
    }

    /// 슛 각도 계산 (골대 양쪽 포스트 사이의 시야각)
    pub fn shot_angle_from(&self, pos: MeterPos) -> f32 {
        let (y_min, y_max) = self.y_range();

        // 양쪽 포스트까지의 벡터
        let to_post1 = (self.center.0 - pos.0, y_min - pos.1);
        let to_post2 = (self.center.0 - pos.0, y_max - pos.1);

        // 두 벡터 사이의 각도
        let dot = to_post1.0 * to_post2.0 + to_post1.1 * to_post2.1;
        let mag1 = (to_post1.0 * to_post1.0 + to_post1.1 * to_post1.1).sqrt();
        let mag2 = (to_post2.0 * to_post2.0 + to_post2.1 * to_post2.1).sqrt();

        if mag1 * mag2 > 0.0 {
            (dot / (mag1 * mag2)).clamp(-1.0, 1.0).acos()
        } else {
            0.0
        }
    }
}

/// 경기장의 두 골대
#[derive(Debug, Clone, Copy)]
pub struct Goals {
    /// Home 팀이 지키는 골대 (index 0, x=0)
    pub home: Goal,
    /// Away 팀이 지키는 골대 (index 1, x=105)
    pub away: Goal,
}

impl Default for Goals {
    fn default() -> Self {
        Self::new()
    }
}

impl Goals {
    pub fn new() -> Self {
        Self { home: Goal::home_goal(), away: Goal::away_goal() }
    }

    /// 팀이 지켜야 할 골대 (실점하면 안 되는 골대)
    ///
    /// Home 팀 → Home 골대 (x=0)
    /// Away 팀 → Away 골대 (x=105)
    pub fn defending_goal(&self, team: TeamSide) -> &Goal {
        match team {
            TeamSide::Home => &self.home,
            TeamSide::Away => &self.away,
        }
    }

    /// 팀이 공격해야 할 골대 (득점해야 하는 골대)
    ///
    /// Home 팀 → Away 골대 (x=105)
    /// Away 팀 → Home 골대 (x=0)
    pub fn attacking_goal(&self, team: TeamSide) -> &Goal {
        match team {
            TeamSide::Home => &self.away,
            TeamSide::Away => &self.home,
        }
    }

    /// 선수 인덱스로 공격 골대 조회
    ///
    /// 선수 0~10: Home 팀 → Away 골대 (x=105)
    /// 선수 11~21: Away 팀 → Home 골대 (x=0)
    pub fn attacking_goal_for_player(&self, player_idx: usize) -> &Goal {
        let team = TeamSide::from_track_id(player_idx);
        self.attacking_goal(team)
    }

    /// 선수 인덱스로 수비 골대 조회
    ///
    /// 선수 0~10: Home 팀 → Home 골대 (x=0)
    /// 선수 11~21: Away 팀 → Away 골대 (x=105)
    pub fn defending_goal_for_player(&self, player_idx: usize) -> &Goal {
        let team = TeamSide::from_track_id(player_idx);
        self.defending_goal(team)
    }

    /// 공 위치로 골 체크 - 어느 팀이 득점했는지 반환
    ///
    /// Returns: Some(scoring_team) if goal scored, None otherwise
    pub fn check_goal(&self, ball_pos: MeterPos, ball_height: f32) -> Option<TeamSide> {
        // Home 골대에 공이 들어감 = Away 팀 득점
        if self.home.contains(ball_pos, ball_height) {
            return Some(TeamSide::Away);
        }

        // Away 골대에 공이 들어감 = Home 팀 득점
        if self.away.contains(ball_pos, ball_height) {
            return Some(TeamSide::Home);
        }

        None
    }
}

// P17: TeamSide impl이 models/match_setup.rs로 이동됨
// - opponent(), from_player_idx(): models/match_setup.rs
// - is_home_team(), is_away_team(): models/match_setup.rs (이름 변경)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_positions() {
        let goals = Goals::new();

        // Home 골대는 x=0에 있다
        assert_eq!(goals.home.center.0, 0.0);
        assert_eq!(goals.home.owner, TeamSide::Home);

        // Away 골대는 x=105에 있다
        assert_eq!(goals.away.center.0, field::LENGTH_M);
        assert_eq!(goals.away.owner, TeamSide::Away);

        // 두 골대 모두 y=34 (필드 중심)에 있다
        assert_eq!(goals.home.center.1, field::CENTER_Y);
        assert_eq!(goals.away.center.1, field::CENTER_Y);
    }

    #[test]
    fn test_attacking_defending_goals() {
        let goals = Goals::new();

        // Home 팀: 지키는 골대 = Home (x=0), 공격 골대 = Away (x=105)
        assert_eq!(goals.defending_goal(TeamSide::Home).center.0, 0.0);
        assert_eq!(goals.attacking_goal(TeamSide::Home).center.0, field::LENGTH_M);

        // Away 팀: 지키는 골대 = Away (x=105), 공격 골대 = Home (x=0)
        assert_eq!(goals.defending_goal(TeamSide::Away).center.0, field::LENGTH_M);
        assert_eq!(goals.attacking_goal(TeamSide::Away).center.0, 0.0);
    }

    #[test]
    fn test_player_goals() {
        let goals = Goals::new();

        // Home 선수 (0~10): 공격 골대 = Away (x=105)
        assert_eq!(goals.attacking_goal_for_player(0).center.0, field::LENGTH_M);
        assert_eq!(
            goals.attacking_goal_for_player(10).center.0,
            field::LENGTH_M
        );

        // Away 선수 (11~21): 공격 골대 = Home (x=0)
        assert_eq!(goals.attacking_goal_for_player(11).center.0, 0.0);
        assert_eq!(goals.attacking_goal_for_player(21).center.0, 0.0);
    }

    #[test]
    fn test_goal_check() {
        let goals = Goals::new();

        // FIX_2601/0107: GOAL_LINE_EPSILON=1.5m 적용
        // Home 골대에 공이 들어감 (x < -1.5, y in range, height in range)
        // = Away 팀 득점
        let ball_in_home_goal = (-2.0, field::CENTER_Y);
        assert_eq!(goals.check_goal(ball_in_home_goal, 1.0), Some(TeamSide::Away));

        // Away 골대에 공이 들어감 (x > 106.5, y in range, height in range)
        // = Home 팀 득점
        let ball_in_away_goal = (107.0, field::CENTER_Y);
        assert_eq!(goals.check_goal(ball_in_away_goal, 1.0), Some(TeamSide::Home));

        // 골대 밖 (y out of range)
        let ball_wide = (-2.0, 20.0);
        assert_eq!(goals.check_goal(ball_wide, 1.0), None);

        // 골대 밖 (height too high)
        let ball_high = (-2.0, field::CENTER_Y);
        assert_eq!(goals.check_goal(ball_high, 3.0), None);

        // 필드 안
        let ball_in_field = (50.0, field::CENTER_Y);
        assert_eq!(goals.check_goal(ball_in_field, 0.5), None);

        // FIX_2601/0109: GOAL_LINE_EPSILON 0.15m로 수정
        // 경계값 테스트: -0.15m는 골 아님 (정확히 경계)
        let ball_at_boundary = (-0.15, field::CENTER_Y);
        assert_eq!(goals.check_goal(ball_at_boundary, 1.0), None);

        // 경계값 테스트: -0.16m는 골 (whole ball crossed)
        let ball_past_boundary = (-0.16, field::CENTER_Y);
        assert_eq!(goals.check_goal(ball_past_boundary, 1.0), Some(TeamSide::Away));
    }

    #[test]
    fn test_goal_distance() {
        let goals = Goals::new();

        // 필드 중앙에서 Away 골대까지 거리
        let center = (field::CENTER_X, field::CENTER_Y);
        let dist = goals.attacking_goal(TeamSide::Home).distance_from(center);
        assert!((dist - field::CENTER_X).abs() < 0.1);

        // 필드 중앙에서 Home 골대까지 거리
        let dist = goals.defending_goal(TeamSide::Home).distance_from(center);
        assert!((dist - field::CENTER_X).abs() < 0.1);
    }

    #[test]
    fn test_shot_angle() {
        let goals = Goals::new();
        let target = goals.attacking_goal(TeamSide::Home);

        // 정면에서 멀리 있을 때 각도가 작음
        let far_center = (80.0, field::CENTER_Y);
        let angle_far = target.shot_angle_from(far_center);

        // 정면에서 가까이 있을 때 각도가 큼
        let close_center = (100.0, field::CENTER_Y);
        let angle_close = target.shot_angle_from(close_center);

        assert!(
            angle_close > angle_far,
            "가까울수록 슛 각도가 커야 함: close={}, far={}",
            angle_close,
            angle_far
        );

        // 측면에서는 각도가 작음
        let side = (100.0, 10.0);
        let angle_side = target.shot_angle_from(side);
        assert!(
            angle_side < angle_close,
            "측면에서는 각도가 작아야 함: side={}, center={}",
            angle_side,
            angle_close
        );
    }

    #[test]
    fn test_team_side_opponent() {
        assert_eq!(TeamSide::Home.opponent(), TeamSide::Away);
        assert_eq!(TeamSide::Away.opponent(), TeamSide::Home);
    }

    #[test]
    fn test_team_side_from_player_idx() {
        assert_eq!(TeamSide::from_player_idx(0), TeamSide::Home);
        assert_eq!(TeamSide::from_player_idx(10), TeamSide::Home);
        assert_eq!(TeamSide::from_player_idx(11), TeamSide::Away);
        assert_eq!(TeamSide::from_player_idx(21), TeamSide::Away);
    }
}
