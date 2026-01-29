//! Event importance calculation for highlight selection

use crate::replay::ReplayEvent;

/// Trait for calculating event importance scores
pub trait ImportanceCalculator {
    /// Calculate importance score (0.0 - 1.0) for an event
    /// Higher scores indicate more important events that should be highlighted
    fn calculate(&self, event: &ReplayEvent) -> f32;
}

/// Standard importance calculator based on event type
pub struct StandardImportanceCalculator;

impl ImportanceCalculator for StandardImportanceCalculator {
    fn calculate(&self, event: &ReplayEvent) -> f32 {
        match event {
            // Critical events
            ReplayEvent::Goal { .. } => 1.0,
            ReplayEvent::Penalty { scored: true, .. } => 0.95,
            ReplayEvent::Card { card_type, .. } => match card_type {
                crate::replay::CardType::Red => 0.9,
                crate::replay::CardType::Yellow => 0.7,
            },

            // High importance events
            ReplayEvent::Shot { on_target: true, .. } => 0.7,
            ReplayEvent::Penalty { scored: false, .. } => 0.75, // Missed penalties still interesting
            ReplayEvent::Save { .. } => 0.65,
            ReplayEvent::Foul { .. } => 0.6,

            // Medium importance events
            ReplayEvent::CornerKick { .. } => 0.5,
            ReplayEvent::Shot { on_target: false, .. } => 0.4,
            ReplayEvent::FreeKick { .. } => 0.45,
            ReplayEvent::Offside { .. } => 0.3,

            // Low importance events (still tracked but rarely highlighted)
            ReplayEvent::Throw { .. } => 0.15,
            ReplayEvent::Pass { .. } => 0.1,
            ReplayEvent::BallMove { .. } => 0.05,
            ReplayEvent::Run { .. } => 0.1,
            ReplayEvent::Dribble { .. } => 0.25,
            ReplayEvent::ThroughBall { .. } => 0.35,

            // Communication and tactical events
            ReplayEvent::Communication { .. } => 0.15,
            ReplayEvent::Header { .. } => 0.4,
            ReplayEvent::Boundary { .. } => 0.2,

            // Structural events (used for context, not highlighted individually)
            ReplayEvent::KickOff { .. } => 0.0,
            ReplayEvent::HalfTime { .. } => 0.0,
            ReplayEvent::FullTime { .. } => 0.0,
            ReplayEvent::Substitution { .. } => 0.2,

            // 0108: Possession events (low importance - for analytics)
            ReplayEvent::Possession { .. } => 0.1,

            // 0108: Decision events (very low importance - for debugging)
            ReplayEvent::Decision { .. } => 0.05,
        }
    }
}

/// Get importance calculator by name
/// Currently only "standard" is supported, but this allows future extensibility
pub fn get_calculator(name: &str) -> Box<dyn ImportanceCalculator> {
    match name {
        "standard" => Box::new(StandardImportanceCalculator),
        // Future: "ai" => Box::new(AIImportanceCalculator),
        _ => Box::new(StandardImportanceCalculator),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::{CardType, EventBase};

    fn make_event_base(t: f64) -> EventBase {
        EventBase { t, player_id: Some(1), team_id: Some(1) }
    }

    #[test]
    fn test_goal_importance() {
        let calc = StandardImportanceCalculator;
        let goal = ReplayEvent::Goal {
            base: make_event_base(18.0),
            at: crate::replay::MeterPos { x: 52.5, y: 34.0 },
            assist_player_id: None,
        };

        assert_eq!(calc.calculate(&goal), 1.0);
    }

    #[test]
    fn test_red_card_importance() {
        let calc = StandardImportanceCalculator;
        let red_card = ReplayEvent::Card {
            base: make_event_base(45.0),
            card_type: CardType::Red,
            yellow_count: None,
            from_second_yellow: None,
        };

        assert_eq!(calc.calculate(&red_card), 0.9);
    }

    #[test]
    fn test_yellow_card_importance() {
        let calc = StandardImportanceCalculator;
        let yellow_card = ReplayEvent::Card {
            base: make_event_base(30.0),
            card_type: CardType::Yellow,
            yellow_count: None,
            from_second_yellow: None,
        };

        assert_eq!(calc.calculate(&yellow_card), 0.7);
    }

    #[test]
    fn test_shot_on_target() {
        let calc = StandardImportanceCalculator;
        let shot = ReplayEvent::Shot {
            base: make_event_base(25.0),
            from: crate::replay::MeterPos { x: 40.0, y: 34.0 },
            target: crate::replay::MeterPos { x: 52.5, y: 34.0 },
            on_target: true,
            xg: None,
            shot_speed: None,
            long_shots_skill: None,
            finishing_skill: None,
            technique: None,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            curve_factor: None,
        };

        assert_eq!(calc.calculate(&shot), 0.7);
    }

    #[test]
    fn test_shot_off_target() {
        let calc = StandardImportanceCalculator;
        let shot = ReplayEvent::Shot {
            base: make_event_base(25.0),
            from: crate::replay::MeterPos { x: 40.0, y: 34.0 },
            target: crate::replay::MeterPos { x: 52.5, y: 34.0 },
            on_target: false,
            xg: None,
            shot_speed: None,
            long_shots_skill: None,
            finishing_skill: None,
            technique: None,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            curve_factor: None,
        };

        assert_eq!(calc.calculate(&shot), 0.4);
    }

    #[test]
    fn test_structural_events_zero_importance() {
        let calc = StandardImportanceCalculator;

        let kickoff = ReplayEvent::KickOff { base: make_event_base(0.0) };
        assert_eq!(calc.calculate(&kickoff), 0.0);

        let halftime = ReplayEvent::HalfTime { base: make_event_base(45.0) };
        assert_eq!(calc.calculate(&halftime), 0.0);

        let fulltime = ReplayEvent::FullTime { base: make_event_base(90.0) };
        assert_eq!(calc.calculate(&fulltime), 0.0);
    }

    #[test]
    fn test_get_calculator() {
        let calc = get_calculator("standard");
        let goal = ReplayEvent::Goal {
            base: make_event_base(18.0),
            at: crate::replay::MeterPos { x: 52.5, y: 34.0 },
            assist_player_id: None,
        };

        assert_eq!(calc.calculate(&goal), 1.0);
    }

    #[test]
    fn test_get_calculator_fallback() {
        // Unknown calculator names should fallback to standard
        let calc = get_calculator("unknown");
        let goal = ReplayEvent::Goal {
            base: make_event_base(18.0),
            at: crate::replay::MeterPos { x: 52.5, y: 34.0 },
            assist_player_id: None,
        };

        assert_eq!(calc.calculate(&goal), 1.0);
    }
}
