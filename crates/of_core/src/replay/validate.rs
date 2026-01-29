use super::types::*;

/// Validates a replay document for consistency and correctness
pub fn validate_replay(doc: &ReplayDoc) -> Result<(), String> {
    if doc.version == 0 {
        return Err("ReplayDoc version must be > 0".into());
    }

    if doc.pitch_m.width_m <= 0.0 || doc.pitch_m.height_m <= 0.0 {
        return Err("PitchSpec must have positive dimensions".into());
    }

    if doc.events.is_empty() {
        return Err("ReplayDoc must contain at least one event".into());
    }

    // Validate time ordering
    let mut last_time = -1.0;
    for (i, event) in doc.events.iter().enumerate() {
        let base = event.base();
        if base.t < last_time {
            return Err(format!(
                "Event at index {} has time {} which is before previous event time {}",
                i, base.t, last_time
            ));
        }
        last_time = base.t;
    }

    // Validate positions are within pitch bounds
    for (i, event) in doc.events.iter().enumerate() {
        match event {
            ReplayEvent::Pass { from, to, .. } => {
                validate_position(from, &doc.pitch_m, i, "from")?;
                validate_position(to, &doc.pitch_m, i, "to")?;
            }
            ReplayEvent::Shot { from, .. } => {
                validate_position(from, &doc.pitch_m, i, "from")?;
            }
            ReplayEvent::Goal { at, .. } => {
                validate_position(at, &doc.pitch_m, i, "at")?;
            }
            ReplayEvent::Foul { at, .. } => {
                validate_position(at, &doc.pitch_m, i, "at")?;
            }
            ReplayEvent::FreeKick { spot, .. } => {
                validate_position(spot, &doc.pitch_m, i, "spot")?;
            }
            ReplayEvent::CornerKick { spot, .. } => {
                validate_position(spot, &doc.pitch_m, i, "spot")?;
            }
            ReplayEvent::BallMove { to, .. } => {
                validate_position(to, &doc.pitch_m, i, "to")?;
            }
            _ => {} // Events without positions
        }
    }

    Ok(())
}

fn validate_position(
    pos: &MeterPos,
    pitch: &PitchSpec,
    event_idx: usize,
    field_name: &str,
) -> Result<(), String> {
    if pos.x < 0.0 || pos.x > pitch.width_m {
        return Err(format!(
            "Event {} has invalid x position {} in field '{}' (pitch width: {})",
            event_idx, pos.x, field_name, pitch.width_m
        ));
    }
    if pos.y < 0.0 || pos.y > pitch.height_m {
        return Err(format!(
            "Event {} has invalid y position {} in field '{}' (pitch height: {})",
            event_idx, pos.y, field_name, pitch.height_m
        ));
    }
    Ok(())
}

/// Validates basic replay structure without deep validation
pub fn validate_replay_basic(doc: &ReplayDoc) -> Result<(), String> {
    if doc.version == 0 {
        return Err("ReplayDoc version must be > 0".into());
    }

    if doc.pitch_m.width_m <= 0.0 || doc.pitch_m.height_m <= 0.0 {
        return Err("PitchSpec must have positive dimensions".into());
    }

    if doc.events.is_empty() {
        return Err("ReplayDoc must contain at least one event".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_replay() -> ReplayDoc {
        ReplayDoc {
            version: 1,
            pitch_m: PitchSpec { width_m: 105.0, height_m: 68.0 },
            events: vec![ReplayEvent::KickOff {
                base: EventBase { t: 0.0, player_id: None, team_id: Some(0) },
            }],
            rosters: ReplayRosters::default(),
            timeline: Vec::new(),
            tactics: ReplayTeamsTactics::default(),
        }
    }

    #[test]
    fn test_valid_replay() {
        let replay = create_test_replay();
        assert!(validate_replay(&replay).is_ok());
    }

    #[test]
    fn test_invalid_version() {
        let mut replay = create_test_replay();
        replay.version = 0;
        assert!(validate_replay(&replay).is_err());
    }

    #[test]
    fn test_invalid_pitch_dimensions() {
        let mut replay = create_test_replay();
        replay.pitch_m.width_m = -1.0;
        assert!(validate_replay(&replay).is_err());
    }

    #[test]
    fn test_empty_events() {
        let mut replay = create_test_replay();
        replay.events.clear();
        assert!(validate_replay(&replay).is_err());
    }

    #[test]
    fn test_position_validation() {
        let mut replay = create_test_replay();
        replay.events.push(ReplayEvent::Goal {
            base: EventBase { t: 10.0, player_id: Some(9), team_id: Some(0) },
            at: MeterPos { x: 200.0, y: 34.0 }, // Invalid x position
            assist_player_id: None,
        });
        assert!(validate_replay(&replay).is_err());
    }

    #[test]
    fn test_time_ordering() {
        let mut replay = create_test_replay();
        replay.events.push(ReplayEvent::Goal {
            base: EventBase {
                t: -1.0, // Time before kickoff
                player_id: Some(9),
                team_id: Some(0),
            },
            at: MeterPos { x: 50.0, y: 34.0 },
            assist_player_id: None,
        });
        assert!(validate_replay(&replay).is_err());
    }
}
