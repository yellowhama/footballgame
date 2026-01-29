//! Ability → MotionParams SSOT
//!
//! This module is the single authority for converting `PlayerAttributes` into
//! locomotion parameters used by P15 (`update_player_motion`).
//!
//! Goal: avoid drift (no ad-hoc "quick mapping" elsewhere).

use crate::engine::physics_constants::player_inertia::*;
use crate::models::player::PlayerAttributes;

/// Physical limits derived from ability attributes (base / unscaled).
///
/// Notes:
/// - This is not a full rigidbody model.
/// - Runtime scaling (stamina, buffs, exceptions) should be applied via
///   `scale_by_stamina` (and other policy layers).
#[derive(Clone, Debug)]
pub struct PlayerMotionParams {
    /// Top speed (m/s) - pace based
    pub max_speed: f32,
    /// Acceleration (m/s²) - acceleration based
    pub accel: f32,
    /// Deceleration (m/s²) - balance + strength + agility based
    pub decel: f32,
    /// Turning skill (0..1) - agility + balance based
    pub turn_skill: f32,
    /// Drag coefficient - stamina + natural_fitness based (lower = less decay)
    pub drag: f32,
}

impl Default for PlayerMotionParams {
    fn default() -> Self {
        Self {
            max_speed: MAX_SPEED_BASE + MAX_SPEED_RANGE * 0.5,
            accel: ACCEL_BASE + ACCEL_RANGE * 0.5,
            decel: DECEL_BASE + DECEL_RANGE * 0.5,
            turn_skill: 0.5,
            drag: DRAG_BASE,
        }
    }
}

/// Convert player attributes into base locomotion parameters.
///
/// v1 requirement: keep behavior consistent with the legacy P15 mapping.
pub fn ability_to_motion_params(attrs: &PlayerAttributes) -> PlayerMotionParams {
    let pace_n = n100(attrs.pace);
    let acc_n = n100(attrs.acceleration);
    let bal_n = n100(attrs.balance);
    let agi_n = n100(attrs.agility);
    let str_n = n100(attrs.strength);
    let sta_n = n100(attrs.stamina);
    let nf_n = n100(attrs.natural_fitness);

    // Deceleration: balance 50% + strength 30% + agility 20%
    let decel_factor =
        DECEL_BALANCE_WEIGHT * bal_n + DECEL_STRENGTH_WEIGHT * str_n + DECEL_AGILITY_WEIGHT * agi_n;

    // Drag: stamina/fitness reduce drag (more endurance => less decay)
    let drag_factor = 1.2 - DRAG_STAMINA_WEIGHT * sta_n - DRAG_FITNESS_WEIGHT * nf_n;

    PlayerMotionParams {
        max_speed: MAX_SPEED_BASE + pace_n * MAX_SPEED_RANGE,
        accel: ACCEL_BASE + acc_n * ACCEL_RANGE,
        decel: DECEL_BASE + decel_factor * DECEL_RANGE,
        turn_skill: (TURN_AGILITY_WEIGHT * agi_n + TURN_BALANCE_WEIGHT * bal_n).clamp(0.0, 1.0),
        drag: (DRAG_BASE * drag_factor).clamp(DRAG_MIN, DRAG_MAX),
    }
}

/// Apply runtime stamina scaling to base parameters.
///
/// `sta_attr` is reserved for future tuning (v1 keeps behavior identical to the
/// legacy P15 curve).
pub fn scale_by_stamina(
    base: &PlayerMotionParams,
    stamina_state01: f32,
    _sta_attr: u8,
) -> PlayerMotionParams {
    let stamina01 = stamina_state01.clamp(0.0, 1.0);
    let fatigue_mult = FATIGUE_MIN_MULT + FATIGUE_RANGE * stamina01;

    PlayerMotionParams {
        max_speed: base.max_speed * fatigue_mult,
        accel: base.accel * fatigue_mult,
        decel: base.decel * (0.8 + 0.2 * stamina01), // decel is less affected
        turn_skill: base.turn_skill * fatigue_mult,
        drag: base.drag,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs_with(
        pace: u8,
        acceleration: u8,
        agility: u8,
        balance: u8,
        strength: u8,
        stamina: u8,
        natural_fitness: u8,
    ) -> PlayerAttributes {
        let mut attrs = PlayerAttributes::default();
        attrs.pace = pace;
        attrs.acceleration = acceleration;
        attrs.agility = agility;
        attrs.balance = balance;
        attrs.strength = strength;
        attrs.stamina = stamina;
        attrs.natural_fitness = natural_fitness;
        attrs
    }

    #[test]
    fn test_ability_to_motion_params_pace_extremes() {
        let mut attrs = PlayerAttributes::default();

        attrs.pace = 100;
        let max = ability_to_motion_params(&attrs);
        assert!((max.max_speed - (MAX_SPEED_BASE + MAX_SPEED_RANGE)).abs() < 0.01);

        attrs.pace = 0;
        let min = ability_to_motion_params(&attrs);
        assert!((min.max_speed - MAX_SPEED_BASE).abs() < 0.01);
    }

    #[test]
    fn test_scale_by_stamina_respects_min_mult() {
        let base = PlayerMotionParams {
            max_speed: 9.0,
            accel: 4.0,
            decel: 5.0,
            turn_skill: 0.8,
            drag: 0.05,
        };

        let exhausted = scale_by_stamina(&base, 0.0, 50);
        assert!((exhausted.max_speed - base.max_speed * FATIGUE_MIN_MULT).abs() < 1e-6);
        assert!((exhausted.accel - base.accel * FATIGUE_MIN_MULT).abs() < 1e-6);
        assert!((exhausted.decel - base.decel * 0.8).abs() < 1e-6);
        assert!((exhausted.drag - base.drag).abs() < 1e-6);
    }

    #[test]
    fn test_ability_profiles_ordering() {
        // Speed: high pace/accel
        let speed = ability_to_motion_params(&attrs_with(95, 95, 60, 55, 55, 60, 60));
        // Agile: high agility/balance
        let agile = ability_to_motion_params(&attrs_with(70, 70, 95, 90, 55, 60, 60));
        // Physical: high strength/balance (better braking)
        let physical = ability_to_motion_params(&attrs_with(70, 70, 60, 90, 95, 60, 60));
        // Endurance: high stamina/fitness (lower drag)
        let endurance = ability_to_motion_params(&attrs_with(70, 70, 60, 60, 60, 95, 95));

        assert!(speed.max_speed > agile.max_speed);
        assert!(speed.max_speed > physical.max_speed);
        assert!(speed.accel > agile.accel);
        assert!(speed.accel > physical.accel);

        assert!(agile.turn_skill > speed.turn_skill);
        assert!(agile.turn_skill > physical.turn_skill);

        assert!(physical.decel > speed.decel);
        assert!(physical.decel > agile.decel);

        assert!(endurance.drag < speed.drag);
        assert!(endurance.drag < agile.drag);
        assert!(endurance.drag < physical.drag);
    }
}
