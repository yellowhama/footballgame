//! Team Momentum System (FIX_2601/0123)
//!
//! Tracks match-time momentum for each team, influenced by:
//! - Goals scored/conceded
//! - Cards (yellow/red)
//! - Captain's leadership attribute (stabilization)
//!
//! FM Reference: Leadership is NOT used directly in match simulation,
//! but we use it here to stabilize momentum swings (captain effect).

/// Momentum trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MomentumTrend {
    /// Momentum rising (recent positive events)
    Rising,
    /// Momentum falling (recent negative events)
    Falling,
    /// Momentum stable (no recent significant changes)
    #[default]
    Stable,
}

/// Team momentum state during a match
///
/// Value range: 0.0 (demoralized) to 1.0 (peak confidence)
/// Neutral baseline: 0.5
#[derive(Debug, Clone, Copy)]
pub struct TeamMomentum {
    /// Current momentum value (0.0-1.0, neutral: 0.5)
    pub value: f32,
    /// Captain's leadership attribute (1-20, normalized from raw)
    pub captain_leadership: f32,
    /// Current trend direction
    pub trend: MomentumTrend,
    /// Ticks since last event (for trend decay)
    ticks_since_event: u32,
}

impl Default for TeamMomentum {
    fn default() -> Self {
        Self {
            value: 0.5,
            captain_leadership: 10.0, // Average leadership
            trend: MomentumTrend::Stable,
            ticks_since_event: 0,
        }
    }
}

impl TeamMomentum {
    /// Create momentum with specific captain leadership
    pub fn with_captain_leadership(leadership: f32) -> Self {
        Self {
            captain_leadership: leadership.clamp(1.0, 20.0),
            ..Default::default()
        }
    }

    /// Apply momentum change from an event
    ///
    /// # Arguments
    /// * `delta` - Raw momentum change (positive = good, negative = bad)
    ///
    /// Leadership effect:
    /// - High leadership (18+): Negative deltas reduced by 50%
    /// - Low leadership (5-): Negative deltas increased by 20%
    pub fn apply_event(&mut self, delta: f32) {
        let adjusted_delta = if delta < 0.0 {
            // Negative events are modulated by leadership
            let leadership_factor = self.leadership_stabilization_factor();
            delta * leadership_factor
        } else {
            // Positive events are not affected by leadership
            delta
        };

        self.value = (self.value + adjusted_delta).clamp(0.0, 1.0);
        self.trend = if adjusted_delta > 0.01 {
            MomentumTrend::Rising
        } else if adjusted_delta < -0.01 {
            MomentumTrend::Falling
        } else {
            MomentumTrend::Stable
        };
        self.ticks_since_event = 0;
    }

    /// Calculate leadership stabilization factor for negative events
    ///
    /// Returns a multiplier (0.5 to 1.2) applied to negative momentum changes.
    /// - leadership 18+: 0.5 (50% reduction)
    /// - leadership 10: 1.0 (no change)
    /// - leadership 5-: 1.2 (20% increase)
    fn leadership_stabilization_factor(&self) -> f32 {
        if self.captain_leadership >= 18.0 {
            0.5
        } else if self.captain_leadership >= 14.0 {
            0.7
        } else if self.captain_leadership >= 10.0 {
            1.0
        } else if self.captain_leadership >= 6.0 {
            1.1
        } else {
            1.2
        }
    }

    /// Apply halftime recovery (leadership bonus)
    ///
    /// High leadership captains provide momentum recovery during halftime.
    /// - leadership 18+: +0.08 recovery toward neutral
    /// - leadership 14+: +0.05 recovery toward neutral
    /// - leadership 10+: +0.03 recovery toward neutral
    /// - leadership <10: +0.01 recovery toward neutral
    pub fn apply_halftime_recovery(&mut self) {
        let recovery = if self.captain_leadership >= 18.0 {
            0.08
        } else if self.captain_leadership >= 14.0 {
            0.05
        } else if self.captain_leadership >= 10.0 {
            0.03
        } else {
            0.01
        };

        // Move toward neutral (0.5)
        if self.value < 0.5 {
            self.value = (self.value + recovery).min(0.5);
        } else if self.value > 0.5 {
            self.value = (self.value - recovery * 0.5).max(0.5); // Slower decay from high momentum
        }

        self.trend = MomentumTrend::Stable;
    }

    /// Tick the momentum system (called each simulation tick)
    ///
    /// Applies gradual decay toward neutral over time.
    pub fn tick(&mut self) {
        self.ticks_since_event += 1;

        // Gradual decay toward neutral every ~60 ticks (15 seconds)
        if self.ticks_since_event > 60 {
            let decay_rate = 0.001; // Very slow decay
            if self.value > 0.5 {
                self.value = (self.value - decay_rate).max(0.5);
            } else if self.value < 0.5 {
                self.value = (self.value + decay_rate).min(0.5);
            }

            // Reset trend to stable after decay
            if (self.value - 0.5).abs() < 0.01 {
                self.trend = MomentumTrend::Stable;
            }
        }
    }

    /// Get momentum modifier for decision-making
    ///
    /// Returns a value that can be used to adjust player decisions:
    /// - High momentum (>0.7): More aggressive play
    /// - Low momentum (<0.3): More conservative play
    /// - Neutral: No adjustment
    pub fn decision_modifier(&self) -> f32 {
        // Map 0.0-1.0 to -0.2 to +0.2
        (self.value - 0.5) * 0.4
    }

    /// Check if team is in high momentum state (confident)
    pub fn is_high(&self) -> bool {
        self.value >= 0.65
    }

    /// Check if team is in low momentum state (demoralized)
    pub fn is_low(&self) -> bool {
        self.value <= 0.35
    }

    /// Create momentum with specific value (for testing)
    #[doc(hidden)]
    pub fn with_value(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            ..Default::default()
        }
    }
}

/// Momentum event constants (FIX_2601/0123)
pub mod events {
    /// Goal scored by this team
    pub const GOAL_SCORED: f32 = 0.15;
    /// Goal scored while already leading (momentum boost)
    pub const LEAD_EXTENDED: f32 = 0.20;
    /// Goal conceded by this team
    pub const GOAL_CONCEDED: f32 = -0.12;
    /// Red card received
    pub const RED_CARD: f32 = -0.15;
    /// Yellow card received
    pub const YELLOW_CARD: f32 = -0.03;
    /// Penalty awarded to this team
    pub const PENALTY_AWARDED: f32 = 0.08;
    /// Penalty conceded by this team
    pub const PENALTY_CONCEDED: f32 = -0.10;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_momentum() {
        let m = TeamMomentum::default();
        assert!((m.value - 0.5).abs() < 0.001);
        assert_eq!(m.trend, MomentumTrend::Stable);
    }

    #[test]
    fn test_goal_scored_increases_momentum() {
        let mut m = TeamMomentum::default();
        m.apply_event(events::GOAL_SCORED);
        assert!(m.value > 0.5);
        assert_eq!(m.trend, MomentumTrend::Rising);
    }

    #[test]
    fn test_goal_conceded_decreases_momentum() {
        let mut m = TeamMomentum::default();
        m.apply_event(events::GOAL_CONCEDED);
        assert!(m.value < 0.5);
        assert_eq!(m.trend, MomentumTrend::Falling);
    }

    #[test]
    fn test_high_leadership_reduces_negative_impact() {
        let mut high_leader = TeamMomentum::with_captain_leadership(18.0);
        let mut low_leader = TeamMomentum::with_captain_leadership(5.0);

        high_leader.apply_event(events::GOAL_CONCEDED);
        low_leader.apply_event(events::GOAL_CONCEDED);

        // High leadership should have less impact
        assert!(high_leader.value > low_leader.value);
    }

    #[test]
    fn test_halftime_recovery() {
        let mut m = TeamMomentum::with_captain_leadership(18.0);
        m.apply_event(events::GOAL_CONCEDED);
        m.apply_event(events::GOAL_CONCEDED);

        let before = m.value;
        m.apply_halftime_recovery();

        // Should recover toward neutral
        assert!(m.value > before);
        assert!(m.value <= 0.5);
    }

    #[test]
    fn test_momentum_clamped() {
        let mut m = TeamMomentum::default();

        // Apply many positive events
        for _ in 0..10 {
            m.apply_event(events::GOAL_SCORED);
        }
        assert!(m.value <= 1.0);

        // Apply many negative events
        let mut m2 = TeamMomentum::default();
        for _ in 0..10 {
            m2.apply_event(events::RED_CARD);
        }
        assert!(m2.value >= 0.0);
    }

    #[test]
    fn test_decision_modifier_range() {
        let low = TeamMomentum { value: 0.0, ..Default::default() };
        let high = TeamMomentum { value: 1.0, ..Default::default() };

        assert!((low.decision_modifier() - (-0.2)).abs() < 0.001);
        assert!((high.decision_modifier() - 0.2).abs() < 0.001);
    }
}
