// crates/of_core/src/models/match_setup_contracts_test.rs

#[cfg(test)]
mod ci_gates {
    #[allow(unused_imports)] // MatchPlayer used only in conditional tests
    use crate::models::match_setup::{MatchPlayer, MatchSetup, TeamSide};
    use crate::models::player::{Player, PlayerAttributes, Position};
    use crate::models::team::{Formation, Team};

    // ============================================
    // CI_GATE_ATTR_NONE_ZERO
    // Contract: Player.attributes must ALWAYS be Some(...)
    // ============================================

    #[test]
    #[cfg(feature = "strict_contracts")]
    #[should_panic(expected = "P0.75-2 violated")]
    fn ci_gate_attr_none_zero_panic_in_strict_mode() {
        let player = Player {
            name: "Test Player".to_string(),
            position: Position::CM,
            overall: 80,
            condition: 3,
            attributes: None, // ← Contract violation
            equipped_skills: Vec::new(),
            traits: Default::default(),
            personality: Default::default(),
        };

        // In strict mode, this must panic
        let _match_player = MatchPlayer::from_player(&player, 0, None, true);
    }

    #[test]
    #[cfg(not(debug_assertions))] // Only runs in release mode where debug_assert! is disabled
    fn ci_gate_attr_none_zero_fallback_in_release() {
        // In default mode (release/debug without strict), this should warn but fallback
        let player = Player {
            name: "Test Player".to_string(),
            position: Position::CM,
            overall: 80,
            condition: 3,
            attributes: None,
            equipped_skills: Vec::new(),
            traits: Default::default(),
            personality: Default::default(),
        };

        // Should not panic
        let match_player = MatchPlayer::from_player(&player, 0, None, true);

        // Fallback checks
        assert_eq!(match_player.attributes.passing, 50, "Should fallback to default(50)");
    }

    // ============================================
    // CI_GATE_COORD_RANGE
    // Contract: Entities must be within field boundaries + audit buffer
    // ============================================

    fn is_valid_coord(x: f32, y: f32) -> bool {
        const AUDIT_BUFFER: f32 = 1.0;
        let x_valid = (-AUDIT_BUFFER..=105.0 + AUDIT_BUFFER).contains(&x);
        let y_valid = (-AUDIT_BUFFER..=68.0 + AUDIT_BUFFER).contains(&y);

        if !x_valid || !y_valid {
            // In a real scenario, this might log to an audit system.
            // For this test gate, we simply return the validity.
            return false;
        }
        true
    }

    #[test]
    fn ci_gate_coord_range_valid_cases() {
        assert!(is_valid_coord(0.0, 0.0), "Origin valid");
        assert!(is_valid_coord(105.0, 68.0), "Max corner valid");
        assert!(is_valid_coord(-0.5, 68.5), "Within buffer valid");
    }

    #[test]
    fn ci_gate_coord_range_invalid_cases() {
        assert!(!is_valid_coord(-1.1, 0.0), "X underflow invalid");
        assert!(!is_valid_coord(106.1, 0.0), "X overflow invalid");
        assert!(!is_valid_coord(0.0, -1.1), "Y underflow invalid");
        assert!(!is_valid_coord(0.0, 69.1), "Y overflow invalid");
    }

    // ============================================
    // CI_GATE_TRACK_ID_SSOT
    // Contract: track_id must be 0..21 (Home: 0-10, Away: 11-21)
    // ============================================

    #[test]
    fn ci_gate_track_id_definitions() {
        // Verify definition consistency
        assert_eq!(TeamSide::from_track_id(0), TeamSide::Home);
        assert_eq!(TeamSide::from_track_id(10), TeamSide::Home);
        assert_eq!(TeamSide::from_track_id(11), TeamSide::Away);
        assert_eq!(TeamSide::from_track_id(21), TeamSide::Away);

        // Verify helper logic consistency
        for id in 0..11 {
            assert!(TeamSide::is_home(id));
        }
        for id in 11..22 {
            assert!(!TeamSide::is_home(id));
        }
    }

    // ============================================
    // CI_GATE_PLAYER_MAP_NONEMPTY
    // Contract: MatchSetup must have exactly 22 players
    // ============================================

    fn create_dummy_team(name: &str, count: usize) -> Team {
        let mut players = Vec::new();
        // Ensure we supply enough players for TeamSetup logic (takes 11 + 7)
        // If count < 18, it might panic or just take fewer.
        // But for this specific test regarding MatchSetup size, we want to ensure we feed it well.
        let starter_positions = [
            Position::GK,
            Position::LB,
            Position::CB,
            Position::CB,
            Position::RB,
            Position::LM,
            Position::CM,
            Position::CM,
            Position::RM,
            Position::ST,
            Position::ST,
        ];
        let bench_positions = [
            Position::GK,
            Position::DF,
            Position::DF,
            Position::MF,
            Position::MF,
            Position::FW,
            Position::FW,
        ];

        for i in 0..std::cmp::max(count, 18) {
            let position = if i < starter_positions.len() {
                starter_positions[i]
            } else {
                bench_positions[(i - starter_positions.len()) % bench_positions.len()]
            };
            players.push(Player {
                name: format!("{} P{}", name, i),
                position,
                overall: 70,
                condition: 3,
                attributes: Some(PlayerAttributes::default()),
                equipped_skills: Vec::new(),
                traits: Default::default(),
                personality: Default::default(),
            });
        }

        Team { name: name.to_string(), formation: Formation::F442, players }
    }

    #[test]
    fn ci_gate_match_setup_player_count() {
        let home_team = create_dummy_team("Home", 11);
        let away_team = create_dummy_team("Away", 11);

        // Use from_teams factory
        let match_setup = MatchSetup::from_teams(&home_team, &away_team).expect("setup should build");

        // Use public export method to verify slot count
        // (slots field is private, but export exposes all 22 slots)
        let export = match_setup.to_export();

        assert_eq!(
            export.player_slots.len(),
            22,
            "PLAYER_MAP_NONEMPTY violation: Expected 22 players via export"
        );
    }

    #[test]
    fn substitution_roster_swap_updates_pitch_slot_and_marks_bench_used() {
        let home_team = create_dummy_team("Home", 18);
        let away_team = create_dummy_team("Away", 18);
        let mut match_setup = MatchSetup::from_teams(&home_team, &away_team).expect("setup should build");

        let starting = match_setup.to_export_starting_lineup();
        let starter_name_before = starting
            .player_slots
            .iter()
            .find(|s| s.track_id == 1)
            .expect("starter track_id 1")
            .name
            .clone();

        let pitch_track_id = 1usize;
        assert!(
            match_setup.apply_substitution(pitch_track_id, 7).is_err(),
            "bench_slot out of range must error"
        );
        let old_name = match_setup.get_player(pitch_track_id).name.clone();
        let (player_in_name, player_out_name) =
            match_setup.apply_substitution(pitch_track_id, 0).expect("sub ok");

        assert_eq!(player_out_name, old_name);
        assert_eq!(match_setup.get_player(pitch_track_id).name, player_in_name);
        assert!(match_setup.is_sub_used(TeamSide::Home, 0));

        // Starting lineup snapshot must not be retroactively mutated.
        let starting_after = match_setup.to_export_starting_lineup();
        let starter_name_after = starting_after
            .player_slots
            .iter()
            .find(|s| s.track_id == 1)
            .expect("starter track_id 1")
            .name
            .clone();
        assert_eq!(starter_name_before, starter_name_after);

        // Bench slots cannot re-enter once used.
        assert!(match_setup.apply_substitution(pitch_track_id, 0).is_err());
    }

    #[test]
    fn substitution_roster_swap_away_team_uses_away_bench_used_flags() {
        let home_team = create_dummy_team("Home", 18);
        let away_team = create_dummy_team("Away", 18);
        let mut match_setup = MatchSetup::from_teams(&home_team, &away_team).expect("setup should build");

        let pitch_track_id = 12usize; // away outfield pitch slot
        let old_name = match_setup.get_player(pitch_track_id).name.clone();
        let (player_in_name, player_out_name) =
            match_setup.apply_substitution(pitch_track_id, 0).expect("sub ok");

        assert_eq!(player_out_name, old_name);
        assert_eq!(match_setup.get_player(pitch_track_id).name, player_in_name);
        assert!(match_setup.is_sub_used(TeamSide::Away, 0));
        assert!(!match_setup.is_sub_used(TeamSide::Home, 0));
    }
}
