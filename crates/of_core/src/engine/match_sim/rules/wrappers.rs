//! Rule Wrappers - A/B Comparison Layer
//!
//! This module provides wrapper functions for gradual migration from
//! scattered rule checks to the centralized RuleDispatcher.
//!
//! ## Design
//! Each wrapper:
//! 1. Runs legacy code and gets its decision
//! 2. Runs dispatcher and gets its decision
//! 3. Compares and logs discrepancies (if tracking enabled)
//! 4. Returns appropriate decision based on RuleCheckMode

use super::types::{
    Card, ContactEvent, DefenseIntentInfo, FoulType, HandballContactEvent, PassEvent,
    RuleCheckMode, RuleDecision, RuleRestartType, RuleTeamId,
};
use super::RuleDispatcher;
use crate::engine::types::Coord10;
use std::sync::atomic::{AtomicU64, Ordering};

/// Comparison statistics for A/B testing
#[derive(Debug, Default)]
pub struct ComparisonStats {
    /// Total comparisons made
    pub total_comparisons: AtomicU64,
    /// Number of matches (legacy == dispatcher)
    pub matches: AtomicU64,
    /// Number of mismatches
    pub mismatches: AtomicU64,
    /// Goal decision mismatches
    pub goal_mismatches: AtomicU64,
    /// Out of play decision mismatches
    pub out_of_play_mismatches: AtomicU64,
    /// Offside decision mismatches
    pub offside_mismatches: AtomicU64,
    /// Foul decision mismatches
    pub foul_mismatches: AtomicU64,
    /// Handball decision mismatches
    pub handball_mismatches: AtomicU64,
}

impl ComparisonStats {
    /// Create new comparison stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Get match rate (0.0 - 1.0)
    pub fn match_rate(&self) -> f64 {
        let total = self.total_comparisons.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0;
        }
        self.matches.load(Ordering::Relaxed) as f64 / total as f64
    }

    /// Record a comparison result
    pub fn record(&self, decision_type: &str, matched: bool) {
        self.total_comparisons.fetch_add(1, Ordering::Relaxed);
        if matched {
            self.matches.fetch_add(1, Ordering::Relaxed);
        } else {
            self.mismatches.fetch_add(1, Ordering::Relaxed);
            match decision_type {
                "goal" => {
                    self.goal_mismatches.fetch_add(1, Ordering::Relaxed);
                }
                "out_of_play" => {
                    self.out_of_play_mismatches.fetch_add(1, Ordering::Relaxed);
                }
                "offside" => {
                    self.offside_mismatches.fetch_add(1, Ordering::Relaxed);
                }
                "foul" => {
                    self.foul_mismatches.fetch_add(1, Ordering::Relaxed);
                }
                "handball" => {
                    self.handball_mismatches.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
        }
    }

    /// Print summary to log
    pub fn log_summary(&self) {
        let total = self.total_comparisons.load(Ordering::Relaxed);
        let matches = self.matches.load(Ordering::Relaxed);
        let mismatches = self.mismatches.load(Ordering::Relaxed);

        if total > 0 {
            log::info!(
                "Rule comparison stats: {}/{} matches ({:.1}%), {} mismatches",
                matches,
                total,
                self.match_rate() * 100.0,
                mismatches
            );

            if mismatches > 0 {
                log::info!(
                    "  Mismatches by type: goal={}, out_of_play={}, offside={}, foul={}, handball={}",
                    self.goal_mismatches.load(Ordering::Relaxed),
                    self.out_of_play_mismatches.load(Ordering::Relaxed),
                    self.offside_mismatches.load(Ordering::Relaxed),
                    self.foul_mismatches.load(Ordering::Relaxed),
                    self.handball_mismatches.load(Ordering::Relaxed)
                );
            }
        }
    }
}

/// Global comparison stats (for A/B testing)
static COMPARISON_STATS: std::sync::OnceLock<ComparisonStats> = std::sync::OnceLock::new();

/// Get global comparison stats
pub fn comparison_stats() -> &'static ComparisonStats {
    COMPARISON_STATS.get_or_init(ComparisonStats::new)
}

/// Legacy goal check result
#[derive(Debug, Clone)]
pub struct LegacyGoalResult {
    /// Goal detected
    pub is_goal: bool,
    /// Scoring team
    pub scoring_team: Option<RuleTeamId>,
    /// Scorer player index
    pub scorer_idx: Option<usize>,
    /// Assister player index
    pub assister_idx: Option<usize>,
    /// Ball position
    pub position: Coord10,
}

impl LegacyGoalResult {
    /// Convert to RuleDecision
    pub fn to_decision(&self) -> Option<RuleDecision> {
        if self.is_goal {
            self.scorer_idx.map(|scorer| RuleDecision::Goal {
                scorer_idx: scorer,
                assister_idx: self.assister_idx,
                position: self.position.clone(),
            })
        } else {
            None
        }
    }
}

/// Goal check wrapper with A/B comparison
///
/// # Arguments
/// * `mode` - Rule check mode
/// * `dispatcher` - RuleDispatcher instance
/// * `legacy_result` - Result from legacy goal check
/// * `ball_pos` - Current ball position
/// * `last_touch_team` - Team that last touched the ball
///
/// # Returns
/// Whether a goal was scored (based on mode)
pub fn check_goal_wrapper(
    mode: RuleCheckMode,
    dispatcher: &mut RuleDispatcher,
    legacy_result: &LegacyGoalResult,
    ball_pos: &Coord10,
    last_touch_team: RuleTeamId,
    rng_roll: f32,
) -> Option<RuleDecision> {
    // Get dispatcher decision
    let dispatcher_decisions = dispatcher.evaluate_tick(
        ball_pos,
        legacy_result.scoring_team,
        last_touch_team,
        legacy_result.scorer_idx,
        legacy_result.assister_idx,
        None,
        false,
        &[],
        rng_roll,
    );

    let dispatcher_goal = dispatcher_decisions
        .iter()
        .find(|d| matches!(d, RuleDecision::Goal { .. }))
        .cloned();

    // Compare if tracking enabled
    if mode.tracking_enabled() {
        let legacy_decision = legacy_result.to_decision();
        let matched = match (&legacy_decision, &dispatcher_goal) {
            (Some(_), Some(_)) => true,
            (None, None) => true,
            _ => false,
        };

        comparison_stats().record("goal", matched);

        if !matched {
            log::debug!(
                "Goal mismatch: legacy={:?}, dispatcher={:?}",
                legacy_decision.is_some(),
                dispatcher_goal.is_some()
            );
        }
    }

    // Return based on mode
    if mode.dispatcher_applies() {
        dispatcher_goal
    } else {
        legacy_result.to_decision()
    }
}

/// Legacy out of play result
#[derive(Debug, Clone)]
pub struct LegacyOutOfPlayResult {
    /// Is ball out of play
    pub is_out: bool,
    /// Last touch team
    pub last_touch_team: RuleTeamId,
    /// Restart type
    pub restart_type: Option<RuleRestartType>,
    /// Ball position
    pub position: Coord10,
}

impl LegacyOutOfPlayResult {
    /// Convert to RuleDecision
    pub fn to_decision(&self) -> Option<RuleDecision> {
        if self.is_out {
            self.restart_type.map(|restart_type| RuleDecision::OutOfPlay {
                last_touch_team: self.last_touch_team,
                position: self.position.clone(),
                restart_type,
            })
        } else {
            None
        }
    }
}

/// Out of play check wrapper with A/B comparison
pub fn check_out_of_play_wrapper(
    mode: RuleCheckMode,
    dispatcher: &mut RuleDispatcher,
    legacy_result: &LegacyOutOfPlayResult,
    ball_pos: &Coord10,
    last_touch_team: RuleTeamId,
    rng_roll: f32,
) -> Option<RuleDecision> {
    // Get dispatcher decision
    let dispatcher_decisions = dispatcher.evaluate_tick(
        ball_pos,
        None,
        last_touch_team,
        None,
        None,
        None,
        false,
        &[],
        rng_roll,
    );

    let dispatcher_out = dispatcher_decisions
        .iter()
        .find(|d| matches!(d, RuleDecision::OutOfPlay { .. }))
        .cloned();

    // Compare if tracking enabled
    if mode.tracking_enabled() {
        let legacy_decision = legacy_result.to_decision();
        let matched = match (&legacy_decision, &dispatcher_out) {
            (Some(RuleDecision::OutOfPlay { restart_type: r1, .. }),
             Some(RuleDecision::OutOfPlay { restart_type: r2, .. })) => r1 == r2,
            (None, None) => true,
            _ => false,
        };

        comparison_stats().record("out_of_play", matched);

        if !matched {
            log::debug!(
                "Out of play mismatch: legacy={:?}, dispatcher={:?}",
                legacy_decision,
                dispatcher_out
            );
        }
    }

    // Return based on mode
    if mode.dispatcher_applies() {
        dispatcher_out
    } else {
        legacy_result.to_decision()
    }
}

/// Legacy offside result
#[derive(Debug, Clone)]
pub struct LegacyOffsideResult {
    /// Is offside detected
    pub is_offside: bool,
    /// Offside player index
    pub player_idx: Option<usize>,
    /// Player position
    pub position: Option<Coord10>,
    /// Pass origin
    pub pass_origin: Option<Coord10>,
}

impl LegacyOffsideResult {
    /// Convert to RuleDecision
    pub fn to_decision(&self) -> Option<RuleDecision> {
        if self.is_offside {
            match (self.player_idx, &self.position, &self.pass_origin) {
                (Some(player_idx), Some(position), Some(pass_origin)) => {
                    Some(RuleDecision::Offside {
                        player_idx,
                        position: position.clone(),
                        pass_origin: pass_origin.clone(),
                    })
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

/// Offside check wrapper with A/B comparison
pub fn check_offside_wrapper(
    mode: RuleCheckMode,
    dispatcher: &mut RuleDispatcher,
    legacy_result: &LegacyOffsideResult,
    pass_event: Option<&PassEvent>,
    ball_pos: &Coord10,
    last_touch_team: RuleTeamId,
    rng_roll: f32,
) -> Option<RuleDecision> {
    // Get dispatcher decision
    let dispatcher_decisions = dispatcher.evaluate_tick(
        ball_pos,
        None,
        last_touch_team,
        None,
        None,
        pass_event,
        legacy_result.is_offside, // Pass legacy offside detection to dispatcher
        &[],
        rng_roll,
    );

    let dispatcher_offside = dispatcher_decisions
        .iter()
        .find(|d| matches!(d, RuleDecision::Offside { .. }))
        .cloned();

    // Compare if tracking enabled
    if mode.tracking_enabled() {
        let legacy_decision = legacy_result.to_decision();
        let matched = match (&legacy_decision, &dispatcher_offside) {
            (Some(_), Some(_)) => true,
            (None, None) => true,
            _ => false,
        };

        comparison_stats().record("offside", matched);

        if !matched {
            log::debug!(
                "Offside mismatch: legacy={:?}, dispatcher={:?}",
                legacy_decision.is_some(),
                dispatcher_offside.is_some()
            );
        }
    }

    // Return based on mode
    if mode.dispatcher_applies() {
        dispatcher_offside
    } else {
        legacy_result.to_decision()
    }
}

/// Legacy foul result
#[derive(Debug, Clone)]
pub struct LegacyFoulResult {
    /// Is foul detected
    pub is_foul: bool,
    /// Offender player index
    pub offender_idx: Option<usize>,
    /// Victim player index
    pub victim_idx: Option<usize>,
    /// Foul type
    pub foul_type: Option<FoulType>,
    /// Position
    pub position: Option<Coord10>,
    /// Card given
    pub card: Option<Card>,
}

impl LegacyFoulResult {
    /// Convert to RuleDecision
    pub fn to_decision(&self) -> Option<RuleDecision> {
        if self.is_foul {
            match (self.offender_idx, self.victim_idx, self.foul_type, &self.position) {
                (Some(offender_idx), Some(victim_idx), Some(foul_type), Some(position)) => {
                    Some(RuleDecision::Foul {
                        offender_idx,
                        victim_idx,
                        foul_type,
                        position: position.clone(),
                        card: self.card,
                    })
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

/// Foul check wrapper with A/B comparison
pub fn check_foul_wrapper(
    mode: RuleCheckMode,
    dispatcher: &mut RuleDispatcher,
    legacy_result: &LegacyFoulResult,
    contacts: &[ContactEvent],
    ball_pos: &Coord10,
    last_touch_team: RuleTeamId,
    rng_roll: f32,
) -> Option<RuleDecision> {
    // Get dispatcher decision
    let dispatcher_decisions = dispatcher.evaluate_tick(
        ball_pos,
        None,
        last_touch_team,
        None,
        None,
        None,
        false,
        contacts,
        rng_roll,
    );

    let dispatcher_foul = dispatcher_decisions
        .iter()
        .find(|d| matches!(d, RuleDecision::Foul { .. }))
        .cloned();

    // Compare if tracking enabled
    if mode.tracking_enabled() {
        let legacy_decision = legacy_result.to_decision();
        let matched = match (&legacy_decision, &dispatcher_foul) {
            (Some(_), Some(_)) => true,
            (None, None) => true,
            _ => false,
        };

        comparison_stats().record("foul", matched);

        if !matched {
            log::debug!(
                "Foul mismatch: legacy={:?}, dispatcher={:?}",
                legacy_decision,
                dispatcher_foul
            );
        }
    }

    // Return based on mode
    if mode.dispatcher_applies() {
        dispatcher_foul
    } else {
        legacy_result.to_decision()
    }
}

/// Unified rule context for wrapper calls
#[derive(Debug, Clone)]
pub struct RuleContext {
    /// Rule check mode
    pub mode: RuleCheckMode,
    /// Current ball position
    pub ball_pos: Coord10,
    /// Last touch team
    pub last_touch_team: RuleTeamId,
    /// RNG roll for this tick
    pub rng_roll: f32,
}

impl RuleContext {
    /// Create a new rule context
    pub fn new(
        mode: RuleCheckMode,
        ball_pos: Coord10,
        last_touch_team: RuleTeamId,
        rng_roll: f32,
    ) -> Self {
        Self {
            mode,
            ball_pos,
            last_touch_team,
            rng_roll,
        }
    }

    /// Create from environment
    pub fn from_env(ball_pos: Coord10, last_touch_team: RuleTeamId, rng_roll: f32) -> Self {
        Self {
            mode: RuleCheckMode::from_env(),
            ball_pos,
            last_touch_team,
            rng_roll,
        }
    }
}

// ============================================================================
// FIX_2601/0123 Phase 6: Duel Foul Wrapper (Technique-Aware)
// ============================================================================

/// Legacy duel foul result (from duel.rs FoulInfo)
#[derive(Debug, Clone)]
pub struct LegacyDuelFoulResult {
    /// Foul occurred
    pub occurred: bool,
    /// Yellow card
    pub yellow_card: bool,
    /// Red card
    pub red_card: bool,
    /// Penalty kick awarded
    pub penalty_kick: bool,
    /// Free kick position (pitch ratio 0-1)
    pub free_kick_position: Option<f32>,
    /// Defender player index
    pub defender_idx: usize,
    /// Attacker player index
    pub attacker_idx: usize,
    /// Foul position
    pub position: Coord10,
}

impl LegacyDuelFoulResult {
    /// Convert to RuleDecision
    pub fn to_decision(&self) -> Option<RuleDecision> {
        if !self.occurred {
            return None;
        }

        let foul_type = if self.penalty_kick {
            FoulType::Penalty
        } else {
            FoulType::Direct
        };

        let card = if self.red_card {
            Some(Card::Red)
        } else if self.yellow_card {
            Some(Card::Yellow)
        } else {
            None
        };

        Some(RuleDecision::Foul {
            offender_idx: self.defender_idx,
            victim_idx: self.attacker_idx,
            foul_type,
            position: self.position.clone(),
            card,
        })
    }
}

/// Duel foul check wrapper with A/B comparison (technique-aware)
///
/// This wrapper is used by duel.rs for integration with the RuleDispatcher.
/// It supports the full defense intent information for accurate foul probability.
///
/// # Arguments
/// * `mode` - Rule check mode
/// * `dispatcher` - RuleDispatcher instance
/// * `legacy_result` - Result from legacy duel foul check
/// * `intent_info` - Defense intent information (from defense_intent system)
/// * `contact` - Contact event
/// * `ball_pos` - Current ball position
/// * `last_touch_team` - Team that last touched the ball
/// * `rng_roll` - Pre-generated RNG roll
///
/// # Returns
/// RuleDecision if foul occurred, None otherwise
pub fn check_duel_foul_wrapper(
    mode: RuleCheckMode,
    dispatcher: &mut RuleDispatcher,
    legacy_result: &LegacyDuelFoulResult,
    intent_info: &DefenseIntentInfo,
    contact: &ContactEvent,
    ball_pos: &Coord10,
    last_touch_team: RuleTeamId,
    rng_roll: f32,
) -> Option<RuleDecision> {
    // Get dispatcher decision using technique-aware evaluation
    let dispatcher_foul = dispatcher.evaluate_contact_with_intent(contact, intent_info, rng_roll);

    // Compare if tracking enabled
    if mode.tracking_enabled() {
        let legacy_decision = legacy_result.to_decision();
        let matched = match (&legacy_decision, &dispatcher_foul) {
            (Some(_), Some(_)) => true,
            (None, None) => true,
            _ => false,
        };

        comparison_stats().record("foul", matched);

        if !matched {
            log::debug!(
                "Duel foul mismatch: legacy={:?}, dispatcher={:?}, technique={:?}",
                legacy_decision.is_some(),
                dispatcher_foul.is_some(),
                intent_info.technique
            );
        }
    }

    // Return based on mode
    if mode.dispatcher_applies() {
        dispatcher_foul
    } else {
        legacy_result.to_decision()
    }
}

// ============================================================================
// FIX_2601/0123 Phase 6: Handball Wrapper
// ============================================================================

/// Legacy handball result
#[derive(Debug, Clone)]
pub struct LegacyHandballResult {
    /// Handball detected
    pub is_handball: bool,
    /// Player who committed handball
    pub player_idx: usize,
    /// Position of handball
    pub position: Coord10,
    /// Whether it was deliberate
    pub deliberate: bool,
}

impl LegacyHandballResult {
    /// Convert to RuleDecision
    pub fn to_decision(&self) -> Option<RuleDecision> {
        if !self.is_handball {
            return None;
        }

        Some(RuleDecision::Handball {
            player_idx: self.player_idx,
            position: self.position.clone(),
            deliberate: self.deliberate,
        })
    }
}

/// Handball check wrapper with A/B comparison
///
/// # Arguments
/// * `mode` - Rule check mode
/// * `dispatcher` - RuleDispatcher instance
/// * `legacy_result` - Result from legacy handball check
/// * `handball_contact` - Handball contact event (if available for detailed evaluation)
/// * `ball_pos` - Current ball position
/// * `last_touch_team` - Team that last touched the ball
/// * `rng_roll` - Pre-generated RNG roll
///
/// # Returns
/// RuleDecision if handball occurred, None otherwise
pub fn check_handball_wrapper(
    mode: RuleCheckMode,
    dispatcher: &mut RuleDispatcher,
    legacy_result: &LegacyHandballResult,
    handball_contact: Option<&HandballContactEvent>,
    ball_pos: &Coord10,
    last_touch_team: RuleTeamId,
    rng_roll: f32,
) -> Option<RuleDecision> {
    // Get dispatcher decision
    let dispatcher_handball = if let Some(contact) = handball_contact {
        dispatcher.evaluate_handball(contact, rng_roll)
    } else {
        // Without detailed contact info, use simple evaluation
        // (ball position check already done by legacy code)
        None
    };

    // Compare if tracking enabled
    if mode.tracking_enabled() {
        let legacy_decision = legacy_result.to_decision();
        let matched = match (&legacy_decision, &dispatcher_handball) {
            (Some(_), Some(_)) => true,
            (None, None) => true,
            _ => false,
        };

        comparison_stats().record("handball", matched);

        if !matched {
            log::debug!(
                "Handball mismatch: legacy={:?}, dispatcher={:?}",
                legacy_decision.is_some(),
                dispatcher_handball.is_some()
            );
        }
    }

    // Return based on mode
    if mode.dispatcher_applies() {
        dispatcher_handball
    } else {
        legacy_result.to_decision()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison_stats_match_rate() {
        let stats = ComparisonStats::new();

        // No comparisons = 100% match rate
        assert!((stats.match_rate() - 1.0).abs() < 0.001);

        // 1 match, 0 mismatch = 100%
        stats.record("goal", true);
        assert!((stats.match_rate() - 1.0).abs() < 0.001);

        // 1 match, 1 mismatch = 50%
        stats.record("goal", false);
        assert!((stats.match_rate() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_legacy_goal_result_to_decision() {
        let result = LegacyGoalResult {
            is_goal: true,
            scoring_team: Some(RuleTeamId::Home),
            scorer_idx: Some(9),
            assister_idx: Some(7),
            position: Coord10::from_meters(0.0, 34.0),
        };

        let decision = result.to_decision();
        assert!(decision.is_some());
        if let Some(RuleDecision::Goal { scorer_idx, assister_idx, .. }) = decision {
            assert_eq!(scorer_idx, 9);
            assert_eq!(assister_idx, Some(7));
        }

        // No goal
        let no_goal = LegacyGoalResult {
            is_goal: false,
            scoring_team: None,
            scorer_idx: None,
            assister_idx: None,
            position: Coord10::from_meters(50.0, 34.0),
        };
        assert!(no_goal.to_decision().is_none());
    }

    #[test]
    fn test_legacy_out_of_play_result_to_decision() {
        let result = LegacyOutOfPlayResult {
            is_out: true,
            last_touch_team: RuleTeamId::Away,
            restart_type: Some(RuleRestartType::ThrowIn),
            position: Coord10::from_meters(50.0, -1.0),
        };

        let decision = result.to_decision();
        assert!(decision.is_some());
        if let Some(RuleDecision::OutOfPlay { restart_type, .. }) = decision {
            assert_eq!(restart_type, RuleRestartType::ThrowIn);
        }
    }

    #[test]
    fn test_rule_context_creation() {
        let ctx = RuleContext::new(
            RuleCheckMode::StatisticsOnly,
            Coord10::from_meters(50.0, 34.0),
            RuleTeamId::Home,
            0.5,
        );

        assert_eq!(ctx.mode, RuleCheckMode::StatisticsOnly);
        assert!(!ctx.mode.dispatcher_applies());
        assert!(ctx.mode.legacy_applies());
    }
}
