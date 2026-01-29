//! XP Bucket
//!
//! Phase 5: 경기 중 XP 누적 버킷
//!
//! 각 스탯별로 XP를 누적하고, 경기 종료 시 스탯 성장으로 변환

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::hero_action_tag::{HeroXpEvent, PlayerAttribute};
use super::xp_calculator::calculate_xp;

/// 경기 중 XP 누적 버킷
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeroXpBucket {
    /// 스탯별 누적 XP
    accumulated: HashMap<PlayerAttribute, f32>,
    /// 이벤트 히스토리 (디버깅/리플레이용)
    events: Vec<HeroXpEvent>,
    /// 총 이벤트 수
    total_events: usize,
    /// 성공한 이벤트 수
    successful_events: usize,
}

impl HeroXpBucket {
    /// 새 버킷 생성
    pub fn new() -> Self {
        Self::default()
    }

    /// XP 이벤트 추가
    ///
    /// 이벤트의 XP를 계산하여 관련 스탯에 분배
    pub fn add_event(&mut self, event: &HeroXpEvent) {
        let total_xp = calculate_xp(event);

        // 태그의 영향 스탯에 XP 분배
        for (attr, weight) in event.tag.affected_attributes() {
            let attr_xp = total_xp * weight;
            *self.accumulated.entry(attr).or_default() += attr_xp;
        }

        // 통계 업데이트
        self.events.push(event.clone());
        self.total_events += 1;
        if event.success {
            self.successful_events += 1;
        }
    }

    /// 특정 스탯의 누적 XP 조회
    pub fn get_xp(&self, attr: PlayerAttribute) -> f32 {
        *self.accumulated.get(&attr).unwrap_or(&0.0)
    }

    /// 모든 누적 XP 조회
    pub fn get_all_xp(&self) -> &HashMap<PlayerAttribute, f32> {
        &self.accumulated
    }

    /// 총 누적 XP
    pub fn total_xp(&self) -> f32 {
        self.accumulated.values().sum()
    }

    /// 이벤트 히스토리 조회
    pub fn events(&self) -> &[HeroXpEvent] {
        &self.events
    }

    /// 총 이벤트 수
    pub fn total_events(&self) -> usize {
        self.total_events
    }

    /// 성공률
    pub fn success_rate(&self) -> f32 {
        if self.total_events == 0 {
            0.0
        } else {
            self.successful_events as f32 / self.total_events as f32
        }
    }

    /// 버킷 초기화 (새 경기 시작)
    pub fn clear(&mut self) {
        self.accumulated.clear();
        self.events.clear();
        self.total_events = 0;
        self.successful_events = 0;
    }

    /// 이전 경기에서 이월된 XP 적용
    pub fn apply_overflow(&mut self, overflow: &HashMap<PlayerAttribute, f32>) {
        for (attr, xp) in overflow {
            *self.accumulated.entry(*attr).or_default() += xp;
        }
    }

    /// 훈련 시너지 보너스 적용
    ///
    /// 최근 훈련한 스탯에 대해 XP 보너스
    pub fn apply_training_synergy(&mut self, trained_attrs: &[PlayerAttribute], bonus_rate: f32) {
        for attr in trained_attrs {
            if let Some(xp) = self.accumulated.get_mut(attr) {
                *xp *= 1.0 + bonus_rate;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::growth::hero_action_tag::HeroActionTag;

    #[test]
    fn test_empty_bucket() {
        let bucket = HeroXpBucket::new();
        assert_eq!(bucket.total_events(), 0);
        assert_eq!(bucket.total_xp(), 0.0);
        assert_eq!(bucket.success_rate(), 0.0);
    }

    #[test]
    fn test_add_event() {
        let mut bucket = HeroXpBucket::new();

        let event = HeroXpEvent::new(HeroActionTag::ThroughPass, true, 45)
            .with_pressure(0.5)
            .with_difficulty(0.5);

        bucket.add_event(&event);

        assert_eq!(bucket.total_events(), 1);
        assert!(bucket.total_xp() > 0.0);
        assert!(bucket.get_xp(PlayerAttribute::Passing) > 0.0);
        assert!(bucket.get_xp(PlayerAttribute::Vision) > 0.0);
    }

    #[test]
    fn test_success_rate() {
        let mut bucket = HeroXpBucket::new();

        bucket.add_event(&HeroXpEvent::new(HeroActionTag::SafePass, true, 10));
        bucket.add_event(&HeroXpEvent::new(HeroActionTag::SafePass, true, 20));
        bucket.add_event(&HeroXpEvent::new(HeroActionTag::SafePass, false, 30));

        assert!((bucket.success_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_training_synergy() {
        let mut bucket = HeroXpBucket::new();
        bucket.add_event(&HeroXpEvent::new(HeroActionTag::SafePass, true, 10));

        let passing_before = bucket.get_xp(PlayerAttribute::Passing);

        bucket.apply_training_synergy(&[PlayerAttribute::Passing], 0.3);

        let passing_after = bucket.get_xp(PlayerAttribute::Passing);

        assert!((passing_after / passing_before - 1.3).abs() < 0.01);
    }

    #[test]
    fn test_clear() {
        let mut bucket = HeroXpBucket::new();
        bucket.add_event(&HeroXpEvent::new(HeroActionTag::SafePass, true, 10));

        bucket.clear();

        assert_eq!(bucket.total_events(), 0);
        assert_eq!(bucket.total_xp(), 0.0);
    }
}
