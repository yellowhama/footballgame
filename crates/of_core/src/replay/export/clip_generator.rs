//! Clip generation logic for different event types

use super::context::{build_context, calculate_buildup_importance};
use super::types::HighlightClip;
use crate::replay::ReplayEvent;

/// Generate highlight clips for a goal event
///
/// Creates a multi-clip sequence:
/// 1. Buildup clip (5-10s before goal)
/// 2. Goal moment (with slow motion)
/// 3. Replay from side angle
/// 4. Replay from top/drone view
pub fn generate_goal_clip(event: &ReplayEvent, all_events: &[ReplayEvent]) -> Vec<HighlightClip> {
    let event_time = event.base().t as f32;
    let event_id = format!("evt_{:.0}", event_time * 100.0); // Simple ID generation

    // Build context for buildup
    let context = build_context(event, all_events, 10.0);
    let buildup_duration = if context.len() > 5 { 10.0 } else { 5.0 };
    let buildup_importance = calculate_buildup_importance(&context);

    vec![
        // 1. Buildup clip
        HighlightClip {
            id: format!("{}_buildup", event_id),
            clip_type: "goal_buildup".to_string(),
            start: event_time - buildup_duration,
            end: event_time,
            camera: "Cine_Main".to_string(),
            effect: None,
            importance: buildup_importance,
            event_id: event_id.clone(),
            sub_clips: vec![],
        },
        // 2. Goal moment (slow motion)
        HighlightClip {
            id: format!("{}_moment", event_id),
            clip_type: "goal_highlight".to_string(),
            start: event_time,
            end: event_time + 2.0,
            camera: "Cine_Ball".to_string(),
            effect: Some("slowmo".to_string()),
            importance: 1.0,
            event_id: event_id.clone(),
            sub_clips: vec![],
        },
        // 3. Replay from side
        HighlightClip {
            id: format!("{}_replay1", event_id),
            clip_type: "goal_replay".to_string(),
            start: event_time,
            end: event_time + 2.0,
            camera: "Cine_Side".to_string(),
            effect: Some("slowmo".to_string()),
            importance: 0.8,
            event_id: event_id.clone(),
            sub_clips: vec![],
        },
        // 4. Replay from top
        HighlightClip {
            id: format!("{}_replay2", event_id),
            clip_type: "goal_replay".to_string(),
            start: event_time,
            end: event_time + 2.0,
            camera: "Cine_Top".to_string(),
            effect: None,
            importance: 0.7,
            event_id,
            sub_clips: vec![],
        },
    ]
}

/// Generate highlight clip for a foul event
pub fn generate_foul_clip(event: &ReplayEvent, importance: f32) -> Vec<HighlightClip> {
    let event_time = event.base().t as f32;
    let event_id = format!("evt_{:.0}", event_time * 100.0);

    vec![HighlightClip {
        id: format!("{}_foul", event_id),
        clip_type: "foul".to_string(),
        start: (event_time - 1.0).max(0.0),
        end: event_time + 2.0,
        camera: "Cine_Side".to_string(),
        effect: None,
        importance,
        event_id,
        sub_clips: vec![],
    }]
}

/// Generate highlight clip for a corner kick
pub fn generate_corner_clip(event: &ReplayEvent, importance: f32) -> Vec<HighlightClip> {
    let event_time = event.base().t as f32;
    let event_id = format!("evt_{:.0}", event_time * 100.0);

    vec![HighlightClip {
        id: format!("{}_corner", event_id),
        clip_type: "corner".to_string(),
        start: (event_time - 1.0).max(0.0),
        end: event_time + 3.0,
        camera: "Cine_Top".to_string(),
        effect: None,
        importance,
        event_id,
        sub_clips: vec![],
    }]
}

/// Generate generic highlight clip for other events
pub fn generate_generic_clip(event: &ReplayEvent, importance: f32) -> Vec<HighlightClip> {
    let event_time = event.base().t as f32;
    let event_id = format!("evt_{:.0}", event_time * 100.0);

    let clip_type = match event {
        ReplayEvent::Shot { .. } => "shot",
        ReplayEvent::Save { .. } => "save",
        ReplayEvent::Card { .. } => "card",
        ReplayEvent::Penalty { .. } => "penalty",
        _ => "generic",
    };

    vec![HighlightClip {
        id: format!("{}_{}", event_id, clip_type),
        clip_type: clip_type.to_string(),
        start: (event_time - 1.0).max(0.0),
        end: event_time + 3.0,
        camera: "Cine_Main".to_string(),
        effect: None,
        importance,
        event_id,
        sub_clips: vec![],
    }]
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
    fn test_generate_goal_clip_structure() {
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
            ReplayEvent::Goal {
                base: make_event_base(18.0),
                at: make_pos(52.5, 34.0),
                assist_player_id: None,
            },
        ];

        let goal = &events[1];
        let clips = generate_goal_clip(goal, &events);

        // Should generate 4 clips: buildup + moment + 2 replays
        assert_eq!(clips.len(), 4);

        // Check buildup clip
        assert_eq!(clips[0].clip_type, "goal_buildup");
        assert_eq!(clips[0].camera, "Cine_Main");
        assert!(clips[0].effect.is_none());

        // Check goal moment
        assert_eq!(clips[1].clip_type, "goal_highlight");
        assert_eq!(clips[1].camera, "Cine_Ball");
        assert_eq!(clips[1].effect, Some("slowmo".to_string()));
        assert_eq!(clips[1].importance, 1.0);

        // Check replays
        assert_eq!(clips[2].clip_type, "goal_replay");
        assert_eq!(clips[2].camera, "Cine_Side");
        assert_eq!(clips[3].camera, "Cine_Top");
    }

    #[test]
    fn test_generate_goal_clip_timing() {
        let events = vec![ReplayEvent::Goal {
            base: make_event_base(18.0),
            at: make_pos(52.5, 34.0),
            assist_player_id: None,
        }];

        let goal = &events[0];
        let clips = generate_goal_clip(goal, &events);

        // Check timing constraints
        assert!(clips[0].start < 18.0); // Buildup starts before goal
        assert_eq!(clips[0].end, 18.0); // Buildup ends at goal
        assert_eq!(clips[1].start, 18.0); // Moment starts at goal
        assert!(clips[1].end > 18.0); // Moment extends after goal
    }

    #[test]
    fn test_generate_foul_clip() {
        let foul = ReplayEvent::Foul {
            base: make_event_base(25.0),
            at: make_pos(40.0, 34.0),
            foul_type: None,
            severity: None,
            intentional: None,
            location_danger: None,
            aggression_level: None,
        };

        let clips = generate_foul_clip(&foul, 0.6);

        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].clip_type, "foul");
        assert_eq!(clips[0].camera, "Cine_Side");
        assert_eq!(clips[0].importance, 0.6);
        assert!(clips[0].start >= 24.0); // Starts ~1s before
        assert!(clips[0].end > 25.0); // Extends after foul
    }

    #[test]
    fn test_generate_corner_clip() {
        let corner =
            ReplayEvent::CornerKick { base: make_event_base(30.0), spot: make_pos(105.0, 0.0) };

        let clips = generate_corner_clip(&corner, 0.5);

        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].clip_type, "corner");
        assert_eq!(clips[0].camera, "Cine_Top");
        assert_eq!(clips[0].importance, 0.5);
    }

    #[test]
    fn test_generate_generic_clip_shot() {
        let shot = ReplayEvent::Shot {
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
        };

        let clips = generate_generic_clip(&shot, 0.7);

        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].clip_type, "shot");
        assert_eq!(clips[0].camera, "Cine_Main");
    }

    #[test]
    fn test_clip_id_generation() {
        let goal = ReplayEvent::Goal {
            base: make_event_base(18.5),
            at: make_pos(52.5, 34.0),
            assist_player_id: None,
        };

        let clips = generate_goal_clip(&goal, std::slice::from_ref(&goal));

        // IDs should be based on event time
        assert!(clips[0].id.contains("evt_1850"));
        assert!(clips[0].id.contains("buildup"));
        assert!(clips[1].id.contains("moment"));
    }

    #[test]
    fn test_foul_at_start_of_match() {
        let foul = ReplayEvent::Foul {
            base: make_event_base(0.5), // Very early in match
            at: make_pos(52.5, 34.0),
            foul_type: None,
            severity: None,
            intentional: None,
            location_danger: None,
            aggression_level: None,
        };

        let clips = generate_foul_clip(&foul, 0.6);

        // Start time should not go negative
        assert!(clips[0].start >= 0.0);
    }
}
