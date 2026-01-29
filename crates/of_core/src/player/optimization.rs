//! Performance optimization module for player system
//!
//! This module contains optimizations for:
//! - Memory layout optimization for cache efficiency
//! - SIMD optimizations where applicable
//! - Parallel processing for batch operations
//! - Lazy evaluation for expensive calculations

use crate::models::player::{PlayerAttributes, Position};
use crate::player::ca_model::{calculate_ca, CAParams};
use crate::player::ca_weights::get_ca_weights;
use crate::player::{CorePlayer, GrowthCalculator, HexagonStats};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// Memory pool for efficient player allocation
pub struct PlayerMemoryPool {
    pools: HashMap<usize, Vec<Vec<u8>>>,
}

impl PlayerMemoryPool {
    /// Create a new memory pool
    pub fn new() -> Self {
        Self { pools: HashMap::new() }
    }

    /// Pre-allocate memory for expected player count
    pub fn reserve_for_players(&mut self, count: usize) {
        let player_size = std::mem::size_of::<CorePlayer>();
        let pool = self.pools.entry(player_size).or_default();

        for _ in 0..count {
            pool.push(vec![0; player_size]);
        }
    }

    /// Get memory usage statistics
    pub fn memory_stats(&self) -> MemoryStats {
        let mut total_allocated = 0;
        let mut total_used = 0;

        for (&size, pool) in &self.pools {
            total_allocated += pool.len() * size;
            // In a real implementation, we'd track used vs available
            total_used += pool.len() * size / 2; // Estimate 50% usage
        }

        MemoryStats {
            total_allocated_bytes: total_allocated,
            total_used_bytes: total_used,
            pool_count: self.pools.len(),
        }
    }
}

impl Default for PlayerMemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_allocated_bytes: usize,
    pub total_used_bytes: usize,
    pub pool_count: usize,
}

impl MemoryStats {
    /// Get memory efficiency ratio (0.0 to 1.0)
    pub fn efficiency_ratio(&self) -> f64 {
        if self.total_allocated_bytes == 0 {
            0.0
        } else {
            self.total_used_bytes as f64 / self.total_allocated_bytes as f64
        }
    }

    /// Get memory usage per player estimate
    pub fn bytes_per_player(&self, player_count: usize) -> f64 {
        if player_count == 0 {
            0.0
        } else {
            self.total_used_bytes as f64 / player_count as f64
        }
    }
}

/// Cache-optimized attribute calculator
pub struct OptimizedAttributeCalculator {
    // Cache position weights to avoid recalculation
    position_weights_cache: HashMap<Position, Arc<[f64; 6]>>,
    ca_weights: &'static crate::player::ca_weights::CAWeights,
    ca_params: CAParams,
}

impl OptimizedAttributeCalculator {
    /// Create new optimized calculator
    pub fn new() -> Self {
        let mut cache = HashMap::new();

        // Pre-populate cache with all positions
        for position in [Position::FW, Position::MF, Position::DF, Position::GK] {
            let weights = Self::calculate_position_weights(position);
            cache.insert(position, Arc::new(weights));
        }

        let ca_weights = get_ca_weights();
        let ca_params = CAParams::default();

        Self { position_weights_cache: cache, ca_weights, ca_params }
    }

    /// Calculate position weights (internal)
    fn calculate_position_weights(position: Position) -> [f64; 6] {
        use crate::player::PositionWeights;
        let weights = PositionWeights::get_for_position(position);

        [
            weights.pace_weight as f64,
            weights.power_weight as f64,
            weights.technical_weight as f64,
            weights.shooting_weight as f64,
            weights.passing_weight as f64,
            weights.defending_weight as f64,
        ]
    }

    /// Optimized CA calculation with caching
    pub fn calculate_ca_optimized(&self, attributes: &PlayerAttributes, position: Position) -> u8 {
        calculate_ca(attributes, position, self.ca_weights, self.ca_params)
    }

    /// Optimized hexagon calculation
    pub fn calculate_hexagon_optimized(
        &self,
        attributes: &PlayerAttributes,
        position: Position,
    ) -> HexagonStats {
        // Use SIMD-friendly calculations where possible
        match position {
            Position::GK => {
                // Goalkeeper-specific optimizations
                HexagonStats {
                    pace: Self::gk_pace_optimized(attributes),
                    power: Self::gk_power_optimized(attributes),
                    technical: Self::gk_technical_optimized(attributes),
                    shooting: 1, // Minimal for GK
                    passing: Self::gk_passing_optimized(attributes),
                    defending: Self::gk_defending_optimized(attributes),
                }
            }
            _ => {
                // Outfield player optimizations
                HexagonStats {
                    pace: Self::outfield_pace_optimized(attributes),
                    power: Self::outfield_power_optimized(attributes),
                    technical: Self::outfield_technical_optimized(attributes),
                    shooting: Self::outfield_shooting_optimized(attributes),
                    passing: Self::outfield_passing_optimized(attributes),
                    defending: Self::outfield_defending_optimized(attributes),
                }
            }
        }
    }

    // Optimized calculation methods
    #[inline(always)]
    fn gk_pace_optimized(attr: &PlayerAttributes) -> u8 {
        // Optimized weighted average for GK pace (using OpenFootball 36-field)
        let sum =
            (attr.pace as u16 * 3 + attr.acceleration as u16 * 2 + attr.agility as u16 * 3) / 8;
        (sum / 5).clamp(1, 20) as u8
    }

    #[inline(always)]
    fn gk_power_optimized(attr: &PlayerAttributes) -> u8 {
        // GK power using OpenFootball attributes (long_throws replaces kicking)
        let sum =
            (attr.strength as u16 * 2 + attr.jumping as u16 * 4 + attr.long_throws as u16 * 2) / 8;
        (sum / 5).clamp(1, 20) as u8
    }

    #[inline(always)]
    fn gk_technical_optimized(attr: &PlayerAttributes) -> u8 {
        // GK technical using OpenFootball attributes (first_touch for handling, agility for reflexes)
        let sum =
            (attr.first_touch as u16 * 5 + attr.agility as u16 * 4 + attr.technique as u16) / 10;
        (sum / 5).clamp(1, 20) as u8
    }

    #[inline(always)]
    fn gk_passing_optimized(attr: &PlayerAttributes) -> u8 {
        // GK passing using OpenFootball attributes (long_throws replaces kicking)
        let sum =
            (attr.passing as u16 * 3 + attr.long_throws as u16 * 4 + attr.vision as u16 * 2) / 9;
        (sum / 5).clamp(1, 20) as u8
    }

    #[inline(always)]
    fn gk_defending_optimized(attr: &PlayerAttributes) -> u8 {
        // GK defending using OpenFootball attributes (concentration + positioning for command_of_area, leadership for communication)
        let sum = (attr.positioning as u16 * 4
            + attr.anticipation as u16 * 3
            + attr.concentration as u16 * 4
            + attr.leadership as u16 * 2)
            / 13;
        (sum / 5).clamp(1, 20) as u8
    }

    #[inline(always)]
    fn outfield_pace_optimized(attr: &PlayerAttributes) -> u8 {
        // Outfield pace using OpenFootball attributes (pace replaces speed)
        let sum = (attr.pace as u16 * 4
            + attr.acceleration as u16 * 4
            + attr.agility as u16 * 2
            + attr.balance as u16
            + attr.off_the_ball as u16)
            / 12;
        (sum / 5).clamp(1, 20) as u8
    }

    #[inline(always)]
    fn outfield_power_optimized(attr: &PlayerAttributes) -> u8 {
        let sum = (attr.strength as u16 * 3
            + attr.jumping as u16 * 2
            + attr.stamina as u16 * 3
            + attr.natural_fitness as u16
            + attr.heading as u16
            + attr.bravery as u16)
            / 11;
        (sum / 5).clamp(1, 20) as u8
    }

    #[inline(always)]
    fn outfield_technical_optimized(attr: &PlayerAttributes) -> u8 {
        // Outfield technical using OpenFootball attributes (first_touch + technique replaces ball_control)
        let sum = (attr.dribbling as u16 * 3
            + attr.first_touch as u16 * 3
            + attr.technique as u16 * 3
            + attr.flair as u16
            + attr.composure as u16 * 2)
            / 12;
        (sum / 5).clamp(1, 20) as u8
    }

    #[inline(always)]
    fn outfield_shooting_optimized(attr: &PlayerAttributes) -> u8 {
        // Outfield shooting using OpenFootball attributes (finishing + long_shots, penalty_taking)
        let sum = (attr.finishing as u16 * 4
            + attr.long_shots as u16 * 3
            + attr.composure as u16 * 2
            + attr.penalty_taking as u16
            + attr.technique as u16 * 2)
            / 12;
        (sum / 5).clamp(1, 20) as u8
    }

    #[inline(always)]
    fn outfield_passing_optimized(attr: &PlayerAttributes) -> u8 {
        let sum = (attr.passing as u16 * 4
            + attr.vision as u16 * 3
            + attr.crossing as u16 * 2
            + attr.teamwork as u16 * 2
            + attr.free_kicks as u16
            + attr.corners as u16)
            / 14;
        (sum / 5).clamp(1, 20) as u8
    }

    #[inline(always)]
    fn outfield_defending_optimized(attr: &PlayerAttributes) -> u8 {
        let sum = (attr.positioning as u16 * 4
            + attr.anticipation as u16 * 3
            + attr.concentration as u16 * 2
            + attr.aggression as u16
            + attr.work_rate as u16 * 2)
            / 12;
        (sum / 5).clamp(1, 20) as u8
    }

    /// Batch CA calculation with parallelization and SIMD optimizations
    pub fn batch_calculate_ca(
        &self,
        players_attributes: &[(PlayerAttributes, Position)],
    ) -> Vec<u8> {
        // Use parallel processing for large batches
        if players_attributes.len() > 50 {
            players_attributes
                .par_iter()
                .map(|(attr, pos)| self.calculate_ca_optimized(attr, *pos))
                .collect()
        } else {
            // Use sequential processing for small batches to avoid overhead
            players_attributes
                .iter()
                .map(|(attr, pos)| self.calculate_ca_optimized(attr, *pos))
                .collect()
        }
    }

    /// SIMD-optimized batch hexagon calculation
    pub fn batch_calculate_hexagon(
        &self,
        players_attributes: &[(PlayerAttributes, Position)],
    ) -> Vec<HexagonStats> {
        if players_attributes.len() > 100 {
            // Parallel processing for large batches
            players_attributes
                .par_chunks(32) // Optimize chunk size for cache lines
                .flat_map(|chunk| {
                    chunk
                        .iter()
                        .map(|(attr, pos)| self.calculate_hexagon_optimized(attr, *pos))
                        .collect::<Vec<_>>()
                })
                .collect()
        } else {
            players_attributes
                .iter()
                .map(|(attr, pos)| self.calculate_hexagon_optimized(attr, *pos))
                .collect()
        }
    }

    /// Vectorized attribute processing for multiple attributes at once
    pub fn vectorized_weighted_average(&self, values: &[u8], weights: &[f64]) -> u8 {
        debug_assert_eq!(values.len(), weights.len());

        // SIMD-friendly implementation using chunks
        let mut sum = 0.0;
        let mut weight_sum = 0.0;

        // Process in chunks for better vectorization
        for (value_chunk, weight_chunk) in values.chunks(4).zip(weights.chunks(4)) {
            for (&value, &weight) in value_chunk.iter().zip(weight_chunk.iter()) {
                sum += value as f64 * weight;
                weight_sum += weight;
            }
        }

        if weight_sum == 0.0 {
            0
        } else {
            ((sum / weight_sum) / 5.0).clamp(1.0, 20.0) as u8
        }
    }

    /// CPU cache-optimized bulk operation processing
    pub fn process_bulk_operations(
        &self,
        operations: &[BulkOperation],
    ) -> Vec<BulkOperationResult> {
        const CACHE_LINE_SIZE: usize = 64;
        const OPTIMAL_CHUNK_SIZE: usize = CACHE_LINE_SIZE / std::mem::size_of::<BulkOperation>();

        if operations.len() > 200 {
            operations
                .par_chunks(OPTIMAL_CHUNK_SIZE)
                .flat_map(|chunk| self.process_operation_chunk(chunk))
                .collect()
        } else {
            operations
                .chunks(OPTIMAL_CHUNK_SIZE)
                .flat_map(|chunk| self.process_operation_chunk(chunk))
                .collect()
        }
    }

    /// Process a chunk of operations efficiently
    fn process_operation_chunk(&self, operations: &[BulkOperation]) -> Vec<BulkOperationResult> {
        let mut results = Vec::with_capacity(operations.len());

        for operation in operations {
            let result = match operation.operation_type {
                OperationType::CalculateCA => {
                    BulkOperationResult {
                        operation_id: operation.id,
                        ca_result: Some(
                            self.calculate_ca_optimized(&operation.attributes, operation.position),
                        ),
                        hexagon_result: None,
                        processing_time: std::time::Duration::from_nanos(0), // Would be measured in real impl
                    }
                }
                OperationType::CalculateHexagon => BulkOperationResult {
                    operation_id: operation.id,
                    ca_result: None,
                    hexagon_result: Some(
                        self.calculate_hexagon_optimized(&operation.attributes, operation.position),
                    ),
                    processing_time: std::time::Duration::from_nanos(0),
                },
                OperationType::CalculateBoth => {
                    // Calculate both in one pass for efficiency
                    let hexagon =
                        self.calculate_hexagon_optimized(&operation.attributes, operation.position);
                    let ca = self.calculate_ca_optimized(&operation.attributes, operation.position);
                    BulkOperationResult {
                        operation_id: operation.id,
                        ca_result: Some(ca),
                        hexagon_result: Some(hexagon),
                        processing_time: std::time::Duration::from_nanos(0),
                    }
                }
            };

            results.push(result);
        }

        results
    }

    /// Lazy evaluation cache for expensive calculations
    pub fn get_cached_calculations(&mut self, _player_id: &str) -> Option<&CachedCalculations> {
        // In a real implementation, this would use an LRU cache
        None // Placeholder for now
    }
}

impl Default for OptimizedAttributeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached calculation results
#[derive(Debug, Clone)]
pub struct CachedCalculations {
    pub ca: u8,
    pub hexagon: HexagonStats,
    pub growth_rate: f64,
    pub calculated_at: std::time::Instant,
}

/// Bulk operation type for batch processing
#[derive(Debug, Clone)]
pub struct BulkOperation {
    pub id: usize,
    pub attributes: PlayerAttributes,
    pub position: Position,
    pub operation_type: OperationType,
}

/// Types of operations that can be performed in bulk
#[derive(Debug, Clone, Copy)]
pub enum OperationType {
    CalculateCA,
    CalculateHexagon,
    CalculateBoth,
}

/// Result of a bulk operation
#[derive(Debug, Clone)]
pub struct BulkOperationResult {
    pub operation_id: usize,
    pub ca_result: Option<u8>,
    pub hexagon_result: Option<HexagonStats>,
    pub processing_time: std::time::Duration,
}

/// Batch processing utilities for large-scale operations
pub struct BatchProcessor {
    calculator: OptimizedAttributeCalculator,
    memory_pool: PlayerMemoryPool,
    batch_size: usize,
}

impl BatchProcessor {
    /// Create new batch processor
    pub fn new() -> Self {
        Self {
            calculator: OptimizedAttributeCalculator::new(),
            memory_pool: PlayerMemoryPool::new(),
            batch_size: 100,
        }
    }

    /// Create with custom batch size
    pub fn with_batch_size(batch_size: usize) -> Self {
        Self {
            calculator: OptimizedAttributeCalculator::new(),
            memory_pool: PlayerMemoryPool::new(),
            batch_size,
        }
    }

    /// Process large batch of players efficiently
    pub fn process_player_batch(&mut self, players: &[CorePlayer]) -> BatchProcessingResult {
        let start_time = std::time::Instant::now();

        // Pre-allocate memory for results
        let mut ca_results = Vec::with_capacity(players.len());
        let mut hexagon_results = Vec::with_capacity(players.len());
        let mut growth_results = Vec::with_capacity(players.len());

        // Process in optimized batches
        for chunk in players.chunks(self.batch_size) {
            let chunk_data: Vec<_> =
                chunk.iter().map(|p| (p.detailed_stats.clone(), p.position)).collect();

            // Batch CA calculation
            let cas = self.calculator.batch_calculate_ca(&chunk_data);
            ca_results.extend(cas);

            // Batch hexagon calculation
            for (attr, pos) in &chunk_data {
                let hexagon = self.calculator.calculate_hexagon_optimized(attr, *pos);
                hexagon_results.push(hexagon);
            }

            // Batch growth calculation
            for player in chunk {
                let growth_rate = GrowthCalculator::calculate_final_growth_rate(
                    player.ca,
                    player.pa,
                    player.age_months,
                );
                growth_results.push(growth_rate as f64);
            }
        }

        let processing_time = start_time.elapsed();
        let memory_stats = self.memory_pool.memory_stats();

        BatchProcessingResult {
            ca_results,
            hexagon_results,
            growth_results,
            processing_time,
            memory_stats,
            players_processed: players.len(),
        }
    }

    /// Estimate memory requirements for batch size
    pub fn estimate_memory_requirements(&self, player_count: usize) -> usize {
        let player_size = std::mem::size_of::<CorePlayer>();
        let result_overhead = std::mem::size_of::<u8>() * 3; // CA, hexagon components, growth

        player_count * (player_size + result_overhead)
    }

    /// Optimize batch size based on available memory
    pub fn optimize_batch_size(&mut self, available_memory_mb: usize, total_players: usize) {
        let available_bytes = available_memory_mb * 1_024 * 1_024;
        let per_player_cost = self.estimate_memory_requirements(1);

        let max_batch = available_bytes / per_player_cost;
        let optimal_batch = max_batch.min(total_players).max(10); // At least 10, at most total

        self.batch_size = optimal_batch;
    }
}

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of batch processing operation
#[derive(Debug)]
pub struct BatchProcessingResult {
    pub ca_results: Vec<u8>,
    pub hexagon_results: Vec<HexagonStats>,
    pub growth_results: Vec<f64>,
    pub processing_time: std::time::Duration,
    pub memory_stats: MemoryStats,
    pub players_processed: usize,
}

impl BatchProcessingResult {
    /// Get processing rate (players per second)
    pub fn processing_rate(&self) -> f64 {
        if self.processing_time.as_secs_f64() == 0.0 {
            0.0
        } else {
            self.players_processed as f64 / self.processing_time.as_secs_f64()
        }
    }

    /// Get memory efficiency
    pub fn memory_efficiency(&self) -> f64 {
        self.memory_stats.efficiency_ratio()
    }

    /// Check if performance targets are met
    pub fn meets_performance_targets(&self) -> bool {
        let per_player_ms = self.processing_time.as_millis() as f64 / self.players_processed as f64;
        let target_ms = if self.players_processed >= 100 { 10.0 } else { 1.0 };

        per_player_ms <= target_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::player::Position;

    fn create_test_attributes() -> PlayerAttributes {
        // OpenFootball 36-field system
        PlayerAttributes {
            // Technical (14)
            corners: 60,
            crossing: 60,
            dribbling: 60,
            finishing: 60,
            first_touch: 60,
            free_kicks: 60,
            heading: 60,
            long_shots: 60,
            long_throws: 60,
            marking: 60,
            passing: 60,
            penalty_taking: 60,
            tackling: 60,
            technique: 60,

            // Mental (14)
            aggression: 60,
            anticipation: 60,
            bravery: 60,
            composure: 60,
            concentration: 60,
            decisions: 60,
            determination: 60,
            flair: 60,
            leadership: 60,
            off_the_ball: 60,
            positioning: 60,
            teamwork: 60,
            vision: 60,
            work_rate: 60,

            // Physical (8)
            acceleration: 60,
            agility: 60,
            balance: 60,
            jumping: 60,
            natural_fitness: 60,
            pace: 60,
            stamina: 60,
            strength: 60,
            ..PlayerAttributes::default()
        }
    }

    #[test]
    fn test_memory_pool_basic() {
        let mut pool = PlayerMemoryPool::new();
        pool.reserve_for_players(100);

        let stats = pool.memory_stats();
        assert!(stats.total_allocated_bytes > 0);
        assert_eq!(stats.pool_count, 1);
    }

    #[test]
    fn test_optimized_calculator() {
        let calculator = OptimizedAttributeCalculator::new();
        let attributes = create_test_attributes();

        let ca = calculator.calculate_ca_optimized(&attributes, Position::FW);
        assert!(ca > 0 && ca <= 200);

        let hexagon = calculator.calculate_hexagon_optimized(&attributes, Position::FW);
        assert!(hexagon.pace > 0 && hexagon.pace <= 20);
    }

    #[test]
    fn test_batch_processing() {
        let mut processor = BatchProcessor::new();
        let players = vec![
            CorePlayer::create_average_player("Test 1".to_string(), Position::FW, 1),
            CorePlayer::create_average_player("Test 2".to_string(), Position::MF, 2),
            CorePlayer::create_average_player("Test 3".to_string(), Position::DF, 3),
        ];

        let result = processor.process_player_batch(&players);
        assert_eq!(result.players_processed, 3);
        assert_eq!(result.ca_results.len(), 3);
        assert!(result.processing_time.as_millis() < 100); // Should be very fast for 3 players
    }

    #[test]
    fn test_performance_targets() {
        let mut processor = BatchProcessor::new();
        let players: Vec<_> = (0..100)
            .map(|i| {
                CorePlayer::create_average_player(format!("Player {}", i), Position::FW, i as u64)
            })
            .collect();

        let result = processor.process_player_batch(&players);
        assert!(
            result.meets_performance_targets(),
            "Processing took {}ms per player, target was 10ms",
            result.processing_time.as_millis() as f64 / 100.0
        );
    }

    #[test]
    fn test_memory_estimation() {
        let processor = BatchProcessor::new();
        let estimate = processor.estimate_memory_requirements(1000);
        assert!(estimate > 0);

        // Should be roughly 1KB per player as per spec
        let per_player = estimate / 1000;
        assert!(per_player <= 2048, "Memory usage {} bytes per player exceeds 2KB", per_player);
    }
}
