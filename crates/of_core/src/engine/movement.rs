//! Player movement and position calculation utilities
//!
//! This module contains:
//! - PositionRole enum for classifying player roles
//! - Formation slot to position key mapping
//! - Tactical target position calculations

use super::positioning::{PositionKey, PositionWaypoints};
use super::tactical_brain::{DefensiveGoal, OffensiveGoal, SpaceCreationType};

/// Player role groups for behavior modifiers
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionRole {
    Goalkeeper,
    Defender,
    Midfielder,
    Forward,
}

/// Classify position into role group for behavior modifiers
pub fn get_position_role(position_key: PositionKey) -> PositionRole {
    use PositionKey::*;
    match position_key {
        GK => PositionRole::Goalkeeper,
        LB | LCB | CB | RCB | RB | LWB | RWB | CDM | LDM | RDM => PositionRole::Defender,
        LM | LCM | CM | RCM | RM | LAM | CAM | RAM => PositionRole::Midfielder,
        LW | RW | LF | CF | RF | ST => PositionRole::Forward,
    }
}

/// Convert slot index to PositionKey based on formation
pub fn slot_to_position_key(slot: usize, formation: &str) -> PositionKey {
    match formation {
        "4-4-2" | "442" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LB,
            2 => PositionKey::LCB,
            3 => PositionKey::RCB,
            4 => PositionKey::RB,
            5 => PositionKey::LM,
            6 => PositionKey::LCM,
            7 => PositionKey::RCM,
            8 => PositionKey::RM,
            9 => PositionKey::LF,
            10 => PositionKey::RF,
            _ => PositionKey::CM,
        },
        "4-3-3" | "433" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LB,
            2 => PositionKey::LCB,
            3 => PositionKey::RCB,
            4 => PositionKey::RB,
            5 => PositionKey::LCM,
            6 => PositionKey::CM,
            7 => PositionKey::RCM,
            8 => PositionKey::LW,
            9 => PositionKey::ST,
            10 => PositionKey::RW,
            _ => PositionKey::CM,
        },
        "4-2-3-1" | "4231" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LB,
            2 => PositionKey::LCB,
            3 => PositionKey::RCB,
            4 => PositionKey::RB,
            5 => PositionKey::LDM,
            6 => PositionKey::RDM,
            7 => PositionKey::LAM,
            8 => PositionKey::CAM,
            9 => PositionKey::RAM,
            10 => PositionKey::ST,
            _ => PositionKey::CM,
        },
        "4-1-4-1" | "4141" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LB,
            2 => PositionKey::LCB,
            3 => PositionKey::RCB,
            4 => PositionKey::RB,
            5 => PositionKey::CDM,
            6 => PositionKey::LM,
            7 => PositionKey::LCM,
            8 => PositionKey::RCM,
            9 => PositionKey::RM,
            10 => PositionKey::ST,
            _ => PositionKey::CM,
        },
        "4-4-1-1" | "4411" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LB,
            2 => PositionKey::LCB,
            3 => PositionKey::RCB,
            4 => PositionKey::RB,
            5 => PositionKey::LM,
            6 => PositionKey::LCM,
            7 => PositionKey::RCM,
            8 => PositionKey::RM,
            9 => PositionKey::CF,
            10 => PositionKey::ST,
            _ => PositionKey::CM,
        },
        "4-3-2-1" | "4321" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LB,
            2 => PositionKey::LCB,
            3 => PositionKey::RCB,
            4 => PositionKey::RB,
            5 => PositionKey::LCM,
            6 => PositionKey::CM,
            7 => PositionKey::RCM,
            8 => PositionKey::LF,
            9 => PositionKey::RF,
            10 => PositionKey::ST,
            _ => PositionKey::CM,
        },
        "4-2-2-2" | "4222" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LB,
            2 => PositionKey::LCB,
            3 => PositionKey::RCB,
            4 => PositionKey::RB,
            5 => PositionKey::LDM,
            6 => PositionKey::RDM,
            7 => PositionKey::LAM,
            8 => PositionKey::RAM,
            9 => PositionKey::LF,
            10 => PositionKey::RF,
            _ => PositionKey::CM,
        },
        "4-5-1" | "451" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LB,
            2 => PositionKey::LCB,
            3 => PositionKey::RCB,
            4 => PositionKey::RB,
            5 => PositionKey::LM,
            6 => PositionKey::LCM,
            7 => PositionKey::CM,
            8 => PositionKey::RCM,
            9 => PositionKey::RM,
            10 => PositionKey::ST,
            _ => PositionKey::CM,
        },
        "3-5-2" | "352" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LCB,
            2 => PositionKey::CB,
            3 => PositionKey::RCB,
            4 => PositionKey::LWB,
            5 => PositionKey::LCM,
            6 => PositionKey::CM,
            7 => PositionKey::RCM,
            8 => PositionKey::RWB,
            9 => PositionKey::LF,
            10 => PositionKey::RF,
            _ => PositionKey::CM,
        },
        "3-4-3" | "343" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LCB,
            2 => PositionKey::CB,
            3 => PositionKey::RCB,
            4 => PositionKey::LM,
            5 => PositionKey::LCM,
            6 => PositionKey::RCM,
            7 => PositionKey::RM,
            8 => PositionKey::LW,
            9 => PositionKey::ST,
            10 => PositionKey::RW,
            _ => PositionKey::CM,
        },
        "3-4-2-1" | "3421" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LCB,
            2 => PositionKey::CB,
            3 => PositionKey::RCB,
            4 => PositionKey::LM,
            5 => PositionKey::LCM,
            6 => PositionKey::RCM,
            7 => PositionKey::RM,
            8 => PositionKey::LAM,
            9 => PositionKey::RAM,
            10 => PositionKey::ST,
            _ => PositionKey::CM,
        },
        "3-4-1-2" | "3412" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LCB,
            2 => PositionKey::CB,
            3 => PositionKey::RCB,
            4 => PositionKey::LM,
            5 => PositionKey::LCM,
            6 => PositionKey::RCM,
            7 => PositionKey::RM,
            8 => PositionKey::CAM,
            9 => PositionKey::LF,
            10 => PositionKey::RF,
            _ => PositionKey::CM,
        },
        "5-3-2" | "532" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LWB,
            2 => PositionKey::LCB,
            3 => PositionKey::CB,
            4 => PositionKey::RCB,
            5 => PositionKey::RWB,
            6 => PositionKey::LCM,
            7 => PositionKey::CM,
            8 => PositionKey::RCM,
            9 => PositionKey::LF,
            10 => PositionKey::RF,
            _ => PositionKey::CM,
        },
        "5-4-1" | "541" => match slot {
            0 => PositionKey::GK,
            1 => PositionKey::LWB,
            2 => PositionKey::LCB,
            3 => PositionKey::CB,
            4 => PositionKey::RCB,
            5 => PositionKey::RWB,
            6 => PositionKey::LM,
            7 => PositionKey::LCM,
            8 => PositionKey::RCM,
            9 => PositionKey::RM,
            10 => PositionKey::ST,
            _ => PositionKey::CM,
        },
        _ => panic!("UNSUPPORTED_FORMATION: slot_to_position_key({slot}, {formation})"),
    }
}

/// Fallback position when waypoints not found
pub fn get_fallback_position(slot: usize) -> (f32, f32) {
    let positions = [
        (0.5, 0.04),  // GK
        (0.15, 0.25), // LB
        (0.35, 0.20), // LCB
        (0.65, 0.20), // RCB
        (0.85, 0.25), // RB
        (0.15, 0.50), // LM
        (0.35, 0.45), // LCM
        (0.65, 0.45), // RCM
        (0.85, 0.50), // RM
        (0.35, 0.78), // LF
        (0.65, 0.78), // RF
    ];
    positions.get(slot).copied().unwrap_or((0.5, 0.5))
}

/// Calculate target position based on offensive tactical goal
///
/// FIX_2601/0104: All positions are now in team-relative coordinates
/// Y=0 is always own goal, Y=1 is always opponent goal (for both teams)
pub fn calculate_offensive_target(
    goal: OffensiveGoal,
    wp: &PositionWaypoints,
    ball_pos: (f32, f32),
    player_pos: (f32, f32),
    _is_home: bool, // No longer needed - both teams use same coord system
) -> (f32, f32) {
    // Fallback to enhanced version without opponent data
    calculate_offensive_target_enhanced(goal, wp, ball_pos, player_pos, None, None, None)
}

/// Context for enhanced offensive movement calculation
#[derive(Debug, Clone)]
pub struct OffensiveContext {
    /// Opponent positions (Coord10 format)
    pub opponents: Vec<Coord10>,
    /// Ball holder index (for triangle formation)
    pub ball_holder_idx: Option<usize>,
    /// This player's index
    pub player_idx: usize,
    /// Teammate positions for triangle assignment
    pub teammates: Vec<(usize, Coord10)>,
}

/// Calculate target with enhanced context (channels, triangles, congestion)
///
/// FIX_2601/0110: Uses dynamic build-up system modules
pub fn calculate_offensive_target_enhanced(
    goal: OffensiveGoal,
    wp: &PositionWaypoints,
    ball_pos: (f32, f32),
    player_pos: (f32, f32),
    _opponents: Option<&[Coord10]>,
    _triangle_ctx: Option<(usize, usize, &[(usize, Coord10)])>, // (ball_holder_idx, player_idx, teammates)
    _channel_ctx: Option<(i32, f32)>,                           // (ball_y, attack_direction)
) -> (f32, f32) {
    match goal {
        OffensiveGoal::MoveToBall => {
            // Move directly toward ball position
            ball_pos
        }
        OffensiveGoal::AttackGoal => {
            // FIX_2601/0104: Static waypoint-based attacking target
            // Returns team-relative coordinates (Y=0.85 = attacking position toward opponent goal)
            (wp.offensive.0, 0.85)
        }
        OffensiveGoal::FindSpace => {
            // FIX_2601/0104: Simple space finding (spread from ball position)
            // Returns team-relative coordinates
            let offset_x = if player_pos.0 < ball_pos.0 { -0.10 } else { 0.10 };
            let offset_y = 0.05;
            ((ball_pos.0 + offset_x).clamp(0.05, 0.95), (ball_pos.1 + offset_y).clamp(0.05, 0.95))
        }
        OffensiveGoal::SupportTeammate => {
            // FIX_2601/0104: Simple support positioning (offset from ball)
            // Returns team-relative coordinates
            let support_offset = 0.08;
            let offset_x = if player_pos.0 < ball_pos.0 { -support_offset } else { support_offset };
            let offset_y = if player_pos.1 < ball_pos.1 { -support_offset } else { support_offset };
            ((ball_pos.0 + offset_x).clamp(0.05, 0.95), (ball_pos.1 + offset_y).clamp(0.05, 0.95))
        }
        OffensiveGoal::HoldPosition => {
            // Stay at tactical position
            wp.offensive
        }
    }
}

// =============================================================================
// FIX_2601/0105: Explicit Direction Target Calculation (NO Y-flip!)
// =============================================================================

use super::types::DirectionContext;

/// Calculate offensive target with EXPLICIT direction - NO coordinate flipping!
///
/// Uses DirectionContext to determine attack direction explicitly.
/// All coordinates are in WORLD space (normalized 0.0-1.0).
///
/// - ball_pos: (width, length) in world coords
/// - player_pos: (width, length) in world coords
/// - Returns: target position in WORLD coords
pub fn calculate_offensive_target_explicit(
    goal: OffensiveGoal,
    wp: &PositionWaypoints,
    ball_pos: (f32, f32),
    player_pos: (f32, f32),
    ctx: &DirectionContext,
) -> (f32, f32) {
    match goal {
        OffensiveGoal::MoveToBall => {
            // Move directly toward ball - world coords work directly
            ball_pos
        }
        OffensiveGoal::AttackGoal => {
            // 명시적: 상대 골대 방향으로 이동
            let target_x = wp.offensive.0; // Lateral position from waypoint
            let target_y = if ctx.attacks_right {
                0.85 // Home: toward y=1.0 (away goal)
            } else {
                0.15 // Away: toward y=0.0 (home goal)
            };
            (target_x, target_y)
        }
        OffensiveGoal::FindSpace => {
            // 명시적: 공 위치에서 공격 방향으로 오프셋
            let forward_offset = ctx.forward_offset(0.05);
            let lateral_offset = if player_pos.0 < ball_pos.0 { -0.10 } else { 0.10 };
            (
                (ball_pos.0 + lateral_offset).clamp(0.05, 0.95),
                (ball_pos.1 + forward_offset).clamp(0.05, 0.95),
            )
        }
        OffensiveGoal::SupportTeammate => {
            // 공에서 약간 뒤로 + 옆으로 (support position)
            let back_offset = ctx.forward_offset(-0.08); // 공격 반대 방향
            let lateral_offset = if player_pos.0 < ball_pos.0 { -0.08 } else { 0.08 };
            (
                (ball_pos.0 + lateral_offset).clamp(0.05, 0.95),
                (ball_pos.1 + back_offset).clamp(0.05, 0.95),
            )
        }
        OffensiveGoal::HoldPosition => {
            // 명시적: waypoint를 팀 방향에 맞게 변환
            let target_y = if ctx.attacks_right {
                wp.offensive.1 // Home: use as-is
            } else {
                1.0 - wp.offensive.1 // Away: flip to world coords
            };
            (wp.offensive.0, target_y)
        }
    }
}

// =============================================================================
// FIX_2601/0110: Dynamic Build-up System Integration
// =============================================================================

/// Context for dynamic build-up calculations
///
/// FIX_2601/0110: Provides opponent and teammate data for channel finding
/// and passing triangle formation.
#[derive(Debug, Clone)]
pub struct BuildupContext {
    /// Opponent defender positions (Coord10 format)
    pub opponent_defenders: Vec<Coord10>,
    /// Ball holder index (for triangle formation)
    pub ball_holder_idx: Option<usize>,
    /// This player's index
    pub player_idx: usize,
    /// Teammate positions for triangle assignment (idx, position)
    pub teammates: Vec<(usize, Coord10)>,
}

/// Calculate offensive target with Dynamic Build-up System
///
/// FIX_2601/0110: Enhanced version that uses:
/// - Channel Finder: Find gaps between defenders for AttackGoal
/// - Passing Triangles: Create triangle formations for SupportTeammate
///
/// Falls back to static positions when buildup_ctx is None.
pub fn calculate_offensive_target_with_buildup(
    goal: OffensiveGoal,
    wp: &PositionWaypoints,
    ball_pos: (f32, f32),
    player_pos: (f32, f32),
    ctx: &DirectionContext,
    buildup_ctx: Option<&BuildupContext>,
) -> (f32, f32) {
    match goal {
        OffensiveGoal::MoveToBall => ball_pos,
        OffensiveGoal::AttackGoal => {
            // FIX_2601/0110 Phase 1: Try to find a channel between defenders
            if let Some(buildup) = buildup_ctx {
                if !buildup.opponent_defenders.is_empty() {
                    use crate::engine::match_sim::channel_finder::find_best_channel_for_player;

                    // Convert normalized player position to Coord10
                    let player_coord10 = Coord10::from_normalized_legacy(player_pos);

                    // Ball position in Coord10
                    let ball_x = Coord10::from_normalized_legacy(ball_pos).x;

                    // Attack direction: +1 for home (toward x=1050), -1 for away
                    let attack_dir = if ctx.attacks_right { 1.0 } else { -1.0 };

                    // Find best channel for this player
                    if let Some(channel) = find_best_channel_for_player(
                        player_coord10,
                        &buildup.opponent_defenders,
                        attack_dir,
                        ball_x,
                    ) {
                        // Convert channel center back to normalized coords
                        let (target_x, target_y) = channel.center.to_normalized_legacy();

                        // Add forward progression toward goal
                        let forward_offset = ctx.forward_offset(0.10);
                        let final_y = (target_y + forward_offset).clamp(0.05, 0.95);

                        return (target_x.clamp(0.05, 0.95), final_y);
                    }
                }
            }

            // Fallback: static waypoint-based target
            let target_x = wp.offensive.0;
            let target_y = if ctx.attacks_right { 0.85 } else { 0.15 };
            (target_x, target_y)
        }
        OffensiveGoal::FindSpace => {
            // FIX_2601/0107 Phase 8.4: Use ForwardMovementPattern for intelligent space finding
            if let Some(buildup) = buildup_ctx {
                use crate::engine::match_sim::steering::ForwardMovementPattern;

                // Convert positions to Coord10 for pattern calculation
                let player_coord10 = Coord10::from_normalized_legacy(player_pos);
                let ball_coord10 = Coord10::from_normalized_legacy(ball_pos);

                // Determine if player is a wide player (wings/fullbacks)
                let is_wide = player_pos.0 < 0.25 || player_pos.0 > 0.75;

                // Check if ball holder is under pressure (nearby opponents)
                let ball_holder_under_pressure = buildup.ball_holder_idx.is_some_and(|holder_idx| {
                    let holder_pos = if holder_idx < 22 {
                        buildup.teammates.iter()
                            .find(|(idx, _)| *idx == holder_idx)
                            .map(|(_, pos)| pos)
                            .cloned()
                            .unwrap_or(ball_coord10)
                    } else {
                        ball_coord10
                    };
                    // Check if any opponent is within 10m (100 Coord10 units)
                    buildup.opponent_defenders.iter().any(|opp| {
                        let dx = opp.x - holder_pos.x;
                        let dy = opp.y - holder_pos.y;
                        (dx * dx + dy * dy) < 10000 // 10m = 100 units, squared
                    })
                });

                // Select movement pattern
                let pattern = ForwardMovementPattern::select_pattern(
                    player_coord10,
                    ball_coord10,
                    &buildup.opponent_defenders,
                    is_wide,
                    ball_holder_under_pressure,
                );

                // Calculate target based on pattern
                let attack_dir = if ctx.attacks_right { 1.0 } else { -1.0 };
                let target_coord10 = pattern.calculate_target(
                    player_coord10,
                    ball_coord10,
                    &buildup.opponent_defenders,
                    attack_dir,
                );

                // Convert back to normalized coords
                let (target_x, target_y) = target_coord10.to_normalized_legacy();
                return (
                    target_x.clamp(0.05, 0.95),
                    target_y.clamp(0.05, 0.95),
                );
            }

            // Fallback: simple offset
            let forward_offset = ctx.forward_offset(0.05);
            let lateral_offset = if player_pos.0 < ball_pos.0 { -0.10 } else { 0.10 };
            (
                (ball_pos.0 + lateral_offset).clamp(0.05, 0.95),
                (ball_pos.1 + forward_offset).clamp(0.05, 0.95),
            )
        }
        OffensiveGoal::SupportTeammate => {
            // FIX_2601/0110 Phase 2: Use passing triangle formation
            if let Some(buildup) = buildup_ctx {
                if let Some(holder_idx) = buildup.ball_holder_idx {
                    if !buildup.teammates.is_empty() {
                        use crate::engine::match_sim::passing_triangles::{
                            PassingTriangle, TriangleAssignment,
                        };

                        // Ball position in Coord10
                        let ball_coord10 = Coord10::from_normalized_legacy(ball_pos);

                        // Attack direction: +1 for home, -1 for away
                        let attack_dir = if ctx.attacks_right { 1.0 } else { -1.0 };

                        // Calculate triangle
                        let triangle =
                            PassingTriangle::calculate(ball_coord10, attack_dir, ball_coord10.y);

                        // Assign players to triangle vertices
                        let assignment =
                            TriangleAssignment::assign(&triangle, holder_idx, &buildup.teammates);

                        // Get target for this player
                        if let Some(target) =
                            assignment.get_target_for_player(buildup.player_idx, &triangle)
                        {
                            // Convert back to normalized coords
                            let (target_x, target_y) = target.to_normalized_legacy();
                            return (
                                target_x.clamp(0.05, 0.95),
                                target_y.clamp(0.05, 0.95),
                            );
                        }
                    }
                }
            }

            // Fallback: simple offset from ball
            let back_offset = ctx.forward_offset(-0.08);
            let lateral_offset = if player_pos.0 < ball_pos.0 { -0.08 } else { 0.08 };
            (
                (ball_pos.0 + lateral_offset).clamp(0.05, 0.95),
                (ball_pos.1 + back_offset).clamp(0.05, 0.95),
            )
        }
        OffensiveGoal::HoldPosition => {
            let target_y = if ctx.attacks_right { wp.offensive.1 } else { 1.0 - wp.offensive.1 };
            (wp.offensive.0, target_y)
        }
    }
}

/// Calculate defensive target with EXPLICIT direction - NO coordinate flipping!
///
/// Uses DirectionContext to determine defensive positioning explicitly.
/// All coordinates are in WORLD space.
pub fn calculate_defensive_target_explicit(
    goal: DefensiveGoal,
    wp: &PositionWaypoints,
    ball_pos: (f32, f32),
    _player_pos: (f32, f32),
    ctx: &DirectionContext,
) -> (f32, f32) {
    match goal {
        DefensiveGoal::PressOpponent => {
            // Press toward ball - world coords work directly
            ball_pos
        }
        DefensiveGoal::TrackBack => {
            // 명시적: 자기 골대 방향으로 복귀
            let target_y = if ctx.attacks_right {
                wp.defensive.1 // Home: low y = toward own goal (y=0)
            } else {
                1.0 - wp.defensive.1 // Away: high y = toward own goal (y=1)
            };
            (wp.defensive.0, target_y)
        }
        DefensiveGoal::MarkPlayer => {
            // 공과 자기 골대 사이에 위치
            let own_goal_y = ctx.own_goal_x();
            let mark_y = (ball_pos.1 + own_goal_y) / 2.0;
            (ball_pos.0, mark_y.clamp(0.05, 0.95))
        }
        DefensiveGoal::BlockPassingLane => {
            // 공과 자기 골대 사이 30% 지점
            let own_goal_y = ctx.own_goal_x();
            let block_y = ball_pos.1 + (own_goal_y - ball_pos.1) * 0.3;
            (ball_pos.0, block_y.clamp(0.05, 0.95))
        }
        DefensiveGoal::HoldPosition => {
            // 명시적: waypoint를 팀 방향에 맞게 변환
            let target_y = if ctx.attacks_right {
                wp.defensive.1 // Home: use as-is
            } else {
                1.0 - wp.defensive.1 // Away: flip to world coords
            };
            (wp.defensive.0, target_y)
        }
    }
}

// =============================================================================
// FIX_2601/0109: Space Creation Target Calculation
// =============================================================================

use crate::engine::types::Coord10;

/// Calculate target position based on space creation type
///
/// Each space type has specific movement patterns:
/// - HalfSpace: Move to 1/3 or 2/3 width corridors
/// - BetweenLines: Find gap between opponent MF and DEF (~Y=0.50)
/// - WideOverload: Move to wing on ball side
/// - DeepPocket: Move to attacking third with lateral offset
/// - ThirdManRun: Sprint beyond immediate threat zone
pub fn calculate_space_target(
    space_type: SpaceCreationType,
    wp: &PositionWaypoints,
    ball_pos: (f32, f32),
    player_pos: (f32, f32),
    is_away: bool,
) -> (f32, f32) {
    // Fallback without opponent data
    calculate_space_target_with_opponents(space_type, wp, ball_pos, player_pos, None, is_away)
}

/// Calculate target position with optional opponent awareness
///
/// FIX_2601/0110: Enhanced with real-time congestion calculation for DeepPocket
/// Note: ball_pos and player_pos are in TEAM-RELATIVE coords, opponents is in WORLD Coord10
pub fn calculate_space_target_with_opponents(
    space_type: SpaceCreationType,
    wp: &PositionWaypoints,
    ball_pos: (f32, f32),
    player_pos: (f32, f32),
    opponents: Option<&[Coord10]>,
    is_away: bool,
) -> (f32, f32) {
    match space_type {
        SpaceCreationType::HalfSpace => {
            // Move to half-space zone (1/3 or 2/3 width)
            let hs_x = if player_pos.0 < 0.5 { 0.33 } else { 0.67 };
            let hs_y = (wp.offensive.1 + 0.05).min(0.90);
            (hs_x, hs_y)
        }
        SpaceCreationType::BetweenLines => {
            // Position in gap between opponent MF and DEF lines
            // Typically around Y=0.45-0.55 in attacking half
            let between_y = 0.50;
            let between_x = wp.offensive.0; // Keep lateral position
            (between_x, between_y)
        }
        SpaceCreationType::WideOverload => {
            // Move wide on ball side for numerical advantage
            let wing_x = if ball_pos.0 < 0.5 { 0.10 } else { 0.90 };
            let wing_y = (ball_pos.1 + 0.10).min(0.90);
            (wing_x, wing_y)
        }
        SpaceCreationType::DeepPocket => {
            // FIX_2601/0110: Use congestion calculation if opponents available
            if let Some(opps) = opponents {
                use crate::engine::match_sim::congestion::find_low_congestion_position;

                // CRITICAL: ball_pos is in TEAM-RELATIVE coords
                // opponents (opps) is in WORLD Coord10 coords
                // Convert ball_pos to WORLD coords for consistent search
                let ball_length_world = if is_away { 1.0 - ball_pos.1 } else { ball_pos.1 };

                // For Away team attacking toward x=0, "forward" means DECREASING x
                let forward_offset_world = if is_away { -0.15 } else { 0.15 };
                let search_length_world =
                    (ball_length_world + forward_offset_world).clamp(0.05, 0.95);
                let search_center =
                    Coord10::from_normalized_legacy((player_pos.0, search_length_world));

                let best_pos = find_low_congestion_position(
                    search_center,
                    150, // 15m search radius
                    opps,
                    5, // 5x5 sample grid
                );

                // best_pos is in WORLD Coord10 coords
                let (target_width, target_length_world) = best_pos.to_normalized_legacy();

                // Convert back to team-relative for Away team
                let target_length =
                    if is_away { 1.0 - target_length_world } else { target_length_world };

                (target_width, target_length)
            } else {
                // Fallback: static offset (in team-relative coords)
                let offset_x = if player_pos.0 < ball_pos.0 { -0.15 } else { 0.15 };
                let pocket_x = (ball_pos.0 + offset_x).clamp(0.10, 0.90);
                let pocket_y = (ball_pos.1 + 0.12).min(0.88);
                (pocket_x, pocket_y)
            }
        }
        SpaceCreationType::ThirdManRun => {
            // Sprint beyond the immediate pass recipient toward goal
            // Position deep (Y=0.85) with slight lateral movement
            let run_x = wp.offensive.0;
            let run_y = 0.85;
            (run_x, run_y)
        }
    }
}

/// Calculate target position based on defensive tactical goal
///
/// FIX_2601/0104: All positions are now in team-relative coordinates
/// Y=0 is always own goal, Y=1 is always opponent goal (for both teams)
pub fn calculate_defensive_target(
    goal: DefensiveGoal,
    wp: &PositionWaypoints,
    ball_pos: (f32, f32),
    _player_pos: (f32, f32),
    _is_home: bool, // No longer needed - both teams use same coord system
) -> (f32, f32) {
    match goal {
        DefensiveGoal::PressOpponent => {
            // Move aggressively toward ball carrier
            ball_pos
        }
        DefensiveGoal::TrackBack => {
            // Sprint back to defensive position
            wp.defensive
        }
        DefensiveGoal::MarkPlayer => {
            // For now, mark defensive area (future: track specific opponent)
            // FIX_2601/0104: In team-relative coords, Y=0.05 is always near own goal
            // Position between ball and own goal
            let mark_x = ball_pos.0;
            let mark_y = (ball_pos.1 + 0.05) / 2.0;
            (mark_x, mark_y)
        }
        DefensiveGoal::BlockPassingLane => {
            // Position between ball and own goal to intercept passes
            // FIX_2601/0104: In team-relative coords, Y=0.05 is always near own goal
            let block_x = ball_pos.0;
            let block_y = ball_pos.1 + (0.05 - ball_pos.1) * 0.3;
            (block_x, block_y)
        }
        DefensiveGoal::HoldPosition => {
            // Maintain defensive shape
            wp.defensive
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_role_classification() {
        assert_eq!(get_position_role(PositionKey::GK), PositionRole::Goalkeeper);
        assert_eq!(get_position_role(PositionKey::CB), PositionRole::Defender);
        assert_eq!(get_position_role(PositionKey::CDM), PositionRole::Defender);
        assert_eq!(get_position_role(PositionKey::CM), PositionRole::Midfielder);
        assert_eq!(get_position_role(PositionKey::CAM), PositionRole::Midfielder);
        assert_eq!(get_position_role(PositionKey::ST), PositionRole::Forward);
        assert_eq!(get_position_role(PositionKey::LW), PositionRole::Forward);
    }

    #[test]
    fn test_slot_to_position_442() {
        assert_eq!(slot_to_position_key(0, "4-4-2"), PositionKey::GK);
        assert_eq!(slot_to_position_key(1, "4-4-2"), PositionKey::LB);
        assert_eq!(slot_to_position_key(9, "4-4-2"), PositionKey::LF);
        assert_eq!(slot_to_position_key(10, "4-4-2"), PositionKey::RF);
    }

    #[test]
    fn test_slot_to_position_433() {
        assert_eq!(slot_to_position_key(0, "4-3-3"), PositionKey::GK);
        assert_eq!(slot_to_position_key(8, "4-3-3"), PositionKey::LW);
        assert_eq!(slot_to_position_key(9, "4-3-3"), PositionKey::ST);
        assert_eq!(slot_to_position_key(10, "4-3-3"), PositionKey::RW);
    }

    #[test]
    fn test_fallback_position() {
        let gk_pos = get_fallback_position(0);
        assert!((gk_pos.0 - 0.5).abs() < 0.01);
        assert!(gk_pos.1 < 0.1); // GK near goal line

        let invalid_pos = get_fallback_position(99);
        assert_eq!(invalid_pos, (0.5, 0.5)); // Default center
    }

    // =========================================================================
    // FIX_2601/0109: Space Target Tests
    // =========================================================================

    #[test]
    fn test_space_target_half_space() {
        let wp = PositionWaypoints {
            base: (0.4, 0.5),
            offensive: (0.4, 0.6),
            defensive: (0.4, 0.3),
            left_shift: (0.3, 0.5),
            right_shift: (0.5, 0.5),
        };
        let ball = (0.5, 0.5);

        // Left side player -> moves to left half-space (0.33)
        let left =
            calculate_space_target(SpaceCreationType::HalfSpace, &wp, ball, (0.3, 0.5), false);
        assert!((left.0 - 0.33).abs() < 0.01);

        // Right side player -> moves to right half-space (0.67)
        let right =
            calculate_space_target(SpaceCreationType::HalfSpace, &wp, ball, (0.7, 0.5), false);
        assert!((right.0 - 0.67).abs() < 0.01);
    }

    #[test]
    fn test_space_target_between_lines() {
        let wp = PositionWaypoints {
            base: (0.4, 0.5),
            offensive: (0.4, 0.6),
            defensive: (0.4, 0.3),
            left_shift: (0.3, 0.5),
            right_shift: (0.5, 0.5),
        };
        let ball = (0.5, 0.5);
        let player = (0.4, 0.45);

        let target =
            calculate_space_target(SpaceCreationType::BetweenLines, &wp, ball, player, false);
        assert!((target.1 - 0.50).abs() < 0.01); // Y should be ~0.50
        assert!((target.0 - 0.40).abs() < 0.01); // X from waypoint
    }

    #[test]
    fn test_space_target_wide_overload() {
        let wp = PositionWaypoints {
            base: (0.2, 0.5),
            offensive: (0.2, 0.6),
            defensive: (0.2, 0.3),
            left_shift: (0.1, 0.5),
            right_shift: (0.3, 0.5),
        };
        let player = (0.2, 0.6);

        // Ball on left side -> wing at 0.10
        let left =
            calculate_space_target(SpaceCreationType::WideOverload, &wp, (0.3, 0.5), player, false);
        assert!((left.0 - 0.10).abs() < 0.01);

        // Ball on right side -> wing at 0.90
        let right =
            calculate_space_target(SpaceCreationType::WideOverload, &wp, (0.7, 0.5), player, false);
        assert!((right.0 - 0.90).abs() < 0.01);
    }

    #[test]
    fn test_space_target_deep_pocket() {
        let wp = PositionWaypoints {
            base: (0.5, 0.5),
            offensive: (0.5, 0.7),
            defensive: (0.5, 0.3),
            left_shift: (0.4, 0.5),
            right_shift: (0.6, 0.5),
        };
        let ball = (0.5, 0.6);

        // Player on left of ball -> offset left
        let left =
            calculate_space_target(SpaceCreationType::DeepPocket, &wp, ball, (0.3, 0.6), false);
        assert!(left.0 < ball.0); // Should be to left of ball
        assert!(left.1 > ball.1); // Should be ahead of ball

        // Player on right of ball -> offset right
        let right =
            calculate_space_target(SpaceCreationType::DeepPocket, &wp, ball, (0.7, 0.6), false);
        assert!(right.0 > ball.0); // Should be to right of ball
    }

    #[test]
    fn test_space_target_third_man_run() {
        let wp = PositionWaypoints {
            base: (0.5, 0.5),
            offensive: (0.5, 0.7),
            defensive: (0.5, 0.3),
            left_shift: (0.4, 0.5),
            right_shift: (0.6, 0.5),
        };
        let ball = (0.5, 0.5);
        let player = (0.5, 0.6);

        let target =
            calculate_space_target(SpaceCreationType::ThirdManRun, &wp, ball, player, false);
        assert!((target.1 - 0.85).abs() < 0.01); // Deep run Y=0.85
        assert!((target.0 - 0.50).abs() < 0.01); // Keep lateral from waypoint
    }
}
