//! Phase Duration Constants
//!
//! P7 Spec: Phase별 지속 시간 상수 정의
//!
//! 틱 시스템: 240 ticks/minute = 4 ticks/second = 250ms/tick

// P0: Core types moved to action_queue
use super::super::action_queue::PhaseActionType;

/// 틱당 시간 (초) - SSOT: `engine/timestep.rs` (decision tick)
pub const TICK_DT: f32 = crate::engine::timestep::DECISION_DT;

/// 초당 틱 수
pub const TICKS_PER_SECOND: u64 = crate::engine::ball_physics_params::DECISION_TICKS_PER_SECOND;

// ============================================================================
// Tackle Phase Durations
// ============================================================================

/// 태클 Approach Phase 최대 틱 (거리에 따라 줄어듦)
pub const TACKLE_APPROACH_MAX_TICKS: u8 = 20; // 5초

/// 태클 Approach 속도 (m/s)
pub const TACKLE_APPROACH_SPEED: f32 = 4.0;

/// 태클 Commit 거리 (m) - 이 거리 이내면 Commit Phase로 전환
pub const TACKLE_COMMIT_DISTANCE: f32 = 1.5;

/// 태클 Commit Phase 기본 틱 (TackleType별로 다름)
pub const TACKLE_COMMIT_STANDING_TICKS: u8 = 2; // 0.5초
pub const TACKLE_COMMIT_SLIDING_TICKS: u8 = 4; // 1초
pub const TACKLE_COMMIT_SHOULDER_TICKS: u8 = 1; // 0.25초

/// 태클 Resolve Phase 틱 (즉시)
pub const TACKLE_RESOLVE_TICKS: u8 = 1;

/// 태클 Recovery Phase 틱 (TackleOutcome별로 다름)
pub const TACKLE_RECOVERY_CLEAN_TICKS: u8 = 4; // 1초
pub const TACKLE_RECOVERY_DEFLECT_TICKS: u8 = 8; // 2초
pub const TACKLE_RECOVERY_MISS_TICKS: u8 = 12; // 3초
pub const TACKLE_RECOVERY_FOUL_TICKS: u8 = 16; // 4초

/// 태클 Cooldown Phase 틱
/// P7 수치 조정: 16 → 24 (6초로 증가, 과다 태클 방지)
pub const TACKLE_COOLDOWN_TICKS: u8 = 24; // 6초 (같은 선수가 연속 태클 불가)

/// 태클 시도 가능 최대 거리 (m)
pub const TACKLE_MAX_DISTANCE: f32 = 5.0;

/// 태클 적중 거리 (m)
pub const TACKLE_HIT_DISTANCE: f32 = 1.0;

// ============================================================================
// Dribble Phase Durations
// ============================================================================

/// 드리블 Gather Phase 틱 (첫 터치 준비)
pub const DRIBBLE_GATHER_TICKS: u8 = 2;

/// 드리블 Touch Phase 틱
pub const DRIBBLE_TOUCH_TICKS: u8 = 1;

/// 드리블 Carry Phase 틱
pub const DRIBBLE_CARRY_TICKS: u8 = 7;

/// 드리블 Touch 간격 (Touch + Carry = 8틱 = 2초)
pub const DRIBBLE_TOUCH_INTERVAL: u8 = 8;

/// 드리블 터치 시 공 이동 거리 (m)
pub const DRIBBLE_TOUCH_DISTANCE: f32 = 1.5;

/// 드리블 최대 공-선수 분리 거리 (m)
pub const DRIBBLE_MAX_SEPARATION: f32 = 2.5;

/// 드리블 최소 공-선수 분리 거리 (m)
pub const DRIBBLE_MIN_SEPARATION: f32 = 0.3;

/// 드리블 공 컨트롤 범위 (m) - 이 이상 떨어지면 공 놓침
pub const DRIBBLE_CONTROL_RANGE: f32 = 3.0;

/// 드리블 Evade Phase 틱
pub const DRIBBLE_EVADE_TICKS: u8 = 4;

/// 드리블 Evade 속도 (m/s)
pub const DRIBBLE_EVADE_SPEED: f32 = 3.0;

/// KnockOn (치달) 스프린트 틱
pub const KNOCKON_SPRINT_TICKS: u8 = 8; // 2초 스프린트

// ============================================================================
// Pass Phase Durations
// ============================================================================

/// 패스 Windup Phase 기본 틱
pub const PASS_WINDUP_GROUND_TICKS: u8 = 1;
pub const PASS_WINDUP_LOFTED_TICKS: u8 = 2;
pub const PASS_WINDUP_THROUGH_TICKS: u8 = 2;
pub const PASS_WINDUP_CROSS_TICKS: u8 = 3;
pub const PASS_WINDUP_BACK_TICKS: u8 = 1;

/// 패스 Kick Phase 틱
pub const PASS_KICK_TICKS: u8 = 1;

/// 패스 Recovery Phase 틱
pub const PASS_RECOVERY_TICKS: u8 = 2;

/// 패스 Cooldown Phase 틱
pub const PASS_COOLDOWN_TICKS: u8 = 4;

// ============================================================================
// Shot Phase Durations
// ============================================================================

/// 슛 Windup Phase 기본 틱
pub const SHOT_WINDUP_NORMAL_TICKS: u8 = 3;
pub const SHOT_WINDUP_FINESSE_TICKS: u8 = 3;
pub const SHOT_WINDUP_POWER_TICKS: u8 = 4;
pub const SHOT_WINDUP_CHIP_TICKS: u8 = 2;
pub const SHOT_WINDUP_HEADER_TICKS: u8 = 1;
pub const SHOT_WINDUP_VOLLEY_TICKS: u8 = 2;
pub const SHOT_WINDUP_ONETOUCH_TICKS: u8 = 1;

/// 슛 Strike Phase 틱
pub const SHOT_STRIKE_TICKS: u8 = 1;

/// 슛 Recovery Phase 틱
pub const SHOT_RECOVERY_TICKS: u8 = 4;

/// 슛 Cooldown Phase 틱
pub const SHOT_COOLDOWN_TICKS: u8 = 8;

// ============================================================================
// Ball Physics Constants
// ============================================================================
// NOTE: Legacy compatibility constants.
// SSOT for the contract-path parameters lives in `engine/ball_physics_params.rs`.

/// 잔디 마찰 계수 (속도 유지율/틱)
pub const GRASS_FRICTION: f32 = crate::engine::ball_physics_params::LEGACY_GRASS_FRICTION_PER_DECISION_TICK;

/// 최소 속도 (m/s) - 이하면 정지
pub const BALL_MIN_VELOCITY: f32 = crate::engine::ball_physics_params::DEFAULT.stop_speed_mps;

/// 중력 가속도 (m/s²)
pub const GRAVITY: f32 = crate::engine::ball_physics_params::GRAVITY_MPS2;

/// 바운스 계수 (속도 유지율)
pub const BOUNCE_COEFFICIENT: f32 = crate::engine::ball_physics_params::DEFAULT.woodwork_restitution;

/// 최대 바운스 횟수
pub const MAX_BOUNCES: u8 = crate::engine::ball_physics_params::DEFAULT.max_bounces;

// ============================================================================
// Body Blocking Constants
// ============================================================================

/// 선수 몸통 반경 (m)
pub const BODY_RADIUS: f32 = 0.4;

/// 압박 영향 반경 (m)
pub const INFLUENCE_RADIUS: f32 = 1.5;

/// 인터셉트 반경 (m)
pub const INTERCEPT_RADIUS: f32 = 1.2;

/// 인터셉트 시간 여유 비율
pub const INTERCEPT_TIME_MARGIN: f32 = 0.8;

// ============================================================================
// Defensive Positioning Constants
// ============================================================================

/// 압박 목표 거리 (m)
pub const PRESS_TARGET_DISTANCE: f32 = 1.8;

/// 압박 최소 유지 거리 (m)
pub const PRESS_MIN_DISTANCE: f32 = 1.2;

/// 압박 최대 유지 거리 (m)
pub const PRESS_MAX_DISTANCE: f32 = 2.5;

/// 압박 접근 속도 (m/s)
pub const PRESS_APPROACH_SPEED: f32 = 4.5;

/// 압박 추적 속도 (m/s)
pub const PRESS_FOLLOW_SPEED: f32 = 3.5;

/// 압박 조정 속도 (m/s)
pub const PRESS_ADJUST_SPEED: f32 = 2.0;

/// 압박 유지 최대 틱 (이후 교대)
pub const PRESS_MAX_TICKS: u16 = 80;

/// 압박 후 회복 틱
pub const PRESS_RECOVERY_TICKS: u8 = 16;

/// 마킹 범위 (m)
pub const MARKING_RANGE: f32 = 15.0;

/// 마킹 오프셋 (m)
pub const MARKING_OFFSET: f32 = 1.5;

/// 팀 슬라이드 최대 거리 (m)
pub const SIDE_SHIFT_MAX: f32 = 12.0;

// ============================================================================
// Helper Functions
// ============================================================================

/// PhaseActionType과 Phase에 따른 기본 지속 시간 반환
pub fn get_phase_duration(
    action_type: PhaseActionType,
    phase: super::super::action_queue::ActionPhase,
) -> u64 {
    use super::super::action_queue::ActionPhase;

    match action_type {
        PhaseActionType::Tackle => match phase {
            ActionPhase::Approach => TACKLE_APPROACH_MAX_TICKS as u64,
            ActionPhase::Commit => TACKLE_COMMIT_STANDING_TICKS as u64,
            ActionPhase::Resolve => TACKLE_RESOLVE_TICKS as u64,
            ActionPhase::Recover => TACKLE_RECOVERY_MISS_TICKS as u64,
            ActionPhase::Cooldown => TACKLE_COOLDOWN_TICKS as u64,
            _ => 0,
        },

        PhaseActionType::Pass => match phase {
            ActionPhase::Approach => 0,
            ActionPhase::Commit => PASS_KICK_TICKS as u64,
            ActionPhase::Resolve => 0,
            ActionPhase::Recover => PASS_RECOVERY_TICKS as u64,
            ActionPhase::Cooldown => PASS_COOLDOWN_TICKS as u64,
            _ => 0,
        },

        PhaseActionType::Shot => match phase {
            ActionPhase::Approach => 0,
            ActionPhase::Commit => SHOT_STRIKE_TICKS as u64,
            ActionPhase::Resolve => 0,
            ActionPhase::Recover => SHOT_RECOVERY_TICKS as u64,
            ActionPhase::Cooldown => SHOT_COOLDOWN_TICKS as u64,
            _ => 0,
        },

        PhaseActionType::Dribble => match phase {
            ActionPhase::Approach => DRIBBLE_GATHER_TICKS as u64,
            ActionPhase::Commit => DRIBBLE_TOUCH_TICKS as u64,
            ActionPhase::Resolve => 0,
            ActionPhase::Recover => 0,
            ActionPhase::Cooldown => 0,
            _ => 0,
        },

        PhaseActionType::Move => match phase {
            ActionPhase::Approach => 0, // 거리에 따라 동적
            _ => 0,
        },

        _ => 0,
    }
}

/// 거리(m)에 따른 Approach 틱 수 계산
pub fn calculate_approach_ticks(distance: f32, speed: f32) -> u8 {
    let time_seconds = distance / speed;
    let ticks = (time_seconds / TICK_DT).ceil() as u8;
    ticks.clamp(1, TACKLE_APPROACH_MAX_TICKS)
}

/// 틱을 초로 변환
pub fn ticks_to_seconds(ticks: u64) -> f32 {
    ticks as f32 * TICK_DT
}

/// 초를 틱으로 변환
pub fn seconds_to_ticks(seconds: f32) -> u64 {
    (seconds / TICK_DT).ceil() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_conversion() {
        assert_eq!(ticks_to_seconds(4), 1.0);
        assert_eq!(seconds_to_ticks(1.0), 4);
        assert_eq!(seconds_to_ticks(2.5), 10);
    }

    #[test]
    fn test_approach_ticks() {
        // 4m 거리, 4m/s 속도 = 1초 = 4틱
        assert_eq!(calculate_approach_ticks(4.0, 4.0), 4);

        // 1m 거리 = 최소 1틱
        assert_eq!(calculate_approach_ticks(0.5, 4.0), 1);

        // 100m 거리 = 최대 제한
        assert_eq!(calculate_approach_ticks(100.0, 4.0), TACKLE_APPROACH_MAX_TICKS);
    }

    #[test]
    fn test_dribble_cycle() {
        // Touch + Carry = 8틱 = 2초
        assert_eq!(DRIBBLE_TOUCH_TICKS + DRIBBLE_CARRY_TICKS, DRIBBLE_TOUCH_INTERVAL);
    }

    #[test]
    fn test_tackle_cooldown() {
        // 6초 쿨다운 (P7 수치 조정)
        assert_eq!(ticks_to_seconds(TACKLE_COOLDOWN_TICKS as u64), 6.0);
    }
}
