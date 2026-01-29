use super::skill::SpecialSkill;
use super::trait_system::{TraitId, TraitSlots};
use crate::player::personality::{DecisionModifiers, PersonalityArchetype};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Player data for match simulation engine.
///
/// # Boundary Contract
/// - This is the Rust engine representation
/// - Received from GDScript via FootballRustEngine._convert_roster_for_rust()
/// - See docs/spec/03_data_schemas.md "GDScript ↔ Rust Boundary Contract"
///
/// # Related
/// - GDScript Match: scripts/core/MatchPlayer.gd (match-time entity)
/// - GDScript Save: autoload/core/GlobalCharacterData.gd (character creation)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Player {
    pub name: String,
    pub position: Position,
    pub overall: u8,

    /// Match-time condition level (FIX01 / ConditionSystem alignment)
    /// - Range: 1..=5 (TERRIBLE..EXCELLENT)
    /// - Default: 3 (AVERAGE) for backward-compat deserialization only.
    #[serde(default = "default_condition_level")]
    pub condition: u8,

    // 42 attributes will be added later, using overall for now
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<PlayerAttributes>,

    /// ⚠️ DEPRECATED: Legacy skill system (use `traits` instead)
    /// 장착된 특수 스킬 (최대 5개) - 기존 호환용
    #[serde(default)]
    pub equipped_skills: Vec<SpecialSkill>,

    /// ✅ NEW: Unified Trait System (30 traits, 4 slots, 3 tiers)
    /// 통합 특성 시스템 (4슬롯 × Bronze/Silver/Gold)
    #[serde(default)]
    pub traits: TraitSlots,

    /// Personality archetype (drives tactical decision modifiers)
    #[serde(default)]
    pub personality: PersonalityArchetype,
}

fn default_condition_level() -> u8 {
    3
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum Position {
    GK,
    LB,
    CB,
    RB,
    LWB,
    RWB,
    CDM,
    CM,
    CAM,
    LM,
    RM,
    LW,
    RW,
    CF,
    ST,
    // Generic positions
    DF,
    MF,
    FW,
}

impl Position {
    /// Decode from compact numeric codes used in binary match requests.
    /// Matches the ordering used by GDExtension MRQ0/MRB0 (0=GK ... 17=FW).
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Position::GK),
            1 => Some(Position::LB),
            2 => Some(Position::CB),
            3 => Some(Position::RB),
            4 => Some(Position::LWB),
            5 => Some(Position::RWB),
            6 => Some(Position::CDM),
            7 => Some(Position::CM),
            8 => Some(Position::CAM),
            9 => Some(Position::LM),
            10 => Some(Position::RM),
            11 => Some(Position::LW),
            12 => Some(Position::RW),
            13 => Some(Position::CF),
            14 => Some(Position::ST),
            15 => Some(Position::DF),
            16 => Some(Position::MF),
            17 => Some(Position::FW),
            _ => None,
        }
    }

    pub fn is_goalkeeper(&self) -> bool {
        matches!(self, Position::GK)
    }

    pub fn is_defender(&self) -> bool {
        matches!(
            self,
            Position::LB
                | Position::CB
                | Position::RB
                | Position::LWB
                | Position::RWB
                | Position::DF
        )
    }

    pub fn is_midfielder(&self) -> bool {
        matches!(
            self,
            Position::CDM
                | Position::CM
                | Position::CAM
                | Position::LM
                | Position::RM
                | Position::MF
        )
    }

    pub fn is_forward(&self) -> bool {
        matches!(self, Position::LW | Position::RW | Position::CF | Position::ST | Position::FW)
    }

    /// Convert specific position to generic position category
    pub fn to_generic_position(&self) -> Position {
        match self {
            Position::GK => Position::GK,
            Position::LB | Position::CB | Position::RB | Position::LWB | Position::RWB => {
                Position::DF
            }
            Position::CDM | Position::CM | Position::CAM | Position::LM | Position::RM => {
                Position::MF
            }
            Position::LW | Position::RW | Position::CF | Position::ST => Position::FW,
            // Already generic positions
            Position::DF | Position::MF | Position::FW => *self,
        }
    }

    /// Check if position change is compatible (allows for reasonable transitions)
    pub fn is_compatible_position(&self, target: Position) -> bool {
        match (self, target) {
            // Same position is always compatible
            (a, b) if *a == b => true,

            // GK conversions are very difficult
            (Position::GK, _) | (_, Position::GK) => false,

            // Within same category is generally compatible
            (a, b) if a.to_generic_position() == b.to_generic_position() => true,

            // Adjacent positions can sometimes work
            // Defenders
            (Position::CB, Position::CDM) | (Position::CDM, Position::CB) => true,
            (Position::LB, Position::LM) | (Position::LM, Position::LB) => true,
            (Position::RB, Position::RM) | (Position::RM, Position::RB) => true,
            (Position::LWB, Position::LW) | (Position::LW, Position::LWB) => true,
            (Position::RWB, Position::RW) | (Position::RW, Position::RWB) => true,

            // Midfield transitions
            (Position::CDM, Position::CM) | (Position::CM, Position::CDM) => true,
            (Position::CM, Position::CAM) | (Position::CAM, Position::CM) => true,
            (Position::CAM, Position::CF) | (Position::CF, Position::CAM) => true,
            (Position::LM, Position::LW) | (Position::LW, Position::LM) => true,
            (Position::RM, Position::RW) | (Position::RW, Position::RM) => true,

            // Forward transitions
            (Position::CF, Position::ST) | (Position::ST, Position::CF) => true,

            // Generic position transitions
            (Position::DF, Position::MF) | (Position::MF, Position::DF) => true,
            (Position::MF, Position::FW) | (Position::FW, Position::MF) => true,

            // Everything else is incompatible
            _ => false,
        }
    }

    /// Get primary attributes for this position (most important skills)
    pub fn get_primary_attributes(&self) -> Vec<&'static str> {
        match self {
            Position::GK => {
                vec!["reflexes", "handling", "aerial_ability", "command_of_area", "communication"]
            }
            Position::CB | Position::DF => {
                vec!["positioning", "anticipation", "strength", "heading", "concentration"]
            }
            Position::LB | Position::RB => {
                vec!["positioning", "speed", "stamina", "crossing", "work_rate"]
            }
            Position::LWB | Position::RWB => {
                vec!["speed", "stamina", "crossing", "positioning", "work_rate"]
            }
            Position::CDM => {
                vec!["positioning", "anticipation", "work_rate", "passing", "strength"]
            }
            Position::CM | Position::MF => {
                vec!["passing", "vision", "technique", "stamina", "teamwork"]
            }
            Position::CAM => vec!["technique", "vision", "passing", "ball_control", "composure"],
            Position::LM | Position::RM => {
                vec!["speed", "stamina", "crossing", "passing", "work_rate"]
            }
            Position::LW | Position::RW => {
                vec!["speed", "acceleration", "dribbling", "crossing", "finishing"]
            }
            Position::CF => vec!["technique", "ball_control", "vision", "finishing", "composure"],
            Position::ST | Position::FW => {
                vec!["finishing", "shooting", "speed", "positioning", "composure"]
            }
        }
    }

    /// Calculate position change cost (0.0 = free, 1.0 = impossible)
    pub fn position_change_cost(&self, target: Position) -> f32 {
        if self == &target {
            return 0.0; // Same position
        }

        if !self.is_compatible_position(target) {
            return 1.0; // Incompatible change
        }

        match (self, target) {
            // Same category changes are relatively easy
            (a, b) if a.to_generic_position() == b.to_generic_position() => 0.2,

            // Adjacent position changes
            (Position::CB, Position::CDM) | (Position::CDM, Position::CB) => 0.3,
            (Position::LB, Position::LM) | (Position::LM, Position::LB) => 0.3,
            (Position::RB, Position::RM) | (Position::RM, Position::RB) => 0.3,
            (Position::CAM, Position::CF) | (Position::CF, Position::CAM) => 0.3,

            // Wing transitions
            (Position::LWB, Position::LW) | (Position::LW, Position::LWB) => 0.4,
            (Position::RWB, Position::RW) | (Position::RW, Position::RWB) => 0.4,
            (Position::LM, Position::LW) | (Position::LW, Position::LM) => 0.2,
            (Position::RM, Position::RW) | (Position::RW, Position::RM) => 0.2,

            // Cross-category changes (harder)
            (Position::DF, Position::MF) | (Position::MF, Position::DF) => 0.5,
            (Position::MF, Position::FW) | (Position::FW, Position::MF) => 0.5,

            // Everything else that's compatible but not listed above
            _ => 0.6,
        }
    }

    /// Get all positions this player could potentially play
    pub fn get_compatible_positions(&self) -> Vec<Position> {
        let all_positions = [
            Position::GK,
            Position::LB,
            Position::CB,
            Position::RB,
            Position::LWB,
            Position::RWB,
            Position::CDM,
            Position::CM,
            Position::CAM,
            Position::LM,
            Position::RM,
            Position::LW,
            Position::RW,
            Position::CF,
            Position::ST,
            Position::DF,
            Position::MF,
            Position::FW,
        ];

        all_positions.iter().filter(|&pos| self.is_compatible_position(*pos)).copied().collect()
    }

    /// Get position display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            Position::GK => "Goalkeeper",
            Position::LB => "Left Back",
            Position::CB => "Centre Back",
            Position::RB => "Right Back",
            Position::LWB => "Left Wing Back",
            Position::RWB => "Right Wing Back",
            Position::CDM => "Defensive Midfielder",
            Position::CM => "Central Midfielder",
            Position::CAM => "Attacking Midfielder",
            Position::LM => "Left Midfielder",
            Position::RM => "Right Midfielder",
            Position::LW => "Left Winger",
            Position::RW => "Right Winger",
            Position::CF => "Centre Forward",
            Position::ST => "Striker",
            Position::DF => "Defender",
            Position::MF => "Midfielder",
            Position::FW => "Forward",
        }
    }

    /// Get position abbreviation for compact display
    pub fn abbreviation(&self) -> &'static str {
        match self {
            Position::GK => "GK",
            Position::LB => "LB",
            Position::CB => "CB",
            Position::RB => "RB",
            Position::LWB => "LWB",
            Position::RWB => "RWB",
            Position::CDM => "CDM",
            Position::CM => "CM",
            Position::CAM => "CAM",
            Position::LM => "LM",
            Position::RM => "RM",
            Position::LW => "LW",
            Position::RW => "RW",
            Position::CF => "CF",
            Position::ST => "ST",
            Position::DF => "DEF",
            Position::MF => "MID",
            Position::FW => "FWD",
        }
    }

    /// Convert position to tactical line role for advanced QA metrics.
    ///
    /// # Returns
    /// - `"GK"` for goalkeepers
    /// - `"DEF"` for defenders (LB, CB, RB, LWB, RWB, DF)
    /// - `"MID"` for midfielders (CDM, CM, CAM, LM, RM, MF)
    /// - `"FWD"` for forwards (LW, RW, CF, ST, FW)
    pub fn to_line_role(&self) -> &'static str {
        if self.is_goalkeeper() {
            "GK"
        } else if self.is_defender() {
            "DEF"
        } else if self.is_midfielder() {
            "MID"
        } else {
            "FWD"
        }
    }
}

impl FromStr for Position {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GK" => Ok(Position::GK),
            "LB" => Ok(Position::LB),
            "CB" => Ok(Position::CB),
            "RB" => Ok(Position::RB),
            "LWB" => Ok(Position::LWB),
            "RWB" => Ok(Position::RWB),
            "CDM" => Ok(Position::CDM),
            "CM" => Ok(Position::CM),
            "CAM" => Ok(Position::CAM),
            "LM" => Ok(Position::LM),
            "RM" => Ok(Position::RM),
            "LW" => Ok(Position::LW),
            "RW" => Ok(Position::RW),
            "CF" => Ok(Position::CF),
            "ST" => Ok(Position::ST),
            "DF" | "DEF" => Ok(Position::DF),
            "MF" | "MID" => Ok(Position::MF),
            "FW" | "FWD" => Ok(Position::FW),
            _ => Err(format!("Invalid position: {}", s)),
        }
    }
}

/// Open-Football original player skills structure (36 attributes)
/// Technical (14) + Mental (14) + Physical (8) + Goalkeeper (11) = 47 fields total
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerAttributes {
    // Technical attributes (14) - exact match with Open-Football original
    pub corners: u8,
    pub crossing: u8,
    pub dribbling: u8,
    pub finishing: u8,
    pub first_touch: u8,
    pub free_kicks: u8,
    pub heading: u8,
    pub long_shots: u8,
    pub long_throws: u8,
    pub marking: u8,
    pub passing: u8,
    #[serde(alias = "penalty_kicks")]
    pub penalty_taking: u8,
    pub tackling: u8,
    pub technique: u8,

    // Mental attributes (14) - exact match with Open-Football original
    pub aggression: u8,
    pub anticipation: u8,
    pub bravery: u8,
    pub composure: u8,
    pub concentration: u8,
    pub decisions: u8,
    pub determination: u8,
    pub flair: u8,
    pub leadership: u8,
    pub off_the_ball: u8,
    pub positioning: u8,
    pub teamwork: u8,
    pub vision: u8,
    pub work_rate: u8,

    // Physical attributes (8) - exact match with Open-Football original
    pub acceleration: u8,
    pub agility: u8,
    pub balance: u8,
    pub jumping: u8,
    pub natural_fitness: u8,
    pub pace: u8,
    pub stamina: u8,
    pub strength: u8,

    // Goalkeeper attributes (11) - v5 schema, FM 2023 columns 48-58
    // Non-GK players will have 0 or low values
    #[serde(default)]
    pub gk_aerial_reach: u8,
    #[serde(default)]
    pub gk_command_of_area: u8,
    #[serde(default)]
    pub gk_communication: u8,
    #[serde(default)]
    pub gk_eccentricity: u8,
    #[serde(default)]
    pub gk_handling: u8,
    #[serde(default)]
    pub gk_kicking: u8,
    #[serde(default)]
    pub gk_one_on_ones: u8,
    #[serde(default)]
    pub gk_reflexes: u8,
    #[serde(default)]
    pub gk_rushing_out: u8,
    #[serde(default)]
    pub gk_punching: u8,
    #[serde(default)]
    pub gk_throwing: u8,
}

impl Default for PlayerAttributes {
    fn default() -> Self {
        Self {
            // Technical attributes (14) - 50 as average
            corners: 50,
            crossing: 50,
            dribbling: 50,
            finishing: 50,
            first_touch: 50,
            free_kicks: 50,
            heading: 50,
            long_shots: 50,
            long_throws: 50,
            marking: 50,
            passing: 50,
            penalty_taking: 50,
            tackling: 50,
            technique: 50,

            // Mental attributes (14) - 50 as average
            aggression: 50,
            anticipation: 50,
            bravery: 50,
            composure: 50,
            concentration: 50,
            decisions: 50,
            determination: 50,
            flair: 50,
            leadership: 50,
            off_the_ball: 50,
            positioning: 50,
            teamwork: 50,
            vision: 50,
            work_rate: 50,

            // Physical attributes (8) - 50 as average
            acceleration: 50,
            agility: 50,
            balance: 50,
            jumping: 50,
            natural_fitness: 50,
            pace: 50,
            stamina: 50,
            strength: 50,

            // Goalkeeper attributes (11) - 0 for non-GK by default
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
        }
    }
}

impl PlayerAttributes {
    /// Derive a full 36-attribute set from a proxy tuple (overall, position, seed).
    /// - Deterministic for the same (overall, position, seed)
    /// - Primary attributes for the position get a positive bump
    /// - All values are clamped to 1..=100
    pub fn derive_from_proxy(overall: u8, position: Position, seed: u64) -> Self {
        // Base around overall (clamped)
        let base = overall.clamp(1, 100);
        let mut attrs = PlayerAttributes::from_uniform(base);

        // Deterministic RNG from inputs
        let mix = seed
            ^ ((overall as u64) << 32)
            ^ ((position as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut rng = StdRng::seed_from_u64(mix);

        // Bump primary attributes
        for key in position.get_primary_attributes() {
            let delta: i8 = rng.gen_range(4..=8);
            Self::bump_attr(&mut attrs, key, delta);
        }

        // Small random variance to keep profiles from being perfectly flat
        const ALL_KEYS: [&str; 36] = [
            "corners",
            "crossing",
            "dribbling",
            "finishing",
            "first_touch",
            "free_kicks",
            "heading",
            "long_shots",
            "long_throws",
            "marking",
            "passing",
            "penalty_taking",
            "tackling",
            "technique",
            "aggression",
            "anticipation",
            "bravery",
            "composure",
            "concentration",
            "decisions",
            "determination",
            "flair",
            "leadership",
            "off_the_ball",
            "positioning",
            "teamwork",
            "vision",
            "work_rate",
            "acceleration",
            "agility",
            "balance",
            "jumping",
            "natural_fitness",
            "pace",
            "stamina",
            "strength",
        ];
        for key in ALL_KEYS {
            let delta: i8 = rng.gen_range(-2..=2);
            Self::bump_attr(&mut attrs, key, delta);
        }

        // GK-specific subtle bias if needed
        if position.is_goalkeeper() {
            for key in
                ["reflexes", "handling", "aerial_ability", "command_of_area", "communication"]
            {
                Self::bump_attr(&mut attrs, key, rng.gen_range(3..=5));
            }
        }

        attrs
    }

    fn bump_attr(attrs: &mut PlayerAttributes, key: &str, delta: i8) {
        let apply = |v: &mut u8, d: i8| {
            let nv = (*v as i16 + d as i16).clamp(1, 100) as u8;
            *v = nv;
        };
        match key {
            // Technical
            "corners" => apply(&mut attrs.corners, delta),
            "crossing" => apply(&mut attrs.crossing, delta),
            "dribbling" => apply(&mut attrs.dribbling, delta),
            "finishing" => apply(&mut attrs.finishing, delta),
            "first_touch" => apply(&mut attrs.first_touch, delta),
            "free_kicks" => apply(&mut attrs.free_kicks, delta),
            "heading" => apply(&mut attrs.heading, delta),
            "long_shots" => apply(&mut attrs.long_shots, delta),
            "long_throws" => apply(&mut attrs.long_throws, delta),
            "marking" => apply(&mut attrs.marking, delta),
            "passing" => apply(&mut attrs.passing, delta),
            "penalty_taking" => apply(&mut attrs.penalty_taking, delta),
            "tackling" => apply(&mut attrs.tackling, delta),
            "technique" => apply(&mut attrs.technique, delta),

            // Mental
            "aggression" => apply(&mut attrs.aggression, delta),
            "anticipation" => apply(&mut attrs.anticipation, delta),
            "bravery" => apply(&mut attrs.bravery, delta),
            "composure" => apply(&mut attrs.composure, delta),
            "concentration" => apply(&mut attrs.concentration, delta),
            "decisions" => apply(&mut attrs.decisions, delta),
            "determination" => apply(&mut attrs.determination, delta),
            "flair" => apply(&mut attrs.flair, delta),
            "leadership" => apply(&mut attrs.leadership, delta),
            "off_the_ball" => apply(&mut attrs.off_the_ball, delta),
            "positioning" => apply(&mut attrs.positioning, delta),
            "teamwork" => apply(&mut attrs.teamwork, delta),
            "vision" => apply(&mut attrs.vision, delta),
            "work_rate" => apply(&mut attrs.work_rate, delta),

            // Physical
            "acceleration" => apply(&mut attrs.acceleration, delta),
            "agility" => apply(&mut attrs.agility, delta),
            "balance" => apply(&mut attrs.balance, delta),
            "jumping" => apply(&mut attrs.jumping, delta),
            "natural_fitness" => apply(&mut attrs.natural_fitness, delta),
            "pace" => apply(&mut attrs.pace, delta),
            "stamina" => apply(&mut attrs.stamina, delta),
            "strength" => apply(&mut attrs.strength, delta),

            // GK extras are not in PlayerAttributes (36 attrs total, no GK-specific fields)
            // If needed in future, add to struct or ignore via fallthrough
            _ => {}
        }
    }

    /// Creates attributes with a uniform value (clamped to 1..=100).
    pub fn from_uniform(val: u8) -> Self {
        // Clamp to 1..=100 to avoid degenerate 0/overflow values coming from external sources.
        let v = val.clamp(1, 100);
        Self {
            corners: v,
            crossing: v,
            dribbling: v,
            finishing: v,
            first_touch: v,
            free_kicks: v,
            heading: v,
            long_shots: v,
            long_throws: v,
            marking: v,
            passing: v,
            penalty_taking: v,
            tackling: v,
            technique: v,

            aggression: v,
            anticipation: v,
            bravery: v,
            composure: v,
            concentration: v,
            decisions: v,
            determination: v,
            flair: v,
            leadership: v,
            off_the_ball: v,
            positioning: v,
            teamwork: v,
            vision: v,
            work_rate: v,

            acceleration: v,
            agility: v,
            balance: v,
            jumping: v,
            natural_fitness: v,
            pace: v,
            stamina: v,
            strength: v,

            // GK attributes - default to 0 for from_uniform (typically outfield players)
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
        }
    }

    /// Get attribute by YAML key name (ACTION_SCORING_SSOT integration)
    pub fn get_by_key(&self, key: &str) -> Option<u8> {
        match key {
            "pace" => Some(self.pace),
            "acceleration" => Some(self.acceleration),
            "passing" => Some(self.passing),
            "vision" => Some(self.vision),
            "technique" => Some(self.technique),
            "composure" => Some(self.composure),
            "decisions" => Some(self.decisions),
            "first_touch" => Some(self.first_touch),
            "dribbling" => Some(self.dribbling),
            "finishing" => Some(self.finishing),
            "long_shots" => Some(self.long_shots),
            "tackling" => Some(self.tackling),
            "anticipation" => Some(self.anticipation),
            "positioning" => Some(self.positioning),
            "agility" => Some(self.agility),
            "balance" => Some(self.balance),
            "strength" => Some(self.strength),
            "stamina" => Some(self.stamina),
            "work_rate" => Some(self.work_rate),
            "off_the_ball" => Some(self.off_the_ball),
            "crossing" => Some(self.crossing),
            "heading" => Some(self.heading),
            "marking" => Some(self.marking),
            "teamwork" => Some(self.teamwork),
            "corners" => Some(self.corners),
            "free_kicks" => Some(self.free_kicks),
            "penalty_taking" => Some(self.penalty_taking),
            "long_throws" => Some(self.long_throws),
            "aggression" => Some(self.aggression),
            "bravery" => Some(self.bravery),
            "concentration" => Some(self.concentration),
            "determination" => Some(self.determination),
            "flair" => Some(self.flair),
            "leadership" => Some(self.leadership),
            "jumping" => Some(self.jumping),
            "natural_fitness" => Some(self.natural_fitness),
            _ => None,
        }
    }

    /// Calculate PACE from relevant attributes with weighted average (Open-Football compatible)
    pub fn calculate_pace(&self) -> u8 {
        let attributes =
            [self.pace, self.acceleration, self.agility, self.balance, self.off_the_ball];
        Self::weighted_average(&attributes, &[3, 3, 2, 1, 1])
    }

    /// Calculate POWER from relevant attributes with weighted average
    pub fn calculate_power(&self) -> u8 {
        let attributes = [
            self.strength,
            self.jumping,
            self.stamina,
            self.natural_fitness,
            self.heading,
            self.bravery,
        ];
        Self::weighted_average(&attributes, &[3, 2, 2, 1, 1, 1])
    }

    /// Calculate TECHNICAL from relevant attributes with weighted average
    pub fn calculate_technical(&self) -> u8 {
        let attributes = [self.dribbling, self.first_touch, self.technique, self.flair];
        Self::weighted_average(&attributes, &[3, 3, 3, 1])
    }

    /// Calculate SHOOTING from relevant attributes with weighted average
    pub fn calculate_shooting(&self) -> u8 {
        let attributes = [self.finishing, self.long_shots, self.composure, self.penalty_taking];
        Self::weighted_average(&attributes, &[3, 3, 1, 1])
    }

    /// Calculate PASSING from relevant attributes with weighted average
    pub fn calculate_passing(&self) -> u8 {
        let attributes = [
            self.passing,
            self.vision,
            self.crossing,
            self.teamwork,
            self.free_kicks,
            self.corners,
        ];
        Self::weighted_average(&attributes, &[3, 3, 2, 1, 1, 1])
    }

    /// Calculate DEFENDING from relevant attributes with weighted average
    pub fn calculate_defending(&self) -> u8 {
        let attributes = [
            self.positioning,
            self.anticipation,
            self.concentration,
            self.aggression,
            self.work_rate,
            self.determination,
        ];
        Self::weighted_average(&attributes, &[3, 2, 2, 1, 1, 1])
    }

    /// Helper method to calculate weighted average, capped at 20 for hexagon display
    fn weighted_average(values: &[u8], weights: &[u8]) -> u8 {
        if values.is_empty() || weights.is_empty() || values.len() != weights.len() {
            return 0;
        }

        let sum: u32 = values.iter().zip(weights.iter()).map(|(v, w)| *v as u32 * *w as u32).sum();
        let weight_sum: u32 = weights.iter().map(|w| *w as u32).sum();

        if weight_sum == 0 {
            return 0;
        }

        // Calculate the weighted average but scale it to use full 0-20 range
        // Convert from 0-100 attribute range to 0-20 hexagon range
        let average = sum / weight_sum;
        ((average as f32 * 0.2).round() as u32).min(20) as u8 // Scale 0-100 to 0-20
    }

    /// For goalkeepers, use adapted calculation with OpenFootball original attributes
    /// Since OpenFootball has no GK-specific attributes, we use base attributes
    pub fn calculate_gk_hexagon(&self) -> (u8, u8, u8, u8, u8, u8) {
        let pace = self.calculate_gk_pace();
        let power = self.calculate_gk_power();
        let technical = self.calculate_gk_handling();
        let shooting = self.calculate_gk_distribution();
        let passing = self.calculate_gk_passing();
        let defending = self.calculate_gk_defending();

        (pace, power, technical, shooting, passing, defending)
    }

    fn calculate_gk_pace(&self) -> u8 {
        // GK specific: agility and anticipation
        Self::weighted_average(&[self.agility, self.anticipation, self.pace], &[3, 2, 1])
    }

    fn calculate_gk_power(&self) -> u8 {
        // GK specific: strength, jumping (aerial dominance)
        Self::weighted_average(&[self.strength, self.jumping, self.heading], &[3, 2, 1])
    }

    fn calculate_gk_handling(&self) -> u8 {
        // GK specific: first_touch, concentration, composure (adapted from OpenFootball base attributes)
        Self::weighted_average(&[self.first_touch, self.concentration, self.composure], &[3, 2, 1])
    }

    fn calculate_gk_distribution(&self) -> u8 {
        // GK specific: long_throws and passing
        Self::weighted_average(&[self.long_throws, self.passing], &[3, 1])
    }

    fn calculate_gk_passing(&self) -> u8 {
        // GK specific: passing and vision
        Self::weighted_average(&[self.passing, self.vision], &[3, 1])
    }

    fn calculate_gk_defending(&self) -> u8 {
        // GK specific: positioning, anticipation, concentration (adapted from OpenFootball base attributes)
        Self::weighted_average(
            &[self.positioning, self.anticipation, self.concentration],
            &[3, 2, 1],
        )
    }

    /// Apply position penalty to mental + technical attributes
    ///
    /// # Position Suitability Factor
    ///
    /// - `1.0`: Natural position (Rating 15-20) - No penalty
    /// - `0.85`: Good (Rating 11-14) - 15% penalty
    /// - `0.6`: Adequate (Rating 6-10) - 40% penalty
    /// - `0.3`: Very poor (Rating 1-5) - 70% penalty
    /// - `0.0`: Cannot play (Rating 0) - 100% penalty
    ///
    /// # Affected Attributes (28 total)
    ///
    /// **Technical (14)**: corners, crossing, dribbling, finishing, first_touch,
    /// free_kicks, heading, long_shots, long_throws, marking, passing,
    /// penalty_taking, tackling, technique
    ///
    /// **Mental (14)**: aggression, anticipation, bravery, composure, concentration,
    /// decisions, determination, flair, leadership, off_the_ball, positioning,
    /// teamwork, vision, work_rate
    ///
    /// **Physical (8) - NOT AFFECTED**: pace, acceleration, agility, balance,
    /// jumping, natural_fitness, stamina, strength
    ///
    /// # Rationale
    ///
    /// Physical attributes (speed, strength) don't change based on position,
    /// but tactical understanding (positioning, decisions) and technical execution
    /// (passing, finishing) degrade when playing out of position.
    pub fn apply_position_penalty(&mut self, suitability: f32) {
        let factor = suitability.clamp(0.0, 1.0);

        // Technical (14 attributes)
        self.corners = ((self.corners as f32) * factor) as u8;
        self.crossing = ((self.crossing as f32) * factor) as u8;
        self.dribbling = ((self.dribbling as f32) * factor) as u8;
        self.finishing = ((self.finishing as f32) * factor) as u8;
        self.first_touch = ((self.first_touch as f32) * factor) as u8;
        self.free_kicks = ((self.free_kicks as f32) * factor) as u8;
        self.heading = ((self.heading as f32) * factor) as u8;
        self.long_shots = ((self.long_shots as f32) * factor) as u8;
        self.long_throws = ((self.long_throws as f32) * factor) as u8;
        self.marking = ((self.marking as f32) * factor) as u8;
        self.passing = ((self.passing as f32) * factor) as u8;
        self.penalty_taking = ((self.penalty_taking as f32) * factor) as u8;
        self.tackling = ((self.tackling as f32) * factor) as u8;
        self.technique = ((self.technique as f32) * factor) as u8;

        // Mental (14 attributes)
        self.aggression = ((self.aggression as f32) * factor) as u8;
        self.anticipation = ((self.anticipation as f32) * factor) as u8;
        self.bravery = ((self.bravery as f32) * factor) as u8;
        self.composure = ((self.composure as f32) * factor) as u8;
        self.concentration = ((self.concentration as f32) * factor) as u8;
        self.decisions = ((self.decisions as f32) * factor) as u8;
        self.determination = ((self.determination as f32) * factor) as u8;
        self.flair = ((self.flair as f32) * factor) as u8;
        self.leadership = ((self.leadership as f32) * factor) as u8;
        self.off_the_ball = ((self.off_the_ball as f32) * factor) as u8;
        self.positioning = ((self.positioning as f32) * factor) as u8;
        self.teamwork = ((self.teamwork as f32) * factor) as u8;
        self.vision = ((self.vision as f32) * factor) as u8;
        self.work_rate = ((self.work_rate as f32) * factor) as u8;

        // Physical (8 attributes) - UNCHANGED
        // pace, acceleration, agility, balance, jumping, natural_fitness, stamina, strength
        // Rationale: Physical traits don't change based on tactical position
    }
}

/// Convenience wrapper for callers that only have string positions (e.g., MRQ0).
/// Falls back to Position::FW when parsing fails, and derives deterministic attributes.
pub fn derive_attributes_from_proxy(
    overall: u8,
    position_str: &str,
    seed: u32,
) -> PlayerAttributes {
    use std::str::FromStr;

    let pos = Position::from_str(position_str).unwrap_or(Position::FW);
    PlayerAttributes::derive_from_proxy(overall, pos, seed as u64)
}

impl Player {
    /// Personality-based decision modifiers used by tactical/decision systems
    pub fn decision_modifiers(&self) -> DecisionModifiers {
        self.personality.decision_modifiers()
    }

    /// Check if player has a specific trait equipped
    pub fn has_trait(&self, id: TraitId) -> bool {
        self.traits.has_trait(id)
    }

    /// Get stat bonus from all equipped traits
    pub fn get_trait_stat_bonus(&self, stat: super::trait_system::StatType) -> f32 {
        self.traits.get_stat_bonus(stat)
    }

    /// Get action multiplier from all equipped traits
    pub fn get_trait_action_multiplier(&self, action: super::trait_system::ActionType) -> f32 {
        self.traits.get_action_multiplier(action)
    }
}

// ============================================================================//
// Tests
// ============================================================================//
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_uniform_sets_all_fields() {
        let attrs = PlayerAttributes::from_uniform(80);
        // spot check across categories
        assert_eq!(attrs.pace, 80);
        assert_eq!(attrs.finishing, 80);
        assert_eq!(attrs.anticipation, 80);
        assert_eq!(attrs.strength, 80);
    }

    #[test]
    fn from_uniform_clamps_to_bounds() {
        let low = PlayerAttributes::from_uniform(0);
        assert_eq!(low.pace, 1);
        let high = PlayerAttributes::from_uniform(120);
        assert_eq!(high.pace, 100);
    }
}
