//! Validation module for player data
//!
//! Provides comprehensive validation for player creation and updates,
//! ensuring data integrity and business rule compliance.

use crate::models::player::Position;
use std::fmt;

/// Comprehensive error types for player validation
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Name validation errors
    InvalidName(String),

    /// Age validation errors
    InvalidAge(f32),

    /// Current Ability validation errors
    InvalidCA(u8),

    /// Potential Ability validation errors
    InvalidPA(u8),

    /// PA must be >= CA
    PALessThanCA { ca: u8, pa: u8 },

    /// Position validation errors
    InvalidPosition(String),

    /// Attribute validation errors
    InvalidAttribute { attribute: String, value: u8 },

    /// Growth profile validation errors
    InvalidGrowthProfile(String),

    /// Training response validation errors
    InvalidTrainingResponse { multiplier_type: String, value: f32 },

    /// Generic validation error
    ValidationFailed(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidName(msg) => write!(f, "Invalid name: {}", msg),
            ValidationError::InvalidAge(age) => {
                write!(f, "Invalid age: {}. Must be between 15.0 and 18.0 months", age)
            }
            ValidationError::InvalidCA(ca) => {
                write!(f, "Invalid CA: {}. Must be between 0 and 200", ca)
            }
            ValidationError::InvalidPA(pa) => {
                write!(f, "Invalid PA: {}. Must be between 80 and 180", pa)
            }
            ValidationError::PALessThanCA { ca, pa } => {
                write!(f, "PA ({}) must be >= CA ({})", pa, ca)
            }
            ValidationError::InvalidPosition(pos) => write!(f, "Invalid position: {}", pos),
            ValidationError::InvalidAttribute { attribute, value } => {
                write!(f, "Invalid attribute {}: {}. Must be between 0 and 100", attribute, value)
            }
            ValidationError::InvalidGrowthProfile(msg) => {
                write!(f, "Invalid growth profile: {}", msg)
            }
            ValidationError::InvalidTrainingResponse { multiplier_type, value } => {
                write!(
                    f,
                    "Invalid training response {}: {}. Must be between 0.5 and 2.0",
                    multiplier_type, value
                )
            }
            ValidationError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Player validation utility
pub struct PlayerValidator;

impl PlayerValidator {
    /// Validate player name (1-50 characters, alphanumeric + spaces)
    pub fn validate_name(name: &str) -> Result<(), ValidationError> {
        if name.is_empty() {
            return Err(ValidationError::InvalidName("Name cannot be empty".to_string()));
        }

        if name.len() > 50 {
            return Err(ValidationError::InvalidName(
                "Name cannot exceed 50 characters".to_string(),
            ));
        }

        // Check for valid characters (alphanumeric, spaces, basic punctuation)
        if !name.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || ".-'".contains(c)) {
            return Err(ValidationError::InvalidName(
                "Name contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate age (15.0-18.0 months for high school)
    pub fn validate_age(age_months: f32) -> Result<(), ValidationError> {
        if !(15.0..=18.0).contains(&age_months) {
            return Err(ValidationError::InvalidAge(age_months));
        }
        Ok(())
    }

    /// Validate Current Ability (0-200 range)
    pub fn validate_ca(ca: u8) -> Result<(), ValidationError> {
        if ca > 200 {
            return Err(ValidationError::InvalidCA(ca));
        }
        Ok(())
    }

    /// Validate Potential Ability (80-180 range, must be >= CA)
    pub fn validate_pa(pa: u8, ca: Option<u8>) -> Result<(), ValidationError> {
        if !(80..=180).contains(&pa) {
            return Err(ValidationError::InvalidPA(pa));
        }

        if let Some(current_ca) = ca {
            if pa < current_ca {
                return Err(ValidationError::PALessThanCA { ca: current_ca, pa });
            }
        }

        Ok(())
    }

    /// Validate CA and PA together
    pub fn validate_ca_pa(ca: u8, pa: u8) -> Result<(), ValidationError> {
        Self::validate_ca(ca)?;
        Self::validate_pa(pa, Some(ca))?;
        Ok(())
    }

    /// Validate position
    pub fn validate_position(_position: &Position) -> Result<(), ValidationError> {
        // All defined positions are valid
        Ok(())
    }

    /// Validate position string
    pub fn validate_position_string(position: &str) -> Result<(), ValidationError> {
        let valid_positions = [
            "GK", "LB", "CB", "RB", "LWB", "RWB", "CDM", "CM", "CAM", "LM", "RM", "LW", "RW", "CF",
            "ST", "DF", "MF", "FW",
        ];

        if !valid_positions.contains(&position) {
            return Err(ValidationError::InvalidPosition(position.to_string()));
        }

        Ok(())
    }

    /// Validate individual attribute (0-100 range)
    pub fn validate_attribute(attribute_name: &str, value: u8) -> Result<(), ValidationError> {
        if value > 100 {
            return Err(ValidationError::InvalidAttribute {
                attribute: attribute_name.to_string(),
                value,
            });
        }
        Ok(())
    }

    /// Validate all attributes in a PlayerAttributes struct (Open-Football compatible)
    pub fn validate_all_attributes(
        attributes: &crate::models::player::PlayerAttributes,
    ) -> Result<(), ValidationError> {
        // Technical attributes (14) - Open-Football compatible
        Self::validate_attribute("corners", attributes.corners)?;
        Self::validate_attribute("crossing", attributes.crossing)?;
        Self::validate_attribute("dribbling", attributes.dribbling)?;
        Self::validate_attribute("finishing", attributes.finishing)?;
        Self::validate_attribute("first_touch", attributes.first_touch)?;
        Self::validate_attribute("free_kicks", attributes.free_kicks)?;
        Self::validate_attribute("heading", attributes.heading)?;
        Self::validate_attribute("long_shots", attributes.long_shots)?;
        Self::validate_attribute("long_throws", attributes.long_throws)?;
        Self::validate_attribute("marking", attributes.marking)?;
        Self::validate_attribute("passing", attributes.passing)?;
        Self::validate_attribute("penalty_taking", attributes.penalty_taking)?;
        Self::validate_attribute("tackling", attributes.tackling)?;
        Self::validate_attribute("technique", attributes.technique)?;

        // Mental attributes (14) - Open-Football compatible
        Self::validate_attribute("aggression", attributes.aggression)?;
        Self::validate_attribute("anticipation", attributes.anticipation)?;
        Self::validate_attribute("bravery", attributes.bravery)?;
        Self::validate_attribute("composure", attributes.composure)?;
        Self::validate_attribute("concentration", attributes.concentration)?;
        Self::validate_attribute("decisions", attributes.decisions)?;
        Self::validate_attribute("determination", attributes.determination)?;
        Self::validate_attribute("flair", attributes.flair)?;
        Self::validate_attribute("leadership", attributes.leadership)?;
        Self::validate_attribute("off_the_ball", attributes.off_the_ball)?;
        Self::validate_attribute("positioning", attributes.positioning)?;
        Self::validate_attribute("teamwork", attributes.teamwork)?;
        Self::validate_attribute("vision", attributes.vision)?;
        Self::validate_attribute("work_rate", attributes.work_rate)?;

        // Physical attributes (8) - Open-Football compatible
        Self::validate_attribute("acceleration", attributes.acceleration)?;
        Self::validate_attribute("agility", attributes.agility)?;
        Self::validate_attribute("balance", attributes.balance)?;
        Self::validate_attribute("jumping", attributes.jumping)?;
        Self::validate_attribute("natural_fitness", attributes.natural_fitness)?;
        Self::validate_attribute("pace", attributes.pace)?;
        Self::validate_attribute("stamina", attributes.stamina)?;
        Self::validate_attribute("strength", attributes.strength)?;

        Ok(())
    }

    /// Validate training response multipliers (0.5-2.0 range)
    pub fn validate_training_response(
        response: &crate::player::types::TrainingResponse,
    ) -> Result<(), ValidationError> {
        if !response.is_valid() {
            if response.technical_multiplier < 0.5 || response.technical_multiplier > 2.0 {
                return Err(ValidationError::InvalidTrainingResponse {
                    multiplier_type: "technical".to_string(),
                    value: response.technical_multiplier,
                });
            }
            if response.physical_multiplier < 0.5 || response.physical_multiplier > 2.0 {
                return Err(ValidationError::InvalidTrainingResponse {
                    multiplier_type: "physical".to_string(),
                    value: response.physical_multiplier,
                });
            }
            if response.mental_multiplier < 0.5 || response.mental_multiplier > 2.0 {
                return Err(ValidationError::InvalidTrainingResponse {
                    multiplier_type: "mental".to_string(),
                    value: response.mental_multiplier,
                });
            }
        }
        Ok(())
    }

    /// Validate growth profile
    pub fn validate_growth_profile(
        profile: &crate::player::types::GrowthProfile,
    ) -> Result<(), ValidationError> {
        if profile.growth_rate < 0.0 || profile.growth_rate > 1.0 {
            return Err(ValidationError::InvalidGrowthProfile(format!(
                "Growth rate {} must be between 0.0 and 1.0",
                profile.growth_rate
            )));
        }

        if profile.injury_prone < 0.0 || profile.injury_prone > 1.0 {
            return Err(ValidationError::InvalidGrowthProfile(format!(
                "Injury proneness {} must be between 0.0 and 1.0",
                profile.injury_prone
            )));
        }

        Self::validate_training_response(&profile.training_response)?;

        Ok(())
    }

    /// Comprehensive validation for CorePlayer creation
    pub fn validate_core_player(
        player: &crate::player::types::CorePlayer,
    ) -> Result<(), ValidationError> {
        Self::validate_name(&player.name)?;
        Self::validate_age(player.age_months)?;
        Self::validate_ca_pa(player.ca, player.pa)?;
        Self::validate_position(&player.position)?;
        Self::validate_all_attributes(&player.detailed_stats)?;
        Self::validate_growth_profile(&player.growth_profile)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Note: PlayerAttributes import needed for validation tests when expanded
    use crate::player::types::{GrowthProfile, TrainingResponse};

    #[test]
    fn test_validate_name() {
        assert!(PlayerValidator::validate_name("John Doe").is_ok());
        assert!(PlayerValidator::validate_name("José María").is_ok());
        assert!(PlayerValidator::validate_name("O'Connor").is_ok());

        assert!(PlayerValidator::validate_name("").is_err());
        assert!(PlayerValidator::validate_name(&"a".repeat(51)).is_err());
        assert!(PlayerValidator::validate_name("Invalid@Name").is_err());
    }

    #[test]
    fn test_validate_age() {
        assert!(PlayerValidator::validate_age(15.0).is_ok());
        assert!(PlayerValidator::validate_age(16.5).is_ok());
        assert!(PlayerValidator::validate_age(18.0).is_ok());

        assert!(PlayerValidator::validate_age(14.9).is_err());
        assert!(PlayerValidator::validate_age(18.1).is_err());
    }

    #[test]
    fn test_validate_ca() {
        assert!(PlayerValidator::validate_ca(0).is_ok());
        assert!(PlayerValidator::validate_ca(100).is_ok());
        assert!(PlayerValidator::validate_ca(200).is_ok());

        assert!(PlayerValidator::validate_ca(201).is_err());
    }

    #[test]
    fn test_validate_pa() {
        assert!(PlayerValidator::validate_pa(80, Some(60)).is_ok());
        assert!(PlayerValidator::validate_pa(120, Some(100)).is_ok());
        assert!(PlayerValidator::validate_pa(180, Some(150)).is_ok());

        assert!(PlayerValidator::validate_pa(79, Some(60)).is_err()); // Below minimum
        assert!(PlayerValidator::validate_pa(181, Some(60)).is_err()); // Above maximum
        assert!(PlayerValidator::validate_pa(100, Some(120)).is_err()); // PA < CA
    }

    #[test]
    fn test_validate_ca_pa() {
        assert!(PlayerValidator::validate_ca_pa(60, 120).is_ok());
        assert!(PlayerValidator::validate_ca_pa(0, 80).is_ok());
        assert!(PlayerValidator::validate_ca_pa(200, 180).is_err()); // CA too high for min PA
        assert!(PlayerValidator::validate_ca_pa(100, 90).is_err()); // PA < CA
    }

    #[test]
    fn test_validate_attribute() {
        assert!(PlayerValidator::validate_attribute("speed", 0).is_ok());
        assert!(PlayerValidator::validate_attribute("speed", 50).is_ok());
        assert!(PlayerValidator::validate_attribute("speed", 100).is_ok());

        assert!(PlayerValidator::validate_attribute("speed", 101).is_err());
    }

    #[test]
    fn test_validate_training_response() {
        let valid_response = TrainingResponse::balanced();
        assert!(PlayerValidator::validate_training_response(&valid_response).is_ok());

        let invalid_response = TrainingResponse {
            technical_multiplier: 3.0, // Too high
            physical_multiplier: 1.0,
            mental_multiplier: 1.0,
        };
        assert!(PlayerValidator::validate_training_response(&invalid_response).is_err());
    }

    #[test]
    fn test_validate_growth_profile() {
        let valid_profile = GrowthProfile::new();
        assert!(PlayerValidator::validate_growth_profile(&valid_profile).is_ok());

        let invalid_profile = GrowthProfile {
            growth_rate: 1.5, // Too high
            specialization: Vec::new(),
            training_response: TrainingResponse::balanced(),
            injury_prone: 0.1,
        };
        assert!(PlayerValidator::validate_growth_profile(&invalid_profile).is_err());
    }
}
