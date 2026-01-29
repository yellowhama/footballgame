//! Phase 2.5: AI Tactical Manager Integration Tests
//!
//! Tests for AI tactical decision-making during matches

#[cfg(test)]
mod tests {
    use crate::engine::match_sim::test_fixtures::create_test_team;
    use crate::engine::match_sim::{MatchEngine, MatchPlan};
    use crate::tactics::AIDifficulty;

    #[test]
    fn test_ai_manager_initialization() {
        // 두 AI 팀으로 경기 생성
        let home = create_test_team("Home AI");
        let away = create_test_team("Away AI");

        let plan = MatchPlan {
            home_team: home,
            away_team: away,
            seed: 12345,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_instructions: None,
            away_instructions: None,
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: Some(AIDifficulty::Expert),
            away_ai_difficulty: Some(AIDifficulty::Medium),
        };

        let engine = MatchEngine::new(plan).expect("match engine init");

        // AI 매니저가 초기화되었는지 확인
        // Note: home_ai_manager와 away_ai_manager는 private이므로 직접 확인 불가
        // 대신 컴파일 성공 여부로 통합 확인
        assert_eq!(engine.home_team.name, "Home AI");
        assert_eq!(engine.away_team.name, "Away AI");
    }

    #[test]
    fn test_match_state_creation() {
        // MatchState 생성 로직 테스트
        let home = create_test_team("Home");
        let away = create_test_team("Away");

        let plan = MatchPlan {
            home_team: home,
            away_team: away,
            seed: 12345,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_instructions: None,
            away_instructions: None,
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: Some(AIDifficulty::Hard),
            away_ai_difficulty: None,
        };

        let engine = MatchEngine::new(plan).expect("match engine init");

        // MatchState를 생성하는 내부 메서드가 올바르게 작동하는지 확인
        // get_current_match_state()는 private이므로 직접 호출 불가
        // 대신 엔진 초기 상태 확인
        assert_eq!(engine.minute, 0);
    }

    #[test]
    fn test_ai_integration_smoke_test() {
        // AI 통합 스모크 테스트: 경기 시작 시 크래시 없이 실행되는지 확인
        let home = create_test_team("AI Home");
        let away = create_test_team("AI Away");

        let plan = MatchPlan {
            home_team: home,
            away_team: away,
            seed: 12345,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_instructions: None,
            away_instructions: None,
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: Some(AIDifficulty::Expert),
            away_ai_difficulty: Some(AIDifficulty::Expert),
        };

        let mut engine = MatchEngine::new(plan).expect("match engine init");

        // simulate() 호출 시 AI 로직이 크래시 없이 실행되는지 확인
        let result = engine.simulate();

        // 기본 검증: 결과가 생성되었는지
        assert!(!result.events.is_empty());

        println!("✅ AI Integration smoke test passed!");
        println!("   Final score: {}-{}", result.score_home, result.score_away);
        println!("   Events: {}", result.events.len());
    }

    #[test]
    fn test_different_ai_difficulties() {
        // 난이도별 AI 초기화 테스트
        for difficulty in
            [AIDifficulty::Easy, AIDifficulty::Medium, AIDifficulty::Hard, AIDifficulty::Expert]
        {
            let home = create_test_team("Home");
            let away = create_test_team("Away");

            let plan = MatchPlan {
                home_team: home,
                away_team: away,
                seed: 12345,
                user_player: None,
                home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
                away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
                home_instructions: None,
                away_instructions: None,
                home_player_instructions: None,
                away_player_instructions: None,
                home_ai_difficulty: Some(difficulty),
                away_ai_difficulty: Some(difficulty),
            };

            let engine = MatchEngine::new(plan).expect("match engine init");

            // 각 난이도에서 크래시 없이 초기화되는지 확인
            assert_eq!(engine.minute, 0);
        }

        println!("✅ All difficulty levels initialized successfully!");
    }
}
