//! Cross and Through Ball System
//!
//! This module contains cross and through ball logic for MatchEngine:
//! - Cross action execution
//! - Through ball action execution
//!
//! Extracted from match_sim/mod.rs for better organization.

use super::pitch_zone;
use super::zone_transition;
use super::MatchEngine;
use crate::engine::actions::{self, CrossContext, CrossRolls, CrossTarget};
use crate::engine::types::ThroughBallResult;
use crate::models::trait_system::TraitId;
use crate::models::MatchEvent;
use rand::Rng;

impl MatchEngine {
    // ===========================================
    // Cross and Through Ball System
    // ===========================================

    /// Cross execution
    /// 크로스 실행 (순수 함수 패턴 적용)
    pub(crate) fn execute_cross_action(&mut self, from_idx: usize, is_home: bool) {
        // FIX_2601: Store both Coord10 and normalized versions
        let from_pos_coord = self.player_positions[from_idx];
        let from_pos = from_pos_coord.to_normalized_legacy();
        let from_pos_m = from_pos_coord.to_meters();
        // FIX_2601: Use proper attack direction (accounts for halftime)
        let attacks_right = self.attacks_right(is_home);
        let instructions = if is_home { &self.home_instructions } else { &self.away_instructions };
        let style = zone_transition::ZoneTransitionStyle::from_instructions(instructions);
        let from_zone = pitch_zone::zone_of_position(from_pos_m.0, from_pos_m.1, attacks_right);

        // ============================================
        // 1. 크로스 타겟 리스트 생성 (공격수들)
        // ============================================
        let forward_range = if is_home { 8..11 } else { 19..22 };
        let mut targets: Vec<CrossTarget> = Vec::new();

        for idx in forward_range {
            // FIX_2601: Convert Coord10 to normalized/meters
            let target_pos_coord = self.player_positions[idx];
            let target_pos = target_pos_coord.to_normalized_legacy();
            let target_pos_m = target_pos_coord.to_meters();
            let to_zone =
                pitch_zone::zone_of_position(target_pos_m.0, target_pos_m.1, attacks_right);
            let zone_weight = zone_transition::cross_factor(style, from_zone, to_zone);
            targets.push(CrossTarget {
                idx,
                name: self.get_player_name(idx),
                heading: self.get_player_heading(idx),
                jumping: self.get_player_jumping(idx),
                position: target_pos,
                zone_weight,
            });
        }

        // 타겟이 없으면 실패
        if targets.is_empty() {
            self.assign_possession_to_nearest_defender(is_home);
            return;
        }

        // 거리 계산 (첫 번째 타겟 기준)
        // FIX_2601: Use meters for distance calculation
        let first_target_pos_coord = self.player_positions[targets[0].idx];
        let distance_m = from_pos_coord.distance_to_m(&first_target_pos_coord);

        // ============================================
        // 2. CrossContext 생성
        // ============================================
        let ctx = CrossContext {
            crosser_idx: from_idx,
            crosser_name: self.get_player_name(from_idx),
            is_home,
            crosser_pos: from_pos,
            crossing: self.get_player_crossing(from_idx),
            technique: self.get_player_technique(from_idx),
            vision: self.get_player_vision(from_idx),
            distance_m,
            targets,
            has_crosser: self.player_has_gold_trait(from_idx, TraitId::Crosser),
        };

        // ============================================
        // 3. CrossRolls 생성
        // ============================================
        let rolls = CrossRolls { accuracy_roll: self.rng.gen(), header_roll: self.rng.gen() };

        // ============================================
        // 4. 순수 함수 호출
        // ============================================
        let result = actions::resolve_cross(&ctx, &rolls);

        // ============================================
        // 5. 결과 적용
        // ============================================
        if result.successful_delivery {
            if let Some(target_idx) = result.target_reached {
                // FIX_2601/1120: Update ball position to target's position to prevent teleportation
                self.ball.current_owner = Some(target_idx);
                self.ball.position = self.player_positions[target_idx];
            }
        } else {
            self.assign_possession_to_nearest_defender(is_home);
        }
    }

    /// Through ball execution (ability-based offside test)
    pub(crate) fn execute_through_ball_action(&mut self, from_idx: usize, is_home: bool) {
        let valid_targets = self.find_valid_pass_targets(from_idx, is_home);

        // 공격수 중에서 선택 (슬롯 8-10)
        let forward_targets: Vec<usize> = valid_targets
            .into_iter()
            .filter(|&i| {
                let slot = if is_home { i } else { i - 11 };
                slot >= 8 // 공격수 슬롯
            })
            .collect();

        if forward_targets.is_empty() {
            self.assign_possession_to_nearest_defender(is_home);
            return;
        }

        // 6-Factor로 최적 타겟 선택
        let target_idx = {
            let mut best = forward_targets[0];
            let mut best_score = 0.0;
            for &t in &forward_targets {
                let score = self.calculate_pass_score_6factor(from_idx, t, is_home);
                if score > best_score {
                    best_score = score;
                    best = t;
                }
            }
            best
        };

        // 능력치 기반 쓰루패스 시도
        let result = self.attempt_through_ball_with_abilities(from_idx, target_idx, is_home);

        match result {
            ThroughBallResult::Success => {
                // 성공: 공격수에게 소유권 전환
                // FIX_2601/1120: Update ball position to target's position to prevent teleportation
                self.ball.current_owner = Some(target_idx);
                self.ball.position = self.player_positions[target_idx];
            }
            ThroughBallResult::Offside | ThroughBallResult::OffsideTrap => {
                // 오프사이드: 이벤트 emit + 수비팀에게 소유권
                // C6: Use target_idx directly as track_id
                let event = MatchEvent::offside(
                    self.current_minute(),
                    self.current_timestamp_ms(),
                    is_home,
                    target_idx,
                );
                self.emit_event(event);
                let receiver_pos = self.get_player_position_by_index(target_idx);
                self.apply_offside_restart(is_home, receiver_pos);
            }
            ThroughBallResult::Intercepted => {
                // 인터셉트: 수비팀에게 소유권
                self.assign_possession_to_nearest_defender(is_home);
            }
            ThroughBallResult::BadPass | ThroughBallResult::BadTiming => {
                // 실패: 수비팀에게 소유권
                self.assign_possession_to_nearest_defender(is_home);
            }
        }
    }
}
