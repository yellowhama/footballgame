use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Position in meters on the field
/// Coordinates: -0.5 to 105.5 (x), -0.5 to 68.5 (y) with ±0.5m tolerance
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct MeterPos {
    pub x: f64, // Length coordinate (m)
    pub y: f64, // Width coordinate (m)
}

/// Velocity vector in m/s
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct Velocity {
    pub x: f64, // -40 to +40 m/s
    pub y: f64, // -40 to +40 m/s
}

/// Ball state with trajectory
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct BallState {
    pub from: MeterPos,
    pub to: MeterPos,
    pub speed_mps: f64, // 0-35 m/s
    #[serde(default = "default_curve")]
    pub curve: CurveType,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CurveType {
    None,
    Inside,
    Outside,
}

fn default_curve() -> CurveType {
    CurveType::None
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Team {
    Home,
    Away,
}

/// Schema metadata for replay files
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct SchemaInfo {
    pub name: String, // "of_replay"
    pub version: u32, // 1
}

/// Build information for debugging and versioning
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Default)]
pub struct BuildInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub of_core: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gdext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_tag: Option<String>,
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

// Implementation methods
impl MeterPos {
    pub fn swap_axes_in_place(&mut self) {
        std::mem::swap(&mut self.x, &mut self.y);
    }

    /// Validate position is within field boundaries (including ±0.5m tolerance)
    pub fn is_valid(&self) -> bool {
        self.x >= -0.5 && self.x <= 105.5 && self.y >= -0.5 && self.y <= 68.5
    }

    /// Calculate distance between two positions
    pub fn distance_to(&self, other: &MeterPos) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

impl Velocity {
    pub fn swap_axes_in_place(&mut self) {
        std::mem::swap(&mut self.x, &mut self.y);
    }

    /// Check if velocity is within realistic bounds
    pub fn is_valid(&self) -> bool {
        self.x.abs() <= 40.0 && self.y.abs() <= 40.0
    }

    /// Calculate magnitude of velocity vector
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl BallState {
    pub fn swap_axes_in_place(&mut self) {
        self.from.swap_axes_in_place();
        self.to.swap_axes_in_place();
    }

    /// Check if ball state is valid
    pub fn is_valid(&self) -> bool {
        self.from.is_valid()
            && self.to.is_valid()
            && self.speed_mps >= 0.0
            && self.speed_mps <= 35.0
    }

    /// Calculate distance of ball movement
    pub fn distance(&self) -> f64 {
        self.from.distance_to(&self.to)
    }
}

impl Default for SchemaInfo {
    fn default() -> Self {
        Self { name: "of_replay".to_string(), version: 1 }
    }
}

// ============================================================================
// 0108: Communication Intent Events (Phase 2)
// ============================================================================

/// Decision intent logging - captures why a player chose a specific action
/// Used for debugging, analysis, and replay visualization
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct DecisionIntent {
    /// Player making the decision
    pub player_id: u32,
    /// Tick when decision was made
    pub tick: u32,
    /// The action that was chosen (e.g., "SafePass", "TakeOn", "Shot")
    pub chosen_action: String,
    /// Confidence in the choice (0.0-1.0, from Gate B softmax)
    pub confidence: f32,
    /// Top alternative actions considered
    pub alternatives: Vec<ActionAlternative>,
    /// Situational context at decision time
    pub context: IntentContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_utility: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_pos: Option<MeterPos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_pos: Option<MeterPos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_player_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pass_targets: Vec<IntentTarget>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nearby_opponents: Vec<MeterPos>,
}

/// Alternative action that was considered but not chosen
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ActionAlternative {
    /// Action name
    pub action: String,
    /// Probability assigned to this action (0.0-1.0)
    pub probability: f32,
}

/// Pass target candidate (position in meters, quality score)
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct IntentTarget {
    pub player_id: u32,
    pub pos: MeterPos,
    pub quality: f32,
}

/// Context snapshot at decision time
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Default)]
pub struct IntentContext {
    /// Defensive pressure level (0.0 = open, 1.0 = heavily pressed)
    pub pressure_level: f32,
    /// Player stamina (0.0-1.0)
    pub stamina_percent: f32,
    /// True if in attacking third of the field
    pub in_attacking_third: bool,
    /// Distance to ball in meters
    pub ball_distance: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_pos_validation() {
        let valid_pos = MeterPos { x: 52.5, y: 34.0 };
        assert!(valid_pos.is_valid());

        let invalid_pos = MeterPos { x: 200.0, y: 34.0 };
        assert!(!invalid_pos.is_valid());
    }

    #[test]
    fn test_velocity_validation() {
        let valid_vel = Velocity { x: 10.0, y: 15.0 };
        assert!(valid_vel.is_valid());
        assert!((valid_vel.magnitude() - 18.0278).abs() < 0.001);

        let invalid_vel = Velocity { x: 50.0, y: 0.0 };
        assert!(!invalid_vel.is_valid());
    }

    #[test]
    fn test_ball_state_validation() {
        let ball = BallState {
            from: MeterPos { x: 50.0, y: 30.0 },
            to: MeterPos { x: 55.0, y: 35.0 },
            speed_mps: 20.0,
            curve: CurveType::None,
        };
        assert!(ball.is_valid());
        assert!((ball.distance() - 7.071).abs() < 0.001);
    }

    #[test]
    fn test_distance_calculation() {
        let pos1 = MeterPos { x: 0.0, y: 0.0 };
        let pos2 = MeterPos { x: 3.0, y: 4.0 };
        assert_eq!(pos1.distance_to(&pos2), 5.0);
    }
}
