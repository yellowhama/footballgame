use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::events::{CardType, Event, PassOutcome, SetPieceKind, ShotEvent, ShotOutcome};

/// 4-level highlight system for different viewing preferences
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HighlightLevel {
    /// Skip - No events, just match result
    Skip,
    /// Simple - Only goals and major incidents (red cards, penalties)
    Simple,
    /// MyPlayer - All events involving specified player(s) + Simple highlights
    MyPlayer,
    /// Full - All events (complete replay)
    Full,
}

/// Configuration for MyPlayer highlight level
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct MyPlayerConfig {
    /// Player IDs to track
    pub player_ids: Vec<String>,
    /// Include events where these players are involved indirectly (e.g., passes to/from them)
    pub include_indirect: bool,
    /// Include goals and major incidents even if players not involved
    pub include_major_incidents: bool,
}

/// Result of highlight filtering
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct HighlightResult {
    pub level: HighlightLevel,
    pub events: Vec<Event>,
    pub summary: HighlightSummary,
}

/// Summary statistics for highlight filtering
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct HighlightSummary {
    pub total_events: usize,
    pub filtered_events: usize,
    pub goals: usize,
    pub major_incidents: usize,
    pub player_events: Option<usize>, // Only for MyPlayer level
}

/// Event importance classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventImportance {
    /// Skip level - Empty, no events
    Skip = 0,
    /// Major incidents only
    Major = 1,
    /// Important events (shots, key passes, fouls)
    Important = 2,
    /// All ball events
    All = 3,
}

/// Trait for filtering events based on highlight levels
pub trait HighlightFilter {
    fn filter_events(&self, events: Vec<Event>) -> HighlightResult;
}

impl HighlightLevel {
    /// Get the importance threshold for this highlight level
    pub fn importance_threshold(&self) -> EventImportance {
        match self {
            HighlightLevel::Skip => EventImportance::Skip,
            HighlightLevel::Simple => EventImportance::Major,
            HighlightLevel::MyPlayer => EventImportance::Important,
            HighlightLevel::Full => EventImportance::All,
        }
    }
}

impl Event {
    /// Classify the importance of this event
    pub fn importance(&self) -> EventImportance {
        match self {
            // Major incidents - always included in Simple+
            Event::Shot(shot) => match shot.outcome {
                ShotOutcome::Goal => EventImportance::Major,
                ShotOutcome::Saved | ShotOutcome::Post => EventImportance::Important,
                ShotOutcome::Off => EventImportance::All,
            },
            Event::Foul(foul) => match foul.card {
                CardType::Red => EventImportance::Major,
                CardType::Yellow => EventImportance::Important,
                CardType::None => EventImportance::All,
            },
            Event::SetPiece(set_piece) => {
                use super::events::SetPieceKind;
                match set_piece.kind {
                    SetPieceKind::Penalty => EventImportance::Major,
                    SetPieceKind::FreeKick | SetPieceKind::Corner => EventImportance::Important,
                    SetPieceKind::ThrowIn | SetPieceKind::GoalKick => EventImportance::All,
                    SetPieceKind::KickOff => EventImportance::Important, // Match start/restart
                }
            }
            Event::Substitution(_) => EventImportance::Important,

            // Important events - included in MyPlayer+
            Event::Save(_) => EventImportance::Important,
            Event::Pass(pass) => {
                match pass.outcome {
                    PassOutcome::Complete => {
                        // Key passes (long distance or in dangerous areas) are more important
                        let distance = pass.base.pos.distance_to(&pass.end_pos);
                        if distance > 30.0 || pass.end_pos.x > 75.0 || pass.end_pos.x < 30.0 {
                            EventImportance::Important
                        } else {
                            EventImportance::All
                        }
                    }
                    PassOutcome::Intercepted | PassOutcome::Out => EventImportance::Important,
                }
            }

            // All other events - only in Full
            Event::Dribble(_) => EventImportance::All,
            Event::Tackle(_) => EventImportance::All,

            // Possession events - low importance for highlights
            Event::Possession(_) => EventImportance::All,
        }
    }

    /// Check if this event involves the specified player directly
    pub fn involves_player(&self, player_id: &str) -> bool {
        let base = self.base();
        if base.player_id == player_id {
            return true;
        }

        match self {
            Event::Pass(pass) => pass.receiver_id == player_id,
            Event::Tackle(tackle) => tackle.opponent_id == player_id,
            Event::Foul(foul) => foul.opponent_id == player_id,
            Event::Substitution(sub) => sub.out_id == player_id || sub.in_id == player_id,
            _ => false,
        }
    }

    /// Check if this event involves the specified player indirectly (nearby or related)
    pub fn involves_player_indirectly(&self, player_id: &str, _events: &[Event]) -> bool {
        // For now, just check direct involvement
        // In a full implementation, this could check for:
        // - Players within 10m of the action
        // - Players involved in the previous/next event in a sequence
        // - Assists for goals
        self.involves_player(player_id)
    }
}

impl HighlightFilter for HighlightLevel {
    fn filter_events(&self, events: Vec<Event>) -> HighlightResult {
        let total_events = events.len();
        let threshold = self.importance_threshold();

        let filtered_events: Vec<Event> = match self {
            HighlightLevel::Skip => vec![],
            HighlightLevel::Full => events,
            _ => events.into_iter().filter(|event| event.importance() <= threshold).collect(),
        };

        let summary = HighlightSummary {
            total_events,
            filtered_events: filtered_events.len(),
            goals: filtered_events
                .iter()
                .filter(|e| matches!(e, Event::Shot(ShotEvent { outcome: ShotOutcome::Goal, .. })))
                .count(),
            major_incidents: filtered_events
                .iter()
                .filter(|e| e.importance() == EventImportance::Major)
                .count(),
            player_events: None,
        };

        HighlightResult { level: self.clone(), events: filtered_events, summary }
    }
}

impl HighlightFilter for MyPlayerConfig {
    fn filter_events(&self, events: Vec<Event>) -> HighlightResult {
        let total_events = events.len();
        let mut player_event_count = 0;

        let filtered_events: Vec<Event> = events
            .into_iter()
            .filter(|event| {
                // Check if any of our tracked players are involved
                let player_involved = self.player_ids.iter().any(|pid| {
                    if self.include_indirect {
                        event.involves_player_indirectly(pid, &[])
                    } else {
                        event.involves_player(pid)
                    }
                });

                // Always include major incidents if configured
                let is_major =
                    self.include_major_incidents && event.importance() == EventImportance::Major;

                if player_involved {
                    player_event_count += 1;
                }

                player_involved || is_major
            })
            .collect();

        let summary = HighlightSummary {
            total_events,
            filtered_events: filtered_events.len(),
            goals: filtered_events
                .iter()
                .filter(|e| matches!(e, Event::Shot(ShotEvent { outcome: ShotOutcome::Goal, .. })))
                .count(),
            major_incidents: filtered_events
                .iter()
                .filter(|e| e.importance() == EventImportance::Major)
                .count(),
            player_events: Some(player_event_count),
        };

        HighlightResult { level: HighlightLevel::MyPlayer, events: filtered_events, summary }
    }
}

/// Convenience functions for creating highlight filters
impl HighlightLevel {
    /// Create a skip-level filter (no events)
    pub fn skip() -> Self {
        HighlightLevel::Skip
    }

    /// Create a simple filter (goals and major incidents only)
    pub fn simple() -> Self {
        HighlightLevel::Simple
    }

    /// Create a full filter (all events)
    pub fn full() -> Self {
        HighlightLevel::Full
    }
}

impl MyPlayerConfig {
    /// Create a new MyPlayer configuration for a single player
    pub fn single_player(player_id: String) -> Self {
        Self { player_ids: vec![player_id], include_indirect: false, include_major_incidents: true }
    }

    /// Create a new MyPlayer configuration for multiple players
    pub fn multiple_players(player_ids: Vec<String>) -> Self {
        Self { player_ids, include_indirect: false, include_major_incidents: true }
    }

    /// Enable indirect event inclusion
    pub fn with_indirect_events(mut self) -> Self {
        self.include_indirect = true;
        self
    }

    /// Disable major incident inclusion
    pub fn without_major_incidents(mut self) -> Self {
        self.include_major_incidents = false;
        self
    }
}

/// Utility function to analyze event distribution for highlight optimization
pub fn analyze_event_distribution(events: &[Event]) -> EventDistribution {
    let mut distribution = EventDistribution::default();

    for event in events {
        distribution.total += 1;

        match event.importance() {
            EventImportance::Major => distribution.major += 1,
            EventImportance::Important => distribution.important += 1,
            EventImportance::All => distribution.all += 1,
            EventImportance::Skip => {} // Should never happen
        }

        match event {
            Event::Shot(_) => distribution.shots += 1,
            Event::Pass(_) => distribution.passes += 1,
            Event::Dribble(_) => distribution.dribbles += 1,
            Event::Tackle(_) => distribution.tackles += 1,
            Event::Foul(_) => distribution.fouls += 1,
            Event::Save(_) => distribution.saves += 1,
            Event::SetPiece(_) => distribution.set_pieces += 1,
            Event::Substitution(_) => distribution.substitutions += 1,
            Event::Possession(_) => {} // Possession events not tracked in distribution
        }
    }

    distribution
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EventDistribution {
    pub total: usize,
    pub major: usize,
    pub important: usize,
    pub all: usize,
    pub shots: usize,
    pub passes: usize,
    pub dribbles: usize,
    pub tackles: usize,
    pub fouls: usize,
    pub saves: usize,
    pub set_pieces: usize,
    pub substitutions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::replay::{
        events::{BaseEvent, DribbleEvent, DribbleOutcome, PassEvent, PassOutcome},
        types::{BallState, CurveType, MeterPos, Team},
    };

    #[test]
    fn test_event_importance_classification() {
        let goal_shot = Event::Shot(ShotEvent {
            base: BaseEvent::new(
                100.0,
                10,
                Team::Home,
                "H9".to_string(),
                MeterPos { x: 80.0, y: 30.0 },
            ),
            target: MeterPos { x: 105.0, y: 34.0 },
            xg: 0.8,
            on_target: true,
            ball: BallState {
                from: MeterPos { x: 80.0, y: 30.0 },
                to: MeterPos { x: 105.0, y: 34.0 },
                speed_mps: 25.0,
                curve: CurveType::None,
            },
            outcome: ShotOutcome::Goal,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            finishing_skill: None,
            curve_factor: None,
        });

        assert_eq!(goal_shot.importance(), EventImportance::Major);

        let simple_pass = Event::Pass(PassEvent {
            base: BaseEvent::new(
                50.0,
                5,
                Team::Home,
                "H5".to_string(),
                MeterPos { x: 40.0, y: 30.0 },
            ),
            end_pos: MeterPos { x: 45.0, y: 32.0 },
            receiver_id: "H6".to_string(),
            ground: true,
            ball: BallState {
                from: MeterPos { x: 40.0, y: 30.0 },
                to: MeterPos { x: 45.0, y: 32.0 },
                speed_mps: 15.0,
                curve: CurveType::None,
            },
            outcome: PassOutcome::Complete,
            distance_m: None,
            passing_skill: None,
            vision: None,
            technique: None,
            force: None,
            danger_level: None,
            is_switch_of_play: None,
            is_line_breaking: None,
            is_through_ball: None,
            intended_target_pos: None,
        });

        assert_eq!(simple_pass.importance(), EventImportance::All);
    }

    #[test]
    fn test_highlight_filtering() {
        let events = vec![
            Event::Shot(ShotEvent {
                base: BaseEvent::new(
                    100.0,
                    10,
                    Team::Home,
                    "H9".to_string(),
                    MeterPos { x: 80.0, y: 30.0 },
                ),
                target: MeterPos { x: 105.0, y: 34.0 },
                xg: 0.8,
                on_target: true,
                ball: BallState {
                    from: MeterPos { x: 80.0, y: 30.0 },
                    to: MeterPos { x: 105.0, y: 34.0 },
                    speed_mps: 25.0,
                    curve: CurveType::None,
                },
                outcome: ShotOutcome::Goal,
                shot_type: None,
                defender_pressure: None,
                angle_to_goal: None,
                distance_to_goal: None,
                composure: None,
                finishing_skill: None,
                curve_factor: None,
            }),
            Event::Pass(PassEvent {
                base: BaseEvent::new(
                    50.0,
                    5,
                    Team::Home,
                    "H5".to_string(),
                    MeterPos { x: 40.0, y: 30.0 },
                ),
                end_pos: MeterPos { x: 45.0, y: 32.0 },
                receiver_id: "H6".to_string(),
                ground: true,
                ball: BallState {
                    from: MeterPos { x: 40.0, y: 30.0 },
                    to: MeterPos { x: 45.0, y: 32.0 },
                    speed_mps: 15.0,
                    curve: CurveType::None,
                },
                outcome: PassOutcome::Complete,
                distance_m: None,
                passing_skill: None,
                vision: None,
                technique: None,
                force: None,
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
                intended_target_pos: None,
            }),
        ];

        // Simple filtering should only include the goal
        let simple_result = HighlightLevel::Simple.filter_events(events.clone());
        assert_eq!(simple_result.events.len(), 1);
        assert_eq!(simple_result.summary.goals, 1);

        // Full filtering should include all events
        let full_result = HighlightLevel::Full.filter_events(events);
        assert_eq!(full_result.events.len(), 2);
    }

    #[test]
    fn test_my_player_filtering() {
        let events = vec![
            Event::Shot(ShotEvent {
                base: BaseEvent::new(
                    100.0,
                    10,
                    Team::Home,
                    "H9".to_string(),
                    MeterPos { x: 80.0, y: 30.0 },
                ),
                target: MeterPos { x: 105.0, y: 34.0 },
                xg: 0.8,
                on_target: true,
                ball: BallState {
                    from: MeterPos { x: 80.0, y: 30.0 },
                    to: MeterPos { x: 105.0, y: 34.0 },
                    speed_mps: 25.0,
                    curve: CurveType::None,
                },
                outcome: ShotOutcome::Goal,
                shot_type: None,
                defender_pressure: None,
                angle_to_goal: None,
                distance_to_goal: None,
                composure: None,
                finishing_skill: None,
                curve_factor: None,
            }),
            Event::Dribble(DribbleEvent {
                base: BaseEvent::new(
                    200.0,
                    20,
                    Team::Away,
                    "A10".to_string(),
                    MeterPos { x: 25.0, y: 30.0 },
                ),
                path: vec![MeterPos { x: 26.0, y: 31.0 }, MeterPos { x: 28.0, y: 32.0 }],
                end_pos: MeterPos { x: 30.0, y: 33.0 },
                beats: vec!["H3".to_string()],
                outcome: DribbleOutcome::Kept,
                success: None,
                opponents_evaded: None,
                space_gained: None,
                pressure_level: None,
                dribbling_skill: None,
                agility: None,
            }),
        ];

        let config = MyPlayerConfig::single_player("H9".to_string());
        let result = config.filter_events(events);

        // Should include the shot by H9 and the goal (major incident)
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.summary.player_events, Some(1));
    }

    #[test]
    fn test_player_involvement() {
        let pass_event = Event::Pass(PassEvent {
            base: BaseEvent::new(
                50.0,
                5,
                Team::Home,
                "H5".to_string(),
                MeterPos { x: 40.0, y: 30.0 },
            ),
            end_pos: MeterPos { x: 45.0, y: 32.0 },
            receiver_id: "H6".to_string(),
            ground: true,
            ball: BallState {
                from: MeterPos { x: 40.0, y: 30.0 },
                to: MeterPos { x: 45.0, y: 32.0 },
                speed_mps: 15.0,
                curve: CurveType::None,
            },
            outcome: PassOutcome::Complete,
            distance_m: None,
            passing_skill: None,
            vision: None,
            technique: None,
            force: None,
            danger_level: None,
            is_switch_of_play: None,
            is_line_breaking: None,
            is_through_ball: None,
            intended_target_pos: None,
        });

        assert!(pass_event.involves_player("H5")); // Passer
        assert!(pass_event.involves_player("H6")); // Receiver
        assert!(!pass_event.involves_player("H7")); // Uninvolved
    }

    #[test]
    fn test_event_distribution_analysis() {
        let events = vec![
            Event::Shot(ShotEvent {
                base: BaseEvent::new(
                    100.0,
                    10,
                    Team::Home,
                    "H9".to_string(),
                    MeterPos { x: 80.0, y: 30.0 },
                ),
                target: MeterPos { x: 105.0, y: 34.0 },
                xg: 0.8,
                on_target: true,
                ball: BallState {
                    from: MeterPos { x: 80.0, y: 30.0 },
                    to: MeterPos { x: 105.0, y: 34.0 },
                    speed_mps: 25.0,
                    curve: CurveType::None,
                },
                outcome: ShotOutcome::Goal,
                shot_type: None,
                defender_pressure: None,
                angle_to_goal: None,
                distance_to_goal: None,
                composure: None,
                finishing_skill: None,
                curve_factor: None,
            }),
            Event::Pass(PassEvent {
                base: BaseEvent::new(
                    50.0,
                    5,
                    Team::Home,
                    "H5".to_string(),
                    MeterPos { x: 40.0, y: 30.0 },
                ),
                end_pos: MeterPos { x: 45.0, y: 32.0 },
                receiver_id: "H6".to_string(),
                ground: true,
                ball: BallState {
                    from: MeterPos { x: 40.0, y: 30.0 },
                    to: MeterPos { x: 45.0, y: 32.0 },
                    speed_mps: 15.0,
                    curve: CurveType::None,
                },
                outcome: PassOutcome::Complete,
                distance_m: None,
                passing_skill: None,
                vision: None,
                technique: None,
                force: None,
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
                intended_target_pos: None,
            }),
        ];

        let distribution = analyze_event_distribution(&events);
        assert_eq!(distribution.total, 2);
        assert_eq!(distribution.major, 1);
        assert_eq!(distribution.all, 1);
        assert_eq!(distribution.shots, 1);
        assert_eq!(distribution.passes, 1);
    }
}

// =============================================================================
// P15-REPLAY-POLISH: Highlight Clip Extraction System
// =============================================================================

/// Configuration for highlight clip extraction
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct HighlightClipConfig {
    /// Seconds of context before the key event
    pub pre_context_secs: f64,
    /// Seconds of context after the key event
    pub post_context_secs: f64,
    /// Minimum score threshold for including a clip (0.0 - 1.0)
    pub min_score: f32,
    /// Maximum number of clips to extract
    pub max_clips: usize,
    /// Merge clips that are within this many seconds of each other
    pub merge_threshold_secs: f64,
    /// Include buildup events leading to key moments
    pub include_buildup: bool,
}

impl Default for HighlightClipConfig {
    fn default() -> Self {
        Self {
            pre_context_secs: 5.0,
            post_context_secs: 3.0,
            min_score: 0.3,
            max_clips: 20,
            merge_threshold_secs: 2.0,
            include_buildup: true,
        }
    }
}

impl HighlightClipConfig {
    /// Create a config for short highlight reels (goals only)
    pub fn goals_only() -> Self {
        Self {
            pre_context_secs: 8.0,
            post_context_secs: 5.0,
            min_score: 0.9,
            max_clips: 10,
            merge_threshold_secs: 3.0,
            include_buildup: true,
        }
    }

    /// Create a config for extended highlights (all major events)
    pub fn extended() -> Self {
        Self {
            pre_context_secs: 6.0,
            post_context_secs: 4.0,
            min_score: 0.5,
            max_clips: 50,
            merge_threshold_secs: 2.0,
            include_buildup: true,
        }
    }

    /// Create a config for full match review
    pub fn full_review() -> Self {
        Self {
            pre_context_secs: 3.0,
            post_context_secs: 2.0,
            min_score: 0.2,
            max_clips: 100,
            merge_threshold_secs: 1.0,
            include_buildup: false,
        }
    }
}

/// A single highlight clip with time range and events
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct HighlightClip {
    /// Unique clip identifier
    pub id: u32,
    /// Start time in seconds from match start
    pub start_t: f64,
    /// End time in seconds from match start
    pub end_t: f64,
    /// The key event that triggered this clip
    pub key_event_idx: usize,
    /// Highlight score (0.0 - 1.0)
    pub score: f32,
    /// Type of highlight
    pub clip_type: HighlightClipType,
    /// All events within this clip's time range
    pub event_indices: Vec<usize>,
    /// Human-readable description
    pub description: String,
}

impl HighlightClip {
    /// Duration of the clip in seconds
    pub fn duration(&self) -> f64 {
        self.end_t - self.start_t
    }
}

/// Type of highlight clip
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HighlightClipType {
    /// Goal scored
    Goal,
    /// Shot on target (saved or post)
    ShotOnTarget,
    /// Shot off target (near miss)
    ShotOffTarget,
    /// Red card
    RedCard,
    /// Yellow card (notable)
    YellowCard,
    /// Penalty kick
    Penalty,
    /// Brilliant save
    Save,
    /// Successful dribble past defender
    SkillfulDribble,
    /// Key pass or assist
    KeyPass,
    /// Dangerous attack sequence
    DangerousAttack,
    /// Counter attack
    CounterAttack,
    /// Set piece (corner, free kick)
    SetPiece,
}

/// Result of highlight clip extraction
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct HighlightClipResult {
    /// Extracted clips sorted by time
    pub clips: Vec<HighlightClip>,
    /// Total duration of all clips in seconds
    pub total_duration: f64,
    /// Summary statistics
    pub summary: ClipExtractionSummary,
}

/// Summary of clip extraction
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ClipExtractionSummary {
    /// Total events analyzed
    pub events_analyzed: usize,
    /// Number of key events identified
    pub key_events_found: usize,
    /// Number of clips extracted
    pub clips_extracted: usize,
    /// Number of clips merged
    pub clips_merged: usize,
    /// Breakdown by clip type
    pub clips_by_type: Vec<(String, usize)>,
}

/// Highlight clip extractor
pub struct HighlightClipExtractor {
    config: HighlightClipConfig,
}

impl HighlightClipExtractor {
    pub fn new(config: HighlightClipConfig) -> Self {
        Self { config }
    }

    /// Extract highlight clips from events
    pub fn extract_clips(&self, events: &[Event]) -> HighlightClipResult {
        // Step 1: Score all events and identify key moments
        let mut scored_events: Vec<(usize, f32, HighlightClipType)> = Vec::new();

        for (idx, event) in events.iter().enumerate() {
            if let Some((score, clip_type)) = self.score_event(event, events, idx) {
                if score >= self.config.min_score {
                    scored_events.push((idx, score, clip_type));
                }
            }
        }

        // Step 2: Sort by score (highest first)
        scored_events.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Step 3: Create clips from top events
        let mut clips: Vec<HighlightClip> = Vec::new();

        for (clip_id, (idx, score, clip_type)) in
            scored_events.iter().take(self.config.max_clips * 2).enumerate()
        {
            let key_event = &events[*idx];
            let key_t = key_event.base().t;

            // Calculate clip time range
            let start_t = (key_t - self.config.pre_context_secs).max(0.0);
            let end_t = key_t + self.config.post_context_secs;

            // Find buildup if configured
            let adjusted_start = if self.config.include_buildup {
                self.find_buildup_start(events, *idx, start_t)
            } else {
                start_t
            };

            // Collect all events in this range
            let event_indices: Vec<usize> = events
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    let t = e.base().t;
                    t >= adjusted_start && t <= end_t
                })
                .map(|(i, _)| i)
                .collect();

            let description = self.generate_description(key_event, clip_type);

            clips.push(HighlightClip {
                id: clip_id as u32,
                start_t: adjusted_start,
                end_t,
                key_event_idx: *idx,
                score: *score,
                clip_type: clip_type.clone(),
                event_indices,
                description,
            });
        }

        // Step 4: Sort clips by time
        clips
            .sort_by(|a, b| a.start_t.partial_cmp(&b.start_t).unwrap_or(std::cmp::Ordering::Equal));

        // Step 5: Merge overlapping clips
        let (merged_clips, merge_count) = self.merge_overlapping_clips(clips);

        // Step 6: Limit to max clips
        let final_clips: Vec<HighlightClip> =
            merged_clips.into_iter().take(self.config.max_clips).collect();

        // Calculate summary
        let total_duration: f64 = final_clips.iter().map(|c| c.duration()).sum();
        let clips_by_type = self.count_clips_by_type(&final_clips);

        HighlightClipResult {
            clips: final_clips.clone(),
            total_duration,
            summary: ClipExtractionSummary {
                events_analyzed: events.len(),
                key_events_found: scored_events.len(),
                clips_extracted: final_clips.len(),
                clips_merged: merge_count,
                clips_by_type,
            },
        }
    }

    /// Score an event for highlight importance
    fn score_event(
        &self,
        event: &Event,
        all_events: &[Event],
        idx: usize,
    ) -> Option<(f32, HighlightClipType)> {
        match event {
            Event::Shot(shot) => {
                let base_score = match shot.outcome {
                    ShotOutcome::Goal => 1.0,
                    ShotOutcome::Saved => 0.6 + shot.xg as f32 * 0.2,
                    ShotOutcome::Post => 0.7 + shot.xg as f32 * 0.2,
                    ShotOutcome::Off => 0.3 + shot.xg as f32 * 0.3,
                };

                // Bonus for high xG shots
                let xg_bonus = if shot.xg > 0.5 { 0.1 } else { 0.0 };

                let clip_type = match shot.outcome {
                    ShotOutcome::Goal => HighlightClipType::Goal,
                    ShotOutcome::Saved | ShotOutcome::Post => HighlightClipType::ShotOnTarget,
                    ShotOutcome::Off => HighlightClipType::ShotOffTarget,
                };

                Some(((base_score + xg_bonus).min(1.0), clip_type))
            }
            Event::Save(save) => {
                // Great saves are more interesting
                let difficulty = save.save_difficulty.unwrap_or(0.5);
                let score = 0.4 + difficulty * 0.4;
                Some((score, HighlightClipType::Save))
            }
            Event::Foul(foul) => match foul.card {
                CardType::Red => Some((0.9, HighlightClipType::RedCard)),
                CardType::Yellow => Some((0.4, HighlightClipType::YellowCard)),
                CardType::None => None,
            },
            Event::SetPiece(sp) => match sp.kind {
                SetPieceKind::Penalty => Some((0.85, HighlightClipType::Penalty)),
                SetPieceKind::Corner | SetPieceKind::FreeKick => {
                    // Check if this set piece leads to a shot
                    let leads_to_shot = self.check_leads_to_shot(all_events, idx);
                    if leads_to_shot {
                        Some((0.5, HighlightClipType::SetPiece))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Event::Dribble(dribble) => {
                // Successful dribbles past defenders are interesting
                if !dribble.beats.is_empty() {
                    let score = 0.3 + (dribble.beats.len() as f32 * 0.15).min(0.3);
                    Some((score, HighlightClipType::SkillfulDribble))
                } else {
                    None
                }
            }
            Event::Pass(pass) => {
                // Key passes (long, penetrating, or in danger zone) that lead to shots
                if pass.outcome == PassOutcome::Complete {
                    let distance = pass.distance_m.unwrap_or(0.0);
                    let is_in_box = pass.end_pos.x > 88.0; // Penalty area

                    if is_in_box && self.check_leads_to_shot(all_events, idx) {
                        let score = 0.35 + (distance as f32 / 100.0).min(0.2);
                        return Some((score, HighlightClipType::KeyPass));
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Check if an event leads to a shot within the next few events
    fn check_leads_to_shot(&self, events: &[Event], idx: usize) -> bool {
        // Look at next 5 events within 10 seconds
        let current_t = events[idx].base().t;
        for i in (idx + 1)..events.len().min(idx + 6) {
            let event = &events[i];
            if event.base().t > current_t + 10.0 {
                break;
            }
            if matches!(event, Event::Shot(_)) {
                return true;
            }
        }
        false
    }

    /// Find the start of a buildup sequence leading to the key event
    fn find_buildup_start(&self, events: &[Event], key_idx: usize, default_start: f64) -> f64 {
        let key_event = &events[key_idx];
        let key_team = &key_event.base().team;
        let key_t = key_event.base().t;

        // Look backwards for possession chain from same team
        let mut buildup_start = default_start;
        let mut possession_count = 0;

        for i in (0..key_idx).rev() {
            let event = &events[i];
            let event_t = event.base().t;

            // Don't go back more than 15 seconds
            if key_t - event_t > 15.0 {
                break;
            }

            // Stop if possession changed teams
            if &event.base().team != key_team {
                // Check if this was a turnover we care about
                if event.is_possession_change() {
                    // This might be a counter-attack start
                    if possession_count >= 2 {
                        buildup_start = event_t - 1.0;
                    }
                    break;
                }
            } else {
                // Same team event
                if event.is_ball_event() {
                    possession_count += 1;
                    buildup_start = event_t - 0.5;
                }
            }

            // Limit buildup to 6 meaningful events
            if possession_count >= 6 {
                break;
            }
        }

        buildup_start.max(0.0)
    }

    /// Merge overlapping clips
    fn merge_overlapping_clips(&self, clips: Vec<HighlightClip>) -> (Vec<HighlightClip>, usize) {
        if clips.is_empty() {
            return (clips, 0);
        }

        let mut merged: Vec<HighlightClip> = Vec::new();
        let mut merge_count = 0;

        for clip in clips {
            if let Some(last) = merged.last_mut() {
                // Check if clips overlap or are close enough to merge
                if clip.start_t <= last.end_t + self.config.merge_threshold_secs {
                    // Merge: extend end time and combine events
                    last.end_t = last.end_t.max(clip.end_t);

                    // Keep the higher score
                    if clip.score > last.score {
                        last.score = clip.score;
                        last.clip_type = clip.clip_type;
                        last.key_event_idx = clip.key_event_idx;
                        last.description = clip.description;
                    }

                    // Combine event indices (deduplicated)
                    for idx in clip.event_indices {
                        if !last.event_indices.contains(&idx) {
                            last.event_indices.push(idx);
                        }
                    }
                    last.event_indices.sort();

                    merge_count += 1;
                } else {
                    merged.push(clip);
                }
            } else {
                merged.push(clip);
            }
        }

        (merged, merge_count)
    }

    /// Generate human-readable description for a clip
    fn generate_description(&self, event: &Event, clip_type: &HighlightClipType) -> String {
        let base = event.base();
        let minute = base.minute;
        let player = &base.player_id;
        let team = match base.team {
            super::types::Team::Home => "Home",
            super::types::Team::Away => "Away",
        };

        match clip_type {
            HighlightClipType::Goal => {
                format!("{}' - GOAL! {} ({}) scores", minute, player, team)
            }
            HighlightClipType::ShotOnTarget => {
                format!("{}' - Shot on target by {} ({})", minute, player, team)
            }
            HighlightClipType::ShotOffTarget => {
                format!("{}' - Shot off target by {} ({})", minute, player, team)
            }
            HighlightClipType::RedCard => {
                format!("{}' - RED CARD for {} ({})", minute, player, team)
            }
            HighlightClipType::YellowCard => {
                format!("{}' - Yellow card for {} ({})", minute, player, team)
            }
            HighlightClipType::Penalty => {
                format!("{}' - PENALTY awarded to {}", minute, team)
            }
            HighlightClipType::Save => {
                format!("{}' - Great save by {} ({})", minute, player, team)
            }
            HighlightClipType::SkillfulDribble => {
                format!("{}' - Skillful dribble by {} ({})", minute, player, team)
            }
            HighlightClipType::KeyPass => {
                format!("{}' - Key pass by {} ({})", minute, player, team)
            }
            HighlightClipType::DangerousAttack => {
                format!("{}' - Dangerous attack by {}", minute, team)
            }
            HighlightClipType::CounterAttack => {
                format!("{}' - Counter attack by {}", minute, team)
            }
            HighlightClipType::SetPiece => {
                format!("{}' - Set piece by {}", minute, team)
            }
        }
    }

    /// Count clips by type
    fn count_clips_by_type(&self, clips: &[HighlightClip]) -> Vec<(String, usize)> {
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for clip in clips {
            let type_name = format!("{:?}", clip.clip_type);
            *counts.entry(type_name).or_insert(0) += 1;
        }

        let mut result: Vec<_> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }
}

/// Convenience function to extract highlights with default config
pub fn extract_highlight_clips(events: &[Event]) -> HighlightClipResult {
    let extractor = HighlightClipExtractor::new(HighlightClipConfig::default());
    extractor.extract_clips(events)
}

/// Convenience function to extract goals-only highlights
pub fn extract_goals_highlights(events: &[Event]) -> HighlightClipResult {
    let extractor = HighlightClipExtractor::new(HighlightClipConfig::goals_only());
    extractor.extract_clips(events)
}

/// Convenience function to extract extended highlights
pub fn extract_extended_highlights(events: &[Event]) -> HighlightClipResult {
    let extractor = HighlightClipExtractor::new(HighlightClipConfig::extended());
    extractor.extract_clips(events)
}

#[cfg(test)]
mod clip_extraction_tests {
    use super::*;
    use crate::models::replay::{
        events::{BaseEvent, FoulEvent, SaveEvent},
        types::{BallState, CurveType, MeterPos, Team},
    };

    fn create_goal_event(t: f64, minute: u32, player: &str) -> Event {
        Event::Shot(ShotEvent {
            base: BaseEvent::new(
                t,
                minute,
                Team::Home,
                player.to_string(),
                MeterPos { x: 85.0, y: 35.0 },
            ),
            target: MeterPos { x: 105.0, y: 34.0 },
            xg: 0.75,
            on_target: true,
            ball: BallState {
                from: MeterPos { x: 85.0, y: 35.0 },
                to: MeterPos { x: 105.0, y: 34.0 },
                speed_mps: 28.0,
                curve: CurveType::None,
            },
            outcome: ShotOutcome::Goal,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            finishing_skill: None,
            curve_factor: None,
        })
    }

    fn create_save_event(t: f64, minute: u32, player: &str, difficulty: f32) -> Event {
        Event::Save(SaveEvent {
            base: BaseEvent::new(
                t,
                minute,
                Team::Home,
                player.to_string(),
                MeterPos { x: 3.0, y: 34.0 },
            ),
            ball: BallState {
                from: MeterPos { x: 20.0, y: 30.0 },
                to: MeterPos { x: 3.0, y: 34.0 },
                speed_mps: 25.0,
                curve: CurveType::None,
            },
            parry_to: Some(MeterPos { x: 10.0, y: 40.0 }),
            shot_from: Some(MeterPos { x: 20.0, y: 30.0 }),
            shot_power: Some(0.8),
            save_difficulty: Some(difficulty),
            reflexes_skill: None,
            handling_skill: None,
            diving_skill: None,
        })
    }

    fn create_red_card_event(t: f64, minute: u32, player: &str) -> Event {
        Event::Foul(FoulEvent {
            base: BaseEvent::new(
                t,
                minute,
                Team::Away,
                player.to_string(),
                MeterPos { x: 50.0, y: 30.0 },
            ),
            opponent_id: "H10".to_string(),
            card: CardType::Red,
        })
    }

    #[test]
    fn test_extract_goal_clips() {
        let events = vec![
            create_goal_event(600.0, 10, "H9"),
            create_goal_event(1800.0, 30, "H11"),
            create_goal_event(3600.0, 60, "A7"),
        ];

        let result = extract_highlight_clips(&events);

        assert_eq!(result.clips.len(), 3);
        assert!(result.clips.iter().all(|c| c.clip_type == HighlightClipType::Goal));
        assert!(result.clips.iter().all(|c| c.score >= 0.9));
    }

    #[test]
    fn test_goals_only_config() {
        let events = vec![
            create_goal_event(600.0, 10, "H9"),
            create_save_event(900.0, 15, "A1", 0.5),
            create_goal_event(1800.0, 30, "H11"),
        ];

        let result = extract_goals_highlights(&events);

        // Goals only should filter out the save
        assert_eq!(result.summary.clips_extracted, 2);
        assert!(result.clips.iter().all(|c| c.clip_type == HighlightClipType::Goal));
    }

    #[test]
    fn test_extended_highlights_includes_saves() {
        let events = vec![
            create_goal_event(600.0, 10, "H9"),
            create_save_event(900.0, 15, "A1", 0.8),
            create_goal_event(1800.0, 30, "H11"),
        ];

        let result = extract_extended_highlights(&events);

        // Extended should include the save
        assert!(result.clips.len() >= 2);
    }

    #[test]
    fn test_red_card_high_score() {
        let events = vec![create_red_card_event(1500.0, 25, "A5")];

        let result = extract_highlight_clips(&events);

        assert_eq!(result.clips.len(), 1);
        assert_eq!(result.clips[0].clip_type, HighlightClipType::RedCard);
        assert!(result.clips[0].score >= 0.8);
    }

    #[test]
    fn test_clip_merging() {
        // Two events close together should be merged
        let events = vec![
            create_goal_event(600.0, 10, "H9"),
            create_goal_event(603.0, 10, "H9"), // 3 seconds later
        ];

        let config = HighlightClipConfig { merge_threshold_secs: 5.0, ..Default::default() };
        let extractor = HighlightClipExtractor::new(config);
        let result = extractor.extract_clips(&events);

        // Should be merged into one clip
        assert_eq!(result.clips.len(), 1);
        assert!(result.summary.clips_merged > 0);
    }

    #[test]
    fn test_clip_description() {
        let events = vec![create_goal_event(600.0, 10, "H9")];

        let result = extract_highlight_clips(&events);

        assert!(!result.clips.is_empty());
        let desc = &result.clips[0].description;
        assert!(desc.contains("GOAL"));
        assert!(desc.contains("H9"));
        assert!(desc.contains("10'"));
    }

    #[test]
    fn test_clip_time_ranges() {
        let events = vec![create_goal_event(600.0, 10, "H9")];

        let config = HighlightClipConfig {
            pre_context_secs: 5.0,
            post_context_secs: 3.0,
            ..Default::default()
        };
        let extractor = HighlightClipExtractor::new(config);
        let result = extractor.extract_clips(&events);

        assert_eq!(result.clips.len(), 1);
        let clip = &result.clips[0];
        assert!(clip.start_t <= 595.0); // At most 5 seconds before (with possible buildup)
        assert_eq!(clip.end_t, 603.0); // Exactly 3 seconds after
    }

    #[test]
    fn test_max_clips_limit() {
        // Create many goal events
        let events: Vec<Event> =
            (0..30).map(|i| create_goal_event(i as f64 * 200.0, i as u32 * 3, "H9")).collect();

        let config = HighlightClipConfig { max_clips: 10, ..Default::default() };
        let extractor = HighlightClipExtractor::new(config);
        let result = extractor.extract_clips(&events);

        assert!(result.clips.len() <= 10);
    }

    #[test]
    fn test_summary_statistics() {
        let events = vec![
            create_goal_event(600.0, 10, "H9"),
            create_save_event(900.0, 15, "A1", 0.8),
            create_red_card_event(1500.0, 25, "A5"),
        ];

        let result = extract_extended_highlights(&events);

        assert_eq!(result.summary.events_analyzed, 3);
        assert!(result.summary.key_events_found >= 2);
        assert!(result.total_duration > 0.0);
        assert!(!result.summary.clips_by_type.is_empty());
    }
}
