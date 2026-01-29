//! Test Fixtures Module
//!
//! Centralized test helper functions for MatchEngine tests.
//! Eliminates code duplication across test files.
//!
//! ## Usage
//! ```rust
//! #[cfg(test)]
//! use crate::engine::match_sim::test_fixtures::*;
//! ```

use crate::models::player::PlayerAttributes;
use crate::models::team::Formation;
use crate::models::{Player, Position, Team};
use crate::player::PersonalityArchetype;

// =============================================================================
// Team Creation Helpers
// =============================================================================

/// Create a test team with 11 players using standard 4-4-2 formation.
///
/// Each player has:
/// - Position-appropriate role
/// - Overall rating of 70
/// - Default attributes (all 10s)
/// - Default personality
pub fn create_test_team(name: &str) -> Team {
    create_test_team_with_overall(name, 70)
}

/// Create a test team with specified overall rating for all players.
pub fn create_test_team_with_overall(name: &str, overall: u8) -> Team {
    let positions = standard_442_positions();
    let mut players = Vec::with_capacity(11);

    for (i, &pos) in positions.iter().enumerate() {
        players.push(Player {
            name: format!("{} Player {}", name, i + 1),
            position: pos,
            overall,
            condition: 3,
            attributes: Some(default_attributes()),
            equipped_skills: Vec::new(),
            traits: Default::default(),
            personality: PersonalityArchetype::default(),
        });
    }

    Team { name: name.to_string(), formation: Formation::F442, players }
}

/// Create a test team with custom attributes for specific mental stats.
///
/// Useful for testing audacity, decision-making, etc.
pub fn create_test_team_with_mental(name: &str, flair: u8, aggression: u8, decisions: u8) -> Team {
    let positions = standard_442_positions();
    let mut players = Vec::with_capacity(11);

    for (i, &pos) in positions.iter().enumerate() {
        let mut attrs = default_attributes();
        attrs.flair = flair;
        attrs.aggression = aggression;
        attrs.decisions = decisions;

        players.push(Player {
            name: format!("{} Player {}", name, i + 1),
            position: pos,
            overall: 70,
            condition: 3,
            attributes: Some(attrs),
            equipped_skills: Vec::new(),
            traits: Default::default(),
            personality: PersonalityArchetype::default(),
        });
    }

    Team { name: name.to_string(), formation: Formation::F442, players }
}

/// Create a test team with 11 starters + 7 subs (18 players total).
///
/// Subs have overall rating of 65.
pub fn create_test_team_with_subs(name: &str) -> Team {
    let starting_positions = standard_442_positions();
    let mut players = Vec::with_capacity(18);

    // Create starting 11
    for (i, &pos) in starting_positions.iter().enumerate() {
        players.push(Player {
            name: format!("{} Player {}", name, i + 1),
            position: pos,
            overall: 70,
            condition: 3,
            attributes: Some(default_attributes()),
            equipped_skills: Vec::new(),
            traits: Default::default(),
            personality: PersonalityArchetype::default(),
        });
    }

    // Create 7 subs
    let sub_positions = [
        Position::GK,
        Position::CB,
        Position::CM,
        Position::CM,
        Position::ST,
        Position::LW,
        Position::RW,
    ];
    for (i, &pos) in sub_positions.iter().enumerate() {
        players.push(Player {
            name: format!("{} Sub {}", name, i + 1),
            position: pos,
            overall: 65,
            condition: 3,
            attributes: Some(default_attributes()),
            equipped_skills: Vec::new(),
            traits: Default::default(),
            personality: PersonalityArchetype::default(),
        });
    }

    Team { name: name.to_string(), formation: Formation::F442, players }
}

// =============================================================================
// Engine Creation Helpers
// =============================================================================

/// Create a basic test MatchEngine with two teams (initialized).
///
/// - Home team: "Home" with 4-4-2
/// - Away team: "Away" with 4-3-3
/// - Seed: 12345
/// - No user player
pub fn create_test_engine() -> super::MatchEngine {
    let mut engine = create_test_engine_uninit();
    engine.init();
    engine
}

/// Create a test MatchEngine without calling init().
///
/// Useful for tests that need to set up state before initialization.
pub fn create_test_engine_uninit() -> super::MatchEngine {
    let plan = super::MatchPlan {
        home_team: create_test_team("Home"),
        away_team: create_test_team("Away"),
        seed: 12345,
        user_player: None,
        home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        home_instructions: None,
        away_instructions: None,
        home_player_instructions: None,
        away_player_instructions: None,
        home_ai_difficulty: None,
        away_ai_difficulty: None,
    };

    super::MatchEngine::new(plan).expect("match engine init")
}

/// Create a test engine with custom mental attributes for home team.
pub fn create_test_engine_with_mental(
    home_flair: u8,
    home_aggression: u8,
    home_decisions: u8,
) -> super::MatchEngine {
    let home = create_test_team_with_mental("Home", home_flair, home_aggression, home_decisions);
    let away = create_test_team_with_mental("Away", 10, 10, 10);

    let plan = super::MatchPlan {
        home_team: home,
        away_team: away,
        seed: 12345,
        user_player: None,
        home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        home_instructions: None,
        away_instructions: None,
        home_player_instructions: None,
        away_player_instructions: None,
        home_ai_difficulty: None,
        away_ai_difficulty: None,
    };

    let mut engine = super::MatchEngine::new(plan).expect("match engine init");
    engine.init();
    engine
}

/// Create a test engine with custom attributes for a specific home player.
///
/// # Arguments
/// - `track_id`: Player index (0-10 for home team)
/// - `attrs`: Custom PlayerAttributes for that player
pub fn create_test_engine_with_player_attrs(
    track_id: usize,
    attrs: PlayerAttributes,
) -> super::MatchEngine {
    assert!(track_id < 11, "expected home starter track_id 0-10, got {}", track_id);

    let mut home_team = create_test_team("Home");
    home_team.players[track_id].attributes = Some(attrs);

    let plan = super::MatchPlan {
        home_team,
        away_team: create_test_team("Away"),
        seed: 12345,
        user_player: None,
        home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
        home_instructions: None,
        away_instructions: None,
        home_player_instructions: None,
        away_player_instructions: None,
        home_ai_difficulty: None,
        away_ai_difficulty: None,
    };

    let mut engine = super::MatchEngine::new(plan).expect("match engine init");
    engine.init();
    engine
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Standard 4-4-2 formation positions (11 players).
fn standard_442_positions() -> [Position; 11] {
    [
        Position::GK,
        Position::LB,
        Position::CB,
        Position::CB,
        Position::RB,
        Position::LM,
        Position::CM,
        Position::CM,
        Position::RM,
        Position::ST,
        Position::ST,
    ]
}

/// Default player attributes (all stats set to 10).
///
/// Match Engine uses 0-100 scale (50 = average professional level).
/// Note: FM2023 1-20 scale is converted via fm_to_match_engine_attrs().
pub fn default_attributes() -> PlayerAttributes {
    PlayerAttributes::default()
}

/// Create attributes with specific values for key stats.
///
/// Useful for testing specific mechanics.
pub fn attrs_with_values(
    finishing: u8,
    passing: u8,
    dribbling: u8,
    tackling: u8,
    pace: u8,
) -> PlayerAttributes {
    let mut attrs = default_attributes();
    attrs.finishing = finishing;
    attrs.passing = passing;
    attrs.dribbling = dribbling;
    attrs.tackling = tackling;
    attrs.pace = pace;
    attrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_team_has_11_players() {
        let team = create_test_team("Test");
        assert_eq!(team.players.len(), 11);
    }

    #[test]
    fn test_create_test_team_all_have_attributes() {
        let team = create_test_team("Test");
        for player in &team.players {
            assert!(player.attributes.is_some());
        }
    }

    #[test]
    fn test_create_test_engine_initializes() {
        let engine = create_test_engine();
        assert_eq!(engine.player_positions.len(), 22);
    }

    #[test]
    fn test_default_attributes_all_fifties() {
        let attrs = default_attributes();
        // 0-100 scale: 50 = average professional level
        assert_eq!(attrs.passing, 50);
        assert_eq!(attrs.finishing, 50);
        assert_eq!(attrs.tackling, 50);
        assert_eq!(attrs.pace, 50);
        assert_eq!(attrs.stamina, 50);
    }
}
