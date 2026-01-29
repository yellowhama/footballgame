//! Global Game State Manager
//!
//! This module provides a thread-safe global state manager for runtime game state.
//! The `GameState` struct holds the active game data and can be converted to/from
//! `GameSave` for persistence.

use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock};

use crate::coach::{CardInventory, Deck};
use crate::player::CorePlayer;
use crate::quest::QuestManagerState;
use crate::save::{GameProgress, GameSave, GameSettings, MatchRecord};
use crate::training::session::TrainingManager;

/// Global game state singleton
pub static GAME_STATE: Lazy<Arc<RwLock<GameState>>> =
    Lazy::new(|| Arc::new(RwLock::new(GameState::default())));

/// Runtime game state
///
/// This struct holds all active game data during runtime.
/// It can be converted to `GameSave` for persistence and restored from it.
#[derive(Debug, Clone)]
pub struct GameState {
    /// Player roster - all recruited players
    pub players: Vec<CorePlayer>,

    /// Coach card collection and inventory
    pub card_inventory: CardInventory,

    /// Saved deck configurations
    pub saved_decks: Vec<Deck>,

    /// Currently active deck ID for training
    pub active_deck_id: Option<String>,

    /// Training system state
    pub training_manager: TrainingManager,

    /// Match history and results
    pub match_history: Vec<MatchRecord>,

    /// Game progress and achievements
    pub progress: GameProgress,

    /// Player preferences and settings
    pub game_settings: GameSettings,

    /// Quest system state
    pub quest_manager: QuestManagerState,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Create a new empty game state
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            card_inventory: CardInventory::new(),
            saved_decks: Vec::new(),
            active_deck_id: None,
            training_manager: TrainingManager::new(),
            match_history: Vec::new(),
            progress: GameProgress::default(),
            game_settings: GameSettings::default(),
            quest_manager: QuestManagerState::default(),
        }
    }

    /// Convert runtime state to save format
    pub fn to_save(&self) -> GameSave {
        GameSave {
            version: crate::save::SAVE_VERSION,
            timestamp: crate::save::format::current_timestamp(),
            players: self.players.clone(),
            card_inventory: self.card_inventory.clone(),
            saved_decks: self.saved_decks.clone(),
            active_deck_id: self.active_deck_id.clone(),
            training_manager: self.training_manager.clone(),
            match_history: self.match_history.clone(),
            progress: self.progress.clone(),
            game_settings: self.game_settings.clone(),
            quest_manager: self.quest_manager.clone(),
            player_appearance: None,
        }
    }

    /// Restore runtime state from save data
    pub fn from_save(save: &GameSave) -> Self {
        Self {
            players: save.players.clone(),
            card_inventory: save.card_inventory.clone(),
            saved_decks: save.saved_decks.clone(),
            active_deck_id: save.active_deck_id.clone(),
            training_manager: save.training_manager.clone(),
            match_history: save.match_history.clone(),
            progress: save.progress.clone(),
            game_settings: save.game_settings.clone(),
            quest_manager: save.quest_manager.clone(),
        }
    }

    // ========================
    // Player Management
    // ========================

    /// Add a player to the roster
    pub fn add_player(&mut self, player: CorePlayer) {
        self.players.push(player);
    }

    /// Remove a player by ID
    pub fn remove_player(&mut self, player_id: &str) -> Option<CorePlayer> {
        if let Some(idx) = self.players.iter().position(|p| p.id == player_id) {
            Some(self.players.remove(idx))
        } else {
            None
        }
    }

    /// Get a player by ID
    pub fn get_player(&self, player_id: &str) -> Option<&CorePlayer> {
        self.players.iter().find(|p| p.id == player_id)
    }

    /// Get a mutable reference to a player by ID
    pub fn get_player_mut(&mut self, player_id: &str) -> Option<&mut CorePlayer> {
        self.players.iter_mut().find(|p| p.id == player_id)
    }

    // ========================
    // Deck Management
    // ========================

    /// Set the active deck by ID
    pub fn set_active_deck(&mut self, deck_id: Option<String>) {
        self.active_deck_id = deck_id;
    }

    /// Get the active deck
    pub fn get_active_deck(&self) -> Option<&Deck> {
        self.active_deck_id.as_ref().and_then(|id| self.saved_decks.iter().find(|d| d.name == *id))
    }

    /// Save a deck configuration
    pub fn save_deck(&mut self, deck: Deck) {
        // Update existing or add new
        if let Some(existing) = self.saved_decks.iter_mut().find(|d| d.name == deck.name) {
            *existing = deck;
        } else {
            self.saved_decks.push(deck);
        }
    }

    // ========================
    // Match Recording
    // ========================

    /// Record a match result
    pub fn record_match(&mut self, record: MatchRecord) {
        // Extract info before moving
        let result = record.result.clone();
        let score_home = record.score_home;
        let score_away = record.score_away;

        self.match_history.push(record);
        self.progress.total_matches += 1;

        match result {
            crate::save::MatchResult::Win => self.progress.stats.wins += 1,
            crate::save::MatchResult::Draw => self.progress.stats.draws += 1,
            crate::save::MatchResult::Loss => self.progress.stats.losses += 1,
        }

        self.progress.stats.goals_for += score_home as u32;
        self.progress.stats.goals_against += score_away as u32;
    }

    // ========================
    // Progress Management
    // ========================

    /// Advance to next week
    pub fn advance_week(&mut self) {
        self.progress.current_week += 1;

        // Check for season change (52 weeks = 1 season)
        if self.progress.current_week > 52 {
            self.progress.current_week = 1;
            self.progress.current_season += 1;

            // Reset player season stats
            for player in &mut self.players {
                player.start_new_season();
            }
        }
    }

    /// Unlock an achievement
    pub fn unlock_achievement(&mut self, achievement_id: &str) {
        if !self.progress.achievements.contains(&achievement_id.to_string()) {
            self.progress.achievements.push(achievement_id.to_string());
        }
    }
}

// ========================
// Global State Access Functions
// ========================

/// Get a read lock on the global game state
pub fn get_state() -> std::sync::RwLockReadGuard<'static, GameState> {
    GAME_STATE.read().expect("GAME_STATE lock poisoned")
}

/// Get a write lock on the global game state
pub fn get_state_mut() -> std::sync::RwLockWriteGuard<'static, GameState> {
    GAME_STATE.write().expect("GAME_STATE lock poisoned")
}

/// Reset the global state to default
pub fn reset_state() {
    *GAME_STATE.write().expect("GAME_STATE lock poisoned") = GameState::new();
}

/// Replace the entire global state
pub fn set_state(new_state: GameState) {
    *GAME_STATE.write().expect("GAME_STATE lock poisoned") = new_state;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_state_roundtrip() {
        let mut state = GameState::new();

        // Add some data
        state.progress.current_week = 10;
        state.progress.current_season = 2;
        state.active_deck_id = Some("test_deck".to_string());

        // Convert to save
        let save = state.to_save();

        // Convert back
        let restored = GameState::from_save(&save);

        assert_eq!(restored.progress.current_week, 10);
        assert_eq!(restored.progress.current_season, 2);
        assert_eq!(restored.active_deck_id, Some("test_deck".to_string()));
    }

    #[test]
    fn test_advance_week_season_transition() {
        let mut state = GameState::new();
        state.progress.current_week = 52;
        state.progress.current_season = 1;

        state.advance_week();

        assert_eq!(state.progress.current_week, 1);
        assert_eq!(state.progress.current_season, 2);
    }

    #[test]
    fn test_achievement_deduplication() {
        let mut state = GameState::new();

        state.unlock_achievement("first_win");
        state.unlock_achievement("first_win"); // Duplicate
        state.unlock_achievement("first_title");

        assert_eq!(state.progress.achievements.len(), 2);
    }
}
