pub mod events;
pub mod match_result;
pub mod match_setup;
pub mod match_statistics;
pub mod oracle;
pub mod person;
pub mod player;
pub mod replay;
pub mod rules;
pub mod skill;
pub mod team;
pub mod trait_balance;
pub mod trait_system;

#[cfg(test)]
mod match_setup_contracts_test;

pub use events::{
    EventDetails, EventType, InjurySeverity, MatchEvent, SubstitutionDetails, VarReviewDetails,
    VarReviewOutcome,
};
pub use match_result::{
    generate_best_moments, BestMoment, DeterminismMeta, DeterminismMode, HashAlgorithm, HeatMapPoint,
    MatchPositionData, MatchResult, MatchSummary, MomentType, MyPlayerStats, PenaltyShootoutResult,
    PlayerState, Statistics,
};
pub use match_statistics::{EventCoordinates, MatchStatistics, ShotEvent};
pub use oracle::{EventCounts, FixtureInfo, Invariants, MatchResultSnapshot, OracleSnapshot};
pub use person::Person;
pub use player::{Player, Position};
pub use replay::*;
pub use skill::{ActionType, SkillContext, SpecialSkill};
pub use team::{Formation, Team};
pub use trait_system::{
    ActionType as TraitActionType, EquippedTrait, StatType, TraitCategory, TraitError, TraitId,
    TraitSlots, TraitTier,
};

// P17: MatchSetup exports
pub use match_setup::{MatchPlayer, MatchSetup, PlayerSlot, TeamSetup, TeamSide};
// P17 Phase 5: Viewer Export types
pub use match_setup::{MatchSetupExport, PlayerSlotExport, TeamSetupExport};

// RuleBook System (IFAB Laws of the Game)
pub use rules::{
    // Core types
    RuleId, RestartType,
    // Offside (Law 11)
    OffsideDetails, OffsideInvolvementType, OffsideRestartContext,
    TouchType, TouchReference, ReferencePoint,
    DefenderTouchType, DeflectionContext,
    // Fouls (Law 12)
    FoulDetails, FoulSanction, FoulSeverity, FoulType,
    // YAML data structures
    FoulsRuleData, OffsideRuleData,
};
