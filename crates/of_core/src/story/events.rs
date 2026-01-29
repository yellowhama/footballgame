//! Event Processing Module
//!
//! OpenFootball 매치 이벤트를 스토리 이벤트로 변환하고 처리

use super::types::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// 이벤트 프로세서 - 매치 데이터를 스토리 이벤트로 변환
pub struct EventProcessor {
    event_queue: VecDeque<StoryEvent>,
    event_history: Vec<ProcessedEvent>,
    match_event_converter: MatchEventConverter,
}

impl Default for EventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl EventProcessor {
    pub fn new() -> Self {
        Self {
            event_queue: VecDeque::new(),
            event_history: Vec::new(),
            match_event_converter: MatchEventConverter::new(),
        }
    }

    /// 매치 이벤트 처리
    pub fn process_match_events(&mut self, match_events: &[MatchEvent]) -> Vec<StoryEvent> {
        let mut story_events = Vec::new();

        for match_event in match_events {
            if let Some(story_event) = self.match_event_converter.convert(match_event) {
                story_events.push(story_event);
            }
        }

        story_events
    }

    /// 이벤트 큐에 추가
    pub fn queue_event(&mut self, event: StoryEvent) {
        self.event_queue.push_back(event);
    }

    /// 다음 이벤트 가져오기
    pub fn get_next_event(&mut self) -> Option<StoryEvent> {
        self.event_queue.pop_front()
    }

    /// 이벤트 처리 기록
    pub fn record_event(&mut self, event: &StoryEvent, choice_index: Option<usize>) {
        self.event_history.push(ProcessedEvent {
            event_id: event.id.clone(),
            week: 0, // Should be set from state
            choice_index,
            timestamp: chrono::Utc::now().timestamp(),
        });
    }

    /// 이벤트 발생 여부 확인
    pub fn has_event_occurred(&self, event_id: &str) -> bool {
        self.event_history.iter().any(|e| e.event_id == event_id)
    }

    /// 특정 주차의 이벤트 가져오기
    pub fn get_events_for_week(&self, week: u32) -> Vec<&ProcessedEvent> {
        self.event_history.iter().filter(|e| e.week == week).collect()
    }
}

/// 처리된 이벤트 기록
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedEvent {
    pub event_id: String,
    pub week: u32,
    pub choice_index: Option<usize>,
    pub timestamp: i64,
}

/// OpenFootball 매치 이벤트 (임시 정의 - 실제는 OpenFootball에서 가져옴)
#[derive(Debug, Clone)]
pub struct MatchEvent {
    pub event_type: MatchEventType,
    pub minute: u8,
    pub player_id: String,
    pub data: MatchEventData,
}

#[derive(Debug, Clone)]
pub enum MatchEventType {
    Goal,
    Assist,
    YellowCard,
    RedCard,
    Substitution,
    Injury,
    PenaltyScored,
    PenaltyMissed,
    SaveMade,
}

#[derive(Debug, Clone)]
pub enum MatchEventData {
    Goal { scorer: String, assister: Option<String> },
    Card { reason: String },
    Substitution { player_in: String, player_out: String },
    Injury { severity: u8 },
    Penalty { success: bool },
}

/// 매치 이벤트를 스토리 이벤트로 변환
pub struct MatchEventConverter {
    conversion_rules: Vec<ConversionRule>,
}

impl Default for MatchEventConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl MatchEventConverter {
    pub fn new() -> Self {
        let mut converter = Self { conversion_rules: Vec::new() };
        converter.initialize_rules();
        converter
    }

    fn initialize_rules(&mut self) {
        // 첫 골 규칙
        self.conversion_rules.push(ConversionRule {
            match_type: MatchEventType::Goal,
            condition: Box::new(|event, context| context.is_first_goal && event.minute < 10),
            converter: Box::new(|event, _context| StoryEvent {
                id: format!("early_goal_{}", event.minute),
                event_type: StoryEventType::MatchTriggered,
                title: "Early Strike!".to_string(),
                description: "You scored in the opening minutes!".to_string(),
                choices: vec![
                    EventChoice {
                        id: "celebrate".to_string(),
                        text: "Celebrate with the fans".to_string(),
                        requirements: vec![],
                        effects: vec![StoryEffect::ModifyMorale(10)],
                        next_event_id: None,
                    },
                    EventChoice {
                        id: "focus".to_string(),
                        text: "Stay focused on the game".to_string(),
                        requirements: vec![],
                        effects: vec![StoryEffect::ModifyCA(1)],
                        next_event_id: None,
                    },
                ],
                conditions: vec![],
                week_range: None,
                priority: EventPriority::High,
                tags: vec!["goal".to_string(), "early".to_string()],
            }),
        });

        // 해트트릭 규칙
        self.conversion_rules.push(ConversionRule {
            match_type: MatchEventType::Goal,
            condition: Box::new(|_event, context| context.goals_in_match == 3),
            converter: Box::new(|_event, _context| StoryEvent {
                id: "hat_trick".to_string(),
                event_type: StoryEventType::MatchTriggered,
                title: "Hat Trick Hero!".to_string(),
                description: "You've scored three goals in a single match!".to_string(),
                choices: vec![
                    EventChoice {
                        id: "dedicate".to_string(),
                        text: "Dedicate to family".to_string(),
                        requirements: vec![],
                        effects: vec![
                            StoryEffect::ModifyRelationship("family".to_string(), 20),
                            StoryEffect::ModifyMorale(15),
                        ],
                        next_event_id: None,
                    },
                    EventChoice {
                        id: "team_credit".to_string(),
                        text: "Credit the team".to_string(),
                        requirements: vec![],
                        effects: vec![
                            StoryEffect::ModifyRelationship("teammates".to_string(), 15),
                            StoryEffect::ModifyCA(2),
                        ],
                        next_event_id: None,
                    },
                ],
                conditions: vec![],
                week_range: None,
                priority: EventPriority::Critical,
                tags: vec!["hat_trick".to_string(), "milestone".to_string()],
            }),
        });

        // 레드카드 규칙
        self.conversion_rules.push(ConversionRule {
            match_type: MatchEventType::RedCard,
            condition: Box::new(|_event, _context| true),
            converter: Box::new(|_event, _context| StoryEvent {
                id: "red_card_received".to_string(),
                event_type: StoryEventType::MatchTriggered,
                title: "Sent Off!".to_string(),
                description: "You received a red card and were sent off.".to_string(),
                choices: vec![
                    EventChoice {
                        id: "apologize".to_string(),
                        text: "Apologize to team".to_string(),
                        requirements: vec![],
                        effects: vec![
                            StoryEffect::ModifyRelationship("coach".to_string(), -10),
                            StoryEffect::ModifyRelationship("teammates".to_string(), 5),
                        ],
                        next_event_id: None,
                    },
                    EventChoice {
                        id: "defend".to_string(),
                        text: "Defend your actions".to_string(),
                        requirements: vec![],
                        effects: vec![
                            StoryEffect::ModifyRelationship("coach".to_string(), -20),
                            StoryEffect::ModifyMorale(-10),
                        ],
                        next_event_id: None,
                    },
                ],
                conditions: vec![],
                week_range: None,
                priority: EventPriority::High,
                tags: vec!["red_card".to_string(), "discipline".to_string()],
            }),
        });
    }

    pub fn convert(&self, match_event: &MatchEvent) -> Option<StoryEvent> {
        let context = self.create_context(match_event);

        for rule in &self.conversion_rules {
            if rule.matches(match_event, &context) {
                return Some((rule.converter)(match_event, &context));
            }
        }

        None
    }

    fn create_context(&self, _match_event: &MatchEvent) -> ConversionContext {
        // 실제 구현에서는 현재 경기 상태를 추적
        ConversionContext {
            is_first_goal: false, // Should track from match state
            goals_in_match: 0,    // Should track from match state
            current_score: (0, 0),
            is_home: true,
            is_derby: false,
            is_cup_final: false,
        }
    }
}

/// 변환 컨텍스트
#[derive(Debug, Clone)]
pub struct ConversionContext {
    pub is_first_goal: bool,
    pub goals_in_match: u32,
    pub current_score: (u32, u32),
    pub is_home: bool,
    pub is_derby: bool,
    pub is_cup_final: bool,
}

/// 변환 규칙
struct ConversionRule {
    match_type: MatchEventType,
    condition: Box<dyn Fn(&MatchEvent, &ConversionContext) -> bool>,
    converter: Box<dyn Fn(&MatchEvent, &ConversionContext) -> StoryEvent>,
}

impl ConversionRule {
    fn matches(&self, event: &MatchEvent, context: &ConversionContext) -> bool {
        std::mem::discriminant(&event.event_type) == std::mem::discriminant(&self.match_type)
            && (self.condition)(event, context)
    }
}

/// 이벤트 생성기 - 동적 이벤트 생성
#[allow(dead_code)]
pub struct EventGenerator {
    templates: Vec<EventTemplate>,
}

impl Default for EventGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl EventGenerator {
    pub fn new() -> Self {
        Self { templates: Vec::new() }
    }

    /// 스킬 마일스톤 이벤트 생성
    pub fn generate_skill_milestone_event(
        skill_name: &str,
        old_level: u8,
        new_level: u8,
    ) -> StoryEvent {
        StoryEvent {
            id: format!("skill_milestone_{}_{}", skill_name, new_level),
            event_type: StoryEventType::SkillMilestone,
            title: format!("{} Breakthrough!", skill_name),
            description: format!(
                "Your {} skill has improved from {} to {}!",
                skill_name, old_level, new_level
            ),
            choices: vec![
                EventChoice {
                    id: "thank_coach".to_string(),
                    text: "Thank the coach for training".to_string(),
                    requirements: vec![],
                    effects: vec![StoryEffect::ModifyRelationship("coach".to_string(), 10)],
                    next_event_id: None,
                },
                EventChoice {
                    id: "train_harder".to_string(),
                    text: "Push yourself even harder".to_string(),
                    requirements: vec![],
                    effects: vec![
                        StoryEffect::ModifyFatigue(20),
                        StoryEffect::ModifySkill(skill_name.to_string(), 2),
                    ],
                    next_event_id: None,
                },
            ],
            conditions: vec![],
            week_range: None,
            priority: EventPriority::Normal,
            tags: vec!["skill".to_string(), "milestone".to_string()],
        }
    }

    /// 관계 이벤트 생성
    pub fn generate_relationship_event(
        character_id: &str,
        relationship_level: i32,
    ) -> Option<StoryEvent> {
        match relationship_level {
            50 => Some(StoryEvent {
                id: format!("relationship_milestone_{}_50", character_id),
                event_type: StoryEventType::Relationship,
                title: "Growing Bond".to_string(),
                description: format!("Your relationship with {} is strengthening.", character_id),
                choices: vec![EventChoice {
                    id: "acknowledge".to_string(),
                    text: "Appreciate the friendship".to_string(),
                    requirements: vec![],
                    effects: vec![StoryEffect::ModifyMorale(5)],
                    next_event_id: None,
                }],
                conditions: vec![],
                week_range: None,
                priority: EventPriority::Low,
                tags: vec!["relationship".to_string()],
            }),
            100 => Some(StoryEvent {
                id: format!("relationship_milestone_{}_100", character_id),
                event_type: StoryEventType::Relationship,
                title: "True Friendship".to_string(),
                description: format!("You and {} have become close friends.", character_id),
                choices: vec![EventChoice {
                    id: "celebrate".to_string(),
                    text: "Celebrate the friendship".to_string(),
                    requirements: vec![],
                    effects: vec![
                        StoryEffect::ModifyMorale(10),
                        StoryEffect::UnlockSpecialAbility("Team Player".to_string()),
                    ],
                    next_event_id: None,
                }],
                conditions: vec![],
                week_range: None,
                priority: EventPriority::High,
                tags: vec!["relationship".to_string(), "milestone".to_string()],
            }),
            _ => None,
        }
    }

    /// 매치 트리거 이벤트 생성 (골, 해트트릭, MVP 등)
    pub fn generate_match_triggered_event(event_id: &str, description: &str) -> StoryEvent {
        let (title, priority, effects) = match event_id {
            "hat_trick" => (
                "해트트릭!".to_string(),
                EventPriority::High,
                vec![StoryEffect::ModifyMorale(20), StoryEffect::ModifyCA(1)],
            ),
            "man_of_the_match" => (
                "경기의 MVP!".to_string(),
                EventPriority::High,
                vec![StoryEffect::ModifyMorale(15)],
            ),
            "first_goal" => {
                ("첫 골!".to_string(), EventPriority::Normal, vec![StoryEffect::ModifyMorale(10)])
            }
            "comeback_win" => {
                ("역전승!".to_string(), EventPriority::High, vec![StoryEffect::ModifyMorale(25)])
            }
            _ => {
                ("경기 이벤트".to_string(), EventPriority::Low, vec![StoryEffect::ModifyMorale(5)])
            }
        };

        StoryEvent {
            id: format!("match_{}_{}", event_id, chrono::Utc::now().timestamp_millis()),
            event_type: StoryEventType::MatchTriggered,
            title,
            description: description.to_string(),
            choices: vec![EventChoice {
                id: "acknowledge".to_string(),
                text: "계속하기".to_string(),
                requirements: vec![],
                effects,
                next_event_id: None,
            }],
            conditions: vec![],
            week_range: None,
            priority,
            tags: vec!["match".to_string(), event_id.to_string()],
        }
    }
}

/// 이벤트 템플릿
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTemplate {
    pub id: String,
    pub template_type: String,
    pub base_event: StoryEvent,
    pub variations: Vec<EventVariation>,
}

/// 이벤트 변형
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventVariation {
    pub condition: StoryCondition,
    pub modifications: EventModifications,
}

/// 이벤트 수정사항
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventModifications {
    pub title_suffix: Option<String>,
    pub description_override: Option<String>,
    pub additional_choices: Vec<EventChoice>,
    pub additional_effects: Vec<StoryEffect>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_processor() {
        let mut processor = EventProcessor::new();

        let event = StoryEvent {
            id: "test".to_string(),
            event_type: StoryEventType::Random,
            title: "Test".to_string(),
            description: "Test event".to_string(),
            choices: vec![],
            conditions: vec![],
            week_range: None,
            priority: EventPriority::Normal,
            tags: vec![],
        };

        processor.queue_event(event.clone());
        assert!(processor.get_next_event().is_some());
        assert!(processor.get_next_event().is_none());
    }

    #[test]
    fn test_skill_milestone_generation() {
        let event = EventGenerator::generate_skill_milestone_event("Passing", 10, 15);
        assert_eq!(event.event_type, StoryEventType::SkillMilestone);
        assert!(event.title.contains("Passing"));
        assert_eq!(event.choices.len(), 2);
    }
}
