use of_engine::{Player as EngPlayer, Team as EngTeam};

#[cfg(feature = "vendor_skills")]
use of_engine::Skills36;

#[cfg(feature = "vendor_skills")]
use of_engine::{EngineTactics, MatchTacticType, TacticSelectionReason, TacticalStyle};

#[cfg(feature = "vendor_skills")]
use of_engine::{
    PersonAttributes, PersonBehaviour, PersonBehaviourState, Relations, Staff, StaffAttributes,
    StaffCoaching, StaffDataAnalysis, StaffGoalkeeperCoaching, StaffKnowledge, StaffLicenseType,
    StaffMedical, StaffMental,
};

/// 엔진으로 넘길 때 필요한 인터페이스.
/// - 42개 속성을 직접 사용하여 정확한 매핑
/// - attack/defense/goalkeeping 3개 속성 제거
pub trait EngineBridgePlayer {
    fn name(&self) -> &str;
    fn position(&self) -> &str; // 포지션 정보 필요
    fn ca(&self) -> u8; // CA 직접 전달
    fn condition(&self) -> f32; // 컨디션 정보
    fn player_attributes(&self) -> &of_core::models::player::PlayerAttributes; // 42개 속성 직접 전달

    /// 36속성 항상 제공 (42개 속성에서 직접 변환)
    #[cfg(feature = "vendor_skills")]
    fn skills36(&self) -> Skills36; // Option 제거, 항상 제공

    /// Fallback 모드용 간단한 값 (임시)
    #[cfg(not(feature = "vendor_skills"))]
    fn get_overall_rating(&self) -> u8 {
        self.ca()
    }
}

/// 코어 플레이어 → 엔진 플레이어 (새로운 구조)
pub fn to_engine_player<P: EngineBridgePlayer>(p: &P) -> EngPlayer {
    EngPlayer {
        name: p.name().to_string(),
        position: p.position().to_string(),
        attributes: p.player_attributes().clone(),
        ca: p.ca(),
        condition: p.condition(),
        #[cfg(feature = "vendor_skills")]
        skills36: Some(p.skills36()),
        ..Default::default()
    }
}

/// 코어 팀 → 엔진 팀
#[cfg(feature = "vendor_skills")]
pub fn to_engine_team<P: EngineBridgePlayer>(
    team_name: &str,
    players: &[P],
    formation: Option<String>,
    auto_select_tactics: bool,
    explicit_tactics: Option<EngineTactics>,
    preferred_style: Option<TacticalStyle>,
    substitutes: Option<&[P]>,
    captain_name: Option<String>,
    penalty_taker_name: Option<String>,
    free_kick_taker_name: Option<String>,
    auto_select_roles: bool,
) -> EngTeam {
    let substitute_players = substitutes
        .map(|subs| subs.iter().map(to_engine_player).collect())
        .unwrap_or_default();

    let mut team = EngTeam {
        name: team_name.to_string(),
        players: players.iter().map(to_engine_player).collect(),
        substitutes: substitute_players,
        formation,
        #[cfg(feature = "vendor_skills")]
        tactics: None,
        captain_name,
        penalty_taker_name,
        free_kick_taker_name,
        auto_select_roles,
        id: 0,
    };

    team.tactics = resolve_initial_tactics(
        &team,
        auto_select_tactics,
        explicit_tactics,
        preferred_style,
    );

    team
}

#[cfg(not(feature = "vendor_skills"))]
pub fn to_engine_team<P: EngineBridgePlayer>(
    team_name: &str,
    players: &[P],
    formation: Option<String>,
    substitutes: Option<&[P]>,
    captain_name: Option<String>,
    penalty_taker_name: Option<String>,
    free_kick_taker_name: Option<String>,
    auto_select_roles: bool,
) -> EngTeam {
    let substitute_players = substitutes
        .map(|subs| subs.iter().map(to_engine_player).collect())
        .unwrap_or_default();

    EngTeam {
        name: team_name.to_string(),
        players: players.iter().map(to_engine_player).collect(),
        substitutes: substitute_players,
        formation,
        captain_name,
        penalty_taker_name,
        free_kick_taker_name,
        auto_select_roles,
        ..Default::default()
    }
}

#[cfg(feature = "vendor_skills")]
fn resolve_initial_tactics(
    team: &EngTeam,
    auto_select_tactics: bool,
    explicit: Option<EngineTactics>,
    preferred_style: Option<TacticalStyle>,
) -> Option<EngineTactics> {
    if let Some(tactics) = explicit {
        return Some(tactics);
    }

    if let Some(formation) = team.formation.as_deref() {
        return Some(EngineTactics::new(parse_formation_to_tactic_type(
            formation,
        )));
    }

    if auto_select_tactics {
        return Some(determine_match_tactics(team, None, preferred_style, None));
    }

    None
}

#[cfg(feature = "vendor_skills")]
pub fn determine_match_tactics(
    team: &EngTeam,
    opponent: Option<&EngTeam>,
    preferred_style: Option<TacticalStyle>,
    fallback_formation: Option<&str>,
) -> EngineTactics {
    if let Some(style) = preferred_style {
        let tactic_type = tactic_for_style(style);
        let strength = calculate_strength_for_tactic(team, tactic_type);
        return EngineTactics::with_reason(
            tactic_type,
            TacticSelectionReason::CoachPreference,
            strength,
        );
    }

    if let Some(other) = opponent {
        if let Some(opponent_tactics) = &other.tactics {
            let counter = counter_tactic(opponent_tactics.tactic_type);
            let strength = calculate_strength_for_tactic(team, counter);
            return EngineTactics::with_reason(
                counter,
                TacticSelectionReason::OpponentCounter,
                strength,
            );
        }
    }

    if let Some(formation) = fallback_formation {
        let tactic_type = parse_formation_to_tactic_type(formation);
        let strength = calculate_strength_for_tactic(team, tactic_type);
        return EngineTactics::with_reason(tactic_type, TacticSelectionReason::Default, strength);
    }

    let (tactic_type, strength) = guess_tactic_from_players(team);
    EngineTactics::with_reason(
        tactic_type,
        TacticSelectionReason::TeamComposition,
        strength,
    )
}

/// Phase 2: 상황 인식 전술 선택
/// 모랄과 최근 경기 결과를 반영하여 전술 조정
#[cfg(feature = "vendor_skills")]
pub fn select_contextual_tactics(
    team: &EngTeam,
    team_morale: Option<f32>,
    recent_results: Option<&[String]>,
    preferred_style: Option<TacticalStyle>,
) -> EngineTactics {
    // 기본 전술 선택
    let base_tactics = determine_match_tactics(team, None, preferred_style, None);

    // 모랄 조정 계수
    let morale_modifier = match team_morale {
        Some(m) if m > 0.7 => 1.15, // 높은 모랄 → 공격적
        Some(m) if m < 0.3 => 0.85, // 낮은 모랄 → 보수적
        Some(m) if m > 0.5 => 1.05, // 평균 이상
        Some(_) => 0.95,            // 평균 이하
        None => 1.0,
    };

    // 최근 폼 조정 계수
    let form_modifier = if let Some(results) = recent_results {
        let losses = results.iter().filter(|r| r.as_str() == "L").count();
        let wins = results.iter().filter(|r| r.as_str() == "W").count();

        if losses >= 3 {
            0.80 // 3연패 이상 → 매우 보수적
        } else if losses >= 2 {
            0.90 // 2연패 → 약간 보수적
        } else if wins >= 3 {
            1.15 // 3연승 → 공격적
        } else if wins >= 2 {
            1.05 // 2연승 → 약간 공격적
        } else {
            1.0
        }
    } else {
        1.0
    };

    // 전술 강도 조정
    let adjusted_strength =
        (base_tactics.formation_strength * morale_modifier * form_modifier).clamp(0.2, 0.95);

    // 연패 시 포메이션 변경
    let final_tactic_type = if let Some(results) = recent_results {
        let losses = results.iter().filter(|r| r.as_str() == "L").count();
        if losses >= 3 {
            // 공격형에서 수비형으로
            match base_tactics.tactic_type {
                MatchTacticType::T433 | MatchTacticType::T343 => MatchTacticType::T451,
                MatchTacticType::T4231 => MatchTacticType::T4141,
                other => other,
            }
        } else {
            base_tactics.tactic_type
        }
    } else {
        base_tactics.tactic_type
    };

    EngineTactics::with_reason(
        final_tactic_type,
        TacticSelectionReason::Default, // TODO: Add ContextualAdjustment reason
        adjusted_strength,
    )
}

/// 상황 인식 전술 요약 정보
#[cfg(feature = "vendor_skills")]
pub fn get_contextual_tactics_summary(
    team_morale: Option<f32>,
    recent_results: Option<&[String]>,
) -> String {
    let mut reasons = Vec::new();

    if let Some(m) = team_morale {
        if m > 0.7 {
            reasons.push(format!("높은 모랄 ({:.0}%)", m * 100.0));
        } else if m < 0.3 {
            reasons.push(format!("낮은 모랄 ({:.0}%)", m * 100.0));
        }
    }

    if let Some(results) = recent_results {
        let losses = results.iter().filter(|r| r.as_str() == "L").count();
        let wins = results.iter().filter(|r| r.as_str() == "W").count();

        if losses >= 3 {
            reasons.push(format!("{}연패", losses));
        } else if wins >= 3 {
            reasons.push(format!("{}연승", wins));
        }
    }

    if reasons.is_empty() {
        "기본 전술".to_string()
    } else {
        reasons.join(", ")
    }
}

/// Phase 5: 상대 전술 카운터
/// 상대 포메이션을 분석하여 카운터 전술 자동 선택
#[cfg(feature = "vendor_skills")]
pub fn select_counter_tactic(team: &EngTeam, opponent_formation: &str) -> (EngineTactics, String) {
    // 상대 포메이션 파싱
    let opponent_type = parse_formation_to_tactic_type(opponent_formation);

    // 카운터 포메이션 결정
    let counter_type = counter_tactic(opponent_type);

    // 팀 적합도 계산
    let counter_tactics = EngineTactics::new(counter_type);
    let fitness = calculate_strength_for_tactic(team, counter_type);

    // 적합도가 너무 낮으면 팀 구성 기반으로 폴백
    if fitness < 0.4 {
        let (fallback_type, fallback_strength) = guess_tactic_from_players(team);
        let reason = format!(
            "카운터 전술 {:?} 적합도 낮음 ({:.0}%), 팀 구성 기반 {:?} 선택",
            counter_type,
            fitness * 100.0,
            fallback_type
        );
        let tactics = EngineTactics::with_reason(
            fallback_type,
            TacticSelectionReason::TeamComposition,
            fallback_strength,
        );
        return (tactics, reason);
    }

    let reason = format!(
        "상대 {} 카운터: {:?} (적합도 {:.0}%)",
        opponent_formation,
        counter_type,
        fitness * 100.0
    );

    let tactics = EngineTactics::with_reason(
        counter_type,
        TacticSelectionReason::OpponentCounter,
        fitness,
    );

    (tactics, reason)
}

/// 카운터 전술 정보 구조체
#[cfg(feature = "vendor_skills")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CounterTacticInfo {
    pub opponent_formation: String,
    pub counter_formation: String,
    pub counter_tactic_type: String,
    pub fitness: f32,
    pub is_fallback: bool,
    pub reason: String,
}

/// 카운터 전술 분석 (JSON 친화적)
#[cfg(feature = "vendor_skills")]
pub fn analyze_counter_tactic(team: &EngTeam, opponent_formation: &str) -> CounterTacticInfo {
    let opponent_type = parse_formation_to_tactic_type(opponent_formation);
    let counter_type = counter_tactic(opponent_type);
    let fitness = calculate_strength_for_tactic(team, counter_type);

    let is_fallback = fitness < 0.4;
    let (final_type, final_fitness, reason) = if is_fallback {
        let (fallback_type, fallback_strength) = guess_tactic_from_players(team);
        let reason = format!(
            "카운터 {:?} 적합도 {:.0}% (낮음) → 팀 구성 기반 {:?}",
            counter_type,
            fitness * 100.0,
            fallback_type
        );
        (fallback_type, fallback_strength, reason)
    } else {
        let reason = format!(
            "상대 {} → 카운터 {:?} (적합도 {:.0}%)",
            opponent_formation,
            counter_type,
            fitness * 100.0
        );
        (counter_type, fitness, reason)
    };

    CounterTacticInfo {
        opponent_formation: opponent_formation.to_string(),
        counter_formation: format!("{:?}", counter_type),
        counter_tactic_type: format!("{:?}", final_type),
        fitness: final_fitness,
        is_fallback,
        reason,
    }
}

/// Phase 4: 감독 성향 구조체
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Coach {
    pub name: String,
    pub tactical_knowledge: u8,   // 0-20
    pub attacking_preference: u8, // 0-20
    pub defending_preference: u8, // 0-20
    pub man_management: u8,       // 0-20
    pub discipline: u8,           // 0-20
}

impl Default for Coach {
    fn default() -> Self {
        Coach {
            name: "Default Coach".to_string(),
            tactical_knowledge: 10,
            attacking_preference: 10,
            defending_preference: 10,
            man_management: 10,
            discipline: 10,
        }
    }
}

/// JsonManager를 Coach로 변환
impl From<&crate::JsonManager> for Coach {
    fn from(json: &crate::JsonManager) -> Self {
        Coach {
            name: json.name.clone(),
            tactical_knowledge: json.knowledge.tactical_knowledge,
            attacking_preference: json.coaching.attacking,
            defending_preference: json.coaching.defending,
            man_management: json.mental.man_management,
            discipline: json.mental.discipline,
        }
    }
}

/// JsonManager를 engine Staff로 변환
/// SquadSelector::select() 호출에 필요
#[cfg(feature = "vendor_skills")]
pub fn json_manager_to_engine_staff(json: &crate::JsonManager) -> Staff {
    use chrono::NaiveDate;
    use of_engine::FullName;

    Staff {
        id: json.id,
        full_name: FullName::with_full(json.name.clone(), String::new(), json.name.clone()),
        contract: None,
        country_id: 0,
        behaviour: PersonBehaviour::default(),
        birth_date: NaiveDate::from_ymd_opt(1980, 1, 1).unwrap(),
        relations: Relations::new(),
        license: parse_license(&json.license),
        attributes: PersonAttributes {
            adaptability: json.mental.adaptability as f32 / 20.0,
            ambition: 0.5,
            controversy: 0.5,
            loyalty: 0.5,
            pressure: 0.5,
            professionalism: 0.5,
            sportsmanship: 0.5,
            temperament: 0.5,
        },
        staff_attributes: StaffAttributes {
            coaching: StaffCoaching {
                attacking: json.coaching.attacking,
                defending: json.coaching.defending,
                fitness: json.coaching.fitness,
                mental: json.coaching.mental,
                tactical: json.coaching.tactical,
                technical: json.coaching.technical,
                working_with_youngsters: json.coaching.working_with_youngsters,
            },
            goalkeeping: StaffGoalkeeperCoaching {
                distribution: 10,
                handling: 10,
                shot_stopping: 10,
            },
            mental: StaffMental {
                adaptability: json.mental.adaptability,
                determination: json.mental.determination,
                discipline: json.mental.discipline,
                man_management: json.mental.man_management,
                motivating: json.mental.motivating,
            },
            knowledge: StaffKnowledge {
                judging_player_ability: json.knowledge.judging_player_ability,
                judging_player_potential: json.knowledge.judging_player_potential,
                tactical_knowledge: json.knowledge.tactical_knowledge,
            },
            data_analysis: StaffDataAnalysis {
                judging_player_data: 10,
                judging_team_data: 10,
                presenting_data: 10,
            },
            medical: StaffMedical {
                physiotherapy: 10,
                sports_science: 10,
                non_player_tendencies: 10,
            },
        },
        focus: None,
        fatigue: 0.0,
        job_satisfaction: 0.5,
        recent_performance: Default::default(),
        coaching_style: Default::default(),
        training_schedule: vec![],
    }
}

#[cfg(feature = "vendor_skills")]
fn parse_license(license: &str) -> StaffLicenseType {
    match license.to_lowercase().as_str() {
        "continental_a" | "a" => StaffLicenseType::ContinentalA,
        "continental_b" | "b" => StaffLicenseType::ContinentalB,
        "continental_c" | "c" => StaffLicenseType::ContinentalC,
        "national_a" => StaffLicenseType::NationalA,
        "national_b" => StaffLicenseType::NationalB,
        "national_c" => StaffLicenseType::NationalC,
        "continental_pro" | "pro" => StaffLicenseType::ContinentalPro,
        _ => StaffLicenseType::NationalC,
    }
}

// =============================================================================
// CorePlayer/CoreTeam → Engine Player/Team 변환 (SquadSelector용)
// =============================================================================

/// CorePlayer를 engine의 실제 Player 타입으로 변환
/// SquadSelector::select() 호출에 필요
#[cfg(feature = "vendor_skills")]
pub fn core_player_to_engine_player(
    player: &crate::CorePlayer,
    id: u32,
) -> of_engine::EnginePlayer {
    use chrono::NaiveDate;
    use of_engine::{
        EnginePlayerAttributes, FullName, Mental, Physical, PlayerMailbox, PlayerPosition,
        PlayerPositionType, PlayerPositions, PlayerPreferredFoot, PlayerSkills, PlayerStatistics,
        PlayerStatisticsHistory, PlayerStatus, PlayerTraining, PlayerTrainingHistory, Technical,
    };

    // 포지션 문자열을 PlayerPositionType으로 변환
    let position_type = parse_position_to_type(&player.position);

    // 스킬 값 계산 (CA 기반, 0-20 스케일)
    let skill_value = (player.ca as f32 / 200.0 * 20.0).clamp(1.0, 20.0);

    // PlayerSkills 생성
    let skills = PlayerSkills {
        technical: Technical {
            corners: skill_value,
            crossing: skill_value,
            dribbling: skill_value,
            finishing: skill_value,
            first_touch: skill_value,
            free_kicks: skill_value,
            heading: skill_value,
            long_shots: skill_value,
            long_throws: skill_value,
            marking: skill_value,
            passing: skill_value,
            penalty_taking: skill_value,
            tackling: skill_value,
            technique: skill_value,
        },
        mental: Mental {
            aggression: skill_value,
            anticipation: skill_value,
            bravery: skill_value,
            composure: skill_value,
            concentration: skill_value,
            decisions: skill_value,
            determination: skill_value,
            flair: skill_value,
            leadership: skill_value,
            off_the_ball: skill_value,
            positioning: skill_value,
            teamwork: skill_value,
            vision: skill_value,
            work_rate: skill_value,
        },
        physical: Physical {
            acceleration: skill_value,
            agility: skill_value,
            balance: skill_value,
            jumping: skill_value,
            natural_fitness: skill_value,
            pace: skill_value,
            stamina: skill_value,
            strength: skill_value,
            match_readiness: skill_value,
        },
    };

    // PlayerPositions 생성 (주 포지션에 높은 레벨)
    let positions = PlayerPositions {
        positions: vec![PlayerPosition {
            position: position_type,
            level: 20, // 최대 레벨
        }],
    };

    // PlayerAttributes 생성
    let player_attributes = EnginePlayerAttributes {
        is_banned: false,
        is_injured: false,
        condition: (player.condition * 100.0) as i16,
        fitness: 100,
        jadedness: 0,
        weight: 75,
        height: 180,
        value: player.ca * 10000,
        current_reputation: player.ca as i16,
        home_reputation: player.ca as i16,
        world_reputation: player.ca as i16,
        current_ability: player.ca.min(200) as u8,
        potential_ability: player.pa.min(200) as u8,
        international_apps: 0,
        international_goals: 0,
        under_21_international_apps: 0,
        under_21_international_goals: 0,
    };

    of_engine::EnginePlayer::new(
        id,
        FullName::with_full(player.name.clone(), String::new(), player.name.clone()),
        NaiveDate::from_ymd_opt(1995, 1, 1).unwrap(),
        0, // country_id
        skills,
        PersonAttributes::default(),
        player_attributes,
        None, // contract
        positions,
    )
}

/// 포지션 문자열을 PlayerPositionType으로 변환
#[cfg(feature = "vendor_skills")]
fn parse_position_to_type(pos: &str) -> of_engine::PlayerPositionType {
    use of_engine::PlayerPositionType;

    match pos.to_uppercase().as_str() {
        "GK" => PlayerPositionType::Goalkeeper,
        "LB" | "DL" => PlayerPositionType::DefenderLeft,
        "CB" | "DC" => PlayerPositionType::DefenderCenter,
        "RB" | "DR" => PlayerPositionType::DefenderRight,
        "LWB" => PlayerPositionType::WingbackLeft,
        "RWB" => PlayerPositionType::WingbackRight,
        "DM" | "CDM" => PlayerPositionType::DefensiveMidfielder,
        "CM" | "MC" => PlayerPositionType::MidfielderCenter,
        "LM" | "ML" => PlayerPositionType::MidfielderLeft,
        "RM" | "MR" => PlayerPositionType::MidfielderRight,
        "AM" | "CAM" | "AMC" => PlayerPositionType::AttackingMidfielderCenter,
        "LW" | "AML" => PlayerPositionType::AttackingMidfielderLeft,
        "RW" | "AMR" => PlayerPositionType::AttackingMidfielderRight,
        "ST" | "CF" | "FW" => PlayerPositionType::Striker,
        _ => PlayerPositionType::MidfielderCenter, // 기본값
    }
}

/// CoreTeam을 engine의 실제 Team 타입으로 변환
/// SquadSelector::select() 호출에 필요
#[cfg(feature = "vendor_skills")]
pub fn core_team_to_engine_team(
    team: &crate::CoreTeam,
    team_id: u32,
    manager: Option<&crate::JsonManager>,
) -> (of_engine::EngineTeam, of_engine::Staff) {
    use chrono::NaiveTime;
    use of_engine::{
        EngineTeam, PlayerCollection, StaffCollection, StaffStub, Tactics, TeamReputation,
        TeamType, TrainingSchedule,
    };

    // 선수들 변환
    let players: Vec<of_engine::EnginePlayer> = team
        .players
        .iter()
        .enumerate()
        .map(|(idx, p)| core_player_to_engine_player(p, team_id * 1000 + idx as u32))
        .collect();

    // 교체 선수 변환
    let substitutes: Vec<of_engine::EnginePlayer> = team
        .substitutes
        .as_ref()
        .map(|subs| {
            subs.iter()
                .enumerate()
                .map(|(idx, p)| core_player_to_engine_player(p, team_id * 1000 + 100 + idx as u32))
                .collect()
        })
        .unwrap_or_default();

    let player_collection = PlayerCollection::new(players);
    let staff_collection = StaffCollection::new(vec![]);

    // Tactics 설정
    let tactics = team.formation.as_ref().map(|f| {
        let tactic_type = parse_formation_to_tactic_type(f);
        Tactics::new(tactic_type)
    });

    let mut engine_team = EngineTeam::new(
        team_id,
        0, // league_id
        0, // club_id
        team.name.clone(),
        team.name.to_lowercase().replace(' ', "-"),
        TeamType::Main,
        TrainingSchedule::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
        ),
        TeamReputation::new(50, 50, 50),
        player_collection,
        staff_collection,
    );

    // tactics 설정
    engine_team.tactics = tactics;

    // Staff 생성
    let staff = manager
        .map(json_manager_to_engine_staff)
        .unwrap_or_else(StaffStub::default);

    (engine_team, staff)
}

/// SquadSelector::select()를 직접 호출하여 최적 스쿼드 선택
#[cfg(feature = "vendor_skills")]
pub fn select_squad_with_engine(
    team: &crate::CoreTeam,
    manager: Option<&crate::JsonManager>,
) -> of_engine::PlayerSelectionResult {
    use of_engine::SquadSelector;

    let (engine_team, staff) = core_team_to_engine_team(team, 1, manager);
    SquadSelector::select(&engine_team, &staff)
}

/// Phase 4: 감독 영향 적용
#[cfg(feature = "vendor_skills")]
pub fn apply_coach_influence(tactics: &mut EngineTactics, coach: &Coach) -> CoachInfluenceResult {
    let mut result = CoachInfluenceResult {
        original_tactic: format!("{:?}", tactics.tactic_type),
        final_tactic: format!("{:?}", tactics.tactic_type),
        original_strength: tactics.formation_strength,
        final_strength: tactics.formation_strength,
        adjustments: vec![],
    };

    // 1. 전술 지식에 따른 포메이션 제한
    let allowed_formations = match coach.tactical_knowledge {
        0..=9 => vec![MatchTacticType::T442],
        10..=12 => vec![MatchTacticType::T442, MatchTacticType::T451],
        13..=15 => vec![
            MatchTacticType::T442,
            MatchTacticType::T451,
            MatchTacticType::T433,
            MatchTacticType::T4231,
        ],
        _ => vec![], // 모든 포메이션 허용
    };

    if !allowed_formations.is_empty() && !allowed_formations.contains(&tactics.tactic_type) {
        let original = tactics.tactic_type;
        tactics.tactic_type = MatchTacticType::T442;
        result.adjustments.push(format!(
            "전술 지식 부족 ({}) - {:?} → T442",
            coach.tactical_knowledge, original
        ));
        result.final_tactic = "T442".to_string();
    }

    // 2. 공격/수비 성향에 따른 강도 조정
    let attack_ratio = coach.attacking_preference as f32 / coach.defending_preference.max(1) as f32;

    let style_modifier = if attack_ratio > 1.3 {
        result.adjustments.push(format!(
            "공격 성향 ({}/{}) → 강도 +10%",
            coach.attacking_preference, coach.defending_preference
        ));
        1.1
    } else if attack_ratio < 0.7 {
        result.adjustments.push(format!(
            "수비 성향 ({}/{}) → 강도 -10%",
            coach.attacking_preference, coach.defending_preference
        ));
        0.9
    } else {
        1.0
    };

    // 3. 전술 지식 보너스
    let knowledge_bonus = (coach.tactical_knowledge as f32 - 10.0) / 100.0;
    if knowledge_bonus.abs() > 0.01 {
        result.adjustments.push(format!(
            "전술 지식 {} → 강도 {:+.0}%",
            coach.tactical_knowledge,
            knowledge_bonus * 100.0
        ));
    }

    // 4. 선수 관리 능력 보너스
    let management_bonus = (coach.man_management as f32 - 10.0) / 200.0;
    if management_bonus.abs() > 0.005 {
        result.adjustments.push(format!(
            "선수 관리 {} → 강도 {:+.0}%",
            coach.man_management,
            management_bonus * 100.0
        ));
    }

    // 최종 강도 계산
    let final_strength = tactics.formation_strength
        * style_modifier
        * (1.0 + knowledge_bonus)
        * (1.0 + management_bonus);

    tactics.formation_strength = final_strength.clamp(0.2, 0.95);
    result.final_strength = tactics.formation_strength;

    result
}

/// 감독 영향 결과
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoachInfluenceResult {
    pub original_tactic: String,
    pub final_tactic: String,
    pub original_strength: f32,
    pub final_strength: f32,
    pub adjustments: Vec<String>,
}

/// 감독 기반 전술 선택 및 적용
#[cfg(feature = "vendor_skills")]
pub fn select_tactics_with_coach(
    team: &EngTeam,
    coach: &Coach,
    preferred_style: Option<TacticalStyle>,
) -> (EngineTactics, CoachInfluenceResult) {
    // 기본 전술 선택
    let mut tactics = determine_match_tactics(team, None, preferred_style, None);

    // 감독 영향 적용
    let result = apply_coach_influence(&mut tactics, coach);

    (tactics, result)
}

#[cfg(feature = "vendor_skills")]
fn calculate_strength_for_tactic(team: &EngTeam, tactic_type: MatchTacticType) -> f32 {
    let (defenders, midfielders, forwards) = count_position_groups(team);
    let target = target_distribution(tactic_type);

    let diff = ((defenders as f32 - target.0).abs()
        + (midfielders as f32 - target.1).abs()
        + (forwards as f32 - target.2).abs())
        / 15.0;

    (0.5 - diff).clamp(0.2, 0.9)
}

#[cfg(feature = "vendor_skills")]
fn guess_tactic_from_players(team: &EngTeam) -> (MatchTacticType, f32) {
    let (defenders, midfielders, forwards) = count_position_groups(team);

    let tactic_type = if defenders >= 5 {
        MatchTacticType::T451
    } else if forwards >= 3 {
        MatchTacticType::T433
    } else if midfielders >= 5 {
        MatchTacticType::T4231
    } else {
        MatchTacticType::T442
    };

    let strength = calculate_strength_for_tactic(team, tactic_type);
    (tactic_type, strength)
}

#[cfg(feature = "vendor_skills")]
fn tactic_for_style(style: TacticalStyle) -> MatchTacticType {
    match style {
        TacticalStyle::Attacking | TacticalStyle::WidePlay => MatchTacticType::T433,
        TacticalStyle::Defensive | TacticalStyle::Compact => MatchTacticType::T451,
        TacticalStyle::Possession => MatchTacticType::T4231,
        TacticalStyle::WingPlay => MatchTacticType::T352,
        TacticalStyle::Counterattack => MatchTacticType::T4411,
        TacticalStyle::Experimental => MatchTacticType::T1333,
        TacticalStyle::Balanced => MatchTacticType::T442,
    }
}

#[cfg(feature = "vendor_skills")]
fn counter_tactic(reference: MatchTacticType) -> MatchTacticType {
    match reference {
        MatchTacticType::T433 => MatchTacticType::T451,
        MatchTacticType::T451 => MatchTacticType::T433,
        MatchTacticType::T4231 => MatchTacticType::T352,
        MatchTacticType::T352 => MatchTacticType::T4231,
        MatchTacticType::T442Diamond
        | MatchTacticType::T442DiamondWide
        | MatchTacticType::T442Narrow => MatchTacticType::T4231,
        MatchTacticType::T4141 | MatchTacticType::T4411 => MatchTacticType::T433,
        MatchTacticType::T343 => MatchTacticType::T451,
        MatchTacticType::T1333 | MatchTacticType::T4312 | MatchTacticType::T4222 => {
            MatchTacticType::T442
        }
        MatchTacticType::T442 => MatchTacticType::T4231,
    }
}

#[cfg(feature = "vendor_skills")]
fn target_distribution(tactic: MatchTacticType) -> (f32, f32, f32) {
    match tactic {
        MatchTacticType::T433 => (4.0, 3.0, 3.0),
        MatchTacticType::T451 => (4.0, 5.0, 1.0),
        MatchTacticType::T4231 => (4.0, 5.0, 1.0),
        MatchTacticType::T352 => (3.0, 5.0, 2.0),
        MatchTacticType::T442Diamond
        | MatchTacticType::T442DiamondWide
        | MatchTacticType::T442
        | MatchTacticType::T442Narrow => (4.0, 4.0, 2.0),
        MatchTacticType::T4141 => (4.0, 5.0, 1.0),
        MatchTacticType::T4411 => (4.0, 5.0, 1.0),
        MatchTacticType::T343 => (3.0, 4.0, 3.0),
        MatchTacticType::T1333 => (1.0, 6.0, 3.0),
        MatchTacticType::T4312 => (4.0, 5.0, 1.0),
        MatchTacticType::T4222 => (4.0, 4.0, 2.0),
    }
}

#[cfg(feature = "vendor_skills")]
pub fn parse_formation_to_tactic_type(formation: &str) -> MatchTacticType {
    match formation.trim() {
        "4-3-3" => MatchTacticType::T433,
        "4-5-1" => MatchTacticType::T451,
        "4-2-3-1" => MatchTacticType::T4231,
        "3-5-2" => MatchTacticType::T352,
        "4-4-2 Diamond" => MatchTacticType::T442Diamond,
        "4-4-2 Diamond Wide" => MatchTacticType::T442DiamondWide,
        "4-4-2 Narrow" => MatchTacticType::T442Narrow,
        "4-1-4-1" => MatchTacticType::T4141,
        "4-4-1-1" => MatchTacticType::T4411,
        "3-4-3" => MatchTacticType::T343,
        "1-3-3-3" => MatchTacticType::T1333,
        "4-3-1-2" => MatchTacticType::T4312,
        "4-2-2-2" => MatchTacticType::T4222,
        other if other.eq_ignore_ascii_case("3-5-2 wb") => MatchTacticType::T352,
        _ => MatchTacticType::T442,
    }
}

#[cfg(feature = "vendor_skills")]
fn count_position_groups(team: &EngTeam) -> (u32, u32, u32) {
    let mut defenders = 0;
    let mut midfielders = 0;
    let mut forwards = 0;

    for player in &team.players {
        match classify_position(&player.position) {
            PositionGroup::Defender => defenders += 1,
            PositionGroup::Midfielder => midfielders += 1,
            PositionGroup::Forward => forwards += 1,
            PositionGroup::Goalkeeper => {}
        }
    }

    (defenders, midfielders, forwards)
}

#[cfg(feature = "vendor_skills")]
fn classify_position(position: &str) -> PositionGroup {
    match position.to_ascii_uppercase().as_str() {
        "GK" => PositionGroup::Goalkeeper,
        "CB" | "LCB" | "RCB" | "LB" | "RB" | "LWB" | "RWB" | "SW" | "DF" => PositionGroup::Defender,
        "CDM" | "DM" | "CM" | "LCM" | "RCM" | "CAM" | "AM" | "LM" | "RM" | "MF" => {
            PositionGroup::Midfielder
        }
        "ST" | "CF" | "FW" | "LF" | "RF" | "LW" | "RW" | "WF" => PositionGroup::Forward,
        _ => PositionGroup::Midfielder,
    }
}

#[cfg(feature = "vendor_skills")]
enum PositionGroup {
    Goalkeeper,
    Defender,
    Midfielder,
    Forward,
}

#[cfg(feature = "vendor_skills")]
pub fn evaluate_formation_fitness(team: &EngTeam, formation: &str) -> (MatchTacticType, f32) {
    let tactic_type = parse_formation_to_tactic_type(formation);
    let fitness = calculate_strength_for_tactic(team, tactic_type);
    (tactic_type, fitness)
}

// 근사 매핑 함수 제거됨 - 이제 42개 속성에서 직접 1:1 매핑 사용

// ------------------------------------------------------------
// 기존 CorePlayer에 대한 기본 impl
// ------------------------------------------------------------
use crate::CorePlayer;
use of_core::player::types::CorePlayer as OfCorePlayer;

// of_core::CorePlayer (복잡한 구조)에 대한 구현
impl EngineBridgePlayer for OfCorePlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn position(&self) -> &str {
        // Position enum을 문자열로 변환
        match self.position {
            of_core::models::Position::GK => "GK",
            of_core::models::Position::LB => "LB",
            of_core::models::Position::CB => "CB",
            of_core::models::Position::RB => "RB",
            of_core::models::Position::LWB => "LWB",
            of_core::models::Position::RWB => "RWB",
            of_core::models::Position::CDM => "CDM",
            of_core::models::Position::CM => "CM",
            of_core::models::Position::CAM => "CAM",
            of_core::models::Position::LM => "LM",
            of_core::models::Position::RM => "RM",
            of_core::models::Position::LW => "LW",
            of_core::models::Position::RW => "RW",
            of_core::models::Position::ST => "ST",
            of_core::models::Position::CF => "CF",
            of_core::models::Position::FW => "FW",
            of_core::models::Position::DF => "DF",
            of_core::models::Position::MF => "MF",
        }
    }

    fn ca(&self) -> u8 {
        self.ca.min(255) as u8 // CA를 u8로 변환
    }

    fn condition(&self) -> f32 {
        1.0
    }

    fn player_attributes(&self) -> &of_core::models::player::PlayerAttributes {
        &self.detailed_stats
    }

    /// 36속성 정확한 매핑 (PlayerAttributes → Skills36)
    #[cfg(feature = "vendor_skills")]
    fn skills36(&self) -> Skills36 {
        // 42개 속성에서 36개로 정확한 1:1 매핑
        use crate::mapper::extract_skills36_from_player;
        extract_skills36_from_player(self)
    }
}

// crate::CorePlayer (간단한 구조)에 대한 구현
impl EngineBridgePlayer for CorePlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn position(&self) -> &str {
        &self.position
    }

    fn ca(&self) -> u8 {
        self.ca.min(255) as u8
    }

    fn condition(&self) -> f32 {
        self.condition
    }

    fn player_attributes(&self) -> &of_core::models::player::PlayerAttributes {
        // 간단한 CorePlayer를 위한 정적 기본값 반환
        // CA 기반으로 적절한 속성 값 생성 (CA 70 기준)
        static DEFAULT_ATTRIBUTES: once_cell::sync::Lazy<
            of_core::models::player::PlayerAttributes,
        > = once_cell::sync::Lazy::new(|| {
            let base_val = 10u8; // CA 70 정도의 선수 기준값
            of_core::models::player::PlayerAttributes {
                // Technical attributes (14)
                corners: base_val,
                crossing: base_val,
                dribbling: base_val,
                finishing: base_val,
                first_touch: base_val,
                free_kicks: base_val,
                heading: base_val,
                long_shots: base_val,
                long_throws: base_val,
                marking: base_val,
                passing: base_val,
                penalty_taking: base_val,
                tackling: base_val,
                technique: base_val,
                // Mental attributes (14)
                aggression: base_val,
                anticipation: base_val,
                bravery: base_val,
                composure: base_val,
                concentration: base_val,
                decisions: base_val,
                determination: base_val,
                flair: base_val,
                leadership: base_val,
                off_the_ball: base_val,
                positioning: base_val,
                teamwork: base_val,
                vision: base_val,
                work_rate: base_val,
                // Physical attributes (8) - OpenFootball standard
                acceleration: base_val,
                agility: base_val,
                balance: base_val,
                jumping: base_val,
                natural_fitness: base_val,
                pace: base_val,
                stamina: base_val,
                strength: base_val,
            }
        });
        &DEFAULT_ATTRIBUTES
    }

    #[cfg(feature = "vendor_skills")]
    fn skills36(&self) -> Skills36 {
        // Skills36은 0-100 스케일 (wrapper.rs에서 /5.0으로 0-20 변환)
        // CA를 그대로 사용 (CA 54 → skill 54 → /5.0 → 10.8)
        let base_skill = self.ca.min(100) as u8;
        Skills36 {
            // Technical
            corners: base_skill,
            crossing: base_skill,
            dribbling: base_skill,
            finishing: base_skill,
            first_touch: base_skill,
            free_kicks: base_skill,
            heading: base_skill,
            long_shots: base_skill,
            long_throws: base_skill,
            marking: base_skill,
            passing: base_skill,
            penalty_taking: base_skill,
            tackling: base_skill,
            technique: base_skill,
            // Mental
            aggression: base_skill,
            anticipation: base_skill,
            bravery: base_skill,
            composure: base_skill,
            concentration: base_skill,
            decisions: base_skill,
            determination: base_skill,
            flair: base_skill,
            leadership: base_skill,
            off_the_ball: base_skill,
            positioning: base_skill,
            teamwork: base_skill,
            vision: base_skill,
            work_rate: base_skill,
            // Physical
            acceleration: base_skill,
            agility: base_skill,
            balance: base_skill,
            jumping: base_skill,
            natural_fitness: base_skill,
            pace: base_skill,
            stamina: base_skill,
            strength: base_skill,
        }
    }
}

// =============================================================================
// Phase 1B: Engine Training Integration
// =============================================================================

/// Convert of_core TrainingTarget to engine TrainingType
#[cfg(feature = "vendor_skills")]
pub fn training_target_to_engine_type(
    target: &of_core::training::types::TrainingTarget,
) -> of_engine::EngineTrainingType {
    use of_core::training::types::TrainingTarget;
    use of_engine::EngineTrainingType;
    match target {
        TrainingTarget::Pace => EngineTrainingType::Speed,
        TrainingTarget::Power => EngineTrainingType::Strength,
        TrainingTarget::Technical => EngineTrainingType::BallControl,
        TrainingTarget::Shooting => EngineTrainingType::Shooting,
        TrainingTarget::Passing => EngineTrainingType::Passing,
        TrainingTarget::Defending => EngineTrainingType::Positioning,
        TrainingTarget::Mental => EngineTrainingType::VideoAnalysis,
        TrainingTarget::Endurance => EngineTrainingType::Endurance,
        TrainingTarget::Balanced => EngineTrainingType::TeamShape,
    }
}

/// Convert of_core TrainingIntensity to engine TrainingIntensity
#[cfg(feature = "vendor_skills")]
pub fn training_intensity_to_engine(
    intensity: &of_core::training::stamina::TrainingIntensity,
) -> of_engine::EngineTrainingIntensity {
    use of_core::training::stamina::TrainingIntensity;
    use of_engine::EngineTrainingIntensity;
    match intensity {
        TrainingIntensity::Rest => EngineTrainingIntensity::VeryLight,
        TrainingIntensity::Light => EngineTrainingIntensity::Light,
        TrainingIntensity::Normal => EngineTrainingIntensity::Moderate,
        TrainingIntensity::Intensive => EngineTrainingIntensity::High,
    }
}

/// Execute training using the engine's PlayerTraining system
/// Returns the training effects calculated by the engine
#[cfg(feature = "vendor_skills")]
pub fn execute_engine_training(
    player: &crate::CorePlayer,
    coach: &Staff,
    target: &of_core::training::types::TrainingTarget,
    intensity: &of_core::training::stamina::TrainingIntensity,
    date: chrono::NaiveDateTime,
) -> of_engine::PlayerTrainingResult {
    use of_engine::{EnginePlayerTraining, EngineTrainingSession};

    // Convert CorePlayer to engine Player
    let engine_player = core_player_to_engine_player(player, 1);

    // Create engine TrainingSession
    let engine_session = EngineTrainingSession {
        session_type: training_target_to_engine_type(target),
        intensity: training_intensity_to_engine(intensity),
        duration_minutes: 90, // Standard training duration
        focus_positions: vec![],
        participants: vec![],
    };

    // Execute training through engine
    EnginePlayerTraining::train(&engine_player, coach, &engine_session, date)
}

// =============================================================================
// Phase T1: Training Result with Injury Risk
// =============================================================================

/// Extended training result with injury risk and fatigue information
#[cfg(feature = "vendor_skills")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainingResultExtended {
    pub base_result: of_core::training::types::TrainingResult,
    pub injury_risk: f32,          // 0.0-1.0
    pub fatigue_increase: f32,     // Amount of fatigue added
    pub recovery_days: u8,         // Recommended recovery days
    pub risk_factors: Vec<String>, // Reasons for injury risk
}

/// Calculate fatigue increase based on training intensity
#[cfg(feature = "vendor_skills")]
fn calculate_fatigue_increase(intensity: &of_core::training::stamina::TrainingIntensity) -> f32 {
    use of_core::training::stamina::TrainingIntensity;
    match intensity {
        TrainingIntensity::Rest => 0.0,
        TrainingIntensity::Light => 0.05,
        TrainingIntensity::Normal => 0.10,
        TrainingIntensity::Intensive => 0.20,
    }
}

/// Execute training with extended injury risk information
#[cfg(feature = "vendor_skills")]
pub fn execute_training_with_risk(
    player: &crate::CorePlayer,
    coach: &Staff,
    target: &of_core::training::types::TrainingTarget,
    intensity: &of_core::training::stamina::TrainingIntensity,
    current_fatigue: f32, // Current player fatigue (0.0-1.0)
) -> TrainingResultExtended {
    let date = chrono::Utc::now().naive_utc();
    let engine_result = execute_engine_training(player, coach, target, intensity, date);

    let fatigue_increase = calculate_fatigue_increase(intensity);
    let base_injury_risk = engine_result.effects.injury_risk;

    // Adjust injury risk based on current fatigue
    let fatigue_modifier = if current_fatigue > 0.7 {
        1.5 // High fatigue increases injury risk by 50%
    } else if current_fatigue > 0.5 {
        1.2 // Moderate fatigue increases by 20%
    } else {
        1.0
    };

    // Adjust for low condition
    let condition_modifier = if player.condition < 0.5 {
        1.5
    } else if player.condition < 0.8 {
        1.2
    } else {
        1.0
    };

    let final_injury_risk =
        (base_injury_risk * fatigue_modifier * condition_modifier).clamp(0.0, 1.0);

    // Collect risk factors
    let mut risk_factors = Vec::new();
    if current_fatigue > 0.7 {
        risk_factors.push("높은 피로도".to_string());
    }
    if player.condition < 0.5 {
        risk_factors.push("낮은 컨디션".to_string());
    }
    if matches!(
        intensity,
        of_core::training::stamina::TrainingIntensity::Intensive
    ) {
        risk_factors.push("고강도 훈련".to_string());
    }
    if base_injury_risk > 0.15 {
        risk_factors.push("기본 부상 위험".to_string());
    }

    // Calculate recovery days
    let recovery_days = if final_injury_risk > 0.5 {
        3
    } else if final_injury_risk > 0.3 {
        2
    } else if final_injury_risk > 0.1 {
        1
    } else {
        0
    };

    // Create session for base result
    let session = of_core::training::types::TrainingSession {
        training_type: of_core::training::types::TrainingType::Individual,
        target: target.clone(),
        intensity: intensity.clone(),
        stamina_cost: match intensity {
            of_core::training::stamina::TrainingIntensity::Rest => 0,
            of_core::training::stamina::TrainingIntensity::Light => 5,
            of_core::training::stamina::TrainingIntensity::Normal => 10,
            of_core::training::stamina::TrainingIntensity::Intensive => 20,
        },
        base_effect: 1.0,
        coach_bonus: 0.0,
    };

    let base_result = engine_effects_to_training_result(&engine_result.effects, &session);

    TrainingResultExtended {
        base_result,
        injury_risk: final_injury_risk,
        fatigue_increase,
        recovery_days,
        risk_factors,
    }
}

/// Hybrid training execution - uses engine when available, falls back to of_core
/// This is the main entry point for training from Godot
pub fn execute_hybrid_training(
    player: &mut of_core::player::types::CorePlayer,
    session: &of_core::training::types::TrainingSession,
    condition: of_core::training::condition::Condition,
    seed: u64,
    #[cfg(feature = "vendor_skills")] coach: Option<&Staff>,
) -> of_core::training::types::TrainingResult {
    #[cfg(feature = "vendor_skills")]
    {
        // Try engine training if coach is available
        if let Some(coach) = coach {
            let date = chrono::Utc::now().naive_utc();

            // Convert CorePlayer to adapter's CorePlayer for engine bridge
            let adapter_player = crate::CorePlayer {
                name: player.name.clone(),
                ca: player.ca as u32,
                pa: player.pa as u32,
                position: format!("{:?}", player.position),
                condition: condition.efficiency_multiplier(),
            };

            // Execute through engine
            let engine_result = execute_engine_training(
                &adapter_player,
                coach,
                &session.target,
                &session.intensity,
                date,
            );

            // Convert engine result to of_core format and apply to player
            let result = engine_effects_to_training_result(&engine_result.effects, session);

            // Apply the attribute changes to the player
            for (attr_name, value) in &result.improved_attributes {
                if let Err(_) = apply_engine_gains_to_player(player, attr_name, *value) {
                    // Log error but continue
                }
            }

            return result;
        }
    }

    // Fallback: use of_core's TrainingEffectEngine
    of_core::training::effects::TrainingEffectEngine::execute_training(
        player,
        session,
        condition,
        seed,
        Vec::new(),
    )
}

/// Apply engine training gains to CorePlayer
#[cfg(feature = "vendor_skills")]
fn apply_engine_gains_to_player(
    player: &mut of_core::player::types::CorePlayer,
    attr_name: &str,
    value: f32,
) -> Result<(), String> {
    let increase = value.ceil() as u8;
    if increase == 0 {
        return Ok(());
    }

    let attrs = &mut player.detailed_stats;
    match attr_name {
        "stamina" => attrs.stamina = (attrs.stamina + increase).min(100),
        "strength" => attrs.strength = (attrs.strength + increase).min(100),
        "pace" => attrs.pace = (attrs.pace + increase).min(100),
        "agility" => attrs.agility = (attrs.agility + increase).min(100),
        "first_touch" => attrs.first_touch = (attrs.first_touch + increase).min(100),
        "passing" => attrs.passing = (attrs.passing + increase).min(100),
        "finishing" => attrs.finishing = (attrs.finishing + increase).min(100),
        "technique" => attrs.technique = (attrs.technique + increase).min(100),
        "positioning" => attrs.positioning = (attrs.positioning + increase).min(100),
        "decisions" => attrs.decisions = (attrs.decisions + increase).min(100),
        "vision" => attrs.vision = (attrs.vision + increase).min(100),
        "teamwork" => attrs.teamwork = (attrs.teamwork + increase).min(100),
        _ => return Err(format!("Unknown attribute: {}", attr_name)),
    }
    Ok(())
}

/// Convert engine TrainingEffects to of_core TrainingResult format
#[cfg(feature = "vendor_skills")]
pub fn engine_effects_to_training_result(
    effects: &of_engine::TrainingEffects,
    session: &of_core::training::types::TrainingSession,
) -> of_core::training::types::TrainingResult {
    let mut improved_attributes = Vec::new();

    // Collect physical gains
    if effects.physical_gains.stamina > 0.0 {
        improved_attributes.push(("stamina".to_string(), effects.physical_gains.stamina));
    }
    if effects.physical_gains.strength > 0.0 {
        improved_attributes.push(("strength".to_string(), effects.physical_gains.strength));
    }
    if effects.physical_gains.pace > 0.0 {
        improved_attributes.push(("pace".to_string(), effects.physical_gains.pace));
    }
    if effects.physical_gains.agility > 0.0 {
        improved_attributes.push(("agility".to_string(), effects.physical_gains.agility));
    }

    // Collect technical gains
    if effects.technical_gains.first_touch > 0.0 {
        improved_attributes.push((
            "first_touch".to_string(),
            effects.technical_gains.first_touch,
        ));
    }
    if effects.technical_gains.passing > 0.0 {
        improved_attributes.push(("passing".to_string(), effects.technical_gains.passing));
    }
    if effects.technical_gains.finishing > 0.0 {
        improved_attributes.push(("finishing".to_string(), effects.technical_gains.finishing));
    }
    if effects.technical_gains.technique > 0.0 {
        improved_attributes.push(("technique".to_string(), effects.technical_gains.technique));
    }

    // Collect mental gains
    if effects.mental_gains.positioning > 0.0 {
        improved_attributes.push(("positioning".to_string(), effects.mental_gains.positioning));
    }
    if effects.mental_gains.decisions > 0.0 {
        improved_attributes.push(("decisions".to_string(), effects.mental_gains.decisions));
    }
    if effects.mental_gains.vision > 0.0 {
        improved_attributes.push(("vision".to_string(), effects.mental_gains.vision));
    }
    if effects.mental_gains.teamwork > 0.0 {
        improved_attributes.push(("teamwork".to_string(), effects.mental_gains.teamwork));
    }

    // Calculate total CA change
    let ca_change: f32 = improved_attributes.iter().map(|(_, v)| v).sum();

    // Simple deterministic injury check based on risk threshold
    let injury_occurred = effects.injury_risk > 0.15;

    of_core::training::types::TrainingResult {
        session: session.clone(),
        actual_effect: ca_change,
        improved_attributes,
        ca_change,
        injury_occurred,
        message: if ca_change > 0.1 {
            "효과적인 훈련이었습니다!".to_string()
        } else {
            "훈련을 완료했습니다.".to_string()
        },
        coach_bonus_log: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CorePlayer;
    use of_core::player::types::CorePlayer as OfCorePlayer;

    #[test]
    fn test_engine_bridge_basic() {
        let player = CorePlayer {
            name: "Test Player".into(),
            ca: 100,
            pa: 150,
            position: "MF".into(),
            condition: 1.0,
        };

        let eng_player = to_engine_player(&player);
        assert_eq!(eng_player.name, "Test Player");
        // attack, defense, goalkeeping은 이제 기본값만 가짐
        // 실제 데이터는 skills36에 있음
        #[cfg(feature = "vendor_skills")]
        {
            assert!(eng_player.skills36.is_some());
        }
    }

    #[cfg(feature = "vendor_skills")]
    #[test]
    fn test_skills36_generation() {
        let player = CorePlayer {
            name: "FW Test".into(),
            ca: 150,
            pa: 180,
            position: "FW".into(),
            condition: 1.0,
        };

        let skills = player.skills36();
        // CorePlayer uses CA as base for all skills (no position-specific weighting)
        // Just verify skills are generated with reasonable values based on CA (150 → base 100 clamped)
        assert!(skills.finishing > 0);
        assert!(skills.passing > 0);
        // All skills should be equal for simple CorePlayer (CA-based)
        assert_eq!(skills.finishing, skills.marking);
    }

    #[cfg(feature = "vendor_skills")]
    fn build_engine_team(positions: &[&str]) -> of_engine::Team {
        use of_engine::Player as EnginePlayer;

        of_engine::Team {
            name: "Test".into(),
            players: positions
                .iter()
                .enumerate()
                .map(|(idx, pos)| EnginePlayer {
                    name: format!("Player {idx}"),
                    position: pos.to_string(),
                    attributes: of_core::models::player::PlayerAttributes::default(),
                    ca: 60,
                    condition: 1.0,
                    skills36: None,
                })
                .collect(),
            substitutes: vec![],
            formation: None,
            tactics: None,
            captain_name: None,
            penalty_taker_name: None,
            free_kick_taker_name: None,
            auto_select_roles: false,
        }
    }

    #[cfg(feature = "vendor_skills")]
    #[test]
    fn determine_tactics_prefers_style_hint() {
        let team = build_engine_team(&[
            "GK", "CB", "CB", "LB", "RB", "CM", "CM", "CM", "LW", "RW", "ST",
        ]);
        let tactics = determine_match_tactics(&team, None, Some(TacticalStyle::Attacking), None);
        assert_eq!(tactics.tactic_type, MatchTacticType::T433);
        assert_eq!(
            tactics.selected_reason,
            TacticSelectionReason::CoachPreference
        );
    }

    #[cfg(feature = "vendor_skills")]
    #[test]
    fn determine_tactics_counter_opponent_when_available() {
        let team = build_engine_team(&[
            "GK", "CB", "CB", "LB", "RB", "CM", "CM", "CM", "LW", "RW", "ST",
        ]);
        let mut opponent = build_engine_team(&[
            "GK", "CB", "CB", "LB", "RB", "CM", "CM", "CM", "LW", "RW", "ST",
        ]);
        opponent.tactics = Some(of_engine::EngineTactics::new(MatchTacticType::T433));

        let tactics = determine_match_tactics(&team, Some(&opponent), None, None);
        assert_eq!(tactics.tactic_type, MatchTacticType::T451);
        assert_eq!(
            tactics.selected_reason,
            TacticSelectionReason::OpponentCounter
        );
    }

    #[cfg(feature = "vendor_skills")]
    #[test]
    fn determine_tactics_uses_formation_hint() {
        let team = build_engine_team(&[
            "GK", "CB", "CB", "LB", "RB", "CM", "CM", "CM", "LW", "RW", "ST",
        ]);
        let tactics = determine_match_tactics(&team, None, None, Some("4-5-1"));
        assert_eq!(tactics.tactic_type, MatchTacticType::T451);
        assert_eq!(tactics.selected_reason, TacticSelectionReason::Default);
    }
}
