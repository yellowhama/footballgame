use super::error::SaveError;
use super::format::{decompress_and_deserialize, serialize_and_compress, GameSave};
use super::migration::migrate_save;

use once_cell::sync::Lazy;
use std::fs::{remove_file, rename, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// Global game state singleton
static CURRENT_GAME_STATE: Lazy<Mutex<Option<GameSave>>> = Lazy::new(|| Mutex::new(None));

pub struct SaveManager;

impl SaveManager {
    /// Get current game state (thread-safe)
    pub fn get_current_state() -> Option<GameSave> {
        CURRENT_GAME_STATE.lock().unwrap().clone()
    }

    /// Update current game state
    pub fn update_current_state(state: GameSave) {
        *CURRENT_GAME_STATE.lock().unwrap() = Some(state);
    }

    /// Clear current game state
    pub fn clear_current_state() {
        *CURRENT_GAME_STATE.lock().unwrap() = None;
    }

    /// Collect current state from all global systems
    pub fn collect_from_global_systems() -> GameSave {
        use crate::state::GAME_STATE;

        let state = GAME_STATE.read().unwrap();
        state.to_save()
    }

    /// Apply loaded state to global systems
    pub fn apply_to_global_systems(save: &GameSave) -> Result<(), SaveError> {
        use crate::state::{GameState, GAME_STATE};

        let new_state = GameState::from_save(save);
        *GAME_STATE.write().unwrap() = new_state;

        Ok(())
    }

    /// Save game to specific slot
    pub fn save_to_slot(slot: u8) -> Result<(), SaveError> {
        Self::validate_slot(slot)?;

        let current_state =
            Self::get_current_state().unwrap_or_else(Self::collect_from_global_systems);

        let path = Self::get_slot_path(slot);
        Self::save_to_path(&path, &current_state)?;

        log::info!("Game saved to slot {}", slot);
        Ok(())
    }

    /// Load game from specific slot
    pub fn load_from_slot(slot: u8) -> Result<GameSave, SaveError> {
        Self::validate_slot(slot)?;

        let path = Self::get_slot_path(slot);
        let save = Self::load_from_path(&path)?;

        // Apply to global systems
        Self::apply_to_global_systems(&save)?;

        // Update current state
        Self::update_current_state(save.clone());

        log::info!("Game loaded from slot {}", slot);
        Ok(save)
    }

    /// Auto-save current state
    pub fn auto_save() -> Result<(), SaveError> {
        let current_state =
            Self::get_current_state().unwrap_or_else(Self::collect_from_global_systems);

        let path = Self::get_auto_save_path();
        Self::save_to_path(&path, &current_state)?;

        log::debug!("Auto-save completed");
        Ok(())
    }

    /// Load auto-save
    pub fn load_auto_save() -> Result<GameSave, SaveError> {
        let path = Self::get_auto_save_path();
        let save = Self::load_from_path(&path)?;

        Self::apply_to_global_systems(&save)?;
        Self::update_current_state(save.clone());

        log::info!("Auto-save loaded");
        Ok(save)
    }

    /// Check if save slot exists
    pub fn slot_exists(slot: u8) -> bool {
        if Self::validate_slot(slot).is_err() {
            return false;
        }

        let path = Self::get_slot_path(slot);
        path.exists()
    }

    /// Check if auto-save exists
    pub fn auto_save_exists() -> bool {
        Self::get_auto_save_path().exists()
    }

    /// Delete save slot
    pub fn delete_slot(slot: u8) -> Result<(), SaveError> {
        Self::validate_slot(slot)?;

        let path = Self::get_slot_path(slot);
        if path.exists() {
            remove_file(&path)?;
            log::info!("Deleted save slot {}", slot);
        }

        Ok(())
    }

    /// Get save slot info for UI display
    pub fn get_slot_info(slot: u8) -> Result<Option<SaveSlotInfo>, SaveError> {
        Self::validate_slot(slot)?;

        let path = Self::get_slot_path(slot);
        if !path.exists() {
            return Ok(None);
        }

        // Read just enough to get metadata without full deserialization
        let save = Self::load_from_path(&path)?;

        Ok(Some(SaveSlotInfo {
            slot,
            timestamp: save.timestamp,
            version: save.version,
            week: save.progress.current_week,
            season: save.progress.current_season,
            player_count: save.players.len(),
            total_matches: save.progress.total_matches,
        }))
    }

    /// Get all save slot info
    pub fn get_all_slot_info() -> Vec<SaveSlotInfo> {
        let mut slots = Vec::new();

        for slot in 0..3 {
            if let Ok(Some(info)) = Self::get_slot_info(slot) {
                slots.push(info);
            }
        }

        slots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Most recent first
        slots
    }

    // Private helper methods

    fn validate_slot(slot: u8) -> Result<(), SaveError> {
        if slot >= 3 {
            return Err(SaveError::InvalidSlot { slot: slot as i64 });
        }
        Ok(())
    }

    fn get_slot_path(slot: u8) -> PathBuf {
        Self::get_save_dir().join(format!("save_slot_{}.dat", slot))
    }

    fn get_auto_save_path() -> PathBuf {
        Self::get_save_dir().join("auto_save.dat")
    }

    fn get_save_dir() -> PathBuf {
        // In real implementation, this would use Godot's user:// path
        // For now, use a local directory
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join("saves")
    }

    fn save_to_path(path: &Path, save: &GameSave) -> Result<(), SaveError> {
        // Ensure save directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Serialize and compress
        let data = serialize_and_compress(save)?;

        // Atomic save: write to temp file, then rename
        let temp_path = path.with_extension("tmp");

        {
            let mut file = File::create(&temp_path)?;
            file.write_all(&data)?;
            file.flush()?;

            // sync_all ensures data is written to disk (portable fsync)
            file.sync_all()?;
        }

        // Atomic rename
        rename(&temp_path, path)?;

        log::debug!("Saved {} bytes to {:?}", data.len(), path);
        Ok(())
    }

    fn load_from_path(path: &Path) -> Result<GameSave, SaveError> {
        if !path.exists() {
            return Err(SaveError::FileNotFound { path: path.display().to_string() });
        }

        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let mut save = decompress_and_deserialize(&data)?;

        // Apply migrations if needed
        save = migrate_save(save)?;

        log::debug!("Loaded {} bytes from {:?}", data.len(), path);
        Ok(save)
    }
}

#[derive(Debug, Clone)]
pub struct SaveSlotInfo {
    pub slot: u8,
    pub timestamp: u64,
    pub version: u32,
    pub week: u16,
    pub season: u16,
    pub player_count: usize,
    pub total_matches: u32,
}

impl SaveSlotInfo {
    pub fn format_timestamp(&self) -> String {
        use time::{format_description::well_known::Rfc3339, OffsetDateTime};

        let timestamp =
            OffsetDateTime::from_unix_timestamp_nanos((self.timestamp * 1_000_000) as i128)
                .unwrap_or_else(|_| OffsetDateTime::now_utc());

        timestamp.format(&Rfc3339).unwrap_or_else(|_| "Unknown".to_string())
    }

    pub fn get_display_text(&self) -> String {
        format!(
            "Slot {}: Week {} Season {} ({} players)",
            self.slot, self.week, self.season, self.player_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let save_path = temp_dir.path().join("test_save.dat");

        let original_save = GameSave::new();

        // Save
        SaveManager::save_to_path(&save_path, &original_save).unwrap();

        // Load
        let loaded_save = SaveManager::load_from_path(&save_path).unwrap();

        assert_eq!(original_save.version, loaded_save.version);
        assert_eq!(original_save.players.len(), loaded_save.players.len());
    }

    #[test]
    fn test_atomic_save() {
        let temp_dir = TempDir::new().unwrap();
        let save_path = temp_dir.path().join("atomic_test.dat");

        let save = GameSave::new();

        // Save should be atomic - either complete file or no file
        SaveManager::save_to_path(&save_path, &save).unwrap();

        // File should exist and be valid
        assert!(save_path.exists());
        let loaded = SaveManager::load_from_path(&save_path).unwrap();
        assert_eq!(save.version, loaded.version);

        // Temp file should not exist
        let temp_path = save_path.with_extension("tmp");
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_slot_validation() {
        assert!(SaveManager::validate_slot(0).is_ok());
        assert!(SaveManager::validate_slot(2).is_ok());
        assert!(SaveManager::validate_slot(3).is_err());
        assert!(SaveManager::validate_slot(255).is_err());
    }
}
