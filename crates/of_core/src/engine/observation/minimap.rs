//! MiniMap Observation (Google Football SMM style)
//!
//! FIX_2601 Phase 4: TickSnapshot-based spatial observation.

use serde::{Deserialize, Serialize};

use super::common::{find_active_player, to_team_view_pos};
use super::ObservationBuilder;
use crate::engine::physics_constants::field;
use crate::engine::tick_snapshot::TickSnapshot;
use crate::engine::types::Coord10;

// =============================================================================
// MiniMapSpec
// =============================================================================

/// MiniMap specification
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MiniMapSpec {
    /// Width in cells (default: 96)
    pub width: usize,
    /// Height in cells (default: 72)
    pub height: usize,
}

impl Default for MiniMapSpec {
    fn default() -> Self {
        Self { width: 96, height: 72 }
    }
}

// =============================================================================
// MiniMapObservation
// =============================================================================

/// MiniMap observation with spatial planes
///
/// Each plane is a flattened 2D array (row-major order).
///
/// ## Planes
/// - `self_team`: Positions of own team players (1.0 at player cells)
/// - `opponent_team`: Positions of opponent players
/// - `ball`: Ball position
/// - `active_player`: Active/controlled player position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniMapObservation {
    /// Is observing from home team perspective
    pub is_home: bool,
    /// Current simulation tick
    pub tick: u64,
    /// Width in cells
    pub width: usize,
    /// Height in cells
    pub height: usize,
    /// Plane labels for reference
    pub plane_labels: Vec<String>,
    /// Spatial planes (each is width × height, row-major)
    pub planes: Vec<Vec<f32>>,
}

impl MiniMapObservation {
    /// Number of planes in the observation
    pub const PLANE_COUNT: usize = 4;

    /// Plane indices
    pub const PLANE_SELF: usize = 0;
    pub const PLANE_OPPONENT: usize = 1;
    pub const PLANE_BALL: usize = 2;
    pub const PLANE_ACTIVE: usize = 3;

    /// Convert to 3D tensor shape (C, H, W)
    pub fn to_tensor_shape(&self) -> (usize, usize, usize) {
        (self.planes.len(), self.height, self.width)
    }

    /// Get a specific plane as a 2D slice reference
    pub fn get_plane(&self, idx: usize) -> Option<&[f32]> {
        self.planes.get(idx).map(|p| p.as_slice())
    }

    /// Convert all planes to a single flat vector (CHW order)
    pub fn to_flat_chw(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(self.planes.len() * self.width * self.height);
        for plane in &self.planes {
            v.extend_from_slice(plane);
        }
        v
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

// =============================================================================
// MiniMapBuilder
// =============================================================================

/// Builder for MiniMapObservation from TickSnapshot
pub struct MiniMapBuilder {
    spec: MiniMapSpec,
}

impl MiniMapBuilder {
    /// Create a new MiniMapBuilder with the given specification
    pub fn new(spec: MiniMapSpec) -> Self {
        Self { spec }
    }

    /// Create a new MiniMapBuilder with default specification
    pub fn default_spec() -> Self {
        Self::new(MiniMapSpec::default())
    }
}

impl ObservationBuilder for MiniMapBuilder {
    type Output = MiniMapObservation;

    fn build(&self, snapshot: &TickSnapshot, is_home: bool) -> Self::Output {
        let attacks_right = if is_home {
            snapshot.home_attacks_right
        } else {
            !snapshot.home_attacks_right
        };

        let width = self.spec.width.max(1);
        let height = self.spec.height.max(1);

        // 4 planes: self, opponent, ball, active
        let mut planes = vec![vec![0.0; width * height]; MiniMapObservation::PLANE_COUNT];

        // Find active player for this team
        let active_idx = find_active_player(snapshot, is_home);

        // Place players on appropriate planes
        for player in snapshot.players.iter() {
            let is_home_player = player.is_home;
            let is_self_team = is_home_player == is_home;
            let plane_idx = if is_self_team {
                MiniMapObservation::PLANE_SELF
            } else {
                MiniMapObservation::PLANE_OPPONENT
            };

            let tv_pos = to_team_view_pos(player.pos, attacks_right);
            let (cx, cy) = pos_to_cell(tv_pos, width, height);
            planes[plane_idx][cy * width + cx] = 1.0;

            // Mark active player on separate plane
            if active_idx == Some(player.id) {
                planes[MiniMapObservation::PLANE_ACTIVE][cy * width + cx] = 1.0;
            }
        }

        // Place ball
        let ball_tv = to_team_view_pos(snapshot.ball.pos, attacks_right);
        let (bx, by) = pos_to_cell(ball_tv, width, height);
        planes[MiniMapObservation::PLANE_BALL][by * width + bx] = 1.0;

        MiniMapObservation {
            is_home,
            tick: snapshot.tick,
            width,
            height,
            plane_labels: vec![
                "self_team".to_string(),
                "opponent_team".to_string(),
                "ball".to_string(),
                "active_player".to_string(),
            ],
            planes,
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert TeamView position (meters) to cell indices
fn pos_to_cell(pos_m: (f32, f32), width: usize, height: usize) -> (usize, usize) {
    let x_ratio = (pos_m.0 / field::LENGTH_M).clamp(0.0, 1.0);
    let y_ratio = (pos_m.1 / field::WIDTH_M).clamp(0.0, 1.0);

    let cx = ((x_ratio * (width - 1) as f32).round() as usize).min(width - 1);
    let cy = ((y_ratio * (height - 1) as f32).round() as usize).min(height - 1);

    (cx, cy)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tick_snapshot::{
        BallSnap, BallStateTag, GameModeTag, OffBallObjectiveSnap, PlayerSnap, StickyActionsSnap,
        TeamSnap,
    };

    fn create_test_snapshot() -> TickSnapshot {
        TickSnapshot {
            tick: 100,
            minute: 45,
            seed: 42,
            ball: BallSnap {
                state: BallStateTag::Controlled,
                pos: Coord10::from_meters(52.5, 34.0), // Center
                owner: Some(5),
                target_pos: None,
                eta_tick: None,
                intended_receiver: None,
                is_shot: false,
            },
            players: std::array::from_fn(|i| PlayerSnap {
                id: i as u8,
                is_home: i < 11,
                pos: Coord10::from_meters(52.5 + (i as f32 * 2.0), 34.0),
                state: crate::engine::tick_snapshot::PlayerStateTag::Idle,
                stamina: 1.0,
                dist_to_ball: (i as i32 - 5).abs() * 20,
            }),
            teams: TeamSnap {
                home_attacks_right: true,
                home_has_possession: true,
            },
            tackle_cooldowns: [0; 22],
            offball_objectives: [OffBallObjectiveSnap::default(); 22],
            last_pass_target: None,
            home_attacks_right: true,
            player_velocities: [(0.0, 0.0); 22],
            score: (0, 0),
            game_mode: GameModeTag::Normal,
            sticky_actions: [StickyActionsSnap::default(); 22],
        }
    }

    #[test]
    fn test_minimap_builder() {
        let snapshot = create_test_snapshot();
        let builder = MiniMapBuilder::default_spec();
        let obs = builder.build(&snapshot, true);

        assert!(obs.is_home);
        assert_eq!(obs.tick, 100);
        assert_eq!(obs.width, 96);
        assert_eq!(obs.height, 72);
        assert_eq!(obs.planes.len(), 4);
    }

    #[test]
    fn test_minimap_tensor_shape() {
        let snapshot = create_test_snapshot();
        let builder = MiniMapBuilder::default_spec();
        let obs = builder.build(&snapshot, true);

        let shape = obs.to_tensor_shape();
        assert_eq!(shape, (4, 72, 96)); // (C, H, W)
    }

    #[test]
    fn test_minimap_flat_chw() {
        let snapshot = create_test_snapshot();
        let builder = MiniMapBuilder::default_spec();
        let obs = builder.build(&snapshot, true);

        let flat = obs.to_flat_chw();
        assert_eq!(flat.len(), 4 * 72 * 96);
    }

    #[test]
    fn test_pos_to_cell_center() {
        let pos = (52.5, 34.0); // Center of field
        let (cx, cy) = pos_to_cell(pos, 96, 72);
        assert_eq!(cx, 48); // 96 / 2
        assert_eq!(cy, 36); // 72 / 2
    }

    #[test]
    fn test_pos_to_cell_corners() {
        // Origin
        let (cx, cy) = pos_to_cell((0.0, 0.0), 96, 72);
        assert_eq!(cx, 0);
        assert_eq!(cy, 0);

        // Far corner
        let (cx, cy) = pos_to_cell((field::LENGTH_M, field::WIDTH_M), 96, 72);
        assert_eq!(cx, 95);
        assert_eq!(cy, 71);
    }

    #[test]
    fn test_ball_on_plane() {
        let snapshot = create_test_snapshot();
        let builder = MiniMapBuilder::default_spec();
        let obs = builder.build(&snapshot, true);

        // Ball is at center, check it's on the ball plane
        let ball_plane = obs.get_plane(MiniMapObservation::PLANE_BALL).unwrap();
        let center_idx = 36 * 96 + 48; // y=36, x=48
        assert_eq!(ball_plane[center_idx], 1.0);
    }

    #[test]
    fn test_active_player_on_plane() {
        let snapshot = create_test_snapshot();
        let builder = MiniMapBuilder::default_spec();
        let obs = builder.build(&snapshot, true);

        // Player 5 is the ball owner, should be on active plane
        let active_plane = obs.get_plane(MiniMapObservation::PLANE_ACTIVE).unwrap();
        let has_active = active_plane.iter().any(|&v| v == 1.0);
        assert!(has_active);
    }
}
