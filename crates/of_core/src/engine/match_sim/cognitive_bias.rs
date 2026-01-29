//! Cognitive Bias System (P16 Subjective Utility v2.1)
//!
//! Player "bias glasses" used ONLY in Gate B (decision scoring).
//! Keep execution (Gate C) clean: ActionModel handles skill/pressure there.
//!
//! ## 핵심 원칙
//! - **Gate B 전용**: Bias는 의사결정 단계에서만 적용
//! - **Gate C 미적용**: 실행은 ActionModel이 담당 (Bias 없음)
//! - **데이터 기반**: if문 없이 능력치가 행동을 결정
//!
//! ## CognitiveBias 파라미터 (6개)
//! - `confidence_factor`: 확률 과대평가 (composure + flair)
//! - `bravery_factor`: 실패 비용 과소평가 (bravery + aggression)
//! - `greed_factor`: 개인 보상 과대평가 (flair - decisions)
//! - `team_cost_sensitivity`: 팀 비용 민감도 (teamwork + concentration)
//! - `decision_noise`: 판단력 노이즈 (1 - decisions)
//! - `tunnel_vision`: 터널 시야 (buff driven)

// rand::Rng is used in utility.rs, not here

// ============================================================================
// CognitiveBias 구조체
// ============================================================================

/// 선수의 인지 편향 (Gate B 전용)
#[derive(Debug, Clone, Copy)]
pub struct CognitiveBias {
    /// 성공 확률 과대평가 (Logit shift)
    /// composure + flair 기반, 0.5 ~ 1.5
    pub confidence_factor: f32,

    /// 실패 비용 과소평가 (분모)
    /// bravery + aggression 기반, 0.5 ~ 2.0
    pub bravery_factor: f32,

    /// 골/개인 보상 과대평가
    /// flair - decisions 기반, 0.0 ~ 0.4
    pub greed_factor: f32,

    /// 팀 비용 민감도
    /// teamwork + concentration 기반, 0.5 ~ 1.5
    pub team_cost_sensitivity: f32,

    /// 판단력 노이즈 (Decision 낮을수록 큼)
    /// 0.0 ~ 0.3
    pub decision_noise: f32,

    /// 터널 시야 (패스 보상 감소, 개인기 보상 증가)
    /// dribble streak, 연속 제치기 시 증가
    /// 0.0 ~ 1.0
    pub tunnel_vision: f32,
}

impl Default for CognitiveBias {
    fn default() -> Self {
        Self {
            confidence_factor: 1.0,
            bravery_factor: 1.0,
            greed_factor: 0.15,
            team_cost_sensitivity: 1.0,
            decision_noise: 0.15,
            tunnel_vision: 0.0,
        }
    }
}

// ============================================================================
// BiasSnapshot (Replay/Debug용)
// ============================================================================

/// Replay/Debug용 경량 스냅샷
#[derive(Debug, Clone, Copy, Default)]
pub struct BiasSnapshot {
    pub confidence_factor: f32,
    pub bravery_factor: f32,
    pub greed_factor: f32,
    pub team_cost_sensitivity: f32,
    pub decision_noise: f32,
    pub tunnel_vision: f32,
}

impl From<&CognitiveBias> for BiasSnapshot {
    fn from(bias: &CognitiveBias) -> Self {
        Self {
            confidence_factor: bias.confidence_factor,
            bravery_factor: bias.bravery_factor,
            greed_factor: bias.greed_factor,
            team_cost_sensitivity: bias.team_cost_sensitivity,
            decision_noise: bias.decision_noise,
            tunnel_vision: bias.tunnel_vision,
        }
    }
}

// ============================================================================
// MentalBuff (Gate B에서만 적용)
// ============================================================================

/// 멘탈 버프 (if 없이 수치만 조정)
#[derive(Debug, Clone, Copy)]
pub enum MentalBuff {
    /// On Fire: 연속 좋은 플레이 후
    OnFire { confidence_add: f32, greed_add: f32 },
    /// Panicked: 큰 실수 후
    Panicked { confidence_sub: f32, bravery_div: f32 },
    /// Tunnel Vision: 직접 추가
    TunnelVision { tunnel_add: f32 },
    /// Dribble Streak: 연속 제치기
    DribbleStreak { streak_count: u8 },
    /// Hero Mode: 히어로 상황 (winning goal 찬스 등)
    HeroMode { confidence_add: f32, bravery_mul: f32 },
}

// ============================================================================
// CognitiveBias 구현
// ============================================================================

impl CognitiveBias {
    /// 상세 능력치에서 Bias 생성 (Full Implementation)
    ///
    /// # Arguments
    /// * `composure` - 침착성 (0-100)
    /// * `flair` - 기교 (0-100)
    /// * `bravery` - 용기 (0-100)
    /// * `aggression` - 적극성 (0-100)
    /// * `decisions` - 판단력 (0-100)
    /// * `teamwork` - 팀워크 (0-100)
    /// * `concentration` - 집중력 (0-100)
    pub fn from_attributes(
        composure: f32,
        flair: f32,
        bravery: f32,
        aggression: f32,
        decisions: f32,
        teamwork: f32,
        concentration: f32,
    ) -> Self {
        // 0~1로 정규화
        let norm = |attr: f32| (attr / 100.0).clamp(0.0, 1.0);

        let c = norm(composure);
        let f = norm(flair);
        let b = norm(bravery);
        let a = norm(aggression);
        let d = norm(decisions);
        let tw = norm(teamwork);
        let con = norm(concentration);

        Self {
            // 자신감: composure + flair (높을수록 과대평가)
            confidence_factor: 0.5 + c * 0.5 + f * 0.5,

            // 용기: bravery + aggression (높을수록 리스크 과소평가)
            bravery_factor: 0.5 + b * 0.75 + a * 0.75,

            // 탐욕: flair - decisions (높을수록 개인 플레이 선호)
            greed_factor: (f - d * 0.5 + 0.2).clamp(0.0, 0.4),

            // 팀 비용: teamwork + concentration
            team_cost_sensitivity: 0.5 + tw * 0.5 + con * 0.5,

            // 노이즈: decision 낮을수록 큼
            decision_noise: (1.0 - d) * 0.3,

            // 터널 시야: 초기값 0 (버프로 증가)
            tunnel_vision: 0.0,
        }
    }

    /// 종합 능력치만으로 Bias 생성 (Bootstrap/Fallback)
    /// 상세 능력치 데이터 없을 때 사용
    pub fn from_overall(overall: u8) -> Self {
        // overall: 0~100 스케일
        let norm = (overall as f32 / 100.0).clamp(0.0, 1.0);

        Self {
            // 높은 overall = 높은 자신감
            confidence_factor: 0.7 + norm * 0.6, // 0.7 ~ 1.3

            // 중간값 기준
            bravery_factor: 1.0 + (norm - 0.5) * 0.5, // 0.75 ~ 1.25

            // 낮은 overall = 높은 탐욕 (보상 추구)
            greed_factor: (0.3 - norm * 0.2).clamp(0.0, 0.4),

            // 높은 overall = 높은 팀 의식
            team_cost_sensitivity: 0.7 + norm * 0.6,

            // 높은 overall = 낮은 노이즈
            decision_noise: (1.0 - norm) * 0.25,

            tunnel_vision: 0.0,
        }
    }

    /// 스냅샷 생성
    #[inline]
    pub fn snapshot(&self) -> BiasSnapshot {
        BiasSnapshot::from(self)
    }

    /// Gate B에서만: 버프 스택 적용
    pub fn apply_buffs(&mut self, buffs: &[MentalBuff]) {
        for buff in buffs {
            match buff {
                MentalBuff::OnFire { confidence_add, greed_add } => {
                    self.confidence_factor += confidence_add;
                    self.greed_factor += greed_add;
                }
                MentalBuff::Panicked { confidence_sub, bravery_div } => {
                    self.confidence_factor -= confidence_sub;
                    self.bravery_factor /= bravery_div;
                }
                MentalBuff::TunnelVision { tunnel_add } => {
                    self.tunnel_vision += tunnel_add;
                }
                MentalBuff::DribbleStreak { streak_count } => {
                    // 연속 제치기 시 터널 시야 증가 (0.15 per beat)
                    self.tunnel_vision += (*streak_count as f32) * 0.15;
                    self.confidence_factor += (*streak_count as f32) * 0.1;
                }
                MentalBuff::HeroMode { confidence_add, bravery_mul } => {
                    self.confidence_factor += confidence_add;
                    self.bravery_factor *= bravery_mul;
                }
            }
        }

        // Clamp to valid ranges
        self.confidence_factor = self.confidence_factor.clamp(0.3, 2.0);
        self.bravery_factor = self.bravery_factor.clamp(0.3, 3.0);
        self.greed_factor = self.greed_factor.clamp(0.0, 0.6);
        self.team_cost_sensitivity = self.team_cost_sensitivity.clamp(0.3, 2.0);
        self.decision_noise = self.decision_noise.clamp(0.0, 0.5);
        self.tunnel_vision = self.tunnel_vision.clamp(0.0, 1.0);
    }

    // ========== Multiplier 메서드 ==========

    /// 성공 보상 증폭 계수
    /// greed가 높을수록 개인 플레이 보상 증가
    #[inline]
    pub fn reward_multiplier(&self, is_selfish_action: bool) -> f32 {
        if is_selfish_action {
            1.0 + self.greed_factor + self.tunnel_vision * 0.3
        } else {
            1.0 - self.greed_factor * 0.5 - self.tunnel_vision * 0.5
        }
    }

    /// 실패 비용 감쇠 계수
    /// bravery가 높을수록 비용 인식 감소
    #[inline]
    pub fn fail_cost_multiplier(&self) -> f32 {
        (1.0 / self.bravery_factor).clamp(0.3, 2.0)
    }

    /// 터널 시야 패스 페널티
    /// tunnel_vision 높을수록 패스 보상 감소
    #[inline]
    pub fn pass_penalty(&self) -> f32 {
        1.0 - self.tunnel_vision * 0.5
    }

    /// 압박 상황에서의 패스 페널티 (v2.1)
    #[inline]
    pub fn pass_penalty_with_pressure(&self, pressure: f32) -> f32 {
        let p = pressure.clamp(0.0, 1.0);
        (1.0 - self.tunnel_vision * 0.7 * p).clamp(0.25, 1.0)
    }
}

// ============================================================================
// AudacityContext 호환 레이어
// ============================================================================

/// 기존 AudacityContext와의 호환을 위한 컨버터
/// P10-13에서 P16으로 마이그레이션 시 사용
pub struct AudacityCompatLayer;

impl AudacityCompatLayer {
    /// AudacityContext 스타일 값에서 CognitiveBias 생성
    pub fn from_audacity_context(
        flair: f32,       // 0~1
        audacity: f32,    // 0~1 (aggression + (1-decisions))
        desperation: f32, // 0~1
    ) -> CognitiveBias {
        CognitiveBias {
            confidence_factor: 0.8 + flair * 0.4 + desperation * 0.2,
            bravery_factor: 0.8 + audacity * 0.7,
            greed_factor: (flair * 0.3 + audacity * 0.1).clamp(0.0, 0.4),
            team_cost_sensitivity: 1.0 - audacity * 0.3,
            decision_noise: audacity * 0.25,
            tunnel_vision: desperation * 0.3,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_overall() {
        // 높은 overall
        let high = CognitiveBias::from_overall(85);
        assert!(high.confidence_factor > 1.0);
        assert!(high.decision_noise < 0.1);

        // 낮은 overall
        let low = CognitiveBias::from_overall(40);
        assert!(low.confidence_factor < 1.0);
        assert!(low.decision_noise > 0.1);
    }

    #[test]
    fn test_from_attributes() {
        // 천재형 (높은 flair, 낮은 decisions)
        let genius = CognitiveBias::from_attributes(
            70.0, // composure
            90.0, // flair (높음)
            60.0, // bravery
            50.0, // aggression
            45.0, // decisions (낮음)
            50.0, // teamwork
            50.0, // concentration
        );
        assert!(genius.greed_factor > 0.25, "Genius should be greedy");
        assert!(genius.confidence_factor > 1.0, "Genius should be confident");

        // 책임감 있는 선수 (높은 teamwork, decisions, 낮은 flair)
        let responsible = CognitiveBias::from_attributes(
            70.0, // composure
            30.0, // flair (낮음 - 책임감 있는 선수)
            50.0, // bravery
            40.0, // aggression
            85.0, // decisions (높음)
            90.0, // teamwork (높음)
            80.0, // concentration
        );
        assert!(responsible.team_cost_sensitivity > 1.2);
        // greed = (0.3 - 0.85*0.5 + 0.2) = 0.075
        assert!(responsible.greed_factor < 0.15, "greed={}", responsible.greed_factor);
        assert!(responsible.decision_noise < 0.1);
    }

    #[test]
    fn test_apply_buffs() {
        let mut bias = CognitiveBias::default();
        let initial_conf = bias.confidence_factor;

        bias.apply_buffs(&[MentalBuff::DribbleStreak { streak_count: 2 }]);

        assert!(bias.tunnel_vision > 0.0, "Tunnel vision should increase");
        assert!(bias.confidence_factor > initial_conf, "Confidence should increase");
    }

    #[test]
    fn test_reward_multiplier() {
        let mut bias = CognitiveBias::default();
        bias.greed_factor = 0.3;
        bias.tunnel_vision = 0.5;

        let selfish = bias.reward_multiplier(true);
        let team = bias.reward_multiplier(false);

        assert!(selfish > team, "Selfish action should have higher reward");
    }

    #[test]
    fn test_pass_penalty() {
        let mut bias = CognitiveBias::default();
        bias.tunnel_vision = 0.0;
        assert_eq!(bias.pass_penalty(), 1.0);

        bias.tunnel_vision = 1.0;
        assert!(bias.pass_penalty() < 1.0, "High tunnel vision should penalize pass");
    }

    #[test]
    fn test_clamp_after_buffs() {
        let mut bias = CognitiveBias::default();

        // 극단적인 버프 적용
        bias.apply_buffs(&[MentalBuff::OnFire { confidence_add: 5.0, greed_add: 1.0 }]);

        // Clamp 확인
        assert!(bias.confidence_factor <= 2.0);
        assert!(bias.greed_factor <= 0.6);
    }
}
