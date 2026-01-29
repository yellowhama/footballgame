//! Trait Balance Configuration
//!
//! Centralized balance values for Gold trait special effects.
//! Adjust these values to tune game balance.

/// Gold Trait Balance Configuration
/// All values are percentages (0.0 - 1.0) or multipliers
pub struct GoldTraitBalance {
    // === Shooting Traits ===
    /// Sniper: Minimum accuracy required for guaranteed goal
    pub sniper_accuracy_threshold: f32,
    /// Sniper: Goal probability when threshold met
    pub sniper_goal_prob: f32,

    /// Cannon: Minimum distance (meters) for GK bypass
    pub cannon_min_distance: f32,
    /// Cannon: GK save probability when active (0 = can't save)
    pub cannon_gk_save_prob: f32,

    /// LobMaster: GK save probability on lob shots (0 = can't save)
    pub lob_master_gk_save_prob: f32,

    /// Acrobat: xG multiplier for volleys/overhead kicks
    pub acrobat_xg_multiplier: f32,

    /// Poacher: Intercept chance when active (0 = bypassed)
    pub poacher_intercept_chance: f32,

    // === Defense Traits ===
    /// Vacuum: Minimum tackle success rate
    pub vacuum_min_tackle_success: f32,
    /// Vacuum: Foul probability
    pub vacuum_foul_prob: f32,

    /// Wall: Minimum blocking success rate
    pub wall_min_block_success: f32,

    /// Reader: Tackle success bonus
    pub reader_tackle_bonus: f32,
    /// Reader: Intercept chance bonus
    pub reader_intercept_bonus: f32,

    /// Bully: Tackle success bonus
    pub bully_tackle_bonus: f32,
    /// Bully: Opponent dribble penalty
    pub bully_opponent_dribble_penalty: f32,

    /// Shadow: Opponent dribble success cap
    pub shadow_opponent_dribble_cap: f32,

    // === Dribbling Traits ===
    /// Speedster: Success rate when faster than defender
    pub speedster_success_rate: f32,

    /// Technician: 1v1 dribble success rate
    pub technician_success_rate: f32,

    /// Tank: Minimum dribble success rate (when tackled)
    pub tank_min_success_rate: f32,

    /// Magnet: Ball control success rate
    pub magnet_ball_control_rate: f32,

    // === Passing Traits ===
    /// Architect: Long pass intercept chance (0 = can't intercept)
    pub architect_intercept_chance: f32,

    // === Goalkeeper Traits ===
    /// Spider: Curve shot save bonus
    pub spider_curve_save_bonus: f32,

    /// Sweeper: Close range save multiplier
    pub sweeper_close_save_multiplier: f32,
    /// Sweeper: Close range threshold (meters)
    pub sweeper_close_range: f32,

    // === Physical Traits ===
    /// Engine: Fatigue penalty override (0 = no fatigue)
    pub engine_fatigue_penalty: f32,
}

impl Default for GoldTraitBalance {
    fn default() -> Self {
        Self {
            // Shooting - Current values (may need adjustment)
            sniper_accuracy_threshold: 0.80,
            sniper_goal_prob: 1.0, // 100% goal

            cannon_min_distance: 20.0,
            cannon_gk_save_prob: 0.0, // GK can't save

            lob_master_gk_save_prob: 0.0, // GK can't save

            acrobat_xg_multiplier: 2.0, // 2x xG

            poacher_intercept_chance: 0.0, // No intercept

            // Defense
            vacuum_min_tackle_success: 0.85,
            vacuum_foul_prob: 0.0, // No fouls

            wall_min_block_success: 0.80,

            reader_tackle_bonus: 0.15,    // +15%
            reader_intercept_bonus: 0.25, // +25%

            bully_tackle_bonus: 0.20,             // +20%
            bully_opponent_dribble_penalty: 0.15, // -15%

            shadow_opponent_dribble_cap: 0.40, // Max 40% success

            // Dribbling
            speedster_success_rate: 0.95,   // 95% (not quite 100%)
            technician_success_rate: 0.95,  // 95%
            tank_min_success_rate: 0.70,    // Min 70%
            magnet_ball_control_rate: 0.98, // 98%

            // Passing
            architect_intercept_chance: 0.0, // Can't intercept

            // Goalkeeper
            spider_curve_save_bonus: 0.30,      // +30%
            sweeper_close_save_multiplier: 2.0, // 2x
            sweeper_close_range: 8.0,           // Within 8m

            // Physical
            engine_fatigue_penalty: 0.0, // No fatigue
        }
    }
}

impl GoldTraitBalance {
    /// Create a "nerfed" balance for testing (less extreme values)
    pub fn nerfed() -> Self {
        Self {
            sniper_accuracy_threshold: 0.85,
            sniper_goal_prob: 0.90, // 90% goal (not guaranteed)

            cannon_min_distance: 25.0, // Farther required
            cannon_gk_save_prob: 0.10, // 10% save chance

            lob_master_gk_save_prob: 0.15, // 15% save chance

            acrobat_xg_multiplier: 1.5, // 1.5x instead of 2x

            poacher_intercept_chance: 0.05, // 5% chance still

            vacuum_min_tackle_success: 0.75,
            vacuum_foul_prob: 0.05, // 5% foul chance

            wall_min_block_success: 0.70,

            reader_tackle_bonus: 0.10,
            reader_intercept_bonus: 0.20,

            bully_tackle_bonus: 0.15,
            bully_opponent_dribble_penalty: 0.10,

            shadow_opponent_dribble_cap: 0.50,

            speedster_success_rate: 0.85,
            technician_success_rate: 0.85,
            tank_min_success_rate: 0.60,
            magnet_ball_control_rate: 0.90,

            architect_intercept_chance: 0.05,

            spider_curve_save_bonus: 0.20,
            sweeper_close_save_multiplier: 1.5,
            sweeper_close_range: 6.0,

            engine_fatigue_penalty: 0.05, // 5% fatigue still
        }
    }

    /// Create a "buffed" balance for testing (more extreme values)
    pub fn buffed() -> Self {
        Self {
            sniper_accuracy_threshold: 0.75,
            sniper_goal_prob: 1.0,

            cannon_min_distance: 18.0,
            cannon_gk_save_prob: 0.0,

            lob_master_gk_save_prob: 0.0,

            acrobat_xg_multiplier: 2.5,

            poacher_intercept_chance: 0.0,

            vacuum_min_tackle_success: 0.90,
            vacuum_foul_prob: 0.0,

            wall_min_block_success: 0.85,

            reader_tackle_bonus: 0.20,
            reader_intercept_bonus: 0.30,

            bully_tackle_bonus: 0.25,
            bully_opponent_dribble_penalty: 0.20,

            shadow_opponent_dribble_cap: 0.35,

            speedster_success_rate: 0.98,
            technician_success_rate: 0.98,
            tank_min_success_rate: 0.75,
            magnet_ball_control_rate: 0.99,

            architect_intercept_chance: 0.0,

            spider_curve_save_bonus: 0.40,
            sweeper_close_save_multiplier: 2.5,
            sweeper_close_range: 10.0,

            engine_fatigue_penalty: 0.0,
        }
    }
}

use once_cell::sync::Lazy;
use std::sync::RwLock;

/// 스레드 안전한 글로벌 Gold Trait Balance 설정
/// - Lazy: 첫 접근 시 초기화
/// - RwLock: 멀티스레드 읽기/쓰기 안전
static GOLD_BALANCE: Lazy<RwLock<GoldTraitBalance>> =
    Lazy::new(|| RwLock::new(GoldTraitBalance::default()));

/// Get the current gold trait balance configuration
///
/// Returns a read guard that can be used like `&GoldTraitBalance`
/// via Deref coercion.
pub fn get_gold_balance() -> std::sync::RwLockReadGuard<'static, GoldTraitBalance> {
    GOLD_BALANCE.read().expect("GOLD_BALANCE lock poisoned")
}

/// Set a custom balance configuration (for testing)
pub fn set_gold_balance(balance: GoldTraitBalance) {
    *GOLD_BALANCE.write().expect("GOLD_BALANCE lock poisoned") = balance;
}

/// Reset to default balance
pub fn reset_gold_balance() {
    *GOLD_BALANCE.write().expect("GOLD_BALANCE lock poisoned") = GoldTraitBalance::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_balance() {
        let balance = GoldTraitBalance::default();
        assert_eq!(balance.sniper_goal_prob, 1.0);
        assert_eq!(balance.speedster_success_rate, 0.95);
    }

    #[test]
    fn test_nerfed_balance() {
        let balance = GoldTraitBalance::nerfed();
        assert!(balance.sniper_goal_prob < 1.0);
        assert!(balance.speedster_success_rate < 0.95);
    }
}
