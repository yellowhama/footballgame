// Test case to reproduce the crash with Skills36
use of_adapter::{simulate_match, CorePlayer, CoreTeam, MatchConfig};

fn main() {
    eprintln!("=== Starting crash reproduction test ===");

    // Create a simple team with CA 54 players (like 이서준)
    let mut players = vec![];

    // GK
    players.push(CorePlayer {
        name: "Test GK".into(),
        ca: 118,
        pa: 143,
        position: "GK".into(),
        condition: 0.75,
    });

    // Defenders: LB, CB, CB, RB
    for i in 0..4 {
        players.push(CorePlayer {
            name: format!("Test DF{}", i),
            ca: 60,
            pa: 80,
            position: match i {
                0 => "LB",
                1 | 2 => "CB",
                3 => "RB",
                _ => "CB",
            }
            .into(),
            condition: 0.75,
        });
    }

    // Midfielders: LM, CM, CM, RM
    for i in 0..4 {
        players.push(CorePlayer {
            name: format!("Test MF{}", i),
            ca: 63,
            pa: 83,
            position: match i {
                0 => "LM",
                1 | 2 => "CM",
                3 => "RM",
                _ => "CM",
            }
            .into(),
            condition: 0.75,
        });
    }

    // Forwards: ST, ST
    for i in 0..2 {
        players.push(CorePlayer {
            name: format!("Test ST{}", i),
            ca: 54, // Same as 이서준
            pa: 199,
            position: "ST".into(),
            condition: 0.50,
        });
    }

    eprintln!("Created team with {} players", players.len());
    eprintln!("First 11 positions:");
    for (idx, p) in players.iter().take(11).enumerate() {
        eprintln!(
            "  [{}] {} | Pos: {} | CA: {}",
            idx, p.name, p.position, p.ca
        );
    }

    let home = CoreTeam {
        name: "Test Home".into(),
        players: players.clone(),
        formation: Some("4-4-2".into()),
        #[cfg(feature = "vendor_skills")]
        tactics: None,
        #[cfg(feature = "vendor_skills")]
        preferred_style: None,
        substitutes: None,
        captain_name: None,
        penalty_taker_name: None,
        free_kick_taker_name: None,
        auto_select_roles: false,
        team_morale: None,
        recent_results: None,
    };

    let away = CoreTeam {
        name: "Test Away".into(),
        players: players,
        formation: Some("4-4-2".into()),
        #[cfg(feature = "vendor_skills")]
        tactics: None,
        #[cfg(feature = "vendor_skills")]
        preferred_style: None,
        substitutes: None,
        captain_name: None,
        penalty_taker_name: None,
        free_kick_taker_name: None,
        auto_select_roles: false,
        team_morale: None,
        recent_results: None,
    };

    let config = MatchConfig {
        home,
        away,
        seed: 123478056, // Same seed as the crash
        auto_select_tactics: true,
        home_instructions: None,
        away_instructions: None,
        highlight_level: None,
        player_name: None,
        tick_interval_ms: None,
        include_position_data: false,
        include_stored_events: false,
        #[cfg(feature = "vendor_skills")]
        kickoff_tactics: None,
        kickoff_team_instructions: None,
        use_contextual_tactics: false,
        #[cfg(feature = "vendor_skills")]
        home_manager: None,
        #[cfg(feature = "vendor_skills")]
        away_manager: None,
    };

    eprintln!("=== Calling simulate_match ===");
    match simulate_match(&config) {
        Ok(result) => {
            eprintln!("✅ Match completed successfully!");
            eprintln!("Engine event count: {}", result.engine_event_count);
            eprintln!("Replay event count: {}", result.replay.events.len());
        }
        Err(e) => {
            eprintln!("❌ Match simulation failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
