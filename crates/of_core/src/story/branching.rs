//! Route Branching System
//!
//! 3-Route System (Elite/Standard/Underdog) 관리 및 분기 처리

use super::types::*;
use std::collections::HashMap;

/// 루트 매니저 - 3-Route System 관리
pub struct RouteManager {
    route_definitions: HashMap<StoryRoute, RouteDefinition>,
    branch_points: Vec<BranchPoint>,
    route_history: Vec<RouteTransition>,
}

impl Default for RouteManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteManager {
    pub fn new() -> Self {
        let mut manager = Self {
            route_definitions: HashMap::new(),
            branch_points: Vec::new(),
            route_history: Vec::new(),
        };
        manager.initialize_routes();
        manager
    }

    /// 루트 정의 초기화
    fn initialize_routes(&mut self) {
        // Elite Route 정의
        self.route_definitions.insert(
            StoryRoute::Elite,
            RouteDefinition {
                route: StoryRoute::Elite,
                ca_range: (140, 200),
                description: "The path of champions and legends".to_string(),
                characteristics: vec![
                    "High-pressure matches".to_string(),
                    "Media attention".to_string(),
                    "Championship contention".to_string(),
                    "International recognition".to_string(),
                ],
                exclusive_events: vec![
                    "elite_media_interview".to_string(),
                    "national_team_callup".to_string(),
                    "ballon_dor_nomination".to_string(),
                ],
                skill_multipliers: SkillMultipliers { technical: 1.2, mental: 1.3, physical: 1.1 },
            },
        );

        // Standard Route 정의
        self.route_definitions.insert(
            StoryRoute::Standard,
            RouteDefinition {
                route: StoryRoute::Standard,
                ca_range: (100, 139),
                description: "The balanced journey of growth".to_string(),
                characteristics: vec![
                    "Steady progression".to_string(),
                    "Team dynamics focus".to_string(),
                    "Local fan support".to_string(),
                    "Consistent playing time".to_string(),
                ],
                exclusive_events: vec![
                    "team_captain_offer".to_string(),
                    "local_hero_status".to_string(),
                    "mentor_younger_players".to_string(),
                ],
                skill_multipliers: SkillMultipliers { technical: 1.1, mental: 1.1, physical: 1.1 },
            },
        );

        // Underdog Route 정의
        self.route_definitions.insert(
            StoryRoute::Underdog,
            RouteDefinition {
                route: StoryRoute::Underdog,
                ca_range: (0, 99),
                description: "The inspiring tale of determination".to_string(),
                characteristics: vec![
                    "Overcoming adversity".to_string(),
                    "Proving doubters wrong".to_string(),
                    "Building from basics".to_string(),
                    "Finding opportunities".to_string(),
                ],
                exclusive_events: vec![
                    "surprise_breakthrough".to_string(),
                    "fan_favorite_underdog".to_string(),
                    "against_all_odds".to_string(),
                ],
                skill_multipliers: SkillMultipliers { technical: 1.0, mental: 1.2, physical: 1.0 },
            },
        );

        // 분기점 정의
        self.initialize_branch_points();
    }

    /// 분기점 초기화
    fn initialize_branch_points(&mut self) {
        // Week 12: 첫 번째 분기점
        self.branch_points.push(BranchPoint {
            week: 12,
            name: "First Assessment".to_string(),
            description: "Your early season performance determines your path".to_string(),
            criteria: vec![
                BranchCriterion {
                    target_route: StoryRoute::Elite,
                    conditions: vec![
                        StoryCondition::CA(ComparisonOp::GreaterEqual, 140),
                        StoryCondition::Goals(ComparisonOp::GreaterEqual, 8),
                    ],
                },
                BranchCriterion {
                    target_route: StoryRoute::Underdog,
                    conditions: vec![
                        StoryCondition::CA(ComparisonOp::Less, 100),
                        StoryCondition::Goals(ComparisonOp::Less, 3),
                    ],
                },
            ],
        });

        // Week 24: 중간 분기점
        self.branch_points.push(BranchPoint {
            week: 24,
            name: "Mid-Season Review".to_string(),
            description: "Halfway through, your trajectory becomes clear".to_string(),
            criteria: vec![
                BranchCriterion {
                    target_route: StoryRoute::Elite,
                    conditions: vec![
                        StoryCondition::CA(ComparisonOp::GreaterEqual, 145),
                        StoryCondition::Goals(ComparisonOp::GreaterEqual, 15),
                        StoryCondition::EventOccurred("hat_trick".to_string()),
                    ],
                },
                BranchCriterion {
                    target_route: StoryRoute::Underdog,
                    conditions: vec![
                        StoryCondition::CA(ComparisonOp::Less, 95),
                        StoryCondition::MatchesPlayed(ComparisonOp::Less, 15),
                    ],
                },
            ],
        });

        // Week 36: 최종 분기점
        self.branch_points.push(BranchPoint {
            week: 36,
            name: "Season Finale".to_string(),
            description: "The final stretch determines your ultimate path".to_string(),
            criteria: vec![BranchCriterion {
                target_route: StoryRoute::Elite,
                conditions: vec![
                    StoryCondition::CA(ComparisonOp::GreaterEqual, 150),
                    StoryCondition::Goals(ComparisonOp::GreaterEqual, 25),
                    StoryCondition::Or(vec![
                        StoryCondition::EventOccurred("championship_won".to_string()),
                        StoryCondition::EventOccurred("golden_boot".to_string()),
                    ]),
                ],
            }],
        });
    }

    /// 현재 주차에 분기점이 있는지 확인
    pub fn has_branch_point(&self, week: u32) -> bool {
        self.branch_points.iter().any(|bp| bp.week == week)
    }

    /// 분기점 평가 및 루트 결정
    pub fn evaluate_branch_point(
        &mut self,
        week: u32,
        state: &StoryState,
        evaluator: &super::conditions::ConditionEvaluator,
    ) -> Option<StoryRoute> {
        if let Some(branch_point) = self.branch_points.iter().find(|bp| bp.week == week) {
            for criterion in &branch_point.criteria {
                if evaluator.evaluate_all(&criterion.conditions, state) {
                    let transition = RouteTransition {
                        from_route: state.current_route,
                        to_route: criterion.target_route,
                        week,
                        reason: branch_point.name.clone(),
                    };
                    self.route_history.push(transition);
                    return Some(criterion.target_route);
                }
            }
        }
        None
    }

    /// 루트별 이벤트 필터링
    pub fn filter_events_for_route(
        &self,
        events: Vec<StoryEvent>,
        route: StoryRoute,
    ) -> Vec<StoryEvent> {
        events
            .into_iter()
            .filter(|event| {
                // 루트별 전용 이벤트 체크
                if let Some(route_def) = self.route_definitions.get(&route) {
                    if route_def.exclusive_events.contains(&event.id) {
                        return true;
                    }
                }

                // 루트 조건이 있는 이벤트 체크
                !event.conditions.iter().any(|cond| {
                    if let StoryCondition::Route(required_route) = cond {
                        *required_route != route
                    } else {
                        false
                    }
                })
            })
            .collect()
    }

    /// 루트 전환 이벤트 생성
    pub fn generate_transition_event(&self, from: StoryRoute, to: StoryRoute) -> StoryEvent {
        let title = match (from, to) {
            (StoryRoute::Standard, StoryRoute::Elite) => "Rising to Elite!",
            (StoryRoute::Underdog, StoryRoute::Standard) => "Breaking Through!",
            (StoryRoute::Underdog, StoryRoute::Elite) => "Miracle Ascension!",
            (StoryRoute::Elite, StoryRoute::Standard) => "Adjusting Expectations",
            (StoryRoute::Standard, StoryRoute::Underdog) => "Facing Challenges",
            _ => "Path Change",
        };

        StoryEvent {
            id: format!("route_transition_{}_{}", from as u8, to as u8),
            event_type: StoryEventType::Route,
            title: title.to_string(),
            description: format!(
                "Your journey takes a new direction as you transition from {:?} to {:?}",
                from, to
            ),
            choices: vec![
                EventChoice {
                    id: "accept".to_string(),
                    text: "Embrace the new path".to_string(),
                    requirements: vec![],
                    effects: vec![
                        StoryEffect::ModifyMorale(10),
                        StoryEffect::SetFlag(format!("route_{:?}_entered", to), true),
                    ],
                    next_event_id: None,
                },
                EventChoice {
                    id: "determined".to_string(),
                    text: "Prove yourself on this path".to_string(),
                    requirements: vec![],
                    effects: vec![StoryEffect::ModifyCA(2), StoryEffect::ModifyMorale(5)],
                    next_event_id: None,
                },
            ],
            conditions: vec![],
            week_range: None,
            priority: EventPriority::Critical,
            tags: vec!["route_change".to_string()],
        }
    }

    /// 루트별 성장률 계산
    pub fn calculate_growth_modifier(&self, route: StoryRoute, skill_type: SkillType) -> f32 {
        self.route_definitions
            .get(&route)
            .map(|def| match skill_type {
                SkillType::Technical => def.skill_multipliers.technical,
                SkillType::Mental => def.skill_multipliers.mental,
                SkillType::Physical => def.skill_multipliers.physical,
            })
            .unwrap_or(1.0)
    }

    /// 루트 진행도 계산 (0.0 ~ 1.0)
    pub fn calculate_route_progress(&self, state: &StoryState) -> f32 {
        let route_def = match self.route_definitions.get(&state.current_route) {
            Some(def) => def,
            None => return 0.0,
        };

        let ca_progress = {
            let (min_ca, max_ca) = route_def.ca_range;
            let ca = state.player_stats.ca as f32;
            ((ca - min_ca as f32) / (max_ca - min_ca) as f32).clamp(0.0, 1.0)
        };

        // 목표 달성도 (예: 골 수)
        let goal_progress = match state.current_route {
            StoryRoute::Elite => (state.player_stats.goals as f32 / 30.0).min(1.0),
            StoryRoute::Standard => (state.player_stats.goals as f32 / 20.0).min(1.0),
            StoryRoute::Underdog => (state.player_stats.goals as f32 / 10.0).min(1.0),
        };

        // 가중 평균
        ca_progress * 0.6 + goal_progress * 0.4
    }
}

/// 루트 정의
#[derive(Debug, Clone)]
struct RouteDefinition {
    route: StoryRoute,
    ca_range: (u8, u8),
    description: String,
    characteristics: Vec<String>,
    exclusive_events: Vec<String>,
    skill_multipliers: SkillMultipliers,
}

/// 스킬 성장 배수
#[derive(Debug, Clone)]
struct SkillMultipliers {
    technical: f32,
    mental: f32,
    physical: f32,
}

/// 스킬 타입
#[derive(Debug, Clone)]
pub enum SkillType {
    Technical,
    Mental,
    Physical,
}

/// 분기점
#[derive(Debug, Clone)]
struct BranchPoint {
    week: u32,
    name: String,
    description: String,
    criteria: Vec<BranchCriterion>,
}

/// 분기 기준
#[derive(Debug, Clone)]
struct BranchCriterion {
    target_route: StoryRoute,
    conditions: Vec<StoryCondition>,
}

/// 루트 전환 기록
#[derive(Debug, Clone)]
struct RouteTransition {
    from_route: StoryRoute,
    to_route: StoryRoute,
    week: u32,
    reason: String,
}

/// 루트 예측기 - AI 기반 루트 예측
pub struct RoutePredictor;

impl RoutePredictor {
    /// 현재 상태로 미래 루트 예측
    pub fn predict_route(&self, state: &StoryState, weeks_ahead: u32) -> RoutePrediction {
        let current_ca = state.player_stats.ca;
        let goals_per_week = if state.current_week > 0 {
            state.player_stats.goals as f32 / state.current_week as f32
        } else {
            0.0
        };

        // 간단한 선형 예측 (실제로는 ML 모델 사용)
        let predicted_goals =
            state.player_stats.goals + (goals_per_week * weeks_ahead as f32) as u32;
        let ca_growth_rate = 0.5; // 주당 평균 CA 성장
        let predicted_ca =
            (current_ca as f32 + ca_growth_rate * weeks_ahead as f32).min(200.0) as u8;

        let predicted_route = match predicted_ca {
            140..=255 => StoryRoute::Elite,
            100..=139 => StoryRoute::Standard,
            _ => StoryRoute::Underdog,
        };

        let confidence = self.calculate_confidence(state, predicted_route);

        RoutePrediction {
            likely_route: predicted_route,
            confidence,
            key_factors: vec![
                format!("Predicted CA: {}", predicted_ca),
                format!("Predicted Goals: {}", predicted_goals),
            ],
        }
    }

    /// 예측 신뢰도 계산
    fn calculate_confidence(&self, state: &StoryState, predicted_route: StoryRoute) -> f32 {
        // CA가 경계에 가까울수록 신뢰도 낮음
        let ca_distance_to_boundary = match (state.player_stats.ca, predicted_route) {
            (ca, StoryRoute::Elite) if ca >= 140 => (ca - 140).min(10) as f32 / 10.0,
            (ca, StoryRoute::Standard) if (100..140).contains(&ca) => {
                let dist_to_lower = (ca - 100) as f32;
                let dist_to_upper = (140 - ca) as f32;
                dist_to_lower.min(dist_to_upper) / 20.0
            }
            (ca, StoryRoute::Underdog) if ca < 100 => (100 - ca).min(20) as f32 / 20.0,
            _ => 0.5,
        };

        ca_distance_to_boundary.clamp(0.3, 0.9)
    }
}

/// 루트 예측 결과
#[derive(Debug, Clone)]
pub struct RoutePrediction {
    pub likely_route: StoryRoute,
    pub confidence: f32,
    pub key_factors: Vec<String>,
}

/// 루트별 컨텐츠 생성기
pub struct RouteContentGenerator;

impl RouteContentGenerator {
    /// 루트별 대화 스타일 생성
    pub fn generate_dialogue_style(
        &self,
        route: StoryRoute,
        character_role: CharacterRole,
    ) -> String {
        match (route, character_role) {
            (StoryRoute::Elite, CharacterRole::Coach) => {
                "Professional and demanding, expecting excellence".to_string()
            }
            (StoryRoute::Elite, CharacterRole::Media) => {
                "Intense scrutiny and high expectations".to_string()
            }
            (StoryRoute::Standard, CharacterRole::Coach) => {
                "Supportive and constructive, focused on improvement".to_string()
            }
            (StoryRoute::Standard, CharacterRole::Teammate) => {
                "Friendly competition and mutual respect".to_string()
            }
            (StoryRoute::Underdog, CharacterRole::Coach) => {
                "Encouraging and patient, building confidence".to_string()
            }
            (StoryRoute::Underdog, CharacterRole::Fan) => {
                "Passionate support for the underdog story".to_string()
            }
            _ => "Neutral and professional".to_string(),
        }
    }

    /// 루트별 훈련 강도 조정
    pub fn adjust_training_intensity(&self, route: StoryRoute, base_intensity: f32) -> f32 {
        match route {
            StoryRoute::Elite => base_intensity * 1.3,    // 높은 강도
            StoryRoute::Standard => base_intensity * 1.1, // 보통 강도
            StoryRoute::Underdog => base_intensity * 0.9, // 낮은 강도, 기초 중심
        }
    }

    /// 루트별 리워드 계산
    pub fn calculate_route_rewards(&self, route: StoryRoute, base_reward: i32) -> RouteRewards {
        match route {
            StoryRoute::Elite => RouteRewards {
                ca_bonus: (base_reward as f32 * 1.5) as i32,
                morale_bonus: base_reward / 2,
                special_unlock_chance: 0.3,
                exclusive_item_chance: 0.2,
            },
            StoryRoute::Standard => RouteRewards {
                ca_bonus: base_reward,
                morale_bonus: base_reward,
                special_unlock_chance: 0.15,
                exclusive_item_chance: 0.1,
            },
            StoryRoute::Underdog => RouteRewards {
                ca_bonus: (base_reward as f32 * 0.8) as i32,
                morale_bonus: (base_reward as f32 * 1.5) as i32,
                special_unlock_chance: 0.1,
                exclusive_item_chance: 0.05,
            },
        }
    }
}

/// 루트별 리워드 구조
#[derive(Debug, Clone)]
pub struct RouteRewards {
    pub ca_bonus: i32,
    pub morale_bonus: i32,
    pub special_unlock_chance: f32,
    pub exclusive_item_chance: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_manager_initialization() {
        let manager = RouteManager::new();
        assert_eq!(manager.route_definitions.len(), 3);
        assert!(!manager.branch_points.is_empty());
    }

    #[test]
    fn test_branch_point_detection() {
        let manager = RouteManager::new();
        assert!(manager.has_branch_point(12));
        assert!(manager.has_branch_point(24));
        assert!(manager.has_branch_point(36));
        assert!(!manager.has_branch_point(15));
    }

    #[test]
    fn test_route_prediction() {
        let predictor = RoutePredictor;
        let mut state = StoryState::default();
        state.player_stats.ca = 120;
        state.player_stats.goals = 10;
        state.current_week = 10;

        let prediction = predictor.predict_route(&state, 10);
        assert_eq!(prediction.likely_route, StoryRoute::Standard);
        assert!(prediction.confidence > 0.0 && prediction.confidence <= 1.0);
    }

    #[test]
    fn test_content_generation() {
        let generator = RouteContentGenerator;

        let dialogue = generator.generate_dialogue_style(StoryRoute::Elite, CharacterRole::Coach);
        assert!(dialogue.contains("excellence"));

        let intensity = generator.adjust_training_intensity(StoryRoute::Elite, 1.0);
        assert!(intensity > 1.0);

        let rewards = generator.calculate_route_rewards(StoryRoute::Underdog, 100);
        assert!(rewards.morale_bonus > rewards.ca_bonus);
    }
}
