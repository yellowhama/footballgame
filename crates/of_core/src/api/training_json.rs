// 훈련 시스템 JSON API
use serde::{Deserialize, Serialize};
use serde_json;

use crate::coach::Deck;
use crate::player::{CorePlayer, PlayerCareerStats};
use crate::training::{
    DailyActivityResult, DayOfWeek, DaySlot, RecommendedActivity, TrainingIntensity,
    TrainingLoadSnapshot, TrainingManager, TrainingTarget, WeeklyPlan,
};

/// 훈련 요청 - Godot에서 전송
#[derive(Debug, Deserialize)]
pub struct TrainingRequest {
    pub schema_version: u8,
    pub request_type: TrainingRequestType,
    pub player_id: String,
    pub seed: u64,
    #[serde(default)]
    pub active_deck: Option<Deck>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum TrainingRequestType {
    /// 주간 계획 요청
    GetWeeklyPlan { week_number: u16 },

    /// 팀훈련 실행
    ExecuteTeamTraining {
        target: String,            // "balanced", "technical", "physical" 등
        intensity: Option<String>, // 생략 시 normal
    },

    /// 개인훈련 실행
    ExecutePersonalTraining {
        target: String,    // "pace", "power", "technical" 등
        intensity: String, // "light", "normal", "intensive"
    },

    /// 특별훈련 실행
    ExecuteSpecialTraining { target: String, intensity: String },

    /// 휴식 실행
    ExecuteRest { forced: bool },

    /// 현재 상태 조회
    GetStatus,

    /// 훈련 통계 조회
    GetStatistics,

    /// 활동 추천 요청
    GetRecommendation,

    /// 부상 상태 조회
    GetInjuryStatus,

    /// 회복 진행 (하루)
    AdvanceRecovery,

    /// 주간 계획 업데이트
    UpdateWeeklyPlan { week_number: u16, schedule: Vec<DayScheduleInput> },

    /// 특정 날짜 슬롯 업데이트
    UpdateDaySlot { week_number: u16, day: String, slots: Vec<DaySlotInput> },
}

#[derive(Debug, Deserialize)]
pub struct DayScheduleInput {
    pub day: String,
    pub slots: Vec<DaySlotInput>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DaySlotInput {
    TeamTraining { target: String },
    FreeTime,
    Rest { forced: bool },
    Match { opponent: String, is_home: bool },
    SpecialEvent { name: String, description: String },
}

/// 훈련 응답 - Godot으로 전송
#[derive(Debug, Serialize)]
pub struct TrainingResponse {
    pub schema_version: u8,
    pub success: bool,
    pub response_type: TrainingResponseType,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum TrainingResponseType {
    /// 주간 계획 응답
    WeeklyPlan {
        week_number: u16,
        team_training_count: usize,
        free_slots: usize,
        schedule: Vec<DaySchedule>,
    },

    /// 훈련 결과
    TrainingResult {
        ca_before: u8,
        ca_after: u8,
        stamina_before: u8,
        stamina_after: u8,
        condition: String,
        improved_attributes: Vec<AttributeImprovement>,
        injury_occurred: bool,
        message: String,
        training_load: TrainingLoadSnapshot,
        injury_risk: f32,
        coach_bonus_log: Vec<crate::training::CoachBonusLog>,
    },

    /// 휴식 결과
    RestResult { stamina_before: u8, stamina_after: u8, condition: String, was_forced: bool },

    /// 상태 정보
    Status {
        ca: u8,
        pa: u8,
        stamina: u8,
        condition: String,
        consecutive_training_days: u8,
        consecutive_rest_days: u8,
        injury_risk: f32,
    },

    /// 통계 정보
    Statistics {
        total_sessions: u32,
        total_ca_growth: f32,
        average_growth_per_session: f32,
        injury_count: u32,
    },

    /// 활동 추천
    Recommendation {
        recommended: String, // "personal_training", "light_training", "rest", "forced_rest"
        reason: String,
    },

    /// 부상 상태
    InjuryStatus {
        is_injured: bool,
        can_train: bool,
        can_play: bool,
        injury: Option<InjuryInfo>,
        injury_history_count: usize,
    },

    /// 회복 진행 결과
    RecoveryResult { days_remaining: u8, recovered: bool, injury_type: Option<String> },

    /// 훈련 불가 응답
    CannotTrain { reason: String },

    /// 계획 업데이트 결과
    PlanUpdated { week_number: u16, training_count: u8, rest_count: u8, match_count: u8 },
}

#[derive(Debug, Serialize)]
pub struct InjuryInfo {
    pub injury_type: String,
    pub severity: String,
    pub recovery_days_remaining: u8,
    pub recovery_days_total: u8,
    pub recovery_progress: f32,
    pub affected_attributes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DaySchedule {
    pub day: String,
    pub slots: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AttributeImprovement {
    pub attribute: String,
    pub growth: f32,
}

/// 간단한 플레이어 입력 (Godot에서 전송)
#[derive(Debug, Deserialize)]
struct PlayerInput {
    pub id: String,
    pub name: String,
    pub position: String, // "FW", "MF", etc.
    pub ca: u8,
    pub pa: u8,
    pub age_months: f32,
    pub detailed_stats: std::collections::HashMap<String, u8>, // 36 OpenFootball attributes
}

impl PlayerInput {
    /// Convert to CorePlayer with defaults for missing fields
    fn to_core_player(self) -> Result<CorePlayer, String> {
        use crate::models::player::{PlayerAttributes, Position};
        use crate::player::{GrowthProfile, HexagonStats, PersonAttributes};
        use crate::SpecialAbilityCollection;
        use chrono::Utc;

        // Parse position
        let position = match self.position.to_uppercase().as_str() {
            "GK" => Position::GK,
            "DF" => Position::DF,
            "CB" => Position::CB,
            "LB" => Position::LB,
            "RB" => Position::RB,
            "LWB" => Position::LWB,
            "RWB" => Position::RWB,
            "MF" => Position::MF,
            "CM" => Position::CM,
            "CDM" => Position::CDM,
            "CAM" => Position::CAM,
            "LM" => Position::LM,
            "RM" => Position::RM,
            "FW" => Position::FW,
            "ST" => Position::ST,
            "CF" => Position::CF,
            "LW" => Position::LW,
            "RW" => Position::RW,
            _ => return Err(format!("Invalid position: {}", self.position)),
        };

        // Build PlayerAttributes from detailed_stats HashMap
        let detailed_stats = PlayerAttributes {
            // Technical (14)
            corners: *self.detailed_stats.get("corners").unwrap_or(&10),
            crossing: *self.detailed_stats.get("crossing").unwrap_or(&10),
            dribbling: *self.detailed_stats.get("dribbling").unwrap_or(&10),
            finishing: *self.detailed_stats.get("finishing").unwrap_or(&10),
            first_touch: *self.detailed_stats.get("first_touch").unwrap_or(&10),
            free_kicks: *self.detailed_stats.get("free_kicks").unwrap_or(&10),
            heading: *self.detailed_stats.get("heading").unwrap_or(&10),
            long_shots: *self.detailed_stats.get("long_shots").unwrap_or(&10),
            long_throws: *self.detailed_stats.get("long_throws").unwrap_or(&10),
            marking: *self.detailed_stats.get("marking").unwrap_or(&10),
            passing: *self.detailed_stats.get("passing").unwrap_or(&10),
            penalty_taking: *self.detailed_stats.get("penalty_taking").unwrap_or(&10),
            tackling: *self.detailed_stats.get("tackling").unwrap_or(&10),
            technique: *self.detailed_stats.get("technique").unwrap_or(&10),
            // Mental (14)
            aggression: *self.detailed_stats.get("aggression").unwrap_or(&10),
            anticipation: *self.detailed_stats.get("anticipation").unwrap_or(&10),
            bravery: *self.detailed_stats.get("bravery").unwrap_or(&10),
            composure: *self.detailed_stats.get("composure").unwrap_or(&10),
            concentration: *self.detailed_stats.get("concentration").unwrap_or(&10),
            decisions: *self.detailed_stats.get("decisions").unwrap_or(&10),
            determination: *self.detailed_stats.get("determination").unwrap_or(&10),
            flair: *self.detailed_stats.get("flair").unwrap_or(&10),
            leadership: *self.detailed_stats.get("leadership").unwrap_or(&10),
            off_the_ball: *self.detailed_stats.get("off_the_ball").unwrap_or(&10),
            positioning: *self.detailed_stats.get("positioning").unwrap_or(&10),
            teamwork: *self.detailed_stats.get("teamwork").unwrap_or(&10),
            vision: *self.detailed_stats.get("vision").unwrap_or(&10),
            work_rate: *self.detailed_stats.get("work_rate").unwrap_or(&10),
            // Physical (8)
            acceleration: *self.detailed_stats.get("acceleration").unwrap_or(&10),
            agility: *self.detailed_stats.get("agility").unwrap_or(&10),
            balance: *self.detailed_stats.get("balance").unwrap_or(&10),
            jumping: *self.detailed_stats.get("jumping").unwrap_or(&10),
            natural_fitness: *self.detailed_stats.get("natural_fitness").unwrap_or(&10),
            pace: *self.detailed_stats.get("pace").unwrap_or(&10),
            stamina: *self.detailed_stats.get("stamina").unwrap_or(&10),
            strength: *self.detailed_stats.get("strength").unwrap_or(&10),
            // GK attributes - default 0 for training JSON (typically outfield)
            gk_aerial_reach: 0,
            gk_command_of_area: 0,
            gk_communication: 0,
            gk_eccentricity: 0,
            gk_handling: 0,
            gk_kicking: 0,
            gk_one_on_ones: 0,
            gk_reflexes: 0,
            gk_rushing_out: 0,
            gk_punching: 0,
            gk_throwing: 0,
        };

        // Create defaults for missing fields
        let growth_profile = GrowthProfile::new();
        let personality = PersonAttributes::generate_random(12345);
        let hexagon_stats = HexagonStats::calculate_from_detailed(&detailed_stats, position);
        let now = Utc::now();

        Ok(CorePlayer {
            id: self.id,
            name: self.name,
            position,
            age_months: self.age_months,
            ca: self.ca,
            pa: self.pa,
            detailed_stats,
            hexagon_stats,
            growth_profile,
            personality,
            special_abilities: SpecialAbilityCollection::new(),
            instructions: None,
            current_injury: None,
            injury_history: Vec::new(),
            injury_proneness: 0.1, // 기본 10% 부상 성향
            created_at: now,
            updated_at: now,
            career_stats: PlayerCareerStats::new(),
        })
    }
}

/// 메인 엔트리 포인트 - JSON 요청을 처리하고 JSON 응답 반환
pub fn execute_training_json(
    request_json: &str,
    player_json: &str,
    manager_json: &str,
) -> Result<String, String> {
    // 요청 파싱
    let request: TrainingRequest =
        serde_json::from_str(request_json).map_err(|e| format!("Invalid JSON request: {}", e))?;

    // 스키마 버전 확인
    if request.schema_version != 1 {
        return Err(format!("Unsupported schema version: {}", request.schema_version));
    }

    // 플레이어 로드 (간단한 입력 → CorePlayer 변환)
    let player_input: PlayerInput =
        serde_json::from_str(player_json).map_err(|e| format!("Invalid player JSON: {}", e))?;
    let mut player = player_input.to_core_player()?;

    // 훈련 매니저 로드
    let mut manager: TrainingManager =
        serde_json::from_str(manager_json).map_err(|e| format!("Invalid manager JSON: {}", e))?;

    // 활성 덱 동기화
    if let Some(active_deck) = &request.active_deck {
        manager.set_active_deck(active_deck.clone());
    }

    // 요청 처리
    let response_type = match request.request_type {
        TrainingRequestType::GetWeeklyPlan { week_number } => {
            // 주간 계획 생성 (실제로는 저장된 계획을 로드하거나 생성)
            let plan =
                WeeklyPlan::new(week_number, chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap());

            TrainingResponseType::WeeklyPlan {
                week_number: plan.week_number,
                team_training_count: plan.count_team_training(),
                free_slots: plan.count_free_slots(),
                schedule: plan
                    .schedule
                    .iter()
                    .map(|(day, slots)| DaySchedule {
                        day: day.display_name().to_string(),
                        slots: slots.iter().map(|s| s.display_text()).collect(),
                    })
                    .collect(),
            }
        }

        TrainingRequestType::ExecuteTeamTraining { target, intensity } => {
            // 부상으로 인한 훈련 불가 체크
            if !player.can_train() {
                return serde_json::to_string(&TrainingResponse {
                    schema_version: 1,
                    success: false,
                    response_type: TrainingResponseType::CannotTrain {
                        reason: format!(
                            "부상으로 훈련 불가 (남은 회복 기간: {}일)",
                            player.current_injury.as_ref().map_or(0, |i| i.recovery_days_remaining)
                        ),
                    },
                    error_message: Some("Player is injured".to_string()),
                })
                .map_err(|e| e.to_string());
            }

            let training_target = parse_training_target(&target)?;
            let training_intensity = match intensity {
                Some(value) => parse_intensity(&value)?,
                None => TrainingIntensity::Normal,
            };

            // CA 저장
            let ca_before = player.ca;
            let stamina_before = manager.stamina_system.current();

            // 훈련 실행
            let result = manager.execute_team_training(
                &mut player,
                training_target,
                training_intensity,
                request.seed,
            );

            match result {
                DailyActivityResult::TrainingCompleted { result, new_stamina, new_condition } => {
                    let load_snapshot = manager.training_load_snapshot();
                    let injury_risk =
                        manager.stamina_system.injury_risk() * load_snapshot.injury_risk_modifier;
                    TrainingResponseType::TrainingResult {
                        ca_before,
                        ca_after: player.ca,
                        stamina_before,
                        stamina_after: new_stamina,
                        condition: format!("{:?}", new_condition),
                        improved_attributes: result
                            .improved_attributes
                            .iter()
                            .map(|(name, growth)| AttributeImprovement {
                                attribute: name.clone(),
                                growth: *growth,
                            })
                            .collect(),
                        injury_occurred: result.injury_occurred,
                        message: result.message,
                        training_load: load_snapshot,
                        injury_risk,
                        coach_bonus_log: result.coach_bonus_log.clone(),
                    }
                }
                DailyActivityResult::TrainingFailed { reason } => {
                    return Err(format!("Training failed: {}", reason));
                }
                _ => {
                    return Err("Unexpected result type".to_string());
                }
            }
        }

        TrainingRequestType::ExecutePersonalTraining { target, intensity } => {
            // 부상으로 인한 훈련 불가 체크
            if !player.can_train() {
                return serde_json::to_string(&TrainingResponse {
                    schema_version: 1,
                    success: false,
                    response_type: TrainingResponseType::CannotTrain {
                        reason: format!(
                            "부상으로 훈련 불가 (남은 회복 기간: {}일)",
                            player.current_injury.as_ref().map_or(0, |i| i.recovery_days_remaining)
                        ),
                    },
                    error_message: Some("Player is injured".to_string()),
                })
                .map_err(|e| e.to_string());
            }

            let training_target = parse_training_target(&target)?;
            let training_intensity = parse_intensity(&intensity)?;

            let ca_before = player.ca;
            let stamina_before = manager.stamina_system.current();

            let result = manager.execute_personal_training(
                &mut player,
                training_target,
                training_intensity,
                request.seed,
            );

            match result {
                DailyActivityResult::TrainingCompleted { result, new_stamina, new_condition } => {
                    let load_snapshot = manager.training_load_snapshot();
                    let injury_risk =
                        manager.stamina_system.injury_risk() * load_snapshot.injury_risk_modifier;
                    TrainingResponseType::TrainingResult {
                        ca_before,
                        ca_after: player.ca,
                        stamina_before,
                        stamina_after: new_stamina,
                        condition: format!("{:?}", new_condition),
                        improved_attributes: result
                            .improved_attributes
                            .iter()
                            .map(|(name, growth)| AttributeImprovement {
                                attribute: name.clone(),
                                growth: *growth,
                            })
                            .collect(),
                        injury_occurred: result.injury_occurred,
                        message: result.message,
                        training_load: load_snapshot,
                        injury_risk,
                        coach_bonus_log: result.coach_bonus_log.clone(),
                    }
                }
                DailyActivityResult::TrainingFailed { reason } => {
                    return Err(format!("Training failed: {}", reason));
                }
                _ => {
                    return Err("Unexpected result type".to_string());
                }
            }
        }

        TrainingRequestType::ExecuteSpecialTraining { target, intensity } => {
            // 부상으로 인한 훈련 불가 체크
            if !player.can_train() {
                return serde_json::to_string(&TrainingResponse {
                    schema_version: 1,
                    success: false,
                    response_type: TrainingResponseType::CannotTrain {
                        reason: format!(
                            "부상으로 훈련 불가 (남은 회복 기간: {}일)",
                            player.current_injury.as_ref().map_or(0, |i| i.recovery_days_remaining)
                        ),
                    },
                    error_message: Some("Player is injured".to_string()),
                })
                .map_err(|e| e.to_string());
            }

            let training_target = parse_training_target(&target)?;
            let training_intensity = parse_intensity(&intensity)?;

            let ca_before = player.ca;
            let stamina_before = manager.stamina_system.current();

            let result = manager.execute_special_training(
                &mut player,
                training_target,
                training_intensity,
                request.seed,
            );

            match result {
                DailyActivityResult::TrainingCompleted { result, new_stamina, new_condition } => {
                    let load_snapshot = manager.training_load_snapshot();
                    let injury_risk =
                        manager.stamina_system.injury_risk() * load_snapshot.injury_risk_modifier;
                    TrainingResponseType::TrainingResult {
                        ca_before,
                        ca_after: player.ca,
                        stamina_before,
                        stamina_after: new_stamina,
                        condition: format!("{:?}", new_condition),
                        improved_attributes: result
                            .improved_attributes
                            .iter()
                            .map(|(name, growth)| AttributeImprovement {
                                attribute: name.clone(),
                                growth: *growth,
                            })
                            .collect(),
                        injury_occurred: result.injury_occurred,
                        message: result.message,
                        training_load: load_snapshot,
                        injury_risk,
                        coach_bonus_log: result.coach_bonus_log.clone(),
                    }
                }
                DailyActivityResult::TrainingFailed { reason } => {
                    return Err(format!("Training failed: {}", reason));
                }
                _ => {
                    return Err("Unexpected result type".to_string());
                }
            }
        }

        TrainingRequestType::ExecuteRest { forced } => {
            let stamina_before = manager.stamina_system.current();

            let result = manager.process_daily_activity(
                &mut player,
                &DaySlot::Rest { forced },
                request.seed,
            );

            match result {
                DailyActivityResult::RestCompleted {
                    stamina_recovered: _,
                    new_stamina,
                    new_condition,
                    was_forced,
                } => TrainingResponseType::RestResult {
                    stamina_before,
                    stamina_after: new_stamina,
                    condition: format!("{:?}", new_condition),
                    was_forced,
                },
                _ => {
                    return Err("Unexpected result type".to_string());
                }
            }
        }

        TrainingRequestType::GetStatus => {
            let injury_risk = calculate_injury_risk(manager.stamina_system.current());

            TrainingResponseType::Status {
                ca: player.ca,
                pa: player.pa,
                stamina: manager.stamina_system.current(),
                condition: format!("{:?}", manager.condition),
                consecutive_training_days: manager.consecutive_training_days,
                consecutive_rest_days: manager.consecutive_rest_days,
                injury_risk,
            }
        }

        TrainingRequestType::GetStatistics => {
            let stats = manager.get_training_stats();

            TrainingResponseType::Statistics {
                total_sessions: stats.total_sessions,
                total_ca_growth: stats.total_ca_growth,
                average_growth_per_session: stats.average_growth_per_session,
                injury_count: stats.injury_count,
            }
        }

        TrainingRequestType::GetRecommendation => {
            let recommendation = manager.recommend_activity();

            let (recommended, reason) = match recommendation {
                RecommendedActivity::PersonalTraining => {
                    ("personal_training", "체력이 충분하고 컨디션이 좋습니다")
                }
                RecommendedActivity::LightTraining => {
                    ("light_training", "체력이 적당하여 가벼운 훈련이 좋습니다")
                }
                RecommendedActivity::Rest => ("rest", "피로가 누적되어 휴식이 필요합니다"),
                RecommendedActivity::ForcedRest => {
                    ("forced_rest", "체력이 매우 부족하여 강제 휴식이 필요합니다")
                }
            };

            TrainingResponseType::Recommendation {
                recommended: recommended.to_string(),
                reason: reason.to_string(),
            }
        }

        TrainingRequestType::GetInjuryStatus => {
            let injury_info = player.current_injury.as_ref().map(|inj| InjuryInfo {
                injury_type: inj.injury_type.display_name().to_string(),
                severity: inj.severity.display_name().to_string(),
                recovery_days_remaining: inj.recovery_days_remaining,
                recovery_days_total: inj.recovery_days_total,
                recovery_progress: inj.recovery_progress(),
                affected_attributes: inj.affected_attributes.clone(),
            });

            TrainingResponseType::InjuryStatus {
                is_injured: player.is_injured(),
                can_train: player.can_train(),
                can_play: player.can_play_match(),
                injury: injury_info,
                injury_history_count: player.injury_history.len(),
            }
        }

        TrainingRequestType::AdvanceRecovery => {
            if let Some(recovered) = player.advance_recovery() {
                TrainingResponseType::RecoveryResult {
                    days_remaining: 0,
                    recovered: true,
                    injury_type: Some(recovered.injury_type.display_name().to_string()),
                }
            } else if let Some(injury) = &player.current_injury {
                TrainingResponseType::RecoveryResult {
                    days_remaining: injury.recovery_days_remaining,
                    recovered: false,
                    injury_type: Some(injury.injury_type.display_name().to_string()),
                }
            } else {
                TrainingResponseType::RecoveryResult {
                    days_remaining: 0,
                    recovered: false,
                    injury_type: None,
                }
            }
        }

        TrainingRequestType::UpdateWeeklyPlan { week_number, schedule } => {
            // 입력 변환
            let plan_schedule: Vec<(DayOfWeek, Vec<DaySlot>)> = schedule
                .into_iter()
                .map(|day_input| {
                    let day = parse_day_of_week(&day_input.day)?;
                    let slots = day_input
                        .slots
                        .into_iter()
                        .map(parse_day_slot_input)
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok((day, slots))
                })
                .collect::<Result<Vec<_>, String>>()?;

            // 업데이트
            manager.update_weekly_plan(week_number, plan_schedule)?;

            // 통계 계산
            let (training_count, rest_count, match_count) =
                if let Some(plan) = manager.get_weekly_plan(week_number) {
                    let mut t = 0u8;
                    let mut r = 0u8;
                    let mut m = 0u8;
                    for (_, slots) in &plan.schedule {
                        for slot in slots {
                            match slot {
                                DaySlot::TeamTraining(_) => t += 1,
                                DaySlot::Rest { .. } => r += 1,
                                DaySlot::Match { .. } => m += 1,
                                _ => {}
                            }
                        }
                    }
                    (t, r, m)
                } else {
                    (0, 0, 0)
                };

            TrainingResponseType::PlanUpdated {
                week_number,
                training_count,
                rest_count,
                match_count,
            }
        }

        TrainingRequestType::UpdateDaySlot { week_number, day, slots } => {
            let day_of_week = parse_day_of_week(&day)?;
            let day_slots =
                slots.into_iter().map(parse_day_slot_input).collect::<Result<Vec<_>, _>>()?;

            manager.update_day_slot(week_number, day_of_week, day_slots)?;

            // 통계 계산
            let (training_count, rest_count, match_count) =
                if let Some(plan) = manager.get_weekly_plan(week_number) {
                    let mut t = 0u8;
                    let mut r = 0u8;
                    let mut m = 0u8;
                    for (_, slots) in &plan.schedule {
                        for slot in slots {
                            match slot {
                                DaySlot::TeamTraining(_) => t += 1,
                                DaySlot::Rest { .. } => r += 1,
                                DaySlot::Match { .. } => m += 1,
                                _ => {}
                            }
                        }
                    }
                    (t, r, m)
                } else {
                    (0, 0, 0)
                };

            TrainingResponseType::PlanUpdated {
                week_number,
                training_count,
                rest_count,
                match_count,
            }
        }
    };

    // 응답 생성
    let response =
        TrainingResponse { schema_version: 1, success: true, response_type, error_message: None };

    // JSON 변환 및 반환
    serde_json::to_string(&response).map_err(|e| format!("Failed to serialize response: {}", e))
}

// 헬퍼 함수들
fn parse_training_target(target: &str) -> Result<TrainingTarget, String> {
    match target.to_lowercase().as_str() {
        "balanced" => Ok(TrainingTarget::Balanced),
        "pace" => Ok(TrainingTarget::Pace),
        "power" => Ok(TrainingTarget::Power),
        "technical" => Ok(TrainingTarget::Technical),
        "shooting" => Ok(TrainingTarget::Shooting),
        "passing" => Ok(TrainingTarget::Passing),
        "defending" => Ok(TrainingTarget::Defending),
        "mental" => Ok(TrainingTarget::Mental),
        "physical" | "endurance" => Ok(TrainingTarget::Endurance),
        _ => Err(format!("Unknown training target: {}", target)),
    }
}

fn parse_intensity(intensity: &str) -> Result<TrainingIntensity, String> {
    match intensity.to_lowercase().as_str() {
        "light" => Ok(TrainingIntensity::Light),
        "normal" => Ok(TrainingIntensity::Normal),
        "intensive" => Ok(TrainingIntensity::Intensive),
        _ => Err(format!("Unknown intensity: {}", intensity)),
    }
}

fn calculate_injury_risk(stamina: u8) -> f32 {
    match stamina {
        40..=100 => 0.01,
        30..=39 => 0.05,
        20..=29 => 0.15,
        10..=19 => 0.30,
        5..=9 => 0.50,
        _ => 0.80,
    }
}

fn parse_day_of_week(day: &str) -> Result<DayOfWeek, String> {
    match day.to_lowercase().as_str() {
        "monday" | "월요일" => Ok(DayOfWeek::Monday),
        "tuesday" | "화요일" => Ok(DayOfWeek::Tuesday),
        "wednesday" | "수요일" => Ok(DayOfWeek::Wednesday),
        "thursday" | "목요일" => Ok(DayOfWeek::Thursday),
        "friday" | "금요일" => Ok(DayOfWeek::Friday),
        "saturday" | "토요일" => Ok(DayOfWeek::Saturday),
        "sunday" | "일요일" => Ok(DayOfWeek::Sunday),
        _ => Err(format!("Invalid day: {}", day)),
    }
}

fn parse_day_slot_input(input: DaySlotInput) -> Result<DaySlot, String> {
    match input {
        DaySlotInput::TeamTraining { target } => {
            let t = parse_training_target(&target)?;
            Ok(DaySlot::TeamTraining(t))
        }
        DaySlotInput::FreeTime => Ok(DaySlot::FreeTime),
        DaySlotInput::Rest { forced } => Ok(DaySlot::Rest { forced }),
        DaySlotInput::Match { opponent, is_home } => Ok(DaySlot::Match { opponent, is_home }),
        DaySlotInput::SpecialEvent { name, description } => {
            Ok(DaySlot::SpecialEvent { name, description })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_training_target() {
        assert!(parse_training_target("balanced").is_ok());
        assert!(parse_training_target("pace").is_ok());
        assert!(parse_training_target("invalid").is_err());
    }

    #[test]
    fn test_parse_intensity() {
        assert!(parse_intensity("light").is_ok());
        assert!(parse_intensity("normal").is_ok());
        assert!(parse_intensity("intensive").is_ok());
        assert!(parse_intensity("invalid").is_err());
    }

    #[test]
    fn test_injury_risk_calculation() {
        assert_eq!(calculate_injury_risk(100), 0.01);
        assert_eq!(calculate_injury_risk(35), 0.05);
        assert_eq!(calculate_injury_risk(25), 0.15);
        assert_eq!(calculate_injury_risk(15), 0.30);
        assert_eq!(calculate_injury_risk(7), 0.50);
        assert_eq!(calculate_injury_risk(2), 0.80);
    }
}
