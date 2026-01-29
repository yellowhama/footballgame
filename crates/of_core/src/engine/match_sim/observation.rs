//! Observation Wrappers for ML/AI Pipelines
//!
//! Google Football-style observation formats:
//! - `SimpleVectorObservation`: 115-float vector (positions, directions, game mode)
//! - `MiniMapObservation`: 72×96 spatial planes (self, opponent, ball, active)
//!
//! All observations use TeamView coordinates where:
//! - Own goal is at x=0, opponent goal at x=105m
//! - Eliminates home/away bias for learning algorithms

use serde::{Deserialize, Serialize};

use super::MatchEngine;
use crate::engine::action_queue::{BallState, RestartType};
use crate::engine::physics_constants::field;
use crate::engine::types::{Coord10, DirectionContext, TeamViewCoord10};

// =============================================================================
// Game Mode (for one-hot encoding)
// =============================================================================

/// Game mode for observation encoding (Google Football compatible)
///
/// Maps to 7-element one-hot vector in flat observations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameMode {
    /// Normal play - ball is in play
    Normal = 0,
    /// Kick-off after goal or start of half
    KickOff = 1,
    /// Goal kick from goalkeeper
    GoalKick = 2,
    /// Free kick (direct or indirect)
    FreeKick = 3,
    /// Corner kick
    Corner = 4,
    /// Throw-in from sideline
    ThrowIn = 5,
    /// Penalty kick
    Penalty = 6,
    /// Drop ball - neutral restart
    DropBall = 7,
}

impl GameMode {
    /// Number of game modes (for one-hot encoding)
    pub const COUNT: usize = 8;

    /// Convert to one-hot vector (8 elements)
    pub fn to_one_hot(&self) -> [f32; Self::COUNT] {
        let mut v = [0.0; Self::COUNT];
        v[*self as usize] = 1.0;
        v
    }

    /// Create from BallState
    pub fn from_ball_state(ball_state: &BallState) -> Self {
        match ball_state {
            BallState::Controlled { .. } | BallState::InFlight { .. } | BallState::Loose { .. } => {
                GameMode::Normal
            }
            BallState::OutOfPlay { restart_type, .. } => match restart_type {
                RestartType::KickOff => GameMode::KickOff,
                RestartType::GoalKick => GameMode::GoalKick,
                RestartType::FreeKick => GameMode::FreeKick,
                RestartType::Corner => GameMode::Corner,
                RestartType::ThrowIn => GameMode::ThrowIn,
                RestartType::Penalty => GameMode::Penalty,
                RestartType::DropBall => GameMode::DropBall,
            },
        }
    }
}

// =============================================================================
// Ball Observation
// =============================================================================

/// Ball observation in TeamView coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamViewBallObservation {
    /// Position in meters (TeamView: 0=own goal, 105=opponent goal)
    pub pos_m: (f32, f32),
    /// Velocity in m/s (TeamView aligned)
    pub vel_mps: (f32, f32),
    /// Normalized direction (unit vector, 0 if stationary)
    pub direction: (f32, f32),
    /// Height above ground in meters
    pub height_m: f32,
    /// Owner player index (0-21), None if loose/in-flight
    pub owner_idx: Option<u8>,
}

// =============================================================================
// Player Observation
// =============================================================================

/// Player observation in TeamView coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamViewPlayerObservation {
    /// Track ID (0-21)
    pub track_id: u8,
    /// Team ID: 0=self, 1=opponent (relative to observer)
    pub team_id: u8,
    /// Position in meters (TeamView aligned)
    pub pos_m: (f32, f32),
    /// Velocity in m/s (TeamView aligned)
    pub vel_mps: (f32, f32),
    /// Normalized direction (unit vector)
    pub direction: (f32, f32),
    /// Stamina (0.0=exhausted, 1.0=fresh)
    pub stamina: f32,
    /// Is currently sprinting
    pub is_sprinting: bool,
    /// Is the ball owner
    pub is_ball_owner: bool,
}

// =============================================================================
// Simple Vector Observation (Google Football simple115_v2 style)
// =============================================================================

/// SimpleVector observation for ML pipelines
///
/// Contains all game state in a structured format that can be flattened
/// to a fixed-size vector for neural network input.
///
/// ## Flat Vector Layout (115 floats)
/// ```text
/// [0-1]     Ball position (x, y) normalized to [0, 1]
/// [2-3]     Ball direction (dx, dy) normalized
/// [4-25]    Self team positions (11 × 2)
/// [26-47]   Self team directions (11 × 2)
/// [48-69]   Opponent team positions (11 × 2)
/// [70-91]   Opponent team directions (11 × 2)
/// [92-102]  Active player one-hot (11 elements)
/// [103-105] Sticky actions (sprint, dribble, press)
/// [106-112] Game mode one-hot (7 elements)
/// [113]     Score difference (normalized -1 to 1)
/// [114]     Time remaining (normalized 0 to 1)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleVectorObservation {
    /// Is observing from home team perspective
    pub is_home: bool,
    /// Current simulation tick
    pub tick: u64,
    /// Current minute (0-90+)
    pub minute: u8,
    /// Score (self, opponent) - relative to observer
    pub score: (u8, u8),
    /// Current game mode
    pub game_mode: GameMode,
    /// Ball state
    pub ball: TeamViewBallObservation,
    /// All 22 players (self team first, then opponents)
    pub players: Vec<TeamViewPlayerObservation>,
    /// Active player index (ball owner or nearest to ball for self team)
    pub active_player_idx: Option<u8>,
    /// Sticky actions for active player
    pub sticky_actions: (bool, bool, bool), // (sprint, dribble, press)
}

impl SimpleVectorObservation {
    /// Total size of flat vector output
    pub const FLAT_SIZE: usize = 115;

    /// Convert to flat f32 vector for ML pipelines
    ///
    /// ## Layout (115 floats)
    /// - `[0-1]`: Ball position (normalized 0-1)
    /// - `[2-3]`: Ball direction (unit vector)
    /// - `[4-25]`: Self team positions (11 × 2, normalized)
    /// - `[26-47]`: Self team directions (11 × 2, unit vectors)
    /// - `[48-69]`: Opponent positions (11 × 2, normalized)
    /// - `[70-91]`: Opponent directions (11 × 2, unit vectors)
    /// - `[92-102]`: Active player one-hot (11 elements)
    /// - `[103-105]`: Sticky actions (sprint, dribble, press)
    /// - `[106-112]`: Game mode one-hot (7 elements)
    /// - `[113]`: Score difference (normalized -1 to 1)
    /// - `[114]`: Time remaining (normalized 0 to 1)
    pub fn to_flat_vector(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(Self::FLAT_SIZE);

        // [0-1] Ball position (normalized)
        v.push(self.ball.pos_m.0 / field::LENGTH_M);
        v.push(self.ball.pos_m.1 / field::WIDTH_M);

        // [2-3] Ball direction
        v.push(self.ball.direction.0);
        v.push(self.ball.direction.1);

        // Separate players into self and opponent teams
        let (self_team, opp_team): (Vec<_>, Vec<_>) =
            self.players.iter().partition(|p| p.team_id == 0);

        // [4-25] Self team positions (11 × 2)
        for i in 0..11 {
            if let Some(p) = self_team.get(i) {
                v.push(p.pos_m.0 / field::LENGTH_M);
                v.push(p.pos_m.1 / field::WIDTH_M);
            } else {
                v.push(0.0);
                v.push(0.0);
            }
        }

        // [26-47] Self team directions (11 × 2)
        for i in 0..11 {
            if let Some(p) = self_team.get(i) {
                v.push(p.direction.0);
                v.push(p.direction.1);
            } else {
                v.push(0.0);
                v.push(0.0);
            }
        }

        // [48-69] Opponent positions (11 × 2)
        for i in 0..11 {
            if let Some(p) = opp_team.get(i) {
                v.push(p.pos_m.0 / field::LENGTH_M);
                v.push(p.pos_m.1 / field::WIDTH_M);
            } else {
                v.push(0.0);
                v.push(0.0);
            }
        }

        // [70-91] Opponent directions (11 × 2)
        for i in 0..11 {
            if let Some(p) = opp_team.get(i) {
                v.push(p.direction.0);
                v.push(p.direction.1);
            } else {
                v.push(0.0);
                v.push(0.0);
            }
        }

        // [92-102] Active player one-hot (11 elements for self team)
        let active_self_idx =
            self.active_player_idx.and_then(|idx| self_team.iter().position(|p| p.track_id == idx));
        for i in 0..11 {
            v.push(if active_self_idx == Some(i) { 1.0 } else { 0.0 });
        }

        // [103-105] Sticky actions
        v.push(if self.sticky_actions.0 { 1.0 } else { 0.0 }); // sprint
        v.push(if self.sticky_actions.1 { 1.0 } else { 0.0 }); // dribble
        v.push(if self.sticky_actions.2 { 1.0 } else { 0.0 }); // press

        // [106-112] Game mode one-hot
        v.extend_from_slice(&self.game_mode.to_one_hot());

        // [113] Score difference (normalized to -1..1, capped at ±5)
        let score_diff = self.score.0 as i32 - self.score.1 as i32;
        v.push((score_diff as f32 / 5.0).clamp(-1.0, 1.0));

        // [114] Time remaining (normalized, 90 minutes = 1.0)
        v.push(1.0 - (self.minute as f32 / 90.0).clamp(0.0, 1.0));

        debug_assert_eq!(v.len(), Self::FLAT_SIZE);
        v
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Convert to pretty-printed JSON
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

// =============================================================================
// MiniMap Observation (Google Football SMM style)
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
// MatchEngine Observation Builders
// =============================================================================

impl MatchEngine {
    /// Build a TeamView-aligned vector observation for the given team.
    ///
    /// Returns a structured observation that can be converted to a flat vector
    /// using `to_flat_vector()` for ML pipelines.
    pub fn build_team_view_simple_observation(&self, is_home: bool) -> SimpleVectorObservation {
        let ctx = self.team_view_context(is_home);

        // Ball observation
        let ball_tv = ctx.to_team_view(self.ball.position);
        let (ball_vx, ball_vy) = self.ball.velocity.to_mps();
        let ball_vel_tv = to_team_view_velocity(ctx, (ball_vx, ball_vy));
        let ball_dir = normalize_direction(ball_vel_tv);

        // Get game mode from action queue
        let game_mode = GameMode::from_ball_state(self.action_queue.ball_state());

        // Get ball owner
        let ball_owner_idx = self.ball.current_owner;

        // Build player observations
        let mut players = Vec::with_capacity(22);
        for idx in 0..22 {
            let pos = self.player_positions[idx];
            let pos_tv = ctx.to_team_view(pos);
            let vel = self.player_velocities[idx];
            let vel_tv = to_team_view_velocity(ctx, vel);
            let direction = normalize_direction(vel_tv);

            // Determine team_id: 0=self, 1=opponent (relative to observer)
            let is_home_player = idx < 11;
            let is_self_team = is_home_player == is_home;
            let team_id = if is_self_team { 0 } else { 1 };

            players.push(TeamViewPlayerObservation {
                track_id: idx as u8,
                team_id,
                pos_m: pos_tv.to_meters(),
                vel_mps: vel_tv,
                direction,
                stamina: self.stamina[idx],
                is_sprinting: self.sprint_state[idx],
                is_ball_owner: ball_owner_idx == Some(idx),
            });
        }

        // Find active player (ball owner or nearest self-team player to ball)
        let active_player_idx = self.find_active_player(is_home);

        // Get sticky actions for active player
        let sticky_actions = active_player_idx
            .map(|idx| {
                let s = &self.sticky_actions[idx as usize];
                (s.sprint, s.dribble, s.press)
            })
            .unwrap_or((false, false, false));

        // Score relative to observer
        let (home_score, away_score) = self.get_score();
        let score = if is_home { (home_score, away_score) } else { (away_score, home_score) };

        SimpleVectorObservation {
            is_home,
            tick: self.current_tick,
            minute: self.minute,
            score,
            game_mode,
            ball: TeamViewBallObservation {
                pos_m: ball_tv.to_meters(),
                vel_mps: ball_vel_tv,
                direction: ball_dir,
                height_m: self.ball.height as f32 / 10.0,
                owner_idx: ball_owner_idx.map(|idx| idx as u8),
            },
            players,
            active_player_idx,
            sticky_actions,
        }
    }

    /// Build a TeamView-aligned minimap observation (SMM-style planes).
    ///
    /// Returns a spatial observation with 4 planes:
    /// - Plane 0: Self team positions
    /// - Plane 1: Opponent team positions
    /// - Plane 2: Ball position
    /// - Plane 3: Active player position
    pub fn build_team_view_minimap_observation(
        &self,
        is_home: bool,
        spec: MiniMapSpec,
    ) -> MiniMapObservation {
        let ctx = self.team_view_context(is_home);
        let width = spec.width.max(1);
        let height = spec.height.max(1);

        // 4 planes: self, opponent, ball, active
        let mut planes = vec![vec![0.0; width * height]; MiniMapObservation::PLANE_COUNT];

        // Find active player for this team
        let active_idx = self.find_active_player(is_home);

        // Place players on appropriate planes
        for (idx, pos) in self.player_positions.iter().enumerate() {
            let is_home_player = idx < 11;
            let is_self_team = is_home_player == is_home;
            let plane_idx = if is_self_team {
                MiniMapObservation::PLANE_SELF
            } else {
                MiniMapObservation::PLANE_OPPONENT
            };

            let tv = clamp_team_view(ctx.to_team_view(*pos));
            let (cx, cy) = team_view_to_cell(tv, width, height);
            planes[plane_idx][cy * width + cx] = 1.0;

            // Mark active player on separate plane
            if active_idx == Some(idx as u8) {
                planes[MiniMapObservation::PLANE_ACTIVE][cy * width + cx] = 1.0;
            }
        }

        // Place ball
        let ball_tv = clamp_team_view(ctx.to_team_view(self.ball.position));
        let (bx, by) = team_view_to_cell(ball_tv, width, height);
        planes[MiniMapObservation::PLANE_BALL][by * width + bx] = 1.0;

        MiniMapObservation {
            is_home,
            tick: self.current_tick,
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

    /// Get direction context for the given team
    fn team_view_context(&self, is_home: bool) -> DirectionContext {
        if is_home {
            self.home_ctx
        } else {
            self.away_ctx
        }
    }

    /// Find the active player for the given team
    ///
    /// Returns ball owner if on this team, otherwise nearest player to ball
    fn find_active_player(&self, is_home: bool) -> Option<u8> {
        // If ball owner is on this team, they're active
        if let Some(owner_idx) = self.ball.current_owner {
            let is_home_player = owner_idx < 11;
            if is_home_player == is_home {
                return Some(owner_idx as u8);
            }
        }

        // Otherwise, find nearest player to ball on this team
        let team_range = if is_home { 0..11 } else { 11..22 };
        let ball_pos = self.ball.position;

        let mut min_dist = i32::MAX;
        let mut nearest_idx = None;

        for idx in team_range {
            let dist = self.player_positions[idx].distance_to(&ball_pos);
            if dist < min_dist {
                min_dist = dist;
                nearest_idx = Some(idx as u8);
            }
        }

        nearest_idx
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Transform velocity to TeamView coordinates
fn to_team_view_velocity(ctx: DirectionContext, vel_mps: (f32, f32)) -> (f32, f32) {
    if ctx.attacks_right {
        vel_mps
    } else {
        (-vel_mps.0, vel_mps.1)
    }
}

/// Normalize a velocity vector to a unit direction vector
fn normalize_direction(vel: (f32, f32)) -> (f32, f32) {
    let mag = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
    if mag < 0.001 {
        (0.0, 0.0)
    } else {
        (vel.0 / mag, vel.1 / mag)
    }
}

/// Clamp TeamView coordinates to field bounds
fn clamp_team_view(tv: TeamViewCoord10) -> TeamViewCoord10 {
    TeamViewCoord10 {
        x: tv.x.clamp(0, Coord10::FIELD_LENGTH_10),
        y: tv.y.clamp(0, Coord10::FIELD_WIDTH_10),
    }
}

/// Convert TeamView coordinates to cell indices on the minimap grid
fn team_view_to_cell(tv: TeamViewCoord10, width: usize, height: usize) -> (usize, usize) {
    let width_f = (width - 1) as f32;
    let height_f = (height - 1) as f32;
    let x_ratio = tv.x as f32 / Coord10::FIELD_LENGTH_10 as f32;
    let y_ratio = tv.y as f32 / Coord10::FIELD_WIDTH_10 as f32;
    let cx = (x_ratio * width_f).round() as i32;
    let cy = (y_ratio * height_f).round() as i32;
    let cx = cx.clamp(0, width as i32 - 1) as usize;
    let cy = cy.clamp(0, height as i32 - 1) as usize;
    (cx, cy)
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_mode_one_hot() {
        let mode = GameMode::Normal;
        let one_hot = mode.to_one_hot();
        assert_eq!(one_hot.len(), 8); // 7 + DropBall
        assert_eq!(one_hot[0], 1.0);
        assert_eq!(one_hot[1], 0.0);

        let mode = GameMode::Corner;
        let one_hot = mode.to_one_hot();
        assert_eq!(one_hot[4], 1.0);
        assert_eq!(one_hot[0], 0.0);

        let mode = GameMode::DropBall;
        let one_hot = mode.to_one_hot();
        assert_eq!(one_hot[7], 1.0);
        assert_eq!(one_hot[0], 0.0);
    }

    #[test]
    fn test_normalize_direction() {
        // Zero velocity
        let dir = normalize_direction((0.0, 0.0));
        assert_eq!(dir, (0.0, 0.0));

        // Unit vector
        let dir = normalize_direction((3.0, 4.0));
        assert!((dir.0 - 0.6).abs() < 0.001);
        assert!((dir.1 - 0.8).abs() < 0.001);

        // Already unit
        let dir = normalize_direction((1.0, 0.0));
        assert!((dir.0 - 1.0).abs() < 0.001);
        assert!((dir.1 - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_team_view_to_cell() {
        let spec = MiniMapSpec::default();

        // Center of field
        let center = TeamViewCoord10 { x: 525, y: 340 };
        let (cx, cy) = team_view_to_cell(center, spec.width, spec.height);
        assert_eq!(cx, 48); // 96 / 2 = 48
        assert_eq!(cy, 36); // 72 / 2 = 36

        // Origin (own goal)
        let origin = TeamViewCoord10 { x: 0, y: 0 };
        let (cx, cy) = team_view_to_cell(origin, spec.width, spec.height);
        assert_eq!(cx, 0);
        assert_eq!(cy, 0);

        // Far corner (opponent goal)
        let far = TeamViewCoord10 { x: field::LENGTH_COORD10, y: field::WIDTH_COORD10 };
        let (cx, cy) = team_view_to_cell(far, spec.width, spec.height);
        assert_eq!(cx, 95); // width - 1
        assert_eq!(cy, 71); // height - 1
    }

    #[test]
    fn test_simple_vector_flat_size() {
        // Verify the constant matches the actual layout
        // Ball: 2 pos + 2 dir = 4
        // Self positions: 11 * 2 = 22
        // Self directions: 11 * 2 = 22
        // Opp positions: 11 * 2 = 22
        // Opp directions: 11 * 2 = 22
        // Active one-hot: 11
        // Sticky: 3
        // Game mode: 7
        // Score diff: 1
        // Time remaining: 1
        // Total: 4 + 22 + 22 + 22 + 22 + 11 + 3 + 7 + 1 + 1 = 115
        assert_eq!(SimpleVectorObservation::FLAT_SIZE, 115);
    }

    #[test]
    fn test_minimap_plane_count() {
        assert_eq!(MiniMapObservation::PLANE_COUNT, 4);
        assert_eq!(MiniMapObservation::PLANE_SELF, 0);
        assert_eq!(MiniMapObservation::PLANE_OPPONENT, 1);
        assert_eq!(MiniMapObservation::PLANE_BALL, 2);
        assert_eq!(MiniMapObservation::PLANE_ACTIVE, 3);
    }

    #[test]
    fn test_minimap_tensor_shape() {
        let spec = MiniMapSpec::default();
        let obs = MiniMapObservation {
            is_home: true,
            tick: 0,
            width: spec.width,
            height: spec.height,
            plane_labels: vec![
                "self_team".into(),
                "opponent_team".into(),
                "ball".into(),
                "active_player".into(),
            ],
            planes: vec![vec![0.0; spec.width * spec.height]; 4],
        };

        let shape = obs.to_tensor_shape();
        assert_eq!(shape, (4, 72, 96)); // (C, H, W)

        let flat = obs.to_flat_chw();
        assert_eq!(flat.len(), 4 * 72 * 96);
    }

    #[test]
    fn test_to_team_view_velocity() {
        let ctx_right = DirectionContext::new(true); // attacks right
        let ctx_left = DirectionContext::new(false); // attacks left

        let vel = (5.0, 3.0);

        // Attacks right: no change
        let tv = to_team_view_velocity(ctx_right, vel);
        assert_eq!(tv, (5.0, 3.0));

        // Attacks left: x flipped
        let tv = to_team_view_velocity(ctx_left, vel);
        assert_eq!(tv, (-5.0, 3.0));
    }

    #[test]
    fn test_clamp_team_view() {
        // Within bounds
        let tv = TeamViewCoord10 { x: 500, y: 300 };
        let clamped = clamp_team_view(tv);
        assert_eq!(clamped.x, 500);
        assert_eq!(clamped.y, 300);

        // Out of bounds
        let tv = TeamViewCoord10 { x: -100, y: 1000 };
        let clamped = clamp_team_view(tv);
        assert_eq!(clamped.x, 0);
        assert_eq!(clamped.y, Coord10::FIELD_WIDTH_10);
    }
}
