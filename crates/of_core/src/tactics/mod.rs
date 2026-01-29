// crates/of_core/src/tactics/mod.rs
// Tactical system module for OpenFootball integration

pub mod ai_profiles;
pub mod famous_tactics;
pub mod openfootball_bridge;
pub mod team_instructions;

// Re-export main types
pub use openfootball_bridge::{
    FormationData, MatchTacticType, PlayerPositionType, PositionWithCoords, TacticalStyle,
};

pub use team_instructions::{
    BuildUpStyle, DefensiveLine, TacticalPreset, TeamInstructions, TeamPressing, TeamTempo,
    TeamWidth,
};

// Famous tactics presets
pub use famous_tactics::{FamousTactics, TacticalStyle as FamousTacticsStyle};

// AI tactical profiles
pub use ai_profiles::{
    AIDifficulty, AITacticalManager, AITacticalProfile, MatchState, ADAPTIVE_AI, AGGRESSIVE_AI,
    BALANCED_AI, COUNTER_AI, DEFENSIVE_AI,
};
