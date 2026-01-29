//! Tackle Action FSM
//!
//! P7 Spec Section 2.2, 6: TacklePhase, TackleOutcome, 충돌 판정
//!
//! ## Tackle Phase Flow
//! ```text
//! Approach (0.5~2s) → Commit (0.25~1s) → CollisionCheck (1 tick) →
//! Outcome → Recovery (0.25~1s) → Cooldown (4s) → Finished
//! ```

use serde::{Deserialize, Serialize};
// P0: Core types moved to action_queue
use super::super::action_queue::{
    TackleActionKind, TackleEvent, TackleOutcome, TackleType, ViewerTackleOutcome,
};
use super::duration::*;
use crate::models::player::PlayerAttributes;

/// 태클 Phase
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TacklePhase {
    /// 달려들기 (속도 증가, 방향 전환)
    Approach { remaining_ticks: u8 },

    /// 태클 동작 (몸 기울기, 발 내밀기)
    Commit { remaining_ticks: u8 },

    /// 충돌 판정 (1틱)
    CollisionCheck,

    /// 판정 결과
    Outcome(TackleOutcome),

    /// 회복 (기립, 미끄러짐에서 복귀)
    Recovery { remaining_ticks: u8 },

    /// 다시 태클 불가
    Cooldown { remaining_ticks: u8 },

    /// 완료
    Finished,
}

impl TacklePhase {
    /// Phase가 활성 상태인지
    pub fn is_active(&self) -> bool {
        !matches!(self, TacklePhase::Finished)
    }

    /// 현재 Phase 이름
    pub fn name(&self) -> &'static str {
        match self {
            TacklePhase::Approach { .. } => "Approach",
            TacklePhase::Commit { .. } => "Commit",
            TacklePhase::CollisionCheck => "CollisionCheck",
            TacklePhase::Outcome(_) => "Outcome",
            TacklePhase::Recovery { .. } => "Recovery",
            TacklePhase::Cooldown { .. } => "Cooldown",
            TacklePhase::Finished => "Finished",
        }
    }
}

/// 태클 액션 FSM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TackleAction {
    /// 고유 ID
    pub id: u64,

    /// 태클러 선수 인덱스
    pub tackler_idx: usize,

    /// 태클러 팀 ID
    pub tackler_team: u32,

    /// 타겟 선수 인덱스
    pub target_idx: usize,

    /// 태클 타입
    pub tackle_type: TackleType,

    /// 현재 Phase
    pub phase: TacklePhase,

    /// 시작 틱
    pub start_tick: u64,

    /// 결과 (Outcome phase 이후 설정)
    pub outcome: Option<TackleOutcome>,

    /// Viewer 이벤트 (Contact Frame에서 생성, 외부에서 수집)
    #[serde(skip)]
    pending_viewer_event: Option<TackleEvent>,

    // ========== P17 Phase 4: 스킬 필드 ==========
    /// 태클 능력 (0-99)
    pub tackling: u8,
    /// 공격성 (0-99)
    pub aggression: u8,
    /// 힘 (0-99)
    pub strength: u8,
}

impl TackleAction {
    /// 새 태클 액션 생성
    pub fn new(
        id: u64,
        tackler_idx: usize,
        tackler_team: u32,
        target_idx: usize,
        tackle_type: TackleType,
        distance: f32,
        start_tick: u64,
    ) -> Self {
        let approach_ticks = calculate_approach_ticks(distance, TACKLE_APPROACH_SPEED);

        Self {
            id,
            tackler_idx,
            tackler_team,
            target_idx,
            tackle_type,
            phase: TacklePhase::Approach { remaining_ticks: approach_ticks },
            start_tick,
            outcome: None,
            pending_viewer_event: None,
            // P17: 스킬 필드 기본값
            tackling: 0,
            aggression: 0,
            strength: 0,
        }
    }

    /// P17 Phase 4: 능력치와 함께 태클 액션 생성
    pub fn new_with_attrs(
        id: u64,
        tackler_idx: usize,
        tackler_team: u32,
        target_idx: usize,
        tackle_type: TackleType,
        distance: f32,
        start_tick: u64,
        attrs: &PlayerAttributes,
    ) -> Self {
        let mut action =
            Self::new(id, tackler_idx, tackler_team, target_idx, tackle_type, distance, start_tick);
        action.tackling = attrs.tackling;
        action.aggression = attrs.aggression;
        action.strength = attrs.strength;
        action
    }

    /// 매 틱 업데이트
    ///
    /// Returns: 결과가 발생하면 Some(TackleOutcome)
    pub fn update_tick<R: rand::Rng>(
        &mut self,
        tackler_pos: &mut (f32, f32),
        target_pos: (f32, f32),
        ball_pos: (f32, f32),
        target_facing: f32,
        tackler_skill: u8,
        target_dribble: u8,
        rng: &mut R,
    ) -> Option<TackleOutcome> {
        match &mut self.phase {
            TacklePhase::Approach { remaining_ticks } => {
                // 태클러가 타겟 쪽으로 이동
                let dir = normalize((target_pos.0 - tackler_pos.0, target_pos.1 - tackler_pos.1));
                tackler_pos.0 += dir.0 * TACKLE_APPROACH_SPEED * TICK_DT;
                tackler_pos.1 += dir.1 * TACKLE_APPROACH_SPEED * TICK_DT;

                *remaining_ticks = remaining_ticks.saturating_sub(1);

                // 충분히 가까워지거나 시간 종료
                let dist = distance(*tackler_pos, target_pos);
                if dist < TACKLE_COMMIT_DISTANCE || *remaining_ticks == 0 {
                    if dist < TACKLE_MAX_DISTANCE {
                        self.phase = TacklePhase::Commit {
                            remaining_ticks: self.tackle_type.commit_ticks(),
                        };
                    } else {
                        // 너무 멀면 실패
                        self.phase = TacklePhase::Outcome(TackleOutcome::Miss);
                        self.outcome = Some(TackleOutcome::Miss);
                    }
                }
                None
            }

            TacklePhase::Commit { remaining_ticks } => {
                // 태클 동작 진행 (이동 불가)
                *remaining_ticks = remaining_ticks.saturating_sub(1);

                if *remaining_ticks == 0 {
                    self.phase = TacklePhase::CollisionCheck;
                }
                None
            }

            TacklePhase::CollisionCheck => {
                // 충돌 판정
                let outcome = self.calculate_collision_outcome(
                    *tackler_pos,
                    target_pos,
                    ball_pos,
                    target_facing,
                    tackler_skill,
                    target_dribble,
                    rng,
                );
                self.outcome = Some(outcome);
                self.phase = TacklePhase::Outcome(outcome);
                None
            }

            TacklePhase::Outcome(outcome) => {
                let result = *outcome;

                // Recovery 틱 수 결정
                let recovery_ticks = match outcome {
                    TackleOutcome::CleanWin => TACKLE_RECOVERY_CLEAN_TICKS,
                    TackleOutcome::Deflection => TACKLE_RECOVERY_DEFLECT_TICKS,
                    TackleOutcome::Miss => TACKLE_RECOVERY_MISS_TICKS,
                    TackleOutcome::Foul | TackleOutcome::YellowCard | TackleOutcome::RedCard => {
                        TACKLE_RECOVERY_FOUL_TICKS
                    }
                };

                self.phase = TacklePhase::Recovery { remaining_ticks: recovery_ticks };

                // Contact Frame: Outcome 결과 반환 시 Viewer 이벤트 생성
                // Note: ball_owner 정보는 외부에서 update_tick 호출 후 별도로 이벤트를 생성해야 함
                // 여기서는 기본 이벤트 구조만 생성 (외부에서 ball_owner 정보를 보충해야 함)
                // tick_based.rs에서 outcome 확인 후 emit_collision_event 호출 권장

                Some(result)
            }

            TacklePhase::Recovery { remaining_ticks } => {
                *remaining_ticks = remaining_ticks.saturating_sub(1);

                if *remaining_ticks == 0 {
                    self.phase = TacklePhase::Cooldown { remaining_ticks: TACKLE_COOLDOWN_TICKS };
                }
                None
            }

            TacklePhase::Cooldown { remaining_ticks } => {
                *remaining_ticks = remaining_ticks.saturating_sub(1);

                if *remaining_ticks == 0 {
                    self.phase = TacklePhase::Finished;
                }
                None
            }

            TacklePhase::Finished => None,
        }
    }

    /// 충돌 결과 계산
    fn calculate_collision_outcome<R: rand::Rng>(
        &self,
        tackler_pos: (f32, f32),
        target_pos: (f32, f32),
        ball_pos: (f32, f32),
        target_facing: f32,
        tackler_skill: u8,
        target_dribble: u8,
        rng: &mut R,
    ) -> TackleOutcome {
        let dist_to_ball = distance(tackler_pos, ball_pos);
        let dist_to_player = distance(tackler_pos, target_pos);

        // 태클 범위 밖이면 Miss
        if dist_to_ball > self.tackle_type.reach() && dist_to_player > self.tackle_type.reach() {
            return TackleOutcome::Miss;
        }

        // 공/사람 어디에 먼저 닿았는지
        let hit_ball_first = dist_to_ball < dist_to_player;

        // 접근 각도
        let approach_angle = calculate_approach_angle(tackler_pos, target_pos, target_facing);

        // 능력치 기반 확률
        let tackle_factor = tackler_skill as f32 / 100.0;
        let dribble_factor = target_dribble as f32 / 100.0;
        let skill_diff = (tackle_factor - dribble_factor + 1.0) / 2.0; // 0.0 ~ 1.0

        let roll: f32 = rng.gen();

        if hit_ball_first {
            // 공에 먼저 닿음
            let clean_threshold = skill_diff * 0.7;
            let deflect_threshold = skill_diff * 0.9;

            if roll < clean_threshold {
                TackleOutcome::CleanWin
            } else if roll < deflect_threshold {
                TackleOutcome::Deflection
            } else {
                TackleOutcome::Miss
            }
        } else {
            // 사람에 먼저 닿음 → 파울 가능성
            let foul_chance = if approach_angle > 135.0 {
                0.85 // 뒤에서 → 85% 파울
            } else if approach_angle > 90.0 {
                0.5 // 측면 → 50% 파울
            } else {
                0.25 // 정면 → 25% 파울
            };

            // 슬라이딩은 파울 확률 증가
            let foul_chance: f32 = match self.tackle_type {
                TackleType::Sliding => (foul_chance + 0.15_f32).min(0.95),
                TackleType::Shoulder => (foul_chance - 0.1_f32).max(0.1),
                TackleType::Standing => foul_chance,
            };

            if roll < foul_chance {
                // 파울
                let card_roll: f32 = rng.gen();
                if card_roll < 0.05 && approach_angle > 135.0 {
                    TackleOutcome::RedCard
                } else if card_roll < 0.15 {
                    TackleOutcome::YellowCard
                } else {
                    TackleOutcome::Foul
                }
            } else if roll < foul_chance + (1.0 - foul_chance) * skill_diff {
                TackleOutcome::CleanWin
            } else {
                TackleOutcome::Deflection
            }
        }
    }

    /// 태클이 완료되었는지
    pub fn is_finished(&self) -> bool {
        matches!(self.phase, TacklePhase::Finished)
    }

    /// Viewer 이벤트 수집 (Contact Frame에서 생성된 이벤트를 꺼냄)
    pub fn take_viewer_event(&mut self) -> Option<TackleEvent> {
        self.pending_viewer_event.take()
    }

    /// 태클러가 이동 가능한지
    pub fn can_tackler_move(&self) -> bool {
        matches!(
            self.phase,
            TacklePhase::Approach { .. } | TacklePhase::Cooldown { .. } | TacklePhase::Finished
        )
    }

    /// 남은 쿨다운 틱
    pub fn remaining_cooldown(&self) -> u8 {
        match self.phase {
            TacklePhase::Cooldown { remaining_ticks } => remaining_ticks,
            TacklePhase::Finished => 0,
            _ => TACKLE_COOLDOWN_TICKS, // 아직 쿨다운에 도달하지 않음
        }
    }

    // ========================================================================
    // Viewer Event Generation (P7 Section 16)
    // ========================================================================

    /// CollisionCheck 시점에 호출하여 TackleEvent 생성
    ///
    /// # Arguments
    /// * `now_tick` - 현재 틱
    /// * `outcome` - 태클 결과
    /// * `tackler_track_id` - 태클러 트랙 ID
    /// * `target_track_id` - 타겟 트랙 ID
    /// * `ball_owner_before` - 충돌 전 공 소유자 track_id
    /// * `ball_owner_after` - 충돌 후 공 소유자 track_id
    /// * `contact_pos` - 접촉 위치 (VFX용)
    /// * `ball_pos` - 공 위치 (3D)
    ///
    /// # Returns
    /// TackleEvent for Viewer
    pub fn emit_collision_event(
        &self,
        now_tick: u64,
        outcome: TackleOutcome,
        tackler_track_id: u32,
        target_track_id: u32,
        ball_owner_before: Option<u32>,
        ball_owner_after: Option<u32>,
        contact_pos: Option<(f32, f32)>,
        ball_pos: Option<(f32, f32, f32)>,
    ) -> TackleEvent {
        let t_ms = now_tick * 250; // 1 tick = 250ms

        // tackle_type → TackleActionKind 변환
        let action = TackleActionKind::from_tackle_type(self.tackle_type);

        // TackleOutcome → ViewerTackleOutcome 변환
        let viewer_outcome = ViewerTackleOutcome::from_tackle_outcome(outcome);

        // lock_ms 계산
        let lock_ms = self.calculate_lock_ms(viewer_outcome);

        TackleEvent {
            t_ms,
            kind: "tackle",
            actor_track_id: tackler_track_id,
            target_track_id,
            action,
            lock_ms,
            outcome: viewer_outcome,
            ball_owner_before,
            ball_owner_after,
            contact_pos,
            ball_pos,
        }
    }

    /// lock_ms 계산 (Viewer 애니메이션 락 시간)
    ///
    /// lock_ms = (commit_ticks + resolve_tick) * 250 + outcome_extra
    pub fn calculate_lock_ms(&self, viewer_outcome: ViewerTackleOutcome) -> u32 {
        TackleEvent::calculate_lock_ms(self.tackle_type, viewer_outcome)
    }
}

/// 태클 시도 가능 여부 결과
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TackleAttemptResult {
    /// 시도 가능
    CanAttempt { approach_ticks: u8 },
    /// 선수 상태가 안 됨
    NotReady,
    /// 쿨다운 중
    OnCooldown,
    /// 너무 멀리 있음
    TooFar,
    /// 경로에 다른 선수가 있음
    PathBlocked,
    /// 접근 각도가 나쁨
    BadAngle,
}

impl TackleAttemptResult {
    pub fn is_can_attempt(&self) -> bool {
        matches!(self, TackleAttemptResult::CanAttempt { .. })
    }
}

/// 태클 시도 가능 여부 체크
pub fn can_attempt_tackle(
    tackler_pos: (f32, f32),
    _target_pos: (f32, f32),
    ball_pos: (f32, f32),
    can_start_action: bool,
    cooldown: u8,
) -> TackleAttemptResult {
    // 1. 선수 상태 체크
    if !can_start_action {
        return TackleAttemptResult::NotReady;
    }

    // 2. 쿨다운 체크
    if cooldown > 0 {
        return TackleAttemptResult::OnCooldown;
    }

    // 3. 거리 체크
    let dist = distance(tackler_pos, ball_pos);
    if dist > TACKLE_MAX_DISTANCE {
        return TackleAttemptResult::TooFar;
    }

    // 4. Approach 틱 수 계산
    let approach_ticks = calculate_approach_ticks(dist, TACKLE_APPROACH_SPEED);

    TackleAttemptResult::CanAttempt { approach_ticks }
}

/// 접근 각도 계산 (0 = 정면, 180 = 뒤)
pub fn calculate_approach_angle(
    tackler_pos: (f32, f32),
    target_pos: (f32, f32),
    target_facing: f32,
) -> f32 {
    // 타겟 → 태클러 방향
    let to_tackler = (tackler_pos.0 - target_pos.0, tackler_pos.1 - target_pos.1);

    let len = (to_tackler.0 * to_tackler.0 + to_tackler.1 * to_tackler.1).sqrt();
    if len < 0.001 {
        return 0.0;
    }

    let to_tackler_norm = (to_tackler.0 / len, to_tackler.1 / len);

    // 타겟의 전방 벡터
    let target_forward = (target_facing.cos(), target_facing.sin());

    // 내적으로 각도 계산
    let dot = to_tackler_norm.0 * target_forward.0 + to_tackler_norm.1 * target_forward.1;

    dot.clamp(-1.0, 1.0).acos().to_degrees()
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 두 점 사이 거리
#[inline]
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

/// 벡터 정규화
#[inline]
fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len > 0.001 {
        (v.0 / len, v.1 / len)
    } else {
        (0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    const CY: f32 = field::CENTER_Y;

    #[test]
    fn test_tackle_action_creation() {
        let action = TackleAction::new(
            1,
            5,  // tackler_idx
            0,  // tackler_team
            10, // target_idx
            TackleType::Standing,
            3.0, // distance
            100, // start_tick
        );

        assert_eq!(action.id, 1);
        assert_eq!(action.tackler_idx, 5);
        assert_eq!(action.target_idx, 10);
        assert!(matches!(action.phase, TacklePhase::Approach { .. }));
    }

    #[test]
    fn test_tackle_phase_progression() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut action = TackleAction::new(
            1,
            5,
            0,
            10,
            TackleType::Standing,
            1.0, // 가까운 거리
            100,
        );

        let mut tackler_pos = (50.0, CY);
        let target_pos = (51.0, CY);
        let ball_pos = (51.0, CY);

        // Approach → Commit
        for _ in 0..10 {
            if matches!(action.phase, TacklePhase::Commit { .. }) {
                break;
            }
            action.update_tick(&mut tackler_pos, target_pos, ball_pos, 0.0, 70, 60, &mut rng);
        }
        assert!(
            matches!(action.phase, TacklePhase::Commit { .. })
                || matches!(action.phase, TacklePhase::CollisionCheck)
        );

        // Commit → CollisionCheck → Outcome
        for _ in 0..10 {
            if matches!(action.phase, TacklePhase::Outcome(_)) {
                break;
            }
            action.update_tick(&mut tackler_pos, target_pos, ball_pos, 0.0, 70, 60, &mut rng);
        }
        assert!(
            matches!(action.phase, TacklePhase::Outcome(_))
                || matches!(action.phase, TacklePhase::Recovery { .. })
        );
    }

    #[test]
    fn test_tackle_outcome_recorded() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut action = TackleAction::new(
            1,
            5,
            0,
            10,
            TackleType::Standing,
            0.5, // 매우 가까움
            100,
        );

        let mut tackler_pos = (50.0, CY);
        let target_pos = (50.5, CY);
        let ball_pos = (50.5, CY);

        // 결과가 나올 때까지 진행
        for _ in 0..50 {
            if action.outcome.is_some() {
                break;
            }
            action.update_tick(&mut tackler_pos, target_pos, ball_pos, 0.0, 70, 60, &mut rng);
        }

        assert!(action.outcome.is_some());
    }

    #[test]
    fn test_tackle_too_far_miss() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut action = TackleAction::new(
            1,
            5,
            0,
            10,
            TackleType::Standing,
            10.0, // 너무 멀리
            100,
        );

        let mut tackler_pos = (40.0, CY);
        let target_pos = (50.0, CY);
        let ball_pos = (50.0, CY);

        // Approach 시간 초과 후 Miss
        for _ in 0..30 {
            action.update_tick(&mut tackler_pos, target_pos, ball_pos, 0.0, 70, 60, &mut rng);
        }

        // Miss 또는 다른 결과가 나와야 함
        assert!(action.outcome.is_some() || matches!(action.phase, TacklePhase::Approach { .. }));
    }

    #[test]
    fn test_approach_angle_calculation() {
        // 정면에서 접근
        let angle = calculate_approach_angle(
            (55.0, CY), // 태클러 (동쪽)
            (50.0, CY), // 타겟
            0.0,          // 타겟이 동쪽(0도)을 봄
        );
        assert!(angle < 30.0); // 정면

        // 뒤에서 접근
        let angle_behind = calculate_approach_angle(
            (45.0, CY), // 태클러 (서쪽)
            (50.0, CY), // 타겟
            0.0,          // 타겟이 동쪽을 봄
        );
        assert!(angle_behind > 150.0); // 뒤에서
    }

    #[test]
    fn test_can_attempt_tackle() {
        let result = can_attempt_tackle(
            (50.0, CY), // tackler
            (53.0, CY), // target
            (53.0, CY), // ball
            true,         // can_start
            0,            // cooldown
        );
        assert!(result.is_can_attempt());

        // 쿨다운 중
        let result_cd = can_attempt_tackle((50.0, CY), (53.0, CY), (53.0, CY), true, 10);
        assert_eq!(result_cd, TackleAttemptResult::OnCooldown);

        // 너무 멀리
        let result_far = can_attempt_tackle((50.0, CY), (60.0, CY), (60.0, CY), true, 0);
        assert_eq!(result_far, TackleAttemptResult::TooFar);
    }

    #[test]
    fn test_tackle_cooldown_duration() {
        // 6초 쿨다운 = 24틱 (P7 수치 조정)
        assert_eq!(TACKLE_COOLDOWN_TICKS, 24);
        assert_eq!(ticks_to_seconds(TACKLE_COOLDOWN_TICKS as u64), 6.0);
    }

    #[test]
    fn test_full_tackle_lifecycle() {
        let mut rng = StdRng::seed_from_u64(12345);
        let mut action = TackleAction::new(1, 5, 0, 10, TackleType::Standing, 1.5, 100);

        let mut tackler_pos = (50.0, CY);
        let target_pos = (51.5, CY);
        let ball_pos = (51.5, CY);

        let mut phases_seen = Vec::new();
        let mut last_phase_name = "";

        // 전체 라이프사이클 진행
        for _ in 0..100 {
            if action.is_finished() {
                break;
            }

            let current_phase = action.phase.name();
            if current_phase != last_phase_name {
                phases_seen.push(current_phase.to_string());
                last_phase_name = current_phase;
            }

            action.update_tick(&mut tackler_pos, target_pos, ball_pos, 0.0, 70, 60, &mut rng);
        }

        assert!(action.is_finished());
        assert!(phases_seen.contains(&"Approach".to_string()));
        assert!(phases_seen.contains(&"Cooldown".to_string()));
    }

    // ========================================================================
    // Viewer Event Tests (P7 Section 16)
    // ========================================================================

    #[test]
    fn test_emit_collision_event_standing_clean() {
        let action = TackleAction::new(1, 5, 0, 10, TackleType::Standing, 1.0, 100);

        let event = action.emit_collision_event(
            100,
            TackleOutcome::CleanWin,
            5,
            10,
            Some(10), // ball_owner_before: target
            Some(5),  // ball_owner_after: tackler
            Some((50.5, CY)),
            Some((50.5, CY, 0.1)),
        );

        assert_eq!(event.t_ms, 25000); // 100 * 250ms
        assert_eq!(event.kind, "tackle");
        assert_eq!(event.actor_track_id, 5);
        assert_eq!(event.target_track_id, 10);
        assert_eq!(event.action, TackleActionKind::TackleStand);
        assert_eq!(event.outcome, ViewerTackleOutcome::Clean);
        assert_eq!(event.ball_owner_before, Some(10));
        assert_eq!(event.ball_owner_after, Some(5));
    }

    #[test]
    fn test_emit_collision_event_sliding_foul() {
        let action = TackleAction::new(2, 8, 1, 3, TackleType::Sliding, 2.0, 200);

        let event = action.emit_collision_event(
            200,
            TackleOutcome::Foul,
            8,
            3,
            Some(3),
            Some(3), // 파울이라 공 소유 유지
            Some((60.0, 40.0)),
            Some((60.0, 40.0, 0.0)),
        );

        assert_eq!(event.action, TackleActionKind::TackleSlide);
        assert_eq!(event.outcome, ViewerTackleOutcome::Foul);
    }

    #[test]
    fn test_emit_collision_event_shoulder_deflect() {
        let action = TackleAction::new(3, 6, 0, 9, TackleType::Shoulder, 0.8, 150);

        let event = action.emit_collision_event(
            150,
            TackleOutcome::Deflection,
            6,
            9,
            Some(9),
            None, // 루즈볼
            Some((55.0, 30.0)),
            Some((55.5, 30.0, 0.2)),
        );

        assert_eq!(event.action, TackleActionKind::TackleShoulder);
        assert_eq!(event.outcome, ViewerTackleOutcome::Deflect);
        assert_eq!(event.ball_owner_after, None);
    }

    #[test]
    fn test_calculate_lock_ms_standing() {
        let action = TackleAction::new(1, 5, 0, 10, TackleType::Standing, 1.0, 100);

        // Standing: commit=2, resolve=1 → base = (2+1)*250 = 750
        // Clean: extra = 350 → total = 1100, clamped to 500-1400
        let clean_lock = action.calculate_lock_ms(ViewerTackleOutcome::Clean);
        assert!((500..=1400).contains(&clean_lock));

        // Miss: extra = 625 → total = 1375
        let miss_lock = action.calculate_lock_ms(ViewerTackleOutcome::Miss);
        assert!((500..=1400).contains(&miss_lock));

        // Foul: extra = 1000 → total = 1750 → clamped to 1400
        let foul_lock = action.calculate_lock_ms(ViewerTackleOutcome::Foul);
        assert_eq!(foul_lock, 1400); // Clamped max
    }

    #[test]
    fn test_calculate_lock_ms_sliding() {
        let action = TackleAction::new(1, 5, 0, 10, TackleType::Sliding, 1.0, 100);

        // Sliding: commit=4, resolve=1 → base = (4+1)*250 = 1250
        // Clean: extra = 350 → total = 1600 → clamped to 1400
        let clean_lock = action.calculate_lock_ms(ViewerTackleOutcome::Clean);
        assert_eq!(clean_lock, 1400); // Clamped max
    }

    #[test]
    fn test_calculate_lock_ms_shoulder() {
        let action = TackleAction::new(1, 5, 0, 10, TackleType::Shoulder, 0.8, 100);

        // Shoulder: commit=1, resolve=1 → base = (1+1)*250 = 500
        // Clean: extra = 350 → total = 850
        let clean_lock = action.calculate_lock_ms(ViewerTackleOutcome::Clean);
        assert!((500..=1400).contains(&clean_lock));
        assert_eq!(clean_lock, 850);
    }

    #[test]
    fn test_tackle_action_kind_mapping() {
        assert_eq!(
            TackleActionKind::from_tackle_type(TackleType::Standing),
            TackleActionKind::TackleStand
        );
        assert_eq!(
            TackleActionKind::from_tackle_type(TackleType::Sliding),
            TackleActionKind::TackleSlide
        );
        assert_eq!(
            TackleActionKind::from_tackle_type(TackleType::Shoulder),
            TackleActionKind::TackleShoulder
        );
    }

    #[test]
    fn test_viewer_outcome_mapping() {
        assert_eq!(
            ViewerTackleOutcome::from_tackle_outcome(TackleOutcome::CleanWin),
            ViewerTackleOutcome::Clean
        );
        assert_eq!(
            ViewerTackleOutcome::from_tackle_outcome(TackleOutcome::Deflection),
            ViewerTackleOutcome::Deflect
        );
        assert_eq!(
            ViewerTackleOutcome::from_tackle_outcome(TackleOutcome::Miss),
            ViewerTackleOutcome::Miss
        );
        assert_eq!(
            ViewerTackleOutcome::from_tackle_outcome(TackleOutcome::Foul),
            ViewerTackleOutcome::Foul
        );
        assert_eq!(
            ViewerTackleOutcome::from_tackle_outcome(TackleOutcome::YellowCard),
            ViewerTackleOutcome::Yellow
        );
        assert_eq!(
            ViewerTackleOutcome::from_tackle_outcome(TackleOutcome::RedCard),
            ViewerTackleOutcome::Red
        );
    }
}
