//! Match Growth
//!
//! Phase 5: 경기 종료 시 XP → 스탯 성장 변환
//!
//! ## 성장 곡선
//! 스탯이 높을수록 성장에 더 많은 XP 필요
//!
//! | 현재 스탯 | XP 임계값 |
//! |----------|----------|
//! | 0-40     | 10       |
//! | 41-60    | 15       |
//! | 61-75    | 25       |
//! | 76-85    | 40       |
//! | 86-90    | 60       |
//! | 91-95    | 100      |
//! | 96-99    | 200      |

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::hero_action_tag::PlayerAttribute;
use super::xp_bucket::HeroXpBucket;

/// 스탯 +1 에 필요한 XP 임계값
///
/// 스탯이 높을수록 성장에 더 많은 XP 필요
pub fn growth_threshold(current_stat: i8) -> f32 {
    match current_stat {
        0..=40 => 10.0,
        41..=60 => 15.0,
        61..=75 => 25.0,
        76..=85 => 40.0,
        86..=90 => 60.0,
        91..=95 => 100.0,
        96..=99 => 200.0,
        _ => 500.0, // 99 초과 (실질적으로 불가능)
    }
}

/// 경기 종료 시 XP → 스탯 변환 결과
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeroMatchGrowth {
    /// 실제 스탯 증가량
    pub stat_gains: HashMap<PlayerAttribute, i8>,
    /// 다음 경기로 이월되는 XP (임계값 미만)
    pub xp_overflow: HashMap<PlayerAttribute, f32>,
    /// 총 획득 XP
    pub total_xp_earned: f32,
    /// UI 강조용 하이라이트 (성장한 스탯들)
    pub highlight_gains: Vec<(PlayerAttribute, i8)>,
}

impl HeroMatchGrowth {
    /// XP 버킷을 스탯 성장으로 변환
    ///
    /// # Arguments
    /// - `bucket`: 경기 중 누적된 XP 버킷
    /// - `current_stats`: 현재 선수 스탯 조회 함수
    ///
    /// # Returns
    /// 성장 결과 (증가량 + 이월 XP)
    pub fn from_bucket<F>(bucket: &HeroXpBucket, current_stats: F) -> Self
    where
        F: Fn(PlayerAttribute) -> i8,
    {
        let mut gains = HashMap::new();
        let mut overflow = HashMap::new();
        let mut highlight = Vec::new();
        let total = bucket.total_xp();

        for (attr, xp) in bucket.get_all_xp() {
            let current = current_stats(*attr);
            let threshold = growth_threshold(current);

            if *xp >= threshold {
                // 성장 포인트 계산 (최대 +3)
                let raw_points = (*xp / threshold).floor() as i8;
                let points = raw_points.min(3);
                let leftover = xp - (points as f32 * threshold);

                if points > 0 {
                    gains.insert(*attr, points);
                    highlight.push((*attr, points));
                }
                overflow.insert(*attr, leftover);
            } else {
                // 임계값 미만 → 전량 이월
                overflow.insert(*attr, *xp);
            }
        }

        // 하이라이트 정렬 (증가량 높은 순)
        highlight.sort_by(|a, b| b.1.cmp(&a.1));

        Self {
            stat_gains: gains,
            xp_overflow: overflow,
            total_xp_earned: total,
            highlight_gains: highlight,
        }
    }

    /// 스탯 성장이 있는지
    pub fn has_growth(&self) -> bool {
        !self.stat_gains.is_empty()
    }

    /// 특정 스탯의 성장량
    pub fn get_gain(&self, attr: PlayerAttribute) -> i8 {
        *self.stat_gains.get(&attr).unwrap_or(&0)
    }

    /// 특정 스탯의 이월 XP
    pub fn get_overflow(&self, attr: PlayerAttribute) -> f32 {
        *self.xp_overflow.get(&attr).unwrap_or(&0.0)
    }

    /// 총 성장 포인트
    pub fn total_gains(&self) -> i8 {
        self.stat_gains.values().sum()
    }

    /// 요약 텍스트 생성
    pub fn summary(&self) -> String {
        if self.highlight_gains.is_empty() {
            "경기에서 큰 성장은 없었지만, 경험치가 쌓였습니다.".to_string()
        } else {
            let gains: Vec<String> = self
                .highlight_gains
                .iter()
                .take(3)
                .map(|(attr, gain)| format!("{:?} +{}", attr, gain))
                .collect();
            format!("성장: {}", gains.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::growth::hero_action_tag::HeroActionTag;
    use crate::engine::growth::hero_action_tag::HeroXpEvent;

    #[test]
    fn test_growth_threshold() {
        assert_eq!(growth_threshold(30), 10.0);
        assert_eq!(growth_threshold(50), 15.0);
        assert_eq!(growth_threshold(70), 25.0);
        assert_eq!(growth_threshold(80), 40.0);
        assert_eq!(growth_threshold(88), 60.0);
        assert_eq!(growth_threshold(93), 100.0);
        assert_eq!(growth_threshold(98), 200.0);
    }

    #[test]
    fn test_match_growth_from_bucket() {
        let mut bucket = HeroXpBucket::new();

        // 많은 스루 패스 성공 → Passing, Vision 성장 예상
        for _ in 0..10 {
            bucket.add_event(
                &HeroXpEvent::new(HeroActionTag::ThroughPass, true, 45)
                    .with_pressure(0.5)
                    .with_difficulty(0.5),
            );
        }

        // 현재 스탯: 모두 50 (threshold 15)
        let growth = HeroMatchGrowth::from_bucket(&bucket, |_| 50);

        assert!(growth.has_growth());
        assert!(growth.get_gain(PlayerAttribute::Passing) > 0);
        assert!(growth.get_gain(PlayerAttribute::Vision) > 0);
    }

    #[test]
    fn test_growth_max_3_per_stat() {
        let mut bucket = HeroXpBucket::new();

        // 매우 많은 이벤트
        for _ in 0..100 {
            bucket.add_event(&HeroXpEvent::new(HeroActionTag::SafePass, true, 45));
        }

        let growth = HeroMatchGrowth::from_bucket(&bucket, |_| 30);

        // Passing 성장이 최대 3
        assert!(growth.get_gain(PlayerAttribute::Passing) <= 3);
    }

    #[test]
    fn test_overflow() {
        let mut bucket = HeroXpBucket::new();

        // 임계값 미만의 XP
        bucket.add_event(&HeroXpEvent::new(HeroActionTag::SafePass, true, 45));

        let growth = HeroMatchGrowth::from_bucket(&bucket, |_| 80); // threshold 40

        // 성장 없음
        assert!(!growth.has_growth());
        // 이월 XP 존재
        assert!(growth.get_overflow(PlayerAttribute::Passing) > 0.0);
    }

    #[test]
    fn test_summary() {
        let growth = HeroMatchGrowth::default();
        assert!(growth.summary().contains("성장은 없었지만"));

        let mut growth = HeroMatchGrowth::default();
        growth.highlight_gains.push((PlayerAttribute::Passing, 2));
        assert!(growth.summary().contains("Passing +2"));
    }
}
