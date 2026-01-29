//! Match modifiers (sparse scalar bundle).
//!
//! This module is the SSOT for how external systems (deck/coach, events, etc.)
//! inject small, deterministic scalar effects into match simulation without
//! touching decision logic.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeamMatchModifiers {
    pub pass_success_mult: f32,
    pub shot_accuracy_mult: f32,
    pub shot_power_mult: f32,
    pub tackle_success_mult: f32,
    pub press_intensity_add: f32,
    pub stamina_drain_mult: f32,
}

impl Default for TeamMatchModifiers {
    fn default() -> Self {
        Self {
            pass_success_mult: 1.0,
            shot_accuracy_mult: 1.0,
            shot_power_mult: 1.0,
            tackle_success_mult: 1.0,
            press_intensity_add: 0.0,
            stamina_drain_mult: 1.0,
        }
    }
}

impl TeamMatchModifiers {
    pub fn apply_mod_id(&mut self, mod_id: u8, value: f32) {
        match mod_id {
            1 => self.pass_success_mult = clamp_finite(value, 1.00, 1.20, 1.0),
            2 => self.shot_accuracy_mult = clamp_finite(value, 1.00, 1.20, 1.0),
            3 => self.shot_power_mult = clamp_finite(value, 1.00, 1.20, 1.0),
            4 => self.tackle_success_mult = clamp_finite(value, 1.00, 1.20, 1.0),
            5 => self.press_intensity_add = clamp_finite(value, 0.00, 0.30, 0.0),
            6 => self.stamina_drain_mult = clamp_finite(value, 0.90, 1.20, 1.0),
            _ => {}
        }
    }

    pub fn apply_mod_list(&mut self, mods: &[(u8, f32)]) {
        for (id, value) in mods {
            self.apply_mod_id(*id, *value);
        }
    }
}

fn clamp_finite(value: f32, min: f32, max: f32, default: f32) -> f32 {
    if !value.is_finite() {
        return default;
    }
    value.clamp(min, max)
}

