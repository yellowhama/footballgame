//! DSA Summary (authoritative, Rust-owned)
//!
//! Purpose:
//! - Provide deterministic post-match "Distributed Sensing Analytics" summaries
//!   derived from `MatchResult.position_data` + minimal event hints.
//! - This is *not* a gameplay SSOT. It is post-match telemetry only.
//! - Godot live DSA remains a lightweight runtime proxy; this module is the
//!   authoritative, replay-stable version for reports/CI.

use super::physics_constants::field;
use crate::analysis::metrics::gini::gini_coefficient_f32;
use crate::calibration::zone::pos_to_posplay_zone_meters;
use crate::models::{EventType, MatchEvent, MatchPositionData, MatchResult, TeamSide};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsaSummary {
    pub duration_minutes: u8,
    /// UI-friendly minute series (0..duration).
    pub minute_series: DsaMinuteSeries,
    /// Top hubs + inequality metric (Gini).
    pub hub: DsaHubSummary,
    /// Zone transition matrices (20×20, flattened).
    pub routes: DsaRouteSummary,
    /// Basic QA warnings derived from DSA signals (non-blocking).
    pub qa_warnings: Vec<DsaQaWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DsaMinuteSeries {
    /// Opponent pressure against the current ball carrier (0..1), averaged per minute.
    pub pressure: Vec<f32>,
    /// Ball speed (m/s), averaged per minute.
    pub tempo: Vec<f32>,
    /// Zone transitions count per minute.
    pub transitions: Vec<u32>,
    /// Ball speed (m/s) while HOME team is in possession, averaged per minute.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tempo_home: Vec<f32>,
    /// Ball speed (m/s) while AWAY team is in possession, averaged per minute.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tempo_away: Vec<f32>,
    /// Zone transitions per minute while HOME team is in possession.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub transitions_home: Vec<u32>,
    /// Zone transitions per minute while AWAY team is in possession.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub transitions_away: Vec<u32>,
    /// Optional: pressure against home/away while they are in possession (0..1).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub pressure_against_home: Vec<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub pressure_against_away: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DsaHubSummary {
    pub gini: f32,
    pub top3: Vec<DsaHubPlayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsaHubPlayer {
    pub track_id: u8,
    pub team: String, // "home" / "away"
    pub hub_score: f32,
    pub owner_time_s: f32,
    pub receive_count: u32,
    pub release_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DsaRouteSummary {
    pub zone_count: u8,
    /// Home team transition counts (flattened zone_count×zone_count).
    pub transitions_home: Vec<u32>,
    /// Away team transition counts (flattened zone_count×zone_count).
    pub transitions_away: Vec<u32>,
    /// Top-N routes (sorted by count desc).
    pub top_routes: Vec<DsaRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsaRoute {
    pub team: String, // "home"/"away"
    pub from_zone: u8,
    pub to_zone: u8,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DsaQaWarningKind {
    HubInequalityHigh,
    LaneImbalanceHigh,
    ZoneDwellSkewHigh,
    HighPressureLowTransitions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsaQaWarning {
    pub kind: DsaQaWarningKind,
    pub message: String,
}

const ZONE_LANES: u8 = 5;
const ZONE_QUARTERS: u8 = 4;
const ZONE_COUNT: u8 = ZONE_LANES * ZONE_QUARTERS; // 20

const DEFAULT_HALF_TIME_MS: u64 = 45 * 60_000;

const OWNER_MAX_DIST_M: f32 = 3.0;
const OWNER_HYSTERESIS_MS: u64 = 1200;
const OWNER_HYSTERESIS_HEIGHT_M: f32 = 0.25;
const OWNER_HYSTERESIS_SPEED_MPS: f32 = 6.0;
const PRESSURE_RADIUS_M: f32 = 6.0;
const PRESSURE_NORM_CAP: f32 = 3.0;

pub fn analyze_dsa_summary(result: &MatchResult, duration_minutes: u8) -> Option<DsaSummary> {
    let pos = result.position_data.as_ref()?;
    Some(analyze_from_position_data(pos, &result.events, duration_minutes))
}

fn analyze_from_position_data(
    position_data: &MatchPositionData,
    events: &[MatchEvent],
    duration_minutes: u8,
) -> DsaSummary {
    let duration = duration_minutes.max(1).min(180) as usize;

    let halftime_ms = resolve_half_time_ms(events);

    let mut minute_samples = vec![0u32; duration + 1];
    let mut minute_pressure_sum = vec![0f32; duration + 1];
    let mut minute_tempo_sum = vec![0f32; duration + 1];
    let mut minute_tempo_home_sum = vec![0f32; duration + 1];
    let mut minute_tempo_home_samples = vec![0u32; duration + 1];
    let mut minute_tempo_away_sum = vec![0f32; duration + 1];
    let mut minute_tempo_away_samples = vec![0u32; duration + 1];
    let mut minute_transitions_home = vec![0u32; duration + 1];
    let mut minute_transitions_away = vec![0u32; duration + 1];

    let mut minute_pressure_home_sum = vec![0f32; duration + 1];
    let mut minute_pressure_home_samples = vec![0u32; duration + 1];
    let mut minute_pressure_away_sum = vec![0f32; duration + 1];
    let mut minute_pressure_away_samples = vec![0u32; duration + 1];

    // Route matrices
    let zc = ZONE_COUNT as usize;
    let mut mat_home = vec![0u32; zc * zc];
    let mut mat_away = vec![0u32; zc * zc];

    // Hub tracking
    let mut owner_time_s = vec![0f32; 22];
    let mut recv = vec![0u32; 22];
    let mut rel = vec![0u32; 22];

    // Keep current player positions by advancing per-player iterators.
    let mut player_idx = vec![0usize; 22];
    let mut player_pos = vec![(0f32, 0f32); 22];
    let mut player_has = vec![false; 22];

    let mut last_t_ms: Option<u64> = None;
    let mut last_ball_pos: Option<(f32, f32)> = None;
    let mut last_owner: Option<u8> = None;
    let mut last_confirmed_owner: Option<u8> = None;
    let mut last_confirmed_owner_t_ms: Option<u64> = None;
    let mut last_possession_team: Option<TeamSide> = None;
    let mut last_zone_in_possession: Option<u8> = None;

    for (ball_i, ball_item) in position_data.ball.iter().enumerate() {
        let t_ms = ball_item.timestamp;
        let minute_idx = (t_ms / 60_000) as usize;
        if minute_idx > duration {
            break;
        }

        // dt since last ball sample
        let dt_s = if let Some(prev) = last_t_ms {
            (t_ms.saturating_sub(prev) as f32) / 1000.0
        } else {
            0.0
        };

        // Advance player timelines up to t_ms.
        for pid in 0..22 {
            let list = &position_data.players[pid];
            while player_idx[pid] < list.len() && list[player_idx[pid]].timestamp <= t_ms {
                player_pos[pid] = list[player_idx[pid]].position;
                player_has[pid] = true;
                player_idx[pid] += 1;
            }
        }

        let ball_pos = ball_item.position;

        // Ball speed
        let ball_speed = if let Some((vx, vy)) = ball_item.velocity {
            (vx * vx + vy * vy).sqrt()
        } else if let (Some(prev_pos), Some(prev_t)) = (last_ball_pos, last_t_ms) {
            let dt = (t_ms.saturating_sub(prev_t) as f32) / 1000.0;
            if dt > 0.0001 {
                let dx = ball_pos.0 - prev_pos.0;
                let dy = ball_pos.1 - prev_pos.1;
                (dx * dx + dy * dy).sqrt() / dt
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Estimate current owner as nearest player to the ball, with a short
        // hysteresis window to keep possession continuity during passes.
        let (owner_tid_raw, owner_dist) = nearest_player_to(ball_pos, &player_pos, &player_has);
        let owner_tid_raw = if owner_dist <= OWNER_MAX_DIST_M {
            owner_tid_raw
        } else {
            None
        };

        if owner_tid_raw.is_some() {
            last_confirmed_owner = owner_tid_raw;
            last_confirmed_owner_t_ms = Some(t_ms);
        }

        let ball_height = ball_item.height.unwrap_or(0.0);
        let owner_tid = owner_tid_raw.or_else(|| {
            let Some(last_owner) = last_confirmed_owner else {
                return None;
            };
            let Some(last_owner_t_ms) = last_confirmed_owner_t_ms else {
                return None;
            };
            let dt_ms = t_ms.saturating_sub(last_owner_t_ms);
            let in_flight_or_fast =
                ball_height >= OWNER_HYSTERESIS_HEIGHT_M || ball_speed >= OWNER_HYSTERESIS_SPEED_MPS;
            if dt_ms <= OWNER_HYSTERESIS_MS && in_flight_or_fast {
                Some(last_owner)
            } else {
                None
            }
        });

        // Pressure (0..1) against owner.
        let pressure_norm = if let Some(owner) = owner_tid {
            let owner_team = TeamSide::from_track_id(owner as usize);
            let mut sum = 0.0;
            for pid in 0..22 {
                if !player_has[pid] || pid as u8 == owner {
                    continue;
                }
                if TeamSide::from_track_id(pid) == owner_team {
                    continue;
                }
                let dx = player_pos[pid].0 - ball_pos.0;
                let dy = player_pos[pid].1 - ball_pos.1;
                let d = (dx * dx + dy * dy).sqrt();
                sum += (-d / PRESSURE_RADIUS_M).exp();
            }
            (sum / PRESSURE_NORM_CAP).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Minute aggregates
        minute_samples[minute_idx] += 1;
        minute_pressure_sum[minute_idx] += pressure_norm;
        minute_tempo_sum[minute_idx] += ball_speed;

        if let Some(owner) = owner_tid {
            match TeamSide::from_track_id(owner as usize) {
                TeamSide::Home => {
                    minute_pressure_home_sum[minute_idx] += pressure_norm;
                    minute_pressure_home_samples[minute_idx] += 1;
                    minute_tempo_home_sum[minute_idx] += ball_speed;
                    minute_tempo_home_samples[minute_idx] += 1;
                }
                TeamSide::Away => {
                    minute_pressure_away_sum[minute_idx] += pressure_norm;
                    minute_pressure_away_samples[minute_idx] += 1;
                    minute_tempo_away_sum[minute_idx] += ball_speed;
                    minute_tempo_away_samples[minute_idx] += 1;
                }
            }
        }

        // Hub + route bookkeeping (needs dt)
        if dt_s > 0.0 {
            if let Some(owner) = owner_tid {
                if (owner as usize) < owner_time_s.len() {
                    owner_time_s[owner as usize] += dt_s;
                }
            }
        }

        if let Some((prev_owner, cur_owner)) = match (last_owner, owner_tid) {
            (Some(a), Some(b)) if a != b => Some((a, b)),
            _ => None,
        } {
            if (cur_owner as usize) < recv.len() {
                recv[cur_owner as usize] += 1;
            }
            if (prev_owner as usize) < rel.len() {
                rel[prev_owner as usize] += 1;
            }
        }

        // Zone transitions (team-view, possession-scoped).
        //
        // Important: compute in the possessing team's perspective (mirrored by attack direction),
        // and reset the last-zone when possession changes to avoid false transitions on turnovers.
        let possession_team = owner_tid.map(|tid| TeamSide::from_track_id(tid as usize));
        if possession_team != last_possession_team {
            last_zone_in_possession = None;
            last_possession_team = possession_team;
        }

        if let Some(team) = possession_team {
            let attacks_right = attacks_right_for_team(team, t_ms, halftime_ms);
            let zone = pos_to_team_zone_id(ball_pos, attacks_right);
            if let Some(prev_zone) = last_zone_in_possession {
                if zone != prev_zone {
                    let idx = (prev_zone as usize) * zc + (zone as usize);
                    match team {
                        TeamSide::Home => {
                            mat_home[idx] += 1;
                            minute_transitions_home[minute_idx] += 1;
                        }
                        TeamSide::Away => {
                            mat_away[idx] += 1;
                            minute_transitions_away[minute_idx] += 1;
                        }
                    }
                }
            }
            last_zone_in_possession = Some(zone);
        }

        last_t_ms = Some(t_ms);
        last_ball_pos = Some(ball_pos);
        last_owner = owner_tid;
    }

    // Finalize minute series
    let mut pressure = vec![0.0f32; duration + 1];
    let mut tempo = vec![0.0f32; duration + 1];
    let mut tempo_home = vec![0.0f32; duration + 1];
    let mut tempo_away = vec![0.0f32; duration + 1];

    for m in 0..=duration {
        let n = minute_samples[m].max(1) as f32;
        pressure[m] = (minute_pressure_sum[m] / n).clamp(0.0, 1.0);
        tempo[m] = minute_tempo_sum[m] / n;

        if minute_tempo_home_samples[m] > 0 {
            tempo_home[m] = minute_tempo_home_sum[m] / minute_tempo_home_samples[m] as f32;
        }
        if minute_tempo_away_samples[m] > 0 {
            tempo_away[m] = minute_tempo_away_sum[m] / minute_tempo_away_samples[m] as f32;
        }
    }

    let mut transitions = vec![0u32; duration + 1];
    for m in 0..=duration {
        transitions[m] = minute_transitions_home[m] + minute_transitions_away[m];
    }

    let mut pressure_against_home = vec![0.0f32; duration + 1];
    let mut pressure_against_away = vec![0.0f32; duration + 1];
    for m in 0..=duration {
        if minute_pressure_home_samples[m] > 0 {
            pressure_against_home[m] =
                (minute_pressure_home_sum[m] / minute_pressure_home_samples[m] as f32).clamp(0.0, 1.0);
        }
        if minute_pressure_away_samples[m] > 0 {
            pressure_against_away[m] =
                (minute_pressure_away_sum[m] / minute_pressure_away_samples[m] as f32).clamp(0.0, 1.0);
        }
    }

    // Hub scores + gini
    let hub_scores = compute_hub_scores(&owner_time_s, &recv, &rel);
    let gini = gini_coefficient_f32(&hub_scores).unwrap_or(0.0) as f32;
    let top3 = top3_hubs(&hub_scores, &owner_time_s, &recv, &rel);

    // Top routes
    let top_routes = top_routes(&mat_home, &mat_away, 10);

    // QA warnings (non-blocking)
    let qa_warnings =
        compute_qa_warnings(duration_minutes, &pressure, &transitions, gini, &mat_home, &mat_away);

    DsaSummary {
        duration_minutes,
        minute_series: DsaMinuteSeries {
            pressure,
            tempo,
            transitions,
            tempo_home,
            tempo_away,
            transitions_home: minute_transitions_home,
            transitions_away: minute_transitions_away,
            pressure_against_home,
            pressure_against_away,
        },
        hub: DsaHubSummary { gini, top3 },
        routes: DsaRouteSummary {
            zone_count: ZONE_COUNT,
            transitions_home: mat_home,
            transitions_away: mat_away,
            top_routes,
        },
        qa_warnings,
    }
}

fn nearest_player_to(
    pos: (f32, f32),
    player_pos: &[(f32, f32)],
    player_has: &[bool],
) -> (Option<u8>, f32) {
    let mut best: Option<u8> = None;
    let mut best_d = f32::INFINITY;
    for (i, has) in player_has.iter().enumerate() {
        if !*has {
            continue;
        }
        let dx = player_pos[i].0 - pos.0;
        let dy = player_pos[i].1 - pos.1;
        let d = (dx * dx + dy * dy).sqrt();
        if d < best_d {
            best_d = d;
            best = Some(i as u8);
        }
    }
    (best, best_d)
}

fn resolve_half_time_ms(events: &[MatchEvent]) -> u64 {
    events
        .iter()
        .find(|e| e.event_type == EventType::HalfTime)
        .map(|e| e.timestamp_ms.unwrap_or(e.minute as u64 * 60_000))
        .unwrap_or(DEFAULT_HALF_TIME_MS)
}

fn attacks_right_for_team(team: TeamSide, t_ms: u64, halftime_ms: u64) -> bool {
    // Coordinate contract: Home attacks right in 1st half, then switches sides.
    let home_attacks_right = t_ms < halftime_ms;
    match team {
        TeamSide::Home => home_attacks_right,
        TeamSide::Away => !home_attacks_right,
    }
}

fn pos_to_team_zone_id(pos_m: (f32, f32), attacks_right: bool) -> u8 {
    let x = pos_m.0.clamp(0.0, field::LENGTH_M);
    let y = pos_m.1.clamp(0.0, field::WIDTH_M);
    pos_to_posplay_zone_meters(x, y, attacks_right).to_index() as u8
}

fn compute_hub_scores(owner_time_s: &[f32], recv: &[u32], rel: &[u32]) -> Vec<f32> {
    const W_OWNER_TIME_S: f32 = 1.0;
    const W_RECV: f32 = 2.0;
    const W_REL: f32 = 2.0;

    let mut out = vec![0.0f32; 22];
    for i in 0..22 {
        out[i] = owner_time_s[i] * W_OWNER_TIME_S + (recv[i] as f32) * W_RECV + (rel[i] as f32) * W_REL;
    }
    out
}

fn top3_hubs(hub_scores: &[f32], owner_time_s: &[f32], recv: &[u32], rel: &[u32]) -> Vec<DsaHubPlayer> {
    let mut idx: Vec<usize> = (0..hub_scores.len()).collect();
    idx.sort_by(|a, b| hub_scores[*b].partial_cmp(&hub_scores[*a]).unwrap_or(std::cmp::Ordering::Equal));
    idx.truncate(3);

    idx.into_iter()
        .map(|i| {
            let track_id = i as u8;
            let team = match TeamSide::from_track_id(track_id as usize) {
                TeamSide::Home => "home",
                TeamSide::Away => "away",
            };
            DsaHubPlayer {
                track_id,
                team: team.to_string(),
                hub_score: hub_scores[i],
                owner_time_s: owner_time_s[i],
                receive_count: recv[i],
                release_count: rel[i],
            }
        })
        .collect()
}

fn top_routes(mat_home: &[u32], mat_away: &[u32], top_n: usize) -> Vec<DsaRoute> {
    let mut routes: Vec<DsaRoute> = Vec::new();
    let zc = ZONE_COUNT as usize;

    for (team, mat) in [("home", mat_home), ("away", mat_away)] {
        for from in 0..zc {
            for to in 0..zc {
                let c = mat[from * zc + to];
                if c == 0 {
                    continue;
                }
                routes.push(DsaRoute {
                    team: team.to_string(),
                    from_zone: from as u8,
                    to_zone: to as u8,
                    count: c,
                });
            }
        }
    }

    routes.sort_by(|a, b| b.count.cmp(&a.count));
    routes.truncate(top_n);
    routes
}

fn compute_qa_warnings(
    duration_minutes: u8,
    pressure: &[f32],
    transitions: &[u32],
    gini: f32,
    mat_home: &[u32],
    mat_away: &[u32],
) -> Vec<DsaQaWarning> {
    let mut warnings = Vec::new();

    // 1) Hub inequality (proxy: gini on hub scores)
    if gini > 0.65 {
        warnings.push(DsaQaWarning {
            kind: DsaQaWarningKind::HubInequalityHigh,
            message: format!("Hub inequality high (gini={:.2})", gini),
        });
    }

    // 2) Lane imbalance (proxy via transition matrices aggregated by lane)
    if let Some(msg) = lane_imbalance_warning(mat_home, mat_away) {
        warnings.push(DsaQaWarning {
            kind: DsaQaWarningKind::LaneImbalanceHigh,
            message: msg,
        });
    }

    // 3) Zone dwell skew (proxy via transition matrix row sums)
    if let Some(msg) = zone_skew_warning(mat_home, mat_away) {
        warnings.push(DsaQaWarning {
            kind: DsaQaWarningKind::ZoneDwellSkewHigh,
            message: msg,
        });
    }

    // 4) High pressure but very low transitions for long periods (simple)
    let duration = duration_minutes.max(1).min(180) as usize;
    let mut flagged = 0;
    for m in 0..=duration {
        if pressure.get(m).copied().unwrap_or(0.0) >= 0.75 && transitions.get(m).copied().unwrap_or(0) <= 1 {
            flagged += 1;
        }
    }
    if flagged >= 8 {
        warnings.push(DsaQaWarning {
            kind: DsaQaWarningKind::HighPressureLowTransitions,
            message: format!(
                "High pressure with low transitions for {} minutes (pressure>=0.75 & transitions<=1)",
                flagged
            ),
        });
    }

    warnings
}

fn lane_imbalance_warning(mat_home: &[u32], mat_away: &[u32]) -> Option<String> {
    let mut lane_counts = [0u32; ZONE_LANES as usize];
    let zc = ZONE_COUNT as usize;

    for mat in [mat_home, mat_away] {
        for from in 0..zc {
            for to in 0..zc {
                let c = mat[from * zc + to];
                if c == 0 {
                    continue;
                }
                let lane = (to % (ZONE_LANES as usize)).min((ZONE_LANES as usize) - 1);
                lane_counts[lane] += c;
            }
        }
    }

    let total: u32 = lane_counts.iter().sum();
    if total == 0 {
        return None;
    }
    let max_share = lane_counts
        .iter()
        .map(|c| (*c as f32) / (total as f32))
        .fold(0.0, f32::max);

    if max_share > 0.60 {
        Some(format!("Lane imbalance high (max lane share={:.0}%)", max_share * 100.0))
    } else {
        None
    }
}

fn zone_skew_warning(mat_home: &[u32], mat_away: &[u32]) -> Option<String> {
    let zc = ZONE_COUNT as usize;
    let mut zone_activity = vec![0u32; zc];

    for mat in [mat_home, mat_away] {
        for from in 0..zc {
            for to in 0..zc {
                let c = mat[from * zc + to];
                zone_activity[to] += c;
            }
        }
    }

    let total: u32 = zone_activity.iter().sum();
    if total == 0 {
        return None;
    }
    let max_share = zone_activity
        .iter()
        .map(|c| (*c as f32) / (total as f32))
        .fold(0.0, f32::max);

    if max_share > 0.25 {
        Some(format!("Zone dwell skew high (max zone share={:.0}%)", max_share * 100.0))
    } else {
        None
    }
}
