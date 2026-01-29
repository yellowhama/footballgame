//! Clip Reducer - ChanceScore-based highlight and key moment detection
//!
//! This module implements the 3-Mode Viewing System's clip detection logic:
//! - Full Match: Entire match playback
//! - Highlight: Clips with ChanceScore ≥ 0.10
//! - Key Moment: Clips with ChanceScore ≥ 0.25 or goals/penalties
//!
//! ## Core Concepts
//!
//! ### ChanceScore Calculation
//! ```text
//! ChanceScore = max(
//!     ShotXG,
//!     PassAssistXG * 0.85,
//!     CarryIntoBoxScore,
//!     OneOnOneScore
//! )
//! ```
//!
//! ### Possession-Chain Based Clips
//! Clips start at the moment the attacking team gained possession (+ pre-roll),
//! not at a fixed time offset. This ensures meaningful context.
//!
//! ### Clip Limits
//! - MIN_CLIP_DURATION_MS: 5 seconds
//! - MAX_CLIP_DURATION_MS: 12 seconds
//! - MERGE_GAP_MS: 1.5 seconds (merge nearby clips)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::{MeterPos, ReplayEvent};

// ============================================================================
// Constants
// ============================================================================

/// Minimum clip duration (milliseconds)
pub const MIN_CLIP_DURATION_MS: u64 = 5_000;

/// Maximum clip duration (milliseconds)
pub const MAX_CLIP_DURATION_MS: u64 = 12_000;

/// Merge gap threshold (milliseconds)
pub const MERGE_GAP_MS: u64 = 1_500;

/// Pre-roll before clip start (milliseconds)
pub const CLIP_PRE_ROLL_MS: u64 = 500;

/// Post-roll after trigger event (milliseconds)
pub const CLIP_POST_ROLL_MS: u64 = 2_000;

/// Highlight mode threshold
pub const HIGHLIGHT_THRESHOLD: f32 = 0.10;

/// Key moment mode threshold
pub const KEY_MOMENT_THRESHOLD: f32 = 0.25;

/// Auto-include threshold (goals, penalties)
pub const AUTO_INCLUDE_THRESHOLD: f32 = 1.0;

// Zone definitions (normalized coordinates)
/// Penalty area: pos.1 ∈ [0.83, 1.0], pos.0 ∈ [0.37, 0.63]
pub const PENALTY_AREA_MIN_LENGTH: f32 = 0.83;
pub const PENALTY_AREA_MAX_LENGTH: f32 = 1.0;
pub const PENALTY_AREA_MIN_WIDTH: f32 = 0.37;
pub const PENALTY_AREA_MAX_WIDTH: f32 = 0.63;

/// Zone 14: pos.1 ∈ [0.67, 0.83], pos.0 ∈ [0.30, 0.70]
pub const ZONE14_MIN_LENGTH: f32 = 0.67;
pub const ZONE14_MAX_LENGTH: f32 = 0.83;
pub const ZONE14_MIN_WIDTH: f32 = 0.30;
pub const ZONE14_MAX_WIDTH: f32 = 0.70;

// Field dimensions (meters) - FIFA 105x68 standard
pub const FIELD_LENGTH_M: f64 = 105.0;
pub const FIELD_WIDTH_M: f64 = 68.0;

// ============================================================================
// Data Structures
// ============================================================================

/// Clip mode classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipMode {
    /// Full match playback (no filtering)
    FullMatch,
    /// Highlight clips (ChanceScore ≥ 0.10)
    Highlight,
    /// Key moment clips (ChanceScore ≥ 0.25 or goals/penalties)
    KeyMoment,
}

/// Clip definition with time range and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipDefinition {
    /// Unique clip ID
    pub id: String,
    /// Clip mode classification
    pub mode: ClipMode,
    /// Start time (milliseconds from match start)
    pub start_ms: u64,
    /// End time (milliseconds from match start)
    pub end_ms: u64,
    /// ChanceScore value
    pub chance_score: f32,
    /// Index of trigger event in event stream
    pub trigger_event_idx: usize,
    /// Human-readable description
    pub description: String,
}

/// Active carry tracking context
#[derive(Debug, Clone)]
pub struct CarryContext {
    /// Starting position (meters)
    pub start_pos: MeterPos,
    /// Starting tick
    pub start_tick: u64,
    /// Current position (meters)
    pub current_pos: MeterPos,
    /// Maximum pressure encountered
    pub pressure: f32,
}

/// Main clip reducer - processes event stream and generates clips
#[derive(Debug)]
pub struct ClipReducer {
    /// Event stream (owned)
    events: Vec<ReplayEvent>,
    /// Detected clips
    clips: Vec<ClipDefinition>,
    /// Current tick
    current_tick: u64,
    /// Current possession owner (track_id)
    current_possession: Option<u32>,
    /// Active carries by player (track_id -> CarryContext)
    active_carries: HashMap<u32, CarryContext>,
    /// Last shot event index (for pass assist detection)
    last_shot_idx: Option<usize>,
    /// Match duration (milliseconds)
    match_duration_ms: u64,
}

// ============================================================================
// Core Implementation
// ============================================================================

impl ClipReducer {
    /// Create new ClipReducer
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            clips: Vec::new(),
            current_tick: 0,
            current_possession: None,
            active_carries: HashMap::new(),
            last_shot_idx: None,
            match_duration_ms: 0,
        }
    }

    /// Initialize with event stream
    pub fn load_events(&mut self, events: Vec<ReplayEvent>) {
        // Find match duration from last event
        if let Some(last_evt) = events.last() {
            self.match_duration_ms = (last_evt.base().t * 1000.0) as u64;
        }

        self.events = events;
        self.clips.clear();
        self.current_tick = 0;
        self.current_possession = None;
        self.active_carries.clear();
        self.last_shot_idx = None;
    }

    /// Process all events and generate clips
    pub fn process(&mut self) {
        for i in 0..self.events.len() {
            self.process_event(i);
        }

        // Finalize: sort and merge clips
        self.finalize();
    }

    /// Process single event
    fn process_event(&mut self, event_idx: usize) {
        let event = &self.events[event_idx];

        match event {
            ReplayEvent::Shot { base, from, xg, .. } => {
                self.on_shot(event_idx, base.t, *from, xg.unwrap_or(0.0) as f32);
            }
            ReplayEvent::Goal { base, at, .. } => {
                self.on_goal(event_idx, base.t, *at);
            }
            ReplayEvent::Penalty { base, .. } => {
                self.on_penalty(event_idx, base.t);
            }
            // Note: Possession change and carry tracking would require additional event types
            // For now, we'll implement the core shot/goal handling
            _ => {}
        }
    }

    /// Handle shot event
    fn on_shot(&mut self, event_idx: usize, t: f64, _from: MeterPos, xg: f32) {
        self.last_shot_idx = Some(event_idx);

        let chance_score = xg;

        if chance_score >= HIGHLIGHT_THRESHOLD {
            self.create_clip(event_idx, t, chance_score, "Shot");
        }
    }

    /// Handle goal event
    fn on_goal(&mut self, event_idx: usize, t: f64, _at: MeterPos) {
        // Goals auto-include with max ChanceScore
        self.create_clip(event_idx, t, AUTO_INCLUDE_THRESHOLD, "Goal");
    }

    /// Handle penalty event
    fn on_penalty(&mut self, event_idx: usize, t: f64) {
        // Penalties auto-include
        self.create_clip(event_idx, t, AUTO_INCLUDE_THRESHOLD, "Penalty");
    }

    /// Create clip from trigger event
    fn create_clip(&mut self, trigger_idx: usize, trigger_t: f64, chance_score: f32, desc: &str) {
        let trigger_ms = (trigger_t * 1000.0) as u64;

        // Find clip start (possession-chain based)
        let start_ms = self.find_clip_start(trigger_idx, trigger_ms);

        // Find clip end (2s after trigger OR next possession change)
        let end_ms = self.find_clip_end(trigger_idx, trigger_ms);

        // Apply duration limits
        let (start_ms, end_ms) = self.apply_duration_limits(start_ms, end_ms, trigger_ms);

        // Determine mode
        let mode = if chance_score >= KEY_MOMENT_THRESHOLD {
            ClipMode::KeyMoment
        } else if chance_score >= HIGHLIGHT_THRESHOLD {
            ClipMode::Highlight
        } else {
            return; // Don't create clip
        };

        let clip = ClipDefinition {
            id: format!("clip_{}_{}", trigger_ms, (chance_score * 100.0) as u32),
            mode,
            start_ms,
            end_ms,
            chance_score,
            trigger_event_idx: trigger_idx,
            description: desc.to_string(),
        };

        self.clips.push(clip);
    }

    /// Find clip start time (possession-chain based)
    fn find_clip_start(&self, _trigger_idx: usize, trigger_ms: u64) -> u64 {
        // TODO: Walk backwards to find possession change
        // For now, use simple time offset
        // -8 seconds
        trigger_ms.saturating_sub(8_000)
    }

    /// Find clip end time
    fn find_clip_end(&self, _trigger_idx: usize, trigger_ms: u64) -> u64 {
        // End: 2 seconds after trigger OR next possession change
        let max_end_ms = trigger_ms + CLIP_POST_ROLL_MS;

        // TODO: Check for possession change between trigger and max_end_ms
        // For now, return max_end_ms
        max_end_ms.min(self.match_duration_ms)
    }

    /// Apply duration limits (5s min, 12s max)
    fn apply_duration_limits(
        &self,
        mut start_ms: u64,
        mut end_ms: u64,
        trigger_ms: u64,
    ) -> (u64, u64) {
        let mut duration = end_ms.saturating_sub(start_ms);

        // Too short: extend backward first, then forward if needed
        if duration < MIN_CLIP_DURATION_MS {
            let needed = MIN_CLIP_DURATION_MS - duration;
            start_ms = start_ms.saturating_sub(needed);

            // Recalculate duration after extending backward
            duration = end_ms.saturating_sub(start_ms);

            // Still too short? Extend forward
            if duration < MIN_CLIP_DURATION_MS {
                let still_needed = MIN_CLIP_DURATION_MS - duration;
                end_ms = end_ms.saturating_add(still_needed);
            }
        }

        // Too long: trim (keep trigger centered)
        let final_duration = end_ms.saturating_sub(start_ms);
        if final_duration > MAX_CLIP_DURATION_MS {
            let half = MAX_CLIP_DURATION_MS / 2;
            start_ms = trigger_ms.saturating_sub(half);
            end_ms = start_ms + MAX_CLIP_DURATION_MS;
        }

        // Clamp to match boundaries
        start_ms = start_ms.clamp(0, self.match_duration_ms);
        end_ms = end_ms.clamp(start_ms, self.match_duration_ms);

        (start_ms, end_ms)
    }

    /// Finalize: sort and merge clips
    fn finalize(&mut self) {
        // Sort by start time
        self.clips.sort_by_key(|c| c.start_ms);

        // Merge overlapping/nearby clips
        self.merge_clips();
    }

    /// Merge clips with gap ≤ MERGE_GAP_MS
    fn merge_clips(&mut self) {
        if self.clips.is_empty() {
            return;
        }

        let mut merged = Vec::new();
        let mut i = 0;

        while i < self.clips.len() {
            let mut current = self.clips[i].clone();

            // Look ahead for nearby clips
            while i + 1 < self.clips.len() {
                let next = &self.clips[i + 1];

                // Gap ≤ 1.5s: merge
                if next.start_ms.saturating_sub(current.end_ms) <= MERGE_GAP_MS {
                    current.end_ms = next.end_ms;
                    current.chance_score = current.chance_score.max(next.chance_score);

                    // Update mode based on max ChanceScore
                    current.mode = if current.chance_score >= KEY_MOMENT_THRESHOLD {
                        ClipMode::KeyMoment
                    } else {
                        ClipMode::Highlight
                    };

                    // Update description
                    if next.chance_score > current.chance_score {
                        current.description = next.description.clone();
                    }

                    i += 1;
                } else {
                    break;
                }
            }

            merged.push(current);
            i += 1;
        }

        self.clips = merged;
    }

    /// Get all clips
    pub fn get_clips(&self) -> &[ClipDefinition] {
        &self.clips
    }

    /// Get clips filtered by mode
    pub fn get_clips_by_mode(&self, mode: ClipMode) -> Vec<ClipDefinition> {
        self.clips.iter().filter(|c| c.mode == mode).cloned().collect()
    }

    /// Get clip count
    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }
}

impl Default for ClipReducer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ChanceScore Calculation Functions
// ============================================================================

/// Calculate ChanceScore for carry into box
///
/// Returns score based on zone entry and pressure:
/// - Penalty Area entry: 0.12
/// - Zone 14 entry: 0.10
/// - Pressure bonus (>0.7): +0.03
pub fn calculate_carry_into_box_score(
    start_pos: MeterPos,
    end_pos: MeterPos,
    pressure: f32,
) -> f32 {
    let mut score = 0.0;

    // Convert meters to normalized coordinates
    let start_norm = meters_to_normalized(start_pos);
    let end_norm = meters_to_normalized(end_pos);

    // Check penalty area entry
    if !is_in_penalty_area(start_norm) && is_in_penalty_area(end_norm) {
        score = 0.12;
    }
    // Check Zone 14 entry
    else if !is_in_zone14(start_norm) && is_in_zone14(end_norm) {
        score = 0.10;
    }

    // Pressure bonus
    if pressure > 0.7 {
        score += 0.03;
    }

    score
}

/// Calculate ChanceScore for pass assist (pass leading to shot)
pub fn calculate_pass_assist_xg(receiver_shot_xg: f32) -> f32 {
    // Discount by 0.85
    receiver_shot_xg * 0.85
}

/// Calculate ChanceScore for one-on-one situation
pub fn calculate_one_on_one_score(
    carrier_pos: MeterPos,
    nearest_defender_distance: f32,
    distance_to_goal: f32,
) -> f32 {
    let carrier_norm = meters_to_normalized(carrier_pos);

    // Must be in attacking third
    if carrier_norm.1 < 0.67 {
        return 0.0;
    }

    // Isolated situation: defender >5m away, goal <20m away
    if nearest_defender_distance > 5.0 && distance_to_goal < 20.0 {
        return 0.15;
    }

    0.0
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get event base
fn _get_event_base(event: &ReplayEvent) -> &super::types::EventBase {
    event.base()
}

/// Convert meters to normalized coordinates (0.0-1.0)
fn meters_to_normalized(pos: MeterPos) -> (f32, f32) {
    let norm_x = (pos.x / FIELD_LENGTH_M) as f32;
    let norm_y = (pos.y / FIELD_WIDTH_M) as f32;
    (norm_y, norm_x) // Note: (width, length) order for normalized coordinates
}

/// Check if position is in penalty area
fn is_in_penalty_area(norm_pos: (f32, f32)) -> bool {
    norm_pos.1 >= PENALTY_AREA_MIN_LENGTH
        && norm_pos.1 <= PENALTY_AREA_MAX_LENGTH
        && norm_pos.0 >= PENALTY_AREA_MIN_WIDTH
        && norm_pos.0 <= PENALTY_AREA_MAX_WIDTH
}

/// Check if position is in Zone 14
fn is_in_zone14(norm_pos: (f32, f32)) -> bool {
    norm_pos.1 >= ZONE14_MIN_LENGTH
        && norm_pos.1 <= ZONE14_MAX_LENGTH
        && norm_pos.0 >= ZONE14_MIN_WIDTH
        && norm_pos.0 <= ZONE14_MAX_WIDTH
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::EventBase;

    fn make_event_base(t: f64) -> EventBase {
        EventBase { t, player_id: Some(1), team_id: Some(1) }
    }

    fn make_pos(x: f64, y: f64) -> MeterPos {
        MeterPos { x, y }
    }

    #[test]
    fn test_carry_into_box_penalty_area() {
        let start = make_pos(80.0, 34.0); // Outside penalty area
        let end = make_pos(95.0, 34.0); // Inside penalty area
        let score = calculate_carry_into_box_score(start, end, 0.0);
        assert_eq!(score, 0.12);
    }

    #[test]
    fn test_carry_into_box_zone14() {
        let start = make_pos(60.0, 34.0); // Outside Zone 14
        let end = make_pos(75.0, 34.0); // Inside Zone 14
        let score = calculate_carry_into_box_score(start, end, 0.0);
        assert_eq!(score, 0.10);
    }

    #[test]
    fn test_carry_with_pressure() {
        let start = make_pos(80.0, 34.0);
        let end = make_pos(95.0, 34.0);
        let score = calculate_carry_into_box_score(start, end, 0.8);
        assert!((score - 0.15).abs() < 0.001); // 0.12 + 0.03, with float tolerance
    }

    #[test]
    fn test_pass_assist_xg() {
        let shot_xg = 0.5;
        let assist_score = calculate_pass_assist_xg(shot_xg);
        assert_eq!(assist_score, 0.425); // 0.5 * 0.85
    }

    #[test]
    fn test_one_on_one_score() {
        let pos = make_pos(90.0, 34.0); // Attacking third
        let score = calculate_one_on_one_score(pos, 6.0, 15.0);
        assert_eq!(score, 0.15);
    }

    #[test]
    fn test_one_on_one_not_attacking_third() {
        let pos = make_pos(50.0, 34.0); // Midfield
        let score = calculate_one_on_one_score(pos, 6.0, 15.0);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_clip_reducer_shot() {
        let mut reducer = ClipReducer::new();

        let events = vec![ReplayEvent::Shot {
            base: make_event_base(10.0),
            from: make_pos(90.0, 34.0),
            target: make_pos(105.0, 34.0),
            on_target: true,
            xg: Some(0.15), // Use 0.15 to ensure Highlight mode (not KeyMoment)
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
        }];

        reducer.load_events(events);
        reducer.process();

        assert_eq!(reducer.clip_count(), 1);
        assert_eq!(reducer.get_clips()[0].mode, ClipMode::Highlight);
        assert_eq!(reducer.get_clips()[0].chance_score, 0.15);
    }

    #[test]
    fn test_clip_reducer_goal() {
        let mut reducer = ClipReducer::new();

        let events = vec![ReplayEvent::Goal {
            base: make_event_base(15.0),
            at: make_pos(105.0, 34.0),
            assist_player_id: None,
        }];

        reducer.load_events(events);
        reducer.process();

        assert_eq!(reducer.clip_count(), 1);
        assert_eq!(reducer.get_clips()[0].mode, ClipMode::KeyMoment);
        assert_eq!(reducer.get_clips()[0].chance_score, 1.0);
    }

    #[test]
    fn test_clip_merging() {
        let mut reducer = ClipReducer::new();

        // Two shots close together (1s gap)
        let events = vec![
            ReplayEvent::Shot {
                base: make_event_base(10.0),
                from: make_pos(90.0, 34.0),
                target: make_pos(105.0, 34.0),
                on_target: true,
                xg: Some(0.15),
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
            ReplayEvent::Shot {
                base: make_event_base(11.0),
                from: make_pos(92.0, 34.0),
                target: make_pos(105.0, 34.0),
                on_target: true,
                xg: Some(0.20),
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

        reducer.load_events(events);
        reducer.process();

        // Should merge into 1 clip
        assert_eq!(reducer.clip_count(), 1);
        assert_eq!(reducer.get_clips()[0].chance_score, 0.20); // max of two
    }

    #[test]
    fn test_duration_limits_too_short() {
        let mut reducer = ClipReducer::new();
        reducer.match_duration_ms = 100_000; // Set match duration to avoid clamping issues

        let (start, end) = reducer.apply_duration_limits(10_000, 12_000, 11_000);

        let duration = end - start;
        assert!(duration >= MIN_CLIP_DURATION_MS);
    }

    #[test]
    fn test_duration_limits_too_long() {
        let mut reducer = ClipReducer::new();
        reducer.match_duration_ms = 100_000;

        let (start, end) = reducer.apply_duration_limits(10_000, 30_000, 20_000);

        let duration = end - start;
        assert!(duration <= MAX_CLIP_DURATION_MS);
    }
}
