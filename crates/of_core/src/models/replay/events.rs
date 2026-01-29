use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::{BallState, MeterPos, Team, Velocity};

/// Additional metadata for events
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct EventMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>, // 0.0-1.0
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

/// Base event structure - all events inherit from this
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct BaseEvent {
    pub t: f64,      // Time in seconds from match start
    pub minute: u32, // Match minute (0-130)
    pub team: Team,  // HOME or AWAY
    pub player_id: String,
    pub pos: MeterPos, // Player position when event occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vel: Option<Velocity>, // Player velocity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<EventMeta>, // Additional metadata
}

/// Event enumeration - all possible event types
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(tag = "etype")]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Pass(PassEvent),
    Dribble(DribbleEvent),
    Shot(ShotEvent),
    Save(SaveEvent),
    Tackle(TackleEvent),
    Foul(FoulEvent),
    SetPiece(SetPieceEvent),
    Substitution(SubstitutionEvent),
    /// Ball possession change event (0108: Open-Football Integration)
    Possession(PossessionEvent),
}

/// Pass event with receiver and outcome
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct PassEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub end_pos: MeterPos,
    pub receiver_id: String,
    #[serde(default = "default_ground")]
    pub ground: bool, // true = ground pass, false = aerial
    pub ball: BallState,
    pub outcome: PassOutcome,
    // Phase 3: Pass quality details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_m: Option<f64>, // Pass distance in meters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passing_skill: Option<f32>, // Player passing skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision: Option<f32>, // Player vision skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub technique: Option<f32>, // Player technique skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<f32>, // Pass power/force

    // 0108 Phase 4: Tactical metadata
    /// Intercept danger level (0.0 = safe, 1.0 = high risk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub danger_level: Option<f32>,
    /// Switch of play: lateral pass covering >40% of field width
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_switch_of_play: Option<bool>,
    /// Line-breaking pass: pass that bypasses 2+ defensive lines
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_line_breaking: Option<bool>,
    /// Through ball: pass into space behind defensive line
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_through_ball: Option<bool>,

    // FIX_2601/0123: Intended target position at selection time
    // Used for accurate forward_pass_rate calculation in QA metrics
    /// Position of the intended target at the moment of pass decision (not current position)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intended_target_pos: Option<MeterPos>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PassOutcome {
    Complete,
    Intercepted,
    Out,
}

fn default_ground() -> bool {
    true
}

/// Dribble event with path and beaten players
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct DribbleEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub path: Vec<MeterPos>, // Dribble path points
    pub end_pos: MeterPos,
    #[serde(default)]
    pub beats: Vec<String>, // IDs of beaten players (unique)
    pub outcome: DribbleOutcome,
    // Phase 3: Dribble quality details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>, // Overall dribble success
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponents_evaded: Option<u32>, // Number of opponents beaten
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_gained: Option<f32>, // Meters of space gained
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressure_level: Option<f32>, // 0.0-1.0 defensive pressure faced
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dribbling_skill: Option<f32>, // Player dribbling skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agility: Option<f32>, // Player agility
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DribbleOutcome {
    Kept,   // Successfully kept possession
    Tackle, // Lost to tackle
    Foul,   // Lost to foul
    Out,    // Ball went out of bounds
}

/// Shot event with xG and target
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ShotEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub target: MeterPos, // Where shot was aimed
    pub xg: f64,          // Expected goals (0.0-1.0)
    pub on_target: bool,  // Shot on target?
    pub ball: BallState,
    pub outcome: ShotOutcome,
    // Phase 3: Shot context details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shot_type: Option<String>, // "volley", "header", "placed", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defender_pressure: Option<f32>, // 0.0-1.0 (0=open, 1=heavily marked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub angle_to_goal: Option<f32>, // Degrees from goal center
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_to_goal: Option<f32>, // Meters to goal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composure: Option<f32>, // Player composure skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finishing_skill: Option<f32>, // Player finishing skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curve_factor: Option<f32>, // Curve/spin on the shot
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShotOutcome {
    Goal,  // Goal scored
    Saved, // Saved by goalkeeper
    Post,  // Hit post/crossbar
    Off,   // Off target
}

/// Save event by goalkeeper
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct SaveEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub ball: BallState, // Ball trajectory being saved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parry_to: Option<MeterPos>, // Where ball was parried to
    // Phase 3: GK save quality details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shot_from: Option<MeterPos>, // Where shot originated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shot_power: Option<f32>, // Power of the shot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_difficulty: Option<f32>, // 0.0-1.0 (0=easy, 1=world class)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflexes_skill: Option<f32>, // GK reflexes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handling_skill: Option<f32>, // GK handling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diving_skill: Option<f32>, // GK diving
}

/// Tackle event between players
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct TackleEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub opponent_id: String, // Player being tackled
    pub success: bool,       // Successful tackle?
}

/// Foul event with card
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct FoulEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub opponent_id: String, // Player fouled
    #[serde(default = "default_card")]
    pub card: CardType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    None,
    Yellow,
    Red,
}

fn default_card() -> CardType {
    CardType::None
}

/// Set piece event (free kick, corner, etc.)
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct SetPieceEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub kind: SetPieceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ball: Option<BallState>, // Ball movement if taken
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SetPieceKind {
    FreeKick,
    Corner,
    Penalty,
    ThrowIn,
    /// Goal kick restart (ENGINE_CONTRACT Section 5)
    GoalKick,
    /// Kickoff (match start, after goal)
    KickOff,
}

/// Substitution event
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct SubstitutionEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    pub out_id: String, // Player going off
    pub in_id: String,  // Player coming on
}

/// Possession change event (0108: Open-Football Integration)
/// Tracks when and how ball possession changes between teams/players
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct PossessionEvent {
    #[serde(flatten)]
    pub base: BaseEvent,
    /// Type of possession change
    pub change_type: PossessionChangeType,
    /// Previous ball owner (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_owner_id: Option<String>,
    /// Previous team in possession
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_team: Option<Team>,
}

/// How the possession was gained or lost
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PossessionChangeType {
    // Gaining possession
    /// Won the ball via tackle
    Tackle,
    /// Intercepted a pass
    Interception,
    /// Picked up a loose ball
    LooseBall,
    /// Received a pass successfully
    PassReceive,
    /// Goalkeeper collected the ball
    GkCollect,
    /// Won an aerial duel
    AerialWon,

    // Losing possession
    /// Lost the ball to a tackle
    Tackled,
    /// Pass was intercepted
    PassIntercepted,
    /// Ball went out of bounds
    OutOfBounds,
    /// Gave away possession (bad touch, etc.)
    Dispossessed,
    /// Shot taken (ends possession)
    ShotTaken,

    // Neutral
    /// Ball contested between players
    Contested,
}

// Event utility methods
impl Event {
    /// Get the base event data for any event type
    pub fn base(&self) -> &BaseEvent {
        match self {
            Event::Pass(e) => &e.base,
            Event::Dribble(e) => &e.base,
            Event::Shot(e) => &e.base,
            Event::Save(e) => &e.base,
            Event::Tackle(e) => &e.base,
            Event::Foul(e) => &e.base,
            Event::SetPiece(e) => &e.base,
            Event::Substitution(e) => &e.base,
            Event::Possession(e) => &e.base,
        }
    }

    /// Get mutable reference to base event data
    pub fn base_mut(&mut self) -> &mut BaseEvent {
        match self {
            Event::Pass(e) => &mut e.base,
            Event::Dribble(e) => &mut e.base,
            Event::Shot(e) => &mut e.base,
            Event::Save(e) => &mut e.base,
            Event::Tackle(e) => &mut e.base,
            Event::Foul(e) => &mut e.base,
            Event::SetPiece(e) => &mut e.base,
            Event::Substitution(e) => &mut e.base,
            Event::Possession(e) => &mut e.base,
        }
    }

    pub fn swap_axes_in_place(&mut self) {
        match self {
            Event::Pass(e) => {
                e.base.swap_axes_in_place();
                e.end_pos.swap_axes_in_place();
                e.ball.swap_axes_in_place();
            }
            Event::Dribble(e) => {
                e.base.swap_axes_in_place();
                for p in &mut e.path {
                    p.swap_axes_in_place();
                }
                e.end_pos.swap_axes_in_place();
            }
            Event::Shot(e) => {
                e.base.swap_axes_in_place();
                e.target.swap_axes_in_place();
                e.ball.swap_axes_in_place();
            }
            Event::Save(e) => {
                e.base.swap_axes_in_place();
                e.ball.swap_axes_in_place();
                if let Some(p) = e.parry_to.as_mut() {
                    p.swap_axes_in_place();
                }
                if let Some(p) = e.shot_from.as_mut() {
                    p.swap_axes_in_place();
                }
            }
            Event::Tackle(e) => e.base.swap_axes_in_place(),
            Event::Foul(e) => e.base.swap_axes_in_place(),
            Event::SetPiece(e) => {
                e.base.swap_axes_in_place();
                if let Some(ball) = e.ball.as_mut() {
                    ball.swap_axes_in_place();
                }
            }
            Event::Substitution(e) => e.base.swap_axes_in_place(),
            Event::Possession(e) => e.base.swap_axes_in_place(),
        }
    }

    /// Get the event type as string
    pub fn event_type(&self) -> &str {
        match self {
            Event::Pass(_) => "pass",
            Event::Dribble(_) => "dribble",
            Event::Shot(_) => "shot",
            Event::Save(_) => "save",
            Event::Tackle(_) => "tackle",
            Event::Foul(_) => "foul",
            Event::SetPiece(_) => "set_piece",
            Event::Substitution(_) => "substitution",
            Event::Possession(_) => "possession",
        }
    }

    /// Check if this is a ball-in-play event (not substitution)
    pub fn is_ball_event(&self) -> bool {
        !matches!(self, Event::Substitution(_))
    }

    /// Check if this event involves the ball changing possession
    pub fn is_possession_change(&self) -> bool {
        matches!(
            self,
            Event::Pass(PassEvent { outcome: PassOutcome::Intercepted, .. })
                | Event::Tackle(TackleEvent { success: true, .. })
                | Event::Foul(_)
                | Event::Shot(_)
                | Event::Possession(_)
        )
    }
}

impl BaseEvent {
    /// Create a new base event with required fields
    pub fn new(t: f64, minute: u32, team: Team, player_id: String, pos: MeterPos) -> Self {
        Self { t, minute, team, player_id, pos, vel: None, meta: None }
    }

    /// Add velocity to the event
    pub fn with_velocity(mut self, vel: Velocity) -> Self {
        self.vel = Some(vel);
        self
    }

    /// Add metadata to the event
    pub fn with_meta(mut self, meta: EventMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn swap_axes_in_place(&mut self) {
        self.pos.swap_axes_in_place();
        if let Some(vel) = self.vel.as_mut() {
            vel.swap_axes_in_place();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::replay::types::{BallState, CurveType, MeterPos};

    #[test]
    fn test_event_base_access() {
        let event = Event::Shot(ShotEvent {
            base: BaseEvent::new(
                613.1,
                10,
                Team::Home,
                "H9".to_string(),
                MeterPos { x: 83.2, y: 27.0 },
            ),
            target: MeterPos { x: 105.0, y: 33.8 },
            xg: 0.18,
            on_target: true,
            ball: BallState {
                from: MeterPos { x: 83.2, y: 27.0 },
                to: MeterPos { x: 105.0, y: 33.8 },
                speed_mps: 26.2,
                curve: CurveType::None,
            },
            outcome: ShotOutcome::Goal,
            // Phase 3 fields
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            finishing_skill: None,
            curve_factor: None,
        });

        assert_eq!(event.base().t, 613.1);
        assert_eq!(event.event_type(), "shot");
        assert!(event.is_ball_event());
        assert!(event.is_possession_change());
    }

    #[test]
    fn test_base_event_builder() {
        let base =
            BaseEvent::new(100.0, 2, Team::Away, "A5".to_string(), MeterPos { x: 40.0, y: 30.0 })
                .with_velocity(Velocity { x: 5.0, y: -2.0 });

        assert_eq!(base.t, 100.0);
        assert_eq!(base.team, Team::Away);
        assert!(base.vel.is_some());
        assert_eq!(base.vel.unwrap().x, 5.0);
    }
}
