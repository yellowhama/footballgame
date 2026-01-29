//! mapper.rs
//! Open-Football EngineEvent → of_core::replay::ReplayEvent 변환기
//! PlayerAttributes → Skills36 매핑 기능 추가

use of_core::replay::{
    CardType, EventBase, FieldVector, GoalHeatSample, MeterPos, PitchSpec, ReplayDoc, ReplayEvent,
    ReplayPlayer, ReplayRoster, ReplayRosters, ReplayTimelineEntry,
};
use of_engine::{EngineEvent, Event, MatchResult, PositionData, PositionFrame, TeamSnapshot};
use std::cmp::Ordering;

#[cfg(feature = "vendor_skills")]
use of_engine::StoredEventKind;
#[cfg(feature = "vendor_skills")]
use std::collections::HashMap;

// Skills36 매핑을 위한 imports
use of_core::models::player::PlayerAttributes;
use of_core::player::types::CorePlayer;
#[cfg(feature = "vendor_skills")]
use of_engine::Skills36;

// Engine field dimensions in internal units (Open-Football)
const ENGINE_FIELD_WIDTH: f64 = 840.0;
const ENGINE_FIELD_HEIGHT: f64 = 545.0;
// Target pitch dimensions in meters (FIFA standard)
const PITCH_WIDTH_M: f64 = 105.0;
const PITCH_HEIGHT_M: f64 = 68.0;

#[cfg(feature = "vendor_skills")]
const RUN_SPEED_THRESHOLD: f64 = 4.8;
#[cfg(feature = "vendor_skills")]
const RUN_MIN_DISTANCE: f64 = 8.0;
#[cfg(feature = "vendor_skills")]
const RUN_MIN_DURATION_MS: u64 = 1_200;
#[cfg(feature = "vendor_skills")]
const DRIBBLE_SPEED_THRESHOLD: f64 = 3.2;
#[cfg(feature = "vendor_skills")]
const DRIBBLE_MIN_DISTANCE: f64 = 4.0;
#[cfg(feature = "vendor_skills")]
const DRIBBLE_MIN_DURATION_MS: u64 = 800;
#[cfg(feature = "vendor_skills")]
const BALL_CONTROL_RADIUS: f64 = 4.5;

pub fn build_replay_doc(engine_result: &MatchResult) -> (ReplayDoc, Vec<GoalHeatSample>) {
    let mut goal_heat_samples = Vec::new();
    let events = convert_engine_events_to_replay(engine_result, &mut goal_heat_samples);
    let timeline = build_timeline(engine_result);
    let rosters = ReplayRosters {
        home: snapshot_to_replay_roster(&engine_result.home_team),
        away: snapshot_to_replay_roster(&engine_result.away_team),
    };
    let tactics = build_replay_tactics(engine_result);

    (
        ReplayDoc {
            pitch_m: PitchSpec {
                width_m: PITCH_WIDTH_M,
                height_m: PITCH_HEIGHT_M,
            },
            events,
            version: 1,
            rosters,
            timeline,
            tactics,
        },
        goal_heat_samples,
    )
}

fn build_replay_tactics(engine_result: &MatchResult) -> of_core::replay::ReplayTeamsTactics {
    #[cfg(feature = "vendor_skills")]
    {
        use of_core::replay::{ReplayTeamTactics, ReplayTeamsTactics};

        let home = engine_result.home_tactics.as_ref().map(|t| ReplayTeamTactics {
            tactic_type: format!("{:?}", t.tactic_type),
            tactical_style: format!("{:?}", t.tactical_style()),
            formation_strength: t.formation_strength,
            selected_reason: format!("{:?}", t.selected_reason),
        });

        let away = engine_result.away_tactics.as_ref().map(|t| ReplayTeamTactics {
            tactic_type: format!("{:?}", t.tactic_type),
            tactical_style: format!("{:?}", t.tactical_style()),
            formation_strength: t.formation_strength,
            selected_reason: format!("{:?}", t.selected_reason),
        });

        ReplayTeamsTactics { home, away }
    }
    #[cfg(not(feature = "vendor_skills"))]
    {
        of_core::replay::ReplayTeamsTactics::default()
    }
}

/// Convert engine match result to replay events
pub fn convert_engine_events_to_replay(
    engine_result: &MatchResult,
    goal_heat_samples: &mut Vec<GoalHeatSample>,
) -> Vec<ReplayEvent> {
    let mut events = vec![];

    #[cfg(feature = "vendor_skills")]
    let player_team_lookup =
        build_player_team_lookup(&engine_result.home_team, &engine_result.away_team);

    // 시작 이벤트
    events.push(ReplayEvent::KickOff {
        base: EventBase {
            t: 0.0,
            player_id: None,
            team_id: Some(0),
        },
    });

    if !engine_result.engine_events.is_empty() {
        events.extend(convert_stored_events_to_replay(
            &engine_result.engine_events,
            engine_result.position_data.as_ref(),
            #[cfg(feature = "vendor_skills")]
            &player_team_lookup,
            goal_heat_samples,
        ));
    } else {
        for event in engine_result.events.iter() {
            if event.event_type == "KickOff" {
                continue;
            }
            let converted = convert_single_event(event);
            collect_heat_from_replay_events(converted.iter(), goal_heat_samples);
            events.extend(converted);
        }
    }

    if !events
        .iter()
        .any(|ev| matches!(ev, ReplayEvent::FullTime { .. }))
    {
        #[cfg(feature = "vendor_skills")]
        let fallback_time = if !engine_result.engine_events.is_empty() {
            engine_result
                .engine_events
                .last()
                .map(|e| e.timestamp as f64 / 1000.0)
                .unwrap_or(90.0 * 60.0)
        } else {
            engine_result
                .events
                .iter()
                .find(|e| e.event_type == "FullTime")
                .and_then(|e| e.timestamp_ms)
                .map(|ms| ms as f64 / 1000.0)
                .unwrap_or(90.0 * 60.0)
        };

        #[cfg(not(feature = "vendor_skills"))]
        let fallback_time = engine_result
            .events
            .iter()
            .find(|e| e.event_type == "FullTime")
            .and_then(|e| e.timestamp_ms)
            .map(|ms| ms as f64 / 1000.0)
            .unwrap_or(90.0 * 60.0);

        events.push(ReplayEvent::FullTime {
            base: EventBase {
                t: fallback_time,
                player_id: None,
                team_id: None,
            },
        });
    }

    events.sort_by(|a, b| {
        a.base()
            .t
            .partial_cmp(&b.base().t)
            .unwrap_or(Ordering::Equal)
    });

    events
}

fn convert_stored_events_to_replay(
    engine_events: &[EngineEvent],
    position_data: Option<&PositionData>,
    #[cfg(feature = "vendor_skills")] team_lookup: &HashMap<u32, u32>,
    goal_heat_samples: &mut Vec<GoalHeatSample>,
) -> Vec<ReplayEvent> {
    let mut replay_events = Vec::new();
    let mut pending_shots: HashMap<u32, usize> = HashMap::new();
    let mut pending_passes: HashMap<u32, usize> = HashMap::new();
    #[cfg(feature = "vendor_skills")]
    let mut movement_events_present = false;

    for stored in engine_events {
        let time_seconds = stored.timestamp as f64 / 1000.0;
        let fallback_mid = fallback_midfield_pos(stored.timestamp);
        let fallback_attack = fallback_attack_pos(stored.timestamp);

        match &stored.kind {
            StoredEventKind::Goal {
                player_id,
                team_id,
                assist_player_id,
                ..
            } => {
                let team = map_team_identifier(*team_id);
                let at = player_position_meter(position_data, *player_id, stored.timestamp)
                    .unwrap_or(fallback_attack);

                replay_events.push(ReplayEvent::Goal {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: team,
                    },
                    at,
                    assist_player_id: *assist_player_id,
                });

                let pending_entry = pending_shots.remove(player_id);
                if let Some(idx) = pending_entry {
                    if let Some(event) = replay_events.get_mut(idx) {
                        if let ReplayEvent::Shot { on_target, .. } = event {
                            *on_target = true;
                        }
                    }
                } else {
                    record_goal_heat_sample(goal_heat_samples, team, &at, 1.0, "goal");
                }
            }
            StoredEventKind::Assist {
                player_id, team_id, ..
            } => {
                let origin = player_position_meter(position_data, *player_id, stored.timestamp)
                    .unwrap_or(fallback_mid);
                replay_events.push(ReplayEvent::BallMove {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
                    to: origin,
                });
            }
            StoredEventKind::Pass {
                from_player_id,
                to_player_id,
                team_id,
                target,
                distance,
                force,
                ground,
                passing_skill,
                vision,
                technique,
                ..
            } => {
                let origin =
                    player_position_meter(position_data, *from_player_id, stored.timestamp)
                        .unwrap_or(fallback_mid);
                let target_pos = meter_pos_from_target(*target);
                let distance_m = distance_between(&origin, &target_pos);
                let base_event = EventBase {
                    t: time_seconds,
                    player_id: Some(*from_player_id),
                    team_id: map_team_identifier(*team_id),
                };
                replay_events.push(ReplayEvent::Pass {
                    base: base_event.clone(),
                    from: origin,
                    to: target_pos,
                    receiver_id: Some(*to_player_id),
                    distance_m: Some(distance_m),
                    force: Some(*force),
                    is_clearance: false,
                    ground: Some(*ground),
                    outcome: None,
                    passing_skill: *passing_skill,
                    vision: *vision,
                    technique: *technique,
                });
                #[cfg(feature = "vendor_skills")]
                if qualifies_through_ball(base_event.team_id, distance_m, &origin, &target_pos) {
                    replay_events.push(ReplayEvent::ThroughBall {
                        base: base_event,
                        from: origin,
                        to: target_pos,
                        receiver_id: Some(*to_player_id),
                        distance_m,
                        defense_splitting: None,
                        offside_risk: None,
                        pass_accuracy_required: None,
                        receiver_speed: None,
                        vision_quality: None,
                    });
                }
            }
            StoredEventKind::ThroughBall {
                from_player_id,
                to_player_id,
                team_id,
                from,
                target,
                distance,
                force,
                ..
            } => {
                let origin = meter_pos_from_target(*from);
                let target_pos = meter_pos_from_target(*target);
                let mapped_team = map_team_identifier(*team_id);
                replay_events.push(ReplayEvent::ThroughBall {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*from_player_id),
                        team_id: mapped_team,
                    },
                    from: origin,
                    to: target_pos,
                    receiver_id: Some(*to_player_id),
                    distance_m: *distance as f64,
                    defense_splitting: None,
                    offside_risk: None,
                    pass_accuracy_required: None,
                    receiver_speed: None,
                    vision_quality: None,
                });
                #[cfg(feature = "vendor_skills")]
                {
                    record_goal_heat_sample(
                        goal_heat_samples,
                        mapped_team,
                        &target_pos,
                        (*distance as f64).max(0.0) * 0.01,
                        "through_ball",
                    );
                }
            }
            StoredEventKind::Shot {
                player_id,
                team_id,
                target,
                force,
                xg,
                long_shots_skill,
                finishing_skill,
                technique,
                ..
            } => {
                let origin = player_position_meter(position_data, *player_id, stored.timestamp)
                    .unwrap_or(fallback_attack);
                let idx = replay_events.len();
                let target_pos = meter_pos_from_target(*target);
                let mapped_team = map_team_identifier(*team_id);
                let stored_xg = if *xg > 0.0 { Some(*xg as f64) } else { None };
                let xg_estimate = estimate_shot_xg(&origin, &target_pos, mapped_team);
                let xg_value = stored_xg.unwrap_or(xg_estimate);
                replay_events.push(ReplayEvent::Shot {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: mapped_team,
                    },
                    from: origin,
                    target: target_pos,
                    on_target: false,
                    xg: Some(xg_value),
                    shot_speed: Some(*force),
                    long_shots_skill: *long_shots_skill,
                    finishing_skill: *finishing_skill,
                    technique: *technique,
                    shot_type: None,
                    defender_pressure: None,
                    angle_to_goal: None,
                    distance_to_goal: None,
                    composure: None,
                    curve_factor: None,
                });
                pending_shots.insert(*player_id, idx);
                record_goal_heat_sample(
                    goal_heat_samples,
                    mapped_team,
                    &target_pos,
                    xg_estimate,
                    "shot",
                );
            }
            StoredEventKind::Save { player_id, team_id, .. } => {
                let at = player_position_meter(position_data, *player_id, stored.timestamp)
                    .unwrap_or(fallback_attack);
                replay_events.push(ReplayEvent::Save {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
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
                });
            }
            StoredEventKind::Foul { player_id, team_id, .. } => {
                let at = player_position_meter(position_data, *player_id, stored.timestamp)
                    .unwrap_or(fallback_mid);
                replay_events.push(ReplayEvent::Foul {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
                    at,
                    foul_type: None,
                    severity: None,
                    intentional: None,
                    location_danger: None,
                    aggression_level: None,
                });
            }
            StoredEventKind::Tackle { player_id, team_id, .. } => {
                let to = player_position_meter(position_data, *player_id, stored.timestamp)
                    .unwrap_or(fallback_mid);
                replay_events.push(ReplayEvent::BallMove {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
                    to,
                });
            }
            StoredEventKind::Possession {
                player_id, team_id, ..
            } => {
                let to = player_position_meter(position_data, *player_id, stored.timestamp)
                    .unwrap_or(fallback_mid);
                let base = EventBase {
                    t: time_seconds,
                    player_id: Some(*player_id),
                    team_id: map_team_identifier(*team_id),
                };
                replay_events.push(ReplayEvent::BallMove {
                    base: EventBase {
                        t: base.t,
                        player_id: base.player_id,
                        team_id: base.team_id,
                    },
                    to,
                });

                if let Some(idx) = pending_passes.remove(player_id) {
                    let pass_team = replay_events
                        .get(idx)
                        .map(|ev| ev.base().team_id)
                        .unwrap_or(None);
                    let current_team = map_team_identifier(*team_id);
                    if let Some(ReplayEvent::Pass { outcome, .. }) =
                        replay_events.get_mut(idx)
                    {
                        *outcome = match (pass_team, current_team) {
                            (Some(pt), Some(ct)) if pt == ct => {
                                Some(of_core::replay::PassOutcome::Complete)
                            }
                            (Some(_), Some(_)) => {
                                Some(of_core::replay::PassOutcome::Intercepted)
                            }
                            _ => outcome.clone(),
                        };
                    }
                }
            }
            StoredEventKind::Clearance {
                player_id,
                team_id,
                target,
                ..
            } => {
                let origin = player_position_meter(position_data, *player_id, stored.timestamp)
                    .unwrap_or(fallback_mid);
                replay_events.push(ReplayEvent::Pass {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
                    from: origin,
                    to: meter_pos_from_target(*target),
                    receiver_id: None,
                    distance_m: None,
                    force: None,
                    is_clearance: true,
                    ground: None,
                    outcome: None,
                    passing_skill: None,
                    vision: None,
                    technique: None,
                });
            }
            StoredEventKind::YellowCard {
                player_id,
                team_id,
                yellow_count,
                ..
            } => {
                replay_events.push(ReplayEvent::Card {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
                    card_type: CardType::Yellow,
                    yellow_count: Some(*yellow_count),
                    from_second_yellow: None,
                });
            }
            StoredEventKind::RedCard {
                player_id,
                team_id,
                from_second_yellow,
            } => {
                replay_events.push(ReplayEvent::Card {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
                    card_type: CardType::Red,
                    yellow_count: None,
                    from_second_yellow: Some(*from_second_yellow),
                });
            }
            StoredEventKind::Offside { .. } => {
                // Offside doesn't yet have a dedicated UI representation in this replay
            }
            StoredEventKind::Run {
                player_id,
                team_id,
                from,
                to,
                distance,
                speed,
                with_ball,
                pace_skill,
                stamina,
                condition,
                ..
            } => {
                replay_events.push(ReplayEvent::Run {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
                    from: meter_pos_from_target(*from),
                    to: meter_pos_from_target(*to),
                    distance_m: *distance as f64,
                    speed_mps: Some(*speed as f64),
                    with_ball: *with_ball,
                    pace_skill: *pace_skill,
                    stamina: *stamina,
                    condition: *condition,
                    run_purpose: None,
                    sprint_intensity: None,
                    tactical_value: None,
                    off_the_ball: None,
                    work_rate: None,
                });
                #[cfg(feature = "vendor_skills")]
                {
                    movement_events_present = true;
                }
            }
            StoredEventKind::Dribble {
                player_id,
                team_id,
                from,
                to,
                distance,
                touches,
                ..
            } => {
                replay_events.push(ReplayEvent::Dribble {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
                    from: meter_pos_from_target(*from),
                    to: meter_pos_from_target(*to),
                    distance_m: *distance as f64,
                    touches: Some(*touches),
                    success: None,
                    opponents_evaded: None,
                    space_gained: None,
                    pressure_level: None,
                    dribbling_skill: None,
                    agility: None,
                    balance: None,
                    close_control: None,
                });
                #[cfg(feature = "vendor_skills")]
                {
                    movement_events_present = true;
                }
            }
            StoredEventKind::Communication {
                player_id,
                team_id,
                message,
                target,
                ..
            } => {
                let at = player_position_meter(position_data, *player_id, stored.timestamp)
                    .unwrap_or(fallback_mid);
                replay_events.push(ReplayEvent::Communication {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
                    at,
                    message: message.clone(),
                    target: target.as_ref().map(|vec| meter_pos_from_target(*vec)),
                    comm_type: None,
                    urgency: None,
                    response_required: None,
                    effective: None,
                });
            }
            StoredEventKind::Header {
                player_id,
                team_id,
                from,
                direction,
                heading_skill,
                jumping_reach,
                height,
                ..
            } => {
                let from = meter_pos_from_target(*from);
                replay_events.push(ReplayEvent::Header {
                    base: EventBase {
                        t: time_seconds,
                        player_id: Some(*player_id),
                        team_id: map_team_identifier(*team_id),
                    },
                    from,
                    direction: Some(direction_to_field_vector(*direction)),
                    heading_skill: *heading_skill,
                    jumping_reach: *jumping_reach,
                    height: *height,
                    win_chance: None,
                    opponent_distance: None,
                    aerial_challenge: None,
                    aerial_strength: None,
                });
            }
            StoredEventKind::Boundary {
                position,
                last_touch_player_id,
                last_touch_team_id,
            } => {
                let mapped_team = last_touch_team_id.and_then(|team| map_team_identifier(team));
                replay_events.push(ReplayEvent::Boundary {
                    base: EventBase {
                        t: time_seconds,
                        player_id: *last_touch_player_id,
                        team_id: mapped_team,
                    },
                    position: meter_pos_from_boundary(*position),
                    last_touch_player_id: *last_touch_player_id,
                    last_touch_team_id: mapped_team,
                });

                if let Some(player_id) = last_touch_player_id {
                    if let Some(idx) = pending_passes.remove(player_id) {
                        if let Some(ReplayEvent::Pass { outcome, .. }) =
                            replay_events.get_mut(idx)
                        {
                            if outcome.is_none() {
                                *outcome = Some(of_core::replay::PassOutcome::Out);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "vendor_skills")]
    if !movement_events_present {
        if let Some(data) = position_data {
            replay_events.extend(extract_movement_events(data, team_lookup));
        }
    } else if let Some(data) = position_data {
        // We still derive supplemental heat-map samples even if native events exist.
        replay_events.extend(extract_movement_events(data, team_lookup));
    }

    replay_events
}

fn map_team_identifier(team_id: u32) -> Option<u32> {
    match team_id {
        1 => Some(0),
        2 => Some(1),
        _ => None,
    }
}

/// Convert single engine event to replay event
fn convert_single_event(event: &Event) -> Vec<ReplayEvent> {
    let metadata = parse_event_metadata(event);
    let time_seconds = event
        .timestamp_ms
        .map(|ms| ms as f64 / 1000.0)
        .unwrap_or((event.minute as f64) * 60.0);
    let minute_scalar = event
        .timestamp_ms
        .map(|ms| ms as f64 / 60_000.0)
        .unwrap_or(event.minute as f64);

    let base_x = 30.0 + (minute_scalar * 0.5) % 60.0;
    let base_y = 20.0 + (minute_scalar * 0.3) % 28.0;
    let attack_x = 75.0 + (minute_scalar * 0.2) % 25.0;
    let attack_y = 24.0 + (minute_scalar * 0.4) % 20.0;

    let team_id = metadata.team_id;
    let player_id = metadata
        .player_id
        .or_else(|| extract_numeric_id(&metadata.player_name))
        .unwrap_or_else(|| {
            get_pseudo_random_player_id(time_seconds, &metadata.raw_type_lower, metadata.team_id)
        });

    let base_event = EventBase {
        t: time_seconds,
        player_id: Some(player_id),
        team_id,
    };
    let midfield_pos = MeterPos {
        x: base_x,
        y: base_y,
    };
    let attack_pos = MeterPos {
        x: attack_x,
        y: attack_y,
    };
    let pass_target = MeterPos {
        x: base_x + 15.0,
        y: base_y + 5.0,
    };
    let dribble_target = MeterPos {
        x: base_x + 10.0,
        y: base_y + 3.0,
    };

    match &metadata.category {
        EventCategory::Goal => {
            let shot_target = goal_center_for_team(team_id);
            vec![
                ReplayEvent::Shot {
                    base: base_event.clone(),
                    from: attack_pos,
                    target: shot_target,
                    on_target: true,
                    xg: Some(estimate_shot_xg(&attack_pos, &shot_target, team_id)),
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
                },
                ReplayEvent::Goal {
                    base: base_event,
                    at: shot_target,
                    assist_player_id: None,
                },
            ]
        }
        EventCategory::Assist => vec![ReplayEvent::Pass {
            base: base_event,
            from: midfield_pos,
            to: pass_target,
            receiver_id: None,
            distance_m: None,
            force: None,
            is_clearance: false,
            ground: None,
            outcome: None,
            passing_skill: None,
            vision: None,
            technique: None,
        }],
        EventCategory::Generic => {
            let lower = metadata.raw_type_lower.as_str();

            if lower.starts_with("shot|") {
                let mut events = Vec::new();
                let on_target =
                    lower.contains("target") || lower.contains("saved") || lower.contains("goal");
                let shot_target = goal_center_for_team(team_id);
                events.push(ReplayEvent::Shot {
                    base: base_event.clone(),
                    from: attack_pos,
                    target: shot_target,
                    on_target,
                    xg: Some(estimate_shot_xg(&attack_pos, &shot_target, team_id)),
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
                });
                if lower.contains("goal") {
                    events.push(ReplayEvent::Goal {
                        base: base_event,
                        at: shot_target,
                        assist_player_id: None,
                    });
                }
                return events;
            }

            if lower.contains("goal") {
                let mut events = Vec::new();
                let shot_target = goal_center_for_team(team_id);
                if lower.contains("shot") {
                    events.push(ReplayEvent::Shot {
                        base: base_event.clone(),
                        from: attack_pos,
                        target: shot_target,
                        on_target: true,
                        xg: Some(estimate_shot_xg(&attack_pos, &shot_target, team_id)),
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
                    });
                }
                events.push(ReplayEvent::Goal {
                    base: base_event,
                    at: shot_target,
                    assist_player_id: None,
                });
                return events;
            }

            if lower.contains("shot") {
                let shot_target = goal_center_for_team(team_id);
                return vec![ReplayEvent::Shot {
                    base: base_event,
                    from: attack_pos,
                    target: shot_target,
                    on_target: lower.contains("target") || lower.contains("saved"),
                    xg: Some(estimate_shot_xg(&attack_pos, &shot_target, team_id)),
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
                }];
            }

            if lower.contains("pass") || lower.contains("dribble") {
                let destination = if lower.contains("dribble") {
                    dribble_target
                } else {
                    pass_target
                };
                return vec![ReplayEvent::Pass {
                    base: base_event,
                    from: midfield_pos,
                    to: destination,
                    receiver_id: None,
                    distance_m: None,
                    force: None,
                    is_clearance: false,
                    ground: None,
                    outcome: None,
                    passing_skill: None,
                    vision: None,
                    technique: None,
                }];
            }

            if lower.contains("tackle") || lower.contains("foul") || lower.contains("card") {
                return vec![ReplayEvent::BallMove {
                    base: base_event,
                    to: midfield_pos,
                }];
            }

            if lower.contains("corner") || lower.contains("freekick") || lower.contains("penalty") {
                let setpiece_x = if lower.contains("corner") {
                    match team_id {
                        Some(0) => 100.0,
                        Some(_) => 5.0,
                        None => base_x,
                    }
                } else if lower.contains("penalty") {
                    match team_id {
                        Some(0) => 94.0,
                        Some(_) => 11.0,
                        None => base_x,
                    }
                } else {
                    base_x
                };
                return vec![ReplayEvent::Shot {
                    base: base_event,
                    from: MeterPos {
                        x: setpiece_x,
                        y: 34.0,
                    },
                    target: goal_center_for_team(team_id),
                    on_target: false,
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
                }];
            }

            if lower.contains("halftime") {
                return vec![ReplayEvent::HalfTime {
                    base: EventBase {
                        t: base_event.t,
                        player_id: None,
                        team_id: None,
                    },
                }];
            }

            if lower.contains("fulltime") {
                return vec![ReplayEvent::FullTime {
                    base: EventBase {
                        t: base_event.t,
                        player_id: None,
                        team_id: None,
                    },
                }];
            }

            vec![ReplayEvent::BallMove {
                base: base_event,
                to: midfield_pos,
            }]
        }
    }
}

#[cfg(feature = "vendor_skills")]
fn build_player_team_lookup(home: &TeamSnapshot, away: &TeamSnapshot) -> HashMap<u32, u32> {
    let mut lookup = HashMap::new();
    for player in &home.players {
        lookup.insert(player.id, 1);
    }
    for player in &away.players {
        lookup.insert(player.id, 2);
    }
    lookup
}

#[cfg(feature = "vendor_skills")]
struct FrameMetrics {
    timestamp: u64,
    pos: MeterPos,
    has_ball: bool,
}

#[cfg(feature = "vendor_skills")]
struct SegmentInfo {
    start_idx: usize,
    end_idx: usize,
    distance: f64,
    duration_ms: u64,
    ball_ratio: f64,
}

#[cfg(feature = "vendor_skills")]
struct SegmentAccumulator {
    start_idx: usize,
    end_idx: usize,
    distance: f64,
    duration_ms: u64,
    samples: u32,
    ball_samples: u32,
}

#[cfg(feature = "vendor_skills")]
fn extract_movement_events(
    position_data: &PositionData,
    team_lookup: &HashMap<u32, u32>,
) -> Vec<ReplayEvent> {
    let mut events = Vec::new();
    let ball_frames = if position_data.ball.is_empty() {
        None
    } else {
        Some(position_data.ball.as_slice())
    };

    for (player_id_str, frames) in position_data.players.iter() {
        if frames.len() < 2 {
            continue;
        }
        let Ok(player_id) = player_id_str.parse::<u32>() else {
            continue;
        };
        let Some(raw_team_id) = team_lookup.get(&player_id) else {
            continue;
        };
        let mapped_team = map_team_identifier(*raw_team_id);
        let frame_metrics = build_frame_metrics(frames, ball_frames);
        if frame_metrics.len() < 2 {
            continue;
        }

        let run_segments = detect_movement_segments(
            &frame_metrics,
            RUN_SPEED_THRESHOLD,
            RUN_MIN_DISTANCE,
            RUN_MIN_DURATION_MS,
            false,
        );
        for segment in run_segments {
            let start = &frame_metrics[segment.start_idx];
            let end = &frame_metrics[segment.end_idx];
            let speed = if segment.duration_ms > 0 {
                Some(segment.distance / (segment.duration_ms as f64 / 1000.0))
            } else {
                None
            };
            events.push(ReplayEvent::Run {
                base: EventBase {
                    t: start.timestamp as f64 / 1000.0,
                    player_id: Some(player_id),
                    team_id: mapped_team,
                },
                from: start.pos,
                to: end.pos,
                distance_m: segment.distance,
                speed_mps: speed,
                with_ball: segment.ball_ratio >= 0.5,
                pace_skill: None,
                stamina: None,
                condition: None,
                run_purpose: None,
                sprint_intensity: None,
                tactical_value: None,
                off_the_ball: None,
                work_rate: None,
            });
        }

        let dribble_segments = detect_movement_segments(
            &frame_metrics,
            DRIBBLE_SPEED_THRESHOLD,
            DRIBBLE_MIN_DISTANCE,
            DRIBBLE_MIN_DURATION_MS,
            true,
        );
        for segment in dribble_segments {
            let start = &frame_metrics[segment.start_idx];
            let end = &frame_metrics[segment.end_idx];
            events.push(ReplayEvent::Dribble {
                base: EventBase {
                    t: start.timestamp as f64 / 1000.0,
                    player_id: Some(player_id),
                    team_id: mapped_team,
                },
                from: start.pos,
                to: end.pos,
                distance_m: segment.distance,
                touches: None,
                success: None,
                opponents_evaded: None,
                space_gained: None,
                pressure_level: None,
                dribbling_skill: None,
                agility: None,
                balance: None,
                close_control: None,
            });
        }
    }

    events
}

#[cfg(feature = "vendor_skills")]
fn build_frame_metrics(
    frames: &[PositionFrame],
    ball_frames: Option<&[PositionFrame]>,
) -> Vec<FrameMetrics> {
    frames
        .iter()
        .map(|frame| {
            let has_ball = ball_frames
                .and_then(|ball| find_frame_at_timestamp(ball, frame.timestamp))
                .map(|ball_frame| {
                    let player_pos = meter_pos_from_frame(frame);
                    let ball_pos = meter_pos_from_frame(ball_frame);
                    distance_between(&player_pos, &ball_pos) <= BALL_CONTROL_RADIUS
                })
                .unwrap_or(false);
            FrameMetrics {
                timestamp: frame.timestamp,
                pos: meter_pos_from_frame(frame),
                has_ball,
            }
        })
        .collect()
}

#[cfg(feature = "vendor_skills")]
fn detect_movement_segments(
    frames: &[FrameMetrics],
    min_speed: f64,
    min_distance: f64,
    min_duration_ms: u64,
    require_ball: bool,
) -> Vec<SegmentInfo> {
    let mut segments = Vec::new();
    let mut current: Option<SegmentAccumulator> = None;

    for idx in 1..frames.len() {
        let prev = &frames[idx - 1];
        let curr = &frames[idx];
        let dt = curr.timestamp.saturating_sub(prev.timestamp);
        if dt == 0 {
            continue;
        }
        let step_distance = distance_between(&prev.pos, &curr.pos);
        if step_distance == 0.0 {
            continue;
        }
        let speed = step_distance / (dt as f64 / 1000.0);
        let ball_contact = prev.has_ball || curr.has_ball;
        let condition = speed >= min_speed && (!require_ball || ball_contact);

        if condition {
            if let Some(acc) = current.as_mut() {
                acc.end_idx = idx;
                acc.distance += step_distance;
                acc.duration_ms += dt;
                acc.samples += 1;
                if ball_contact {
                    acc.ball_samples += 1;
                }
            } else {
                current = Some(SegmentAccumulator {
                    start_idx: idx - 1,
                    end_idx: idx,
                    distance: step_distance,
                    duration_ms: dt,
                    samples: 1,
                    ball_samples: if ball_contact { 1 } else { 0 },
                });
            }
        } else if let Some(acc) = current.take() {
            push_segment_if_valid(acc, min_distance, min_duration_ms, &mut segments);
        }
    }

    if let Some(acc) = current {
        push_segment_if_valid(acc, min_distance, min_duration_ms, &mut segments);
    }

    segments
}

#[cfg(feature = "vendor_skills")]
fn push_segment_if_valid(
    acc: SegmentAccumulator,
    min_distance: f64,
    min_duration_ms: u64,
    segments: &mut Vec<SegmentInfo>,
) {
    if acc.distance >= min_distance && acc.duration_ms >= min_duration_ms {
        let ratio = if acc.samples == 0 {
            0.0
        } else {
            acc.ball_samples as f64 / acc.samples as f64
        };
        segments.push(SegmentInfo {
            start_idx: acc.start_idx,
            end_idx: acc.end_idx,
            distance: acc.distance,
            duration_ms: acc.duration_ms,
            ball_ratio: ratio,
        });
    }
}

fn defending_side_from_team(team: Option<u32>) -> u32 {
    match team {
        Some(0) => 1,
        Some(1) => 0,
        Some(other) => {
            if other == 0 {
                1
            } else {
                0
            }
        }
        None => 1,
    }
}

fn record_goal_heat_sample(
    samples: &mut Vec<GoalHeatSample>,
    shooting_team: Option<u32>,
    target: &MeterPos,
    weight: f64,
    kind: &str,
) {
    let defending_side = defending_side_from_team(shooting_team);
    samples.push(GoalHeatSample {
        team_side: defending_side,
        x: target.x.clamp(0.0, PITCH_WIDTH_M),
        y: target.y.clamp(0.0, PITCH_HEIGHT_M),
        weight: weight.max(0.05),
        kind: Some(kind.to_string()),
    });
}

fn collect_heat_from_replay_events<'a>(
    events: impl Iterator<Item = &'a ReplayEvent>,
    samples: &mut Vec<GoalHeatSample>,
) {
    let mut shot_recorded = false;
    let mut fallback_goal: Option<(&EventBase, &MeterPos)> = None;
    for event in events {
        match event {
            ReplayEvent::Shot {
                base, target, xg, ..
            } => {
                shot_recorded = true;
                record_goal_heat_sample(samples, base.team_id, target, xg.unwrap_or(0.4), "shot");
            }
            ReplayEvent::Goal { base, at, .. } => {
                fallback_goal = Some((base, at));
            }
            _ => {}
        }
    }

    if !shot_recorded {
        if let Some((base, at)) = fallback_goal {
            record_goal_heat_sample(samples, base.team_id, at, 1.0, "goal");
        }
    }
}

fn distance_between(a: &MeterPos, b: &MeterPos) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(feature = "vendor_skills")]
fn qualifies_through_ball(
    team_id: Option<u32>,
    distance_m: f64,
    from: &MeterPos,
    to: &MeterPos,
) -> bool {
    if distance_m < 15.0 {
        return false;
    }
    match team_id {
        Some(0) => to.x > from.x + 5.0,
        Some(1) => to.x < from.x - 5.0,
        _ => (to.x - from.x).abs() > 5.0,
    }
}

fn meter_pos_from_target(target: [f32; 3]) -> MeterPos {
    let x_units = target[0].clamp(0.0, ENGINE_FIELD_WIDTH as f32) as f64;
    let y_units = target[1].clamp(0.0, ENGINE_FIELD_HEIGHT as f32) as f64;
    MeterPos {
        x: x_units * (PITCH_WIDTH_M / ENGINE_FIELD_WIDTH),
        y: y_units * (PITCH_HEIGHT_M / ENGINE_FIELD_HEIGHT),
    }
}

fn meter_pos_from_boundary(position: [f32; 2]) -> MeterPos {
    let x_units = position[0].clamp(0.0, ENGINE_FIELD_WIDTH as f32) as f64;
    let y_units = position[1].clamp(0.0, ENGINE_FIELD_HEIGHT as f32) as f64;
    MeterPos {
        x: x_units * (PITCH_WIDTH_M / ENGINE_FIELD_WIDTH),
        y: y_units * (PITCH_HEIGHT_M / ENGINE_FIELD_HEIGHT),
    }
}

fn direction_to_field_vector(direction: [f32; 3]) -> FieldVector {
    FieldVector {
        x: direction[0] as f64,
        y: direction[2] as f64,
    }
}

fn goal_center_for_team(team_id: Option<u32>) -> MeterPos {
    match team_id {
        Some(0) => MeterPos { x: 105.0, y: 34.0 },
        Some(1) => MeterPos { x: 0.0, y: 34.0 },
        _ => MeterPos { x: 52.5, y: 34.0 },
    }
}

fn estimate_shot_xg(from: &MeterPos, target: &MeterPos, team_id: Option<u32>) -> f64 {
    let goal_center = goal_center_for_team(team_id);
    let distance = ((from.x - goal_center.x).powi(2) + (from.y - goal_center.y).powi(2)).sqrt();
    let angle_component = ((from.y - goal_center.y).abs() / 68.0).clamp(0.0, 1.0); // 68m = standard pitch width
    let raw = 1.05 - (distance / 45.0) - angle_component * 0.35;
    raw.clamp(0.02, 0.95)
}

fn is_cross(from: &MeterPos, to: &MeterPos, team_id: Option<u32>, ground: Option<bool>) -> bool {
    const FIELD_WIDTH: f64 = 68.0;
    const WING_THRESHOLD: f64 = 0.3; // 필드 폭의 30%

    let center_y = FIELD_WIDTH / 2.0;
    let is_from_wing = (from.y - center_y).abs() > FIELD_WIDTH * WING_THRESHOLD;
    let is_to_box = is_in_opponent_penalty_box(to, team_id);

    let is_air = matches!(ground, Some(false));
    is_from_wing && is_to_box && is_air
}

fn is_in_opponent_penalty_box(pos: &MeterPos, team_id: Option<u32>) -> bool {
    const FIELD_LENGTH: f64 = 105.0;
    const BOX_WIDTH: f64 = 40.32;
    const BOX_LENGTH: f64 = 16.5;
    const FIELD_WIDTH: f64 = 68.0;

    let (min_x, max_x) = match team_id {
        Some(0) => (FIELD_LENGTH - BOX_LENGTH, FIELD_LENGTH), // 오른쪽 공격
        Some(1) => (0.0, BOX_LENGTH),                         // 왼쪽 공격
        _ => return false,
    };

    let center_y = FIELD_WIDTH / 2.0;
    let min_y = center_y - BOX_WIDTH / 2.0;
    let max_y = center_y + BOX_WIDTH / 2.0;

    pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y
}

fn classify_curve(from: &MeterPos, to: &MeterPos, team_id: Option<u32>) -> of_core::models::replay::types::CurveType {
    use of_core::models::replay::types::CurveType;

    let goal_center = goal_center_for_team(team_id);

    let v_pass = normalize_2d(&MeterPos { x: to.x - from.x, y: to.y - from.y });
    let v_goal = normalize_2d(&MeterPos { x: goal_center.x - from.x, y: goal_center.y - from.y });

    let angle_sign = v_goal.x * v_pass.y - v_goal.y * v_pass.x;
    let dot = v_goal.x * v_pass.x + v_goal.y * v_pass.y;

    if dot.abs() < 1e-6 {
        return CurveType::None;
    }

    let angle_mag = dot.acos();
    const MIN_CURVE_ANGLE: f64 = 0.1; // ~5.7도

    if angle_mag < MIN_CURVE_ANGLE {
        return CurveType::None;
    }

    match team_id {
        Some(0) => {
            if angle_sign > 0.0 { CurveType::Inside } else { CurveType::Outside }
        }
        Some(1) => {
            if angle_sign < 0.0 { CurveType::Inside } else { CurveType::Outside }
        }
        _ => CurveType::None,
    }
}

fn normalize_2d(v: &MeterPos) -> MeterPos {
    let len = (v.x * v.x + v.y * v.y).sqrt();
    if len < 1e-6 {
        MeterPos { x: 0.0, y: 0.0 }
    } else {
        MeterPos { x: v.x / len, y: v.y / len }
    }
}

#[cfg(feature = "vendor_skills")]
fn player_position_meter(
    position_data: Option<&PositionData>,
    player_id: u32,
    timestamp: u64,
) -> Option<MeterPos> {
    let data = position_data?;
    let frames = data.players.get(&player_id.to_string())?;
    let frame = find_frame_at_timestamp(frames, timestamp)?;
    Some(meter_pos_from_frame(frame))
}

#[cfg(feature = "vendor_skills")]
fn find_frame_at_timestamp<'a>(
    frames: &'a [PositionFrame],
    timestamp: u64,
) -> Option<&'a PositionFrame> {
    if frames.is_empty() {
        return None;
    }

    match frames.binary_search_by_key(&timestamp, |f| f.timestamp) {
        Ok(idx) => frames.get(idx),
        Err(0) => frames.get(0),
        Err(idx) => frames.get(idx.saturating_sub(1)).or_else(|| frames.last()),
    }
}

#[cfg(feature = "vendor_skills")]
fn meter_pos_from_frame(frame: &PositionFrame) -> MeterPos {
    MeterPos {
        x: (frame.position[0].clamp(0.0, ENGINE_FIELD_WIDTH as f32) as f64)
            * (PITCH_WIDTH_M / ENGINE_FIELD_WIDTH),
        y: (frame.position[1].clamp(0.0, ENGINE_FIELD_HEIGHT as f32) as f64)
            * (PITCH_HEIGHT_M / ENGINE_FIELD_HEIGHT),
    }
}

#[cfg(feature = "vendor_skills")]
fn fallback_midfield_pos(timestamp: u64) -> MeterPos {
    let minute_scalar = timestamp as f64 / 60_000.0;
    let x = 30.0 + (minute_scalar * 0.5) % 60.0;
    let y = 20.0 + (minute_scalar * 0.3) % 28.0;
    MeterPos { x, y }
}

#[cfg(feature = "vendor_skills")]
fn fallback_attack_pos(timestamp: u64) -> MeterPos {
    let minute_scalar = timestamp as f64 / 60_000.0;
    let x = 75.0 + (minute_scalar * 0.2) % 25.0;
    let y = 24.0 + (minute_scalar * 0.4) % 20.0;
    MeterPos { x, y }
}

fn build_timeline(engine_result: &MatchResult) -> Vec<ReplayTimelineEntry> {
    engine_result
        .events
        .iter()
        .map(|event| ReplayTimelineEntry {
            t: event
                .timestamp_ms
                .map(|ms| ms as f64 / 1000.0)
                .unwrap_or((event.minute as f64) * 60.0),
            label: event.event_type.clone(),
            team_id: extract_team_id_from_label(&event.event_type),
            player_id: None,
        })
        .collect()
}

fn snapshot_to_replay_roster(snapshot: &TeamSnapshot) -> ReplayRoster {
    let players = snapshot
        .players
        .iter()
        .map(|player| ReplayPlayer {
            id: player.id,
            name: player.name.clone(),
            position: player.position.clone(),
            ca: player.ca as u32,
            condition: player.condition,
        })
        .collect();

    ReplayRoster {
        name: snapshot.name.clone(),
        players,
    }
}

fn extract_team_id_from_label(label: &str) -> Option<u32> {
    let lower = label.to_lowercase();
    if lower.contains("home") {
        Some(0)
    } else if lower.contains("away") {
        Some(1)
    } else {
        None
    }
}

#[derive(Debug)]
enum EventCategory {
    Goal,
    Assist,
    Generic,
}

#[derive(Debug)]
struct EventMetadata {
    category: EventCategory,
    team_id: Option<u32>,
    player_id: Option<u32>,
    player_name: String,
    raw_type_lower: String,
}

/// Generate a pseudo-random player_id based on event time and raw_type
/// This ensures consistent but varied player assignments when metadata doesn't provide player_id
fn get_pseudo_random_player_id(time_seconds: f64, raw_type: &str, team_id: Option<u32>) -> u32 {
    // Use event time, type, and team as seed for pseudo-randomness
    let time_hash = (time_seconds * 1000.0) as u64;
    let type_hash = raw_type
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let team_hash = team_id.unwrap_or(0) as u64;

    let seed = time_hash.wrapping_add(type_hash).wrapping_add(team_hash);

    // Assign player_id based on event type for more realistic distribution
    if raw_type.contains("goal") || raw_type.contains("shot") {
        // Forwards and attacking midfielders (7-10)
        ((seed % 4) + 7) as u32
    } else if raw_type.contains("pass") || raw_type.contains("assist") {
        // Midfielders (4-7)
        ((seed % 4) + 4) as u32
    } else if raw_type.contains("tackle") || raw_type.contains("foul") {
        // Defenders (2-5)
        ((seed % 4) + 2) as u32
    } else if raw_type.contains("save") {
        // Goalkeepers (0-1)
        (seed % 2) as u32
    } else {
        // Any player (0-10)
        (seed % 11) as u32
    }
}

fn parse_event_metadata(event: &Event) -> EventMetadata {
    let raw_type_lower = event.event_type.to_lowercase();

    if let Some(payload) = event.event_type.strip_prefix("Goal|") {
        let parts: Vec<&str> = payload.split('|').collect();
        let team_id = parts.get(0).and_then(|label| label_to_team_id(*label));
        let player_name = parts.get(1).unwrap_or(&"Player").to_string();
        let player_id = parts.get(2).and_then(|id| id.parse::<u32>().ok());
        return EventMetadata {
            category: EventCategory::Goal,
            team_id,
            player_id,
            player_name,
            raw_type_lower,
        };
    }

    if let Some(payload) = event.event_type.strip_prefix("Assist|") {
        let parts: Vec<&str> = payload.split('|').collect();
        let team_id = parts.get(0).and_then(|label| label_to_team_id(*label));
        let player_name = parts.get(1).unwrap_or(&"Player").to_string();
        let player_id = parts.get(2).and_then(|id| id.parse::<u32>().ok());

        return EventMetadata {
            category: EventCategory::Assist,
            team_id,
            player_id,
            player_name,
            raw_type_lower,
        };
    }

    let tokens: Vec<&str> = event.event_type.split('|').collect();
    if tokens.len() >= 2 {
        let team_id = tokens.get(1).and_then(|label| label_to_team_id(*label));
        if let Some(team_id) = team_id {
            let (player_id, player_name) = match tokens[0] {
                "Pass" => (
                    tokens.get(3).and_then(|token| token.parse::<u32>().ok()),
                    tokens
                        .get(5)
                        .or_else(|| tokens.get(4))
                        .unwrap_or(&"Player")
                        .to_string(),
                ),
                "Shot" => (
                    tokens.get(3).and_then(|token| token.parse::<u32>().ok()),
                    tokens.get(4).unwrap_or(&"Player").to_string(),
                ),
                "Dribble" | "Tackle" | "Interception" | "Recovery" | "Corner" => (
                    tokens.get(2).and_then(|token| token.parse::<u32>().ok()),
                    tokens.get(3).unwrap_or(&"Player").to_string(),
                ),
                "FreeKick" => (
                    tokens.get(2).and_then(|token| token.parse::<u32>().ok()),
                    tokens.get(3).unwrap_or(&"Player").to_string(),
                ),
                "Foul" => (
                    tokens.get(3).and_then(|token| token.parse::<u32>().ok()),
                    tokens.get(4).unwrap_or(&"Player").to_string(),
                ),
                _ => (
                    tokens.get(2).and_then(|token| token.parse::<u32>().ok()),
                    tokens.get(3).unwrap_or(&"Player").to_string(),
                ),
            };

            return EventMetadata {
                category: EventCategory::Generic,
                team_id: Some(team_id),
                player_id,
                player_name,
                raw_type_lower,
            };
        }
    }

    let mut team_id = None;
    let mut player_name = "Player".to_string();

    if let Some(name) = event.event_type.split(":Home:").nth(1) {
        team_id = Some(0);
        player_name = name.to_string();
    } else if let Some(name) = event.event_type.split(":Away:").nth(1) {
        team_id = Some(1);
        player_name = name.to_string();
    }

    EventMetadata {
        category: EventCategory::Generic,
        team_id,
        player_id: None,
        player_name,
        raw_type_lower,
    }
}

fn extract_numeric_id(name: &str) -> Option<u32> {
    let digits: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u32>().ok()
    }
}

fn label_to_team_id(label: &str) -> Option<u32> {
    match label {
        "Home" => Some(0),
        "Away" => Some(1),
        _ => None,
    }
}

/// Open-Football 엔진 이벤트를 우리 프로젝트의 ReplayEvent로 변환 (레거시)
pub fn map_engine_events(engine_events: Vec<Event>) -> Vec<ReplayEvent> {
    let mut replay_events = Vec::new();

    for (i, ev) in engine_events.iter().enumerate() {
        // For now, we'll create basic events with dummy time and data
        // TODO: Replace with actual Open-Football event parsing when available

        match ev.event_type.as_str() {
            "kickoff" => {
                replay_events.push(ReplayEvent::KickOff {
                    base: EventBase {
                        t: i as f64,
                        player_id: None,
                        team_id: Some(0),
                    },
                });
            }
            "goal" => {
                replay_events.push(ReplayEvent::Goal {
                    base: EventBase {
                        t: i as f64,
                        player_id: Some(9), // Dummy player ID
                        team_id: Some(0),
                    },
                    at: MeterPos { x: 50.0, y: 34.0 }, // Dummy position
                    assist_player_id: None,
                });
            }
            "pass" => {
                replay_events.push(ReplayEvent::Pass {
                    base: EventBase {
                        t: i as f64,
                        player_id: Some(5), // Dummy player ID
                        team_id: Some(0),
                    },
                    from: MeterPos { x: 30.0, y: 25.0 },
                    to: MeterPos { x: 40.0, y: 30.0 },
                    receiver_id: None,
                    distance_m: Some(10.0),
                    force: None,
                    is_clearance: false,
                    ground: None,
                    outcome: None,
                    passing_skill: None,
                    vision: None,
                    technique: None,
                });
            }
            "shot" => {
                replay_events.push(ReplayEvent::Shot {
                    base: EventBase {
                        t: i as f64,
                        player_id: Some(10), // Dummy player ID
                        team_id: Some(0),
                    },
                    from: MeterPos { x: 90.0, y: 34.0 },
                    target: MeterPos { x: 105.0, y: 34.0 },
                    on_target: true,
                    xg: Some(0.15),
                    shot_speed: Some(22.0),
                    long_shots_skill: None,
                    finishing_skill: None,
                    technique: None,
                    shot_type: None,
                    defender_pressure: None,
                    angle_to_goal: None,
                    distance_to_goal: None,
                    composure: None,
                    curve_factor: None,
                });
            }
            _ => {
                // Default to BallMove for unknown events
                replay_events.push(ReplayEvent::BallMove {
                    base: EventBase {
                        t: i as f64,
                        player_id: None,
                        team_id: None,
                    },
                    to: MeterPos { x: 52.5, y: 34.0 }, // Center of field
                });
            }
        }
    }

    replay_events
}

/// Create a simple match with basic events (for testing without Open-Football engine)
pub fn create_simple_match_events() -> Vec<ReplayEvent> {
    vec![
        ReplayEvent::KickOff {
            base: EventBase {
                t: 0.0,
                player_id: None,
                team_id: Some(0),
            },
        },
        ReplayEvent::Pass {
            base: EventBase {
                t: 30.0,
                player_id: Some(5),
                team_id: Some(0),
            },
            from: MeterPos { x: 52.5, y: 34.0 },
            to: MeterPos { x: 70.0, y: 30.0 },
            receiver_id: None,
            distance_m: Some(18.0),
            force: None,
            is_clearance: false,
            ground: None,
            outcome: None,
            passing_skill: None,
            vision: None,
            technique: None,
        },
        ReplayEvent::Shot {
            base: EventBase {
                t: 35.0,
                player_id: Some(9),
                team_id: Some(0),
            },
            from: MeterPos { x: 95.0, y: 34.0 },
            target: MeterPos { x: 105.0, y: 34.0 },
            on_target: true,
            xg: Some(0.25),
            shot_speed: Some(24.0),
            long_shots_skill: None,
            finishing_skill: None,
            technique: None,
            shot_type: None,
            defender_pressure: None,
            angle_to_goal: None,
            distance_to_goal: None,
            composure: None,
            curve_factor: None,
        },
        ReplayEvent::Goal {
            base: EventBase {
                t: 36.0,
                player_id: Some(9),
                team_id: Some(0),
            },
            at: MeterPos { x: 105.0, y: 34.0 },
            assist_player_id: None,
        },
        ReplayEvent::HalfTime {
            base: EventBase {
                t: 45.0 * 60.0, // 45 minutes
                player_id: None,
                team_id: None,
            },
        },
        ReplayEvent::FullTime {
            base: EventBase {
                t: 90.0 * 60.0, // 90 minutes
                player_id: None,
                team_id: None,
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_events() {
        let mapped = map_engine_events(Vec::new());
        assert!(mapped.is_empty());
    }

    #[test]
    fn test_simple_match_creation() {
        let events = create_simple_match_events();
        assert!(!events.is_empty());
        assert!(matches!(events[0], ReplayEvent::KickOff { .. }));
        assert!(matches!(
            events.last().unwrap(),
            ReplayEvent::FullTime { .. }
        ));
    }

    #[cfg(feature = "vendor_skills")]
    #[test]
    fn test_skills36_mapping() {
        use of_core::models::player::PlayerAttributes;

        let mut attrs = PlayerAttributes::default();
        attrs.passing = 75;
        attrs.vision = 80;
        attrs.pace = 85;

        let skills = map_player_attributes_to_skills36(&attrs);
        assert_eq!(skills.passing, 75);
        assert_eq!(skills.vision, 80);
        assert_eq!(skills.pace, 85);
    }
}

// ============================================================================
// Skills36 매핑 함수들 (PlayerAttributes → Skills36)
// ============================================================================

/// PlayerAttributes를 Skills36으로 정확히 1:1 매핑
///
/// 우리의 42개 속성 중 36개를 OpenFootball Skills36으로 변환합니다.
/// - Technical 14개: 직접 매핑
/// - Mental 14개: 직접 매핑
/// - Physical 8개: 직접 매핑
/// - GK 6개 속성: 무시됨 (Skills36에 없음)
#[cfg(feature = "vendor_skills")]
pub fn map_player_attributes_to_skills36(attrs: &PlayerAttributes) -> Skills36 {
    Skills36 {
        // Technical (14개) - 직접 1:1 매핑
        corners: attrs.corners,
        crossing: attrs.crossing,
        dribbling: attrs.dribbling,
        finishing: attrs.finishing,
        first_touch: attrs.first_touch,
        free_kicks: attrs.free_kicks,
        heading: attrs.heading,
        long_shots: attrs.long_shots,
        long_throws: attrs.long_throws,
        marking: attrs.marking,
        passing: attrs.passing,
        penalty_taking: attrs.penalty_taking,
        tackling: attrs.tackling,
        technique: attrs.technique,

        // Mental (14개) - 직접 1:1 매핑
        aggression: attrs.aggression,
        anticipation: attrs.anticipation,
        bravery: attrs.bravery,
        composure: attrs.composure,
        concentration: attrs.concentration,
        decisions: attrs.decisions,
        determination: attrs.determination,
        flair: attrs.flair,
        leadership: attrs.leadership,
        off_the_ball: attrs.off_the_ball,
        positioning: attrs.positioning,
        teamwork: attrs.teamwork,
        vision: attrs.vision,
        work_rate: attrs.work_rate,

        // Physical (8개) - 직접 1:1 매핑
        acceleration: attrs.acceleration,
        agility: attrs.agility,
        balance: attrs.balance,
        jumping: attrs.jumping,
        natural_fitness: attrs.natural_fitness,
        pace: attrs.pace, // pace를 사용 (speed는 별칭)
        stamina: attrs.stamina,
        strength: attrs.strength,
    }
}

/// CorePlayer에서 Skills36 추출
///
/// CorePlayer의 detailed_stats (PlayerAttributes)를 Skills36으로 변환합니다.
#[cfg(feature = "vendor_skills")]
pub fn extract_skills36_from_player(player: &CorePlayer) -> Skills36 {
    map_player_attributes_to_skills36(&player.detailed_stats)
}

/// Skills36을 PlayerAttributes로 역매핑 (테스트용)
///
/// Skills36의 36개 속성을 PlayerAttributes 42개로 변환합니다.
/// 추가 속성들은 기본값 또는 계산값으로 채웁니다.
#[cfg(feature = "vendor_skills")]
pub fn map_skills36_to_player_attributes(skills: &Skills36) -> PlayerAttributes {
    PlayerAttributes {
        // Technical - 직접 역매핑
        corners: skills.corners,
        crossing: skills.crossing,
        dribbling: skills.dribbling,
        finishing: skills.finishing,
        first_touch: skills.first_touch,
        free_kicks: skills.free_kicks,
        heading: skills.heading,
        long_shots: skills.long_shots,
        long_throws: skills.long_throws,
        marking: skills.marking,
        passing: skills.passing,
        penalty_taking: skills.penalty_taking,
        tackling: skills.tackling,
        technique: skills.technique,

        // Note: ball_control and shooting are not in our PlayerAttributes
        // (they're composite attributes in Skills36)

        // Mental - 직접 역매핑
        aggression: skills.aggression,
        anticipation: skills.anticipation,
        bravery: skills.bravery,
        composure: skills.composure,
        concentration: skills.concentration,
        decisions: skills.decisions,
        determination: skills.determination,
        flair: skills.flair,
        leadership: skills.leadership,
        off_the_ball: skills.off_the_ball,
        positioning: skills.positioning,
        teamwork: skills.teamwork,
        vision: skills.vision,
        work_rate: skills.work_rate,

        // Physical - 직접 역매핑
        acceleration: skills.acceleration,
        agility: skills.agility,
        balance: skills.balance,
        jumping: skills.jumping,
        natural_fitness: skills.natural_fitness,
        pace: skills.pace,
        stamina: skills.stamina,
        strength: skills.strength,
        // Note: GK 6 attributes (reflexes, handling, aerial_ability,
        // command_of_area, communication, kicking) are not included in
        // our PlayerAttributes struct (Skills36 doesn't have them either)
    }
}

/// 포지션별 Skills36 가중치 적용
///
/// 포지션에 따라 중요한 속성에 가중치를 적용합니다.
#[cfg(feature = "vendor_skills")]
pub fn apply_position_weights_to_skills36(skills: &mut Skills36, position: &str) {
    match position {
        "GK" => {
            // GK는 Skills36에 전문 속성이 없으므로 수비적 속성 강화
            skills.positioning = (skills.positioning as f32 * 1.2).min(100.0) as u8;
            skills.concentration = (skills.concentration as f32 * 1.2).min(100.0) as u8;
            skills.anticipation = (skills.anticipation as f32 * 1.2).min(100.0) as u8;
        }
        "DF" | "CB" | "LB" | "RB" => {
            // 수비수: 수비 속성 강화
            skills.marking = (skills.marking as f32 * 1.15).min(100.0) as u8;
            skills.tackling = (skills.tackling as f32 * 1.15).min(100.0) as u8;
            skills.positioning = (skills.positioning as f32 * 1.15).min(100.0) as u8;
            skills.strength = (skills.strength as f32 * 1.1).min(100.0) as u8;
        }
        "MF" | "CM" | "CDM" | "CAM" => {
            // 미드필더: 패스와 비전 강화
            skills.passing = (skills.passing as f32 * 1.15).min(100.0) as u8;
            skills.vision = (skills.vision as f32 * 1.15).min(100.0) as u8;
            skills.technique = (skills.technique as f32 * 1.1).min(100.0) as u8;
            skills.work_rate = (skills.work_rate as f32 * 1.1).min(100.0) as u8;
        }
        "FW" | "ST" | "CF" | "LW" | "RW" => {
            // 공격수: 공격 속성 강화
            skills.finishing = (skills.finishing as f32 * 1.2).min(100.0) as u8;
            skills.dribbling = (skills.dribbling as f32 * 1.15).min(100.0) as u8;
            skills.pace = (skills.pace as f32 * 1.1).min(100.0) as u8;
            skills.acceleration = (skills.acceleration as f32 * 1.1).min(100.0) as u8;
        }
        _ => {} // 기본 포지션은 변경 없음
    }
}
