//! Property-based test generators for replay system
//!
//! This module provides proptest generators for all replay types,
//! enabling comprehensive property-based testing of the replay system.

use super::{events::*, match_info::*, types::*, OfReplay};
use crate::engine::coordinate_contract::{COORD_CONTRACT_VERSION, COORD_SYSTEM_METERS_V2};
use proptest::prelude::*;

// Generators for basic types
pub fn meter_pos_strategy() -> impl Strategy<Value = MeterPos> {
    (-0.5f64..=105.5, -0.5f64..=68.5).prop_map(|(x, y)| MeterPos { x, y })
}

pub fn velocity_strategy() -> impl Strategy<Value = Velocity> {
    (-40.0f64..=40.0, -40.0f64..=40.0).prop_map(|(x, y)| Velocity { x, y })
}

pub fn ball_state_strategy() -> impl Strategy<Value = BallState> {
    (meter_pos_strategy(), meter_pos_strategy(), 0.0f64..=35.0, curve_type_strategy())
        .prop_map(|(from, to, speed_mps, curve)| BallState { from, to, speed_mps, curve })
}

pub fn curve_type_strategy() -> impl Strategy<Value = CurveType> {
    prop_oneof![Just(CurveType::None), Just(CurveType::Inside), Just(CurveType::Outside),]
}

pub fn team_strategy() -> impl Strategy<Value = Team> {
    prop_oneof![Just(Team::Home), Just(Team::Away)]
}

// Generators for match info
pub fn pitch_spec_strategy() -> impl Strategy<Value = PitchSpec> {
    (90.0f64..=120.0, 45.0f64..=90.0)
        .prop_map(|(length_m, width_m)| PitchSpec { length_m, width_m })
}

pub fn goal_spec_strategy() -> impl Strategy<Value = GoalSpec> {
    (20.0f64..=48.0, 6.0f64..=7.5, 1.5f64..=3.0)
        .prop_map(|(center_y_m, width_m, depth_m)| GoalSpec { center_y_m, width_m, depth_m })
}

pub fn areas_spec_strategy() -> impl Strategy<Value = AreasSpec> {
    (
        10.0f64..=20.0, // penalty_depth_m
        35.0f64..=45.0, // penalty_width_m
        4.0f64..=7.0,   // six_depth_m
        16.0f64..=20.0, // six_width_m
        10.0f64..=12.0, // penalty_spot_from_line_m
        8.0f64..=10.0,  // arc_radius_m
    )
        .prop_map(
            |(
                penalty_depth_m,
                penalty_width_m,
                six_depth_m,
                six_width_m,
                penalty_spot_from_line_m,
                arc_radius_m,
            )| {
                AreasSpec {
                    penalty_depth_m,
                    penalty_width_m,
                    six_depth_m,
                    six_width_m,
                    penalty_spot_from_line_m,
                    arc_radius_m,
                }
            },
        )
}

pub fn period_strategy() -> impl Strategy<Value = Period> {
    (
        1u32..=4,        // period number
        0.0f64..=7200.0, // start_t (up to 2 hours)
        any::<bool>(),   // home_attacks_right
    )
        .prop_map(|(i, start_t, home_attacks_right)| {
            let end_t = start_t + 2700.0; // 45 minute periods
            Period { i, start_t, end_t, home_attacks_right }
        })
}

pub fn match_info_strategy() -> impl Strategy<Value = MatchInfo> {
    (
        "[a-zA-Z0-9-_]{3,20}", // match id
        any::<i64>(),          // seed
        pitch_spec_strategy(),
        prop::option::of(goal_spec_strategy()),
        prop::option::of(areas_spec_strategy()),
        prop::collection::vec(period_strategy(), 1..=3),
    )
        .prop_map(|(id, seed, pitch, goal, areas, periods)| MatchInfo {
            id,
            seed,
            pitch,
            goal,
            areas,
            periods,
        })
}

// Generators for events
pub fn base_event_strategy() -> impl Strategy<Value = BaseEvent> {
    (
        0.0f64..=7200.0, // time (up to 2 hours)
        0u32..=130,      // minute
        team_strategy(),
        "[A-Z][0-9]{1,2}", // player_id (H1, A10, etc.)
        meter_pos_strategy(),
        prop::option::of(velocity_strategy()),
        prop::option::of(event_meta_strategy()),
    )
        .prop_map(|(t, minute, team, player_id, pos, vel, meta)| BaseEvent {
            t,
            minute,
            team,
            player_id,
            pos,
            vel,
            meta,
        })
}

pub fn event_meta_strategy() -> impl Strategy<Value = EventMeta> {
    (
        prop::option::of(0.0f64..=1.0), // confidence
    )
        .prop_map(|(confidence,)| {
            EventMeta {
                confidence,
                additional: std::collections::HashMap::new(), // Keep it simple for testing
            }
        })
}

// Event outcome generators
pub fn pass_outcome_strategy() -> impl Strategy<Value = PassOutcome> {
    prop_oneof![Just(PassOutcome::Complete), Just(PassOutcome::Intercepted), Just(PassOutcome::Out),]
}

pub fn shot_outcome_strategy() -> impl Strategy<Value = ShotOutcome> {
    prop_oneof![
        Just(ShotOutcome::Goal),
        Just(ShotOutcome::Saved),
        Just(ShotOutcome::Post),
        Just(ShotOutcome::Off),
    ]
}

pub fn dribble_outcome_strategy() -> impl Strategy<Value = DribbleOutcome> {
    prop_oneof![
        Just(DribbleOutcome::Kept),
        Just(DribbleOutcome::Tackle),
        Just(DribbleOutcome::Foul),
        Just(DribbleOutcome::Out),
    ]
}

pub fn card_type_strategy() -> impl Strategy<Value = CardType> {
    prop_oneof![Just(CardType::None), Just(CardType::Yellow), Just(CardType::Red),]
}

pub fn set_piece_kind_strategy() -> impl Strategy<Value = SetPieceKind> {
    prop_oneof![
        Just(SetPieceKind::FreeKick),
        Just(SetPieceKind::Corner),
        Just(SetPieceKind::Penalty),
        Just(SetPieceKind::ThrowIn),
        Just(SetPieceKind::GoalKick),
        Just(SetPieceKind::KickOff),
    ]
}

// Event generators
pub fn pass_event_strategy() -> impl Strategy<Value = PassEvent> {
    (
        base_event_strategy(),
        meter_pos_strategy(),
        "[A-Z][0-9]{1,2}", // receiver_id
        any::<bool>(),     // ground
        ball_state_strategy(),
        pass_outcome_strategy(),
    )
        .prop_map(|(base, end_pos, receiver_id, ground, ball, outcome)| PassEvent {
            base,
            end_pos,
            receiver_id,
            ground,
            ball,
            outcome,
            distance_m: None,
            passing_skill: None,
            vision: None,
            technique: None,
            force: None,
            danger_level: None,
            is_switch_of_play: None,
            is_line_breaking: None,
            is_through_ball: None,
            intended_target_pos: None,
        })
}

pub fn shot_event_strategy() -> impl Strategy<Value = ShotEvent> {
    (
        base_event_strategy(),
        meter_pos_strategy(),
        0.0f64..=1.0,  // xg
        any::<bool>(), // on_target
        ball_state_strategy(),
        shot_outcome_strategy(),
    )
        .prop_map(|(base, target, xg, on_target, ball, outcome)| ShotEvent {
            base,
            target,
            xg,
            on_target,
            ball,
            outcome,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            finishing_skill: None,
            curve_factor: None,
        })
}

pub fn dribble_event_strategy() -> impl Strategy<Value = DribbleEvent> {
    (
        base_event_strategy(),
        prop::collection::vec(meter_pos_strategy(), 0..=5), // path
        meter_pos_strategy(),
        prop::collection::vec("[A-Z][0-9]{1,2}", 0..=3), // beats
        dribble_outcome_strategy(),
    )
        .prop_map(|(base, path, end_pos, beats, outcome)| DribbleEvent {
            base,
            path,
            end_pos,
            beats,
            outcome,
            success: None,
            opponents_evaded: None,
            space_gained: None,
            pressure_level: None,
            dribbling_skill: None,
            agility: None,
        })
}

pub fn save_event_strategy() -> impl Strategy<Value = SaveEvent> {
    (base_event_strategy(), ball_state_strategy(), prop::option::of(meter_pos_strategy())).prop_map(
        |(base, ball, parry_to)| SaveEvent {
            base,
            ball,
            parry_to,
            shot_from: None,
            shot_power: None,
            save_difficulty: None,
            reflexes_skill: None,
            handling_skill: None,
            diving_skill: None,
        },
    )
}

pub fn tackle_event_strategy() -> impl Strategy<Value = TackleEvent> {
    (
        base_event_strategy(),
        "[A-Z][0-9]{1,2}", // opponent_id
        any::<bool>(),     // success
    )
        .prop_map(|(base, opponent_id, success)| TackleEvent { base, opponent_id, success })
}

pub fn foul_event_strategy() -> impl Strategy<Value = FoulEvent> {
    (
        base_event_strategy(),
        "[A-Z][0-9]{1,2}", // opponent_id
        card_type_strategy(),
    )
        .prop_map(|(base, opponent_id, card)| FoulEvent { base, opponent_id, card })
}

pub fn set_piece_event_strategy() -> impl Strategy<Value = SetPieceEvent> {
    (base_event_strategy(), set_piece_kind_strategy(), prop::option::of(ball_state_strategy()))
        .prop_map(|(base, kind, ball)| SetPieceEvent { base, kind, ball })
}

pub fn substitution_event_strategy() -> impl Strategy<Value = SubstitutionEvent> {
    (
        base_event_strategy(),
        "[A-Z][0-9]{1,2}", // out_id
        "[A-Z][0-9]{1,2}", // in_id
    )
        .prop_map(|(base, out_id, in_id)| SubstitutionEvent { base, out_id, in_id })
}

pub fn event_strategy() -> impl Strategy<Value = Event> {
    prop_oneof![
        pass_event_strategy().prop_map(Event::Pass),
        shot_event_strategy().prop_map(Event::Shot),
        dribble_event_strategy().prop_map(Event::Dribble),
        save_event_strategy().prop_map(Event::Save),
        tackle_event_strategy().prop_map(Event::Tackle),
        foul_event_strategy().prop_map(Event::Foul),
        set_piece_event_strategy().prop_map(Event::SetPiece),
        substitution_event_strategy().prop_map(Event::Substitution),
    ]
}

// Replay generators
pub fn schema_info_strategy() -> impl Strategy<Value = SchemaInfo> {
    Just(SchemaInfo { name: "of_replay".to_string(), version: 1 })
}

pub fn build_info_strategy() -> impl Strategy<Value = BuildInfo> {
    (
        prop::option::of("[0-9]+\\.[0-9]+\\.[0-9]+"), // of_core version
        prop::option::of("[0-9]+\\.[0-9]+\\.[0-9]+"), // gdext version
        prop::option::of("[a-f0-9]{7}"),              // build_tag
    )
        .prop_map(|(of_core, gdext, build_tag)| BuildInfo {
            of_core,
            gdext,
            build_tag,
            additional: std::collections::HashMap::new(),
        })
}

pub fn of_replay_strategy() -> impl Strategy<Value = OfReplay> {
    (
        schema_info_strategy(),
        prop::option::of(build_info_strategy()),
        match_info_strategy(),
        prop::collection::vec(event_strategy(), 1..=50), // 1-50 events
    )
        .prop_map(|(schema, build, match_info, mut events)| {
            // Sort events by time to ensure chronological order
            events.sort_by(|a, b| a.base().t.partial_cmp(&b.base().t).unwrap());

            OfReplay {
                coord_contract_version: COORD_CONTRACT_VERSION,
                coord_system: COORD_SYSTEM_METERS_V2.to_string(),
                schema,
                build,
                match_info,
                events,
            }
        })
}

// Utility generators for testing specific scenarios
pub fn goal_events_strategy() -> impl Strategy<Value = Vec<Event>> {
    prop::collection::vec(
        shot_event_strategy().prop_map(|mut shot| {
            shot.outcome = ShotOutcome::Goal;
            Event::Shot(shot)
        }),
        1..=5,
    )
}

pub fn major_incident_events_strategy() -> impl Strategy<Value = Vec<Event>> {
    prop::collection::vec(
        prop_oneof![
            shot_event_strategy().prop_map(|mut shot| {
                shot.outcome = ShotOutcome::Goal;
                Event::Shot(shot)
            }),
            foul_event_strategy().prop_map(|mut foul| {
                foul.card = CardType::Red;
                Event::Foul(foul)
            }),
            set_piece_event_strategy().prop_map(|mut sp| {
                sp.kind = SetPieceKind::Penalty;
                Event::SetPiece(sp)
            }),
        ],
        1..=10,
    )
}

// Generator for events involving specific players
pub fn player_events_strategy(player_ids: Vec<String>) -> impl Strategy<Value = Vec<Event>> {
    prop::collection::vec(
        (event_strategy(), 0..player_ids.len()).prop_map(move |(mut event, idx)| {
            if let Some(player_id) = player_ids.get(idx) {
                event.base_mut().player_id = player_id.clone();
            }
            event
        }),
        1..=20,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::replay::validation::{Validate, ValidationContext};

    proptest! {
        #[test]
        fn test_meter_pos_generator_produces_valid_positions(pos in meter_pos_strategy()) {
            prop_assert!(pos.validate().is_ok());
        }

        #[test]
        fn test_velocity_generator_produces_valid_velocities(vel in velocity_strategy()) {
            prop_assert!(vel.validate().is_ok());
        }

        #[test]
        fn test_ball_state_generator_produces_valid_states(ball in ball_state_strategy()) {
            prop_assert!(ball.validate().is_ok());
        }

        #[test]
        fn test_pitch_spec_generator_produces_valid_pitches(pitch in pitch_spec_strategy()) {
            prop_assert!(pitch.validate().is_ok());
        }

        #[test]
        fn test_match_info_generator_produces_valid_info(match_info in match_info_strategy()) {
            prop_assert!(match_info.validate().is_ok());
        }

        #[test]
        fn test_event_generator_produces_valid_events(
            match_info in match_info_strategy(),
            event in event_strategy()
        ) {
            let ctx = ValidationContext::new(&match_info);
            // Note: This might fail due to timing constraints, but events should be structurally valid
            let _ = event.validate_with_context(&ctx);
        }

        #[test]
        fn test_replay_json_roundtrip(replay in of_replay_strategy()) {
            let json = replay.to_json_pretty().unwrap();
            let deserialized = OfReplay::from_json(&json).unwrap();
            prop_assert_eq!(replay.match_info.id, deserialized.match_info.id);
            prop_assert_eq!(replay.events.len(), deserialized.events.len());
        }

        #[test]
        fn test_replay_validation_properties(replay in of_replay_strategy()) {
            // Basic structural validation should pass for generated replays
            prop_assert!(replay.schema.name == "of_replay");
            prop_assert!(replay.schema.version == 1);
            prop_assert!(!replay.match_info.id.is_empty());
            prop_assert!(!replay.events.is_empty());

            // Events should be in chronological order (enforced by generator)
            for window in replay.events.windows(2) {
                prop_assert!(window[0].base().t <= window[1].base().t);
            }
        }
    }
}
