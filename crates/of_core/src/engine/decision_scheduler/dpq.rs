//! Decision Priority Queue (DPQ) scheduler.
//!
//! v1.1: This is intentionally minimal:
//! - We track per-player due tick (22 players).
//! - The v1.1 cadence schedules everyone every tick (no behavior change).
//!
//! v1.2: Variable cadence based on proximity to ball action:
//! - Active zone (within 20m of ball, ball owner, pass target): every tick
//! - Passive zone (distant players): every 4 ticks
//! - Pull-forward mechanism for sudden context changes

use super::cadence::{self, CadenceLevel};

/// Per-player decision scheduler (22 players, track_id 0..21).
#[derive(Debug, Clone)]
pub struct DecisionScheduler {
    due_tick: [u64; 22],
}

impl Default for DecisionScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionScheduler {
    pub fn new() -> Self {
        Self { due_tick: [0; 22] }
    }

    /// Reset all players to be due "now" (used at match init/reset boundaries).
    pub fn reset(&mut self) {
        self.due_tick = [0; 22];
    }

    #[inline]
    pub fn is_due(&self, player_idx: usize, current_tick: u64) -> bool {
        self.due_tick
            .get(player_idx)
            .is_some_and(|due| current_tick >= *due)
    }

    // =========================================================================
    // v1.1: Every tick (no behavior change)
    // =========================================================================

    /// Record that a decision was executed for this player (v1.1 cadence).
    pub fn mark_executed_v1_1(&mut self, player_idx: usize, current_tick: u64) {
        if let Some(due) = self.due_tick.get_mut(player_idx) {
            *due = cadence::next_due_tick_every_tick(current_tick);
        }
    }

    // =========================================================================
    // v1.2: Variable cadence (Active/Passive zones)
    // =========================================================================

    /// Record that a decision was executed with variable cadence (v1.2).
    ///
    /// Active players decide every tick, passive players every 4 ticks.
    pub fn mark_executed_v1_2(&mut self, player_idx: usize, current_tick: u64, level: CadenceLevel) {
        if let Some(due) = self.due_tick.get_mut(player_idx) {
            *due = cadence::next_due_tick_v1_2(current_tick, level);
        }
    }

    /// Pull forward a player's decision to the current tick (danger mechanism).
    ///
    /// Used when a distant (passive) player suddenly needs to decide immediately,
    /// e.g., when they become a pass target or an opponent enters their zone.
    pub fn pull_forward(&mut self, player_idx: usize, current_tick: u64) {
        if let Some(due) = self.due_tick.get_mut(player_idx) {
            // Only pull forward if they're not already due
            if *due > current_tick {
                *due = current_tick;
            }
        }
    }

    /// Pull forward multiple players at once (batch operation).
    pub fn pull_forward_batch(&mut self, player_indices: &[usize], current_tick: u64) {
        for &idx in player_indices {
            self.pull_forward(idx, current_tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark_executed_v1_2_active() {
        let mut scheduler = DecisionScheduler::new();
        scheduler.mark_executed_v1_2(5, 100, CadenceLevel::Active);
        assert!(!scheduler.is_due(5, 100)); // Not due at tick 100
        assert!(scheduler.is_due(5, 101)); // Due at tick 101
    }

    #[test]
    fn test_mark_executed_v1_2_passive() {
        let mut scheduler = DecisionScheduler::new();
        scheduler.mark_executed_v1_2(5, 100, CadenceLevel::Passive);
        assert!(!scheduler.is_due(5, 100)); // Not due at tick 100
        assert!(!scheduler.is_due(5, 103)); // Not due at tick 103
        assert!(scheduler.is_due(5, 104)); // Due at tick 104
    }

    #[test]
    fn test_pull_forward() {
        let mut scheduler = DecisionScheduler::new();
        // Schedule player 5 for tick 104 (passive)
        scheduler.mark_executed_v1_2(5, 100, CadenceLevel::Passive);
        assert!(!scheduler.is_due(5, 101));

        // Pull forward to tick 101
        scheduler.pull_forward(5, 101);
        assert!(scheduler.is_due(5, 101)); // Now due immediately
    }

    #[test]
    fn test_pull_forward_already_due() {
        let mut scheduler = DecisionScheduler::new();
        // Player is already due (tick 0 by default)
        assert!(scheduler.is_due(5, 100));

        // Pull forward should be no-op
        scheduler.pull_forward(5, 100);
        assert!(scheduler.is_due(5, 100));
    }

    #[test]
    fn test_pull_forward_batch() {
        let mut scheduler = DecisionScheduler::new();
        // Schedule multiple players as passive
        scheduler.mark_executed_v1_2(3, 100, CadenceLevel::Passive);
        scheduler.mark_executed_v1_2(7, 100, CadenceLevel::Passive);
        scheduler.mark_executed_v1_2(11, 100, CadenceLevel::Passive);

        assert!(!scheduler.is_due(3, 101));
        assert!(!scheduler.is_due(7, 101));
        assert!(!scheduler.is_due(11, 101));

        // Pull forward batch
        scheduler.pull_forward_batch(&[3, 7, 11], 101);

        assert!(scheduler.is_due(3, 101));
        assert!(scheduler.is_due(7, 101));
        assert!(scheduler.is_due(11, 101));
    }
}
