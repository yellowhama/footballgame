//! Team-wide Tactical Instructions System
//!
//! Provides team-level tactical settings that affect all players' behavior in matches.
//! This extends beyond individual player instructions to shape overall team strategy.

use serde::{Deserialize, Serialize};

/// Team-wide tactical instructions that affect overall team behavior
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamInstructions {
    /// Defensive line height/depth
    pub defensive_line: DefensiveLine,
    /// Team width (narrow vs wide play)
    #[serde(alias = "width")]
    pub team_width: TeamWidth,
    /// Tempo of play (speed of build-up)
    #[serde(alias = "tempo")]
    pub team_tempo: TeamTempo,
    /// Team-wide pressing intensity
    #[serde(alias = "pressing")]
    pub pressing_intensity: TeamPressing,
    /// Build-up style from defense
    #[serde(alias = "build_up_play")]
    pub build_up_style: BuildUpStyle,
    /// Use offside trap tactic
    #[serde(default)]
    pub use_offside_trap: bool,
}

impl Default for TeamInstructions {
    fn default() -> Self {
        Self {
            defensive_line: DefensiveLine::Normal,
            team_width: TeamWidth::Normal,
            team_tempo: TeamTempo::Normal,
            pressing_intensity: TeamPressing::Medium,
            build_up_style: BuildUpStyle::Mixed,
            use_offside_trap: false,
        }
    }
}

impl TeamInstructions {
    /// Create new team instructions with default values
    pub fn new() -> Self {
        Self::default()
    }

    // ========================================================================
    // Engine Wiring - Conversion Functions (P0 Patch 0)
    // ========================================================================

    /// Convert tactical pressing intensity to numeric factor (0.0 = very low, 1.0 = very high)
    ///
    /// Used by engine to adjust pressing range and defensive behavior.
    pub fn get_pressing_factor(&self) -> f32 {
        match self.pressing_intensity {
            TeamPressing::VeryLow => 0.2,
            TeamPressing::Low => 0.4,
            TeamPressing::Medium => 0.6,
            TeamPressing::High => 0.8,
            TeamPressing::VeryHigh => 1.0,
        }
    }

    /// Convert team tempo to numeric factor (0.0 = very slow, 1.0 = very fast)
    ///
    /// Used by engine to adjust softmax temperature and decision timing.
    pub fn get_tempo_factor(&self) -> f32 {
        match self.team_tempo {
            TeamTempo::VerySlow => 0.2,
            TeamTempo::Slow => 0.4,
            TeamTempo::Normal => 0.6,
            TeamTempo::Fast => 0.8,
            TeamTempo::VeryFast => 1.0,
        }
    }

    /// Convert team width to meters adjustment (-5.0 narrow, +5.0 wide)
    ///
    /// Used by engine to adjust player spacing and formation width.
    pub fn get_width_bias_m(&self) -> f32 {
        match self.team_width {
            TeamWidth::VeryNarrow => -5.0,
            TeamWidth::Narrow => -2.5,
            TeamWidth::Normal => 0.0,
            TeamWidth::Wide => 2.5,
            TeamWidth::VeryWide => 5.0,
        }
    }

    /// Get defensive line height adjustment (0.0 = very deep, 1.0 = very high)
    ///
    /// Used by engine to adjust defensive positioning and offside trap behavior.
    pub fn get_defensive_line_height(&self) -> f32 {
        match self.defensive_line {
            DefensiveLine::VeryDeep => 0.2,
            DefensiveLine::Deep => 0.4,
            DefensiveLine::Normal => 0.6,
            DefensiveLine::High => 0.8,
            DefensiveLine::VeryHigh => 1.0,
        }
    }

    // ========================================================================
    // End Engine Wiring
    // ========================================================================

    /// Create team instructions for a specific tactical style
    pub fn for_style(style: TacticalPreset) -> Self {
        match style {
            TacticalPreset::HighPressing => Self {
                defensive_line: DefensiveLine::VeryHigh,
                team_width: TeamWidth::Wide,
                team_tempo: TeamTempo::VeryFast,
                pressing_intensity: TeamPressing::VeryHigh,
                build_up_style: BuildUpStyle::Short,
                use_offside_trap: true, // 높은 라인 + 트랩
            },
            TacticalPreset::Counterattack => Self {
                defensive_line: DefensiveLine::Deep,
                team_width: TeamWidth::Narrow,
                team_tempo: TeamTempo::Fast,
                pressing_intensity: TeamPressing::Low,
                build_up_style: BuildUpStyle::Direct,
                use_offside_trap: false, // 낮은 라인은 트랩 안함
            },
            TacticalPreset::Possession => Self {
                defensive_line: DefensiveLine::High,
                team_width: TeamWidth::Wide,
                team_tempo: TeamTempo::Slow,
                pressing_intensity: TeamPressing::Medium,
                build_up_style: BuildUpStyle::Short,
                use_offside_trap: true, // 높은 라인 + 트랩
            },
            TacticalPreset::Balanced => Self::default(),
            TacticalPreset::Defensive => Self {
                defensive_line: DefensiveLine::VeryDeep,
                team_width: TeamWidth::Narrow,
                team_tempo: TeamTempo::Normal,
                pressing_intensity: TeamPressing::Low,
                build_up_style: BuildUpStyle::Direct,
                use_offside_trap: false, // 낮은 라인은 트랩 안함
            },
        }
    }

    /// Get Korean display text for this instruction set
    pub fn describe_korean(&self) -> String {
        format!(
            "수비라인: {}, 팀폭: {}, 템포: {}, 압박: {}, 빌드업: {}",
            self.defensive_line.display_name_ko(),
            self.team_width.display_name_ko(),
            self.team_tempo.display_name_ko(),
            self.pressing_intensity.display_name_ko(),
            self.build_up_style.display_name_ko()
        )
    }
}

/// Defensive line height - affects offside trap potential and space behind defense
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DefensiveLine {
    /// Very high line - constant offside trap, aggressive pressing
    VeryHigh,
    /// High line - proactive positioning, good for pressing
    High,
    /// Normal line - balanced positioning
    #[serde(alias = "Medium")]
    Normal,
    /// Deep line - conservative, more space in front
    #[serde(alias = "Low")]
    Deep,
    /// Very deep line - extreme counter-attack setup
    #[serde(alias = "VeryLow")]
    VeryDeep,
}

impl DefensiveLine {
    pub fn display_name_ko(&self) -> &'static str {
        match self {
            Self::VeryHigh => "매우 높음",
            Self::High => "높음",
            Self::Normal => "보통",
            Self::Deep => "낮음",
            Self::VeryDeep => "매우 낮음",
        }
    }

    pub fn display_name_en(&self) -> &'static str {
        match self {
            Self::VeryHigh => "Very High",
            Self::High => "High",
            Self::Normal => "Normal",
            Self::Deep => "Deep",
            Self::VeryDeep => "Very Deep",
        }
    }

    /// Get numeric value for match engine (-2 to +2)
    pub fn to_numeric(&self) -> i8 {
        match self {
            Self::VeryHigh => 2,
            Self::High => 1,
            Self::Normal => 0,
            Self::Deep => -1,
            Self::VeryDeep => -2,
        }
    }

    /// Modifier to positioning attribute (affects defensive shape)
    pub fn positioning_modifier(&self) -> i8 {
        match self {
            Self::VeryHigh => 3, // Higher line requires better positioning
            Self::High => 2,
            Self::Normal => 0,
            Self::Deep => -1,
            Self::VeryDeep => -2,
        }
    }

    /// Modifier to pace/acceleration (affects ability to recover)
    pub fn pace_requirement(&self) -> i8 {
        match self {
            Self::VeryHigh => 4, // Very high line needs fast defenders
            Self::High => 2,
            Self::Normal => 0,
            Self::Deep => -1,
            Self::VeryDeep => -2,
        }
    }
}

/// Team width - affects spacing between players and pitch coverage
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeamWidth {
    /// Very wide - maximize pitch width, stretch opposition
    VeryWide,
    /// Wide - good pitch coverage
    Wide,
    /// Normal - balanced spacing
    #[serde(alias = "Medium")]
    Normal,
    /// Narrow - compact shape, control center
    Narrow,
    /// Very narrow - extreme compactness
    VeryNarrow,
}

impl TeamWidth {
    pub fn display_name_ko(&self) -> &'static str {
        match self {
            Self::VeryWide => "매우 넓음",
            Self::Wide => "넓음",
            Self::Normal => "보통",
            Self::Narrow => "좁음",
            Self::VeryNarrow => "매우 좁음",
        }
    }

    pub fn display_name_en(&self) -> &'static str {
        match self {
            Self::VeryWide => "Very Wide",
            Self::Wide => "Wide",
            Self::Normal => "Normal",
            Self::Narrow => "Narrow",
            Self::VeryNarrow => "Very Narrow",
        }
    }

    pub fn to_numeric(&self) -> i8 {
        match self {
            Self::VeryWide => 2,
            Self::Wide => 1,
            Self::Normal => 0,
            Self::Narrow => -1,
            Self::VeryNarrow => -2,
        }
    }

    /// Modifier to crossing attribute
    pub fn crossing_modifier(&self) -> i8 {
        match self {
            Self::VeryWide => 3,
            Self::Wide => 2,
            Self::Normal => 0,
            Self::Narrow => -2,
            Self::VeryNarrow => -3,
        }
    }
}

/// Team tempo - affects speed of transitions and build-up
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeamTempo {
    /// Very fast tempo - rapid transitions
    VeryFast,
    /// Fast tempo - quick build-up
    Fast,
    /// Normal tempo - balanced
    #[serde(alias = "Medium")]
    Normal,
    /// Slow tempo - patient build-up
    Slow,
    /// Very slow tempo - possession-focused
    VerySlow,
}

impl TeamTempo {
    pub fn display_name_ko(&self) -> &'static str {
        match self {
            Self::VeryFast => "매우 빠름",
            Self::Fast => "빠름",
            Self::Normal => "보통",
            Self::Slow => "느림",
            Self::VerySlow => "매우 느림",
        }
    }

    pub fn display_name_en(&self) -> &'static str {
        match self {
            Self::VeryFast => "Very Fast",
            Self::Fast => "Fast",
            Self::Normal => "Normal",
            Self::Slow => "Slow",
            Self::VerySlow => "Very Slow",
        }
    }

    pub fn to_numeric(&self) -> i8 {
        match self {
            Self::VeryFast => 2,
            Self::Fast => 1,
            Self::Normal => 0,
            Self::Slow => -1,
            Self::VerySlow => -2,
        }
    }

    /// Modifier to work rate and stamina drain
    pub fn stamina_drain_modifier(&self) -> f32 {
        match self {
            Self::VeryFast => 1.4, // 40% more stamina drain
            Self::Fast => 1.2,     // 20% more
            Self::Normal => 1.0,
            Self::Slow => 0.9,     // 10% less
            Self::VerySlow => 0.8, // 20% less
        }
    }

    /// Modifier to passing speed/decisions
    pub fn decision_time_modifier(&self) -> i8 {
        match self {
            Self::VeryFast => -2, // Less time for decisions
            Self::Fast => -1,
            Self::Normal => 0,
            Self::Slow => 1,
            Self::VerySlow => 2, // More time for decisions
        }
    }
}

/// Team-wide pressing intensity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeamPressing {
    /// Very high pressing - constant pressure
    VeryHigh,
    /// High pressing - aggressive
    High,
    /// Medium pressing - balanced
    Medium,
    /// Low pressing - conservative
    Low,
    /// Very low pressing - minimal pressure
    VeryLow,
}

impl TeamPressing {
    pub fn display_name_ko(&self) -> &'static str {
        match self {
            Self::VeryHigh => "매우 강함",
            Self::High => "강함",
            Self::Medium => "보통",
            Self::Low => "약함",
            Self::VeryLow => "매우 약함",
        }
    }

    pub fn display_name_en(&self) -> &'static str {
        match self {
            Self::VeryHigh => "Very High",
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
            Self::VeryLow => "Very Low",
        }
    }

    pub fn to_numeric(&self) -> i8 {
        match self {
            Self::VeryHigh => 2,
            Self::High => 1,
            Self::Medium => 0,
            Self::Low => -1,
            Self::VeryLow => -2,
        }
    }

    /// Modifier to work rate and aggression
    pub fn work_rate_modifier(&self) -> i8 {
        match self {
            Self::VeryHigh => 4,
            Self::High => 2,
            Self::Medium => 0,
            Self::Low => -2,
            Self::VeryLow => -3,
        }
    }

    /// Stamina drain multiplier from pressing
    pub fn stamina_cost_modifier(&self) -> f32 {
        match self {
            Self::VeryHigh => 1.5,
            Self::High => 1.25,
            Self::Medium => 1.0,
            Self::Low => 0.85,
            Self::VeryLow => 0.7,
        }
    }
}

/// Build-up style from defense
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BuildUpStyle {
    /// Short passing from back
    #[serde(alias = "ShortPassing")]
    Short,
    /// Mixed approach
    Mixed,
    /// Direct/long balls
    #[serde(alias = "DirectPassing")]
    Direct,
}

impl BuildUpStyle {
    pub fn display_name_ko(&self) -> &'static str {
        match self {
            Self::Short => "짧은 패스",
            Self::Mixed => "혼합",
            Self::Direct => "직접",
        }
    }

    pub fn display_name_en(&self) -> &'static str {
        match self {
            Self::Short => "Short Passing",
            Self::Mixed => "Mixed",
            Self::Direct => "Direct",
        }
    }

    pub fn to_numeric(&self) -> i8 {
        match self {
            Self::Short => -1,
            Self::Mixed => 0,
            Self::Direct => 1,
        }
    }

    /// Modifier to passing vs long_shots preference
    pub fn passing_modifier(&self) -> i8 {
        match self {
            Self::Short => 3,
            Self::Mixed => 0,
            Self::Direct => -2,
        }
    }

    pub fn long_passing_modifier(&self) -> i8 {
        match self {
            Self::Short => -2,
            Self::Mixed => 0,
            Self::Direct => 3,
        }
    }
}

/// Tactical presets for quick setup
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TacticalPreset {
    /// High defensive line, intense pressing, fast tempo
    HighPressing,
    /// Deep line, direct play, counter-attacks
    Counterattack,
    /// High line, slow tempo, short passing
    Possession,
    /// Balanced approach
    Balanced,
    /// Very deep, low pressing, direct
    Defensive,
}

impl TacticalPreset {
    pub fn display_name_ko(&self) -> &'static str {
        match self {
            Self::HighPressing => "높은 압박",
            Self::Counterattack => "역습",
            Self::Possession => "점유율",
            Self::Balanced => "균형",
            Self::Defensive => "수비적",
        }
    }

    pub fn display_name_en(&self) -> &'static str {
        match self {
            Self::HighPressing => "High Pressing",
            Self::Counterattack => "Counterattack",
            Self::Possession => "Possession",
            Self::Balanced => "Balanced",
            Self::Defensive => "Defensive",
        }
    }

    pub fn description_ko(&self) -> &'static str {
        match self {
            Self::HighPressing => "매우 높은 수비 라인과 강한 압박으로 상대를 압도",
            Self::Counterattack => "낮은 수비 라인으로 수비 후 빠른 역습",
            Self::Possession => "짧은 패스와 점유율로 경기를 지배",
            Self::Balanced => "균형 잡힌 만능 전술",
            Self::Defensive => "매우 수비적인 전술로 실점 최소화",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_team_instructions() {
        let instructions = TeamInstructions::default();
        assert_eq!(instructions.defensive_line, DefensiveLine::Normal);
        assert_eq!(instructions.team_width, TeamWidth::Normal);
        assert_eq!(instructions.team_tempo, TeamTempo::Normal);
    }

    #[test]
    fn test_tactical_presets() {
        let high_pressing = TeamInstructions::for_style(TacticalPreset::HighPressing);
        assert_eq!(high_pressing.defensive_line, DefensiveLine::VeryHigh);
        assert_eq!(high_pressing.pressing_intensity, TeamPressing::VeryHigh);

        let counter = TeamInstructions::for_style(TacticalPreset::Counterattack);
        assert_eq!(counter.defensive_line, DefensiveLine::Deep);
        assert_eq!(counter.build_up_style, BuildUpStyle::Direct);
    }

    #[test]
    fn test_numeric_values() {
        assert_eq!(DefensiveLine::VeryHigh.to_numeric(), 2);
        assert_eq!(DefensiveLine::VeryDeep.to_numeric(), -2);
        assert_eq!(TeamWidth::VeryWide.to_numeric(), 2);
        assert_eq!(TeamTempo::VerySlow.to_numeric(), -2);
    }

    #[test]
    fn test_modifiers() {
        // Defensive line positioning requirement
        assert_eq!(DefensiveLine::VeryHigh.positioning_modifier(), 3);
        assert_eq!(DefensiveLine::VeryHigh.pace_requirement(), 4);

        // Pressing stamina cost
        assert_eq!(TeamPressing::VeryHigh.stamina_cost_modifier(), 1.5);
        assert_eq!(TeamPressing::Low.stamina_cost_modifier(), 0.85);

        // Tempo stamina drain
        assert_eq!(TeamTempo::VeryFast.stamina_drain_modifier(), 1.4);
        assert_eq!(TeamTempo::Slow.stamina_drain_modifier(), 0.9);
    }

    #[test]
    fn test_korean_display() {
        let instructions = TeamInstructions::for_style(TacticalPreset::HighPressing);
        let description = instructions.describe_korean();
        assert!(description.contains("매우 높음"));
        assert!(description.contains("매우 빠름"));
    }

    #[test]
    fn test_godot_format_deserialization() {
        // Test deserialization from Godot's JSON format (Phase 4)
        let godot_json = r#"{
            "tempo": "Fast",
            "pressing": "High",
            "width": "Wide",
            "build_up_play": "ShortPassing",
            "defensive_line": "High"
        }"#;

        let instructions: TeamInstructions = serde_json::from_str(godot_json).unwrap();
        assert_eq!(instructions.team_tempo, TeamTempo::Fast);
        assert_eq!(instructions.pressing_intensity, TeamPressing::High);
        assert_eq!(instructions.team_width, TeamWidth::Wide);
        assert_eq!(instructions.build_up_style, BuildUpStyle::Short);
        assert_eq!(instructions.defensive_line, DefensiveLine::High);
        assert!(!instructions.use_offside_trap); // default value
    }

    #[test]
    fn test_godot_format_with_medium_values() {
        // Test Godot's "Medium" values mapping to Rust's "Normal"
        let godot_json = r#"{
            "tempo": "Medium",
            "pressing": "Medium",
            "width": "Medium",
            "build_up_play": "Mixed",
            "defensive_line": "Medium"
        }"#;

        let instructions: TeamInstructions = serde_json::from_str(godot_json).unwrap();
        assert_eq!(instructions.team_tempo, TeamTempo::Normal);
        assert_eq!(instructions.pressing_intensity, TeamPressing::Medium);
        assert_eq!(instructions.team_width, TeamWidth::Normal);
        assert_eq!(instructions.build_up_style, BuildUpStyle::Mixed);
        assert_eq!(instructions.defensive_line, DefensiveLine::Normal);
    }

    #[test]
    fn test_godot_format_defensive_values() {
        // Test Godot's defensive line value aliases
        let godot_json = r#"{
            "tempo": "VerySlow",
            "pressing": "VeryLow",
            "width": "Narrow",
            "build_up_play": "DirectPassing",
            "defensive_line": "VeryLow"
        }"#;

        let instructions: TeamInstructions = serde_json::from_str(godot_json).unwrap();
        assert_eq!(instructions.team_tempo, TeamTempo::VerySlow);
        assert_eq!(instructions.pressing_intensity, TeamPressing::VeryLow);
        assert_eq!(instructions.team_width, TeamWidth::Narrow);
        assert_eq!(instructions.build_up_style, BuildUpStyle::Direct);
        assert_eq!(instructions.defensive_line, DefensiveLine::VeryDeep);
    }
}
