//! Set Piece Restart Helpers (Goal Kick, Throw-In)
//!
//! This module contains restart helpers used by MatchEngine:
//! - Goal kick execution with build-up style selection
//! - Throw-in target selection
//!
//! Set piece FSM logic for corner/free/penalty lives in tick_based.rs
//! (phase_action/set_piece.rs).

use super::MatchEngine;
use crate::engine::physics_constants::{field, skills};
use crate::engine::types::coord10::Coord10; // FIX_2512
use crate::models::MatchEvent;
use rand::Rng;

impl MatchEngine {
    // ===========================================
    // Restart Helpers (Goal Kick, Throw-In)
    // ===========================================

    /// Find nearest attacker
    /// FIX_2601: Updated to use Coord10.distance_to_m()
    pub(crate) fn find_nearest_attacker(&self, from_idx: usize, is_home: bool) -> Option<usize> {
        let (start_idx, end_idx) = if is_home { (0, 11) } else { (11, 22) };
        let from_pos = self.player_positions[from_idx];

        let mut nearest_idx = None;
        let mut min_dist = f32::MAX;

        for idx in start_idx..end_idx {
            if idx == from_idx {
                continue;
            }

            let pos = self.player_positions[idx];
            let dist = from_pos.distance_to_m(&pos);

            if dist < min_dist {
                min_dist = dist;
                nearest_idx = Some(idx);
            }
        }

        nearest_idx
    }

    // ===========================================
    // P5.4: Strategic Restart System
    // ===========================================

    /// Execute goal kick with tactical build-up style
    pub(crate) fn execute_goal_kick(&mut self, gk_idx: usize, is_home: bool) {
        use crate::tactics::team_instructions::BuildUpStyle;

        self.emit_event(MatchEvent::goal_kick(
            self.minute,
            self.current_timestamp_ms(),
            is_home,
            gk_idx,
        ));

        // Get team's build-up style
        let build_up = if is_home {
            self.home_instructions.build_up_style
        } else {
            self.away_instructions.build_up_style
        };

        // Position ball at goal area
        // FIX_2601/0116: Use attacks_right for goal position (not is_home)
        // If attacking right, defending goal is at x=5.25 (left side)
        // If attacking left, defending goal is at x=99.75 (right side)
        let attacks_right = self.attacks_right(is_home);
        self.ball.position = if attacks_right {
            Coord10::from_meters(5.25, field::CENTER_Y)
        } else {
            Coord10::from_meters(99.75, field::CENTER_Y)
        };
        self.ball.current_owner = Some(gk_idx);

        // Decide based on build-up style
        match build_up {
            BuildUpStyle::Short => {
                // Short goal kick: pass to nearest defender
                if let Some(target) = self.find_nearest_defender_for_goal_kick(gk_idx, is_home) {
                    self.ball.current_owner = Some(target);
                    // FIX_2601: target_pos는 Coord10, to_meters() 사용
                    let target_pos = self.player_positions[target];
                    let target_m = target_pos.to_meters();
                    let current_m = self.ball.position.to_meters();
                    let mid_x = (current_m.0 + target_m.0) / 2.0;
                    let mid_y = target_m.1;
                    self.ball.position = Coord10::from_meters(mid_x, mid_y);
                }
            }
            BuildUpStyle::Direct => {
                // Long goal kick: launch towards midfield/attack
                self.execute_long_goal_kick(gk_idx, is_home);
            }
            BuildUpStyle::Mixed => {
                // 60% short, 40% long
                if self.rng.gen::<f32>() < 0.6 {
                    if let Some(target) = self.find_nearest_defender_for_goal_kick(gk_idx, is_home)
                    {
                        self.ball.current_owner = Some(target);
                        // FIX_2601: target_pos는 Coord10, to_meters() 사용
                        let target_pos = self.player_positions[target];
                        let target_m = target_pos.to_meters();
                        let current_m = self.ball.position.to_meters();
                        let mid_x = (current_m.0 + target_m.0) / 2.0;
                        let mid_y = target_m.1;
                        self.ball.position = Coord10::from_meters(mid_x, mid_y);
                    }
                } else {
                    self.execute_long_goal_kick(gk_idx, is_home);
                }
            }
        }
    }

    /// Find nearest defender for short goal kick
    /// FIX_2601: Updated to use Coord10.distance_to_m()
    fn find_nearest_defender_for_goal_kick(&self, gk_idx: usize, is_home: bool) -> Option<usize> {
        let (start_idx, end_idx) = if is_home { (1, 5) } else { (12, 16) }; // Defenders only
        let gk_pos = self.player_positions[gk_idx];

        let mut nearest_idx = None;
        let mut min_dist = f32::MAX;

        for idx in start_idx..end_idx {
            let pos = self.player_positions[idx];
            let dist = gk_pos.distance_to_m(&pos);

            if dist < min_dist {
                min_dist = dist;
                nearest_idx = Some(idx);
            }
        }

        nearest_idx
    }

    /// Execute long goal kick towards midfield
    fn execute_long_goal_kick(&mut self, gk_idx: usize, is_home: bool) {
        // Target midfield area
        // FIX_2601/0116: Use attacks_right for forward direction
        let attacks_right = self.attacks_right(is_home);
        // FIX_2601/1128: Target midfield instead of attacking half
        let target_x = if attacks_right {
            0.45 + self.rng.gen::<f32>() * 0.15 // 0.45 ~ 0.60 (midfield)
        } else {
            0.40 + self.rng.gen::<f32>() * 0.15 // 0.40 ~ 0.55 (midfield)
        };
        let target_y = 0.3 + self.rng.gen::<f32>() * 0.4; // 0.3 ~ 0.7

        // Find nearest teammate to target position
        let (start_idx, end_idx) = if is_home { (0, 11) } else { (11, 22) };

        let mut best_target = None;
        let mut min_dist = f32::MAX;

        for idx in start_idx..end_idx {
            if idx == gk_idx {
                continue;
            }

            // FIX_2601/0104: pos는 Coord10, normalized로 변환
            // to_normalized_legacy() returns (width, length)
            // target_x is length direction, target_y is width direction
            let pos = self.player_positions[idx];
            let pos_norm = pos.to_normalized_legacy();
            let dist = ((pos_norm.1 - target_x).powi(2) + (pos_norm.0 - target_y).powi(2)).sqrt();

            if dist < min_dist {
                min_dist = dist;
                best_target = Some(idx);
            }
        }

        // Long kick success rate based on GK kicking attribute (using strength as proxy)
        let kick_skill = skills::normalize(self.get_player_strength(gk_idx));
        let success = self.rng.gen::<f32>() < (0.5 + kick_skill * 0.3);

        if success {
            if let Some(target) = best_target {
                self.ball.current_owner = Some(target);
                // FIX_2601: player_positions는 Coord10
                let target_pos = self.player_positions[target];
                self.ball.position = target_pos; // 이미 Coord10
            }
        } else {
            // Long kick intercepted - loose ball or opponent gets it
            self.ball.current_owner = None;
            // target_x, target_y는 normalized (0-1)
            self.ball.position = Coord10::from_normalized((target_x, target_y));
        }
    }

    /// Execute throw-in with strategic target selection
    ///
    /// FIX_2601/0123: long_throws 속성 연동
    /// - long_throws가 높으면 최대 투구 거리 증가 (15m → 최대 40m)
    /// - long_throws가 높으면 투구 정확도 증가
    pub(crate) fn execute_throw_in(&mut self, touch_line_pos: (f32, f32), is_home: bool) {
        // FIX_2601/0123: 스로어 선택 (스로인 위치에서 가장 가까운 선수)
        let throw_x = (touch_line_pos.0 * field::LENGTH_M).clamp(5.25, 99.75);
        let throw_y = if touch_line_pos.1 <= 0.01 { 3.4 } else { 64.6 };
        let throw_pos = Coord10::from_meters(throw_x, throw_y);

        let (start_idx, end_idx) = if is_home { (0, 11) } else { (11, 22) };

        // 스로어 선택: 스로인 위치에서 가장 가까운 선수
        let mut thrower_idx = start_idx;
        let mut min_dist = f32::MAX;
        for idx in start_idx..end_idx {
            let dist = throw_pos.distance_to_m(&self.player_positions[idx]);
            if dist < min_dist {
                min_dist = dist;
                thrower_idx = idx;
            }
        }

        // FIX_2601/0123: long_throws 속성으로 최대 투구 거리 결정
        // 기본 15m, long_throws가 높으면 최대 40m (롱스로 스페셜리스트)
        let long_throws = self.get_player_long_throws(thrower_idx);
        let long_throws_norm = skills::normalize(long_throws);
        // long_throws 1 → 15m, long_throws 20 → 40m
        let max_throw_distance = 15.0 + long_throws_norm * 25.0;

        self.ball.position = throw_pos;

        // FIX_2601/0109: Use attacks_right for forward direction calculation
        let attacks_right = self.attacks_right(is_home);

        let mut best_target = None;
        let mut best_score = f32::MIN;

        for idx in start_idx..end_idx {
            if idx == thrower_idx {
                continue; // 스로어 자신은 제외
            }

            // FIX_2601: pos는 Coord10, to_meters()로 미터 좌표 획득
            let pos = self.player_positions[idx];
            let pos_m = pos.to_meters();
            let (px, py) = pos_m;
            let dist = ((px - throw_x).powi(2) + (py - throw_y).powi(2)).sqrt();

            // FIX_2601/0123: 동적 최대 투구 거리 사용
            if dist > max_throw_distance {
                continue;
            }

            // FIX_2601/1128: Remove forward bias - prefer open players instead
            // Score: prefer players with more space (fewer nearby opponents)
            let space_score = {
                let opponent_range = if is_home { 11..22 } else { 0..11 };
                let mut nearby_opponents = 0;
                for opp_idx in opponent_range {
                    let opp_pos = self.player_positions[opp_idx];
                    let opp_m = opp_pos.to_meters();
                    let opp_dist = ((opp_m.0 - px).powi(2) + (opp_m.1 - py).powi(2)).sqrt();
                    if opp_dist < 8.0 {
                        nearby_opponents += 1;
                    }
                }
                match nearby_opponents {
                    0 => 1.0,
                    1 => 0.7,
                    2 => 0.4,
                    _ => 0.2,
                }
            };
            // FIX_2601/0123: 동적 최대 거리 사용
            let score = space_score - (dist / max_throw_distance) * 0.3;

            if score > best_score {
                best_score = score;
                best_target = Some(idx);
            }
        }

        let receiver_idx = if let Some(target) = best_target {
            target
        } else {
            self.find_nearest_attacker(thrower_idx, is_home)
                .unwrap_or(if is_home { 0 } else { 11 })
        };

        // FIX_2601/0106 Phase 1.2: Model throw-in as a short deterministic delivery segment.
        //
        // This creates an explicit arrival tick (Trap surface) so we can enforce
        // rulebook GK handling restrictions on throw-ins without routing through PassStarted
        // (offside exemptions remain structural).
        use crate::engine::action_queue::{ActionType, BallState, InFlightOrigin};
        use crate::engine::ball::{max_height_from_profile, HeightProfile};
        use crate::engine::types::Vel10;

        let receiver_pos = self.player_positions[receiver_idx];
        let receiver_pos_m = receiver_pos.to_meters();
        let dist_m = ((receiver_pos_m.0 - throw_x).powi(2) + (receiver_pos_m.1 - throw_y).powi(2)).sqrt();
        // FIX_2601/0123: 동적 최대 거리 사용
        let dist01 = (dist_m / max_throw_distance).clamp(0.0, 1.0);

        let start_tick = self.current_tick;
        let arrival_tick = self.current_tick + 1;

        // FIX_2601/0123: long_throws 속성으로 투구 품질 결정
        // - 높은 long_throws: 더 빠르고 정확한 투구
        // - 낮은 long_throws: 느리고 부정확한 투구 (상대가 가로채기 쉬움)
        let height_profile = HeightProfile::Arc;
        let lift_ratio = (0.30 + dist01 * 0.25).clamp(0.0, 1.0);
        let ball_height = max_height_from_profile(height_profile, lift_ratio);
        // FIX_2601/0123: long_throws가 높을수록 빠른 투구 (가로채기 어려움)
        let base_speed = 8.0 + dist01 * 6.0; // 8..14 m/s (기본)
        let speed_bonus = long_throws_norm * 4.0; // 최대 +4 m/s
        let ball_speed = base_speed + speed_bonus;

        self.action_queue.set_ball_state(BallState::InFlight {
            from_pos: Coord10::from_meters(throw_x, throw_y),
            to_pos: receiver_pos,
            start_tick,
            end_tick: arrival_tick,
            height_profile,
            lift_ratio,
            intended_receiver: Some(receiver_idx),
            is_shot: false,
            start_height_01m: 0,
            end_height_01m: 0,
        });
        self.action_queue
            .set_in_flight_origin(InFlightOrigin::ThrowIn { throwing_home: is_home });
        self.action_queue.schedule_new(
            arrival_tick,
            ActionType::Trap { ball_speed, ball_height },
            receiver_idx,
            100,
        );

        // Keep legacy Ball struct consistent for the next tick's sync step.
        self.ball.position = Coord10::from_meters(throw_x, throw_y);
        self.ball.velocity = Vel10::default();
        self.ball.velocity_z = 0;
        self.ball.height = 0;
        self.ball.current_owner = None;
        self.ball.pending_owner = Some(receiver_idx);
        self.ball.from_position = Some(Coord10::from_meters(throw_x, throw_y));
        self.ball.to_position = Some(receiver_pos);
        self.ball.flight_progress = 0.0;
        self.ball.flight_speed = ball_speed;
        self.ball.is_in_flight = true;
        self.ball.height_profile = height_profile;
        self.ball.flight_start_height_01m = 0;
        self.ball.flight_end_height_01m = 0;
        self.emit_event(MatchEvent::throw_in(
            self.minute,
            self.current_timestamp_ms(),
            is_home,
            receiver_idx,
        ));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_execute_throw_in_creates_delivery_segment_and_trap_surface() {
        use crate::engine::action_queue::{ActionType, BallState, InFlightOrigin};
        use crate::engine::match_sim::test_fixtures::create_test_engine;
        use crate::engine::types::coord10::Coord10;
        use crate::engine::physics_constants::field;

        let mut engine = create_test_engine();
        engine.current_tick = 10;
        engine.action_queue.clear();

        // Normalized touchline position (x along length, y selects lower/upper sideline).
        let touch_line_pos = (0.5, 0.0); // ~midfield, lower sideline
        engine.execute_throw_in(touch_line_pos, true);

        let expected_throw_x = (touch_line_pos.0 * field::LENGTH_M).clamp(5.25, 99.75);
        let expected_throw_y = 3.4;

        let expected_receiver_idx = match engine.action_queue.ball_state() {
            BallState::InFlight {
                from_pos,
                to_pos,
                start_tick,
                end_tick,
                intended_receiver,
                is_shot,
                ..
            } => {
                assert_eq!(*start_tick, 10);
                assert_eq!(*end_tick, 11);
                assert!(!*is_shot);
                assert_eq!(
                    *from_pos,
                    Coord10::from_meters(expected_throw_x, expected_throw_y)
                );
                let receiver_idx = intended_receiver.expect("expected an intended receiver");
                assert_eq!(*to_pos, engine.player_positions[receiver_idx]);
                receiver_idx
            }
            other => panic!("expected InFlight after throw-in, got {:?}", other),
        };

        assert_eq!(
            engine.action_queue.in_flight_origin,
            Some(InFlightOrigin::ThrowIn { throwing_home: true })
        );

        let next = engine
            .action_queue
            .peek_next()
            .expect("expected a scheduled Trap at arrival tick");
        assert_eq!(next.execute_tick, 11);
        assert_eq!(next.player_idx, expected_receiver_idx);
        assert!(matches!(next.action_type, ActionType::Trap { .. }));
    }
}
