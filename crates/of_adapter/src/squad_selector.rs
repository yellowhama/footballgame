//! SquadSelector: 포지션별 최적 선수 선택 및 스쿼드 구성
//!
//! 포메이션에 따른 11명 선발 + 7명 벤치 자동 선택

use std::collections::HashMap;

/// 포지션별 핵심 속성 정의
pub fn get_position_weights(position: &str) -> Vec<(&'static str, f32)> {
    match position.to_uppercase().as_str() {
        "GK" => vec![
            ("handling", 1.0),
            ("positioning", 0.9),
            ("reflexes", 0.9),
            ("concentration", 0.7),
            ("decisions", 0.6),
            ("composure", 0.6),
        ],
        "CB" => vec![
            ("tackling", 1.0),
            ("marking", 0.9),
            ("positioning", 0.9),
            ("heading", 0.8),
            ("strength", 0.7),
            ("jumping", 0.7),
            ("concentration", 0.6),
            ("anticipation", 0.6),
        ],
        "LB" | "RB" => vec![
            ("pace", 1.0),
            ("stamina", 0.9),
            ("crossing", 0.8),
            ("tackling", 0.8),
            ("work_rate", 0.7),
            ("positioning", 0.7),
            ("dribbling", 0.5),
            ("passing", 0.5),
        ],
        "LWB" | "RWB" => vec![
            ("pace", 1.0),
            ("stamina", 1.0),
            ("crossing", 0.9),
            ("tackling", 0.7),
            ("work_rate", 0.8),
            ("dribbling", 0.6),
        ],
        "CDM" | "DM" => vec![
            ("tackling", 1.0),
            ("positioning", 0.9),
            ("anticipation", 0.9),
            ("passing", 0.8),
            ("work_rate", 0.8),
            ("stamina", 0.7),
            ("decisions", 0.6),
            ("strength", 0.6),
        ],
        "CM" => vec![
            ("passing", 1.0),
            ("vision", 0.9),
            ("decisions", 0.9),
            ("stamina", 0.8),
            ("work_rate", 0.8),
            ("teamwork", 0.7),
            ("technique", 0.6),
            ("composure", 0.6),
        ],
        "CAM" | "AM" => vec![
            ("vision", 1.0),
            ("passing", 0.9),
            ("technique", 0.9),
            ("dribbling", 0.8),
            ("composure", 0.8),
            ("decisions", 0.7),
            ("flair", 0.6),
            ("finishing", 0.5),
        ],
        "LM" | "RM" => vec![
            ("pace", 1.0),
            ("stamina", 0.9),
            ("crossing", 0.8),
            ("passing", 0.7),
            ("work_rate", 0.8),
            ("dribbling", 0.6),
        ],
        "LW" | "RW" => vec![
            ("pace", 1.0),
            ("dribbling", 0.9),
            ("crossing", 0.8),
            ("acceleration", 0.8),
            ("agility", 0.7),
            ("technique", 0.7),
            ("flair", 0.5),
            ("work_rate", 0.5),
        ],
        "CF" => vec![
            ("technique", 1.0),
            ("ball_control", 0.9),
            ("vision", 0.8),
            ("finishing", 0.8),
            ("composure", 0.8),
            ("passing", 0.6),
        ],
        "ST" => vec![
            ("finishing", 1.0),
            ("composure", 0.9),
            ("off_the_ball", 0.9),
            ("heading", 0.7),
            ("anticipation", 0.7),
            ("decisions", 0.6),
            ("pace", 0.5),
            ("strength", 0.5),
        ],
        _ => vec![
            ("decisions", 0.8),
            ("anticipation", 0.7),
            ("composure", 0.7),
        ],
    }
}

/// 포메이션에서 필요한 포지션 목록 추출
pub fn get_formation_positions(formation: &str) -> Vec<&'static str> {
    match formation.to_uppercase().replace("-", "").as_str() {
        "T442" | "442" => vec![
            "GK", "LB", "CB", "CB", "RB", "LM", "CM", "CM", "RM", "ST", "ST",
        ],
        "T433" | "433" => vec![
            "GK", "LB", "CB", "CB", "RB", "CM", "CM", "CM", "LW", "ST", "RW",
        ],
        "T451" | "451" => vec![
            "GK", "LB", "CB", "CB", "RB", "LM", "CM", "CM", "CM", "RM", "ST",
        ],
        "T4231" | "4231" => vec![
            "GK", "LB", "CB", "CB", "RB", "CDM", "CDM", "LW", "CAM", "RW", "ST",
        ],
        "T352" | "352" => vec![
            "GK", "CB", "CB", "CB", "LWB", "CM", "CM", "RWB", "CAM", "ST", "ST",
        ],
        "T4141" | "4141" => vec![
            "GK", "LB", "CB", "CB", "RB", "CDM", "LM", "CM", "CM", "RM", "ST",
        ],
        "T343" | "343" => vec![
            "GK", "CB", "CB", "CB", "LM", "CM", "CM", "RM", "LW", "ST", "RW",
        ],
        "T532" | "532" => vec![
            "GK", "LWB", "CB", "CB", "CB", "RWB", "CM", "CM", "CM", "ST", "ST",
        ],
        _ => vec![
            "GK", "LB", "CB", "CB", "RB", "CM", "CM", "CM", "LW", "ST", "RW",
        ],
    }
}

/// 선수의 포지션 적합도 점수 계산
#[derive(Debug, Clone)]
pub struct PlayerScore {
    pub player_idx: usize,
    pub name: String,
    pub position: String,
    pub ca: u32,
    pub condition: f32,
    pub fit_score: f32,
    pub total_score: f32,
}

/// 속성값 가져오기 헬퍼
pub fn get_attribute_value(
    technical: &HashMap<String, u8>,
    mental: &HashMap<String, u8>,
    physical: &HashMap<String, u8>,
    attr_name: &str,
) -> u8 {
    // Technical
    if let Some(&v) = technical.get(attr_name) {
        return v;
    }
    // 속성명 매핑 (JSON 필드명 → 엔진 필드명)
    let mapped_name = match attr_name {
        "ball_control" | "first_touch" => "ball_control",
        "heading" | "heading_accuracy" => "heading_accuracy",
        "passing" | "short_passing" => "short_passing",
        "reflexes" => "reflexes", // GK용 - mental에 없으면 기본값
        "handling" => "handling", // GK용
        _ => attr_name,
    };

    if let Some(&v) = technical.get(mapped_name) {
        return v;
    }

    // Mental
    if let Some(&v) = mental.get(attr_name) {
        return v;
    }

    // Physical
    if let Some(&v) = physical.get(attr_name) {
        return v;
    }

    10 // 기본값
}

/// 선수의 특정 포지션 적합도 계산
///
/// 엔진 가중치 공식 기반 (selector.rs:326-349):
/// - Position proficiency: 40%
/// - Physical condition: 25%
/// - Overall ability: 20%
/// - Tactical fit: 10%
/// - Reputation: 5% (여기서는 CA로 대체)
pub fn calculate_position_fit(
    required_pos: &str,
    player_pos: &str,
    technical: &HashMap<String, u8>,
    mental: &HashMap<String, u8>,
    physical: &HashMap<String, u8>,
    ca: u32,
    condition: f32,
) -> f32 {
    // 1. 포지션 호환성 체크
    let pos_penalty = calculate_position_penalty(player_pos, required_pos);
    if pos_penalty >= 1.0 {
        return 0.0; // 완전 비호환 (GK <-> 필드플레이어)
    }

    // 2. Position proficiency (40%) - 포지션 레벨을 0-20 스케일로
    let position_level = (1.0 - pos_penalty) * 20.0;
    let position_score = position_level * 0.4;

    // 3. Physical condition (25%) - 0-100을 0-20 스케일로
    let condition_score = (condition * 20.0) * 0.25;

    // 4. Overall ability (20%) - CA를 0-200에서 0-20 스케일로
    let ability_score = (ca as f32 / 200.0 * 20.0) * 0.2;

    // 5. Tactical fit (10%) - 핵심 속성 기반
    let weights = get_position_weights(required_pos);
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;

    for (attr, weight) in &weights {
        let value = get_attribute_value(technical, mental, physical, attr) as f32;
        weighted_sum += value * weight;
        total_weight += weight;
    }

    let attr_avg = if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        10.0
    };
    let tactical_fit_score = attr_avg * 0.1;

    // 6. Reputation (5%) - CA의 일부로 대체
    let reputation_score = (ca as f32 / 200.0 * 20.0) * 0.05;

    // 최종 점수 (0-20 스케일)
    position_score + condition_score + ability_score + tactical_fit_score + reputation_score
}

/// 포지션 변경 페널티 (0.0 = 완벽, 1.0 = 불가)
fn calculate_position_penalty(player_pos: &str, required_pos: &str) -> f32 {
    let p = player_pos.to_uppercase();
    let r = required_pos.to_uppercase();

    if p == r {
        return 0.0;
    }

    // 같은 라인 내 변경
    let defenders = ["CB", "LB", "RB", "LWB", "RWB"];
    let midfielders = ["CDM", "DM", "CM", "CAM", "AM", "LM", "RM"];
    let wingers = ["LW", "RW", "LM", "RM"];
    let forwards = ["ST", "CF", "LW", "RW"];

    // GK는 교체 불가
    if p == "GK" || r == "GK" {
        if p == r {
            return 0.0;
        }
        return 1.0;
    }

    // 같은 카테고리
    if (defenders.contains(&p.as_str()) && defenders.contains(&r.as_str()))
        || (midfielders.contains(&p.as_str()) && midfielders.contains(&r.as_str()))
        || (forwards.contains(&p.as_str()) && forwards.contains(&r.as_str()))
    {
        return 0.2;
    }

    // 윙어 호환
    if wingers.contains(&p.as_str()) && wingers.contains(&r.as_str()) {
        return 0.1;
    }

    // 인접 포지션
    let adjacent_pairs = [
        ("CB", "CDM"),
        ("CDM", "CM"),
        ("CM", "CAM"),
        ("CAM", "CF"),
        ("CF", "ST"),
        ("LB", "LM"),
        ("RB", "RM"),
        ("LM", "LW"),
        ("RM", "RW"),
        ("LWB", "LW"),
        ("RWB", "RW"),
    ];

    for (a, b) in &adjacent_pairs {
        if (p == *a && r == *b) || (p == *b && r == *a) {
            return 0.3;
        }
    }

    // 크로스 라인 (수비-미드, 미드-공격)
    if (defenders.contains(&p.as_str()) && midfielders.contains(&r.as_str()))
        || (midfielders.contains(&p.as_str()) && defenders.contains(&r.as_str()))
        || (midfielders.contains(&p.as_str()) && forwards.contains(&r.as_str()))
        || (forwards.contains(&p.as_str()) && midfielders.contains(&r.as_str()))
    {
        return 0.5;
    }

    // 수비-공격 직접 변경
    0.8
}

/// 스쿼드 선택 결과
#[derive(Debug, Clone)]
pub struct SquadSelection {
    /// 선발 11명 (포지션 순서대로)
    pub starters: Vec<PlayerScore>,
    /// 벤치 7명
    pub substitutes: Vec<PlayerScore>,
    /// 선택되지 않은 선수들
    pub reserves: Vec<PlayerScore>,
}

/// 주어진 스쿼드에서 포메이션에 맞는 최적의 11명 선택
///
/// # Arguments
/// * `players` - (이름, 포지션, CA, 컨디션, technical, mental, physical) 튜플 리스트
/// * `formation` - 포메이션 코드 (예: "T442", "T433")
///
/// # Returns
/// * `SquadSelection` - 선발, 벤치, 예비 선수 목록
pub fn select_squad(
    players: &[(
        String,
        String,
        u32,
        f32,
        HashMap<String, u8>,
        HashMap<String, u8>,
        HashMap<String, u8>,
    )],
    formation: &str,
) -> SquadSelection {
    let positions = get_formation_positions(formation);

    // 각 선수의 각 포지션별 점수 계산
    let mut player_scores: Vec<Vec<(usize, f32)>> = vec![Vec::new(); positions.len()];

    for (idx, (name, pos, ca, cond, tech, mental, phys)) in players.iter().enumerate() {
        for (pos_idx, required_pos) in positions.iter().enumerate() {
            let score = calculate_position_fit(required_pos, pos, tech, mental, phys, *ca, *cond);
            if score > 0.0 {
                player_scores[pos_idx].push((idx, score));
            }
        }
    }

    // 각 포지션별로 점수 정렬
    for scores in &mut player_scores {
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    }

    // 그리디 선택: 각 포지션에 최고 점수 선수 배정 (중복 방지)
    let mut selected: Vec<Option<usize>> = vec![None; positions.len()];
    let mut used_players: std::collections::HashSet<usize> = std::collections::HashSet::new();

    // 우선순위: GK > 특수 포지션 > 일반 포지션
    let mut position_order: Vec<usize> = (0..positions.len()).collect();
    position_order.sort_by_key(|&i| match positions[i] {
        "GK" => 0,
        "ST" | "CF" => 1,
        "LW" | "RW" => 2,
        "CAM" | "CDM" => 3,
        _ => 4,
    });

    for &pos_idx in &position_order {
        for &(player_idx, _score) in &player_scores[pos_idx] {
            if !used_players.contains(&player_idx) {
                selected[pos_idx] = Some(player_idx);
                used_players.insert(player_idx);
                break;
            }
        }
    }

    // 선발 목록 생성
    let mut starters = Vec::new();
    for (pos_idx, maybe_idx) in selected.iter().enumerate() {
        if let Some(player_idx) = maybe_idx {
            let (name, pos, ca, cond, tech, mental, phys) = &players[*player_idx];
            let fit_score =
                calculate_position_fit(positions[pos_idx], pos, tech, mental, phys, *ca, *cond);
            starters.push(PlayerScore {
                player_idx: *player_idx,
                name: name.clone(),
                position: positions[pos_idx].to_string(),
                ca: *ca,
                condition: *cond,
                fit_score,
                total_score: fit_score,
            });
        }
    }

    // 벤치 선택: 남은 선수 중 CA * 컨디션 순
    let mut remaining: Vec<(usize, f32)> = players
        .iter()
        .enumerate()
        .filter(|(idx, _)| !used_players.contains(idx))
        .map(|(idx, (_, _, ca, cond, _, _, _))| (idx, *ca as f32 * cond))
        .collect();
    remaining.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (bench_indices, reserve_indices): (Vec<_>, Vec<_>) =
        remaining.iter().enumerate().partition(|(i, _)| *i < 7);

    let substitutes: Vec<PlayerScore> = bench_indices
        .iter()
        .map(|(_, (idx, _))| {
            let (name, pos, ca, cond, _, _, _) = &players[*idx];
            PlayerScore {
                player_idx: *idx,
                name: name.clone(),
                position: pos.clone(),
                ca: *ca,
                condition: *cond,
                fit_score: 0.0,
                total_score: *ca as f32 * cond,
            }
        })
        .collect();

    let reserves: Vec<PlayerScore> = reserve_indices
        .iter()
        .map(|(_, (idx, _))| {
            let (name, pos, ca, cond, _, _, _) = &players[*idx];
            PlayerScore {
                player_idx: *idx,
                name: name.clone(),
                position: pos.clone(),
                ca: *ca,
                condition: *cond,
                fit_score: 0.0,
                total_score: *ca as f32 * cond,
            }
        })
        .collect();

    SquadSelection {
        starters,
        substitutes,
        reserves,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formation_positions() {
        let positions = get_formation_positions("T442");
        assert_eq!(positions.len(), 11);
        assert_eq!(positions[0], "GK");
    }

    #[test]
    fn test_position_penalty() {
        assert_eq!(calculate_position_penalty("ST", "ST"), 0.0);
        assert_eq!(calculate_position_penalty("ST", "CF"), 0.2); // Same forwards category
        assert!(calculate_position_penalty("GK", "ST") >= 1.0);
    }
}
