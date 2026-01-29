//! TransitionSystem (Phase 1.4)
//!
//! Models a short "chaos window" after possession changes.
//!
//! Design goals:
//! - Decision-tick only (250ms cadence; matches MarkingManager trigger cadence)
//! - Stable (does not flicker during ball flight / loose-ball frames)
//! - SSOT state: remaining time + which team lost the ball

use crate::engine::timestep::DECISION_DT;
use crate::models::TeamSide;

pub const TRANSITION_WINDOW_MS: u32 = 3000;

// 0.25s -> 250ms; keep this derived and asserted so it never silently drifts.
pub const DECISION_TICK_MS: u32 = (DECISION_DT * 1000.0) as u32;
const _: () = assert!(DECISION_TICK_MS == 250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionState {
    Inactive,
    Active { remaining_ms: u32, team_lost_ball: TeamSide },
}

impl TransitionState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    pub fn remaining_ms(&self) -> Option<u32> {
        match self {
            Self::Inactive => None,
            Self::Active { remaining_ms, .. } => Some(*remaining_ms),
        }
    }

    pub fn team_lost_ball(&self) -> Option<TeamSide> {
        match self {
            Self::Inactive => None,
            Self::Active { team_lost_ball, .. } => Some(*team_lost_ball),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransitionSystem {
    state: TransitionState,
}

impl TransitionSystem {
    pub fn new() -> Self {
        Self { state: TransitionState::Inactive }
    }

    pub fn state(&self) -> TransitionState {
        self.state
    }

    /// Update transition state once per decision tick.
    ///
    /// Rules:
    /// - On possession change: start window at full duration (do not decrement on the same tick).
    /// - Otherwise, if active: decrement by `DECISION_TICK_MS` and end at 0.
    ///
    /// `prev_home_has_ball` is the possession value before the update; this defines who lost the ball.
    pub fn update(&mut self, possession_changed: bool, prev_home_has_ball: bool) {
        if possession_changed {
            let team_lost_ball = if prev_home_has_ball { TeamSide::Home } else { TeamSide::Away };
            self.state =
                TransitionState::Active { remaining_ms: TRANSITION_WINDOW_MS, team_lost_ball };
            return;
        }

        let TransitionState::Active { remaining_ms, team_lost_ball } = self.state else {
            return;
        };

        let next_remaining = remaining_ms.saturating_sub(DECISION_TICK_MS);
        if next_remaining == 0 {
            self.state = TransitionState::Inactive;
        } else {
            self.state = TransitionState::Active { remaining_ms: next_remaining, team_lost_ball };
        }
    }
}

impl Default for TransitionSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_starts_with_full_window_and_counts_down() {
        let mut t = TransitionSystem::new();

        // Start: inactive
        assert_eq!(t.state(), TransitionState::Inactive);

        // Possession change starts transition; does not decrement on the same tick.
        t.update(true, true);
        assert_eq!(
            t.state(),
            TransitionState::Active { remaining_ms: 3000, team_lost_ball: TeamSide::Home }
        );

        // Countdown tick
        t.update(false, true);
        assert_eq!(
            t.state(),
            TransitionState::Active { remaining_ms: 2750, team_lost_ball: TeamSide::Home }
        );

        // Run to completion
        for _ in 0..10 {
            t.update(false, true);
        }

        // 2750 - 10*250 = 250ms remaining
        assert_eq!(
            t.state(),
            TransitionState::Active { remaining_ms: 250, team_lost_ball: TeamSide::Home }
        );

        // Final tick ends
        t.update(false, true);
        assert_eq!(t.state(), TransitionState::Inactive);
    }

    #[test]
    fn transition_team_lost_ball_is_previous_possessor() {
        let mut t = TransitionSystem::new();

        // If home had the ball before the flip, home lost it.
        t.update(true, true);
        assert_eq!(t.state().team_lost_ball(), Some(TeamSide::Home));

        // New flip where away had the ball before: away lost it.
        t.update(true, false);
        assert_eq!(t.state().team_lost_ball(), Some(TeamSide::Away));
    }
}
