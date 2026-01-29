//! Career Player Mode - Controlled Player State
//!
//! This module provides the state management for Career Player Mode,
//! where a single player is controlled by the user during on-ball moments.

/// Controlled player mode state
#[derive(Debug, Clone, Default)]
pub struct ControlledPlayerMode {
    /// 모드 활성화 여부
    pub enabled: bool,

    /// 유저가 컨트롤하는 선수 track_id (0..21)
    pub controlled_track_id: usize,

    /// 마지막 소비한 명령 seq (중복 방지)
    pub last_consumed_seq: u32,

    /// 입력 락 종료 틱
    pub lock_until_tick: u64,
}

impl ControlledPlayerMode {
    /// Create a new controlled player mode instance
    pub fn new(controlled_track_id: usize) -> Self {
        Self { enabled: true, controlled_track_id, last_consumed_seq: 0, lock_until_tick: 0 }
    }

    /// 해당 선수가 컨트롤 대상인지 확인
    pub fn is_controlled(&self, track_id: usize) -> bool {
        self.enabled && track_id == self.controlled_track_id
    }

    /// 현재 입력이 락되어 있는지 확인
    pub fn is_locked(&self, current_tick: u64) -> bool {
        current_tick < self.lock_until_tick
    }

    /// 입력 락 설정
    pub fn lock(&mut self, current_tick: u64, duration_ticks: u64) {
        self.lock_until_tick = current_tick + duration_ticks;
    }

    /// 남은 락 시간 (ticks)
    pub fn remaining_lock_ticks(&self, current_tick: u64) -> u64 {
        self.lock_until_tick.saturating_sub(current_tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controlled_mode_new() {
        let mode = ControlledPlayerMode::new(9);
        assert_eq!(mode.controlled_track_id, 9);
        assert!(mode.enabled);
        assert_eq!(mode.last_consumed_seq, 0);
        assert_eq!(mode.lock_until_tick, 0);
    }

    #[test]
    fn test_is_controlled() {
        let mode = ControlledPlayerMode::new(9);
        assert!(mode.is_controlled(9));
        assert!(!mode.is_controlled(10));
    }

    #[test]
    fn test_is_controlled_when_disabled() {
        let mut mode = ControlledPlayerMode::new(9);
        mode.enabled = false;
        assert!(!mode.is_controlled(9));
    }

    #[test]
    fn test_lock() {
        let mut mode = ControlledPlayerMode::new(9);
        assert!(!mode.is_locked(100));

        mode.lock(100, 3);
        assert!(mode.is_locked(100));
        assert!(mode.is_locked(102));
        assert!(!mode.is_locked(103));
    }

    #[test]
    fn test_remaining_lock_ticks() {
        let mut mode = ControlledPlayerMode::new(9);
        mode.lock(100, 5);

        assert_eq!(mode.remaining_lock_ticks(100), 5);
        assert_eq!(mode.remaining_lock_ticks(102), 3);
        assert_eq!(mode.remaining_lock_ticks(105), 0);
        assert_eq!(mode.remaining_lock_ticks(110), 0);
    }

    #[test]
    fn test_default() {
        let mode = ControlledPlayerMode::default();
        assert!(!mode.enabled);
        assert_eq!(mode.controlled_track_id, 0);
        assert_eq!(mode.last_consumed_seq, 0);
        assert_eq!(mode.lock_until_tick, 0);
    }
}
