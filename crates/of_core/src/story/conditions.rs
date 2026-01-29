//! Condition Evaluation Module
//!
//! 스토리 조건 평가 및 검증

use super::types::*;
use std::collections::HashMap;
use std::sync::Arc;

/// 조건 평가기
pub struct ConditionEvaluator {
    custom_evaluators:
        HashMap<String, Arc<dyn Fn(&serde_json::Value, &StoryState) -> bool + Send + Sync>>,
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionEvaluator {
    pub fn new() -> Self {
        let mut evaluator = Self { custom_evaluators: HashMap::new() };
        evaluator.register_default_evaluators();
        evaluator
    }

    /// 기본 커스텀 평가기 등록
    fn register_default_evaluators(&mut self) {
        // 경기 연승 체크
        self.register_custom_evaluator(
            "winning_streak",
            Arc::new(|value, _state| {
                if let Some(streak) = value.as_u64() {
                    // 실제 구현에서는 state에서 연승 정보를 가져와야 함
                    streak >= 3
                } else {
                    false
                }
            }),
        );

        // 특정 스킬 조합 체크
        self.register_custom_evaluator(
            "has_skill_combo",
            Arc::new(|value, _state| {
                if let Some(combo) = value.as_str() {
                    // 실제 구현에서는 스킬 조합 체크
                    combo == "technical_master"
                } else {
                    false
                }
            }),
        );
    }

    /// 조건 평가
    pub fn evaluate(&self, condition: &StoryCondition, state: &StoryState) -> bool {
        match condition {
            StoryCondition::Week(week) => state.current_week == *week,

            StoryCondition::WeekRange(start, end) => {
                state.current_week >= *start && state.current_week <= *end
            }

            StoryCondition::CA(op, value) => {
                self.compare_values(state.player_stats.ca as i32, *value as i32, op)
            }

            StoryCondition::Goals(op, value) => {
                self.compare_values(state.player_stats.goals as i32, *value as i32, op)
            }

            StoryCondition::Assists(op, value) => {
                self.compare_values(state.player_stats.assists as i32, *value as i32, op)
            }

            StoryCondition::MatchesPlayed(op, value) => {
                self.compare_values(state.player_stats.matches_played as i32, *value as i32, op)
            }

            StoryCondition::Route(route) => state.current_route == *route,

            StoryCondition::Personality(_personality) => {
                // 실제 구현에서는 플레이어의 personality를 체크
                // 임시로 false 반환
                false
            }

            StoryCondition::HasSpecialAbility(_ability) => {
                // 실제 구현에서는 특수능력 보유 여부 체크
                // 임시로 false 반환
                false
            }

            StoryCondition::Relationship(character, op, value) => {
                if let Some(relationship_value) = state.relationships.get(character) {
                    self.compare_values(*relationship_value, *value, op)
                } else {
                    false
                }
            }

            StoryCondition::EventOccurred(event_id) => state.occurred_events.contains(event_id),

            StoryCondition::EventNotOccurred(event_id) => !state.occurred_events.contains(event_id),

            StoryCondition::Random(probability) => rand::random::<f32>() < *probability,

            StoryCondition::And(conditions) => conditions.iter().all(|c| self.evaluate(c, state)),

            StoryCondition::Or(conditions) => conditions.iter().any(|c| self.evaluate(c, state)),

            StoryCondition::Not(condition) => !self.evaluate(condition, state),
        }
    }

    /// 여러 조건 평가
    pub fn evaluate_all(&self, conditions: &[StoryCondition], state: &StoryState) -> bool {
        conditions.iter().all(|c| self.evaluate(c, state))
    }

    /// 여러 조건 중 하나라도 만족하는지 평가
    pub fn evaluate_any(&self, conditions: &[StoryCondition], state: &StoryState) -> bool {
        conditions.iter().any(|c| self.evaluate(c, state))
    }

    /// 비교 연산 수행
    fn compare_values(&self, left: i32, right: i32, op: &ComparisonOp) -> bool {
        match op {
            ComparisonOp::Equal => left == right,
            ComparisonOp::NotEqual => left != right,
            ComparisonOp::Greater => left > right,
            ComparisonOp::GreaterEqual => left >= right,
            ComparisonOp::Less => left < right,
            ComparisonOp::LessEqual => left <= right,
        }
    }

    /// 커스텀 평가기 등록
    pub fn register_custom_evaluator(
        &mut self,
        name: &str,
        evaluator: Arc<dyn Fn(&serde_json::Value, &StoryState) -> bool + Send + Sync>,
    ) {
        self.custom_evaluators.insert(name.to_string(), evaluator);
    }

    /// 커스텀 조건 평가
    pub fn evaluate_custom(
        &self,
        name: &str,
        value: &serde_json::Value,
        state: &StoryState,
    ) -> bool {
        if let Some(evaluator) = self.custom_evaluators.get(name) {
            evaluator(value, state)
        } else {
            false
        }
    }
}

/// 선택지 요구사항 검증기
pub struct RequirementValidator {
    evaluator: ConditionEvaluator,
}

impl Default for RequirementValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RequirementValidator {
    pub fn new() -> Self {
        Self { evaluator: ConditionEvaluator::new() }
    }

    /// 선택지 요구사항 검증
    pub fn validate_choice_requirements(
        &self,
        requirements: &[ChoiceRequirement],
        state: &StoryState,
    ) -> bool {
        requirements.iter().all(|req| self.validate_requirement(req, state))
    }

    /// 개별 요구사항 검증
    fn validate_requirement(&self, requirement: &ChoiceRequirement, state: &StoryState) -> bool {
        match requirement {
            ChoiceRequirement::MinCA(min_ca) => state.player_stats.ca >= *min_ca,

            ChoiceRequirement::MaxCA(max_ca) => state.player_stats.ca <= *max_ca,

            ChoiceRequirement::Personality(_personality) => {
                // 실제 구현에서는 플레이어 personality 체크
                true
            }

            ChoiceRequirement::SpecialAbility(_ability, _tier) => {
                // 실제 구현에서는 특수능력 및 티어 체크
                true
            }

            ChoiceRequirement::Relationship(character, min_value) => {
                state.relationships.get(character).is_some_and(|value| *value >= *min_value)
            }

            ChoiceRequirement::Custom(name, value) => {
                self.evaluator.evaluate_custom(name, value, state)
            }
        }
    }

    /// 사용 가능한 선택지 필터링
    pub fn filter_available_choices<'a>(
        &self,
        choices: &'a [EventChoice],
        state: &StoryState,
    ) -> Vec<&'a EventChoice> {
        choices
            .iter()
            .filter(|choice| self.validate_choice_requirements(&choice.requirements, state))
            .collect()
    }
}

/// 조건 빌더 - 복잡한 조건을 쉽게 생성
pub struct ConditionBuilder {
    conditions: Vec<StoryCondition>,
}

impl Default for ConditionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionBuilder {
    pub fn new() -> Self {
        Self { conditions: Vec::new() }
    }

    pub fn week(mut self, week: u32) -> Self {
        self.conditions.push(StoryCondition::Week(week));
        self
    }

    pub fn week_range(mut self, start: u32, end: u32) -> Self {
        self.conditions.push(StoryCondition::WeekRange(start, end));
        self
    }

    pub fn min_ca(mut self, ca: u8) -> Self {
        self.conditions.push(StoryCondition::CA(ComparisonOp::GreaterEqual, ca));
        self
    }

    pub fn max_ca(mut self, ca: u8) -> Self {
        self.conditions.push(StoryCondition::CA(ComparisonOp::LessEqual, ca));
        self
    }

    pub fn route(mut self, route: StoryRoute) -> Self {
        self.conditions.push(StoryCondition::Route(route));
        self
    }

    pub fn min_goals(mut self, goals: u32) -> Self {
        self.conditions.push(StoryCondition::Goals(ComparisonOp::GreaterEqual, goals));
        self
    }

    pub fn event_occurred(mut self, event_id: String) -> Self {
        self.conditions.push(StoryCondition::EventOccurred(event_id));
        self
    }

    pub fn random(mut self, probability: f32) -> Self {
        self.conditions.push(StoryCondition::Random(probability));
        self
    }

    pub fn build(self) -> Vec<StoryCondition> {
        self.conditions
    }

    pub fn build_and(self) -> StoryCondition {
        StoryCondition::And(self.conditions)
    }

    pub fn build_or(self) -> StoryCondition {
        StoryCondition::Or(self.conditions)
    }
}

/// 조건 분석기 - 디버깅 및 최적화용
pub struct ConditionAnalyzer;

impl ConditionAnalyzer {
    /// 조건 복잡도 계산
    pub fn calculate_complexity(condition: &StoryCondition) -> u32 {
        match condition {
            StoryCondition::And(conditions) | StoryCondition::Or(conditions) => {
                1 + conditions.iter().map(Self::calculate_complexity).sum::<u32>()
            }
            StoryCondition::Not(condition) => 1 + Self::calculate_complexity(condition),
            _ => 1,
        }
    }

    /// 조건이 항상 참인지 검사
    pub fn is_always_true(condition: &StoryCondition) -> bool {
        match condition {
            StoryCondition::Or(conditions) => conditions.iter().any(Self::is_always_true),
            StoryCondition::Not(condition) => Self::is_always_false(condition),
            _ => false,
        }
    }

    /// 조건이 항상 거짓인지 검사
    pub fn is_always_false(condition: &StoryCondition) -> bool {
        match condition {
            StoryCondition::And(conditions) => conditions.iter().any(Self::is_always_false),
            StoryCondition::Not(condition) => Self::is_always_true(condition),
            _ => false,
        }
    }

    /// 조건 최적화
    pub fn optimize(condition: StoryCondition) -> StoryCondition {
        match condition {
            StoryCondition::And(conditions) => {
                let optimized: Vec<_> = conditions
                    .into_iter()
                    .map(Self::optimize)
                    .filter(|c| !Self::is_always_true(c))
                    .collect();

                if optimized.iter().any(Self::is_always_false) {
                    // 하나라도 항상 거짓이면 전체가 거짓
                    StoryCondition::EventOccurred("_never_".to_string())
                } else if optimized.is_empty() {
                    // 모든 조건이 항상 참이면
                    StoryCondition::EventNotOccurred("_never_".to_string())
                } else if optimized.len() == 1 {
                    optimized.into_iter().next().unwrap()
                } else {
                    StoryCondition::And(optimized)
                }
            }
            StoryCondition::Or(conditions) => {
                let optimized: Vec<_> = conditions
                    .into_iter()
                    .map(Self::optimize)
                    .filter(|c| !Self::is_always_false(c))
                    .collect();

                if optimized.iter().any(Self::is_always_true) {
                    // 하나라도 항상 참이면 전체가 참
                    StoryCondition::EventNotOccurred("_never_".to_string())
                } else if optimized.is_empty() {
                    // 모든 조건이 항상 거짓이면
                    StoryCondition::EventOccurred("_never_".to_string())
                } else if optimized.len() == 1 {
                    optimized.into_iter().next().unwrap()
                } else {
                    StoryCondition::Or(optimized)
                }
            }
            StoryCondition::Not(boxed_condition) => {
                let inner = Self::optimize(*boxed_condition);
                if Self::is_always_true(&inner) {
                    StoryCondition::EventOccurred("_never_".to_string())
                } else if Self::is_always_false(&inner) {
                    StoryCondition::EventNotOccurred("_never_".to_string())
                } else {
                    StoryCondition::Not(Box::new(inner))
                }
            }
            _ => condition,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_evaluator() {
        let evaluator = ConditionEvaluator::new();
        let mut state = StoryState::default();
        state.current_week = 10;
        state.player_stats.ca = 120;

        // Test week condition
        let condition = StoryCondition::Week(10);
        assert!(evaluator.evaluate(&condition, &state));

        // Test CA comparison
        let condition = StoryCondition::CA(ComparisonOp::Greater, 100);
        assert!(evaluator.evaluate(&condition, &state));

        // Test AND condition
        let condition = StoryCondition::And(vec![
            StoryCondition::Week(10),
            StoryCondition::CA(ComparisonOp::Greater, 100),
        ]);
        assert!(evaluator.evaluate(&condition, &state));
    }

    #[test]
    fn test_condition_builder() {
        let conditions = ConditionBuilder::new()
            .week_range(5, 10)
            .min_ca(100)
            .route(StoryRoute::Standard)
            .build();

        assert_eq!(conditions.len(), 3);
    }

    #[test]
    fn test_requirement_validator() {
        let validator = RequirementValidator::new();
        let mut state = StoryState::default();
        state.player_stats.ca = 120;

        let requirements = vec![ChoiceRequirement::MinCA(100), ChoiceRequirement::MaxCA(150)];

        assert!(validator.validate_choice_requirements(&requirements, &state));
    }

    #[test]
    fn test_condition_complexity() {
        let simple = StoryCondition::Week(10);
        assert_eq!(ConditionAnalyzer::calculate_complexity(&simple), 1);

        let complex = StoryCondition::And(vec![
            StoryCondition::Week(10),
            StoryCondition::Or(vec![
                StoryCondition::CA(ComparisonOp::Greater, 100),
                StoryCondition::Goals(ComparisonOp::Greater, 10),
            ]),
        ]);
        assert_eq!(ConditionAnalyzer::calculate_complexity(&complex), 5);
    }
}
