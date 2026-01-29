//! Unified Action Evaluator (UAE)
//!
//! FIX_2601/0108: 모든 축구 액션을 6가지 요소로 평가하는 통일된 시스템
//!
//! ## 핵심 구조
//!
//! ```text
//! State(8개) → ActionSet(상태별 후보) → HardGate → UAE(6요소) → TeamCoord → Execute
//! ```
//!
//! ## 6요소
//!
//! 1. Distance (거리 적합성) - 20%
//! 2. Safety (안전성) - 25%
//! 3. Readiness (준비 상태) - 15%
//! 4. Progression (진행도) - 20%
//! 5. Space (공간) - 10%
//! 6. Tactical (전술) - 10%
//!
//! ## 핵심 원칙
//!
//! 1. 모든 값 [0.0, 1.0] 정규화 (attr/100.0)
//! 2. HardGate는 UAE 전에 후보 제거 (Safety=0 아님)
//! 3. Shoot progression = xG (1.0 고정 아님)
//! 4. 가중치는 multiplicative (additive 아님)

pub mod types;
pub mod state;
pub mod hard_gate;
pub mod weights;
pub mod action_set;
pub mod team_coord;
pub mod pipeline;
pub mod evaluators;

// Re-exports
pub use types::{Action, ActionScore, ActionWeights, ScoredAction, WeightMultiplier};
pub use types::{CrossZone, PassLane, Position, Vec2, Zone};
pub use state::{PlayerPhaseState, RoleTag, StateContext};
pub use hard_gate::HardGate;
pub use weights::WeightCalculator;
pub use action_set::ActionSetBuilder;
pub use team_coord::TeamCoordinator;
pub use pipeline::{DecisionPipeline, PipelineConfig, PipelineResult};
