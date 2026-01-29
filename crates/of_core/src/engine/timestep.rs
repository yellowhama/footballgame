/// timestep.rs
/// Dual Timestep Architecture Constants
///
/// Phase 1.0.1: Core foundation for "Think slowly, move smoothly"
///
/// Philosophy:
/// - Decision Tick (250ms): Tactical brain, marking, contest
/// - Integration Tick (50ms): Physics, smooth motion, interpolation
///
/// This allows deliberate tactical decisions while maintaining visual smoothness.

/// Decision timestep (250ms) - tactical brain tick rate
pub const DECISION_DT: f32 = 0.25;

/// Integration timestep (50ms) - physics update rate
pub const SUBSTEP_DT: f32 = 0.05;

/// Number of substeps per decision window
pub const SUBSTEPS_PER_DECISION: u8 = 5;

// Compile-time validation
const _: () = assert!(DECISION_DT / SUBSTEP_DT == SUBSTEPS_PER_DECISION as f32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestep_consistency() {
        assert_eq!(DECISION_DT, 0.25);
        assert_eq!(SUBSTEP_DT, 0.05);
        assert_eq!(SUBSTEPS_PER_DECISION, 5);
        assert_eq!(DECISION_DT / SUBSTEP_DT, 5.0);
    }

    #[test]
    fn test_ticks_per_minute() {
        // 60 seconds / 0.25s = 240 ticks/minute
        let ticks_per_minute = (60.0 / DECISION_DT) as u64;
        assert_eq!(ticks_per_minute, 240);
    }

    #[test]
    fn test_substeps_per_minute() {
        // 240 decisions × 5 substeps = 1200 substeps/minute
        let substeps_per_minute = (60.0 / SUBSTEP_DT) as u64;
        assert_eq!(substeps_per_minute, 1200);
    }
}
