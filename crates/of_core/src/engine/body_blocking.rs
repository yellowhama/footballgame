//! Body Blocking & Physical Interference
//!
//! P7 Phase 6: 선수의 물리적 몸체를 고려한 경로 차단/인터셉트 계산
//!
//! ## 핵심 기능
//! - 경로 차단 체크 (is_path_blocked)
//! - 인터셉트 가능 선수 찾기 (find_interceptors)
//! - 슛 블로커 찾기 (find_shot_blockers)
//! - 선수 충돌 해결 (resolve_player_collisions)
//!
//! ## Constants
//! - BODY_RADIUS: 0.4m (선수 몸통 반경)
//! - INFLUENCE_RADIUS: 1.5m (압박 영향 반경)
//! - INTERCEPT_RADIUS: 1.2m (패스 인터셉트 반경)

use crate::engine::player_state::PlayerState;

// ============================================================================
// Constants
// ============================================================================

/// Body Blocking 상수
pub mod constants {
    /// 기본 몸통 반경 (m)
    pub const BODY_RADIUS: f32 = 0.4;

    /// 압박 영향 반경 (m)
    pub const INFLUENCE_RADIUS: f32 = 1.5;

    /// 스탠딩 태클 반경 (m)
    pub const STANDING_TACKLE_REACH: f32 = 1.5;

    /// 슬라이딩 태클 반경 (m)
    pub const SLIDING_TACKLE_REACH: f32 = 2.2;

    /// 어깨 태클 반경 (m)
    pub const SHOULDER_TACKLE_REACH: f32 = 0.8;

    /// 패스 인터셉트 반경 (m)
    pub const INTERCEPT_RADIUS: f32 = 1.2;

    /// 볼 컨트롤 반경 (m)
    pub const BALL_CONTROL_RADIUS: f32 = 0.5;

    /// 인터셉트 시간 여유 비율 (패스 도착 시간 대비)
    pub const INTERCEPT_TIME_MARGIN: f32 = 0.8;

    /// 태클 시작 가능 최대 거리 (m)
    pub const MAX_TACKLE_START_DISTANCE: f32 = 5.0;

    /// 선수 기본 이동 속도 (m/s) - 인터셉트 시간 계산용
    pub const DEFAULT_PLAYER_SPEED: f32 = 5.0;
}

pub use constants::*;

// ============================================================================
// Player Physics
// ============================================================================

/// 선수의 물리적 속성
#[derive(Debug, Clone, Copy)]
pub struct PlayerPhysics {
    /// 몸통 반경 (충돌 판정용)
    pub body_radius: f32,

    /// 영향권 반경 (압박/간섭용)
    pub influence_radius: f32,

    /// 태클 도달 반경
    pub tackle_reach: f32,

    /// 패스 인터셉트 반경
    pub intercept_reach: f32,
}

impl Default for PlayerPhysics {
    fn default() -> Self {
        Self {
            body_radius: BODY_RADIUS,
            influence_radius: INFLUENCE_RADIUS,
            tackle_reach: STANDING_TACKLE_REACH,
            intercept_reach: INTERCEPT_RADIUS,
        }
    }
}

// ============================================================================
// Geometry Functions
// ============================================================================

/// 두 점 사이 거리
#[inline]
pub fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

/// 두 점 사이 거리의 제곱 (비교용, sqrt 생략)
#[inline]
pub fn distance_sq(a: (f32, f32), b: (f32, f32)) -> f32 {
    (b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)
}

/// 벡터 정규화
#[inline]
pub fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0.powi(2) + v.1.powi(2)).sqrt();
    if len > 0.001 {
        (v.0 / len, v.1 / len)
    } else {
        (0.0, 0.0)
    }
}

/// 각도(라디안)에서 방향 벡터
#[inline]
pub fn angle_to_direction(angle: f32) -> (f32, f32) {
    (angle.cos(), angle.sin())
}

/// 방향 벡터에서 각도(라디안)
#[inline]
pub fn direction_to_angle(dir: (f32, f32)) -> f32 {
    dir.1.atan2(dir.0)
}

/// 점에서 선분까지의 최단 거리
pub fn point_to_line_distance(
    point: (f32, f32),
    line_start: (f32, f32),
    line_end: (f32, f32),
) -> f32 {
    let (px, py) = point;
    let (x1, y1) = line_start;
    let (x2, y2) = line_end;

    let line_len_sq = (x2 - x1).powi(2) + (y2 - y1).powi(2);

    if line_len_sq < 0.0001 {
        // 선분 길이가 거의 0이면 점-점 거리
        return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
    }

    // 선분 위의 가장 가까운 점의 파라미터 t (0~1)
    let t = ((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / line_len_sq;
    let t = t.clamp(0.0, 1.0);

    // 선분 위 가장 가까운 점
    let closest_x = x1 + t * (x2 - x1);
    let closest_y = y1 + t * (y2 - y1);

    // 거리 계산
    ((px - closest_x).powi(2) + (py - closest_y).powi(2)).sqrt()
}

/// 선분 위에서 특정 점에 가장 가까운 점 찾기
pub fn find_closest_point_on_line(
    point: (f32, f32),
    line_start: (f32, f32),
    line_end: (f32, f32),
) -> (f32, f32) {
    let (px, py) = point;
    let (x1, y1) = line_start;
    let (x2, y2) = line_end;

    let line_len_sq = (x2 - x1).powi(2) + (y2 - y1).powi(2);

    if line_len_sq < 0.0001 {
        return line_start;
    }

    let t = ((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / line_len_sq;
    let t = t.clamp(0.0, 1.0);

    (x1 + t * (x2 - x1), y1 + t * (y2 - y1))
}

// ============================================================================
// Path Blocking
// ============================================================================

/// 두 점 사이의 경로가 다른 선수에 의해 차단되었는지 체크
pub fn is_path_blocked(
    from: (f32, f32),
    to: (f32, f32),
    blocker_positions: &[(f32, f32)],
    blocker_radius: f32,
) -> bool {
    for pos in blocker_positions {
        let dist = point_to_line_distance(*pos, from, to);
        if dist < blocker_radius {
            return true;
        }
    }
    false
}

/// 특정 선수를 제외하고 경로 차단 체크
pub fn is_path_blocked_except(
    from: (f32, f32),
    to: (f32, f32),
    all_positions: &[(f32, f32)],
    exclude_indices: &[usize],
    blocker_radius: f32,
) -> bool {
    for (idx, pos) in all_positions.iter().enumerate() {
        if exclude_indices.contains(&idx) {
            continue;
        }
        let dist = point_to_line_distance(*pos, from, to);
        if dist < blocker_radius {
            return true;
        }
    }
    false
}

/// 경로를 차단하는 선수들의 인덱스 반환
pub fn find_blockers(
    from: (f32, f32),
    to: (f32, f32),
    all_positions: &[(f32, f32)],
    exclude_indices: &[usize],
    blocker_radius: f32,
) -> Vec<usize> {
    let mut blockers = Vec::new();

    for (idx, pos) in all_positions.iter().enumerate() {
        if exclude_indices.contains(&idx) {
            continue;
        }
        let dist = point_to_line_distance(*pos, from, to);
        if dist < blocker_radius {
            blockers.push(idx);
        }
    }

    blockers
}

// ============================================================================
// Tackle Attempt Validation
// ============================================================================

/// 태클 시도 결과
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TackleAttemptResult {
    /// 시도 가능
    CanAttempt { approach_ticks: u8 },
    /// 선수 상태가 안 됨 (Recovering, InAction 등)
    NotReady,
    /// 쿨다운 중
    OnCooldown,
    /// 너무 멀리 있음
    TooFar,
    /// 경로에 다른 선수가 있음
    PathBlocked,
    /// 접근 각도가 나쁨 (뒤에서 → 파울 확률 높음, 여전히 가능)
    BadAngle { approach_ticks: u8 },
}

/// 태클 시도 가능 여부 체크
pub fn can_attempt_tackle(
    tackler_idx: usize,
    tackler_pos: (f32, f32),
    tackler_state: &PlayerState,
    tackler_cooldown: u8,
    ball_pos: (f32, f32),
    ball_owner_idx: usize,
    ball_owner_pos: (f32, f32),
    ball_owner_facing: f32,
    all_positions: &[(f32, f32)],
) -> TackleAttemptResult {
    // 1. 선수 상태 체크
    if !tackler_state.can_start_action() {
        return TackleAttemptResult::NotReady;
    }

    // 2. 쿨다운 체크
    if tackler_cooldown > 0 {
        return TackleAttemptResult::OnCooldown;
    }

    // 3. 거리 체크 (Approach 시작 가능 거리)
    let dist = distance(tackler_pos, ball_pos);
    if dist > MAX_TACKLE_START_DISTANCE {
        return TackleAttemptResult::TooFar;
    }

    // 4. 경로 차단 체크 (태클러-공 사이에 다른 선수 있으면 불가)
    if is_path_blocked_except(
        tackler_pos,
        ball_pos,
        all_positions,
        &[tackler_idx, ball_owner_idx],
        BODY_RADIUS,
    ) {
        return TackleAttemptResult::PathBlocked;
    }

    // 5. Approach 틱 수 계산
    let approach_ticks = calculate_approach_ticks(dist);

    // 6. 각도 체크 (뒤에서 접근하면 경고)
    let approach_angle = calculate_approach_angle(tackler_pos, ball_owner_pos, ball_owner_facing);

    if approach_angle > 150.0 {
        // 뒤에서 접근: 가능하지만 파울 확률 높음
        return TackleAttemptResult::BadAngle { approach_ticks };
    }

    TackleAttemptResult::CanAttempt { approach_ticks }
}

/// 거리 기반 Approach 틱 수 계산
pub fn calculate_approach_ticks(dist: f32) -> u8 {
    // 거리 / 접근 속도 / 틱당 시간
    // 접근 속도: ~4m/s, 틱당 시간: 0.25s
    let ticks = (dist / 4.0 / 0.25).ceil() as u8;
    ticks.clamp(1, 20) // 최소 1, 최대 20틱
}

/// 태클러의 접근 각도 계산 (0 = 정면, 180 = 뒤)
pub fn calculate_approach_angle(
    tackler_pos: (f32, f32),
    target_pos: (f32, f32),
    target_facing: f32,
) -> f32 {
    // 타겟 → 태클러 방향
    let to_tackler = (tackler_pos.0 - target_pos.0, tackler_pos.1 - target_pos.1);

    // 정규화
    let len = (to_tackler.0.powi(2) + to_tackler.1.powi(2)).sqrt();
    if len < 0.001 {
        return 0.0;
    }

    let to_tackler_normalized = (to_tackler.0 / len, to_tackler.1 / len);

    // 타겟의 전방 벡터
    let target_forward = (target_facing.cos(), target_facing.sin());

    // 내적으로 각도 계산
    let dot =
        to_tackler_normalized.0 * target_forward.0 + to_tackler_normalized.1 * target_forward.1;

    // acos의 결과는 라디안, 도로 변환
    dot.clamp(-1.0, 1.0).acos().to_degrees()
}

// ============================================================================
// Pass Interception
// ============================================================================

/// 인터셉트 후보 정보
#[derive(Debug, Clone)]
pub struct InterceptCandidate {
    pub player_idx: usize,
    pub intercept_point: (f32, f32),
    pub time_to_reach: f32,
    pub ball_arrival_time: f32,
    pub intercept_probability: f32,
}

/// 패스 경로에서 인터셉트 가능한 선수들 찾기
pub fn find_interceptors(
    pass_start: (f32, f32),
    pass_end: (f32, f32),
    pass_speed: f32,
    player_positions: &[(f32, f32)],
    player_speeds: &[f32],
    defending_team_range: std::ops::Range<usize>,
) -> Vec<InterceptCandidate> {
    let mut interceptors = Vec::new();

    if pass_speed <= 0.0 {
        return interceptors;
    }

    for idx in defending_team_range {
        if idx >= player_positions.len() || idx >= player_speeds.len() {
            continue;
        }

        let player_pos = player_positions[idx];
        let player_speed = player_speeds[idx].max(0.1); // 최소 속도

        // 패스 경로까지의 최단 거리
        let dist_to_line = point_to_line_distance(player_pos, pass_start, pass_end);

        // 인터셉트 반경 이내인지
        if dist_to_line > INFLUENCE_RADIUS + INTERCEPT_RADIUS {
            continue;
        }

        // 인터셉트 지점 계산
        let intercept_point = find_closest_point_on_line(player_pos, pass_start, pass_end);

        // 인터셉트 지점까지 도달 시간
        let dist_to_intercept = distance(player_pos, intercept_point);
        let time_to_reach = dist_to_intercept / player_speed;

        // 공이 인터셉트 지점을 지나는 시간 계산
        let dist_to_intercept_point = distance(pass_start, intercept_point);
        let ball_arrival_time = dist_to_intercept_point / pass_speed;

        // 여유를 두고 도달 가능하면 인터셉트 가능
        if time_to_reach < ball_arrival_time * INTERCEPT_TIME_MARGIN {
            interceptors.push(InterceptCandidate {
                player_idx: idx,
                intercept_point,
                time_to_reach,
                ball_arrival_time,
                intercept_probability: calculate_intercept_probability(
                    time_to_reach,
                    ball_arrival_time,
                    dist_to_line,
                ),
            });
        }
    }

    // 확률 높은 순으로 정렬
    // FIX_2601/0110: Use secondary key to break ties and avoid index order bias
    interceptors.sort_by(|a, b| {
        match b.intercept_probability.partial_cmp(&a.intercept_probability) {
            Some(std::cmp::Ordering::Equal) | None => {
                // Tie-breaker: use time_to_reach (faster player first)
                a.time_to_reach.partial_cmp(&b.time_to_reach).unwrap_or(std::cmp::Ordering::Equal)
            }
            Some(ord) => ord,
        }
    });

    interceptors
}

/// 인터셉트 성공 확률 계산
fn calculate_intercept_probability(
    time_to_reach: f32,
    ball_arrival_time: f32,
    dist_to_line: f32,
) -> f32 {
    // 시간 여유가 많을수록 확률 높음
    let time_ratio = if ball_arrival_time > 0.0 {
        1.0 - (time_to_reach / ball_arrival_time).min(1.0)
    } else {
        0.0
    };

    // 거리가 가까울수록 확률 높음
    let dist_factor = 1.0 - (dist_to_line / INFLUENCE_RADIUS).min(1.0);

    // 두 요소 결합
    (time_ratio * 0.6 + dist_factor * 0.4).clamp(0.0, 1.0)
}

// ============================================================================
// Shot Blocking
// ============================================================================

/// 블락 후보 정보
#[derive(Debug, Clone)]
pub struct BlockCandidate {
    pub player_idx: usize,
    pub block_point: (f32, f32),
    pub dist_to_line: f32,
    pub ball_arrival_time: f32,
    pub block_probability: f32,
}

/// 슈팅 경로를 막을 수 있는 선수들 찾기
pub fn find_shot_blockers(
    shot_start: (f32, f32),
    shot_end: (f32, f32),
    shot_speed: f32,
    player_positions: &[(f32, f32)],
    player_states: &[PlayerState],
    defending_team_range: std::ops::Range<usize>,
) -> Vec<BlockCandidate> {
    let mut blockers = Vec::new();

    if shot_speed <= 0.0 {
        return blockers;
    }

    for idx in defending_team_range {
        if idx >= player_positions.len() || idx >= player_states.len() {
            continue;
        }

        // InAction이나 Recovering 상태면 블락 불가
        if !player_states[idx].can_start_action() {
            continue;
        }

        let player_pos = player_positions[idx];
        let dist_to_line = point_to_line_distance(player_pos, shot_start, shot_end);

        // 슛은 빠르므로 좁은 반경만 체크
        if dist_to_line > BODY_RADIUS + 0.5 {
            continue;
        }

        // 공이 지나가는 지점
        let block_point = find_closest_point_on_line(player_pos, shot_start, shot_end);
        let dist_to_block_point = distance(shot_start, block_point);

        // 이미 슈터 뒤에 있으면 블락 불가
        if dist_to_block_point < 0.5 {
            continue;
        }

        let ball_arrival_time = dist_to_block_point / shot_speed;

        blockers.push(BlockCandidate {
            player_idx: idx,
            block_point,
            dist_to_line,
            ball_arrival_time,
            block_probability: calculate_block_probability(dist_to_line),
        });
    }

    blockers
}

/// 블락 확률 계산 (거리 기반)
fn calculate_block_probability(dist_to_line: f32) -> f32 {
    if dist_to_line < BODY_RADIUS {
        0.9 // 거의 확실히 블락
    } else if dist_to_line < BODY_RADIUS + 0.3 {
        0.5 // 반반
    } else {
        0.2 // 낮은 확률
    }
}

// ============================================================================
// Body Collision
// ============================================================================

/// 충돌 정보
#[derive(Debug, Clone, Copy)]
pub struct CollisionInfo {
    /// 겹침 거리 (m)
    pub overlap: f32,
    /// 충돌 방향 (player1 → player2)
    pub direction: (f32, f32),
    /// 분리 벡터 (각 선수가 이만큼 이동해야 분리됨)
    pub separation_vector: (f32, f32),
}

/// 두 선수의 충돌 체크
pub fn check_player_collision(
    player1_pos: (f32, f32),
    player2_pos: (f32, f32),
) -> Option<CollisionInfo> {
    let dist = distance(player1_pos, player2_pos);

    if dist < BODY_RADIUS * 2.0 {
        let overlap = BODY_RADIUS * 2.0 - dist;
        let direction = if dist > 0.001 {
            ((player2_pos.0 - player1_pos.0) / dist, (player2_pos.1 - player1_pos.1) / dist)
        } else {
            (1.0, 0.0) // 기본 방향
        };

        Some(CollisionInfo {
            overlap,
            direction,
            separation_vector: (direction.0 * overlap * 0.5, direction.1 * overlap * 0.5),
        })
    } else {
        None
    }
}

/// 선수 간 충돌 해결 (위치 조정)
pub fn resolve_player_collisions(positions: &mut [(f32, f32)], player_states: &[PlayerState]) {
    let n = positions.len().min(player_states.len());

    for i in 0..n {
        for j in (i + 1)..n {
            if let Some(collision) = check_player_collision(positions[i], positions[j]) {
                // InAction 상태인 선수는 밀리지 않음
                let i_movable = player_states[i].can_be_pushed();
                let j_movable = player_states[j].can_be_pushed();

                match (i_movable, j_movable) {
                    (true, true) => {
                        // 둘 다 이동 가능: 반반씩 밀림
                        positions[i].0 -= collision.separation_vector.0;
                        positions[i].1 -= collision.separation_vector.1;
                        positions[j].0 += collision.separation_vector.0;
                        positions[j].1 += collision.separation_vector.1;
                    }
                    (true, false) => {
                        // i만 이동 가능
                        positions[i].0 -= collision.separation_vector.0 * 2.0;
                        positions[i].1 -= collision.separation_vector.1 * 2.0;
                    }
                    (false, true) => {
                        // j만 이동 가능
                        positions[j].0 += collision.separation_vector.0 * 2.0;
                        positions[j].1 += collision.separation_vector.1 * 2.0;
                    }
                    (false, false) => {
                        // 둘 다 이동 불가: 그대로 유지
                    }
                }
            }
        }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance() {
        assert!((distance((0.0, 0.0), (3.0, 4.0)) - 5.0).abs() < 0.001);
        assert!((distance((1.0, 1.0), (1.0, 1.0))).abs() < 0.001);
    }

    #[test]
    fn test_point_to_line_distance() {
        // 선분 (0,0) -> (10,0), 점 (5, 3)
        let dist = point_to_line_distance((5.0, 3.0), (0.0, 0.0), (10.0, 0.0));
        assert!((dist - 3.0).abs() < 0.001);

        // 점이 선분 시작점 밖에 있을 때
        let dist2 = point_to_line_distance((-2.0, 0.0), (0.0, 0.0), (10.0, 0.0));
        assert!((dist2 - 2.0).abs() < 0.001);

        // 점이 선분 끝점 밖에 있을 때
        let dist3 = point_to_line_distance((12.0, 0.0), (0.0, 0.0), (10.0, 0.0));
        assert!((dist3 - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_find_closest_point_on_line() {
        let closest = find_closest_point_on_line((5.0, 3.0), (0.0, 0.0), (10.0, 0.0));
        assert!((closest.0 - 5.0).abs() < 0.001);
        assert!((closest.1 - 0.0).abs() < 0.001);

        // 점이 선분 밖에 있을 때 - 끝점 반환
        let closest2 = find_closest_point_on_line((-2.0, 0.0), (0.0, 0.0), (10.0, 0.0));
        assert!((closest2.0 - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_is_path_blocked() {
        let blockers = vec![(5.0, 0.3), (20.0, 0.0)];

        // (0,0) -> (10,0) 사이에 (5, 0.3)가 있음 (BODY_RADIUS = 0.4)
        assert!(is_path_blocked((0.0, 0.0), (10.0, 0.0), &blockers, BODY_RADIUS));

        // (0,0) -> (10,10) 사이에 blocker 없음
        assert!(!is_path_blocked((0.0, 0.0), (10.0, 10.0), &blockers, BODY_RADIUS));
    }

    #[test]
    fn test_is_path_blocked_except() {
        let positions = vec![
            (0.0, 0.0),  // idx 0: 시작점
            (10.0, 0.0), // idx 1: 끝점
            (5.0, 0.3),  // idx 2: 경로 위
        ];

        // 2번을 제외하면 차단 안됨
        assert!(!is_path_blocked_except(
            positions[0],
            positions[1],
            &positions,
            &[0, 1, 2],
            BODY_RADIUS
        ));

        // 2번 포함하면 차단됨
        assert!(is_path_blocked_except(
            positions[0],
            positions[1],
            &positions,
            &[0, 1],
            BODY_RADIUS
        ));
    }

    #[test]
    fn test_find_blockers() {
        let positions = vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (5.0, 0.3),
            (5.0, 5.0), // 경로에서 떨어진 곳
        ];

        let blockers = find_blockers(
            (0.0, 0.0),
            (10.0, 0.0),
            &positions,
            &[0, 1], // 시작점과 끝점 제외
            BODY_RADIUS,
        );

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0], 2);
    }

    #[test]
    fn test_approach_angle() {
        // 타겟이 동쪽(0도)을 보고 있고, 태클러가 서쪽에서 접근 (뒤에서)
        let angle = calculate_approach_angle(
            (0.0, 0.0), // 태클러
            (5.0, 0.0), // 타겟
            0.0,        // 타겟 facing (동쪽)
        );
        assert!((angle - 180.0).abs() < 1.0);

        // 태클러가 동쪽에서 접근 (정면)
        let angle2 = calculate_approach_angle((10.0, 0.0), (5.0, 0.0), 0.0);
        assert!(angle2 < 10.0);

        // 태클러가 옆에서 접근 (90도)
        let angle3 = calculate_approach_angle((5.0, 5.0), (5.0, 0.0), 0.0);
        assert!((angle3 - 90.0).abs() < 10.0);
    }

    #[test]
    fn test_calculate_approach_ticks() {
        // 2m 거리: 2 / 4 / 0.25 = 2 ticks
        assert_eq!(calculate_approach_ticks(2.0), 2);

        // 0.5m 거리: 최소 1 tick
        assert_eq!(calculate_approach_ticks(0.5), 1);

        // 100m 거리: 최대 20 ticks
        assert_eq!(calculate_approach_ticks(100.0), 20);
    }

    #[test]
    fn test_can_attempt_tackle() {
        let positions = vec![
            (0.0, 0.0),  // tackler
            (3.0, 0.0),  // ball owner
            (10.0, 0.0), // third player (not blocking)
        ];

        // 태클러가 정면에서 접근하도록 facing을 서쪽(PI)으로 설정
        let result = can_attempt_tackle(
            0,
            positions[0],
            &PlayerState::Idle,
            0,
            positions[1],
            1,
            positions[1],
            std::f32::consts::PI, // 서쪽을 향함 → 태클러가 정면에서 접근
            &positions,
        );

        assert!(matches!(result, TackleAttemptResult::CanAttempt { .. }));
    }

    #[test]
    fn test_can_attempt_tackle_bad_angle() {
        let positions = vec![
            (0.0, 0.0), // tackler
            (3.0, 0.0), // ball owner
        ];

        // 공 소유자가 동쪽(0)을 향하면 태클러는 뒤에서 접근하는 것
        let result = can_attempt_tackle(
            0,
            positions[0],
            &PlayerState::Idle,
            0,
            positions[1],
            1,
            positions[1],
            0.0, // 동쪽을 향함 → 태클러가 뒤에서 접근
            &positions,
        );

        assert!(matches!(result, TackleAttemptResult::BadAngle { .. }));
    }

    #[test]
    fn test_can_attempt_tackle_too_far() {
        let positions = vec![(0.0, 0.0), (10.0, 0.0)];

        let result = can_attempt_tackle(
            0,
            positions[0],
            &PlayerState::Idle,
            0,
            positions[1],
            1,
            positions[1],
            0.0,
            &positions,
        );

        assert!(matches!(result, TackleAttemptResult::TooFar));
    }

    #[test]
    fn test_can_attempt_tackle_on_cooldown() {
        let positions = vec![(0.0, 0.0), (2.0, 0.0)];

        let result = can_attempt_tackle(
            0,
            positions[0],
            &PlayerState::Idle,
            5, // 쿨다운 중
            positions[1],
            1,
            positions[1],
            0.0,
            &positions,
        );

        assert!(matches!(result, TackleAttemptResult::OnCooldown));
    }

    #[test]
    fn test_find_interceptors() {
        let positions = vec![
            (0.0, 0.0),  // passer
            (10.0, 0.0), // receiver
            (5.0, 1.0),  // defender (경로 근처)
            (5.0, 10.0), // defender (경로에서 멀리)
        ];
        let speeds = vec![5.0, 5.0, 5.0, 5.0];

        let interceptors = find_interceptors(
            positions[0],
            positions[1],
            10.0, // pass speed
            &positions,
            &speeds,
            2..4, // defenders
        );

        // 경로 근처의 수비수만 인터셉터로 선택되어야 함
        assert_eq!(interceptors.len(), 1);
        assert_eq!(interceptors[0].player_idx, 2);
    }

    #[test]
    fn test_find_shot_blockers() {
        let positions = vec![
            (0.0, 0.0), // shooter
            (5.0, 0.3), // defender close to shot path
            (5.0, 5.0), // defender far from shot path
        ];
        let states = vec![PlayerState::Idle, PlayerState::Idle, PlayerState::Idle];

        let blockers = find_shot_blockers(
            positions[0],
            (10.0, 0.0), // shot target
            20.0,        // shot speed
            &positions,
            &states,
            1..3, // defenders
        );

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].player_idx, 1);
    }

    #[test]
    fn test_player_collision() {
        // 두 선수가 겹쳐있음 (거리 0.5 < BODY_RADIUS * 2 = 0.8)
        let collision = check_player_collision((0.0, 0.0), (0.5, 0.0));
        assert!(collision.is_some());

        let info = collision.unwrap();
        assert!(info.overlap > 0.0);
        assert!((info.overlap - 0.3).abs() < 0.001); // 0.8 - 0.5 = 0.3

        // 두 선수가 떨어져있음
        let no_collision = check_player_collision((0.0, 0.0), (2.0, 0.0));
        assert!(no_collision.is_none());
    }

    #[test]
    fn test_resolve_player_collisions() {
        let mut positions = vec![(0.0, 0.0), (0.5, 0.0)];
        let states = vec![PlayerState::Idle, PlayerState::Idle];

        resolve_player_collisions(&mut positions, &states);

        // 충돌 해결 후 두 선수가 BODY_RADIUS * 2 이상 떨어져야 함
        let dist = distance(positions[0], positions[1]);
        assert!(dist >= BODY_RADIUS * 2.0 - 0.01);
    }

    #[test]
    fn test_intercept_probability() {
        // 시간 여유가 많고 거리도 가까울 때: 높은 확률
        let high_prob = calculate_intercept_probability(0.2, 1.0, 0.5);
        assert!(high_prob > 0.6);

        // 시간 여유가 거의 없을 때: 낮은 확률
        let low_prob = calculate_intercept_probability(0.9, 1.0, 1.5);
        assert!(low_prob < 0.3);
    }

    #[test]
    fn test_normalize() {
        let (x, y) = normalize((3.0, 4.0));
        assert!((x - 0.6).abs() < 0.001);
        assert!((y - 0.8).abs() < 0.001);

        // 길이 0인 벡터
        let (x2, y2) = normalize((0.0, 0.0));
        assert_eq!((x2, y2), (0.0, 0.0));
    }
}
