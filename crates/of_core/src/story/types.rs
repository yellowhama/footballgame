//! Story System Core Types
//!
//! OpenFootball 데이터 기반 스토리 이벤트 타입 정의
//! PersonAttributes와 SpecialAbility 시스템 통합

use crate::player::personality::PersonalityArchetype;
use crate::special_ability::types::AbilityTier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 스토리 이벤트 기본 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryEvent {
    pub id: String,
    pub event_type: StoryEventType,
    pub title: String,
    pub description: String,
    pub choices: Vec<EventChoice>,
    pub conditions: Vec<StoryCondition>,
    pub week_range: Option<(u32, u32)>,
    pub priority: EventPriority,
    pub tags: Vec<String>,
}

/// 이벤트 타입 - OpenFootball 매치 데이터와 연동
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StoryEventType {
    MatchTriggered, // 경기 중 발생 (골, 어시스트, 레드카드 등)
    SkillMilestone, // 스킬 레벨업/마일스톤
    Relationship,   // 관계 변화
    Training,       // 훈련 관련
    Fixed,          // 고정 시점 이벤트
    Random,         // 랜덤 이벤트
    Route,          // 루트 분기
}

/// 이벤트 우선순위
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Ord, PartialOrd, Eq)]
pub enum EventPriority {
    Critical, // 루트 분기 등 중요 이벤트
    High,     // 주요 스토리 이벤트
    Normal,   // 일반 이벤트
    Low,      // 부가 이벤트
}

/// 선택지 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventChoice {
    pub id: String,
    pub text: String,
    pub requirements: Vec<ChoiceRequirement>,
    pub effects: Vec<StoryEffect>,
    pub next_event_id: Option<String>,
}

/// 선택지 요구사항
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChoiceRequirement {
    MinCA(u8),
    MaxCA(u8),
    Personality(PersonalityArchetype),
    SpecialAbility(String, AbilityTier),
    Relationship(String, i32),
    Custom(String, serde_json::Value),
}

/// 스토리 조건
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoryCondition {
    Week(u32),
    WeekRange(u32, u32),
    CA(ComparisonOp, u8),
    Goals(ComparisonOp, u32),
    Assists(ComparisonOp, u32),
    MatchesPlayed(ComparisonOp, u32),
    Route(StoryRoute),
    Personality(PersonalityArchetype),
    HasSpecialAbility(String),
    Relationship(String, ComparisonOp, i32),
    EventOccurred(String),
    EventNotOccurred(String),
    Random(f32),
    And(Vec<StoryCondition>),
    Or(Vec<StoryCondition>),
    Not(Box<StoryCondition>),
}

/// 비교 연산자
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}

/// 스토리 루트 (3-Route System)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StoryRoute {
    Elite,    // CA 140+ 엘리트 루트
    Standard, // CA 100-139 표준 루트
    Underdog, // CA <100 언더독 루트
}

/// 스토리 효과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoryEffect {
    ModifyCA(i8),
    ModifySkill(String, i8),
    ModifyRelationship(String, i32),
    UnlockSpecialAbility(String),
    SetPersonality(PersonalityArchetype),
    TriggerEvent(String),
    SetFlag(String, bool),
    ModifyMorale(i32),
    ModifyFatigue(i32),
    Custom(String, serde_json::Value),
}

/// 스토리 상태 관리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryState {
    pub current_week: u32,
    pub current_route: StoryRoute,
    pub occurred_events: Vec<String>,
    pub active_flags: HashMap<String, bool>,
    pub relationships: HashMap<String, i32>,
    pub player_stats: PlayerStoryStats,
    /// 선수 사기 (0-100)
    pub morale: i32,
    /// 선수 피로도 (0-100)
    pub fatigue: i32,
}

impl Default for StoryState {
    fn default() -> Self {
        Self {
            current_week: 1,
            current_route: StoryRoute::Standard,
            occurred_events: Vec::new(),
            active_flags: HashMap::new(),
            relationships: HashMap::new(),
            player_stats: PlayerStoryStats::default(),
            morale: 50, // 중립 상태에서 시작
            fatigue: 0, // 피로 없이 시작
        }
    }
}

/// 플레이어 스토리 관련 통계
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerStoryStats {
    pub ca: u8,
    pub goals: u32,
    pub assists: u32,
    pub matches_played: u32,
    pub training_sessions: u32,
    pub special_moments: Vec<SpecialMoment>,
}

/// 특별한 순간 기록
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialMoment {
    pub week: u32,
    pub moment_type: MomentType,
    pub description: String,
    pub impact: f32,
}

/// 특별한 순간 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MomentType {
    FirstGoal,
    HatTrick,
    PerfectPerformance,
    ComebackVictory,
    TitleWin,
    SkillBreakthrough,
    RelationshipMilestone,
}

/// 캐릭터 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryCharacter {
    pub id: String,
    pub name: String,
    pub role: CharacterRole,
    pub personality: PersonalityArchetype,
    pub relationship_level: i32,
    pub dialogue_style: DialogueStyle,
}

/// 캐릭터 역할
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CharacterRole {
    Coach,
    Teammate,
    Rival,
    Mentor,
    Friend,
    Family,
    Media,
    Fan,
}

/// 대화 스타일
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueStyle {
    Formal,
    Casual,
    Motivational,
    Critical,
    Supportive,
    Analytical,
}

/// 이벤트 레지스트리
pub struct EventRegistry {
    events: HashMap<String, StoryEvent>,
    fixed_events: HashMap<u32, Vec<StoryEvent>>,
    conditional_events: Vec<StoryEvent>,
    random_events: Vec<StoryEvent>,
}

impl Default for EventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            fixed_events: HashMap::new(),
            conditional_events: Vec::new(),
            random_events: Vec::new(),
        }
    }

    pub fn register_event(&mut self, event: StoryEvent) {
        let id = event.id.clone();

        match event.event_type {
            StoryEventType::Fixed => {
                if let Some((start_week, _)) = event.week_range {
                    self.fixed_events.entry(start_week).or_default().push(event.clone());
                }
            }
            StoryEventType::Random => {
                self.random_events.push(event.clone());
            }
            _ => {
                self.conditional_events.push(event.clone());
            }
        }

        self.events.insert(id, event);
    }

    pub fn get_event(&self, id: &str) -> Option<&StoryEvent> {
        self.events.get(id)
    }

    pub fn get_fixed_events(&self, week: u32) -> Vec<StoryEvent> {
        self.fixed_events.get(&week).cloned().unwrap_or_default()
    }

    pub fn get_conditional_events(&self) -> &[StoryEvent] {
        &self.conditional_events
    }

    pub fn get_random_event(&self) -> Option<StoryEvent> {
        use rand::seq::SliceRandom;
        self.random_events.choose(&mut rand::thread_rng()).cloned()
    }
}

/// 루트 계산기
pub struct RouteCalculator;

impl RouteCalculator {
    pub fn calculate(state: &StoryState) -> StoryRoute {
        let ca = state.player_stats.ca;
        let performance_factor = Self::calculate_performance_factor(state);

        let adjusted_ca = (ca as f32 * performance_factor) as u8;

        match adjusted_ca {
            140..=255 => StoryRoute::Elite,
            100..=139 => StoryRoute::Standard,
            _ => StoryRoute::Underdog,
        }
    }

    fn calculate_performance_factor(state: &StoryState) -> f32 {
        let goals_per_match = if state.player_stats.matches_played > 0 {
            state.player_stats.goals as f32 / state.player_stats.matches_played as f32
        } else {
            0.0
        };

        let assists_per_match = if state.player_stats.matches_played > 0 {
            state.player_stats.assists as f32 / state.player_stats.matches_played as f32
        } else {
            0.0
        };

        // 성과 기반 보정 (0.8 ~ 1.2)
        let base_factor = 1.0;
        let goals_bonus = (goals_per_match * 0.2).min(0.1);
        let assists_bonus = (assists_per_match * 0.15).min(0.1);

        (base_factor + goals_bonus + assists_bonus).clamp(0.8, 1.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_calculation() {
        let mut state = StoryState::default();

        // Test Elite route
        state.player_stats.ca = 150;
        assert_eq!(RouteCalculator::calculate(&state), StoryRoute::Elite);

        // Test Standard route
        state.player_stats.ca = 120;
        assert_eq!(RouteCalculator::calculate(&state), StoryRoute::Standard);

        // Test Underdog route
        state.player_stats.ca = 80;
        assert_eq!(RouteCalculator::calculate(&state), StoryRoute::Underdog);
    }

    #[test]
    fn test_event_registry() {
        let mut registry = EventRegistry::new();

        let event = StoryEvent {
            id: "test_event".to_string(),
            event_type: StoryEventType::Fixed,
            title: "Test Event".to_string(),
            description: "A test event".to_string(),
            choices: vec![],
            conditions: vec![],
            week_range: Some((5, 10)),
            priority: EventPriority::Normal,
            tags: vec![],
        };

        registry.register_event(event);

        assert!(registry.get_event("test_event").is_some());
        assert!(!registry.get_fixed_events(5).is_empty());
    }
}
