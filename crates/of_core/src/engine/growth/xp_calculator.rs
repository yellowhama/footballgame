//! XP Calculator
//!
//! Phase 5: XP 계산 로직
//!
//! ## 핵심 공식
//! XP = base_xp × success_mult × pressure_bonus × fatigue_mod × difficulty_bonus

use super::hero_action_tag::HeroXpEvent;

/// 단일 XP 이벤트의 포인트 계산
///
/// # Arguments
/// - `event`: XP 이벤트 정보
///
/// # Returns
/// 계산된 XP 값
pub fn calculate_xp(event: &HeroXpEvent) -> f32 {
    let base_xp = event.tag.base_xp();

    // 성공/실패 보정 (실패해도 일부 XP 획득 - 학습 효과)
    let success_mult = if event.success { 1.0 } else { 0.3 };

    // 압박 상황 보너스 (위험할수록 더 많은 XP)
    let pressure_bonus = 1.0 + event.pressure_level * 0.5;

    // 피로 보정
    // - 성공 시: 피로할수록 XP 감소 (컨디션 영향)
    // - 실패 시: 피로 중 실패는 학습 효과 있음
    let fatigue_mod = if event.success {
        1.0 - event.fatigue_level * 0.2
    } else {
        1.0 + event.fatigue_level * 0.1
    };

    // 상황 난이도 보너스
    let difficulty_bonus = 1.0 + event.context_difficulty * 0.3;

    base_xp * success_mult * pressure_bonus * fatigue_mod * difficulty_bonus
}

/// 주변 수비수로부터 압박 정도 계산
///
/// # Arguments
/// - `player_pos`: 플레이어 위치 (미터)
/// - `opponents`: 상대 선수들 위치 (미터)
///
/// # Returns
/// 압박 레벨 (0.0 ~ 1.0)
pub fn calculate_pressure(player_pos: (f32, f32), opponents: &[(f32, f32)]) -> f32 {
    const PRESSURE_RADIUS: f32 = 5.0; // 5m 이내
    let mut pressure = 0.0;

    for opp_pos in opponents {
        let dx = player_pos.0 - opp_pos.0;
        let dy = player_pos.1 - opp_pos.1;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < PRESSURE_RADIUS {
            pressure += (PRESSURE_RADIUS - dist) / PRESSURE_RADIUS;
        }
    }

    // 최대 2명 수비수의 영향 (1.0 cap)
    (pressure / 2.0).min(1.0)
}

/// 패스 상황 난이도 계산
///
/// # Arguments
/// - `from`: 패서 위치 (미터)
/// - `to`: 수신자 위치 (미터)
/// - `defenders`: 수비수들 위치 (미터)
///
/// # Returns
/// 난이도 레벨 (0.0 ~ 1.0)
pub fn calculate_pass_difficulty(
    from: (f32, f32),
    to: (f32, f32),
    defenders: &[(f32, f32)],
) -> f32 {
    // 거리 요인 (30m 이상이면 최대)
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let dist = (dx * dx + dy * dy).sqrt();
    let dist_factor = (dist / 30.0).min(1.0);

    // 패스 라인 위 수비수 수
    let blocked_count = count_defenders_in_lane(from, to, defenders);
    let blocked_factor = (blocked_count as f32 * 0.3).min(0.7);

    (dist_factor * 0.5 + blocked_factor).min(1.0)
}

/// 패스 라인 위의 수비수 수 계산
fn count_defenders_in_lane(from: (f32, f32), to: (f32, f32), defenders: &[(f32, f32)]) -> usize {
    const LANE_WIDTH: f32 = 2.0; // 2m 폭

    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let length = (dx * dx + dy * dy).sqrt();

    if length < 0.1 {
        return 0;
    }

    // 패스 방향 정규화
    let dir_x = dx / length;
    let dir_y = dy / length;

    let mut count = 0;
    for def in defenders {
        // 수비수에서 패스 라인까지의 수직 거리
        let to_def_x = def.0 - from.0;
        let to_def_y = def.1 - from.1;

        // 패스 라인에 투영
        let proj = to_def_x * dir_x + to_def_y * dir_y;

        // 패스 구간 내에 있는지
        if proj > 0.0 && proj < length {
            // 수직 거리 계산
            let perp_x = to_def_x - proj * dir_x;
            let perp_y = to_def_y - proj * dir_y;
            let perp_dist = (perp_x * perp_x + perp_y * perp_y).sqrt();

            if perp_dist < LANE_WIDTH {
                count += 1;
            }
        }
    }

    count
}

/// 드리블 난이도 계산
pub fn calculate_dribble_difficulty(
    _player_pos: (f32, f32),
    nearest_defender_dist: f32,
    is_aggressive: bool,
) -> f32 {
    // 수비수와의 거리가 가까울수록 어려움
    let dist_factor =
        if nearest_defender_dist > 5.0 { 0.0 } else { (5.0 - nearest_defender_dist) / 5.0 };

    // 공격적 드리블은 추가 난이도
    let aggressive_bonus = if is_aggressive { 0.2 } else { 0.0 };

    (dist_factor + aggressive_bonus).min(1.0)
}

/// 슈팅 난이도 계산
pub fn calculate_shot_difficulty(distance_to_goal: f32, angle_to_goal: f32, pressure: f32) -> f32 {
    // 거리 요인 (25m 이상이면 최대)
    let dist_factor = (distance_to_goal / 25.0).min(1.0) * 0.4;

    // 각도 요인 (좁은 각도일수록 어려움, 0-45도 범위)
    let angle_factor = (1.0 - angle_to_goal / 45.0).max(0.0) * 0.3;

    // 압박 요인
    let pressure_factor = pressure * 0.3;

    (dist_factor + angle_factor + pressure_factor).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;
    use crate::engine::growth::hero_action_tag::HeroActionTag;

    #[test]
    fn test_calculate_xp_success() {
        let event = HeroXpEvent::new(HeroActionTag::ThroughPass, true, 45)
            .with_pressure(0.6)
            .with_fatigue(0.3)
            .with_difficulty(0.5);

        let xp = calculate_xp(&event);

        // base(5.0) × success(1.0) × pressure(1.3) × fatigue(0.94) × difficulty(1.15)
        // ≈ 5.0 × 1.3 × 0.94 × 1.15 ≈ 7.03
        assert!(xp > 6.5, "XP should be above 6.5, got {}", xp);
        assert!(xp < 8.0, "XP should be below 8.0, got {}", xp);
    }

    #[test]
    fn test_calculate_xp_failure() {
        let event = HeroXpEvent::new(HeroActionTag::ThroughPass, false, 45);
        let xp = calculate_xp(&event);

        // base(5.0) × failure(0.3) = 1.5
        assert!(xp > 1.0, "Failed action should give some XP");
        assert!(xp < 2.0, "Failed action XP should be reduced");
    }

    #[test]
    fn test_calculate_pressure() {
        // 수비수 없음
        let pressure = calculate_pressure((50.0, 35.0), &[]);
        assert!((pressure - 0.0).abs() < 0.001);

        // 수비수 2.5m 거리에 1명
        let pressure = calculate_pressure((50.0, 35.0), &[(field::CENTER_X, 35.0)]);       
        assert!(pressure > 0.2, "Should have some pressure");
        assert!(pressure < 0.6, "Pressure not too high with 1 defender");

        // 수비수 2명 가까이
        let pressure = calculate_pressure((50.0, 35.0), &[(51.0, 35.0), (50.0, 36.0)]);
        assert!(pressure > 0.6, "Should have high pressure with 2 defenders");
    }

    #[test]
    fn test_calculate_pass_difficulty() {
        // 짧은 패스, 수비수 없음
        let diff = calculate_pass_difficulty((50.0, 35.0), (55.0, 35.0), &[]);
        assert!(diff < 0.2, "Short pass with no defenders should be easy");

        // 긴 패스
        let diff = calculate_pass_difficulty((20.0, 35.0), (70.0, 35.0), &[]);
        assert!(diff > 0.4, "Long pass should be harder");

        // 수비수가 라인 위에
        let diff = calculate_pass_difficulty((50.0, 35.0), (60.0, 35.0), &[(55.0, 35.0)]);
        assert!(diff > 0.3, "Pass through defender should be hard");
    }

    #[test]
    fn test_calculate_dribble_difficulty() {
        // 수비수 멀리
        let diff = calculate_dribble_difficulty((50.0, 35.0), 10.0, false);
        assert!(diff < 0.1);

        // 수비수 가까이 + 공격적 드리블
        let diff = calculate_dribble_difficulty((50.0, 35.0), 2.0, true);
        assert!(diff > 0.6);
    }
}
