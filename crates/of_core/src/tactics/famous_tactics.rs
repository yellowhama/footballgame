//! Famous Tactical Presets
//!
//! Pre-defined tactical setups based on famous football tactics and styles.
//! These provide easy-to-use presets for users who want to quickly apply
//! well-known tactical approaches without manual configuration.

use crate::tactics::team_instructions::*;

/// Famous tactical preset definition
///
/// Note: Named `FamousTactics` to avoid conflict with existing `TacticalPreset` enum
#[derive(Debug, Clone, PartialEq)]
pub struct FamousTactics {
    /// English name
    pub name: &'static str,
    /// Korean name
    pub name_ko: &'static str,
    /// English description
    pub description: &'static str,
    /// Korean description
    pub description_ko: &'static str,
    /// Tactical settings
    pub instructions: TeamInstructions,
    /// Tactical style category
    pub style: TacticalStyle,
}

/// Tactical style classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalStyle {
    /// Very attacking style
    VeryAttacking,
    /// Attacking style
    Attacking,
    /// Balanced style
    Balanced,
    /// Defensive style
    Defensive,
    /// Very defensive style
    VeryDefensive,
}

impl TacticalStyle {
    pub fn display_name_en(&self) -> &'static str {
        match self {
            Self::VeryAttacking => "Very Attacking",
            Self::Attacking => "Attacking",
            Self::Balanced => "Balanced",
            Self::Defensive => "Defensive",
            Self::VeryDefensive => "Very Defensive",
        }
    }

    pub fn display_name_ko(&self) -> &'static str {
        match self {
            Self::VeryAttacking => "매우 공격적",
            Self::Attacking => "공격적",
            Self::Balanced => "균형",
            Self::Defensive => "수비적",
            Self::VeryDefensive => "매우 수비적",
        }
    }
}

// ============================================================================
// Famous Tactics Presets
// ============================================================================

/// Tiki-Taka: Possession-based football with wide spacing
pub const TIKI_TAKA: FamousTactics = FamousTactics {
    name: "Tiki-Taka",
    name_ko: "티키타카",
    description: "Possession-based football with wide spacing and patient build-up",
    description_ko: "넓은 간격과 인내심 있는 빌드업으로 볼 점유율 중심 축구",
    instructions: TeamInstructions {
        team_tempo: TeamTempo::Normal,
        pressing_intensity: TeamPressing::Medium,
        team_width: TeamWidth::VeryWide,
        build_up_style: BuildUpStyle::Short,
        defensive_line: DefensiveLine::Normal,
        use_offside_trap: false,
    },
    style: TacticalStyle::Balanced,
};

/// Gegenpressing: High-intensity pressing after losing possession
pub const GEGENPRESSING: FamousTactics = FamousTactics {
    name: "Gegenpressing",
    name_ko: "게겐프레싱",
    description: "Immediate high pressing after losing the ball, fast transitions",
    description_ko: "볼을 잃은 즉시 강력한 압박, 빠른 전환 공격",
    instructions: TeamInstructions {
        team_tempo: TeamTempo::Fast,
        pressing_intensity: TeamPressing::VeryHigh,
        team_width: TeamWidth::Normal,
        build_up_style: BuildUpStyle::Direct,
        defensive_line: DefensiveLine::VeryHigh,
        use_offside_trap: true,
    },
    style: TacticalStyle::VeryAttacking,
};

/// Catenaccio: Ultra-defensive Italian style
pub const CATENACCIO: FamousTactics = FamousTactics {
    name: "Catenaccio",
    name_ko: "카테나치오",
    description: "Ultra-defensive Italian style, minimize conceding goals",
    description_ko: "극단적 수비 중심 이탈리아 스타일, 실점 최소화",
    instructions: TeamInstructions {
        team_tempo: TeamTempo::VerySlow,
        pressing_intensity: TeamPressing::VeryLow,
        team_width: TeamWidth::Narrow,
        build_up_style: BuildUpStyle::Short,
        defensive_line: DefensiveLine::VeryDeep,
        use_offside_trap: false,
    },
    style: TacticalStyle::VeryDefensive,
};

/// Total Football: All-out attack with maximum pressing
pub const TOTAL_FOOTBALL: FamousTactics = FamousTactics {
    name: "Total Football",
    name_ko: "토탈 풋볼",
    description: "All-out attacking football with maximum intensity",
    description_ko: "전방위 압박과 공격, 최대 강도",
    instructions: TeamInstructions {
        team_tempo: TeamTempo::VeryFast,
        pressing_intensity: TeamPressing::VeryHigh,
        team_width: TeamWidth::VeryWide,
        build_up_style: BuildUpStyle::Direct,
        defensive_line: DefensiveLine::VeryHigh,
        use_offside_trap: true,
    },
    style: TacticalStyle::VeryAttacking,
};

/// Counter-Attacking Football: Defend deep, counter quickly
pub const COUNTER_ATTACK: FamousTactics = FamousTactics {
    name: "Counter-Attack",
    name_ko: "역습 축구",
    description: "Defend deep with low pressing, counter-attack quickly with direct play",
    description_ko: "깊은 수비 후 빠른 역습, 직접적인 플레이",
    instructions: TeamInstructions {
        team_tempo: TeamTempo::Fast,
        pressing_intensity: TeamPressing::Low,
        team_width: TeamWidth::Normal,
        build_up_style: BuildUpStyle::Direct,
        defensive_line: DefensiveLine::Deep,
        use_offside_trap: false,
    },
    style: TacticalStyle::Defensive,
};

/// Park the Bus: Extreme defensive approach
pub const PARK_THE_BUS: FamousTactics = FamousTactics {
    name: "Park the Bus",
    name_ko: "버스 주차",
    description: "Extreme defensive setup, park everyone in the box",
    description_ko: "극도의 수비, 골 앞 초밀집",
    instructions: TeamInstructions {
        team_tempo: TeamTempo::VerySlow,
        pressing_intensity: TeamPressing::VeryLow,
        team_width: TeamWidth::VeryNarrow,
        build_up_style: BuildUpStyle::Short,
        defensive_line: DefensiveLine::VeryDeep,
        use_offside_trap: false,
    },
    style: TacticalStyle::VeryDefensive,
};

/// All famous tactical presets
pub const FAMOUS_TACTICS: &[FamousTactics] =
    &[TIKI_TAKA, GEGENPRESSING, CATENACCIO, TOTAL_FOOTBALL, COUNTER_ATTACK, PARK_THE_BUS];

// ============================================================================
// Helper Functions
// ============================================================================

impl FamousTactics {
    /// Find a preset by name (English or Korean)
    pub fn find_by_name(name: &str) -> Option<&'static FamousTactics> {
        FAMOUS_TACTICS.iter().find(|p| p.name == name || p.name_ko == name)
    }

    /// Filter presets by tactical style
    pub fn filter_by_style(style: TacticalStyle) -> Vec<&'static FamousTactics> {
        FAMOUS_TACTICS.iter().filter(|p| p.style == style).collect()
    }

    /// Get all presets as a slice
    pub fn all() -> &'static [FamousTactics] {
        FAMOUS_TACTICS
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_count() {
        assert_eq!(FAMOUS_TACTICS.len(), 6);
    }

    #[test]
    fn test_tiki_taka_settings() {
        assert_eq!(TIKI_TAKA.name, "Tiki-Taka");
        assert_eq!(TIKI_TAKA.name_ko, "티키타카");
        assert_eq!(TIKI_TAKA.instructions.team_tempo, TeamTempo::Normal);
        assert_eq!(TIKI_TAKA.instructions.team_width, TeamWidth::VeryWide);
        assert_eq!(TIKI_TAKA.instructions.build_up_style, BuildUpStyle::Short);
        assert_eq!(TIKI_TAKA.style, TacticalStyle::Balanced);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // Testing constant struct data
    fn test_gegenpressing_high_intensity() {
        assert_eq!(GEGENPRESSING.name, "Gegenpressing");
        assert_eq!(GEGENPRESSING.instructions.pressing_intensity, TeamPressing::VeryHigh);
        assert_eq!(GEGENPRESSING.instructions.team_tempo, TeamTempo::Fast);
        assert_eq!(GEGENPRESSING.instructions.defensive_line, DefensiveLine::VeryHigh);
        assert_eq!(GEGENPRESSING.style, TacticalStyle::VeryAttacking);
        assert!(GEGENPRESSING.instructions.use_offside_trap);
    }

    #[test]
    fn test_catenaccio_ultra_defensive() {
        assert_eq!(CATENACCIO.name_ko, "카테나치오");
        assert_eq!(CATENACCIO.instructions.team_tempo, TeamTempo::VerySlow);
        assert_eq!(CATENACCIO.instructions.pressing_intensity, TeamPressing::VeryLow);
        assert_eq!(CATENACCIO.instructions.defensive_line, DefensiveLine::VeryDeep);
        assert_eq!(CATENACCIO.style, TacticalStyle::VeryDefensive);
    }

    #[test]
    fn test_total_football_extreme_attack() {
        assert_eq!(TOTAL_FOOTBALL.instructions.team_tempo, TeamTempo::VeryFast);
        assert_eq!(TOTAL_FOOTBALL.instructions.pressing_intensity, TeamPressing::VeryHigh);
        assert_eq!(TOTAL_FOOTBALL.instructions.team_width, TeamWidth::VeryWide);
        assert_eq!(TOTAL_FOOTBALL.style, TacticalStyle::VeryAttacking);
    }

    #[test]
    fn test_counter_attack_settings() {
        assert_eq!(COUNTER_ATTACK.instructions.team_tempo, TeamTempo::Fast);
        assert_eq!(COUNTER_ATTACK.instructions.pressing_intensity, TeamPressing::Low);
        assert_eq!(COUNTER_ATTACK.instructions.build_up_style, BuildUpStyle::Direct);
        assert_eq!(COUNTER_ATTACK.style, TacticalStyle::Defensive);
    }

    #[test]
    fn test_park_the_bus_extreme_defensive() {
        assert_eq!(PARK_THE_BUS.instructions.team_tempo, TeamTempo::VerySlow);
        assert_eq!(PARK_THE_BUS.instructions.team_width, TeamWidth::VeryNarrow);
        assert_eq!(PARK_THE_BUS.instructions.defensive_line, DefensiveLine::VeryDeep);
        assert_eq!(PARK_THE_BUS.style, TacticalStyle::VeryDefensive);
    }

    #[test]
    fn test_find_by_name_english() {
        assert!(FamousTactics::find_by_name("Tiki-Taka").is_some());
        assert!(FamousTactics::find_by_name("Gegenpressing").is_some());
        assert!(FamousTactics::find_by_name("Unknown").is_none());
    }

    #[test]
    fn test_find_by_name_korean() {
        assert!(FamousTactics::find_by_name("티키타카").is_some());
        assert!(FamousTactics::find_by_name("게겐프레싱").is_some());
        assert!(FamousTactics::find_by_name("카테나치오").is_some());
    }

    #[test]
    fn test_filter_by_style() {
        let very_attacking = FamousTactics::filter_by_style(TacticalStyle::VeryAttacking);
        assert_eq!(very_attacking.len(), 2); // Gegenpressing, Total Football

        let balanced = FamousTactics::filter_by_style(TacticalStyle::Balanced);
        assert_eq!(balanced.len(), 1); // Tiki-Taka

        let very_defensive = FamousTactics::filter_by_style(TacticalStyle::VeryDefensive);
        assert_eq!(very_defensive.len(), 2); // Catenaccio, Park the Bus
    }

    #[test]
    fn test_all_presets_unique_names() {
        let names: Vec<_> = FAMOUS_TACTICS.iter().map(|p| p.name).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_presets_have_korean_names() {
        for preset in FAMOUS_TACTICS {
            assert!(!preset.name_ko.is_empty());
            assert!(!preset.description_ko.is_empty());
        }
    }

    #[test]
    fn test_stamina_impact() {
        // Total Football should have highest stamina drain
        let total_football_tempo = TOTAL_FOOTBALL.instructions.team_tempo.stamina_drain_modifier();
        let total_football_pressing =
            TOTAL_FOOTBALL.instructions.pressing_intensity.stamina_cost_modifier();
        let total_football_total = total_football_tempo * total_football_pressing;

        // Park the Bus should have lowest stamina drain
        let park_tempo = PARK_THE_BUS.instructions.team_tempo.stamina_drain_modifier();
        let park_pressing = PARK_THE_BUS.instructions.pressing_intensity.stamina_cost_modifier();
        let park_total = park_tempo * park_pressing;

        assert!(total_football_total > park_total);
        // Total Football: 1.4 * 1.5 = 2.1
        // Park the Bus: 0.8 * 0.7 = 0.56
    }
}
