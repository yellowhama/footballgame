//! Rule Dispatcher - Centralized Rule Evaluation Hub
//!
//! Based on basketball RE analysis (FIX_2601/0123/01_RULE_DISPATCHER_PATTERN).
//!
//! ## Key Design Principles
//! 1. **Deterministic evaluation order** - Rules are always checked in the same sequence
//! 2. **Team-neutral evaluation** - No home/away bias in rule checking
//! 3. **Position-based sorting** - When multiple events occur, sort by position not team
//!
//! ## Evaluation Order (Fixed, Do Not Change)
//! 1. Goal check (highest priority)
//! 2. Out of play check
//! 3. Offside check (on pass events)
//! 4. Foul check (on contact events)
//! 5. Handball check

use super::types::{
    Card, ContactEvent, ContactType, DefenseIntentInfo, FieldBounds, FoulType,
    HandballContactEvent, PassEvent, RuleDecision, RuleEvaluationStats, RuleRestartType,
    RuleTeamId,
};
use crate::engine::types::Coord10;

/// Configuration for rule evaluation
#[derive(Debug, Clone)]
pub struct RuleConfig {
    /// Base chance for a tackle to be called as a foul
    pub tackle_foul_base_chance: f32,
    /// Additional foul chance for dangerous tackles
    pub dangerous_tackle_modifier: f32,
    /// Additional foul chance for tackles from behind
    pub behind_tackle_modifier: f32,
    /// Severity threshold for yellow card
    pub yellow_card_threshold: f32,
    /// Severity threshold for red card
    pub red_card_threshold: f32,
    /// Enable goal-line technology
    pub goal_line_tech_enabled: bool,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            tackle_foul_base_chance: 0.15,
            dangerous_tackle_modifier: 0.35,
            behind_tackle_modifier: 0.25,
            yellow_card_threshold: 0.6,
            red_card_threshold: 0.85,
            goal_line_tech_enabled: true,
        }
    }
}

/// Centralized rule dispatcher
///
/// This struct coordinates all rule evaluation in a deterministic,
/// team-neutral manner. It replaces the scattered rule checks
/// throughout the codebase with a single evaluation point.
pub struct RuleDispatcher {
    /// Field boundaries
    field_bounds: FieldBounds,
    /// Rule configuration
    config: RuleConfig,
    /// Evaluation statistics (for QA/debugging)
    stats: RuleEvaluationStats,
    /// Current tick (for deterministic RNG seeding)
    current_tick: u64,
}

impl RuleDispatcher {
    /// Create a new rule dispatcher
    pub fn new() -> Self {
        Self {
            field_bounds: FieldBounds::default(),
            config: RuleConfig::default(),
            stats: RuleEvaluationStats::default(),
            current_tick: 0,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: RuleConfig) -> Self {
        Self {
            field_bounds: FieldBounds::default(),
            config,
            stats: RuleEvaluationStats::default(),
            current_tick: 0,
        }
    }

    /// Get evaluation statistics
    pub fn stats(&self) -> &RuleEvaluationStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = RuleEvaluationStats::default();
    }

    /// Set current tick for deterministic seeding
    pub fn set_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Main evaluation entry point
    ///
    /// Evaluates all rules in deterministic order and returns decisions.
    /// The order is: Goal > OutOfPlay > Offside > Foul > Handball
    ///
    /// # Arguments
    /// * `ball_pos` - Current ball position
    /// * `ball_in_goal` - Whether ball has crossed goal line
    /// * `last_touch_team` - Team that last touched the ball
    /// * `scorer_idx` - Player who scored (if goal)
    /// * `assister_idx` - Player who assisted (if goal)
    /// * `pass_event` - Pending pass event (if any)
    /// * `contacts` - Contact events this tick
    /// * `rng_roll` - Pre-generated RNG roll for determinism
    pub fn evaluate_tick(
        &mut self,
        ball_pos: &Coord10,
        ball_in_goal: Option<RuleTeamId>,
        last_touch_team: RuleTeamId,
        scorer_idx: Option<usize>,
        assister_idx: Option<usize>,
        pass_event: Option<&PassEvent>,
        offside_detected: bool,
        contacts: &[ContactEvent],
        rng_roll: f32,
    ) -> Vec<RuleDecision> {
        self.stats.ticks_evaluated += 1;
        let mut decisions = Vec::new();

        // 1. Goal check (highest priority)
        if let Some(scoring_team) = ball_in_goal {
            if let Some(scorer) = scorer_idx {
                let decision = RuleDecision::Goal {
                    scorer_idx: scorer,
                    assister_idx,
                    position: ball_pos.clone(),
                };
                self.record_decision(&decision);
                decisions.push(decision);
                return decisions; // Goal stops all other checks
            }
        }

        // 2. Out of play check
        if self.field_bounds.is_out_of_bounds(ball_pos) {
            if let Some(restart_type) =
                self.field_bounds.out_of_bounds_restart(ball_pos, last_touch_team)
            {
                let decision = RuleDecision::OutOfPlay {
                    last_touch_team,
                    position: ball_pos.clone(),
                    restart_type,
                };
                self.record_decision(&decision);
                decisions.push(decision);
                return decisions; // Out of play stops other checks
            }
        }

        // 3. Offside check (on pass events)
        if let Some(pass) = pass_event {
            if offside_detected {
                let decision = RuleDecision::Offside {
                    player_idx: pass.receiver_idx,
                    position: pass.target.clone(),
                    pass_origin: pass.origin.clone(),
                };
                self.record_decision(&decision);
                decisions.push(decision);
                return decisions;
            }
        }

        // 4. Foul check (on contacts)
        // Sort contacts by position for deterministic order (NOT by team)
        let mut sorted_contacts: Vec<_> = contacts.iter().collect();
        sorted_contacts.sort_by(|a, b| {
            let (ax, ay) = a.position.to_meters();
            let (bx, by) = b.position.to_meters();
            ax.partial_cmp(&bx)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| ay.partial_cmp(&by).unwrap_or(std::cmp::Ordering::Equal))
        });

        for contact in sorted_contacts {
            if let Some(foul_decision) = self.evaluate_contact(contact, rng_roll) {
                self.record_decision(&foul_decision);
                decisions.push(foul_decision);
            }
        }

        // If no decisions, return Continue
        if decisions.is_empty() {
            decisions.push(RuleDecision::Continue);
        }

        decisions
    }

    /// Evaluate a single contact event for foul
    fn evaluate_contact(&self, contact: &ContactEvent, rng_roll: f32) -> Option<RuleDecision> {
        let contact_type = contact.contact_type();

        // Calculate foul probability
        let foul_chance = self.calculate_foul_chance(&contact_type, contact.intensity);

        // Check if foul occurs
        if rng_roll >= foul_chance {
            return None; // No foul
        }

        // Determine foul type
        let foul_type = self.determine_foul_type(contact);

        // Determine card
        let card = self.determine_card(&contact_type, contact.intensity);

        Some(RuleDecision::Foul {
            offender_idx: contact.tackler_idx,
            victim_idx: contact.ball_carrier_idx,
            foul_type,
            position: contact.position.clone(),
            card,
        })
    }

    /// Calculate probability of foul call
    fn calculate_foul_chance(&self, contact_type: &ContactType, intensity: f32) -> f32 {
        let mut chance = self.config.tackle_foul_base_chance;

        match contact_type {
            ContactType::BallWon => {
                // Ball won first reduces foul chance significantly
                chance *= 0.2;
            }
            ContactType::Clean => {
                // Standard chance
            }
            ContactType::FromBehind => {
                chance += self.config.behind_tackle_modifier;
            }
            ContactType::Dangerous => {
                chance += self.config.dangerous_tackle_modifier;
            }
        }

        // Intensity affects foul chance
        chance *= 0.5 + intensity;

        chance.clamp(0.0, 0.95)
    }

    /// Determine foul type based on position and circumstances
    fn determine_foul_type(&self, contact: &ContactEvent) -> FoulType {
        // Check if in penalty area
        if let Some(defending_team) = self.field_bounds.is_in_penalty_area(&contact.position) {
            // Foul by defender in their own penalty area
            let tackler_team = RuleTeamId::from_player_index(contact.tackler_idx);
            if tackler_team == defending_team {
                return FoulType::Penalty;
            }
        }

        // Check for dangerous play
        if contact.contact_type() == ContactType::Dangerous {
            return FoulType::SeriousFoulPlay;
        }

        FoulType::Direct
    }

    /// Determine if card should be given
    fn determine_card(&self, contact_type: &ContactType, intensity: f32) -> Option<Card> {
        let severity = match contact_type {
            ContactType::Dangerous => 0.7 + intensity * 0.3,
            ContactType::FromBehind => 0.4 + intensity * 0.4,
            _ => intensity * 0.5,
        };

        if severity >= self.config.red_card_threshold {
            Some(Card::Red)
        } else if severity >= self.config.yellow_card_threshold {
            Some(Card::Yellow)
        } else {
            None
        }
    }

    // ========================================================================
    // FIX_2601/0123 Phase 6: Technique-Aware Foul Evaluation
    // ========================================================================

    /// Evaluate contact with defense intent information (technique-aware)
    ///
    /// This is the preferred method when defense intent info is available,
    /// providing more accurate foul probability based on technique type.
    pub fn evaluate_contact_with_intent(
        &self,
        contact: &ContactEvent,
        intent_info: &DefenseIntentInfo,
        rng_roll: f32,
    ) -> Option<RuleDecision> {
        // Calculate foul probability using technique-aware calculation
        let foul_chance = self.calculate_technique_foul_chance(intent_info, contact.intensity);

        // Check if foul occurs
        if rng_roll >= foul_chance {
            return None; // No foul
        }

        // Determine foul type based on position
        let foul_type = self.determine_foul_type(contact);

        // Determine card using technique-aware calculation
        let card = self.determine_card_with_intent(intent_info, contact);

        Some(RuleDecision::Foul {
            offender_idx: contact.tackler_idx,
            victim_idx: contact.ball_carrier_idx,
            foul_type,
            position: contact.position.clone(),
            card,
        })
    }

    /// Calculate foul probability based on technique and defender attributes
    ///
    /// Formula:
    /// base_prob * outcome_mod * skill_mod * aggression_mod
    ///
    /// Where:
    /// - base_prob: technique-specific base probability
    /// - outcome_mod: 0.5 if defense won, 1.5 if attack won
    /// - skill_mod: 1.0 - (tackling / 200.0) reduces foul chance for skilled players
    /// - aggression_mod: 1.0 + (aggression / 100.0) * 0.3 increases for aggressive players
    fn calculate_technique_foul_chance(&self, intent_info: &DefenseIntentInfo, intensity: f32) -> f32 {
        let base_prob = intent_info.technique.base_foul_probability();

        // Outcome modifier: winning the ball reduces foul chance
        let outcome_mod = if intent_info.defense_won { 0.5 } else { 1.5 };

        // Skill modifier: better tackling = fewer fouls
        let skill_mod = 1.0 - (intent_info.defender_tackling / 200.0);

        // Aggression modifier: more aggressive = more fouls
        let aggression_mod = 1.0 + (intent_info.defender_aggression / 100.0) * 0.3;

        // Intensity affects foul chance
        let intensity_mod = 0.5 + intensity;

        (base_prob * outcome_mod * skill_mod * aggression_mod * intensity_mod).clamp(0.0, 0.95)
    }

    /// Determine card using technique-aware evaluation
    fn determine_card_with_intent(
        &self,
        intent_info: &DefenseIntentInfo,
        contact: &ContactEvent,
    ) -> Option<Card> {
        let mut yellow_prob = intent_info.technique.base_yellow_probability();
        let mut red_prob = intent_info.technique.base_red_probability();

        // Context modifiers
        let contact_type = contact.contact_type();

        // From behind increases card probability
        if matches!(contact_type, ContactType::FromBehind) {
            yellow_prob += 0.20;
            red_prob += 0.10;
        }

        // Dangerous play (high intensity)
        if matches!(contact_type, ContactType::Dangerous) {
            yellow_prob += 0.30;
            red_prob += 0.15;
        }

        // In penalty area
        if let Some(_) = self.field_bounds.is_in_penalty_area(&contact.position) {
            yellow_prob += 0.10;
        }

        // High aggression
        if intent_info.defender_aggression > 85.0 {
            yellow_prob += 0.10;
            red_prob += 0.05;
        }

        // Use intensity as additional modifier
        let severity = contact.intensity * 0.3;
        yellow_prob += severity;
        red_prob += severity * 0.3;

        // Determine card based on thresholds
        if red_prob >= self.config.red_card_threshold {
            Some(Card::Red)
        } else if yellow_prob >= self.config.yellow_card_threshold {
            Some(Card::Yellow)
        } else {
            None
        }
    }

    // ========================================================================
    // FIX_2601/0123 Phase 6: Handball Evaluation
    // ========================================================================

    /// Evaluate handball contact event
    pub fn evaluate_handball(
        &self,
        handball: &HandballContactEvent,
        rng_roll: f32,
    ) -> Option<RuleDecision> {
        // Calculate handball probability
        let handball_prob = handball.deliberate_probability();

        // Check if handball is called
        if rng_roll >= handball_prob {
            return None;
        }

        let deliberate = handball.deliberate_movement || handball.unnatural_position;

        Some(RuleDecision::Handball {
            player_idx: handball.player_idx,
            position: handball.position.clone(),
            deliberate,
        })
    }

    /// Record decision for statistics
    fn record_decision(&mut self, decision: &RuleDecision) {
        match decision {
            RuleDecision::Goal { scorer_idx, .. } => {
                self.stats.goals_detected += 1;
                if RuleTeamId::from_player_index(*scorer_idx).is_home() {
                    self.stats.home_decisions += 1;
                } else {
                    self.stats.away_decisions += 1;
                }
            }
            RuleDecision::OutOfPlay { last_touch_team, .. } => {
                self.stats.out_of_play_detected += 1;
                // Benefiting team gets the decision count
                if last_touch_team.opponent().is_home() {
                    self.stats.home_decisions += 1;
                } else {
                    self.stats.away_decisions += 1;
                }
            }
            RuleDecision::Offside { player_idx, .. } => {
                self.stats.offsides_detected += 1;
                // Defending team benefits
                if !RuleTeamId::from_player_index(*player_idx).is_home() {
                    self.stats.home_decisions += 1;
                } else {
                    self.stats.away_decisions += 1;
                }
            }
            RuleDecision::Foul { victim_idx, .. } => {
                self.stats.fouls_detected += 1;
                // Victim team benefits
                if RuleTeamId::from_player_index(*victim_idx).is_home() {
                    self.stats.home_decisions += 1;
                } else {
                    self.stats.away_decisions += 1;
                }
            }
            RuleDecision::Handball { player_idx, .. } => {
                self.stats.handballs_detected += 1;
                if !RuleTeamId::from_player_index(*player_idx).is_home() {
                    self.stats.home_decisions += 1;
                } else {
                    self.stats.away_decisions += 1;
                }
            }
            RuleDecision::Continue => {}
        }
    }

    /// Convert decision to restart type (for state machine integration)
    pub fn decision_to_restart(&self, decision: &RuleDecision) -> Option<RuleRestartType> {
        match decision {
            RuleDecision::Continue => None,
            RuleDecision::Goal { .. } => Some(RuleRestartType::Kickoff),
            RuleDecision::OutOfPlay { restart_type, .. } => Some(*restart_type),
            RuleDecision::Offside { .. } => Some(RuleRestartType::IndirectFreeKick),
            RuleDecision::Foul { foul_type, .. } => Some(foul_type.restart_type()),
            RuleDecision::Handball { position, .. } => {
                // Penalty if in penalty area, otherwise direct FK
                if self.field_bounds.is_in_penalty_area(position).is_some() {
                    Some(RuleRestartType::PenaltyKick)
                } else {
                    Some(RuleRestartType::DirectFreeKick)
                }
            }
        }
    }
}

impl Default for RuleDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher_creation() {
        let dispatcher = RuleDispatcher::new();
        assert_eq!(dispatcher.stats().ticks_evaluated, 0);
    }

    #[test]
    fn test_evaluation_order_goal_first() {
        let mut dispatcher = RuleDispatcher::new();
        let ball_pos = Coord10::from_meters(-1.0, 34.0); // Past goal line

        let decisions = dispatcher.evaluate_tick(
            &ball_pos,
            Some(RuleTeamId::Home), // Goal for home
            RuleTeamId::Home,
            Some(9), // Striker scored
            Some(7), // Midfielder assisted
            None,
            false,
            &[],
            0.5,
        );

        assert_eq!(decisions.len(), 1);
        assert!(matches!(decisions[0], RuleDecision::Goal { .. }));
    }

    #[test]
    fn test_evaluation_order_out_of_play_before_foul() {
        let mut dispatcher = RuleDispatcher::new();
        let ball_pos = Coord10::from_meters(50.0, -1.0); // Out on touchline

        let contact = ContactEvent {
            tackler_idx: 3,
            ball_carrier_idx: 14,
            position: Coord10::from_meters(50.0, 1.0),
            contact_angle: 0.0,
            ball_won_first: false,
            intensity: 0.7,
        };

        let decisions = dispatcher.evaluate_tick(
            &ball_pos,
            None,
            RuleTeamId::Away,
            None,
            None,
            None,
            false,
            &[contact],
            0.1, // Would be a foul
        );

        // Out of play should take precedence
        assert_eq!(decisions.len(), 1);
        assert!(matches!(decisions[0], RuleDecision::OutOfPlay { .. }));
    }

    #[test]
    fn test_contact_sorting_by_position() {
        let mut dispatcher = RuleDispatcher::new();
        let ball_pos = Coord10::from_meters(50.0, 34.0);

        // Create contacts at different positions
        let contact1 = ContactEvent {
            tackler_idx: 11, // Away team first in index
            ball_carrier_idx: 5,
            position: Coord10::from_meters(60.0, 30.0), // Further right
            contact_angle: 0.0,
            ball_won_first: false,
            intensity: 0.6,
        };

        let contact2 = ContactEvent {
            tackler_idx: 3, // Home team
            ball_carrier_idx: 15,
            position: Coord10::from_meters(40.0, 30.0), // Further left
            contact_angle: 0.0,
            ball_won_first: false,
            intensity: 0.6,
        };

        // Contacts given in "wrong" order (away first)
        let decisions = dispatcher.evaluate_tick(
            &ball_pos,
            None,
            RuleTeamId::Home,
            None,
            None,
            None,
            false,
            &[contact1, contact2],
            0.05, // Low roll = both fouls
        );

        // Should have evaluated contact2 (x=40) before contact1 (x=60)
        // because sorting is by position, not by team
        if decisions.len() >= 2 {
            if let (RuleDecision::Foul { offender_idx: idx1, .. }, RuleDecision::Foul { offender_idx: idx2, .. }) =
                (&decisions[0], &decisions[1])
            {
                // First foul should be at x=40 (offender=3)
                assert_eq!(*idx1, 3, "First foul should be from home player at x=40");
                assert_eq!(*idx2, 11, "Second foul should be from away player at x=60");
            }
        }
    }

    #[test]
    fn test_penalty_area_foul() {
        let mut dispatcher = RuleDispatcher::new();
        let ball_pos = Coord10::from_meters(8.0, 34.0); // In home penalty area

        let contact = ContactEvent {
            tackler_idx: 2, // Home defender
            ball_carrier_idx: 19, // Away attacker
            position: Coord10::from_meters(8.0, 34.0),
            contact_angle: 0.0,
            ball_won_first: false,
            intensity: 0.5,
        };

        let decisions = dispatcher.evaluate_tick(
            &ball_pos,
            None,
            RuleTeamId::Away,
            None,
            None,
            None,
            false,
            &[contact],
            0.05,
        );

        assert_eq!(decisions.len(), 1);
        if let RuleDecision::Foul { foul_type, .. } = &decisions[0] {
            assert_eq!(*foul_type, FoulType::Penalty);
        } else {
            panic!("Expected Foul decision");
        }
    }

    #[test]
    fn test_stats_tracking() {
        let mut dispatcher = RuleDispatcher::new();
        let ball_pos = Coord10::from_meters(50.0, 34.0);

        // Simulate several ticks
        for _ in 0..10 {
            dispatcher.evaluate_tick(
                &ball_pos,
                None,
                RuleTeamId::Home,
                None,
                None,
                None,
                false,
                &[],
                0.5,
            );
        }

        assert_eq!(dispatcher.stats().ticks_evaluated, 10);
    }

    #[test]
    fn test_decision_to_restart() {
        let dispatcher = RuleDispatcher::new();

        assert_eq!(dispatcher.decision_to_restart(&RuleDecision::Continue), None);

        let goal = RuleDecision::Goal {
            scorer_idx: 9,
            assister_idx: None,
            position: Coord10::from_meters(0.0, 34.0),
        };
        assert_eq!(
            dispatcher.decision_to_restart(&goal),
            Some(RuleRestartType::Kickoff)
        );

        let offside = RuleDecision::Offside {
            player_idx: 9,
            position: Coord10::from_meters(80.0, 34.0),
            pass_origin: Coord10::from_meters(70.0, 34.0),
        };
        assert_eq!(
            dispatcher.decision_to_restart(&offside),
            Some(RuleRestartType::IndirectFreeKick)
        );
    }
}
