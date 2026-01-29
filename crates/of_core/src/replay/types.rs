use serde::{Deserialize, Serialize};

/// 축구장 좌표(미터) - FIFA 105x68 기준
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct MeterPos {
    pub x: f64, // 0.0..=105.0 권장
    pub y: f64, // 0.0..=68.0 권장
}

/// 필드 상의 방향/벡터 (단위: 미터 비율)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct FieldVector {
    pub x: f64,
    pub y: f64,
}

/// 공통 이벤트 메타
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventBase {
    /// 경과 시간(초)
    pub t: f64,
    /// 관련 선수 ID(없을 수 있음)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_id: Option<u32>,
    /// 팀 ID(없을 수 있음)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<u32>,
}

/// 카드 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CardType {
    Yellow,
    Red,
}

fn default_false() -> bool {
    false
}

/// 이벤트 타입 - 포괄적인 축구 경기 이벤트
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReplayEvent {
    // 기본 경기장 이벤트
    KickOff {
        base: EventBase,
    },
    Pass {
        base: EventBase,
        from: MeterPos,
        to: MeterPos,
        #[serde(skip_serializing_if = "Option::is_none")]
        receiver_id: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        distance_m: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        force: Option<f32>,
        #[serde(default = "default_false")]
        is_clearance: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        ground: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        outcome: Option<PassOutcome>,
        #[serde(skip_serializing_if = "Option::is_none")]
        passing_skill: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        vision: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        technique: Option<f32>,
        // 0108 Phase 4: Tactical metadata
        /// Intercept danger level (0.0 = safe, 1.0 = high risk)
        #[serde(skip_serializing_if = "Option::is_none")]
        danger_level: Option<f32>,
        /// Switch of play: lateral pass covering >40% of field width
        #[serde(skip_serializing_if = "Option::is_none")]
        is_switch_of_play: Option<bool>,
        /// Line-breaking pass: pass that bypasses 2+ defensive lines
        #[serde(skip_serializing_if = "Option::is_none")]
        is_line_breaking: Option<bool>,
        /// Through ball: pass into space behind defensive line
        #[serde(skip_serializing_if = "Option::is_none")]
        is_through_ball: Option<bool>,
    },
    Shot {
        base: EventBase,
        from: MeterPos,
        target: MeterPos,
        on_target: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        xg: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        shot_speed: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        long_shots_skill: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        finishing_skill: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        technique: Option<f32>,
        // Phase 3: Shot context details
        #[serde(skip_serializing_if = "Option::is_none")]
        shot_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        defender_pressure: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        angle_to_goal: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        distance_to_goal: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        composure: Option<f32>,
        // ✅ A2: Curved shot support
        #[serde(skip_serializing_if = "Option::is_none")]
        curve_factor: Option<f32>,
    },
    Run {
        base: EventBase,
        from: MeterPos,
        to: MeterPos,
        distance_m: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        speed_mps: Option<f64>,
        #[serde(default = "default_false")]
        with_ball: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        pace_skill: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        stamina: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        condition: Option<i16>,
        // Phase 3: Run tactical context
        #[serde(skip_serializing_if = "Option::is_none")]
        run_purpose: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sprint_intensity: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tactical_value: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        off_the_ball: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        work_rate: Option<f32>,
    },
    Dribble {
        base: EventBase,
        from: MeterPos,
        to: MeterPos,
        distance_m: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        touches: Option<u32>,
        // Phase 3: Dribble quality details
        #[serde(skip_serializing_if = "Option::is_none")]
        success: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        opponents_evaded: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        space_gained: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pressure_level: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dribbling_skill: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agility: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        balance: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        close_control: Option<f32>,
    },
    ThroughBall {
        base: EventBase,
        from: MeterPos,
        to: MeterPos,
        #[serde(skip_serializing_if = "Option::is_none")]
        receiver_id: Option<u32>,
        distance_m: f64,
        // Phase 3: ThroughBall tactical details
        #[serde(skip_serializing_if = "Option::is_none")]
        defense_splitting: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        offside_risk: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pass_accuracy_required: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        receiver_speed: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        vision_quality: Option<f32>,
    },
    Goal {
        base: EventBase,
        at: MeterPos,
        #[serde(skip_serializing_if = "Option::is_none")]
        assist_player_id: Option<u32>,
    },
    Foul {
        base: EventBase,
        at: MeterPos,
        // Phase 3: Foul context details
        #[serde(skip_serializing_if = "Option::is_none")]
        foul_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        severity: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        intentional: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        location_danger: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        aggression_level: Option<f32>,
    },
    FreeKick {
        base: EventBase,
        spot: MeterPos,
    },
    CornerKick {
        base: EventBase,
        spot: MeterPos,
    },
    BallMove {
        base: EventBase,
        to: MeterPos,
    },

    // 확장된 특별 이벤트
    Card {
        base: EventBase,
        card_type: CardType,
        #[serde(skip_serializing_if = "Option::is_none")]
        yellow_count: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        from_second_yellow: Option<bool>,
    },
    Substitution {
        base: EventBase,
        in_player_id: Option<u32>,
    },
    HalfTime {
        base: EventBase,
    },
    FullTime {
        base: EventBase,
    },
    Offside {
        base: EventBase,
        at: MeterPos,
    },
    Save {
        base: EventBase,
        at: MeterPos,
        #[serde(skip_serializing_if = "Option::is_none")]
        parry_to: Option<MeterPos>,
        // Phase 3: GK save quality details
        #[serde(skip_serializing_if = "Option::is_none")]
        shot_from: Option<MeterPos>,
        #[serde(skip_serializing_if = "Option::is_none")]
        shot_power: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        save_difficulty: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        shot_speed: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reflexes_skill: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        handling_skill: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        diving_skill: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        positioning_quality: Option<f32>,
    },
    Throw {
        base: EventBase,
        from: MeterPos,
        to: MeterPos,
    },
    Penalty {
        base: EventBase,
        at: MeterPos,
        scored: bool,
    },
    Communication {
        base: EventBase,
        at: MeterPos,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        target: Option<MeterPos>,
        // Phase 3: Communication context details
        #[serde(skip_serializing_if = "Option::is_none")]
        comm_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        urgency: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_required: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        effective: Option<bool>,
    },
    Header {
        base: EventBase,
        from: MeterPos,
        #[serde(skip_serializing_if = "Option::is_none")]
        direction: Option<FieldVector>,
        #[serde(skip_serializing_if = "Option::is_none")]
        heading_skill: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        jumping_reach: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<u8>,
        // Phase 3: Aerial duel context
        #[serde(skip_serializing_if = "Option::is_none")]
        win_chance: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        opponent_distance: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        aerial_challenge: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        aerial_strength: Option<f32>,
    },
    Boundary {
        base: EventBase,
        position: MeterPos,
        #[serde(skip_serializing_if = "Option::is_none")]
        last_touch_player_id: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        last_touch_team_id: Option<u32>,
    },

    // 0108: Open-Football Integration - Possession Change Tracking
    /// Possession change event for analytics and debugging
    Possession {
        base: EventBase,
        /// Position where possession changed
        at: MeterPos,
        /// How possession was gained/lost
        change_type: PossessionChangeType,
        /// Previous owner (if any)
        #[serde(skip_serializing_if = "Option::is_none")]
        prev_owner_id: Option<u32>,
        /// Previous team (0=home, 1=away)
        #[serde(skip_serializing_if = "Option::is_none")]
        prev_team_id: Option<u32>,
    },

    // 0108: Decision Intent Logging
    /// Player decision event for debugging and analysis
    Decision {
        base: EventBase,
        /// Position where decision was made
        at: MeterPos,
        /// Chosen action (e.g., "SafePass", "TakeOn", "Shoot")
        action: String,
        /// Utility score of chosen action (0.0 - 1.0)
        #[serde(skip_serializing_if = "Option::is_none")]
        utility: Option<f32>,
    },
}

/// How the possession was gained or lost (0108: Open-Football Integration)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PossessionChangeType {
    // Gaining possession
    /// Won the ball via tackle
    Tackle,
    /// Intercepted a pass
    Interception,
    /// Picked up a loose ball
    LooseBall,
    /// Received a pass successfully
    PassReceive,
    /// Goalkeeper collected the ball
    GkCollect,
    /// Won an aerial duel
    AerialWon,

    // Losing possession
    /// Lost the ball to a tackle
    Tackled,
    /// Pass was intercepted
    PassIntercepted,
    /// Ball went out of bounds
    OutOfBounds,
    /// Gave away possession (bad touch, etc.)
    Dispossessed,
    /// Shot taken (ends possession)
    ShotTaken,

    // Neutral
    /// Ball contested between players
    Contested,
    /// Kickoff/restart (team change without contest)
    Restart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PassOutcome {
    Complete,
    Intercepted,
    Out,
}
/// ���÷��� ���� ��Ʈ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayDoc {
    /// FIFA �԰�: width=105, height=68 (����)
    pub pitch_m: PitchSpec,
    /// �ð� �������� ���ĵ� �̺�Ʈ��
    pub events: Vec<ReplayEvent>,
    /// ����(��Ű�� ���̱׷��̼� ���)
    pub version: u32,
    /// ���÷��� ��Ʈ
    #[serde(default)]
    pub rosters: ReplayRosters,
    /// UI�� ����ϱ� ���� �ð� ��Ÿ��
    #[serde(default)]
    pub timeline: Vec<ReplayTimelineEntry>,
    /// 팀 전술 정보 (홈/원정)
    #[serde(default)]
    pub tactics: ReplayTeamsTactics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PitchSpec {
    pub width_m: f64,  // ���� 105.0
    pub height_m: f64, // ���� 68.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReplayRosters {
    pub home: ReplayRoster,
    pub away: ReplayRoster,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReplayRoster {
    pub name: String,
    #[serde(default)]
    pub players: Vec<ReplayPlayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReplayPlayer {
    pub id: u32,
    pub name: String,
    pub position: String,
    pub ca: u32,
    pub condition: f32,
    /// Optional appearance data for kit/character shader
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<PlayerAppearanceData>,
}

/// Player appearance data for replay visualization (kit colors, pattern)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlayerAppearanceData {
    /// Hair color type: "black", "blonde", "redhead", "other"
    pub hair_color: String,
    /// Kit primary color RGB
    pub kit_primary: [u8; 3],
    /// Kit secondary color RGB
    pub kit_secondary: [u8; 3],
    /// Kit pattern type: 0=Solid, 1=Hoops, 2=Stripes, 3=Checker, 4=Diagonal
    pub kit_pattern: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReplayTimelineEntry {
    pub t: f64,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_id: Option<u32>,
}

/// 팀 전술 정보 (포메이션/스타일)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReplayTeamsTactics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home: Option<ReplayTeamTactics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub away: Option<ReplayTeamTactics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayTeamTactics {
    /// 전술 타입 (예: "T442", "T433" 등)
    pub tactic_type: String,
    /// 전술 스타일 (Attacking/Defensive/Balanced/...)
    pub tactical_style: String,
    /// 포메이션 적합도 (0.0~1.0)
    pub formation_strength: f32,
    /// 선택 이유 (CoachPreference/TeamComposition/...)
    pub selected_reason: String,
}

/// 골 에어리어 히트샘플
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GoalHeatSample {
    /// 수비 골대 (0=홈, 1=어웨이)
    pub team_side: u32,
    /// 필드 좌표 (미터 단위)
    pub x: f64,
    pub y: f64,
    /// 가중치 (xG 기반)
    pub weight: f64,
    /// 이벤트 종류 (shot/goal 등)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum ReplayError {
    #[error("invalid json: {0}")]
    InvalidJson(String),
    #[error("schema/validation error: {0}")]
    Validation(String),
    #[error("io error: {0}")]
    Io(String),
}

impl From<serde_json::Error> for ReplayError {
    fn from(e: serde_json::Error) -> Self {
        ReplayError::InvalidJson(e.to_string())
    }
}

// Helper method for extracting base from any event
impl ReplayEvent {
    pub fn base(&self) -> &EventBase {
        match self {
            ReplayEvent::KickOff { base }
            | ReplayEvent::Pass { base, .. }
            | ReplayEvent::Shot { base, .. }
            | ReplayEvent::Run { base, .. }
            | ReplayEvent::Dribble { base, .. }
            | ReplayEvent::ThroughBall { base, .. }
            | ReplayEvent::Goal { base, .. }
            | ReplayEvent::Foul { base, .. }
            | ReplayEvent::FreeKick { base, .. }
            | ReplayEvent::CornerKick { base, .. }
            | ReplayEvent::BallMove { base, .. }
            | ReplayEvent::Card { base, .. }
            | ReplayEvent::Substitution { base, .. }
            | ReplayEvent::HalfTime { base }
            | ReplayEvent::FullTime { base }
            | ReplayEvent::Offside { base, .. }
            | ReplayEvent::Save { base, .. }
            | ReplayEvent::Throw { base, .. }
            | ReplayEvent::Penalty { base, .. }
            | ReplayEvent::Communication { base, .. }
            | ReplayEvent::Header { base, .. }
            | ReplayEvent::Boundary { base, .. }
            | ReplayEvent::Possession { base, .. }
            | ReplayEvent::Decision { base, .. } => base,
        }
    }

    // ==========================================
    // Test helper constructors
    // ==========================================

    /// Create a simple Pass event for testing
    #[cfg(test)]
    pub fn test_pass(base: EventBase, from: MeterPos, to: MeterPos) -> Self {
        ReplayEvent::Pass {
            base,
            from,
            to,
            receiver_id: None,
            distance_m: None,
            force: None,
            is_clearance: false,
            ground: None,
            outcome: None,
            passing_skill: None,
            vision: None,
            technique: None,
            // 0108 Phase 4: Tactical metadata
            danger_level: None,
            is_switch_of_play: None,
            is_line_breaking: None,
            is_through_ball: None,
        }
    }

    /// Create a simple Goal event for testing
    #[cfg(test)]
    pub fn test_goal(base: EventBase, at: MeterPos) -> Self {
        ReplayEvent::Goal { base, at, assist_player_id: None }
    }

    /// Create a simple Shot event for testing
    #[cfg(test)]
    pub fn test_shot(base: EventBase, from: MeterPos, target: MeterPos) -> Self {
        ReplayEvent::Shot {
            base,
            from,
            target,
            on_target: true,
            xg: None,
            shot_speed: None,
            long_shots_skill: None,
            finishing_skill: None,
            technique: None,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            curve_factor: None,
        }
    }

    /// Create a simple Foul event for testing
    #[cfg(test)]
    pub fn test_foul(base: EventBase, at: MeterPos) -> Self {
        ReplayEvent::Foul {
            base,
            at,
            foul_type: None,
            severity: None,
            intentional: None,
            location_danger: None,
            aggression_level: None,
        }
    }

    /// Create a simple Card event for testing
    #[cfg(test)]
    pub fn test_card(base: EventBase, card_type: CardType) -> Self {
        ReplayEvent::Card { base, card_type, yellow_count: None, from_second_yellow: None }
    }

    /// Create a simple Run event for testing
    #[cfg(test)]
    pub fn test_run(base: EventBase, from: MeterPos, to: MeterPos) -> Self {
        ReplayEvent::Run {
            base,
            from,
            to,
            distance_m: 10.0,
            speed_mps: None,
            with_ball: false,
            pace_skill: None,
            stamina: None,
            condition: None,
            run_purpose: None,
            sprint_intensity: None,
            tactical_value: None,
            off_the_ball: None,
            work_rate: None,
        }
    }

    /// Create a simple Dribble event for testing
    #[cfg(test)]
    pub fn test_dribble(base: EventBase, from: MeterPos, to: MeterPos) -> Self {
        ReplayEvent::Dribble {
            base,
            from,
            to,
            distance_m: 5.0,
            touches: None,
            success: None,
            opponents_evaded: None,
            space_gained: None,
            pressure_level: None,
            dribbling_skill: None,
            agility: None,
            balance: None,
            close_control: None,
        }
    }
}
