//! Open-Football Replay System
//!
//! This module provides a comprehensive replay system for football matches,
//! including event recording, validation, and 4-level highlight filtering.
//!
//! # Architecture
//!
//! The replay system is organized into several modules:
//! - `types`: Core data types (positions, velocities, ball states)
//! - `events`: All event types and their structures
//! - `match_info`: Match metadata and field specifications
//! - `validation`: Comprehensive validation logic
//! - `highlights`: 4-level filtering system (Skip/Simple/MyPlayer/Full)
//!
//! # Usage
//!
//! ```rust
//! use of_core::models::replay::{Event, HighlightFilter, HighlightLevel, MyPlayerConfig, OfReplay};
//!
//! let events: Vec<Event> = Vec::new();
//!
//! // Create a new replay
//! let replay = OfReplay::new("match-001".to_string(), 12345, events);
//!
//! // Filter highlights
//! let simple_highlights = HighlightLevel::Simple.filter_events(replay.events.clone());
//! let player_highlights = MyPlayerConfig::single_player("H9".to_string())
//!     .with_indirect_events()
//!     .filter_events(replay.events.clone());
//!
//! let _ = (simple_highlights, player_highlights);
//! ```

pub mod events;
pub mod highlights;
pub mod match_info;
pub mod migration;
pub mod types;
pub mod validation;

#[cfg(test)]
pub mod proptest_gen;

#[cfg(test)]
pub mod snapshot_tests;

// Re-export core types
pub use types::{BallState, BuildInfo, CurveType, MeterPos, SchemaInfo, Team, Velocity};

// Re-export events
pub use events::{
    BaseEvent, CardType, DribbleEvent, DribbleOutcome, Event, EventMeta, FoulEvent, PassEvent,
    PassOutcome, SaveEvent, SetPieceEvent, SetPieceKind, ShotEvent, ShotOutcome, SubstitutionEvent,
    TackleEvent,
};

// Re-export match info
pub use match_info::{AreasSpec, GoalSpec, MatchInfo, Period, PitchSpec};

// Re-export validation
pub use validation::{
    validate_event_flow, validate_event_sequence, Validate, ValidationContext, ValidationError,
    ValidationResult,
};

// Re-export highlights
pub use highlights::{
    analyze_event_distribution, extract_extended_highlights, extract_goals_highlights,
    extract_highlight_clips, ClipExtractionSummary, EventDistribution, EventImportance,
    HighlightClip, HighlightClipConfig, HighlightClipExtractor, HighlightClipResult,
    HighlightClipType, HighlightFilter, HighlightLevel, HighlightResult, HighlightSummary,
    MyPlayerConfig,
};

// Re-export migration (FIX_2601/0123 #10)
pub use migration::{
    needs_migration, supported_versions, swap_coordinates, verify_migration,
    MigrationContext, MigrationError, MigrationResult,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::engine::coordinate_contract::{
    COORD_CONTRACT_VERSION, COORD_SYSTEM_LEGACY_AXIS_SWAP, COORD_SYSTEM_METERS_V2,
};

/// Main replay structure containing all match data
fn coord_contract_version_default_legacy() -> u8 {
    0
}

fn coord_system_default_legacy() -> String {
    COORD_SYSTEM_LEGACY_AXIS_SWAP.to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct OfReplay {
    #[serde(default = "coord_contract_version_default_legacy")]
    pub coord_contract_version: u8,
    #[serde(default = "coord_system_default_legacy")]
    pub coord_system: String,
    pub schema: SchemaInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildInfo>,
    pub match_info: MatchInfo,
    pub events: Vec<Event>,
}

impl OfReplay {
    /// Create new replay with minimal required data
    pub fn new(match_id: String, seed: i64, events: Vec<Event>) -> Self {
        Self {
            coord_contract_version: COORD_CONTRACT_VERSION,
            coord_system: COORD_SYSTEM_METERS_V2.to_string(),
            schema: SchemaInfo::default(),
            build: None,
            match_info: MatchInfo::new(match_id, seed),
            events,
        }
    }

    /// Set build information
    pub fn with_build_info(mut self, of_core: String, gdext: String, build_tag: String) -> Self {
        self.build = Some(BuildInfo {
            of_core: Some(of_core),
            gdext: Some(gdext),
            build_tag: Some(build_tag),
            additional: HashMap::new(),
        });
        self
    }

    /// Set goal specifications
    pub fn with_goal_spec(mut self, goal: GoalSpec) -> Self {
        self.match_info.goal = Some(goal);
        self
    }

    /// Set area specifications
    pub fn with_areas_spec(mut self, areas: AreasSpec) -> Self {
        self.match_info.areas = Some(areas);
        self
    }

    /// Validate entire replay structure
    pub fn validate(&self) -> ValidationResult<()> {
        // Validate schema
        if self.schema.name != "of_replay" || self.schema.version != 1 {
            return Err(ValidationError::Schema("Invalid schema name or version".to_string()));
        }

        if self.coord_contract_version != COORD_CONTRACT_VERSION
            || self.coord_system.as_str() != COORD_SYSTEM_METERS_V2
        {
            return Err(ValidationError::Schema(format!(
                "Invalid coord contract metadata: version={}, system={}",
                self.coord_contract_version, self.coord_system
            )));
        }

        // Validate match info
        self.match_info.validate()?;

        // Validate events
        if self.events.is_empty() {
            return Err(ValidationError::Event("At least one event is required".to_string()));
        }

        let context = ValidationContext::new(&self.match_info);
        validate_event_sequence(&self.events, &context)?;
        validate_event_flow(&self.events, &context)?;

        Ok(())
    }

    /// Get highlight filtered version of this replay
    pub fn highlights(&self, level: HighlightLevel) -> HighlightResult {
        level.filter_events(self.events.clone())
    }

    /// Get highlights for specific player(s)
    pub fn player_highlights(&self, config: MyPlayerConfig) -> HighlightResult {
        config.filter_events(self.events.clone())
    }

    /// Get replay statistics
    pub fn statistics(&self) -> ReplayStatistics {
        let distribution = analyze_event_distribution(&self.events);
        let duration = self.match_info.total_duration();

        ReplayStatistics {
            match_id: self.match_info.id.clone(),
            total_events: self.events.len(),
            duration_seconds: duration,
            goals: distribution.shots, // Approximation - should count actual goals
            distribution,
        }
    }

    /// Generate JSON schema for this replay structure
    pub fn json_schema() -> schemars::schema::RootSchema {
        schemars::schema_for!(OfReplay)
    }

    /// Serialize to JSON with pretty printing
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut replay: Self = serde_json::from_str(json)?;
        replay.normalize_coord_contract_in_place();
        Ok(replay)
    }

    pub fn normalize_coord_contract_in_place(&mut self) {
        match self.coord_system.as_str() {
            COORD_SYSTEM_METERS_V2 => {
                // Ensure version is consistent even if a file wrote only the string.
                self.coord_contract_version = COORD_CONTRACT_VERSION;
            }
            COORD_SYSTEM_LEGACY_AXIS_SWAP => {
                for event in &mut self.events {
                    event.swap_axes_in_place();
                }

                self.coord_contract_version = COORD_CONTRACT_VERSION;
                self.coord_system = COORD_SYSTEM_METERS_V2.to_string();
            }
            _ => {}
        }
    }

    /// Get events within a time range
    pub fn events_in_range(&self, start_t: f64, end_t: f64) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|event| {
                let t = event.base().t;
                t >= start_t && t <= end_t
            })
            .collect()
    }

    /// Get events involving a specific player
    pub fn events_by_player(&self, player_id: &str) -> Vec<&Event> {
        self.events.iter().filter(|event| event.involves_player(player_id)).collect()
    }

    /// Get shot events
    pub fn shot_events(&self) -> Vec<&ShotEvent> {
        self.events
            .iter()
            .filter_map(|event| match event {
                Event::Shot(shot) => Some(shot),
                _ => None,
            })
            .collect()
    }

    /// Get pass events
    pub fn pass_events(&self) -> Vec<&PassEvent> {
        self.events
            .iter()
            .filter_map(|event| match event {
                Event::Pass(pass) => Some(pass),
                _ => None,
            })
            .collect()
    }
}

/// Statistical summary of a replay
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ReplayStatistics {
    pub match_id: String,
    pub total_events: usize,
    pub duration_seconds: f64,
    pub goals: usize,
    pub distribution: EventDistribution,
}

impl Default for OfReplay {
    fn default() -> Self {
        Self::new("default-match".to_string(), 0, vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::replay::{
        events::{ShotEvent, ShotOutcome},
        types::{BallState, CurveType, MeterPos},
    };

    #[test]
    fn test_replay_creation() {
        let events = vec![Event::Shot(ShotEvent {
            base: BaseEvent::new(
                613.1,
                10,
                Team::Home,
                "H9".to_string(),
                MeterPos { x: 83.2, y: 27.0 },
            ),
            target: MeterPos { x: 105.0, y: 33.8 },
            xg: 0.18,
            on_target: true,
            ball: BallState {
                from: MeterPos { x: 83.2, y: 27.0 },
                to: MeterPos { x: 105.0, y: 33.8 },
                speed_mps: 26.2,
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
        })];

        let replay = OfReplay::new("test-match".to_string(), 12345, events);
        assert!(replay.validate().is_ok());
        assert_eq!(replay.match_info.id, "test-match");
        assert_eq!(replay.events.len(), 1);
    }

    #[test]
    fn test_highlight_filtering() {
        let events = vec![Event::Shot(ShotEvent {
            base: BaseEvent::new(
                613.1,
                10,
                Team::Home,
                "H9".to_string(),
                MeterPos { x: 83.2, y: 27.0 },
            ),
            target: MeterPos { x: 105.0, y: 33.8 },
            xg: 0.18,
            on_target: true,
            ball: BallState {
                from: MeterPos { x: 83.2, y: 27.0 },
                to: MeterPos { x: 105.0, y: 33.8 },
                speed_mps: 26.2,
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
        })];

        let replay = OfReplay::new("test-match".to_string(), 12345, events);

        let simple_highlights = replay.highlights(HighlightLevel::Simple);
        assert_eq!(simple_highlights.events.len(), 1);
        assert_eq!(simple_highlights.summary.goals, 1);

        let player_highlights =
            replay.player_highlights(MyPlayerConfig::single_player("H9".to_string()));
        assert_eq!(player_highlights.events.len(), 1);
    }

    #[test]
    fn test_json_serialization() {
        let replay = OfReplay::new("test-match".to_string(), 12345, vec![]);

        let json = replay.to_json_pretty().unwrap();
        let deserialized = OfReplay::from_json(&json).unwrap();

        assert_eq!(replay.match_info.id, deserialized.match_info.id);
        assert_eq!(replay.match_info.seed, deserialized.match_info.seed);
    }

    #[test]
    fn test_event_queries() {
        let events = vec![
            Event::Shot(ShotEvent {
                base: BaseEvent::new(
                    100.0,
                    10,
                    Team::Home,
                    "H9".to_string(),
                    MeterPos { x: 83.2, y: 27.0 },
                ),
                target: MeterPos { x: 105.0, y: 33.8 },
                xg: 0.18,
                on_target: true,
                ball: BallState {
                    from: MeterPos { x: 83.2, y: 27.0 },
                    to: MeterPos { x: 105.0, y: 33.8 },
                    speed_mps: 26.2,
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
            Event::Shot(ShotEvent {
                base: BaseEvent::new(
                    200.0,
                    20,
                    Team::Away,
                    "A10".to_string(),
                    MeterPos { x: 25.0, y: 30.0 },
                ),
                target: MeterPos { x: 0.0, y: 34.0 },
                xg: 0.15,
                on_target: false,
                ball: BallState {
                    from: MeterPos { x: 25.0, y: 30.0 },
                    to: MeterPos { x: 0.0, y: 34.0 },
                    speed_mps: 22.0,
                    curve: CurveType::None,
                },
                outcome: ShotOutcome::Off,
                shot_type: None,
                defender_pressure: None,
                angle_to_goal: None,
                distance_to_goal: None,
                composure: None,
                finishing_skill: None,
                curve_factor: None,
            }),
        ];

        let replay = OfReplay::new("test-match".to_string(), 12345, events);

        // Test time range queries
        let first_half = replay.events_in_range(0.0, 150.0);
        assert_eq!(first_half.len(), 1);

        // Test player queries
        let h9_events = replay.events_by_player("H9");
        assert_eq!(h9_events.len(), 1);

        let a10_events = replay.events_by_player("A10");
        assert_eq!(a10_events.len(), 1);
    }
}
