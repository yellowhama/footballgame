//! IFAB Laws of the Game - Rule Data Loading
//!
//! YAML 파일에서 규칙 데이터를 로드하고 캐싱합니다.
//!
//! ## 사용법
//!
//! ```rust
//! use of_core::data::rules::{get_offside_rules, get_fouls_rules};
//!
//! let offside = get_offside_rules();
//! println!("Law {}: {}", offside.law_number, offside.name);
//!
//! let fouls = get_fouls_rules();
//! println!("Law {}: {}", fouls.law_number, fouls.name);
//! ```

use std::sync::OnceLock;

use crate::models::rules::{
    DefenderTouchType, FoulsRuleData, OffsideRuleData, RestartType,
};

// =============================================================================
// Embedded YAML Data
// =============================================================================

/// Law 11: Offside YAML 데이터 (컴파일 타임 임베딩)
pub const LAW_11_YAML: &str = include_str!("../../../../data/rules/law_11_offside.yaml");

/// Law 12: Fouls and Misconduct YAML 데이터 (컴파일 타임 임베딩)
pub const LAW_12_YAML: &str = include_str!("../../../../data/rules/law_12_fouls.yaml");

/// Law 9/10/13-17: Ball In/Out, Goal, Restarts YAML 데이터
pub const LAW_09_10_13_17_YAML: &str =
    include_str!("../../../../data/rules/law_09_10_13_17.yaml");

/// EventType → RuleId 매핑 YAML 데이터
pub const EVENT_TO_RULE_MAP_YAML: &str =
    include_str!("../../../../data/rules/event_to_rule_map.yaml");

/// 확장 설명 템플릿 YAML 데이터
pub const EXPLAIN_TEMPLATES_YAML: &str =
    include_str!("../../../../data/rules/explain_templates_extended.yaml");

// =============================================================================
// Static Caching
// =============================================================================

static OFFSIDE_RULES: OnceLock<OffsideRuleData> = OnceLock::new();
static FOULS_RULES: OnceLock<FoulsRuleData> = OnceLock::new();

// =============================================================================
// Public API
// =============================================================================

/// Law 11 (Offside) 규칙 데이터 로드
///
/// 최초 호출 시 YAML 파싱, 이후 캐시된 데이터 반환.
///
/// # Panics
///
/// YAML 파싱에 실패하면 패닉합니다 (컴파일 타임에 임베딩된 데이터이므로
/// 정상적인 빌드에서는 발생하지 않음).
pub fn get_offside_rules() -> &'static OffsideRuleData {
    OFFSIDE_RULES.get_or_init(|| {
        serde_yaml::from_str(LAW_11_YAML).expect("Failed to parse law_11_offside.yaml")
    })
}

/// Law 12 (Fouls and Misconduct) 규칙 데이터 로드
///
/// 최초 호출 시 YAML 파싱, 이후 캐시된 데이터 반환.
///
/// # Panics
///
/// YAML 파싱에 실패하면 패닉합니다.
pub fn get_fouls_rules() -> &'static FoulsRuleData {
    FOULS_RULES.get_or_init(|| {
        serde_yaml::from_str(LAW_12_YAML).expect("Failed to parse law_12_fouls.yaml")
    })
}

// =============================================================================
// Utility Functions
// =============================================================================

/// 오프사이드 설명 생성 (기본 템플릿)
///
/// # Arguments
///
/// * `player_name` - 오프사이드 선수 이름
/// * `margin_m` - 오프사이드 마진 (미터)
/// * `use_korean` - true: 한국어, false: 영어
pub fn format_offside_explanation(player_name: &str, margin_m: f32, use_korean: bool) -> String {
    let rules = get_offside_rules();

    if let Some(ref templates) = rules.explanation_templates {
        if let Some(ref basic) = templates.basic {
            let template = if use_korean {
                &basic.template
            } else {
                &basic.template_en
            };

            return template
                .replace("{player_name}", player_name)
                .replace("{margin_m:.2f}", &format!("{:.2}", margin_m));
        }
    }

    // Fallback
    if use_korean {
        format!(
            "{}이(가) 오프사이드 위치에서 플레이에 관여 (마진: {:.2}m)",
            player_name, margin_m
        )
    } else {
        format!(
            "{} was offside by {:.2}m",
            player_name, margin_m
        )
    }
}

/// 파울 설명 생성 (기본 템플릿)
///
/// # Arguments
///
/// * `player_name` - 파울 선수 이름
/// * `victim_name` - 피해 선수 이름
/// * `foul_type` - 파울 유형
/// * `use_korean` - true: 한국어, false: 영어
pub fn format_foul_explanation(
    player_name: &str,
    victim_name: &str,
    foul_type: &crate::models::rules::FoulType,
    use_korean: bool,
) -> String {
    let offence_type = if use_korean {
        foul_type.name_ko()
    } else {
        foul_type.name_en()
    };

    if use_korean {
        format!(
            "{}이(가) {}에게 {} 파울",
            player_name, victim_name, offence_type
        )
    } else {
        format!(
            "{} committed a {} foul on {}",
            player_name, offence_type, victim_name
        )
    }
}

/// 오프사이드 예외 설명 생성 (Law 11 예외)
///
/// 골킥, 스로인, 코너킥에서 직접 받은 경우 오프사이드 예외 설명.
///
/// # Arguments
///
/// * `restart_type` - 재시작 유형
/// * `use_korean` - true: 한국어, false: 영어
pub fn format_offside_exception_explanation(
    restart_type: &RestartType,
    use_korean: bool,
) -> Option<String> {
    if !restart_type.is_offside_exception() {
        return None;
    }

    let restart_name = if use_korean {
        restart_type.name_ko()
    } else {
        restart_type.name_en()
    };

    Some(if use_korean {
        format!(
            "이 상황은 **{}에서 공을 직접 받은 경우**라 오프사이드가 적용되지 않습니다.",
            restart_name
        )
    } else {
        format!(
            "No offside - the ball was received directly from a **{}**.",
            restart_name.to_lowercase()
        )
    })
}

/// 수비수 deliberate play/deflection/save 설명 생성
///
/// # Arguments
///
/// * `defender_touch` - 수비수 터치 유형
/// * `use_korean` - true: 한국어, false: 영어
pub fn format_deflection_explanation(
    defender_touch: &DefenderTouchType,
    use_korean: bool,
) -> Option<String> {
    match defender_touch {
        DefenderTouchType::None => None,
        DefenderTouchType::DeliberatePlay => Some(if use_korean {
            "수비수가 공을 **의도적으로 플레이**한 것으로 판정되어 오프사이드가 리셋되었습니다.".to_string()
        } else {
            "The defender **deliberately played** the ball, resetting the offside situation.".to_string()
        }),
        DefenderTouchType::Deflection => Some(if use_korean {
            "수비수의 단순 **굴절**은 오프사이드를 리셋하지 않습니다.".to_string()
        } else {
            "A mere **deflection** by the defender does not reset the offside.".to_string()
        }),
        DefenderTouchType::Save => Some(if use_korean {
            "골키퍼의 **세이브**는 오프사이드를 리셋하지 않습니다.".to_string()
        } else {
            "A **save** by the goalkeeper does not reset the offside.".to_string()
        }),
    }
}

/// 재시작 설명 생성 (Law 15-17)
///
/// # Arguments
///
/// * `restart_type` - 재시작 유형
/// * `last_touch_team` - 마지막 터치 팀 이름
/// * `use_korean` - true: 한국어, false: 영어
pub fn format_restart_explanation(
    restart_type: &RestartType,
    last_touch_team: &str,
    use_korean: bool,
) -> String {
    match restart_type {
        RestartType::ThrowIn => {
            if use_korean {
                format!(
                    "공이 터치라인을 완전히 넘어갔습니다. 마지막 터치: {}. **스로인**.",
                    last_touch_team
                )
            } else {
                format!(
                    "Ball wholly crossed touchline. Last touch: {}. **Throw-in**.",
                    last_touch_team
                )
            }
        }
        RestartType::GoalKick => {
            if use_korean {
                "공이 골라인을 완전히 넘어갔습니다. 마지막 터치: 공격팀. **골킥**.".to_string()
            } else {
                "Ball wholly crossed goal line. Last touch: attacking team. **Goal kick**.".to_string()
            }
        }
        RestartType::CornerKick => {
            if use_korean {
                "공이 골라인을 완전히 넘어갔습니다. 마지막 터치: 수비팀. **코너킥**.".to_string()
            } else {
                "Ball wholly crossed goal line. Last touch: defending team. **Corner kick**.".to_string()
            }
        }
        _ => {
            if use_korean {
                format!("재시작: {}", restart_type.name_ko())
            } else {
                format!("Restart: {}", restart_type.name_en())
            }
        }
    }
}

// =============================================================================
// Phase 4: UI "Why?" Button - Full Explanation Generators
// =============================================================================

use crate::models::rules::{FoulDetails, OffsideDetails, OffsideInvolvementType};

/// Generate complete "Why?" explanation for an offside event
///
/// This is the main entry point for UI to get a full explanation of an offside call.
/// Returns a multi-line explanation suitable for display in a dialog/popup.
///
/// # Arguments
/// * `details` - The OffsideDetails from the event
/// * `player_name` - Name of the offside player
/// * `use_korean` - true for Korean, false for English
pub fn generate_offside_why_explanation(
    details: &OffsideDetails,
    player_name: &str,
    use_korean: bool,
) -> String {
    let mut lines: Vec<String> = Vec::new();

    // 1. Basic offside statement
    lines.push(format_offside_explanation(player_name, details.margin_m, use_korean));

    // 2. Check for exceptions first
    if let Some(ref restart_ctx) = details.restart_context {
        if restart_ctx.offside_exception_applies {
            if let Some(exception_msg) =
                format_offside_exception_explanation(&restart_ctx.restart_type, use_korean)
            {
                lines.push(exception_msg);
                return lines.join("\n\n");
            }
        }
    }

    // 3. Add involvement type explanation
    if let Some(ref involvement) = details.involvement_type {
        let involvement_msg = match involvement {
            OffsideInvolvementType::InterferingWithPlay => {
                if use_korean {
                    "판정 사유: **플레이 관여** (볼을 터치하거나 플레이하려고 함)".to_string()
                } else {
                    "Reason: **Interfering with play** (touching or attempting to play the ball)"
                        .to_string()
                }
            }
            OffsideInvolvementType::InterferingWithOpponent => {
                if use_korean {
                    "판정 사유: **상대방 방해** (상대방의 볼 플레이를 방해하거나 시야를 가림)"
                        .to_string()
                } else {
                    "Reason: **Interfering with an opponent** (preventing opponent from playing the ball or obstructing line of vision)".to_string()
                }
            }
            OffsideInvolvementType::GainingAdvantage => {
                if use_korean {
                    "판정 사유: **이익 획득** (골대나 상대방에 맞고 나온 볼을 플레이)".to_string()
                } else {
                    "Reason: **Gaining advantage** (playing a ball that rebounds from goal frame or opponent)".to_string()
                }
            }
        };
        lines.push(involvement_msg);
    }

    // 4. Add deflection context if relevant
    if let Some(ref deflection_ctx) = details.deflection_context {
        if let Some(defl_msg) =
            format_deflection_explanation(&deflection_ctx.last_touch_by_defender, use_korean)
        {
            lines.push(defl_msg);
        }
    }

    // 5. Add technical details
    let tech_details = if use_korean {
        format!(
            "기술 정보: 오프사이드 라인 {:.1}m, 마진 {:.2}m",
            details.offside_line_m, details.margin_m
        )
    } else {
        format!(
            "Technical: Offside line at {:.1}m, margin {:.2}m",
            details.offside_line_m, details.margin_m
        )
    };
    lines.push(tech_details);

    lines.join("\n\n")
}

/// Generate complete "Why?" explanation for a foul event
///
/// This is the main entry point for UI to get a full explanation of a foul call.
/// Returns a multi-line explanation suitable for display in a dialog/popup.
///
/// # Arguments
/// * `details` - The FoulDetails from the event
/// * `player_name` - Name of the player who committed the foul
/// * `victim_name` - Name of the fouled player
/// * `use_korean` - true for Korean, false for English
pub fn generate_foul_why_explanation(
    details: &FoulDetails,
    player_name: &str,
    victim_name: &str,
    use_korean: bool,
) -> String {
    let mut lines: Vec<String> = Vec::new();

    // 1. Basic foul statement
    if let Some(ref foul_type) = details.foul_type {
        lines.push(format_foul_explanation(player_name, victim_name, foul_type, use_korean));
    } else {
        let basic = if use_korean {
            format!("{}이(가) {}에게 파울", player_name, victim_name)
        } else {
            format!("{} committed a foul on {}", player_name, victim_name)
        };
        lines.push(basic);
    }

    // 2. Severity explanation
    let severity_msg = match details.severity {
        crate::models::rules::FoulSeverity::Careless => {
            if use_korean {
                "**부주의한** 파울 - 직접 프리킥".to_string()
            } else {
                "**Careless** foul - direct free kick".to_string()
            }
        }
        crate::models::rules::FoulSeverity::Reckless => {
            if use_korean {
                "**무모한** 파울 - 옐로카드 + 직접 프리킥".to_string()
            } else {
                "**Reckless** foul - yellow card + direct free kick".to_string()
            }
        }
        crate::models::rules::FoulSeverity::ExcessiveForce => {
            if use_korean {
                "**과도한 힘** 사용 - 레드카드 + 직접 프리킥".to_string()
            } else {
                "**Excessive force** - red card + direct free kick".to_string()
            }
        }
    };
    lines.push(severity_msg);

    // 3. DOGSO explanation if applicable
    if details.is_dogso {
        let dogso_msg = if details.in_penalty_area && details.attempted_to_play_ball {
            if use_korean {
                "페널티 에어리어 내 **DOGSO**이나 볼 플레이 시도로 **옐로카드로 감경**".to_string()
            } else {
                "**DOGSO** in penalty area but attempted to play ball - **reduced to yellow card**"
                    .to_string()
            }
        } else {
            if use_korean {
                "**명백한 득점 기회 저지 (DOGSO)** - 레드카드".to_string()
            } else {
                "**Denying an obvious goal-scoring opportunity (DOGSO)** - red card".to_string()
            }
        };
        lines.push(dogso_msg);
    }

    // 4. Location info
    let location_msg = if details.in_penalty_area {
        if use_korean {
            "위치: **페널티 에어리어 내** → 페널티킥".to_string()
        } else {
            "Location: **Inside penalty area** → Penalty kick".to_string()
        }
    } else {
        if use_korean {
            "위치: 페널티 에어리어 밖 → 직접 프리킥".to_string()
        } else {
            "Location: Outside penalty area → Direct free kick".to_string()
        }
    };
    lines.push(location_msg);

    lines.join("\n\n")
}

/// Check if an event should show the "Why?" button
///
/// # Arguments
/// * `event_details` - The EventDetails from the event (may be None)
pub fn should_show_why_button(event_details: Option<&crate::models::EventDetails>) -> bool {
    match event_details {
        Some(details) => {
            // Show "Why?" if we have rule details
            details.offside_details.is_some()
                || details.foul_details.is_some()
                || details.rule_id.is_some()
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_offside_rules() {
        let rules = get_offside_rules();
        assert_eq!(rules.rule_id, "LAW_11_OFFSIDE");
        assert_eq!(rules.law_number, 11);
        assert_eq!(rules.name, "오프사이드");
        assert_eq!(rules.name_en, "Offside");
    }

    #[test]
    fn test_load_fouls_rules() {
        let rules = get_fouls_rules();
        assert_eq!(rules.rule_id, "LAW_12_FOULS_MISCONDUCT");
        assert_eq!(rules.law_number, 12);
        assert_eq!(rules.name, "파울과 비신사적 행위");
        assert_eq!(rules.name_en, "Fouls and Misconduct");
    }

    #[test]
    fn test_foul_severity_data() {
        let rules = get_fouls_rules();
        assert_eq!(rules.foul_severity.len(), 3);

        let careless = &rules.foul_severity[0];
        assert_eq!(careless.id, "CARELESS");
        assert_eq!(careless.level, 1);
        assert_eq!(careless.name, "부주의");

        let reckless = &rules.foul_severity[1];
        assert_eq!(reckless.id, "RECKLESS");
        assert_eq!(reckless.level, 2);

        let excessive = &rules.foul_severity[2];
        assert_eq!(excessive.id, "EXCESSIVE_FORCE");
        assert_eq!(excessive.level, 3);
    }

    #[test]
    fn test_format_offside_explanation_korean() {
        let explanation = format_offside_explanation("손흥민", 0.35, true);
        assert!(explanation.contains("손흥민"));
        assert!(explanation.contains("0.35"));
        assert!(explanation.contains("오프사이드"));
    }

    #[test]
    fn test_format_offside_explanation_english() {
        let explanation = format_offside_explanation("Son", 0.35, false);
        assert!(explanation.contains("Son"));
        assert!(explanation.contains("0.35"));
        assert!(explanation.contains("offside"));
    }

    #[test]
    fn test_format_foul_explanation() {
        use crate::models::rules::FoulType;

        let explanation_ko =
            format_foul_explanation("김민재", "손흥민", &FoulType::Tackling, true);
        assert!(explanation_ko.contains("김민재"));
        assert!(explanation_ko.contains("손흥민"));
        assert!(explanation_ko.contains("태클"));

        let explanation_en =
            format_foul_explanation("Kim", "Son", &FoulType::Tackling, false);
        assert!(explanation_en.contains("Kim"));
        assert!(explanation_en.contains("Son"));
        assert!(explanation_en.contains("Tackling"));
    }

    #[test]
    fn test_offside_exception_goal_kick() {
        let explanation = format_offside_exception_explanation(&RestartType::GoalKick, true);
        assert!(explanation.is_some());
        assert!(explanation.unwrap().contains("골킥"));

        let explanation_en = format_offside_exception_explanation(&RestartType::GoalKick, false);
        assert!(explanation_en.is_some());
        assert!(explanation_en.unwrap().contains("goal kick"));
    }

    #[test]
    fn test_offside_exception_throw_in() {
        let explanation = format_offside_exception_explanation(&RestartType::ThrowIn, true);
        assert!(explanation.is_some());
        assert!(explanation.unwrap().contains("스로인"));
    }

    #[test]
    fn test_offside_exception_corner() {
        let explanation = format_offside_exception_explanation(&RestartType::CornerKick, true);
        assert!(explanation.is_some());
        assert!(explanation.unwrap().contains("코너킥"));
    }

    #[test]
    fn test_no_offside_exception_for_normal_play() {
        let explanation = format_offside_exception_explanation(&RestartType::Normal, true);
        assert!(explanation.is_none());
    }

    #[test]
    fn test_deflection_deliberate_play() {
        let explanation = format_deflection_explanation(&DefenderTouchType::DeliberatePlay, true);
        assert!(explanation.is_some());
        assert!(explanation.unwrap().contains("의도적으로 플레이"));
    }

    #[test]
    fn test_deflection_no_reset() {
        let explanation = format_deflection_explanation(&DefenderTouchType::Deflection, true);
        assert!(explanation.is_some());
        assert!(explanation.unwrap().contains("리셋하지 않습니다"));
    }

    #[test]
    fn test_save_no_reset() {
        let explanation = format_deflection_explanation(&DefenderTouchType::Save, true);
        assert!(explanation.is_some());
        assert!(explanation.unwrap().contains("세이브"));
    }

    #[test]
    fn test_restart_explanation_throw_in() {
        let explanation = format_restart_explanation(&RestartType::ThrowIn, "홈팀", true);
        assert!(explanation.contains("터치라인"));
        assert!(explanation.contains("홈팀"));
        assert!(explanation.contains("스로인"));
    }

    #[test]
    fn test_restart_explanation_goal_kick() {
        let explanation = format_restart_explanation(&RestartType::GoalKick, "", true);
        assert!(explanation.contains("골라인"));
        assert!(explanation.contains("골킥"));
    }

    #[test]
    fn test_restart_explanation_corner() {
        let explanation = format_restart_explanation(&RestartType::CornerKick, "", true);
        assert!(explanation.contains("골라인"));
        assert!(explanation.contains("코너킥"));
    }

    #[test]
    fn test_yaml_files_valid() {
        // Verify all YAML files can be read (compilation test)
        assert!(!LAW_09_10_13_17_YAML.is_empty());
        assert!(!EVENT_TO_RULE_MAP_YAML.is_empty());
        assert!(!EXPLAIN_TEMPLATES_YAML.is_empty());
    }

    // =========================================================================
    // Phase 4: UI "Why?" Button Tests
    // =========================================================================

    #[test]
    fn test_generate_offside_why_explanation_korean() {
        use crate::models::rules::{
            DefenderTouchType, DeflectionContext, OffsideRestartContext, ReferencePoint,
            TouchReference, TouchType,
        };

        let details = OffsideDetails {
            margin_m: 0.45,
            offside_line_m: 75.0,
            passer_track_id: Some(8),
            involvement_type: Some(OffsideInvolvementType::InterferingWithPlay),
            restart_context: Some(OffsideRestartContext {
                restart_type: RestartType::Normal,
                offside_exception_applies: false,
            }),
            touch_reference: Some(TouchReference {
                touch_type: TouchType::Kick,
                reference_point: ReferencePoint::FirstContact,
            }),
            deflection_context: Some(DeflectionContext {
                last_touch_by_defender: DefenderTouchType::None,
                resets_offside: false,
            }),
        };

        let explanation = generate_offside_why_explanation(&details, "손흥민", true);
        assert!(explanation.contains("손흥민"));
        assert!(explanation.contains("오프사이드"));
        assert!(explanation.contains("플레이 관여"));
        assert!(explanation.contains("0.45"));
    }

    #[test]
    fn test_generate_offside_why_explanation_english() {
        use crate::models::rules::{
            DefenderTouchType, DeflectionContext, OffsideRestartContext, ReferencePoint,
            TouchReference, TouchType,
        };

        let details = OffsideDetails {
            margin_m: 0.45,
            offside_line_m: 75.0,
            passer_track_id: Some(8),
            involvement_type: Some(OffsideInvolvementType::InterferingWithPlay),
            restart_context: Some(OffsideRestartContext {
                restart_type: RestartType::Normal,
                offside_exception_applies: false,
            }),
            touch_reference: Some(TouchReference {
                touch_type: TouchType::Kick,
                reference_point: ReferencePoint::FirstContact,
            }),
            deflection_context: Some(DeflectionContext {
                last_touch_by_defender: DefenderTouchType::None,
                resets_offside: false,
            }),
        };

        let explanation = generate_offside_why_explanation(&details, "Son", false);
        assert!(explanation.contains("Son"));
        assert!(explanation.contains("offside"));
        assert!(explanation.contains("Interfering with play"));
        assert!(explanation.contains("0.45"));
    }

    #[test]
    fn test_generate_foul_why_explanation_korean() {
        use crate::models::rules::FoulType;

        let details = FoulDetails {
            severity: crate::models::rules::FoulSeverity::Reckless,
            foul_type: Some(FoulType::Tackling),
            is_dogso: false,
            in_penalty_area: false,
            victim_track_id: Some(10),
            attempted_to_play_ball: true,
        };

        let explanation = generate_foul_why_explanation(&details, "김민재", "손흥민", true);
        assert!(explanation.contains("김민재"));
        assert!(explanation.contains("손흥민"));
        assert!(explanation.contains("무모한"));
        assert!(explanation.contains("옐로카드"));
    }

    #[test]
    fn test_generate_foul_why_explanation_english() {
        use crate::models::rules::FoulType;

        let details = FoulDetails {
            severity: crate::models::rules::FoulSeverity::Reckless,
            foul_type: Some(FoulType::Tackling),
            is_dogso: false,
            in_penalty_area: false,
            victim_track_id: Some(10),
            attempted_to_play_ball: true,
        };

        let explanation = generate_foul_why_explanation(&details, "Kim", "Son", false);
        assert!(explanation.contains("Kim"));
        assert!(explanation.contains("Son"));
        assert!(explanation.contains("Reckless"));
        assert!(explanation.contains("yellow card"));
    }

    #[test]
    fn test_generate_foul_why_dogso_penalty_area() {
        use crate::models::rules::FoulType;

        let details = FoulDetails {
            severity: crate::models::rules::FoulSeverity::ExcessiveForce,
            foul_type: Some(FoulType::Tackling),
            is_dogso: true,
            in_penalty_area: true,
            victim_track_id: Some(10),
            attempted_to_play_ball: true,
        };

        let explanation = generate_foul_why_explanation(&details, "Player", "Victim", true);
        assert!(explanation.contains("DOGSO"));
        assert!(explanation.contains("옐로카드로 감경"));
    }

    #[test]
    fn test_should_show_why_button_with_offside() {
        use crate::models::EventDetails;

        let details = EventDetails {
            offside_details: Some(OffsideDetails {
                margin_m: 0.5,
                offside_line_m: 75.0,
                passer_track_id: None,
                involvement_type: None,
                restart_context: None,
                touch_reference: None,
                deflection_context: None,
            }),
            ..Default::default()
        };

        assert!(should_show_why_button(Some(&details)));
    }

    #[test]
    fn test_should_show_why_button_with_foul() {
        use crate::models::EventDetails;

        let details = EventDetails {
            foul_details: Some(FoulDetails {
                severity: crate::models::rules::FoulSeverity::Careless,
                foul_type: None,
                is_dogso: false,
                in_penalty_area: false,
                victim_track_id: None,
                attempted_to_play_ball: true,
            }),
            ..Default::default()
        };

        assert!(should_show_why_button(Some(&details)));
    }

    #[test]
    fn test_should_show_why_button_without_details() {
        assert!(!should_show_why_button(None));
    }
}
