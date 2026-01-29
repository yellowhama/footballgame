//! ReplayRecorder - Bridge between tick_based engine and ReplayEvent/ReplayDoc
//!
//! This module provides a thin bridge to convert action results from the
//! tick-based simulation engine into ReplayEvent types for replay visualization.

use super::types::*;

/// Bridge between tick_based engine and ReplayEvent/ReplayDoc
#[derive(Debug)]
pub struct ReplayRecorder {
    /// Pitch dimensions
    pub pitch: PitchSpec,
    /// Recorded events
    pub events: Vec<ReplayEvent>,
    /// Timeline entries for UI
    pub timeline: Vec<ReplayTimelineEntry>,
    /// Team rosters
    pub rosters: ReplayRosters,
    /// Team tactics info
    pub tactics: ReplayTeamsTactics,
}

impl Default for ReplayRecorder {
    fn default() -> Self {
        Self {
            pitch: PitchSpec { width_m: 105.0, height_m: 68.0 },
            events: Vec::new(),
            timeline: Vec::new(),
            rosters: ReplayRosters::default(),
            tactics: ReplayTeamsTactics::default(),
        }
    }
}

impl ReplayRecorder {
    /// Create a new recorder with pitch spec and rosters
    pub fn new(pitch: PitchSpec, rosters: ReplayRosters) -> Self {
        Self {
            pitch,
            events: Vec::with_capacity(3000), // ~3000 events per match typical
            timeline: Vec::with_capacity(50),
            rosters,
            tactics: ReplayTeamsTactics::default(),
        }
    }

    /// Create EventBase with common metadata
    fn make_base(&self, t_seconds: f64, player_id: Option<u32>, team_id: Option<u32>) -> EventBase {
        EventBase { t: t_seconds, player_id, team_id }
    }

    // ========================================
    // Core Recording Methods
    // ========================================

    /// Record kickoff event
    pub fn record_kickoff(&mut self, t_seconds: f64, team_id: u32, player_id: u32) {
        self.events.push(ReplayEvent::KickOff {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
        });
        self.timeline.push(ReplayTimelineEntry {
            t: t_seconds,
            label: "KICK OFF".to_string(),
            team_id: Some(team_id),
            player_id: Some(player_id),
        });
    }

    /// Record pass event
    ///
    /// # 0108 Phase 4: Tactical metadata
    /// - danger_level: Intercept risk (0.0-1.0)
    /// - is_switch_of_play: |from.y - to.y| > 68.0 * 0.4 = 27.2m
    /// - is_line_breaking: Pass that bypasses 2+ defensive lines (caller must set)
    /// - is_through_ball: Pass into space behind defensive line (caller must set)
    pub fn record_pass(
        &mut self,
        t_seconds: f64,
        team_id: u32,
        player_id: u32,
        from: MeterPos,
        to: MeterPos,
        receiver_id: Option<u32>,
        distance_m: f64,
    ) {
        // 0108: Calculate switch of play (lateral distance > 40% of field width)
        // Field width = 68m, 40% = 27.2m
        const SWITCH_THRESHOLD: f64 = 68.0 * 0.4; // 27.2m
        let lateral_dist = (to.y - from.y).abs();
        let is_switch = lateral_dist > SWITCH_THRESHOLD;

        self.events.push(ReplayEvent::Pass {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            from,
            to,
            receiver_id,
            distance_m: Some(distance_m),
            force: None,
            is_clearance: false,
            ground: None,
            outcome: None,
            passing_skill: None,
            vision: None,
            technique: None,
            // 0108 Phase 4: Tactical metadata
            danger_level: None, // Calculated by caller if needed
            is_switch_of_play: if is_switch { Some(true) } else { None },
            is_line_breaking: None, // Requires defensive line data
            is_through_ball: None,  // Requires defensive line data
        });
    }

    /// Record pass event with full tactical metadata
    ///
    /// # 0108 Phase 4: Extended pass recording with tactical analysis
    pub fn record_pass_with_metadata(
        &mut self,
        t_seconds: f64,
        team_id: u32,
        player_id: u32,
        from: MeterPos,
        to: MeterPos,
        receiver_id: Option<u32>,
        distance_m: f64,
        danger_level: Option<f32>,
        is_line_breaking: bool,
        is_through_ball: bool,
    ) {
        // Calculate switch of play
        const SWITCH_THRESHOLD: f64 = 68.0 * 0.4; // 27.2m
        let lateral_dist = (to.y - from.y).abs();
        let is_switch = lateral_dist > SWITCH_THRESHOLD;

        self.events.push(ReplayEvent::Pass {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            from,
            to,
            receiver_id,
            distance_m: Some(distance_m),
            force: None,
            is_clearance: false,
            ground: None,
            outcome: None,
            passing_skill: None,
            vision: None,
            technique: None,
            // 0108 Phase 4: Tactical metadata
            danger_level,
            is_switch_of_play: if is_switch { Some(true) } else { None },
            is_line_breaking: if is_line_breaking { Some(true) } else { None },
            is_through_ball: if is_through_ball { Some(true) } else { None },
        });
    }

    /// Record shot event
    pub fn record_shot(
        &mut self,
        t_seconds: f64,
        team_id: u32,
        player_id: u32,
        from: MeterPos,
        target: MeterPos,
        on_target: bool,
        xg: Option<f64>,
    ) {
        self.events.push(ReplayEvent::Shot {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            from,
            target,
            on_target,
            xg,
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
        // Add to timeline for important events
        self.timeline.push(ReplayTimelineEntry {
            t: t_seconds,
            label: "SHOT".to_string(),
            team_id: Some(team_id),
            player_id: Some(player_id),
        });
    }

    /// Record goal event
    pub fn record_goal(
        &mut self,
        t_seconds: f64,
        team_id: u32,
        player_id: u32,
        at: MeterPos,
        assist_player_id: Option<u32>,
    ) {
        self.events.push(ReplayEvent::Goal {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            at,
            assist_player_id,
        });
        self.timeline.push(ReplayTimelineEntry {
            t: t_seconds,
            label: "GOAL".to_string(),
            team_id: Some(team_id),
            player_id: Some(player_id),
        });
    }

    /// Record save event
    pub fn record_save(&mut self, t_seconds: f64, team_id: u32, player_id: u32, at: MeterPos) {
        self.events.push(ReplayEvent::Save {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            at,
            parry_to: None,
            shot_from: None,
            shot_power: None,
            save_difficulty: None,
            shot_speed: None,
            reflexes_skill: None,
            handling_skill: None,
            diving_skill: None,
            positioning_quality: None,
        });
    }

    /// Record tackle event (logged as Run with purpose="tackle")
    pub fn record_tackle(
        &mut self,
        t_seconds: f64,
        team_id: u32,
        player_id: u32,
        at: MeterPos,
        success: bool,
    ) {
        // Note: ReplayEvent doesn't have a dedicated Tackle variant
        // Using Run with run_purpose="tackle" or "tackle_failed"
        self.events.push(ReplayEvent::Run {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            from: at,
            to: at,
            distance_m: 0.0,
            speed_mps: None,
            with_ball: false,
            pace_skill: None,
            stamina: None,
            condition: None,
            run_purpose: Some(if success { "tackle" } else { "tackle_failed" }.to_string()),
            sprint_intensity: None,
            tactical_value: None,
            off_the_ball: None,
            work_rate: None,
        });
    }

    /// Record foul event
    pub fn record_foul(&mut self, t_seconds: f64, team_id: u32, player_id: u32, at: MeterPos) {
        self.events.push(ReplayEvent::Foul {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            at,
            foul_type: None,
            severity: None,
            intentional: None,
            location_danger: None,
            aggression_level: None,
        });
        self.timeline.push(ReplayTimelineEntry {
            t: t_seconds,
            label: "FOUL".to_string(),
            team_id: Some(team_id),
            player_id: Some(player_id),
        });
    }

    /// Record dribble event
    pub fn record_dribble(
        &mut self,
        t_seconds: f64,
        team_id: u32,
        player_id: u32,
        from: MeterPos,
        to: MeterPos,
    ) {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let distance_m = (dx * dx + dy * dy).sqrt();
        self.events.push(ReplayEvent::Dribble {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            from,
            to,
            distance_m,
            touches: None,
            success: None,
            opponents_evaded: None,
            space_gained: None,
            pressure_level: None,
            dribbling_skill: None,
            agility: None,
            balance: None,
            close_control: None,
        });
    }

    /// Record boundary (out of bounds) event
    pub fn record_boundary(&mut self, t_seconds: f64, position: MeterPos) {
        self.events.push(ReplayEvent::Boundary {
            base: self.make_base(t_seconds, None, None),
            position,
            last_touch_player_id: None,
            last_touch_team_id: None,
        });
    }

    /// Record boundary with last touch info
    pub fn record_boundary_with_touch(
        &mut self,
        t_seconds: f64,
        position: MeterPos,
        last_touch_player_id: Option<u32>,
        last_touch_team_id: Option<u32>,
    ) {
        self.events.push(ReplayEvent::Boundary {
            base: self.make_base(t_seconds, None, None),
            position,
            last_touch_player_id,
            last_touch_team_id,
        });
    }

    /// Record card event
    pub fn record_card(&mut self, t_seconds: f64, team_id: u32, player_id: u32, is_red: bool) {
        self.events.push(ReplayEvent::Card {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            card_type: if is_red { CardType::Red } else { CardType::Yellow },
            yellow_count: None,
            from_second_yellow: None,
        });
        self.timeline.push(ReplayTimelineEntry {
            t: t_seconds,
            label: if is_red { "RED CARD" } else { "YELLOW CARD" }.to_string(),
            team_id: Some(team_id),
            player_id: Some(player_id),
        });
    }

    /// Record free kick event
    pub fn record_free_kick(&mut self, t_seconds: f64, team_id: u32, spot: MeterPos) {
        self.events.push(ReplayEvent::FreeKick {
            base: self.make_base(t_seconds, None, Some(team_id)),
            spot,
        });
    }

    /// Record corner kick event
    pub fn record_corner_kick(&mut self, t_seconds: f64, team_id: u32, spot: MeterPos) {
        self.events.push(ReplayEvent::CornerKick {
            base: self.make_base(t_seconds, None, Some(team_id)),
            spot,
        });
    }

    /// Record throw-in event
    pub fn record_throw_in(
        &mut self,
        t_seconds: f64,
        team_id: u32,
        player_id: u32,
        from: MeterPos,
        to: MeterPos,
    ) {
        self.events.push(ReplayEvent::Throw {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            from,
            to,
        });
    }

    /// Record half-time event
    pub fn record_half_time(&mut self, t_seconds: f64) {
        self.events.push(ReplayEvent::HalfTime { base: self.make_base(t_seconds, None, None) });
        self.timeline.push(ReplayTimelineEntry {
            t: t_seconds,
            label: "HALF TIME".to_string(),
            team_id: None,
            player_id: None,
        });
    }

    /// Record full-time event
    pub fn record_full_time(&mut self, t_seconds: f64) {
        self.events.push(ReplayEvent::FullTime { base: self.make_base(t_seconds, None, None) });
        self.timeline.push(ReplayTimelineEntry {
            t: t_seconds,
            label: "FULL TIME".to_string(),
            team_id: None,
            player_id: None,
        });
    }

    /// Record offside event
    pub fn record_offside(&mut self, t_seconds: f64, team_id: u32, player_id: u32, at: MeterPos) {
        self.events.push(ReplayEvent::Offside {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            at,
        });
    }

    /// Record penalty event
    pub fn record_penalty(&mut self, t_seconds: f64, team_id: u32, at: MeterPos, scored: bool) {
        self.events.push(ReplayEvent::Penalty {
            base: self.make_base(t_seconds, None, Some(team_id)),
            at,
            scored,
        });
        self.timeline.push(ReplayTimelineEntry {
            t: t_seconds,
            label: if scored { "PENALTY GOAL" } else { "PENALTY MISS" }.to_string(),
            team_id: Some(team_id),
            player_id: None,
        });
    }

    /// Record substitution event
    pub fn record_substitution(
        &mut self,
        t_seconds: f64,
        team_id: u32,
        out_player_id: u32,
        in_player_id: u32,
    ) {
        self.events.push(ReplayEvent::Substitution {
            base: self.make_base(t_seconds, Some(out_player_id), Some(team_id)),
            in_player_id: Some(in_player_id),
        });
    }

    /// Record possession change event (0108: Open-Football Integration)
    ///
    /// Tracks when ball possession changes between teams/players for analytics
    pub fn record_possession(
        &mut self,
        t_seconds: f64,
        new_team_id: u32,
        new_player_id: u32,
        at: MeterPos,
        change_type: PossessionChangeType,
        prev_owner_id: Option<u32>,
        prev_team_id: Option<u32>,
    ) {
        self.events.push(ReplayEvent::Possession {
            base: self.make_base(t_seconds, Some(new_player_id), Some(new_team_id)),
            at,
            change_type,
            prev_owner_id,
            prev_team_id,
        });
    }

    /// Record player decision event (0108: Decision Intent Logging)
    ///
    /// Records the action a player decided to take, useful for debugging and analysis
    pub fn record_decision(
        &mut self,
        t_seconds: f64,
        team_id: u32,
        player_id: u32,
        at: MeterPos,
        action: String,
        utility: Option<f32>,
    ) {
        self.events.push(ReplayEvent::Decision {
            base: self.make_base(t_seconds, Some(player_id), Some(team_id)),
            at,
            action,
            utility,
        });
    }

    // ========================================
    // Conversion Methods
    // ========================================

    /// Convert to ReplayDoc (consuming self)
    pub fn into_doc(self, version: u32) -> ReplayDoc {
        ReplayDoc {
            pitch_m: self.pitch,
            events: self.events,
            version,
            rosters: self.rosters,
            timeline: self.timeline,
            tactics: self.tactics,
        }
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get timeline count
    pub fn timeline_count(&self) -> usize {
        self.timeline.len()
    }

    /// Set tactics info
    pub fn set_tactics(&mut self, tactics: ReplayTeamsTactics) {
        self.tactics = tactics;
    }
}

// ========================================
// Helper Types
// ========================================

/// Result from a single tick execution - captures what happened for replay
#[derive(Debug, Clone)]
pub enum TickResult {
    PassStarted { passer_idx: usize, target_idx: usize, from: (f32, f32), to: (f32, f32) },
    ShotTaken { shooter_idx: usize, from: (f32, f32), target: (f32, f32), xg: f32 },
    GoalScored { scorer_idx: usize, assist_idx: Option<usize> },
    SaveMade { gk_idx: usize },
    TackleSuccess { tackler_idx: usize },
    TackleFailed { tackler_idx: usize },
    FoulCommitted { fouler_idx: usize },
    CardIssued { player_idx: usize, is_red: bool },
    OutOfBounds { position: (f32, f32) },
    Dribble { player_idx: usize, from: (f32, f32), to: (f32, f32) },
    KickOff { team_id: u32, player_idx: usize },
    HalfTime,
    FullTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_recorder_creates_events() {
        let pitch = PitchSpec { width_m: 105.0, height_m: 68.0 };
        let rosters = ReplayRosters::default();
        let mut recorder = ReplayRecorder::new(pitch, rosters);

        recorder.record_kickoff(0.0, 0, 10);
        recorder.record_pass(
            1.5,
            0,
            10,
            MeterPos { x: 52.5, y: 34.0 },
            MeterPos { x: 60.0, y: 40.0 },
            Some(9),
            10.0,
        );
        recorder.record_shot(
            5.0,
            0,
            9,
            MeterPos { x: 85.0, y: 34.0 },
            MeterPos { x: 105.0, y: 34.0 },
            true,
            Some(0.25),
        );
        recorder.record_goal(5.5, 0, 9, MeterPos { x: 105.0, y: 34.0 }, Some(10));

        assert_eq!(recorder.event_count(), 4);
        assert!(recorder.timeline.iter().any(|e| e.label == "GOAL"));
    }

    #[test]
    fn test_into_doc() {
        let pitch = PitchSpec { width_m: 105.0, height_m: 68.0 };
        let rosters = ReplayRosters::default();
        let mut recorder = ReplayRecorder::new(pitch, rosters);

        recorder.record_kickoff(0.0, 0, 10);
        recorder.record_half_time(45.0 * 60.0);
        recorder.record_full_time(90.0 * 60.0);

        let doc = recorder.into_doc(1);

        assert_eq!(doc.version, 1);
        assert_eq!(doc.events.len(), 3);
        assert_eq!(doc.timeline.len(), 3);
        assert_eq!(doc.pitch_m.width_m, 105.0);
    }

    #[test]
    fn test_tackle_recording() {
        let pitch = PitchSpec { width_m: 105.0, height_m: 68.0 };
        let mut recorder = ReplayRecorder::new(pitch, ReplayRosters::default());

        recorder.record_tackle(10.0, 0, 5, MeterPos { x: 50.0, y: 34.0 }, true);
        recorder.record_tackle(15.0, 1, 15, MeterPos { x: 60.0, y: 30.0 }, false);

        assert_eq!(recorder.event_count(), 2);

        // Check that tackles are recorded as Run events with purpose
        if let ReplayEvent::Run { run_purpose, .. } = &recorder.events[0] {
            assert_eq!(run_purpose.as_deref(), Some("tackle"));
        } else {
            panic!("Expected Run event");
        }

        if let ReplayEvent::Run { run_purpose, .. } = &recorder.events[1] {
            assert_eq!(run_purpose.as_deref(), Some("tackle_failed"));
        } else {
            panic!("Expected Run event");
        }
    }

    #[test]
    fn test_card_recording() {
        let pitch = PitchSpec { width_m: 105.0, height_m: 68.0 };
        let mut recorder = ReplayRecorder::new(pitch, ReplayRosters::default());

        recorder.record_card(20.0, 0, 5, false); // Yellow
        recorder.record_card(30.0, 1, 15, true); // Red

        assert_eq!(recorder.event_count(), 2);
        assert_eq!(recorder.timeline_count(), 2);

        assert!(recorder.timeline.iter().any(|e| e.label == "YELLOW CARD"));
        assert!(recorder.timeline.iter().any(|e| e.label == "RED CARD"));
    }
}
