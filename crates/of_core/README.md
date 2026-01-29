# of_core - Deterministic Football Match Simulation Engine

A high-performance, deterministic football match simulation engine written in Rust for the Football Player Game.

## Features

- **100% Deterministic**: Same seed always produces identical results
- **Realistic Simulation**: Realistic score distributions (2.5-3 goals average)
- **High Performance**: <2ms per match simulation
- **JSON API**: Easy integration with game engines
- **Cross-Platform**: Windows, Linux, Android, iOS support

## Usage

```rust
use of_core::simulate_match_json;

let request = r#"{
    "schema_version": 1,
    "seed": 12345,
    "home_team": {
        "name": "Seoul FC",
        "formation": "4-4-2",
        "players": [...]
    },
    "away_team": {
        "name": "Busan FC",
        "formation": "4-3-3",
        "players": [...]
    }
}"#;

let result = simulate_match_json(request).unwrap();
println!("{}", result);
```

## API

### Request Format

```json
{
    "schema_version": 1,
    "seed": 12345,
    "home_team": {
        "name": "Team Name",
        "formation": "4-4-2",
        "players": [
            {
                "name": "Player Name",
                "position": "GK",
                "overall": 75
            }
            // ... 18 players total (11 starting + 7 subs)
        ]
    },
    "away_team": {
        // Same structure as home_team
    }
}
```

### Response Format

```json
{
    "schema_version": 1,
    "score_home": 2,
    "score_away": 1,
    "events": [
        {
            "minute": 23,
            "type": "goal",
            "is_home_team": true,
            "player": "Player Name",
            "details": {
                "assist_by": "Assist Player"
            }
        }
        // ... 20-30 events per match
    ],
    "statistics": {
        "possession_home": 52.5,
        "possession_away": 47.5,
        "shots_home": 12,
        "shots_away": 8,
        "shots_on_target_home": 5,
        "shots_on_target_away": 3,
        "xg_home": 1.8,
        "xg_away": 1.2,
        // ... more statistics
    }
}
```

## Supported Formations

- 4-4-2
- 4-3-3
- 3-5-2
- 5-3-2
- 4-2-3-1
- 4-1-4-1
- 3-4-3
- 5-4-1

## Player Positions

**Goalkeepers**: GK
**Defenders**: LB, CB, RB, LWB, RWB, DF
**Midfielders**: CDM, CM, CAM, LM, RM, MF
**Forwards**: LW, RW, CF, ST, FW

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test        # Run all tests
cargo bench       # Run benchmarks
```

## Performance

- Single match: <2ms (p99)
- Season (380 matches): <3 seconds
- Parallel: 10,000 matches/second

## License

MIT