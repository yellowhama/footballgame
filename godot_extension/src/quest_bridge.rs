//! Quest System Bridge for Godot
//!
//! Godot과 Quest System 간의 연결 브릿지

use godot::prelude::*;
use of_core::quest::*;
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Quest Manager를 위한 정적 인스턴스
static QUEST_MANAGER: Lazy<Mutex<Option<QuestManager>>> = Lazy::new(|| Mutex::new(None));

/// Quest System Bridge - Godot과 Rust Quest System 연결
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct QuestBridge {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for QuestBridge {
    fn init(base: Base<RefCounted>) -> Self {
        godot_print!("QuestBridge initialized");
        Self { base }
    }
}

#[godot_api]
impl QuestBridge {
    /// Quest System 초기화
    ///
    /// # Arguments
    /// * `config_json` - 초기 설정 JSON (optional)
    ///   - current_time: u64 (game timestamp)
    ///
    /// # Returns
    /// 초기화 결과 JSON
    #[func]
    pub fn quest_init(&self, config_json: GString) -> GString {
        let config_str = config_json.to_string();

        godot_print!("Initializing Quest System");

        let mut manager = QuestManager::new();

        // 설정 파싱 (있는 경우)
        if !config_str.is_empty() {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) {
                if let Some(current_time) = config.get("current_time").and_then(|v| v.as_u64()) {
                    manager.set_current_time(current_time);
                }
            }
        }

        // 샘플 퀘스트 등록
        self.register_sample_quests(&mut manager);

        // 전역 매니저에 저장
        match QUEST_MANAGER.lock() {
            Ok(mut guard) => {
                *guard = Some(manager);
            }
            Err(poisoned) => {
                godot_error!("QUEST_MANAGER mutex poisoned during init");
                let mut guard = poisoned.into_inner();
                *guard = Some(manager);
            }
        }

        let response = serde_json::json!({
            "success": true,
            "message": "Quest System initialized"
        });

        GString::from(response.to_string())
    }

    /// 모든 퀘스트 목록 조회
    #[func]
    pub fn get_all_quests(&self) -> GString {
        let manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_poisoned) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_ref() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        let state = manager.get_state();
        let quests: Vec<serde_json::Value> = state
            .quests
            .values()
            .map(|q| self.quest_to_json(q))
            .collect();

        let response = serde_json::json!({
            "success": true,
            "quests": quests
        });

        GString::from(response.to_string())
    }

    /// 특정 상태의 퀘스트 조회
    ///
    /// # Arguments
    /// * `status` - "locked", "active", "completed", "failed"
    #[func]
    pub fn get_quests_by_status(&self, status: GString) -> GString {
        let status_str = status.to_string();
        let quest_status = match status_str.as_str() {
            "locked" => QuestStatus::Locked,
            "active" => QuestStatus::Active,
            "completed" => QuestStatus::Completed,
            "failed" => QuestStatus::Failed,
            _ => {
                return self.create_error_response(
                    &format!("Invalid status: {}", status_str),
                    "INVALID_STATUS",
                );
            }
        };

        let manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_ref() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        let quests: Vec<serde_json::Value> = manager
            .get_quests_by_status(quest_status)
            .iter()
            .map(|q| self.quest_to_json(q))
            .collect();

        let response = serde_json::json!({
            "success": true,
            "status": status_str,
            "quests": quests
        });

        GString::from(response.to_string())
    }

    /// 활성 퀘스트 목록 조회
    #[func]
    pub fn get_active_quests(&self) -> GString {
        let manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_ref() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        let quests: Vec<serde_json::Value> = manager
            .get_active_quests()
            .iter()
            .map(|q| self.quest_to_json(q))
            .collect();

        let response = serde_json::json!({
            "success": true,
            "quests": quests
        });

        GString::from(response.to_string())
    }

    /// 특정 퀘스트 상세 조회
    #[func]
    pub fn get_quest(&self, quest_id: GString) -> GString {
        let id = quest_id.to_string();

        let manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_ref() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        match manager.get_quest(&id) {
            Some(quest) => {
                let response = serde_json::json!({
                    "success": true,
                    "quest": self.quest_to_json(quest)
                });
                GString::from(response.to_string())
            }
            None => self.create_error_response(&format!("Quest not found: {}", id), "NOT_FOUND"),
        }
    }

    /// 퀘스트 활성화 (시작)
    #[func]
    pub fn activate_quest(&self, quest_id: GString) -> GString {
        let id = quest_id.to_string();

        let mut manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_mut() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        match manager.activate_quest(&id) {
            Ok(()) => {
                let response = serde_json::json!({
                    "success": true,
                    "message": format!("Quest {} activated", id),
                    "quest_id": id
                });
                GString::from(response.to_string())
            }
            Err(e) => self.create_error_response(
                &format!("Failed to activate: {:?}", e),
                "ACTIVATION_FAILED",
            ),
        }
    }

    /// 퀘스트 목표 진행 업데이트
    ///
    /// # Arguments
    /// * `update_json` - JSON 데이터
    ///   - quest_id: String
    ///   - objective_index: usize
    ///   - value: i32 (progress to add)
    #[func]
    pub fn update_objective(&self, update_json: GString) -> GString {
        let update_str = update_json.to_string();

        let update_data: serde_json::Value = match serde_json::from_str(&update_str) {
            Ok(v) => v,
            Err(e) => {
                return self.create_error_response(&format!("Invalid JSON: {}", e), "INVALID_JSON");
            }
        };

        let quest_id = update_data
            .get("quest_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let objective_index = update_data
            .get("objective_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let value = update_data
            .get("value")
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32;

        let mut manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_mut() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        match manager.update_objective(quest_id, objective_index, value) {
            Ok(completed) => {
                let response = serde_json::json!({
                    "success": true,
                    "quest_id": quest_id,
                    "objective_index": objective_index,
                    "value_added": value,
                    "quest_completed": completed
                });
                GString::from(response.to_string())
            }
            Err(e) => {
                self.create_error_response(&format!("Update failed: {:?}", e), "UPDATE_FAILED")
            }
        }
    }

    /// 목표 타입별 전체 업데이트 (이벤트 훅용)
    ///
    /// # Arguments
    /// * `objective_type` - "win", "train", "stat", "event"
    /// * `value` - 추가할 진행값
    #[func]
    pub fn update_all_by_type(&self, objective_type: GString, value: i32) -> GString {
        let type_str = objective_type.to_string();
        let obj_type = match type_str.as_str() {
            "win" => ObjectiveType::Win,
            "train" => ObjectiveType::Train,
            "stat" => ObjectiveType::Stat,
            "event" => ObjectiveType::Event,
            _ => {
                return self
                    .create_error_response(&format!("Invalid type: {}", type_str), "INVALID_TYPE");
            }
        };

        let mut manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_mut() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        let completed_quests = manager.update_all_active_objectives(obj_type, value);

        let response = serde_json::json!({
            "success": true,
            "objective_type": type_str,
            "value_added": value,
            "completed_quests": completed_quests
        });

        GString::from(response.to_string())
    }

    /// 자동 언락 (레벨 기반)
    #[func]
    pub fn auto_unlock(&self, player_level: i32, squad_level: GString) -> GString {
        let squad = match squad_level.to_string().as_str() {
            "youth" => Some(SquadLevel::Youth),
            "b_team" => Some(SquadLevel::BTeam),
            "a_team" => Some(SquadLevel::ATeam),
            "" => None,
            _ => None,
        };

        let mut manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_mut() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        let unlocked = manager.auto_unlock_quests(player_level as u32, squad);

        let response = serde_json::json!({
            "success": true,
            "unlocked_quests": unlocked
        });

        GString::from(response.to_string())
    }

    /// 시간 제한 체크 (실패 처리)
    #[func]
    pub fn check_time_limits(&self, current_time: i64) -> GString {
        let mut manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_mut() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        manager.set_current_time(current_time as u64);
        let failed = manager.check_time_limits();

        let response = serde_json::json!({
            "success": true,
            "failed_quests": failed
        });

        GString::from(response.to_string())
    }

    /// 퀘스트 통계 조회
    #[func]
    pub fn get_statistics(&self) -> GString {
        let manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_ref() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        let stats = manager.get_statistics();

        let response = serde_json::json!({
            "success": true,
            "statistics": {
                "total": stats.total,
                "completed": stats.completed,
                "active": stats.active,
                "failed": stats.failed,
                "locked": stats.locked
            }
        });

        GString::from(response.to_string())
    }

    /// 퀘스트 상태 저장
    #[func]
    pub fn save_quest_state(&self) -> GString {
        let manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_ref() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        let state = manager.get_state();

        match serde_json::to_string(state) {
            Ok(json_str) => {
                use base64::{engine::general_purpose, Engine as _};
                let encoded = general_purpose::STANDARD.encode(json_str.as_bytes());

                let response = serde_json::json!({
                    "success": true,
                    "save_data": encoded,
                    "size_bytes": json_str.len()
                });
                GString::from(response.to_string())
            }
            Err(e) => self.create_error_response(&format!("Save failed: {}", e), "SAVE_FAILED"),
        }
    }

    /// 퀘스트 상태 로드
    #[func]
    pub fn load_quest_state(&self, save_data: GString, current_time: i64) -> GString {
        use base64::{engine::general_purpose, Engine as _};

        let encoded = save_data.to_string();

        let buffer = match general_purpose::STANDARD.decode(&encoded) {
            Ok(buf) => buf,
            Err(e) => {
                return self
                    .create_error_response(&format!("Invalid save data: {}", e), "INVALID_DATA");
            }
        };

        let json_str = match String::from_utf8(buffer) {
            Ok(s) => s,
            Err(e) => {
                return self
                    .create_error_response(&format!("Invalid UTF-8: {}", e), "INVALID_DATA");
            }
        };

        let state: QuestManagerState = match serde_json::from_str(&json_str) {
            Ok(s) => s,
            Err(e) => {
                return self
                    .create_error_response(&format!("Invalid state JSON: {}", e), "INVALID_DATA");
            }
        };

        let manager = QuestManager::from_state(state, current_time as u64);

        match QUEST_MANAGER.lock() {
            Ok(mut guard) => {
                *guard = Some(manager);
            }
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = Some(manager);
            }
        }

        let response = serde_json::json!({
            "success": true,
            "message": "Quest state loaded"
        });

        GString::from(response.to_string())
    }

    /// 새 퀘스트 추가 (런타임)
    #[func]
    pub fn add_quest(&self, quest_json: GString) -> GString {
        let quest_str = quest_json.to_string();

        let quest_data: serde_json::Value = match serde_json::from_str(&quest_str) {
            Ok(v) => v,
            Err(e) => {
                return self.create_error_response(&format!("Invalid JSON: {}", e), "INVALID_JSON");
            }
        };

        let id = quest_data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let title = quest_data
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();
        let description = quest_data
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let quest_type_str = quest_data
            .get("quest_type")
            .and_then(|v| v.as_str())
            .unwrap_or("side");

        let quest_type = match quest_type_str {
            "main" => QuestType::Main,
            "daily" => QuestType::Daily,
            _ => QuestType::Side,
        };

        if id.is_empty() {
            return self.create_error_response("Quest ID is required", "MISSING_ID");
        }

        let mut quest = Quest::new(id.clone(), title, description, quest_type);

        // 목표 파싱
        if let Some(objectives) = quest_data.get("objectives").and_then(|v| v.as_array()) {
            for obj_data in objectives {
                let desc = obj_data
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let target = obj_data
                    .get("target_value")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;
                let obj_type_str = obj_data
                    .get("objective_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("event");

                let obj_type = match obj_type_str {
                    "win" => ObjectiveType::Win,
                    "train" => ObjectiveType::Train,
                    "stat" => ObjectiveType::Stat,
                    _ => ObjectiveType::Event,
                };

                quest
                    .objectives
                    .push(Objective::new(desc, target, obj_type));
            }
        }

        // 보상 파싱
        if let Some(rewards) = quest_data.get("rewards") {
            if let Some(xp) = rewards.get("xp").and_then(|v| v.as_i64()) {
                quest.rewards.xp = xp as i32;
            }
        }

        // 시간 제한
        if let Some(time_limit) = quest_data.get("time_limit").and_then(|v| v.as_u64()) {
            quest.time_limit = Some(time_limit as u32);
        }

        let mut manager_guard = match QUEST_MANAGER.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return self.create_error_response("Mutex poisoned", "MUTEX_POISONED");
            }
        };

        let manager = match manager_guard.as_mut() {
            Some(m) => m,
            None => {
                return self
                    .create_error_response("Quest System not initialized", "NOT_INITIALIZED");
            }
        };

        manager.add_quest(quest);

        let response = serde_json::json!({
            "success": true,
            "message": format!("Quest {} added", id),
            "quest_id": id
        });

        GString::from(response.to_string())
    }

    // Helper functions

    fn create_error_response(&self, message: &str, code: &str) -> GString {
        let response = serde_json::json!({
            "success": false,
            "error": message,
            "error_code": code
        });
        GString::from(response.to_string())
    }

    fn quest_to_json(&self, quest: &Quest) -> serde_json::Value {
        let objectives: Vec<serde_json::Value> = quest
            .objectives
            .iter()
            .map(|obj| {
                serde_json::json!({
                    "description": obj.description,
                    "target_value": obj.target_value,
                    "current_value": obj.current_value,
                    "objective_type": format!("{:?}", obj.objective_type),
                    "is_complete": obj.is_complete(),
                    "progress_percentage": obj.progress_percentage()
                })
            })
            .collect();

        serde_json::json!({
            "id": quest.id,
            "title": quest.title,
            "description": quest.description,
            "quest_type": format!("{:?}", quest.quest_type),
            "status": format!("{:?}", quest.status),
            "objectives": objectives,
            "rewards": {
                "xp": quest.rewards.xp,
                "items": quest.rewards.items,
            },
            "time_limit": quest.time_limit,
            "started_at": quest.started_at,
            "completed_at": quest.completed_at,
            "is_complete": quest.is_complete(),
            "progress_percentage": quest.progress_percentage()
        })
    }

    fn register_sample_quests(&self, manager: &mut QuestManager) {
        // Main Quest: First Steps
        let mut quest1 = Quest::new(
            "main_first_steps".to_string(),
            "First Steps".to_string(),
            "Begin your journey as a professional footballer.".to_string(),
            QuestType::Main,
        );
        quest1.objectives.push(Objective::new(
            "Complete your first training session".to_string(),
            1,
            ObjectiveType::Train,
        ));
        quest1.status = QuestStatus::Active; // Start unlocked
        quest1.rewards.xp = 100;
        manager.add_quest(quest1);

        // Main Quest: Rising Star
        let mut quest2 = Quest::new(
            "main_rising_star".to_string(),
            "Rising Star".to_string(),
            "Prove yourself on the pitch.".to_string(),
            QuestType::Main,
        );
        quest2.objectives.push(Objective::new(
            "Win 3 matches".to_string(),
            3,
            ObjectiveType::Win,
        ));
        quest2.unlock_condition.required_quests = vec!["main_first_steps".to_string()];
        quest2.rewards.xp = 250;
        manager.add_quest(quest2);

        // Side Quest: Training Dedication
        let mut quest3 = Quest::new(
            "side_training_dedication".to_string(),
            "Training Dedication".to_string(),
            "Show your commitment to improvement.".to_string(),
            QuestType::Side,
        );
        quest3.objectives.push(Objective::new(
            "Complete 5 training sessions".to_string(),
            5,
            ObjectiveType::Train,
        ));
        quest3.status = QuestStatus::Active;
        quest3.rewards.xp = 150;
        manager.add_quest(quest3);

        // Daily Quest: Daily Training
        let mut quest4 = Quest::new(
            "daily_training".to_string(),
            "Daily Training".to_string(),
            "Complete today's training.".to_string(),
            QuestType::Daily,
        );
        quest4.objectives.push(Objective::new(
            "Complete 1 training session".to_string(),
            1,
            ObjectiveType::Train,
        ));
        quest4.status = QuestStatus::Active;
        quest4.time_limit = Some(1); // 1 day
        quest4.rewards.xp = 25;
        manager.add_quest(quest4);
    }
}
