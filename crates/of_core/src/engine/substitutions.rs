//! Player Substitution Logic
//!
//! match_sim.rs에서 추출된 교체 관련 메서드들.
//!
//! ## 포함 함수
//! - process_substitutions: 피로도 기반 자동 교체 처리
//! - find_substitution_candidate: 교체 후보 선수 찾기
//! - execute_substitution: 교체 실행
//! - force_injury_substitution: 부상으로 인한 강제 교체
//!
//! FIX_2601/0106 P1: **Roster swap (SSOT)**
//! - on-pitch 선수 접근은 `MatchSetup` assignment를 통해 수행
//! - 교체 시 pitch slot(track_id 0..21)의 점유자가 벤치 선수로 바뀜
//! - 벤치 선수는 재투입 불가(`sub_used`), 레드카드 퇴장은 교체 불가

use super::match_sim::MatchEngine;
use crate::engine::player_state::PlayerState;
use crate::models::{EventDetails, EventType, MatchEvent, SubstitutionDetails, TeamSide};

impl MatchEngine {
    /// P3: 교체 처리 - 피로한 선수를 벤치 선수로 교체
    pub(crate) fn process_substitutions(&mut self) {
        const MAX_SUBS: u8 = 5;

        // Update fatigue for all playing players based on their stamina
        for idx in 0..22 {
            // Calculate fatigue increase based on stamina
            let stamina = self.get_player_stamina(idx);
            let stamina_factor = 1.0 - (stamina / 20.0).min(0.8); // Higher stamina = less fatigue
            let fatigue_rate = 0.01 * stamina_factor; // 1% base per minute check
            self.player_fatigue[idx] = (self.player_fatigue[idx] + fatigue_rate).min(1.0);
        }

        // Check home team substitutions
        if self.substitutions_made.0 < MAX_SUBS {
            if let Some((pitch_track_id, bench_slot)) = self.find_substitution_candidate(true) {
                self.execute_substitution(pitch_track_id, bench_slot, true);
            }
        }

        // Check away team substitutions
        if self.substitutions_made.1 < MAX_SUBS {
            if let Some((pitch_track_id, bench_slot)) = self.find_substitution_candidate(false) {
                self.execute_substitution(pitch_track_id, bench_slot, false);
            }
        }
    }

    /// 교체 후보 찾기 - 피로도가 높은 선수와 적합한 교체 선수 반환
    pub(crate) fn find_substitution_candidate(&self, is_home: bool) -> Option<(usize, u8)> {
        let (start_idx, end_idx) = if is_home { (0, 11) } else { (11, 22) };

        // Find most tired player (above 0.65 fatigue threshold)
        let mut most_tired_idx: Option<usize> = None;
        let mut max_fatigue = 0.65; // Minimum fatigue threshold for substitution

        for idx in start_idx..end_idx {
            // Skip goalkeeper (idx 0 or 11) - rarely substitute GK
            if idx == start_idx {
                continue;
            }
            // Skip already injured players
            if self.injured_players.contains(&idx) {
                continue;
            }
            // Sent-off player cannot be replaced (rulebook)
            if matches!(self.get_player_fsm_state(idx), Some(PlayerState::SentOff)) {
                continue;
            }

            if self.player_fatigue[idx] > max_fatigue {
                max_fatigue = self.player_fatigue[idx];
                most_tired_idx = Some(idx);
            }
        }

        let pitch_track_id = most_tired_idx?;
        let tired_pos = self.get_match_player(pitch_track_id).position;

        let team = if is_home { TeamSide::Home } else { TeamSide::Away };
        let bench = if is_home {
            &self.setup.home.substitutes
        } else {
            &self.setup.away.substitutes
        };

        // Find best substitute from bench (0..6), skipping already-used slots.
        for (bench_slot, sub_player) in bench.iter().enumerate() {
            let bench_slot = bench_slot as u8;
            if self.setup.is_sub_used(team, bench_slot) {
                continue;
            }

            let same_zone = (sub_player.position.is_defender() && tired_pos.is_defender())
                || (sub_player.position.is_midfielder() && tired_pos.is_midfielder())
                || (sub_player.position.is_forward() && tired_pos.is_forward())
                || (sub_player.position.is_goalkeeper() && tired_pos.is_goalkeeper());

            if sub_player.position == tired_pos || same_zone {
                return Some((pitch_track_id, bench_slot));
            }
        }

        // If no position match, use first available bench player.
        for bench_slot in 0..bench.len() {
            let bench_slot = bench_slot as u8;
            if !self.setup.is_sub_used(team, bench_slot) {
                return Some((pitch_track_id, bench_slot));
            }
        }

        None
    }

    /// 교체 실행 (SSOT roster swap)
    pub(crate) fn execute_substitution(&mut self, pitch_track_id: usize, bench_slot: u8, is_home: bool) {
        // Safety guard: callers should keep these consistent.
        if TeamSide::is_home(pitch_track_id) != is_home {
            return;
        }

        // Apply SSOT roster assignment (bench -> pitch slot)
        let (player_in_name, player_out_name) =
            match self.setup.apply_substitution(pitch_track_id, bench_slot) {
                Ok(v) => v,
                Err(_) => return,
            };

        // Record the substitution event (C5: engine-confirmed timestamp)
        self.emit_event(MatchEvent {
            minute: self.minute,
            timestamp_ms: Some(self.current_timestamp_ms()),
            event_type: EventType::Substitution,
            is_home_team: is_home,
            player_track_id: Some(pitch_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails {
                substitution: Some(SubstitutionDetails {
                    player_in_name,
                    player_out_name,
                    bench_slot,
                }),
                ..Default::default()
            }),
        });

        // Update substitution count
        if is_home {
            self.substitutions_made.0 += 1;
        } else {
            self.substitutions_made.1 += 1;
        }

        // Reset runtime state for the new occupant.
        self.reset_pitch_slot_after_substitution(pitch_track_id);
    }

    /// P3: 부상으로 인한 강제 교체
    pub(crate) fn force_injury_substitution(&mut self, injured_idx: usize, is_home: bool) {
        let used_subs = if is_home {
            self.substitutions_made.0
        } else {
            self.substitutions_made.1
        };

        // Can only substitute if we haven't used all 5 subs
        if used_subs >= 5 {
            // No subs left - team plays with 10 players
            // This is handled by the injury tracking
            return;
        }

        let injured_pos = self.get_match_player(injured_idx).position;
        let team = if is_home { TeamSide::Home } else { TeamSide::Away };
        let bench = if is_home {
            &self.setup.home.substitutes
        } else {
            &self.setup.away.substitutes
        };

        // Find suitable substitute from bench
        for (bench_slot, sub_player) in bench.iter().enumerate() {
            let bench_slot = bench_slot as u8;
            if self.setup.is_sub_used(team, bench_slot) {
                continue;
            }

            let same_zone = (sub_player.position.is_defender() && injured_pos.is_defender())
                || (sub_player.position.is_midfielder() && injured_pos.is_midfielder())
                || (sub_player.position.is_forward() && injured_pos.is_forward())
                || (sub_player.position.is_goalkeeper() && injured_pos.is_goalkeeper());

            if sub_player.position == injured_pos || same_zone {
                self.execute_substitution(injured_idx, bench_slot, is_home);
                return;
            }
        }

        // If no position match, use first available bench player
        for bench_slot in 0..bench.len() {
            let bench_slot = bench_slot as u8;
            if !self.setup.is_sub_used(team, bench_slot) {
                self.execute_substitution(injured_idx, bench_slot, is_home);
                return;
            }
        }
    }
}

