// 훈련 세션 관리 시스템
use crate::player::types::CorePlayer;
use crate::training::condition::Condition;
use crate::training::effects::TrainingEffectEngine;
use crate::training::stamina::{StaminaSystem, TrainingIntensity};
use crate::training::types::{
    CoachBonusLog, TrainingResult, TrainingSession, TrainingTarget, TrainingType,
};
use crate::training::weekly_plan::{DayOfWeek, DaySlot, WeeklyPlan};
use chrono::{DateTime, NaiveDate, Utc};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 훈련 세션 관리자
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingManager {
    /// 현재 체력 시스템
    #[serde(default)]
    pub stamina_system: StaminaSystem,
    /// 현재 컨디션
    #[serde(default)]
    pub condition: Condition,
    /// 연속 훈련 일수
    #[serde(default)]
    pub consecutive_training_days: u8,
    /// 연속 휴식 일수
    #[serde(default)]
    pub consecutive_rest_days: u8,
    /// 주간 훈련 기록
    #[serde(default)]
    pub week_training_count: u8,
    /// 훈련 히스토리
    #[serde(default)]
    pub training_history: Vec<TrainingRecord>,
    /// 현재 활성 덱 (옵션)
    #[serde(default)]
    pub active_deck: Option<crate::coach::Deck>,
    /// 훈련 부하 상태
    #[serde(default)]
    pub training_load: PlayerTrainingLoad,
    /// 주간 계획 저장소
    #[serde(default)]
    pub weekly_plans: HashMap<u16, WeeklyPlan>,
    /// 현재 주차
    #[serde(default = "default_training_week")]
    pub current_week: u16,
}

fn default_training_week() -> u16 {
    1
}

impl TrainingManager {
    /// 새 훈련 관리자 생성
    pub fn new() -> Self {
        Self {
            stamina_system: StaminaSystem::new(),
            condition: Condition::Normal,
            consecutive_training_days: 0,
            consecutive_rest_days: 0,
            week_training_count: 0,
            training_history: Vec::new(),
            active_deck: None,
            training_load: PlayerTrainingLoad::new(),
            weekly_plans: HashMap::new(),
            current_week: 1,
        }
    }

    /// 주간 계획 조회
    pub fn get_weekly_plan(&self, week_number: u16) -> Option<&WeeklyPlan> {
        self.weekly_plans.get(&week_number)
    }

    /// 주간 계획 조회 또는 생성
    pub fn get_or_create_weekly_plan(
        &mut self,
        week_number: u16,
        start_date: NaiveDate,
    ) -> &WeeklyPlan {
        self.weekly_plans
            .entry(week_number)
            .or_insert_with(|| WeeklyPlan::new(week_number, start_date))
    }

    /// 주간 계획 업데이트
    pub fn update_weekly_plan(
        &mut self,
        week_number: u16,
        schedule: Vec<(DayOfWeek, Vec<DaySlot>)>,
    ) -> Result<(), String> {
        // 기존 계획 가져오기 또는 생성
        let plan = self
            .weekly_plans
            .entry(week_number)
            .or_insert_with(|| WeeklyPlan::default_for_week(week_number));

        plan.schedule = schedule;
        Ok(())
    }

    /// 특정 날짜 슬롯 업데이트
    pub fn update_day_slot(
        &mut self,
        week_number: u16,
        day: DayOfWeek,
        slots: Vec<DaySlot>,
    ) -> Result<(), String> {
        let plan =
            self.weekly_plans.get_mut(&week_number).ok_or_else(|| "Plan not found".to_string())?;

        // 해당 요일 찾아서 업데이트
        for (d, s) in &mut plan.schedule {
            if *d == day {
                *s = slots;
                return Ok(());
            }
        }

        // 요일이 없으면 추가
        plan.schedule.push((day, slots));
        Ok(())
    }

    /// 활성 덱 설정
    pub fn set_active_deck(&mut self, deck: crate::coach::Deck) {
        self.active_deck = Some(deck);
    }

    /// 덱 보너스 + 로그 계산
    fn get_deck_bonus_with_log(&self, target: &TrainingTarget) -> (f32, Vec<CoachBonusLog>) {
        self.active_deck
            .as_ref()
            .map(|deck| deck.calculate_training_bonus_with_log(target))
            .unwrap_or((1.0, Vec::new()))
    }

    /// 현재 훈련 부하 스냅샷 반환
    pub fn training_load_snapshot(&self) -> TrainingLoadSnapshot {
        self.training_load.snapshot()
    }

    fn record_training_load(&mut self, session: &TrainingSession) {
        self.training_load.record_session(session, Utc::now());
    }

    fn apply_rest_to_training_load(&mut self) {
        self.training_load.apply_rest();
    }

    /// 일일 활동 처리
    pub fn process_daily_activity(
        &mut self,
        player: &mut CorePlayer,
        activity: &DaySlot,
        seed: u64,
    ) -> DailyActivityResult {
        match activity {
            DaySlot::TeamTraining(target) => {
                self.execute_team_training(player, *target, TrainingIntensity::Normal, seed)
            }
            DaySlot::FreeTime => {
                // 자유 시간 - 선택 가능
                DailyActivityResult::FreeTimeAvailable {
                    available_stamina: self.stamina_system.current(),
                    recommended_activity: self.recommend_activity(),
                }
            }
            DaySlot::Rest { forced } => self.execute_rest(*forced, seed),
            DaySlot::Match { .. } => self.execute_match_day(),
            DaySlot::SpecialEvent { name, .. } => {
                DailyActivityResult::EventCompleted { event_name: name.clone() }
            }
        }
    }

    /// 팀 훈련 실행
    pub fn execute_team_training(
        &mut self,
        player: &mut CorePlayer,
        target: TrainingTarget,
        intensity: TrainingIntensity,
        seed: u64,
    ) -> DailyActivityResult {
        // 훈련 세션 생성 (덱 보너스 포함)
        let (deck_bonus, deck_log) = self.get_deck_bonus_with_log(&target);
        let mut session = TrainingSession::new(TrainingType::Team, target, intensity);
        session.coach_bonus = deck_bonus;

        if !self.stamina_system.can_train(session.stamina_cost) {
            return DailyActivityResult::TrainingFailed {
                reason: format!("체력 부족: {} 필요", session.stamina_cost),
            };
        }

        // 체력 소모
        if self.stamina_system.consume(session.stamina_cost).is_err() {
            return DailyActivityResult::TrainingFailed {
                reason: "체력 소모 실패".to_string()
            };
        }

        // 훈련 효과 적용
        let result = TrainingEffectEngine::execute_training(
            player,
            &session,
            self.condition,
            seed,
            deck_log,
        );

        // 상태 업데이트
        self.consecutive_training_days += 1;
        self.consecutive_rest_days = 0;
        self.week_training_count += 1;

        // 덱 사용 기록
        if let Some(ref mut deck) = self.active_deck {
            deck.record_use();
        }

        // 기록 저장
        self.training_history.push(TrainingRecord {
            session: session.clone(),
            condition: self.condition,
            result: result.clone(),
        });
        self.record_training_load(&session);

        // 컨디션 업데이트 (다음 날)
        self.update_condition(seed);

        DailyActivityResult::TrainingCompleted {
            result,
            new_stamina: self.stamina_system.current(),
            new_condition: self.condition,
        }
    }

    /// 개인 훈련 실행
    pub fn execute_personal_training(
        &mut self,
        player: &mut CorePlayer,
        target: TrainingTarget,
        intensity: TrainingIntensity,
        seed: u64,
    ) -> DailyActivityResult {
        // 체력 체크
        let (deck_bonus, deck_log) = self.get_deck_bonus_with_log(&target);
        let mut session = TrainingSession::new(TrainingType::Individual, target, intensity);
        session.coach_bonus = deck_bonus;

        if !self.stamina_system.can_train(session.stamina_cost) {
            return DailyActivityResult::TrainingFailed {
                reason: format!("체력 부족: {} 필요", session.stamina_cost),
            };
        }

        // 체력 소모
        if self.stamina_system.consume(session.stamina_cost).is_err() {
            return DailyActivityResult::TrainingFailed {
                reason: "체력 소모 실패".to_string()
            };
        }

        // 훈련 효과 적용
        let result = TrainingEffectEngine::execute_training(
            player,
            &session,
            self.condition,
            seed,
            deck_log,
        );

        // 상태 업데이트
        self.consecutive_training_days += 1;
        self.consecutive_rest_days = 0;

        // 덱 사용 기록
        if let Some(ref mut deck) = self.active_deck {
            deck.record_use();
        }

        // 기록 저장
        self.training_history.push(TrainingRecord {
            session: session.clone(),
            condition: self.condition,
            result: result.clone(),
        });
        self.record_training_load(&session);

        DailyActivityResult::TrainingCompleted {
            result,
            new_stamina: self.stamina_system.current(),
            new_condition: self.condition,
        }
    }

    /// 특별 훈련 실행
    pub fn execute_special_training(
        &mut self,
        player: &mut CorePlayer,
        target: TrainingTarget,
        intensity: TrainingIntensity,
        seed: u64,
    ) -> DailyActivityResult {
        let (deck_bonus, deck_log) = self.get_deck_bonus_with_log(&target);
        let mut session = TrainingSession::new(TrainingType::Special, target, intensity);
        session.coach_bonus = deck_bonus;

        if !self.stamina_system.can_train(session.stamina_cost) {
            return DailyActivityResult::TrainingFailed {
                reason: format!("체력 부족: {} 필요", session.stamina_cost),
            };
        }

        if self.stamina_system.consume(session.stamina_cost).is_err() {
            return DailyActivityResult::TrainingFailed {
                reason: "체력 소모 실패".to_string()
            };
        }

        let result = TrainingEffectEngine::execute_training(
            player,
            &session,
            self.condition,
            seed,
            deck_log,
        );

        self.consecutive_training_days += 1;
        self.consecutive_rest_days = 0;

        if let Some(ref mut deck) = self.active_deck {
            deck.record_use();
        }

        self.training_history.push(TrainingRecord {
            session: session.clone(),
            condition: self.condition,
            result: result.clone(),
        });
        self.record_training_load(&session);

        self.update_condition(seed);

        DailyActivityResult::TrainingCompleted {
            result,
            new_stamina: self.stamina_system.current(),
            new_condition: self.condition,
        }
    }

    /// 휴식 실행
    pub fn execute_rest(&mut self, forced: bool, seed: u64) -> DailyActivityResult {
        // 체력 회복
        self.stamina_system.rest();
        self.apply_rest_to_training_load();

        // 상태 업데이트
        self.consecutive_rest_days += 1;
        self.consecutive_training_days = 0;

        // 컨디션 개선 시도 (휴식 보너스)
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        if self.consecutive_rest_days >= 2 {
            self.condition = self.condition.try_improve(&mut rng);
        }

        DailyActivityResult::RestCompleted {
            stamina_recovered: 30,
            new_stamina: self.stamina_system.current(),
            new_condition: self.condition,
            was_forced: forced,
        }
    }

    /// 경기일 처리
    fn execute_match_day(&mut self) -> DailyActivityResult {
        // 경기는 체력 30 소모
        let _ = self.stamina_system.consume(30);

        // 연속 기록 리셋
        self.consecutive_training_days = 0;
        self.consecutive_rest_days = 0;

        DailyActivityResult::MatchPlayed {
            stamina_consumed: 30,
            new_stamina: self.stamina_system.current(),
        }
    }

    /// 활동 추천
    pub fn recommend_activity(&self) -> RecommendedActivity {
        let _stamina = self.stamina_system.current();
        let status = self.stamina_system.status();

        use crate::training::stamina::StaminaStatus;
        match status {
            StaminaStatus::Excellent | StaminaStatus::Good => {
                if self.consecutive_training_days >= 3 {
                    RecommendedActivity::Rest
                } else {
                    RecommendedActivity::PersonalTraining
                }
            }
            StaminaStatus::Normal => {
                if self.consecutive_training_days >= 2 {
                    RecommendedActivity::LightTraining
                } else {
                    RecommendedActivity::PersonalTraining
                }
            }
            StaminaStatus::Tired => RecommendedActivity::Rest,
            StaminaStatus::Exhausted => RecommendedActivity::ForcedRest,
        }
    }

    /// 컨디션 업데이트
    fn update_condition(&mut self, seed: u64) {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        self.condition = Condition::calculate(
            self.stamina_system.current(),
            self.consecutive_training_days,
            self.consecutive_rest_days,
            &mut rng,
        );
    }

    /// 주간 리셋
    pub fn reset_week(&mut self) {
        self.week_training_count = 0;
        self.training_load.weekly_reset();
        // 히스토리는 유지 (통계용)
    }

    /// 훈련 통계 가져오기
    pub fn get_training_stats(&self) -> TrainingStats {
        let total_sessions = self.training_history.len();
        let total_growth: f32 = self.training_history.iter().map(|r| r.result.ca_change).sum();

        let injury_count =
            self.training_history.iter().filter(|r| r.result.injury_occurred).count();

        TrainingStats {
            total_sessions: total_sessions as u32,
            total_ca_growth: total_growth,
            average_growth_per_session: if total_sessions > 0 {
                total_growth / total_sessions as f32
            } else {
                0.0
            },
            injury_count: injury_count as u32,
            current_condition: self.condition,
            current_stamina: self.stamina_system.current(),
        }
    }
}

/// 훈련 부하 스냅샷 (UI/리포트 전달용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoadSnapshot {
    pub acute_load: f32,
    pub chronic_load: f32,
    pub load_ratio: f32,
    pub cumulative_fatigue: f32,
    pub sessions_this_week: u8,
    pub needs_rest: bool,
    pub injury_risk_modifier: f32,
}

/// 선수 훈련 부하 추적기
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerTrainingLoad {
    acute_load: f32,
    chronic_load: f32,
    load_ratio: f32,
    cumulative_fatigue: f32,
    sessions_this_week: u8,
    last_high_intensity: Option<DateTime<Utc>>,
}

impl PlayerTrainingLoad {
    pub fn new() -> Self {
        Self {
            acute_load: 0.0,
            chronic_load: 0.0,
            load_ratio: 1.0,
            cumulative_fatigue: 0.0,
            sessions_this_week: 0,
            last_high_intensity: None,
        }
    }

    pub fn record_session(&mut self, session: &TrainingSession, timestamp: DateTime<Utc>) {
        let load_value = session.stamina_cost as f32;
        self.acute_load = self.acute_load * 0.9 + load_value * 0.1;
        self.chronic_load = self.chronic_load * 0.97 + load_value * 0.03;
        self.load_ratio =
            if self.chronic_load > 0.0 { self.acute_load / self.chronic_load } else { 1.0 };
        self.cumulative_fatigue = (self.cumulative_fatigue + load_value * 0.2).min(100.0);

        if matches!(session.intensity, TrainingIntensity::Intensive) {
            self.last_high_intensity = Some(timestamp);
        }

        self.sessions_this_week = self.sessions_this_week.saturating_add(1);
    }

    pub fn apply_rest(&mut self) {
        self.acute_load *= 0.85;
        self.cumulative_fatigue *= 0.75;
        if self.cumulative_fatigue < 1.0 {
            self.cumulative_fatigue = 0.0;
        }
    }

    pub fn weekly_reset(&mut self) {
        self.sessions_this_week = 0;
        self.cumulative_fatigue *= 0.7;
    }

    pub fn needs_rest(&self) -> bool {
        self.cumulative_fatigue > 75.0 || self.load_ratio > 1.5 || self.sessions_this_week >= 6
    }

    pub fn injury_risk_modifier(&self) -> f32 {
        if self.load_ratio > 1.5 {
            1.5
        } else if self.load_ratio > 1.3 {
            1.2
        } else if self.load_ratio < 0.8 {
            1.1
        } else {
            1.0
        }
    }

    pub fn snapshot(&self) -> TrainingLoadSnapshot {
        TrainingLoadSnapshot {
            acute_load: (self.acute_load * 100.0).round() / 100.0,
            chronic_load: (self.chronic_load * 100.0).round() / 100.0,
            load_ratio: (self.load_ratio * 100.0).round() / 100.0,
            cumulative_fatigue: (self.cumulative_fatigue * 100.0).round() / 100.0,
            sessions_this_week: self.sessions_this_week,
            needs_rest: self.needs_rest(),
            injury_risk_modifier: self.injury_risk_modifier(),
        }
    }
}

impl Default for PlayerTrainingLoad {
    fn default() -> Self {
        Self::new()
    }
}

/// 일일 활동 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DailyActivityResult {
    /// 훈련 완료
    TrainingCompleted { result: TrainingResult, new_stamina: u8, new_condition: Condition },
    /// 훈련 실패
    TrainingFailed { reason: String },
    /// 휴식 완료
    RestCompleted {
        stamina_recovered: u8,
        new_stamina: u8,
        new_condition: Condition,
        was_forced: bool,
    },
    /// 경기 참가
    MatchPlayed { stamina_consumed: u8, new_stamina: u8 },
    /// 자유 시간
    FreeTimeAvailable { available_stamina: u8, recommended_activity: RecommendedActivity },
    /// 이벤트 완료
    EventCompleted { event_name: String },
}

/// 추천 활동
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendedActivity {
    PersonalTraining,
    LightTraining,
    Rest,
    ForcedRest,
}

/// 훈련 기록
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecord {
    pub session: TrainingSession,
    pub condition: Condition,
    pub result: TrainingResult,
}

/// 훈련 통계
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStats {
    pub total_sessions: u32,
    pub total_ca_growth: f32,
    pub average_growth_per_session: f32,
    pub injury_count: u32,
    pub current_condition: Condition,
    pub current_stamina: u8,
}

impl Default for TrainingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::player::PlayerAttributes;

    #[test]
    fn test_training_manager_creation() {
        let manager = TrainingManager::new();
        assert_eq!(manager.stamina_system.current(), 100);
        assert_eq!(manager.condition, Condition::Normal);
        assert_eq!(manager.consecutive_training_days, 0);
    }

    #[test]
    fn test_consecutive_training_tracking() {
        let mut manager = TrainingManager::new();
        let mut player = CorePlayer {
            id: "test-player".to_string(),
            name: "테스트".to_string(),
            age_months: 16.0 * 12.0,
            ca: 80,
            pa: 120,
            position: crate::models::player::Position::CM,
            detailed_stats: PlayerAttributes::default(),
            hexagon_stats: crate::HexagonCalculator::calculate_all(
                &PlayerAttributes::default(),
                crate::models::player::Position::CM,
            ),
            growth_profile: crate::player::types::GrowthProfile::default(),
            personality: crate::player::personality::PersonAttributes::default(),
            special_abilities: crate::special_ability::SpecialAbilityCollection::new(),
            instructions: Default::default(), // Added missing field
            current_injury: None,
            injury_history: Vec::new(),
            injury_proneness: 0.3,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            career_stats: crate::player::PlayerCareerStats::new(),
        };

        // 팀 훈련 실행
        let _ = manager.execute_team_training(
            &mut player,
            TrainingTarget::Balanced,
            TrainingIntensity::Normal,
            42,
        );
        assert_eq!(manager.consecutive_training_days, 1);
        assert_eq!(manager.consecutive_rest_days, 0);

        // 휴식
        let _ = manager.execute_rest(false, 7);
        assert_eq!(manager.consecutive_training_days, 0);
        assert_eq!(manager.consecutive_rest_days, 1);
    }

    #[test]
    fn test_stamina_management() {
        let mut manager = TrainingManager::new();
        assert_eq!(manager.stamina_system.current(), 100);

        // 휴식으로 체력 회복
        manager.execute_rest(false, 11);
        assert_eq!(manager.stamina_system.current(), 100); // 이미 최대

        // 체력 소모
        manager.stamina_system.consume(50).unwrap();
        assert_eq!(manager.stamina_system.current(), 50);

        // 휴식으로 회복
        manager.execute_rest(false, 12);
        assert_eq!(manager.stamina_system.current(), 80); // +30
    }
}
