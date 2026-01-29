//! Weight Composer (Contract v1.0)
//!
//! Aggregates various modifiers (TeamInstructions, Tactics Card, Personality, etc.)
//! into a final weight factor using log-linear addition.
//!
//! ## 핵심 공식
//! ln(W_total) = ln(W_base) + Σ ln(factor_i)

/// 가중치 합성 규칙
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackRule {
    /// 모든 요소를 로그 공간에서 합산 (기본값)
    AddLn,
    /// 가장 강한 효과 하나만 적용
    MaxOnly,
    /// 상위 K개의 평균 적용
    TopK(usize),
}

/// 개별 가중치 요소
#[derive(Debug, Clone)]
pub struct WeightFactor {
    pub name: String,
    pub factor: f32, // 0.7 ~ 1.3 권장
    pub rule: StackRule,
}

/// 가중치 합성기
#[derive(Debug, Clone, Default)]
pub struct WeightComposer {
    pub base_weight: f32,
    pub factors: Vec<WeightFactor>,
}

impl WeightComposer {
    pub fn new(base_weight: f32) -> Self {
        Self { base_weight: base_weight.max(0.01), factors: Vec::new() }
    }

    /// 요소 추가
    pub fn add(&mut self, name: &str, factor: f32, rule: StackRule) {
        self.factors.push(WeightFactor {
            name: name.to_string(),
            factor: factor.clamp(0.01, 100.0),
            rule,
        });
    }

    /// 최종 가중치 계산
    pub fn compose(&self) -> f32 {
        let mut ln_total = self.base_weight.ln();

        // 규칙별로 그룹화하여 처리
        let mut add_ln_sum = 0.0;
        let mut top_k_list: Vec<f32> = Vec::new();

        // 단순 구현: MaxOnly는 전체 중 최대값 하나
        let mut max_ln = f32::NEG_INFINITY;
        let mut has_max_only = false;

        for f in &self.factors {
            let ln_f = f.factor.ln();
            match f.rule {
                StackRule::AddLn => {
                    add_ln_sum += ln_f;
                }
                StackRule::MaxOnly => {
                    if ln_f > max_ln {
                        max_ln = ln_f;
                    }
                    has_max_only = true;
                }
                StackRule::TopK(_) => {
                    top_k_list.push(ln_f);
                }
            }
        }

        ln_total += add_ln_sum;

        if has_max_only {
            ln_total += max_ln;
        }

        // TopK 처리 (여기선 일단 AddLn처럼 처리하거나 상위 K개만 선택)
        // 실제 복잡한 룰은 필요시 확장
        for ln_f in top_k_list {
            ln_total += ln_f;
        }

        ln_total.exp().clamp(0.05, 20.0)
    }

    /// 상세 내역 로깅용
    pub fn breakdown(&self) -> String {
        let mut parts = vec![format!("Base: {:.2}", self.base_weight)];
        for f in &self.factors {
            parts.push(format!("{}: {:.2} ({:?})", f.name, f.factor, f.rule));
        }
        format!("[{}] -> Final: {:.2}", parts.join(", "), self.compose())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weight_composition() {
        let mut composer = WeightComposer::new(1.0);
        composer.add("Tactics", 1.2, StackRule::AddLn);
        composer.add("Card", 1.1, StackRule::AddLn);

        let final_w = composer.compose();
        // 1.0 * 1.2 * 1.1 = 1.32
        assert!((final_w - 1.32).abs() < 0.001);
    }

    #[test]
    fn test_max_only_rule() {
        let mut composer = WeightComposer::new(1.0);
        composer.add("CardA", 1.5, StackRule::MaxOnly);
        composer.add("CardB", 1.2, StackRule::MaxOnly);

        let final_w = composer.compose();
        // 1.0 * max(1.5, 1.2) = 1.5
        assert!((final_w - 1.5).abs() < 0.001);
    }
}
