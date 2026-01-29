//! # Event-Stat Consistency Checker
//!
//! Layer 3 of Football Likeness QA - validates event counts match statistics.
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: REALTIME_SYSTEMS_ANALYSIS.md (Football Likeness QA)

use crate::models::replay::events::{
    Event, PassOutcome, ShotOutcome, CardType, SetPieceKind,
};
use crate::calibration::stat_snapshot::MatchStatSnapshot;

/// Consistency check result for a single stat type.
#[derive(Debug, Clone)]
pub struct StatConsistency {
    /// Name of the statistic
    pub stat_name: String,
    /// Count from events
    pub event_count: u32,
    /// Count from statistics
    pub stat_count: u32,
    /// Whether they match
    pub matches: bool,
    /// Difference (stat_count - event_count)
    pub difference: i32,
}

impl StatConsistency {
    /// Create a new consistency check result.
    pub fn new(stat_name: &str, event_count: u32, stat_count: u32) -> Self {
        Self {
            stat_name: stat_name.to_string(),
            event_count,
            stat_count,
            matches: event_count == stat_count,
            difference: stat_count as i32 - event_count as i32,
        }
    }

    /// Check with tolerance (for stats that may have counting ambiguity).
    pub fn matches_with_tolerance(&self, tolerance: u32) -> bool {
        self.difference.unsigned_abs() <= tolerance
    }
}

/// Full consistency report for a match.
#[derive(Debug, Clone, Default)]
pub struct ConsistencyReport {
    /// Individual stat checks
    pub checks: Vec<StatConsistency>,
    /// Overall consistency score (0-100)
    pub consistency_score: f32,
    /// List of mismatches for reporting
    pub mismatches: Vec<String>,
    /// Critical mismatches (must be zero for pass)
    pub critical_mismatches: Vec<String>,
}

impl ConsistencyReport {
    /// Whether all critical stats match.
    pub fn all_critical_match(&self) -> bool {
        self.critical_mismatches.is_empty()
    }

    /// Add a consistency check.
    pub fn add_check(&mut self, stat_name: &str, event_count: u32, stat_count: u32, critical: bool) {
        let check = StatConsistency::new(stat_name, event_count, stat_count);
        if !check.matches {
            let msg = format!(
                "{}: events={}, stats={} (diff={})",
                stat_name, event_count, stat_count, check.difference
            );
            self.mismatches.push(msg.clone());
            if critical {
                self.critical_mismatches.push(msg);
            }
        }
        self.checks.push(check);
    }

    /// Add a check with tolerance.
    pub fn add_check_with_tolerance(
        &mut self,
        stat_name: &str,
        event_count: u32,
        stat_count: u32,
        tolerance: u32,
        critical: bool,
    ) {
        let check = StatConsistency::new(stat_name, event_count, stat_count);
        if !check.matches_with_tolerance(tolerance) {
            let msg = format!(
                "{}: events={}, stats={} (diff={}, tol={})",
                stat_name, event_count, stat_count, check.difference, tolerance
            );
            self.mismatches.push(msg.clone());
            if critical {
                self.critical_mismatches.push(msg);
            }
        }
        self.checks.push(check);
    }

    /// Calculate overall score.
    pub fn calculate_score(&mut self) {
        if self.checks.is_empty() {
            self.consistency_score = 100.0;
            return;
        }

        // Weight critical stats more heavily
        let mut weighted_matches = 0.0;
        let mut total_weight = 0.0;

        for check in &self.checks {
            let weight = if is_critical_stat(&check.stat_name) { 2.0 } else { 1.0 };
            total_weight += weight;
            if check.matches {
                weighted_matches += weight;
            }
        }

        self.consistency_score = if total_weight > 0.0 {
            100.0 * weighted_matches / total_weight
        } else {
            100.0
        };
    }

    /// Get pass/fail status based on critical mismatches and score threshold.
    pub fn passed(&self, min_score: f32) -> bool {
        self.all_critical_match() && self.consistency_score >= min_score
    }
}

/// Check if a stat is critical (must match exactly).
fn is_critical_stat(stat_name: &str) -> bool {
    matches!(
        stat_name,
        "goals" | "shots" | "shots_on_target" | "red_cards" | "penalties"
    )
}

/// Statistics that should be validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatType {
    Goals,
    Shots,
    ShotsOnTarget,
    Passes,
    PassesCompleted,
    Tackles,
    TacklesWon,
    Fouls,
    Corners,
    YellowCards,
    RedCards,
    Saves,
    Interceptions,
}

impl StatType {
    /// Name for reporting.
    pub fn name(&self) -> &'static str {
        match self {
            StatType::Goals => "goals",
            StatType::Shots => "shots",
            StatType::ShotsOnTarget => "shots_on_target",
            StatType::Passes => "passes",
            StatType::PassesCompleted => "passes_completed",
            StatType::Tackles => "tackles",
            StatType::TacklesWon => "tackles_won",
            StatType::Fouls => "fouls",
            StatType::Corners => "corners",
            StatType::YellowCards => "yellow_cards",
            StatType::RedCards => "red_cards",
            StatType::Saves => "saves",
            StatType::Interceptions => "interceptions",
        }
    }

    /// Whether this stat must match exactly.
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            StatType::Goals | StatType::Shots | StatType::ShotsOnTarget | StatType::RedCards
        )
    }

    /// Tolerance for matching (non-critical stats may have counting differences).
    pub fn tolerance(&self) -> u32 {
        match self {
            StatType::Goals | StatType::RedCards => 0,
            StatType::Shots | StatType::ShotsOnTarget => 0,
            StatType::Passes | StatType::PassesCompleted => 2,
            StatType::Tackles | StatType::TacklesWon => 1,
            StatType::Fouls => 1,
            _ => 1,
        }
    }
}

// ============================================================================
// Event Counting Functions
// ============================================================================

/// Count events from event list.
#[derive(Debug, Clone, Default)]
pub struct EventCounts {
    pub goals: u32,
    pub shots: u32,
    pub shots_on_target: u32,
    pub passes: u32,
    pub passes_completed: u32,
    pub tackles: u32,
    pub tackles_won: u32,
    pub fouls: u32,
    pub corners: u32,
    pub yellow_cards: u32,
    pub red_cards: u32,
    pub saves: u32,
    pub interceptions: u32,
    pub penalties: u32,
}

impl EventCounts {
    /// Count all events from event list.
    pub fn from_events(events: &[Event]) -> Self {
        let mut counts = Self::default();

        for event in events {
            match event {
                Event::Shot(shot) => {
                    counts.shots += 1;
                    if shot.on_target {
                        counts.shots_on_target += 1;
                    }
                    if shot.outcome == ShotOutcome::Goal {
                        counts.goals += 1;
                    }
                }
                Event::Pass(pass) => {
                    counts.passes += 1;
                    if pass.outcome == PassOutcome::Complete {
                        counts.passes_completed += 1;
                    }
                    // Intercepted pass counts toward interceptions
                    if pass.outcome == PassOutcome::Intercepted {
                        counts.interceptions += 1;
                    }
                }
                Event::Tackle(tackle) => {
                    counts.tackles += 1;
                    if tackle.success {
                        counts.tackles_won += 1;
                    }
                }
                Event::Foul(foul) => {
                    counts.fouls += 1;
                    match foul.card {
                        CardType::Yellow => counts.yellow_cards += 1,
                        CardType::Red => counts.red_cards += 1,
                        CardType::None => {}
                    }
                }
                Event::SetPiece(sp) => {
                    match sp.kind {
                        SetPieceKind::Corner => counts.corners += 1,
                        SetPieceKind::Penalty => counts.penalties += 1,
                        _ => {}
                    }
                }
                Event::Save(_) => {
                    counts.saves += 1;
                }
                Event::Dribble(_) | Event::Substitution(_) | Event::Possession(_) => {
                    // Don't count these for consistency checking
                }
            }
        }

        counts
    }

    /// Get count for a specific stat type.
    pub fn get(&self, stat_type: StatType) -> u32 {
        match stat_type {
            StatType::Goals => self.goals,
            StatType::Shots => self.shots,
            StatType::ShotsOnTarget => self.shots_on_target,
            StatType::Passes => self.passes,
            StatType::PassesCompleted => self.passes_completed,
            StatType::Tackles => self.tackles,
            StatType::TacklesWon => self.tackles_won,
            StatType::Fouls => self.fouls,
            StatType::Corners => self.corners,
            StatType::YellowCards => self.yellow_cards,
            StatType::RedCards => self.red_cards,
            StatType::Saves => self.saves,
            StatType::Interceptions => self.interceptions,
        }
    }
}

// ============================================================================
// Main Validation Functions
// ============================================================================

/// Validate event counts match statistics.
///
/// # Arguments
/// * `events` - Match events
/// * `stats` - Match statistics snapshot
///
/// # Returns
/// ConsistencyReport with all check results
pub fn validate_event_stat_consistency(
    events: &[Event],
    stats: &MatchStatSnapshot,
) -> ConsistencyReport {
    let event_counts = EventCounts::from_events(events);
    let mut report = ConsistencyReport::default();

    // Critical checks (must match exactly)
    report.add_check("goals", event_counts.goals, stats.goals, true);
    report.add_check("shots", event_counts.shots, stats.shot_attempts, true);
    report.add_check("shots_on_target", event_counts.shots_on_target, stats.shots_on_target, true);

    // Non-critical checks (with tolerance)
    report.add_check_with_tolerance(
        "passes",
        event_counts.passes,
        stats.pass_attempts,
        2,
        false,
    );
    report.add_check_with_tolerance(
        "passes_completed",
        event_counts.passes_completed,
        stats.pass_successes,
        2,
        false,
    );
    report.add_check_with_tolerance(
        "tackles",
        event_counts.tackles,
        stats.tackles,
        1,
        false,
    );
    report.add_check_with_tolerance(
        "tackles_won",
        event_counts.tackles_won,
        stats.tackle_successes,
        1,
        false,
    );
    report.add_check_with_tolerance(
        "interceptions",
        event_counts.interceptions,
        stats.interceptions,
        2,
        false,
    );

    report.calculate_score();
    report
}

/// Validate consistency for both teams.
pub fn validate_match_consistency(
    home_events: &[Event],
    away_events: &[Event],
    home_stats: &MatchStatSnapshot,
    away_stats: &MatchStatSnapshot,
) -> (ConsistencyReport, ConsistencyReport) {
    let home_report = validate_event_stat_consistency(home_events, home_stats);
    let away_report = validate_event_stat_consistency(away_events, away_stats);
    (home_report, away_report)
}

/// Quick validation that returns overall pass/fail.
pub fn quick_validate(events: &[Event], stats: &MatchStatSnapshot) -> bool {
    let report = validate_event_stat_consistency(events, stats);
    report.passed(90.0) // Require 90% consistency score
}

/// Detailed mismatch analysis for debugging.
pub fn analyze_mismatches(events: &[Event], stats: &MatchStatSnapshot) -> Vec<String> {
    let report = validate_event_stat_consistency(events, stats);

    let mut analysis = Vec::new();

    if !report.all_critical_match() {
        analysis.push("CRITICAL MISMATCHES:".to_string());
        for mismatch in &report.critical_mismatches {
            analysis.push(format!("  - {}", mismatch));
        }
    }

    if !report.mismatches.is_empty() {
        analysis.push(format!(
            "Total mismatches: {} / {} checks",
            report.mismatches.len(),
            report.checks.len()
        ));
        analysis.push(format!("Consistency score: {:.1}%", report.consistency_score));
    }

    analysis
}

// ============================================================================
// Score and Grade
// ============================================================================

/// Consistency grade levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyGrade {
    /// Perfect consistency (100%)
    Perfect,
    /// Excellent (95-100%)
    Excellent,
    /// Good (85-95%)
    Good,
    /// Acceptable (70-85%)
    Acceptable,
    /// Poor (<70%)
    Poor,
    /// Critical failure (critical mismatch)
    Critical,
}

impl ConsistencyGrade {
    /// Get grade from report.
    pub fn from_report(report: &ConsistencyReport) -> Self {
        if !report.all_critical_match() {
            return ConsistencyGrade::Critical;
        }

        match report.consistency_score {
            s if s >= 100.0 => ConsistencyGrade::Perfect,
            s if s >= 95.0 => ConsistencyGrade::Excellent,
            s if s >= 85.0 => ConsistencyGrade::Good,
            s if s >= 70.0 => ConsistencyGrade::Acceptable,
            _ => ConsistencyGrade::Poor,
        }
    }

    /// Check if grade is passing.
    pub fn is_passing(&self) -> bool {
        matches!(
            self,
            ConsistencyGrade::Perfect
                | ConsistencyGrade::Excellent
                | ConsistencyGrade::Good
                | ConsistencyGrade::Acceptable
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::replay::events::*;
    use crate::models::replay::types::{Team, MeterPos, BallState, CurveType};

    fn make_base_event(team: Team) -> BaseEvent {
        BaseEvent::new(0.0, 0, team, "P1".to_string(), MeterPos { x: 50.0, y: 34.0 })
    }

    fn make_shot_event(outcome: ShotOutcome, on_target: bool) -> Event {
        Event::Shot(ShotEvent {
            base: make_base_event(Team::Home),
            target: MeterPos { x: 105.0, y: 34.0 },
            xg: 0.1,
            on_target,
            ball: BallState {
                from: MeterPos { x: 50.0, y: 34.0 },
                to: MeterPos { x: 105.0, y: 34.0 },
                speed_mps: 25.0,
                curve: CurveType::None,
            },
            outcome,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            finishing_skill: None,
            curve_factor: None,
        })
    }

    fn make_pass_event(outcome: PassOutcome) -> Event {
        Event::Pass(PassEvent {
            base: make_base_event(Team::Home),
            end_pos: MeterPos { x: 60.0, y: 34.0 },
            receiver_id: "P2".to_string(),
            ground: true,
            ball: BallState {
                from: MeterPos { x: 50.0, y: 34.0 },
                to: MeterPos { x: 60.0, y: 34.0 },
                speed_mps: 15.0,
                curve: CurveType::None,
            },
            outcome,
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
        })
    }

    fn make_tackle_event(success: bool) -> Event {
        Event::Tackle(TackleEvent {
            base: make_base_event(Team::Home),
            opponent_id: "A1".to_string(),
            success,
        })
    }

    fn make_foul_event(card: CardType) -> Event {
        Event::Foul(FoulEvent {
            base: make_base_event(Team::Home),
            opponent_id: "A1".to_string(),
            card,
        })
    }

    fn make_corner_event() -> Event {
        Event::SetPiece(SetPieceEvent {
            base: make_base_event(Team::Home),
            kind: SetPieceKind::Corner,
            ball: None,
        })
    }

    fn make_save_event() -> Event {
        Event::Save(SaveEvent {
            base: make_base_event(Team::Away),
            ball: BallState {
                from: MeterPos { x: 20.0, y: 34.0 },
                to: MeterPos { x: 0.0, y: 34.0 },
                speed_mps: 25.0,
                curve: CurveType::None,
            },
            parry_to: None,
            shot_from: None,
            shot_power: None,
            save_difficulty: None,
            reflexes_skill: None,
            handling_skill: None,
            diving_skill: None,
        })
    }

    #[test]
    fn test_stat_consistency() {
        let check = StatConsistency::new("goals", 2, 2);
        assert!(check.matches);
        assert_eq!(check.difference, 0);

        let check = StatConsistency::new("passes", 450, 452);
        assert!(!check.matches);
        assert!(check.matches_with_tolerance(2));
    }

    #[test]
    fn test_consistency_report() {
        let mut report = ConsistencyReport::default();
        report.add_check("goals", 2, 2, true);
        report.add_check("shots", 15, 15, true);
        report.add_check_with_tolerance("passes", 450, 452, 2, false);
        report.calculate_score();

        assert!(report.all_critical_match());
        assert!(report.mismatches.is_empty()); // Pass within tolerance doesn't count
    }

    #[test]
    fn test_critical_stats() {
        assert!(is_critical_stat("goals"));
        assert!(is_critical_stat("shots"));
        assert!(!is_critical_stat("passes"));
        assert!(!is_critical_stat("tackles"));
    }

    #[test]
    fn test_event_counts_shots() {
        let events = vec![
            make_shot_event(ShotOutcome::Goal, true),
            make_shot_event(ShotOutcome::Saved, true),
            make_shot_event(ShotOutcome::Off, false),
        ];

        let counts = EventCounts::from_events(&events);

        assert_eq!(counts.shots, 3);
        assert_eq!(counts.shots_on_target, 2);
        assert_eq!(counts.goals, 1);
    }

    #[test]
    fn test_event_counts_passes() {
        let events = vec![
            make_pass_event(PassOutcome::Complete),
            make_pass_event(PassOutcome::Complete),
            make_pass_event(PassOutcome::Intercepted),
            make_pass_event(PassOutcome::Out),
        ];

        let counts = EventCounts::from_events(&events);

        assert_eq!(counts.passes, 4);
        assert_eq!(counts.passes_completed, 2);
        assert_eq!(counts.interceptions, 1);
    }

    #[test]
    fn test_event_counts_tackles() {
        let events = vec![
            make_tackle_event(true),
            make_tackle_event(true),
            make_tackle_event(false),
        ];

        let counts = EventCounts::from_events(&events);

        assert_eq!(counts.tackles, 3);
        assert_eq!(counts.tackles_won, 2);
    }

    #[test]
    fn test_event_counts_fouls_and_cards() {
        let events = vec![
            make_foul_event(CardType::None),
            make_foul_event(CardType::Yellow),
            make_foul_event(CardType::Yellow),
            make_foul_event(CardType::Red),
        ];

        let counts = EventCounts::from_events(&events);

        assert_eq!(counts.fouls, 4);
        assert_eq!(counts.yellow_cards, 2);
        assert_eq!(counts.red_cards, 1);
    }

    #[test]
    fn test_event_counts_corners_saves() {
        let events = vec![
            make_corner_event(),
            make_corner_event(),
            make_save_event(),
        ];

        let counts = EventCounts::from_events(&events);

        assert_eq!(counts.corners, 2);
        assert_eq!(counts.saves, 1);
    }

    #[test]
    fn test_validate_consistency_perfect() {
        let events = vec![
            make_shot_event(ShotOutcome::Goal, true),
            make_shot_event(ShotOutcome::Saved, true),
            make_pass_event(PassOutcome::Complete),
            make_pass_event(PassOutcome::Complete),
            make_tackle_event(true),
        ];

        let mut stats = MatchStatSnapshot::default();
        stats.goals = 1;
        stats.shot_attempts = 2;
        stats.shots_on_target = 2;
        stats.pass_attempts = 2;
        stats.pass_successes = 2;
        stats.tackles = 1;
        stats.tackle_successes = 1;
        stats.interceptions = 0;

        let report = validate_event_stat_consistency(&events, &stats);

        assert!(report.all_critical_match());
        assert_eq!(report.consistency_score, 100.0);
    }

    #[test]
    fn test_validate_consistency_mismatch() {
        let events = vec![
            make_shot_event(ShotOutcome::Goal, true),
            make_shot_event(ShotOutcome::Saved, true),
        ];

        let mut stats = MatchStatSnapshot::default();
        stats.goals = 2; // Mismatch! Events say 1 goal
        stats.shot_attempts = 2;
        stats.shots_on_target = 2;

        let report = validate_event_stat_consistency(&events, &stats);

        assert!(!report.all_critical_match());
        assert!(report.critical_mismatches.len() > 0);
    }

    #[test]
    fn test_consistency_grade() {
        let mut report = ConsistencyReport::default();
        report.add_check("goals", 2, 2, true);
        report.add_check("shots", 10, 10, true);
        report.calculate_score();

        let grade = ConsistencyGrade::from_report(&report);
        assert_eq!(grade, ConsistencyGrade::Perfect);
        assert!(grade.is_passing());
    }

    #[test]
    fn test_consistency_grade_critical_fail() {
        let mut report = ConsistencyReport::default();
        report.add_check("goals", 2, 3, true); // Critical mismatch
        report.calculate_score();

        let grade = ConsistencyGrade::from_report(&report);
        assert_eq!(grade, ConsistencyGrade::Critical);
        assert!(!grade.is_passing());
    }

    #[test]
    fn test_quick_validate() {
        let events = vec![
            make_shot_event(ShotOutcome::Goal, true),
        ];

        let mut stats = MatchStatSnapshot::default();
        stats.goals = 1;
        stats.shot_attempts = 1;
        stats.shots_on_target = 1;

        assert!(quick_validate(&events, &stats));
    }

    #[test]
    fn test_analyze_mismatches() {
        let events = vec![
            make_shot_event(ShotOutcome::Goal, true),
        ];

        let mut stats = MatchStatSnapshot::default();
        stats.goals = 2; // Mismatch
        stats.shot_attempts = 1;
        stats.shots_on_target = 1;

        let analysis = analyze_mismatches(&events, &stats);

        assert!(analysis.iter().any(|s| s.contains("CRITICAL")));
        assert!(analysis.iter().any(|s| s.contains("goals")));
    }

    #[test]
    fn test_tolerance_within_bounds() {
        let mut report = ConsistencyReport::default();

        // 2 off is within tolerance
        report.add_check_with_tolerance("passes", 100, 102, 2, false);
        report.calculate_score();

        // No mismatch recorded
        assert!(report.mismatches.is_empty());
    }

    #[test]
    fn test_tolerance_exceeded() {
        let mut report = ConsistencyReport::default();

        // 5 off exceeds tolerance of 2
        report.add_check_with_tolerance("passes", 100, 105, 2, false);
        report.calculate_score();

        // Mismatch recorded
        assert!(!report.mismatches.is_empty());
    }
}
