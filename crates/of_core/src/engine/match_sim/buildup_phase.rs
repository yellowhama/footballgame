//! BuildupPhase - Field Position-Based Tactical Phases
//!
//! Based on Open-Football's 3-Phase Buildup System (0106_BUILDUP_PLAY_COMPARISON.md):
//! - OwnThird: Safety-focused, slow buildup (< 33% field progress)
//! - MiddleThird: Balanced, transition play (33-66% field progress)
//! - FinalThird: Risk-tolerant, attacking focus (> 66% field progress)
//!
//! Each phase modifies:
//! - Safety weight multiplier (패스 안전성 중요도)
//! - Progression weight multiplier (전진 패스 선호도)
//! - Shot allowance (슛 허용 여부)

use crate::engine::physics_constants::field;

/// Field position-based tactical phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildupPhase {
    /// Own third (< 33% progress toward opponent goal)
    /// - High safety priority, low progression
    /// - Conservative passing, avoid risky plays
    #[default]
    OwnThird,

    /// Middle third (33-66% progress)
    /// - Balanced approach
    /// - Normal pass evaluation weights
    MiddleThird,

    /// Final third (> 66% progress)
    /// - High risk tolerance, attacking focus
    /// - Encourages through balls and shots
    FinalThird,
}

impl BuildupPhase {
    /// Determine phase from ball position
    ///
    /// # Arguments
    /// * `ball_x_m` - Ball X position in meters (0-105)
    /// * `attacks_right` - True if team attacks toward X=105
    ///
    /// # Returns
    /// The current buildup phase for the attacking team
    pub fn from_ball_position(ball_x_m: f32, attacks_right: bool) -> Self {
        // Calculate progress toward opponent goal (0.0 = own goal, 1.0 = opponent goal)
        // FIX_2601/0106 P2-9: Clamp for out-of-bounds ball positions
        let progress = if attacks_right {
            (ball_x_m / field::LENGTH_M).clamp(0.0, 1.0)
        } else {
            (1.0 - ball_x_m / field::LENGTH_M).clamp(0.0, 1.0)
        };

        if progress < 0.33 {
            BuildupPhase::OwnThird
        } else if progress < 0.66 {
            BuildupPhase::MiddleThird
        } else {
            BuildupPhase::FinalThird
        }
    }

    /// Safety factor multiplier for pass evaluation
    ///
    /// - OwnThird: 1.5x (prioritize safe passes near own goal)
    /// - MiddleThird: 1.0x (balanced)
    /// - FinalThird: 0.7x (allow riskier passes for penetration)
    #[inline]
    pub fn safety_multiplier(&self) -> f32 {
        match self {
            BuildupPhase::OwnThird => 1.5,
            BuildupPhase::MiddleThird => 1.0,
            BuildupPhase::FinalThird => 0.7,
        }
    }

    /// Progression factor multiplier for pass evaluation
    ///
    /// - OwnThird: 0.75x (conservative but still allows progression)
    /// - MiddleThird: 1.0x (balanced)
    /// - FinalThird: 1.5x (encourage final balls)
    ///
    /// FIX_2601/0110: Increased OwnThird from 0.5 to 0.75 to prevent teams
    /// from getting stuck with Short build-up style. The old value caused
    /// circulation_w (0.195) > progression_w (0.16), making backward passes
    /// more attractive than forward passes in own third.
    #[inline]
    pub fn progression_multiplier(&self) -> f32 {
        match self {
            BuildupPhase::OwnThird => 0.75,
            BuildupPhase::MiddleThird => 1.0,
            BuildupPhase::FinalThird => 1.5,
        }
    }

    /// Whether shooting is allowed in this phase
    ///
    /// - OwnThird: false (never shoot from own third)
    /// - MiddleThird: false (only exceptional cases)
    /// - FinalThird: true (normal shooting zone)
    #[inline]
    pub fn allows_shot(&self) -> bool {
        matches!(self, BuildupPhase::FinalThird)
    }

    /// Minimum distance to goal (meters) for shot attempt
    ///
    /// Returns None if no shot allowed, Some(max_distance) otherwise
    #[inline]
    pub fn shot_max_distance(&self) -> Option<f32> {
        match self {
            BuildupPhase::OwnThird => None,
            BuildupPhase::MiddleThird => None, // Could be Some(25.0) for long shots
            BuildupPhase::FinalThird => Some(35.0),
        }
    }

    /// Risk tolerance level (0.0 = conservative, 1.0 = aggressive)
    #[inline]
    pub fn risk_tolerance(&self) -> f32 {
        match self {
            BuildupPhase::OwnThird => 0.2,
            BuildupPhase::MiddleThird => 0.5,
            BuildupPhase::FinalThird => 0.8,
        }
    }

    /// Preferred pass type for this phase
    #[inline]
    pub fn preferred_pass_style(&self) -> PassStyle {
        match self {
            BuildupPhase::OwnThird => PassStyle::SafeShort,
            BuildupPhase::MiddleThird => PassStyle::Progressive,
            BuildupPhase::FinalThird => PassStyle::Penetrating,
        }
    }
}

/// Pass style preference based on phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassStyle {
    /// Short, safe passes (own third)
    SafeShort,
    /// Forward-looking but balanced (middle third)
    Progressive,
    /// Through balls and final passes (final third)
    Penetrating,
}

impl PassStyle {
    /// Whether this style allows long passes
    #[inline]
    pub fn allows_long_pass(&self) -> bool {
        !matches!(self, PassStyle::SafeShort)
    }

    /// Whether this style allows through balls
    #[inline]
    pub fn allows_through_ball(&self) -> bool {
        matches!(self, PassStyle::Penetrating)
    }
}

/// Phase-aware pass weight adjustment
///
/// Returns adjusted weights based on current buildup phase
pub fn adjust_pass_weights(
    base_safety: f32,
    base_progression: f32,
    phase: BuildupPhase,
) -> (f32, f32) {
    let safety = (base_safety * phase.safety_multiplier()).clamp(0.05, 0.50);
    let progression = (base_progression * phase.progression_multiplier()).clamp(0.05, 0.60);
    (safety, progression)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_from_position_attacks_right() {
        // Home team attacks right (toward X=105)
        assert_eq!(BuildupPhase::from_ball_position(10.0, true), BuildupPhase::OwnThird);
        assert_eq!(
            BuildupPhase::from_ball_position(field::CENTER_X, true),
            BuildupPhase::MiddleThird
        );
        assert_eq!(BuildupPhase::from_ball_position(80.0, true), BuildupPhase::FinalThird);
    }

    #[test]
    fn test_phase_from_position_attacks_left() {
        // Away team attacks left (toward X=0)
        assert_eq!(BuildupPhase::from_ball_position(95.0, false), BuildupPhase::OwnThird);
        assert_eq!(
            BuildupPhase::from_ball_position(field::CENTER_X, false),
            BuildupPhase::MiddleThird
        );
        assert_eq!(BuildupPhase::from_ball_position(20.0, false), BuildupPhase::FinalThird);
    }

    #[test]
    fn test_safety_multiplier() {
        assert_eq!(BuildupPhase::OwnThird.safety_multiplier(), 1.5);
        assert_eq!(BuildupPhase::MiddleThird.safety_multiplier(), 1.0);
        assert_eq!(BuildupPhase::FinalThird.safety_multiplier(), 0.7);
    }

    #[test]
    fn test_progression_multiplier() {
        assert_eq!(BuildupPhase::OwnThird.progression_multiplier(), 0.75);
        assert_eq!(BuildupPhase::MiddleThird.progression_multiplier(), 1.0);
        assert_eq!(BuildupPhase::FinalThird.progression_multiplier(), 1.5);
    }

    #[test]
    fn test_shot_allowed() {
        assert!(!BuildupPhase::OwnThird.allows_shot());
        assert!(!BuildupPhase::MiddleThird.allows_shot());
        assert!(BuildupPhase::FinalThird.allows_shot());
    }

    #[test]
    fn test_adjust_pass_weights() {
        let base_safety = 0.15;
        let base_progression = 0.40;

        // Own third: safety up, progression down
        let (safety, prog) =
            adjust_pass_weights(base_safety, base_progression, BuildupPhase::OwnThird);
        assert!((safety - 0.225).abs() < 0.01); // 0.15 * 1.5
        assert!((prog - 0.30).abs() < 0.01); // 0.40 * 0.75

        // Middle third: unchanged
        let (safety, prog) =
            adjust_pass_weights(base_safety, base_progression, BuildupPhase::MiddleThird);
        assert!((safety - 0.15).abs() < 0.01);
        assert!((prog - 0.40).abs() < 0.01);

        // Final third: safety down, progression up
        let (safety, prog) =
            adjust_pass_weights(base_safety, base_progression, BuildupPhase::FinalThird);
        assert!((safety - 0.105).abs() < 0.01); // 0.15 * 0.7
        assert!((prog - 0.60).abs() < 0.01); // 0.40 * 1.5, clamped to 0.60
    }

    #[test]
    fn test_risk_tolerance() {
        assert!(
            BuildupPhase::OwnThird.risk_tolerance() < BuildupPhase::MiddleThird.risk_tolerance()
        );
        assert!(
            BuildupPhase::MiddleThird.risk_tolerance() < BuildupPhase::FinalThird.risk_tolerance()
        );
    }

    #[test]
    fn test_pass_style() {
        assert!(!BuildupPhase::OwnThird.preferred_pass_style().allows_long_pass());
        assert!(BuildupPhase::MiddleThird.preferred_pass_style().allows_long_pass());
        assert!(BuildupPhase::FinalThird.preferred_pass_style().allows_through_ball());
    }
}
