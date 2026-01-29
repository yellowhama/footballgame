//! Subjective Utility System (P16 v2.1)
//!
//! Master Formula: U = P_hat * V_win_hat + (1 - P_hat) * (-V_lose_hat)
//!
//! ## 핵심 원칙
//! - **Logit Space 왜곡**: 확률이 항상 0~1 유지
//! - **Pressure-Weighted Noise**: 평상시 30%, 압박 시 100%
//! - **Gate B 전용**: 이 모듈은 의사결정 단계에서만 사용
//! - **Softmax Selection**: Temperature 기반 확률적 선택

use super::cognitive_bias::CognitiveBias;
use rand::Rng;

// ============================================================================
// Math Helpers
// ============================================================================

/// Logit 변환: 확률 → 로그오즈
#[inline]
pub fn logit(p: f32) -> f32 {
    let p_clamped = p.clamp(0.001, 0.999);
    (p_clamped / (1.0 - p_clamped)).ln()
}

/// Sigmoid 역변환: 로그오즈 → 확률
#[inline]
pub fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Box-Muller 정규분포 (mean=0, std=1) [v2.1]
pub fn normal01(rng: &mut impl Rng) -> f32 {
    let u1 = rng.gen::<f32>().max(1e-6);
    let u2 = rng.gen::<f32>();
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * std::f32::consts::PI * u2;
    r * theta.cos()
}

// ============================================================================
// CandidateFacts
// ============================================================================

/// 후보 액션에 대한 객관적 사실 (물리엔진/ActionModel 산출)
#[derive(Debug, Clone, Copy)]
pub struct CandidateFacts {
    /// 객관적 성공 확률
    pub p_true: f32,
    /// 성공 시 보상 (xG, 전진 거리 등)
    pub v_win_true: f32,
    /// 실패 시 비용 (역습 위험 등)
    pub v_lose_true: f32,
    /// 압박 상황 (0~1) - 노이즈 스케일링용 [v2.1]
    pub pressure: f32,
    /// 이기적 액션 여부 (Shot, TakeOn 등)
    pub is_selfish: bool,
    /// 패스 계열 액션 여부
    pub is_pass_like: bool,
}

impl Default for CandidateFacts {
    fn default() -> Self {
        Self {
            p_true: 0.5,
            v_win_true: 0.3,
            v_lose_true: 0.3,
            pressure: 0.0,
            is_selfish: false,
            is_pass_like: false,
        }
    }
}

impl CandidateFacts {
    /// 새 CandidateFacts 생성
    pub fn new(
        p_true: f32,
        v_win_true: f32,
        v_lose_true: f32,
        pressure: f32,
        is_selfish: bool,
        is_pass_like: bool,
    ) -> Self {
        Self {
            p_true: p_true.clamp(0.0, 1.0),
            v_win_true: v_win_true.clamp(0.0, 1.0),
            v_lose_true: v_lose_true.clamp(0.0, 1.0),
            pressure: pressure.clamp(0.0, 1.0),
            is_selfish,
            is_pass_like,
        }
    }

    /// 객관적 EU 계산 (bias 없음, back-compat용)
    #[inline]
    pub fn objective_utility(&self) -> f32 {
        let p = self.p_true.clamp(0.0, 1.0);
        p * self.v_win_true + (1.0 - p) * (-self.v_lose_true)
    }
}

// ============================================================================
// UtilityResult
// ============================================================================

/// Utility 계산 결과 (Replay/Debug용)
#[derive(Debug, Clone)]
pub struct UtilityResult {
    /// 최종 Utility 값
    pub utility: f32,
    /// 왜곡된 성공 확률
    pub p_hat: f32,
    /// 왜곡된 성공 보상
    pub v_win_hat: f32,
    /// 왜곡된 실패 비용
    pub v_lose_hat: f32,
    /// 원본 Facts
    pub facts: CandidateFacts,
}

impl UtilityResult {
    /// Replay 설명 문자열 생성
    pub fn explain(&self) -> String {
        format!(
            "P: {:.0}%→{:.0}% | V+: {:.2}→{:.2} | V-: {:.2}→{:.2} | U={:.3}",
            self.facts.p_true * 100.0,
            self.p_hat * 100.0,
            self.facts.v_win_true,
            self.v_win_hat,
            self.facts.v_lose_true,
            self.v_lose_hat,
            self.utility
        )
    }
}

// ============================================================================
// Core Functions
// ============================================================================

/// 안정적인 확률 왜곡 [v2.1: pressure-weighted noise]
///
/// Logit 공간에서 왜곡하여 확률이 항상 0~1을 유지
pub fn distort_probability(
    p_true: f32,
    bias: &CognitiveBias,
    pressure: f32,
    rng: &mut impl Rng,
) -> f32 {
    // 1. Logit 공간으로 이동
    let logit_p = logit(p_true);

    // 2. 자신감으로 shift (1.0 기준, >1 = 과대평가)
    let k_conf = ((bias.confidence_factor - 1.0) * 1.25).clamp(-1.5, 1.5);

    // 3. Pressure-weighted noise [v2.1]
    // 평상시엔 노이즈 30%만, 압박 상황에서 100%
    // → 평소엔 안정적, 위기 상황에서만 판단력 흔들림
    let noise_scale = bias.decision_noise * (0.3 + 0.7 * pressure.clamp(0.0, 1.0));
    let noise = normal01(rng) * noise_scale;

    // 4. 왜곡된 로그오즈
    let logit_hat = logit_p + k_conf + noise;

    // 5. 다시 확률로 변환 (항상 0~1 유지!)
    sigmoid(logit_hat)
}

/// 결정론적 확률 왜곡 (테스트/디버그용, 노이즈 없음)
pub fn distort_probability_deterministic(p_true: f32, bias: &CognitiveBias) -> f32 {
    let logit_p = logit(p_true);
    let k_conf = ((bias.confidence_factor - 1.0) * 1.25).clamp(-1.5, 1.5);
    sigmoid(logit_p + k_conf)
}

/// 단 하나의 마스터 함수 (Gate B 전용) [v2.1]
///
/// Facts + Bias → UtilityResult
pub fn calculate_utility_result(
    facts: CandidateFacts,
    bias: &CognitiveBias,
    rng: &mut impl Rng,
) -> UtilityResult {
    // 1. 확률 왜곡 (pressure-weighted noise 적용)
    let p_hat = distort_probability(facts.p_true, bias, facts.pressure, rng);

    // 2. 성공 보상 왜곡
    let mut v_win_hat = facts.v_win_true * bias.reward_multiplier(facts.is_selfish);

    // 3. 터널 시야: 패스 보상 감소
    if facts.is_pass_like {
        v_win_hat *= bias.pass_penalty_with_pressure(facts.pressure);
    }

    // 4. 실패 비용 왜곡 (bravery로 감소, team_cost로 증가)
    let v_lose_hat = facts.v_lose_true
        * bias.fail_cost_multiplier()
        * (1.0 + 0.3 * (bias.team_cost_sensitivity - 1.0));

    // 5. Expected Utility 공식
    let utility = p_hat * v_win_hat + (1.0 - p_hat) * (-v_lose_hat);

    UtilityResult { utility, p_hat, v_win_hat, v_lose_hat, facts }
}

/// biased_utility: 더 간결한 버전 [v2.0]
///
/// Facts와 Bias만 받아서 Utility 값만 반환
pub fn biased_utility(facts: &CandidateFacts, bias: &CognitiveBias, rng: &mut impl Rng) -> f32 {
    let p_hat = distort_probability(facts.p_true, bias, facts.pressure, rng);

    let mut v_win_hat = facts.v_win_true * bias.reward_multiplier(facts.is_selfish);

    if facts.is_pass_like {
        v_win_hat *= bias.pass_penalty_with_pressure(facts.pressure);
    }

    let v_lose_hat = facts.v_lose_true * bias.fail_cost_multiplier();

    p_hat * v_win_hat + (1.0 - p_hat) * (-v_lose_hat)
}

// ============================================================================
// Temperature & Softmax Selection
// ============================================================================

/// Temperature 계산 [v2.1]
///
/// "천재 = 변덕쟁이" ❌ → "재능 + 판단 = 다양성" ✅
///
/// # Arguments
/// * `flair` - 창의성/기교 (0~1)
/// * `decisions` - 판단력 (0~1)
/// * `concentration` - 집중력 (0~1)
pub fn calculate_temperature(flair: f32, decisions: f32, concentration: f32) -> f32 {
    let creativity = flair.clamp(0.0, 1.0);
    let consistency = decisions.clamp(0.0, 1.0);
    let focus = concentration.clamp(0.0, 1.0);

    // 창의성 높고 일관성 높으면 → "다양하지만 좋은" 선택
    // 창의성 높고 일관성 낮으면 → "온갖 시도" (진짜 변덕)
    // 창의성 낮고 일관성 높으면 → "안전한 선택만"
    let base_temp = 0.25 + 0.35 * creativity;
    let stability = 1.0 - consistency * 0.4 - focus * 0.2;

    (base_temp * stability).clamp(0.15, 0.70)
}

/// 종합 능력치만으로 Temperature 계산 (Fallback)
pub fn calculate_temperature_from_overall(overall: u8) -> f32 {
    let norm = (overall as f32 / 100.0).clamp(0.0, 1.0);
    // 높은 overall = 낮은 온도 (더 결정론적)
    (0.50 - norm * 0.25).clamp(0.15, 0.70)
}

/// Softmax 확률적 선택 [v2.1]
///
/// Temperature가 낮을수록 argmax에 가까워짐
///
/// # Returns
/// 선택된 인덱스
pub fn softmax_select(utilities: &[f32], temperature: f32, rng: &mut impl Rng) -> usize {
    if utilities.is_empty() {
        return 0;
    }

    let t = temperature.max(1e-3);

    // Utility를 지수로 변환 (overflow 방지를 위해 max 빼기)
    let max_u = utilities.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let weights: Vec<f32> = utilities.iter().map(|u| ((u - max_u) / t).exp()).collect();

    let total: f32 = weights.iter().sum();
    if total <= 0.0 || !total.is_finite() {
        return 0;
    }

    // 룰렛 휠 선택
    let mut pick = rng.gen::<f32>() * total;
    for (i, w) in weights.iter().enumerate() {
        pick -= w;
        if pick <= 0.0 {
            return i;
        }
    }

    utilities.len() - 1
}

/// Softmax 확률 분포 반환 (디버그/Replay용)
pub fn softmax_probabilities(utilities: &[f32], temperature: f32) -> Vec<f32> {
    if utilities.is_empty() {
        return vec![];
    }

    let t = temperature.max(1e-3);
    let max_u = utilities.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let weights: Vec<f32> = utilities.iter().map(|u| ((u - max_u) / t).exp()).collect();

    let total: f32 = weights.iter().sum();
    if total <= 0.0 || !total.is_finite() {
        let n = utilities.len();
        return vec![1.0 / n as f32; n];
    }

    weights.iter().map(|w| w / total).collect()
}

// ============================================================================
// UtilityBreakdown (DecisionLog용)
// ============================================================================

/// 각 후보의 Utility 분해 [v2.0]
#[derive(Debug, Clone)]
pub struct UtilityBreakdown {
    pub candidate_idx: usize,
    pub facts: CandidateFacts,
    pub p_hat: f32,
    pub v_win_hat: f32,
    pub v_lose_hat: f32,
    pub utility: f32,
    pub selection_prob: f32, // softmax에서의 선택 확률
}

impl UtilityBreakdown {
    /// UtilityResult에서 UtilityBreakdown 생성
    pub fn from_result(candidate_idx: usize, result: &UtilityResult, selection_prob: f32) -> Self {
        Self {
            candidate_idx,
            facts: result.facts,
            p_hat: result.p_hat,
            v_win_hat: result.v_win_hat,
            v_lose_hat: result.v_lose_hat,
            utility: result.utility,
            selection_prob,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn test_logit_sigmoid_inverse() {
        for p in [0.1, 0.25, 0.5, 0.75, 0.9] {
            let result = sigmoid(logit(p));
            assert!((result - p).abs() < 0.001, "logit/sigmoid should be inverse");
        }
    }

    #[test]
    fn test_distort_probability_stays_in_range() {
        let mut rng = test_rng();
        let bias = CognitiveBias::default();

        for _ in 0..100 {
            let p_true = rng.gen::<f32>();
            let pressure = rng.gen::<f32>();
            let p_hat = distort_probability(p_true, &bias, pressure, &mut rng);

            assert!((0.0..=1.0).contains(&p_hat), "p_hat should stay in [0, 1], got {}", p_hat);
        }
    }

    #[test]
    fn test_confidence_increases_perceived_probability() {
        let low_conf = CognitiveBias { confidence_factor: 0.5, ..Default::default() };
        let high_conf = CognitiveBias { confidence_factor: 1.5, ..Default::default() };

        let p_true = 0.3;
        let p_low = distort_probability_deterministic(p_true, &low_conf);
        let p_high = distort_probability_deterministic(p_true, &high_conf);

        assert!(p_high > p_low, "High confidence should increase perceived probability");
    }

    #[test]
    fn test_utility_calculation() {
        let mut rng = test_rng();
        let bias = CognitiveBias::default();

        let facts = CandidateFacts {
            p_true: 0.5,
            v_win_true: 0.5,
            v_lose_true: 0.3,
            pressure: 0.0,
            is_selfish: false,
            is_pass_like: false,
        };

        let result = calculate_utility_result(facts, &bias, &mut rng);

        // 기본 bias로는 utility가 양수여야 함 (50% 확률, win > lose)
        assert!(
            result.utility > -0.5 && result.utility < 0.5,
            "Utility should be reasonable, got {}",
            result.utility
        );
    }

    #[test]
    fn test_selfish_action_reward_boost() {
        let mut rng = test_rng();
        let bias = CognitiveBias { greed_factor: 0.3, ..Default::default() };

        let selfish_facts = CandidateFacts {
            p_true: 0.5,
            v_win_true: 0.5,
            v_lose_true: 0.3,
            pressure: 0.0,
            is_selfish: true,
            is_pass_like: false,
        };

        let team_facts = CandidateFacts {
            p_true: 0.5,
            v_win_true: 0.5,
            v_lose_true: 0.3,
            pressure: 0.0,
            is_selfish: false,
            is_pass_like: false,
        };

        let selfish_result = calculate_utility_result(selfish_facts, &bias, &mut rng);
        let mut rng2 = test_rng();
        let team_result = calculate_utility_result(team_facts, &bias, &mut rng2);

        assert!(
            selfish_result.v_win_hat > team_result.v_win_hat,
            "Selfish action should have higher reward with greed"
        );
    }

    #[test]
    fn test_pass_penalty_with_tunnel_vision() {
        let mut rng = test_rng();
        let bias = CognitiveBias { tunnel_vision: 0.8, ..Default::default() };

        let pass_facts = CandidateFacts {
            p_true: 0.8,
            v_win_true: 0.5,
            v_lose_true: 0.2,
            pressure: 0.5,
            is_selfish: false,
            is_pass_like: true,
        };

        let result = calculate_utility_result(pass_facts, &bias, &mut rng);

        assert!(
            result.v_win_hat < pass_facts.v_win_true,
            "Pass reward should be reduced with tunnel vision"
        );
    }

    #[test]
    fn test_temperature_calculation() {
        // 높은 창의성, 낮은 일관성 = 높은 온도
        let high_temp = calculate_temperature(0.9, 0.2, 0.3);

        // 낮은 창의성, 높은 일관성 = 낮은 온도
        let low_temp = calculate_temperature(0.3, 0.9, 0.8);

        assert!(
            high_temp > low_temp,
            "Creative but inconsistent player should have higher temperature"
        );

        // 범위 확인
        assert!((0.15..=0.70).contains(&high_temp));
        assert!((0.15..=0.70).contains(&low_temp));
    }

    #[test]
    fn test_softmax_select_prefers_higher_utility() {
        let mut rng = test_rng();
        let utilities = vec![-0.5, 0.3, 0.1, -0.2];
        let temperature = 0.3;

        let mut counts = [0usize; 4];
        for _ in 0..1000 {
            let idx = softmax_select(&utilities, temperature, &mut rng);
            counts[idx] += 1;
        }

        // 가장 높은 utility(0.3, idx=1)가 가장 많이 선택되어야 함
        assert!(counts[1] > counts[0], "Higher utility should be selected more often");
        assert!(counts[1] > counts[2], "Higher utility should be selected more often");
        assert!(counts[1] > counts[3], "Higher utility should be selected more often");
    }

    #[test]
    fn test_softmax_probabilities_sum_to_one() {
        let utilities = vec![0.1, 0.3, -0.2, 0.0];
        let temperature = 0.4;

        let probs = softmax_probabilities(&utilities, temperature);
        let sum: f32 = probs.iter().sum();

        assert!((sum - 1.0).abs() < 0.001, "Probabilities should sum to 1.0");
    }

    #[test]
    fn test_low_temperature_approaches_argmax() {
        let mut rng = test_rng();
        let utilities = vec![-0.5, 0.5, 0.1]; // idx 1 is clearly best
        let temperature = 0.15; // very low

        let mut counts = [0usize; 3];
        for _ in 0..100 {
            let idx = softmax_select(&utilities, temperature, &mut rng);
            counts[idx] += 1;
        }

        // 낮은 온도에서는 최고 utility가 거의 항상 선택됨
        assert!(counts[1] > 90, "Low temperature should approach argmax");
    }

    #[test]
    fn test_utility_result_explain() {
        let result = UtilityResult {
            utility: 0.15,
            p_hat: 0.24,
            v_win_hat: 0.45,
            v_lose_hat: 0.45,
            facts: CandidateFacts {
                p_true: 0.10,
                v_win_true: 0.30,
                v_lose_true: 0.80,
                pressure: 0.5,
                is_selfish: true,
                is_pass_like: false,
            },
        };

        let explanation = result.explain();
        assert!(explanation.contains("P:"), "Should contain probability info");
        assert!(explanation.contains("U="), "Should contain utility value");
    }

    #[test]
    fn test_objective_utility() {
        let facts = CandidateFacts {
            p_true: 0.6,
            v_win_true: 0.4,
            v_lose_true: 0.2,
            pressure: 0.0,
            is_selfish: false,
            is_pass_like: false,
        };

        // EU = 0.6 * 0.4 + 0.4 * (-0.2) = 0.24 - 0.08 = 0.16
        let eu = facts.objective_utility();
        assert!((eu - 0.16).abs() < 0.001);
    }

    #[test]
    fn test_pressure_increases_noise_effect() {
        let mut rng1 = StdRng::seed_from_u64(123);
        let mut rng2 = StdRng::seed_from_u64(123);

        let bias =
            CognitiveBias { decision_noise: 0.3, confidence_factor: 1.0, ..Default::default() };

        // 여러번 샘플링해서 분산 비교
        let mut low_pressure_samples = Vec::new();
        let mut high_pressure_samples = Vec::new();

        for _ in 0..100 {
            low_pressure_samples.push(distort_probability(0.5, &bias, 0.0, &mut rng1));
            high_pressure_samples.push(distort_probability(0.5, &bias, 1.0, &mut rng2));
        }

        fn variance(samples: &[f32]) -> f32 {
            let mean = samples.iter().sum::<f32>() / samples.len() as f32;
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / samples.len() as f32
        }

        let low_var = variance(&low_pressure_samples);
        let high_var = variance(&high_pressure_samples);

        // 압박 상황에서 분산이 더 커야 함 (노이즈 영향이 더 큼)
        assert!(
            high_var > low_var * 1.5,
            "High pressure should have higher variance. low={}, high={}",
            low_var,
            high_var
        );
    }
}
