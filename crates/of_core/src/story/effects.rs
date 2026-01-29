//! Effect Processing Module
//!
//! 스토리 효과 처리 및 상태 변경

use super::types::*;
use crate::error::CoreError;
use crate::player::personality::PersonalityArchetype;
use std::collections::HashMap;
use std::sync::Arc;

/// 효과 처리기
pub struct EffectProcessor {
    custom_processors: HashMap<
        String,
        Arc<dyn Fn(&serde_json::Value, &mut StoryState) -> Result<(), CoreError> + Send + Sync>,
    >,
    effect_history: Vec<AppliedEffect>,
}

impl Default for EffectProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectProcessor {
    pub fn new() -> Self {
        let mut processor = Self { custom_processors: HashMap::new(), effect_history: Vec::new() };
        processor.register_default_processors();
        processor
    }

    /// 기본 커스텀 프로세서 등록
    fn register_default_processors(&mut self) {
        // 훈련 보너스 적용
        self.register_custom_processor(
            "training_bonus",
            Arc::new(|value, state| {
                if let Some(_bonus) = value.as_f64() {
                    // 실제 구현에서는 훈련 효율에 보너스 적용
                    state.active_flags.insert("training_bonus".to_string(), true);
                }
                Ok(())
            }),
        );

        // 특별 이벤트 트리거
        self.register_custom_processor(
            "special_event",
            Arc::new(|value, _state| {
                if let Some(event_id) = value.as_str() {
                    // 실제 구현에서는 특별 이벤트 큐에 추가
                    println!("Triggering special event: {}", event_id);
                }
                Ok(())
            }),
        );
    }

    /// 효과 적용
    pub fn apply_effect(
        &mut self,
        effect: &StoryEffect,
        state: &mut StoryState,
    ) -> Result<(), CoreError> {
        let result = match effect {
            StoryEffect::ModifyCA(delta) => self.modify_ca(state, *delta),

            StoryEffect::ModifySkill(skill_name, delta) => {
                self.modify_skill(state, skill_name, *delta)
            }

            StoryEffect::ModifyRelationship(character, delta) => {
                self.modify_relationship(state, character, *delta)
            }

            StoryEffect::UnlockSpecialAbility(ability) => {
                self.unlock_special_ability(state, ability)
            }

            StoryEffect::SetPersonality(personality) => self.set_personality(state, *personality),

            StoryEffect::TriggerEvent(event_id) => self.trigger_event(state, event_id),

            StoryEffect::SetFlag(flag, value) => self.set_flag(state, flag, *value),

            StoryEffect::ModifyMorale(delta) => self.modify_morale(state, *delta),

            StoryEffect::ModifyFatigue(delta) => self.modify_fatigue(state, *delta),

            StoryEffect::Custom(name, value) => {
                self.apply_custom_effect(name, value, state)?;
                Ok(())
            }
        };

        // 효과 적용 기록
        if result.is_ok() {
            self.record_effect(effect);
        }

        result
    }

    /// 여러 효과 적용
    pub fn apply_effects(
        &mut self,
        effects: &[StoryEffect],
        state: &mut StoryState,
    ) -> Result<(), CoreError> {
        for effect in effects {
            self.apply_effect(effect, state)?;
        }
        Ok(())
    }

    /// CA 수정
    fn modify_ca(&mut self, state: &mut StoryState, delta: i8) -> Result<(), CoreError> {
        let new_ca = (state.player_stats.ca as i16 + delta as i16).clamp(0, 200) as u8;
        state.player_stats.ca = new_ca;
        Ok(())
    }

    /// 스킬 수정
    fn modify_skill(
        &mut self,
        _state: &mut StoryState,
        skill_name: &str,
        delta: i8,
    ) -> Result<(), CoreError> {
        // 실제 구현에서는 OpenFootball 스킬 시스템과 연동
        println!("Modifying skill {}: {:+}", skill_name, delta);
        Ok(())
    }

    /// 관계 수정
    fn modify_relationship(
        &mut self,
        state: &mut StoryState,
        character: &str,
        delta: i32,
    ) -> Result<(), CoreError> {
        let current = state.relationships.get(character).copied().unwrap_or(0);
        let new_value = (current + delta).clamp(-100, 100);
        state.relationships.insert(character.to_string(), new_value);
        Ok(())
    }

    /// 특수능력 해금
    fn unlock_special_ability(
        &mut self,
        _state: &mut StoryState,
        ability: &str,
    ) -> Result<(), CoreError> {
        // 실제 구현에서는 특수능력 시스템과 연동
        println!("Unlocking special ability: {}", ability);
        Ok(())
    }

    /// 성격 설정
    fn set_personality(
        &mut self,
        _state: &mut StoryState,
        personality: PersonalityArchetype,
    ) -> Result<(), CoreError> {
        // 실제 구현에서는 플레이어 성격 설정
        println!("Setting personality: {:?}", personality);
        Ok(())
    }

    /// 이벤트 트리거
    fn trigger_event(&mut self, state: &mut StoryState, event_id: &str) -> Result<(), CoreError> {
        state.occurred_events.push(event_id.to_string());
        Ok(())
    }

    /// 플래그 설정
    fn set_flag(
        &mut self,
        state: &mut StoryState,
        flag: &str,
        value: bool,
    ) -> Result<(), CoreError> {
        state.active_flags.insert(flag.to_string(), value);
        Ok(())
    }

    /// 사기 수정
    fn modify_morale(&mut self, state: &mut StoryState, delta: i32) -> Result<(), CoreError> {
        let new_morale = (state.morale + delta).clamp(0, 100);
        state.morale = new_morale;
        Ok(())
    }

    /// 피로도 수정
    fn modify_fatigue(&mut self, state: &mut StoryState, delta: i32) -> Result<(), CoreError> {
        let new_fatigue = (state.fatigue + delta).clamp(0, 100);
        state.fatigue = new_fatigue;
        Ok(())
    }

    /// 커스텀 효과 적용
    fn apply_custom_effect(
        &mut self,
        name: &str,
        value: &serde_json::Value,
        state: &mut StoryState,
    ) -> Result<(), CoreError> {
        if let Some(processor) = self.custom_processors.get(name) {
            processor(value, state)
        } else {
            Err(CoreError::InvalidParameter(format!("Unknown custom effect: {}", name)))
        }
    }

    /// 커스텀 프로세서 등록
    pub fn register_custom_processor(
        &mut self,
        name: &str,
        processor: Arc<
            dyn Fn(&serde_json::Value, &mut StoryState) -> Result<(), CoreError> + Send + Sync,
        >,
    ) {
        self.custom_processors.insert(name.to_string(), processor);
    }

    /// 효과 적용 기록
    fn record_effect(&mut self, effect: &StoryEffect) {
        self.effect_history.push(AppliedEffect {
            effect: effect.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        });
    }

    /// 효과 되돌리기 (Undo)
    pub fn revert_last_effect(&mut self, state: &mut StoryState) -> Result<(), CoreError> {
        if let Some(applied) = self.effect_history.pop() {
            self.revert_effect(&applied.effect, state)
        } else {
            Err(CoreError::NotFound("No effect to revert".into()))
        }
    }

    /// 특정 효과 되돌리기
    fn revert_effect(
        &mut self,
        effect: &StoryEffect,
        state: &mut StoryState,
    ) -> Result<(), CoreError> {
        match effect {
            StoryEffect::ModifyCA(delta) => self.modify_ca(state, -delta),
            StoryEffect::ModifySkill(skill, delta) => self.modify_skill(state, skill, -delta),
            StoryEffect::ModifyRelationship(character, delta) => {
                self.modify_relationship(state, character, -delta)
            }
            StoryEffect::ModifyMorale(delta) => self.modify_morale(state, -delta),
            StoryEffect::ModifyFatigue(delta) => self.modify_fatigue(state, -delta),
            StoryEffect::SetFlag(flag, _) => {
                state.active_flags.remove(flag);
                Ok(())
            }
            _ => Ok(()), // 일부 효과는 되돌릴 수 없음
        }
    }
}

/// 적용된 효과 기록
#[derive(Debug, Clone)]
struct AppliedEffect {
    effect: StoryEffect,
    timestamp: i64,
}

/// 효과 빌더 - 복잡한 효과 조합 생성
pub struct EffectBuilder {
    effects: Vec<StoryEffect>,
}

impl Default for EffectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectBuilder {
    pub fn new() -> Self {
        Self { effects: Vec::new() }
    }

    pub fn modify_ca(mut self, delta: i8) -> Self {
        self.effects.push(StoryEffect::ModifyCA(delta));
        self
    }

    pub fn modify_skill(mut self, skill: String, delta: i8) -> Self {
        self.effects.push(StoryEffect::ModifySkill(skill, delta));
        self
    }

    pub fn modify_relationship(mut self, character: String, delta: i32) -> Self {
        self.effects.push(StoryEffect::ModifyRelationship(character, delta));
        self
    }

    pub fn unlock_ability(mut self, ability: String) -> Self {
        self.effects.push(StoryEffect::UnlockSpecialAbility(ability));
        self
    }

    pub fn trigger_event(mut self, event_id: String) -> Self {
        self.effects.push(StoryEffect::TriggerEvent(event_id));
        self
    }

    pub fn set_flag(mut self, flag: String, value: bool) -> Self {
        self.effects.push(StoryEffect::SetFlag(flag, value));
        self
    }

    pub fn modify_morale(mut self, delta: i32) -> Self {
        self.effects.push(StoryEffect::ModifyMorale(delta));
        self
    }

    pub fn modify_fatigue(mut self, delta: i32) -> Self {
        self.effects.push(StoryEffect::ModifyFatigue(delta));
        self
    }

    pub fn build(self) -> Vec<StoryEffect> {
        self.effects
    }
}

/// 효과 검증기 - 효과 적용 전 검증
pub struct EffectValidator;

impl EffectValidator {
    /// 효과가 안전한지 검증
    pub fn is_safe(effect: &StoryEffect, state: &StoryState) -> bool {
        match effect {
            StoryEffect::ModifyCA(delta) => {
                let new_ca = state.player_stats.ca as i16 + *delta as i16;
                (0..=200).contains(&new_ca)
            }
            StoryEffect::ModifyRelationship(_, delta) => {
                // 관계 수치가 범위 내인지 체크
                delta.abs() <= 100
            }
            _ => true,
        }
    }

    /// 여러 효과 검증
    pub fn validate_effects(effects: &[StoryEffect], state: &StoryState) -> Result<(), String> {
        for effect in effects {
            if !Self::is_safe(effect, state) {
                return Err(format!("Unsafe effect: {:?}", effect));
            }
        }
        Ok(())
    }

    /// 효과 충돌 검사
    pub fn check_conflicts(effects: &[StoryEffect]) -> Vec<(usize, usize)> {
        let mut conflicts = Vec::new();

        for (i, effect1) in effects.iter().enumerate() {
            for (j, effect2) in effects.iter().enumerate().skip(i + 1) {
                if Self::effects_conflict(effect1, effect2) {
                    conflicts.push((i, j));
                }
            }
        }

        conflicts
    }

    /// 두 효과가 충돌하는지 검사
    fn effects_conflict(effect1: &StoryEffect, effect2: &StoryEffect) -> bool {
        match (effect1, effect2) {
            (StoryEffect::SetPersonality(_), StoryEffect::SetPersonality(_)) => true,
            (StoryEffect::SetFlag(flag1, _), StoryEffect::SetFlag(flag2, _)) => flag1 == flag2,
            _ => false,
        }
    }
}

/// 효과 최적화기
pub struct EffectOptimizer;

impl EffectOptimizer {
    /// 중복 효과 제거 및 병합
    pub fn optimize(effects: Vec<StoryEffect>) -> Vec<StoryEffect> {
        let mut optimized = Vec::new();
        let mut ca_delta = 0i8;
        let mut morale_delta = 0i32;
        let mut fatigue_delta = 0i32;
        let mut relationships: HashMap<String, i32> = HashMap::new();
        let mut skills: HashMap<String, i8> = HashMap::new();

        for effect in effects {
            match effect {
                StoryEffect::ModifyCA(delta) => ca_delta += delta,
                StoryEffect::ModifyMorale(delta) => morale_delta += delta,
                StoryEffect::ModifyFatigue(delta) => fatigue_delta += delta,
                StoryEffect::ModifyRelationship(character, delta) => {
                    *relationships.entry(character).or_insert(0) += delta;
                }
                StoryEffect::ModifySkill(skill, delta) => {
                    *skills.entry(skill).or_insert(0) += delta;
                }
                _ => optimized.push(effect),
            }
        }

        // 병합된 효과 추가
        if ca_delta != 0 {
            optimized.push(StoryEffect::ModifyCA(ca_delta));
        }
        if morale_delta != 0 {
            optimized.push(StoryEffect::ModifyMorale(morale_delta));
        }
        if fatigue_delta != 0 {
            optimized.push(StoryEffect::ModifyFatigue(fatigue_delta));
        }
        for (character, delta) in relationships {
            if delta != 0 {
                optimized.push(StoryEffect::ModifyRelationship(character, delta));
            }
        }
        for (skill, delta) in skills {
            if delta != 0 {
                optimized.push(StoryEffect::ModifySkill(skill, delta));
            }
        }

        optimized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_processor() {
        let mut processor = EffectProcessor::new();
        let mut state = StoryState::default();
        state.player_stats.ca = 100;

        let effect = StoryEffect::ModifyCA(10);
        processor.apply_effect(&effect, &mut state).unwrap();
        assert_eq!(state.player_stats.ca, 110);

        // Test revert
        processor.revert_last_effect(&mut state).unwrap();
        assert_eq!(state.player_stats.ca, 100);
    }

    #[test]
    fn test_effect_builder() {
        let effects = EffectBuilder::new()
            .modify_ca(5)
            .modify_morale(10)
            .set_flag("training_complete".to_string(), true)
            .build();

        assert_eq!(effects.len(), 3);
    }

    #[test]
    fn test_effect_optimizer() {
        let effects = vec![
            StoryEffect::ModifyCA(5),
            StoryEffect::ModifyCA(3),
            StoryEffect::ModifyMorale(10),
            StoryEffect::ModifyMorale(-5),
        ];

        let optimized = EffectOptimizer::optimize(effects);
        assert_eq!(optimized.len(), 2); // CA와 Morale 각각 하나씩
    }
}
