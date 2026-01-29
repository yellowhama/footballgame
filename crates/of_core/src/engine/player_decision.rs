//! Player decision system based on individual instructions
//!
//! Calculates action probabilities and defensive contributions based on PlayerInstructions.

use crate::player::instructions::{
    DefensiveWork, Depth, DribblingFrequency, Mentality, PassingStyle, PlayerInstructions,
    PressingIntensity, ShootingTendency, Width,
};
use std::collections::HashMap;

/// Possible player actions during a match
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerAction {
    Pass,
    ShortPass,
    LongPass,
    ThroughBall,
    Cross,
    Shoot,
    Dribble,
    /// Take-on: 수비수를 제치는 적극적 돌파 시도
    /// Dribble(Carry)과 달리 수비수와의 1:1 Duel을 유발함
    TakeOn,
    Hold,
    Tackle,
    Header,
}

/// Player decision result
#[derive(Clone, Debug)]
pub struct PlayerDecision {
    pub player_id: String,
    pub action_probabilities: HashMap<PlayerAction, f32>,
    pub defensive_contribution: f32,
}

impl PlayerDecision {
    /// Calculate action probabilities based on player instructions
    pub fn calculate_action_probabilities(
        instructions: &PlayerInstructions,
        position: &str,
        in_attacking_third: bool,
    ) -> HashMap<PlayerAction, f32> {
        let mut probs = HashMap::new();

        // Base probabilities by position
        // 2025-12-11: Shot probabilities reduced ~10x for realistic match output
        // Real football: ~25 shots per match, not ~290
        let (base_shoot, base_dribble, base_pass) = match position {
            "ST" | "CF" => (0.025, 0.20, 0.50),        // Was 0.25
            "LW" | "RW" => (0.015, 0.25, 0.50),        // Was 0.15
            "CAM" | "AM" => (0.018, 0.18, 0.55),       // Was 0.18
            "CM" | "CDM" => (0.008, 0.12, 0.60),       // Was 0.08
            "LM" | "RM" => (0.010, 0.20, 0.55),        // Was 0.10
            "CB" | "LB" | "RB" => (0.003, 0.08, 0.70), // Was 0.03
            "GK" => (0.001, 0.02, 0.85),               // Was 0.01
            _ => (0.010, 0.15, 0.55),                  // Was 0.10
        };

        // Shooting tendency modifier
        let shoot_mod = match instructions.shooting {
            ShootingTendency::ShootOnSight => 1.8,
            ShootingTendency::Normal => 1.0,
            ShootingTendency::Conservative => 0.4,
        };
        let shoot_prob =
            if in_attacking_third { base_shoot * shoot_mod } else { base_shoot * shoot_mod * 0.3 };
        probs.insert(PlayerAction::Shoot, shoot_prob);

        // Dribbling frequency modifier
        let dribble_mod = match instructions.dribbling {
            DribblingFrequency::Often => 1.6,
            DribblingFrequency::Normal => 1.0,
            DribblingFrequency::Rarely => 0.3,
        };
        probs.insert(PlayerAction::Dribble, base_dribble * dribble_mod);

        // Passing style affects pass type distribution
        let (short_ratio, long_ratio, through_ratio) = match instructions.passing {
            PassingStyle::Short => (0.65, 0.15, 0.20),
            PassingStyle::Mixed => (0.45, 0.30, 0.25),
            PassingStyle::Direct => (0.25, 0.50, 0.25),
        };
        probs.insert(PlayerAction::ShortPass, base_pass * short_ratio);
        probs.insert(PlayerAction::LongPass, base_pass * long_ratio);
        probs.insert(PlayerAction::ThroughBall, base_pass * through_ratio);

        // Width affects crossing
        let cross_base = match position {
            "LW" | "RW" | "LM" | "RM" | "LB" | "RB" | "LWB" | "RWB" => 0.15,
            _ => 0.05,
        };
        let cross_mod = match instructions.width {
            Width::StayWide => 1.5,
            Width::Normal => 1.0,
            Width::CutInside => 0.5,
            Width::Roam => 0.8,
        };
        probs.insert(PlayerAction::Cross, cross_base * cross_mod);

        // Hold ball modifier based on mentality
        let hold_mod = match instructions.mentality {
            Mentality::Conservative => 1.4,
            Mentality::Balanced => 1.0,
            Mentality::Aggressive => 0.6,
        };
        probs.insert(PlayerAction::Hold, 0.10 * hold_mod);

        // Mentality adjusts overall risk-taking
        match instructions.mentality {
            Mentality::Aggressive => {
                // Increase risky actions
                if let Some(p) = probs.get_mut(&PlayerAction::Shoot) {
                    *p *= 1.2;
                }
                if let Some(p) = probs.get_mut(&PlayerAction::Dribble) {
                    *p *= 1.15;
                }
                if let Some(p) = probs.get_mut(&PlayerAction::ThroughBall) {
                    *p *= 1.2;
                }
            }
            Mentality::Conservative => {
                // Increase safe actions
                if let Some(p) = probs.get_mut(&PlayerAction::ShortPass) {
                    *p *= 1.3;
                }
                if let Some(p) = probs.get_mut(&PlayerAction::Shoot) {
                    *p *= 0.7;
                }
            }
            Mentality::Balanced => {}
        }

        // Normalize probabilities
        let total: f32 = probs.values().sum();
        if total > 0.0 {
            for prob in probs.values_mut() {
                *prob /= total;
            }
        }

        probs
    }

    /// Calculate defensive contribution based on instructions
    pub fn calculate_defensive_contribution(
        instructions: &PlayerInstructions,
        position: &str,
    ) -> f32 {
        // Base contribution by position
        let base = match position {
            "GK" => 0.0,
            "CB" | "SW" => 1.0,
            "LB" | "RB" | "LWB" | "RWB" => 0.85,
            "CDM" | "DM" => 0.8,
            "CM" => 0.6,
            "LM" | "RM" => 0.5,
            "CAM" | "AM" => 0.35,
            "LW" | "RW" => 0.3,
            "CF" => 0.25,
            "ST" => 0.2,
            _ => 0.5,
        };

        // Defensive work modifier
        let work_mod = match instructions.defensive_work {
            DefensiveWork::High => 1.5,
            DefensiveWork::Normal => 1.0,
            DefensiveWork::Minimal => 0.3,
        };

        // Pressing intensity modifier
        let pressing_mod = match instructions.pressing {
            PressingIntensity::High => 1.3,
            PressingIntensity::Medium => 1.0,
            PressingIntensity::Low => 0.7,
        };

        // Depth modifier: forward players contribute less to defense
        let depth_mod = match instructions.depth {
            Depth::StayBack => 1.25,
            Depth::Balanced => 1.0,
            Depth::GetForward => 0.6,
        };

        let result: f32 = base * work_mod * pressing_mod * depth_mod;
        result.clamp(0.0, 1.5)
    }

    /// Calculate offensive contribution based on instructions
    pub fn calculate_offensive_contribution(
        instructions: &PlayerInstructions,
        position: &str,
    ) -> f32 {
        // Base contribution by position
        let base = match position {
            "ST" | "CF" => 1.0,
            "LW" | "RW" => 0.9,
            "CAM" | "AM" => 0.85,
            "LM" | "RM" => 0.7,
            "CM" => 0.6,
            "LWB" | "RWB" => 0.5,
            "CDM" | "DM" => 0.4,
            "LB" | "RB" => 0.35,
            "CB" | "SW" => 0.2,
            "GK" => 0.05,
            _ => 0.5,
        };

        // Mentality modifier
        let mentality_mod = match instructions.mentality {
            Mentality::Aggressive => 1.3,
            Mentality::Balanced => 1.0,
            Mentality::Conservative => 0.7,
        };

        // Depth modifier: forward depth increases offensive contribution
        let depth_mod = match instructions.depth {
            Depth::GetForward => 1.3,
            Depth::Balanced => 1.0,
            Depth::StayBack => 0.7,
        };

        // Shooting tendency affects offensive output
        let shooting_mod = match instructions.shooting {
            ShootingTendency::ShootOnSight => 1.15,
            ShootingTendency::Normal => 1.0,
            ShootingTendency::Conservative => 0.9,
        };

        let result: f32 = base * mentality_mod * depth_mod * shooting_mod;
        result.clamp(0.0, 1.5)
    }

    /// Calculate stamina consumption rate based on instructions
    pub fn calculate_stamina_consumption(instructions: &PlayerInstructions) -> f32 {
        let mut consumption = 1.0;

        // High defensive work drains stamina
        consumption *= match instructions.defensive_work {
            DefensiveWork::High => 1.3,
            DefensiveWork::Normal => 1.0,
            DefensiveWork::Minimal => 0.8,
        };

        // High pressing drains stamina
        consumption *= match instructions.pressing {
            PressingIntensity::High => 1.25,
            PressingIntensity::Medium => 1.0,
            PressingIntensity::Low => 0.85,
        };

        // Getting forward drains stamina
        consumption *= match instructions.depth {
            Depth::GetForward => 1.15,
            Depth::Balanced => 1.0,
            Depth::StayBack => 0.9,
        };

        // Aggressive mentality slightly increases stamina use
        consumption *= match instructions.mentality {
            Mentality::Aggressive => 1.1,
            Mentality::Balanced => 1.0,
            Mentality::Conservative => 0.95,
        };

        let result: f32 = consumption;
        result.clamp(0.6, 1.8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::instructions::PlayerRole;

    #[test]
    fn test_shoot_on_sight_increases_shooting() {
        let mut instructions = PlayerInstructions::default();
        instructions.shooting = ShootingTendency::ShootOnSight;

        let probs = PlayerDecision::calculate_action_probabilities(&instructions, "ST", true);
        let shoot_prob = probs.get(&PlayerAction::Shoot).unwrap_or(&0.0);

        let normal_instructions = PlayerInstructions::default();
        let normal_probs =
            PlayerDecision::calculate_action_probabilities(&normal_instructions, "ST", true);
        let normal_shoot = normal_probs.get(&PlayerAction::Shoot).unwrap_or(&0.0);

        assert!(
            shoot_prob > normal_shoot,
            "ShootOnSight should increase shooting: {} vs {}",
            shoot_prob,
            normal_shoot
        );
    }

    #[test]
    fn test_high_defensive_work_increases_contribution() {
        let mut high_work = PlayerInstructions::default();
        high_work.defensive_work = DefensiveWork::High;

        let mut minimal_work = PlayerInstructions::default();
        minimal_work.defensive_work = DefensiveWork::Minimal;

        let high_contrib = PlayerDecision::calculate_defensive_contribution(&high_work, "CM");
        let low_contrib = PlayerDecision::calculate_defensive_contribution(&minimal_work, "CM");

        assert!(
            high_contrib > low_contrib * 2.0,
            "High work should greatly increase contribution: {} vs {}",
            high_contrib,
            low_contrib
        );
    }

    #[test]
    fn test_role_presets_affect_behavior() {
        let target_man = PlayerRole::TargetMan.default_instructions();
        let playmaker = PlayerRole::Playmaker.default_instructions();

        let tm_probs = PlayerDecision::calculate_action_probabilities(&target_man, "ST", true);
        let pm_probs = PlayerDecision::calculate_action_probabilities(&playmaker, "CAM", true);

        let tm_shoot = tm_probs.get(&PlayerAction::Shoot).unwrap_or(&0.0);
        let pm_shoot = pm_probs.get(&PlayerAction::Shoot).unwrap_or(&0.0);

        assert!(
            tm_shoot > pm_shoot,
            "TargetMan should shoot more than Playmaker: {} vs {}",
            tm_shoot,
            pm_shoot
        );
    }

    #[test]
    fn test_position_affects_base_probabilities() {
        let instructions = PlayerInstructions::default();

        let st_probs = PlayerDecision::calculate_action_probabilities(&instructions, "ST", true);
        let cb_probs = PlayerDecision::calculate_action_probabilities(&instructions, "CB", true);

        let st_shoot = st_probs.get(&PlayerAction::Shoot).unwrap_or(&0.0);
        let cb_shoot = cb_probs.get(&PlayerAction::Shoot).unwrap_or(&0.0);

        assert!(
            *st_shoot > *cb_shoot * 3.0,
            "ST should shoot much more than CB: {} vs {}",
            st_shoot,
            cb_shoot
        );
    }

    #[test]
    fn test_stamina_consumption_high_pressing() {
        let mut high_pressing = PlayerInstructions::default();
        high_pressing.pressing = PressingIntensity::High;
        high_pressing.defensive_work = DefensiveWork::High;

        let consumption = PlayerDecision::calculate_stamina_consumption(&high_pressing);
        assert!(consumption > 1.4, "High pressing should consume more stamina: {}", consumption);
    }

    #[test]
    fn test_probabilities_sum_to_one() {
        let instructions = PlayerInstructions::default();
        let probs = PlayerDecision::calculate_action_probabilities(&instructions, "CM", true);

        let total: f32 = probs.values().sum();
        assert!((total - 1.0).abs() < 0.01, "Probabilities should sum to 1.0: {}", total);
    }
}
