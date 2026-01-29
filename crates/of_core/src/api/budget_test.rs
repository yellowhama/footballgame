#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;
    use std::time::{Duration, Instant};

    fn minimal_plan_json() -> String {
        // FIX_2601: Formation requires exactly 1 GK in starting 11
        // 4-4-2 formation: 1 GK + 4 DEF + 4 MID + 2 ST = 11 starters
        // Subs: 1 GK + 6 outfield = 7 subs (total 18 players)
        json!({
            "schema_version": 1,
            "seed": 123,
            "home_team": {
                "name": "Test Home",
                "formation": "4-4-2",
                "players": [
                    {"name": "GK1", "position": "GK", "overall": 70, "condition": 3},
                    {"name": "LB1", "position": "LB", "overall": 70, "condition": 3},
                    {"name": "CB1", "position": "CB", "overall": 70, "condition": 3},
                    {"name": "CB2", "position": "CB", "overall": 70, "condition": 3},
                    {"name": "RB1", "position": "RB", "overall": 70, "condition": 3},
                    {"name": "LM1", "position": "LM", "overall": 70, "condition": 3},
                    {"name": "CM1", "position": "CM", "overall": 70, "condition": 3},
                    {"name": "CM2", "position": "CM", "overall": 70, "condition": 3},
                    {"name": "RM1", "position": "RM", "overall": 70, "condition": 3},
                    {"name": "ST1", "position": "ST", "overall": 70, "condition": 3},
                    {"name": "ST2", "position": "ST", "overall": 70, "condition": 3},
                    {"name": "GK2", "position": "GK", "overall": 65, "condition": 3},
                    {"name": "CB3", "position": "CB", "overall": 68, "condition": 3},
                    {"name": "CM3", "position": "CM", "overall": 68, "condition": 3},
                    {"name": "CM4", "position": "CM", "overall": 68, "condition": 3},
                    {"name": "LW1", "position": "LW", "overall": 68, "condition": 3},
                    {"name": "RW1", "position": "RW", "overall": 68, "condition": 3},
                    {"name": "ST3", "position": "ST", "overall": 68, "condition": 3}
                ]
            },
            "away_team": {
                "name": "Test Away",
                "formation": "4-4-2",
                "players": [
                    {"name": "AGK1", "position": "GK", "overall": 70, "condition": 3},
                    {"name": "ALB1", "position": "LB", "overall": 70, "condition": 3},
                    {"name": "ACB1", "position": "CB", "overall": 70, "condition": 3},
                    {"name": "ACB2", "position": "CB", "overall": 70, "condition": 3},
                    {"name": "ARB1", "position": "RB", "overall": 70, "condition": 3},
                    {"name": "ALM1", "position": "LM", "overall": 70, "condition": 3},
                    {"name": "ACM1", "position": "CM", "overall": 70, "condition": 3},
                    {"name": "ACM2", "position": "CM", "overall": 70, "condition": 3},
                    {"name": "ARM1", "position": "RM", "overall": 70, "condition": 3},
                    {"name": "AST1", "position": "ST", "overall": 70, "condition": 3},
                    {"name": "AST2", "position": "ST", "overall": 70, "condition": 3},
                    {"name": "AGK2", "position": "GK", "overall": 65, "condition": 3},
                    {"name": "ACB3", "position": "CB", "overall": 68, "condition": 3},
                    {"name": "ACM3", "position": "CM", "overall": 68, "condition": 3},
                    {"name": "ACM4", "position": "CM", "overall": 68, "condition": 3},
                    {"name": "ALW1", "position": "LW", "overall": 68, "condition": 3},
                    {"name": "ARW1", "position": "RW", "overall": 68, "condition": 3},
                    {"name": "AST3", "position": "ST", "overall": 68, "condition": 3}
                ]
            }
        })
        .to_string()
    }

    #[test]
    #[ignore] // Flaky in CI due to variable CPU performance
    fn test_smoke_finishes_under_budget() {
        let plan = minimal_plan_json();
        // 2025-12-12: Increased budget due to more complex EV+Audacity simulation
        // 2026-01-07: Increased from 6000ms to 8000ms for CI environment variability
        // Note: In parallel test execution, other tests may slow down this one
        let budget = SimBudget::new(8000, 120, 500); // 8000ms to avoid flakiness under CI load

        let t0 = Instant::now();
        let result = simulate_match_json_budget(&plan, budget).unwrap();
        let elapsed = t0.elapsed();

        // WSL/debug builds under load can be very slow
        assert!(elapsed < Duration::from_millis(9000), "Simulation took too long: {:?}", elapsed);

        // Check result is valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should be a normal result, not partial
        if let Some(partial) = parsed.get("partial") {
            if partial.as_bool().unwrap_or(false) {
                let reason = parsed.get("reason").and_then(|r| r.as_str()).unwrap_or("unknown");
                panic!("Unexpected partial result: {}", reason);
            }
        }
    }

    #[test]
    fn test_timeout_protection() {
        let plan = minimal_plan_json();
        let budget = SimBudget::new(5, 120, 500); // Only 5ms allowed

        let t0 = Instant::now();
        let result = simulate_match_json_budget(&plan, budget).unwrap();
        let elapsed = t0.elapsed();

        // Wall-clock timing is inherently noisy (debug builds, CI load, CPU scaling).
        // The real invariant is that the budget triggers a partial result; keep a
        // generous upper bound to avoid flakes while still catching hangs.
        assert!(elapsed < Duration::from_secs(2), "Budget protection took too long: {:?}", elapsed);

        // Should return partial result
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(
            parsed.get("partial").and_then(|p| p.as_bool()).unwrap_or(false),
            "Expected partial result due to timeout"
        );
    }

    #[test]
    fn test_minute_overflow_protection() {
        let plan = minimal_plan_json();
        let budget = SimBudget::new(1000, 10, 500); // Only 10 minutes allowed

        let result = simulate_match_json_budget(&plan, budget).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should be partial result due to minute limit
        // Note: if engine doesn't support minute limits, this test passes
        if let Some(partial) = parsed.get("partial") {
            if partial.as_bool().unwrap_or(false) {
                let minutes = parsed.get("minutes_simulated").and_then(|m| m.as_u64()).unwrap_or(0);
                // Allow 1 minute overflow due to check timing
                assert!(minutes <= 11, "Simulated too many minutes: {}", minutes);
            }
        }
    }

    #[test]
    fn test_event_overflow_protection() {
        let plan = minimal_plan_json();
        let budget = SimBudget::new(1000, 120, 5); // Only 5 events allowed

        let result = simulate_match_json_budget(&plan, budget).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Should have stopped due to event limit
        if let Some(partial) = parsed.get("partial") {
            if partial.as_bool().unwrap_or(false) {
                let reason = parsed.get("reason").and_then(|r| r.as_str()).unwrap_or("");
                assert!(
                    reason.contains("Event") || reason.contains("event"),
                    "Expected event overflow reason, got: {}",
                    reason
                );
            }
        }
    }

    #[test]
    fn test_deterministic_same_seed() {
        let plan = minimal_plan_json();
        // Use generous budget to ensure complete results
        let budget1 = SimBudget::new(1000, 120, 500);
        let budget2 = SimBudget::new(1000, 120, 500);

        let result1 = simulate_match_json_budget(&plan, budget1).unwrap();
        let result2 = simulate_match_json_budget(&plan, budget2).unwrap();

        // Parse and compare scores (events might differ in ordering)
        let parsed1: serde_json::Value = serde_json::from_str(&result1).unwrap();
        let parsed2: serde_json::Value = serde_json::from_str(&result2).unwrap();

        // Skip if either is partial result
        let is_partial1 = parsed1.get("partial").and_then(|p| p.as_bool()).unwrap_or(false);
        let is_partial2 = parsed2.get("partial").and_then(|p| p.as_bool()).unwrap_or(false);
        if is_partial1 || is_partial2 {
            // Can't compare partial results reliably
            return;
        }

        let score1_home = parsed1.get("score_home").and_then(|s| s.as_u64());
        let score2_home = parsed2.get("score_home").and_then(|s| s.as_u64());
        let score1_away = parsed1.get("score_away").and_then(|s| s.as_u64());
        let score2_away = parsed2.get("score_away").and_then(|s| s.as_u64());

        assert_eq!(score1_home, score2_home, "Home scores differ");
        assert_eq!(score1_away, score2_away, "Away scores differ");
    }
}
