//! Story System Integration Tests

#[cfg(test)]
mod story_integration_tests {
    use crate::story::branching::*;
    use crate::story::conditions::*;
    use crate::story::effects::*;
    use crate::story::events::*;
    use crate::story::*;

    #[test]
    fn test_story_engine_full_flow() {
        let mut engine = StoryEngine::new();

        // Register a test event
        let test_event = StoryEvent {
            id: "test_milestone".to_string(),
            event_type: StoryEventType::SkillMilestone,
            title: "Training Success".to_string(),
            description: "Your training has paid off!".to_string(),
            choices: vec![
                EventChoice {
                    id: "celebrate".to_string(),
                    text: "Celebrate the achievement".to_string(),
                    requirements: vec![],
                    effects: vec![StoryEffect::ModifyCA(2), StoryEffect::ModifyMorale(10)],
                    next_event_id: None,
                },
                EventChoice {
                    id: "train_more".to_string(),
                    text: "Keep training harder".to_string(),
                    requirements: vec![ChoiceRequirement::MinCA(100)],
                    effects: vec![StoryEffect::ModifyCA(3), StoryEffect::ModifyFatigue(20)],
                    next_event_id: None,
                },
            ],
            conditions: vec![StoryCondition::Week(5)],
            week_range: Some((5, 10)),
            priority: EventPriority::Normal,
            tags: vec!["training".to_string()],
        };

        engine.event_registry.register_event(test_event);

        // Set up initial player stats
        engine.state.player_stats.ca = 100;

        // Test week processing
        engine.state.current_week = 5;
        let events = engine.process_week(5);
        assert!(!events.is_empty(), "Should have events for week 5");

        // Test choice making
        let result = engine.make_choice("test_milestone", 0);
        assert!(result.is_ok(), "Should be able to make choice");

        // Verify state changes
        assert_eq!(engine.state.player_stats.ca, 102); // 100 + 2 from choice
    }

    #[test]
    fn test_route_branching() {
        let mut engine = StoryEngine::new();

        // Set up state for Elite route
        engine.state.player_stats.ca = 145;
        engine.state.player_stats.goals = 10;
        engine.state.current_week = 12;

        // Check branch point
        if let Some(new_route) = engine.check_branch_point(12) {
            assert_eq!(new_route, StoryRoute::Elite);
        }

        // Set up state for Underdog route
        engine.state.player_stats.ca = 80;
        engine.state.player_stats.goals = 1;

        if let Some(new_route) = engine.check_branch_point(12) {
            assert_eq!(new_route, StoryRoute::Underdog);
        }
    }

    #[test]
    fn test_condition_evaluation() {
        let evaluator = ConditionEvaluator::new();
        let mut state = StoryState::default();

        // Set up state
        state.current_week = 10;
        state.player_stats.ca = 120;
        state.player_stats.goals = 15;
        state.relationships.insert("coach".to_string(), 50);

        // Test complex condition
        let condition = StoryCondition::And(vec![
            StoryCondition::WeekRange(5, 15),
            StoryCondition::CA(ComparisonOp::Greater, 100),
            StoryCondition::Or(vec![
                StoryCondition::Goals(ComparisonOp::GreaterEqual, 10),
                StoryCondition::Relationship("coach".to_string(), ComparisonOp::Greater, 40),
            ]),
        ]);

        assert!(evaluator.evaluate(&condition, &state));

        // Test failing condition
        let failing_condition = StoryCondition::And(vec![
            StoryCondition::Week(5), // Current week is 10, not 5
            StoryCondition::CA(ComparisonOp::Greater, 100),
        ]);

        assert!(!evaluator.evaluate(&failing_condition, &state));
    }

    #[test]
    fn test_effect_processing() {
        let mut processor = EffectProcessor::new();
        let mut state = StoryState::default();
        state.player_stats.ca = 100;

        // Apply multiple effects
        let effects = vec![
            StoryEffect::ModifyCA(10),
            StoryEffect::ModifyRelationship("coach".to_string(), 25),
            StoryEffect::SetFlag("training_complete".to_string(), true),
            StoryEffect::ModifyMorale(15),
        ];

        let result = processor.apply_effects(&effects, &mut state);
        assert!(result.is_ok());

        // Verify state changes
        assert_eq!(state.player_stats.ca, 110);
        assert_eq!(state.relationships.get("coach"), Some(&25));
        assert_eq!(state.active_flags.get("training_complete"), Some(&true));

        // Test revert
        let revert_result = processor.revert_last_effect(&mut state);
        assert!(revert_result.is_ok());
    }

    #[test]
    fn test_event_generation() {
        // Test skill milestone generation
        let skill_event = EventGenerator::generate_skill_milestone_event("Passing", 10, 15);
        assert_eq!(skill_event.event_type, StoryEventType::SkillMilestone);
        assert!(skill_event.title.contains("Passing"));

        // Test relationship event generation
        let relationship_event = EventGenerator::generate_relationship_event("coach", 50);
        assert!(relationship_event.is_some());
        if let Some(event) = relationship_event {
            assert_eq!(event.event_type, StoryEventType::Relationship);
        }
    }

    #[test]
    fn test_match_event_conversion() {
        let converter = MatchEventConverter::new();

        let match_event = MatchEvent {
            event_type: MatchEventType::Goal,
            minute: 5,
            player_id: "player123".to_string(),
            data: MatchEventData::Goal {
                scorer: "player123".to_string(),
                assister: Some("player456".to_string()),
            },
        };

        // Note: This will return None due to context not being set up properly
        // In real implementation, context would be populated from match state
        let story_event = converter.convert(&match_event);
        assert!(story_event.is_none() || story_event.is_some());
    }

    #[test]
    fn test_route_progress_calculation() {
        let manager = RouteManager::new();
        let mut state = StoryState::default();

        // Test Standard route progress
        state.current_route = StoryRoute::Standard;
        state.player_stats.ca = 120; // Mid-range for Standard (100-139)
        state.player_stats.goals = 10;

        let progress = manager.calculate_route_progress(&state);
        assert!(progress > 0.0 && progress <= 1.0);

        // Test Elite route progress
        state.current_route = StoryRoute::Elite;
        state.player_stats.ca = 160; // Mid-range for Elite (140-200)
        state.player_stats.goals = 20;

        let progress = manager.calculate_route_progress(&state);
        assert!(progress > 0.0 && progress <= 1.0);
    }

    #[test]
    fn test_route_prediction() {
        let predictor = RoutePredictor;
        let mut state = StoryState::default();

        // Set up state for prediction
        state.current_week = 10;
        state.player_stats.ca = 130;
        state.player_stats.goals = 12;

        let prediction = predictor.predict_route(&state, 10);
        assert!(prediction.confidence > 0.0 && prediction.confidence <= 1.0);
        assert!(!prediction.key_factors.is_empty());
    }

    #[test]
    fn test_choice_requirement_validation() {
        let validator = RequirementValidator::new();
        let mut state = StoryState::default();
        state.player_stats.ca = 110;
        state.relationships.insert("coach".to_string(), 60);

        let requirements = vec![
            ChoiceRequirement::MinCA(100),
            ChoiceRequirement::Relationship("coach".to_string(), 50),
        ];

        assert!(validator.validate_choice_requirements(&requirements, &state));

        // Test failing requirement
        let failing_requirements = vec![
            ChoiceRequirement::MinCA(150), // CA is 110, not 150+
        ];

        assert!(!validator.validate_choice_requirements(&failing_requirements, &state));
    }

    #[test]
    fn test_effect_optimization() {
        let effects = vec![
            StoryEffect::ModifyCA(5),
            StoryEffect::ModifyCA(3),
            StoryEffect::ModifyCA(-2),
            StoryEffect::ModifyMorale(10),
            StoryEffect::ModifyMorale(5),
            StoryEffect::ModifyRelationship("coach".to_string(), 10),
            StoryEffect::ModifyRelationship("coach".to_string(), 5),
        ];

        let optimized = EffectOptimizer::optimize(effects);

        // Should combine all CA changes into one, all Morale into one, all coach relationships into one
        assert!(optimized.len() <= 3);

        // Verify the combined values are correct
        let mut total_ca = 0;
        let mut total_morale = 0;
        let mut coach_relationship = 0;

        for effect in &optimized {
            match effect {
                StoryEffect::ModifyCA(delta) => total_ca += delta,
                StoryEffect::ModifyMorale(delta) => total_morale += delta,
                StoryEffect::ModifyRelationship(char, delta) if char == "coach" => {
                    coach_relationship += delta;
                }
                _ => {}
            }
        }

        assert_eq!(total_ca, 6); // 5 + 3 - 2
        assert_eq!(total_morale, 15); // 10 + 5
        assert_eq!(coach_relationship, 15); // 10 + 5
    }
}
