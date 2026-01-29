//! Replay Format v2: 정수 좌표 + 이중 레이트
//!
//! FIX_2512 Phase 2 - TASK_05
//!
//! ## 설계 목표
//! - Coord10 기반 정수 좌표 (부동소수점 오차 제거)
//! - 50ms sim tick + 100~200ms save tick (메모리 효율)
//! - 결정론적 재생 보장
//! - JSON 직렬화 지원

use crate::engine::types::coord10::{Coord10, Vel10};
use serde::{Deserialize, Serialize};

// ============================================================================
// ReplayV2: 루트 구조체
// ============================================================================

/// Replay Format v2 (정수 좌표 + 이중 레이트)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayV2 {
    /// 버전 (2)
    pub version: u8,

    /// 메타데이터
    pub meta: ReplayMetaV2,

    /// 저장 프레임 (100~200ms 간격)
    pub save_frames: Vec<SaveFrameV2>,

    /// 이벤트 타임라인
    pub events: Vec<ReplayEventV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayMetaV2 {
    /// 좌표 단위 (mm) - 100 = 0.1m
    pub coord_unit_mm: u16,

    /// 시뮬레이션 tick (ms)
    pub sim_tick_ms: u8,

    /// 뷰어 재생 tick (ms)
    pub view_tick_ms: u8,

    /// 저장 주기 (ms)
    pub save_tick_ms: u16,

    /// 필드 크기 (0.1m 단위)
    pub field_x_max: i32,
    pub field_y_max: i32,

    /// 엔티티 수 (ball + players)
    pub track_count: u8,

    /// 매치 정보
    pub match_info: MatchInfoV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchInfoV2 {
    pub seed: u64,
    pub score_home: u8,
    pub score_away: u8,
    pub duration_minutes: u8,
}

// ============================================================================
// SaveFrameV2: 스냅샷
// ============================================================================

/// 저장 프레임 (100~200ms 간격)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFrameV2 {
    /// 시간 (ms)
    pub t_ms: u32,

    /// 엔티티 (ball + 22 players)
    pub entities: [EntitySnapV2; 23],
}

/// 엔티티 스냅샷 (정수 좌표)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct EntitySnapV2 {
    /// 좌표 (0.1m 단위)
    pub x10: i16,
    pub y10: i16,

    /// 속도 (0.1m/s 단위)
    pub vx10: i16,
    pub vy10: i16,

    /// 상태 (0=idle, 1=run, 2=dribble, 3=tackle, ...)
    pub state: u8,

    /// 플래그 (bit 0=has_ball, bit 1=injured, ...)
    pub flags: u8,

    /// 디버그/웨이포인트 (옵션)
    pub wx10: i16,
    pub wy10: i16,
}

impl EntitySnapV2 {
    /// Coord10 → EntitySnapV2
    pub fn from_coord(coord: Coord10, vel: Vel10) -> Self {
        Self {
            x10: coord.x as i16,
            y10: coord.y as i16,
            vx10: vel.vx as i16,
            vy10: vel.vy as i16,
            ..Default::default()
        }
    }

    /// EntitySnapV2 → Coord10
    pub fn to_coord(&self) -> Coord10 {
        Coord10 { x: self.x10 as i32, y: self.y10 as i32, z: 0 }
    }

    /// EntitySnapV2 → Vel10
    pub fn to_vel(&self) -> Vel10 {
        Vel10 { vx: self.vx10 as i32, vy: self.vy10 as i32 }
    }
}

// ============================================================================
// ReplayEventV2: 이벤트
// ============================================================================

/// 리플레이 이벤트 (상태 변화 시점만 기록)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEventV2 {
    /// 시간 (ms)
    pub t_ms: u32,

    /// 이벤트 종류 (0=goal, 1=tackle, 2=pass, 3=shot, ...)
    pub kind: u16,

    /// 주체/객체 track_id
    pub a: u8,
    pub b: u8,

    /// 위치 (0.1m 단위)
    pub x10: i16,
    pub y10: i16,

    /// 보조 데이터 (정수 스케일)
    /// - distance: 0.1m 단위
    /// - power: 0..1000 (0.001 단위)
    /// - xg: 0..1000 (0.001 단위)
    pub aux: [i16; 4],
}

impl ReplayEventV2 {
    pub fn new_goal(t_ms: u32, scorer_id: u8, x10: i16, y10: i16) -> Self {
        Self {
            t_ms,
            kind: 0, // goal
            a: scorer_id,
            b: 0,
            x10,
            y10,
            aux: [0; 4],
        }
    }

    pub fn new_pass(t_ms: u32, passer_id: u8, receiver_id: u8, distance_m: f32) -> Self {
        Self {
            t_ms,
            kind: 2, // pass
            a: passer_id,
            b: receiver_id,
            x10: 0,
            y10: 0,
            aux: [(distance_m * 10.0) as i16, 0, 0, 0],
        }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_snap_roundtrip() {
        let coord = Coord10::from_meters(52.5, 34.0);
        let vel = Vel10::from_mps(7.2, 0.0);

        let snap = EntitySnapV2::from_coord(coord, vel);

        assert_eq!(snap.x10, 525);
        assert_eq!(snap.y10, 340);
        assert_eq!(snap.vx10, 72);
        assert_eq!(snap.vy10, 0);

        let coord2 = snap.to_coord();
        assert_eq!(coord, coord2);
    }

    #[test]
    fn test_replay_v2_serde() {
        let replay = ReplayV2 {
            version: 2,
            meta: ReplayMetaV2 {
                coord_unit_mm: 100,
                sim_tick_ms: 50,
                view_tick_ms: 50,
                save_tick_ms: 100,
                field_x_max: 1050,
                field_y_max: 680,
                track_count: 23,
                match_info: MatchInfoV2 {
                    seed: 12345,
                    score_home: 2,
                    score_away: 1,
                    duration_minutes: 90,
                },
            },
            save_frames: vec![],
            events: vec![],
        };

        let json = serde_json::to_string(&replay).unwrap();
        let replay2: ReplayV2 = serde_json::from_str(&json).unwrap();

        assert_eq!(replay.version, replay2.version);
        assert_eq!(replay.meta.save_tick_ms, replay2.meta.save_tick_ms);
    }

    #[test]
    fn test_replay_event_goal() {
        let event = ReplayEventV2::new_goal(45000, 9, 525, 340);

        assert_eq!(event.t_ms, 45000); // 45초
        assert_eq!(event.kind, 0); // goal
        assert_eq!(event.a, 9); // scorer track_id
        assert_eq!(event.x10, 525); // 52.5m
    }
}
