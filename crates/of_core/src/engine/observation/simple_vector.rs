//! SimpleVector Observation (Google Football simple115_v2 style)
//!
//! FIX_2601 Phase 4: TickSnapshot-based flat vector observation.

use serde::{Deserialize, Serialize};

use super::common::{find_active_player, normalize_direction, to_team_view_pos, to_team_view_vel};
use super::ObservationBuilder;
use crate::engine::physics_constants::field;
use crate::engine::tick_snapshot::{GameModeTag, TickSnapshot};
use crate::engine::types::Coord10;

// =============================================================================
// TeamView Ball Observation
// =============================================================================

/// Ball observation in TeamView coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamViewBallObs {
    /// Position in meters (TeamView: 0=own goal, 105=opponent goal)
    pub pos_m: (f32, f32),
    /// Velocity in m/s (TeamView aligned)
    pub vel_mps: (f32, f32),
    /// Normalized direction (unit vector, 0 if stationary)
    pub direction: (f32, f32),
    /// Owner player index (0-21), None if loose/in-flight
    pub owner_idx: Option<u8>,
}

// =============================================================================
// TeamView Player Observation
// =============================================================================

/// Player observation in TeamView coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamViewPlayerObs {
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
    /// Is the ball owner
    pub is_ball_owner: bool,
}

// =============================================================================
// SimpleVectorObservation
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
/// [106-113] Game mode one-hot (8 elements)
/// [114]     Score difference (normalized -1 to 1)
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
    pub game_mode: GameModeTag,
    /// Ball state
    pub ball: TeamViewBallObs,
    /// All 22 players (self team first, then opponents)
    pub players: Vec<TeamViewPlayerObs>,
    /// Active player index (ball owner or nearest to ball for self team)
    pub active_player_idx: Option<u8>,
    /// Sticky actions for active player
    pub sticky_actions: (bool, bool, bool), // (sprint, dribble, press)
}

impl SimpleVectorObservation {
    /// Total size of flat vector output
    pub const FLAT_SIZE: usize = 115;

    /// Convert to flat f32 vector for ML pipelines
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

        // [106-113] Game mode one-hot (8 elements)
        v.extend_from_slice(&self.game_mode.to_one_hot());

        // [114] Score difference (normalized to -1..1, capped at ±5)
        let score_diff = self.score.0 as i32 - self.score.1 as i32;
        v.push((score_diff as f32 / 5.0).clamp(-1.0, 1.0));

        debug_assert_eq!(v.len(), Self::FLAT_SIZE);
        v
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

// =============================================================================
// SimpleVectorBuilder
// =============================================================================

/// Builder for SimpleVectorObservation from TickSnapshot
pub struct SimpleVectorBuilder;

impl ObservationBuilder for SimpleVectorBuilder {
    type Output = SimpleVectorObservation;

    fn build(&self, snapshot: &TickSnapshot, is_home: bool) -> Self::Output {
        let attacks_right = if is_home {
            snapshot.home_attacks_right
        } else {
            !snapshot.home_attacks_right
        };

        // Ball observation
        let ball_pos = snapshot.ball.pos;
        let ball_tv_pos = to_team_view_pos(ball_pos, attacks_right);
        let ball_vel = (0.0, 0.0); // TickSnapshot doesn't have ball velocity yet
        let ball_tv_vel = to_team_view_vel(ball_vel, attacks_right);
        let ball_dir = normalize_direction(ball_tv_vel);

        // Build player observations
        let mut players = Vec::with_capacity(22);
        for idx in 0..22 {
            let player = &snapshot.players[idx];
            let pos_tv = to_team_view_pos(player.pos, attacks_right);
            let vel = snapshot.player_velocities[idx];
            let vel_tv = to_team_view_vel(vel, attacks_right);
            let direction = normalize_direction(vel_tv);

            // Determine team_id: 0=self, 1=opponent (relative to observer)
            let is_home_player = player.is_home;
            let is_self_team = is_home_player == is_home;
            let team_id = if is_self_team { 0 } else { 1 };

            players.push(TeamViewPlayerObs {
                track_id: player.id,
                team_id,
                pos_m: pos_tv,
                vel_mps: vel_tv,
                direction,
                stamina: player.stamina,
                is_ball_owner: snapshot.ball.owner == Some(player.id),
            });
        }

        // Find active player (ball owner or nearest self-team player to ball)
        let active_player_idx = find_active_player(snapshot, is_home);

        // Get sticky actions for active player
        let sticky_actions = active_player_idx
            .map(|idx| snapshot.sticky_actions[idx as usize].to_tuple())
            .unwrap_or((false, false, false));

        // Score relative to observer
        let (home_score, away_score) = snapshot.score;
        let score = if is_home {
            (home_score, away_score)
        } else {
            (away_score, home_score)
        };

        SimpleVectorObservation {
            is_home,
            tick: snapshot.tick,
            minute: snapshot.minute,
            score,
            game_mode: snapshot.game_mode,
            ball: TeamViewBallObs {
                pos_m: ball_tv_pos,
                vel_mps: ball_tv_vel,
                direction: ball_dir,
                owner_idx: snapshot.ball.owner,
            },
            players,
            active_player_idx,
            sticky_actions,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tick_snapshot::{
        BallSnap, BallStateTag, OffBallObjectiveSnap, PlayerSnap, StickyActionsSnap, TeamSnap,
    };

    fn create_test_snapshot() -> TickSnapshot {
        TickSnapshot {
            tick: 100,
            minute: 45,
            seed: 42,
            ball: BallSnap {
                state: BallStateTag::Controlled,
                pos: Coord10::from_meters(52.5, 34.0),
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
            player_velocities: std::array::from_fn(|i| (1.0 + i as f32 * 0.1, 0.5)),
            score: (2, 1),
            game_mode: GameModeTag::Normal,
            sticky_actions: std::array::from_fn(|i| {
                if i == 5 {
                    StickyActionsSnap::new(true, false, false)
                } else {
                    StickyActionsSnap::default()
                }
            }),
        }
    }

    #[test]
    fn test_simple_vector_builder_home() {
        let snapshot = create_test_snapshot();
        let builder = SimpleVectorBuilder;
        let obs = builder.build(&snapshot, true);

        assert!(obs.is_home);
        assert_eq!(obs.tick, 100);
        assert_eq!(obs.minute, 45);
        assert_eq!(obs.score, (2, 1)); // Home perspective
        assert_eq!(obs.active_player_idx, Some(5)); // Ball owner
        assert!(obs.sticky_actions.0); // Sprint active
    }

    #[test]
    fn test_simple_vector_builder_away() {
        let snapshot = create_test_snapshot();
        let builder = SimpleVectorBuilder;
        let obs = builder.build(&snapshot, false);

        assert!(!obs.is_home);
        assert_eq!(obs.score, (1, 2)); // Away perspective (flipped)
    }

    #[test]
    fn test_flat_vector_size() {
        let snapshot = create_test_snapshot();
        let builder = SimpleVectorBuilder;
        let obs = builder.build(&snapshot, true);
        let flat = obs.to_flat_vector();

        assert_eq!(flat.len(), SimpleVectorObservation::FLAT_SIZE);
        assert_eq!(flat.len(), 115);
    }

    #[test]
    fn test_normalize_direction() {
        let dir = normalize_direction((3.0, 4.0));
        assert!((dir.0 - 0.6).abs() < 0.001);
        assert!((dir.1 - 0.8).abs() < 0.001);

        let dir = normalize_direction((0.0, 0.0));
        assert_eq!(dir, (0.0, 0.0));
    }

    #[test]
    fn test_team_view_conversion() {
        let pos = Coord10::from_meters(80.0, 34.0);

        // Attacks right: no change
        let tv = to_team_view_pos(pos, true);
        assert!((tv.0 - 80.0).abs() < 0.1);

        // Attacks left: x flipped
        let tv = to_team_view_pos(pos, false);
        assert!((tv.0 - 25.0).abs() < 0.1); // 105 - 80 = 25
    }
}
