//! Data structures for broadcast timeline export

use serde::{Deserialize, Serialize};

/// High-level highlight timeline containing all clips for a match
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HighlightTimeline {
    /// Timeline format version for compatibility
    pub version: String,
    /// Unique match identifier
    pub match_id: String,
    /// All highlight clips in the timeline
    pub clips: Vec<HighlightClip>,
    /// Timeline metadata
    pub metadata: TimelineMetadata,
}

/// Individual highlight clip in the timeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HighlightClip {
    /// Unique clip identifier
    pub id: String,
    /// Type of clip (goal_highlight, goal_buildup, foul, etc.)
    pub clip_type: String,
    /// Start time in match (seconds)
    pub start: f32,
    /// End time in match (seconds)
    pub end: f32,
    /// Camera preset to use (Cine_Main, Cine_Side, Cine_Top, Cine_Ball)
    pub camera: String,
    /// Visual effect to apply (slowmo, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    /// Importance score (0.0 - 1.0)
    pub importance: f32,
    /// Source event ID that generated this clip
    pub event_id: String,
    /// Sub-clips for multi-angle replays
    #[serde(default)]
    pub sub_clips: Vec<HighlightClip>,
}

/// Metadata about the generated timeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineMetadata {
    /// Total duration of match in seconds
    pub total_duration: f32,
    /// Number of clips in timeline
    pub clip_count: usize,
    /// When this timeline was generated
    pub generated_at: String, // ISO 8601 timestamp
}

/// Error types for broadcast export operations
#[derive(thiserror::Error, Debug)]
pub enum ExportError {
    #[error("JSON serialization failed: {0}")]
    Serialization(String),

    #[error("JSON deserialization failed: {0}")]
    Deserialization(String),

    #[error("File read failed: {0}")]
    FileRead(String),

    #[error("File write failed: {0}")]
    FileWrite(String),

    #[error("Invalid timeline data: {0}")]
    InvalidData(String),

    #[error("No events to process")]
    NoEvents,
}
