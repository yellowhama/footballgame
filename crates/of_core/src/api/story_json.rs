//! Story System JSON API
//!
//! Godot과 통신하기 위한 JSON 기반 Story System API

use crate::error::CoreError;
use crate::story::conditions::RequirementValidator;
use crate::story::*;
use serde::{Deserialize, Serialize};
use serde_json;

/// Story System 초기화 요청
#[derive(Debug, Deserialize)]
pub struct StoryInitRequest {
    pub schema_version: u8,
    pub player_ca: u8,
    pub player_personality: Option<String>,
    pub initial_week: u32,
}

/// Story System 초기화 응답
#[derive(Debug, Serialize)]
pub struct StoryInitResponse {
    pub schema_version: u8,
    pub success: bool,
    pub initial_route: String,
    pub initial_events: Vec<StoryEventJson>,
}

/// 주차별 이벤트 요청
#[derive(Debug, Deserialize)]
pub struct WeekEventRequest {
    pub schema_version: u8,
    pub week: u32,
    pub match_events: Option<Vec<MatchEventJson>>,
    pub skill_changes: Option<Vec<SkillChangeJson>>,
}

/// 매치 이벤트 JSON
#[derive(Debug, Deserialize)]
pub struct MatchEventJson {
    pub event_type: String,
    pub minute: u8,
    pub player_id: String,
    pub data: serde_json::Value,
}

/// 스킬 변화 JSON
#[derive(Debug, Deserialize)]
pub struct SkillChangeJson {
    pub skill_name: String,
    pub old_value: u8,
    pub new_value: u8,
}

/// 주차별 이벤트 응답
#[derive(Debug, Serialize)]
pub struct WeekEventResponse {
    pub schema_version: u8,
    pub events: Vec<StoryEventJson>,
    pub current_route: String,
    pub route_progress: f32,
}

/// Story Event JSON 표현
#[derive(Debug, Serialize, Deserialize)]
pub struct StoryEventJson {
    pub id: String,
    pub event_type: String,
    pub title: String,
    pub description: String,
    pub choices: Vec<EventChoiceJson>,
    pub priority: String,
    pub tags: Vec<String>,
}

/// Event Choice JSON 표현
#[derive(Debug, Serialize, Deserialize)]
pub struct EventChoiceJson {
    pub id: String,
    pub text: String,
    pub available: bool,
    pub requirement_text: Option<String>,
}

/// 선택지 처리 요청
#[derive(Debug, Deserialize)]
pub struct ChoiceRequest {
    pub schema_version: u8,
    pub event_id: String,
    pub choice_index: usize,
}

/// 선택지 처리 응답
#[derive(Debug, Serialize)]
pub struct ChoiceResponse {
    pub schema_version: u8,
    pub success: bool,
    pub effects_applied: Vec<String>,
    pub next_event: Option<StoryEventJson>,
    pub state_changes: StateChangesJson,
}

/// 상태 변화 JSON
#[derive(Debug, Serialize)]
pub struct StateChangesJson {
    pub ca_change: Option<i8>,
    pub morale_change: Option<i32>,
    pub fatigue_change: Option<i32>,
    pub relationship_changes: Vec<RelationshipChangeJson>,
    pub flags_changed: Vec<String>,
}

/// 관계 변화 JSON
#[derive(Debug, Serialize)]
pub struct RelationshipChangeJson {
    pub character: String,
    pub change: i32,
    pub new_value: i32,
}

/// 루트 예측 요청
#[derive(Debug, Deserialize)]
pub struct RoutePredictionRequest {
    pub schema_version: u8,
    pub weeks_ahead: u32,
}

/// 루트 예측 응답
#[derive(Debug, Serialize)]
pub struct RoutePredictionResponse {
    pub schema_version: u8,
    pub likely_route: String,
    pub confidence: f32,
    pub key_factors: Vec<String>,
}

// 전역 Story Engine 인스턴스 (실제로는 더 나은 관리 필요)
use once_cell::sync::Lazy;
use std::sync::Mutex;

static STORY_ENGINE: Lazy<Mutex<Option<StoryEngine>>> = Lazy::new(|| Mutex::new(None));

// ============================================================================
// FFI 안전 헬퍼 함수
// ============================================================================

/// C 문자열 포인터를 안전하게 Rust String으로 변환
///
/// # Safety
/// - null 포인터 체크
/// - UTF-8 변환 실패 시 lossy 변환 사용
fn safe_c_str_to_string(ptr: *const std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: null 체크 완료, CStr은 null-terminated 보장
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
    Some(c_str.to_string_lossy().into_owned())
}

/// Rust String을 C 문자열 포인터로 안전하게 변환
///
/// NUL 바이트가 있으면 제거 후 변환 (패닉 방지)
fn safe_string_to_c(s: String) -> *mut std::os::raw::c_char {
    // NUL 바이트 제거 (CString::new() 패닉 방지)
    let cleaned = s.replace('\0', "");
    match std::ffi::CString::new(cleaned) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => {
            // 최후의 수단: 빈 문자열 반환
            std::ffi::CString::new("").expect("Empty string should never fail").into_raw()
        }
    }
}

/// 에러 응답 JSON을 C 문자열로 변환하는 헬퍼
fn error_response_c(error_msg: &str) -> *mut std::os::raw::c_char {
    let response = json!({
        "schema_version": 1,
        "success": false,
        "error": error_msg
    });
    safe_string_to_c(response.to_string())
}

// ============================================================================
// FFI 함수
// ============================================================================

/// Story System 초기화 (JSON API)
#[no_mangle]
pub extern "C" fn story_init_json(
    request_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let request_str = match safe_c_str_to_string(request_json) {
        Some(s) => s,
        None => return error_response_c("Invalid input: null pointer"),
    };

    match process_story_init(&request_str) {
        Ok(response) => safe_string_to_c(response),
        Err(e) => error_response_c(&e.to_string()),
    }
}

/// 주차별 이벤트 처리 (JSON API)
#[no_mangle]
pub extern "C" fn story_process_week_json(
    request_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let request_str = match safe_c_str_to_string(request_json) {
        Some(s) => s,
        None => return error_response_c("Invalid input: null pointer"),
    };

    match process_week_events(&request_str) {
        Ok(response) => safe_string_to_c(response),
        Err(e) => error_response_c(&e.to_string()),
    }
}

/// 선택지 처리 (JSON API)
#[no_mangle]
pub extern "C" fn story_make_choice_json(
    request_json: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let request_str = match safe_c_str_to_string(request_json) {
        Some(s) => s,
        None => return error_response_c("Invalid input: null pointer"),
    };

    match process_choice(&request_str) {
        Ok(response) => safe_string_to_c(response),
        Err(e) => error_response_c(&e.to_string()),
    }
}

/// 메모리 해제 함수
///
/// # Safety
/// 이 함수는 `story_*_json` 함수들이 반환한 포인터만 사용해야 합니다.
/// 동일한 포인터를 두 번 해제하면 undefined behavior가 발생합니다.
#[no_mangle]
pub unsafe extern "C" fn story_free_string(s: *mut std::os::raw::c_char) {
    if s.is_null() {
        return;
    }
    // SAFETY: 이 포인터는 safe_string_to_c()에서 CString::into_raw()로 생성됨
    // 호출자는 이 함수를 한 번만 호출해야 함
    let _ = std::ffi::CString::from_raw(s);
}

// 내부 처리 함수들
fn process_story_init(request_json: &str) -> Result<String, CoreError> {
    let request: StoryInitRequest = serde_json::from_str(request_json)
        .map_err(|e| CoreError::InvalidParameter(format!("Invalid JSON: {}", e)))?;

    // 검증
    if request.schema_version != 1 {
        return Err(CoreError::InvalidParameter(format!(
            "Unsupported schema version: {}",
            request.schema_version
        )));
    }

    // Story Engine 생성 및 초기화
    let mut engine = StoryEngine::new();
    engine.state.player_stats.ca = request.player_ca;
    engine.state.current_week = request.initial_week;

    // 초기 이벤트 생성
    let initial_events = engine.process_week(request.initial_week);
    let current_route = engine.get_current_route();

    // 전역 인스턴스에 저장
    {
        let mut engine_lock = STORY_ENGINE.lock().expect("STORY_ENGINE lock poisoned");
        *engine_lock = Some(engine);
    }

    // 응답 생성
    let response = StoryInitResponse {
        schema_version: 1,
        success: true,
        initial_route: format!("{:?}", current_route),
        initial_events: initial_events.into_iter().map(convert_event_to_json).collect(),
    };

    serde_json::to_string(&response)
        .map_err(|e| CoreError::InvalidParameter(format!("Failed to serialize response: {}", e)))
}

fn process_week_events(request_json: &str) -> Result<String, CoreError> {
    let request: WeekEventRequest = serde_json::from_str(request_json)
        .map_err(|e| CoreError::InvalidParameter(format!("Invalid JSON: {}", e)))?;

    // Engine 가져오기
    let mut engine_lock = STORY_ENGINE.lock().expect("STORY_ENGINE lock poisoned");
    let engine = engine_lock
        .as_mut()
        .ok_or_else(|| CoreError::NotInitialized("Story engine not initialized".into()))?;

    // 주차 업데이트
    engine.state.current_week = request.week;

    // 매치 이벤트 처리
    if let Some(match_events) = request.match_events {
        for event in match_events {
            convert_match_event_to_story_event(&event, engine);
        }
    }

    // 스킬 변화 처리
    if let Some(skill_changes) = request.skill_changes {
        for change in skill_changes {
            // 스킬 마일스톤 이벤트 생성
            if change.new_value > change.old_value && change.new_value % 5 == 0 {
                let milestone_event = EventGenerator::generate_skill_milestone_event(
                    &change.skill_name,
                    change.old_value,
                    change.new_value,
                );
                engine.event_registry.register_event(milestone_event);
            }
        }
    }

    // 분기점 체크
    if let Some(new_route) = engine.check_branch_point(request.week) {
        engine.state.current_route = new_route;
    }

    // 주차별 이벤트 처리
    let events = engine.process_week(request.week);
    let current_route = engine.get_current_route();
    let progress = engine.route_manager.calculate_route_progress(&engine.state);

    // 응답 생성
    let response = WeekEventResponse {
        schema_version: 1,
        events: events.into_iter().map(convert_event_to_json).collect(),
        current_route: format!("{:?}", current_route),
        route_progress: progress,
    };

    serde_json::to_string(&response)
        .map_err(|e| CoreError::InvalidParameter(format!("Failed to serialize response: {}", e)))
}

fn process_choice(request_json: &str) -> Result<String, CoreError> {
    let request: ChoiceRequest = serde_json::from_str(request_json)
        .map_err(|e| CoreError::InvalidParameter(format!("Invalid JSON: {}", e)))?;

    // Engine 가져오기
    let mut engine_lock = STORY_ENGINE.lock().expect("STORY_ENGINE lock poisoned");
    let engine = engine_lock
        .as_mut()
        .ok_or_else(|| CoreError::NotInitialized("Story engine not initialized".into()))?;

    // 선택 전 상태 저장
    let old_ca = engine.state.player_stats.ca;
    let old_morale = engine.state.morale;
    let old_fatigue = engine.state.fatigue;
    let old_relationships = engine.state.relationships.clone();

    engine.make_choice(&request.event_id, request.choice_index)?;

    // 상태 변화 계산
    let ca_change = engine.state.player_stats.ca as i8 - old_ca as i8;
    let morale_change = engine.state.morale - old_morale;
    let fatigue_change = engine.state.fatigue - old_fatigue;

    let relationship_changes: Vec<RelationshipChangeJson> = engine
        .state
        .relationships
        .iter()
        .filter_map(|(character, &new_value)| {
            let old_value = old_relationships.get(character).copied().unwrap_or(0);
            if new_value != old_value {
                Some(RelationshipChangeJson {
                    character: character.clone(),
                    change: new_value - old_value,
                    new_value,
                })
            } else {
                None
            }
        })
        .collect();

    // 응답 생성
    let response = ChoiceResponse {
        schema_version: 1,
        success: true,
        effects_applied: vec![format!("Choice {} applied", request.choice_index)],
        next_event: None,
        state_changes: StateChangesJson {
            ca_change: if ca_change != 0 { Some(ca_change) } else { None },
            morale_change: if morale_change != 0 { Some(morale_change) } else { None },
            fatigue_change: if fatigue_change != 0 { Some(fatigue_change) } else { None },
            relationship_changes,
            flags_changed: vec![],
        },
    };

    serde_json::to_string(&response)
        .map_err(|e| CoreError::InvalidParameter(format!("Failed to serialize response: {}", e)))
}

/// 매치 이벤트를 스토리 이벤트로 변환
fn convert_match_event_to_story_event(match_event: &MatchEventJson, engine: &mut StoryEngine) {
    match match_event.event_type.as_str() {
        "goal" => {
            // 골 이벤트 - 사기 상승, 경기 통계 업데이트
            engine.state.player_stats.goals += 1;
            engine.state.morale = (engine.state.morale + 10).min(100);

            // 해트트릭 체크
            if engine.state.player_stats.goals % 3 == 0 {
                let hat_trick_event = EventGenerator::generate_match_triggered_event(
                    "hat_trick",
                    &format!("해트트릭! {} 골째입니다!", engine.state.player_stats.goals),
                );
                engine.event_registry.register_event(hat_trick_event);
            }
        }
        "assist" => {
            // 어시스트 이벤트
            engine.state.player_stats.assists += 1;
            engine.state.morale = (engine.state.morale + 5).min(100);
        }
        "yellow_card" | "red_card" => {
            // 카드 이벤트 - 사기 감소
            let penalty = if match_event.event_type == "red_card" { 15 } else { 5 };
            engine.state.morale = (engine.state.morale - penalty).max(0);
        }
        "injury" => {
            // 부상 이벤트 - 피로도 증가, 사기 감소
            engine.state.fatigue = (engine.state.fatigue + 30).min(100);
            engine.state.morale = (engine.state.morale - 20).max(0);
        }
        "substitution" => {
            // 교체 이벤트 - 피로도 감소 (하프타임 휴식 등)
            engine.state.fatigue = (engine.state.fatigue - 10).max(0);
        }
        "match_end" => {
            // 경기 종료 - 출전 경기 수 증가, 피로도 증가
            engine.state.player_stats.matches_played += 1;
            engine.state.fatigue = (engine.state.fatigue + 15).min(100);

            // 경기 결과에 따른 사기 변화 (data에서 추출)
            if let Some(result) = match_event.data.get("result").and_then(|r| r.as_str()) {
                match result {
                    "win" => engine.state.morale = (engine.state.morale + 15).min(100),
                    "draw" => {}
                    "loss" => engine.state.morale = (engine.state.morale - 10).max(0),
                    _ => {}
                }
            }
        }
        "man_of_the_match" => {
            // 맨오브더매치 - 사기 대폭 상승
            engine.state.morale = (engine.state.morale + 20).min(100);

            let motm_event = EventGenerator::generate_match_triggered_event(
                "man_of_the_match",
                "오늘 경기의 MVP로 선정되었습니다!",
            );
            engine.event_registry.register_event(motm_event);
        }
        _ => {
            // 알 수 없는 이벤트 타입은 무시
        }
    }
}

// 변환 헬퍼 함수
fn convert_event_to_json(event: StoryEvent) -> StoryEventJson {
    // RequirementValidator를 사용하여 선택지 요구사항 검증
    let engine_lock = STORY_ENGINE.lock().expect("STORY_ENGINE lock poisoned");
    let validator = RequirementValidator::new();

    StoryEventJson {
        id: event.id,
        event_type: format!("{:?}", event.event_type),
        title: event.title,
        description: event.description,
        choices: event
            .choices
            .into_iter()
            .map(|choice| {
                // 엔진 상태가 있으면 요구사항 검증
                let (available, requirement_text) = if let Some(engine) = engine_lock.as_ref() {
                    let is_available =
                        validator.validate_choice_requirements(&choice.requirements, &engine.state);
                    let req_text = if !is_available && !choice.requirements.is_empty() {
                        Some(format_requirements(&choice.requirements))
                    } else {
                        None
                    };
                    (is_available, req_text)
                } else {
                    (true, None) // 엔진이 없으면 기본값
                };

                EventChoiceJson { id: choice.id, text: choice.text, available, requirement_text }
            })
            .collect(),
        priority: format!("{:?}", event.priority),
        tags: event.tags,
    }
}

/// 요구사항을 사람이 읽을 수 있는 형태로 변환
fn format_requirements(requirements: &[ChoiceRequirement]) -> String {
    requirements
        .iter()
        .map(|req| match req {
            ChoiceRequirement::MinCA(ca) => format!("CA {}+ 필요", ca),
            ChoiceRequirement::MaxCA(ca) => format!("CA {} 이하", ca),
            ChoiceRequirement::Personality(p) => format!("{:?} 성격 필요", p),
            ChoiceRequirement::SpecialAbility(name, tier) => {
                format!("{} ({:?}) 능력 필요", name, tier)
            }
            ChoiceRequirement::Relationship(char, val) => {
                format!("{} 호감도 {}+ 필요", char, val)
            }
            ChoiceRequirement::Custom(name, _) => format!("{} 조건 필요", name),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

// JSON 매크로 임포트
use serde_json::json;
