//! Phase-Based Action System
//!
//! P7: Phase-Based Action Engine - 모든 액션을 N틱 동안 이어지는 FSM으로 구현
//!
//! ## 핵심 개념
//! - **ActionPhase**: 액션의 실행 단계 (Pending → Approach → Commit → Resolve → Recover → Cooldown → Finished)
//!   → **P0**: types.rs의 핵심 타입들은 action_queue.rs로 이동됨
//! - **ActiveAction**: 실행 중인 FSM 액션
//! - **ActionMeta**: 액션별 메타데이터 (Tackle, Pass, Shot, Dribble)
//!
//! ## Intent → Technique → Physics 패턴
//! - **ActionModel trait**: 모든 액션에 공통 인터페이스 제공
//! - **Intent**: 왜 이 행동을 하냐
//! - **Technique**: 어떻게 실행하냐
//! - **PhysicsParams**: 실행 파라미터 (속도/거리/리스크/지속 등)
//!
//! ## 모듈 구조
//! - ~~`types`~~: **P0에서 action_queue.rs로 이동 완료**
//! - `duration`: Phase별 지속 시간 상수
//! - `action_common`: ActionModel trait + 공통 유틸리티
//! - `tackle`: TackleAction FSM
//! - `dribble`: DribbleAction FSM
//! - `pass`: PassAction FSM
//! - `shot`: ShotAction FSM

pub mod action_common; // 공통 ActionModel trait
pub mod ball_physics; // P7 Phase 3: Ball Physics FSM
pub mod dribble; // P7 Phase 4: Dribble FSM
pub mod duration;
pub mod pass; // P7 Phase 5: Pass FSM
pub mod set_piece;
pub mod shot; // P7 Phase 5: Shot FSM
pub mod tackle; // P7 Phase 2: Tackle FSM // P9: Set Piece FSM (Corner, FreeKick, Penalty)

pub use action_common::ActionModel;
// P0: Core types moved to action_queue.rs
// (ActionPhase, ActiveAction, ActionMeta, PhaseActionType, TackleType, PassType, ShotType, etc.)
pub use ball_physics::{BallPhysics, BallPhysicsState, HeightCurve, RestartType};
pub use dribble::{
    can_start_dribble, should_trigger_evade, DribbleAction, DribbleContext, DribbleIntent,
    DribbleModel, DribblePhase, DribblePhysicsParams, DribbleResult, DribbleSkills,
    DribbleTechnique,
};
pub use duration::*;
pub use pass::{
    calculate_pass_difficulty, choose_pass_technique, pass_base_success_prob, PassAction,
    PassContext, PassIntent, PassModel, PassPhase, PassPhysicsParams, PassResult, PassSkills,
    PassTechnique, PassTechniqueSelection,
};
pub use set_piece::{
    calculate_aerial_score, calculate_cross_accuracy, resolve_aerial_duel, resolve_direct_freekick,
    resolve_penalty, AerialDefender, AerialTarget, CornerKickContext, CornerTactic,
    FreeKickContext, FreeKickTactic, PenaltyContext, SetPieceAction, SetPiecePhase, SetPieceResult,
    SetPieceType,
};
pub use shot::{
    calculate_save_probability, calculate_xg, calculate_xg_with_target, choose_shot_technique,
    ShooterSkills, ShotAction, ShotContext, ShotIntent, ShotModel, ShotPhase, ShotPhysicsParams,
    ShotResult, ShotTechnique,
};
pub use tackle::{
    calculate_approach_angle, can_attempt_tackle, TackleAction, TackleAttemptResult, TacklePhase,
};
