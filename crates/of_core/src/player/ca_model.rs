use crate::models::player::{PlayerAttributes, Position};
use crate::player::ca_weights::{CAWeights, PositionGroup};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Debug, Clone, Copy)]
pub struct CAParams {
    pub alpha: f32,
    pub t: f32,
    pub p: f32,
    pub k: f32,
    pub offset: f32,
}

impl Default for CAParams {
    fn default() -> Self {
        Self { alpha: 1.28, t: 0.88, p: 1.60, k: 0.022, offset: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CAProfile {
    Balanced,
    Spiky,
    Athlete,
    Brain,
}

pub fn calculate_ca(
    attrs: &PlayerAttributes,
    position: Position,
    weights: &CAWeights,
    params: CAParams,
) -> u8 {
    let group = match weights.group_for_position(position) {
        Some(group) => group,
        None => return 1,
    };

    let mut weighted_sum = 0.0f32;
    let mut penalty_sum = 0.0f32;
    let mut weight_sum = 0.0f32;

    for key in weights.attr_keys() {
        let weight = *group.weights.get(key).unwrap_or(&0) as f32;
        if weight <= 0.0 {
            continue;
        }
        let value = attrs.get_by_key(key).unwrap_or(0) as f32 / 100.0;
        weight_sum += weight;
        weighted_sum += weight * value.powf(params.alpha);
        if value > params.t {
            penalty_sum += weight * (value - params.t).powf(params.p);
        }
    }

    if weight_sum <= 0.0 {
        return 1;
    }

    let weighted_avg = weighted_sum / weight_sum;
    let penalty = penalty_sum;
    let raw = 200.0 * weighted_avg + 200.0 * params.k * penalty + params.offset;
    raw.round().clamp(1.0, 200.0) as u8
}

pub fn generate_attributes(
    ca_target: u8,
    position: Position,
    profile: CAProfile,
    seed: u64,
    weights: &CAWeights,
    params: CAParams,
) -> PlayerAttributes {
    let base = (15.0 + ca_target as f32 * 0.45).round();
    let base = base.clamp(weights.attr_range.min as f32, weights.attr_range.max as f32) as u8;
    let mut attrs = PlayerAttributes::from_uniform(base);

    let group = match weights.group_for_position(position) {
        Some(group) => group,
        None => return attrs,
    };

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut keys = weights.attr_keys().to_vec();
    keys.shuffle(&mut rng);
    keys.sort_by(|a, b| {
        let wa = *group.weights.get(a).unwrap_or(&0);
        let wb = *group.weights.get(b).unwrap_or(&0);
        wb.cmp(&wa)
    });

    apply_profile(&mut attrs, group, &keys, profile, &mut rng, weights);
    clamp_with_caps(&mut attrs, group, weights);
    correct_to_target(&mut attrs, position, group, &keys, ca_target, weights, params);
    clamp_with_caps(&mut attrs, group, weights);

    attrs
}

fn apply_profile(
    attrs: &mut PlayerAttributes,
    group: &PositionGroup,
    keys: &[String],
    profile: CAProfile,
    rng: &mut ChaCha8Rng,
    weights: &CAWeights,
) {
    let top_n = 6usize.min(keys.len());
    let bottom_n = 6usize.min(keys.len());
    let spike = 6i16;
    let drop = 4i16;

    match profile {
        CAProfile::Balanced => {
            let jitter_keys: Vec<_> = keys.iter().take(top_n).collect();
            for key in jitter_keys {
                if group.weights.get(key.as_str()).copied().unwrap_or(0) == 0 {
                    continue;
                }
                let delta = rng.gen_range(-2..=2);
                bump_attr(attrs, key, delta, weights);
            }
        }
        CAProfile::Spiky => {
            for key in keys.iter().take(top_n) {
                if group.weights.get(key.as_str()).copied().unwrap_or(0) == 0 {
                    continue;
                }
                let delta = rng.gen_range(spike - 2..=spike + 2);
                bump_attr(attrs, key, delta as i8, weights);
            }
            for key in keys.iter().rev().take(bottom_n) {
                if group.weights.get(key.as_str()).copied().unwrap_or(0) == 0 {
                    continue;
                }
                let delta = rng.gen_range(drop - 2..=drop + 2);
                bump_attr(attrs, key, -(delta as i8), weights);
            }
        }
        CAProfile::Athlete => {
            for key in physical_keys() {
                bump_attr(attrs, key, 6, weights);
            }
            for key in keys.iter().rev().take(bottom_n) {
                if group.weights.get(key.as_str()).copied().unwrap_or(0) <= 2 {
                    bump_attr(attrs, key, -3, weights);
                }
            }
        }
        CAProfile::Brain => {
            for key in brain_keys() {
                bump_attr(attrs, key, 6, weights);
            }
            for key in keys.iter().rev().take(bottom_n) {
                if group.weights.get(key.as_str()).copied().unwrap_or(0) <= 2 {
                    bump_attr(attrs, key, -3, weights);
                }
            }
        }
    }
}

fn correct_to_target(
    attrs: &mut PlayerAttributes,
    position: Position,
    group: &PositionGroup,
    keys: &[String],
    ca_target: u8,
    weights: &CAWeights,
    params: CAParams,
) {
    let max_iters = 30;
    let tolerance = 2i16;
    let weighted_keys: Vec<&String> = keys
        .iter()
        .filter(|key| group.weights.get(key.as_str()).copied().unwrap_or(0) > 0)
        .collect();
    let top_n = 6usize.min(weighted_keys.len());
    let bottom_n = 6usize.min(weighted_keys.len());

    for _ in 0..max_iters {
        let ca_est = calculate_ca(attrs, position, weights, params);
        let err = ca_target as i16 - ca_est as i16;
        if err.abs() <= tolerance {
            break;
        }

        if err > 0 {
            let step = if err > 12 {
                3
            } else if err > 6 {
                2
            } else {
                1
            };
            let mut adjusted = bump_first(attrs, weighted_keys.iter().take(top_n), step, weights);
            if !adjusted {
                adjusted =
                    bump_first(attrs, weighted_keys.iter().skip(top_n).take(top_n), step, weights);
            }
            if !adjusted {
                break;
            }
        } else {
            let step = if err < -12 {
                -3
            } else if err < -6 {
                -2
            } else {
                -1
            };
            let mut adjusted = if err < -12 {
                bump_first(attrs, weighted_keys.iter().take(top_n), step, weights)
            } else {
                bump_first(attrs, weighted_keys.iter().rev().take(bottom_n), step, weights)
            };
            if !adjusted {
                adjusted = bump_first(
                    attrs,
                    weighted_keys.iter().rev().skip(bottom_n).take(bottom_n),
                    step,
                    weights,
                );
            }
            if !adjusted {
                break;
            }
        }
    }
}

fn bump_first<'a, I>(attrs: &mut PlayerAttributes, keys: I, delta: i8, weights: &CAWeights) -> bool
where
    I: IntoIterator<Item = &'a &'a String>,
{
    for key in keys {
        if bump_attr(attrs, key.as_str(), delta, weights) {
            return true;
        }
    }
    false
}

fn clamp_with_caps(attrs: &mut PlayerAttributes, group: &PositionGroup, weights: &CAWeights) {
    for key in weights.attr_keys() {
        let value = attrs.get_by_key(key).unwrap_or(weights.attr_range.min);
        let mut min = weights.attr_range.min;
        let mut max = weights.attr_range.max;
        if let Some(cap) = group.caps.get(key) {
            if let Some(cap_min) = cap.min {
                min = min.max(cap_min);
            }
            if let Some(cap_max) = cap.max {
                max = max.min(cap_max);
            }
        }
        let clamped = value.clamp(min, max);
        set_attr(attrs, key, clamped);
    }
}

fn bump_attr(attrs: &mut PlayerAttributes, key: &str, delta: i8, weights: &CAWeights) -> bool {
    let current = attrs.get_by_key(key).unwrap_or(weights.attr_range.min);
    let target = (current as i16 + delta as i16)
        .clamp(weights.attr_range.min as i16, weights.attr_range.max as i16) as u8;
    set_attr(attrs, key, target)
}

fn physical_keys() -> [&'static str; 8] {
    [
        "acceleration",
        "agility",
        "balance",
        "jumping",
        "natural_fitness",
        "pace",
        "stamina",
        "strength",
    ]
}

fn brain_keys() -> [&'static str; 8] {
    [
        "decisions",
        "vision",
        "passing",
        "composure",
        "anticipation",
        "positioning",
        "teamwork",
        "work_rate",
    ]
}

fn set_attr(attrs: &mut PlayerAttributes, key: &str, value: u8) -> bool {
    match key {
        "corners" => attrs.corners = value,
        "crossing" => attrs.crossing = value,
        "dribbling" => attrs.dribbling = value,
        "finishing" => attrs.finishing = value,
        "first_touch" => attrs.first_touch = value,
        "free_kicks" => attrs.free_kicks = value,
        "heading" => attrs.heading = value,
        "long_shots" => attrs.long_shots = value,
        "long_throws" => attrs.long_throws = value,
        "marking" => attrs.marking = value,
        "passing" => attrs.passing = value,
        "penalty_taking" => attrs.penalty_taking = value,
        "tackling" => attrs.tackling = value,
        "technique" => attrs.technique = value,
        "aggression" => attrs.aggression = value,
        "anticipation" => attrs.anticipation = value,
        "bravery" => attrs.bravery = value,
        "composure" => attrs.composure = value,
        "concentration" => attrs.concentration = value,
        "decisions" => attrs.decisions = value,
        "determination" => attrs.determination = value,
        "flair" => attrs.flair = value,
        "leadership" => attrs.leadership = value,
        "off_the_ball" => attrs.off_the_ball = value,
        "positioning" => attrs.positioning = value,
        "teamwork" => attrs.teamwork = value,
        "vision" => attrs.vision = value,
        "work_rate" => attrs.work_rate = value,
        "acceleration" => attrs.acceleration = value,
        "agility" => attrs.agility = value,
        "balance" => attrs.balance = value,
        "jumping" => attrs.jumping = value,
        "natural_fitness" => attrs.natural_fitness = value,
        "pace" => attrs.pace = value,
        "stamina" => attrs.stamina = value,
        "strength" => attrs.strength = value,
        _ => return false,
    }
    true
}
