//! Engine Contract Tests (Contract v1.0)
//!
//! Verifies that the engine adheres to the Hard Rules:
//! 1. Coordinate SSOT (105x68)
//! 2. UID Resolve
//! 3. Event Actor
//! 4. Probability Sanity
//! 5. Budget Explosion

#[cfg(test)]
mod tests {
    use crate::engine::match_sim::decision_topology::*;
    use crate::engine::match_sim::test_fixtures::create_test_team;
    use crate::engine::match_sim::{MatchEngine, MatchPlan};
    use crate::engine::experimental::ExpConfig;
    use crate::models::{EventType, MatchEvent};

    #[test]
    fn test_coordinate_contract() {
        // 모든 선수의 좌표가 0-105, 0-68 범위 내에 있는지 확인
        // (정규화된 좌표 0-1 기준이므로 0-1 범위 체크 후 변환 체크)
        let mut engine = create_mock_engine();
        engine.init();

        for &pos in &engine.player_positions {
            assert!(pos.is_in_bounds(), "Player position out of bounds: {:?}", pos);
        }
    }

    #[test]
    fn test_uid_resolve_contract() {
        // 모든 선수의 track_id가 유효한 UID로 매핑되는지 확인
        let engine = create_mock_engine();
        for i in 0..22 {
            let player = engine.get_match_player(i);
            assert!(!player.name.is_empty(), "Player {} should have a name/UID", i);
        }
    }

    #[test]
    fn test_probability_sanity() {
        // Softmax 합이 1인지 확인
        use crate::engine::match_sim::utility::softmax_probabilities;
        let utilities = vec![0.5, -0.2, 0.8, 0.1];
        let probs = softmax_probabilities(&utilities, 0.3);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);

        for p in probs {
            assert!((0.0..=1.0).contains(&p));
        }
    }

    #[test]
    fn test_budget_explosion_gate() {
        // Budget Gate가 연타를 막는지 확인
        let ctx = DecisionContext {
            action_history: Some(ActionHistory {
                recent_shots: 5, // 이미 5번 쏨
                recent_tackles: 0,
                stamina: 1.0,
            }),
            ..Default::default()
        };

        let candidates = vec![CandidateAction::ShootNormal];
        let gated = apply_budget_gates(&candidates, &ctx);

        // 슛에 대한 패널티가 적용되어야 함 (modifier < 0.1)
        assert!(gated[0].1.to_weight() < 0.1);
    }

    #[test]
    fn test_event_actor_contract() {
        // Test case 1: Empty event list (should pass)
        let empty_events: Vec<MatchEvent> = vec![];
        assert!(validate_event_actors(&empty_events, 0.01).is_ok());

        // Test case 2: All events have actors (should pass)
        let events_with_actors = vec![
            MatchEvent {
                minute: 10,
                timestamp_ms: Some(600000),
                event_type: EventType::Shot,
                is_home_team: true,
                player_track_id: Some(5), // Has actor
                target_track_id: None,
                details: None,
            },
            MatchEvent {
                minute: 15,
                timestamp_ms: Some(900000),
                event_type: EventType::Pass,
                is_home_team: true,
                player_track_id: Some(7), // Has actor
                target_track_id: Some(9),
                details: None,
            },
            MatchEvent {
                minute: 20,
                timestamp_ms: Some(1200000),
                event_type: EventType::Tackle,
                is_home_team: false,
                player_track_id: Some(15), // Has actor
                target_track_id: None,
                details: None,
            },
        ];

        assert!(validate_event_actors(&events_with_actors, 0.01).is_ok());

        // Test case 3: Above threshold (should fail)
        let mut above_threshold = vec![MatchEvent {
            minute: 10,
            timestamp_ms: Some(600000),
            event_type: EventType::Shot,
            is_home_team: true,
            player_track_id: Some(5),
            target_track_id: None,
            details: None,
        }];

        // Add 2 events without actors (2/3 = 66% > 1%)
        for i in 0..2 {
            above_threshold.push(MatchEvent {
                minute: 20 + i,
                timestamp_ms: Some(1200000 + i as u64 * 100000),
                event_type: EventType::Dribble,
                is_home_team: true,
                player_track_id: None, // Missing actor
                target_track_id: None,
                details: None,
            });
        }

        let result = validate_event_actors(&above_threshold, 0.01);
        assert!(result.is_err());

        // Verify error message
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("2/3"));
        assert!(err_msg.contains("66.7%"));

        // Test case 4: Mix of major and non-major events (should ignore non-major)
        let mut mixed_events = vec![
            // Major event with actor
            MatchEvent {
                minute: 10,
                timestamp_ms: Some(600000),
                event_type: EventType::Shot,
                is_home_team: true,
                player_track_id: Some(5),
                target_track_id: None,
                details: None,
            },
        ];

        // Non-major events (should be ignored even without actors)
        mixed_events.push(MatchEvent {
            minute: 0,
            timestamp_ms: Some(0),
            event_type: EventType::KickOff,
            is_home_team: true,
            player_track_id: None, // OK - not major event
            target_track_id: None,
            details: None,
        });
        mixed_events.push(MatchEvent {
            minute: 30,
            timestamp_ms: Some(1800000),
            event_type: EventType::Corner,
            is_home_team: false,
            player_track_id: None, // OK - not major event
            target_track_id: None,
            details: None,
        });

        // Should pass: 0/1 major events missing actors (non-major ignored)
        assert!(validate_event_actors(&mixed_events, 0.01).is_ok());
    }
    #[test]
    fn test_full_time_event_emitted_on_simulate() {
        let mut engine = create_mock_engine();
        let result = engine.simulate();
        assert!(
            result
                .events
                .iter()
                .any(|event| matches!(event.event_type, EventType::FullTime))
        );
    }

    /// Validate that major events have actor_track_id set
    ///
    /// Contract 6.3 (ENGINE_SSOT_AND_CI_CONTRACT.md):
    /// Major events (pass, shot, tackle, dribble, foul, goal) must have actor_track_id.
    /// Returns error if ratio of events without actors exceeds threshold.
    ///
    /// # Arguments
    /// * `events` - Event list to validate
    /// * `threshold` - Maximum allowed ratio of events without actors (0.01 = 1%)
    ///
    /// # Returns
    /// Ok(()) if contract satisfied, Err(message) otherwise
    fn validate_event_actors(events: &[MatchEvent], threshold: f32) -> Result<(), String> {
        // Define major event types (require actor per contract)
        let major_types = [
            EventType::Shot,
            EventType::ShotOnTarget,
            EventType::ShotOffTarget,
            EventType::ShotBlocked,
            EventType::Pass,
            EventType::Tackle,
            EventType::Dribble,
            EventType::Foul,
            EventType::Goal,
        ];

        // Filter to major events only
        let major_events: Vec<_> =
            events.iter().filter(|e| major_types.contains(&e.event_type)).collect();

        // Empty is OK (no events to check)
        if major_events.is_empty() {
            return Ok(());
        }

        // Count events missing actor
        let missing_actors = major_events.iter().filter(|e| e.player_track_id.is_none()).count();

        // Calculate ratio
        let ratio = missing_actors as f32 / major_events.len() as f32;

        // Validate against threshold
        if ratio > threshold {
            Err(format!(
                "Event actor contract violation: {}/{} ({:.1}%) major events missing actor (threshold: {:.1}%)",
                missing_actors,
                major_events.len(),
                ratio * 100.0,
                threshold * 100.0
            ))
        } else {
            Ok(())
        }
    }

    #[test]
    fn test_rng_gate_contract() {
        // Contract 1.4 (ENGINE_SSOT_AND_CI_CONTRACT.md):
        // Gate 내부에서는 RNG 호출 금지
        // RNG는 오직 softmax sampling / duel 성공 판정에서만 허용

        let gate_source = include_str!("../weights/gate.rs");
        let budget_source = include_str!("../weights/budget.rs");

        // 금지 패턴 (대소문자 구분)
        let forbidden_patterns =
            ["rand::", "Rng", ".rng", "rng.", ".roll(", "roll(", "gen::<", "thread_rng"];

        for pattern in forbidden_patterns {
            assert!(
                !gate_source.contains(pattern),
                "RNG contract violation: gate.rs contains forbidden pattern '{}' - Gate 내부에서 RNG 호출 금지",
                pattern
            );
            assert!(
                !budget_source.contains(pattern),
                "RNG contract violation: budget.rs contains forbidden pattern '{}' - Gate 내부에서 RNG 호출 금지",
                pattern
            );
        }

        // softmax.rs는 허용 (Weight → Probability 변환 이후)
        // duel.rs는 허용 (duel 성공 판정)
        // 위 두 모듈은 RNG 사용 OK이므로 체크하지 않음
    }

    // Helper: Mock Engine 생성 - use shared test fixtures
    #[allow(dead_code)]
    fn create_mock_engine() -> super::super::MatchEngine {
        crate::engine::match_sim::test_fixtures::create_test_engine()
    }

    #[test]
    fn test_dpq_v1_1_no_behavior_change_determinism() {
        // FIX_2601/0113 v1.1: DPQ는 "routing-only" 이므로 dpq_enabled on/off에서
        // 결과 이벤트 해시가 완전히 동일해야 한다.
        //
        // NOTE: full 90min simulation은 느리므로, step_decision_tick_streaming()으로
        // 짧은 구간만 실행하여 빠른 회귀 테스트로 유지한다.

        use sha2::{Digest, Sha256};

        fn sha256_hex(bytes: &[u8]) -> String {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            let digest = hasher.finalize();
            let mut out = String::with_capacity(digest.len() * 2);
            for b in digest {
                out.push_str(&format!("{:02x}", b));
            }
            out
        }

        let plan = MatchPlan {
            home_team: create_test_team("Home"),
            away_team: create_test_team("Away"),
            seed: 12345,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_instructions: None,
            away_instructions: None,
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: None,
            away_ai_difficulty: None,
        };

        let match_duration_min: u8 = 2;

        // Baseline (dpq_enabled=false)
        let mut engine_base = MatchEngine::new(plan.clone()).expect("match engine init");
        let (hs, as_, poss, _duration) = engine_base.init();
        engine_base.current_tick = 0;
        while engine_base.step_decision_tick_streaming(hs, as_, poss, match_duration_min) {}

        let base_events_json =
            serde_json::to_string(&engine_base.result.events).expect("events json");
        let base_hash = sha256_hex(base_events_json.as_bytes());

        // DPQ enabled (v1.1 routing-only)
        let mut dpq_cfg = ExpConfig::default();
        dpq_cfg.decision.dpq_enabled = true;

        let mut engine_dpq = MatchEngine::new(plan)
            .expect("match engine init")
            .with_exp_config(&dpq_cfg);
        let (hs2, as2, poss2, _duration2) = engine_dpq.init();
        engine_dpq.current_tick = 0;
        while engine_dpq.step_decision_tick_streaming(hs2, as2, poss2, match_duration_min) {}

        let dpq_events_json = serde_json::to_string(&engine_dpq.result.events).expect("events json");
        let dpq_hash = sha256_hex(dpq_events_json.as_bytes());

        assert_ne!(base_hash, "", "expected non-empty hash");
        assert_eq!(
            base_hash, dpq_hash,
            "DPQ v1.1 must not change event stream (routing-only)"
        );

        assert_eq!(engine_dpq.result.statistics.decisions_skipped, 0);
        assert_eq!(
            engine_base.result.statistics.decisions_executed,
            engine_dpq.result.statistics.decisions_executed,
            "DPQ v1.1 must not change decision execution count"
        );
    }
}

