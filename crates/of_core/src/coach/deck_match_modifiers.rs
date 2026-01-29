use super::card::{create_default_coach, create_default_manager, Specialty};
use super::deck::Deck;
use super::tactics::TacticalStyle;

#[derive(Debug, Clone, Copy, Default)]
pub struct SpecialtyWeights {
    pub speed: f32,
    pub power: f32,
    pub technical: f32,
    pub mental: f32,
    pub balanced: f32,
}

#[derive(Debug, Clone)]
pub struct DeckMatchModifiers {
    pub quality01: f32,
    pub specialty_weights: SpecialtyWeights,
    pub tactical_styles: Vec<TacticalStyle>,

    pub pass_success_mult: f32,
    pub shot_accuracy_mult: f32,
    pub shot_power_mult: f32,
    pub tackle_success_mult: f32,
    pub press_intensity_add: f32,
    pub stamina_drain_mult: f32,
}

impl DeckMatchModifiers {
    pub fn to_mod_list(&self) -> Vec<(u8, f32)> {
        vec![
            (1, self.pass_success_mult),
            (2, self.shot_accuracy_mult),
            (3, self.shot_power_mult),
            (4, self.tackle_success_mult),
            (5, self.press_intensity_add),
            (6, self.stamina_drain_mult),
        ]
    }
}

pub fn derive_match_modifiers(deck: &Deck) -> DeckMatchModifiers {
    let default_manager = create_default_manager();
    let manager = deck.manager_card.as_ref().unwrap_or(&default_manager);

    let mut coaches = Vec::with_capacity(3);
    for slot in &deck.coach_cards {
        let coach = match slot {
            Some(c) => c.clone(),
            None => create_default_coach(Specialty::Balanced),
        };
        coaches.push(coach);
    }

    let avg_rarity = (manager.rarity as u8 as f32
        + coaches.iter().map(|c| c.rarity as u8 as f32).sum::<f32>())
        / 4.0;
    let quality01 = clamp01((avg_rarity - 1.0) / 4.0);

    let weights = specialty_weights(manager.specialty, coaches.iter().map(|c| c.specialty));

    let tactical_styles = collect_tactical_styles(deck);
    let tactics_pass_bonus = calc_tactics_pass_bonus(quality01, &tactical_styles);
    let tactics_press_bonus = calc_tactics_press_bonus(quality01, &tactical_styles);
    let tactics_stamina_bonus = calc_tactics_stamina_bonus(quality01, &tactical_styles);
    let tactics_shot_acc_bonus = calc_tactics_shot_acc_bonus(quality01, &tactical_styles);
    let tactics_shot_pow_bonus = calc_tactics_shot_pow_bonus(quality01, &tactical_styles);
    let tactics_tackle_bonus = calc_tactics_tackle_bonus(quality01, &tactical_styles);

    let pass_success_mult = clamp(
        1.00,
        1.20,
        1.00
            + quality01
                * (0.12 * weights.technical + 0.06 * weights.mental + 0.06 * weights.balanced)
            + tactics_pass_bonus,
    );
    let shot_accuracy_mult = clamp(
        1.00,
        1.20,
        1.00
            + quality01
                * (0.10 * weights.technical + 0.08 * weights.mental + 0.06 * weights.balanced)
            + tactics_shot_acc_bonus,
    );
    let shot_power_mult = clamp(
        1.00,
        1.20,
        1.00 + quality01 * (0.12 * weights.power + 0.06 * weights.balanced) + tactics_shot_pow_bonus,
    );
    let tackle_success_mult = clamp(
        1.00,
        1.20,
        1.00
            + quality01 * (0.10 * weights.power + 0.06 * weights.mental + 0.06 * weights.balanced)
            + tactics_tackle_bonus,
    );
    let press_intensity_add = clamp(
        0.00,
        0.30,
        quality01 * (0.18 * weights.speed + 0.10 * weights.mental + 0.06 * weights.balanced)
            + tactics_press_bonus,
    );
    let stamina_drain_mult = clamp(
        0.90,
        1.20,
        1.00
            + quality01 * (0.10 * weights.speed - 0.10 * weights.mental - 0.06 * weights.balanced)
            + tactics_stamina_bonus,
    );

    DeckMatchModifiers {
        quality01,
        specialty_weights: weights,
        tactical_styles,
        pass_success_mult,
        shot_accuracy_mult,
        shot_power_mult,
        tackle_success_mult,
        press_intensity_add,
        stamina_drain_mult,
    }
}

fn clamp(min: f32, max: f32, x: f32) -> f32 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

fn clamp01(x: f32) -> f32 {
    clamp(0.0, 1.0, x)
}

fn specialty_weights(
    manager_specialty: Specialty,
    coach_specialties: impl Iterator<Item = Specialty>,
) -> SpecialtyWeights {
    let mut speed = 0.0;
    let mut power = 0.0;
    let mut technical = 0.0;
    let mut mental = 0.0;
    let mut balanced = 0.0;

    for s in std::iter::once(manager_specialty).chain(coach_specialties) {
        match s {
            Specialty::Speed => speed += 1.0,
            Specialty::Power => power += 1.0,
            Specialty::Technical => technical += 1.0,
            Specialty::Mental => mental += 1.0,
            Specialty::Balanced => balanced += 1.0,
        }
    }

    let total = 4.0;
    SpecialtyWeights {
        speed: speed / total,
        power: power / total,
        technical: technical / total,
        mental: mental / total,
        balanced: balanced / total,
    }
}

fn collect_tactical_styles(deck: &Deck) -> Vec<TacticalStyle> {
    // Keep stable ordering for deterministic output.
    let mut present = std::collections::HashSet::new();
    for slot in &deck.tactics_cards {
        if let Some(t) = slot {
            present.insert(t.tactical_style);
        }
    }

    let ordered = [
        TacticalStyle::Defensive,
        TacticalStyle::Balanced,
        TacticalStyle::Attacking,
        TacticalStyle::CounterAttack,
        TacticalStyle::Possession,
        TacticalStyle::Pressing,
        TacticalStyle::DirectPlay,
        TacticalStyle::WingPlay,
    ];

    ordered.iter().copied().filter(|s| present.contains(s)).collect()
}

fn calc_tactics_pass_bonus(quality01: f32, styles: &[TacticalStyle]) -> f32 {
    let mut bonus = 0.0;
    if styles.contains(&TacticalStyle::Possession) {
        bonus += 0.03 * quality01;
    }
    if styles.contains(&TacticalStyle::DirectPlay) {
        bonus += 0.01 * quality01;
    }
    if styles.contains(&TacticalStyle::WingPlay) {
        bonus += 0.01 * quality01;
    }
    bonus
}

fn calc_tactics_press_bonus(quality01: f32, styles: &[TacticalStyle]) -> f32 {
    let mut bonus = 0.0;
    if styles.contains(&TacticalStyle::Pressing) {
        bonus += 0.04 * quality01;
    }
    bonus
}

fn calc_tactics_stamina_bonus(quality01: f32, styles: &[TacticalStyle]) -> f32 {
    let mut bonus = 0.0;
    if styles.contains(&TacticalStyle::Defensive) {
        bonus -= 0.03 * quality01;
    }
    if styles.contains(&TacticalStyle::Pressing) {
        bonus += 0.03 * quality01;
    }
    bonus
}

fn calc_tactics_shot_acc_bonus(quality01: f32, styles: &[TacticalStyle]) -> f32 {
    let mut bonus = 0.0;
    if styles.contains(&TacticalStyle::Attacking) {
        bonus += 0.02 * quality01;
    }
    if styles.contains(&TacticalStyle::CounterAttack) {
        bonus += 0.02 * quality01;
    }
    bonus
}

fn calc_tactics_shot_pow_bonus(quality01: f32, styles: &[TacticalStyle]) -> f32 {
    let mut bonus = 0.0;
    if styles.contains(&TacticalStyle::DirectPlay) {
        bonus += 0.03 * quality01;
    }
    bonus
}

fn calc_tactics_tackle_bonus(quality01: f32, styles: &[TacticalStyle]) -> f32 {
    let mut bonus = 0.0;
    if styles.contains(&TacticalStyle::Defensive) {
        bonus += 0.03 * quality01;
    }
    bonus
}
