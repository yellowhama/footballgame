//! Interpretation Layer v1 (Replay & Analytics meaning layer)
//!
//! Scope:
//! - Post-match only (does not affect simulation outcomes)
//! - Deterministic: same MatchResult (+ optional DSA) => same output
//! - Output is designed to be UI-friendly and Bridge-friendly (Godot Dictionary)
//!
//! Spec: docs/specs/fix_2601/0115/0115_INTERPRETATION_LAYER_REPLAY_ANALYTICS_V1_SPEC.md

use super::dsa_summary::{DsaQaWarningKind, DsaSummary};
use crate::models::{EventType, MatchResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HighlightKindV1 {
    DecisionCollapse,
    StructureBreak,
    PressureOverload,
    TransitionFailure,
    OverReliance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchInterpretationV1 {
    pub report: MatchAnalysisReportV1,
    pub highlights: Vec<HighlightClipV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchAnalysisReportV1 {
    pub summary: MatchAnalysisSummaryV1,
    pub what_worked: Vec<String>,
    pub what_broke_top3: Vec<String>,
    pub why_it_broke: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_actions: Option<NextActionsV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchAnalysisSummaryV1 {
    pub score: ScoreV1,
    pub headline: String,
    pub subline: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScoreV1 {
    pub home: u8,
    pub away: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextActionsV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tactics: Option<NextActionItemV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training: Option<NextActionItemV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deck: Option<NextActionItemV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextActionItemV1 {
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightClipV1 {
    pub clip_id: String,
    pub kind: HighlightKindV1,
    pub t0_ms: u64,
    pub t1_ms: u64,
    pub minute0: u8,
    pub minute1: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<HighlightFocusV1>,
    pub evidence: HighlightEvidenceV1,
    pub interpretation: HighlightInterpretationV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_actions: Option<HighlightNextActionsV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightFocusV1 {
    pub team: String, // "home"/"away"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_id: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HighlightEvidenceV1 {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub primary_events: Vec<MatchEventRefV1>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub action_results: Vec<ActionResultRefV1>,
    pub metrics: Vec<EvidenceMetricV1>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub snapshots: Vec<SnapshotRefV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEventRefV1 {
    pub t_ms: u64,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub is_home_team: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_track_id: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResultRefV1 {
    pub t_ms: u64,
    pub actor_track_id: u8,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceMetricV1 {
    pub key: String,
    pub value: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRefV1 {
    pub t_ms: u64,
    pub ball: (f32, f32),
    pub players_subset: Vec<PlayerPosRefV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPosRefV1 {
    pub track_id: u8,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightInterpretationV1 {
    pub headline: String,
    pub explanation: String,
    pub confidence: f32,
    pub blame_balance: BlameBalanceV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BlameBalanceV1 {
    pub player: f32,
    pub structure: f32,
    pub opponent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightNextActionsV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tactics_suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deck_suggestion: Option<String>,
}

pub fn build_interpretation_v1(result: &MatchResult, dsa: Option<&DsaSummary>) -> MatchInterpretationV1 {
    let home = result.score_home;
    let away = result.score_away;
    let mut highlights = Vec::new();

    if let Some(dsa) = dsa {
        highlights.extend(build_highlights_from_dsa(result, dsa));
    } else {
        highlights.extend(build_highlights_fallback(result));
    }

    canonicalize_highlights(&mut highlights);

    let what_broke_top3 = highlights
        .iter()
        .take(3)
        .map(|h| format!("{}' {}", h.minute0, h.interpretation.headline))
        .collect::<Vec<_>>();

    let headline = if !highlights.is_empty() {
        highlights[0].interpretation.headline.clone()
    } else {
        "경기 흐름을 요약할 데이터가 부족했다".to_string()
    };

    let subline = if highlights.len() >= 2 {
        format!(
            "{} + {}",
            highlights[0].kind_label(),
            highlights[1].kind_label()
        )
    } else if highlights.len() == 1 {
        highlights[0].kind_label()
    } else {
        "해석 없음".to_string()
    };

    MatchInterpretationV1 {
        report: MatchAnalysisReportV1 {
            summary: MatchAnalysisSummaryV1 {
                score: ScoreV1 { home, away },
                headline,
                subline,
            },
            what_worked: Vec::new(),
            what_broke_top3,
            why_it_broke: Vec::new(),
            next_actions: None,
        },
        highlights,
    }
}

impl HighlightClipV1 {
    fn kind_label(&self) -> String {
        match self.kind {
            HighlightKindV1::DecisionCollapse => "선택지 붕괴".to_string(),
            HighlightKindV1::StructureBreak => "구조 붕괴".to_string(),
            HighlightKindV1::PressureOverload => "압박 과부하".to_string(),
            HighlightKindV1::TransitionFailure => "전이 실패".to_string(),
            HighlightKindV1::OverReliance => "한 명 몰빵".to_string(),
        }
    }
}

fn canonicalize_highlights(highlights: &mut Vec<HighlightClipV1>) {
    for h in highlights.iter_mut() {
        h.evidence.metrics.sort_by(|a, b| a.key.cmp(&b.key));
        h.evidence.primary_events.sort_by(|a, b| {
            (a.t_ms, a.is_home_team, a.player_track_id).cmp(&(b.t_ms, b.is_home_team, b.player_track_id))
        });
        h.evidence.snapshots.sort_by_key(|s| s.t_ms);
    }

    highlights.sort_by(|a, b| {
        (
            a.t0_ms,
            highlight_kind_order(a.kind),
            a.clip_id.as_str(),
        )
            .cmp(&(b.t0_ms, highlight_kind_order(b.kind), b.clip_id.as_str()))
    });
}

fn highlight_kind_order(kind: HighlightKindV1) -> u8 {
    match kind {
        HighlightKindV1::DecisionCollapse => 0,
        HighlightKindV1::StructureBreak => 1,
        HighlightKindV1::PressureOverload => 2,
        HighlightKindV1::TransitionFailure => 3,
        HighlightKindV1::OverReliance => 4,
    }
}

fn build_highlights_from_dsa(result: &MatchResult, dsa: &DsaSummary) -> Vec<HighlightClipV1> {
    let mut highlights = Vec::new();

    // 1) Pressure overload: pick max pressure minute with an event signal.
    if let Some((minute, pressure)) = max_f32_index(&dsa.minute_series.pressure) {
        if pressure >= 0.65 {
            if let Some((t_ms, event_refs)) = pick_evidence_event_window(result, minute, decision_collapse_fallback_event_types()) {
                highlights.push(make_pressure_overload_clip(minute, t_ms, pressure, event_refs, 0.75));
            } else {
                highlights.push(make_pressure_overload_clip(minute, minute_to_ms(minute), pressure, Vec::new(), 0.68));
            }
        }
    }

    // 2) Transition failure: low transitions with moderate pressure.
    if let Some((minute, transitions)) = min_u32_index(&dsa.minute_series.transitions) {
        let pressure = dsa.minute_series.pressure.get(minute as usize).copied().unwrap_or(0.0);
        if transitions <= 1 && pressure >= 0.55 {
            let (t_ms, event_refs) = pick_evidence_event_window(result, minute, transition_failure_event_types())
                .unwrap_or((minute_to_ms(minute), Vec::new()));
            highlights.push(make_transition_failure_clip(minute, t_ms, pressure, transitions, event_refs, 0.72));
        }
    }

    // 3) Over-reliance: gini + top hub.
    if dsa.hub.gini >= 0.55 {
        if let Some(top) = dsa.hub.top3.first() {
            let focus_team = top.team.clone();
            let focus_minute = peak_pressure_against_team(dsa, &focus_team).unwrap_or(45);
            let (t_ms, event_refs) = pick_evidence_event_window(result, focus_minute, over_reliance_event_types())
                .unwrap_or((minute_to_ms(focus_minute), Vec::new()));
            highlights.push(make_over_reliance_clip(
                focus_minute,
                t_ms,
                dsa.hub.gini,
                top,
                event_refs,
                0.74,
            ));
        }
    }

    // 4) Structure break v1 fallback: qa warnings only.
    if dsa
        .qa_warnings
        .iter()
        .any(|w| matches!(w.kind, DsaQaWarningKind::LaneImbalanceHigh | DsaQaWarningKind::ZoneDwellSkewHigh))
    {
        let minute = max_f32_index(&dsa.minute_series.pressure).map(|(m, _)| m).unwrap_or(60);
        let (t_ms, event_refs) = pick_evidence_event_window(result, minute, structure_break_event_types())
            .unwrap_or((minute_to_ms(minute), Vec::new()));
        highlights.push(make_structure_break_clip(minute, t_ms, &dsa.qa_warnings, event_refs, 0.60));
    }

    // 5) Decision collapse v1 fallback: HighPressureLowTransitions QA warning.
    if dsa
        .qa_warnings
        .iter()
        .any(|w| matches!(w.kind, DsaQaWarningKind::HighPressureLowTransitions))
    {
        let minute = max_f32_index(&dsa.minute_series.pressure).map(|(m, _)| m).unwrap_or(60);
        let pressure = dsa.minute_series.pressure.get(minute as usize).copied().unwrap_or(0.0);
        let transitions = dsa.minute_series.transitions.get(minute as usize).copied().unwrap_or(0);
        let (t_ms, event_refs) = pick_evidence_event_window(result, minute, decision_collapse_fallback_event_types())
            .unwrap_or((minute_to_ms(minute), Vec::new()));
        highlights.push(make_decision_collapse_clip(minute, t_ms, pressure, transitions, event_refs, 0.55));
    }

    highlights
}

fn build_highlights_fallback(result: &MatchResult) -> Vec<HighlightClipV1> {
    // v1 fallback: pick one event-like window as PressureOverload with low confidence.
    let minute = 60;
    let (t_ms, event_refs) = pick_evidence_event_window(result, minute, decision_collapse_fallback_event_types())
        .unwrap_or((minute_to_ms(minute), Vec::new()));
    vec![make_pressure_overload_clip(minute, t_ms, 0.0, event_refs, 0.50)]
}

fn make_pressure_overload_clip(
    minute: u8,
    center_t_ms: u64,
    pressure: f32,
    primary_events: Vec<MatchEventRefV1>,
    confidence: f32,
) -> HighlightClipV1 {
    HighlightClipV1 {
        clip_id: format!("pressure_overload_{}", minute),
        kind: HighlightKindV1::PressureOverload,
        t0_ms: center_t_ms.saturating_sub(5_000),
        t1_ms: center_t_ms + 3_000,
        minute0: minute,
        minute1: minute,
        focus: None,
        evidence: HighlightEvidenceV1 {
            primary_events,
            action_results: Vec::new(),
            metrics: vec![
                EvidenceMetricV1 {
                    key: "pressure_avg".to_string(),
                    value: pressure,
                    baseline: None,
                    delta: None,
                },
                EvidenceMetricV1 {
                    key: "defenders_within_6m".to_string(),
                    value: if pressure <= 0.0 { 0.0 } else { 2.0 },
                    baseline: None,
                    delta: None,
                },
            ],
            snapshots: Vec::new(),
        },
        interpretation: HighlightInterpretationV1 {
            headline: "공을 잡을 시간이 없었다".to_string(),
            explanation: "2중 압박이 붙으면서 첫 터치 이후 선택 창이 닫혔다.".to_string(),
            confidence,
            blame_balance: BlameBalanceV1 {
                player: 0.2,
                structure: 0.5,
                opponent: 0.3,
            },
        },
        next_actions: None,
    }
}

fn make_transition_failure_clip(
    minute: u8,
    center_t_ms: u64,
    pressure: f32,
    transitions: u32,
    primary_events: Vec<MatchEventRefV1>,
    confidence: f32,
) -> HighlightClipV1 {
    HighlightClipV1 {
        clip_id: format!("transition_failure_{}", minute),
        kind: HighlightKindV1::TransitionFailure,
        t0_ms: center_t_ms.saturating_sub(5_000),
        t1_ms: center_t_ms + 3_000,
        minute0: minute,
        minute1: minute,
        focus: None,
        evidence: HighlightEvidenceV1 {
            primary_events,
            action_results: Vec::new(),
            metrics: vec![
                EvidenceMetricV1 {
                    key: "pressure_avg".to_string(),
                    value: pressure,
                    baseline: None,
                    delta: None,
                },
                EvidenceMetricV1 {
                    key: "transitions".to_string(),
                    value: transitions as f32,
                    baseline: None,
                    delta: None,
                },
            ],
            snapshots: Vec::new(),
        },
        interpretation: HighlightInterpretationV1 {
            headline: "전환에서 막혔다".to_string(),
            explanation: "공을 얻은 직후 전진 루트가 없어 다시 뒤로 흐름이 끊겼다.".to_string(),
            confidence,
            blame_balance: BlameBalanceV1 {
                player: 0.2,
                structure: 0.5,
                opponent: 0.3,
            },
        },
        next_actions: None,
    }
}

fn make_over_reliance_clip(
    minute: u8,
    center_t_ms: u64,
    gini: f32,
    top: &super::dsa_summary::DsaHubPlayer,
    primary_events: Vec<MatchEventRefV1>,
    confidence: f32,
) -> HighlightClipV1 {
    HighlightClipV1 {
        clip_id: format!("over_reliance_{}", minute),
        kind: HighlightKindV1::OverReliance,
        t0_ms: center_t_ms.saturating_sub(5_000),
        t1_ms: center_t_ms + 3_000,
        minute0: minute,
        minute1: minute,
        focus: Some(HighlightFocusV1 {
            team: top.team.clone(),
            track_id: Some(top.track_id),
            player_name: None,
            zone_id: None,
        }),
        evidence: HighlightEvidenceV1 {
            primary_events,
            action_results: Vec::new(),
            metrics: vec![
                EvidenceMetricV1 {
                    key: "gini".to_string(),
                    value: gini,
                    baseline: None,
                    delta: None,
                },
                EvidenceMetricV1 {
                    key: "hub_score_top1".to_string(),
                    value: top.hub_score,
                    baseline: None,
                    delta: None,
                },
            ],
            snapshots: Vec::new(),
        },
        interpretation: HighlightInterpretationV1 {
            headline: "상대가 한 사람만 잠갔다".to_string(),
            explanation: "공이 특정 선수에게 집중되며 패턴이 읽혔다.".to_string(),
            confidence,
            blame_balance: BlameBalanceV1 {
                player: 0.3,
                structure: 0.4,
                opponent: 0.3,
            },
        },
        next_actions: None,
    }
}

fn make_structure_break_clip(
    minute: u8,
    center_t_ms: u64,
    warnings: &[super::dsa_summary::DsaQaWarning],
    primary_events: Vec<MatchEventRefV1>,
    confidence: f32,
) -> HighlightClipV1 {
    let warning_kinds = warnings
        .iter()
        .map(|w| format!("{:?}", w.kind))
        .collect::<Vec<_>>()
        .join(", ");
    HighlightClipV1 {
        clip_id: format!("structure_break_{}", minute),
        kind: HighlightKindV1::StructureBreak,
        t0_ms: center_t_ms.saturating_sub(5_000),
        t1_ms: center_t_ms + 3_000,
        minute0: minute,
        minute1: minute,
        focus: None,
        evidence: HighlightEvidenceV1 {
            primary_events,
            action_results: Vec::new(),
            metrics: vec![
                EvidenceMetricV1 {
                    key: "qa_warnings".to_string(),
                    value: warnings.len() as f32,
                    baseline: None,
                    delta: None,
                },
                EvidenceMetricV1 {
                    key: "warning_kinds".to_string(),
                    value: warning_kinds.len() as f32,
                    baseline: None,
                    delta: None,
                },
            ],
            snapshots: Vec::new(),
        },
        interpretation: HighlightInterpretationV1 {
            headline: "라인이 갈라졌다".to_string(),
            explanation: "기준 위치 이탈이 누적되어 형태가 무너졌다.".to_string(),
            confidence,
            blame_balance: BlameBalanceV1 {
                player: 0.15,
                structure: 0.65,
                opponent: 0.2,
            },
        },
        next_actions: None,
    }
}

fn make_decision_collapse_clip(
    minute: u8,
    center_t_ms: u64,
    pressure: f32,
    transitions: u32,
    primary_events: Vec<MatchEventRefV1>,
    confidence: f32,
) -> HighlightClipV1 {
    HighlightClipV1 {
        clip_id: format!("decision_collapse_{}", minute),
        kind: HighlightKindV1::DecisionCollapse,
        t0_ms: center_t_ms.saturating_sub(5_000),
        t1_ms: center_t_ms + 3_000,
        minute0: minute,
        minute1: minute,
        focus: None,
        evidence: HighlightEvidenceV1 {
            primary_events,
            action_results: Vec::new(),
            metrics: vec![
                EvidenceMetricV1 {
                    key: "pressure_avg".to_string(),
                    value: pressure,
                    baseline: None,
                    delta: None,
                },
                EvidenceMetricV1 {
                    key: "transitions".to_string(),
                    value: transitions as f32,
                    baseline: None,
                    delta: None,
                },
            ],
            snapshots: Vec::new(),
        },
        interpretation: HighlightInterpretationV1 {
            headline: "선택지가 접혔다".to_string(),
            explanation: "압박/각도 때문에 가능한 옵션이 급감했다.".to_string(),
            confidence,
            blame_balance: BlameBalanceV1 {
                player: 0.2,
                structure: 0.5,
                opponent: 0.3,
            },
        },
        next_actions: None,
    }
}

fn minute_to_ms(minute: u8) -> u64 {
    minute as u64 * 60_000
}

fn max_f32_index(values: &[f32]) -> Option<(u8, f32)> {
    let mut best_i: Option<usize> = None;
    let mut best_v: f32 = f32::MIN;
    for (i, &v) in values.iter().enumerate() {
        if v.is_finite() && v > best_v {
            best_v = v;
            best_i = Some(i);
        }
    }
    Some((best_i? as u8, best_v))
}

fn min_u32_index(values: &[u32]) -> Option<(u8, u32)> {
    let mut best_i: Option<usize> = None;
    let mut best_v: u32 = u32::MAX;
    for (i, &v) in values.iter().enumerate() {
        if v < best_v {
            best_v = v;
            best_i = Some(i);
        }
    }
    Some((best_i? as u8, best_v))
}

fn peak_pressure_against_team(dsa: &DsaSummary, team: &str) -> Option<u8> {
    let series = match team {
        "home" => &dsa.minute_series.pressure_against_home,
        "away" => &dsa.minute_series.pressure_against_away,
        _ => return None,
    };
    max_f32_index(series).map(|(m, _)| m)
}

fn pick_evidence_event_window(
    result: &MatchResult,
    minute: u8,
    allowed: &[EventType],
) -> Option<(u64, Vec<MatchEventRefV1>)> {
    let mut refs: Vec<MatchEventRefV1> = Vec::new();
    let mut best_t_ms: Option<u64> = None;
    for ev in result.events.iter() {
        if ev.minute != minute {
            continue;
        }
        if !allowed.contains(&ev.event_type) {
            continue;
        }
        let t_ms = ev.timestamp_ms.unwrap_or_else(|| minute_to_ms(minute));
        best_t_ms = Some(best_t_ms.map(|cur| cur.min(t_ms)).unwrap_or(t_ms));
        refs.push(MatchEventRefV1 {
            t_ms,
            event_type: ev.event_type.clone(),
            is_home_team: ev.is_home_team,
            player_track_id: ev.player_track_id,
        });
    }

    let t_ms = best_t_ms?;
    refs.sort_by(|a, b| (a.t_ms, a.is_home_team, a.player_track_id).cmp(&(b.t_ms, b.is_home_team, b.player_track_id)));
    Some((t_ms, refs))
}

fn decision_collapse_fallback_event_types() -> &'static [EventType] {
    &[
        EventType::Tackle,
        EventType::Foul,
        EventType::Offside,
        EventType::ShotBlocked,
        EventType::ShotOffTarget,
        EventType::ThrowIn,
        EventType::GoalKick,
        EventType::Corner,
    ]
}

fn transition_failure_event_types() -> &'static [EventType] {
    &[
        EventType::Shot,
        EventType::ShotOnTarget,
        EventType::ShotBlocked,
        EventType::ShotOffTarget,
        EventType::KeyChance,
    ]
}

fn over_reliance_event_types() -> &'static [EventType] {
    &[EventType::Pass, EventType::Dribble, EventType::Tackle, EventType::Foul]
}

fn structure_break_event_types() -> &'static [EventType] {
    &[EventType::Pass, EventType::Shot, EventType::Foul, EventType::Offside]
}
