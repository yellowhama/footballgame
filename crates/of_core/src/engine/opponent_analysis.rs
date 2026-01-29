//! Opponent analysis and counter-tactics system
//!
//! Analyzes opponent tactics to find weaknesses and recommend counter-tactics.

use crate::tactics::{
    BuildUpStyle, DefensiveLine, TeamInstructions, TeamPressing, TeamTempo, TeamWidth,
};

/// Identified weakness in opponent tactics
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weakness {
    /// High defensive line vulnerable to through balls
    HighLine,
    /// Narrow width vulnerable to wide attacks and crosses
    NarrowWidth,
    /// Low pressing allows easy build-up
    LowPressing,
    /// Slow tempo vulnerable to quick counters
    SlowTempo,
    /// Wide formation vulnerable to central attacks
    WideFormation,
    /// Deep line creates space in midfield
    DeepLine,
    /// High pressing leaves space behind
    HighPressing,
}

/// Recommended counter-tactic
#[derive(Clone, Debug)]
pub struct CounterTactic {
    pub recommended_style: BuildUpStyle,
    pub recommended_tempo: TeamTempo,
    pub key_areas: Vec<String>,
    /// Localization key list; client translates
    pub description_keys: Vec<String>,
}

/// Result of opponent analysis
#[derive(Clone, Debug)]
pub struct OpponentAnalysis {
    pub weaknesses: Vec<Weakness>,
    pub recommended_counter: CounterTactic,
    pub confidence: f32,
}

impl OpponentAnalysis {
    /// Analyze opponent tactics and identify weaknesses
    pub fn analyze(opponent_instructions: &TeamInstructions) -> Self {
        let mut weaknesses = Vec::new();

        // High defensive line weakness
        match opponent_instructions.defensive_line {
            DefensiveLine::VeryHigh | DefensiveLine::High => {
                weaknesses.push(Weakness::HighLine);
            }
            DefensiveLine::VeryDeep | DefensiveLine::Deep => {
                weaknesses.push(Weakness::DeepLine);
            }
            _ => {}
        }

        // Width weakness
        match opponent_instructions.team_width {
            TeamWidth::Narrow | TeamWidth::VeryNarrow => {
                weaknesses.push(Weakness::NarrowWidth);
            }
            TeamWidth::Wide | TeamWidth::VeryWide => {
                weaknesses.push(Weakness::WideFormation);
            }
            _ => {}
        }

        // Pressing weakness
        match opponent_instructions.pressing_intensity {
            TeamPressing::Low | TeamPressing::VeryLow => {
                weaknesses.push(Weakness::LowPressing);
            }
            TeamPressing::High | TeamPressing::VeryHigh => {
                weaknesses.push(Weakness::HighPressing);
            }
            _ => {}
        }

        // Tempo weakness
        match opponent_instructions.team_tempo {
            TeamTempo::Slow | TeamTempo::VerySlow => {
                weaknesses.push(Weakness::SlowTempo);
            }
            _ => {}
        }

        let recommended_counter = Self::recommend_counter(&weaknesses);
        let confidence = Self::calculate_confidence(&weaknesses);

        Self { weaknesses, recommended_counter, confidence }
    }

    /// Recommend counter-tactics based on identified weaknesses
    fn recommend_counter(weaknesses: &[Weakness]) -> CounterTactic {
        let mut counter = CounterTactic {
            recommended_style: BuildUpStyle::Mixed,
            recommended_tempo: TeamTempo::Normal,
            key_areas: Vec::new(),
            description_keys: Vec::new(),
        };

        let mut description_keys = Vec::new();

        for weakness in weaknesses {
            match weakness {
                Weakness::HighLine => {
                    counter.recommended_style = BuildUpStyle::Direct;
                    counter.recommended_tempo = TeamTempo::Fast;
                    counter.key_areas.push("behind_defense".to_string());
                    description_keys.push("counter_high_line_direct_fast".to_string());
                }
                Weakness::NarrowWidth => {
                    counter.key_areas.push("wide_areas".to_string());
                    counter.key_areas.push("crosses".to_string());
                    description_keys.push("counter_narrow_width_wide_crosses".to_string());
                }
                Weakness::LowPressing => {
                    counter.recommended_style = BuildUpStyle::Short;
                    counter.key_areas.push("possession".to_string());
                    description_keys.push("counter_low_pressing_possession".to_string());
                }
                Weakness::SlowTempo => {
                    counter.recommended_tempo = TeamTempo::VeryFast;
                    counter.key_areas.push("quick_transitions".to_string());
                    description_keys.push("counter_slow_tempo_fast_play".to_string());
                }
                Weakness::WideFormation => {
                    counter.key_areas.push("central_attacks".to_string());
                    description_keys.push("counter_wide_formation_central".to_string());
                }
                Weakness::DeepLine => {
                    counter.key_areas.push("long_shots".to_string());
                    counter.key_areas.push("patient_buildup".to_string());
                    description_keys.push("counter_deep_line_patience".to_string());
                }
                Weakness::HighPressing => {
                    counter.recommended_style = BuildUpStyle::Direct;
                    counter.key_areas.push("behind_press".to_string());
                    description_keys.push("counter_high_pressing_direct".to_string());
                }
            }
        }

        counter.description_keys = description_keys;
        counter
    }

    /// Calculate confidence in analysis
    fn calculate_confidence(weaknesses: &[Weakness]) -> f32 {
        // More extreme tactics = more predictable = higher confidence
        let base_confidence = 0.5;
        let weakness_bonus = weaknesses.len() as f32 * 0.1;
        (base_confidence + weakness_bonus).min(0.95)
    }

    /// Calculate counter-tactics bonus
    ///
    /// Returns a multiplier (1.0 to 1.5) based on how well our tactics
    /// exploit opponent weaknesses.
    pub fn calculate_counter_bonus(
        my_instructions: &TeamInstructions,
        opponent_instructions: &TeamInstructions,
    ) -> f32 {
        let analysis = Self::analyze(opponent_instructions);
        let mut bonus = 1.0f32;

        for weakness in &analysis.weaknesses {
            match weakness {
                Weakness::HighLine => {
                    // Direct buildup against high line
                    if my_instructions.build_up_style == BuildUpStyle::Direct {
                        bonus *= 1.12;
                    }
                    // Fast tempo for quick counters
                    match my_instructions.team_tempo {
                        TeamTempo::Fast | TeamTempo::VeryFast => bonus *= 1.08,
                        _ => {}
                    }
                }
                Weakness::NarrowWidth => {
                    // Wide formation to exploit flanks
                    match my_instructions.team_width {
                        TeamWidth::Wide | TeamWidth::VeryWide => bonus *= 1.10,
                        _ => {}
                    }
                }
                Weakness::LowPressing => {
                    // Short buildup with time on ball
                    if my_instructions.build_up_style == BuildUpStyle::Short {
                        bonus *= 1.08;
                    }
                }
                Weakness::SlowTempo => {
                    // Fast tempo to outpace
                    match my_instructions.team_tempo {
                        TeamTempo::Fast | TeamTempo::VeryFast => bonus *= 1.10,
                        _ => {}
                    }
                }
                Weakness::WideFormation => {
                    // Narrow formation for central penetration
                    match my_instructions.team_width {
                        TeamWidth::Narrow | TeamWidth::VeryNarrow => bonus *= 1.08,
                        _ => {}
                    }
                }
                Weakness::DeepLine => {
                    // Patient buildup
                    match my_instructions.team_tempo {
                        TeamTempo::Slow | TeamTempo::VerySlow => bonus *= 1.06,
                        _ => {}
                    }
                }
                Weakness::HighPressing => {
                    // Direct play to bypass press
                    if my_instructions.build_up_style == BuildUpStyle::Direct {
                        bonus *= 1.10;
                    }
                    // Fast tempo to escape
                    match my_instructions.team_tempo {
                        TeamTempo::Fast | TeamTempo::VeryFast => bonus *= 1.06,
                        _ => {}
                    }
                }
            }
        }

        bonus.min(1.5) // Cap at 50% bonus
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tactics::TacticalPreset;

    #[test]
    fn test_high_line_detected() {
        let mut instructions = TeamInstructions::default();
        instructions.defensive_line = DefensiveLine::VeryHigh;

        let analysis = OpponentAnalysis::analyze(&instructions);
        assert!(
            analysis.weaknesses.contains(&Weakness::HighLine),
            "Should detect high line weakness"
        );
    }

    #[test]
    fn test_narrow_width_detected() {
        let mut instructions = TeamInstructions::default();
        instructions.team_width = TeamWidth::VeryNarrow;

        let analysis = OpponentAnalysis::analyze(&instructions);
        assert!(
            analysis.weaknesses.contains(&Weakness::NarrowWidth),
            "Should detect narrow width weakness"
        );
    }

    #[test]
    fn test_counter_bonus_against_high_line() {
        let mut my_tactics = TeamInstructions::default();
        my_tactics.build_up_style = BuildUpStyle::Direct;
        my_tactics.team_tempo = TeamTempo::Fast;

        let mut opponent = TeamInstructions::default();
        opponent.defensive_line = DefensiveLine::VeryHigh;

        let bonus = OpponentAnalysis::calculate_counter_bonus(&my_tactics, &opponent);
        assert!(bonus > 1.15, "Direct + Fast vs High Line should give good bonus: {}", bonus);
    }

    #[test]
    fn test_counter_bonus_against_narrow() {
        let mut my_tactics = TeamInstructions::default();
        my_tactics.team_width = TeamWidth::VeryWide;

        let mut opponent = TeamInstructions::default();
        opponent.team_width = TeamWidth::VeryNarrow;

        let bonus = OpponentAnalysis::calculate_counter_bonus(&my_tactics, &opponent);
        assert!(bonus > 1.08, "Wide vs Narrow should give bonus: {}", bonus);
    }

    #[test]
    fn test_high_pressing_counter_bonus() {
        let high_pressing = TeamInstructions::for_style(TacticalPreset::HighPressing);
        let counter = TeamInstructions::for_style(TacticalPreset::Counterattack);

        let bonus = OpponentAnalysis::calculate_counter_bonus(&counter, &high_pressing);
        assert!(bonus > 1.1, "Counter vs High Press should give bonus: {}", bonus);
    }

    #[test]
    fn test_bonus_capped_at_1_5() {
        // Create extreme mismatch
        let mut my_tactics = TeamInstructions::default();
        my_tactics.build_up_style = BuildUpStyle::Direct;
        my_tactics.team_tempo = TeamTempo::VeryFast;
        my_tactics.team_width = TeamWidth::VeryWide;

        let mut opponent = TeamInstructions::default();
        opponent.defensive_line = DefensiveLine::VeryHigh;
        opponent.team_width = TeamWidth::VeryNarrow;
        opponent.pressing_intensity = TeamPressing::VeryHigh;

        let bonus = OpponentAnalysis::calculate_counter_bonus(&my_tactics, &opponent);
        assert!(bonus <= 1.5, "Bonus should be capped at 1.5: {}", bonus);
    }

    #[test]
    fn test_recommended_counter_description() {
        let mut instructions = TeamInstructions::default();
        instructions.defensive_line = DefensiveLine::VeryHigh;

        let analysis = OpponentAnalysis::analyze(&instructions);
        assert!(
            !analysis.recommended_counter.description_keys.is_empty(),
            "Should have counter description keys"
        );
        assert!(
            analysis.recommended_counter.key_areas.contains(&"behind_defense".to_string()),
            "Should recommend attacking behind defense"
        );
    }
}
