/// plan_window.rs
/// PlanWindow: 250ms Decision Output Container
///
/// Phase 1.0.2-1.0.4: Core data structures for dual timestep architecture
///
/// Philosophy:
/// "What will happen in the next 250ms" - decided once, executed 5 times at 50ms intervals
// Use existing HeightCurve from phase_action
pub use super::phase_action::ball_physics::HeightCurve;

/// ============================================================================
/// 1.0.2. PlanWindow (250ms Decision Output)
/// ============================================================================

/// Container for "what will happen in the next 250ms" (decided once, executed 5 times)
#[derive(Debug, Clone)]
pub struct PlanWindow {
    /// Which 250ms tick this plan starts from
    pub start_tick_250: u64,

    /// How many 50ms substeps remain (always 5 at creation, decrements to 0)
    pub substeps_remaining: u8,

    /// Per-player movement plans (size=22: 0-10 home, 11-21 away)
    pub player_plans: [PlayerPlan; 22],

    /// Ball physics plan
    pub ball_plan: BallPlan,

    /// Events confirmed during decision phase (goals, fouls, etc.)
    /// These are added to MatchResult at decision time, not during substeps
    pub planned_events: Vec<PlannedEvent>,

    /// Debug data (threat scores, marking assignments)
    /// Only populated if debug mode enabled
    pub debug_slice: Option<DebugSlice>,

    /// Early termination flag (set by exec_substep if OutOfPlay/Goal occurs)
    pub end_now: bool,
}

impl Default for PlanWindow {
    fn default() -> Self {
        Self {
            start_tick_250: 0,
            substeps_remaining: super::timestep::SUBSTEPS_PER_DECISION,
            player_plans: [PlayerPlan::default(); 22],
            ball_plan: BallPlan::default(),
            planned_events: Vec::new(),
            debug_slice: None,
            end_now: false,
        }
    }
}

/// Placeholder for events (will integrate with MatchEvent later)
#[derive(Debug, Clone)]
pub struct PlannedEvent {
    pub event_type: String,
    pub timestamp_ms: u64,
}

/// Placeholder for debug data
#[derive(Debug, Clone, Default)]
pub struct DebugSlice {
    pub carrier_free_score: f32,
    pub transition_remaining_ms: Option<u32>,
}

/// ============================================================================
/// 1.0.3. PlayerPlan (50ms Execution Parameters)
/// ============================================================================

/// Contains target position and physics parameters for a player to execute over 5 substeps
#[derive(Debug, Clone, Copy)]
pub struct PlayerPlan {
    /// Movement goal type
    pub kind: PlayerPlanKind,

    /// ★ TARGET POSITION (결합 공식 결과)
    /// Calculated from: Tactical Anchor + Marking Offset + Transition Bias
    pub target_pos: (f32, f32),

    /// Body orientation goal (for pressure angle, animations)
    pub desired_facing: (f32, f32),

    /// Physics limits (affected by stamina, tactics)
    pub max_speed: f32, // m/s (5-10.44 for field players, from attributes)
    pub accel: f32, // m/s^2
    pub decel: f32, // m/s^2

    /// Role flags (press/cover/mark/support/run)
    pub flags: u32,

    /// Marking target track_id (0-21, or -1 if none)
    pub mark_target_idx: i32,
}

impl Default for PlayerPlan {
    fn default() -> Self {
        Self {
            kind: PlayerPlanKind::IdleHold,
            target_pos: (0.5, 0.5),     // Center of field (normalized)
            desired_facing: (1.0, 0.0), // Default facing right
            max_speed: 7.0,             // Average player speed
            accel: 5.0,
            decel: 8.0,
            flags: 0,
            mark_target_idx: -1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPlanKind {
    IdleHold,       // Stay at position
    MoveTo,         // Generic movement
    PressOwner,     // Aggressive approach to ball carrier
    MarkOpponent,   // Shadow marking target
    InterceptLane,  // Block passing lane
    SupportCarrier, // Offensive support run
}

/// ============================================================================
/// 1.0.4. BallPlan (50ms Ball Physics)
/// ============================================================================

/// Ball physics plan for the next 250ms
#[derive(Debug, Clone)]
pub enum BallPlan {
    /// Ball is controlled by a player (track_id 0-21)
    Controlled { owner_idx: u8 },

    /// Ball is in flight (pass/shot/clearance)
    InFlight {
        start_pos: (f32, f32),
        end_pos: (f32, f32),
        start_v: (f32, f32),
        height_curve: HeightCurve,
        t_total: f32,   // Total flight time (seconds)
        t_elapsed: f32, // Time elapsed since launch (seconds)
    },

    /// Ball is rolling on the ground (slowing down)
    Rolling {
        pos: (f32, f32),
        velocity: (f32, f32),
        friction: f32, // m/s^2 (deceleration)
    },

    /// Ball is bouncing (after hitting ground from height)
    Bouncing {
        pos: (f32, f32),
        velocity: (f32, f32),
        height: f32,
        v_vertical: f32, // Vertical velocity (m/s)
    },

    /// Ball is out of play (awaiting restart)
    OutOfPlay {
        reason: String, // "goal", "corner", "throw_in", "goal_kick"
    },
}

impl Default for BallPlan {
    fn default() -> Self {
        Self::Controlled { owner_idx: 0 }
    }
}

// HeightCurve is imported from phase_action::ball_physics (see top of file)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_window_default() {
        let plan = PlanWindow::default();
        assert_eq!(plan.substeps_remaining, 5);
        assert_eq!(plan.player_plans.len(), 22);
        assert!(!plan.end_now);
    }

    #[test]
    fn test_player_plan_default() {
        let plan = PlayerPlan::default();
        assert_eq!(plan.kind, PlayerPlanKind::IdleHold);
        assert_eq!(plan.mark_target_idx, -1);
        assert!(plan.max_speed > 0.0);
    }

    #[test]
    fn test_ball_plan_controlled() {
        let plan = BallPlan::Controlled { owner_idx: 5 };
        match plan {
            BallPlan::Controlled { owner_idx } => assert_eq!(owner_idx, 5),
            _ => panic!("Expected Controlled"),
        }
    }
}
