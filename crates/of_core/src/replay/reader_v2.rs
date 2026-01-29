//! ReplayReader v2 - JSON 파싱 및 검증
//!
//! FIX_2512 Phase 3 - TASK_07

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::replay::format_v2::*;

/// ReplayV2 JSON 파일 로드
///
/// JSON 파일을 읽어 ReplayV2로 역직렬화하고, Audit Gates 검증을 수행합니다.
///
/// # Arguments
/// * `path` - 리플레이 파일 경로
///
/// # Returns
/// 검증된 ReplayV2 구조체
///
/// # Errors
/// - 파일 읽기 실패
/// - JSON 파싱 실패
/// - Audit Gates 검증 실패 (좌표 범위, track_id, velocity 등)
///
/// # Example
/// ```ignore
/// let replay = load_replay_v2_json("replay.json")?;
/// println!("Loaded {} frames", replay.save_frames.len());
/// ```
pub fn load_replay_v2_json(path: impl AsRef<Path>) -> Result<ReplayV2> {
    let path = path.as_ref();

    // 파일 읽기
    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read replay file: {:?}", path))?;

    // JSON 파싱
    let replay: ReplayV2 =
        serde_json::from_str(&data).with_context(|| "Failed to parse ReplayV2 JSON")?;

    // 검증
    validate_replay_v2(&replay)?;

    Ok(replay)
}

/// ReplayV2 검증 (Audit Gates)
///
/// FIX_2512 Audit Gates를 적용하여 리플레이 무결성을 검증합니다.
fn validate_replay_v2(replay: &ReplayV2) -> Result<()> {
    // Version 체크
    if replay.version != 2 {
        anyhow::bail!("Invalid replay version: expected 2, got {}", replay.version);
    }

    // 메타 검증
    validate_meta(&replay.meta)?;

    // 프레임 검증
    for (i, frame) in replay.save_frames.iter().enumerate() {
        validate_frame(frame, &replay.meta, i)
            .with_context(|| format!("Frame {} validation failed", i))?;
    }

    // 이벤트 검증
    for (i, event) in replay.events.iter().enumerate() {
        validate_event(event, &replay.meta, i)
            .with_context(|| format!("Event {} validation failed", i))?;
    }

    Ok(())
}

/// 메타데이터 검증
fn validate_meta(meta: &ReplayMetaV2) -> Result<()> {
    // coord_unit_mm
    if meta.coord_unit_mm != 100 {
        anyhow::bail!("Invalid coord_unit_mm: expected 100, got {}", meta.coord_unit_mm);
    }

    // tick 범위
    if meta.sim_tick_ms < 10 || meta.sim_tick_ms > 100 {
        anyhow::bail!("Invalid sim_tick_ms: {} (valid: 10-100)", meta.sim_tick_ms);
    }

    if meta.save_tick_ms < 50 || meta.save_tick_ms > 500 {
        anyhow::bail!("Invalid save_tick_ms: {} (valid: 50-500)", meta.save_tick_ms);
    }

    // 필드 크기
    if meta.field_x_max < 900 || meta.field_x_max > 1200 {
        anyhow::bail!("Invalid field_x_max: {} (valid: 900-1200)", meta.field_x_max);
    }

    if meta.field_y_max < 600 || meta.field_y_max > 800 {
        anyhow::bail!("Invalid field_y_max: {} (valid: 600-800)", meta.field_y_max);
    }

    // 엔티티 수
    if meta.track_count != 23 {
        anyhow::bail!("Invalid track_count: expected 23, got {}", meta.track_count);
    }

    Ok(())
}

/// 프레임 검증
fn validate_frame(frame: &SaveFrameV2, meta: &ReplayMetaV2, frame_idx: usize) -> Result<()> {
    // 엔티티 수 체크
    if frame.entities.len() != 23 {
        anyhow::bail!(
            "[Frame {}] Invalid entity count: expected 23, got {}",
            frame_idx,
            frame.entities.len()
        );
    }

    // entities[0] = ball (A1: Ball coordinates)
    validate_entity(&frame.entities[0], meta, frame_idx, 0, true)?;

    // entities[1..22] = players (A2: Player coordinates)
    for (i, entity) in frame.entities[1..].iter().enumerate() {
        let track_id = i + 1;
        validate_entity(entity, meta, frame_idx, track_id, false)?;
    }

    Ok(())
}

/// 엔티티 검증 (Audit Gates A1, A2, A4)
fn validate_entity(
    entity: &EntitySnapV2,
    meta: &ReplayMetaV2,
    frame_idx: usize,
    track_id: usize,
    is_ball: bool,
) -> Result<()> {
    let entity_name = if is_ball { "Ball".to_string() } else { format!("Player {}", track_id - 1) };

    // A1/A2: Coordinate range check (HARD gate)
    let x_min = -10; // -1.0m
    let x_max = meta.field_x_max + 10;
    let y_min = -10;
    let y_max = meta.field_y_max + 10;

    if entity.x10 < x_min || entity.x10 > x_max as i16 {
        anyhow::bail!(
            "[AUDIT-A{}-HARD] Frame {}, {}: x10 out of range ({} not in [{}, {}])",
            if is_ball { 1 } else { 2 },
            frame_idx,
            entity_name,
            entity.x10,
            x_min,
            x_max
        );
    }

    if entity.y10 < y_min || entity.y10 > y_max as i16 {
        anyhow::bail!(
            "[AUDIT-A{}-HARD] Frame {}, {}: y10 out of range ({} not in [{}, {}])",
            if is_ball { 1 } else { 2 },
            frame_idx,
            entity_name,
            entity.y10,
            y_min,
            y_max
        );
    }

    // A4: Velocity magnitude check (HARD gate)
    let vel_mag_sq = (entity.vx10 as i32).pow(2) + (entity.vy10 as i32).pow(2);
    let max_vel_sq = if is_ball {
        200 * 200 // 20 m/s for ball
    } else {
        100 * 100 // 10 m/s for player
    };

    if vel_mag_sq > max_vel_sq {
        let vel_mag = (vel_mag_sq as f32).sqrt() / 10.0;
        anyhow::bail!(
            "[AUDIT-A4-HARD] Frame {}, {}: velocity too high ({:.1} m/s, max: {})",
            frame_idx,
            entity_name,
            vel_mag,
            if is_ball { 20 } else { 10 }
        );
    }

    Ok(())
}

/// 이벤트 검증 (Audit Gates A3)
fn validate_event(event: &ReplayEventV2, meta: &ReplayMetaV2, event_idx: usize) -> Result<()> {
    // A3: track_id integrity (HARD gate)
    if event.a >= meta.track_count {
        anyhow::bail!(
            "[AUDIT-A3-HARD] Event {}: invalid track_id a={} (max={})",
            event_idx,
            event.a,
            meta.track_count - 1
        );
    }

    if event.b >= meta.track_count && event.b != 255 {
        // 255 = N/A
        anyhow::bail!(
            "[AUDIT-A3-HARD] Event {}: invalid track_id b={} (max={})",
            event_idx,
            event.b,
            meta.track_count - 1
        );
    }

    // 좌표 범위 체크
    let x_min = -10;
    let x_max = meta.field_x_max + 10;
    let y_min = -10;
    let y_max = meta.field_y_max + 10;

    if event.x10 < x_min || event.x10 > x_max as i16 {
        anyhow::bail!(
            "[AUDIT-A1-HARD] Event {}: x10 out of range ({} not in [{}, {}])",
            event_idx,
            event.x10,
            x_min,
            x_max
        );
    }

    if event.y10 < y_min || event.y10 > y_max as i16 {
        anyhow::bail!(
            "[AUDIT-A1-HARD] Event {}: y10 out of range ({} not in [{}, {}])",
            event_idx,
            event.y10,
            y_min,
            y_max
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_replay() -> ReplayV2 {
        let meta = ReplayMetaV2 {
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
        };

        let frame = SaveFrameV2 { t_ms: 1000, entities: [EntitySnapV2::default(); 23] };

        let event = ReplayEventV2::new_goal(5000, 9, 525, 340);

        ReplayV2 { version: 2, meta, save_frames: vec![frame], events: vec![event] }
    }

    #[test]
    fn test_validate_replay_v2_success() {
        let replay = create_test_replay();
        assert!(validate_replay_v2(&replay).is_ok());
    }

    #[test]
    fn test_validate_invalid_version() {
        let mut replay = create_test_replay();
        replay.version = 1;
        assert!(validate_replay_v2(&replay).is_err());
    }

    #[test]
    fn test_validate_invalid_coord_unit() {
        let mut replay = create_test_replay();
        replay.meta.coord_unit_mm = 50;
        let result = validate_replay_v2(&replay);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("coord_unit_mm"));
    }

    /// Helper: Check if error chain contains a string (anyhow context issue)
    fn error_chain_contains(err: &anyhow::Error, needle: &str) -> bool {
        // Check root error
        if format!("{:?}", err).contains(needle) {
            return true;
        }
        // Check each error in chain
        for cause in err.chain() {
            if cause.to_string().contains(needle) {
                return true;
            }
        }
        false
    }

    #[test]
    fn test_validate_ball_coordinates_out_of_range() {
        let mut replay = create_test_replay();
        replay.save_frames[0].entities[0].x10 = 5000; // 500.0m (out of range)

        let result = validate_replay_v2(&replay);
        assert!(result.is_err(), "Expected error for out-of-range ball coordinates");
        let err = result.unwrap_err();
        assert!(
            error_chain_contains(&err, "AUDIT-A1-HARD"),
            "Error should contain AUDIT-A1-HARD, got: {:?}",
            err
        );
    }

    #[test]
    fn test_validate_player_coordinates_out_of_range() {
        let mut replay = create_test_replay();
        replay.save_frames[0].entities[1].y10 = -500; // -50.0m (out of range)

        let result = validate_replay_v2(&replay);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            error_chain_contains(&err, "AUDIT-A2-HARD"),
            "Error should contain AUDIT-A2-HARD, got: {:?}",
            err
        );
    }

    #[test]
    fn test_validate_velocity_too_high() {
        let mut replay = create_test_replay();
        replay.save_frames[0].entities[0].vx10 = 300; // 30 m/s (too fast)

        let result = validate_replay_v2(&replay);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            error_chain_contains(&err, "AUDIT-A4-HARD"),
            "Error should contain AUDIT-A4-HARD, got: {:?}",
            err
        );
    }

    #[test]
    fn test_validate_invalid_track_id() {
        let mut replay = create_test_replay();
        replay.events[0].a = 99; // out of range (max=22)

        let result = validate_replay_v2(&replay);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            error_chain_contains(&err, "AUDIT-A3-HARD"),
            "Error should contain AUDIT-A3-HARD, got: {:?}",
            err
        );
    }
}
