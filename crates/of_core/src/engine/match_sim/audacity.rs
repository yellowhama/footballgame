//! # Audacity System (P10-13 Phase 4)
//!
//! **Logic + Desire = Human Decision**
//!
//! EV 기반 의사결정(P12)에 욕망/성향을 섞어서 인간적인 축구를 구현.
//!
//! ## 핵심 개념
//! - **Flair (창의성)**: 하이리스크 하이리턴 행동 선호
//! - **Audacity (대담함)**: 공격성 + (1 - 판단력)
//! - **Desperation (절박함)**: 지는 중 + 후반

use super::MatchEngine;

// ========== Constants (Tuning Points) ==========

/// Glory bonus scale (튜닝 포인트)
pub const GLORY_BONUS_SCALE: f32 = 0.8;

/// High reward threshold (골, 결정적 찬스 등)
pub const HIGH_REWARD_THRESHOLD: f32 = 0.7;

/// Low probability threshold (어려운 시도)
pub const LOW_PROB_THRESHOLD: f32 = 0.35;

/// Maximum risk dampening (위험 인식 왜곡 최대치)
pub const RISK_DAMPEN_MAX: f32 = 0.7;

/// Late game start minute (절박함 시작 시점)
pub const LATE_GAME_START_MINUTE: f32 = 70.0;

// ========== Types ==========

/// Audacity 적용 대상 액션 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AudacityActionKind {
    Shot,
    Pass,
    Dribble,
}

/// Audacity 계산을 위한 컨텍스트
#[derive(Debug, Clone)]
pub struct AudacityContext {
    /// 선수 Flair (0~1)
    pub flair: f32,
    /// 선수 Audacity (0~1)
    pub audacity: f32,
    /// 경기 상황 절박함 (0~1)
    pub desperation: f32,
}

// ========== Implementation ==========

impl MatchEngine {
    /// 선수의 AudacityContext 생성
    ///
    /// # Arguments
    /// * `player_idx` - 선수 인덱스 (0-21)
    ///
    /// # Returns
    /// AudacityContext with flair, audacity, desperation calculated
    pub fn get_audacity_context(&self, player_idx: usize) -> AudacityContext {
        use crate::models::TeamSide;
        let is_home = TeamSide::is_home(player_idx);

        // 선수 능력치 가져오기 (기존 헬퍼 함수 사용, 0-100 스케일)
        // player_attributes.rs에 정의된 함수들은 trait 보정 포함
        let flair = self.get_player_flair(player_idx) / 100.0;
        let aggression = self.get_player_aggression(player_idx) / 100.0;
        let decisions = self.get_player_decisions(player_idx) / 100.0;

        // Audacity: 공격성 + (1 - 판단력)
        let audacity = (aggression * 0.7 + (1.0 - decisions) * 0.3).clamp(0.0, 1.0);

        // Desperation: 지는 중 + 후반
        let minute = self.current_minute() as f32;
        let (gf, ga) = if is_home {
            (self.result.score_home as i32, self.result.score_away as i32)
        } else {
            (self.result.score_away as i32, self.result.score_home as i32)
        };

        // DPER Framework: Apply experimental losing boost
        let exp_losing_boost = self.exp_audacity_losing_boost();
        let losing = if gf < ga { 1.0 + exp_losing_boost } else { 0.0 };

        // DPER Framework: Apply experimental late game urgency
        let exp_late_game_urgency = self.exp_audacity_late_game_urgency();
        let late_game = ((minute - LATE_GAME_START_MINUTE) / 20.0).clamp(0.0, 1.0) * exp_late_game_urgency;

        let desperation = (losing * 0.7 + late_game * 0.3).clamp(0.0, 1.0);

        AudacityContext { flair, audacity, desperation }
    }

    /// Audacity 보정 적용
    ///
    /// rational_ev (P12)에 욕망/성향을 섞어서 최종 EV 반환
    ///
    /// # Arguments
    /// * `rational_ev` - P12에서 계산한 rational EV
    /// * `ctx` - AudacityContext (flair, audacity, desperation)
    /// * `base_prob` - 성공 확률 (xG, pass_success 등)
    /// * `base_reward` - 성공 시 보상
    /// * `base_risk` - 실패 시 비용
    ///
    /// # Returns
    /// Audacity 보정된 최종 EV
    pub fn apply_audacity_boost(
        &self,
        rational_ev: f32,
        ctx: &AudacityContext,
        base_prob: f32,
        base_reward: f32,
        base_risk: f32,
    ) -> f32 {
        // 1. High Risk / High Reward 판정
        let is_high_reward = base_reward > HIGH_REWARD_THRESHOLD;
        let is_low_prob = base_prob < LOW_PROB_THRESHOLD;

        // 2. Glory Bonus 계산
        let glory_bonus = if is_high_reward && is_low_prob {
            // 대담한 선수가 절박한 상황에서 하이리턴 노릴 때 보너스
            let drama = (ctx.flair * 0.6 + ctx.audacity * 0.4) * (0.5 + 0.5 * ctx.desperation);
            base_reward * drama * GLORY_BONUS_SCALE
        } else {
            0.0
        };

        // 3. Risk Dampening (위험 인식 왜곡)
        // 대담한 선수는 위험을 덜 느낌
        let risk_dampen =
            1.0 - (ctx.audacity * 0.4 + ctx.desperation * 0.3).clamp(0.0, RISK_DAMPEN_MAX);
        let perceived_risk = base_risk * risk_dampen;

        // 4. Audacity EV 계산
        let base_expected = base_prob * base_reward;
        let audacity_ev = base_expected + glory_bonus - (1.0 - base_prob) * perceived_risk;

        // 5. Rational EV와 블렌딩
        // alpha: 얼마나 audacity에 의존하는가
        let base_alpha = (ctx.flair * 0.5 + ctx.audacity * 0.3 + ctx.desperation * 0.2).clamp(0.1, 0.9);

        // DPER Framework: Apply experimental audacity scale
        let exp_scale = self.exp_audacity_scale();
        let alpha = (base_alpha * exp_scale).clamp(0.1, 0.9);

        rational_ev * (1.0 - alpha) + audacity_ev * alpha
    }
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::match_sim::test_fixtures::create_test_team_with_mental;

    fn create_test_engine_with_attrs(
        home_flair: u8,
        home_aggression: u8,
        home_decisions: u8,
    ) -> MatchEngine {
        let home =
            create_test_team_with_mental("Home", home_flair, home_aggression, home_decisions);
        let away = create_test_team_with_mental("Away", 50, 50, 50);

        let plan = super::super::MatchPlan {
            home_team: home,
            away_team: away,
            seed: 12345,
            user_player: None,
            home_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            away_match_modifiers: crate::engine::TeamMatchModifiers::default(),
            home_instructions: None,
            away_instructions: None,
            home_player_instructions: None,
            away_player_instructions: None,
            home_ai_difficulty: None,
            away_ai_difficulty: None,
        };

        let mut engine = MatchEngine::new(plan).expect("match engine init");
        engine.initialize_player_positions();
        engine
    }

    #[test]
    fn test_audacity_context_calculation() {
        // High flair (90), high aggression (70), low decisions (55)
        let mut engine = create_test_engine_with_attrs(90, 70, 55);
        engine.result.score_home = 0;
        engine.result.score_away = 1; // 지는 중
        engine.minute = 82;

        let ctx = engine.get_audacity_context(0); // Home player

        // flair = ~90/100 = 0.9 (may be modified slightly by trait system)
        assert!(ctx.flair > 0.8, "flair should be high: {}", ctx.flair);

        // audacity = 0.7 * 0.7 + 0.3 * (1 - 0.55) = 0.49 + 0.135 = 0.625
        assert!(ctx.audacity > 0.5, "audacity: {}", ctx.audacity);

        // desperation: losing (1.0) * 0.7 + late_game (0.6) * 0.3 = 0.88
        assert!(ctx.desperation > 0.7, "desperation: {}", ctx.desperation);
    }

    #[test]
    fn test_glory_bonus_activates() {
        let mut engine = create_test_engine_with_attrs(90, 70, 55);
        engine.result.score_home = 0;
        engine.result.score_away = 1;
        engine.minute = 82;

        let ctx = engine.get_audacity_context(0);

        // Low prob, high reward
        let boosted = engine.apply_audacity_boost(
            -0.28, // rational_ev (negative)
            &ctx, 0.05, // base_prob (low)
            1.0,  // base_reward (high - goal)
            0.35, // base_risk
        );

        // Should be significantly higher than rational
        assert!(boosted > 0.0, "Audacity should boost negative EV: {}", boosted);
    }

    #[test]
    fn test_rational_player_less_audacity() {
        // Rational player: low flair (25), low aggression (30), high decisions (90)
        let mut engine = create_test_engine_with_attrs(25, 30, 90);
        engine.result.score_home = 0;
        engine.result.score_away = 1;
        engine.minute = 82;

        let ctx = engine.get_audacity_context(0);

        let boosted = engine.apply_audacity_boost(-0.28, &ctx, 0.05, 1.0, 0.35);

        // Rational player: boost should be smaller (likely still negative)
        assert!(boosted < 0.2, "Rational player boost should be smaller: {}", boosted);
    }

    #[test]
    fn test_winning_less_desperation() {
        let mut engine = create_test_engine_with_attrs(90, 70, 55);
        engine.result.score_home = 2;
        engine.result.score_away = 0; // 이기는 중
        engine.minute = 20; // 전반

        let ctx = engine.get_audacity_context(0);

        // desperation should be low when winning early
        assert!(ctx.desperation < 0.2, "Winning early game desperation: {}", ctx.desperation);
    }

    #[test]
    fn test_high_prob_no_glory_bonus() {
        let mut engine = create_test_engine_with_attrs(90, 70, 55);
        engine.result.score_home = 0;
        engine.result.score_away = 1;
        engine.minute = 82;

        let ctx = engine.get_audacity_context(0);

        // High prob - no glory bonus should trigger
        let boosted = engine.apply_audacity_boost(
            0.3, // rational_ev (positive)
            &ctx, 0.8,  // base_prob (high - easy chance)
            1.0,  // base_reward (goal)
            0.35, // base_risk
        );

        // High prob shot: no big boost, should stay close to rational
        // Because is_low_prob = false, no glory_bonus
        assert!(boosted > 0.2 && boosted < 0.8, "High prob shot EV: {}", boosted);
    }

    #[test]
    fn test_away_team_context() {
        let mut engine = create_test_engine_with_attrs(50, 50, 50);
        engine.result.score_home = 2;
        engine.result.score_away = 0; // Home winning, Away losing
        engine.minute = 85;

        // Away player (idx 11+)
        let ctx = engine.get_audacity_context(11);

        // Away team is losing, should have high desperation
        assert!(ctx.desperation > 0.7, "Away losing desperation: {}", ctx.desperation);
    }
}
