//! Tactical context for match simulation
//!
//! Manages team and player instructions and calculates event probability modifiers.
//!
//! ## FIX_2601/0106 P4: Match Situation Awareness
//!
//! Adds dynamic tactical adjustments based on:
//! - Score differential (leading, trailing, tied)
//! - Match time (early, mid, late game)
//! - Dynamic pressing intensity

use crate::player::instructions::PlayerInstructions;
use crate::tactics::{BuildUpStyle, TeamInstructions};
use std::collections::HashMap;

// P17: TeamSide를 models에서 re-export
pub use crate::models::TeamSide;

// ============================================================================
// FIX_2601/0106 P4: Match Situation
// ============================================================================

/// Match time phase
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MatchTimePhase {
    /// 0-30 minutes: Teams settling into the game
    #[default]
    Early,
    /// 30-60 minutes: Main phase of play
    Mid,
    /// 60-75 minutes: Teams start tiring
    Late,
    /// 75-90+ minutes: Final push, desperation if losing
    Final,
}

impl MatchTimePhase {
    /// Create from match minute
    pub fn from_minute(minute: u32) -> Self {
        match minute {
            0..=29 => Self::Early,
            30..=59 => Self::Mid,
            60..=74 => Self::Late,
            _ => Self::Final,
        }
    }

    /// Stamina drain multiplier for this phase
    pub fn stamina_drain_mult(&self) -> f32 {
        match self {
            Self::Early => 0.9, // Players fresh
            Self::Mid => 1.0,   // Normal
            Self::Late => 1.1,  // Starting to tire
            Self::Final => 1.2, // Exhaustion kicks in
        }
    }

    /// Pressing intensity modifier
    pub fn pressing_mod(&self) -> f32 {
        match self {
            Self::Early => 1.1, // High energy pressing
            Self::Mid => 1.0,   // Normal
            Self::Late => 0.85, // Conserve energy
            Self::Final => 0.7, // Tired legs
        }
    }
}

/// Score situation from a team's perspective
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ScoreSituation {
    /// Winning by 3+ goals
    Cruising,
    /// Winning by 1-2 goals
    Leading,
    /// Score is tied
    #[default]
    Tied,
    /// Losing by 1-2 goals
    Trailing,
    /// Losing by 3+ goals
    Desperate,
}

impl ScoreSituation {
    /// Create from score differential (own_goals - opponent_goals)
    pub fn from_differential(diff: i8) -> Self {
        match diff {
            3..=i8::MAX => Self::Cruising,
            1..=2 => Self::Leading,
            0 => Self::Tied,
            -2..=-1 => Self::Trailing,
            _ => Self::Desperate,
        }
    }

    /// Pressing intensity modifier
    pub fn pressing_mod(&self) -> f32 {
        match self {
            Self::Cruising => 0.6,  // Conserve energy, game is won
            Self::Leading => 0.85,  // Protect lead
            Self::Tied => 1.0,      // Normal
            Self::Trailing => 1.15, // Push for equalizer
            Self::Desperate => 1.3, // All-out attack
        }
    }

    /// Risk-taking modifier (affects through balls, dribbles, shots)
    pub fn risk_mod(&self) -> f32 {
        match self {
            Self::Cruising => 0.5,  // Play safe
            Self::Leading => 0.7,   // Moderate caution
            Self::Tied => 1.0,      // Normal
            Self::Trailing => 1.2,  // Take more chances
            Self::Desperate => 1.5, // Maximum risk
        }
    }

    /// Defensive line modifier (positive = higher line)
    pub fn defensive_line_mod(&self) -> f32 {
        match self {
            Self::Cruising => -0.15, // Drop deep
            Self::Leading => -0.08,  // Slightly deeper
            Self::Tied => 0.0,       // Normal
            Self::Trailing => 0.1,   // Push up
            Self::Desperate => 0.2,  // Very high line
        }
    }
}

/// Current match situation for dynamic tactical adjustments
///
/// FIX_2601/0106 P4: Combines time, score, and momentum for tactical decisions
#[derive(Clone, Copy, Debug, Default)]
pub struct MatchSituation {
    /// Current minute of the match
    pub minute: u32,
    /// Time phase
    pub time_phase: MatchTimePhase,
    /// Home team score situation
    pub home_situation: ScoreSituation,
    /// Away team score situation
    pub away_situation: ScoreSituation,
    /// Recent possession percentage (0.0-1.0, home perspective)
    pub recent_possession: f32,
    /// Shots in last 10 minutes (home, away)
    pub recent_shots: (u8, u8),
}

impl MatchSituation {
    /// Create from current match state
    pub fn new(minute: u32, home_score: u8, away_score: u8) -> Self {
        let diff = home_score as i8 - away_score as i8;
        Self {
            minute,
            time_phase: MatchTimePhase::from_minute(minute),
            home_situation: ScoreSituation::from_differential(diff),
            away_situation: ScoreSituation::from_differential(-diff),
            recent_possession: 0.5,
            recent_shots: (0, 0),
        }
    }

    /// Update with new minute
    pub fn update_minute(&mut self, minute: u32) {
        self.minute = minute;
        self.time_phase = MatchTimePhase::from_minute(minute);
    }

    /// Update score
    pub fn update_score(&mut self, home_score: u8, away_score: u8) {
        let diff = home_score as i8 - away_score as i8;
        self.home_situation = ScoreSituation::from_differential(diff);
        self.away_situation = ScoreSituation::from_differential(-diff);
    }

    /// Get score situation for a team
    pub fn get_situation(&self, side: TeamSide) -> ScoreSituation {
        match side {
            TeamSide::Home => self.home_situation,
            TeamSide::Away => self.away_situation,
        }
    }

    /// Get combined pressing modifier for a team
    pub fn get_pressing_modifier(&self, side: TeamSide) -> f32 {
        let score_mod = self.get_situation(side).pressing_mod();
        let time_mod = self.time_phase.pressing_mod();
        (score_mod * time_mod).clamp(0.4, 1.5)
    }

    /// Get risk-taking modifier for a team
    pub fn get_risk_modifier(&self, side: TeamSide) -> f32 {
        let score_mod = self.get_situation(side).risk_mod();
        // In final phase, increase risk even more if trailing
        let time_boost = if self.time_phase == MatchTimePhase::Final {
            match self.get_situation(side) {
                ScoreSituation::Trailing => 1.15,
                ScoreSituation::Desperate => 1.25,
                _ => 1.0,
            }
        } else {
            1.0
        };
        (score_mod * time_boost).clamp(0.3, 2.0)
    }

    /// Get defensive line adjustment for a team
    pub fn get_defensive_line_adjustment(&self, side: TeamSide) -> f32 {
        self.get_situation(side).defensive_line_mod()
    }

    /// Is this a high-pressure situation (team needs to score)?
    pub fn is_high_pressure(&self, side: TeamSide) -> bool {
        matches!(self.get_situation(side), ScoreSituation::Trailing | ScoreSituation::Desperate)
            && matches!(self.time_phase, MatchTimePhase::Late | MatchTimePhase::Final)
    }

    /// Is this a time-wasting situation (team protecting lead)?
    pub fn should_slow_tempo(&self, side: TeamSide) -> bool {
        matches!(self.get_situation(side), ScoreSituation::Leading | ScoreSituation::Cruising)
            && matches!(self.time_phase, MatchTimePhase::Late | MatchTimePhase::Final)
    }
}

/// Event types that can be modified by tactics
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TacticalEventType {
    Interception,
    LongPass,
    ShortPass,
    Cross,
    ThroughBall,
    CounterAttack,
    Shot,
    Dribble,
    Tackle,
    Header,
}

/// Tactical context containing all instruction data for a match
///
/// FIX_2601/0106 P4: Now includes MatchSituation for dynamic adjustments
#[derive(Clone, Debug, Default)]
pub struct TacticalContext {
    pub home_instructions: TeamInstructions,
    pub away_instructions: TeamInstructions,
    pub home_player_instructions: HashMap<String, PlayerInstructions>,
    pub away_player_instructions: HashMap<String, PlayerInstructions>,
    /// FIX_2601/0106 P4: Match situation for dynamic tactics
    pub match_situation: MatchSituation,
}

impl TacticalContext {
    /// Create a new tactical context
    pub fn new(home_instructions: TeamInstructions, away_instructions: TeamInstructions) -> Self {
        Self {
            home_instructions,
            away_instructions,
            home_player_instructions: HashMap::new(),
            away_player_instructions: HashMap::new(),
            match_situation: MatchSituation::default(),
        }
    }

    /// Create with player instructions
    pub fn with_player_instructions(
        home_instructions: TeamInstructions,
        away_instructions: TeamInstructions,
        home_player_instructions: HashMap<String, PlayerInstructions>,
        away_player_instructions: HashMap<String, PlayerInstructions>,
    ) -> Self {
        Self {
            home_instructions,
            away_instructions,
            home_player_instructions,
            away_player_instructions,
            match_situation: MatchSituation::default(),
        }
    }

    /// FIX_2601/0106 P4: Update match situation
    pub fn update_situation(&mut self, minute: u32, home_score: u8, away_score: u8) {
        self.match_situation.update_minute(minute);
        self.match_situation.update_score(home_score, away_score);
    }

    /// FIX_2601/0106 P4: Get combined event modifier (base tactics + situation)
    pub fn get_situational_event_modifier(
        &self,
        team_side: TeamSide,
        event_type: TacticalEventType,
    ) -> f32 {
        let base_mod = self.get_event_modifier(team_side, event_type);
        let situation = &self.match_situation;

        // Apply situational modifiers based on event type
        let situational_mod = match event_type {
            TacticalEventType::Shot
            | TacticalEventType::ThroughBall
            | TacticalEventType::Dribble => situation.get_risk_modifier(team_side),
            TacticalEventType::Tackle | TacticalEventType::Interception => {
                situation.get_pressing_modifier(team_side)
            }
            _ => 1.0,
        };

        (base_mod * situational_mod).clamp(0.3, 2.0)
    }

    /// Get team instructions by side
    pub fn get_team_instructions(&self, side: TeamSide) -> &TeamInstructions {
        match side {
            TeamSide::Home => &self.home_instructions,
            TeamSide::Away => &self.away_instructions,
        }
    }

    /// Get player instructions by side
    pub fn get_player_instructions(&self, side: TeamSide) -> &HashMap<String, PlayerInstructions> {
        match side {
            TeamSide::Home => &self.home_player_instructions,
            TeamSide::Away => &self.away_player_instructions,
        }
    }

    /// Calculate event probability modifier based on team tactics
    ///
    /// Returns a multiplier (typically 0.5 to 1.5) that adjusts the base probability
    /// of an event occurring based on the team's tactical instructions.
    pub fn get_event_modifier(&self, team_side: TeamSide, event_type: TacticalEventType) -> f32 {
        let instructions = self.get_team_instructions(team_side);

        match event_type {
            TacticalEventType::Interception => {
                // High pressing increases interception probability
                let pressing_mod = instructions.pressing_intensity.to_numeric() as f32 * 0.03;
                // High defensive line also helps intercept
                let line_mod = instructions.defensive_line.to_numeric() as f32 * 0.015;
                (1.0 + pressing_mod + line_mod).clamp(0.7, 1.5)
            }

            TacticalEventType::LongPass => {
                // Direct build-up increases long pass frequency
                let build_mod = match instructions.build_up_style {
                    BuildUpStyle::Direct => 0.4,
                    BuildUpStyle::Mixed => 0.0,
                    BuildUpStyle::Short => -0.4,
                };
                // Fast tempo also encourages long passes
                let tempo_mod = instructions.team_tempo.to_numeric() as f32 * 0.02;
                (1.0 + build_mod + tempo_mod).clamp(0.5, 1.6)
            }

            TacticalEventType::ShortPass => {
                // Short build-up increases short pass frequency
                let build_mod = match instructions.build_up_style {
                    BuildUpStyle::Short => 0.3,
                    BuildUpStyle::Mixed => 0.0,
                    BuildUpStyle::Direct => -0.3,
                };
                // Slow tempo encourages patient passing
                let tempo_mod = -instructions.team_tempo.to_numeric() as f32 * 0.015;
                (1.0 + build_mod + tempo_mod).clamp(0.6, 1.4)
            }

            TacticalEventType::Cross => {
                // Wide formation increases crossing
                let width_mod = instructions.team_width.to_numeric() as f32 * 0.04;
                // Direct build-up also encourages crosses
                let build_mod = match instructions.build_up_style {
                    BuildUpStyle::Direct => 0.1,
                    BuildUpStyle::Mixed => 0.0,
                    BuildUpStyle::Short => -0.1,
                };
                (1.0 + width_mod + build_mod).clamp(0.5, 1.5)
            }

            TacticalEventType::ThroughBall => {
                // Fast tempo increases through ball attempts
                let tempo_mod = instructions.team_tempo.to_numeric() as f32 * 0.025;
                // High defensive line (pushing up) creates space behind
                let line_mod = instructions.defensive_line.to_numeric() as f32 * 0.02;
                (1.0 + tempo_mod + line_mod).clamp(0.6, 1.5)
            }

            TacticalEventType::CounterAttack => {
                // Deep defensive line + fast tempo = counter attacking
                let line_mod = -instructions.defensive_line.to_numeric() as f32 * 0.025;
                let tempo_mod = instructions.team_tempo.to_numeric() as f32 * 0.03;
                // Direct build-up helps counters
                let build_mod = match instructions.build_up_style {
                    BuildUpStyle::Direct => 0.15,
                    BuildUpStyle::Mixed => 0.0,
                    BuildUpStyle::Short => -0.1,
                };
                (1.0 + line_mod + tempo_mod + build_mod).clamp(0.5, 1.6)
            }

            TacticalEventType::Shot => {
                // Fast tempo increases shot attempts
                let tempo_mod = instructions.team_tempo.to_numeric() as f32 * 0.02;
                // High pressing creates more chances
                let pressing_mod = instructions.pressing_intensity.to_numeric() as f32 * 0.015;
                (1.0 + tempo_mod + pressing_mod).clamp(0.8, 1.3)
            }

            TacticalEventType::Dribble => {
                // Narrow width encourages central dribbling
                let width_mod = -instructions.team_width.to_numeric() as f32 * 0.02;
                // Slow tempo allows more dribbling time
                let tempo_mod = -instructions.team_tempo.to_numeric() as f32 * 0.015;
                (1.0 + width_mod + tempo_mod).clamp(0.7, 1.4)
            }

            TacticalEventType::Tackle => {
                // High pressing increases tackle attempts
                let pressing_mod = instructions.pressing_intensity.to_numeric() as f32 * 0.035;
                // Narrow width increases central tackles
                let width_mod = -instructions.team_width.to_numeric() as f32 * 0.01;
                (1.0 + pressing_mod + width_mod).clamp(0.7, 1.5)
            }

            TacticalEventType::Header => {
                // Wide formation + direct play = more headers
                let width_mod = instructions.team_width.to_numeric() as f32 * 0.03;
                let build_mod = match instructions.build_up_style {
                    BuildUpStyle::Direct => 0.2,
                    BuildUpStyle::Mixed => 0.0,
                    BuildUpStyle::Short => -0.15,
                };
                (1.0 + width_mod + build_mod).clamp(0.6, 1.5)
            }
        }
    }

    /// Calculate stamina drain modifier based on tactics
    ///
    /// High pressing and fast tempo drain stamina faster.
    pub fn get_stamina_drain_modifier(&self, team_side: TeamSide) -> f32 {
        let instructions = self.get_team_instructions(team_side);

        let tempo_drain = instructions.team_tempo.stamina_drain_modifier();
        let pressing_drain = instructions.pressing_intensity.stamina_cost_modifier();

        // Average the two modifiers
        (tempo_drain + pressing_drain) / 2.0
    }

    /// Calculate overall team strength modifier
    ///
    /// This is the combined effect of all tactical settings on team performance.
    pub fn get_team_strength_modifier(&self, team_side: TeamSide) -> f32 {
        let instructions = self.get_team_instructions(team_side);

        let defensive_line_mod = instructions.defensive_line.to_numeric() as f32 * 0.008;
        let tempo_mod = instructions.team_tempo.to_numeric() as f32 * 0.01;
        let pressing_mod = instructions.pressing_intensity.to_numeric() as f32 * 0.008;
        let width_mod = instructions.team_width.to_numeric() as f32 * 0.005;
        let build_mod = match instructions.build_up_style {
            BuildUpStyle::Short => -0.005,
            BuildUpStyle::Mixed => 0.0,
            BuildUpStyle::Direct => 0.01,
        };

        (1.0 + defensive_line_mod + tempo_mod + pressing_mod + width_mod + build_mod)
            .clamp(0.85, 1.15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tactics::team_instructions::TeamWidth;
    use crate::tactics::TacticalPreset;

    #[test]
    fn test_high_pressing_increases_interception() {
        let high_pressing = TeamInstructions::for_style(TacticalPreset::HighPressing);
        let context = TacticalContext::new(high_pressing, TeamInstructions::default());

        let modifier = context.get_event_modifier(TeamSide::Home, TacticalEventType::Interception);
        assert!(modifier > 1.0, "High pressing should increase interception: {}", modifier);
    }

    #[test]
    fn test_direct_buildup_increases_long_pass() {
        let mut instructions = TeamInstructions::default();
        instructions.build_up_style = BuildUpStyle::Direct;
        let context = TacticalContext::new(instructions, TeamInstructions::default());

        let modifier = context.get_event_modifier(TeamSide::Home, TacticalEventType::LongPass);
        assert!(modifier > 1.3, "Direct buildup should increase long pass: {}", modifier);
    }

    #[test]
    fn test_short_buildup_decreases_long_pass() {
        let mut instructions = TeamInstructions::default();
        instructions.build_up_style = BuildUpStyle::Short;
        let context = TacticalContext::new(instructions, TeamInstructions::default());

        let modifier = context.get_event_modifier(TeamSide::Home, TacticalEventType::LongPass);
        assert!(modifier < 0.7, "Short buildup should decrease long pass: {}", modifier);
    }

    #[test]
    fn test_wide_formation_increases_crosses() {
        let mut instructions = TeamInstructions::default();
        instructions.team_width = TeamWidth::VeryWide;
        let context = TacticalContext::new(instructions, TeamInstructions::default());

        let modifier = context.get_event_modifier(TeamSide::Home, TacticalEventType::Cross);
        assert!(modifier > 1.0, "Wide formation should increase crosses: {}", modifier);
    }

    #[test]
    fn test_counter_attack_tactics() {
        let counter = TeamInstructions::for_style(TacticalPreset::Counterattack);
        let context = TacticalContext::new(counter, TeamInstructions::default());

        let modifier = context.get_event_modifier(TeamSide::Home, TacticalEventType::CounterAttack);
        assert!(modifier > 1.2, "Counter tactics should increase counter attack: {}", modifier);
    }

    #[test]
    fn test_stamina_drain_high_pressing() {
        let high_pressing = TeamInstructions::for_style(TacticalPreset::HighPressing);
        let balanced = TeamInstructions::default();
        let context = TacticalContext::new(high_pressing, balanced);

        let home_drain = context.get_stamina_drain_modifier(TeamSide::Home);
        let away_drain = context.get_stamina_drain_modifier(TeamSide::Away);

        assert!(home_drain > away_drain, "High pressing should drain more stamina");
    }

    #[test]
    fn test_team_strength_modifier_bounds() {
        let extreme = TeamInstructions::for_style(TacticalPreset::HighPressing);
        let context = TacticalContext::new(extreme, TeamInstructions::default());

        let modifier = context.get_team_strength_modifier(TeamSide::Home);
        assert!((0.85..=1.15).contains(&modifier), "Modifier should be bounded: {}", modifier);
    }
}
