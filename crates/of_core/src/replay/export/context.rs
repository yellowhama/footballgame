//! Context building for highlight clips
//!
//! Builds context around key events by finding relevant events that happened
//! before the key moment (e.g., the buildup to a goal).

use crate::replay::ReplayEvent;

/// Build context events that happened before a key event
///
/// Returns events within the lookback window that are relevant to the narrative
/// (e.g., passes, dribbles, shots leading up to a goal)
pub fn build_context(
    event: &ReplayEvent,
    all_events: &[ReplayEvent],
    lookback_seconds: f32,
) -> Vec<ReplayEvent> {
    let event_time = event.base().t as f32;
    let start_time = event_time - lookback_seconds;

    all_events
        .iter()
        .filter(|e| {
            let e_time = e.base().t as f32;
            e_time >= start_time && e_time < event_time
        })
        .filter(|e| is_relevant_context_event(e))
        .cloned()
        .collect()
}

/// Determine if an event is relevant for context building
fn is_relevant_context_event(event: &ReplayEvent) -> bool {
    matches!(
        event,
        ReplayEvent::Pass { .. }
            | ReplayEvent::Shot { .. }
            | ReplayEvent::Throw { .. }
            | ReplayEvent::FreeKick { .. }
            | ReplayEvent::CornerKick { .. }
            | ReplayEvent::Save { .. }
            | ReplayEvent::Foul { .. }
    )
}

/// Calculate importance score for buildup context
///
/// More actions in the buildup = more exciting lead-up = higher score
pub fn calculate_buildup_importance(context: &[ReplayEvent]) -> f32 {
    if context.is_empty() {
        return 0.3; // Default buildup importance
    }

    // Scale importance based on number of actions
    // More action = more exciting buildup
    let action_count = context.len() as f32;
    (0.3 + (action_count * 0.05)).min(0.7)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::{EventBase, MeterPos};

    fn make_event_base(t: f64) -> EventBase {
        EventBase { t, player_id: Some(1), team_id: Some(1) }
    }

    fn make_pos(x: f64, y: f64) -> MeterPos {
        MeterPos { x, y }
    }

    #[test]
    fn test_build_context_empty() {
        let goal = ReplayEvent::Goal {
            base: make_event_base(18.0),
            at: make_pos(52.5, 34.0),
            assist_player_id: None,
        };

        let events = vec![goal.clone()];
        let context = build_context(&goal, &events, 10.0);

        assert_eq!(context.len(), 0); // No prior events
    }

    #[test]
    fn test_build_context_with_passes() {
        let events = vec![
            ReplayEvent::Pass {
                base: make_event_base(10.0),
                from: make_pos(30.0, 34.0),
                to: make_pos(40.0, 34.0),
                receiver_id: None,
                distance_m: None,
                force: None,
                is_clearance: false,
                ground: None,
                outcome: None,
                passing_skill: None,
                vision: None,
                technique: None,
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
            },
            ReplayEvent::Pass {
                base: make_event_base(12.0),
                from: make_pos(40.0, 34.0),
                to: make_pos(50.0, 34.0),
                receiver_id: None,
                distance_m: None,
                force: None,
                is_clearance: false,
                ground: None,
                outcome: None,
                passing_skill: None,
                vision: None,
                technique: None,
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
            },
            ReplayEvent::Shot {
                base: make_event_base(15.0),
                from: make_pos(50.0, 34.0),
                target: make_pos(52.5, 34.0),
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
            },
            ReplayEvent::Goal {
                base: make_event_base(18.0),
                at: make_pos(52.5, 34.0),
                assist_player_id: None,
            },
        ];

        let goal = &events[3];
        let context = build_context(goal, &events, 10.0);

        // Should include 2 passes + 1 shot (not the goal itself)
        assert_eq!(context.len(), 3);
    }

    #[test]
    fn test_build_context_excludes_irrelevant_events() {
        let events = vec![
            ReplayEvent::KickOff { base: make_event_base(0.0) },
            ReplayEvent::BallMove { base: make_event_base(5.0), to: make_pos(30.0, 34.0) },
            ReplayEvent::Pass {
                base: make_event_base(10.0),
                from: make_pos(30.0, 34.0),
                to: make_pos(40.0, 34.0),
                receiver_id: None,
                distance_m: None,
                force: None,
                is_clearance: false,
                ground: None,
                outcome: None,
                passing_skill: None,
                vision: None,
                technique: None,
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
            },
            ReplayEvent::Goal {
                base: make_event_base(18.0),
                at: make_pos(52.5, 34.0),
                assist_player_id: None,
            },
        ];

        let goal = &events[3];
        let context = build_context(goal, &events, 20.0);

        // Should only include Pass (not KickOff or BallMove)
        assert_eq!(context.len(), 1);
        assert!(matches!(context[0], ReplayEvent::Pass { .. }));
    }

    #[test]
    fn test_build_context_respects_lookback_window() {
        let events = vec![
            ReplayEvent::Pass {
                base: make_event_base(5.0), // Outside 10s window
                from: make_pos(30.0, 34.0),
                to: make_pos(40.0, 34.0),
                receiver_id: None,
                distance_m: None,
                force: None,
                is_clearance: false,
                ground: None,
                outcome: None,
                passing_skill: None,
                vision: None,
                technique: None,
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
            },
            ReplayEvent::Pass {
                base: make_event_base(12.0), // Inside 10s window
                from: make_pos(40.0, 34.0),
                to: make_pos(50.0, 34.0),
                receiver_id: None,
                distance_m: None,
                force: None,
                is_clearance: false,
                ground: None,
                outcome: None,
                passing_skill: None,
                vision: None,
                technique: None,
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
            },
            ReplayEvent::Goal {
                base: make_event_base(18.0),
                at: make_pos(52.5, 34.0),
                assist_player_id: None,
            },
        ];

        let goal = &events[2];
        let context = build_context(goal, &events, 10.0);

        // Should only include the pass at t=12 (within 10s window)
        assert_eq!(context.len(), 1);
        assert_eq!(context[0].base().t, 12.0);
    }

    #[test]
    fn test_calculate_buildup_importance_empty() {
        let context: Vec<ReplayEvent> = vec![];
        let importance = calculate_buildup_importance(&context);

        assert_eq!(importance, 0.3); // Default
    }

    #[test]
    fn test_calculate_buildup_importance_few_actions() {
        let context = vec![
            ReplayEvent::Pass {
                base: make_event_base(10.0),
                from: make_pos(30.0, 34.0),
                to: make_pos(40.0, 34.0),
                receiver_id: None,
                distance_m: None,
                force: None,
                is_clearance: false,
                ground: None,
                outcome: None,
                passing_skill: None,
                vision: None,
                technique: None,
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
            },
            ReplayEvent::Pass {
                base: make_event_base(12.0),
                from: make_pos(40.0, 34.0),
                to: make_pos(50.0, 34.0),
                receiver_id: None,
                distance_m: None,
                force: None,
                is_clearance: false,
                ground: None,
                outcome: None,
                passing_skill: None,
                vision: None,
                technique: None,
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
            },
        ];

        let importance = calculate_buildup_importance(&context);

        // 0.3 + (2 * 0.05) = 0.4
        assert_eq!(importance, 0.4);
    }

    #[test]
    fn test_calculate_buildup_importance_many_actions() {
        let mut context = vec![];
        for i in 0..10 {
            context.push(ReplayEvent::Pass {
                base: make_event_base(i as f64),
                from: make_pos(30.0, 34.0),
                to: make_pos(40.0, 34.0),
                receiver_id: None,
                distance_m: None,
                force: None,
                is_clearance: false,
                ground: None,
                outcome: None,
                passing_skill: None,
                vision: None,
                technique: None,
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
            });
        }

        let importance = calculate_buildup_importance(&context);

        // 0.3 + (10 * 0.05) = 0.8, but capped at 0.7
        assert_eq!(importance, 0.7);
    }
}
