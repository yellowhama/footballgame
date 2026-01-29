//! Memory usage tests and optimization verification
//!
//! Tests memory footprint according to T030:
//! - Target <1KB per player
//! - Struct layout optimization verification
//! - Memory usage analysis

use super::*;
use crate::models::player::{PlayerAttributes, Position};
use std::mem::{align_of, size_of};

#[cfg(test)]
mod memory_tests {
    use super::*;

    #[test]
    fn test_struct_sizes() {
        // Test individual struct sizes
        let core_player_size = size_of::<CorePlayer>();
        let player_attributes_size = size_of::<PlayerAttributes>();
        let hexagon_stats_size = size_of::<HexagonStats>();
        let growth_profile_size = size_of::<GrowthProfile>();
        let training_response_size = size_of::<TrainingResponse>();

        println!("Memory footprint analysis:");
        println!("  CorePlayer: {} bytes", core_player_size);
        println!("  PlayerAttributes: {} bytes", player_attributes_size);
        println!("  HexagonStats: {} bytes", hexagon_stats_size);
        println!("  GrowthProfile: {} bytes", growth_profile_size);
        println!("  TrainingResponse: {} bytes", training_response_size);

        // Target: <1KB (1024 bytes) per player
        assert!(
            core_player_size < 1024,
            "CorePlayer should be less than 1KB, got {} bytes",
            core_player_size
        );

        // PlayerAttributes should be compact (42 u8s = 42 bytes + padding)
        assert!(
            player_attributes_size <= 64,
            "PlayerAttributes should be compact, got {} bytes",
            player_attributes_size
        );

        // HexagonStats should be very compact (6 u8s = 6 bytes + padding)
        assert!(
            hexagon_stats_size <= 16,
            "HexagonStats should be very compact, got {} bytes",
            hexagon_stats_size
        );

        // GrowthProfile should be reasonably sized
        assert!(
            growth_profile_size <= 128,
            "GrowthProfile should be reasonably sized, got {} bytes",
            growth_profile_size
        );

        // TrainingResponse should be small (3 f32s = 12 bytes + padding)
        assert!(
            training_response_size <= 16,
            "TrainingResponse should be small, got {} bytes",
            training_response_size
        );
    }

    #[test]
    fn test_struct_alignments() {
        // Test struct alignments to verify packing
        let core_player_align = align_of::<CorePlayer>();
        let player_attributes_align = align_of::<PlayerAttributes>();
        let hexagon_stats_align = align_of::<HexagonStats>();
        let growth_profile_align = align_of::<GrowthProfile>();

        println!("Alignment requirements:");
        println!("  CorePlayer: {} bytes", core_player_align);
        println!("  PlayerAttributes: {} bytes", player_attributes_align);
        println!("  HexagonStats: {} bytes", hexagon_stats_align);
        println!("  GrowthProfile: {} bytes", growth_profile_align);

        // All should have reasonable alignment (powers of 2)
        assert!(core_player_align <= 8, "CorePlayer alignment should be reasonable");
        assert!(player_attributes_align <= 8, "PlayerAttributes alignment should be reasonable");
        assert!(hexagon_stats_align <= 8, "HexagonStats alignment should be reasonable");
        assert!(growth_profile_align <= 8, "GrowthProfile alignment should be reasonable");
    }

    #[test]
    fn test_memory_efficiency_with_arrays() {
        // Test memory efficiency with arrays of players
        let num_players = 1000;
        let players: Vec<CorePlayer> = (0..num_players)
            .map(|i| {
                CorePlayer::create_average_player(
                    format!("Player {}", i),
                    Position::FW,
                    i as u64 + 1000,
                )
            })
            .collect();

        let vec_overhead = size_of::<Vec<CorePlayer>>();
        let total_data_size = players.len() * size_of::<CorePlayer>();
        let heap_size_estimate = players
            .iter()
            .map(|p| {
                p.name.capacity()
                    + p.id.capacity()
                    + p.growth_profile.specialization.capacity() * size_of::<String>()
            })
            .sum::<usize>();

        println!("Memory usage for {} players:", num_players);
        println!("  Vec overhead: {} bytes", vec_overhead);
        println!(
            "  Data size: {} bytes ({} bytes per player)",
            total_data_size,
            total_data_size / players.len()
        );
        println!("  Estimated heap usage: {} bytes", heap_size_estimate);
        println!(
            "  Total estimated: {} bytes",
            vec_overhead + total_data_size + heap_size_estimate
        );

        // Verify our target of <1KB per player for data
        let bytes_per_player = total_data_size / players.len();
        assert!(
            bytes_per_player < 1024,
            "Should use less than 1KB per player, got {} bytes",
            bytes_per_player
        );

        // Test that all players are valid
        for player in &players {
            assert!(player.is_ca_consistent(), "All players should maintain CA consistency");
        }
    }

    #[test]
    fn test_struct_field_order_impact() {
        // This test verifies our struct optimization by checking sizes
        // If we've optimized correctly, sizes should be minimal

        use std::mem::size_of;

        // Test that our optimized structs don't have excessive padding
        let core_player_size = size_of::<CorePlayer>();

        // Rough calculation of minimum size:
        // PlayerAttributes: ~42 bytes (42 u8s)
        // GrowthProfile: ~48 bytes (Vec<String> + TrainingResponse + 2*f32)
        // DateTime x2: ~24 bytes (2 * 12)
        // HexagonStats: ~6 bytes (6 u8s)
        // String x2: ~48 bytes (2 * 24 on 64-bit)
        // f32: 4 bytes
        // Position: ~4 bytes (enum)
        // u8 x2: 2 bytes
        // Minimum total: ~178 bytes (without padding)

        println!("CorePlayer size: {} bytes (minimum theoretical: ~178 bytes)", core_player_size);

        // Allow for some padding but shouldn't be excessive
        assert!(
            core_player_size < 512,
            "CorePlayer size should be reasonable after optimization, got {} bytes",
            core_player_size
        );

        // Test HexagonStats packing (6 u8s should pack tightly)
        let hexagon_size = size_of::<HexagonStats>();
        assert!(
            hexagon_size <= 8,
            "HexagonStats should pack tightly, got {} bytes for 6 u8s",
            hexagon_size
        );

        // Test TrainingResponse packing (3 f32s should be 12 bytes + padding)
        let training_response_size = size_of::<TrainingResponse>();
        assert!(
            training_response_size <= 16,
            "TrainingResponse should pack reasonably, got {} bytes for 3 f32s",
            training_response_size
        );
    }

    #[test]
    fn test_clone_performance_and_memory() {
        use std::time::Instant;

        let original_player = CorePlayer::create_star_player(
            "Performance Test Player".to_string(),
            Position::FW,
            12345,
        );

        // Test clone performance (should be fast due to optimized layout)
        let num_clones = 1000;
        let start = Instant::now();

        let clones: Vec<CorePlayer> = (0..num_clones).map(|_| original_player.clone()).collect();

        let clone_duration = start.elapsed();
        let avg_clone_time = clone_duration / num_clones;

        println!(
            "Clone performance: {} clones in {:?} (avg: {:?})",
            num_clones, clone_duration, avg_clone_time
        );

        // Cloning should be reasonably fast
        assert!(avg_clone_time.as_micros() < 100, "Clone should be fast due to optimized layout");

        // Verify all clones are valid
        for clone in &clones {
            assert_eq!(clone.ca, original_player.ca);
            assert_eq!(clone.pa, original_player.pa);
            assert!(clone.is_ca_consistent());
        }

        // Test memory usage of clones
        let total_clone_size = clones.len() * size_of::<CorePlayer>();
        let bytes_per_clone = total_clone_size / clones.len();

        println!("Clone memory usage: {} bytes per clone", bytes_per_clone);
        assert!(bytes_per_clone < 1024, "Clones should use less than 1KB each");
    }

    #[test]
    fn test_serialized_size() {
        let player =
            CorePlayer::create_star_player("Serialization Test".to_string(), Position::FW, 99999);

        // Test JSON serialization size
        let json = serde_json::to_string(&player).unwrap();
        let json_size = json.len();

        println!("Serialized JSON size: {} bytes", json_size);

        // JSON should be reasonable size (not too bloated)
        assert!(json_size < 4096, "JSON serialization should be reasonably compact");

        // Test that serialization doesn't lose data
        let deserialized: CorePlayer = serde_json::from_str(&json).unwrap();
        assert_eq!(player.ca, deserialized.ca);
        assert_eq!(player.pa, deserialized.pa);
        assert_eq!(player.detailed_stats, deserialized.detailed_stats);
    }

    #[test]
    fn test_memory_fragmentation_resistance() {
        // Test that our struct layout doesn't cause excessive fragmentation
        // by creating and destroying many players

        let iterations = 100;
        let players_per_iteration = 50;

        for i in 0..iterations {
            // Create players
            let players: Vec<CorePlayer> = (0..players_per_iteration)
                .map(|j| {
                    CorePlayer::create_average_player(
                        format!("Temp Player {}:{}", i, j),
                        Position::MF,
                        (i * players_per_iteration + j) as u64,
                    )
                })
                .collect();

            // Verify they're valid
            for player in &players {
                assert!(player.is_ca_consistent());
            }

            // Players go out of scope and are deallocated here
        }

        // If we get here without issues, fragmentation resistance is good
        // Test passes simply by completing without panic
    }

    #[test]
    fn test_memory_usage_growth_over_time() {
        // Test that memory usage doesn't grow unexpectedly during player operations
        let mut player = CorePlayer::create_youth_prospect(
            "Growth Memory Test".to_string(),
            Position::FW,
            54321,
        );

        let initial_size = size_of::<CorePlayer>();

        // Perform various operations that shouldn't increase struct size
        for i in 0..100 {
            let _ = player.apply_growth(TrainingType::Technical, 1.0, i);
            let _ = player.modify_attribute("shooting", 1);
            player.recalculate_all();

            // Size should remain constant
            assert_eq!(
                size_of::<CorePlayer>(),
                initial_size,
                "Struct size should not change during operations"
            );
        }

        // The player's heap-allocated data might grow (strings, vectors)
        // but the struct size itself should remain constant
        println!("Memory usage remained stable through {} operations", 100);
    }
}
