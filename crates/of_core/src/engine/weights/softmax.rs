use rand::Rng;

pub fn sample_softmax_ln<R: Rng, T: Copy>(
    rng: &mut R,
    cands: &[crate::engine::match_sim::decision_topology::OutcomeCandidate<T>],
    temp: f32,
) -> Option<crate::engine::match_sim::decision_topology::OutcomeCandidate<T>> {
    if cands.is_empty() {
        return None;
    }
    let t = temp.clamp(0.2, 5.0);

    // scores = ln(W)/T
    let mut scores: Vec<f32> = cands
        .iter()
        .map(|c| (c.w.to_weight().max(crate::engine::weights::W_MIN)).ln() / t)
        .collect();

    // stable softmax
    let maxv = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    for s in &mut scores {
        *s = (*s - maxv).exp();
    }
    let sum: f32 = scores.iter().sum();
    if sum <= 0.0 {
        return Some(cands[0].clone());
    }

    let mut r = rng.gen::<f32>() * sum;
    for (i, w) in scores.iter().enumerate() {
        r -= *w;
        if r <= 0.0 {
            return Some(cands[i].clone());
        }
    }
    Some(cands[cands.len() - 1].clone())
}
