//! Phase 3: TacticalDecisionEngine - 전술 추천/경고 시스템

use serde::{Deserialize, Serialize};

use crate::squad_selector::get_formation_positions;
use crate::CoreTeam;

/// 전술 분석 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalAnalysis {
    pub squad_rating: f32,
    pub formation_fitness: f32,
    pub position_mismatches: Vec<PositionMismatch>,
    pub recommendations: Vec<TacticalRecommendation>,
    pub warnings: Vec<String>,
}

/// 포지션 미스매치 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionMismatch {
    pub player_name: String,
    pub assigned_position: String,
    pub natural_position: String,
    pub fitness_loss: f32,
}

/// 전술 추천
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalRecommendation {
    pub priority: String, // "High", "Medium", "Low"
    pub message: String,
    pub suggested_action: Option<String>,
}

/// 팀 설정 분석
pub fn analyze_team_setup(team: &CoreTeam) -> TacticalAnalysis {
    let mut analysis = TacticalAnalysis {
        squad_rating: 0.0,
        formation_fitness: 0.0,
        position_mismatches: vec![],
        recommendations: vec![],
        warnings: vec![],
    };

    // 1. 스쿼드 평점 계산
    if !team.players.is_empty() {
        analysis.squad_rating =
            team.players.iter().map(|p| p.ca as f32).sum::<f32>() / team.players.len() as f32;
    }

    // 2. 포메이션 적합도 및 미스매치 검사
    if let Some(formation) = &team.formation {
        let positions = get_formation_positions(formation);
        let mut total_fitness = 0.0;
        let mut matched_count = 0;

        for (idx, required_pos) in positions.iter().enumerate() {
            if idx >= team.players.len() {
                break;
            }

            let player = &team.players[idx];
            let fitness = calculate_position_fitness(&player.position, required_pos);
            total_fitness += fitness;
            matched_count += 1;

            // 미스매치 검사 (70% 미만 적합도)
            if fitness < 0.7 {
                let fitness_loss = 1.0 - fitness;
                analysis.position_mismatches.push(PositionMismatch {
                    player_name: player.name.clone(),
                    assigned_position: required_pos.to_string(),
                    natural_position: player.position.clone(),
                    fitness_loss,
                });
            }
        }

        if matched_count > 0 {
            analysis.formation_fitness = total_fitness / matched_count as f32;
        }

        // 포메이션 적합도 경고
        if analysis.formation_fitness < 0.5 {
            analysis.warnings.push(format!(
                "포메이션 {} 적합도가 매우 낮습니다 ({:.0}%)",
                formation,
                analysis.formation_fitness * 100.0
            ));

            let suggested = suggest_better_formation(team);
            analysis.recommendations.push(TacticalRecommendation {
                priority: "High".to_string(),
                message: "포메이션 변경을 강력히 권장합니다".to_string(),
                suggested_action: Some(format!("추천 포메이션: {}", suggested)),
            });
        } else if analysis.formation_fitness < 0.7 {
            analysis.warnings.push(format!(
                "포메이션 {} 적합도가 낮습니다 ({:.0}%)",
                formation,
                analysis.formation_fitness * 100.0
            ));

            analysis.recommendations.push(TacticalRecommendation {
                priority: "Medium".to_string(),
                message: "포메이션 변경을 고려해 보세요".to_string(),
                suggested_action: None,
            });
        }
    } else {
        analysis
            .warnings
            .push("포메이션이 설정되지 않았습니다".to_string());
        analysis.recommendations.push(TacticalRecommendation {
            priority: "High".to_string(),
            message: "포메이션을 설정하세요".to_string(),
            suggested_action: Some("T442를 기본으로 시작해 보세요".to_string()),
        });
    }

    // 3. 포지션 미스매치 경고
    if !analysis.position_mismatches.is_empty() {
        let severe_mismatches: Vec<_> = analysis
            .position_mismatches
            .iter()
            .filter(|m| m.fitness_loss > 0.5)
            .collect();

        if !severe_mismatches.is_empty() {
            analysis.warnings.push(format!(
                "{}명의 선수가 심각한 포지션 미스매치 상태입니다",
                severe_mismatches.len()
            ));

            for mismatch in severe_mismatches {
                analysis.recommendations.push(TacticalRecommendation {
                    priority: "High".to_string(),
                    message: format!(
                        "{}: {} → {} (적합도 {:.0}% 손실)",
                        mismatch.player_name,
                        mismatch.natural_position,
                        mismatch.assigned_position,
                        mismatch.fitness_loss * 100.0
                    ),
                    suggested_action: Some(format!(
                        "{}를 {} 포지션으로 배치하세요",
                        mismatch.player_name, mismatch.natural_position
                    )),
                });
            }
        } else if analysis.position_mismatches.len() > 2 {
            analysis.warnings.push(format!(
                "{}명의 선수가 비선호 포지션에 배치됨",
                analysis.position_mismatches.len()
            ));
        }
    }

    // 4. 선수 수 경고
    if team.players.len() < 11 {
        analysis.warnings.push(format!(
            "선발 선수가 부족합니다 ({}/11)",
            team.players.len()
        ));
        analysis.recommendations.push(TacticalRecommendation {
            priority: "High".to_string(),
            message: "선발 라인업을 완성하세요".to_string(),
            suggested_action: None,
        });
    }

    // 5. 벤치 깊이 경고
    let bench_count = team.substitutes.as_ref().map(|s| s.len()).unwrap_or(0);
    if bench_count < 5 {
        analysis
            .warnings
            .push(format!("벤치 선수가 부족합니다 ({}/7)", bench_count));
        if bench_count < 3 {
            analysis.recommendations.push(TacticalRecommendation {
                priority: "Medium".to_string(),
                message: "벤치 선수를 추가하세요".to_string(),
                suggested_action: None,
            });
        }
    }

    // 6. 컨디션 경고
    let low_condition_players: Vec<_> = team.players.iter().filter(|p| p.condition < 0.5).collect();

    if !low_condition_players.is_empty() {
        analysis.warnings.push(format!(
            "{}명의 선수가 낮은 컨디션 상태입니다",
            low_condition_players.len()
        ));

        for player in low_condition_players {
            if player.condition < 0.3 {
                analysis.recommendations.push(TacticalRecommendation {
                    priority: "High".to_string(),
                    message: format!(
                        "{}: 컨디션 {:.0}%로 매우 낮음",
                        player.name,
                        player.condition * 100.0
                    ),
                    suggested_action: Some(format!("{}를 벤치로 교체 고려", player.name)),
                });
            }
        }
    }

    // 7. CA 균형 분석
    if team.players.len() >= 11 {
        let ca_values: Vec<f32> = team.players.iter().map(|p| p.ca as f32).collect();
        let avg_ca = analysis.squad_rating;
        let variance: f32 = ca_values
            .iter()
            .map(|ca| (ca - avg_ca).powi(2))
            .sum::<f32>()
            / ca_values.len() as f32;
        let std_dev = variance.sqrt();

        if std_dev > 20.0 {
            analysis.recommendations.push(TacticalRecommendation {
                priority: "Low".to_string(),
                message: format!("팀 능력치 편차가 큽니다 (표준편차: {:.1})", std_dev),
                suggested_action: Some("약한 선수 보강을 고려해 보세요".to_string()),
            });
        }
    }

    analysis
}

/// 포지션 적합도 계산
fn calculate_position_fitness(natural: &str, assigned: &str) -> f32 {
    let natural_upper = natural.to_uppercase();
    let assigned_upper = assigned.to_uppercase();

    // 동일 포지션
    if natural_upper == assigned_upper {
        return 1.0;
    }

    // 포지션 그룹 정의
    let gk = ["GK"];
    let cb = ["CB", "DC"];
    let fb = ["LB", "RB", "DL", "DR", "WB", "WBL", "WBR"];
    let dm = ["DM", "CDM", "DMC"];
    let cm = ["CM", "MC", "CMF"];
    let wm = ["LM", "RM", "ML", "MR"];
    let am = ["AM", "AMC", "CAM"];
    let wing = ["LW", "RW", "AML", "AMR"];
    let st = ["ST", "CF", "FW"];

    let in_group =
        |pos: &str, group: &[&str]| -> bool { group.iter().any(|&g| pos.eq_ignore_ascii_case(g)) };

    // 같은 그룹 내
    if (in_group(&natural_upper, &cb) && in_group(&assigned_upper, &cb))
        || (in_group(&natural_upper, &fb) && in_group(&assigned_upper, &fb))
        || (in_group(&natural_upper, &dm) && in_group(&assigned_upper, &dm))
        || (in_group(&natural_upper, &cm) && in_group(&assigned_upper, &cm))
        || (in_group(&natural_upper, &wm) && in_group(&assigned_upper, &wm))
        || (in_group(&natural_upper, &am) && in_group(&assigned_upper, &am))
        || (in_group(&natural_upper, &wing) && in_group(&assigned_upper, &wing))
        || (in_group(&natural_upper, &st) && in_group(&assigned_upper, &st))
    {
        return 0.95;
    }

    // 인접 포지션 (높은 호환성)
    // CB <-> DM
    if (in_group(&natural_upper, &cb) && in_group(&assigned_upper, &dm))
        || (in_group(&natural_upper, &dm) && in_group(&assigned_upper, &cb))
    {
        return 0.75;
    }

    // DM <-> CM
    if (in_group(&natural_upper, &dm) && in_group(&assigned_upper, &cm))
        || (in_group(&natural_upper, &cm) && in_group(&assigned_upper, &dm))
    {
        return 0.8;
    }

    // CM <-> AM
    if (in_group(&natural_upper, &cm) && in_group(&assigned_upper, &am))
        || (in_group(&natural_upper, &am) && in_group(&assigned_upper, &cm))
    {
        return 0.75;
    }

    // WM <-> Wing
    if (in_group(&natural_upper, &wm) && in_group(&assigned_upper, &wing))
        || (in_group(&natural_upper, &wing) && in_group(&assigned_upper, &wm))
    {
        return 0.8;
    }

    // AM <-> ST
    if (in_group(&natural_upper, &am) && in_group(&assigned_upper, &st))
        || (in_group(&natural_upper, &st) && in_group(&assigned_upper, &am))
    {
        return 0.7;
    }

    // Wing <-> ST
    if (in_group(&natural_upper, &wing) && in_group(&assigned_upper, &st))
        || (in_group(&natural_upper, &st) && in_group(&assigned_upper, &wing))
    {
        return 0.65;
    }

    // FB <-> WM (윙백)
    if (in_group(&natural_upper, &fb) && in_group(&assigned_upper, &wm))
        || (in_group(&natural_upper, &wm) && in_group(&assigned_upper, &fb))
    {
        return 0.6;
    }

    // GK는 다른 포지션과 호환 불가
    if in_group(&natural_upper, &gk) || in_group(&assigned_upper, &gk) {
        return 0.1;
    }

    // 기본 낮은 호환성
    0.4
}

/// 팀에 맞는 포메이션 추천
fn suggest_better_formation(team: &CoreTeam) -> String {
    // 포지션 그룹별 선수 수 계산
    let mut defenders = 0;
    let mut midfielders = 0;
    let mut forwards = 0;

    for player in &team.players {
        let pos = player.position.to_uppercase();
        if pos.contains("GK") {
            continue;
        } else if pos.contains('D') || pos == "CB" || pos == "LB" || pos == "RB" {
            defenders += 1;
        } else if pos.contains('M') || pos == "CM" || pos == "DM" || pos == "AM" {
            midfielders += 1;
        } else if pos.contains('F')
            || pos.contains('W')
            || pos == "ST"
            || pos == "CF"
            || pos == "LW"
            || pos == "RW"
        {
            forwards += 1;
        }
    }

    // 선수 구성에 따른 포메이션 추천
    if defenders >= 5 {
        "T532".to_string()
    } else if forwards >= 3 {
        "T433".to_string()
    } else if midfielders >= 5 {
        "T4231".to_string()
    } else if defenders >= 4 && midfielders >= 4 {
        "T442".to_string()
    } else if defenders >= 3 && midfielders >= 5 {
        "T352".to_string()
    } else {
        "T442".to_string() // 기본
    }
}
