use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Quest type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestType {
    Main,
    Side,
    Daily,
}

/// Quest status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestStatus {
    Locked,
    Active,
    Completed,
    Failed,
}

/// Objective type for quest progression tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectiveType {
    Win,   // Win N matches
    Train, // Complete N training sessions
    Stat,  // Achieve stat threshold
    Event, // Trigger specific events
}

/// Squad level for unlock conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SquadLevel {
    Youth,
    BTeam,
    ATeam,
}

/// Unlock condition for quests
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnlockCondition {
    pub min_level: Option<u32>,
    pub required_quests: Vec<String>,
    pub squad_level: Option<SquadLevel>,
    pub custom_condition: Option<String>, // For complex logic
}

/// Reward types
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Rewards {
    pub xp: i32,
    pub stat_boosts: HashMap<String, i32>, // stat_name -> boost_value
    pub items: Vec<String>,
    pub unlock_events: Vec<String>,
}

/// Single quest objective
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    pub description: String,
    pub target_value: i32,
    pub current_value: i32,
    pub objective_type: ObjectiveType,
}

impl Objective {
    pub fn new(description: String, target_value: i32, objective_type: ObjectiveType) -> Self {
        Self { description, target_value, current_value: 0, objective_type }
    }

    pub fn is_complete(&self) -> bool {
        self.current_value >= self.target_value
    }

    pub fn progress_percentage(&self) -> f32 {
        if self.target_value == 0 {
            return 100.0;
        }
        (self.current_value as f32 / self.target_value as f32 * 100.0).min(100.0)
    }

    pub fn update_progress(&mut self, value: i32) {
        self.current_value = (self.current_value + value).min(self.target_value);
    }
}

/// Main quest data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub quest_type: QuestType,
    pub objectives: Vec<Objective>,
    pub rewards: Rewards,
    pub unlock_condition: UnlockCondition,
    pub time_limit: Option<u32>, // in days
    pub status: QuestStatus,
    pub started_at: Option<u64>,   // timestamp
    pub completed_at: Option<u64>, // timestamp
}

impl Quest {
    pub fn new(id: String, title: String, description: String, quest_type: QuestType) -> Self {
        Self {
            id,
            title,
            description,
            quest_type,
            objectives: Vec::new(),
            rewards: Rewards::default(),
            unlock_condition: UnlockCondition::default(),
            time_limit: None,
            status: QuestStatus::Locked,
            started_at: None,
            completed_at: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        !self.objectives.is_empty() && self.objectives.iter().all(|obj| obj.is_complete())
    }

    pub fn is_failed(&self) -> bool {
        self.status == QuestStatus::Failed
    }

    pub fn progress_percentage(&self) -> f32 {
        if self.objectives.is_empty() {
            return 0.0;
        }
        let total: f32 = self.objectives.iter().map(|obj| obj.progress_percentage()).sum();
        total / self.objectives.len() as f32
    }

    pub fn activate(&mut self, timestamp: u64) {
        self.status = QuestStatus::Active;
        self.started_at = Some(timestamp);
    }

    pub fn complete(&mut self, timestamp: u64) {
        self.status = QuestStatus::Completed;
        self.completed_at = Some(timestamp);
    }

    pub fn fail(&mut self) {
        self.status = QuestStatus::Failed;
    }

    pub fn check_time_limit(&self, current_time: u64) -> bool {
        if let (Some(started), Some(limit_days)) = (self.started_at, self.time_limit) {
            let elapsed_days = (current_time - started) / 86400; // seconds per day
            elapsed_days <= limit_days as u64
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objective_progress() {
        let mut obj = Objective::new("Win 5 matches".to_string(), 5, ObjectiveType::Win);
        assert_eq!(obj.progress_percentage(), 0.0);
        assert!(!obj.is_complete());

        obj.update_progress(3);
        assert!((obj.progress_percentage() - 60.0).abs() < 0.001);
        assert!(!obj.is_complete());

        obj.update_progress(2);
        assert_eq!(obj.progress_percentage(), 100.0);
        assert!(obj.is_complete());
    }

    #[test]
    fn test_quest_completion() {
        let mut quest = Quest::new(
            "test_quest".to_string(),
            "Test Quest".to_string(),
            "Test description".to_string(),
            QuestType::Main,
        );

        quest.objectives.push(Objective::new("Obj 1".to_string(), 5, ObjectiveType::Win));
        quest.objectives.push(Objective::new("Obj 2".to_string(), 3, ObjectiveType::Train));

        assert!(!quest.is_complete());

        quest.objectives[0].update_progress(5);
        assert!(!quest.is_complete());

        quest.objectives[1].update_progress(3);
        assert!(quest.is_complete());
    }

    #[test]
    fn test_quest_activation() {
        let mut quest = Quest::new(
            "test_quest".to_string(),
            "Test Quest".to_string(),
            "Test description".to_string(),
            QuestType::Side,
        );

        assert_eq!(quest.status, QuestStatus::Locked);
        quest.activate(1000);
        assert_eq!(quest.status, QuestStatus::Active);
        assert_eq!(quest.started_at, Some(1000));
    }

    #[test]
    fn test_time_limit() {
        let mut quest = Quest::new(
            "test_quest".to_string(),
            "Test Quest".to_string(),
            "Test description".to_string(),
            QuestType::Daily,
        );

        quest.time_limit = Some(7); // 7 days
        quest.activate(0);

        // After 3 days
        assert!(quest.check_time_limit(3 * 86400));

        // After 8 days - failed
        assert!(!quest.check_time_limit(8 * 86400));
    }
}
