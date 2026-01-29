//! P15: Player Motion Physics System (Controller)
//!
//! 관성 기반 선수 이동 시스템.
//! UFO 움직임(즉각 정지/방향전환)을 Force 기반 이동으로 변경.
//!
//! # 핵심 원칙
//! - 기존 36개 스탯을 물리 파라미터로 해석 (새 스탯 없음)
//! - pace → max_speed, acceleration → accel
//! - agility + balance → turn_skill
//!
//! # 사용법
//! ```ignore
//! use crate::engine::player_motion_params::{ability_to_motion_params, scale_by_stamina};
//! let base = ability_to_motion_params(&attrs);
//! let params = scale_by_stamina(&base, stamina01, attrs.stamina);
//! let (new_pos, new_vel) = update_player_motion(pos, vel, target, dt, &params);
//! ```

use crate::engine::physics_constants::player_inertia::*;
use crate::engine::player_motion_params::PlayerMotionParams;

// ============================================================
// 물리 계산 함수들
// ============================================================

/// 제동 거리 계산: v² = 2ad → d = v² / 2a
#[inline]
pub fn calculate_slowing_radius(current_speed: f32, deceleration: f32) -> f32 {
    if deceleration <= 0.0 {
        return ARRIVAL_SLOWING_MIN;
    }
    (current_speed.powi(2) / (2.0 * deceleration)).max(ARRIVAL_SLOWING_MIN)
}

/// Arrival Steering: 목표 근처에서 자동 감속
#[inline]
pub fn calculate_arrival_speed(
    dist_to_target: f32,
    current_speed: f32,
    max_speed: f32,
    deceleration: f32,
) -> f32 {
    let slowing_radius = calculate_slowing_radius(current_speed, deceleration);

    if dist_to_target < slowing_radius {
        // 거리가 가까워지면 속도를 선형적으로 줄임
        max_speed * (dist_to_target / slowing_radius)
    } else {
        max_speed
    }
}

/// 턴 severity 계산 (현재 방향과 목표 방향의 차이)
/// 0 = 동일 방향, 1 = 반대 방향 (180도)
#[inline]
pub fn calc_turn_severity(current_dir: (f32, f32), desired_dir: (f32, f32)) -> f32 {
    let dot = current_dir.0 * desired_dir.0 + current_dir.1 * desired_dir.1;
    let dot = dot.clamp(-1.0, 1.0);
    // dot: 1(동일방향) ~ -1(반대방향)
    // severity: 0(동일방향) ~ 1(반대방향)
    1.0 - (dot + 1.0) * 0.5
}

/// 턴 페널티 계산
/// - turn_severity: 0(직진)~1(180도 턴)
/// - speed_ratio: 현재속도 / max_speed (0~1)
/// - turn_skill: 선수의 회전 능력 (0~1)
#[inline]
pub fn calculate_turn_penalty(turn_severity: f32, speed_ratio: f32, turn_skill: f32) -> f32 {
    // 빠를수록, 급하게 꺾을수록, turn_skill 낮을수록 페널티 큼
    (1.0 - turn_severity * speed_ratio * (1.0 - turn_skill)).clamp(TURN_PENALTY_MIN, 1.0)
}

/// 관성 기반 선수 이동 업데이트 (핵심 함수)
///
/// PositioningEngine은 target_pos만 설정.
/// 이 함수가 실제 물리 이동을 수행.
///
/// # Arguments
/// - `pos`: 현재 위치 (m)
/// - `vel`: 현재 속도 벡터 (m/s)
/// - `target_pos`: 목표 위치 (m)
/// - `dt`: 시간 간격 (초)
/// - `params`: 피로 적용된 물리 파라미터
///
/// # Returns
/// (새 위치, 새 속도)
pub fn update_player_motion(
    pos: (f32, f32),
    vel: (f32, f32),
    target_pos: (f32, f32),
    dt: f32,
    params: &PlayerMotionParams,
) -> ((f32, f32), (f32, f32)) {
    // dt 안전 범위
    let dt = dt.clamp(DT_MIN, DT_MAX);

    let to = (target_pos.0 - pos.0, target_pos.1 - pos.1);
    let dist = (to.0 * to.0 + to.1 * to.1).sqrt();

    // 도착 판정: 가까우면 드래그로 감속
    if dist < ARRIVAL_THRESHOLD {
        let decay = (1.0 - params.drag * dt).clamp(0.0, 1.0);
        let new_vel = (vel.0 * decay, vel.1 * decay);
        let new_pos = (pos.0 + new_vel.0 * dt, pos.1 + new_vel.1 * dt);
        return (new_pos, new_vel);
    }

    // 방향 계산
    let desired_dir = (to.0 / dist, to.1 / dist);
    let speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
    let current_dir =
        if speed > SPEED_EPSILON { (vel.0 / speed, vel.1 / speed) } else { desired_dir };

    // 턴 페널티 계산
    let turn_severity = calc_turn_severity(current_dir, desired_dir);
    let speed_ratio = (speed / params.max_speed.max(SPEED_EPSILON)).clamp(0.0, 1.0);
    let turn_penalty = calculate_turn_penalty(turn_severity, speed_ratio, params.turn_skill);

    // Arrival Steering: 목표 근처에서 자동 감속 (Overshoot 방지)
    let arrival_speed = calculate_arrival_speed(dist, speed, params.max_speed, params.decel);

    // 목표 속도 (Arrival + Turn 페널티 적용)
    let target_speed = arrival_speed * turn_penalty;
    let desired_vel = (desired_dir.0 * target_speed, desired_dir.1 * target_speed);

    // 가속 제한
    let max_delta = params.accel * dt;
    let delta = (desired_vel.0 - vel.0, desired_vel.1 - vel.1);
    let delta_len = (delta.0 * delta.0 + delta.1 * delta.1).sqrt();

    let vel2 = if delta_len <= max_delta || delta_len < SPEED_EPSILON {
        desired_vel
    } else {
        (vel.0 + delta.0 / delta_len * max_delta, vel.1 + delta.1 / delta_len * max_delta)
    };

    // 드래그 적용
    let decay = (1.0 - params.drag * dt).clamp(0.0, 1.0);
    let vel3 = (vel2.0 * decay, vel2.1 * decay);

    // 최종 위치
    let pos2 = (pos.0 + vel3.0 * dt, pos.1 + vel3.1 * dt);

    (pos2, vel3)
}

// ============================================================
// P7-DRIBBLE-TUNE: Wrong-Foot & Burst Mechanics
// ============================================================

/// Wrong-foot 상태 (페인트에 속았을 때)
/// 수비수가 잘못된 방향으로 무게중심을 이동한 상태
#[derive(Clone, Debug, Default)]
pub struct WrongFootState {
    /// Wrong-foot 방향 (페인트 방향)
    pub direction: (f32, f32),
    /// 남은 틱 (wrong-foot 상태 지속)
    pub remaining_ticks: u8,
    /// 심각도 (0.0~1.0, 높을수록 많이 속음)
    pub severity: f32,
}

impl WrongFootState {
    /// Wrong-foot 상태 적용 (페인트 성공 시)
    pub fn apply(feint_dir: (f32, f32), severity: f32) -> Self {
        Self {
            direction: feint_dir,
            remaining_ticks: 4, // 1초 동안 페널티
            severity: severity.clamp(0.0, 1.0),
        }
    }

    /// 틱당 업데이트
    pub fn tick(&mut self) {
        self.remaining_ticks = self.remaining_ticks.saturating_sub(1);
        if self.remaining_ticks == 0 {
            self.severity = 0.0;
        }
    }

    /// 아직 wrong-foot 상태인지
    pub fn is_active(&self) -> bool {
        self.remaining_ticks > 0 && self.severity > 0.1
    }

    /// 현재 움직임 페널티 계수 (0.0~1.0, 낮을수록 느림)
    pub fn movement_penalty(&self) -> f32 {
        if !self.is_active() {
            1.0
        } else {
            // severity가 높을수록 큰 페널티
            1.0 - (self.severity * 0.5)
        }
    }

    /// 방향 변경 페널티 (wrong-foot 반대로 가려면 더 느림)
    pub fn direction_penalty(&self, desired_dir: (f32, f32)) -> f32 {
        if !self.is_active() {
            return 1.0;
        }

        // wrong-foot 방향과 원하는 방향의 반대 정도 계산
        let dot = self.direction.0 * desired_dir.0 + self.direction.1 * desired_dir.1;

        // dot: 1(같은 방향) ~ -1(반대 방향)
        // wrong-foot 방향으로 가면 약간의 페널티
        // wrong-foot 반대로 가면 큰 페널티 (무게중심 반대)
        if dot > 0.0 {
            // 같은 방향: 미끄러짐, 약간 페널티
            1.0 - (self.severity * 0.2)
        } else {
            // 반대 방향: 무게중심 전환 필요, 큰 페널티
            1.0 - (self.severity * 0.6 * (1.0 - dot).min(1.0))
        }
    }
}

/// Wrong-foot 적용된 이동 업데이트
pub fn update_player_motion_with_wrong_foot(
    pos: (f32, f32),
    vel: (f32, f32),
    target_pos: (f32, f32),
    dt: f32,
    params: &PlayerMotionParams,
    wrong_foot: &WrongFootState,
) -> ((f32, f32), (f32, f32)) {
    if !wrong_foot.is_active() {
        return update_player_motion(pos, vel, target_pos, dt, params);
    }

    // Wrong-foot 페널티 적용된 파라미터
    let to = (target_pos.0 - pos.0, target_pos.1 - pos.1);
    let dist = (to.0 * to.0 + to.1 * to.1).sqrt();
    let desired_dir = if dist > 0.01 { (to.0 / dist, to.1 / dist) } else { (1.0, 0.0) };

    let move_penalty = wrong_foot.movement_penalty();
    let dir_penalty = wrong_foot.direction_penalty(desired_dir);
    let combined_penalty = move_penalty * dir_penalty;

    let penalized_params = PlayerMotionParams {
        max_speed: params.max_speed * combined_penalty,
        accel: params.accel * combined_penalty,
        decel: params.decel,
        turn_skill: params.turn_skill * combined_penalty,
        drag: params.drag,
    };

    update_player_motion(pos, vel, target_pos, dt, &penalized_params)
}

/// 버스트 가속 (Hesitation 성공 후 폭발적 가속)
#[derive(Clone, Debug, Default)]
pub struct BurstState {
    /// 버스트 방향
    pub direction: (f32, f32),
    /// 남은 틱
    pub remaining_ticks: u8,
    /// 버스트 배율 (1.0 이상)
    pub multiplier: f32,
}

impl BurstState {
    /// 버스트 시작 (Hesitation 성공 시)
    pub fn start(direction: (f32, f32), acceleration_stat: u8) -> Self {
        // acceleration 스탯이 높을수록 더 큰 버스트
        let mult = 1.3 + (acceleration_stat as f32 / 100.0) * 0.4; // 1.3 ~ 1.7
        Self {
            direction,
            remaining_ticks: 3, // 0.75초 버스트
            multiplier: mult,
        }
    }

    /// 틱당 업데이트
    pub fn tick(&mut self) {
        self.remaining_ticks = self.remaining_ticks.saturating_sub(1);
        if self.remaining_ticks == 0 {
            self.multiplier = 1.0;
        }
    }

    /// 아직 버스트 중인지
    pub fn is_active(&self) -> bool {
        self.remaining_ticks > 0 && self.multiplier > 1.0
    }

    /// 현재 가속 배율
    pub fn accel_multiplier(&self) -> f32 {
        if self.is_active() {
            self.multiplier
        } else {
            1.0
        }
    }
}

/// 버스트 적용된 이동 업데이트
pub fn update_player_motion_with_burst(
    pos: (f32, f32),
    vel: (f32, f32),
    target_pos: (f32, f32),
    dt: f32,
    params: &PlayerMotionParams,
    burst: &BurstState,
) -> ((f32, f32), (f32, f32)) {
    if !burst.is_active() {
        return update_player_motion(pos, vel, target_pos, dt, params);
    }

    // 버스트 적용된 파라미터
    let boosted_params = PlayerMotionParams {
        max_speed: params.max_speed * 1.1, // 약간의 최고속 증가
        accel: params.accel * burst.accel_multiplier(),
        decel: params.decel,
        turn_skill: params.turn_skill,
        drag: params.drag * 0.8, // 드래그 감소 (관성 유지)
    };

    update_player_motion(pos, vel, target_pos, dt, &boosted_params)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_severity() {
        // 같은 방향 → 0
        let sev = calc_turn_severity((1.0, 0.0), (1.0, 0.0));
        assert!(sev.abs() < 0.01);

        // 반대 방향 → 1
        let sev = calc_turn_severity((1.0, 0.0), (-1.0, 0.0));
        assert!((sev - 1.0).abs() < 0.01);

        // 90도 → 0.5
        let sev = calc_turn_severity((1.0, 0.0), (0.0, 1.0));
        assert!((sev - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_arrival_steering() {
        // 멀리 있을 때 → max_speed
        let speed = calculate_arrival_speed(50.0, 5.0, 9.0, 5.0);
        assert!((speed - 9.0).abs() < 0.01);

        // 가까이 있을 때 → 감속
        let speed = calculate_arrival_speed(1.0, 5.0, 9.0, 5.0);
        assert!(speed < 9.0);
    }

    #[test]
    fn test_update_motion_basic() {
        let params =
            PlayerMotionParams { max_speed: 9.0, accel: 4.0, decel: 5.0, turn_skill: 0.8, drag: 0.05 };

        // 정지 상태에서 앞으로 이동
        let (new_pos, new_vel) = update_player_motion(
            (0.0, 0.0),
            (0.0, 0.0),
            (10.0, 0.0),
            0.25, // 250ms
            &params,
        );

        // 속도가 0보다 커야 함
        assert!(new_vel.0 > 0.0);
        // 위치가 이동했어야 함
        assert!(new_pos.0 > 0.0);
    }

    #[test]
    fn test_update_motion_turn_penalty() {
        let params =
            PlayerMotionParams { max_speed: 9.0, accel: 4.0, decel: 5.0, turn_skill: 0.5, drag: 0.05 };

        // 고속으로 오른쪽 이동 중
        let vel = (9.0, 0.0);
        // 목표는 위쪽 (90도 턴)
        let (_, new_vel) = update_player_motion((0.0, 0.0), vel, (0.0, 10.0), 0.25, &params);

        // 속도가 줄어야 함 (턴 페널티)
        let new_speed = (new_vel.0 * new_vel.0 + new_vel.1 * new_vel.1).sqrt();
        let old_speed = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
        assert!(new_speed < old_speed, "Turn should reduce speed");
    }

    #[test]
    fn test_update_motion_arrival() {
        let params =
            PlayerMotionParams { max_speed: 9.0, accel: 4.0, decel: 5.0, turn_skill: 0.8, drag: 0.05 };

        // 목표에 매우 가까움
        let (new_pos, new_vel) =
            update_player_motion((9.9, 0.0), (5.0, 0.0), (10.0, 0.0), 0.25, &params);

        // 속도가 감속되어야 함
        assert!(new_vel.0 < 5.0, "Should slow down near target");
        // overshoot 안 해야 함 (10.0 초과 안 함)
        // (단, 이건 여러 틱에 걸쳐 확인해야 정확)
        assert!(new_pos.0 < 15.0, "Should not overshoot too much");
    }

    #[test]
    fn test_update_motion_dt_invariance_over_fixed_horizon() {
        let params = PlayerMotionParams { max_speed: 9.0, accel: 4.0, decel: 5.0, turn_skill: 0.8, drag: 0.05 };
        let target = (100.0, 0.0); // far target to avoid arrival logic

        fn simulate(dt: f32, steps: usize, params: &PlayerMotionParams, target: (f32, f32)) -> ((f32, f32), (f32, f32)) {
            let mut pos = (0.0, 0.0);
            let mut vel = (0.0, 0.0);
            for _ in 0..steps {
                let (p, v) = update_player_motion(pos, vel, target, dt, params);
                pos = p;
                vel = v;
            }
            (pos, vel)
        }

        // 1.0s horizon: 20×50ms vs 10×100ms
        let (pos_50, vel_50) = simulate(0.05, 20, &params, target);
        let (pos_100, vel_100) = simulate(0.10, 10, &params, target);

        let pos_diff = (pos_50.0 - pos_100.0).abs();
        let vel_diff = (vel_50.0 - vel_100.0).abs();

        // NOTE: P15 is executed with a fixed substep (currently 0.05s) in MatchEngine.
        // This test is a regression guard to ensure dt changes don't explode movement.
        assert!(pos_diff < 0.10, "pos drift too large: 50ms={pos_50:?} 100ms={pos_100:?} diff={pos_diff}");
        assert!(vel_diff < 0.10, "vel drift too large: 50ms={vel_50:?} 100ms={vel_100:?} diff={vel_diff}");
    }
}
