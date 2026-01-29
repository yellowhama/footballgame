/// action_scoring.rs
/// ACTION_SCORING_SSOT 공용 함수 (compute_score, apply_situational, map_peak, etc)
use super::action_scoring_types::*;
use crate::models::player::PlayerAttributes;
use std::collections::HashMap;

const EPS: f32 = 1e-6;

/// ============================================================================
/// 1. normalize(v) - 능력치(1..100) → 0..1
/// ============================================================================

pub fn normalize(value: u8, min_stat: i32, max_stat: i32, clamp: bool) -> f32 {
    let v = value as f32;
    let s = (v - min_stat as f32) / ((max_stat - min_stat) as f32 + EPS);

    if clamp {
        s.clamp(0.0, 1.0)
    } else {
        s
    }
}

/// ============================================================================
/// 2. compute_score(score_spec, player_stats) → q(0..1)
/// ============================================================================

/// 가중합 + 감마 커브 적용
pub fn compute_score(
    score_spec: &ScoreSpec,
    player_attrs: &PlayerAttributes,
    stat_scale: &StatScale,
    default_gamma: f32,
) -> f32 {
    // 1) 가중합
    let mut sum_w = 0.0;
    let mut acc = 0.0;

    // Determinism: HashMap iteration order is not stable. Sort keys for
    // deterministic accumulation (avoids drift in float rounding → decision changes).
    let mut stats: Vec<_> = score_spec.stats.iter().collect();
    stats.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (stat_name, &weight) in stats {
        // YAML 키로 능력치 조회
        let value = player_attrs.get_by_key(stat_name).unwrap_or(50);
        let s = normalize(value, stat_scale.min, stat_scale.max, true);

        acc += s * weight;
        sum_w += weight;
    }

    let q = if sum_w > EPS { acc / sum_w } else { 0.0 };

    // 2) 감마 커브 (상위 체감)
    let gamma = score_spec.gamma.unwrap_or(default_gamma);
    let q2 = q.powf(gamma);

    // 3) 클램프
    q2.clamp(0.0, 1.0)
}

/// ScoreComboSpec용 (ball_outputs.kick_speed_mps.score_combo 같은 케이스)
pub fn compute_score_combo(
    score_combo: &ScoreComboSpec,
    player_attrs: &PlayerAttributes,
    stat_scale: &StatScale,
    default_gamma: f32,
) -> f32 {
    let mut sum_w = 0.0;
    let mut acc = 0.0;

    // Determinism: HashMap iteration order is not stable. Sort keys for
    // deterministic accumulation (avoids drift in float rounding → decision changes).
    let mut stats: Vec<_> = score_combo.stats.iter().collect();
    stats.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (stat_name, &weight) in stats {
        let value = player_attrs.get_by_key(stat_name).unwrap_or(50);
        let s = normalize(value, stat_scale.min, stat_scale.max, true);

        acc += s * weight;
        sum_w += weight;
    }

    let q = if sum_w > EPS { acc / sum_w } else { 0.0 };

    let gamma = score_combo.gamma.unwrap_or(default_gamma);
    let q2 = q.powf(gamma);

    q2.clamp(0.0, 1.0)
}

/// ============================================================================
/// 3. apply_situational(q, factors, penalties, bonuses) → q'(0..1)
/// ============================================================================

/// Situational factors 적용 (pressure, fatigue, etc)
pub fn apply_situational(
    base_quality: f32,
    situational_spec: &SituationalSpec,
    factors: &HashMap<String, f32>,
) -> f32 {
    let mut q = base_quality;

    // Penalties (예: pressure, fatigue)
    if let Some(penalties) = &situational_spec.penalties {
        // Determinism: HashMap iteration order is not stable. Sort keys for
        // deterministic accumulation (avoids drift in float rounding → decision changes).
        let mut entries: Vec<_> = penalties.iter().collect();
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (factor_name, &weight) in entries {
            let factor_value = factors.get(factor_name).copied().unwrap_or(0.0);
            let f = factor_value.clamp(0.0, 1.0);
            q -= weight * f;
        }
    }

    // Bonuses (예: space, support)
    if let Some(bonuses) = &situational_spec.bonuses {
        // Determinism: HashMap iteration order is not stable. Sort keys for
        // deterministic accumulation (avoids drift in float rounding → decision changes).
        let mut entries: Vec<_> = bonuses.iter().collect();
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (factor_name, &weight) in entries {
            let factor_value = factors.get(factor_name).copied().unwrap_or(0.0);
            let f = factor_value.clamp(0.0, 1.0);
            q += weight * (1.0 - f); // 역수 (압박 낮을수록 보너스)
        }
    }

    q.clamp(0.0, 1.0)
}

/// ============================================================================
/// 4. map_peak(peak_range, q) → real_value
/// ============================================================================

/// Quality(0..1) → 실제 물리 파라미터 (m/s, deg/s, etc)
pub fn map_peak(peak_range: &PeakRange, quality: f32, default_gamma: f32) -> f32 {
    let gamma = peak_range.gamma.unwrap_or(default_gamma);
    let q = quality.clamp(0.0, 1.0);
    let q2 = q.powf(gamma);

    peak_range.min + q2 * (peak_range.max - peak_range.min)
}

/// ============================================================================
/// 5. prob_link(prob_spec, q) → probability (로지스틱)
/// ============================================================================

/// Quality → Probability (sigmoid)
pub fn prob_link(prob_link_spec: &ProbLinkSpec, quality: f32) -> f32 {
    let x = prob_link_spec.steep * (quality - prob_link_spec.mid);
    let p = 1.0 / (1.0 + (-x).exp());
    p.clamp(0.0, 1.0)
}

/// ============================================================================
/// 6. mix_quality(intent_q, exec_q, mix_weights) → q_mix
/// ============================================================================

/// Intent + Execution quality 혼합
pub fn mix_quality(mix_weights: &HashMap<String, f32>, intent_q: f32, exec_q: f32) -> f32 {
    let w_intent = mix_weights.get("intent").copied().unwrap_or(0.5);
    let w_exec = mix_weights.get("execution").copied().unwrap_or(0.5);

    let total = w_intent + w_exec + EPS;
    let q = (intent_q * w_intent + exec_q * w_exec) / total;

    q.clamp(0.0, 1.0)
}

/// ============================================================================
/// 7. error_scale(base_error, execution_q, error_link) → final_error
/// ============================================================================

/// Execution quality → Error scaling (낮을수록 오차 증가)
pub fn error_scale(base_error: f32, execution_q: f32, error_link: &ErrorLinkSpec) -> f32 {
    let scale = (1.0 - execution_q.clamp(0.0, 1.0)).powf(error_link.k);
    base_error * scale
}

/// ============================================================================
/// TESTING
/// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::player::PlayerAttributes;

    fn make_test_stat_scale() -> StatScale {
        StatScale { min: 1, max: 100 }
    }

    #[test]
    fn test_normalize() {
        let scale = make_test_stat_scale();
        assert_eq!(normalize(1, scale.min, scale.max, true), 0.0);
        assert_eq!(normalize(100, scale.min, scale.max, true), 1.0);
        assert!((normalize(50, scale.min, scale.max, true) - 0.494949).abs() < 0.001);
    }

    #[test]
    fn test_compute_score() {
        let mut attrs = PlayerAttributes::default();
        attrs.passing = 80;
        attrs.vision = 70;

        let mut score_spec = ScoreSpec { stats: HashMap::new(), gamma: Some(0.85) };
        score_spec.stats.insert("passing".to_string(), 0.6);
        score_spec.stats.insert("vision".to_string(), 0.4);

        let scale = make_test_stat_scale();
        let q = compute_score(&score_spec, &attrs, &scale, 0.85);

        // q = (0.8*0.6 + 0.7*0.4) / (0.6+0.4) = (0.48 + 0.28) / 1.0 = 0.76
        // q^0.85 ≈ 0.78
        assert!(q > 0.7 && q < 0.85, "Expected ~0.78, got {}", q);
    }

    #[test]
    fn test_apply_situational() {
        let mut factors = HashMap::new();
        factors.insert("pressure".to_string(), 0.8);
        factors.insert("fatigue".to_string(), 0.3);

        let mut penalties = HashMap::new();
        penalties.insert("pressure".to_string(), 0.2);
        penalties.insert("fatigue".to_string(), 0.1);

        let situational = SituationalSpec { penalties: Some(penalties), bonuses: None };

        let base_q = 0.85;
        let adjusted = apply_situational(base_q, &situational, &factors);

        // 0.85 - 0.2*0.8 - 0.1*0.3 = 0.85 - 0.16 - 0.03 = 0.66
        assert!((adjusted - 0.66).abs() < 0.01, "Expected ~0.66, got {}", adjusted);
    }

    #[test]
    fn test_map_peak() {
        let peak = PeakRange { min: 5.0, max: 10.44, gamma: Some(0.75) };

        let v_max = map_peak(&peak, 1.0, 0.75);
        assert!((v_max - 10.44).abs() < 0.01, "Max should be 10.44");

        let v_min = map_peak(&peak, 0.0, 0.75);
        assert!((v_min - 5.0).abs() < 0.01, "Min should be 5.0");
    }

    #[test]
    fn test_prob_link() {
        let link = ProbLinkSpec { steep: 6.0, mid: 0.5 };

        let p_mid = prob_link(&link, 0.5);
        assert!((p_mid - 0.5).abs() < 0.01, "Mid should be ~0.5");

        let p_high = prob_link(&link, 0.9);
        assert!(p_high > 0.85, "High quality should yield high probability");

        let p_low = prob_link(&link, 0.1);
        assert!(p_low < 0.15, "Low quality should yield low probability");
    }

    #[test]
    fn test_mix_quality() {
        let mut weights = HashMap::new();
        weights.insert("intent".to_string(), 0.4);
        weights.insert("execution".to_string(), 0.6);

        let q_mixed = mix_quality(&weights, 0.8, 0.6);
        // (0.8*0.4 + 0.6*0.6) / 1.0 = (0.32 + 0.36) / 1.0 = 0.68
        assert!((q_mixed - 0.68).abs() < 0.01, "Expected ~0.68, got {}", q_mixed);
    }

    #[test]
    fn test_error_scale() {
        let link = ErrorLinkSpec { k: 2.0 };

        let base_error = 1.0;

        let error_high_q = error_scale(base_error, 0.9, &link);
        // (1 - 0.9)^2 = 0.1^2 = 0.01
        assert!((error_high_q - 0.01).abs() < 0.001, "High quality should reduce error");

        let error_low_q = error_scale(base_error, 0.3, &link);
        // (1 - 0.3)^2 = 0.7^2 = 0.49
        assert!((error_low_q - 0.49).abs() < 0.01, "Low quality should increase error");
    }
}
