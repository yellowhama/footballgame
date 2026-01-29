//! Story System Module
//!
//! OpenFootball 데이터 기반 동적 스토리 생성 시스템
//! 실시간 매치 데이터를 스토리 이벤트로 변환

pub mod branching;
pub mod conditions;
pub mod effects;
pub mod events;
pub mod localization;
pub mod serialization;
pub mod types;

pub use branching::*;
pub use conditions::*;
pub use effects::*;
pub use events::*;
pub use types::*;

use crate::error::CoreError;

/// Story System 메인 엔진
pub struct StoryEngine {
    /// 현재 스토리 상태
    pub state: StoryState,
    /// 이벤트 레지스트리
    pub event_registry: EventRegistry,
    /// 조건 평가기
    pub condition_evaluator: ConditionEvaluator,
    /// 효과 처리기
    pub effect_processor: EffectProcessor,
    /// 루트 매니저
    pub route_manager: RouteManager,
}

impl Default for StoryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StoryEngine {
    pub fn new() -> Self {
        Self {
            state: StoryState::default(),
            event_registry: EventRegistry::new(),
            condition_evaluator: ConditionEvaluator::new(),
            effect_processor: EffectProcessor::new(),
            route_manager: RouteManager::new(),
        }
    }

    /// 주차별 이벤트 처리
    pub fn process_week(&mut self, week: u32) -> Vec<StoryEvent> {
        let mut events = Vec::new();

        // 1. 고정 이벤트 체크
        events.extend(self.event_registry.get_fixed_events(week));

        // 2. 조건부 이벤트 체크
        for event in self.event_registry.get_conditional_events() {
            if self.condition_evaluator.evaluate_all(&event.conditions, &self.state) {
                events.push(event.clone());
            }
        }

        // 3. 랜덤 이벤트 (20% 확률)
        if rand::random::<f32>() < 0.2 {
            if let Some(event) = self.event_registry.get_random_event() {
                events.push(event);
            }
        }

        events
    }

    /// 선택지 처리
    pub fn make_choice(&mut self, event_id: &str, choice_index: usize) -> Result<(), CoreError> {
        if let Some(event) = self.event_registry.get_event(event_id) {
            if choice_index < event.choices.len() {
                let choice = &event.choices[choice_index];
                self.effect_processor.apply_effects(&choice.effects, &mut self.state)?;
                Ok(())
            } else {
                Err(CoreError::InvalidParameter("Invalid choice index".into()))
            }
        } else {
            Err(CoreError::NotFound(format!("Event {} not found", event_id)))
        }
    }

    /// 현재 루트 계산
    pub fn get_current_route(&self) -> StoryRoute {
        self.state.current_route
    }

    /// 분기점 체크 및 처리
    pub fn check_branch_point(&mut self, week: u32) -> Option<StoryRoute> {
        if self.route_manager.has_branch_point(week) {
            self.route_manager.evaluate_branch_point(week, &self.state, &self.condition_evaluator)
        } else {
            None
        }
    }

    /// 상태 저장
    pub fn save_state(&self) -> StoryState {
        self.state.clone()
    }

    /// 상태 로드
    pub fn load_state(&mut self, state: StoryState) {
        self.state = state;
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod basic_tests {
    use super::*;

    #[test]
    fn test_story_engine_creation() {
        let engine = StoryEngine::new();
        assert_eq!(engine.get_current_route(), StoryRoute::Standard);
    }
}
