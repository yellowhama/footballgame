use of_core::simulate_match_json;

fn main() {
    println!("Quick test of simulate_match_json");

    let test_json = r#"{
        "schema_version": 1,
        "seed": 12345,
        "home_team": {
            "name": "Test Home",
            "formation": "4-4-2",
            "players": [
                {"name": "Player1", "position": "GK", "overall": 70}
            ]
        },
        "away_team": {
            "name": "Test Away",
            "formation": "4-4-2",
            "players": [
                {"name": "AI1", "position": "GK", "overall": 70}
            ]
        }
    }"#;

    println!("Starting simulation...");
    let start = std::time::Instant::now();

    match simulate_match_json(test_json) {
        Ok(result) => {
            let elapsed = start.elapsed();
            println!("Success in {:?}", elapsed);
            println!("Result length: {} bytes", result.len());
            // First 200 chars
            println!("Result preview: {}", &result[..200.min(result.len())]);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}