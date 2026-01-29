//! Team Coordinator
//!
//! FIX_2601/0108: Off-Ball 액션 충돌 방지
//! 같은 타겟을 2명 이상이 담당하는 것을 방지

use std::collections::HashMap;

use super::types::{Action, PlayerId, ScoredAction, Zone};

/// 팀 조율 시스템
#[derive(Debug, Default)]
pub struct TeamCoordinator {
    /// 타겟 → 담당 선수 매핑 (Press, Mark)
    claimed_targets: HashMap<PlayerId, PlayerId>,

    /// 공간 → 담당 선수 매핑 (Run, Cover)
    claimed_spaces: HashMap<Zone, PlayerId>,

    /// 존별 커버 수
    cover_counts: HashMap<Zone, u32>,

    /// 현재 볼캐리어 ID (Press 중복 방지용)
    ball_carrier_id: Option<PlayerId>,
}

impl TeamCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// 새 틱 시작 시 초기화
    pub fn reset(&mut self) {
        self.claimed_targets.clear();
        self.claimed_spaces.clear();
        self.cover_counts.clear();
        self.ball_carrier_id = None;
    }

    /// 현재 볼캐리어 설정 (Press 중복 방지용)
    pub fn set_ball_carrier(&mut self, ball_carrier_id: PlayerId) {
        self.ball_carrier_id = Some(ball_carrier_id);
    }

    /// 충돌 페널티 적용
    ///
    /// 이미 다른 선수가 담당한 타겟/공간이면 점수에 페널티
    pub fn apply_conflict_penalty(&self, action: &mut ScoredAction) {
        let penalty = self.calculate_conflict_penalty(&action.action);
        action.weighted_total *= penalty;
    }

    /// 충돌 페널티 계산
    fn calculate_conflict_penalty(&self, action: &Action) -> f32 {
        match action {
            // Press/Mark: 같은 타겟을 이미 다른 선수가 담당
            Action::Press | Action::Mark { .. } => {
                if let Some(target_id) = self.get_target_id(action) {
                    if self.claimed_targets.contains_key(&target_id) {
                        return 0.3; // 70% 페널티
                    }
                }
            }

            // Run: 같은 공간을 이미 다른 선수가 런
            Action::RunIntoSpace { target } | Action::RunSupport { target_space: target } => {
                let zone = Zone {
                    x: (target.x / 10.5) as i8,
                    y: (target.y / 9.7) as i8,
                };
                if self.claimed_spaces.contains_key(&zone) {
                    return 0.3; // 70% 페널티
                }
            }

            // Cover: 같은 존에 2명 이상이면 약간의 페널티
            Action::Cover { zone } | Action::CoverEmergency { zone } => {
                if let Some(&count) = self.cover_counts.get(zone) {
                    if count >= 2 {
                        return 0.7; // 30% 페널티
                    }
                }
            }

            _ => {}
        }

        1.0 // 페널티 없음
    }

    /// 액션 예약 (담당 등록)
    pub fn claim(&mut self, action: &Action, player_id: PlayerId) {
        match action {
            Action::Press | Action::Mark { .. } => {
                if let Some(target_id) = self.get_target_id(action) {
                    self.claimed_targets.insert(target_id, player_id);
                }
            }

            Action::RunIntoSpace { target } | Action::RunSupport { target_space: target } => {
                let zone = Zone {
                    x: (target.x / 10.5) as i8,
                    y: (target.y / 9.7) as i8,
                };
                self.claimed_spaces.insert(zone, player_id);
            }

            Action::Cover { zone } | Action::CoverEmergency { zone } => {
                *self.cover_counts.entry(*zone).or_insert(0) += 1;
            }

            _ => {}
        }
    }

    /// 타겟이 이미 예약되었는지 확인
    pub fn is_target_claimed(&self, target_id: PlayerId) -> bool {
        self.claimed_targets.contains_key(&target_id)
    }

    /// 공간이 이미 예약되었는지 확인
    pub fn is_space_claimed(&self, zone: &Zone) -> bool {
        self.claimed_spaces.contains_key(zone)
    }

    /// 액션에서 타겟 ID 추출
    fn get_target_id(&self, action: &Action) -> Option<PlayerId> {
        match action {
            Action::Mark { target_id } => Some(*target_id),
            // Press는 볼캐리어를 타겟으로 함
            // set_ball_carrier()로 미리 설정해야 함
            Action::Press => self.ball_carrier_id,
            _ => None,
        }
    }

    /// 현재 담당 현황 요약
    pub fn summary(&self) -> CoordinatorSummary {
        CoordinatorSummary {
            claimed_targets: self.claimed_targets.len(),
            claimed_spaces: self.claimed_spaces.len(),
            total_covers: self.cover_counts.values().sum(),
        }
    }
}

/// 조율 현황 요약
#[derive(Debug, Clone)]
pub struct CoordinatorSummary {
    pub claimed_targets: usize,
    pub claimed_spaces: usize,
    pub total_covers: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::action_evaluator::types::{ActionScore, ActionWeights, PlayerId, Position};

    #[test]
    fn test_target_claiming() {
        let mut coord = TeamCoordinator::new();
        let target = PlayerId::new(10);
        let claimer = PlayerId::new(5);

        // 처음엔 예약 안 됨
        assert!(!coord.is_target_claimed(target));

        // Mark 예약
        let action = Action::Mark { target_id: target };
        coord.claim(&action, claimer);

        // 이제 예약됨
        assert!(coord.is_target_claimed(target));
    }

    #[test]
    fn test_conflict_penalty() {
        let mut coord = TeamCoordinator::new();
        let target = PlayerId::new(10);

        // 첫 번째 선수가 마킹
        coord.claim(&Action::Mark { target_id: target }, PlayerId::new(5));

        // 두 번째 선수가 같은 타겟 마킹 시도
        let score = ActionScore {
            distance: 0.8,
            safety: 0.7,
            readiness: 0.6,
            progression: 0.5,
            space: 0.4,
            tactical: 0.3,
        };
        let mut scored = ScoredAction::new(
            Action::Mark { target_id: target },
            score,
            &ActionWeights::default(),
        );
        let original_total = scored.weighted_total;

        coord.apply_conflict_penalty(&mut scored);

        // 페널티 적용됨
        assert!(scored.weighted_total < original_total);
        assert!((scored.weighted_total - original_total * 0.3).abs() < 0.001);
    }

    #[test]
    fn test_cover_penalty() {
        let mut coord = TeamCoordinator::new();
        let zone = Zone { x: 5, y: 3 };

        // 2명이 같은 존 커버
        coord.claim(&Action::Cover { zone }, PlayerId::new(4));
        coord.claim(&Action::Cover { zone }, PlayerId::new(5));

        // 3번째 선수 시도
        let score = ActionScore {
            distance: 0.8,
            safety: 0.7,
            readiness: 0.6,
            progression: 0.5,
            space: 0.4,
            tactical: 0.3,
        };
        let mut scored = ScoredAction::new(
            Action::Cover { zone },
            score,
            &ActionWeights::default(),
        );
        let original_total = scored.weighted_total;

        coord.apply_conflict_penalty(&mut scored);

        // 약한 페널티 (0.7)
        assert!(scored.weighted_total < original_total);
    }

    #[test]
    fn test_reset() {
        let mut coord = TeamCoordinator::new();

        coord.claim(&Action::Mark { target_id: PlayerId::new(10) }, PlayerId::new(5));
        assert_eq!(coord.summary().claimed_targets, 1);

        coord.reset();
        assert_eq!(coord.summary().claimed_targets, 0);
    }

    #[test]
    fn test_press_duplication_prevention() {
        let mut coord = TeamCoordinator::new();
        let ball_carrier = PlayerId::new(11);

        // 볼캐리어 설정
        coord.set_ball_carrier(ball_carrier);

        // 첫 번째 선수가 Press
        coord.claim(&Action::Press, PlayerId::new(5));
        assert!(coord.is_target_claimed(ball_carrier));

        // 두 번째 선수가 같은 볼캐리어 Press 시도
        let score = ActionScore {
            distance: 0.8,
            safety: 0.7,
            readiness: 0.6,
            progression: 0.5,
            space: 0.4,
            tactical: 0.3,
        };
        let mut scored = ScoredAction::new(Action::Press, score, &ActionWeights::default());
        let original_total = scored.weighted_total;

        coord.apply_conflict_penalty(&mut scored);

        // Press 중복 페널티 적용됨
        assert!(scored.weighted_total < original_total);
        assert!((scored.weighted_total - original_total * 0.3).abs() < 0.001);
    }
}
