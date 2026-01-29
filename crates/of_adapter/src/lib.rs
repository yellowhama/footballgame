//! of_adapter: convert our domain types ↔ Open-Football engine types
//! and provide a clean API to run simulations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use of_core::replay::{GoalHeatSample, ReplayDoc};
use of_core::tactics::team_instructions::TeamInstructions;

#[cfg(feature = "vendor_skills")]
use of_engine::*; // re-export된 open_football 엔진 심볼

#[cfg(feature = "vendor_skills")]
use of_engine::{EngineTactics, PossessionEventKind, StoredEventKind, TacticalStyle};

pub mod engine_bridge;
pub mod mapper;
pub mod squad_selector;
pub mod tactical_analysis;

#[cfg(feature = "vendor_skills")]
pub use engine_bridge::{
    analyze_counter_tactic, apply_coach_influence, core_player_to_engine_player,
    core_team_to_engine_team, engine_effects_to_training_result, execute_engine_training,
    execute_hybrid_training, execute_training_with_risk, get_contextual_tactics_summary,
    json_manager_to_engine_staff, select_contextual_tactics, select_counter_tactic,
    select_squad_with_engine, select_tactics_with_coach, training_intensity_to_engine,
    training_target_to_engine_type, Coach, CoachInfluenceResult, CounterTacticInfo,
    TrainingResultExtended,
};
#[cfg(feature = "vendor_skills")]
pub use engine_bridge::{to_engine_player, to_engine_team, EngineBridgePlayer};

pub use squad_selector::{get_formation_positions, select_squad, PlayerScore, SquadSelection};
pub use tactical_analysis::{
    analyze_team_setup, PositionMismatch, TacticalAnalysis, TacticalRecommendation,
};

/// -----------------------------
/// 우리 프로젝트 고유 타입
/// -----------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorePlayer {
    pub name: String,
    pub ca: u32, // Current Ability
    pub pa: u32, // Potential Ability
    pub position: String,
    /// 컨디션: 0.0(최악, 부상/피로) ~ 1.0(최상)
    /// 기본값은 1.0으로 가정
    pub condition: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreTeam {
    pub name: String,
    pub players: Vec<CorePlayer>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub full_roster: Option<Vec<CorePlayer>>,
    #[serde(default)]
    pub auto_select_squad: bool,
    #[serde(default)]
    pub formation: Option<String>, // Backward compatibility
    #[cfg(feature = "vendor_skills")]
    #[serde(skip_serializing, skip_deserializing, default)]
    pub tactics: Option<EngineTactics>, // Explicit tactics from engine callers
    #[cfg(feature = "vendor_skills")]
    #[serde(skip_serializing, skip_deserializing, default)]
    pub preferred_style: Option<TacticalStyle>, // High-level tactical preference
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub substitutes: Option<Vec<CorePlayer>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub captain_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub penalty_taker_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub free_kick_taker_name: Option<String>,
    #[serde(default)]
    pub auto_select_roles: bool,
    /// Phase 2: 팀 모랄 (0.0 ~ 1.0)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub team_morale: Option<f32>,
    /// Phase 2: 최근 경기 결과 ("W", "D", "L")
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub recent_results: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchConfig {
    pub home: CoreTeam,
    pub away: CoreTeam,
    pub seed: u64,
    /// Automatically select tactics when not explicitly provided
    #[serde(default)]
    pub auto_select_tactics: bool,
    /// Phase 5: Home team tactical instructions (optional)
    #[serde(default)]
    pub home_instructions: Option<TeamInstructions>,
    /// Phase 5: Away team tactical instructions (optional)
    #[serde(default)]
    pub away_instructions: Option<TeamInstructions>,
    /// Highlight level for event filtering (optional, defaults to Simple)
    #[serde(default)]
    pub highlight_level: Option<String>,
    /// Player name for MyPlayer highlight level (optional)
    #[serde(default)]
    pub player_name: Option<String>,
    /// Tick interval in milliseconds (optional, defaults to 50ms)
    #[serde(default)]
    pub tick_interval_ms: Option<u64>,
    /// Include position tracking data in the result (Phase 4)
    #[serde(default)]
    pub include_position_data: bool,
    #[cfg(feature = "vendor_skills")]
    #[serde(skip_serializing, skip_deserializing, default)]
    pub kickoff_tactics: Option<EngineTactics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kickoff_team_instructions: Option<TeamInstructions>,
    /// Phase 2: 상황 인식 전술 선택 활성화
    #[serde(default)]
    pub use_contextual_tactics: bool,
    /// Include full StoredEvent payloads in the result (large payload)
    #[serde(default)]
    pub include_stored_events: bool,
    #[cfg(feature = "vendor_skills")]
    #[serde(skip_serializing, skip_deserializing, default)]
    pub home_manager: Option<JsonManager>,
    #[cfg(feature = "vendor_skills")]
    #[serde(skip_serializing, skip_deserializing, default)]
    pub away_manager: Option<JsonManager>,
    /// Enable extra time for knockout matches (2 x 15 minutes)
    #[serde(default)]
    pub allow_extra_time: bool,
    /// Enable penalty shootout after extra time draw
    #[serde(default)]
    pub allow_penalty_shootout: bool,
    /// Extra time half duration in minutes (default: 15)
    #[serde(default)]
    pub extra_time_minutes: Option<u32>,
    /// Golden goal rule (match ends on first goal in extra time)
    #[serde(default)]
    pub golden_goal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub score_home: u32,
    pub score_away: u32,
    pub replay: ReplayDoc,
    pub stats: serde_json::Value,
    #[serde(default)]
    pub engine_event_count: usize,
    #[cfg(feature = "vendor_skills")]
    pub home_tactic: Option<TacticSummary>,
    #[cfg(feature = "vendor_skills")]
    pub away_tactic: Option<TacticSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_data: Option<of_engine::PositionData>,
    #[serde(default)]
    pub goal_heat_samples: Vec<GoalHeatSample>,
    #[cfg(feature = "vendor_skills")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stored_events: Option<Vec<StoredEventRecord>>,
    /// Whether extra time was played
    #[serde(default)]
    pub extra_time_played: bool,
    /// Whether penalty shootout was played
    #[serde(default)]
    pub penalty_shootout_played: bool,
    /// Penalty shootout score (home, away) if played
    #[serde(skip_serializing_if = "Option::is_none")]
    pub penalty_score: Option<(u32, u32)>,
    /// Card status for players who received cards
    #[serde(default)]
    pub card_status: Vec<of_engine::PlayerCardStatus>,
}

#[cfg(feature = "vendor_skills")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticSummary {
    pub tactic_code: String,
    pub tactic_name: String,
    pub reason: String,
    pub formation_strength: f32,
}

#[cfg(feature = "vendor_skills")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoredEventData {
    Goal {
        player_id: u32,
        team_id: u32,
        is_own_goal: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        assist_player_id: Option<u32>,
    },
    Assist {
        player_id: u32,
        team_id: u32,
    },
    Pass {
        from_player_id: u32,
        to_player_id: u32,
        team_id: u32,
        distance: f32,
        force: f32,
        target: [f32; 3],
    },
    Shot {
        player_id: u32,
        team_id: u32,
        target: [f32; 3],
        force: f64,
    },
    Save {
        player_id: u32,
        team_id: u32,
    },
    Foul {
        player_id: u32,
        team_id: u32,
    },
    Tackle {
        player_id: u32,
        team_id: u32,
    },
    Possession {
        player_id: u32,
        team_id: u32,
        kind: StoredPossessionKind,
    },
    Clearance {
        player_id: u32,
        team_id: u32,
        target: [f32; 3],
    },
    Run {
        player_id: u32,
        team_id: u32,
        from: [f32; 3],
        to: [f32; 3],
        distance: f32,
        speed: f32,
        with_ball: bool,
    },
    Dribble {
        player_id: u32,
        team_id: u32,
        from: [f32; 3],
        to: [f32; 3],
        distance: f32,
        touches: u32,
    },
    ThroughBall {
        from_player_id: u32,
        to_player_id: u32,
        team_id: u32,
        from: [f32; 3],
        target: [f32; 3],
        distance: f32,
        force: f32,
    },
    YellowCard {
        player_id: u32,
        team_id: u32,
        yellow_count: u8,
    },
    RedCard {
        player_id: u32,
        team_id: u32,
        from_second_yellow: bool,
    },
    Offside {
        player_id: u32,
        team_id: u32,
        position: [f32; 2],
    },
    Communication {
        player_id: u32,
        team_id: u32,
        message: String,
        target: Option<[f32; 3]>,
    },
    Header {
        player_id: u32,
        team_id: u32,
        direction: [f32; 3],
    },
    Boundary {
        position: [f32; 2],
        last_touch_player_id: Option<u32>,
        last_touch_team_id: Option<u32>,
    },
}

#[cfg(feature = "vendor_skills")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEventRecord {
    pub timestamp: u64,
    #[serde(flatten)]
    pub data: StoredEventData,
}

#[cfg(feature = "vendor_skills")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredPossessionKind {
    Claim,
    Gain,
    Take,
}

#[cfg(feature = "vendor_skills")]
impl From<&EngineEvent> for StoredEventRecord {
    fn from(event: &EngineEvent) -> Self {
        let record = StoredEventRecord {
            timestamp: event.timestamp,
            data: StoredEventData::from(&event.kind),
        };

        // 🔍 DEBUG: Log first event conversion
        static FIRST_LOG: std::sync::Once = std::sync::Once::new();
        FIRST_LOG.call_once(|| {
            eprintln!("[of_adapter] 🔍 First StoredEventRecord conversion:");
            eprintln!("  - EngineEvent.timestamp: {}", event.timestamp);
            eprintln!("  - StoredEventRecord.timestamp: {}", record.timestamp);
            eprintln!("  - Event kind: {:?}", event.kind);
        });

        record
    }
}

#[cfg(feature = "vendor_skills")]
impl From<&StoredEventKind> for StoredEventData {
    fn from(kind: &StoredEventKind) -> Self {
        match kind {
            StoredEventKind::Goal {
                player_id,
                team_id,
                is_own_goal,
                assist_player_id,
            } => StoredEventData::Goal {
                player_id: *player_id,
                team_id: *team_id,
                is_own_goal: *is_own_goal,
                assist_player_id: *assist_player_id,
            },
            StoredEventKind::Assist { player_id, team_id } => StoredEventData::Assist {
                player_id: *player_id,
                team_id: *team_id,
            },
            StoredEventKind::Pass {
                from_player_id,
                to_player_id,
                team_id,
                distance,
                force,
                target,
                ground: _,
                ..
            } => StoredEventData::Pass {
                from_player_id: *from_player_id,
                to_player_id: *to_player_id,
                team_id: *team_id,
                distance: *distance,
                force: *force,
                target: *target,
            },
            StoredEventKind::Shot {
                player_id,
                team_id,
                target,
                force,
                xg: _,
                ..
            } => StoredEventData::Shot {
                player_id: *player_id,
                team_id: *team_id,
                target: *target,
                force: *force,
            },
            StoredEventKind::Save { player_id, team_id, .. } => StoredEventData::Save {
                player_id: *player_id,
                team_id: *team_id,
            },
            StoredEventKind::Foul { player_id, team_id, .. } => StoredEventData::Foul {
                player_id: *player_id,
                team_id: *team_id,
            },
            StoredEventKind::Tackle { player_id, team_id, .. } => StoredEventData::Tackle {
                player_id: *player_id,
                team_id: *team_id,
            },
            StoredEventKind::Possession {
                player_id,
                team_id,
                kind,
            } => StoredEventData::Possession {
                player_id: *player_id,
                team_id: *team_id,
                kind: StoredPossessionKind::from(*kind),
            },
            StoredEventKind::Clearance {
                player_id,
                team_id,
                target,
                ..
            } => StoredEventData::Clearance {
                player_id: *player_id,
                team_id: *team_id,
                target: *target,
            },
            StoredEventKind::Run {
                player_id,
                team_id,
                from,
                to,
                distance,
                speed,
                with_ball,
                ..
            } => StoredEventData::Run {
                player_id: *player_id,
                team_id: *team_id,
                from: *from,
                to: *to,
                distance: *distance,
                speed: *speed,
                with_ball: *with_ball,
            },
            StoredEventKind::Dribble {
                player_id,
                team_id,
                from,
                to,
                distance,
                touches,
                ..
            } => StoredEventData::Dribble {
                player_id: *player_id,
                team_id: *team_id,
                from: *from,
                to: *to,
                distance: *distance,
                touches: *touches,
            },
            StoredEventKind::ThroughBall {
                from_player_id,
                to_player_id,
                team_id,
                from,
                target,
                distance,
                force,
                ..
            } => StoredEventData::ThroughBall {
                from_player_id: *from_player_id,
                to_player_id: *to_player_id,
                team_id: *team_id,
                from: *from,
                target: *target,
                distance: *distance,
                force: *force,
            },
            StoredEventKind::YellowCard {
                player_id,
                team_id,
                yellow_count,
                ..
            } => StoredEventData::YellowCard {
                player_id: *player_id,
                team_id: *team_id,
                yellow_count: *yellow_count,
            },
            StoredEventKind::RedCard {
                player_id,
                team_id,
                from_second_yellow,
            } => StoredEventData::RedCard {
                player_id: *player_id,
                team_id: *team_id,
                from_second_yellow: *from_second_yellow,
            },
            StoredEventKind::Offside {
                player_id,
                team_id,
                position,
            } => StoredEventData::Offside {
                player_id: *player_id,
                team_id: *team_id,
                position: *position,
            },
            StoredEventKind::Communication {
                player_id,
                team_id,
                message,
                target,
                ..
        } => StoredEventData::Communication {
            player_id: *player_id,
            team_id: *team_id,
            message: message.clone(),
            target: *target,
        },
        StoredEventKind::Header {
            player_id,
            team_id,
            direction,
            ..
        } => StoredEventData::Header {
                player_id: *player_id,
                team_id: *team_id,
                direction: *direction,
            },
            StoredEventKind::Boundary {
                position,
                last_touch_player_id,
                last_touch_team_id,
            } => StoredEventData::Boundary {
                position: *position,
                last_touch_player_id: *last_touch_player_id,
                last_touch_team_id: *last_touch_team_id,
            },
        }
    }
}

#[cfg(feature = "vendor_skills")]
impl From<PossessionEventKind> for StoredPossessionKind {
    fn from(kind: PossessionEventKind) -> Self {
        match kind {
            PossessionEventKind::Claim => StoredPossessionKind::Claim,
            PossessionEventKind::Gain => StoredPossessionKind::Gain,
            PossessionEventKind::Take => StoredPossessionKind::Take,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationFitness {
    pub formation: String,
    pub tactic_code: String,
    pub tactic_name: String,
    pub fitness_score: f32,
    pub reason: String,
}

// =============================================================================
// Phase R1: Team Reputation System
// =============================================================================

/// Team reputation state with momentum and history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamReputationState {
    pub team_id: u32,
    pub team_name: String,
    pub local: u16,       // 0-1000
    pub national: u16,    // 0-1000
    pub continental: u16, // 0-1000
    pub world: u16,       // 0-1000
    pub momentum: f32,    // -1.0 ~ 1.0
    pub history: Vec<ReputationEvent>,
}

impl Default for TeamReputationState {
    fn default() -> Self {
        Self {
            team_id: 0,
            team_name: String::new(),
            local: 500,
            national: 500,
            continental: 300,
            world: 100,
            momentum: 0.0,
            history: Vec::new(),
        }
    }
}

/// A single reputation change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEvent {
    pub date: String, // "2025-11-21"
    pub event_type: ReputationEventType,
    pub change: i16,
    pub new_value: u16,
    pub description: String,
}

/// Types of events that can affect reputation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReputationEventType {
    MatchResult,
    LeaguePosition,
    Trophy,
    PlayerSigning,
    ManagerChange,
    MonthlyDecay,
}

/// Trophy tier for reputation boost calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrophyTier {
    Major, // League title, Champions League
    Minor, // Cup, secondary competitions
    Youth, // Youth tournaments
}

/// Sponsor tier based on reputation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SponsorTier {
    Local,
    National,
    Continental,
    Global,
}

/// Friendly match level based on reputation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FriendlyMatchLevel {
    Amateur,
    Professional,
    WorldClass,
}

/// Unlocked features based on reputation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationUnlocks {
    pub sponsor_tier: SponsorTier,
    pub friendly_match_level: FriendlyMatchLevel,
    pub transfer_budget_multiplier: f32,
    pub can_attract_star_players: bool,
}

impl TeamReputationState {
    /// Create a new reputation state for a team
    pub fn new(team_id: u32, team_name: String, initial_reputation: u16) -> Self {
        Self {
            team_id,
            team_name,
            local: initial_reputation,
            national: initial_reputation,
            continental: (initial_reputation as f32 * 0.6) as u16,
            world: (initial_reputation as f32 * 0.2) as u16,
            momentum: 0.0,
            history: Vec::new(),
        }
    }

    /// Check what features are unlocked at current reputation
    pub fn check_unlocks(&self) -> ReputationUnlocks {
        ReputationUnlocks {
            sponsor_tier: match self.national {
                0..=300 => SponsorTier::Local,
                301..=600 => SponsorTier::National,
                601..=850 => SponsorTier::Continental,
                _ => SponsorTier::Global,
            },
            friendly_match_level: match self.national {
                0..=400 => FriendlyMatchLevel::Amateur,
                401..=700 => FriendlyMatchLevel::Professional,
                _ => FriendlyMatchLevel::WorldClass,
            },
            transfer_budget_multiplier: 1.0 + (self.national as f32 / 1000.0),
            can_attract_star_players: self.national > 700,
        }
    }

    /// Update reputation based on match result
    pub fn update_from_match(
        &mut self,
        goals_for: u32,
        goals_against: u32,
        competition_importance: f32, // 0.0-1.0 (league=0.5, cup_final=1.0)
    ) {
        let goal_diff = goals_for as i32 - goals_against as i32;

        let base_change = match goal_diff {
            d if d >= 3 => 15,
            d if d >= 1 => 8,
            0 => 0,
            d if d >= -2 => -5,
            _ => -12,
        };

        let momentum_bonus = 1.0 + self.momentum * 0.2;
        let final_change = (base_change as f32 * competition_importance * momentum_bonus) as i16;

        // Update national reputation
        let old_value = self.national;
        self.national = (self.national as i32 + final_change as i32).clamp(0, 1000) as u16;

        // Update momentum
        self.momentum = (self.momentum + goal_diff as f32 * 0.05).clamp(-1.0, 1.0);

        // Update related reputations
        self.local = (self.local as i32 + final_change as i32 / 2).clamp(0, 1000) as u16;
        self.continental =
            (self.continental as i32 + final_change as i32 / 3).clamp(0, 1000) as u16;

        // Record history
        let result_text = if goal_diff > 0 {
            "승리"
        } else if goal_diff < 0 {
            "패배"
        } else {
            "무승부"
        };

        self.history.push(ReputationEvent {
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            event_type: ReputationEventType::MatchResult,
            change: final_change,
            new_value: self.national,
            description: format!(
                "{} ({}-{}) 평판 {:+}",
                result_text, goals_for, goals_against, final_change
            ),
        });
    }

    /// Process trophy win
    pub fn process_trophy(&mut self, trophy_name: &str, tier: TrophyTier) {
        let boost = match tier {
            TrophyTier::Major => 100,
            TrophyTier::Minor => 30,
            TrophyTier::Youth => 10,
        };

        let old_value = self.national;
        self.national = (self.national + boost).min(1000);
        self.continental = (self.continental + boost / 2).min(1000);
        self.world = (self.world + boost / 4).min(1000);
        self.momentum = 1.0; // Max momentum on trophy win

        self.history.push(ReputationEvent {
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            event_type: ReputationEventType::Trophy,
            change: boost as i16,
            new_value: self.national,
            description: format!("{} 우승! 평판 +{}", trophy_name, boost),
        });
    }

    /// Process star player signing
    pub fn process_player_signing(&mut self, player_name: &str, player_reputation: u16) {
        if player_reputation > 800 {
            let boost = ((player_reputation - 800) / 10) as u16;
            let old_value = self.national;
            self.national = (self.national + boost).min(1000);

            self.history.push(ReputationEvent {
                date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                event_type: ReputationEventType::PlayerSigning,
                change: boost as i16,
                new_value: self.national,
                description: format!("{} 영입! 평판 +{}", player_name, boost),
            });
        }
    }

    /// Apply monthly decay
    pub fn apply_monthly_decay(&mut self) {
        // Higher reputation decays faster
        let decay = (self.national as f32 * 0.02) as u16;
        let old_value = self.national;
        self.national = self.national.saturating_sub(decay);
        self.momentum *= 0.9; // Momentum also decays

        if decay > 0 {
            self.history.push(ReputationEvent {
                date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                event_type: ReputationEventType::MonthlyDecay,
                change: -(decay as i16),
                new_value: self.national,
                description: format!("월간 감쇠 평판 -{}", decay),
            });
        }
    }

    /// Update league position effect
    pub fn update_league_position(&mut self, position: u32, total_teams: u32) {
        let position_ratio = position as f32 / total_teams as f32;
        let change = if position_ratio <= 0.1 {
            20 // Top 10%
        } else if position_ratio <= 0.25 {
            10 // Top 25%
        } else if position_ratio <= 0.5 {
            0 // Top 50%
        } else if position_ratio <= 0.75 {
            -5 // Bottom half
        } else {
            -15 // Bottom 25%
        };

        let old_value = self.national;
        self.national = (self.national as i32 + change).clamp(0, 1000) as u16;

        self.history.push(ReputationEvent {
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            event_type: ReputationEventType::LeaguePosition,
            change: change as i16,
            new_value: self.national,
            description: format!("리그 {}위/{} 평판 {:+}", position, total_teams, change),
        });
    }
}

// =============================================================================
// Phase T2: Acute/Chronic Workload Ratio (ACWR) Tracking
// =============================================================================

/// Player workload tracking for injury prevention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerLoadTracker {
    pub player_name: String,
    pub daily_loads: Vec<f32>, // Last 28 days of training loads
    pub acute_load: f32,       // Sum of last 7 days
    pub chronic_load: f32,     // Average of last 28 days
    pub acwr: f32,             // Acute:Chronic Workload Ratio
    pub injury_risk_level: InjuryRiskLevel,
}

/// Injury risk level based on ACWR
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InjuryRiskLevel {
    Safe,     // ACWR 0.8-1.3 (optimal zone)
    Warning,  // ACWR 1.3-1.5 or 0.5-0.8
    HighRisk, // ACWR > 1.5 or < 0.5
}

impl Default for PlayerLoadTracker {
    fn default() -> Self {
        Self {
            player_name: String::new(),
            daily_loads: Vec::new(),
            acute_load: 0.0,
            chronic_load: 0.0,
            acwr: 1.0,
            injury_risk_level: InjuryRiskLevel::Safe,
        }
    }
}

impl PlayerLoadTracker {
    /// Create a new tracker for a player
    pub fn new(player_name: String) -> Self {
        Self {
            player_name,
            ..Default::default()
        }
    }

    /// Add a day's training load and recalculate metrics
    pub fn add_daily_load(&mut self, load: f32) {
        self.daily_loads.push(load);
        if self.daily_loads.len() > 28 {
            self.daily_loads.remove(0);
        }
        self.recalculate();
    }

    /// Add rest day (zero load)
    pub fn add_rest_day(&mut self) {
        self.add_daily_load(0.0);
    }

    /// Recalculate acute/chronic loads and ACWR
    fn recalculate(&mut self) {
        let len = self.daily_loads.len();
        self.acute_load = self.daily_loads.iter().rev().take(7).sum();

        if len > 0 {
            let chronic_sum: f32 = self.daily_loads.iter().sum();
            self.chronic_load = chronic_sum / len as f32 * 7.0;
        } else {
            self.chronic_load = 0.0;
        }

        self.acwr = if self.chronic_load > 0.1 {
            self.acute_load / self.chronic_load
        } else {
            1.0
        };

        self.injury_risk_level = match self.acwr {
            r if r >= 0.8 && r <= 1.3 => InjuryRiskLevel::Safe,
            r if (r > 1.3 && r <= 1.5) || (r >= 0.5 && r < 0.8) => InjuryRiskLevel::Warning,
            _ => InjuryRiskLevel::HighRisk,
        };
    }

    /// Get recommendation based on current ACWR
    pub fn get_recommendation(&self) -> String {
        match self.injury_risk_level {
            InjuryRiskLevel::Safe => "훈련 부하가 적정 범위입니다.".to_string(),
            InjuryRiskLevel::Warning => {
                if self.acwr > 1.3 {
                    format!(
                        "주의: 급성 부하 높음 (ACWR {:.2}). 강도를 줄이세요.",
                        self.acwr
                    )
                } else {
                    format!(
                        "주의: 훈련 부하 낮음 (ACWR {:.2}). 점진적으로 늘리세요.",
                        self.acwr
                    )
                }
            }
            InjuryRiskLevel::HighRisk => {
                if self.acwr > 1.5 {
                    format!("위험: 과훈련 (ACWR {:.2}). 즉시 휴식 필요!", self.acwr)
                } else {
                    format!("위험: 훈련 부족 (ACWR {:.2}). 체력 저하 중.", self.acwr)
                }
            }
        }
    }

    /// Calculate injury probability based on ACWR
    pub fn injury_probability(&self) -> f32 {
        match self.acwr {
            r if r >= 0.8 && r <= 1.3 => 0.05,
            r if r > 1.3 && r <= 1.5 => 0.15,
            r if r > 1.5 && r <= 2.0 => 0.30,
            r if r > 2.0 => 0.50,
            r if r >= 0.5 && r < 0.8 => 0.10,
            _ => 0.20,
        }
    }
}

/// Team-wide workload summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamLoadSummary {
    pub players_at_risk: Vec<String>,
    pub players_warning: Vec<String>,
    pub average_acwr: f32,
    pub recommended_intensity: String,
}

/// Calculate team load summary from individual trackers
pub fn calculate_team_load_summary(trackers: &[PlayerLoadTracker]) -> TeamLoadSummary {
    let mut at_risk = Vec::new();
    let mut warning = Vec::new();
    let mut total_acwr = 0.0;

    for tracker in trackers {
        total_acwr += tracker.acwr;
        match tracker.injury_risk_level {
            InjuryRiskLevel::HighRisk => at_risk.push(tracker.player_name.clone()),
            InjuryRiskLevel::Warning => warning.push(tracker.player_name.clone()),
            InjuryRiskLevel::Safe => {}
        }
    }

    let average_acwr = if !trackers.is_empty() {
        total_acwr / trackers.len() as f32
    } else {
        1.0
    };

    let recommended_intensity = if !at_risk.is_empty() {
        "휴식 또는 가벼운 훈련 권장".to_string()
    } else if average_acwr > 1.3 {
        "중간 강도 훈련 권장".to_string()
    } else if average_acwr < 0.8 {
        "강도 높은 훈련 가능".to_string()
    } else {
        "정상 훈련 진행".to_string()
    };

    TeamLoadSummary {
        players_at_risk: at_risk,
        players_warning: warning,
        average_acwr,
        recommended_intensity,
    }
}

// =============================================================================
// Phase S1 & S2: Staff State Tracking and Simulation
// =============================================================================

/// Staff state tracking for fatigue and satisfaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffState {
    pub id: u32,
    pub name: String,
    pub role: StaffRole,
    pub fatigue: f32,                // 0.0-1.0
    pub job_satisfaction: f32,       // 0.0-1.0
    pub training_effectiveness: f32, // 0.0-1.0
    pub health_status: StaffHealthStatus,
    pub weeks_worked: u32,
}

/// Staff roles in the club
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StaffRole {
    HeadCoach,
    AssistantCoach,
    FitnessCoach,
    GoalkeeperCoach,
    Scout,
    Physio,
    YouthCoach,
}

/// Staff health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StaffHealthStatus {
    Healthy,
    Fatigued,
    StressRelated,
    OnLeave,
}

/// Events generated by staff simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffEvent {
    pub staff_id: u32,
    pub event_type: StaffEventType,
    pub message: String,
}

/// Types of staff events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StaffEventType {
    LeaveRequest,       // 휴가 요청
    ContractDemand,     // 재계약 요구
    ResignationWarning, // 사직 경고
    HealthIssue,        // 건강 문제
    EfficiencyDrop,     // 효율 저하
}

impl Default for StaffState {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            role: StaffRole::AssistantCoach,
            fatigue: 0.0,
            job_satisfaction: 0.8,
            training_effectiveness: 1.0,
            health_status: StaffHealthStatus::Healthy,
            weeks_worked: 0,
        }
    }
}

impl StaffState {
    /// Create a new staff state
    pub fn new(id: u32, name: String, role: StaffRole) -> Self {
        Self {
            id,
            name,
            role,
            ..Default::default()
        }
    }

    /// Simulate a week's work and return any events
    pub fn simulate_week(&mut self, workload: f32) -> Vec<StaffEvent> {
        let mut events = Vec::new();
        self.weeks_worked += 1;

        // Fatigue increases with workload
        let fatigue_increase = workload * 0.1;
        self.fatigue = (self.fatigue + fatigue_increase).clamp(0.0, 1.0);

        // Satisfaction decreases with overwork
        if workload > 0.8 {
            self.job_satisfaction = (self.job_satisfaction - 0.05).max(0.0);
        } else if workload < 0.3 {
            // Light work slightly improves satisfaction
            self.job_satisfaction = (self.job_satisfaction + 0.02).min(1.0);
        }

        // Calculate training effectiveness
        self.training_effectiveness = (1.0 - self.fatigue * 0.5) * self.job_satisfaction;

        // Generate events based on state
        if self.fatigue > 0.8 {
            self.health_status = StaffHealthStatus::Fatigued;
            events.push(StaffEvent {
                staff_id: self.id,
                event_type: StaffEventType::LeaveRequest,
                message: format!(
                    "{}이(가) 휴가를 요청합니다. (피로도 {:.0}%)",
                    self.name,
                    self.fatigue * 100.0
                ),
            });
        }

        if self.job_satisfaction < 0.3 {
            events.push(StaffEvent {
                staff_id: self.id,
                event_type: StaffEventType::ResignationWarning,
                message: format!(
                    "{}이(가) 이직을 고려하고 있습니다. (만족도 {:.0}%)",
                    self.name,
                    self.job_satisfaction * 100.0
                ),
            });
        }

        if self.training_effectiveness < 0.5 {
            events.push(StaffEvent {
                staff_id: self.id,
                event_type: StaffEventType::EfficiencyDrop,
                message: format!(
                    "{}의 훈련 효율이 저하되었습니다. ({:.0}%)",
                    self.name,
                    self.training_effectiveness * 100.0
                ),
            });
        }

        // Health issues from prolonged fatigue
        if self.fatigue > 0.9 && self.weeks_worked % 4 == 0 {
            self.health_status = StaffHealthStatus::StressRelated;
            events.push(StaffEvent {
                staff_id: self.id,
                event_type: StaffEventType::HealthIssue,
                message: format!("{}이(가) 스트레스 관련 건강 문제를 호소합니다.", self.name),
            });
        }

        events
    }

    /// Rest the staff member
    pub fn rest(&mut self) {
        self.fatigue = (self.fatigue - 0.3).max(0.0);
        self.job_satisfaction = (self.job_satisfaction + 0.1).min(1.0);
        if self.fatigue < 0.5 {
            self.health_status = StaffHealthStatus::Healthy;
        }
        self.training_effectiveness = (1.0 - self.fatigue * 0.5) * self.job_satisfaction;
    }

    /// Put staff on leave
    pub fn take_leave(&mut self) {
        self.health_status = StaffHealthStatus::OnLeave;
        self.fatigue = 0.0;
        self.job_satisfaction = (self.job_satisfaction + 0.2).min(1.0);
    }

    /// Get status summary
    pub fn get_status_summary(&self) -> String {
        format!(
            "{} ({:?}): 피로 {:.0}%, 만족도 {:.0}%, 효율 {:.0}%",
            self.name,
            self.role,
            self.fatigue * 100.0,
            self.job_satisfaction * 100.0,
            self.training_effectiveness * 100.0
        )
    }
}

/// Team staff management
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamStaffManager {
    pub staff: Vec<StaffState>,
}

impl TeamStaffManager {
    /// Simulate a week for all staff
    pub fn simulate_week(&mut self, base_workload: f32) -> Vec<StaffEvent> {
        let mut all_events = Vec::new();

        for staff in &mut self.staff {
            // Adjust workload by role
            let role_modifier = match staff.role {
                StaffRole::HeadCoach => 1.2,
                StaffRole::AssistantCoach => 1.0,
                StaffRole::FitnessCoach => 1.1,
                StaffRole::GoalkeeperCoach => 0.8,
                StaffRole::Scout => 0.9,
                StaffRole::Physio => 1.0,
                StaffRole::YouthCoach => 0.7,
            };

            let workload = (base_workload * role_modifier).clamp(0.0, 1.0);
            let events = staff.simulate_week(workload);
            all_events.extend(events);
        }

        all_events
    }

    /// Get staff members who need attention
    pub fn get_staff_needing_attention(&self) -> Vec<&StaffState> {
        self.staff
            .iter()
            .filter(|s| s.fatigue > 0.7 || s.job_satisfaction < 0.4)
            .collect()
    }

    /// Get average training effectiveness
    pub fn average_effectiveness(&self) -> f32 {
        if self.staff.is_empty() {
            return 1.0;
        }
        let sum: f32 = self.staff.iter().map(|s| s.training_effectiveness).sum();
        sum / self.staff.len() as f32
    }
}

// =============================================================================
// Phase T3: Team Chemistry from Training
// =============================================================================

/// Chemistry change between players from training together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamChemistryChange {
    pub player_pairs: Vec<(String, String, f32)>, // (name1, name2, change)
    pub overall_change: f32,
}

/// Calculate chemistry improvements from training session
pub fn calculate_training_chemistry(
    participants: &[String],
    session_type: &of_core::training::types::TrainingTarget,
) -> TeamChemistryChange {
    use of_core::training::types::TrainingTarget;

    let mut pairs = Vec::new();
    let base_change = match session_type {
        TrainingTarget::Mental => 0.03, // Tactical training builds most chemistry
        TrainingTarget::Balanced => 0.02,
        TrainingTarget::Passing => 0.02,
        _ => 0.01,
    };

    // Generate pairs
    for i in 0..participants.len() {
        for j in (i + 1)..participants.len() {
            pairs.push((
                participants[i].clone(),
                participants[j].clone(),
                base_change,
            ));
        }
    }

    let overall = base_change * pairs.len() as f32;

    TeamChemistryChange {
        player_pairs: pairs,
        overall_change: overall,
    }
}

/// Full team chemistry state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamChemistry {
    pub overall: u8,          // 0-100
    pub attack_synergy: u8,   // 0-100
    pub midfield_synergy: u8, // 0-100
    pub defense_synergy: u8,  // 0-100
    pub momentum: f32,        // -1.0 to 1.0
}

/// Player chemistry information for calculation
#[derive(Debug, Clone)]
pub struct PlayerChemistryInfo {
    pub id: String,
    pub line: PlayerLine,
    pub nationality: String,
    pub tenure_months: u32,
    pub relationships: std::collections::HashMap<String, i8>, // player_id -> score (-100 to 100)
}

/// Player line position for chemistry grouping
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerLine {
    Attack,
    Midfield,
    Defense,
    Goalkeeper,
}

/// Match result type for chemistry calculation
#[derive(Debug, Clone, Copy)]
pub enum ChemistryMatchResult {
    Win,
    Draw,
    Loss,
}

impl TeamChemistry {
    /// Calculate team chemistry from player data
    pub fn calculate(
        players: &[PlayerChemistryInfo],
        recent_results: &[ChemistryMatchResult],
        manager_tenure_months: u32,
    ) -> Self {
        let mut chemistry = 50.0;

        // 1. Relationship bonus
        let relationship_bonus = Self::calc_relationship_bonus(players);
        chemistry += relationship_bonus;

        // 2. Recent results bonus
        let results_bonus = Self::calc_results_bonus(recent_results);
        chemistry += results_bonus;

        // 3. Manager tenure bonus (max +10)
        let tenure_bonus = (manager_tenure_months as f32 / 12.0).min(10.0);
        chemistry += tenure_bonus;

        // 4. Nationality grouping bonus
        let nationality_bonus = Self::calc_nationality_bonus(players);
        chemistry += nationality_bonus;

        // 5. Line synergies
        let attack = Self::calc_line_synergy(players, PlayerLine::Attack);
        let midfield = Self::calc_line_synergy(players, PlayerLine::Midfield);
        let defense = Self::calc_line_synergy(players, PlayerLine::Defense);

        // Calculate momentum from recent results
        let momentum: f32 = recent_results.iter().rev().take(5).fold(0.0, |acc, r| {
            acc + match r {
                ChemistryMatchResult::Win => 0.1,
                ChemistryMatchResult::Draw => 0.0,
                ChemistryMatchResult::Loss => -0.1,
            }
        });

        TeamChemistry {
            overall: chemistry.clamp(0.0, 100.0) as u8,
            attack_synergy: attack,
            midfield_synergy: midfield,
            defense_synergy: defense,
            momentum: momentum.clamp(-1.0, 1.0),
        }
    }

    fn calc_relationship_bonus(players: &[PlayerChemistryInfo]) -> f32 {
        if players.len() < 2 {
            return 0.0;
        }

        let mut total = 0.0;
        let mut pairs = 0;

        for i in 0..players.len() {
            for j in (i + 1)..players.len() {
                if let Some(&score) = players[i].relationships.get(&players[j].id) {
                    total += score as f32;
                    pairs += 1;
                }
            }
        }

        if pairs > 0 {
            (total / pairs as f32) / 10.0 // Normalize to ~-10 to +10
        } else {
            0.0
        }
    }

    fn calc_results_bonus(results: &[ChemistryMatchResult]) -> f32 {
        results.iter().rev().take(5).fold(0.0, |acc, r| {
            acc + match r {
                ChemistryMatchResult::Win => 2.0,
                ChemistryMatchResult::Draw => 0.0,
                ChemistryMatchResult::Loss => -2.0,
            }
        })
    }

    fn calc_nationality_bonus(players: &[PlayerChemistryInfo]) -> f32 {
        let mut nationality_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();

        for player in players {
            *nationality_counts.entry(&player.nationality).or_insert(0) += 1;
        }

        // Bonus for nationality groups of 3+
        let mut bonus = 0.0;
        for &count in nationality_counts.values() {
            if count >= 3 {
                bonus += (count as f32 - 2.0) * 1.0; // +1 per player beyond 2
            }
        }
        bonus.min(10.0) // Cap at +10
    }

    fn calc_line_synergy(players: &[PlayerChemistryInfo], line: PlayerLine) -> u8 {
        let line_players: Vec<_> = players.iter().filter(|p| p.line == line).collect();

        if line_players.is_empty() {
            return 50;
        }

        let mut synergy = 50.0;

        // Tenure bonus (longer together = better)
        let avg_tenure = line_players.iter().map(|p| p.tenure_months).sum::<u32>() as f32
            / line_players.len() as f32;
        synergy += (avg_tenure / 6.0).min(20.0); // Max +20 for 10+ years

        // Same nationality in line
        let first_nat = &line_players[0].nationality;
        let same_nat = line_players
            .iter()
            .filter(|p| p.nationality == *first_nat)
            .count();
        if same_nat >= 2 {
            synergy += (same_nat as f32 - 1.0) * 5.0;
        }

        synergy.clamp(0.0, 100.0) as u8
    }

    /// Get chemistry effect on match performance
    pub fn get_performance_modifier(&self) -> f32 {
        // Chemistry affects performance: 50 = neutral, 100 = +10%, 0 = -10%
        1.0 + (self.overall as f32 - 50.0) / 500.0
    }
}

// =============================================================================
// Phase S3: Team Responsibilities
// =============================================================================

/// Team responsibility assignments
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamResponsibilities {
    pub training_lead: Option<u32>, // 훈련 담당자 ID
    pub tactics_lead: Option<u32>,  // 전술 담당자 ID
    pub fitness_lead: Option<u32>,  // 피지컬 담당자 ID
    pub youth_lead: Option<u32>,    // 유소년 담당자 ID
    pub scouting_lead: Option<u32>, // 스카우트 담당자 ID
}

impl TeamResponsibilities {
    /// Assign a staff member to a responsibility
    pub fn assign(&mut self, staff_id: u32, role: ResponsibilityRole) {
        match role {
            ResponsibilityRole::Training => self.training_lead = Some(staff_id),
            ResponsibilityRole::Tactics => self.tactics_lead = Some(staff_id),
            ResponsibilityRole::Fitness => self.fitness_lead = Some(staff_id),
            ResponsibilityRole::Youth => self.youth_lead = Some(staff_id),
            ResponsibilityRole::Scouting => self.scouting_lead = Some(staff_id),
        }
    }

    /// Get workload multiplier for a staff member based on responsibilities
    pub fn get_workload_multiplier(&self, staff_id: u32) -> f32 {
        let mut count = 0;
        if self.training_lead == Some(staff_id) {
            count += 1;
        }
        if self.tactics_lead == Some(staff_id) {
            count += 1;
        }
        if self.fitness_lead == Some(staff_id) {
            count += 1;
        }
        if self.youth_lead == Some(staff_id) {
            count += 1;
        }
        if self.scouting_lead == Some(staff_id) {
            count += 1;
        }

        match count {
            0 => 0.5,
            1 => 1.0,
            2 => 1.3,
            _ => 1.5,
        }
    }
}

/// Responsibility roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponsibilityRole {
    Training,
    Tactics,
    Fitness,
    Youth,
    Scouting,
}

// =============================================================================
// Phase E1: Match Insights Calculator
// =============================================================================

/// Match insights calculated from stored events
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchInsights {
    pub total_passes: u32,
    pub successful_passes: u32,
    pub pass_accuracy: f32,
    pub total_shots: u32,
    pub shots_on_target: u32,
    pub shot_accuracy: f32,
    pub goals: u32,
    pub shot_conversion: f32,
    pub tackles_won: u32,
    pub saves: u32,
    pub fouls: u32,
    pub possession_gains: u32,
}

/// Calculate match insights from stored events
pub fn calculate_match_insights(events: &[StoredEventRecord]) -> MatchInsights {
    let mut insights = MatchInsights::default();

    for event in events {
        match &event.data {
            StoredEventData::Pass { .. } => {
                insights.total_passes += 1;
                insights.successful_passes += 1; // Assume recorded passes are successful
            }
            StoredEventData::ThroughBall { .. } => {
                insights.total_passes += 1;
                insights.successful_passes += 1;
            }
            StoredEventData::Shot { .. } => {
                insights.total_shots += 1;
                insights.shots_on_target += 1; // Simplification
            }
            StoredEventData::Goal { .. } => {
                insights.goals += 1;
            }
            StoredEventData::Tackle { .. } => {
                insights.tackles_won += 1;
            }
            StoredEventData::Save { .. } => {
                insights.saves += 1;
            }
            StoredEventData::Foul { .. } => {
                insights.fouls += 1;
            }
            StoredEventData::Possession { .. } => {
                insights.possession_gains += 1;
            }
            _ => {}
        }
    }

    // Calculate rates
    insights.pass_accuracy = if insights.total_passes > 0 {
        insights.successful_passes as f32 / insights.total_passes as f32
    } else {
        0.0
    };

    insights.shot_accuracy = if insights.total_shots > 0 {
        insights.shots_on_target as f32 / insights.total_shots as f32
    } else {
        0.0
    };

    insights.shot_conversion = if insights.total_shots > 0 {
        insights.goals as f32 / insights.total_shots as f32
    } else {
        0.0
    };

    insights
}

/// Get key insights summary as text
pub fn get_insights_summary(insights: &MatchInsights) -> Vec<String> {
    let mut summary = Vec::new();

    if insights.pass_accuracy > 0.85 {
        summary.push(format!(
            "우수한 패스 정확도: {:.0}%",
            insights.pass_accuracy * 100.0
        ));
    } else if insights.pass_accuracy < 0.7 {
        summary.push(format!(
            "패스 정확도 개선 필요: {:.0}%",
            insights.pass_accuracy * 100.0
        ));
    }

    if insights.shot_conversion > 0.2 {
        summary.push(format!(
            "높은 슈팅 성공률: {:.0}%",
            insights.shot_conversion * 100.0
        ));
    }

    if insights.tackles_won > 10 {
        summary.push(format!("적극적인 수비: {} 태클", insights.tackles_won));
    }

    if insights.saves > 5 {
        summary.push(format!("골키퍼 활약: {} 세이브", insights.saves));
    }

    summary
}

#[derive(Error, Debug)]
pub enum SimError {
    #[error("engine error: {0}")]
    Engine(String),
    #[error("conversion error: {0}")]
    Conversion(String),
}

/// -----------------------------
/// JSON 데이터 구조체 (stage_teams_safe.json, dummy_managers.json)
/// -----------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPlayerAttributes {
    pub crossing: u8,
    pub finishing: u8,
    pub heading_accuracy: u8,
    pub short_passing: u8,
    pub volleys: u8,
    pub dribbling: u8,
    pub curve: u8,
    pub free_kick_accuracy: u8,
    pub long_passing: u8,
    pub ball_control: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPlayerMental {
    pub positioning: u8,
    pub vision: u8,
    pub composure: u8,
    pub concentration: u8,
    pub decisions: u8,
    pub determination: u8,
    pub anticipation: u8,
    pub teamwork: u8,
    pub work_rate: u8,
    pub aggression: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPlayerPhysical {
    pub acceleration: u8,
    pub agility: u8,
    pub balance: u8,
    pub jumping: u8,
    pub natural_fitness: u8,
    pub pace: u8,
    pub stamina: u8,
    pub strength: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPlayer {
    #[serde(default)]
    pub id: u32,
    pub name: String,
    pub ca: u32,
    pub pa: u32,
    pub age: u32,
    pub position: String,
    #[serde(default)]
    pub is_player_character: bool,
    pub technical: JsonPlayerAttributes,
    pub mental: JsonPlayerMental,
    pub physical: JsonPlayerPhysical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonTeam {
    pub team_id: u32,
    #[serde(default)]
    pub manager_id: u32,
    pub club_name: String,
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub division: String,
    pub avg_ca: f32,
    #[serde(default)]
    pub formation: String,
    pub squad: Vec<JsonPlayer>,
    #[serde(default)]
    pub is_player_team: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonManagerCoaching {
    pub attacking: u8,
    pub defending: u8,
    pub fitness: u8,
    pub mental: u8,
    pub tactical: u8,
    pub technical: u8,
    pub working_with_youngsters: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonManagerMental {
    pub adaptability: u8,
    pub determination: u8,
    pub discipline: u8,
    pub man_management: u8,
    pub motivating: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonManagerKnowledge {
    pub judging_player_ability: u8,
    pub judging_player_potential: u8,
    pub tactical_knowledge: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonManager {
    pub id: u32,
    pub name: String,
    pub coaching_style: String,
    pub license: String,
    pub coaching: JsonManagerCoaching,
    pub mental: JsonManagerMental,
    pub knowledge: JsonManagerKnowledge,
    #[serde(default)]
    pub description: String,
}

/// -----------------------------
/// JSON → CoreTeam 변환
/// -----------------------------

impl From<&JsonPlayer> for CorePlayer {
    fn from(json: &JsonPlayer) -> Self {
        CorePlayer {
            name: json.name.clone(),
            ca: json.ca,
            pa: json.pa,
            position: json.position.clone(),
            condition: 1.0, // Default full condition
        }
    }
}

impl From<&JsonTeam> for CoreTeam {
    fn from(json: &JsonTeam) -> Self {
        CoreTeam {
            name: json.club_name.clone(),
            players: json.squad.iter().map(CorePlayer::from).collect(),
            full_roster: None,
            auto_select_squad: false,
            formation: if json.formation.is_empty() {
                None
            } else {
                Some(json.formation.clone())
            },
            #[cfg(feature = "vendor_skills")]
            tactics: None,
            #[cfg(feature = "vendor_skills")]
            preferred_style: None,
            substitutes: None,
            captain_name: None,
            penalty_taker_name: None,
            free_kick_taker_name: None,
            auto_select_roles: false,
            team_morale: None,
            recent_results: None,
        }
    }
}

/// Load teams from JSON file
pub fn load_teams_from_json(json_str: &str) -> Result<Vec<JsonTeam>, SimError> {
    serde_json::from_str(json_str)
        .map_err(|e| SimError::Conversion(format!("Failed to parse teams JSON: {}", e)))
}

/// Load managers from JSON file
pub fn load_managers_from_json(json_str: &str) -> Result<Vec<JsonManager>, SimError> {
    #[derive(Deserialize)]
    struct ManagersFile {
        managers: Vec<JsonManager>,
    }

    let file: ManagersFile = serde_json::from_str(json_str)
        .map_err(|e| SimError::Conversion(format!("Failed to parse managers JSON: {}", e)))?;

    Ok(file.managers)
}

/// Find manager by ID
pub fn find_manager(managers: &[JsonManager], manager_id: u32) -> Option<&JsonManager> {
    managers.iter().find(|m| m.id == manager_id)
}

/// Convert JsonTeam to CoreTeam with manager info
pub fn json_team_to_core(team: &JsonTeam, _manager: Option<&JsonManager>) -> CoreTeam {
    CoreTeam::from(team)
}

fn prepare_roster_input(team: &CoreTeam) -> CoreTeam {
    if !team.auto_select_squad {
        return team.clone();
    }

    let mut clone = team.clone();
    if let Some(full_roster) = &team.full_roster {
        clone.players = full_roster.clone();
    }
    clone
}

#[cfg(not(feature = "vendor_skills"))]
fn finalize_team_without_vendor(mut team: CoreTeam) -> CoreTeam {
    if !team.auto_select_squad {
        return team;
    }

    let roster = if let Some(full) = &team.full_roster {
        full.clone()
    } else {
        team.players.clone()
    };

    if roster.is_empty() {
        return team;
    }

    let starter_count = roster.len().min(11);
    team.players = roster[..starter_count].to_vec();
    let bench_end = (starter_count + 7).min(roster.len());
    let bench_slice = &roster[starter_count..bench_end];
    team.substitutes = if bench_slice.is_empty() {
        None
    } else {
        Some(bench_slice.to_vec())
    };
    team
}

/// Get player team from list
pub fn find_player_team(teams: &[JsonTeam]) -> Option<&JsonTeam> {
    teams.iter().find(|t| t.is_player_team)
}

/// Select optimal squad for JsonTeam
///
/// Converts JsonTeam players to the format needed by squad_selector
/// and returns the selection result.
pub fn select_squad_for_json_team(team: &JsonTeam, formation: Option<&str>) -> SquadSelection {
    use std::collections::HashMap;

    let formation = formation.unwrap_or_else(|| {
        if team.formation.is_empty() {
            "T442"
        } else {
            &team.formation
        }
    });

    // Convert JsonPlayers to the tuple format expected by select_squad
    let players: Vec<(
        String,
        String,
        u32,
        f32,
        HashMap<String, u8>,
        HashMap<String, u8>,
        HashMap<String, u8>,
    )> = team
        .squad
        .iter()
        .map(|p| {
            let mut tech = HashMap::new();
            tech.insert("crossing".to_string(), p.technical.crossing);
            tech.insert("finishing".to_string(), p.technical.finishing);
            tech.insert("heading_accuracy".to_string(), p.technical.heading_accuracy);
            tech.insert("short_passing".to_string(), p.technical.short_passing);
            tech.insert("volleys".to_string(), p.technical.volleys);
            tech.insert("dribbling".to_string(), p.technical.dribbling);
            tech.insert("curve".to_string(), p.technical.curve);
            tech.insert(
                "free_kick_accuracy".to_string(),
                p.technical.free_kick_accuracy,
            );
            tech.insert("long_passing".to_string(), p.technical.long_passing);
            tech.insert("ball_control".to_string(), p.technical.ball_control);

            let mut mental = HashMap::new();
            mental.insert("positioning".to_string(), p.mental.positioning);
            mental.insert("vision".to_string(), p.mental.vision);
            mental.insert("composure".to_string(), p.mental.composure);
            mental.insert("concentration".to_string(), p.mental.concentration);
            mental.insert("decisions".to_string(), p.mental.decisions);
            mental.insert("determination".to_string(), p.mental.determination);
            mental.insert("anticipation".to_string(), p.mental.anticipation);
            mental.insert("teamwork".to_string(), p.mental.teamwork);
            mental.insert("work_rate".to_string(), p.mental.work_rate);
            mental.insert("aggression".to_string(), p.mental.aggression);

            let mut phys = HashMap::new();
            phys.insert("acceleration".to_string(), p.physical.acceleration);
            phys.insert("agility".to_string(), p.physical.agility);
            phys.insert("balance".to_string(), p.physical.balance);
            phys.insert("jumping".to_string(), p.physical.jumping);
            phys.insert("natural_fitness".to_string(), p.physical.natural_fitness);
            phys.insert("pace".to_string(), p.physical.pace);
            phys.insert("stamina".to_string(), p.physical.stamina);
            phys.insert("strength".to_string(), p.physical.strength);

            (
                p.name.clone(),
                p.position.clone(),
                p.ca,
                1.0, // Default condition - can be extended later
                tech,
                mental,
                phys,
            )
        })
        .collect();

    select_squad(&players, formation)
}

/// -----------------------------
/// 변환 & 실행 API
/// -----------------------------

pub fn convert_team(
    core: &CoreTeam,
    auto_select_tactics: bool,
) -> Result<of_engine::Team, SimError> {
    let substitutes = core.substitutes.as_deref();
    #[cfg(feature = "vendor_skills")]
    {
        Ok(engine_bridge::to_engine_team(
            &core.name,
            &core.players,
            core.formation.clone(),
            auto_select_tactics,
            core.tactics.clone(),
            core.preferred_style.clone(),
            substitutes,
            core.captain_name.clone(),
            core.penalty_taker_name.clone(),
            core.free_kick_taker_name.clone(),
            core.auto_select_roles,
        ))
    }
    #[cfg(not(feature = "vendor_skills"))]
    {
        let _ = auto_select_tactics;
        Ok(engine_bridge::to_engine_team(
            &core.name,
            &core.players,
            core.formation.clone(),
            substitutes,
            core.captain_name.clone(),
            core.penalty_taker_name.clone(),
            core.free_kick_taker_name.clone(),
            core.auto_select_roles,
        ))
    }
}

#[cfg(feature = "vendor_skills")]
fn summarize_engine_tactics(tactics: &of_engine::EngineTactics) -> TacticSummary {
    TacticSummary {
        tactic_code: format!("{:?}", tactics.tactic_type),
        tactic_name: tactics.tactic_type.display_name().to_string(),
        reason: format!("{:?}", tactics.selected_reason),
        formation_strength: tactics.formation_strength,
    }
}

#[cfg(feature = "vendor_skills")]
pub fn calculate_formation_fitness(team: &CoreTeam, formation: &str) -> FormationFitness {
    use of_engine::Team as EngineTeam;

    let engine_players: Vec<of_engine::Player> =
        team.players.iter().map(to_engine_player).collect();
    let engine_team = EngineTeam {
        name: team.name.clone(),
        players: engine_players,
        substitutes: Vec::new(),
        formation: Some(formation.to_string()),
        tactics: None,
        captain_name: None,
        penalty_taker_name: None,
        free_kick_taker_name: None,
        auto_select_roles: false,
        id: 0,
    };

    let (tactic_type, fitness_score) =
        engine_bridge::evaluate_formation_fitness(&engine_team, formation);

    FormationFitness {
        formation: formation.to_string(),
        tactic_code: format!("{:?}", tactic_type),
        tactic_name: tactic_type.display_name().to_string(),
        fitness_score,
        reason: format!(
            "{} fitness {:.1}%",
            tactic_type.display_name(),
            fitness_score * 100.0
        ),
    }
}

#[cfg(not(feature = "vendor_skills"))]
pub fn calculate_formation_fitness(team: &CoreTeam, formation: &str) -> FormationFitness {
    let _ = team;
    FormationFitness {
        formation: formation.to_string(),
        tactic_code: formation.to_string(),
        tactic_name: formation.to_string(),
        fitness_score: 0.5,
        reason: "Formation fitness available only in vendor build".to_string(),
    }
}

/// Phase 5: Apply team instructions modifiers to all players in a team
fn apply_team_instructions_to_team(
    mut team: of_engine::Team,
    instructions: &TeamInstructions,
) -> of_engine::Team {
    use of_core::models::player::PlayerAttributes;

    // Extract modifiers from instructions
    let def_line_positioning = instructions.defensive_line.positioning_modifier();
    let def_line_pace = instructions.defensive_line.pace_requirement();

    let width_crossing = instructions.team_width.crossing_modifier();

    let tempo_stamina_drain = instructions.team_tempo.stamina_drain_modifier();

    let pressing_work_rate = instructions.pressing_intensity.work_rate_modifier();
    let pressing_stamina_cost = instructions.pressing_intensity.stamina_cost_modifier();

    let buildup_passing = instructions.build_up_style.passing_modifier();
    let buildup_long_passing = instructions.build_up_style.long_passing_modifier();

    // Apply modifiers to each player based on position
    for player in team.players.iter_mut() {
        let pos = player.position.as_str();

        // Apply defensive line modifiers (defenders)
        if is_defender(pos) {
            player.attributes.positioning =
                apply_modifier(player.attributes.positioning, def_line_positioning);
            player.attributes.pace = apply_modifier(player.attributes.pace, def_line_pace);
            player.attributes.acceleration =
                apply_modifier(player.attributes.acceleration, def_line_pace);
        }

        // Apply width modifiers (wide players)
        if is_wide_player(pos) {
            player.attributes.crossing = apply_modifier(player.attributes.crossing, width_crossing);
        }

        // Apply tempo and pressing modifiers (all outfield players)
        if pos != "GK" {
            player.attributes.work_rate =
                apply_modifier(player.attributes.work_rate, pressing_work_rate);
            // Note: stamina drain is multiplicative, applied via condition system
            // For now, we'll adjust natural_fitness as a proxy
            let stamina_modifier = (tempo_stamina_drain * pressing_stamina_cost - 1.0) * 2.0; // Scale to -4 to +4
            player.attributes.natural_fitness =
                apply_modifier_f32(player.attributes.natural_fitness, stamina_modifier as i8);
        }

        // Apply buildup modifiers (midfielders and forwards)
        if is_midfielder_or_forward(pos) {
            player.attributes.passing = apply_modifier(player.attributes.passing, buildup_passing);
            // OpenFootball doesn't have long_passing as separate attribute
            // We'll use passing + vision as proxy for long passing ability
            if buildup_long_passing != 0 {
                player.attributes.vision =
                    apply_modifier(player.attributes.vision, buildup_long_passing);
            }
        }
    }

    team
}

/// Apply integer modifier to attribute value, clamping to 1-20 range
fn apply_modifier(base_value: u8, modifier: i8) -> u8 {
    let new_value = (base_value as i16) + (modifier as i16);
    new_value.clamp(1, 20) as u8
}

/// Apply float modifier (for stamina drain calculations)
fn apply_modifier_f32(base_value: u8, modifier: i8) -> u8 {
    let new_value = (base_value as i16) + (modifier as i16);
    new_value.clamp(1, 20) as u8
}

/// Check if position is a defender
fn is_defender(pos: &str) -> bool {
    matches!(pos, "CB" | "LB" | "RB" | "LWB" | "RWB" | "DF")
}

/// Check if position is a wide player (wingers, fullbacks)
fn is_wide_player(pos: &str) -> bool {
    matches!(pos, "LW" | "RW" | "LM" | "RM" | "LWB" | "RWB" | "LB" | "RB")
}

/// Check if position is midfielder or forward
fn is_midfielder_or_forward(pos: &str) -> bool {
    matches!(
        pos,
        "CM" | "CAM" | "CDM" | "LM" | "RM" | "ST" | "CF" | "FW" | "LW" | "RW" | "MF"
    )
}

pub fn simulate_match(cfg: &MatchConfig) -> Result<MatchResult, SimError> {
    eprintln!("[of_adapter] simulate_match START");

    // 0) SquadSelector를 사용하여 최적 스쿼드 선택 (vendor_skills 기능 사용 시)
    #[cfg(feature = "vendor_skills")]
    let (home_core_team, away_core_team) = {
        use engine_bridge::select_squad_with_engine;

        let home_input = prepare_roster_input(&cfg.home);
        let away_input = prepare_roster_input(&cfg.away);

        // 홈 팀 스쿼드 선택
        let home_selection = select_squad_with_engine(&home_input, cfg.home_manager.as_ref());
        eprintln!(
            "[of_adapter] Home squad selected: {} starters, {} subs",
            home_selection.main_squad.len(),
            home_selection.substitutes.len()
        );

        // 어웨이 팀 스쿼드 선택
        let away_selection = select_squad_with_engine(&away_input, cfg.away_manager.as_ref());
        eprintln!(
            "[of_adapter] Away squad selected: {} starters, {} subs",
            away_selection.main_squad.len(),
            away_selection.substitutes.len()
        );

        // 선택된 선수 ID로 CoreTeam 재구성
        let home_selected_ids: Vec<u32> =
            home_selection.main_squad.iter().map(|mp| mp.id).collect();
        let away_selected_ids: Vec<u32> =
            away_selection.main_squad.iter().map(|mp| mp.id).collect();

        // 선수 재정렬 (선택된 순서대로)
        let reorder_players = |team: &CoreTeam, selected_ids: &[u32]| -> CoreTeam {
            let base_id = 1000u32; // core_team_to_engine_team에서 사용한 ID 기준
            let mut reordered = Vec::with_capacity(selected_ids.len());

            for &id in selected_ids {
                let idx = (id % 1000) as usize;
                if idx < team.players.len() {
                    reordered.push(team.players[idx].clone());
                }
            }

            // 선택되지 않은 선수들은 substitutes로
            let mut subs: Vec<CorePlayer> = team
                .players
                .iter()
                .enumerate()
                .filter(|(idx, _)| !selected_ids.contains(&(base_id + *idx as u32)))
                .map(|(_, p)| p.clone())
                .collect();

            // 기존 substitutes 추가
            if let Some(existing_subs) = &team.substitutes {
                subs.extend(existing_subs.clone());
            }

            CoreTeam {
                name: team.name.clone(),
                players: reordered,
                full_roster: team.full_roster.clone(),
                auto_select_squad: team.auto_select_squad,
                formation: team.formation.clone(),
                tactics: team.tactics.clone(),
                preferred_style: team.preferred_style.clone(),
                substitutes: if subs.is_empty() { None } else { Some(subs) },
                captain_name: team.captain_name.clone(),
                penalty_taker_name: team.penalty_taker_name.clone(),
                free_kick_taker_name: team.free_kick_taker_name.clone(),
                auto_select_roles: team.auto_select_roles,
                team_morale: team.team_morale,
                recent_results: team.recent_results.clone(),
            }
        };

        (
            reorder_players(&home_input, &home_selected_ids),
            reorder_players(&away_input, &away_selected_ids),
        )
    };

    #[cfg(not(feature = "vendor_skills"))]
    let (home_core_team, away_core_team) = {
        let home_input = prepare_roster_input(&cfg.home);
        let away_input = prepare_roster_input(&cfg.away);
        (
            finalize_team_without_vendor(home_input),
            finalize_team_without_vendor(away_input),
        )
    };

    // 1) 팀 변환 (condition 반영 포함)
    eprintln!("[of_adapter] Converting home team...");
    let mut home_team = convert_team(&home_core_team, cfg.auto_select_tactics)?;
    eprintln!("[of_adapter] Converting away team...");
    let mut away_team = convert_team(&away_core_team, cfg.auto_select_tactics)?;
    eprintln!("[of_adapter] Teams converted successfully");

    #[cfg(feature = "vendor_skills")]
    {
        if let Some(kickoff_tactics) = cfg.kickoff_tactics.clone() {
            eprintln!(
                "[of_adapter] Applying kickoff tactics override: {:?}",
                kickoff_tactics.tactic_type
            );
            home_team.tactics = Some(kickoff_tactics.clone());
            away_team.tactics = Some(kickoff_tactics);
        }
        if let Some(instructions) = cfg.kickoff_team_instructions.as_ref() {
            eprintln!("[of_adapter] Applying kickoff team instructions");
            home_team = apply_team_instructions_to_team(home_team, instructions);
            away_team = apply_team_instructions_to_team(away_team, instructions);
        }
    }

    #[cfg(feature = "vendor_skills")]
    if cfg.auto_select_tactics {
        if home_team.tactics.is_none() {
            let preferred_style = cfg.home.preferred_style.clone();
            let resolved_tactics = if let Some(manager) = cfg.home_manager.as_ref() {
                let coach = Coach::from(manager);
                let (tactics, _) =
                    engine_bridge::select_tactics_with_coach(&home_team, &coach, preferred_style);
                tactics
            } else {
                engine_bridge::determine_match_tactics(
                    &home_team,
                    Some(&away_team),
                    preferred_style,
                    cfg.home.formation.as_deref(),
                )
            };
            home_team.tactics = Some(resolved_tactics);
        }
        if away_team.tactics.is_none() {
            let preferred_style = cfg.away.preferred_style.clone();
            let resolved_tactics = if let Some(manager) = cfg.away_manager.as_ref() {
                let coach = Coach::from(manager);
                let (tactics, _) =
                    engine_bridge::select_tactics_with_coach(&away_team, &coach, preferred_style);
                tactics
            } else {
                engine_bridge::determine_match_tactics(
                    &away_team,
                    Some(&home_team),
                    preferred_style,
                    cfg.away.formation.as_deref(),
                )
            };
            away_team.tactics = Some(resolved_tactics);
        }
    }

    // 2) Phase 5: Apply team instructions modifiers if present
    let home_team_modified = if let Some(ref instructions) = cfg.home_instructions {
        apply_team_instructions_to_team(home_team, instructions)
    } else {
        home_team
    };

    let away_team_modified = if let Some(ref instructions) = cfg.away_instructions {
        apply_team_instructions_to_team(away_team, instructions)
    } else {
        away_team
    };

    // 3) 평균 컨디션 계산
    let home_avg_condition = if cfg.home.players.is_empty() {
        1.0
    } else {
        cfg.home.players.iter().map(|p| p.condition).sum::<f32>() / cfg.home.players.len() as f32
    };

    let away_avg_condition = if cfg.away.players.is_empty() {
        1.0
    } else {
        cfg.away.players.iter().map(|p| p.condition).sum::<f32>() / cfg.away.players.len() as f32
    };

    let avg_condition_total = (home_avg_condition + away_avg_condition) / 2.0;

    // 4) 매치 시뮬레이션 실행 - 실제 Open-Football 엔진 호출 (Phase 5: with modifiers)
    eprintln!(
        "[of_adapter] Creating MatchEngineWrapper with seed: {}",
        cfg.seed
    );
    let mut engine = of_engine::MatchEngineWrapper::new(cfg.seed);
    eprintln!("[of_adapter] Engine created");

    // 하이라이트 레벨 설정 (리퀘스트에서 받은 값 사용, 기본값: Simple)
    let highlight_level = match cfg.highlight_level.as_deref() {
        Some("skip") | Some("Skip") => of_engine::HighlightLevel::Skip,
        Some("simple") | Some("Simple") => of_engine::HighlightLevel::Simple,
        Some("my_player") | Some("MyPlayer") | Some("myplayer") => {
            let player_names = cfg
                .player_name
                .as_ref()
                .map(|name| vec![name.clone()])
                .unwrap_or_default();
            of_engine::HighlightLevel::MyPlayer(player_names)
        }
        Some("full") | Some("Full") => of_engine::HighlightLevel::Full,
        _ => of_engine::HighlightLevel::Simple, // 기본값
    };
    eprintln!(
        "[of_adapter] Setting highlight level to {:?}",
        highlight_level
    );
    engine.set_highlight_level(highlight_level);

    // 틱 간격 설정 (기본값 50ms)
    let tick_interval = cfg.tick_interval_ms.unwrap_or(50);
    eprintln!("[of_adapter] Setting tick interval to {}ms", tick_interval);
    engine.set_tick_interval(tick_interval);

    // 시뮬레이션 실행 (modifier가 적용된 팀으로)
    eprintln!("[of_adapter] Calling engine.simulate_match...");
    let engine_result = engine.simulate_match(&home_team_modified, &away_team_modified);
    eprintln!("[of_adapter] Engine simulation completed");

    #[cfg(feature = "vendor_skills")]
    let home_tactic_summary = engine_result
        .home_tactics
        .as_ref()
        .map(summarize_engine_tactics);
    #[cfg(feature = "vendor_skills")]
    let away_tactic_summary = engine_result
        .away_tactics
        .as_ref()
        .map(summarize_engine_tactics);

    // ���� �̺�Ʈ�� ���÷��� �̺�Ʈ�� ��ȯ
    let (replay, goal_heat_samples) = mapper::build_replay_doc(&engine_result);

    let position_data = if cfg.include_position_data {
        engine_result.position_data.clone()
    } else {
        None
    };
    #[cfg(feature = "vendor_skills")]
    let stored_events_payload =
        if cfg.include_stored_events && !engine_result.engine_events.is_empty() {
            Some(
                engine_result
                    .engine_events
                    .iter()
                    .map(StoredEventRecord::from)
                    .collect(),
            )
        } else {
            None
        };

    // 5) 통계 JSON 구성 - 실제 엔진 결과 사용 (Phase 5: with instructions info)
    let mut score_home = engine_result.home_goals;
    let mut score_away = engine_result.away_goals;

    if score_home + score_away == 0 && !engine_result.engine_events.is_empty() {
        let (fallback_home, fallback_away) = count_goals_from_engine_events(
            &engine_result.engine_events,
            engine_result.home_team_id,
            engine_result.away_team_id,
        );
        if fallback_home + fallback_away > 0 {
            eprintln!(
                "[of_adapter] ⚠️ Score fallback applied via engine events: {}-{}",
                fallback_home, fallback_away
            );
            score_home = fallback_home;
            score_away = fallback_away;
        }
    }

    let mut stats = serde_json::json!({
        "goals_home": score_home,
        "goals_away": score_away,
        "home_avg_condition": home_avg_condition,
        "away_avg_condition": away_avg_condition,
        "avg_condition": avg_condition_total,
        "shots_home": engine_result.statistics.home_shots,
        "shots_away": engine_result.statistics.away_shots,
        "shots_on_target_home": engine_result.statistics.home_shots_on_target,
        "shots_on_target_away": engine_result.statistics.away_shots_on_target,
        "possession_home": engine_result.statistics.home_possession,
        "possession_away": engine_result.statistics.away_possession,
        "fouls_home": engine_result.statistics.home_fouls,
        "fouls_away": engine_result.statistics.away_fouls,
        "corners_home": engine_result.statistics.home_corners,
        "corners_away": engine_result.statistics.away_corners,
        "yellow_cards_home": engine_result.statistics.home_yellow_cards,
        "yellow_cards_away": engine_result.statistics.away_yellow_cards,
        "red_cards_home": engine_result.statistics.home_red_cards,
        "red_cards_away": engine_result.statistics.away_red_cards,
        "highlight_level": format!("{:?}", engine_result.highlight_level),
        "total_events": engine_result.events.len(),
        // Phase 5: Include team instructions in stats
        "home_instructions_applied": cfg.home_instructions.is_some(),
        "away_instructions_applied": cfg.away_instructions.is_some(),
        // Phase 2: Include my_player_stats if present
        "my_player_stats": engine_result.my_player_stats.as_ref().map(|ps| {
            serde_json::json!({
                "player_id": ps.player_id,
                "player_name": ps.player_name,
                "goals": ps.goals,
                "assists": ps.assists,
                "shots": ps.shots,
                "passes": ps.passes,
                "tackles": ps.tackles,
                "fouls": ps.fouls,
                "saves": ps.saves,
                "yellow_cards": ps.yellow_cards,
                "red_cards": ps.red_cards,
            })
        }),
    });

    #[cfg(feature = "vendor_skills")]
    if let serde_json::Value::Object(ref mut obj) = stats {
        obj.insert(
            "home_tactic".into(),
            serde_json::to_value(&home_tactic_summary).unwrap_or(serde_json::Value::Null),
        );
        obj.insert(
            "away_tactic".into(),
            serde_json::to_value(&away_tactic_summary).unwrap_or(serde_json::Value::Null),
        );
    }

    Ok(MatchResult {
        score_home,
        score_away,
        replay,
        stats,
        engine_event_count: engine_result.engine_events.len(),
        #[cfg(feature = "vendor_skills")]
        home_tactic: home_tactic_summary,
        #[cfg(feature = "vendor_skills")]
        away_tactic: away_tactic_summary,
        position_data,
        goal_heat_samples,
        #[cfg(feature = "vendor_skills")]
        stored_events: stored_events_payload,
        // Extra time / penalty shootout fields (to be implemented with engine support)
        extra_time_played: false,
        penalty_shootout_played: false,
        penalty_score: None,
        card_status: engine_result.card_status.clone(),
    })
}

#[cfg(feature = "vendor_skills")]
fn count_goals_from_engine_events(
    engine_events: &[EngineEvent],
    home_team_id: u32,
    away_team_id: u32,
) -> (u32, u32) {
    use of_engine::StoredEventKind;

    let mut home = 0;
    let mut away = 0;
    for event in engine_events {
        if let StoredEventKind::Goal { team_id, .. } = &event.kind {
            if *team_id == home_team_id {
                home += 1;
            } else if *team_id == away_team_id {
                away += 1;
            }
        }
    }
    (home, away)
}

#[cfg(not(feature = "vendor_skills"))]
fn count_goals_from_engine_events(
    _engine_events: &[EngineEvent],
    _home_team_id: u32,
    _away_team_id: u32,
) -> (u32, u32) {
    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_simulate() {
        let player = CorePlayer {
            name: "Test Player".into(),
            ca: 100,
            pa: 150,
            position: "MF".into(),
            condition: 1.0,
        };
        let team = CoreTeam {
            name: "Test Team".into(),
            players: vec![player],
            full_roster: None,
            auto_select_squad: false,
            formation: None,
            #[cfg(feature = "vendor_skills")]
            tactics: None,
            #[cfg(feature = "vendor_skills")]
            preferred_style: None,
            substitutes: None,
            captain_name: None,
            penalty_taker_name: None,
            free_kick_taker_name: None,
            auto_select_roles: false,
            team_morale: None,
            recent_results: None,
        };
        let cfg = MatchConfig {
            home: team.clone(),
            away: team,
            seed: 42,
            auto_select_tactics: false,
            home_instructions: None,
            away_instructions: None,
            highlight_level: None,
            player_name: None,
            tick_interval_ms: None,
            include_position_data: true,
            include_stored_events: false,
            #[cfg(feature = "vendor_skills")]
            kickoff_tactics: None,
            kickoff_team_instructions: None,
            use_contextual_tactics: false,
            #[cfg(feature = "vendor_skills")]
            home_manager: None,
            #[cfg(feature = "vendor_skills")]
            away_manager: None,
        };
        let res = simulate_match(&cfg).unwrap();
        assert!(res.stats.is_object());
        assert!(res.stats["home_avg_condition"].is_number());
    }

    #[test]
    fn test_condition_clamping() {
        let team = CoreTeam {
            name: "Test".into(),
            players: vec![
                CorePlayer {
                    name: "P1".into(),
                    ca: 100,
                    pa: 150,
                    position: "FW".into(),
                    condition: 2.0, // over 1.0, should be clamped
                },
                CorePlayer {
                    name: "P2".into(),
                    ca: 100,
                    pa: 150,
                    position: "DF".into(),
                    condition: -0.5, // negative, should be 0.0
                },
            ],
            full_roster: None,
            auto_select_squad: false,
            formation: None,
            #[cfg(feature = "vendor_skills")]
            tactics: None,
            #[cfg(feature = "vendor_skills")]
            preferred_style: None,
            substitutes: None,
            captain_name: None,
            penalty_taker_name: None,
            free_kick_taker_name: None,
            auto_select_roles: false,
            team_morale: None,
            recent_results: None,
        };

        let converted = convert_team(&team, false).unwrap();
        // The conversion should handle clamping internally
        assert_eq!(converted.players.len(), 2);
    }

    #[test]
    fn convert_team_carries_role_metadata() {
        let starters = vec![
            CorePlayer {
                name: "Captain".into(),
                ca: 100,
                pa: 120,
                position: "CM".into(),
                condition: 1.0,
            },
            CorePlayer {
                name: "Penalty".into(),
                ca: 90,
                pa: 110,
                position: "ST".into(),
                condition: 1.0,
            },
        ];
        let substitutes = vec![CorePlayer {
            name: "Bench".into(),
            ca: 80,
            pa: 100,
            position: "RW".into(),
            condition: 1.0,
        }];

        let team = CoreTeam {
            name: "Role Test".into(),
            players: starters,
            full_roster: None,
            auto_select_squad: false,
            formation: Some("4-4-2".into()),
            #[cfg(feature = "vendor_skills")]
            tactics: None,
            #[cfg(feature = "vendor_skills")]
            preferred_style: None,
            substitutes: Some(substitutes),
            captain_name: Some("Captain".into()),
            penalty_taker_name: Some("Penalty".into()),
            free_kick_taker_name: Some("Captain".into()),
            auto_select_roles: true,
            team_morale: None,
            recent_results: None,
        };

        let converted = convert_team(&team, false).unwrap();
        assert_eq!(converted.substitutes.len(), 1);
        assert_eq!(converted.captain_name.as_deref(), Some("Captain"));
        assert_eq!(converted.penalty_taker_name.as_deref(), Some("Penalty"));
        assert!(converted.auto_select_roles);
    }

    #[test]
    fn position_data_flag_controls_payload() {
        let player = CorePlayer {
            name: "Test Player".into(),
            ca: 80,
            pa: 90,
            position: "MF".into(),
            condition: 1.0,
        };
        let team = CoreTeam {
            name: "PD Team".into(),
            players: vec![player],
            full_roster: None,
            auto_select_squad: false,
            formation: None,
            #[cfg(feature = "vendor_skills")]
            tactics: None,
            #[cfg(feature = "vendor_skills")]
            preferred_style: None,
            substitutes: None,
            captain_name: None,
            penalty_taker_name: None,
            free_kick_taker_name: None,
            auto_select_roles: false,
            team_morale: None,
            recent_results: None,
        };

        let mut cfg = MatchConfig {
            home: team.clone(),
            away: team.clone(),
            seed: 7,
            auto_select_tactics: false,
            home_instructions: None,
            away_instructions: None,
            highlight_level: None,
            player_name: None,
            tick_interval_ms: None,
            include_position_data: true,
            include_stored_events: false,
            #[cfg(feature = "vendor_skills")]
            kickoff_tactics: None,
            kickoff_team_instructions: None,
            use_contextual_tactics: false,
            #[cfg(feature = "vendor_skills")]
            home_manager: None,
            #[cfg(feature = "vendor_skills")]
            away_manager: None,
        };

        let result_no_positions = simulate_match(&cfg).unwrap();
        assert!(result_no_positions.position_data.is_none());

        cfg.include_position_data = true;
        let result_with_positions = simulate_match(&cfg).unwrap();
        if cfg!(feature = "vendor_skills") {
            assert!(result_with_positions.position_data.is_some());
        } else {
            assert!(result_with_positions.position_data.is_none());
        }
    }

    #[cfg(feature = "vendor_skills")]
    fn sample_positioned_team() -> CoreTeam {
        let positions = [
            "GK", "LB", "CB", "CB", "RB", "LM", "CM", "CM", "RM", "ST", "ST",
        ];

        CoreTeam {
            name: "Sample".into(),
            players: positions
                .iter()
                .enumerate()
                .map(|(idx, pos)| CorePlayer {
                    name: format!("P{idx}"),
                    ca: 60 + idx as u32,
                    pa: 120 + idx as u32,
                    position: pos.to_string(),
                    condition: 1.0,
                })
                .collect(),
            full_roster: None,
            auto_select_squad: false,
            formation: None,
            #[cfg(feature = "vendor_skills")]
            tactics: None,
            #[cfg(feature = "vendor_skills")]
            preferred_style: None,
            substitutes: None,
            captain_name: None,
            penalty_taker_name: None,
            free_kick_taker_name: None,
            auto_select_roles: false,
            team_morale: None,
            recent_results: None,
        }
    }

    #[cfg(feature = "vendor_skills")]
    #[test]
    fn formation_fitness_reports_tactic_code() {
        let team = sample_positioned_team();
        let result = calculate_formation_fitness(&team, "4-4-2");
        assert_eq!(result.tactic_code, "T442");
        assert!(result.fitness_score > 0.0);
    }
}
