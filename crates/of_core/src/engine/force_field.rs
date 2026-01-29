//! Force Field Navigation System
//!
//! FIX_2601/0112: Google Football 스타일 자기장 기반 이동 결정
//!
//! 드리블 방향을 결정할 때 사용:
//! - 골 방향으로 유인 (Attract)
//! - 상대 선수로부터 반발 (Repel)
//! - 측면 라인으로부터 반발 (Repel)

use super::physics_constants::field;

/// 힘의 유형
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForceType {
    /// 해당 지점으로 끌어당김
    Attract,
    /// 해당 지점으로부터 밀어냄
    Repel,
}

/// 힘의 감쇠 유형
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecayType {
    /// 거리와 무관하게 일정한 힘
    Constant,
    /// 거리에 따라 선형 감소
    Linear,
    /// 거리에 따라 지수 감소
    Exponential,
}

/// 힘 발생점 (Force Spot)
///
/// 특정 위치에서 힘을 발생시키는 점
#[derive(Debug, Clone)]
pub struct ForceSpot {
    /// 힘의 원점 (x, y) in meters
    pub origin: (f32, f32),
    /// 힘의 유형 (유인/반발)
    pub force_type: ForceType,
    /// 감쇠 유형
    pub decay: DecayType,
    /// 힘의 강도 (기본값 1.0)
    pub power: f32,
    /// 감쇠 스케일 (meters) - Linear/Exponential에서 사용
    pub scale: f32,
}

impl ForceSpot {
    /// 새 ForceSpot 생성
    pub fn new(
        origin: (f32, f32),
        force_type: ForceType,
        decay: DecayType,
        power: f32,
        scale: f32,
    ) -> Self {
        Self { origin, force_type, decay, power, scale }
    }

    /// 골 유인력 생성 (Constant)
    /// FIX_2601/0112: 골 유인력은 상대 반발력보다 강해야 함 (2.0 > 1.5)
    pub fn goal_attract(goal_pos: (f32, f32)) -> Self {
        Self::new(goal_pos, ForceType::Attract, DecayType::Constant, 2.0, 50.0)
    }

    /// 상대 선수 반발력 생성 (Exponential)
    /// FIX_2601/0112: 가까울수록 강한 반발 (5m에서 약 0.8)
    pub fn opponent_repel(opponent_pos: (f32, f32)) -> Self {
        Self::new(opponent_pos, ForceType::Repel, DecayType::Exponential, 1.5, 6.0)
    }

    /// 측면 라인 반발력 생성 (Linear)
    pub fn sideline_repel(player_x: f32, y_line: f32) -> Self {
        Self::new((player_x, y_line), ForceType::Repel, DecayType::Linear, 3.0, 15.0)
    }

    /// 특정 위치에서의 힘 벡터 계산
    ///
    /// # Arguments
    /// - `pos`: 힘을 받는 위치 (x, y) in meters
    ///
    /// # Returns
    /// 힘 벡터 (fx, fy)
    pub fn calculate_force(&self, pos: (f32, f32)) -> (f32, f32) {
        let dx = self.origin.0 - pos.0;
        let dy = self.origin.1 - pos.1;
        let dist = (dx * dx + dy * dy).sqrt();

        // 너무 가까우면 0 반환 (발산 방지)
        if dist < 0.001 {
            return (0.0, 0.0);
        }

        // 감쇠에 따른 힘의 크기 계산
        let strength = match self.decay {
            DecayType::Constant => self.power,
            DecayType::Linear => self.power * (1.0 - dist / self.scale).max(0.0),
            DecayType::Exponential => self.power * (-dist / self.scale).exp(),
        };

        // 방향 벡터 (정규화)
        let dir = match self.force_type {
            ForceType::Attract => (dx / dist, dy / dist),
            ForceType::Repel => (-dx / dist, -dy / dist),
        };

        (dir.0 * strength, dir.1 * strength)
    }
}

/// Force Field 기반 드리블 방향 계산
///
/// # Arguments
/// - `player_pos`: 플레이어 위치 (x, y) in meters
/// - `goal_pos`: 목표 골문 위치 (x, y) in meters
/// - `opponents`: 상대 선수 위치 목록
///
/// # Returns
/// 정규화된 드리블 방향 벡터 (dx, dy)
pub fn calculate_dribble_direction(
    player_pos: (f32, f32),
    goal_pos: (f32, f32),
    opponents: &[(f32, f32)],
) -> (f32, f32) {
    let mut force_spots = Vec::with_capacity(opponents.len() + 3);

    // 1. 골 유인력
    force_spots.push(ForceSpot::goal_attract(goal_pos));

    // 2. 상대 선수 반발력 (가까운 선수만, 최대 5명)
    let mut nearby_opponents: Vec<_> = opponents
        .iter()
        .map(|&opp| {
            let dx = opp.0 - player_pos.0;
            let dy = opp.1 - player_pos.1;
            let dist_sq = dx * dx + dy * dy;
            (opp, dist_sq)
        })
        .filter(|(_, dist_sq)| *dist_sq < 400.0) // 20m 이내
        .collect();

    nearby_opponents.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    for (opp, _) in nearby_opponents.into_iter().take(5) {
        force_spots.push(ForceSpot::opponent_repel(opp));
    }

    // 3. 측면 라인 반발력 (y=0, y=68)
    force_spots.push(ForceSpot::sideline_repel(player_pos.0, 0.0));
    force_spots.push(ForceSpot::sideline_repel(player_pos.0, field::WIDTH_M));

    // 4. 골라인 반발력 (자기 진영 골라인만)
    let own_goal_x = if goal_pos.0 > field::CENTER_X { 0.0 } else { field::LENGTH_M };
    if (player_pos.0 - own_goal_x).abs() < 20.0 {
        force_spots.push(ForceSpot::new(
            (own_goal_x, player_pos.1),
            ForceType::Repel,
            DecayType::Linear,
            2.0,
            20.0,
        ));
    }

    // 힘 합산
    let total: (f32, f32) = force_spots
        .iter()
        .map(|spot| spot.calculate_force(player_pos))
        .fold((0.0, 0.0), |acc, f| (acc.0 + f.0, acc.1 + f.1));

    // 정규화
    let mag = (total.0 * total.0 + total.1 * total.1).sqrt();
    if mag > 0.001 {
        (total.0 / mag, total.1 / mag)
    } else {
        // 기본값: 골 방향
        // FIX_2601/0117: Use goal direction as fallback instead of hardcoded (1.0, 0.0)
        let dx = goal_pos.0 - player_pos.0;
        let dy = goal_pos.1 - player_pos.1;
        let d = (dx * dx + dy * dy).sqrt();
        if d > 0.001 {
            (dx / d, dy / d)
        } else {
            // Fallback when player is at goal: use direction based on goal position
            // Goal at x=0 → attack left (-1.0), Goal at x>50 → attack right (1.0)
            let fallback_x = if goal_pos.0 < field::CENTER_X { -1.0 } else { 1.0 };
            (fallback_x, 0.0)
        }
    }
}

/// Force Field 기반 드리블 방향 계산 (단순 버전)
///
/// 상대 선수 정보 없이 골과 측면만 고려
pub fn calculate_dribble_direction_simple(
    player_pos: (f32, f32),
    goal_pos: (f32, f32),
) -> (f32, f32) {
    calculate_dribble_direction(player_pos, goal_pos, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_spot_attract() {
        let spot = ForceSpot::goal_attract((field::LENGTH_M, field::CENTER_Y));
        let force = spot.calculate_force((50.0, field::CENTER_Y));

        // 오른쪽(골 방향)으로 유인
        assert!(force.0 > 0.0);
        assert!(force.1.abs() < 0.01); // y 방향 힘 거의 없음
    }

    #[test]
    fn test_force_spot_repel() {
        let spot = ForceSpot::opponent_repel((55.0, field::CENTER_Y));
        let force = spot.calculate_force((50.0, field::CENTER_Y));

        // 왼쪽(상대 반대 방향)으로 반발
        assert!(force.0 < 0.0);
    }

    #[test]
    fn test_dribble_direction_no_opponents() {
        let dir = calculate_dribble_direction(
            (50.0, field::CENTER_Y),  // 중앙
            (field::LENGTH_M, field::CENTER_Y), // 오른쪽 골
            &[],
        );

        // 오른쪽으로 이동
        assert!(dir.0 > 0.5);
        assert!(dir.1.abs() < 0.3);
    }

    #[test]
    fn test_dribble_direction_with_opponent_ahead() {
        // 상대가 약간 위쪽에 있을 때 아래로 회피
        let dir = calculate_dribble_direction(
            (50.0, field::CENTER_Y),    // 중앙
            (field::LENGTH_M, field::CENTER_Y),   // 오른쪽 골
            &[(55.0, 36.0)], // 정면 약간 위에 상대
        );

        // 오른쪽으로 이동하되, 아래(y 감소)로 회피
        assert!(dir.0 > 0.0, "전진 방향 유지");
        assert!(dir.1 < -0.05, "아래로 회피 (상대가 위에 있으므로)");

        // 상대가 약간 아래쪽에 있을 때 위로 회피
        let dir2 = calculate_dribble_direction(
            (50.0, field::CENTER_Y),
            (field::LENGTH_M, field::CENTER_Y),
            &[(55.0, 32.0)], // 정면 약간 아래에 상대
        );
        assert!(dir2.0 > 0.0, "전진 방향 유지");
        assert!(dir2.1 > 0.05, "위로 회피 (상대가 아래에 있으므로)");
    }

    #[test]
    fn test_dribble_direction_near_sideline() {
        let dir = calculate_dribble_direction(
            (50.0, 5.0),   // 측면 근처 (y=5)
            (field::LENGTH_M, field::CENTER_Y), // 오른쪽 골
            &[],
        );

        // 측면에서 멀어지는 방향 (y 증가)
        assert!(dir.1 > 0.0);
    }

    #[test]
    fn test_decay_types() {
        let player_pos = (50.0, field::CENTER_Y);

        // Constant: 거리와 무관
        let constant =
            ForceSpot::new((60.0, field::CENTER_Y), ForceType::Attract, DecayType::Constant, 1.0, 10.0);
        let f1 = constant.calculate_force(player_pos);
        let f2 = constant.calculate_force((55.0, field::CENTER_Y));
        assert!((f1.0 - f2.0).abs() < 0.01);

        // Linear: 거리에 따라 감소
        let linear = ForceSpot::new((60.0, field::CENTER_Y), ForceType::Attract, DecayType::Linear, 1.0, 20.0);
        let f1 = linear.calculate_force(player_pos);
        let f2 = linear.calculate_force((55.0, field::CENTER_Y)); // 더 가까움
        assert!(f2.0 > f1.0); // 가까울수록 강함

        // Exponential: 거리에 따라 급격히 감소
        let exp =
            ForceSpot::new((60.0, field::CENTER_Y), ForceType::Attract, DecayType::Exponential, 1.0, 5.0);
        let f1 = exp.calculate_force(player_pos);
        let f2 = exp.calculate_force((55.0, field::CENTER_Y));
        assert!(f2.0 > f1.0);
    }
}
