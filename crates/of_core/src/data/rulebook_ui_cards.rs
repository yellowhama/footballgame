//! RuleBook UI Card System (Why Button)
//!
//! Produces a structured, deterministic payload for UI rendering.
//! This is intentionally **contract-first**:
//! - UI renders `cards` only (no client-side sentence assembly).
//! - `raw_payload` is lossless snapshot for replay/analytics deep-links.
//!
//! SSOT: `docs/specs/fix_2601/1120/RULEBOOK_SYSTEM_VNEXT_SPEC.md` (Section: UI Card JSON Schema)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use crate::models::events::{EventType, MatchEvent};
use crate::models::rules::{FoulSeverity, OffsideInvolvementType, RuleId};
use crate::models::EventDetails;

// =============================================================================
// UI Payload Types (Schema v1.0)
// =============================================================================

pub const RULEBOOK_UI_CARD_SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookUiCard {
    pub schema_version: String,
    pub lang: String, // "ko" | "en"
    pub event: RulebookUiEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<RulebookUiRule>,
    pub cards: Vec<CardBlock>,
    pub raw_payload: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookUiEvent {
    pub event_type: String,
    pub timestamp_ms: u64,
    pub minute: u64,
    pub team_side: String, // "home" | "away" | "unknown"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookUiRule {
    pub rule_id: String,
    pub law_number: u64,
    pub law_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub law_name_en: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardBlock {
    pub level: u64, // 1|2|3
    pub title: String,
    pub lines: Vec<CardLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardLine {
    pub kind: String, // plain|bullet|kv|warning|note
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<CardRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardRef {
    pub r#type: String, // player_track_id|pitch_x_m|pitch_line|event_id
    pub id: String,
}

// =============================================================================
// Public API
// =============================================================================

/// Generate a RuleBook UI card for an event type and optional details.
///
/// This produces a schema-valid payload but uses `timestamp_ms=0`, `minute=0`, `team_side=unknown`.
/// Prefer `generate_ui_card_from_match_event()` when event context is available.
pub fn generate_ui_card(
    event_type: &EventType,
    details: Option<&EventDetails>,
    use_korean: bool,
) -> Option<RulebookUiCard> {
    let mut cards: Vec<CardBlock> = Vec::new();

    match event_type {
        EventType::Offside => generate_offside_cards(&mut cards, details, use_korean),
        EventType::Foul | EventType::YellowCard | EventType::RedCard => {
            generate_foul_cards(&mut cards, details, use_korean)
        }
        EventType::Goal | EventType::OwnGoal => generate_goal_cards(&mut cards, event_type, use_korean),
        EventType::ThrowIn | EventType::GoalKick | EventType::Corner => {
            generate_restart_cards(&mut cards, event_type, use_korean)
        }
        EventType::Freekick => generate_freekick_cards(&mut cards, details, use_korean),
        EventType::Penalty => generate_penalty_cards(&mut cards, use_korean),
        EventType::PostHit | EventType::BarHit => generate_woodwork_cards(&mut cards, event_type, use_korean),
        EventType::VarReview => generate_var_cards(&mut cards, details, use_korean),
        _ => return None,
    }

    let rule_id = details.and_then(|d| d.rule_id).or_else(|| RuleId::from_event_type(event_type));
    Some(build_payload(
        event_type,
        rule_id,
        cards,
        build_raw_payload(details),
        use_korean,
        0,
        0,
        "unknown",
    ))
}

/// Generate a RuleBook UI card from a full `MatchEvent` (preferred).
pub fn generate_ui_card_from_match_event(event: &MatchEvent, use_korean: bool) -> Option<RulebookUiCard> {
    let card = generate_ui_card(&event.event_type, event.details.as_ref(), use_korean)?;

    let team_side = if event.is_home_team { "home" } else { "away" };
    Some(RulebookUiCard {
        event: RulebookUiEvent {
            event_type: format!("{:?}", event.event_type),
            timestamp_ms: event.timestamp_ms.unwrap_or(0),
            minute: event.minute as u64,
            team_side: team_side.to_string(),
        },
        raw_payload: serde_json::to_value(event).unwrap_or_else(|_| json!({})),
        ..card
    })
}

// =============================================================================
// Payload Builder
// =============================================================================

fn build_payload(
    event_type: &EventType,
    rule_id: Option<RuleId>,
    cards: Vec<CardBlock>,
    raw_payload: JsonValue,
    use_korean: bool,
    timestamp_ms: u64,
    minute: u64,
    team_side: &str,
) -> RulebookUiCard {
    let rule_id_string = |r: RuleId| {
        serde_json::to_value(r)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("{:?}", r))
    };

    let rule = rule_id.map(|r| RulebookUiRule {
        rule_id: rule_id_string(r),
        law_number: r.law_number() as u64,
        law_name: if use_korean { r.name_ko().to_string() } else { r.name_en().to_string() },
        law_name_en: if use_korean { Some(r.name_en().to_string()) } else { None },
    });

    RulebookUiCard {
        schema_version: RULEBOOK_UI_CARD_SCHEMA_VERSION.to_string(),
        lang: if use_korean { "ko".to_string() } else { "en".to_string() },
        event: RulebookUiEvent {
            event_type: format!("{:?}", event_type),
            timestamp_ms,
            minute,
            team_side: team_side.to_string(),
        },
        rule,
        cards,
        raw_payload,
    }
}

fn build_raw_payload(details: Option<&EventDetails>) -> JsonValue {
    match details {
        Some(d) => serde_json::to_value(d).unwrap_or_else(|_| json!({})),
        None => json!({}),
    }
}

// =============================================================================
// Line helpers
// =============================================================================

fn line_plain(text: impl Into<String>) -> CardLine {
    CardLine { kind: "plain".to_string(), text: text.into(), key: None, value: None, r#ref: None }
}

fn line_bullet(text: impl Into<String>) -> CardLine {
    CardLine { kind: "bullet".to_string(), text: text.into(), key: None, value: None, r#ref: None }
}

fn line_note(text: impl Into<String>) -> CardLine {
    CardLine { kind: "note".to_string(), text: text.into(), key: None, value: None, r#ref: None }
}

fn line_warning(text: impl Into<String>) -> CardLine {
    CardLine { kind: "warning".to_string(), text: text.into(), key: None, value: None, r#ref: None }
}

fn line_kv(key: impl Into<String>, value: JsonValue) -> CardLine {
    let key_s = key.into();
    let text = match &value {
        JsonValue::String(s) => format!("{key_s}: {s}"),
        JsonValue::Number(n) => format!("{key_s}: {n}"),
        JsonValue::Bool(b) => format!("{key_s}: {b}"),
        JsonValue::Null => format!("{key_s}: -"),
        other => format!("{key_s}: {other}"),
    };
    CardLine { kind: "kv".to_string(), text, key: Some(key_s), value: Some(value), r#ref: None }
}

fn with_ref(mut line: CardLine, r#type: &str, id: impl Into<String>) -> CardLine {
    line.r#ref = Some(CardRef { r#type: r#type.to_string(), id: id.into() });
    line
}

// =============================================================================
// Standard cards (P0.5)
// =============================================================================

fn generate_offside_cards(cards: &mut Vec<CardBlock>, details: Option<&EventDetails>, use_korean: bool) {
    // L1 (always)
    cards.push(CardBlock {
        level: 1,
        title: if use_korean { "오프사이드" } else { "Offside" }.to_string(),
        lines: vec![line_plain(if use_korean {
            "공이 플레이되는 순간 공격 선수가 오프사이드 위치에 있었습니다."
        } else {
            "At the moment the ball was played, the attacker was in an offside position."
        })],
    });

    // L2 (details if available; otherwise key points)
    let mut lines: Vec<CardLine> = Vec::new();
    if let Some(d) = details {
        if let Some(ref offside) = d.offside_details {
            lines.push(line_kv(
                if use_korean { "마진(m)" } else { "Margin (m)" },
                json!(format!("{:.2}", offside.margin_m)),
            ));
            lines.push(with_ref(
                line_kv(
                    if use_korean { "기준선 X(m)" } else { "Reference X (m)" },
                    json!(format!("{:.1}", offside.offside_line_m)),
                ),
                "pitch_x_m",
                format!("{:.1}", offside.offside_line_m),
            ));

            if let Some(passer) = offside.passer_track_id {
                lines.push(with_ref(
                    line_kv(
                        if use_korean { "패스한 선수" } else { "Passer" },
                        json!(passer),
                    ),
                    "player_track_id",
                    passer.to_string(),
                ));
            }

            if let Some(ref involvement) = offside.involvement_type {
                let t = match involvement {
                    OffsideInvolvementType::InterferingWithPlay => {
                        if use_korean { "플레이 관여" } else { "Interfering with play" }
                    }
                    OffsideInvolvementType::InterferingWithOpponent => {
                        if use_korean { "상대방 방해" } else { "Interfering with opponent" }
                    }
                    OffsideInvolvementType::GainingAdvantage => {
                        if use_korean { "이익 획득" } else { "Gaining advantage" }
                    }
                };
                lines.push(line_kv(if use_korean { "관여 유형" } else { "Involvement" }, json!(t)));
            }

            if let Some(ref restart_ctx) = offside.restart_context {
                if restart_ctx.offside_exception_applies {
                    lines.push(line_note(if use_korean {
                        "예외: 골킥/스로인/코너킥에서 직접 받은 경우 오프사이드가 적용되지 않습니다."
                    } else {
                        "Exception: No offside if received directly from goal kick / throw-in / corner."
                    }));
                }
            }

            if let Some(ref deflection_ctx) = offside.deflection_context {
                // Keep this deterministic and human-friendly.
                let t = if deflection_ctx.resets_offside {
                    if use_korean { "수비수 deliberate play → 오프사이드 리셋" } else { "Defender deliberate play → offside reset" }
                } else {
                    if use_korean { "deflection/save → 오프사이드 유지" } else { "Deflection/save → offside stays" }
                };
                lines.push(line_kv(if use_korean { "수비수 터치" } else { "Defender touch" }, json!(t)));
            }
        }
    }

    if lines.is_empty() {
        lines = if use_korean {
            vec![
                line_bullet("위치: 공과 두 번째 수비수보다 앞"),
                line_bullet("관여: 플레이 관여/상대 방해/이익 획득"),
                line_bullet("예외: 골킥/스로인/코너킥"),
            ]
        } else {
            vec![
                line_bullet("Position: beyond ball and second-last opponent"),
                line_bullet("Involvement: play/opponent/advantage"),
                line_bullet("Exceptions: goal kick / throw-in / corner"),
            ]
        };
    }

    cards.push(CardBlock {
        level: 2,
        title: if use_korean { "핵심 근거" } else { "Key Points" }.to_string(),
        lines,
    });

    // L3 (always)
    cards.push(CardBlock {
        level: 3,
        title: if use_korean { "규칙 참조" } else { "Rule Reference" }.to_string(),
        lines: vec![line_plain(if use_korean {
            "IFAB Law 11: 공격 선수가 공과 두 번째 수비수보다 상대 골라인에 더 가까우면 오프사이드 위치입니다."
        } else {
            "IFAB Law 11: A player is in an offside position if nearer to the opponent's goal line than both the ball and the second-last opponent."
        })],
    });
}

fn generate_foul_cards(cards: &mut Vec<CardBlock>, details: Option<&EventDetails>, use_korean: bool) {
    // L1
    cards.push(CardBlock {
        level: 1,
        title: if use_korean { "파울" } else { "Foul" }.to_string(),
        lines: vec![line_plain(if use_korean {
            "상대 선수에게 부당한 접촉이 있어 파울로 판정했습니다."
        } else {
            "Unfair contact with an opponent was penalized as a foul."
        })],
    });

    // L2
    let mut lines: Vec<CardLine> = Vec::new();
    if let Some(d) = details {
        if let Some(ref foul) = d.foul_details {
            let severity = match foul.severity {
                FoulSeverity::Careless => if use_korean { "부주의" } else { "Careless" },
                FoulSeverity::Reckless => if use_korean { "무모함(경고 가능)" } else { "Reckless (caution possible)" },
                FoulSeverity::ExcessiveForce => if use_korean { "과도한 힘(퇴장 가능)" } else { "Excessive force (sending-off possible)" },
            };
            lines.push(line_kv(if use_korean { "심각도" } else { "Severity" }, json!(severity)));

            if let Some(ref foul_type) = foul.foul_type {
                let t = if use_korean { foul_type.name_ko() } else { foul_type.name_en() };
                lines.push(line_kv(if use_korean { "파울 유형" } else { "Foul type" }, json!(t)));
            }

            lines.push(line_kv(
                if use_korean { "위치" } else { "Location" },
                json!(if foul.in_penalty_area {
                    if use_korean { "페널티 에어리어 내" } else { "Inside penalty area" }
                } else {
                    if use_korean { "페널티 에어리어 밖" } else { "Outside penalty area" }
                }),
            ));

            if let Some(victim) = foul.victim_track_id {
                lines.push(with_ref(
                    line_kv(if use_korean { "피해 선수" } else { "Victim" }, json!(victim)),
                    "player_track_id",
                    victim.to_string(),
                ));
            }

            if foul.is_dogso {
                lines.push(line_warning(if use_korean {
                    "명백한 득점 기회 저지(DOGSO) 가능성이 있습니다."
                } else {
                    "Possible DOGSO (denying an obvious goal-scoring opportunity)."
                }));
            }
        }
    }

    if lines.is_empty() {
        lines = if use_korean {
            vec![
                line_bullet("직접/간접 프리킥 여부는 파울 유형에 따라 달라집니다."),
                line_bullet("심각도(careless/reckless/excessive force)에 따라 카드가 달라집니다."),
            ]
        } else {
            vec![
                line_bullet("Direct/indirect free kick depends on offence type."),
                line_bullet("Severity (careless/reckless/excessive force) affects sanction."),
            ]
        };
    }

    cards.push(CardBlock {
        level: 2,
        title: if use_korean { "핵심 근거" } else { "Key Points" }.to_string(),
        lines,
    });

    // L3
    cards.push(CardBlock {
        level: 3,
        title: if use_korean { "규칙 참조" } else { "Rule Reference" }.to_string(),
        lines: vec![line_plain(if use_korean {
            "IFAB Law 12: 부주의한/무모한/과도한 힘을 사용한 태클이나 챌린지는 파울입니다."
        } else {
            "IFAB Law 12: A tackle or challenge that is careless, reckless, or using excessive force is a foul."
        })],
    });
}

fn generate_goal_cards(cards: &mut Vec<CardBlock>, event_type: &EventType, use_korean: bool) {
    let is_own_goal = matches!(event_type, EventType::OwnGoal);

    cards.push(CardBlock {
        level: 1,
        title: if use_korean { "골" } else { "Goal" }.to_string(),
        lines: vec![line_plain(if is_own_goal {
            if use_korean {
                "자책골로 기록됩니다. 공이 골라인을 완전히 통과했습니다."
            } else {
                "Recorded as an own goal. The ball wholly crossed the goal line."
            }
        } else {
            if use_korean {
                "공이 골라인을 완전히 통과해 골로 인정됩니다."
            } else {
                "The ball wholly crossed the goal line and a goal is awarded."
            }
        })],
    });

    let lines = if use_korean {
        vec![
            line_bullet("공 전체가 골라인을 완전히 통과"),
            line_bullet("골대 사이, 크로스바 아래"),
            line_bullet("직전에 오프사이드/파울이 있으면 골이 취소될 수 있음"),
        ]
    } else {
        vec![
            line_bullet("Ball wholly crossed the goal line"),
            line_bullet("Between the posts and under the crossbar"),
            line_bullet("Goal may be disallowed if preceded by offside/foul"),
        ]
    };

    cards.push(CardBlock {
        level: 2,
        title: if use_korean { "핵심 근거" } else { "Key Points" }.to_string(),
        lines,
    });

    cards.push(CardBlock {
        level: 3,
        title: if use_korean { "규칙 참조" } else { "Rule Reference" }.to_string(),
        lines: vec![line_plain(if use_korean {
            "IFAB Law 10: 공 전체가 골대 사이와 크로스바 아래 골라인을 통과하면 골입니다."
        } else {
            "IFAB Law 10: A goal is scored when the whole ball passes over the goal line between the posts and under the crossbar."
        })],
    });
}

fn generate_restart_cards(cards: &mut Vec<CardBlock>, event_type: &EventType, use_korean: bool) {
    let (title, summary, condition, law_text) = match event_type {
        EventType::ThrowIn => (
            if use_korean { "스로인" } else { "Throw-in" },
            if use_korean { "공이 터치라인을 완전히 넘어갔습니다." } else { "Ball wholly crossed the touchline." },
            if use_korean { "마지막 터치: 상대팀" } else { "Last touch: opposing team" },
            if use_korean { "IFAB Law 15: 공이 터치라인을 완전히 통과하면 스로인으로 재시작합니다." } else { "IFAB Law 15: A throw-in is awarded when the ball wholly crosses the touchline." },
        ),
        EventType::GoalKick => (
            if use_korean { "골킥" } else { "Goal kick" },
            if use_korean { "공이 골라인을 완전히 넘어갔습니다." } else { "Ball wholly crossed the goal line." },
            if use_korean { "마지막 터치: 공격팀" } else { "Last touch: attacking team" },
            if use_korean { "IFAB Law 16: 공격팀이 마지막으로 터치한 뒤 골라인을 넘으면 골킥입니다." } else { "IFAB Law 16: A goal kick is awarded when last touched by the attackers." },
        ),
        EventType::Corner => (
            if use_korean { "코너킥" } else { "Corner kick" },
            if use_korean { "공이 골라인을 완전히 넘어갔습니다." } else { "Ball wholly crossed the goal line." },
            if use_korean { "마지막 터치: 수비팀" } else { "Last touch: defending team" },
            if use_korean { "IFAB Law 17: 수비팀이 마지막으로 터치한 뒤 골라인을 넘으면 코너킥입니다." } else { "IFAB Law 17: A corner kick is awarded when last touched by the defenders." },
        ),
        _ => return,
    };

    cards.push(CardBlock {
        level: 1,
        title: title.to_string(),
        lines: vec![line_plain(summary)],
    });

    let mut lines = vec![line_bullet(condition)];
    lines.push(line_note(if use_korean {
        "오프사이드 예외: 골킥/스로인/코너킥에서 직접 받은 경우 오프사이드가 적용되지 않습니다."
    } else {
        "Offside exception: No offside if received directly from a goal kick/throw-in/corner."
    }));

    cards.push(CardBlock {
        level: 2,
        title: if use_korean { "핵심 근거" } else { "Key Points" }.to_string(),
        lines,
    });

    cards.push(CardBlock {
        level: 3,
        title: if use_korean { "규칙 참조" } else { "Rule Reference" }.to_string(),
        lines: vec![line_plain(law_text)],
    });
}

fn generate_freekick_cards(cards: &mut Vec<CardBlock>, details: Option<&EventDetails>, use_korean: bool) {
    cards.push(CardBlock {
        level: 1,
        title: if use_korean { "프리킥" } else { "Free Kick" }.to_string(),
        lines: vec![line_plain(if use_korean {
            "파울에 대한 재시작으로 프리킥이 선언되었습니다."
        } else {
            "A free kick was awarded as a restart after a foul."
        })],
    });

    let mut lines: Vec<CardLine> = Vec::new();
    if let Some(d) = details {
        if let Some(rule_id) = d.rule_id {
            let t = match rule_id {
                RuleId::DirectFreeKick => if use_korean { "직접 프리킥" } else { "Direct free kick" },
                RuleId::IndirectFreeKick => if use_korean { "간접 프리킥" } else { "Indirect free kick" },
                _ => if use_korean { "프리킥" } else { "Free kick" },
            };
            lines.push(line_kv(if use_korean { "유형" } else { "Type" }, json!(t)));
        }
    }
    lines.push(line_bullet(if use_korean { "상대 선수 9.15m 이상 거리 유지" } else { "Opponents must be at least 9.15m away" }));

    cards.push(CardBlock {
        level: 2,
        title: if use_korean { "핵심 근거" } else { "Key Points" }.to_string(),
        lines,
    });

    cards.push(CardBlock {
        level: 3,
        title: if use_korean { "규칙 참조" } else { "Rule Reference" }.to_string(),
        lines: vec![line_plain(if use_korean { "IFAB Law 13: 프리킥 재시작 규칙이 적용됩니다." } else { "IFAB Law 13: Free kick restart rules apply." })],
    });
}

fn generate_penalty_cards(cards: &mut Vec<CardBlock>, use_korean: bool) {
    cards.push(CardBlock {
        level: 1,
        title: if use_korean { "페널티킥" } else { "Penalty Kick" }.to_string(),
        lines: vec![line_plain(if use_korean {
            "페널티 에어리어 내에서 직접 프리킥에 해당하는 파울이 발생해 페널티킥이 선언되었습니다."
        } else {
            "A direct free kick offence occurred inside the penalty area, so a penalty kick was awarded."
        })],
    });

    let lines = if use_korean {
        vec![
            line_bullet("페널티 마크에서 킥"),
            line_bullet("골키퍼는 골라인 위에 위치"),
            line_bullet("다른 선수는 페널티 에어리어 밖"),
        ]
    } else {
        vec![
            line_bullet("Kick from the penalty mark"),
            line_bullet("Goalkeeper on the goal line"),
            line_bullet("Other players outside the penalty area"),
        ]
    };
    cards.push(CardBlock {
        level: 2,
        title: if use_korean { "핵심 근거" } else { "Key Points" }.to_string(),
        lines,
    });

    cards.push(CardBlock {
        level: 3,
        title: if use_korean { "규칙 참조" } else { "Rule Reference" }.to_string(),
        lines: vec![line_plain(if use_korean { "IFAB Law 14: 페널티킥 재시작 규칙이 적용됩니다." } else { "IFAB Law 14: Penalty kick restart rules apply." })],
    });
}

fn generate_woodwork_cards(cards: &mut Vec<CardBlock>, event_type: &EventType, use_korean: bool) {
    let title = match event_type {
        EventType::PostHit => if use_korean { "골포스트" } else { "Goalpost" },
        EventType::BarHit => if use_korean { "크로스바" } else { "Crossbar" },
        _ => if use_korean { "골대" } else { "Goal frame" },
    };
    let summary = match event_type {
        EventType::PostHit => if use_korean { "슈팅이 골포스트에 맞았습니다." } else { "Shot hit the goalpost." },
        EventType::BarHit => if use_korean { "슈팅이 크로스바에 맞았습니다." } else { "Shot hit the crossbar." },
        _ => if use_korean { "슈팅이 골대에 맞았습니다." } else { "Shot hit the goal frame." },
    };

    cards.push(CardBlock { level: 1, title: title.to_string(), lines: vec![line_plain(summary)] });
    cards.push(CardBlock {
        level: 2,
        title: if use_korean { "핵심 근거" } else { "Key Points" }.to_string(),
        lines: vec![line_bullet(if use_korean { "공이 골라인을 완전히 넘지 않았습니다." } else { "The ball did not wholly cross the goal line." })],
    });
    cards.push(CardBlock {
        level: 3,
        title: if use_korean { "규칙 참조" } else { "Rule Reference" }.to_string(),
        lines: vec![line_plain(if use_korean { "IFAB Law 10: 공 전체가 골라인을 넘지 않으면 골이 아닙니다." } else { "IFAB Law 10: No goal unless the whole ball crosses the goal line." })],
    });
}

fn generate_var_cards(cards: &mut Vec<CardBlock>, details: Option<&EventDetails>, use_korean: bool) {
    cards.push(CardBlock {
        level: 1,
        title: if use_korean { "VAR" } else { "VAR" }.to_string(),
        lines: vec![line_plain(if use_korean {
            "VAR(비디오 판독)이 진행되었습니다."
        } else {
            "A VAR (video) review occurred."
        })],
    });

    let mut lines: Vec<CardLine> = Vec::new();
    if let Some(d) = details {
        if let Some(ref var) = d.var_review {
            lines.push(line_kv(
                if use_korean { "대상 이벤트" } else { "Reviewed event" },
                json!(format!("{:?}", var.reviewed_event_type)),
            ));
            lines.push(line_kv(
                if use_korean { "결과" } else { "Outcome" },
                json!(format!("{:?}", var.outcome)),
            ));
        }
    }
    if lines.is_empty() {
        lines.push(line_note(if use_korean {
            "현재 버전에서는 VAR이 판정을 번복하지 않습니다."
        } else {
            "In the current version, VAR does not overturn decisions."
        }));
    }

    cards.push(CardBlock {
        level: 2,
        title: if use_korean { "핵심 근거" } else { "Key Points" }.to_string(),
        lines,
    });

    cards.push(CardBlock {
        level: 3,
        title: if use_korean { "규칙 참조" } else { "Rule Reference" }.to_string(),
        lines: vec![line_plain(if use_korean {
            "VAR은 판정의 정확성을 돕기 위한 보조 수단입니다."
        } else {
            "VAR is a tool to assist the accuracy of match decisions."
        })],
    });
}

// =============================================================================
// Tests (schema-focused sanity)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::rules::{
        DeflectionContext, DefenderTouchType, FoulDetails, FoulType, OffsideDetails,
        OffsideRestartContext, RestartType,
    };

    #[test]
    fn test_generate_offside_card_has_3_levels() {
        let details = EventDetails {
            offside_details: Some(OffsideDetails {
                margin_m: 0.35,
                offside_line_m: 75.0,
                passer_track_id: Some(8),
                involvement_type: Some(OffsideInvolvementType::InterferingWithPlay),
                restart_context: Some(OffsideRestartContext {
                    restart_type: RestartType::Normal,
                    offside_exception_applies: false,
                }),
                touch_reference: None,
                deflection_context: Some(DeflectionContext {
                    last_touch_by_defender: DefenderTouchType::None,
                    resets_offside: false,
                }),
            }),
            rule_id: Some(RuleId::OffsidePosition),
            ..Default::default()
        };

        let card = generate_ui_card(&EventType::Offside, Some(&details), true).unwrap();
        assert_eq!(card.schema_version, "1.0");
        assert_eq!(card.lang, "ko");
        assert!(card.rule.is_some());
        assert_eq!(card.cards.len(), 3);
        assert_eq!(card.cards[0].level, 1);
        assert_eq!(card.cards[1].level, 2);
        assert_eq!(card.cards[2].level, 3);
    }

    #[test]
    fn test_generate_restart_card_has_3_levels() {
        let card = generate_ui_card(&EventType::ThrowIn, None, false).unwrap();
        assert_eq!(card.cards.len(), 3);
        assert_eq!(card.cards[0].level, 1);
        assert_eq!(card.cards[1].level, 2);
        assert_eq!(card.cards[2].level, 3);
    }

    #[test]
    fn test_generate_foul_card_includes_victim_ref_when_available() {
        let details = EventDetails {
            foul_details: Some(FoulDetails {
                severity: FoulSeverity::Reckless,
                foul_type: Some(FoulType::Tackling),
                is_dogso: false,
                in_penalty_area: false,
                victim_track_id: Some(10),
                attempted_to_play_ball: true,
            }),
            rule_id: Some(RuleId::FoulReckless),
            ..Default::default()
        };

        let card = generate_ui_card(&EventType::Foul, Some(&details), true).unwrap();
        let victim_line = card.cards[1].lines.iter().find(|l| l.text.contains("피해 선수"));
        assert!(victim_line.is_some());
        assert_eq!(
            victim_line.unwrap().r#ref.as_ref().map(|r| r.r#type.as_str()),
            Some("player_track_id")
        );
    }
}
