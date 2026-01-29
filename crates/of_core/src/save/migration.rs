use super::error::SaveError;
use super::format::GameSave;
use super::SAVE_VERSION;

/// Migrate save data from older versions to current version
pub fn migrate_save(mut save: GameSave) -> Result<GameSave, SaveError> {
    let original_version = save.version;

    // Apply migrations step by step
    save = match save.version {
        0 => migrate_v0_to_v1(save)?,
        1 => save, // Current version, no migration needed
        v if v > SAVE_VERSION => {
            // Future version - might be compatible
            log::warn!("Loading save from future version {} (current: {})", v, SAVE_VERSION);
            save
        }
        _ => {
            return Err(SaveError::VersionMismatch { found: save.version, expected: SAVE_VERSION });
        }
    };

    // Update to current version
    save.version = SAVE_VERSION;
    save.update_timestamp();

    if original_version != SAVE_VERSION {
        log::info!("Migrated save from version {} to {}", original_version, SAVE_VERSION);
    }

    Ok(save)
}

/// Migrate from version 0 to version 1
fn migrate_v0_to_v1(mut save: GameSave) -> Result<GameSave, SaveError> {
    log::info!("Migrating save from version 0 to 1");

    // Example migrations:

    // 1. Initialize new fields that didn't exist in v0
    if save.game_settings.preferred_language.is_empty() {
        save.game_settings.preferred_language = "korean".to_string();
    }

    // 2. Fix any data inconsistencies from v0
    if save.progress.current_week == 0 {
        save.progress.current_week = 1;
    }

    if save.progress.current_season == 0 {
        save.progress.current_season = 1;
    }

    // 3. Validate and fix deck references
    let valid_deck_ids: Vec<String> =
        save.saved_decks.iter().map(|deck| deck.name.clone()).collect();

    if let Some(active_id) = &save.active_deck_id {
        if !valid_deck_ids.contains(active_id) {
            log::warn!("Active deck ID '{}' not found in saved decks, clearing", active_id);
            save.active_deck_id = None;
        }
    }

    // 4. Ensure card inventory consistency
    let mut total_cards = 0;
    for (_, cards) in save.card_inventory.owned_cards.iter() {
        total_cards += cards.len();
    }
    save.card_inventory.total_card_count = total_cards;

    // 5. Update progress stats if they're inconsistent
    let actual_matches = save.match_history.len() as u32;
    if save.progress.total_matches != actual_matches {
        save.progress.total_matches = actual_matches;
    }

    // Calculate win/loss/draw stats from match history
    let mut wins = 0;
    let mut draws = 0;
    let mut losses = 0;
    let mut goals_for = 0;
    let mut goals_against = 0;

    for record in &save.match_history {
        match record.result {
            crate::save::format::MatchResult::Win => wins += 1,
            crate::save::format::MatchResult::Draw => draws += 1,
            crate::save::format::MatchResult::Loss => losses += 1,
        }
        goals_for += record.score_home as u32;
        goals_against += record.score_away as u32;
    }

    save.progress.stats.wins = wins;
    save.progress.stats.draws = draws;
    save.progress.stats.losses = losses;
    save.progress.stats.goals_for = goals_for;
    save.progress.stats.goals_against = goals_against;

    Ok(save)
}

/// Check if a save file needs migration
pub fn needs_migration(save: &GameSave) -> bool {
    save.version < SAVE_VERSION
}

/// Get migration description for UI display
pub fn get_migration_description(from_version: u32, to_version: u32) -> String {
    match (from_version, to_version) {
        (0, 1) => "Adding new game settings and fixing data consistency".to_string(),
        _ => format!("Updating save format from version {} to {}", from_version, to_version),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_v0_to_v1() {
        let mut save = GameSave::new();
        save.version = 0;
        save.progress.current_week = 0; // Invalid value
        save.game_settings.preferred_language = "".to_string(); // Empty

        let migrated = migrate_save(save).unwrap();

        assert_eq!(migrated.version, 1);
        assert_eq!(migrated.progress.current_week, 1);
        assert_eq!(migrated.game_settings.preferred_language, "korean");
    }

    #[test]
    fn test_no_migration_needed() {
        let save = GameSave::new(); // Already current version

        let result = migrate_save(save.clone()).unwrap();

        assert_eq!(result.version, save.version);
    }

    #[test]
    fn test_future_version_warning() {
        let mut save = GameSave::new();
        save.version = 999; // Future version

        let result = migrate_save(save);
        assert!(result.is_ok());
    }
}
