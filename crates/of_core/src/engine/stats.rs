use crate::engine::physics_constants::field;
use crate::models::{HeatMapPoint, MatchPositionData, MatchResult, Statistics};

pub struct StatsCalculator {
    // Configuration for statistics
    pub base_passes_per_minute: f32,
    pub base_tackles_per_minute: f32,
}

impl StatsCalculator {
    pub fn new() -> Self {
        Self {
            // v11 튜닝: 목표 700-1000 패스/경기 → 약 9패스/분
            base_passes_per_minute: 9.5, // ~855 passes per match (양팀 합계)
            base_tackles_per_minute: 0.3, // ~27 tackles per team per match
        }
    }

    pub fn finalize(&self, result: &mut MatchResult, possession_ratio: f32) {
        // Set possession percentages
        result.statistics.possession_home = possession_ratio * 100.0;
        result.statistics.possession_away = (1.0 - possession_ratio) * 100.0;

        // NOTE: passes, tackles, fouls are recorded during simulation (tick_based.rs)
        // DO NOT overwrite with possession-based estimates here
        if result.statistics.pass_attempts_home > 0 {
            result.statistics.pass_accuracy_home =
                result.statistics.passes_home as f32 / result.statistics.pass_attempts_home as f32
                    * 100.0;
        }
        if result.statistics.pass_attempts_away > 0 {
            result.statistics.pass_accuracy_away =
                result.statistics.passes_away as f32 / result.statistics.pass_attempts_away as f32
                    * 100.0;
        }

        let pass_attempts_total = result.statistics.pass_attempts_home as f32
            + result.statistics.pass_attempts_away as f32;
        let pass_distance_total =
            result.statistics.pass_distance_sum_home + result.statistics.pass_distance_sum_away;
        let forward_total = result.statistics.forward_pass_attempts_home as f32
            + result.statistics.forward_pass_attempts_away as f32;
        let circulation_total = result.statistics.circulation_pass_attempts_home as f32
            + result.statistics.circulation_pass_attempts_away as f32;
        let sequence_total = result.statistics.pass_sequence_total_home as f32
            + result.statistics.pass_sequence_total_away as f32;
        let sequence_count = result.statistics.pass_sequence_count_home as f32
            + result.statistics.pass_sequence_count_away as f32;

        result.statistics.pass_distance_avg_m = if pass_attempts_total > 0.0 {
            pass_distance_total / pass_attempts_total
        } else {
            0.0
        };
        result.statistics.forward_pass_ratio = if pass_attempts_total > 0.0 {
            forward_total / pass_attempts_total
        } else {
            0.0
        };
        result.statistics.circulation_pass_ratio = if pass_attempts_total > 0.0 {
            circulation_total / pass_attempts_total
        } else {
            0.0
        };
        result.statistics.pass_sequence_avg_len = if sequence_count > 0.0 {
            sequence_total / sequence_count
        } else {
            0.0
        };

        // Telemetry-derived pacing metrics
        const TICKS_PER_MINUTE: f32 = 240.0;
        let total_ticks = result.statistics.total_ticks as f32;
        let minutes = if total_ticks > 0.0 {
            total_ticks / TICKS_PER_MINUTE
        } else {
            0.0
        };

        result.statistics.ball_in_play_ratio = if total_ticks > 0.0 {
            result.statistics.ball_in_play_ticks as f32 / total_ticks
        } else {
            0.0
        };

        let possessions_total =
            result.statistics.possessions_home as f32 + result.statistics.possessions_away as f32;
        result.statistics.possessions_per_min = if minutes > 0.0 {
            possessions_total / minutes
        } else {
            0.0
        };

        result.statistics.actions_per_min = if minutes > 0.0 {
            result.statistics.actions_total as f32 / minutes
        } else {
            0.0
        };

        result.statistics.decisions_executed_per_min = if minutes > 0.0 {
            result.statistics.decisions_executed as f32 / minutes
        } else {
            0.0
        };
        result.statistics.decisions_skipped_per_min = if minutes > 0.0 {
            result.statistics.decisions_skipped as f32 / minutes
        } else {
            0.0
        };

        result.statistics.pass_attempts_per_possession = if possessions_total > 0.0 {
            pass_attempts_total / possessions_total
        } else {
            0.0
        };

        let hold_total =
            result.statistics.hold_actions_home as f32 + result.statistics.hold_actions_away as f32;
        let carry_total =
            result.statistics.carry_actions_home as f32 + result.statistics.carry_actions_away as f32;
        let hold_carry_total = hold_total + carry_total;
        if hold_carry_total > 0.0 {
            result.statistics.hold_action_ratio = hold_total / hold_carry_total;
            result.statistics.carry_action_ratio = carry_total / hold_carry_total;
        }

        let shot_gate_checks = result.statistics.shot_gate_checks as f32;
        if shot_gate_checks > 0.0 {
            result.statistics.shot_gate_allow_ratio =
                result.statistics.shot_gate_allowed as f32 / shot_gate_checks;
        }

        let clear_shot_checks = result.statistics.clear_shot_checks as f32;
        if clear_shot_checks > 0.0 {
            result.statistics.clear_shot_block_ratio =
                result.statistics.clear_shot_blocked as f32 / clear_shot_checks;
        }

        // Count corners and offsides from events
        for event in &result.events {
            match event.event_type {
                crate::models::EventType::Corner => {
                    if event.is_home_team {
                        result.statistics.corners_home += 1;
                    } else {
                        result.statistics.corners_away += 1;
                    }
                }
                crate::models::EventType::Offside => {
                    if event.is_home_team {
                        result.statistics.offsides_home += 1;
                    } else {
                        result.statistics.offsides_away += 1;
                    }
                }
                _ => {}
            }
        }

        // Ensure possession sums to 100
        let total_poss = result.statistics.possession_home + result.statistics.possession_away;
        if (total_poss - 100.0).abs() > 0.1 {
            let ratio = 100.0 / total_poss;
            result.statistics.possession_home *= ratio;
            result.statistics.possession_away *= ratio;
        }

        // Phase E: Advanced Analytics
        self.calculate_advanced_analytics(result);
    }

    fn calculate_advanced_analytics(&self, result: &mut MatchResult) {
        // Possession zones (18 zones: 3 rows x 6 columns on field)
        if let Some(pos_data) = result.position_data.as_ref() {
            self.calculate_possession_zones(&mut result.statistics, pos_data);
            self.calculate_heat_maps(&mut result.statistics, pos_data);
        }

        // Pass matrix (22x22) - for now initialize empty, will be filled during simulation
        result.statistics.pass_matrix_home = vec![vec![0.0; 22]; 22];
        result.statistics.pass_matrix_away = vec![vec![0.0; 22]; 22];
    }

    fn calculate_possession_zones(&self, stats: &mut Statistics, pos_data: &MatchPositionData) {
        // Field dimensions: 105m x 68m, Coord10: 0-1050 x 0-680
        // Zones: 3 rows (defensive, middle, attacking) x 6 columns (width)

        let mut zone_counts_home = [0u32; 18];
        let mut zone_counts_away = [0u32; 18];
        let mut total_positions = 0u32;

        for player_idx in 0..22 {
            let player_history = &pos_data.players[player_idx];
            for pos_item in player_history {
                // Clamp to field bounds before binning; positions may land on
                // exact boundaries (e.g., y=field width) which would otherwise map to
                // an out-of-range row.
                let x = pos_item.position.0.clamp(0.0, field::LENGTH_M);
                let y = pos_item.position.1.clamp(0.0, field::WIDTH_M);

                // Zone calculation
                let col = ((x / field::LENGTH_M) * 6.0).floor() as usize;
                let row = ((y / field::WIDTH_M) * 3.0).floor() as usize;
                let zone_idx = row.min(2) * 6 + col.min(5);

                if player_idx < 11 {
                    zone_counts_home[zone_idx] += 1;
                } else {
                    zone_counts_away[zone_idx] += 1;
                }
                total_positions += 1;
            }
        }

        if total_positions > 0 {
            stats.possession_zones_home = zone_counts_home.iter()
                .map(|&count| count as f32 / total_positions as f32 * 100.0)
                .collect();
            stats.possession_zones_away = zone_counts_away.iter()
                .map(|&count| count as f32 / total_positions as f32 * 100.0)
                .collect();
        }
    }

    fn calculate_heat_maps(&self, stats: &mut Statistics, pos_data: &MatchPositionData) {
        use std::collections::HashMap;

        let mut position_counts_home: HashMap<(i32, i32), u32> = HashMap::new();
        let mut position_counts_away: HashMap<(i32, i32), u32> = HashMap::new();

        // Grid resolution: 5m x 5m
        const GRID_SIZE: f32 = 5.0;

        for player_idx in 0..22 {
            let player_history = &pos_data.players[player_idx];
            for pos_item in player_history {
                let x = pos_item.position.0;
                let y = pos_item.position.1;

                let grid_x = (x / GRID_SIZE).floor() as i32;
                let grid_y = (y / GRID_SIZE).floor() as i32;

                if player_idx < 11 {
                    *position_counts_home.entry((grid_x, grid_y)).or_insert(0) += 1;
                } else {
                    *position_counts_away.entry((grid_x, grid_y)).or_insert(0) += 1;
                }
            }
        }

        // Convert to heat map points
        stats.heat_map_data_home = position_counts_home.into_iter()
            .map(|((gx, gy), count)| HeatMapPoint {
                x: gx as f32 * GRID_SIZE,
                y: gy as f32 * GRID_SIZE,
                intensity: count as f32,
            })
            .collect();

        stats.heat_map_data_away = position_counts_away.into_iter()
            .map(|((gx, gy), count)| HeatMapPoint {
                x: gx as f32 * GRID_SIZE,
                y: gy as f32 * GRID_SIZE,
                intensity: count as f32,
            })
            .collect();
    }

    pub fn calculate_match_rating(&self, stats: &Statistics) -> f32 {
        // Simple rating calculation based on various stats
        let shots_rating = (stats.shots_home + stats.shots_away) as f32 / 20.0;
        let goals_rating = (stats.xg_home + stats.xg_away) / 3.0;
        let pass_rating =
            ((stats.pass_accuracy_home + stats.pass_accuracy_away) / 2.0 - 70.0) / 30.0;

        ((shots_rating + goals_rating + pass_rating) / 3.0).clamp(0.0, 1.0)
    }
}

impl Default for StatsCalculator {
    fn default() -> Self {
        Self::new()
    }
}
