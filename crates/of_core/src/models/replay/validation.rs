use super::{
    events::{
        DribbleEvent, Event, FoulEvent, PassEvent, PassOutcome, SaveEvent, SetPieceEvent,
        ShotEvent, SubstitutionEvent, TackleEvent,
    },
    match_info::{AreasSpec, GoalSpec, MatchInfo, PitchSpec},
    types::{BallState, MeterPos, Team, Velocity},
};

/// Validation error types
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    Schema(String),
    MatchInfo(String),
    Position(String),
    Velocity(String),
    BallState(String),
    Event(String),
    Timing(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::Schema(msg) => write!(f, "Schema error: {}", msg),
            ValidationError::MatchInfo(msg) => write!(f, "Match info error: {}", msg),
            ValidationError::Position(msg) => write!(f, "Position error: {}", msg),
            ValidationError::Velocity(msg) => write!(f, "Velocity error: {}", msg),
            ValidationError::BallState(msg) => write!(f, "Ball state error: {}", msg),
            ValidationError::Event(msg) => write!(f, "Event error: {}", msg),
            ValidationError::Timing(msg) => write!(f, "Timing error: {}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<String> for ValidationError {
    fn from(msg: String) -> Self {
        ValidationError::Event(msg)
    }
}

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Trait for validatable types
pub trait Validate {
    fn validate(&self) -> ValidationResult<()>;
}

/// Context for validation that includes match specifications
pub struct ValidationContext<'a> {
    pub match_info: &'a MatchInfo,
}

impl<'a> ValidationContext<'a> {
    pub fn new(match_info: &'a MatchInfo) -> Self {
        Self { match_info }
    }
}

// Basic type validations
impl Validate for MeterPos {
    fn validate(&self) -> ValidationResult<()> {
        if !self.is_valid() {
            return Err(ValidationError::Position(format!(
                "Position ({}, {}) is out of bounds",
                self.x, self.y
            )));
        }
        Ok(())
    }
}

impl Validate for Velocity {
    fn validate(&self) -> ValidationResult<()> {
        if !self.is_valid() {
            return Err(ValidationError::Velocity(format!(
                "Velocity ({}, {}) exceeds maximum speed of 40 m/s",
                self.x, self.y
            )));
        }
        Ok(())
    }
}

impl Validate for BallState {
    fn validate(&self) -> ValidationResult<()> {
        self.from
            .validate()
            .map_err(|_| ValidationError::BallState("Invalid 'from' position".to_string()))?;

        self.to
            .validate()
            .map_err(|_| ValidationError::BallState("Invalid 'to' position".to_string()))?;

        if self.speed_mps < 0.0 || self.speed_mps > 35.0 {
            return Err(ValidationError::BallState(format!(
                "Ball speed {} m/s is out of range [0, 35]",
                self.speed_mps
            )));
        }

        Ok(())
    }
}

// Match info validations
impl Validate for PitchSpec {
    fn validate(&self) -> ValidationResult<()> {
        if !self.is_valid() {
            return Err(ValidationError::MatchInfo(format!(
                "Invalid pitch dimensions: {}m x {}m",
                self.length_m, self.width_m
            )));
        }
        Ok(())
    }
}

impl Validate for GoalSpec {
    fn validate(&self) -> ValidationResult<()> {
        if !self.is_valid() {
            return Err(ValidationError::MatchInfo(format!(
                "Invalid goal dimensions: width {}m, depth {}m",
                self.width_m, self.depth_m
            )));
        }
        Ok(())
    }
}

impl Validate for AreasSpec {
    fn validate(&self) -> ValidationResult<()> {
        if !self.is_valid() {
            return Err(ValidationError::MatchInfo("Invalid area specifications".to_string()));
        }
        Ok(())
    }
}

impl Validate for MatchInfo {
    fn validate(&self) -> ValidationResult<()> {
        if self.id.is_empty() {
            return Err(ValidationError::MatchInfo("Match ID cannot be empty".to_string()));
        }

        self.pitch.validate()?;

        if let Some(ref goal) = self.goal {
            goal.validate()?;
        }

        if let Some(ref areas) = self.areas {
            areas.validate()?;
        }

        if self.periods.is_empty() {
            return Err(ValidationError::MatchInfo("At least one period is required".to_string()));
        }

        for (i, period) in self.periods.iter().enumerate() {
            if period.start_t < 0.0 {
                return Err(ValidationError::Timing(format!(
                    "Period {} has negative start time: {}",
                    i + 1,
                    period.start_t
                )));
            }
            if period.end_t <= period.start_t {
                return Err(ValidationError::Timing(format!(
                    "Period {} end time {} must be after start time {}",
                    i + 1,
                    period.end_t,
                    period.start_t
                )));
            }
        }

        Ok(())
    }
}

// Event validations
impl Event {
    pub fn validate_with_context(&self, ctx: &ValidationContext) -> ValidationResult<()> {
        // Validate base event data
        let base = self.base();
        base.pos
            .validate()
            .map_err(|_| ValidationError::Event("Invalid player position".to_string()))?;

        if let Some(ref vel) = base.vel {
            vel.validate()
                .map_err(|_| ValidationError::Event("Invalid player velocity".to_string()))?;
        }

        // Check timing is within match periods
        if ctx.match_info.find_period(base.t).is_none() {
            return Err(ValidationError::Timing(format!(
                "Event time {} is not within any match period",
                base.t
            )));
        }

        // Validate event-specific data
        match self {
            Event::Pass(pass) => validate_pass_event(pass, ctx),
            Event::Shot(shot) => validate_shot_event(shot, ctx),
            Event::Dribble(dribble) => validate_dribble_event(dribble, ctx),
            Event::Save(save) => validate_save_event(save, ctx),
            Event::Tackle(tackle) => validate_tackle_event(tackle, ctx),
            Event::Foul(foul) => validate_foul_event(foul, ctx),
            Event::SetPiece(set_piece) => validate_set_piece_event(set_piece, ctx),
            Event::Substitution(sub) => validate_substitution_event(sub, ctx),
            Event::Possession(_) => Ok(()), // Possession events have no additional validation
        }
    }
}

fn validate_pass_event(pass: &PassEvent, _ctx: &ValidationContext) -> ValidationResult<()> {
    pass.end_pos
        .validate()
        .map_err(|_| ValidationError::Event("Invalid pass end position".to_string()))?;

    pass.ball
        .validate()
        .map_err(|_| ValidationError::Event("Invalid ball state in pass".to_string()))?;

    if pass.receiver_id.is_empty() {
        return Err(ValidationError::Event("Receiver ID cannot be empty".to_string()));
    }

    if pass.base.player_id == pass.receiver_id {
        return Err(ValidationError::Event("Player cannot pass to themselves".to_string()));
    }

    Ok(())
}

fn validate_shot_event(shot: &ShotEvent, ctx: &ValidationContext) -> ValidationResult<()> {
    shot.target
        .validate()
        .map_err(|_| ValidationError::Event("Invalid shot target position".to_string()))?;

    shot.ball
        .validate()
        .map_err(|_| ValidationError::Event("Invalid ball state in shot".to_string()))?;

    if shot.xg < 0.0 || shot.xg > 1.0 {
        return Err(ValidationError::Event(format!(
            "Invalid xG value: {} (must be 0.0-1.0)",
            shot.xg
        )));
    }

    // For goals, validate the target is actually in the goal
    if matches!(shot.outcome, super::events::ShotOutcome::Goal) {
        if let Some(ref goal_spec) = ctx.match_info.goal {
            if !shot.target.is_in_goal(goal_spec, ctx.match_info.pitch.length_m) {
                return Err(ValidationError::Event(
                    "Goal target position is not within goal bounds".to_string(),
                ));
            }
        }
    }

    Ok(())
}

fn validate_dribble_event(
    dribble: &DribbleEvent,
    _ctx: &ValidationContext,
) -> ValidationResult<()> {
    dribble
        .end_pos
        .validate()
        .map_err(|_| ValidationError::Event("Invalid dribble end position".to_string()))?;

    for (i, pos) in dribble.path.iter().enumerate() {
        pos.validate().map_err(|_| {
            ValidationError::Event(format!("Invalid dribble path position at index {}", i))
        })?;
    }

    // Check for duplicate beaten players
    let mut unique_beats = std::collections::HashSet::new();
    for player_id in &dribble.beats {
        if !unique_beats.insert(player_id) {
            return Err(ValidationError::Event(format!("Duplicate beaten player: {}", player_id)));
        }
    }

    Ok(())
}

fn validate_save_event(save: &SaveEvent, _ctx: &ValidationContext) -> ValidationResult<()> {
    save.ball
        .validate()
        .map_err(|_| ValidationError::Event("Invalid ball state in save".to_string()))?;

    if let Some(ref parry_pos) = save.parry_to {
        parry_pos
            .validate()
            .map_err(|_| ValidationError::Event("Invalid parry position".to_string()))?;
    }

    Ok(())
}

fn validate_tackle_event(tackle: &TackleEvent, _ctx: &ValidationContext) -> ValidationResult<()> {
    if tackle.opponent_id.is_empty() {
        return Err(ValidationError::Event("Opponent ID cannot be empty".to_string()));
    }

    if tackle.base.player_id == tackle.opponent_id {
        return Err(ValidationError::Event("Player cannot tackle themselves".to_string()));
    }

    Ok(())
}

fn validate_foul_event(foul: &FoulEvent, _ctx: &ValidationContext) -> ValidationResult<()> {
    if foul.opponent_id.is_empty() {
        return Err(ValidationError::Event("Fouled player ID cannot be empty".to_string()));
    }

    if foul.base.player_id == foul.opponent_id {
        return Err(ValidationError::Event("Player cannot foul themselves".to_string()));
    }

    Ok(())
}

fn validate_set_piece_event(
    set_piece: &SetPieceEvent,
    _ctx: &ValidationContext,
) -> ValidationResult<()> {
    if let Some(ref ball) = set_piece.ball {
        ball.validate()
            .map_err(|_| ValidationError::Event("Invalid ball state in set piece".to_string()))?;
    }

    Ok(())
}

fn validate_substitution_event(
    sub: &SubstitutionEvent,
    _ctx: &ValidationContext,
) -> ValidationResult<()> {
    if sub.out_id.is_empty() {
        return Err(ValidationError::Event("Outgoing player ID cannot be empty".to_string()));
    }

    if sub.in_id.is_empty() {
        return Err(ValidationError::Event("Incoming player ID cannot be empty".to_string()));
    }

    if sub.out_id == sub.in_id {
        return Err(ValidationError::Event("Cannot substitute player with themselves".to_string()));
    }

    Ok(())
}

/// Validate a sequence of events for logical consistency
pub fn validate_event_sequence(events: &[Event], ctx: &ValidationContext) -> ValidationResult<()> {
    if events.is_empty() {
        return Err(ValidationError::Event("Event sequence cannot be empty".to_string()));
    }

    // Check events are in chronological order
    for window in events.windows(2) {
        let current = &window[0];
        let next = &window[1];

        if current.base().t > next.base().t {
            return Err(ValidationError::Timing(format!(
                "Events not in chronological order: {} > {}",
                current.base().t,
                next.base().t
            )));
        }
    }

    // Validate each event
    for (i, event) in events.iter().enumerate() {
        event.validate_with_context(ctx).map_err(|e| match e {
            ValidationError::Event(msg) => ValidationError::Event(format!("Event {}: {}", i, msg)),
            other => other,
        })?;
    }

    Ok(())
}

/// Advanced validation: Check for logical inconsistencies in event flow
pub fn validate_event_flow(events: &[Event], _ctx: &ValidationContext) -> ValidationResult<()> {
    // Track possession changes
    let mut possession_team: Option<Team> = None;

    for event in events {
        let base = event.base();

        // Update possession tracking
        match event {
            Event::Pass(PassEvent { outcome: PassOutcome::Complete, .. }) => {
                possession_team = Some(base.team.clone());
            }
            Event::Pass(PassEvent { outcome: PassOutcome::Intercepted, .. }) => {
                // Possession should change to opposing team
                possession_team = match base.team {
                    Team::Home => Some(Team::Away),
                    Team::Away => Some(Team::Home),
                };
            }
            Event::Tackle(tackle) => {
                if tackle.success {
                    possession_team = Some(base.team.clone());
                }
            }
            _ => {}
        }

        // Validate possession consistency
        if let Some(ref current_possession) = possession_team {
            if event.is_ball_event() && base.team != *current_possession {
                // Allow for immediate counter-attacks after possession changes
                if !event.is_possession_change() {
                    return Err(ValidationError::Event(format!(
                        "Event team {:?} doesn't match possession team {:?} at time {}",
                        base.team, current_possession, base.t
                    )));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::replay::{
        events::{BaseEvent, ShotEvent, ShotOutcome},
        types::{BallState, CurveType},
    };

    #[test]
    fn test_position_validation() {
        let valid_pos = MeterPos { x: 52.5, y: 34.0 };
        assert!(valid_pos.validate().is_ok());

        let invalid_pos = MeterPos { x: 200.0, y: 34.0 };
        assert!(invalid_pos.validate().is_err());
    }

    #[test]
    fn test_ball_state_validation() {
        let valid_ball = BallState {
            from: MeterPos { x: 50.0, y: 30.0 },
            to: MeterPos { x: 55.0, y: 35.0 },
            speed_mps: 20.0,
            curve: CurveType::None,
        };
        assert!(valid_ball.validate().is_ok());

        let invalid_ball = BallState {
            from: MeterPos { x: 50.0, y: 30.0 },
            to: MeterPos { x: 55.0, y: 35.0 },
            speed_mps: 50.0, // Too fast
            curve: CurveType::None,
        };
        assert!(invalid_ball.validate().is_err());
    }

    #[test]
    fn test_event_sequence_validation() {
        let match_info = MatchInfo::new("test".to_string(), 123);
        let ctx = ValidationContext::new(&match_info);

        let event1 = Event::Shot(ShotEvent {
            base: BaseEvent::new(
                100.0,
                2,
                Team::Home,
                "H9".to_string(),
                MeterPos { x: 80.0, y: 30.0 },
            ),
            target: MeterPos { x: 105.0, y: 34.0 },
            xg: 0.2,
            on_target: true,
            ball: BallState {
                from: MeterPos { x: 80.0, y: 30.0 },
                to: MeterPos { x: 105.0, y: 34.0 },
                speed_mps: 25.0,
                curve: CurveType::None,
            },
            outcome: ShotOutcome::Goal,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            finishing_skill: None,
            curve_factor: None,
        });

        let event2 = Event::Shot(ShotEvent {
            base: BaseEvent::new(
                50.0,
                1,
                Team::Away,
                "A10".to_string(),
                MeterPos { x: 25.0, y: 30.0 },
            ),
            target: MeterPos { x: 0.0, y: 34.0 },
            xg: 0.15,
            on_target: false,
            ball: BallState {
                from: MeterPos { x: 25.0, y: 30.0 },
                to: MeterPos { x: 0.0, y: 34.0 },
                speed_mps: 22.0,
                curve: CurveType::None,
            },
            outcome: ShotOutcome::Off,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            finishing_skill: None,
            curve_factor: None,
        });

        // Wrong order (event1 at t=100, event2 at t=50)
        let events = vec![event1, event2];
        assert!(validate_event_sequence(&events, &ctx).is_err());
    }

    #[test]
    fn test_match_info_validation() {
        let valid_match = MatchInfo::new("test-123".to_string(), 42);
        assert!(valid_match.validate().is_ok());

        let invalid_match = MatchInfo {
            id: "".to_string(), // Empty ID
            ..valid_match
        };
        assert!(invalid_match.validate().is_err());
    }
}
