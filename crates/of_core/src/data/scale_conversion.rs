/// Scale conversion utilities for Godot ↔ FM2023 SSOT
///
/// Godot Career Mode: 0-100 scale, 42 attributes
/// FM2023 SSOT: 1-20 scale, 36 attributes
/// Match Engine: 0-100 scale, 36 attributes (converted from FM)
use std::collections::HashMap;

pub struct ScaleConverter;

impl ScaleConverter {
    /// Convert Godot 0-100 to FM 1-20
    pub fn godot_to_fm(value: u8) -> u8 {
        // 0-100 → 1-20 (linear mapping)
        // 0 → 1, 5 → 1, 50 → 10, 95 → 20, 100 → 20
        1 + ((value.min(100) as f32 * 19.0) / 100.0) as u8
    }

    /// Convert FM 1-20 to Godot 0-100
    pub fn fm_to_godot(value: u8) -> u8 {
        // 1-20 → 0-100 (linear mapping)
        // 1 → 5, 10 → 50, 20 → 100
        ((value.saturating_sub(1).min(19) as f32 * 100.0) / 19.0) as u8
    }

    /// Convert FM 1-20 to Match Engine 0-100
    pub fn fm_to_match_engine(value: u8) -> u8 {
        Self::fm_to_godot(value) // Same scale
    }

    /// Convert Godot 42 attrs to FM 36 attrs
    pub fn godot_to_fm_attrs(godot_attrs: &HashMap<String, u8>) -> HashMap<String, u8> {
        let mut fm_attrs = HashMap::new();

        // Technical (14 attrs - same names)
        for attr in &[
            "corners",
            "crossing",
            "dribbling",
            "finishing",
            "first_touch",
            "free_kick_taking",
            "heading",
            "long_shots",
            "long_throws",
            "marking",
            "passing",
            "penalty_taking",
            "tackling",
            "technique",
        ] {
            if let Some(&val) = godot_attrs.get(*attr) {
                fm_attrs.insert(attr.to_string(), Self::godot_to_fm(val));
            }
        }

        // Mental (14 attrs - same names)
        for attr in &[
            "aggression",
            "anticipation",
            "bravery",
            "composure",
            "concentration",
            "decisions",
            "determination",
            "flair",
            "leadership",
            "off_the_ball",
            "positioning",
            "teamwork",
            "vision",
            "work_rate",
        ] {
            if let Some(&val) = godot_attrs.get(*attr) {
                fm_attrs.insert(attr.to_string(), Self::godot_to_fm(val));
            }
        }

        // Physical (8 attrs - same names)
        for attr in &[
            "acceleration",
            "agility",
            "balance",
            "jumping",
            "natural_fitness",
            "pace",
            "stamina",
            "strength",
        ] {
            if let Some(&val) = godot_attrs.get(*attr) {
                fm_attrs.insert(attr.to_string(), Self::godot_to_fm(val));
            }
        }

        // GK attrs: NOT in FM2023 (Godot has 6 GK attrs, FM has 0)
        // Ignore: aerial_reach, command_of_area, communication, handling, kicking, reflexes

        fm_attrs
    }

    /// Convert FM 36 attrs to Godot 42 attrs (with GK defaults)
    pub fn fm_to_godot_attrs(
        fm_attrs: &HashMap<String, u8>,
        position: &str,
    ) -> HashMap<String, u8> {
        let mut godot_attrs = HashMap::new();

        // Convert all 36 FM attrs
        for (attr, &val) in fm_attrs {
            godot_attrs.insert(attr.clone(), Self::fm_to_godot(val));
        }

        // Add GK attrs with smart defaults (if GK position)
        if position == "GK" {
            // GK attrs derived from base attributes (same as OpenFootball engine)
            let reflexes = Self::derive_gk_reflexes(&godot_attrs);
            let handling = Self::derive_gk_handling(&godot_attrs);
            let aerial_reach = Self::derive_gk_aerial(&godot_attrs);
            let command_of_area = Self::derive_gk_command(&godot_attrs);
            let communication = godot_attrs.get("leadership").copied().unwrap_or(50);
            let kicking = godot_attrs.get("passing").copied().unwrap_or(50);

            godot_attrs.insert("reflexes".to_string(), reflexes);
            godot_attrs.insert("handling".to_string(), handling);
            godot_attrs.insert("aerial_reach".to_string(), aerial_reach);
            godot_attrs.insert("command_of_area".to_string(), command_of_area);
            godot_attrs.insert("communication".to_string(), communication);
            godot_attrs.insert("kicking".to_string(), kicking);
        } else {
            // Non-GK: set GK attrs to low values
            for gk_attr in &[
                "reflexes",
                "handling",
                "aerial_reach",
                "command_of_area",
                "communication",
                "kicking",
            ] {
                godot_attrs.insert(gk_attr.to_string(), 10);
            }
        }

        godot_attrs
    }

    /// Convert FM 36 attrs to Match Engine 36 attrs (0-100 scale)
    pub fn fm_to_match_engine_attrs(fm_attrs: &HashMap<String, u8>) -> HashMap<String, u8> {
        let mut match_attrs = HashMap::new();

        for (attr, &val) in fm_attrs {
            match_attrs.insert(attr.clone(), Self::fm_to_match_engine(val));
        }

        match_attrs
    }

    // GK derivation (matches OpenFootball engine logic)
    fn derive_gk_reflexes(attrs: &HashMap<String, u8>) -> u8 {
        // Reflexes ~ Agility + Anticipation
        let agility = attrs.get("agility").copied().unwrap_or(50);
        let anticipation = attrs.get("anticipation").copied().unwrap_or(50);
        ((agility as u16 + anticipation as u16) / 2) as u8
    }

    fn derive_gk_handling(attrs: &HashMap<String, u8>) -> u8 {
        // Handling ~ First Touch + Technique + Concentration
        let first_touch = attrs.get("first_touch").copied().unwrap_or(50);
        let technique = attrs.get("technique").copied().unwrap_or(50);
        let concentration = attrs.get("concentration").copied().unwrap_or(50);
        ((first_touch as u16 + technique as u16 + concentration as u16) / 3) as u8
    }

    fn derive_gk_aerial(attrs: &HashMap<String, u8>) -> u8 {
        // Aerial ~ Jumping + Heading
        let jumping = attrs.get("jumping").copied().unwrap_or(50);
        let heading = attrs.get("heading").copied().unwrap_or(50);
        ((jumping as u16 + heading as u16) / 2) as u8
    }

    fn derive_gk_command(attrs: &HashMap<String, u8>) -> u8 {
        // Command ~ Leadership + Positioning + Concentration
        let leadership = attrs.get("leadership").copied().unwrap_or(50);
        let positioning = attrs.get("positioning").copied().unwrap_or(50);
        let concentration = attrs.get("concentration").copied().unwrap_or(50);
        ((leadership as u16 + positioning as u16 + concentration as u16) / 3) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_conversion_godot_to_fm() {
        assert_eq!(ScaleConverter::godot_to_fm(0), 1);
        assert_eq!(ScaleConverter::godot_to_fm(50), 10);
        assert_eq!(ScaleConverter::godot_to_fm(100), 20);
    }

    #[test]
    fn test_scale_conversion_fm_to_godot() {
        assert_eq!(ScaleConverter::fm_to_godot(1), 0);
        assert_eq!(ScaleConverter::fm_to_godot(10), 47);
        assert_eq!(ScaleConverter::fm_to_godot(20), 100);
    }

    #[test]
    fn test_bidirectional_conversion() {
        for fm_val in 1..=20 {
            let godot = ScaleConverter::fm_to_godot(fm_val);
            let back = ScaleConverter::godot_to_fm(godot);
            assert!((back as i16 - fm_val as i16).abs() <= 1); // Allow ±1 rounding
        }
    }

    #[test]
    fn test_godot_to_fm_attrs() {
        let mut godot_attrs = HashMap::new();
        godot_attrs.insert("finishing".to_string(), 80);
        godot_attrs.insert("passing".to_string(), 60);
        godot_attrs.insert("pace".to_string(), 90);

        let fm_attrs = ScaleConverter::godot_to_fm_attrs(&godot_attrs);

        assert_eq!(fm_attrs.get("finishing"), Some(&16)); // 80 → 16
        assert_eq!(fm_attrs.get("passing"), Some(&12)); // 60 → 12
        assert_eq!(fm_attrs.get("pace"), Some(&18)); // 90 → 18
    }

    #[test]
    fn test_fm_to_godot_attrs_outfield() {
        let mut fm_attrs = HashMap::new();
        fm_attrs.insert("finishing".to_string(), 15);
        fm_attrs.insert("passing".to_string(), 12);
        fm_attrs.insert("pace".to_string(), 18);

        let godot_attrs = ScaleConverter::fm_to_godot_attrs(&fm_attrs, "ST");

        // Check conversions
        assert!(godot_attrs.get("finishing").unwrap() >= &70); // ~73
        assert!(godot_attrs.get("passing").unwrap() >= &55); // ~57
        assert!(godot_attrs.get("pace").unwrap() >= &85); // ~89

        // Check GK attrs set to low values for non-GK
        assert_eq!(godot_attrs.get("reflexes"), Some(&10));
        assert_eq!(godot_attrs.get("handling"), Some(&10));
    }

    #[test]
    fn test_fm_to_godot_attrs_goalkeeper() {
        let mut fm_attrs = HashMap::new();
        fm_attrs.insert("agility".to_string(), 18);
        fm_attrs.insert("anticipation".to_string(), 16);
        fm_attrs.insert("first_touch".to_string(), 14);
        fm_attrs.insert("technique".to_string(), 12);
        fm_attrs.insert("concentration".to_string(), 15);
        fm_attrs.insert("leadership".to_string(), 13);
        fm_attrs.insert("positioning".to_string(), 17);

        let godot_attrs = ScaleConverter::fm_to_godot_attrs(&fm_attrs, "GK");

        // Check GK attrs are derived (not hardcoded to 10)
        let reflexes = godot_attrs.get("reflexes").unwrap();
        let handling = godot_attrs.get("handling").unwrap();
        let command = godot_attrs.get("command_of_area").unwrap();

        assert!(*reflexes > 50); // Should be high (derived from agility + anticipation)
        assert!(*handling > 50); // Should be decent (derived from first_touch + technique + concentration)
        assert!(*command > 50); // Should be decent (derived from leadership + positioning + concentration)
    }
}
