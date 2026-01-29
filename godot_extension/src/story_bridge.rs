//! Story System Bridge for Godot
//!
//! Godot과 Story System 간의 연결 브릿지

use godot::prelude::*;
use of_core::story::*;
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Story System을 위한 정적 엔진 인스턴스
static STORY_ENGINE: Lazy<Mutex<Option<StoryEngine>>> = Lazy::new(|| Mutex::new(None));

/// Story System Bridge - Godot과 Rust Story System 연결
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct StoryBridge {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for StoryBridge {
    fn init(base: Base<RefCounted>) -> Self {
        godot_print!("StoryBridge initialized");
        Self { base }
    }
}

#[godot_api]
impl StoryBridge {
    /// Story System 초기화
    ///
    /// # Arguments
    /// * `config_json` - 초기 설정 JSON
    ///   - player_name: String
    ///   - player_ca: u16
    ///   - team_name: String
    ///   - personality: String
    ///
    /// # Returns
    /// 초기화 성공 여부와 메시지를 담은 JSON
    #[func]
    pub fn story_init(&self, config_json: GString) -> GString {
        let config_str = config_json.to_string();

        // 입력 검증
        if config_str.is_empty() {
            return self.create_error_response("Empty configuration", "EMPTY_CONFIG");
        }

        godot_print!(
            "Initializing Story System with config size: {} bytes",
            config_str.len()
        );

        // JSON 파싱
        let config: serde_json::Value = match serde_json::from_str(&config_str) {
            Ok(val) => val,
            Err(e) => {
                godot_error!("Failed to parse config JSON: {}", e);
                return self.create_error_response(&format!("Invalid JSON: {}", e), "INVALID_JSON");
            }
        };

        // Story Engine 초기화
        let mut engine = StoryEngine::new();

        // 플레이어 정보 설정
        if let Some(player_name) = config.get("player_name").and_then(|v| v.as_str()) {
            // 실제로는 engine.state에 설정
            godot_print!("Player name: {}", player_name);
        }

        if let Some(player_ca) = config.get("player_ca").and_then(|v| v.as_u64()) {
            engine.state.player_stats.ca = player_ca as u8;

            // CA 기반 초기 루트 설정
            engine.state.current_route = match player_ca {
                ca if ca >= 140 => StoryRoute::Elite,
                ca if ca >= 100 => StoryRoute::Standard,
                _ => StoryRoute::Underdog,
            };
        }

        // 이벤트 등록 (샘플)
        self.register_sample_events(&mut engine);

        // Capture route before storing (avoids double-lock + double-unwrap)
        let initial_route = format!("{:?}", engine.state.current_route);

        // 전역 엔진에 저장 with proper error handling for mutex poisoning
        match STORY_ENGINE.lock() {
            Ok(mut guard) => {
                *guard = Some(engine);
            }
            Err(poisoned) => {
                godot_error!("STORY_ENGINE mutex poisoned during init");
                // Recover from poisoned mutex by taking ownership of the data
                let mut guard = poisoned.into_inner();
                *guard = Some(engine);
            }
        }

        // 성공 응답
        let response = serde_json::json!({
            "success": true,
            "message": "Story System initialized",
            "initial_route": initial_route,
            "current_week": 1
        });

        GString::from(response.to_string())
    }

    /// 주차별 이벤트 처리
    ///
    /// # Arguments
    /// * `week_data_json` - 주차 데이터
    ///   - week: u32
    ///   - match_results: Array of match results
    ///   - training_done: bool
    ///
    /// # Returns
    /// 발생한 이벤트 목록 JSON
    #[func]
    pub fn story_process_week(&self, week_data_json: GString) -> GString {
        let week_str = week_data_json.to_string();

        // 엔진 확인 (with proper mutex error handling)
        let mut engine_guard = match STORY_ENGINE.lock() {
            Ok(guard) => guard,
            Err(_poisoned) => {
                godot_error!("STORY_ENGINE mutex poisoned");
                return self
                    .create_error_response("Internal error: mutex poisoned", "MUTEX_POISONED");
            }
        };
        let engine = match engine_guard.as_mut() {
            Some(eng) => eng,
            None => {
                godot_error!("Story System not initialized");
                return self
                    .create_error_response("Story System not initialized", "NOT_INITIALIZED");
            }
        };

        // 주차 데이터 파싱
        let week_data: serde_json::Value = match serde_json::from_str(&week_str) {
            Ok(val) => val,
            Err(e) => {
                godot_error!("Failed to parse week data: {}", e);
                return self.create_error_response(&format!("Invalid JSON: {}", e), "INVALID_JSON");
            }
        };

        let week = week_data.get("week").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

        engine.state.current_week = week;

        // 매치 결과 처리 (골, 어시스트 등)
        if let Some(match_results) = week_data.get("match_results").and_then(|v| v.as_array()) {
            for result in match_results {
                if let Some(goals) = result.get("goals").and_then(|v| v.as_u64()) {
                    engine.state.player_stats.goals += goals as u32;
                }
                if let Some(assists) = result.get("assists").and_then(|v| v.as_u64()) {
                    engine.state.player_stats.assists += assists as u32;
                }
            }
        }

        // 이벤트 처리
        let events = engine.process_week(week);

        // 분기점 체크
        let route_change = engine.check_branch_point(week);

        // 이벤트를 JSON으로 변환
        let events_json: Vec<serde_json::Value> = events
            .iter()
            .map(|event| {
                serde_json::json!({
                    "id": event.id,
                    "title": event.title,
                    "description": event.description,
                    "choices": event.choices.iter().map(|choice| {
                        serde_json::json!({
                            "text": choice.text,
                            "available": true  // 실제로는 요구사항 체크 필요
                        })
                    }).collect::<Vec<_>>(),
                    "type": format!("{:?}", event.event_type)
                })
            })
            .collect();

        let response = serde_json::json!({
            "success": true,
            "week": week,
            "events": events_json,
            "current_route": format!("{:?}", engine.state.current_route),
            "route_changed": route_change.is_some(),
            "player_stats": {
                "ca": engine.state.player_stats.ca,
                "goals": engine.state.player_stats.goals,
                "assists": engine.state.player_stats.assists,
            }
        });

        GString::from(response.to_string())
    }

    /// 선택지 처리
    ///
    /// # Arguments
    /// * `choice_json` - 선택 데이터
    ///   - event_id: String
    ///   - choice_index: usize
    ///
    /// # Returns
    /// 선택 결과 JSON
    #[func]
    pub fn story_make_choice(&self, choice_json: GString) -> GString {
        let choice_str = choice_json.to_string();

        // 엔진 확인 (with proper mutex error handling)
        let mut engine_guard = match STORY_ENGINE.lock() {
            Ok(guard) => guard,
            Err(_poisoned) => {
                godot_error!("STORY_ENGINE mutex poisoned");
                return self
                    .create_error_response("Internal error: mutex poisoned", "MUTEX_POISONED");
            }
        };
        let engine = match engine_guard.as_mut() {
            Some(eng) => eng,
            None => {
                return self
                    .create_error_response("Story System not initialized", "NOT_INITIALIZED");
            }
        };

        // 선택 데이터 파싱
        let choice_data: serde_json::Value = match serde_json::from_str(&choice_str) {
            Ok(val) => val,
            Err(e) => {
                return self.create_error_response(&format!("Invalid JSON: {}", e), "INVALID_JSON");
            }
        };

        let event_id = choice_data
            .get("event_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let choice_index = choice_data
            .get("choice_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // 선택 처리
        match engine.make_choice(event_id, choice_index) {
            Ok(_) => {
                let response = serde_json::json!({
                    "success": true,
                    "message": "Choice processed",
                    "event_id": event_id,
                    "choice_index": choice_index,
                    "updated_stats": {
                        "ca": engine.state.player_stats.ca,
                    }
                });
                GString::from(response.to_string())
            }
            Err(e) => {
                self.create_error_response(&format!("Choice failed: {:?}", e), "CHOICE_FAILED")
            }
        }
    }

    /// 현재 스토리 상태 조회
    #[func]
    pub fn get_story_state(&self) -> GString {
        let engine_guard = match STORY_ENGINE.lock() {
            Ok(guard) => guard,
            Err(_poisoned) => {
                godot_error!("STORY_ENGINE mutex poisoned");
                return self
                    .create_error_response("Internal error: mutex poisoned", "MUTEX_POISONED");
            }
        };
        let engine = match engine_guard.as_ref() {
            Some(eng) => eng,
            None => {
                return self
                    .create_error_response("Story System not initialized", "NOT_INITIALIZED");
            }
        };

        let state = &engine.state;
        let response = serde_json::json!({
            "success": true,
            "current_week": state.current_week,
            "current_route": format!("{:?}", state.current_route),
            "player_stats": {
                "ca": state.player_stats.ca,
                "goals": state.player_stats.goals,
                "assists": state.player_stats.assists,
            },
            "occurred_events": state.occurred_events.len(),
            "active_flags": state.active_flags.len(),
        });

        GString::from(response.to_string())
    }

    /// 스토리 상태 저장
    #[func]
    pub fn save_story_state(&self) -> GString {
        use of_core::story::serialization::save_state_msgpack;

        let engine_guard = match STORY_ENGINE.lock() {
            Ok(guard) => guard,
            Err(_poisoned) => {
                godot_error!("STORY_ENGINE mutex poisoned");
                return self
                    .create_error_response("Internal error: mutex poisoned", "MUTEX_POISONED");
            }
        };
        let engine = match engine_guard.as_ref() {
            Some(eng) => eng,
            None => {
                return self
                    .create_error_response("Story System not initialized", "NOT_INITIALIZED");
            }
        };

        // MessagePack으로 직렬화
        let mut buffer = Vec::new();
        match save_state_msgpack(&engine.state, &mut buffer) {
            Ok(_) => {
                // Base64 인코딩
                use base64::{engine::general_purpose, Engine as _};
                let encoded = general_purpose::STANDARD.encode(&buffer);

                let response = serde_json::json!({
                    "success": true,
                    "save_data": encoded,
                    "size_bytes": buffer.len()
                });
                GString::from(response.to_string())
            }
            Err(e) => self.create_error_response(&format!("Save failed: {:?}", e), "SAVE_FAILED"),
        }
    }

    /// 스토리 상태 로드
    #[func]
    pub fn load_story_state(&self, save_data: GString) -> GString {
        use base64::{engine::general_purpose, Engine as _};
        use of_core::story::serialization::load_state_msgpack;

        let encoded = save_data.to_string();

        // Base64 디코딩
        let buffer = match general_purpose::STANDARD.decode(encoded) {
            Ok(buf) => buf,
            Err(e) => {
                return self
                    .create_error_response(&format!("Invalid save data: {}", e), "INVALID_DATA");
            }
        };

        // MessagePack 역직렬화
        match load_state_msgpack(&buffer[..]) {
            Ok(state) => {
                let mut engine_guard = match STORY_ENGINE.lock() {
                    Ok(guard) => guard,
                    Err(_poisoned) => {
                        godot_error!("STORY_ENGINE mutex poisoned");
                        return self.create_error_response(
                            "Internal error: mutex poisoned",
                            "MUTEX_POISONED",
                        );
                    }
                };
                if let Some(engine) = engine_guard.as_mut() {
                    engine.state = state;

                    let response = serde_json::json!({
                        "success": true,
                        "message": "Story state loaded",
                        "current_week": engine.state.current_week
                    });
                    GString::from(response.to_string())
                } else {
                    self.create_error_response("Story System not initialized", "NOT_INITIALIZED")
                }
            }
            Err(e) => self.create_error_response(&format!("Load failed: {:?}", e), "LOAD_FAILED"),
        }
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

    fn register_sample_events(&self, engine: &mut StoryEngine) {
        // 샘플 이벤트 등록
        let event1 = StoryEvent {
            id: "training_breakthrough".to_string(),
            event_type: StoryEventType::Fixed,
            title: "Training Breakthrough".to_string(),
            description: "You've shown exceptional progress in training!".to_string(),
            choices: vec![
                EventChoice {
                    id: "train_harder".to_string(),
                    text: "Train Even Harder".to_string(),
                    requirements: vec![],
                    effects: vec![StoryEffect::ModifyCA(2)],
                    next_event_id: None,
                },
                EventChoice {
                    id: "take_rest".to_string(),
                    text: "Take a Rest".to_string(),
                    requirements: vec![],
                    effects: vec![], // morale이 없으므로 일단 비워둠
                    next_event_id: None,
                },
            ],
            conditions: vec![],
            week_range: Some((5, 5)), // Week 5에 발생
            priority: EventPriority::Normal,
            tags: vec!["training".to_string()],
        };

        engine.event_registry.register_event(event1);

        // 더 많은 이벤트 등록 가능...
    }
}

// base64 의존성 필요
// Cargo.toml에 추가: base64 = "0.21"
