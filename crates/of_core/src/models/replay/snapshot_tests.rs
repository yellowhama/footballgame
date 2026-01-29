//! Snapshot tests for replay system using insta
//!
//! These tests verify that the JSON output of replays remains consistent
//! across code changes, helping catch unintended serialization changes.

use super::*;
use insta::assert_json_snapshot;

fn create_sample_replay() -> OfReplay {
    let events = vec![
        Event::Pass(PassEvent {
            base: BaseEvent::new(
                120.5,
                2,
                Team::Home,
                "H7".to_string(),
                MeterPos { x: 45.0, y: 30.0 },
            )
            .with_velocity(Velocity { x: 5.0, y: 0.0 }),
            end_pos: MeterPos { x: 55.0, y: 35.0 },
            receiver_id: "H9".to_string(),
            ground: true,
            ball: BallState {
                from: MeterPos { x: 45.0, y: 30.0 },
                to: MeterPos { x: 55.0, y: 35.0 },
                speed_mps: 15.0,
                curve: CurveType::None,
            },
            outcome: PassOutcome::Complete,
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
        }),
        Event::Dribble(DribbleEvent {
            base: BaseEvent::new(
                145.2,
                2,
                Team::Home,
                "H9".to_string(),
                MeterPos { x: 55.0, y: 35.0 },
            ),
            path: vec![MeterPos { x: 56.0, y: 36.0 }, MeterPos { x: 58.0, y: 37.0 }],
            end_pos: MeterPos { x: 60.0, y: 38.0 },
            beats: vec!["A3".to_string()],
            outcome: DribbleOutcome::Kept,
            success: None,
            opponents_evaded: None,
            space_gained: None,
            pressure_level: None,
            dribbling_skill: None,
            agility: None,
        }),
        Event::Shot(ShotEvent {
            base: BaseEvent::new(
                165.8,
                3,
                Team::Home,
                "H9".to_string(),
                MeterPos { x: 95.0, y: 32.0 },
            ),
            target: MeterPos { x: 105.0, y: 30.5 },
            xg: 0.25,
            on_target: true,
            ball: BallState {
                from: MeterPos { x: 95.0, y: 32.0 },
                to: MeterPos { x: 105.0, y: 30.5 },
                speed_mps: 28.0,
                curve: CurveType::Inside,
            },
            outcome: ShotOutcome::Goal,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            finishing_skill: None,
            curve_factor: None,
        }),
        Event::Foul(FoulEvent {
            base: BaseEvent::new(
                892.3,
                15,
                Team::Away,
                "A5".to_string(),
                MeterPos { x: 25.0, y: 40.0 },
            ),
            opponent_id: "H6".to_string(),
            card: CardType::Yellow,
        }),
        Event::SetPiece(SetPieceEvent {
            base: BaseEvent::new(
                895.0,
                15,
                Team::Home,
                "H8".to_string(),
                MeterPos { x: 25.0, y: 40.0 },
            ),
            kind: SetPieceKind::FreeKick,
            ball: Some(BallState {
                from: MeterPos { x: 25.0, y: 40.0 },
                to: MeterPos { x: 35.0, y: 30.0 },
                speed_mps: 18.0,
                curve: CurveType::Outside,
            }),
        }),
        Event::Substitution(SubstitutionEvent {
            base: BaseEvent::new(
                2700.0, // Half-time
                45,
                Team::Away,
                "COACH_A".to_string(),
                MeterPos { x: -5.0, y: 34.0 }, // Sideline
            ),
            out_id: "A11".to_string(),
            in_id: "A12".to_string(),
        }),
    ];

    OfReplay::new("snapshot-test-match".to_string(), 42, events)
        .with_build_info("0.1.0".to_string(), "0.6.0".to_string(), "abc1234".to_string())
        .with_goal_spec(GoalSpec::standard(68.0))
        .with_areas_spec(AreasSpec::standard())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_replay_json_snapshot() {
        let replay = create_sample_replay();
        assert_json_snapshot!("complete_replay", replay);
    }

    #[test]
    fn test_replay_schema_info_snapshot() {
        let replay = create_sample_replay();
        assert_json_snapshot!("schema_info", replay.schema);
    }

    #[test]
    fn test_replay_build_info_snapshot() {
        let replay = create_sample_replay();
        assert_json_snapshot!("build_info", replay.build);
    }

    #[test]
    fn test_replay_match_info_snapshot() {
        let replay = create_sample_replay();
        assert_json_snapshot!("match_info", replay.match_info);
    }

    #[test]
    fn test_individual_event_snapshots() {
        let replay = create_sample_replay();

        // Test each event type
        for (i, event) in replay.events.iter().enumerate() {
            assert_json_snapshot!(format!("event_{}_{}", i, event.event_type()), event);
        }
    }

    #[test]
    fn test_highlight_filtering_snapshots() {
        let replay = create_sample_replay();

        // Test different highlight levels
        let skip = replay.highlights(HighlightLevel::Skip);
        assert_json_snapshot!("highlights_skip", skip);

        let simple = replay.highlights(HighlightLevel::Simple);
        assert_json_snapshot!("highlights_simple", simple);

        let full = replay.highlights(HighlightLevel::Full);
        assert_json_snapshot!("highlights_full", full);

        // Test MyPlayer configuration
        let my_player = replay.player_highlights(
            MyPlayerConfig::single_player("H9".to_string()).with_indirect_events(),
        );
        assert_json_snapshot!("highlights_my_player_h9", my_player);
    }

    #[test]
    fn test_event_distribution_snapshot() {
        let replay = create_sample_replay();
        let distribution = analyze_event_distribution(&replay.events);
        assert_json_snapshot!("event_distribution", distribution);
    }

    #[test]
    fn test_replay_statistics_snapshot() {
        let replay = create_sample_replay();
        let stats = replay.statistics();
        assert_json_snapshot!("replay_statistics", stats);
    }

    #[test]
    fn test_validation_error_snapshots() {
        // Create invalid replay to test error messages
        let mut invalid_replay = create_sample_replay();
        invalid_replay.match_info.id = "".to_string(); // Empty ID

        let validation_result = invalid_replay.validate();
        match validation_result {
            Err(e) => {
                assert_json_snapshot!("validation_error_empty_id", format!("{}", e));
            }
            Ok(_) => panic!("Expected validation to fail"),
        }

        // Test invalid position
        let invalid_pos = MeterPos { x: 200.0, y: 34.0 };
        let pos_validation = invalid_pos.validate();
        match pos_validation {
            Err(e) => {
                assert_json_snapshot!("validation_error_invalid_position", format!("{}", e));
            }
            Ok(_) => panic!("Expected position validation to fail"),
        }
    }

    #[test]
    fn test_empty_replay_snapshot() {
        let empty_replay = OfReplay::new(
            "empty-match".to_string(),
            0,
            vec![], // No events - will fail validation but good for testing
        );

        // Test structure (ignoring validation)
        #[derive(serde::Serialize)]
        struct EmptyReplayStructure {
            schema: SchemaInfo,
            build: Option<BuildInfo>,
            match_info: MatchInfo,
            event_count: usize,
        }

        let structure = EmptyReplayStructure {
            schema: empty_replay.schema,
            build: empty_replay.build,
            match_info: empty_replay.match_info,
            event_count: empty_replay.events.len(),
        };

        assert_json_snapshot!("empty_replay_structure", structure);
    }

    #[test]
    fn test_minimal_valid_replay_snapshot() {
        let minimal_replay = OfReplay::new(
            "minimal-match".to_string(),
            123,
            vec![Event::Pass(PassEvent {
                base: BaseEvent::new(
                    1.0,
                    0,
                    Team::Home,
                    "H1".to_string(),
                    MeterPos { x: 52.5, y: 34.0 },
                ),
                end_pos: MeterPos { x: 55.0, y: 34.0 },
                receiver_id: "H2".to_string(),
                ground: true,
                ball: BallState {
                    from: MeterPos { x: 52.5, y: 34.0 },
                    to: MeterPos { x: 55.0, y: 34.0 },
                    speed_mps: 10.0,
                    curve: CurveType::None,
                },
                outcome: PassOutcome::Complete,
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
            })],
        );

        assert_json_snapshot!("minimal_valid_replay", minimal_replay);
    }

    #[test]
    fn test_complex_match_scenario_snapshot() {
        // Create a more complex match scenario
        let complex_events = vec![
            // Kickoff
            Event::Pass(PassEvent {
                base: BaseEvent::new(
                    0.0,
                    0,
                    Team::Home,
                    "H9".to_string(),
                    MeterPos { x: 52.5, y: 34.0 },
                ),
                end_pos: MeterPos { x: 45.0, y: 30.0 },
                receiver_id: "H7".to_string(),
                ground: true,
                ball: BallState {
                    from: MeterPos { x: 52.5, y: 34.0 },
                    to: MeterPos { x: 45.0, y: 30.0 },
                    speed_mps: 12.0,
                    curve: CurveType::None,
                },
                outcome: PassOutcome::Complete,
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
            }),
            // Quick attack
            Event::Pass(PassEvent {
                base: BaseEvent::new(
                    5.2,
                    0,
                    Team::Home,
                    "H7".to_string(),
                    MeterPos { x: 45.0, y: 30.0 },
                ),
                end_pos: MeterPos { x: 75.0, y: 25.0 },
                receiver_id: "H11".to_string(),
                ground: false, // Long ball
                ball: BallState {
                    from: MeterPos { x: 45.0, y: 30.0 },
                    to: MeterPos { x: 75.0, y: 25.0 },
                    speed_mps: 25.0,
                    curve: CurveType::Outside,
                },
                outcome: PassOutcome::Complete,
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
            }),
            // Shot and save
            Event::Shot(ShotEvent {
                base: BaseEvent::new(
                    8.7,
                    0,
                    Team::Home,
                    "H11".to_string(),
                    MeterPos { x: 92.0, y: 28.0 },
                ),
                target: MeterPos { x: 105.0, y: 32.0 },
                xg: 0.35,
                on_target: true,
                ball: BallState {
                    from: MeterPos { x: 92.0, y: 28.0 },
                    to: MeterPos { x: 105.0, y: 32.0 },
                    speed_mps: 30.0,
                    curve: CurveType::None,
                },
                outcome: ShotOutcome::Saved,
                shot_type: None,
                defender_pressure: None,
                angle_to_goal: None,
                distance_to_goal: None,
                composure: None,
                finishing_skill: None,
                curve_factor: None,
            }),
            Event::Save(SaveEvent {
                base: BaseEvent::new(
                    9.1,
                    0,
                    Team::Away,
                    "A1".to_string(),
                    MeterPos { x: 105.0, y: 34.0 },
                ),
                ball: BallState {
                    from: MeterPos { x: 92.0, y: 28.0 },
                    to: MeterPos { x: 105.0, y: 32.0 },
                    speed_mps: 30.0,
                    curve: CurveType::None,
                },
                parry_to: Some(MeterPos { x: 98.0, y: 45.0 }),
                shot_from: None,
                shot_power: None,
                save_difficulty: None,
                reflexes_skill: None,
                handling_skill: None,
                diving_skill: None,
            }),
        ];

        let complex_replay = OfReplay::new("complex-scenario".to_string(), 999, complex_events);

        assert_json_snapshot!("complex_match_scenario", complex_replay);
    }

    #[test]
    fn test_corner_to_goal_sequence_snapshot() {
        // Test a corner kick that leads to a goal
        let corner_sequence = vec![
            Event::SetPiece(SetPieceEvent {
                base: BaseEvent::new(
                    1205.3,
                    20,
                    Team::Home,
                    "H8".to_string(),
                    MeterPos { x: 105.0, y: 0.0 },
                ),
                kind: SetPieceKind::Corner,
                ball: Some(BallState {
                    from: MeterPos { x: 105.0, y: 0.0 },
                    to: MeterPos { x: 99.0, y: 32.0 },
                    speed_mps: 22.0,
                    curve: CurveType::Inside,
                }),
            }),
            Event::Shot(ShotEvent {
                base: BaseEvent::new(
                    1208.1,
                    20,
                    Team::Home,
                    "H4".to_string(),
                    MeterPos { x: 99.0, y: 32.0 },
                ),
                target: MeterPos { x: 105.0, y: 36.8 },
                xg: 0.45,
                on_target: true,
                ball: BallState {
                    from: MeterPos { x: 99.0, y: 32.0 },
                    to: MeterPos { x: 105.0, y: 36.8 },
                    speed_mps: 18.0,
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
            }),
        ];

        let corner_replay = OfReplay::new("corner-to-goal".to_string(), 777, corner_sequence);

        assert_json_snapshot!("corner_to_goal_sequence", corner_replay);
    }
}
