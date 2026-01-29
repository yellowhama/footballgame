use of_adapter::{simulate_match, CorePlayer, CoreTeam, MatchConfig};

fn main() {
    let home = CoreTeam {
        name: "Academy Team".to_string(),
        players: build_team_players("Home"),
        formation: Some("4-4-2".to_string()),
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
        name: "Rival Squad".to_string(),
        players: build_team_players("Away"),
        formation: Some("4-3-3".to_string()),
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
        seed: 42,
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

    match simulate_match(&config) {
        Ok(result) => {
            println!("engine_event_count={}", result.engine_event_count);
            println!("replay_events={}", result.replay.events.len());
            println!("timeline_entries={}", result.replay.timeline.len());
            println!(
                "roster_players=home:{} away:{}",
                result.replay.rosters.home.players.len(),
                result.replay.rosters.away.players.len()
            );
            if let Some(sample_home) = result.replay.rosters.home.players.get(0) {
                println!(
                    "sample_home_player={}",
                    serde_json::to_string_pretty(sample_home).unwrap()
                );
            }
        }
        Err(err) => {
            eprintln!("simulate_match failed: {err:?}");
            std::process::exit(1);
        }
    }
}

fn build_team_players(prefix: &str) -> Vec<CorePlayer> {
    let positions = [
        "GK", "RB", "CB", "CB", "LB", "DM", "CM", "CM", "RW", "LW", "ST",
    ];
    positions
        .iter()
        .enumerate()
        .map(|(idx, pos)| CorePlayer {
            name: format!("{} Player {}", prefix, idx + 1),
            ca: 60 + idx as u32,
            pa: 120 + idx as u32,
            position: pos.to_string(),
            condition: 0.85,
        })
        .collect()
}
