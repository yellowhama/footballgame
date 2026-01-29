//! Replay generator utilities used in tests (proptest + fallback).

use super::types::{
    EventBase, MeterPos, PitchSpec, ReplayDoc, ReplayEvent, ReplayRosters, ReplayTeamsTactics,
};

#[cfg(feature = "proptest")]
use proptest::prelude::*;

// ============================================================================
// Proptest strategies
// ============================================================================

#[cfg(feature = "proptest")]
pub fn arb_meter_pos() -> impl Strategy<Value = MeterPos> {
    (0.0f64..105.0, 0.0f64..68.0).prop_map(|(x, y)| MeterPos { x, y })
}

#[cfg(feature = "proptest")]
pub fn arb_event_base() -> impl Strategy<Value = EventBase> {
    (0f64..90f64 * 60f64, prop::option::of(0u32..22), prop::option::of(0u32..2))
        .prop_map(|(t, player_id, team_id)| EventBase { t, player_id, team_id })
}

#[cfg(feature = "proptest")]
pub fn arb_card_type() -> impl Strategy<Value = CardType> {
    prop_oneof![Just(CardType::Yellow), Just(CardType::Red),]
}

#[cfg(feature = "proptest")]
pub fn arb_replay_event() -> impl Strategy<Value = ReplayEvent> {
    prop_oneof![
        // Basic match flow events
        arb_event_base().prop_map(|base| ReplayEvent::KickOff { base }),
        (arb_event_base(), arb_meter_pos(), arb_meter_pos()).prop_map(|(base, from, to)| {
            ReplayEvent::Pass {
                base,
                from,
                to,
                receiver_id: None,
                distance_m: None,
                force: None,
                is_clearance: false,
                ground: None,
                outcome: None,
                passing_skill: None,
                vision: None,
                technique: None,
                // 0108 Phase 4: Tactical metadata
                danger_level: None,
                is_switch_of_play: None,
                is_line_breaking: None,
                is_through_ball: None,
            }
        }),
        (arb_event_base(), arb_meter_pos(), any::<bool>()).prop_map(|(base, from, on_target)| {
            ReplayEvent::Shot {
                base,
                from,
                target: from,
                on_target,
                xg: None,
                shot_speed: None,
                long_shots_skill: None,
                finishing_skill: None,
                technique: None,
                shot_type: None,
                defender_pressure: None,
                angle_to_goal: None,
                distance_to_goal: None,
                composure: None,
                curve_factor: None,
            }
        },),
        (arb_event_base(), arb_meter_pos()).prop_map(|(base, at)| ReplayEvent::Goal {
            base,
            at,
            assist_player_id: None,
        }),
        (arb_event_base(), arb_meter_pos()).prop_map(|(base, at)| ReplayEvent::Foul {
            base,
            at,
            foul_type: None,
            severity: None,
            intentional: None,
            location_danger: None,
            aggression_level: None,
        }),
        (arb_event_base(), arb_meter_pos())
            .prop_map(|(base, spot)| ReplayEvent::FreeKick { base, spot }),
        (arb_event_base(), arb_meter_pos())
            .prop_map(|(base, spot)| ReplayEvent::CornerKick { base, spot }),
        (arb_event_base(), arb_meter_pos())
            .prop_map(|(base, to)| ReplayEvent::BallMove { base, to }),
        // Card events
        (arb_event_base(), arb_card_type()).prop_map(|(base, card_type)| ReplayEvent::Card {
            base,
            card_type,
            yellow_count: None,
            from_second_yellow: None,
        }),
        (arb_event_base(), prop::option::of(0u32..22))
            .prop_map(|(base, in_player_id)| ReplayEvent::Substitution { base, in_player_id }),
        arb_event_base().prop_map(|base| ReplayEvent::HalfTime { base }),
        arb_event_base().prop_map(|base| ReplayEvent::FullTime { base }),
        (arb_event_base(), arb_meter_pos())
            .prop_map(|(base, at)| ReplayEvent::Offside { base, at }),
        (arb_event_base(), arb_meter_pos()).prop_map(|(base, at)| ReplayEvent::Save {
            base,
            at,
            parry_to: None,
            shot_from: None,
            shot_power: None,
            save_difficulty: None,
            shot_speed: None,
            reflexes_skill: None,
            handling_skill: None,
            diving_skill: None,
            positioning_quality: None,
        }),
        (arb_event_base(), arb_meter_pos(), arb_meter_pos())
            .prop_map(|(base, from, to)| ReplayEvent::Throw { base, from, to }),
        (arb_event_base(), arb_meter_pos(), any::<bool>())
            .prop_map(|(base, at, scored)| ReplayEvent::Penalty { base, at, scored }),
    ]
}

#[cfg(feature = "proptest")]
pub fn arb_replay_doc() -> impl Strategy<Value = ReplayDoc> {
    (
        1u32..10,
        (50.0f64..120.0, 40.0f64..80.0).prop_map(|(w, h)| PitchSpec { width_m: w, height_m: h }),
        prop::collection::vec(arb_replay_event(), 1..20),
    )
        .prop_map(|(version, pitch_m, events)| ReplayDoc {
            version,
            pitch_m,
            events,
            rosters: ReplayRosters::default(),
            timeline: Vec::new(),
            tactics: ReplayTeamsTactics::default(),
        })
}

// ============================================================================
// Fallback implementation without proptest
// ============================================================================

#[cfg(not(feature = "proptest"))]
pub fn generate_sample_replay() -> ReplayDoc {
    ReplayDoc {
        version: 1,
        pitch_m: PitchSpec { width_m: 105.0, height_m: 68.0 },
        events: vec![
            ReplayEvent::KickOff { base: EventBase { t: 0.0, player_id: None, team_id: Some(0) } },
            ReplayEvent::Goal {
                base: EventBase {
                    t: 1_200.0, // 20 minutes
                    player_id: Some(9),
                    team_id: Some(0),
                },
                at: MeterPos { x: 50.0, y: 34.0 },
                assist_player_id: None,
            },
        ],
        rosters: ReplayRosters::default(),
        timeline: Vec::new(),
        tactics: ReplayTeamsTactics::default(),
    }
}

#[cfg(all(test, not(feature = "proptest")))]
mod tests {
    use super::*;

    #[test]
    fn test_sample_replay_generation() {
        let replay = generate_sample_replay();
        assert_eq!(replay.version, 1);
        assert!(!replay.events.is_empty());
    }
}
