//! Timeline generation and serialization

use super::clip_generator::{
    generate_corner_clip, generate_foul_clip, generate_generic_clip, generate_goal_clip,
};
use super::importance::get_calculator;
use super::types::{ExportError, HighlightTimeline, TimelineMetadata};
use crate::replay::ReplayEvent;

/// Maximum number of highlights to include in timeline
const MAX_HIGHLIGHTS: usize = 20;

/// Minimum importance score for inclusion
const MIN_IMPORTANCE: f32 = 0.5;

/// Generate highlight timeline from replay events
///
/// This is the main entry point for the broadcast export system.
/// It analyzes all events, scores them by importance, and generates
/// a timeline of highlight clips ready for playback.
pub fn generate_timeline(events: &[ReplayEvent]) -> Result<HighlightTimeline, ExportError> {
    if events.is_empty() {
        return Err(ExportError::NoEvents);
    }

    let calculator = get_calculator("standard");
    let mut all_clips = Vec::new();

    // Score all events
    let mut scored_events: Vec<(&ReplayEvent, f32)> =
        events.iter().map(|e| (e, calculator.calculate(e))).collect();

    // Sort by importance (descending)
    scored_events.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take top N events above threshold
    let important_events: Vec<_> = scored_events
        .iter()
        .take(MAX_HIGHLIGHTS)
        .filter(|(_, importance)| *importance >= MIN_IMPORTANCE)
        .copied()
        .collect();

    // Generate clips for each important event
    for (event, importance) in important_events {
        let clips = match event {
            ReplayEvent::Goal { .. } => generate_goal_clip(event, events),
            ReplayEvent::Foul { .. } => generate_foul_clip(event, importance),
            ReplayEvent::CornerKick { .. } => generate_corner_clip(event, importance),
            _ => generate_generic_clip(event, importance),
        };

        all_clips.extend(clips);
    }

    // Sort clips by time
    all_clips.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));

    // Calculate total duration
    let total_duration = events.last().map(|e| e.base().t as f32).unwrap_or(0.0);

    // Generate timestamp (ISO 8601)
    let now = chrono::Utc::now();
    let generated_at = now.to_rfc3339();

    // Create match ID from timestamp
    let match_id = format!("match_{}", now.timestamp());

    Ok(HighlightTimeline {
        version: "1.0".to_string(),
        match_id,
        clips: all_clips.clone(),
        metadata: TimelineMetadata { total_duration, clip_count: all_clips.len(), generated_at },
    })
}

/// Save timeline to JSON file
pub fn save_timeline_json(timeline: &HighlightTimeline, path: &str) -> Result<(), ExportError> {
    let json = serde_json::to_string_pretty(timeline)
        .map_err(|e| ExportError::Serialization(e.to_string()))?;

    std::fs::write(path, json).map_err(|e| ExportError::FileWrite(e.to_string()))?;

    Ok(())
}

/// Load timeline from JSON file
pub fn load_timeline_json(path: &str) -> Result<HighlightTimeline, ExportError> {
    let json = std::fs::read_to_string(path).map_err(|e| ExportError::FileRead(e.to_string()))?;

    let timeline: HighlightTimeline =
        serde_json::from_str(&json).map_err(|e| ExportError::Deserialization(e.to_string()))?;

    Ok(timeline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::{CardType, EventBase, MeterPos};

    fn make_event_base(t: f64, player_id: u32, team_id: u32) -> EventBase {
        EventBase { t, player_id: Some(player_id), team_id: Some(team_id) }
    }

    fn make_pos(x: f64, y: f64) -> MeterPos {
        MeterPos { x, y }
    }

    #[test]
    fn test_generate_timeline_empty_events() {
        let events: Vec<ReplayEvent> = vec![];
        let result = generate_timeline(&events);

        assert!(result.is_err());
        assert!(matches!(result, Err(ExportError::NoEvents)));
    }

    #[test]
    fn test_generate_timeline_with_goal() {
        let events = vec![
            ReplayEvent::KickOff { base: make_event_base(0.0, 1, 1) },
            ReplayEvent::Pass {
                base: make_event_base(10.0, 1, 1),
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
                base: make_event_base(18.0, 1, 1),
                at: make_pos(52.5, 34.0),
                assist_player_id: None,
            },
        ];

        let timeline = generate_timeline(&events).unwrap();

        // Should have clips (goal generates multiple clips)
        assert!(!timeline.clips.is_empty());

        // Metadata should be populated
        assert_eq!(timeline.metadata.clip_count, timeline.clips.len());
        assert_eq!(timeline.metadata.total_duration, 18.0);
        assert!(!timeline.metadata.generated_at.is_empty());

        // Version and match ID should be set
        assert_eq!(timeline.version, "1.0");
        assert!(timeline.match_id.starts_with("match_"));
    }

    #[test]
    fn test_generate_timeline_filters_by_importance() {
        let events = vec![
            // Low importance events (should be filtered out)
            ReplayEvent::Pass {
                base: make_event_base(5.0, 1, 1),
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
            ReplayEvent::BallMove { base: make_event_base(6.0, 1, 1), to: make_pos(45.0, 34.0) },
            // High importance events (should be included)
            ReplayEvent::Goal {
                base: make_event_base(18.0, 1, 1),
                at: make_pos(52.5, 34.0),
                assist_player_id: None,
            },
            ReplayEvent::Card {
                base: make_event_base(30.0, 2, 2),
                card_type: CardType::Red,
                yellow_count: None,
                from_second_yellow: None,
            },
        ];

        let timeline = generate_timeline(&events).unwrap();

        // Should only include high-importance events
        // Goal generates 4 clips, Red card generates 1 clip
        assert!(timeline.clips.len() >= 2);

        // All clips should have importance >= MIN_IMPORTANCE
        // (except buildup clips which have calculated importance)
        let high_importance_clips = timeline
            .clips
            .iter()
            .filter(|c| c.importance >= MIN_IMPORTANCE || c.clip_type == "goal_buildup")
            .count();

        assert_eq!(high_importance_clips, timeline.clips.len());
    }

    #[test]
    fn test_generate_timeline_sorts_by_time() {
        let events = vec![
            ReplayEvent::Goal {
                base: make_event_base(50.0, 1, 1),
                at: make_pos(52.5, 34.0),
                assist_player_id: None,
            },
            ReplayEvent::Goal {
                base: make_event_base(20.0, 1, 1),
                at: make_pos(52.5, 34.0),
                assist_player_id: None,
            },
            ReplayEvent::Goal {
                base: make_event_base(35.0, 1, 1),
                at: make_pos(52.5, 34.0),
                assist_player_id: None,
            },
        ];

        let timeline = generate_timeline(&events).unwrap();

        // Clips should be sorted by start time
        for i in 1..timeline.clips.len() {
            assert!(timeline.clips[i - 1].start <= timeline.clips[i].start);
        }
    }

    #[test]
    fn test_generate_timeline_respects_max_highlights() {
        // Create more events than MAX_HIGHLIGHTS
        let mut events = vec![];
        for i in 0..30 {
            events.push(ReplayEvent::Shot {
                base: make_event_base(i as f64 * 3.0, 1, 1),
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
            });
        }

        let timeline = generate_timeline(&events).unwrap();

        // Should not exceed MAX_HIGHLIGHTS * clips_per_event
        // Shots generate 1 clip each, so max 20 clips
        assert!(timeline.clips.len() <= MAX_HIGHLIGHTS);
    }

    #[test]
    fn test_save_and_load_timeline_json() {
        let events = vec![ReplayEvent::Goal {
            base: make_event_base(18.0, 1, 1),
            at: make_pos(52.5, 34.0),
            assist_player_id: None,
        }];

        let timeline = generate_timeline(&events).unwrap();

        // Save to temp file
        let temp_path = std::env::temp_dir().join("test_timeline.json");
        let temp_path_str = temp_path.to_str().unwrap();

        save_timeline_json(&timeline, temp_path_str).unwrap();

        // Load back
        let loaded = load_timeline_json(temp_path_str).unwrap();

        // Should match original
        assert_eq!(loaded.version, timeline.version);
        assert_eq!(loaded.match_id, timeline.match_id);
        assert_eq!(loaded.clips.len(), timeline.clips.len());
        assert_eq!(loaded.metadata.clip_count, timeline.metadata.clip_count);

        // Cleanup
        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_load_timeline_json_file_not_found() {
        let result = load_timeline_json("/nonexistent/path.json");

        assert!(result.is_err());
        assert!(matches!(result, Err(ExportError::FileRead(_))));
    }

    #[test]
    fn test_timeline_event_type_distribution() {
        let events = vec![
            ReplayEvent::Goal {
                base: make_event_base(10.0, 1, 1),
                at: make_pos(52.5, 34.0),
                assist_player_id: None,
            },
            ReplayEvent::Foul {
                base: make_event_base(20.0, 2, 2),
                at: make_pos(40.0, 34.0),
                foul_type: None,
                severity: None,
                intentional: None,
                location_danger: None,
                aggression_level: None,
            },
            ReplayEvent::CornerKick {
                base: make_event_base(30.0, 1, 1),
                spot: make_pos(105.0, 0.0),
            },
            ReplayEvent::Shot {
                base: make_event_base(40.0, 1, 1),
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
        ];

        let timeline = generate_timeline(&events).unwrap();

        // Check that different event types generated clips
        let clip_types: Vec<_> = timeline.clips.iter().map(|c| &c.clip_type).collect();

        assert!(clip_types.iter().any(|t| t.contains("goal")));
        assert!(clip_types.iter().any(|t| *t == "foul"));
        assert!(clip_types.iter().any(|t| *t == "corner"));
        assert!(clip_types.iter().any(|t| *t == "shot"));
    }
}
