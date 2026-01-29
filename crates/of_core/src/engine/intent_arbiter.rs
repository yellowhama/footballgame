//! Intent Arbiter - Phase2 Conflict Resolution
//!
//! FIX_2601/0117: Central arbiter for resolving conflicts when multiple
//! players attempt incompatible actions in the same tick.
//!
//! ## Conflict Types
//! 1. **BallTouchConflict**: Same tick, 2+ players touch ball
//! 2. **TackleConflict**: Same victim, 2+ tackles
//! 3. **SpaceConflict**: Same target position, 2+ movements (minor)
//!
//! ## Resolution Priority (BallTouch)
//! 1. Ball owner's action (Pass/Shoot/Dribble)
//! 2. Closest to ball (ETA-based)
//! 3. Highest utility score
//! 4. Position-based hash (FIX_2601/0119: fair tie-breaker, no team bias)

use super::match_sim::deterministic_tie_hash;
use super::tick_snapshot::{
    BallStateTag, CommitResult, ConflictResolution, ConflictType, IntentKind, PlayerIntent,
    PlayerStateTag, TickSnapshot,
};

// ============================================================================
// Conflict Detection
// ============================================================================

/// 충돌 정보
#[derive(Clone, Debug)]
pub enum Conflict {
    /// 같은 tick에 공을 터치하는 Intent가 2개 이상
    BallTouch {
        /// 충돌하는 intent 인덱스들
        intent_indices: Vec<usize>,
    },
    /// 같은 대상에게 태클/프레스가 2개 이상
    Tackle {
        /// 태클 대상 (victim)
        victim: u8,
        /// 충돌하는 intent 인덱스들
        intent_indices: Vec<usize>,
    },
}

/// Intent 목록에서 충돌 탐지
pub fn detect_conflicts(intents: &[PlayerIntent]) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    // 1. BallTouchConflict
    let ball_touch_indices: Vec<usize> = intents
        .iter()
        .enumerate()
        .filter(|(_, intent)| intent.meta.touches_ball)
        .map(|(idx, _)| idx)
        .collect();

    if ball_touch_indices.len() > 1 {
        conflicts.push(Conflict::BallTouch {
            intent_indices: ball_touch_indices,
        });
    }

    // 2. TackleConflict
    let tackle_intents: Vec<(usize, u8)> = intents
        .iter()
        .enumerate()
        .filter(|(_, intent)| matches!(intent.kind, IntentKind::Tackle | IntentKind::Press))
        .filter_map(|(idx, intent)| intent.target_player.map(|t| (idx, t)))
        .collect();

    // victim별로 그룹화
    let mut tackle_by_victim: std::collections::HashMap<u8, Vec<usize>> =
        std::collections::HashMap::new();
    for (idx, victim) in tackle_intents {
        tackle_by_victim.entry(victim).or_default().push(idx);
    }

    for (victim, indices) in tackle_by_victim {
        if indices.len() > 1 {
            conflicts.push(Conflict::Tackle {
                victim,
                intent_indices: indices,
            });
        }
    }

    conflicts
}

// ============================================================================
// Conflict Resolution
// ============================================================================

/// 충돌 해결 후 Intent 목록 반환
pub fn resolve_conflicts(
    mut intents: Vec<PlayerIntent>,
    conflicts: Vec<Conflict>,
    snapshot: &TickSnapshot,
) -> (Vec<PlayerIntent>, Vec<ConflictResolution>) {
    let mut resolutions = Vec::new();

    for conflict in conflicts {
        match conflict {
            Conflict::BallTouch { intent_indices } => {
                let resolution = resolve_ball_touch_conflict(&mut intents, &intent_indices, snapshot);
                resolutions.push(resolution);
            }
            Conflict::Tackle {
                victim,
                intent_indices,
            } => {
                let resolution =
                    resolve_tackle_conflict(&mut intents, victim, &intent_indices, snapshot);
                resolutions.push(resolution);
            }
        }
    }

    (intents, resolutions)
}

/// BallTouchConflict 해결
fn resolve_ball_touch_conflict(
    intents: &mut [PlayerIntent],
    conflict_indices: &[usize],
    snapshot: &TickSnapshot,
) -> ConflictResolution {
    let winner_idx = select_ball_touch_winner(intents, conflict_indices, snapshot);
    let winner_intent_idx = conflict_indices[winner_idx];
    let winner_actor = intents[winner_intent_idx].actor;

    let mut losers = Vec::new();

    // 패자들을 fallback으로 변환
    for (i, &intent_idx) in conflict_indices.iter().enumerate() {
        if i != winner_idx {
            let loser_actor = intents[intent_idx].actor;
            losers.push(loser_actor);
            intents[intent_idx] = intents[intent_idx].to_fallback();
        }
    }

    ConflictResolution {
        conflict_type: ConflictType::BallTouch,
        winner: winner_actor,
        losers,
        resolution_reason: determine_ball_touch_reason(intents, winner_intent_idx, snapshot),
    }
}

/// BallTouch 승자 선택
fn select_ball_touch_winner(
    intents: &[PlayerIntent],
    conflict_indices: &[usize],
    snapshot: &TickSnapshot,
) -> usize {
    // 우선순위 1: 공 소유자
    if let Some(owner) = snapshot.ball.owner {
        for (i, &intent_idx) in conflict_indices.iter().enumerate() {
            if intents[intent_idx].actor == owner {
                return i;
            }
        }
    }

    // 우선순위 2: ETA (InFlight/Loose일 때)
    if matches!(
        snapshot.ball.state,
        BallStateTag::InFlight | BallStateTag::Loose
    ) {
        // FIX_2601/0119: Use position for fair tie-breaking instead of track_id
        let mut with_eta: Vec<(usize, i32, f32, (f32, f32))> = conflict_indices
            .iter()
            .enumerate()
            .map(|(i, &intent_idx)| {
                let intent = &intents[intent_idx];
                let player = &snapshot.players[intent.actor as usize];
                let eta = player.dist_to_ball; // 0.1m 단위
                let utility = intent.utility;
                let pos = (player.pos.x as f32 / 10.0, player.pos.y as f32 / 10.0);
                (i, eta, utility, pos)
            })
            .collect();

        // ETA 오름차순 → Utility 내림차순 → position-based hash (no team bias)
        with_eta.sort_by(|a, b| {
            a.1.cmp(&b.1) // ETA 오름차순
                .then_with(|| {
                    b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal) // Utility 내림차순
                })
                .then_with(|| deterministic_tie_hash(0, a.3, 0, b.3)) // FIX_2601/0119
        });

        return with_eta[0].0;
    }

    // 우선순위 3: Utility 높은 순
    // FIX_2601/0119: Use position for fair tie-breaking instead of track_id
    let mut with_utility: Vec<(usize, f32, (f32, f32))> = conflict_indices
        .iter()
        .enumerate()
        .map(|(i, &intent_idx)| {
            let intent = &intents[intent_idx];
            let player = &snapshot.players[intent.actor as usize];
            let pos = (player.pos.x as f32 / 10.0, player.pos.y as f32 / 10.0);
            (i, intent.utility, pos)
        })
        .collect();

    with_utility.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal) // Utility 내림차순
            .then_with(|| deterministic_tie_hash(0, a.2, 0, b.2)) // FIX_2601/0119
    });

    with_utility[0].0
}

/// BallTouch 해결 이유 문자열 생성
fn determine_ball_touch_reason(
    intents: &[PlayerIntent],
    winner_idx: usize,
    snapshot: &TickSnapshot,
) -> String {
    let winner_actor = intents[winner_idx].actor;

    if snapshot.ball.owner == Some(winner_actor) {
        "owner".to_string()
    } else if matches!(
        snapshot.ball.state,
        BallStateTag::InFlight | BallStateTag::Loose
    ) {
        "eta".to_string()
    } else {
        "utility".to_string()
    }
}

/// TackleConflict 해결
fn resolve_tackle_conflict(
    intents: &mut [PlayerIntent],
    victim: u8,
    conflict_indices: &[usize],
    snapshot: &TickSnapshot,
) -> ConflictResolution {
    let victim_pos = snapshot.players[victim as usize].pos;

    // 거리 기반 정렬
    // FIX_2601/0119: Use position for fair tie-breaking instead of track_id
    let mut with_distance: Vec<(usize, i32, f32, (f32, f32))> = conflict_indices
        .iter()
        .enumerate()
        .map(|(i, &intent_idx)| {
            let intent = &intents[intent_idx];
            let player = &snapshot.players[intent.actor as usize];
            let dist = player.pos.distance_to(&victim_pos);
            let pos = (player.pos.x as f32 / 10.0, player.pos.y as f32 / 10.0);
            (i, dist, intent.utility, pos)
        })
        .collect();

    // 거리 오름차순 → Utility 내림차순 → position-based hash (no team bias)
    with_distance.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| {
                b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| deterministic_tie_hash(0, a.3, 0, b.3)) // FIX_2601/0119
    });

    let winner_local_idx = with_distance[0].0;
    let winner_intent_idx = conflict_indices[winner_local_idx];
    let winner_actor = intents[winner_intent_idx].actor;

    let mut losers = Vec::new();

    // 패자들을 fallback(Cover)으로 변환
    for (i, &intent_idx) in conflict_indices.iter().enumerate() {
        if i != winner_local_idx {
            let loser_actor = intents[intent_idx].actor;
            losers.push(loser_actor);
            intents[intent_idx] = intents[intent_idx].to_fallback();
        }
    }

    ConflictResolution {
        conflict_type: ConflictType::Tackle,
        winner: winner_actor,
        losers,
        resolution_reason: "distance".to_string(),
    }
}

// ============================================================================
// Intent Validation
// ============================================================================

/// 유효하지 않은 Intent 필터링
///
/// InAction/Recovering 상태에서 touches_ball 액션은 무효
pub fn filter_valid_intents(intents: Vec<PlayerIntent>, snapshot: &TickSnapshot) -> Vec<PlayerIntent> {
    intents
        .into_iter()
        .filter(|intent| {
            let player = &snapshot.players[intent.actor as usize];

            // InAction/Recovering 상태에서 touches_ball 액션은 무효
            if matches!(
                player.state,
                PlayerStateTag::InAction | PlayerStateTag::Recovering | PlayerStateTag::Staggered
            ) {
                if intent.meta.touches_ball {
                    return false;
                }
            }

            // 비활성 상태면 모든 액션 무효
            if player.state == PlayerStateTag::Inactive {
                return false;
            }

            true
        })
        .collect()
}

// ============================================================================
// Full Pipeline
// ============================================================================

/// Phase2 전체 파이프라인: 유효성 검사 → 충돌 탐지 → 충돌 해결
///
/// # Arguments
/// * `intents` - Phase1에서 생성된 모든 Intent
/// * `snapshot` - tick 시작 시점 스냅샷
///
/// # Returns
/// * 해결된 Intent 목록 (fallback 포함)
/// * CommitResult 통계
pub fn resolve_all_intents(
    intents: Vec<PlayerIntent>,
    snapshot: &TickSnapshot,
) -> (Vec<PlayerIntent>, CommitResult) {
    // Step 1: 유효성 필터링
    let valid_count = intents.len();
    let valid_intents = filter_valid_intents(intents, snapshot);
    let filtered_count = valid_count - valid_intents.len();

    // Step 2: 충돌 탐지
    let conflicts = detect_conflicts(&valid_intents);

    // Step 3: 충돌 해결
    let (resolved_intents, resolutions) = resolve_conflicts(valid_intents, conflicts, snapshot);

    // Step 4: 통계 계산
    let mut result = CommitResult::default();
    result.discarded = filtered_count as u8;
    result.conflict_resolutions = resolutions;

    for intent in &resolved_intents {
        match intent.kind {
            IntentKind::Pass
            | IntentKind::Through
            | IntentKind::Cross
            | IntentKind::Shoot
            | IntentKind::Dribble
            | IntentKind::Carry
            | IntentKind::Clear
            | IntentKind::Trap
            | IntentKind::Header => {
                result.onball_actions += 1;
            }
            IntentKind::Tackle | IntentKind::Intercept | IntentKind::Block => {
                result.defensive_actions += 1;
            }
            IntentKind::Press
            | IntentKind::Cover
            | IntentKind::OffballRun
            | IntentKind::RecoveryRun
            | IntentKind::Hold => {
                result.positioning_updates += 1;
            }
        }
    }

    (resolved_intents, result)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tick_snapshot::{BallSnap, IntentMeta, OffBallObjectiveSnap, PlayerSnap, TeamSnap};
    use crate::engine::types::Coord10;

    fn create_test_snapshot() -> TickSnapshot {
        use crate::engine::tick_snapshot::{GameModeTag, StickyActionsSnap};

        TickSnapshot {
            tick: 1000,
            minute: 45,
            seed: 12345,
            ball: BallSnap {
                state: BallStateTag::Controlled,
                pos: Coord10::CENTER,
                owner: Some(5),
                target_pos: None,
                eta_tick: None,
                intended_receiver: None,
                is_shot: false,
            },
            players: {
                let mut players = [PlayerSnap::default(); 22];
                for i in 0..22 {
                    players[i].id = i as u8;
                    players[i].is_home = i < 11;
                    players[i].pos = Coord10 { x: 500 + (i as i32 * 20), y: 340, z: 0 };
                    players[i].dist_to_ball = (i as i32 - 5).abs() * 50; // 5번이 가장 가까움
                }
                players
            },
            teams: TeamSnap {
                home_attacks_right: true,
                home_has_possession: true,
            },
            // FIX_2601/0118: Additional fields
            tackle_cooldowns: [0; 22],
            offball_objectives: [OffBallObjectiveSnap::default(); 22],
            last_pass_target: None,
            home_attacks_right: true,
            // FIX_2601 Phase 4: Observation fields
            player_velocities: [(0.0, 0.0); 22],
            score: (0, 0),
            game_mode: GameModeTag::Normal,
            sticky_actions: [StickyActionsSnap::default(); 22],
        }
    }

    fn create_intent(actor: u8, kind: IntentKind, utility: f32, touches_ball: bool) -> PlayerIntent {
        PlayerIntent {
            tick: 1000,
            actor,
            kind,
            target_player: None,
            target_pos: None,
            utility,
            prob: 1.0,
            meta: IntentMeta {
                touches_ball,
                is_selfish: false,
                is_pass_like: kind.is_pass_like(),
                risk: 0.0,
                success_prob: 1.0,
            },
        }
    }

    #[test]
    fn test_detect_ball_touch_conflict() {
        let intents = vec![
            create_intent(5, IntentKind::Pass, 0.8, true),
            create_intent(12, IntentKind::Tackle, 0.6, true),
        ];

        let conflicts = detect_conflicts(&intents);
        assert_eq!(conflicts.len(), 1);

        match &conflicts[0] {
            Conflict::BallTouch { intent_indices } => {
                assert_eq!(intent_indices.len(), 2);
            }
            _ => panic!("Expected BallTouch conflict"),
        }
    }

    #[test]
    fn test_owner_wins_ball_touch() {
        let snapshot = create_test_snapshot(); // owner = 5
        let intents = vec![
            create_intent(5, IntentKind::Pass, 0.5, true),   // owner, low utility
            create_intent(12, IntentKind::Tackle, 0.9, true), // not owner, high utility
        ];

        let conflicts = detect_conflicts(&intents);
        let (resolved, resolutions) = resolve_conflicts(intents, conflicts, &snapshot);

        // Owner wins despite lower utility
        assert_eq!(resolutions.len(), 1);
        assert_eq!(resolutions[0].winner, 5);
        assert_eq!(resolutions[0].losers, vec![12]);
        assert_eq!(resolutions[0].resolution_reason, "owner");

        // Loser converted to fallback (Tackle -> Cover)
        assert_eq!(resolved[1].kind, IntentKind::Cover);
        assert!(!resolved[1].meta.touches_ball);
    }

    #[test]
    fn test_eta_wins_on_loose_ball() {
        let mut snapshot = create_test_snapshot();
        snapshot.ball.state = BallStateTag::Loose;
        snapshot.ball.owner = None;

        // Player 3 is closer (dist=100), Player 15 is farther (dist=500)
        snapshot.players[3].dist_to_ball = 100;
        snapshot.players[15].dist_to_ball = 500;

        let intents = vec![
            create_intent(3, IntentKind::Intercept, 0.5, true),
            create_intent(15, IntentKind::Intercept, 0.9, true),
        ];

        let conflicts = detect_conflicts(&intents);
        let (_, resolutions) = resolve_conflicts(intents, conflicts, &snapshot);

        // Closer player wins
        assert_eq!(resolutions[0].winner, 3);
        assert_eq!(resolutions[0].resolution_reason, "eta");
    }

    #[test]
    fn test_utility_wins_when_equal_distance() {
        let mut snapshot = create_test_snapshot();
        snapshot.ball.state = BallStateTag::Loose;
        snapshot.ball.owner = None;

        // Same distance
        snapshot.players[3].dist_to_ball = 100;
        snapshot.players[15].dist_to_ball = 100;

        let intents = vec![
            create_intent(3, IntentKind::Intercept, 0.5, true),
            create_intent(15, IntentKind::Intercept, 0.9, true),
        ];

        let conflicts = detect_conflicts(&intents);
        let (_, resolutions) = resolve_conflicts(intents, conflicts, &snapshot);

        // Higher utility wins
        assert_eq!(resolutions[0].winner, 15);
    }

    #[test]
    fn test_position_tiebreaker() {
        // FIX_2601/0119: Test position-based tie-breaking (no team bias)
        let mut snapshot = create_test_snapshot();
        snapshot.ball.state = BallStateTag::Loose;
        snapshot.ball.owner = None;

        // Same distance and utility
        snapshot.players[3].dist_to_ball = 100;
        snapshot.players[15].dist_to_ball = 100;

        let intents = vec![
            create_intent(3, IntentKind::Intercept, 0.8, true),
            create_intent(15, IntentKind::Intercept, 0.8, true),
        ];

        let conflicts = detect_conflicts(&intents);
        let (_, resolutions) = resolve_conflicts(intents, conflicts, &snapshot);

        // Position-based deterministic tie-breaking (FIX_2601/0119)
        // Winner is determined by position hash, not track_id
        // Just verify determinism: same inputs produce same outputs
        assert_eq!(resolutions.len(), 1);
        assert!(
            resolutions[0].winner == 3 || resolutions[0].winner == 15,
            "Winner should be one of the participants"
        );

        // Verify determinism: run again with same inputs
        let intents2 = vec![
            create_intent(3, IntentKind::Intercept, 0.8, true),
            create_intent(15, IntentKind::Intercept, 0.8, true),
        ];
        let conflicts2 = detect_conflicts(&intents2);
        let (_, resolutions2) = resolve_conflicts(intents2, conflicts2, &snapshot);

        assert_eq!(resolutions[0].winner, resolutions2[0].winner, "Tie-breaker should be deterministic");
    }

    #[test]
    fn test_filter_invalid_intents() {
        let mut snapshot = create_test_snapshot();
        snapshot.players[5].state = PlayerStateTag::InAction;

        let intents = vec![
            create_intent(5, IntentKind::Pass, 0.8, true),  // InAction, touches_ball -> filtered
            create_intent(5, IntentKind::Hold, 0.3, false), // InAction, no touch -> valid
            create_intent(6, IntentKind::Pass, 0.7, true),  // Idle -> valid
        ];

        let valid = filter_valid_intents(intents, &snapshot);
        assert_eq!(valid.len(), 2);
        assert_eq!(valid[0].kind, IntentKind::Hold);
        assert_eq!(valid[1].kind, IntentKind::Pass);
    }

    #[test]
    fn test_tackle_conflict_resolution() {
        let snapshot = create_test_snapshot();

        // Two players tackle the same victim (player 10)
        let mut intents = vec![
            PlayerIntent {
                tick: 1000,
                actor: 12,
                kind: IntentKind::Tackle,
                target_player: Some(10),
                target_pos: None,
                utility: 0.7,
                prob: 1.0,
                meta: IntentMeta {
                    touches_ball: true,
                    ..Default::default()
                },
            },
            PlayerIntent {
                tick: 1000,
                actor: 13,
                kind: IntentKind::Tackle,
                target_player: Some(10),
                target_pos: None,
                utility: 0.8,
                prob: 1.0,
                meta: IntentMeta {
                    touches_ball: true,
                    ..Default::default()
                },
            },
        ];

        let conflicts = detect_conflicts(&intents);
        assert_eq!(conflicts.len(), 2); // 1 BallTouch + 1 Tackle

        // Find tackle conflict
        let tackle_conflict = conflicts
            .iter()
            .find(|c| matches!(c, Conflict::Tackle { .. }));
        assert!(tackle_conflict.is_some());
    }
}
