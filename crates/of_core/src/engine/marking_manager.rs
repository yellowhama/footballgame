use crate::engine::RestartType;
use crate::engine::TransitionState;
use crate::engine::physics_constants::field;
use crate::models::TeamSide;
/// marking_manager.rs
/// Phase 1.3: MarkingManager - Complete Implementation
///
/// Purpose: Assign defensive marks, prevent mob behavior
///
/// Core Systems:
/// 1. MarkState per defender (primary_mark_id, zone_role, mode)
/// 2. 6 Reassignment Triggers (T1-T6)
/// 3. Budget enforcement (press/cover)
/// 4. Cooldown management (prevents jitter)
use serde::{Deserialize, Serialize};

/// ============================================================================
/// MarkState - Per-Defender State
/// ============================================================================

/// Defensive marking state for a single defender
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MarkState {
    /// Which attacker to mark (track_id 0-21, or -1 if none)
    pub primary_mark_id: i8,

    /// Zone responsibility
    pub zone_role: ZoneRole,

    /// Marking tightness
    pub mode: MarkMode,

    /// Last reassignment tick (for cooldown)
    pub last_reassign_tick: u64,

    /// Emergency presser flag (free_score trigger)
    pub is_emergency_presser: bool,

    /// Cover role flag (supports presser)
    pub is_cover: bool,

    /// T3: Mark broken duration tracking (ticks)
    pub broken_duration_ticks: u8,
}

impl Default for MarkState {
    fn default() -> Self {
        Self {
            primary_mark_id: -1,
            zone_role: ZoneRole::HomeZone,
            mode: MarkMode::Normal,
            last_reassign_tick: 0,
            is_emergency_presser: false,
            is_cover: false,
            broken_duration_ticks: 0,
        }
    }
}

/// Zone responsibility types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneRole {
    /// Protect home zone (formation position)
    HomeZone,
    /// Guard passing lane
    LaneGuard,
    /// Protect penalty box
    BoxGuard,
}

/// Marking tightness modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkMode {
    /// Tight marking (<2m)
    Tight,
    /// Normal marking (2-5m)
    Normal,
    /// Loose marking (5-10m, zone-based)
    Loose,
}

/// ============================================================================
/// RoleBudget - Press/Cover Budget Enforcement
/// ============================================================================

/// Budget for press/cover roles (prevents mob ball)
#[derive(Debug, Clone, Copy)]
pub struct RoleBudget {
    /// Base press budget (0-2 depending on tactic)
    pub press_budget: u8,

    /// Base cover budget (1-2 depending on tactic)
    pub cover_budget: u8,

    /// Temporary emergency escalation
    pub press_budget_temp: u8,
}

impl RoleBudget {
    /// High Press tactic (2 pressers, 1 cover)
    pub fn high_press() -> Self {
        Self {
            press_budget: 2,
            cover_budget: 1,
            press_budget_temp: 2, // Already at max
        }
    }

    /// Balanced tactic (1 presser, 1 cover)
    pub fn balanced() -> Self {
        Self { press_budget: 1, cover_budget: 1, press_budget_temp: 1 }
    }

    /// Low Block tactic (0-1 presser, 2 covers)
    pub fn low_block() -> Self {
        Self { press_budget: 1, cover_budget: 2, press_budget_temp: 1 }
    }

    /// Reset temp budgets back to their base values.
    pub fn reset_temp_budgets(&mut self) {
        self.press_budget_temp = self.press_budget;
    }

    /// Apply a temporary press-budget bonus (clamped to max 2).
    pub fn apply_press_bonus(&mut self, bonus: u8) {
        self.press_budget_temp = self.press_budget.saturating_add(bonus).min(2);
    }

    /// Get effective press budget (includes temp)
    pub fn effective_press_budget(&self) -> u8 {
        self.press_budget_temp
    }
}

/// ============================================================================
/// ReassignTrigger - 6 Triggers (ONLY THESE)
/// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReassignTrigger {
    /// T1: Kickoff/restart (full reset)
    KickoffRestart,

    /// T2: Possession change (transition start)
    PossessionChange,

    /// T3: Mark broken (distance + angle + duration)
    MarkBroken,

    /// T4: Rotation detected (zone swap)
    RotationDetected,

    /// T5: Ball switch (cross-field >25m)
    BallSwitch,

    /// T6: Emergency free carrier (free_score >= threshold, IGNORES COOLDOWN)
    EmergencyFreeCarrier,
}

impl ReassignTrigger {
    /// Check if this trigger ignores cooldown
    pub fn ignores_cooldown(&self) -> bool {
        matches!(self, ReassignTrigger::EmergencyFreeCarrier)
    }
}

/// ============================================================================
/// MarkingManager - Main System
/// ============================================================================

/// Marking manager for one team (11 defenders)
#[derive(Debug, Clone)]
pub struct MarkingManager {
    /// Per-defender mark states (11 players, index 0 = GK)
    pub states: [MarkState; 11],

    /// Role budget (tactic-dependent)
    pub budget: RoleBudget,

    /// Reassignment cooldown (ticks)
    pub reassign_cooldown_ticks: u64,

    /// Last ball position (for cross-field detection)
    pub last_ball_pos: (f32, f32),

    /// T4: Rotation detection state (previous tick attacker zones)
    pub previous_attacker_zones: [(u8, u8); 11],

    /// T4: Rotation detection state (previous tick attacker positions)
    pub previous_attacker_positions: [(f32, f32); 11],
}

impl MarkingManager {
    /// Create new manager with balanced tactic
    pub fn new() -> Self {
        Self {
            states: [MarkState::default(); 11],
            budget: RoleBudget::balanced(),
            reassign_cooldown_ticks: 8, // 8 ticks = 2 seconds
            last_ball_pos: (field::CENTER_X, field::CENTER_Y),
            previous_attacker_zones: [(0, 0); 11],
            previous_attacker_positions: [(0.0, 0.0); 11],
        }
    }

    /// Set tactic
    pub fn set_tactic(&mut self, tactic: &str) {
        self.budget = match tactic {
            "high_press" => RoleBudget::high_press(),
            "low_block" => RoleBudget::low_block(),
            _ => RoleBudget::balanced(),
        };
    }

    /// Update marking assignments
    ///
    /// **Parameters**:
    /// - `current_tick`: Current game tick
    /// - `ball_pos`: Current ball position (meters)
    /// - `carrier_id`: Ball carrier track_id (0-21, or None if loose ball)
    /// - `defender_positions`: Defender positions (11 players)
    /// - `attacker_positions`: Attacker positions (11 players)
    /// - `free_score`: CarrierFreeScore (0.0-1.0)
    /// - `emergency_threshold`: Threshold for T6 trigger
    /// - `possession_changed`: true if possession just changed
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        current_tick: u64,
        ball_pos: (f32, f32),
        carrier_id: Option<u8>,
        defender_positions: &[(f32, f32); 11],
        attacker_positions: &[(f32, f32); 11],
        free_score: f32,
        emergency_threshold: f32,
        possession_changed: bool,
        defending_team_side: TeamSide,
        transition_state: TransitionState,
        restart_occurred: bool,
        restart_type: Option<RestartType>,
    ) {
        // Phase 1.4: TransitionSystem integration
        // - budgets reset each tick, then bonuses applied based on transition/emergency
        // - marking loosened while the team that lost the ball is in transition
        self.budget.reset_temp_budgets();
        let is_transition_defense = matches!(
            transition_state,
            TransitionState::Active { team_lost_ball, .. } if team_lost_ball == defending_team_side
        );

        let mut press_bonus: u8 = 0;
        if is_transition_defense {
            press_bonus = press_bonus.saturating_add(1);
        }
        if free_score >= emergency_threshold {
            press_bonus = press_bonus.saturating_add(1);
        }
        self.budget.apply_press_bonus(press_bonus);

        if is_transition_defense {
            for state in self.states.iter_mut().skip(1) {
                if state.mode != MarkMode::Tight {
                    state.mode = MarkMode::Loose;
                }
            }
        } else {
            // Prevent permanent "transition looseness" once the window ends.
            for state in self.states.iter_mut().skip(1) {
                if state.mode == MarkMode::Loose {
                    state.mode = MarkMode::Normal;
                }
            }
        }

        // T3: Update broken mark durations every tick
        self.update_broken_durations(defender_positions, attacker_positions, ball_pos);

        // T4: Rotation detection baseline (computed every tick)
        let current_attacker_zones: [(u8, u8); 11] =
            std::array::from_fn(|i| calculate_zone(attacker_positions[i]));
        let rotation_detected = detect_rotation(
            &current_attacker_zones,
            &self.previous_attacker_zones,
            attacker_positions,
            &self.previous_attacker_positions,
        );

        // Detect triggers
        let mut triggers: Vec<ReassignTrigger> = Vec::new();

        // T1: Kickoff/restart (full reset)
        if restart_occurred {
            triggers.push(ReassignTrigger::KickoffRestart);
        }

        // T2: Possession change
        if possession_changed {
            triggers.push(ReassignTrigger::PossessionChange);
        }

        // T5: Ball switch (cross-field >25m)
        let ball_movement = distance(ball_pos, self.last_ball_pos);
        if ball_movement > 25.0 {
            triggers.push(ReassignTrigger::BallSwitch);
        }

        // T3: Mark broken (distance + angle + duration)
        if self.states.iter().skip(1).any(|s| s.broken_duration_ticks >= MARK_BROKEN_TRIGGER_TICKS)
        {
            triggers.push(ReassignTrigger::MarkBroken);
        }

        // T4: Rotation detected (zone swap)
        if rotation_detected {
            triggers.push(ReassignTrigger::RotationDetected);
        }

        // T6: Emergency free carrier
        if free_score >= emergency_threshold {
            triggers.push(ReassignTrigger::EmergencyFreeCarrier);
        }

        // Execute triggers
        for trigger in triggers {
            if self.can_reassign(current_tick, trigger) {
                self.execute_trigger(
                    trigger,
                    current_tick,
                    ball_pos,
                    carrier_id,
                    defender_positions,
                    attacker_positions,
                    restart_type,
                );
            }
        }

        // Enforce budget
        self.enforce_budget(ball_pos, defender_positions);

        // Update last ball position
        self.last_ball_pos = ball_pos;

        // Update rotation detection baseline for next tick
        self.previous_attacker_zones = current_attacker_zones;
        self.previous_attacker_positions = *attacker_positions;
    }

    /// Check if reassignment allowed (cooldown check)
    fn can_reassign(&self, current_tick: u64, trigger: ReassignTrigger) -> bool {
        if trigger.ignores_cooldown() {
            return true; // T6 always allowed
        }

        // Check if any defender is within cooldown
        for state in &self.states {
            if current_tick < state.last_reassign_tick + self.reassign_cooldown_ticks {
                return false; // Cooldown active, block reassignment
            }
        }

        true
    }

    /// Execute reassignment trigger
    #[allow(clippy::too_many_arguments)]
    fn execute_trigger(
        &mut self,
        trigger: ReassignTrigger,
        current_tick: u64,
        ball_pos: (f32, f32),
        carrier_id: Option<u8>,
        defender_positions: &[(f32, f32); 11],
        attacker_positions: &[(f32, f32); 11],
        restart_type: Option<RestartType>,
    ) {
        match trigger {
            ReassignTrigger::KickoffRestart => {
                self.execute_kickoff_restart(
                    current_tick,
                    ball_pos,
                    restart_type,
                    defender_positions,
                    attacker_positions,
                );
            }
            ReassignTrigger::EmergencyFreeCarrier => {
                self.execute_emergency_presser(
                    current_tick,
                    ball_pos,
                    carrier_id,
                    defender_positions,
                );
            }
            ReassignTrigger::PossessionChange => {
                self.execute_possession_change(
                    current_tick,
                    defender_positions,
                    attacker_positions,
                );
            }
            ReassignTrigger::MarkBroken => {
                self.execute_mark_broken(current_tick, defender_positions, attacker_positions);
            }
            ReassignTrigger::RotationDetected => {
                self.execute_rotation_detected(
                    current_tick,
                    defender_positions,
                    attacker_positions,
                );
            }
            ReassignTrigger::BallSwitch => {
                self.execute_ball_switch(
                    current_tick,
                    ball_pos,
                    defender_positions,
                    attacker_positions,
                );
            }
        }
    }

    /// Reset all marks to defaults (T1)
    fn reset_all_marks(&mut self, current_tick: u64) {
        for state in &mut self.states {
            state.primary_mark_id = -1;
            state.zone_role = ZoneRole::HomeZone;
            state.mode = MarkMode::Normal;
            state.last_reassign_tick = current_tick;
            state.is_emergency_presser = false;
            state.is_cover = false;
            state.broken_duration_ticks = 0;
        }
    }

    /// T1: Kickoff/restart (full reset)
    fn execute_kickoff_restart(
        &mut self,
        current_tick: u64,
        ball_pos: (f32, f32),
        restart_type: Option<RestartType>,
        defender_positions: &[(f32, f32); 11],
        attacker_positions: &[(f32, f32); 11],
    ) {
        self.reset_all_marks(current_tick);

        let is_set_piece_restart = matches!(
            restart_type,
            Some(RestartType::Corner | RestartType::FreeKick | RestartType::Penalty)
        );

        let set_piece_radius_m: f32 = 25.0;
        let mut assigned_attackers = [false; 11];

        if is_set_piece_restart {
            // Defender processing order: bias set pieces toward the ball cluster to reduce corner-box chaos.
            let mut defender_indices: [usize; 10] = std::array::from_fn(|i| i + 1); // Skip GK
            defender_indices.sort_by(|&a, &b| {
                let da = distance(defender_positions[a], ball_pos);
                let db = distance(defender_positions[b], ball_pos);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });

            // Prefer attackers near the ball first, but always guarantee a mark if possible.
            let mut attacker_indices: [usize; 11] = std::array::from_fn(|i| i);
            attacker_indices.sort_by(|&a, &b| {
                let da = distance(attacker_positions[a], ball_pos);
                let db = distance(attacker_positions[b], ball_pos);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });

            for def_idx in defender_indices {
                let def_pos = defender_positions[def_idx];

                // First pass: only consider attackers near the ball.
                let mut best: Option<usize> = None;
                let mut best_dist = f32::INFINITY;
                for &att_idx in &attacker_indices {
                    if assigned_attackers[att_idx] {
                        continue;
                    }
                    if distance(attacker_positions[att_idx], ball_pos) > set_piece_radius_m {
                        continue;
                    }
                    let d = distance(def_pos, attacker_positions[att_idx]);
                    if d < best_dist {
                        best = Some(att_idx);
                        best_dist = d;
                    }
                }

                // Fallback: any nearest unassigned attacker.
                if best.is_none() {
                    for &att_idx in &attacker_indices {
                        if assigned_attackers[att_idx] {
                            continue;
                        }
                        let d = distance(def_pos, attacker_positions[att_idx]);
                        if d < best_dist {
                            best = Some(att_idx);
                            best_dist = d;
                        }
                    }
                }

                if let Some(att_idx) = best {
                    assigned_attackers[att_idx] = true;
                    self.states[def_idx].primary_mark_id = att_idx as i8;
                    self.states[def_idx].mode = MarkMode::Normal;
                    self.states[def_idx].last_reassign_tick = current_tick;
                }
            }
        } else {
            // Default restart: one-to-one greedy proximity assignment (no heap allocations).
            for def_idx in 1..11 {
                let def_pos = defender_positions[def_idx];
                let mut best: Option<usize> = None;
                let mut best_dist = f32::INFINITY;

                for att_idx in 0..11 {
                    if assigned_attackers[att_idx] {
                        continue;
                    }
                    let d = distance(def_pos, attacker_positions[att_idx]);
                    if d < best_dist {
                        best = Some(att_idx);
                        best_dist = d;
                    }
                }

                if let Some(att_idx) = best {
                    assigned_attackers[att_idx] = true;
                    self.states[def_idx].primary_mark_id = att_idx as i8;
                    self.states[def_idx].mode = MarkMode::Normal;
                    self.states[def_idx].last_reassign_tick = current_tick;
                }
            }
        }
    }

    /// T3: Mark broken (distance + angle + duration)
    fn execute_mark_broken(
        &mut self,
        current_tick: u64,
        defender_positions: &[(f32, f32); 11],
        attacker_positions: &[(f32, f32); 11],
    ) {
        let mut broken_defenders: Vec<usize> = (1..11)
            .filter(|&i| self.states[i].broken_duration_ticks >= MARK_BROKEN_TRIGGER_TICKS)
            .collect();

        if broken_defenders.is_empty() {
            return;
        }

        // Prioritize the longest-broken marks, cap total changes per tick
        broken_defenders.sort_by(|&a, &b| {
            self.states[b].broken_duration_ticks.cmp(&self.states[a].broken_duration_ticks)
        });
        broken_defenders.truncate(MARK_BROKEN_MAX_REASSIGNMENTS);

        // Count current assignments (excluding defenders we are about to reassign)
        let mut mark_counts = [0u8; 11];
        for (def_idx, state) in self.states.iter().enumerate() {
            if broken_defenders.contains(&def_idx) {
                continue;
            }
            if state.primary_mark_id >= 0 {
                let idx = state.primary_mark_id as usize;
                if idx < 11 {
                    mark_counts[idx] = mark_counts[idx].saturating_add(1);
                }
            }
        }

        for def_idx in broken_defenders {
            let def_pos = defender_positions[def_idx];

            // Prefer unmarked attackers first
            let mut best: Option<usize> = None;
            let mut best_dist = f32::INFINITY;
            for (att_idx, att_pos) in attacker_positions.iter().enumerate() {
                if mark_counts[att_idx] != 0 {
                    continue;
                }
                let d = distance(def_pos, *att_pos);
                if d < best_dist {
                    best = Some(att_idx);
                    best_dist = d;
                }
            }

            // Fallback: any nearest attacker
            if best.is_none() {
                for (att_idx, att_pos) in attacker_positions.iter().enumerate() {
                    let d = distance(def_pos, *att_pos);
                    if d < best_dist {
                        best = Some(att_idx);
                        best_dist = d;
                    }
                }
            }

            if let Some(att_idx) = best {
                self.states[def_idx].primary_mark_id = att_idx as i8;
                self.states[def_idx].mode = MarkMode::Tight; // Tighten to prevent immediate re-break
                self.states[def_idx].broken_duration_ticks = 0;
                self.states[def_idx].last_reassign_tick = current_tick;
                mark_counts[att_idx] = mark_counts[att_idx].saturating_add(1);
            }
        }
    }

    /// T4: Rotation detected (zone swap)
    fn execute_rotation_detected(
        &mut self,
        current_tick: u64,
        defender_positions: &[(f32, f32); 11],
        attacker_positions: &[(f32, f32); 11],
    ) {
        let attacker_zones: [(u8, u8); 11] =
            std::array::from_fn(|i| calculate_zone(attacker_positions[i]));

        for def_idx in 1..11 {
            // Skip GK
            let def_pos = defender_positions[def_idx];
            let def_zone = calculate_zone(def_pos);

            // Prefer attackers in same/adjacent zone
            let mut best: Option<usize> = None;
            let mut best_dist = f32::INFINITY;
            for (att_idx, att_pos) in attacker_positions.iter().enumerate() {
                if !zones_adjacent_or_same(def_zone, attacker_zones[att_idx]) {
                    continue;
                }
                let d = distance(def_pos, *att_pos);
                if d < best_dist {
                    best = Some(att_idx);
                    best_dist = d;
                }
            }

            // Fallback: nearest attacker overall
            if best.is_none() {
                best = find_nearest_attacker(def_pos, attacker_positions, 0..11);
            }

            if let Some(att_idx) = best {
                self.states[def_idx].primary_mark_id = att_idx as i8;
                self.states[def_idx].mode = MarkMode::Normal;
                self.states[def_idx].broken_duration_ticks = 0;
                self.states[def_idx].last_reassign_tick = current_tick;
            }
        }
    }

    /// Update broken mark durations (T3)
    fn update_broken_durations(
        &mut self,
        defender_positions: &[(f32, f32); 11],
        attacker_positions: &[(f32, f32); 11],
        ball_pos: (f32, f32),
    ) {
        for def_idx in 1..11 {
            // Skip GK
            let state = &mut self.states[def_idx];

            // Press/cover defenders are allowed to break their mark
            if state.is_emergency_presser || state.is_cover {
                state.broken_duration_ticks = 0;
                continue;
            }

            if state.primary_mark_id < 0 {
                state.broken_duration_ticks = 0;
                continue;
            }

            let marked_idx = state.primary_mark_id as usize;
            if marked_idx >= 11 {
                state.broken_duration_ticks = 0;
                continue;
            }

            let marker_pos = defender_positions[def_idx];
            let marked_pos = attacker_positions[marked_idx];

            if is_mark_broken(marker_pos, marked_pos, ball_pos) {
                state.broken_duration_ticks = state.broken_duration_ticks.saturating_add(1);
            } else {
                state.broken_duration_ticks = 0;
            }
        }
    }

    /// T6: Emergency Free Carrier
    fn execute_emergency_presser(
        &mut self,
        current_tick: u64,
        ball_pos: (f32, f32),
        _carrier_id: Option<u8>,
        defender_positions: &[(f32, f32); 11],
    ) {
        // Clear previous emergency flags
        for state in &mut self.states {
            state.is_emergency_presser = false;
            state.is_cover = false;
        }

        // Select nearest defender as emergency presser (skip GK)
        if let Some(nearest_idx) = find_nearest_defender(ball_pos, defender_positions, 1..11) {
            self.states[nearest_idx].is_emergency_presser = true;
            self.states[nearest_idx].mode = MarkMode::Tight;
            self.states[nearest_idx].last_reassign_tick = current_tick;
        }

        // Select 2nd nearest as cover (if budget allows)
        if self.budget.cover_budget > 0 {
            if let Some(cover_idx) =
                find_nth_nearest_defender(ball_pos, defender_positions, 2, 1..11)
            {
                self.states[cover_idx].is_cover = true;
                self.states[cover_idx].mode = MarkMode::Normal;
                self.states[cover_idx].last_reassign_tick = current_tick;
            }
        }
    }

    /// T2: Possession Change
    fn execute_possession_change(
        &mut self,
        current_tick: u64,
        defender_positions: &[(f32, f32); 11],
        attacker_positions: &[(f32, f32); 11],
    ) {
        // Zone-based assignment (simple proximity matching)
        for i in 1..11 {
            // Skip GK
            let nearest_attacker =
                find_nearest_attacker(defender_positions[i], attacker_positions, 0..11);
            if let Some(att_idx) = nearest_attacker {
                self.states[i].primary_mark_id = att_idx as i8;
                self.states[i].mode = MarkMode::Loose; // Loose during transition
                self.states[i].last_reassign_tick = current_tick;
            }
        }
    }

    /// T5: Ball Switch
    fn execute_ball_switch(
        &mut self,
        current_tick: u64,
        ball_pos: (f32, f32),
        defender_positions: &[(f32, f32); 11],
        attacker_positions: &[(f32, f32); 11],
    ) {
        // Re-assign based on new ball position (zone shift)
        for i in 1..11 {
            // Skip GK
            let dist_to_ball = distance(defender_positions[i], ball_pos);
            if dist_to_ball < 20.0 {
                // Only reassign near-ball defenders
                let nearest_attacker =
                    find_nearest_attacker(defender_positions[i], attacker_positions, 0..11);
                if let Some(att_idx) = nearest_attacker {
                    self.states[i].primary_mark_id = att_idx as i8;
                    self.states[i].last_reassign_tick = current_tick;
                }
            }
        }
    }

    /// Enforce budget (max pressers/covers)
    fn enforce_budget(&mut self, ball_pos: (f32, f32), defender_positions: &[(f32, f32); 11]) {
        // Count current pressers
        let presser_count = self.states.iter().filter(|s| s.is_emergency_presser).count();
        let max_pressers = self.budget.effective_press_budget() as usize;

        if presser_count > max_pressers {
            // Downgrade furthest pressers
            let mut presser_indices: Vec<usize> = self
                .states
                .iter()
                .enumerate()
                .filter_map(|(i, s)| if s.is_emergency_presser { Some(i) } else { None })
                .collect();

            // Sort by distance to ball (furthest first)
            presser_indices.sort_by(|&a, &b| {
                let dist_a = distance(defender_positions[a], ball_pos);
                let dist_b = distance(defender_positions[b], ball_pos);
                dist_b.partial_cmp(&dist_a).unwrap() // Reverse order
            });

            // Downgrade excess
            for &idx in presser_indices.iter().skip(max_pressers) {
                self.states[idx].is_emergency_presser = false;
                self.states[idx].mode = MarkMode::Normal;
            }
        }

        // Count current covers
        let cover_count = self.states.iter().filter(|s| s.is_cover).count();
        let max_covers = self.budget.cover_budget as usize;

        if cover_count > max_covers {
            // Downgrade furthest covers
            let mut cover_indices: Vec<usize> = self
                .states
                .iter()
                .enumerate()
                .filter_map(|(i, s)| if s.is_cover { Some(i) } else { None })
                .collect();

            cover_indices.sort_by(|&a, &b| {
                let dist_a = distance(defender_positions[a], ball_pos);
                let dist_b = distance(defender_positions[b], ball_pos);
                dist_b.partial_cmp(&dist_a).unwrap()
            });

            for &idx in cover_indices.iter().skip(max_covers) {
                self.states[idx].is_cover = false;
            }
        }
    }

    /// Get presser track IDs (for visualization)
    pub fn get_presser_ids(&self) -> Vec<u8> {
        self.states
            .iter()
            .enumerate()
            .filter_map(|(i, s)| if s.is_emergency_presser { Some(i as u8) } else { None })
            .collect()
    }
}

impl Default for MarkingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// ============================================================================
/// Helper Functions
/// ============================================================================

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

const MARK_BROKEN_DISTANCE_M: f32 = 10.0;
const MARK_BROKEN_TRIGGER_TICKS: u8 = 3; // 3 ticks * 250ms = 750ms
const MARK_BROKEN_MAX_REASSIGNMENTS: usize = 4;
const ROTATION_MIN_MOVEMENT_M: f32 = 5.0;

fn is_mark_broken(marker_pos: (f32, f32), marked_pos: (f32, f32), ball_pos: (f32, f32)) -> bool {
    // Condition 1: Distance > threshold
    let dist = distance(marker_pos, marked_pos);
    if dist <= MARK_BROKEN_DISTANCE_M {
        return false;
    }

    // Condition 2: Angle > 90 degrees => dot < 0
    let v1 = (marked_pos.0 - marker_pos.0, marked_pos.1 - marker_pos.1);
    let v2 = (ball_pos.0 - marker_pos.0, ball_pos.1 - marker_pos.1);

    let mag1_sq = v1.0 * v1.0 + v1.1 * v1.1;
    let mag2_sq = v2.0 * v2.0 + v2.1 * v2.1;
    if mag1_sq < 1e-6 || mag2_sq < 1e-6 {
        return false;
    }

    let dot = v1.0 * v2.0 + v1.1 * v2.1;
    dot < 0.0
}

fn calculate_zone(pos: (f32, f32)) -> (u8, u8) {
    // 4 depth zones across 105m
    let depth = if pos.0 < 26.25 {
        0
    } else if pos.0 < field::CENTER_X {
        1
    } else if pos.0 < 78.75 {
        2
    } else {
        3
    };

    // 3 width zones across 68m
    let lane = if pos.1 < 22.67 {
        0
    } else if pos.1 < 45.33 {
        1
    } else {
        2
    };

    (depth, lane)
}

fn detect_rotation(
    current_zones: &[(u8, u8); 11],
    previous_zones: &[(u8, u8); 11],
    current_positions: &[(f32, f32); 11],
    previous_positions: &[(f32, f32); 11],
) -> bool {
    let mut swap_count = 0;

    for i in 0..11 {
        for j in (i + 1)..11 {
            let zones_swapped = current_zones[i] == previous_zones[j]
                && current_zones[j] == previous_zones[i]
                && current_zones[i] != current_zones[j];
            if !zones_swapped {
                continue;
            }

            let dist_i = distance(current_positions[i], previous_positions[i]);
            let dist_j = distance(current_positions[j], previous_positions[j]);
            if dist_i > ROTATION_MIN_MOVEMENT_M && dist_j > ROTATION_MIN_MOVEMENT_M {
                swap_count += 1;
            }
        }
    }

    swap_count >= 1
}

fn zones_adjacent_or_same(a: (u8, u8), b: (u8, u8)) -> bool {
    let depth_diff = (a.0 as i8 - b.0 as i8).abs();
    let lane_diff = (a.1 as i8 - b.1 as i8).abs();
    depth_diff <= 1 && lane_diff <= 1
}

fn find_nearest_defender(
    pos: (f32, f32),
    defenders: &[(f32, f32); 11],
    range: std::ops::Range<usize>,
) -> Option<usize> {
    range.min_by(|&a, &b| {
        let dist_a = distance(pos, defenders[a]);
        let dist_b = distance(pos, defenders[b]);
        dist_a.partial_cmp(&dist_b).unwrap()
    })
}

fn find_nth_nearest_defender(
    pos: (f32, f32),
    defenders: &[(f32, f32); 11],
    n: usize,
    range: std::ops::Range<usize>,
) -> Option<usize> {
    let mut indices: Vec<usize> = range.collect();
    indices.sort_by(|&a, &b| {
        let dist_a = distance(pos, defenders[a]);
        let dist_b = distance(pos, defenders[b]);
        dist_a.partial_cmp(&dist_b).unwrap()
    });

    if n > 0 && n <= indices.len() {
        Some(indices[n - 1])
    } else {
        None
    }
}

fn find_nearest_attacker(
    pos: (f32, f32),
    attackers: &[(f32, f32); 11],
    range: std::ops::Range<usize>,
) -> Option<usize> {
    range.min_by(|&a, &b| {
        let dist_a = distance(pos, attackers[a]);
        let dist_b = distance(pos, attackers[b]);
        dist_a.partial_cmp(&dist_b).unwrap()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const CX: f32 = field::CENTER_X;
    const CY: f32 = field::CENTER_Y;

    #[test]
    fn test_budget_enforcement() {
        let mut manager = MarkingManager::new();
        manager.set_tactic("balanced"); // press_budget = 1

        let ball_pos = (50.0, CY);
        let defender_positions = [
            (5.0, CY),  // GK
            (30.0, CY), // Nearest
            (35.0, CY), // 2nd nearest (should be downgraded)
            (40.0, CY),
            (45.0, 30.0),
            (45.0, 38.0),
            (25.0, 30.0),
            (25.0, 38.0),
            (15.0, 30.0),
            (15.0, 38.0),
            (10.0, CY),
        ];

        // Manually set 2 pressers (exceeds budget)
        manager.states[1].is_emergency_presser = true;
        manager.states[2].is_emergency_presser = true;

        manager.enforce_budget(ball_pos, &defender_positions);

        let presser_count = manager.get_presser_ids().len();
        assert!(presser_count <= manager.budget.effective_press_budget() as usize);
    }

    #[test]
    fn test_emergency_trigger() {
        let mut manager = MarkingManager::new();

        let ball_pos = (50.0, CY);
        let carrier_id = Some(15); // Away team player
        let defender_positions = [
            (5.0, CY),  // GK
            (20.0, CY), // Nearest (should become presser)
            (25.0, 30.0), // 2nd nearest (should become cover)
            (25.0, 38.0),
            (30.0, 30.0),
            (30.0, 38.0),
            (35.0, 30.0),
            (35.0, 38.0),
            (40.0, 30.0),
            (40.0, 38.0),
            (45.0, CY),
        ];
        let attacker_positions = [(60.0, CY); 11];

        manager.update(
            10,
            ball_pos,
            carrier_id,
            &defender_positions,
            &attacker_positions,
            0.70, // High free_score
            0.62, // Threshold
            false,
            TeamSide::Home,
            TransitionState::Inactive,
            false,
            None,
        );

        let expected_presser = find_nearest_defender(ball_pos, &defender_positions, 1..11)
            .expect("no non-GK defenders");
        assert!(manager.states[expected_presser].is_emergency_presser);

        if manager.budget.cover_budget > 0 {
            let expected_cover = find_nth_nearest_defender(ball_pos, &defender_positions, 2, 1..11)
                .expect("no non-GK cover defender");
            assert!(manager.states[expected_cover].is_cover);
        }
    }

    #[test]
    fn test_t1_kickoff_restart_resets_marks() {
        let mut manager = MarkingManager::new();

        // Pre-restart messy state
        manager.states[1].primary_mark_id = 5;
        manager.states[1].is_emergency_presser = true;
        manager.states[1].is_cover = true;
        manager.states[1].broken_duration_ticks = 10;

        let ball_pos = (CX, CY);
        let carrier_id = None;
        let defender_positions = [(50.0, CY); 11];
        let attacker_positions = [(60.0, CY); 11];

        manager.update(
            20,
            ball_pos,
            carrier_id,
            &defender_positions,
            &attacker_positions,
            0.0,
            0.62,
            false,
            TeamSide::Home,
            TransitionState::Inactive,
            true,
            Some(RestartType::KickOff),
        );

        assert!(!manager.states[1].is_emergency_presser);
        assert!(!manager.states[1].is_cover);
        assert_eq!(manager.states[1].broken_duration_ticks, 0);
        assert!(manager.states[1].primary_mark_id >= 0);
    }

    #[test]
    fn test_t1_kickoff_restart_assignments_are_unique() {
        use std::collections::HashSet;

        let mut manager = MarkingManager::new();

        let ball_pos = (CX, CY);
        let carrier_id = None;

        // All defenders at the same position used to cause duplicate nearest-attacker assignments.
        let defender_positions: [(f32, f32); 11] =
            std::array::from_fn(|i| if i == 0 { (5.0, CY) } else { (50.0, CY) });

        // Attackers laid out in a line so there are clear unique nearest choices.
        let attacker_positions: [(f32, f32); 11] = std::array::from_fn(|i| (60.0 + i as f32, CY));

        manager.update(
            30,
            ball_pos,
            carrier_id,
            &defender_positions,
            &attacker_positions,
            0.0,
            0.62,
            false,
            TeamSide::Home,
            TransitionState::Inactive,
            true,
            Some(RestartType::KickOff),
        );

        let mut marks = HashSet::new();
        for def_idx in 1..11 {
            let mark = manager.states[def_idx].primary_mark_id;
            assert!(mark >= 0, "defender {def_idx} has no mark");
            assert!(marks.insert(mark), "duplicate mark {mark} assigned");
        }
    }

    #[test]
    fn test_t1_set_piece_restart_biases_toward_ball_cluster_and_is_unique() {
        use std::collections::HashSet;

        let mut manager = MarkingManager::new();

        let ball_pos = (5.0, 5.0);
        let carrier_id = None;

        // Defenders clustered near the restart.
        let defender_positions: [(f32, f32); 11] = std::array::from_fn(|i| {    
            if i == 0 {
                (5.0, CY) // GK
            } else {
                (10.0 + i as f32, 8.0)
            }
        });

        // Attackers: first 6 near the ball (within 25m), rest far away.        
        let attacker_positions: [(f32, f32); 11] =
            std::array::from_fn(|i| if i < 6 { (12.0 + i as f32, 6.0) } else { (90.0, CY) });

        manager.update(
            40,
            ball_pos,
            carrier_id,
            &defender_positions,
            &attacker_positions,
            0.0,
            0.62,
            false,
            TeamSide::Home,
            TransitionState::Inactive,
            true,
            Some(RestartType::Corner),
        );

        let mut marks = HashSet::new();
        let mut near_cluster = 0;
        for def_idx in 1..11 {
            let mark = manager.states[def_idx].primary_mark_id;
            assert!(mark >= 0, "defender {def_idx} has no mark");
            assert!(marks.insert(mark), "duplicate mark {mark} assigned");
            if (mark as usize) < 6 {
                near_cluster += 1;
            }
        }

        assert!(near_cluster >= 6, "expected ≥6 marks on near-ball attackers, got {near_cluster}");
    }

    #[test]
    fn test_t3_mark_broken_duration_and_reassign() {
        let mut manager = MarkingManager::new();

        // Defender 1 is marking attacker 0, but it's broken (far + wrong angle)
        manager.states[1].primary_mark_id = 0;

        let defender_positions: [(f32, f32); 11] =
            std::array::from_fn(|i| if i == 1 { (50.0, CY) } else { (0.0, 0.0) });
        let attacker_positions: [(f32, f32); 11] = std::array::from_fn(|i| {    
            if i == 0 {
                (65.0, CY) // far
            } else if i == 1 {
                (52.0, CY) // close, should become new mark
            } else {
                (0.0, 0.0)
            }
        });
        let ball_pos = (40.0, CY); // behind marker relative to target        

        // Tick 10-11: broken duration accumulates
        manager.update(
            10,
            ball_pos,
            None,
            &defender_positions,
            &attacker_positions,
            0.0,
            0.62,
            false,
            TeamSide::Home,
            TransitionState::Inactive,
            false,
            None,
        );
        assert_eq!(manager.states[1].broken_duration_ticks, 1);

        manager.update(
            11,
            ball_pos,
            None,
            &defender_positions,
            &attacker_positions,
            0.0,
            0.62,
            false,
            TeamSide::Home,
            TransitionState::Inactive,
            false,
            None,
        );
        assert_eq!(manager.states[1].broken_duration_ticks, 2);

        // Tick 12: should trigger reassignment (duration >= 3)
        manager.update(
            12,
            ball_pos,
            None,
            &defender_positions,
            &attacker_positions,
            0.0,
            0.62,
            false,
            TeamSide::Home,
            TransitionState::Inactive,
            false,
            None,
        );
        assert_eq!(manager.states[1].primary_mark_id, 1);
        assert_eq!(manager.states[1].broken_duration_ticks, 0);
        assert_eq!(manager.states[1].mode, MarkMode::Tight);
    }

    #[test]
    fn test_t3_mark_broken_duration_resets_when_fixed() {
        let mut manager = MarkingManager::new();
        manager.states[1].primary_mark_id = 0;

        let defender_positions: [(f32, f32); 11] =
            std::array::from_fn(|i| if i == 1 { (50.0, CY) } else { (0.0, 0.0) });
        let attacker_positions_broken: [(f32, f32); 11] =
            std::array::from_fn(|i| if i == 0 { (65.0, CY) } else { (0.0, 0.0) });
        let attacker_positions_fixed: [(f32, f32); 11] =
            std::array::from_fn(|i| if i == 0 { (52.0, CY) } else { (0.0, 0.0) });
        let ball_pos = (40.0, CY);

        manager.update(
            10,
            ball_pos,
            None,
            &defender_positions,
            &attacker_positions_broken,
            0.0,
            0.62,
            false,
            TeamSide::Home,
            TransitionState::Inactive,
            false,
            None,
        );
        manager.update(
            11,
            ball_pos,
            None,
            &defender_positions,
            &attacker_positions_broken,
            0.0,
            0.62,
            false,
            TeamSide::Home,
            TransitionState::Inactive,
            false,
            None,
        );
        assert_eq!(manager.states[1].broken_duration_ticks, 2);

        // Now mark is no longer broken (attacker moved close)
        manager.update(
            12,
            ball_pos,
            None,
            &defender_positions,
            &attacker_positions_fixed,
            0.0,
            0.62,
            false,
            TeamSide::Home,
            TransitionState::Inactive,
            false,
            None,
        );
        assert_eq!(manager.states[1].broken_duration_ticks, 0);
    }

    #[test]
    fn test_t4_zone_calculation_and_rotation_detection() {
        assert_eq!(calculate_zone((20.0, 15.0)), (0, 0)); // Def-Left
        assert_eq!(calculate_zone((30.0, 35.0)), (1, 1)); // Mid-Def-Center
        assert_eq!(calculate_zone((80.0, 50.0)), (3, 2)); // Att-Right

        let prev_positions: [(f32, f32); 11] = std::array::from_fn(|i| {        
            if i == 0 {
                (40.0, CY)
            } else if i == 1 {
                (60.0, CY)
            } else {
                (0.0, 0.0)
            }
        });
        let curr_positions: [(f32, f32); 11] = std::array::from_fn(|i| {        
            if i == 0 {
                (60.0, CY)
            } else if i == 1 {
                (40.0, CY)
            } else {
                (0.0, 0.0)
            }
        });

        let prev_zones: [(u8, u8); 11] = std::array::from_fn(|i| calculate_zone(prev_positions[i]));
        let curr_zones: [(u8, u8); 11] = std::array::from_fn(|i| calculate_zone(curr_positions[i]));

        assert!(detect_rotation(&curr_zones, &prev_zones, &curr_positions, &prev_positions));
    }

    #[test]
    fn test_transition_modifiers_apply_only_to_team_that_lost_ball() {
        let ball_pos = (CX, CY);
        let defender_positions: [(f32, f32); 11] =
            std::array::from_fn(|i| if i == 0 { (5.0, CY) } else { (50.0 + i as f32, CY) });
        let attacker_positions: [(f32, f32); 11] = std::array::from_fn(|i| (60.0 + i as f32, CY));

        // Losing team (Home) gets transition bonus and loose marking.
        let mut manager = MarkingManager::new();
        manager.set_tactic("balanced"); // base press_budget = 1
        manager.update(
            1,
            ball_pos,
            None,
            &defender_positions,
            &attacker_positions,
            0.0,
            0.62,
            false,
            TeamSide::Home,
            TransitionState::Active { remaining_ms: 3000, team_lost_ball: TeamSide::Home },
            false,
            None,
        );
        assert_eq!(manager.budget.effective_press_budget(), 2);
        assert!(manager
            .states
            .iter()
            .skip(1)
            .all(|s| s.mode == MarkMode::Loose || s.mode == MarkMode::Tight));

        // Non-losing team does not get the transition bonus.
        let mut other = MarkingManager::new();
        other.set_tactic("balanced");
        other.update(
            1,
            ball_pos,
            None,
            &defender_positions,
            &attacker_positions,
            0.0,
            0.62,
            false,
            TeamSide::Away,
            TransitionState::Active { remaining_ms: 3000, team_lost_ball: TeamSide::Home },
            false,
            None,
        );
        assert_eq!(other.budget.effective_press_budget(), 1);
        assert!(other
            .states
            .iter()
            .skip(1)
            .all(|s| s.mode == MarkMode::Normal || s.mode == MarkMode::Tight));
    }
}
