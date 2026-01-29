//! JSON API for player operations
//!
//! This module provides JSON-based API endpoints for Godot integration,
//! supporting player creation, updates, retrieval, and batch operations.

use crate::models::player::{PlayerAttributes, Position};
use crate::player::{
    AttributeChange, AttributeGrowth, CACalculator, CorePlayer, GrowthCalculator, GrowthProfile,
    HexagonStats, MonthlyGrowth, PersonAttributes, PlayerValidator, TrainingType, ValidationError,
};
use crate::special_ability::{
    AbilityActivationContext, AbilityTier, ProcessingResult, SpecialAbility, SpecialAbilityType,
};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// API version for schema compatibility
pub const API_VERSION: &str = "v1";

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub schema_version: String,
    pub timestamp: DateTime<Utc>,
}

/// Structured API error with codes and details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Player creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerCreationRequest {
    pub schema_version: Option<String>,
    pub name: String,
    pub position: Position,
    pub age_months: f32,
    pub ca: Option<u8>,
    pub pa: Option<u8>,
    pub seed: Option<u64>,
    pub custom_attributes: Option<PlayerAttributes>,
    pub growth_profile: Option<GrowthProfile>,
}

/// Player creation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerCreationResponse {
    pub player: CorePlayer,
    pub generated_with_seed: Option<u64>,
}

/// Player update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerUpdateRequest {
    pub schema_version: Option<String>,
    pub player_id: String,
    pub attribute_changes: HashMap<String, i8>, // attribute_name -> change_amount
    pub position_change: Option<Position>,
    pub name_change: Option<String>,
    pub atomic: Option<bool>, // All changes or none (default: true)
}

/// Player update response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerUpdateResponse {
    pub updated_player: CorePlayer,
    pub ca_change: i8,
    pub hexagon_changes: HexagonStatsChange,
    pub attribute_changes: Vec<AttributeChange>,
}

/// Hexagon stats change tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexagonStatsChange {
    pub before: HexagonStats,
    pub after: HexagonStats,
    pub pace_change: i8,
    pub power_change: i8,
    pub technical_change: i8,
    pub shooting_change: i8,
    pub passing_change: i8,
    pub defending_change: i8,
}

/// Player query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerQueryRequest {
    pub schema_version: Option<String>,
    pub player_id: String,
    pub include_fields: Option<Vec<String>>, // Future: field selection
}

/// Player query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerQueryResponse {
    pub player: Option<CorePlayer>,
}

/// Batch player creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPlayerCreationRequest {
    pub schema_version: Option<String>,
    pub players: Vec<PlayerCreationRequest>,
    pub batch_seed: Option<u64>,    // Base seed for batch
    pub max_concurrent: Option<u8>, // Performance limit (default: 10)
}

/// Batch player creation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPlayerCreationResponse {
    pub created_players: Vec<CorePlayer>,
    pub failed_requests: Vec<BatchFailure>,
    pub total_requested: usize,
    pub total_created: usize,
    pub total_failed: usize,
}

/// Batch player update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPlayerUpdateRequest {
    pub schema_version: Option<String>,
    pub updates: Vec<PlayerUpdateRequest>,
    pub max_concurrent: Option<u8>,
}

/// Batch player update response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPlayerUpdateResponse {
    pub updated_players: Vec<CorePlayer>,
    pub failed_updates: Vec<BatchFailure>,
    pub total_requested: usize,
    pub total_updated: usize,
    pub total_failed: usize,
}

/// Individual failure in batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFailure {
    pub index: usize,
    pub player_id: Option<String>,
    pub error: ApiError,
}

/// Growth simulation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthSimulationRequest {
    pub schema_version: Option<String>,
    pub player_id: String,
    pub training_schedule: Vec<TrainingSession>,
    pub months: u32,
    pub random_variance: Option<f32>, // 0.0-1.0, default 0.05 (5%)
    pub seed: Option<u64>,
}

/// Training session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSession {
    pub training_type: TrainingType,
    pub intensity: f32,         // 0.1-2.0 multiplier
    pub sessions_per_month: u8, // 1-8
}

/// Growth simulation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthSimulationResponse {
    pub initial_player: CorePlayer,
    pub final_player: CorePlayer,
    pub growth_history: Vec<MonthlyGrowth>,
    pub total_ca_growth: i16,
    pub total_attribute_changes: HashMap<String, i8>,
    pub average_monthly_growth: f32,
}

/// Player comparison request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerComparisonRequest {
    pub schema_version: Option<String>,
    pub player_ids: Vec<String>,
    pub comparison_type: ComparisonType,
    pub include_similarity_scores: Option<bool>,
}

/// Type of player comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonType {
    Overall,          // CA, hexagon stats, growth potential
    PositionSpecific, // Position-weighted comparison
    Attributes,       // Detailed attribute comparison
    GrowthPotential,  // PA, growth rate, age factors
}

/// Player comparison response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerComparisonResponse {
    pub players: Vec<CorePlayer>,
    pub comparison_matrix: Vec<Vec<f32>>, // Similarity scores matrix
    pub rankings: HashMap<String, Vec<PlayerRanking>>, // Category -> ranked players
    pub summary_stats: ComparisonSummary,
}

/// Player ranking in comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRanking {
    pub player_id: String,
    pub rank: u8,
    pub score: f32,
    pub percentile: f32,
}

/// Summary statistics for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonSummary {
    pub average_ca: f32,
    pub ca_range: (u8, u8),
    pub average_pa: f32,
    pub pa_range: (u8, u8),
    pub position_distribution: HashMap<Position, u8>,
    pub age_range: (f32, f32),
}

/// Bulk export request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkExportRequest {
    pub schema_version: Option<String>,
    pub player_ids: Vec<String>,
    pub format: ExportFormat,
    pub include_metadata: Option<bool>,
}

/// Export format options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    JsonCompact,
    Csv,
}

/// Bulk export response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkExportResponse {
    pub data: String, // Serialized data in requested format
    pub format: ExportFormat,
    pub player_count: usize,
    pub export_timestamp: DateTime<Utc>,
}

/// Bulk import request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportRequest {
    pub schema_version: Option<String>,
    pub data: String, // Serialized data
    pub format: ExportFormat,
    pub validate_only: Option<bool>, // Dry run
    pub overwrite_existing: Option<bool>,
}

/// Bulk import response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportResponse {
    pub imported_players: Vec<CorePlayer>,
    pub validation_errors: Vec<BatchFailure>,
    pub total_processed: usize,
    pub total_imported: usize,
    pub total_errors: usize,
    pub dry_run: bool,
}

impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        Self { code: code.to_string(), message: message.to_string(), details: None }
    }

    pub fn with_details(
        code: &str,
        message: &str,
        details: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self { code: code.to_string(), message: message.to_string(), details: Some(details) }
    }

    pub fn from_validation_error(error: ValidationError) -> Self {
        let code = match error {
            ValidationError::InvalidName(_) => "INVALID_NAME",
            ValidationError::InvalidAge(_) => "INVALID_AGE",
            ValidationError::InvalidCA(_) => "INVALID_CA",
            ValidationError::InvalidPA(_) => "INVALID_PA",
            ValidationError::PALessThanCA { ca: _, pa: _ } => "PA_LESS_THAN_CA",
            ValidationError::InvalidPosition(_) => "INVALID_POSITION",
            ValidationError::InvalidAttribute { attribute: _, value: _ } => "INVALID_ATTRIBUTE",
            ValidationError::InvalidGrowthProfile(_) => "INVALID_GROWTH_PROFILE",
            ValidationError::InvalidTrainingResponse { multiplier_type: _, value: _ } => {
                "INVALID_TRAINING_RESPONSE"
            }
            ValidationError::ValidationFailed(_) => "VALIDATION_FAILED",
        };

        Self::new(code, &error.to_string())
    }
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            schema_version: API_VERSION.to_string(),
            timestamp: Utc::now(),
        }
    }

    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            schema_version: API_VERSION.to_string(),
            timestamp: Utc::now(),
        }
    }
}

impl HexagonStatsChange {
    pub fn new(before: HexagonStats, after: HexagonStats) -> Self {
        Self {
            pace_change: after.pace as i8 - before.pace as i8,
            power_change: after.power as i8 - before.power as i8,
            technical_change: after.technical as i8 - before.technical as i8,
            shooting_change: after.shooting as i8 - before.shooting as i8,
            passing_change: after.passing as i8 - before.passing as i8,
            defending_change: after.defending as i8 - before.defending as i8,
            before,
            after,
        }
    }
}

/// Helper functions for API operations
impl PlayerCreationRequest {
    /// Validate the creation request
    pub fn validate(&self) -> Result<(), ApiError> {
        // PlayerValidator has static methods only

        // Validate name
        if let Err(e) = PlayerValidator::validate_name(&self.name) {
            return Err(ApiError::from_validation_error(e));
        }

        // Validate age
        if let Err(e) = PlayerValidator::validate_age(self.age_months) {
            return Err(ApiError::from_validation_error(e));
        }

        // Validate CA if provided
        if let Some(ca) = self.ca {
            if let Err(e) = PlayerValidator::validate_ca(ca) {
                return Err(ApiError::from_validation_error(e));
            }
        }

        // Validate PA if provided
        if let Some(pa) = self.pa {
            if let Err(e) = PlayerValidator::validate_pa(pa, None) {
                return Err(ApiError::from_validation_error(e));
            }

            // Validate PA >= CA if both provided
            if let Some(ca) = self.ca {
                if pa < ca {
                    return Err(ApiError::from_validation_error(ValidationError::PALessThanCA {
                        ca,
                        pa,
                    }));
                }
            }
        }

        // Validate position
        if let Err(e) = PlayerValidator::validate_position(&self.position) {
            return Err(ApiError::from_validation_error(e));
        }

        // Validate custom attributes if provided
        if let Some(ref attrs) = self.custom_attributes {
            if let Err(e) = PlayerValidator::validate_all_attributes(attrs) {
                return Err(ApiError::from_validation_error(e));
            }
        }

        Ok(())
    }
}

impl PlayerUpdateRequest {
    /// Validate the update request
    pub fn validate(&self) -> Result<(), ApiError> {
        // PlayerValidator has static methods only

        // Validate player ID format (UUID)
        if Uuid::parse_str(&self.player_id).is_err() {
            return Err(ApiError::new("INVALID_PLAYER_ID", "Player ID must be a valid UUID"));
        }

        // Validate attribute changes
        for (attr_name, change) in &self.attribute_changes {
            // Check attribute name is valid
            if !is_valid_attribute_name(attr_name) {
                return Err(ApiError::new(
                    "INVALID_ATTRIBUTE_NAME",
                    &format!("Invalid attribute name: {}", attr_name),
                ));
            }

            // Check change amount is reasonable (-100 to +100)
            if *change < -100 || *change > 100 {
                return Err(ApiError::new(
                    "INVALID_CHANGE_AMOUNT",
                    &format!("Change amount {} is out of range (-100 to +100)", change),
                ));
            }
        }

        // Validate name change if provided
        if let Some(ref new_name) = self.name_change {
            if let Err(e) = PlayerValidator::validate_name(new_name) {
                return Err(ApiError::from_validation_error(e));
            }
        }

        // Validate position change if provided
        if let Some(new_position) = self.position_change {
            if let Err(e) = PlayerValidator::validate_position(&new_position) {
                return Err(ApiError::from_validation_error(e));
            }
        }

        Ok(())
    }
}

/// Check if attribute name is valid
fn is_valid_attribute_name(name: &str) -> bool {
    matches!(
        name,
        "pace"
            | "acceleration"
            | "agility"
            | "balance"
            | "jumping"
            | "natural_fitness"
            | "stamina"
            | "strength"
            | "corners"
            | "crossing"
            | "dribbling"
            | "finishing"
            | "first_touch"
            | "free_kicks"
            | "heading"
            | "long_shots"
            | "long_throws"
            | "marking"
            | "passing"
            | "penalty_taking"
            | "shooting"
            | "tackling"
            | "technique"
            | "anticipation"
            | "bravery"
            | "composure"
            | "concentration"
            | "creativity"
            | "decisions"
            | "determination"
            | "flair"
            | "influence"
            | "off_the_ball"
            | "positioning"
            | "teamwork"
            | "vision"
            | "work_rate"
            | "aerial_ability"
            | "command_of_area"
            | "communication"
            | "eccentricity"
            | "handling"
            | "kicking"
            | "one_on_ones"
            | "reflexes"
            | "rushing_out"
            | "throwing"
            | "aggression"
    )
}

/// Core API implementation functions

/// Create a player from JSON request string
///
/// # Arguments
/// * `request_json` - JSON string containing PlayerCreationRequest
///
/// # Returns
/// JSON string containing ApiResponse<PlayerCreationResponse>
pub fn create_player_json(request_json: &str) -> String {
    info!("Processing player creation request");

    // Parse the request
    let request: PlayerCreationRequest = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse PlayerCreationRequest: {}", e);
            let error = ApiError::new("INVALID_JSON", &format!("Invalid JSON format: {}", e));
            let response: ApiResponse<PlayerCreationResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    // Validate the request
    if let Err(error) = request.validate() {
        warn!("Player creation request validation failed: {:?}", error);
        let response: ApiResponse<PlayerCreationResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    // Create the player
    match create_player_from_request(&request) {
        Ok((player, seed_used)) => {
            info!("Successfully created player: {} (ID: {})", player.name, player.id);
            let response_data = PlayerCreationResponse { player, generated_with_seed: seed_used };
            let response = ApiResponse::success(response_data);
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
        }
        Err(error) => {
            error!("Failed to create player: {}", error.message);
            let response: ApiResponse<PlayerCreationResponse> = ApiResponse::error(error);
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

/// Update a player from JSON request string
///
/// # Arguments
/// * `request_json` - JSON string containing PlayerUpdateRequest
/// * `players` - Mutable reference to HashMap storing players by ID
///
/// # Returns
/// JSON string containing ApiResponse<PlayerUpdateResponse>
pub fn update_player_json(request_json: &str, players: &mut HashMap<String, CorePlayer>) -> String {
    info!("Processing player update request");

    // Parse the request
    let request: PlayerUpdateRequest = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse PlayerUpdateRequest: {}", e);
            let error = ApiError::new("INVALID_JSON", &format!("Invalid JSON format: {}", e));
            let response: ApiResponse<PlayerUpdateResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    // Validate the request
    if let Err(error) = request.validate() {
        warn!("Player update request validation failed: {:?}", error);
        let response: ApiResponse<PlayerUpdateResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    // Find the player
    let player = match players.get_mut(&request.player_id) {
        Some(p) => p,
        None => {
            let error = ApiError::new(
                "PLAYER_NOT_FOUND",
                &format!("Player with ID {} not found", request.player_id),
            );
            let response: ApiResponse<PlayerUpdateResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    // Apply updates
    match apply_player_updates(player, &request) {
        Ok(update_response) => {
            info!("Successfully updated player: {} (ID: {})", player.name, player.id);
            let response = ApiResponse::success(update_response);
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
        }
        Err(error) => {
            error!("Failed to update player: {}", error.message);
            let response: ApiResponse<PlayerUpdateResponse> = ApiResponse::error(error);
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

/// Retrieve a player from JSON request string
///
/// # Arguments
/// * `request_json` - JSON string containing PlayerQueryRequest
/// * `players` - Reference to HashMap storing players by ID
///
/// # Returns
/// JSON string containing ApiResponse<PlayerQueryResponse>
pub fn get_player_json(request_json: &str, players: &HashMap<String, CorePlayer>) -> String {
    debug!("Processing player query request");

    // Parse the request
    let request: PlayerQueryRequest = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse PlayerQueryRequest: {}", e);
            let error = ApiError::new("INVALID_JSON", &format!("Invalid JSON format: {}", e));
            let response: ApiResponse<PlayerQueryResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    // Validate player ID format
    if Uuid::parse_str(&request.player_id).is_err() {
        let error = ApiError::new("INVALID_PLAYER_ID", "Player ID must be a valid UUID");
        let response: ApiResponse<PlayerQueryResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    // Find the player
    let player = players.get(&request.player_id).cloned();

    if player.is_some() {
        debug!("Found player: {}", request.player_id);
    } else {
        debug!("Player not found: {}", request.player_id);
    }

    let response_data = PlayerQueryResponse { player };
    let response = ApiResponse::success(response_data);
    serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
}

/// Helper function to create a player from a request
fn create_player_from_request(
    request: &PlayerCreationRequest,
) -> Result<(CorePlayer, Option<u64>), ApiError> {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    // Set up RNG
    let seed_used = request.seed.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
    });
    let mut rng = ChaCha8Rng::seed_from_u64(seed_used);

    // Generate or use provided attributes
    let detailed_stats = if let Some(ref custom_attrs) = request.custom_attributes {
        custom_attrs.clone()
    } else {
        generate_realistic_attributes(request.position, &mut rng)
    };

    // Generate or use provided growth profile
    let growth_profile = if let Some(ref custom_profile) = request.growth_profile {
        custom_profile.clone()
    } else {
        generate_growth_profile(request.position, &mut rng)
    };

    // Calculate CA and PA if not provided
    let calculated_ca = CACalculator::calculate(&detailed_stats, request.position);
    let ca = request.ca.unwrap_or(calculated_ca);
    let pa = request.pa.unwrap_or_else(|| {
        // Generate reasonable PA based on CA and age
        let base_pa = (ca as f32 * 1.2 + rng.gen_range(10.0..30.0)) as u8;
        base_pa.max(ca + 10).min(180)
    });

    // Validate PA >= CA
    if pa < ca {
        return Err(ApiError::new(
            "INVALID_PA_CA_RATIO",
            &format!("PA ({}) must be greater than or equal to CA ({})", pa, ca),
        ));
    }

    // Generate personality based on seed
    let personality = PersonAttributes::generate_random(seed_used + 1000);

    // Create the player
    let player = CorePlayer::new(
        request.name.clone(),
        request.position,
        request.age_months,
        ca,
        pa,
        detailed_stats,
        growth_profile,
        personality,
    );

    Ok((player, Some(seed_used)))
}

/// Helper function to apply player updates
fn apply_player_updates(
    player: &mut CorePlayer,
    request: &PlayerUpdateRequest,
) -> Result<PlayerUpdateResponse, ApiError> {
    let before_ca = player.ca;
    let before_hexagon = player.hexagon_stats.clone();
    let mut attribute_changes = Vec::new();

    // Apply name change if requested
    if let Some(ref new_name) = request.name_change {
        player.name = new_name.clone();
        player.touch();
    }

    // Apply position change if requested
    if let Some(new_position) = request.position_change {
        if new_position != player.position {
            player.change_position(new_position);
        }
    }

    // Apply attribute changes
    if !request.attribute_changes.is_empty() {
        // Convert HashMap to Vec of tuples for modify_multiple_attributes
        let changes: Vec<(&str, i8)> = request
            .attribute_changes
            .iter()
            .map(|(name, change)| (name.as_str(), *change))
            .collect();

        match player.modify_multiple_attributes(&changes) {
            Ok(attr_changes) => {
                attribute_changes.extend(attr_changes);
            }
            Err(e) => {
                return Err(ApiError::new("ATTRIBUTE_UPDATE_FAILED", &e));
            }
        }
    }

    let ca_change = (player.ca as i16 - before_ca as i16).clamp(-128, 127) as i8;
    let hexagon_changes = HexagonStatsChange::new(before_hexagon, player.hexagon_stats.clone());

    Ok(PlayerUpdateResponse {
        updated_player: player.clone(),
        ca_change,
        hexagon_changes,
        attribute_changes,
    })
}

/// Generate realistic attributes for a position using OpenFootball 36-field system
fn generate_realistic_attributes(position: Position, rng: &mut impl Rng) -> PlayerAttributes {
    use crate::models::player::PlayerAttributes;

    let mut attrs = PlayerAttributes::default();

    // Apply position-specific attribute patterns (OpenFootball 36-field)
    match position {
        Position::FW => {
            // Technical
            attrs.finishing = rng.gen_range(60..85);
            attrs.long_shots = rng.gen_range(55..80);
            attrs.dribbling = rng.gen_range(50..75);
            attrs.first_touch = rng.gen_range(50..70);
            attrs.technique = rng.gen_range(50..70);
            attrs.passing = rng.gen_range(40..65);
            attrs.heading = rng.gen_range(45..70);

            // Mental
            attrs.composure = rng.gen_range(45..70);
            attrs.off_the_ball = rng.gen_range(60..85);
            attrs.positioning = rng.gen_range(55..75);
            attrs.anticipation = rng.gen_range(50..70);
            attrs.decisions = rng.gen_range(45..65);

            // Physical
            attrs.pace = rng.gen_range(65..90);
            attrs.acceleration = rng.gen_range(65..85);
            attrs.agility = rng.gen_range(55..80);

            // Lower defensive stats
            attrs.tackling = rng.gen_range(15..35);
            attrs.marking = rng.gen_range(15..30);
        }
        Position::MF => {
            // Technical
            attrs.passing = rng.gen_range(60..85);
            attrs.technique = rng.gen_range(55..75);
            attrs.first_touch = rng.gen_range(50..75);
            attrs.crossing = rng.gen_range(45..70);
            attrs.dribbling = rng.gen_range(45..70);
            attrs.finishing = rng.gen_range(35..60);
            attrs.long_shots = rng.gen_range(40..65);

            // Mental
            attrs.vision = rng.gen_range(55..80);
            attrs.teamwork = rng.gen_range(60..80);
            attrs.work_rate = rng.gen_range(55..80);
            attrs.positioning = rng.gen_range(50..70);
            attrs.anticipation = rng.gen_range(50..70);
            attrs.decisions = rng.gen_range(50..70);

            // Physical
            attrs.pace = rng.gen_range(50..75);
            attrs.acceleration = rng.gen_range(50..75);
            attrs.stamina = rng.gen_range(60..85);
        }
        Position::DF => {
            // Technical
            attrs.tackling = rng.gen_range(60..85);
            attrs.marking = rng.gen_range(60..85);
            attrs.heading = rng.gen_range(55..80);
            attrs.passing = rng.gen_range(40..65);
            attrs.first_touch = rng.gen_range(35..60);

            // Mental
            attrs.positioning = rng.gen_range(60..85);
            attrs.anticipation = rng.gen_range(55..75);
            attrs.concentration = rng.gen_range(55..75);
            attrs.bravery = rng.gen_range(60..80);
            attrs.decisions = rng.gen_range(50..70);

            // Physical
            attrs.strength = rng.gen_range(60..85);
            attrs.jumping = rng.gen_range(50..75);
            attrs.pace = rng.gen_range(45..70);
            attrs.acceleration = rng.gen_range(45..70);

            // Lower attacking stats
            attrs.finishing = rng.gen_range(10..30);
            attrs.long_shots = rng.gen_range(10..30);
            attrs.dribbling = rng.gen_range(25..45);
        }
        Position::GK => {
            // GK using OpenFootball 36-field (no GK-specific attributes)
            // Technical (adapted for GK)
            attrs.first_touch = rng.gen_range(70..90); // Handling equivalent
            attrs.long_throws = rng.gen_range(60..85);
            attrs.passing = rng.gen_range(40..70);
            attrs.technique = rng.gen_range(50..75);

            // Mental
            attrs.positioning = rng.gen_range(65..85);
            attrs.anticipation = rng.gen_range(60..80);
            attrs.concentration = rng.gen_range(60..80);
            attrs.composure = rng.gen_range(55..80);
            attrs.decisions = rng.gen_range(50..75);
            attrs.leadership = rng.gen_range(50..75);
            attrs.bravery = rng.gen_range(55..80);

            // Physical (adapted for GK)
            attrs.agility = rng.gen_range(65..85); // Reflexes equivalent
            attrs.jumping = rng.gen_range(60..85); // Aerial ability
            attrs.strength = rng.gen_range(55..80);
            attrs.balance = rng.gen_range(55..75);

            // Lower outfield stats
            attrs.pace = rng.gen_range(20..40);
            attrs.acceleration = rng.gen_range(20..40);
            attrs.finishing = rng.gen_range(5..20);
            attrs.dribbling = rng.gen_range(10..30);
            attrs.tackling = rng.gen_range(10..25);
        }
        // Specific defensive positions
        Position::LB | Position::CB | Position::RB | Position::LWB | Position::RWB => {
            // Technical
            attrs.tackling = rng.gen_range(60..85);
            attrs.marking = rng.gen_range(60..85);
            attrs.heading = rng.gen_range(55..80);
            attrs.passing = rng.gen_range(45..70);
            attrs.crossing = if matches!(
                position,
                Position::LB | Position::RB | Position::LWB | Position::RWB
            ) {
                rng.gen_range(45..70)
            } else {
                rng.gen_range(30..50)
            };

            // Mental
            attrs.positioning = rng.gen_range(60..85);
            attrs.anticipation = rng.gen_range(55..75);
            attrs.concentration = rng.gen_range(55..75);
            attrs.bravery = rng.gen_range(60..80);
            attrs.work_rate = rng.gen_range(55..80);

            // Physical
            attrs.strength = rng.gen_range(55..80);
            attrs.jumping = rng.gen_range(50..75);
            attrs.pace = if matches!(
                position,
                Position::LB | Position::RB | Position::LWB | Position::RWB
            ) {
                rng.gen_range(55..80)
            } else {
                rng.gen_range(45..70)
            };
            attrs.stamina = if matches!(position, Position::LWB | Position::RWB) {
                rng.gen_range(65..85)
            } else {
                rng.gen_range(50..75)
            };

            // Lower attacking stats
            attrs.finishing = rng.gen_range(10..30);
            attrs.long_shots = rng.gen_range(10..35);
        }
        // Midfielder positions
        Position::CDM | Position::CM | Position::CAM | Position::LM | Position::RM => {
            // Technical
            attrs.passing = rng.gen_range(60..85);
            attrs.vision = rng.gen_range(55..80);
            attrs.technique = rng.gen_range(55..75);
            attrs.first_touch = rng.gen_range(50..75);
            attrs.crossing = if matches!(position, Position::LM | Position::RM) {
                rng.gen_range(55..80)
            } else {
                rng.gen_range(45..70)
            };
            attrs.finishing = if matches!(position, Position::CAM) {
                rng.gen_range(50..75)
            } else {
                rng.gen_range(35..60)
            };
            attrs.long_shots = if matches!(position, Position::CAM | Position::CM) {
                rng.gen_range(50..75)
            } else {
                rng.gen_range(35..60)
            };
            attrs.tackling = if matches!(position, Position::CDM) {
                rng.gen_range(55..80)
            } else {
                rng.gen_range(40..65)
            };

            // Mental
            attrs.teamwork = rng.gen_range(60..80);
            attrs.work_rate = rng.gen_range(55..80);
            attrs.positioning = rng.gen_range(50..70);
            attrs.anticipation = rng.gen_range(50..70);
            attrs.decisions = rng.gen_range(50..70);

            // Physical
            attrs.pace = rng.gen_range(50..75);
            attrs.acceleration = rng.gen_range(50..75);
            attrs.stamina = rng.gen_range(60..85);
        }
        // Wing positions
        Position::LW | Position::RW => {
            // Technical
            attrs.dribbling = rng.gen_range(60..85);
            attrs.crossing = rng.gen_range(60..80);
            attrs.technique = rng.gen_range(55..75);
            attrs.first_touch = rng.gen_range(55..75);
            attrs.finishing = rng.gen_range(45..70);
            attrs.long_shots = rng.gen_range(40..65);
            attrs.passing = rng.gen_range(50..70);

            // Mental
            attrs.off_the_ball = rng.gen_range(55..75);
            attrs.flair = rng.gen_range(55..80);
            attrs.decisions = rng.gen_range(45..65);
            attrs.composure = rng.gen_range(45..65);

            // Physical
            attrs.pace = rng.gen_range(65..90);
            attrs.acceleration = rng.gen_range(60..85);
            attrs.agility = rng.gen_range(60..85);
            attrs.balance = rng.gen_range(55..75);
        }
        // Forward positions
        Position::CF | Position::ST => {
            // Technical
            attrs.finishing = rng.gen_range(60..85);
            attrs.long_shots = rng.gen_range(55..80);
            attrs.dribbling = rng.gen_range(50..75);
            attrs.first_touch = rng.gen_range(50..70);
            attrs.technique = rng.gen_range(50..70);
            attrs.heading = if matches!(position, Position::ST) {
                rng.gen_range(55..80)
            } else {
                rng.gen_range(40..65)
            };

            // Mental
            attrs.composure = rng.gen_range(50..75);
            attrs.off_the_ball = rng.gen_range(60..85);
            attrs.positioning = rng.gen_range(55..75);
            attrs.anticipation = rng.gen_range(50..70);
            attrs.decisions = rng.gen_range(45..65);

            // Physical
            attrs.pace = rng.gen_range(65..90);
            attrs.acceleration = rng.gen_range(65..85);
            attrs.agility = rng.gen_range(55..80);
            attrs.strength = if matches!(position, Position::ST) {
                rng.gen_range(55..80)
            } else {
                rng.gen_range(45..70)
            };

            // Lower defensive stats
            attrs.tackling = rng.gen_range(15..35);
            attrs.marking = rng.gen_range(15..30);
        }
    }

    // Fill in remaining attributes with moderate values (OpenFootball 36-field complete)
    if attrs.corners == 0 {
        attrs.corners = rng.gen_range(30..60);
    }
    if attrs.free_kicks == 0 {
        attrs.free_kicks = rng.gen_range(30..60);
    }
    if attrs.penalty_taking == 0 {
        attrs.penalty_taking = rng.gen_range(40..70);
    }
    if attrs.aggression == 0 {
        attrs.aggression = rng.gen_range(30..70);
    }
    if attrs.stamina == 0 {
        attrs.stamina = rng.gen_range(50..75);
    }
    if attrs.natural_fitness == 0 {
        attrs.natural_fitness = rng.gen_range(45..70);
    }
    if attrs.concentration == 0 {
        attrs.concentration = rng.gen_range(45..70);
    }
    if attrs.decisions == 0 {
        attrs.decisions = rng.gen_range(40..65);
    }
    if attrs.determination == 0 {
        attrs.determination = rng.gen_range(50..80);
    }
    if attrs.flair == 0 {
        attrs.flair = rng.gen_range(30..60);
    }
    if attrs.leadership == 0 {
        attrs.leadership = rng.gen_range(35..65);
    }
    if attrs.off_the_ball == 0 {
        attrs.off_the_ball = rng.gen_range(40..65);
    }
    if attrs.vision == 0 {
        attrs.vision = rng.gen_range(30..60);
    }
    if attrs.teamwork == 0 {
        attrs.teamwork = rng.gen_range(45..70);
    }
    if attrs.work_rate == 0 {
        attrs.work_rate = rng.gen_range(50..75);
    }
    if attrs.balance == 0 {
        attrs.balance = rng.gen_range(45..70);
    }
    if attrs.agility == 0 {
        attrs.agility = rng.gen_range(45..70);
    }

    attrs
}

/// Generate a growth profile suitable for the position
fn generate_growth_profile(position: Position, rng: &mut impl Rng) -> GrowthProfile {
    use crate::player::TrainingResponse;

    let specialization = match position {
        // Forward positions
        Position::FW | Position::ST | Position::CF | Position::LW | Position::RW => {
            vec!["SHOOTING".to_string(), "PACE".to_string()]
        }
        // Midfield positions
        Position::MF
        | Position::CM
        | Position::CAM
        | Position::CDM
        | Position::LM
        | Position::RM => vec!["PASSING".to_string(), "TECHNICAL".to_string()],
        // Defense positions
        Position::DF
        | Position::CB
        | Position::LB
        | Position::RB
        | Position::LWB
        | Position::RWB => vec!["DEFENDING".to_string(), "POWER".to_string()],
        // Goalkeeper
        Position::GK => vec!["GOALKEEPING".to_string(), "DEFENDING".to_string()],
    };

    let training_response = TrainingResponse {
        technical_multiplier: rng.gen_range(0.8..1.3),
        physical_multiplier: rng.gen_range(0.8..1.3),
        mental_multiplier: rng.gen_range(0.8..1.3),
    };

    GrowthProfile {
        growth_rate: rng.gen_range(0.7..1.2),
        specialization,
        training_response,
        injury_prone: rng.gen_range(0.05..0.2),
    }
}

/// Create multiple players from JSON batch request
///
/// # Arguments
/// * `request_json` - JSON string containing BatchPlayerCreationRequest
///
/// # Returns
/// JSON string containing ApiResponse<BatchPlayerCreationResponse>
pub fn batch_create_players_json(request_json: &str) -> String {
    info!("Processing batch player creation request");

    // Parse the request
    let request: BatchPlayerCreationRequest = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse BatchPlayerCreationRequest: {}", e);
            let error = ApiError::new("INVALID_JSON", &format!("Invalid JSON format: {}", e));
            let response: ApiResponse<BatchPlayerCreationResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    // Validate batch size
    if request.players.len() > 100 {
        let error = ApiError::new("BATCH_SIZE_EXCEEDED", "Maximum batch size is 100 players");
        let response: ApiResponse<BatchPlayerCreationResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    if request.players.is_empty() {
        let error = ApiError::new("EMPTY_BATCH", "Batch request cannot be empty");
        let response: ApiResponse<BatchPlayerCreationResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    let mut created_players = Vec::new();
    let mut failed_requests = Vec::new();

    // Process each player creation request
    for (index, player_request) in request.players.iter().enumerate() {
        // Apply batch seed if provided
        let mut modified_request = player_request.clone();
        if let Some(batch_seed) = request.batch_seed {
            modified_request.seed = Some(batch_seed.wrapping_add(index as u64));
        }

        // Validate each request
        if let Err(error) = modified_request.validate() {
            failed_requests.push(BatchFailure { index, player_id: None, error });
            continue;
        }

        // Create the player
        match create_player_from_request(&modified_request) {
            Ok((player, _seed_used)) => {
                debug!("Created player in batch: {} (index: {})", player.name, index);
                created_players.push(player);
            }
            Err(error) => {
                warn!("Failed to create player at index {}: {}", index, error.message);
                failed_requests.push(BatchFailure { index, player_id: None, error });
            }
        }
    }

    let response_data = BatchPlayerCreationResponse {
        total_requested: request.players.len(),
        total_created: created_players.len(),
        total_failed: failed_requests.len(),
        created_players,
        failed_requests,
    };

    info!(
        "Batch creation completed: {}/{} players created",
        response_data.total_created, response_data.total_requested
    );

    let response = ApiResponse::success(response_data);
    serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
}

/// Update multiple players from JSON batch request
///
/// # Arguments
/// * `request_json` - JSON string containing BatchPlayerUpdateRequest
/// * `players` - Mutable reference to HashMap storing players by ID
///
/// # Returns
/// JSON string containing ApiResponse<BatchPlayerUpdateResponse>
pub fn batch_update_players_json(
    request_json: &str,
    players: &mut HashMap<String, CorePlayer>,
) -> String {
    info!("Processing batch player update request");

    // Parse the request
    let request: BatchPlayerUpdateRequest = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse BatchPlayerUpdateRequest: {}", e);
            let error = ApiError::new("INVALID_JSON", &format!("Invalid JSON format: {}", e));
            let response: ApiResponse<BatchPlayerUpdateResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    // Validate batch size
    if request.updates.len() > 100 {
        let error =
            ApiError::new("BATCH_SIZE_EXCEEDED", "Maximum batch size is 100 player updates");
        let response: ApiResponse<BatchPlayerUpdateResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    if request.updates.is_empty() {
        let error = ApiError::new("EMPTY_BATCH", "Batch request cannot be empty");
        let response: ApiResponse<BatchPlayerUpdateResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    let mut updated_players = Vec::new();
    let mut failed_updates = Vec::new();

    // Process each player update request
    for (index, update_request) in request.updates.iter().enumerate() {
        // Validate each request
        if let Err(error) = update_request.validate() {
            failed_updates.push(BatchFailure {
                index,
                player_id: Some(update_request.player_id.clone()),
                error,
            });
            continue;
        }

        // Find the player
        let player = match players.get_mut(&update_request.player_id) {
            Some(p) => p,
            None => {
                let error = ApiError::new(
                    "PLAYER_NOT_FOUND",
                    &format!("Player with ID {} not found", update_request.player_id),
                );
                failed_updates.push(BatchFailure {
                    index,
                    player_id: Some(update_request.player_id.clone()),
                    error,
                });
                continue;
            }
        };

        // Apply updates
        match apply_player_updates(player, update_request) {
            Ok(_update_response) => {
                debug!("Updated player in batch: {} (index: {})", player.name, index);
                updated_players.push(player.clone());
            }
            Err(error) => {
                warn!("Failed to update player at index {}: {}", index, error.message);
                failed_updates.push(BatchFailure {
                    index,
                    player_id: Some(update_request.player_id.clone()),
                    error,
                });
            }
        }
    }

    let response_data = BatchPlayerUpdateResponse {
        total_requested: request.updates.len(),
        total_updated: updated_players.len(),
        total_failed: failed_updates.len(),
        updated_players,
        failed_updates,
    };

    info!(
        "Batch update completed: {}/{} players updated",
        response_data.total_updated, response_data.total_requested
    );

    let response = ApiResponse::success(response_data);
    serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
}

/// Bulk export players to JSON/CSV
///
/// # Arguments
/// * `request_json` - JSON string containing BulkExportRequest
/// * `players` - Reference to HashMap storing players by ID
///
/// # Returns
/// JSON string containing ApiResponse<BulkExportResponse>
pub fn bulk_export_json(request_json: &str, players: &HashMap<String, CorePlayer>) -> String {
    debug!("Processing bulk export request");

    // Parse the request
    let request: BulkExportRequest = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse BulkExportRequest: {}", e);
            let error = ApiError::new("INVALID_JSON", &format!("Invalid JSON format: {}", e));
            let response: ApiResponse<BulkExportResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    // Validate player IDs and collect players
    let mut export_players = Vec::new();
    for player_id in &request.player_ids {
        if let Some(player) = players.get(player_id) {
            export_players.push(player.clone());
        } else {
            let error =
                ApiError::new("PLAYER_NOT_FOUND", &format!("Player {} not found", player_id));
            let response: ApiResponse<BulkExportResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    }

    // Generate export data based on format
    let data = match request.format {
        ExportFormat::Json => match serde_json::to_string_pretty(&export_players) {
            Ok(json) => json,
            Err(e) => {
                let error = ApiError::new("EXPORT_FAILED", &format!("JSON export failed: {}", e));
                let response: ApiResponse<BulkExportResponse> = ApiResponse::error(error);
                return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            }
        },
        ExportFormat::JsonCompact => match serde_json::to_string(&export_players) {
            Ok(json) => json,
            Err(e) => {
                let error =
                    ApiError::new("EXPORT_FAILED", &format!("JSON compact export failed: {}", e));
                let response: ApiResponse<BulkExportResponse> = ApiResponse::error(error);
                return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            }
        },
        ExportFormat::Csv => generate_csv_export(&export_players),
    };

    let response_data = BulkExportResponse {
        data,
        format: request.format,
        player_count: export_players.len(),
        export_timestamp: Utc::now(),
    };

    debug!(
        "Bulk export completed: {} players exported as {:?}",
        response_data.player_count, response_data.format
    );

    let response = ApiResponse::success(response_data);
    serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
}

/// Bulk import players from JSON/CSV
///
/// # Arguments
/// * `request_json` - JSON string containing BulkImportRequest
/// * `players` - Mutable reference to HashMap storing players by ID
///
/// # Returns
/// JSON string containing ApiResponse<BulkImportResponse>
pub fn bulk_import_json(request_json: &str, players: &mut HashMap<String, CorePlayer>) -> String {
    info!("Processing bulk import request");

    // Parse the request
    let request: BulkImportRequest = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse BulkImportRequest: {}", e);
            let error = ApiError::new("INVALID_JSON", &format!("Invalid JSON format: {}", e));
            let response: ApiResponse<BulkImportResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    let dry_run = request.validate_only.unwrap_or(false);
    let overwrite = request.overwrite_existing.unwrap_or(false);

    // Parse import data based on format
    let import_players: Vec<CorePlayer> = match request.format {
        ExportFormat::Json | ExportFormat::JsonCompact => {
            match serde_json::from_str(&request.data) {
                Ok(players_data) => players_data,
                Err(e) => {
                    let error = ApiError::new(
                        "IMPORT_PARSE_FAILED",
                        &format!("JSON parsing failed: {}", e),
                    );
                    let response: ApiResponse<BulkImportResponse> = ApiResponse::error(error);
                    return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
                }
            }
        }
        ExportFormat::Csv => {
            // CSV import would need more complex parsing - placeholder for now
            let error =
                ApiError::new("CSV_IMPORT_NOT_IMPLEMENTED", "CSV import is not yet implemented");
            let response: ApiResponse<BulkImportResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    let mut imported_players = Vec::new();
    let mut validation_errors = Vec::new();

    // Validate and process each player
    for (index, player) in import_players.iter().enumerate() {
        // Basic validation
        // PlayerValidator has static methods only

        // Validate player fields
        let mut errors = Vec::new();
        if let Err(e) = PlayerValidator::validate_name(&player.name) {
            errors.push(format!("name: {}", e));
        }
        if let Err(e) = PlayerValidator::validate_age(player.age_months) {
            errors.push(format!("age: {}", e));
        }
        if let Err(e) = PlayerValidator::validate_ca(player.ca) {
            errors.push(format!("ca: {}", e));
        }
        if let Err(e) = PlayerValidator::validate_pa(player.pa, Some(player.ca)) {
            errors.push(format!("pa: {}", e));
        }

        if !errors.is_empty() {
            validation_errors.push(BatchFailure {
                index,
                player_id: Some(player.id.clone()),
                error: ApiError::new("VALIDATION_FAILED", &errors.join(", ")),
            });
            continue;
        }

        // Check if player already exists
        if players.contains_key(&player.id) && !overwrite {
            validation_errors.push(BatchFailure {
                index,
                player_id: Some(player.id.clone()),
                error: ApiError::new(
                    "PLAYER_EXISTS",
                    "Player already exists and overwrite is disabled",
                ),
            });
            continue;
        }

        // If not dry run, actually import the player
        if !dry_run {
            players.insert(player.id.clone(), player.clone());
        }

        imported_players.push(player.clone());
    }

    let response_data = BulkImportResponse {
        total_processed: import_players.len(),
        total_imported: imported_players.len(),
        total_errors: validation_errors.len(),
        imported_players,
        validation_errors,
        dry_run,
    };

    info!(
        "Bulk import completed: {}/{} players imported (dry_run: {})",
        response_data.total_imported, response_data.total_processed, dry_run
    );

    let response = ApiResponse::success(response_data);
    serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
}

/// Generate CSV export data from players
fn generate_csv_export(players: &[CorePlayer]) -> String {
    let mut csv = String::new();

    // CSV Header
    csv.push_str("id,name,position,age_months,ca,pa,");
    csv.push_str("pace,power,technical,shooting,passing,defending,");
    csv.push_str("growth_rate,injury_prone,created_at\n");

    // CSV Data
    for player in players {
        csv.push_str(&format!(
            "{},{},{:?},{},{},{},",
            player.id, player.name, player.position, player.age_months, player.ca, player.pa
        ));
        csv.push_str(&format!(
            "{},{},{},{},{},{},",
            player.hexagon_stats.pace,
            player.hexagon_stats.power,
            player.hexagon_stats.technical,
            player.hexagon_stats.shooting,
            player.hexagon_stats.passing,
            player.hexagon_stats.defending
        ));
        csv.push_str(&format!(
            "{},{},{}\n",
            player.growth_profile.growth_rate,
            player.growth_profile.injury_prone,
            player.created_at.format("%Y-%m-%d %H:%M:%S")
        ));
    }

    csv
}

/// Simulate player growth over time from JSON request
///
/// # Arguments
/// * `request_json` - JSON string containing GrowthSimulationRequest
/// * `players` - Reference to HashMap storing players by ID
///
/// # Returns
/// JSON string containing ApiResponse<GrowthSimulationResponse>
pub fn simulate_growth_json(request_json: &str, players: &HashMap<String, CorePlayer>) -> String {
    info!("Processing growth simulation request");

    // Parse the request
    let request: GrowthSimulationRequest = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse GrowthSimulationRequest: {}", e);
            let error = ApiError::new("INVALID_JSON", &format!("Invalid JSON format: {}", e));
            let response: ApiResponse<GrowthSimulationResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    // Validate player ID and find player
    let player = match players.get(&request.player_id) {
        Some(p) => p.clone(),
        None => {
            let error = ApiError::new(
                "PLAYER_NOT_FOUND",
                &format!("Player with ID {} not found", request.player_id),
            );
            let response: ApiResponse<GrowthSimulationResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    // Validate simulation parameters
    if request.months == 0 || request.months > 60 {
        let error = ApiError::new(
            "INVALID_SIMULATION_LENGTH",
            "Simulation must be between 1 and 60 months",
        );
        let response: ApiResponse<GrowthSimulationResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    if request.training_schedule.is_empty() {
        let error = ApiError::new("EMPTY_TRAINING_SCHEDULE", "Training schedule cannot be empty");
        let response: ApiResponse<GrowthSimulationResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    // Validate training sessions
    for session in &request.training_schedule {
        if session.intensity < 0.1 || session.intensity > 2.0 {
            let error = ApiError::new(
                "INVALID_INTENSITY",
                "Training intensity must be between 0.1 and 2.0",
            );
            let response: ApiResponse<GrowthSimulationResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
        if session.sessions_per_month == 0 || session.sessions_per_month > 8 {
            let error = ApiError::new(
                "INVALID_SESSIONS_COUNT",
                "Sessions per month must be between 1 and 8",
            );
            let response: ApiResponse<GrowthSimulationResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    }

    // Run the simulation
    match simulate_player_growth(&player, &request) {
        Ok(response_data) => {
            info!("Growth simulation completed for player: {}", player.name);
            let response = ApiResponse::success(response_data);
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
        }
        Err(error) => {
            error!("Growth simulation failed: {}", error.message);
            let response: ApiResponse<GrowthSimulationResponse> = ApiResponse::error(error);
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

/// Perform the actual growth simulation
fn simulate_player_growth(
    initial_player: &CorePlayer,
    request: &GrowthSimulationRequest,
) -> Result<GrowthSimulationResponse, ApiError> {
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    // Set up RNG for variance
    let seed = request.seed.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
    });
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut player = initial_player.clone();
    let mut growth_history = Vec::new();
    let variance = request.random_variance.unwrap_or(0.05);
    let total_months = request.months;

    // Calculate total attribute changes for tracking
    let mut total_attribute_changes = HashMap::new();

    for month in 1..=total_months {
        let month_start_ca = player.ca;
        let _month_start_age = player.age_months;
        let mut monthly_growths = Vec::new();

        // Age the player by one month
        player.age_months += 1.0;

        // Calculate growth rate for current state
        let growth_rate = GrowthCalculator::calculate_growth_rate(player.ca, player.pa);
        let age_modifier = GrowthCalculator::age_modifier(player.age_months);
        let effective_growth_rate = growth_rate * age_modifier * player.growth_profile.growth_rate;

        // Apply each training session
        for training_session in &request.training_schedule {
            apply_training_session(
                &mut player,
                training_session,
                effective_growth_rate,
                variance,
                &mut rng,
                &mut monthly_growths,
                &mut total_attribute_changes,
            );
        }

        // Recalculate CA and hexagon stats after all growth
        player.recalculate_all();

        // Store monthly growth record
        growth_history.push(MonthlyGrowth {
            month,
            age_months: player.age_months,
            ca_start: month_start_ca,
            ca_end: player.ca,
            attribute_growths: monthly_growths,
            growth_rate: effective_growth_rate,
        });
    }

    let total_ca_growth = player.ca as i16 - initial_player.ca as i16;
    let average_monthly_growth =
        if total_months > 0 { total_ca_growth as f32 / total_months as f32 } else { 0.0 };

    Ok(GrowthSimulationResponse {
        initial_player: initial_player.clone(),
        final_player: player,
        growth_history,
        total_ca_growth,
        total_attribute_changes,
        average_monthly_growth,
    })
}

/// Apply a single training session to the player
fn apply_training_session(
    player: &mut CorePlayer,
    session: &TrainingSession,
    growth_rate: f32,
    variance: f32,
    rng: &mut impl Rng,
    monthly_growths: &mut Vec<AttributeGrowth>,
    total_changes: &mut HashMap<String, i8>,
) {
    let base_multiplier = session.intensity * growth_rate;
    let sessions_effect = session.sessions_per_month as f32 / 4.0; // Normalize to monthly effect

    // Apply position and training type specific effects
    let training_attributes = get_training_attributes(session.training_type, player.position);

    for (attr_name, weight) in training_attributes {
        // Calculate growth amount with variance
        let growth_potential = base_multiplier * sessions_effect * weight;
        let variance_factor = 1.0 + rng.gen_range(-variance..variance);
        let raw_growth = growth_potential * variance_factor;

        // Apply training response multipliers
        let multiplied_growth = match session.training_type {
            TrainingType::Technical | TrainingType::Shooting | TrainingType::Passing => {
                raw_growth * player.growth_profile.training_response.technical_multiplier
            }
            TrainingType::Physical => {
                raw_growth * player.growth_profile.training_response.physical_multiplier
            }
            TrainingType::Mental | TrainingType::Defending => {
                raw_growth * player.growth_profile.training_response.mental_multiplier
            }
            TrainingType::Goalkeeping => {
                raw_growth * player.growth_profile.training_response.technical_multiplier * 0.8
                // Slightly lower for GK
            }
            TrainingType::General => {
                raw_growth
                    * (player.growth_profile.training_response.technical_multiplier
                        + player.growth_profile.training_response.physical_multiplier
                        + player.growth_profile.training_response.mental_multiplier)
                    / 3.0
            }
        };

        // Convert to integer growth (round to nearest)
        let growth_amount = multiplied_growth.round() as i8;

        if growth_amount != 0 {
            // Apply diminishing returns as player approaches PA
            let ca_progress = player.ca as f32 / player.pa as f32;
            let diminishing_factor = if ca_progress > 0.8 {
                (1.0 - ca_progress).max(0.1) // Minimum 10% growth rate
            } else {
                1.0
            };

            let final_growth = ((growth_amount as f32) * diminishing_factor).round() as i8;

            if final_growth != 0 {
                // Apply the attribute change
                if let Ok(change_result) = player.modify_attribute(attr_name, final_growth) {
                    monthly_growths.push(AttributeGrowth {
                        attribute_name: attr_name.to_string(),
                        growth_amount: final_growth,
                        old_value: change_result.old_value,
                        new_value: change_result.new_value,
                    });

                    // Track total changes
                    *total_changes.entry(attr_name.to_string()).or_insert(0) += final_growth;
                }
            }
        }
    }
}

/// Get attributes affected by a training type for a given position
fn get_training_attributes(
    training_type: TrainingType,
    position: Position,
) -> Vec<(&'static str, f32)> {
    match training_type {
        TrainingType::Technical => vec![
            ("technique", 1.0),
            ("first_touch", 0.8),
            ("dribbling", 0.8),
            ("ball_control", 0.6),
            ("flair", 0.4),
        ],
        TrainingType::Physical => vec![
            ("pace", 0.8),
            ("acceleration", 0.8),
            ("stamina", 1.0),
            ("strength", 0.9),
            ("jumping", 0.7),
            ("natural_fitness", 0.6),
            ("agility", 0.7),
            ("balance", 0.5),
        ],
        TrainingType::Mental => vec![
            ("decisions", 1.0),
            ("concentration", 0.9),
            ("anticipation", 0.8),
            ("composure", 0.7),
            ("determination", 0.6),
            ("positioning", 0.8),
            ("vision", 0.6),
            ("teamwork", 0.5),
        ],
        TrainingType::Shooting => vec![
            ("shooting", 1.0),
            ("finishing", 1.0),
            ("long_shots", 0.8),
            ("penalty_taking", 0.6),
            ("composure", 0.4),
        ],
        TrainingType::Passing => vec![
            ("passing", 1.0),
            ("crossing", 0.8),
            ("vision", 0.8),
            ("technique", 0.5),
            ("creativity", 0.6),
            ("free_kicks", 0.4),
            ("corners", 0.4),
        ],
        TrainingType::Defending => vec![
            ("tackling", 1.0),
            ("marking", 1.0),
            ("positioning", 0.9),
            ("anticipation", 0.8),
            ("heading", 0.7),
            ("bravery", 0.6),
            ("aggression", 0.5),
            ("concentration", 0.6),
        ],
        TrainingType::Goalkeeping => match position {
            Position::GK => vec![
                ("handling", 1.0),
                ("reflexes", 1.0),
                ("one_on_ones", 0.9),
                ("aerial_ability", 0.8),
                ("command_of_area", 0.7),
                ("kicking", 0.6),
                ("throwing", 0.5),
                ("rushing_out", 0.6),
                ("communication", 0.5),
            ],
            _ => vec![], // Non-goalkeepers don't benefit from GK training
        },
        TrainingType::General => {
            // Position-specific general training
            match position {
                // Forward positions
                Position::FW | Position::ST | Position::CF | Position::LW | Position::RW => vec![
                    ("finishing", 0.8),
                    ("long_shots", 0.7),
                    ("pace", 0.6),
                    ("dribbling", 0.6),
                    ("technique", 0.5),
                    ("composure", 0.4),
                ],
                // Midfield positions
                Position::MF
                | Position::CM
                | Position::CAM
                | Position::CDM
                | Position::LM
                | Position::RM => vec![
                    ("passing", 0.8),
                    ("vision", 0.7),
                    ("technique", 0.6),
                    ("stamina", 0.6),
                    ("teamwork", 0.5),
                    ("flair", 0.5),
                ],
                // Defense positions
                Position::DF
                | Position::CB
                | Position::LB
                | Position::RB
                | Position::LWB
                | Position::RWB => vec![
                    ("tackling", 0.8),
                    ("marking", 0.8),
                    ("positioning", 0.7),
                    ("heading", 0.6),
                    ("strength", 0.6),
                    ("anticipation", 0.5),
                ],
                // Goalkeeper
                Position::GK => vec![
                    ("long_throws", 0.9),
                    ("positioning", 0.7),
                    ("heading", 0.6),
                    ("stamina", 0.5),
                ],
            }
        }
    }
}

/// Compare multiple players from JSON request
///
/// # Arguments
/// * `request_json` - JSON string containing PlayerComparisonRequest
/// * `players` - Reference to HashMap storing players by ID
///
/// # Returns
/// JSON string containing ApiResponse<PlayerComparisonResponse>
pub fn compare_players_json(request_json: &str, players: &HashMap<String, CorePlayer>) -> String {
    debug!("Processing player comparison request");

    // Parse the request
    let request: PlayerComparisonRequest = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse PlayerComparisonRequest: {}", e);
            let error = ApiError::new("INVALID_JSON", &format!("Invalid JSON format: {}", e));
            let response: ApiResponse<PlayerComparisonResponse> = ApiResponse::error(error);
            return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        }
    };

    // Validate request
    if request.player_ids.len() < 2 {
        let error =
            ApiError::new("INSUFFICIENT_PLAYERS", "At least 2 players are required for comparison");
        let response: ApiResponse<PlayerComparisonResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    if request.player_ids.len() > 20 {
        let error = ApiError::new("TOO_MANY_PLAYERS", "Maximum 20 players can be compared at once");
        let response: ApiResponse<PlayerComparisonResponse> = ApiResponse::error(error);
        return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    }

    // Collect all players
    let mut comparison_players = Vec::new();
    for player_id in &request.player_ids {
        match players.get(player_id) {
            Some(player) => comparison_players.push(player.clone()),
            None => {
                let error = ApiError::new(
                    "PLAYER_NOT_FOUND",
                    &format!("Player with ID {} not found", player_id),
                );
                let response: ApiResponse<PlayerComparisonResponse> = ApiResponse::error(error);
                return serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            }
        }
    }

    // Perform comparison
    match perform_player_comparison(&comparison_players, &request) {
        Ok(response_data) => {
            debug!("Player comparison completed for {} players", comparison_players.len());
            let response = ApiResponse::success(response_data);
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
        }
        Err(error) => {
            error!("Player comparison failed: {}", error.message);
            let response: ApiResponse<PlayerComparisonResponse> = ApiResponse::error(error);
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

/// Perform the actual player comparison
fn perform_player_comparison(
    players: &[CorePlayer],
    request: &PlayerComparisonRequest,
) -> Result<PlayerComparisonResponse, ApiError> {
    let player_count = players.len();

    // Create similarity matrix if requested
    let comparison_matrix = if request.include_similarity_scores.unwrap_or(false) {
        calculate_similarity_matrix(players, &request.comparison_type)
    } else {
        vec![vec![0.0; player_count]; player_count]
    };

    // Generate rankings based on comparison type
    let rankings = generate_rankings(players, &request.comparison_type);

    // Generate summary statistics
    let summary_stats = generate_comparison_summary(players);

    Ok(PlayerComparisonResponse {
        players: players.to_vec(),
        comparison_matrix,
        rankings,
        summary_stats,
    })
}

/// Calculate similarity matrix between players
fn calculate_similarity_matrix(
    players: &[CorePlayer],
    comparison_type: &ComparisonType,
) -> Vec<Vec<f32>> {
    let player_count = players.len();
    let mut matrix = vec![vec![0.0; player_count]; player_count];

    for i in 0..player_count {
        for j in 0..player_count {
            if i == j {
                matrix[i][j] = 1.0; // Perfect similarity to self
            } else {
                matrix[i][j] =
                    calculate_player_similarity(&players[i], &players[j], comparison_type);
            }
        }
    }

    matrix
}

/// Calculate similarity between two players based on comparison type
fn calculate_player_similarity(
    player1: &CorePlayer,
    player2: &CorePlayer,
    comparison_type: &ComparisonType,
) -> f32 {
    match comparison_type {
        ComparisonType::Overall => {
            // Overall similarity based on CA, PA, hexagon stats, and age
            let ca_similarity = 1.0 - ((player1.ca as f32 - player2.ca as f32).abs() / 200.0);
            let pa_similarity = 1.0 - ((player1.pa as f32 - player2.pa as f32).abs() / 180.0);
            let age_similarity =
                1.0 - ((player1.age_months - player2.age_months).abs() / 36.0).min(1.0);
            let hexagon_similarity =
                calculate_hexagon_similarity(&player1.hexagon_stats, &player2.hexagon_stats);

            ca_similarity * 0.3
                + pa_similarity * 0.3
                + hexagon_similarity * 0.3
                + age_similarity * 0.1
        }
        ComparisonType::PositionSpecific => {
            // Position-specific similarity
            if player1.position == player2.position {
                calculate_position_specific_similarity(player1, player2)
            } else {
                // Cross-position similarity is lower but not zero
                calculate_player_similarity(player1, player2, &ComparisonType::Overall) * 0.7
            }
        }
        ComparisonType::Attributes => {
            // Detailed attribute similarity
            calculate_attribute_similarity(&player1.detailed_stats, &player2.detailed_stats)
        }
        ComparisonType::GrowthPotential => {
            // Growth potential similarity
            let pa_ca_gap1 = player1.pa as f32 - player1.ca as f32;
            let pa_ca_gap2 = player2.pa as f32 - player2.ca as f32;
            let gap_similarity = 1.0 - ((pa_ca_gap1 - pa_ca_gap2).abs() / 100.0).min(1.0);

            let growth_rate_similarity = 1.0
                - ((player1.growth_profile.growth_rate - player2.growth_profile.growth_rate).abs()
                    / 2.0)
                    .min(1.0);
            let age_similarity =
                1.0 - ((player1.age_months - player2.age_months).abs() / 36.0).min(1.0);

            gap_similarity * 0.5 + growth_rate_similarity * 0.3 + age_similarity * 0.2
        }
    }
}

/// Calculate hexagon stats similarity
fn calculate_hexagon_similarity(hex1: &HexagonStats, hex2: &HexagonStats) -> f32 {
    let pace_sim = 1.0 - ((hex1.pace as f32 - hex2.pace as f32).abs() / 20.0);
    let power_sim = 1.0 - ((hex1.power as f32 - hex2.power as f32).abs() / 20.0);
    let technical_sim = 1.0 - ((hex1.technical as f32 - hex2.technical as f32).abs() / 20.0);
    let shooting_sim = 1.0 - ((hex1.shooting as f32 - hex2.shooting as f32).abs() / 20.0);
    let passing_sim = 1.0 - ((hex1.passing as f32 - hex2.passing as f32).abs() / 20.0);
    let defending_sim = 1.0 - ((hex1.defending as f32 - hex2.defending as f32).abs() / 20.0);

    (pace_sim + power_sim + technical_sim + shooting_sim + passing_sim + defending_sim) / 6.0
}

/// Calculate position-specific similarity
fn calculate_position_specific_similarity(player1: &CorePlayer, player2: &CorePlayer) -> f32 {
    use crate::player::PositionWeights;

    // Get position weights for comparison emphasis
    let weights = PositionWeights::get_for_position(player1.position);
    let hex1 = &player1.hexagon_stats;
    let hex2 = &player2.hexagon_stats;

    // Calculate weighted similarity
    let pace_weight = weights.pace_weight;
    let power_weight = weights.power_weight;
    let technical_weight = weights.technical_weight;
    let shooting_weight = weights.shooting_weight;
    let passing_weight = weights.passing_weight;
    let defending_weight = weights.defending_weight;

    let pace_sim = 1.0 - ((hex1.pace as f32 - hex2.pace as f32).abs() / 20.0);
    let power_sim = 1.0 - ((hex1.power as f32 - hex2.power as f32).abs() / 20.0);
    let technical_sim = 1.0 - ((hex1.technical as f32 - hex2.technical as f32).abs() / 20.0);
    let shooting_sim = 1.0 - ((hex1.shooting as f32 - hex2.shooting as f32).abs() / 20.0);
    let passing_sim = 1.0 - ((hex1.passing as f32 - hex2.passing as f32).abs() / 20.0);
    let defending_sim = 1.0 - ((hex1.defending as f32 - hex2.defending as f32).abs() / 20.0);

    let total_weight = pace_weight
        + power_weight
        + technical_weight
        + shooting_weight
        + passing_weight
        + defending_weight;

    (pace_sim * pace_weight
        + power_sim * power_weight
        + technical_sim * technical_weight
        + shooting_sim * shooting_weight
        + passing_sim * passing_weight
        + defending_sim * defending_weight)
        / total_weight
}

/// Calculate detailed attribute similarity
fn calculate_attribute_similarity(attrs1: &PlayerAttributes, attrs2: &PlayerAttributes) -> f32 {
    // OpenFootball 36-field attribute comparison
    let mut total_similarity = 0.0;
    let mut count = 0;

    // Compare key OpenFootball attributes
    let attributes = [
        // Physical
        (attrs1.pace, attrs2.pace),
        (attrs1.acceleration, attrs2.acceleration),
        (attrs1.agility, attrs2.agility),
        (attrs1.stamina, attrs2.stamina),
        (attrs1.strength, attrs2.strength),
        (attrs1.balance, attrs2.balance),
        (attrs1.jumping, attrs2.jumping),
        (attrs1.natural_fitness, attrs2.natural_fitness),
        // Technical
        (attrs1.finishing, attrs2.finishing),
        (attrs1.long_shots, attrs2.long_shots),
        (attrs1.passing, attrs2.passing),
        (attrs1.dribbling, attrs2.dribbling),
        (attrs1.first_touch, attrs2.first_touch),
        (attrs1.technique, attrs2.technique),
        (attrs1.crossing, attrs2.crossing),
        (attrs1.heading, attrs2.heading),
        (attrs1.tackling, attrs2.tackling),
        (attrs1.marking, attrs2.marking),
        // Mental
        (attrs1.aggression, attrs2.aggression),
        (attrs1.anticipation, attrs2.anticipation),
        (attrs1.composure, attrs2.composure),
        (attrs1.concentration, attrs2.concentration),
        (attrs1.decisions, attrs2.decisions),
        (attrs1.determination, attrs2.determination),
        (attrs1.vision, attrs2.vision),
        (attrs1.positioning, attrs2.positioning),
        (attrs1.teamwork, attrs2.teamwork),
        (attrs1.work_rate, attrs2.work_rate),
    ];

    for (attr1, attr2) in attributes.iter() {
        total_similarity += 1.0 - ((*attr1 as f32 - *attr2 as f32).abs() / 100.0);
        count += 1;
    }

    total_similarity / count as f32
}

/// Generate rankings based on comparison type
fn generate_rankings(
    players: &[CorePlayer],
    comparison_type: &ComparisonType,
) -> HashMap<String, Vec<PlayerRanking>> {
    let mut rankings = HashMap::new();

    // Generate different ranking categories based on comparison type
    match comparison_type {
        ComparisonType::Overall => {
            rankings.insert("ca_ranking".to_string(), rank_by_ca(players));
            rankings.insert("pa_ranking".to_string(), rank_by_pa(players));
            rankings.insert("overall_rating".to_string(), rank_by_overall(players));
        }
        ComparisonType::PositionSpecific => {
            rankings.insert("ca_ranking".to_string(), rank_by_ca(players));
            rankings.insert(
                "position_effectiveness".to_string(),
                rank_by_position_effectiveness(players),
            );
        }
        ComparisonType::Attributes => {
            rankings.insert("pace_ranking".to_string(), rank_by_hexagon_stat(players, |h| h.pace));
            rankings
                .insert("power_ranking".to_string(), rank_by_hexagon_stat(players, |h| h.power));
            rankings.insert(
                "technical_ranking".to_string(),
                rank_by_hexagon_stat(players, |h| h.technical),
            );
            rankings.insert(
                "shooting_ranking".to_string(),
                rank_by_hexagon_stat(players, |h| h.shooting),
            );
            rankings.insert(
                "passing_ranking".to_string(),
                rank_by_hexagon_stat(players, |h| h.passing),
            );
            rankings.insert(
                "defending_ranking".to_string(),
                rank_by_hexagon_stat(players, |h| h.defending),
            );
        }
        ComparisonType::GrowthPotential => {
            rankings.insert("growth_potential".to_string(), rank_by_growth_potential(players));
            rankings.insert("pa_ca_gap".to_string(), rank_by_pa_ca_gap(players));
        }
    }

    rankings
}

/// Rank players by CA
fn rank_by_ca(players: &[CorePlayer]) -> Vec<PlayerRanking> {
    let mut player_scores: Vec<_> = players.iter().map(|p| (p.id.clone(), p.ca as f32)).collect();

    player_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    create_rankings_from_scores(player_scores)
}

/// Rank players by PA
fn rank_by_pa(players: &[CorePlayer]) -> Vec<PlayerRanking> {
    let mut player_scores: Vec<_> = players.iter().map(|p| (p.id.clone(), p.pa as f32)).collect();

    player_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    create_rankings_from_scores(player_scores)
}

/// Rank players by overall rating (CA + PA + hexagon total)
fn rank_by_overall(players: &[CorePlayer]) -> Vec<PlayerRanking> {
    let mut player_scores: Vec<_> = players
        .iter()
        .map(|p| {
            let hexagon_total = p.hexagon_stats.pace as f32
                + p.hexagon_stats.power as f32
                + p.hexagon_stats.technical as f32
                + p.hexagon_stats.shooting as f32
                + p.hexagon_stats.passing as f32
                + p.hexagon_stats.defending as f32;
            let overall = p.ca as f32 * 0.4 + p.pa as f32 * 0.3 + hexagon_total * 0.3;
            (p.id.clone(), overall)
        })
        .collect();

    player_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    create_rankings_from_scores(player_scores)
}

/// Rank players by position effectiveness
fn rank_by_position_effectiveness(players: &[CorePlayer]) -> Vec<PlayerRanking> {
    let mut player_scores: Vec<_> = players
        .iter()
        .map(|p| {
            // Calculate position-specific effectiveness based on CA and relevant hexagon stats
            let score = match p.position {
                // Forward positions
                Position::FW | Position::ST | Position::CF | Position::LW | Position::RW => {
                    p.ca as f32 * 0.6
                        + (p.hexagon_stats.shooting + p.hexagon_stats.pace) as f32 * 0.2
                }
                // Midfield positions
                Position::MF
                | Position::CM
                | Position::CAM
                | Position::CDM
                | Position::LM
                | Position::RM => {
                    p.ca as f32 * 0.6
                        + (p.hexagon_stats.passing + p.hexagon_stats.technical) as f32 * 0.2
                }
                // Defense positions
                Position::DF
                | Position::CB
                | Position::LB
                | Position::RB
                | Position::LWB
                | Position::RWB => {
                    p.ca as f32 * 0.6
                        + (p.hexagon_stats.defending + p.hexagon_stats.power) as f32 * 0.2
                }
                // Goalkeeper
                Position::GK => p.ca as f32 * 0.8 + p.hexagon_stats.defending as f32 * 0.2,
            };
            (p.id.clone(), score)
        })
        .collect();

    player_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    create_rankings_from_scores(player_scores)
}

/// Rank players by a specific hexagon stat
fn rank_by_hexagon_stat(
    players: &[CorePlayer],
    stat_getter: impl Fn(&HexagonStats) -> u8,
) -> Vec<PlayerRanking> {
    let mut player_scores: Vec<_> =
        players.iter().map(|p| (p.id.clone(), stat_getter(&p.hexagon_stats) as f32)).collect();

    player_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    create_rankings_from_scores(player_scores)
}

/// Rank players by growth potential
fn rank_by_growth_potential(players: &[CorePlayer]) -> Vec<PlayerRanking> {
    let mut player_scores: Vec<_> = players
        .iter()
        .map(|p| {
            let pa_ca_gap = p.pa as f32 - p.ca as f32;
            let growth_rate = GrowthCalculator::calculate_growth_rate(p.ca, p.pa);
            let age_modifier = GrowthCalculator::age_modifier(p.age_months);
            let potential_score =
                pa_ca_gap * growth_rate * age_modifier * p.growth_profile.growth_rate;
            (p.id.clone(), potential_score)
        })
        .collect();

    player_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    create_rankings_from_scores(player_scores)
}

/// Rank players by PA-CA gap
fn rank_by_pa_ca_gap(players: &[CorePlayer]) -> Vec<PlayerRanking> {
    let mut player_scores: Vec<_> =
        players.iter().map(|p| (p.id.clone(), (p.pa - p.ca) as f32)).collect();

    player_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    create_rankings_from_scores(player_scores)
}

/// Create ranking structs from sorted player scores
fn create_rankings_from_scores(sorted_scores: Vec<(String, f32)>) -> Vec<PlayerRanking> {
    let total_players = sorted_scores.len() as f32;

    sorted_scores
        .into_iter()
        .enumerate()
        .map(|(index, (player_id, score))| {
            let rank = (index + 1) as u8;
            let percentile = ((total_players - index as f32) / total_players) * 100.0;

            PlayerRanking { player_id, rank, score, percentile }
        })
        .collect()
}

/// Generate comparison summary statistics
fn generate_comparison_summary(players: &[CorePlayer]) -> ComparisonSummary {
    let ca_values: Vec<u8> = players.iter().map(|p| p.ca).collect();
    let pa_values: Vec<u8> = players.iter().map(|p| p.pa).collect();
    let ages: Vec<f32> = players.iter().map(|p| p.age_months).collect();

    let average_ca = ca_values.iter().map(|&x| x as f32).sum::<f32>() / ca_values.len() as f32;
    let average_pa = pa_values.iter().map(|&x| x as f32).sum::<f32>() / pa_values.len() as f32;

    let ca_range = (*ca_values.iter().min().unwrap(), *ca_values.iter().max().unwrap());
    let pa_range = (*pa_values.iter().min().unwrap(), *pa_values.iter().max().unwrap());
    let age_range = (
        ages.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
        ages.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)),
    );

    let mut position_distribution = HashMap::new();
    for player in players {
        *position_distribution.entry(player.position).or_insert(0) += 1;
    }

    ComparisonSummary {
        average_ca,
        ca_range,
        average_pa,
        pa_range,
        position_distribution,
        age_range,
    }
}

// 🌟 Special Ability API Functions

/// Special ability management request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialAbilityRequest {
    pub schema_version: Option<String>,
    pub player_id: String,
    pub action: SpecialAbilityAction,
}

/// Special ability actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecialAbilityAction {
    AddAbility {
        ability_type: SpecialAbilityType,
        tier: AbilityTier,
    },
    ProcessAbilities {
        match_minute: u32,
        score_difference: i32,
        pressure_level: f32,
        fatigue_level: f32,
        is_crucial_moment: bool,
        team_morale: f32,
    },
    ListAbilities,
    GetAbilityEffects,
}

/// Special ability response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialAbilityResponse {
    pub updated_player: Option<CorePlayer>,
    pub abilities: Vec<SpecialAbility>,
    pub processing_result: Option<ProcessingResult>,
    pub ability_effects: Option<crate::special_ability::SkillEffects>,
    pub messages: Vec<String>,
}

impl SpecialAbilityRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if self.player_id.is_empty() {
            return Err(ApiError::new("INVALID_PLAYER_ID", "Player ID cannot be empty"));
        }

        // Add validation for specific actions if needed
        if let SpecialAbilityAction::ProcessAbilities {
            match_minute,
            pressure_level,
            fatigue_level,
            team_morale,
            ..
        } = &self.action
        {
            if *match_minute > 120 {
                return Err(ApiError::new(
                    "INVALID_MATCH_MINUTE",
                    "Match minute cannot exceed 120",
                ));
            }
            if *pressure_level < 0.0 || *pressure_level > 1.0 {
                return Err(ApiError::new(
                    "INVALID_PRESSURE_LEVEL",
                    "Pressure level must be between 0.0 and 1.0",
                ));
            }
            if *fatigue_level < 0.0 || *fatigue_level > 1.0 {
                return Err(ApiError::new(
                    "INVALID_FATIGUE_LEVEL",
                    "Fatigue level must be between 0.0 and 1.0",
                ));
            }
            if *team_morale < 0.0 || *team_morale > 1.0 {
                return Err(ApiError::new(
                    "INVALID_TEAM_MORALE",
                    "Team morale must be between 0.0 and 1.0",
                ));
            }
        }

        Ok(())
    }
}

/// Manage player special abilities (JSON API)
/// Supports adding abilities, processing combinations, and querying effects
pub fn manage_special_abilities_json(
    request_json: &str,
    players: &mut HashMap<String, CorePlayer>,
) -> String {
    let request: SpecialAbilityRequest = match serde_json::from_str(request_json) {
        Ok(r) => r,
        Err(e) => {
            let error =
                ApiError::new("INVALID_JSON", &format!("Failed to parse request JSON: {}", e));
            return serde_json::to_string(&ApiResponse::<SpecialAbilityResponse>::error(error))
                .unwrap();
        }
    };

    if let Err(e) = request.validate() {
        return serde_json::to_string(&ApiResponse::<SpecialAbilityResponse>::error(e)).unwrap();
    }

    let player = match players.get_mut(&request.player_id) {
        Some(p) => p,
        None => {
            let error = ApiError::new(
                "PLAYER_NOT_FOUND",
                &format!("Player with ID '{}' not found", request.player_id),
            );
            return serde_json::to_string(&ApiResponse::<SpecialAbilityResponse>::error(error))
                .unwrap();
        }
    };

    let mut response_data = SpecialAbilityResponse {
        updated_player: None,
        abilities: vec![],
        processing_result: None,
        ability_effects: None,
        messages: vec![],
    };

    match request.action {
        SpecialAbilityAction::AddAbility { ability_type, tier } => {
            // Check if player already has this exact ability
            if player.has_exact_special_ability(ability_type, tier) {
                let error = ApiError::new(
                    "ABILITY_ALREADY_EXISTS",
                    &format!("Player already has {} at {:?} tier", ability_type.name(), tier),
                );
                return serde_json::to_string(&ApiResponse::<SpecialAbilityResponse>::error(error))
                    .unwrap();
            }

            player.add_special_ability(ability_type, tier);
            response_data.updated_player = Some(player.clone());
            response_data.abilities = player.get_special_abilities().clone();
            response_data.messages.push(format!(
                "Added {} ({:?}) to player",
                ability_type.name(),
                tier
            ));
        }

        SpecialAbilityAction::ProcessAbilities {
            match_minute,
            score_difference,
            pressure_level,
            fatigue_level,
            is_crucial_moment,
            team_morale,
        } => {
            let activation_context = AbilityActivationContext {
                match_minute,
                score_difference,
                pressure_level,
                fatigue_level,
                is_crucial_moment,
                team_morale,
            };

            let processing_result = player.process_special_abilities(&activation_context);
            response_data.updated_player = Some(player.clone());
            response_data.processing_result = Some(processing_result.clone());
            response_data.messages = processing_result.get_messages();
        }

        SpecialAbilityAction::ListAbilities => {
            response_data.abilities = player.get_special_abilities().clone();
            response_data
                .messages
                .push(format!("Player has {} special abilities", response_data.abilities.len()));
        }

        SpecialAbilityAction::GetAbilityEffects => {
            use crate::special_ability::AbilityEffectCalculator;

            let effects =
                AbilityEffectCalculator::calculate_combined_effects(player.get_special_abilities());
            response_data.ability_effects = Some(effects);
            response_data.abilities = player.get_special_abilities().clone();
            response_data.messages.push("Calculated combined ability effects".to_string());
        }
    }

    serde_json::to_string(&ApiResponse::success(response_data)).unwrap()
}

/// Apply special ability effects to OpenFootball skills (JSON API)
/// Used during match simulation to modify player skills
pub fn apply_special_ability_effects_json(
    request_json: &str,
    players: &HashMap<String, CorePlayer>,
) -> String {
    #[derive(Debug, Deserialize)]
    struct ApplyEffectsRequest {
        schema_version: Option<String>,
        player_id: String,
        base_skills: crate::special_ability::OpenFootballSkills,
    }

    #[derive(Debug, Serialize)]
    struct ApplyEffectsResponse {
        player_id: String,
        modified_skills: crate::special_ability::OpenFootballSkills,
        applied_effects: crate::special_ability::SkillEffects,
        ability_count: usize,
    }

    let request: ApplyEffectsRequest = match serde_json::from_str(request_json) {
        Ok(r) => r,
        Err(e) => {
            let error =
                ApiError::new("INVALID_JSON", &format!("Failed to parse request JSON: {}", e));
            return serde_json::to_string(&ApiResponse::<ApplyEffectsResponse>::error(error))
                .unwrap();
        }
    };

    if request.player_id.is_empty() {
        let error = ApiError::new("INVALID_PLAYER_ID", "Player ID cannot be empty");
        return serde_json::to_string(&ApiResponse::<ApplyEffectsResponse>::error(error)).unwrap();
    }

    let player = match players.get(&request.player_id) {
        Some(p) => p,
        None => {
            let error = ApiError::new(
                "PLAYER_NOT_FOUND",
                &format!("Player with ID '{}' not found", request.player_id),
            );
            return serde_json::to_string(&ApiResponse::<ApplyEffectsResponse>::error(error))
                .unwrap();
        }
    };

    let mut modified_skills = request.base_skills;
    player.apply_special_ability_effects_to_openfootball(&mut modified_skills);

    // Calculate the effects that were applied for debugging
    use crate::special_ability::AbilityEffectCalculator;
    let applied_effects =
        AbilityEffectCalculator::calculate_combined_effects(player.get_special_abilities());

    let response_data = ApplyEffectsResponse {
        player_id: request.player_id,
        modified_skills,
        applied_effects,
        ability_count: player.get_special_abilities().len(),
    };

    serde_json::to_string(&ApiResponse::success(response_data)).unwrap()
}

/// Get special ability statistics for a player (JSON API)
/// Provides analysis of player's special ability collection
pub fn get_special_ability_stats_json(
    request_json: &str,
    players: &HashMap<String, CorePlayer>,
) -> String {
    #[derive(Debug, Deserialize)]
    struct StatsRequest {
        schema_version: Option<String>,
        player_id: String,
    }

    #[derive(Debug, Serialize)]
    struct SpecialAbilityStats {
        player_id: String,
        player_name: String,
        total_abilities: usize,
        positive_abilities: usize,
        negative_abilities: usize,
        abilities_by_tier: HashMap<String, usize>,
        abilities_by_category: HashMap<String, usize>,
        combination_history_count: usize,
        estimated_skill_boost: f32,
    }

    let request: StatsRequest = match serde_json::from_str(request_json) {
        Ok(r) => r,
        Err(e) => {
            let error =
                ApiError::new("INVALID_JSON", &format!("Failed to parse request JSON: {}", e));
            return serde_json::to_string(&ApiResponse::<SpecialAbilityStats>::error(error))
                .unwrap();
        }
    };

    let player = match players.get(&request.player_id) {
        Some(p) => p,
        None => {
            let error = ApiError::new(
                "PLAYER_NOT_FOUND",
                &format!("Player with ID '{}' not found", request.player_id),
            );
            return serde_json::to_string(&ApiResponse::<SpecialAbilityStats>::error(error))
                .unwrap();
        }
    };

    let abilities = player.get_special_abilities();
    let positive_abilities = player.get_positive_special_abilities();
    let negative_abilities = player.get_negative_special_abilities();

    // Count by tier
    let mut abilities_by_tier = HashMap::new();
    for ability in abilities {
        let tier_str = format!("{:?}", ability.tier);
        *abilities_by_tier.entry(tier_str).or_insert(0) += 1;
    }

    // Count by category
    let mut abilities_by_category = HashMap::new();
    for ability in abilities {
        let category_str = format!("{:?}", ability.ability_type.category());
        *abilities_by_category.entry(category_str).or_insert(0) += 1;
    }

    // Estimate skill boost (simplified calculation)
    use crate::special_ability::AbilityEffectCalculator;
    let effects = AbilityEffectCalculator::calculate_combined_effects(abilities);
    let estimated_boost = (effects.dribbling
        + effects.passing
        + effects.finishing
        + effects.composure
        + effects.stamina
        + effects.strength)
        / 6.0;

    let stats = SpecialAbilityStats {
        player_id: request.player_id,
        player_name: player.name.clone(),
        total_abilities: abilities.len(),
        positive_abilities: positive_abilities.len(),
        negative_abilities: negative_abilities.len(),
        abilities_by_tier,
        abilities_by_category,
        combination_history_count: player.special_abilities.combination_history.len(),
        estimated_skill_boost: estimated_boost,
    };

    serde_json::to_string(&ApiResponse::success(stats)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::player::Position;
    use std::collections::HashMap;

    fn create_test_player_request_json(
        name: &str,
        position: Position,
        seed: Option<u64>,
    ) -> String {
        serde_json::json!({
            "schema_version": "v1",
            "name": name,
            "position": position,
            "age_months": 16.5,
            "ca": 80,
            "pa": 140,
            "seed": seed
        })
        .to_string()
    }

    #[test]
    fn test_create_player_json_workflow() {
        let request = create_test_player_request_json("Test Player", Position::FW, Some(12345));
        let response = create_player_json(&request);

        let result: ApiResponse<PlayerCreationResponse> =
            serde_json::from_str(&response).expect("Should parse create response");

        assert!(result.success);
        assert_eq!(result.schema_version, "v1");

        let created_player = result.data.unwrap().player;
        assert_eq!(created_player.name, "Test Player");
        assert_eq!(created_player.position, Position::FW);
        assert_eq!(created_player.ca, 80);
        assert_eq!(created_player.pa, 140);
    }

    #[test]
    fn test_update_player_json_workflow() {
        // Create a player first
        let create_request =
            create_test_player_request_json("Update Test", Position::MF, Some(54321));
        let create_response = create_player_json(&create_request);
        let create_result: ApiResponse<PlayerCreationResponse> =
            serde_json::from_str(&create_response).unwrap();
        let player = create_result.data.unwrap().player;

        let mut players = HashMap::new();
        players.insert(player.id.clone(), player.clone());

        // Update the player
        let update_request = serde_json::json!({
            "schema_version": "v1",
            "player_id": player.id,
            "attribute_changes": {
                "finishing": 5,
                "passing": -3
            }
        })
        .to_string();

        let update_response = update_player_json(&update_request, &mut players);
        let update_result: ApiResponse<PlayerUpdateResponse> =
            serde_json::from_str(&update_response).unwrap();

        assert!(update_result.success);
        assert_eq!(update_result.data.unwrap().attribute_changes.len(), 2);
    }

    #[test]
    fn test_get_player_json_workflow() {
        // Create a player first
        let create_request =
            create_test_player_request_json("Query Test", Position::DF, Some(98765));
        let create_response = create_player_json(&create_request);
        let create_result: ApiResponse<PlayerCreationResponse> =
            serde_json::from_str(&create_response).unwrap();
        let player = create_result.data.unwrap().player;

        let mut players = HashMap::new();
        players.insert(player.id.clone(), player.clone());

        // Query the player
        let query_request = serde_json::json!({
            "schema_version": "v1",
            "player_id": player.id
        })
        .to_string();

        let query_response = get_player_json(&query_request, &players);
        let query_result: ApiResponse<PlayerQueryResponse> =
            serde_json::from_str(&query_response).unwrap();

        assert!(query_result.success);
        assert!(query_result.data.unwrap().player.is_some());
    }

    #[test]
    fn test_batch_create_players_json_workflow() {
        let batch_request = serde_json::json!({
            "schema_version": "v1",
            "batch_seed": 11111,
            "players": [
                {
                    "name": "Batch Player 1",
                    "position": "FW",
                    "age_months": 16.0,
                    "ca": 75,
                    "pa": 135
                },
                {
                    "name": "Batch Player 2",
                    "position": "MF",
                    "age_months": 16.5,
                    "ca": 80,
                    "pa": 140
                }
            ]
        })
        .to_string();

        let batch_response = batch_create_players_json(&batch_request);
        let batch_result: ApiResponse<BatchPlayerCreationResponse> =
            serde_json::from_str(&batch_response).unwrap();

        assert!(batch_result.success);
        let batch_data = batch_result.data.unwrap();
        assert_eq!(batch_data.total_created, 2);
        assert_eq!(batch_data.total_failed, 0);
    }

    #[test]
    fn test_growth_simulation_workflow() {
        // Create a test player
        let create_request =
            create_test_player_request_json("Growth Test", Position::FW, Some(22222));
        let create_response = create_player_json(&create_request);
        let create_result: ApiResponse<PlayerCreationResponse> =
            serde_json::from_str(&create_response).unwrap();
        let player = create_result.data.unwrap().player;

        let mut players = HashMap::new();
        players.insert(player.id.clone(), player.clone());

        let growth_request = serde_json::json!({
            "schema_version": "v1",
            "player_id": player.id,
            "months": 6,
            "random_variance": 0.05,
            "seed": 33333,
            "training_schedule": [
                {
                    "training_type": "Shooting",
                    "intensity": 1.0,
                    "sessions_per_month": 4
                }
            ]
        })
        .to_string();

        let growth_response = simulate_growth_json(&growth_request, &players);
        let growth_result: ApiResponse<GrowthSimulationResponse> =
            serde_json::from_str(&growth_response).unwrap();

        assert!(growth_result.success);
        let growth_data = growth_result.data.unwrap();
        assert_eq!(growth_data.growth_history.len(), 6);
    }

    #[test]
    fn test_player_comparison_workflow() {
        // Create multiple test players
        let mut players = HashMap::new();
        let mut player_ids = Vec::new();

        for i in 1..=3 {
            let create_request = create_test_player_request_json(
                &format!("Compare Player {}", i),
                Position::MF,
                Some(i as u64 * 1111),
            );
            let create_response = create_player_json(&create_request);
            let create_result: ApiResponse<PlayerCreationResponse> =
                serde_json::from_str(&create_response).unwrap();
            let player = create_result.data.unwrap().player;
            player_ids.push(player.id.clone());
            players.insert(player.id.clone(), player);
        }

        let comparison_request = serde_json::json!({
            "schema_version": "v1",
            "player_ids": player_ids,
            "comparison_type": "Overall",
            "include_similarity_scores": true
        })
        .to_string();

        let comparison_response = compare_players_json(&comparison_request, &players);
        let comparison_result: ApiResponse<PlayerComparisonResponse> =
            serde_json::from_str(&comparison_response).unwrap();

        assert!(comparison_result.success);
        let comparison_data = comparison_result.data.unwrap();
        assert_eq!(comparison_data.players.len(), 3);
        assert!(comparison_data.rankings.contains_key("ca_ranking"));
    }

    #[test]
    fn test_error_conditions() {
        // Test invalid JSON
        let response = create_player_json("{ invalid json }");
        let result: ApiResponse<PlayerCreationResponse> = serde_json::from_str(&response).unwrap();
        assert!(!result.success);
        assert_eq!(result.error.unwrap().code, "INVALID_JSON");

        // Test batch size exceeded
        let large_batch = serde_json::json!({
            "schema_version": "v1",
            "players": (0..101).map(|i| serde_json::json!({
                "name": format!("Player {}", i),
                "position": "FW",
                "age_months": 16.0
            })).collect::<Vec<_>>()
        })
        .to_string();

        let batch_response = batch_create_players_json(&large_batch);
        let batch_result: ApiResponse<BatchPlayerCreationResponse> =
            serde_json::from_str(&batch_response).unwrap();
        assert!(!batch_result.success);
        assert_eq!(batch_result.error.unwrap().code, "BATCH_SIZE_EXCEEDED");
    }

    #[test]
    fn test_api_response_success() {
        let data = "test_data";
        let response = ApiResponse::success(data);

        assert!(response.success);
        assert_eq!(response.data, Some("test_data"));
        assert!(response.error.is_none());
        assert_eq!(response.schema_version, API_VERSION);
    }

    #[test]
    fn test_api_response_error() {
        let error = ApiError::new("TEST_ERROR", "Test error message");
        let response: ApiResponse<String> = ApiResponse::error(error.clone());

        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error.unwrap().code, "TEST_ERROR");
        assert_eq!(response.schema_version, API_VERSION);
    }

    #[test]
    fn test_player_creation_request_validation() {
        let valid_request = PlayerCreationRequest {
            schema_version: Some(API_VERSION.to_string()),
            name: "Test Player".to_string(),
            position: Position::FW,
            age_months: 16.5,
            ca: Some(100),
            pa: Some(150),
            seed: Some(12345),
            custom_attributes: None,
            growth_profile: None,
        };

        assert!(valid_request.validate().is_ok());

        // Test invalid PA < CA
        let invalid_request = PlayerCreationRequest {
            ca: Some(150),
            pa: Some(100), // PA < CA
            ..valid_request.clone()
        };

        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_player_update_request_validation() {
        let mut attribute_changes = HashMap::new();
        attribute_changes.insert("shooting".to_string(), 10);
        attribute_changes.insert("passing".to_string(), -5);

        let valid_request = PlayerUpdateRequest {
            schema_version: Some(API_VERSION.to_string()),
            player_id: Uuid::new_v4().to_string(),
            attribute_changes,
            position_change: None,
            name_change: None,
            atomic: Some(true),
        };

        assert!(valid_request.validate().is_ok());

        // Test invalid attribute name
        let mut invalid_changes = HashMap::new();
        invalid_changes.insert("invalid_attr".to_string(), 10);

        let invalid_request =
            PlayerUpdateRequest { attribute_changes: invalid_changes, ..valid_request.clone() };

        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_hexagon_stats_change() {
        let before = HexagonStats {
            pace: 10,
            power: 12,
            technical: 8,
            shooting: 15,
            passing: 11,
            defending: 9,
        };
        let after = HexagonStats {
            pace: 12,
            power: 12,
            technical: 10,
            shooting: 16,
            passing: 13,
            defending: 8,
        };

        let change = HexagonStatsChange::new(before, after);

        assert_eq!(change.pace_change, 2);
        assert_eq!(change.power_change, 0);
        assert_eq!(change.technical_change, 2);
        assert_eq!(change.shooting_change, 1);
        assert_eq!(change.passing_change, 2);
        assert_eq!(change.defending_change, -1);
    }

    #[test]
    fn test_is_valid_attribute_name() {
        assert!(is_valid_attribute_name("shooting"));
        assert!(is_valid_attribute_name("passing"));
        assert!(is_valid_attribute_name("handling"));
        assert!(!is_valid_attribute_name("invalid_attr"));
        assert!(!is_valid_attribute_name(""));
    }
}
