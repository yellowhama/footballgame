//! Hero Growth System
//!
//! Phase 5: Hero Time 액션을 선수 스탯 성장에 연결
//!
//! ## 모듈 구조
//! - `hero_action_tag`: HeroActionTag, HeroXpEvent 정의
//! - `xp_calculator`: XP 계산 로직
//! - `xp_bucket`: 경기 중 XP 누적
//! - `match_growth`: XP → 스탯 변환
//!
//! ## 사용 흐름
//! 1. Hero Time 액션 발생 → HeroXpEvent 생성
//! 2. HeroXpBucket.add_event() 호출
//! 3. 경기 종료 시 HeroMatchGrowth::from_bucket() 호출
//! 4. MyPlayer.apply_match_growth() 호출

pub mod hero_action_tag;
pub mod match_growth;
pub mod xp_bucket;
pub mod xp_calculator;

// Re-exports for convenience
pub use hero_action_tag::{HeroActionTag, HeroXpEvent, PlayerAttribute};
pub use match_growth::{growth_threshold, HeroMatchGrowth};
pub use xp_bucket::HeroXpBucket;
pub use xp_calculator::{
    calculate_dribble_difficulty, calculate_pass_difficulty, calculate_pressure, calculate_xp,
};
