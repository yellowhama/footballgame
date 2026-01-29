//! # Experimental Configuration Module (DPER Framework)
//!
//! ExpConfig allows runtime tuning of decision parameters for A/B testing.
//! Part of the Dual-Path Experimental Runner (DPER) system.
//!
//! ## Usage
//!
//! ```rust,ignore
//! let config = ExpConfig::load("experiments/aggressive_v1.json")?;
//! engine.apply_exp_config(&config);
//! ```
//!
//! ## Parameter Groups
//!
//! - **DecisionParams**: Shot xG threshold, pass risk tolerance, dribble bias
//! - **AudacityParams**: Scale, losing boost, late game urgency
//! - **StyleParams**: Tempo bias, width bias, directness bias

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

// ========== ExpConfig Main Structure ==========

/// Experimental configuration for match simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpConfig {
    /// Unique experiment identifier
    pub exp_id: String,
    /// Human-readable name
    pub name: String,
    /// Description of expected behavior changes
    pub description: String,
    /// Decision parameters (shot/pass/dribble thresholds)
    pub decision: DecisionParams,
    /// Audacity system parameters
    pub audacity: AudacityParams,
    /// Play style parameters
    pub style: StyleParams,
    /// Rulebook / referee layer parameters (executor-only; no DecisionTopology coupling)
    #[serde(default)]
    pub rulebook: RulebookParams,
}

// ========== Decision Parameters ==========

/// Parameters that affect action selection (shoot vs pass vs dribble)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionParams {
    /// Minimum xG threshold to consider shooting (default: 0.05)
    /// Lower = more shots, Higher = only high-quality chances
    #[serde(default = "default_shoot_xg_threshold")]
    pub shoot_xg_threshold: f32,

    /// Pass risk tolerance (default: 0.3)
    /// Higher = more risky passes attempted, Lower = safer passes only
    #[serde(default = "default_pass_risk_tolerance")]
    pub pass_risk_tolerance: f32,

    /// Dribble bias (default: 0.0)
    /// Positive = more dribbles, Negative = fewer dribbles
    #[serde(default)]
    pub dribble_bias: f32,

    /// Through ball multiplier (default: 1.0)
    /// > 1.0 = more through balls, < 1.0 = fewer
    #[serde(default = "default_one")]
    pub through_ball_multiplier: f32,

    /// Cross multiplier (default: 1.0)
    /// > 1.0 = more crosses, < 1.0 = fewer
    #[serde(default = "default_one")]
    pub cross_multiplier: f32,

    /// Enable Decision Priority Queue (DPQ) scheduling (default: false).
    ///
    /// v1.1: DPQ is a routing layer (no behavior change) and must not change
    /// match outcomes compared to baseline.
    #[serde(default)]
    pub dpq_enabled: bool,

    /// Enable variable cadence in DPQ (default: false).
    ///
    /// v1.2: Variable decision frequency based on proximity to ball action.
    /// - Active zone (ball owner, pass target, within 20m): every tick (250ms)
    /// - Passive zone (distant players): every 4 ticks (1000ms)
    /// Requires `dpq_enabled = true` to have any effect.
    #[serde(default)]
    pub dpq_variable_cadence: bool,

    /// Enable perception constraints in decision evaluation (default: false).
    ///
    /// v1.3: perception gates what a player can consider, but must remain
    /// deterministic and must not mutate match outcomes from the render layer.
    #[serde(default)]
    pub perception_enabled: bool,

    /// Enable off-ball decision system (default: false).
    ///
    /// v1 (FIX_2601/0115): TTL-based objectives for off-ball players.
    /// - 7 intent types: LinkPlayer, SpaceAttacker, Lurker, WidthHolder (attacking)
    ///   TrackBack, Screen, PressSupport (defending)
    /// - Score6 evaluation (UAE-lite)
    /// - DPQ Top-K scheduling (K=6 normal, K=8 transition)
    #[serde(default)]
    pub offball_decisions_enabled: bool,
}

fn default_shoot_xg_threshold() -> f32 {
    0.05
}
fn default_pass_risk_tolerance() -> f32 {
    0.3
}
fn default_one() -> f32 {
    1.0
}

impl Default for DecisionParams {
    fn default() -> Self {
        Self {
            shoot_xg_threshold: 0.05,
            pass_risk_tolerance: 0.3,
            dribble_bias: 0.0,
            through_ball_multiplier: 1.0,
            cross_multiplier: 1.0,
            dpq_enabled: false,
            dpq_variable_cadence: false,
            perception_enabled: false,
            offball_decisions_enabled: false,
        }
    }
}

// ========== Audacity Parameters ==========

/// Parameters that affect risk-taking behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudacityParams {
    /// Global audacity scale (default: 1.0)
    /// > 1.0 = more risky decisions, < 1.0 = safer decisions
    #[serde(default = "default_one")]
    pub scale: f32,

    /// Additional boost when losing (default: 0.1)
    /// Higher = more aggressive when behind
    #[serde(default = "default_losing_boost")]
    pub losing_boost: f32,

    /// Late game urgency multiplier (default: 1.2)
    /// Applied when minute > 70
    #[serde(default = "default_late_game_urgency")]
    pub late_game_urgency: f32,
}

fn default_losing_boost() -> f32 {
    0.1
}
fn default_late_game_urgency() -> f32 {
    1.2
}

impl Default for AudacityParams {
    fn default() -> Self {
        Self {
            scale: 1.0,
            losing_boost: 0.1,
            late_game_urgency: 1.2,
        }
    }
}

// ========== Style Parameters ==========

/// Parameters that affect overall play style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleParams {
    /// Tempo bias (default: 0.0)
    /// Positive = faster play, Negative = slower possession
    #[serde(default)]
    pub tempo_bias: f32,

    /// Width bias (default: 0.0)
    /// Positive = wider play, Negative = narrower through the middle
    #[serde(default)]
    pub width_bias: f32,

    /// Directness bias (default: 0.0)
    /// Positive = more direct/forward, Negative = more patient buildup
    #[serde(default)]
    pub directness_bias: f32,
}

impl Default for StyleParams {
    fn default() -> Self {
        Self {
            tempo_bias: 0.0,
            width_bias: 0.0,
            directness_bias: 0.0,
        }
    }
}

// ========== Rulebook Parameters ==========

/// Parameters that affect executor/referee outcomes (rulebook-only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookParams {
    /// Enable optional non-GK handball triggers (default: false).
    ///
    /// Important: This must remain executor-only (SSOT) and must not couple into
    /// DecisionTopology scoring.
    #[serde(default)]
    pub non_gk_handball_enabled: bool,

    /// Probability multiplier for non-GK handball triggers (default: 1.0).
    /// Used as a tuning knob to avoid foul/penalty volume explosions.
    #[serde(default = "default_one")]
    pub non_gk_handball_prob_mult: f32,

    /// Enable advantage play (play-on) for certain fouls (default: false).
    ///
    /// Important: This must remain executor-only (SSOT) and must not couple into
    /// DecisionTopology scoring.
    #[serde(default)]
    pub advantage_play_enabled: bool,
}

impl Default for RulebookParams {
    fn default() -> Self {
        Self {
            non_gk_handball_enabled: false,
            non_gk_handball_prob_mult: 1.0,
            advantage_play_enabled: false,
        }
    }
}

// ========== ExpConfig Implementation ==========

impl Default for ExpConfig {
    fn default() -> Self {
        Self {
            exp_id: "baseline".to_string(),
            name: "Baseline".to_string(),
            description: "Default parameters".to_string(),
            decision: DecisionParams::default(),
            audacity: AudacityParams::default(),
            style: StyleParams::default(),
            rulebook: RulebookParams::default(),
        }
    }
}

impl ExpConfig {
    /// Load ExpConfig from JSON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ExpConfigError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| ExpConfigError::IoError(e.to_string()))?;
        let config: ExpConfig = serde_json::from_str(&content)
            .map_err(|e| ExpConfigError::ParseError(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Parse ExpConfig from JSON string
    pub fn from_json(json: &str) -> Result<Self, ExpConfigError> {
        let config: ExpConfig = serde_json::from_str(json)
            .map_err(|e| ExpConfigError::ParseError(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration bounds
    pub fn validate(&self) -> Result<(), ExpConfigError> {
        // Decision params bounds
        if self.decision.shoot_xg_threshold < 0.0 || self.decision.shoot_xg_threshold > 1.0 {
            return Err(ExpConfigError::ValidationError(
                format!("shoot_xg_threshold must be 0.0-1.0, got {}", self.decision.shoot_xg_threshold)
            ));
        }
        if self.decision.pass_risk_tolerance < 0.0 || self.decision.pass_risk_tolerance > 1.0 {
            return Err(ExpConfigError::ValidationError(
                format!("pass_risk_tolerance must be 0.0-1.0, got {}", self.decision.pass_risk_tolerance)
            ));
        }
        if self.decision.dribble_bias < -1.0 || self.decision.dribble_bias > 1.0 {
            return Err(ExpConfigError::ValidationError(
                format!("dribble_bias must be -1.0 to 1.0, got {}", self.decision.dribble_bias)
            ));
        }
        if self.decision.through_ball_multiplier < 0.0 || self.decision.through_ball_multiplier > 2.0 {
            return Err(ExpConfigError::ValidationError(
                format!("through_ball_multiplier must be 0.0-2.0, got {}", self.decision.through_ball_multiplier)
            ));
        }
        if self.decision.cross_multiplier < 0.0 || self.decision.cross_multiplier > 2.0 {
            return Err(ExpConfigError::ValidationError(
                format!("cross_multiplier must be 0.0-2.0, got {}", self.decision.cross_multiplier)
            ));
        }

        // Audacity params bounds
        if self.audacity.scale < 0.0 || self.audacity.scale > 2.0 {
            return Err(ExpConfigError::ValidationError(
                format!("audacity.scale must be 0.0-2.0, got {}", self.audacity.scale)
            ));
        }
        if self.audacity.losing_boost < 0.0 || self.audacity.losing_boost > 0.5 {
            return Err(ExpConfigError::ValidationError(
                format!("audacity.losing_boost must be 0.0-0.5, got {}", self.audacity.losing_boost)
            ));
        }
        if self.audacity.late_game_urgency < 0.5 || self.audacity.late_game_urgency > 2.0 {
            return Err(ExpConfigError::ValidationError(
                format!("audacity.late_game_urgency must be 0.5-2.0, got {}", self.audacity.late_game_urgency)
            ));
        }

        // Style params bounds
        if self.style.tempo_bias < -1.0 || self.style.tempo_bias > 1.0 {
            return Err(ExpConfigError::ValidationError(
                format!("style.tempo_bias must be -1.0 to 1.0, got {}", self.style.tempo_bias)
            ));
        }
        if self.style.width_bias < -1.0 || self.style.width_bias > 1.0 {
            return Err(ExpConfigError::ValidationError(
                format!("style.width_bias must be -1.0 to 1.0, got {}", self.style.width_bias)
            ));
        }
        if self.style.directness_bias < -1.0 || self.style.directness_bias > 1.0 {
            return Err(ExpConfigError::ValidationError(
                format!("style.directness_bias must be -1.0 to 1.0, got {}", self.style.directness_bias)
            ));
        }

        // Rulebook params bounds
        if self.rulebook.non_gk_handball_prob_mult < 0.0 || self.rulebook.non_gk_handball_prob_mult > 10.0 {
            return Err(ExpConfigError::ValidationError(format!(
                "rulebook.non_gk_handball_prob_mult must be 0.0-10.0, got {}",
                self.rulebook.non_gk_handball_prob_mult
            )));
        }

        Ok(())
    }

    /// Create aggressive preset
    pub fn aggressive() -> Self {
        Self {
            exp_id: "aggressive".to_string(),
            name: "Aggressive Play".to_string(),
            description: "More shots, more dribbles, lower xG threshold".to_string(),
            decision: DecisionParams {
                shoot_xg_threshold: 0.03,
                pass_risk_tolerance: 0.4,
                dribble_bias: 0.3,
                through_ball_multiplier: 1.2,
                cross_multiplier: 1.1,
                ..Default::default()
            },
            audacity: AudacityParams {
                scale: 1.2,
                losing_boost: 0.15,
                late_game_urgency: 1.3,
            },
            style: StyleParams {
                tempo_bias: 0.2,
                width_bias: 0.1,
                directness_bias: 0.3,
            },
            rulebook: RulebookParams::default(),
        }
    }

    /// Create conservative preset
    pub fn conservative() -> Self {
        Self {
            exp_id: "conservative".to_string(),
            name: "Conservative Play".to_string(),
            description: "Safer passes, higher xG threshold, possession-focused".to_string(),
            decision: DecisionParams {
                shoot_xg_threshold: 0.08,
                pass_risk_tolerance: 0.15,
                dribble_bias: -0.2,
                through_ball_multiplier: 0.8,
                cross_multiplier: 0.9,
                ..Default::default()
            },
            audacity: AudacityParams {
                scale: 0.8,
                losing_boost: 0.05,
                late_game_urgency: 1.1,
            },
            style: StyleParams {
                tempo_bias: -0.3,
                width_bias: -0.1,
                directness_bias: -0.4,
            },
            rulebook: RulebookParams::default(),
        }
    }

    /// Compute SHA256 hash of config for determinism checks
    pub fn config_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        // Hash key fields
        self.exp_id.hash(&mut hasher);
        format!("{:.6}", self.decision.shoot_xg_threshold).hash(&mut hasher);
        format!("{:.6}", self.decision.pass_risk_tolerance).hash(&mut hasher);
        format!("{:.6}", self.decision.dribble_bias).hash(&mut hasher);
        format!("{:.6}", self.decision.through_ball_multiplier).hash(&mut hasher);
        format!("{:.6}", self.decision.cross_multiplier).hash(&mut hasher);
        self.decision.dpq_enabled.hash(&mut hasher);
        self.decision.dpq_variable_cadence.hash(&mut hasher);
        self.decision.perception_enabled.hash(&mut hasher);
        self.decision.offball_decisions_enabled.hash(&mut hasher);
        format!("{:.6}", self.audacity.scale).hash(&mut hasher);
        format!("{:.6}", self.audacity.losing_boost).hash(&mut hasher);
        format!("{:.6}", self.audacity.late_game_urgency).hash(&mut hasher);
        format!("{:.6}", self.style.tempo_bias).hash(&mut hasher);
        format!("{:.6}", self.style.width_bias).hash(&mut hasher);
        format!("{:.6}", self.style.directness_bias).hash(&mut hasher);
        self.rulebook.non_gk_handball_enabled.hash(&mut hasher);
        format!("{:.6}", self.rulebook.non_gk_handball_prob_mult).hash(&mut hasher);
        self.rulebook.advantage_play_enabled.hash(&mut hasher);

        format!("{:016x}", hasher.finish())
    }
}

// ========== Error Type ==========

/// Errors that can occur when loading/validating ExpConfig
#[derive(Debug, Clone)]
pub enum ExpConfigError {
    IoError(String),
    ParseError(String),
    ValidationError(String),
}

impl std::fmt::Display for ExpConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpConfigError::IoError(msg) => write!(f, "IO error: {}", msg),
            ExpConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ExpConfigError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ExpConfigError {}

// ========== Runtime Parameters (Applied to Engine) ==========

/// Runtime-applied experimental parameters
/// These are computed from ExpConfig and used during simulation
#[derive(Debug, Clone, Default)]
pub struct RuntimeExpParams {
    /// Minimum xG to consider shooting
    pub shoot_xg_threshold: f32,
    /// Pass risk tolerance
    pub pass_risk_tolerance: f32,
    /// Dribble EV bias
    pub dribble_bias: f32,
    /// Through ball multiplier
    pub through_ball_multiplier: f32,
    /// Cross multiplier
    pub cross_multiplier: f32,
    /// Decision scheduling: DPQ enabled
    pub dpq_enabled: bool,
    /// DPQ v1.2: variable cadence enabled
    pub dpq_variable_cadence: bool,
    /// Decision realism: perception enabled
    pub perception_enabled: bool,
    /// Off-ball decision system enabled (FIX_2601/0115)
    pub offball_decisions_enabled: bool,
    /// Audacity scale factor
    pub audacity_scale: f32,
    /// Losing boost
    pub audacity_losing_boost: f32,
    /// Late game urgency
    pub audacity_late_game_urgency: f32,
    /// Tempo bias
    pub tempo_bias: f32,
    /// Width bias
    pub width_bias: f32,
    /// Directness bias
    pub directness_bias: f32,

    /// Rulebook: non-GK handball triggers enabled
    pub non_gk_handball_enabled: bool,
    /// Rulebook: non-GK handball probability multiplier
    pub non_gk_handball_prob_mult: f32,
    /// Rulebook: advantage play enabled
    pub advantage_play_enabled: bool,
}

impl From<&ExpConfig> for RuntimeExpParams {
    fn from(config: &ExpConfig) -> Self {
        Self {
            shoot_xg_threshold: config.decision.shoot_xg_threshold,
            pass_risk_tolerance: config.decision.pass_risk_tolerance,
            dribble_bias: config.decision.dribble_bias,
            through_ball_multiplier: config.decision.through_ball_multiplier,
            cross_multiplier: config.decision.cross_multiplier,
            dpq_enabled: config.decision.dpq_enabled,
            dpq_variable_cadence: config.decision.dpq_variable_cadence,
            perception_enabled: config.decision.perception_enabled,
            offball_decisions_enabled: config.decision.offball_decisions_enabled,
            audacity_scale: config.audacity.scale,
            audacity_losing_boost: config.audacity.losing_boost,
            audacity_late_game_urgency: config.audacity.late_game_urgency,
            tempo_bias: config.style.tempo_bias,
            width_bias: config.style.width_bias,
            directness_bias: config.style.directness_bias,
            non_gk_handball_enabled: config.rulebook.non_gk_handball_enabled,
            non_gk_handball_prob_mult: config.rulebook.non_gk_handball_prob_mult,
            advantage_play_enabled: config.rulebook.advantage_play_enabled,
        }
    }
}

impl RuntimeExpParams {
    /// Create baseline (stable) parameters
    pub fn baseline() -> Self {
        Self {
            shoot_xg_threshold: 0.05,
            pass_risk_tolerance: 0.3,
            dribble_bias: 0.0,
            through_ball_multiplier: 1.0,
            cross_multiplier: 1.0,
            dpq_enabled: false,
            dpq_variable_cadence: false,
            perception_enabled: false,
            offball_decisions_enabled: false,
            audacity_scale: 1.0,
            audacity_losing_boost: 0.1,
            audacity_late_game_urgency: 1.2,
            tempo_bias: 0.0,
            width_bias: 0.0,
            directness_bias: 0.0,
            non_gk_handball_enabled: false,
            non_gk_handball_prob_mult: 1.0,
            advantage_play_enabled: false,
        }
    }
}

// ========== Diff Report ==========

/// Statistics from a single match result for comparison
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchStats {
    /// Goals scored by home team
    pub home_goals: u32,
    /// Goals scored by away team
    pub away_goals: u32,
    /// Home team possession percentage (0-100)
    pub home_possession: f32,
    /// Total shots by home team
    pub home_shots: u32,
    /// Total shots by away team
    pub away_shots: u32,
    /// Shots on target by home team
    pub home_shots_on_target: u32,
    /// Shots on target by away team
    pub away_shots_on_target: u32,
    /// Total passes by home team
    pub home_passes: u32,
    /// Total passes by away team
    pub away_passes: u32,
    /// Pass completion rate by home team (0-1)
    pub home_pass_accuracy: f32,
    /// Pass completion rate by away team (0-1)
    pub away_pass_accuracy: f32,
    /// Total fouls by home team
    pub home_fouls: u32,
    /// Total fouls by away team
    pub away_fouls: u32,
    /// Expected goals home
    pub home_xg: f32,
    /// Expected goals away
    pub away_xg: f32,
}

/// Comparison delta between two match results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsDelta {
    /// Change in total goals (treatment - control)
    pub goals_delta: i32,
    /// Change in home possession (treatment - control)
    pub possession_delta: f32,
    /// Change in total shots (treatment - control)
    pub shots_delta: i32,
    /// Change in shots on target (treatment - control)
    pub shots_on_target_delta: i32,
    /// Change in total passes (treatment - control)
    pub passes_delta: i32,
    /// Change in pass accuracy (treatment - control)
    pub pass_accuracy_delta: f32,
    /// Change in total fouls (treatment - control)
    pub fouls_delta: i32,
    /// Change in total xG (treatment - control)
    pub xg_delta: f32,
}

/// Diff report comparing two experimental runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    /// Experiment ID for control group
    pub control_exp_id: String,
    /// Experiment ID for treatment group
    pub treatment_exp_id: String,
    /// Config hash for control
    pub control_hash: String,
    /// Config hash for treatment
    pub treatment_hash: String,
    /// Seed used for simulation
    pub seed: u64,
    /// Control match statistics
    pub control_stats: MatchStats,
    /// Treatment match statistics
    pub treatment_stats: MatchStats,
    /// Computed deltas
    pub delta: StatsDelta,
    /// Timestamp of comparison (ISO 8601)
    pub timestamp: String,
}

impl DiffReport {
    /// Create a new diff report comparing two match results
    pub fn compare(
        control_exp_id: &str,
        treatment_exp_id: &str,
        control_hash: &str,
        treatment_hash: &str,
        seed: u64,
        control_stats: MatchStats,
        treatment_stats: MatchStats,
    ) -> Self {
        let delta = StatsDelta {
            goals_delta: (treatment_stats.home_goals + treatment_stats.away_goals) as i32
                - (control_stats.home_goals + control_stats.away_goals) as i32,
            possession_delta: treatment_stats.home_possession - control_stats.home_possession,
            shots_delta: (treatment_stats.home_shots + treatment_stats.away_shots) as i32
                - (control_stats.home_shots + control_stats.away_shots) as i32,
            shots_on_target_delta: (treatment_stats.home_shots_on_target
                + treatment_stats.away_shots_on_target) as i32
                - (control_stats.home_shots_on_target + control_stats.away_shots_on_target) as i32,
            passes_delta: (treatment_stats.home_passes + treatment_stats.away_passes) as i32
                - (control_stats.home_passes + control_stats.away_passes) as i32,
            pass_accuracy_delta: ((treatment_stats.home_pass_accuracy
                + treatment_stats.away_pass_accuracy)
                / 2.0)
                - ((control_stats.home_pass_accuracy + control_stats.away_pass_accuracy) / 2.0),
            fouls_delta: (treatment_stats.home_fouls + treatment_stats.away_fouls) as i32
                - (control_stats.home_fouls + control_stats.away_fouls) as i32,
            xg_delta: (treatment_stats.home_xg + treatment_stats.away_xg)
                - (control_stats.home_xg + control_stats.away_xg),
        };

        Self {
            control_exp_id: control_exp_id.to_string(),
            treatment_exp_id: treatment_exp_id.to_string(),
            control_hash: control_hash.to_string(),
            treatment_hash: treatment_hash.to_string(),
            seed,
            control_stats,
            treatment_stats,
            delta,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Check if treatment shows improvement in scoring
    pub fn is_scoring_improved(&self) -> bool {
        self.delta.goals_delta > 0 || self.delta.xg_delta > 0.1
    }

    /// Check if treatment shows improvement in possession
    pub fn is_possession_improved(&self) -> bool {
        self.delta.possession_delta > 2.0
    }

    /// Summary of significant changes
    pub fn summary(&self) -> String {
        let mut changes = Vec::new();

        if self.delta.goals_delta != 0 {
            let dir = if self.delta.goals_delta > 0 { "+" } else { "" };
            changes.push(format!("Goals: {}{}", dir, self.delta.goals_delta));
        }
        if self.delta.possession_delta.abs() > 1.0 {
            changes.push(format!("Possession: {:+.1}%", self.delta.possession_delta));
        }
        if self.delta.shots_delta != 0 {
            let dir = if self.delta.shots_delta > 0 { "+" } else { "" };
            changes.push(format!("Shots: {}{}", dir, self.delta.shots_delta));
        }
        if self.delta.xg_delta.abs() > 0.1 {
            changes.push(format!("xG: {:+.2}", self.delta.xg_delta));
        }

        if changes.is_empty() {
            "No significant changes".to_string()
        } else {
            changes.join(", ")
        }
    }
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ExpConfig::default();
        assert_eq!(config.exp_id, "baseline");
        assert_eq!(config.decision.shoot_xg_threshold, 0.05);
        assert_eq!(config.audacity.scale, 1.0);
        assert_eq!(config.style.tempo_bias, 0.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_aggressive_preset() {
        let config = ExpConfig::aggressive();
        assert_eq!(config.decision.shoot_xg_threshold, 0.03);
        assert!(config.decision.dribble_bias > 0.0);
        assert!(config.audacity.scale > 1.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_conservative_preset() {
        let config = ExpConfig::conservative();
        assert!(config.decision.shoot_xg_threshold > 0.05);
        assert!(config.decision.dribble_bias < 0.0);
        assert!(config.audacity.scale < 1.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_json_parsing() {
        let json = r#"{
            "exp_id": "test_exp",
            "name": "Test Experiment",
            "description": "Test description",
            "decision": {
                "shoot_xg_threshold": 0.04,
                "pass_risk_tolerance": 0.35,
                "dribble_bias": 0.1,
                "through_ball_multiplier": 1.1,
                "cross_multiplier": 0.9
            },
            "audacity": {
                "scale": 1.1,
                "losing_boost": 0.12,
                "late_game_urgency": 1.25
            },
            "style": {
                "tempo_bias": 0.1,
                "width_bias": -0.1,
                "directness_bias": 0.2
            }
        }"#;

        let config = ExpConfig::from_json(json).expect("parse failed");
        assert_eq!(config.exp_id, "test_exp");
        assert_eq!(config.decision.shoot_xg_threshold, 0.04);
        assert_eq!(config.audacity.scale, 1.1);
        assert_eq!(config.style.directness_bias, 0.2);
    }

    #[test]
    fn test_validation_bounds() {
        // Invalid shoot_xg_threshold
        let mut config = ExpConfig::default();
        config.decision.shoot_xg_threshold = 1.5;
        assert!(config.validate().is_err());

        // Invalid audacity scale
        config = ExpConfig::default();
        config.audacity.scale = 3.0;
        assert!(config.validate().is_err());

        // Invalid style bias
        config = ExpConfig::default();
        config.style.tempo_bias = 2.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_hash_determinism() {
        let config1 = ExpConfig::aggressive();
        let config2 = ExpConfig::aggressive();
        assert_eq!(config1.config_hash(), config2.config_hash());

        let config3 = ExpConfig::conservative();
        assert_ne!(config1.config_hash(), config3.config_hash());
    }

    #[test]
    fn test_runtime_params_conversion() {
        let config = ExpConfig::aggressive();
        let params = RuntimeExpParams::from(&config);

        assert_eq!(params.shoot_xg_threshold, config.decision.shoot_xg_threshold);
        assert_eq!(params.audacity_scale, config.audacity.scale);
        assert_eq!(params.tempo_bias, config.style.tempo_bias);
    }

    #[test]
    fn test_partial_json_with_defaults() {
        // JSON with only some fields - others should use defaults
        let json = r#"{
            "exp_id": "partial_test",
            "name": "Partial Test",
            "description": "Only decision params",
            "decision": {
                "shoot_xg_threshold": 0.06
            },
            "audacity": {},
            "style": {}
        }"#;

        let config = ExpConfig::from_json(json).expect("parse failed");
        assert_eq!(config.decision.shoot_xg_threshold, 0.06);
        assert_eq!(config.decision.pass_risk_tolerance, 0.3); // default
        assert_eq!(config.audacity.scale, 1.0); // default
        assert_eq!(config.style.tempo_bias, 0.0); // default
    }

    #[test]
    fn test_diff_report_comparison() {
        let control = MatchStats {
            home_goals: 1,
            away_goals: 0,
            home_possession: 55.0,
            home_shots: 10,
            away_shots: 8,
            home_shots_on_target: 4,
            away_shots_on_target: 2,
            home_passes: 400,
            away_passes: 350,
            home_pass_accuracy: 0.85,
            away_pass_accuracy: 0.80,
            home_fouls: 10,
            away_fouls: 12,
            home_xg: 1.5,
            away_xg: 0.8,
        };

        let treatment = MatchStats {
            home_goals: 2,
            away_goals: 1,
            home_possession: 48.0,
            home_shots: 15,
            away_shots: 10,
            home_shots_on_target: 6,
            away_shots_on_target: 4,
            home_passes: 350,
            away_passes: 380,
            home_pass_accuracy: 0.78,
            away_pass_accuracy: 0.82,
            home_fouls: 14,
            away_fouls: 8,
            home_xg: 2.2,
            away_xg: 1.3,
        };

        let report = DiffReport::compare(
            "baseline",
            "aggressive_v1",
            "hash1",
            "hash2",
            12345,
            control,
            treatment,
        );

        // Goals: (2+1) - (1+0) = 2
        assert_eq!(report.delta.goals_delta, 2);
        // Possession: 48 - 55 = -7
        assert!((report.delta.possession_delta - (-7.0)).abs() < 0.01);
        // Shots: (15+10) - (10+8) = 7
        assert_eq!(report.delta.shots_delta, 7);
        // xG: (2.2+1.3) - (1.5+0.8) = 1.2
        assert!((report.delta.xg_delta - 1.2).abs() < 0.01);

        assert!(report.is_scoring_improved());
        assert!(!report.is_possession_improved());

        let summary = report.summary();
        assert!(summary.contains("Goals: +2"));
        assert!(summary.contains("xG: +1.20"));
    }

    #[test]
    fn test_diff_report_json_serialization() {
        let control = MatchStats::default();
        let treatment = MatchStats {
            home_goals: 1,
            ..Default::default()
        };

        let report = DiffReport::compare(
            "baseline",
            "treatment",
            "h1",
            "h2",
            999,
            control,
            treatment,
        );

        let json = report.to_json().expect("serialization failed");
        assert!(json.contains("\"control_exp_id\": \"baseline\""));
        assert!(json.contains("\"treatment_exp_id\": \"treatment\""));
        assert!(json.contains("\"goals_delta\": 1"));
    }
}
