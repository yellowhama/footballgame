//! Unified Trait System (2025-12-03)
//!
//! 30 traits with 3-tier progression (Bronze/Silver/Gold).
//! Replaces the legacy SpecialSkill (10) + SpecialAbility (84) dual system.
//!
//! Each trait provides:
//! - Passive: Stat bonuses (scales with tier)
//! - Active: Action multipliers (scales with tier)

use serde::{Deserialize, Serialize};

// ============================================================================
// Tier System
// ============================================================================

/// 3-tier progression system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TraitTier {
    #[default]
    Bronze = 1, // Base (1.0x)
    Silver = 2, // Enhanced (1.5x)
    Gold = 3,   // Legendary (2.5x)
}

impl TraitTier {
    /// Stat bonus multiplier
    pub fn stat_multiplier(&self) -> f32 {
        match self {
            TraitTier::Bronze => 1.0,
            TraitTier::Silver => 1.5,
            TraitTier::Gold => 2.5,
        }
    }

    /// Active effect multiplier
    pub fn active_multiplier(&self) -> f32 {
        match self {
            TraitTier::Bronze => 1.1,
            TraitTier::Silver => 1.3,
            TraitTier::Gold => 1.8,
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            TraitTier::Bronze => "🟤",
            TraitTier::Silver => "⚪",
            TraitTier::Gold => "🟡",
        }
    }

    /// Korean name
    pub fn name_ko(&self) -> &'static str {
        match self {
            TraitTier::Bronze => "동특",
            TraitTier::Silver => "은특",
            TraitTier::Gold => "금특",
        }
    }
}

// ============================================================================
// Trait ID (30 traits)
// ============================================================================

/// All available traits (30 total)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraitId {
    // === Shooting & Scoring (7) ===
    Sniper,    // 스나이퍼 - finishing +4, long_shots +3
    Cannon,    // 캐논 슈터 - long_shots +4, shot_power +3
    Finesse,   // 감아차기 - curve +4, finishing +3
    Poacher,   // 침투왕 - positioning +4, anticipation +3
    Panenka,   // 강심장 - composure +5, penalties +4
    LobMaster, // 로빙 슛 - finishing +3, vision +3
    Acrobat,   // 곡예사 - agility +4, balance +3

    // === Passing & Playmaking (5) ===
    Maestro,   // 마에스트로 - vision +4, passing +3
    Crosser,   // 택배 크로스 - crossing +5, curve +3
    DeadBall,  // 프리킥 마스터 - free_kicks +5, corners +4
    Metronome, // 메트로놈 - short_passing +4, composure +3
    Architect, // 건축가 - long_passing +4, vision +3

    // === Dribbling & Ball Control (6) ===
    Speedster,  // 총알탄 - pace +4, acceleration +3
    Technician, // 앵클 브레이커 - dribbling +4, agility +3
    Tank,       // 불도저 - strength +4, balance +3
    Magnet,     // 자석 터치 - first_touch +5, ball_control +4
    Showman,    // 쇼맨 - flair +5, agility +3
    Unshakable, // 탈압박 - ball_control +3, composure +3

    // === Defense & Physical (8) ===
    Vacuum,  // 진공 청소기 - tackling +4, interceptions +3
    Wall,    // 통곡의 벽 - marking +4, heading +3
    AirRaid, // 폭격기 - jumping +4, heading +4
    Engine,  // 무한 동력 - stamina +5, work_rate +4
    Reader,  // 요격기 - anticipation +5, interceptions +4
    Shadow,  // 그림자 - marking +4, agility +3
    Bully,   // 파이터 - strength +5, aggression +4
    Motor,   // 모터 - acceleration +4, dribbling +3

    // === Goalkeeper (4) ===
    Spider,      // 거미손 - diving +4, handling +3
    Sweeper,     // 스위퍼 - speed +4, reflexes +3
    Giant,       // 제공권 - jumping +4, positioning +3
    Quarterback, // 배급자 - kicking +5, throwing +4
}

impl TraitId {
    /// Get all trait IDs
    pub fn all() -> &'static [TraitId] {
        &[
            // Shooting
            TraitId::Sniper,
            TraitId::Cannon,
            TraitId::Finesse,
            TraitId::Poacher,
            TraitId::Panenka,
            TraitId::LobMaster,
            TraitId::Acrobat,
            // Passing
            TraitId::Maestro,
            TraitId::Crosser,
            TraitId::DeadBall,
            TraitId::Metronome,
            TraitId::Architect,
            // Dribbling
            TraitId::Speedster,
            TraitId::Technician,
            TraitId::Tank,
            TraitId::Magnet,
            TraitId::Showman,
            TraitId::Unshakable,
            // Defense
            TraitId::Vacuum,
            TraitId::Wall,
            TraitId::AirRaid,
            TraitId::Engine,
            TraitId::Reader,
            TraitId::Shadow,
            TraitId::Bully,
            TraitId::Motor,
            // Goalkeeper
            TraitId::Spider,
            TraitId::Sweeper,
            TraitId::Giant,
            TraitId::Quarterback,
        ]
    }

    /// Category for this trait
    pub fn category(&self) -> TraitCategory {
        match self {
            TraitId::Sniper
            | TraitId::Cannon
            | TraitId::Finesse
            | TraitId::Poacher
            | TraitId::Panenka
            | TraitId::LobMaster
            | TraitId::Acrobat => TraitCategory::Shooting,

            TraitId::Maestro
            | TraitId::Crosser
            | TraitId::DeadBall
            | TraitId::Metronome
            | TraitId::Architect => TraitCategory::Passing,

            TraitId::Speedster
            | TraitId::Technician
            | TraitId::Tank
            | TraitId::Magnet
            | TraitId::Showman
            | TraitId::Unshakable => TraitCategory::Dribbling,

            TraitId::Vacuum
            | TraitId::Wall
            | TraitId::AirRaid
            | TraitId::Engine
            | TraitId::Reader
            | TraitId::Shadow
            | TraitId::Bully
            | TraitId::Motor => TraitCategory::Defense,

            TraitId::Spider | TraitId::Sweeper | TraitId::Giant | TraitId::Quarterback => {
                TraitCategory::Goalkeeper
            }
        }
    }

    /// Icon for this trait
    pub fn icon(&self) -> &'static str {
        match self {
            // Shooting
            TraitId::Sniper => "🔫",
            TraitId::Cannon => "💣",
            TraitId::Finesse => "🎯",
            TraitId::Poacher => "👻",
            TraitId::Panenka => "🥄",
            TraitId::LobMaster => "🌈",
            TraitId::Acrobat => "🤸",
            // Passing
            TraitId::Maestro => "👁️",
            TraitId::Crosser => "📦",
            TraitId::DeadBall => "📐",
            TraitId::Metronome => "⏱️",
            TraitId::Architect => "🏗️",
            // Dribbling
            TraitId::Speedster => "⚡",
            TraitId::Technician => "🌪️",
            TraitId::Tank => "🛡️",
            TraitId::Magnet => "🧲",
            TraitId::Showman => "🎪",
            TraitId::Unshakable => "🗿",
            // Defense
            TraitId::Vacuum => "🧹",
            TraitId::Wall => "🧱",
            TraitId::AirRaid => "✈️",
            TraitId::Engine => "🔋",
            TraitId::Reader => "🧠",
            TraitId::Shadow => "👤",
            TraitId::Bully => "💪",
            TraitId::Motor => "🏃",
            // Goalkeeper
            TraitId::Spider => "🕸️",
            TraitId::Sweeper => "🧤",
            TraitId::Giant => "🗼",
            TraitId::Quarterback => "🎯",
        }
    }

    /// Korean display name
    pub fn name_ko(&self) -> &'static str {
        match self {
            // Shooting
            TraitId::Sniper => "스나이퍼",
            TraitId::Cannon => "캐논 슈터",
            TraitId::Finesse => "감아차기 장인",
            TraitId::Poacher => "침투왕",
            TraitId::Panenka => "강심장",
            TraitId::LobMaster => "로빙 슛",
            TraitId::Acrobat => "곡예사",
            // Passing
            TraitId::Maestro => "마에스트로",
            TraitId::Crosser => "택배 크로스",
            TraitId::DeadBall => "프리킥 마스터",
            TraitId::Metronome => "메트로놈",
            TraitId::Architect => "건축가",
            // Dribbling
            TraitId::Speedster => "총알탄",
            TraitId::Technician => "앵클 브레이커",
            TraitId::Tank => "불도저",
            TraitId::Magnet => "자석 터치",
            TraitId::Showman => "쇼맨",
            TraitId::Unshakable => "탈압박",
            // Defense
            TraitId::Vacuum => "진공 청소기",
            TraitId::Wall => "통곡의 벽",
            TraitId::AirRaid => "폭격기",
            TraitId::Engine => "무한 동력",
            TraitId::Reader => "요격기",
            TraitId::Shadow => "그림자",
            TraitId::Bully => "파이터",
            TraitId::Motor => "모터",
            // Goalkeeper
            TraitId::Spider => "거미손",
            TraitId::Sweeper => "스위퍼",
            TraitId::Giant => "제공권",
            TraitId::Quarterback => "배급자",
        }
    }

    /// English display name
    pub fn name_en(&self) -> &'static str {
        match self {
            TraitId::Sniper => "Sniper",
            TraitId::Cannon => "Cannon",
            TraitId::Finesse => "Finesse Shot",
            TraitId::Poacher => "Poacher",
            TraitId::Panenka => "Panenka",
            TraitId::LobMaster => "Lob Master",
            TraitId::Acrobat => "Acrobat",
            TraitId::Maestro => "Maestro",
            TraitId::Crosser => "Crosser",
            TraitId::DeadBall => "Dead Ball Specialist",
            TraitId::Metronome => "Metronome",
            TraitId::Architect => "Architect",
            TraitId::Speedster => "Speedster",
            TraitId::Technician => "Technician",
            TraitId::Tank => "Tank",
            TraitId::Magnet => "Magnet Touch",
            TraitId::Showman => "Showman",
            TraitId::Unshakable => "Unshakable",
            TraitId::Vacuum => "Vacuum",
            TraitId::Wall => "Wall",
            TraitId::AirRaid => "Air Raid",
            TraitId::Engine => "Engine",
            TraitId::Reader => "Reader",
            TraitId::Shadow => "Shadow",
            TraitId::Bully => "Bully",
            TraitId::Motor => "Motor",
            TraitId::Spider => "Spider",
            TraitId::Sweeper => "Sweeper",
            TraitId::Giant => "Giant",
            TraitId::Quarterback => "Quarterback",
        }
    }

    /// Base passive stat bonuses (before tier multiplier)
    /// Returns: Vec<(StatType, base_value)>
    pub fn get_base_passive_bonus(&self) -> Vec<(StatType, f32)> {
        match self {
            // Shooting
            TraitId::Sniper => vec![(StatType::Finishing, 4.0), (StatType::LongShots, 3.0)],
            TraitId::Cannon => vec![(StatType::LongShots, 4.0), (StatType::ShotPower, 3.0)],
            TraitId::Finesse => vec![(StatType::Curve, 4.0), (StatType::Finishing, 3.0)],
            TraitId::Poacher => vec![(StatType::Positioning, 4.0), (StatType::Anticipation, 3.0)],
            TraitId::Panenka => vec![(StatType::Composure, 5.0), (StatType::Penalties, 4.0)],
            TraitId::LobMaster => vec![(StatType::Finishing, 3.0), (StatType::Vision, 3.0)],
            TraitId::Acrobat => vec![(StatType::Agility, 4.0), (StatType::Balance, 3.0)],

            // Passing
            TraitId::Maestro => vec![(StatType::Vision, 4.0), (StatType::Passing, 3.0)],
            TraitId::Crosser => vec![(StatType::Crossing, 5.0), (StatType::Curve, 3.0)],
            TraitId::DeadBall => vec![(StatType::FreeKicks, 5.0), (StatType::Corners, 4.0)],
            TraitId::Metronome => vec![(StatType::ShortPassing, 4.0), (StatType::Composure, 3.0)],
            TraitId::Architect => vec![(StatType::LongPassing, 4.0), (StatType::Vision, 3.0)],

            // Dribbling
            TraitId::Speedster => vec![(StatType::Pace, 4.0), (StatType::Acceleration, 3.0)],
            TraitId::Technician => vec![(StatType::Dribbling, 4.0), (StatType::Agility, 3.0)],
            TraitId::Tank => vec![(StatType::Strength, 4.0), (StatType::Balance, 3.0)],
            TraitId::Magnet => vec![(StatType::FirstTouch, 5.0), (StatType::BallControl, 4.0)],
            TraitId::Showman => vec![(StatType::Flair, 5.0), (StatType::Agility, 3.0)],
            TraitId::Unshakable => vec![(StatType::BallControl, 3.0), (StatType::Composure, 3.0)],

            // Defense
            TraitId::Vacuum => vec![(StatType::Tackling, 4.0), (StatType::Interceptions, 3.0)],
            TraitId::Wall => vec![(StatType::Marking, 4.0), (StatType::Heading, 3.0)],
            TraitId::AirRaid => vec![(StatType::Jumping, 4.0), (StatType::Heading, 4.0)],
            TraitId::Engine => vec![(StatType::Stamina, 5.0), (StatType::WorkRate, 4.0)],
            TraitId::Reader => vec![(StatType::Anticipation, 5.0), (StatType::Interceptions, 4.0)],
            TraitId::Shadow => vec![(StatType::Marking, 4.0), (StatType::Agility, 3.0)],
            TraitId::Bully => vec![(StatType::Strength, 5.0), (StatType::Aggression, 4.0)],
            TraitId::Motor => vec![(StatType::Acceleration, 4.0), (StatType::Dribbling, 3.0)],

            // Goalkeeper
            TraitId::Spider => vec![(StatType::Diving, 4.0), (StatType::Handling, 3.0)],
            TraitId::Sweeper => vec![(StatType::Speed, 4.0), (StatType::Reflexes, 3.0)],
            TraitId::Giant => vec![(StatType::Jumping, 4.0), (StatType::GKPositioning, 3.0)],
            TraitId::Quarterback => vec![(StatType::Kicking, 5.0), (StatType::Throwing, 4.0)],
        }
    }

    /// Base active effect multiplier (before tier multiplier)
    pub fn get_base_active_multiplier(&self, action: ActionType) -> f32 {
        match (self, action) {
            // Shooting actions
            (TraitId::Sniper, ActionType::Finishing) => 1.3,
            (TraitId::Cannon, ActionType::LongShot) => 1.4,
            (TraitId::Finesse, ActionType::FinesseShot) => 1.5,
            (TraitId::Poacher, ActionType::Positioning) => 1.3,
            (TraitId::Panenka, ActionType::PenaltyKick) => 1.6,
            (TraitId::LobMaster, ActionType::ChipShot) => 1.5,
            (TraitId::Acrobat, ActionType::Volley) => 1.5,

            // Passing actions
            (TraitId::Maestro, ActionType::ThroughBall) => 1.4,
            (TraitId::Crosser, ActionType::Cross) => 1.4,
            (TraitId::DeadBall, ActionType::FreeKick) => 1.5,
            (TraitId::Metronome, ActionType::ShortPass) => 1.3,
            (TraitId::Architect, ActionType::LongPass) => 1.4,

            // Dribbling actions
            (TraitId::Speedster, ActionType::Sprint) => 1.3,
            (TraitId::Technician, ActionType::Dribble) => 1.5,
            (TraitId::Tank, ActionType::Shielding) => 1.5,
            (TraitId::Magnet, ActionType::FirstTouch) => 1.4,
            (TraitId::Showman, ActionType::SkillMove) => 1.5,
            (TraitId::Unshakable, ActionType::Shielding) => 1.4,

            // Defense actions
            (TraitId::Vacuum, ActionType::Tackle) => 1.5,
            (TraitId::Wall, ActionType::Block) => 1.4,
            (TraitId::AirRaid, ActionType::Header) => 1.5,
            (TraitId::Engine, ActionType::Sprint) => 1.2, // Stamina preservation
            (TraitId::Reader, ActionType::Interception) => 1.5,
            (TraitId::Shadow, ActionType::Marking) => 1.4,
            (TraitId::Bully, ActionType::Tackle) => 1.3,
            (TraitId::Motor, ActionType::Dribble) => 1.3,

            // Goalkeeper actions
            (TraitId::Spider, ActionType::Dive) => 1.5,
            (TraitId::Sweeper, ActionType::Rush) => 1.4,
            (TraitId::Giant, ActionType::Catch) => 1.4,
            (TraitId::Quarterback, ActionType::Throw) => 1.4,

            _ => 1.0,
        }
    }

    /// Special effect description (for Gold tier)
    pub fn get_gold_special_effect(&self) -> &'static str {
        match self {
            TraitId::Sniper => "구석 명중 시 100% 골",
            TraitId::Cannon => "20m 밖 유효슛 시 GK가 절대 못 잡음",
            TraitId::Finesse => "커브 슛 궤적이 황금색으로 빛남",
            TraitId::Poacher => "오프사이드 라인에서 수비 반응 완전 무력화",
            TraitId::Panenka => "1:1/PK에서 칩슛 시 GK 주저앉음 100%",
            TraitId::LobMaster => "GK 3m만 나와도 로빙슛 100% 성공",
            TraitId::Acrobat => "바이시클킥/발리 정확도 2배",
            TraitId::Maestro => "스루패스 시 수비수 역동작(Stun)",
            TraitId::Crosser => "크로스가 황금색으로 빛나며 곡선 극대화",
            TraitId::DeadBall => "프리킥 직접골 확률 2배",
            TraitId::Metronome => "원터치 패스 100% 정확도",
            TraitId::Architect => "사이드 전환 롱패스 인터셉트 불가",
            TraitId::Speedster => "치달 시 100% 속도 경합 승리",
            TraitId::Technician => "1:1 돌파 시 수비수 100% Frozen",
            TraitId::Tank => "태클 당해도 공을 지키며 비틀거림",
            TraitId::Magnet => "어떤 패스도 터치 순간 완벽 컨트롤",
            TraitId::Showman => "개인기 성공 시 관중 환호 + 사기 충전",
            TraitId::Unshakable => "2인 이상 압박에도 공을 뺏기지 않음",
            TraitId::Vacuum => "태클 성공 시 파울 확률 0%",
            TraitId::Wall => "슈팅 몸통 블로킹 확률 극대화",
            TraitId::AirRaid => "공중볼 경합 시 체공 시간 보정",
            TraitId::Engine => "70분 이후에도 능력치 저하 없음",
            TraitId::Reader => "패스 경로 근처에서 자동 인터셉트",
            TraitId::Shadow => "드리블러에게 절대 미끄러지지 않음",
            TraitId::Bully => "어깨싸움 승리 시 상대 체력 대폭 감소",
            TraitId::Motor => "드리블 중에도 최고 속도 유지",
            TraitId::Spider => "감아차기 슛 선방률 극대화",
            TraitId::Sweeper => "1:1 상황 돌진 성공률 2배",
            TraitId::Giant => "크로스/코너 상황에서 반드시 캐치",
            TraitId::Quarterback => "공 잡은 후 역습 패스 100% 정확",
        }
    }
}

// ============================================================================
// Trait Category
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraitCategory {
    Shooting,   // 슈팅 (7)
    Passing,    // 패스 (5)
    Dribbling,  // 드리블 (6)
    Defense,    // 수비 (8)
    Goalkeeper, // 골키퍼 (4)
}

impl TraitCategory {
    pub fn name_ko(&self) -> &'static str {
        match self {
            TraitCategory::Shooting => "슈팅",
            TraitCategory::Passing => "패스",
            TraitCategory::Dribbling => "드리블",
            TraitCategory::Defense => "수비",
            TraitCategory::Goalkeeper => "골키퍼",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TraitCategory::Shooting => "⚽",
            TraitCategory::Passing => "👁️",
            TraitCategory::Dribbling => "🌪️",
            TraitCategory::Defense => "🛡️",
            TraitCategory::Goalkeeper => "🧤",
        }
    }
}

// ============================================================================
// Stat Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatType {
    // Technical
    Finishing,
    LongShots,
    ShotPower,
    Curve,
    Penalties,
    Passing,
    ShortPassing,
    LongPassing,
    Crossing,
    Vision,
    Dribbling,
    BallControl,
    FirstTouch,
    Flair,
    FreeKicks,
    Corners,

    // Physical
    Pace,
    Acceleration,
    Stamina,
    Strength,
    Jumping,
    Agility,
    Balance,

    // Mental
    Composure,
    Positioning,
    Anticipation,
    Marking,
    Interceptions,
    Aggression,
    WorkRate,

    // Heading
    Heading,

    // Tackling
    Tackling,

    // Goalkeeper specific
    Diving,
    Handling,
    Reflexes,
    GKPositioning,
    Kicking,
    Throwing,
    Speed,
}

// ============================================================================
// Action Types (for active effects)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionType {
    // Shooting
    Finishing,
    LongShot,
    FinesseShot,
    ChipShot,
    Volley,
    Header,
    PenaltyKick,

    // Passing
    ShortPass,
    LongPass,
    ThroughBall,
    Cross,
    FreeKick,

    // Dribbling
    Dribble,
    Sprint,
    Shielding,
    SkillMove,
    FirstTouch,

    // Defense
    Tackle,
    Interception,
    Block,
    Marking,

    // Goalkeeper
    Dive,
    Rush,
    Catch,
    Throw,

    // Movement
    Positioning,
}

// ============================================================================
// Equipped Trait (ID + Tier)
// ============================================================================

/// A trait equipped by a player (combines ID and tier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EquippedTrait {
    pub id: TraitId,
    pub tier: TraitTier,
}

impl EquippedTrait {
    pub fn new(id: TraitId, tier: TraitTier) -> Self {
        Self { id, tier }
    }

    pub fn bronze(id: TraitId) -> Self {
        Self::new(id, TraitTier::Bronze)
    }

    pub fn silver(id: TraitId) -> Self {
        Self::new(id, TraitTier::Silver)
    }

    pub fn gold(id: TraitId) -> Self {
        Self::new(id, TraitTier::Gold)
    }

    /// Get display name with tier icon
    pub fn display_name(&self) -> String {
        format!("{} {} {}", self.tier.icon(), self.id.icon(), self.id.name_ko())
    }

    /// Get passive stat bonuses (with tier scaling)
    pub fn get_passive_bonuses(&self) -> Vec<(StatType, f32)> {
        let multiplier = self.tier.stat_multiplier();
        self.id
            .get_base_passive_bonus()
            .into_iter()
            .map(|(stat, val)| (stat, val * multiplier))
            .collect()
    }

    /// Get active effect multiplier (with tier scaling)
    pub fn get_active_multiplier(&self, action: ActionType) -> f32 {
        let base = self.id.get_base_active_multiplier(action);
        if base > 1.0 {
            // Scale the bonus portion by tier
            let bonus = (base - 1.0) * self.tier.active_multiplier();
            1.0 + bonus
        } else {
            1.0
        }
    }

    /// Check if this trait has a special effect (Gold tier only)
    pub fn has_special_effect(&self) -> bool {
        self.tier == TraitTier::Gold
    }

    /// Get special effect description (Gold tier only)
    pub fn get_special_effect(&self) -> Option<&'static str> {
        if self.tier == TraitTier::Gold {
            Some(self.id.get_gold_special_effect())
        } else {
            None
        }
    }
}

// ============================================================================
// Trait Slots (4 slots per player)
// ============================================================================

/// Player's equipped traits (max 4 slots)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TraitSlots {
    slots: [Option<EquippedTrait>; 4],
    /// Slots unlocked (1-4 based on level)
    unlocked: u8,
}

impl TraitSlots {
    pub fn new() -> Self {
        Self {
            slots: [None; 4],
            unlocked: 1, // Start with 1 slot
        }
    }

    /// Create with specific unlock level
    pub fn with_unlocked(unlocked: u8) -> Self {
        Self { slots: [None; 4], unlocked: unlocked.min(4) }
    }

    /// Get number of unlocked slots
    pub fn unlocked_count(&self) -> u8 {
        self.unlocked
    }

    /// Unlock next slot (call when player levels up)
    pub fn unlock_slot(&mut self) {
        if self.unlocked < 4 {
            self.unlocked += 1;
        }
    }

    /// Equip a trait to a specific slot
    pub fn equip(&mut self, slot: usize, trait_: EquippedTrait) -> Result<(), TraitError> {
        if slot >= 4 {
            return Err(TraitError::InvalidSlot);
        }
        if slot >= self.unlocked as usize {
            return Err(TraitError::SlotLocked);
        }
        // Check for duplicates
        if self.has_trait(trait_.id) {
            return Err(TraitError::DuplicateTrait);
        }
        self.slots[slot] = Some(trait_);
        Ok(())
    }

    /// Remove trait from slot
    pub fn unequip(&mut self, slot: usize) -> Option<EquippedTrait> {
        if slot >= 4 {
            return None;
        }
        self.slots[slot].take()
    }

    /// Get trait in slot
    pub fn get(&self, slot: usize) -> Option<&EquippedTrait> {
        self.slots.get(slot).and_then(|t| t.as_ref())
    }

    /// Check if player has a specific trait
    pub fn has_trait(&self, id: TraitId) -> bool {
        self.slots.iter().any(|slot| slot.as_ref().map(|t| t.id == id).unwrap_or(false))
    }

    /// Check if player has a specific trait at Gold tier (for special effects)
    pub fn has_gold_trait(&self, id: TraitId) -> bool {
        self.slots.iter().any(|slot| {
            slot.as_ref().map(|t| t.id == id && t.tier == TraitTier::Gold).unwrap_or(false)
        })
    }

    /// Get trait tier if equipped (for conditional effects)
    pub fn get_trait_tier(&self, id: TraitId) -> Option<TraitTier> {
        self.slots.iter().filter_map(|slot| slot.as_ref()).find(|t| t.id == id).map(|t| t.tier)
    }

    /// Get all equipped traits
    pub fn equipped(&self) -> impl Iterator<Item = &EquippedTrait> {
        self.slots.iter().filter_map(|t| t.as_ref())
    }

    /// Get total passive bonus for a stat
    pub fn get_stat_bonus(&self, stat: StatType) -> f32 {
        self.equipped()
            .flat_map(|t| t.get_passive_bonuses())
            .filter(|(s, _)| *s == stat)
            .map(|(_, v)| v)
            .sum()
    }

    /// Get combined action multiplier from all traits
    pub fn get_action_multiplier(&self, action: ActionType) -> f32 {
        self.equipped().map(|t| t.get_active_multiplier(action)).fold(1.0, |acc, m| acc * m)
    }
}

// ============================================================================
// Trait Merge System
// ============================================================================

/// Merge result
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub success: bool,
    pub result_trait: Option<EquippedTrait>,
    pub consumed: Vec<EquippedTrait>,
}

/// Merge 3 same-tier traits into higher tier
pub fn merge_traits(traits: [EquippedTrait; 3]) -> MergeResult {
    // All must be same ID and tier
    let first = traits[0];
    if !traits.iter().all(|t| t.id == first.id && t.tier == first.tier) {
        return MergeResult { success: false, result_trait: None, consumed: vec![] };
    }

    // Gold cannot be merged further
    if first.tier == TraitTier::Gold {
        return MergeResult { success: false, result_trait: None, consumed: vec![] };
    }

    // Upgrade tier
    let new_tier = match first.tier {
        TraitTier::Bronze => TraitTier::Silver,
        TraitTier::Silver => TraitTier::Gold,
        TraitTier::Gold => TraitTier::Gold, // Cannot happen
    };

    MergeResult {
        success: true,
        result_trait: Some(EquippedTrait::new(first.id, new_tier)),
        consumed: traits.to_vec(),
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitError {
    InvalidSlot,
    SlotLocked,
    DuplicateTrait,
}

impl std::fmt::Display for TraitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraitError::InvalidSlot => write!(f, "Invalid slot index"),
            TraitError::SlotLocked => write!(f, "Slot is not unlocked yet"),
            TraitError::DuplicateTrait => write!(f, "Cannot equip duplicate trait"),
        }
    }
}

impl std::error::Error for TraitError {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_multipliers() {
        assert_eq!(TraitTier::Bronze.stat_multiplier(), 1.0);
        assert_eq!(TraitTier::Silver.stat_multiplier(), 1.5);
        assert_eq!(TraitTier::Gold.stat_multiplier(), 2.5);
    }

    #[test]
    fn test_trait_passive_bonus_scaling() {
        let bronze = EquippedTrait::bronze(TraitId::Cannon);
        let gold = EquippedTrait::gold(TraitId::Cannon);

        let bronze_bonus = bronze.get_passive_bonuses();
        let gold_bonus = gold.get_passive_bonuses();

        // Cannon: long_shots +4, shot_power +3
        // Bronze: 4.0 * 1.0 = 4.0
        // Gold: 4.0 * 2.5 = 10.0
        assert_eq!(bronze_bonus[0], (StatType::LongShots, 4.0));
        assert_eq!(gold_bonus[0], (StatType::LongShots, 10.0));
    }

    #[test]
    fn test_trait_slots() {
        let mut slots = TraitSlots::new();

        // Only 1 slot unlocked initially
        assert_eq!(slots.unlocked_count(), 1);

        // Can equip to slot 0
        let cannon = EquippedTrait::bronze(TraitId::Cannon);
        assert!(slots.equip(0, cannon).is_ok());

        // Cannot equip to locked slot 1
        let sniper = EquippedTrait::bronze(TraitId::Sniper);
        assert_eq!(slots.equip(1, sniper), Err(TraitError::SlotLocked));

        // Unlock and equip
        slots.unlock_slot();
        assert!(slots.equip(1, sniper).is_ok());

        // Cannot equip duplicate
        let cannon2 = EquippedTrait::silver(TraitId::Cannon);
        slots.unlock_slot();
        assert_eq!(slots.equip(2, cannon2), Err(TraitError::DuplicateTrait));
    }

    #[test]
    fn test_merge_system() {
        let bronze_traits = [
            EquippedTrait::bronze(TraitId::Cannon),
            EquippedTrait::bronze(TraitId::Cannon),
            EquippedTrait::bronze(TraitId::Cannon),
        ];

        let result = merge_traits(bronze_traits);
        assert!(result.success);
        assert_eq!(result.result_trait.unwrap().tier, TraitTier::Silver);
    }

    #[test]
    fn test_all_traits_count() {
        assert_eq!(TraitId::all().len(), 30);
    }
}
