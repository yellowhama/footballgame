//! Player system module
//!
//! This module contains the core player development system including:
//! - CorePlayer struct with CA/PA system
//! - HexagonStats for 6-sided ability visualization
//! - Growth profiles and progression system
//! - Position-specific weighting and calculations
//! - Validation and error handling

pub mod ca_model;
pub mod ca_weights;
pub mod calculator;
pub mod growth_calculator;
pub mod hexagon;
pub mod instructions;
pub mod optimization;
pub mod personality;
pub mod position_weights;
pub mod skill_system;
pub mod types;
pub mod validation;

// Legacy re-export for backward compatibility
pub mod ca_calculator {
    pub use super::calculator::*;
}

// Re-export main types
pub use calculator::{CACalculationDetails, CACalculator};
pub use growth_calculator::GrowthCalculator;
pub use hexagon::HexagonCalculator;
pub use instructions::{
    apply_instructions_modifiers, DefensiveWork, Depth, DribblingFrequency, Mentality,
    PassingStyle, PlayerInstructions, PlayerRole, PressingIntensity, ShootingTendency, Width,
};
pub use optimization::{
    BatchProcessingResult, BatchProcessor, BulkOperation, BulkOperationResult, CachedCalculations,
    MemoryStats, OperationType, OptimizedAttributeCalculator, PlayerMemoryPool,
};
pub use personality::{
    DecisionModifiers, OpenFootPersonAttributes, PersonAttributes, PersonalityArchetype,
};
pub use position_weights::PositionWeights;
pub use skill_system::SkillCalculator;
pub use types::{
    AttributeChange, AttributeGrowth, CorePlayer, GrowthPotential, GrowthProfile, HexagonStats,
    InjuryEffect, InjurySeverity, MonthlyGrowth, PlayerCareerStats, SeasonStats, TrainingResponse,
    TrainingType,
};
pub use validation::{PlayerValidator, ValidationError};

#[cfg(test)]
pub mod tests;

#[cfg(test)]
pub mod memory_test;
