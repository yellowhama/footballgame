use super::types::*;
use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type QuestResult<T> = std::result::Result<T, CoreError>;

/// Quest manager state for save/load
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuestManagerState {
    pub quests: HashMap<String, Quest>,
    pub active_quest_ids: Vec<String>,
}

/// Core quest management system
pub struct QuestManager {
    state: QuestManagerState,
    current_time: u64, // Current game timestamp
}

impl Default for QuestManager {
    fn default() -> Self {
        Self::new()
    }
}

impl QuestManager {
    pub fn new() -> Self {
        Self { state: QuestManagerState::default(), current_time: 0 }
    }

    /// Load from save data
    pub fn from_state(state: QuestManagerState, current_time: u64) -> Self {
        Self { state, current_time }
    }

    /// Get current state for saving
    pub fn get_state(&self) -> &QuestManagerState {
        &self.state
    }

    /// Update current game time
    pub fn set_current_time(&mut self, timestamp: u64) {
        self.current_time = timestamp;
    }

    /// Add a new quest to the system
    pub fn add_quest(&mut self, quest: Quest) {
        self.state.quests.insert(quest.id.clone(), quest);
    }

    /// Get quest by ID
    pub fn get_quest(&self, quest_id: &str) -> Option<&Quest> {
        self.state.quests.get(quest_id)
    }

    /// Get mutable quest by ID
    pub fn get_quest_mut(&mut self, quest_id: &str) -> Option<&mut Quest> {
        self.state.quests.get_mut(quest_id)
    }

    /// Get all quests with specific status
    pub fn get_quests_by_status(&self, status: QuestStatus) -> Vec<&Quest> {
        self.state.quests.values().filter(|q| q.status == status).collect()
    }

    /// Get all quests of specific type
    pub fn get_quests_by_type(&self, quest_type: QuestType) -> Vec<&Quest> {
        self.state.quests.values().filter(|q| q.quest_type == quest_type).collect()
    }

    /// Get all active quests
    pub fn get_active_quests(&self) -> Vec<&Quest> {
        self.state.active_quest_ids.iter().filter_map(|id| self.state.quests.get(id)).collect()
    }

    /// Check if unlock conditions are met
    pub fn check_unlock_conditions(
        &self,
        condition: &UnlockCondition,
        player_level: u32,
        squad_level: Option<SquadLevel>,
    ) -> bool {
        // Check minimum level
        if let Some(min_level) = condition.min_level {
            if player_level < min_level {
                return false;
            }
        }

        // Check squad level
        if let Some(required_squad) = &condition.squad_level {
            if squad_level.as_ref() != Some(required_squad) {
                return false;
            }
        }

        // Check required quests completion
        for required_id in &condition.required_quests {
            if let Some(quest) = self.state.quests.get(required_id) {
                if quest.status != QuestStatus::Completed {
                    return false;
                }
            } else {
                // Required quest doesn't exist
                return false;
            }
        }

        true
    }

    /// Unlock a quest
    pub fn unlock_quest(
        &mut self,
        quest_id: &str,
        player_level: u32,
        squad_level: Option<SquadLevel>,
    ) -> QuestResult<()> {
        // Clone unlock condition first to avoid borrow issues
        let unlock_condition = self
            .state
            .quests
            .get(quest_id)
            .ok_or_else(|| CoreError::NotFound(format!("Quest not found: {}", quest_id)))?
            .unlock_condition
            .clone();

        let status = self.state.quests.get(quest_id).unwrap().status;

        if status != QuestStatus::Locked {
            return Err(CoreError::InvalidParameter(format!("Quest {} is not locked", quest_id)));
        }

        if !self.check_unlock_conditions(&unlock_condition, player_level, squad_level) {
            return Err(CoreError::InvalidParameter(format!(
                "Quest {} unlock conditions not met",
                quest_id
            )));
        }

        // Now mutably borrow to update status
        let quest = self.state.quests.get_mut(quest_id).unwrap();
        quest.status = QuestStatus::Active;
        Ok(())
    }

    /// Activate a quest (player manually starts it)
    pub fn activate_quest(&mut self, quest_id: &str) -> QuestResult<()> {
        let quest = self
            .state
            .quests
            .get_mut(quest_id)
            .ok_or_else(|| CoreError::NotFound(format!("Quest not found: {}", quest_id)))?;

        if quest.status != QuestStatus::Active {
            return Err(CoreError::InvalidParameter(format!(
                "Quest {} cannot be activated",
                quest_id
            )));
        }

        // Activate quest with current timestamp
        quest.activate(self.current_time);

        // Add to active quests if not already there
        if !self.state.active_quest_ids.contains(&quest_id.to_string()) {
            self.state.active_quest_ids.push(quest_id.to_string());
        }

        Ok(())
    }

    /// Update objective progress for a quest
    pub fn update_objective(
        &mut self,
        quest_id: &str,
        objective_index: usize,
        value: i32,
    ) -> QuestResult<bool> {
        let quest = self
            .state
            .quests
            .get_mut(quest_id)
            .ok_or_else(|| CoreError::NotFound(format!("Quest not found: {}", quest_id)))?;

        if quest.status != QuestStatus::Active {
            return Err(CoreError::InvalidParameter(format!("Quest {} is not active", quest_id)));
        }

        let objective = quest.objectives.get_mut(objective_index).ok_or_else(|| {
            CoreError::NotFound(format!("Objective {} not found", objective_index))
        })?;

        objective.update_progress(value);

        // Check if quest is now complete
        let quest_complete = quest.is_complete();
        if quest_complete {
            self.complete_quest(quest_id)?;
        }

        Ok(quest_complete)
    }

    /// Update objective by type (useful for event hooks)
    pub fn update_objective_by_type(
        &mut self,
        quest_id: &str,
        objective_type: ObjectiveType,
        value: i32,
    ) -> QuestResult<bool> {
        let quest = self
            .state
            .quests
            .get_mut(quest_id)
            .ok_or_else(|| CoreError::NotFound(format!("Quest not found: {}", quest_id)))?;

        if quest.status != QuestStatus::Active {
            return Ok(false);
        }

        // Update all matching objectives
        for objective in quest.objectives.iter_mut() {
            if objective.objective_type == objective_type {
                objective.update_progress(value);
            }
        }

        // Check if quest is now complete
        let quest_complete = quest.is_complete();
        if quest_complete {
            self.complete_quest(quest_id)?;
        }

        Ok(quest_complete)
    }

    /// Update all active quests for a specific objective type
    pub fn update_all_active_objectives(
        &mut self,
        objective_type: ObjectiveType,
        value: i32,
    ) -> Vec<String> {
        let mut completed_quests = Vec::new();

        // Clone active quest IDs to avoid borrow issues
        let active_ids: Vec<String> = self.state.active_quest_ids.clone();

        for quest_id in active_ids {
            if let Ok(completed) = self.update_objective_by_type(&quest_id, objective_type, value) {
                if completed {
                    completed_quests.push(quest_id);
                }
            }
        }

        completed_quests
    }

    /// Complete a quest
    fn complete_quest(&mut self, quest_id: &str) -> QuestResult<()> {
        let quest = self
            .state
            .quests
            .get_mut(quest_id)
            .ok_or_else(|| CoreError::NotFound(format!("Quest not found: {}", quest_id)))?;

        quest.complete(self.current_time);

        // Remove from active quests
        self.state.active_quest_ids.retain(|id| id != quest_id);

        Ok(())
    }

    /// Fail a quest
    pub fn fail_quest(&mut self, quest_id: &str) -> QuestResult<()> {
        let quest = self
            .state
            .quests
            .get_mut(quest_id)
            .ok_or_else(|| CoreError::NotFound(format!("Quest not found: {}", quest_id)))?;

        quest.fail();

        // Remove from active quests
        self.state.active_quest_ids.retain(|id| id != quest_id);

        Ok(())
    }

    /// Check time limits on all active quests
    pub fn check_time_limits(&mut self) -> Vec<String> {
        let mut failed_quests = Vec::new();
        let active_ids: Vec<String> = self.state.active_quest_ids.clone();

        for quest_id in active_ids {
            if let Some(quest) = self.state.quests.get(&quest_id) {
                if !quest.check_time_limit(self.current_time) {
                    // Quest failed due to time limit
                    if self.fail_quest(&quest_id).is_ok() {
                        failed_quests.push(quest_id);
                    }
                }
            }
        }

        failed_quests
    }

    /// Auto-unlock quests based on conditions
    pub fn auto_unlock_quests(
        &mut self,
        player_level: u32,
        squad_level: Option<SquadLevel>,
    ) -> Vec<String> {
        let mut unlocked_quests = Vec::new();

        // Find all locked quests
        let locked_quest_ids: Vec<String> =
            self.get_quests_by_status(QuestStatus::Locked).iter().map(|q| q.id.clone()).collect();

        for quest_id in locked_quest_ids {
            if self.unlock_quest(&quest_id, player_level, squad_level).is_ok() {
                unlocked_quests.push(quest_id);
            }
        }

        unlocked_quests
    }

    /// Get quest statistics
    pub fn get_statistics(&self) -> QuestStatistics {
        let total = self.state.quests.len();
        let completed = self.get_quests_by_status(QuestStatus::Completed).len();
        let active = self.state.active_quest_ids.len();
        let failed = self.get_quests_by_status(QuestStatus::Failed).len();
        let locked = self.get_quests_by_status(QuestStatus::Locked).len();

        QuestStatistics { total, completed, active, failed, locked }
    }
}

/// Quest statistics for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestStatistics {
    pub total: usize,
    pub completed: usize,
    pub active: usize,
    pub failed: usize,
    pub locked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_quest(id: &str, quest_type: QuestType) -> Quest {
        let mut quest = Quest::new(
            id.to_string(),
            format!("{} Quest", id),
            "Test quest".to_string(),
            quest_type,
        );
        quest.objectives.push(Objective::new("Win 5 matches".to_string(), 5, ObjectiveType::Win));
        quest
    }

    #[test]
    fn test_add_and_get_quest() {
        let mut manager = QuestManager::new();
        let quest = create_test_quest("test1", QuestType::Main);
        manager.add_quest(quest);

        assert!(manager.get_quest("test1").is_some());
        assert!(manager.get_quest("nonexistent").is_none());
    }

    #[test]
    fn test_unlock_quest() {
        let mut manager = QuestManager::new();
        let mut quest = create_test_quest("test1", QuestType::Main);
        quest.unlock_condition.min_level = Some(5);
        manager.add_quest(quest);

        // Should fail with low level
        assert!(manager.unlock_quest("test1", 3, None).is_err());

        // Should succeed with sufficient level
        assert!(manager.unlock_quest("test1", 5, None).is_ok());
        assert_eq!(manager.get_quest("test1").unwrap().status, QuestStatus::Active);
    }

    #[test]
    fn test_activate_quest() {
        let mut manager = QuestManager::new();
        let mut quest = create_test_quest("test1", QuestType::Main);
        quest.status = QuestStatus::Active;
        manager.add_quest(quest);

        manager.set_current_time(1000);
        assert!(manager.activate_quest("test1").is_ok());

        let quest = manager.get_quest("test1").unwrap();
        assert_eq!(quest.started_at, Some(1000));
        assert!(manager.state.active_quest_ids.contains(&"test1".to_string()));
    }

    #[test]
    fn test_update_objective() {
        let mut manager = QuestManager::new();
        let mut quest = create_test_quest("test1", QuestType::Main);
        quest.status = QuestStatus::Active;
        quest.started_at = Some(0);
        manager.add_quest(quest);
        manager.state.active_quest_ids.push("test1".to_string());

        // Update progress
        assert!(manager.update_objective("test1", 0, 3).is_ok());
        assert_eq!(manager.get_quest("test1").unwrap().objectives[0].current_value, 3);

        // Complete quest
        manager.set_current_time(2000);
        let completed = manager.update_objective("test1", 0, 2).unwrap();
        assert!(completed);
        assert_eq!(manager.get_quest("test1").unwrap().status, QuestStatus::Completed);
        assert!(!manager.state.active_quest_ids.contains(&"test1".to_string()));
    }

    #[test]
    fn test_update_all_active_objectives() {
        let mut manager = QuestManager::new();

        let mut quest1 = create_test_quest("test1", QuestType::Main);
        quest1.status = QuestStatus::Active;
        quest1.started_at = Some(0);

        let mut quest2 = create_test_quest("test2", QuestType::Side);
        quest2.status = QuestStatus::Active;
        quest2.started_at = Some(0);

        manager.add_quest(quest1);
        manager.add_quest(quest2);
        manager.state.active_quest_ids.push("test1".to_string());
        manager.state.active_quest_ids.push("test2".to_string());

        manager.set_current_time(3000);
        let completed = manager.update_all_active_objectives(ObjectiveType::Win, 5);

        // Both quests should be completed
        assert_eq!(completed.len(), 2);
        assert_eq!(manager.get_quest("test1").unwrap().status, QuestStatus::Completed);
        assert_eq!(manager.get_quest("test2").unwrap().status, QuestStatus::Completed);
    }

    #[test]
    fn test_time_limit_check() {
        let mut manager = QuestManager::new();
        let mut quest = create_test_quest("test1", QuestType::Daily);
        quest.status = QuestStatus::Active;
        quest.started_at = Some(0);
        quest.time_limit = Some(7); // 7 days
        manager.add_quest(quest);
        manager.state.active_quest_ids.push("test1".to_string());

        // After 3 days - should still be active
        manager.set_current_time(3 * 86400);
        let failed = manager.check_time_limits();
        assert!(failed.is_empty());

        // After 8 days - should fail
        manager.set_current_time(8 * 86400);
        let failed = manager.check_time_limits();
        assert_eq!(failed.len(), 1);
        assert_eq!(manager.get_quest("test1").unwrap().status, QuestStatus::Failed);
    }

    #[test]
    fn test_auto_unlock() {
        let mut manager = QuestManager::new();

        let mut quest1 = create_test_quest("test1", QuestType::Main);
        quest1.unlock_condition.min_level = Some(5);

        let mut quest2 = create_test_quest("test2", QuestType::Side);
        quest2.unlock_condition.min_level = Some(10);

        manager.add_quest(quest1);
        manager.add_quest(quest2);

        // Level 5 - should unlock quest1 only
        let unlocked = manager.auto_unlock_quests(5, None);
        assert_eq!(unlocked.len(), 1);
        assert!(unlocked.contains(&"test1".to_string()));

        // Level 10 - should unlock quest2
        let unlocked = manager.auto_unlock_quests(10, None);
        assert_eq!(unlocked.len(), 1);
        assert!(unlocked.contains(&"test2".to_string()));
    }

    #[test]
    fn test_statistics() {
        let mut manager = QuestManager::new();

        let mut quest1 = create_test_quest("test1", QuestType::Main);
        quest1.status = QuestStatus::Completed;

        let mut quest2 = create_test_quest("test2", QuestType::Side);
        quest2.status = QuestStatus::Active;
        manager.state.active_quest_ids.push("test2".to_string());

        let quest3 = create_test_quest("test3", QuestType::Daily);

        manager.add_quest(quest1);
        manager.add_quest(quest2);
        manager.add_quest(quest3);

        let stats = manager.get_statistics();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.active, 1);
        assert_eq!(stats.locked, 1);
        assert_eq!(stats.failed, 0);
    }
}
