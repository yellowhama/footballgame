//! ReplayWriter v2 - Coord10 기반 스냅샷 기록
//!
//! FIX_2512 Phase 3 - TASK_06

use crate::replay::format_v2::*;

/// Replay v2 Writer
///
/// MatchEngine에서 50ms 틱마다 엔티티 상태를 기록하여
/// ReplayV2 포맷으로 저장합니다.
pub struct ReplayWriterV2 {
    /// 메타데이터
    meta: ReplayMetaV2,

    /// 저장 프레임 (100~200ms 간격)
    save_frames: Vec<SaveFrameV2>,

    /// 이벤트 타임라인
    events: Vec<ReplayEventV2>,

    /// 마지막 저장 tick (ms)
    last_save_tick: u32,
}

impl ReplayWriterV2 {
    /// 새로운 ReplayWriter 생성
    ///
    /// # Arguments
    /// * `meta` - 리플레이 메타데이터 (필드 크기, tick rate 등)
    pub fn new(meta: ReplayMetaV2) -> Self {
        Self {
            meta,
            save_frames: Vec::with_capacity(60000), // 90분 @ 100ms = ~54,000 프레임
            events: Vec::with_capacity(500),        // 이벤트 ~200개 예상
            last_save_tick: 0,
        }
    }

    /// 스냅샷 프레임 추가
    ///
    /// # Arguments
    /// * `t_ms` - 현재 시간 (ms)
    /// * `entities` - Ball + 22 Players 스냅샷 (23개)
    ///
    /// # Example
    /// ```ignore
    /// let entities = [EntitySnapV2::default(); 23];
    /// writer.add_frame(1000, entities);  // 1초 시점 기록
    /// ```
    pub fn add_frame(&mut self, t_ms: u32, entities: [EntitySnapV2; 23]) {
        self.save_frames.push(SaveFrameV2 { t_ms, entities });
        self.last_save_tick = t_ms;
    }

    /// 이벤트 추가
    ///
    /// # Arguments
    /// * `event` - 리플레이 이벤트 (goal, pass, shot 등)
    ///
    /// # Example
    /// ```ignore
    /// let event = ReplayEventV2::new_goal(5000, 9, 525, 340);
    /// writer.add_event(event);
    /// ```
    pub fn add_event(&mut self, event: ReplayEventV2) {
        self.events.push(event);
    }

    /// 매치 종료 시 score 업데이트
    ///
    /// # Arguments
    /// * `score_home` - 홈팀 득점
    /// * `score_away` - 원정팀 득점
    pub fn set_final_score(&mut self, score_home: u8, score_away: u8) {
        self.meta.match_info.score_home = score_home;
        self.meta.match_info.score_away = score_away;
    }

    /// ReplayV2 완성 및 반환
    ///
    /// Writer를 소비하고 최종 ReplayV2 구조체를 반환합니다.
    ///
    /// # Returns
    /// 완성된 ReplayV2 객체
    pub fn finalize(self) -> ReplayV2 {
        ReplayV2 { version: 2, meta: self.meta, save_frames: self.save_frames, events: self.events }
    }

    /// 통계 정보 반환
    pub fn stats(&self) -> WriterStats {
        WriterStats {
            frame_count: self.save_frames.len(),
            event_count: self.events.len(),
            duration_ms: self.last_save_tick,
            estimated_size_mb: self.estimate_size_mb(),
        }
    }

    /// 예상 파일 크기 계산 (MB)
    fn estimate_size_mb(&self) -> f32 {
        // EntitySnapV2: 16 bytes × 23 = 368 bytes per frame
        let frame_bytes = self.save_frames.len() * 368;

        // ReplayEventV2: ~20 bytes per event
        let event_bytes = self.events.len() * 20;

        // 메타 + 오버헤드: ~1KB
        let total_bytes = frame_bytes + event_bytes + 1024;

        total_bytes as f32 / (1024.0 * 1024.0)
    }
}

/// Writer 통계
#[derive(Debug, Clone)]
pub struct WriterStats {
    /// 저장된 프레임 수
    pub frame_count: usize,

    /// 저장된 이벤트 수
    pub event_count: usize,

    /// 리플레이 길이 (ms)
    pub duration_ms: u32,

    /// 예상 파일 크기 (MB)
    pub estimated_size_mb: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_meta() -> ReplayMetaV2 {
        ReplayMetaV2 {
            coord_unit_mm: 100,
            sim_tick_ms: 50,
            view_tick_ms: 50,
            save_tick_ms: 100,
            field_x_max: 1050,
            field_y_max: 680,
            track_count: 23,
            match_info: MatchInfoV2 {
                seed: 12345,
                score_home: 0,
                score_away: 0,
                duration_minutes: 90,
            },
        }
    }

    #[test]
    fn test_writer_new() {
        let meta = create_test_meta();
        let writer = ReplayWriterV2::new(meta);

        assert_eq!(writer.save_frames.len(), 0);
        assert_eq!(writer.events.len(), 0);
        assert_eq!(writer.last_save_tick, 0);
    }

    #[test]
    fn test_add_frame() {
        let meta = create_test_meta();
        let mut writer = ReplayWriterV2::new(meta);

        let entities = [EntitySnapV2::default(); 23];
        writer.add_frame(1000, entities);

        assert_eq!(writer.save_frames.len(), 1);
        assert_eq!(writer.last_save_tick, 1000);
    }

    #[test]
    fn test_add_event() {
        let meta = create_test_meta();
        let mut writer = ReplayWriterV2::new(meta);

        let event = ReplayEventV2::new_goal(5000, 9, 525, 340);
        writer.add_event(event);

        assert_eq!(writer.events.len(), 1);
        assert_eq!(writer.events[0].kind, 0); // goal
        assert_eq!(writer.events[0].a, 9);
    }

    #[test]
    fn test_finalize() {
        let meta = create_test_meta();
        let mut writer = ReplayWriterV2::new(meta);

        let entities = [EntitySnapV2::default(); 23];
        writer.add_frame(1000, entities);

        let event = ReplayEventV2::new_goal(5000, 9, 525, 340);
        writer.add_event(event);

        writer.set_final_score(2, 1);

        let replay = writer.finalize();

        assert_eq!(replay.version, 2);
        assert_eq!(replay.save_frames.len(), 1);
        assert_eq!(replay.events.len(), 1);
        assert_eq!(replay.meta.match_info.score_home, 2);
        assert_eq!(replay.meta.match_info.score_away, 1);
    }

    #[test]
    fn test_stats() {
        let meta = create_test_meta();
        let mut writer = ReplayWriterV2::new(meta);

        // 10초 시뮬레이션 (100개 프레임 @ 100ms)
        for i in 0..100 {
            let entities = [EntitySnapV2::default(); 23];
            writer.add_frame(i * 100, entities);
        }

        let stats = writer.stats();
        assert_eq!(stats.frame_count, 100);
        assert_eq!(stats.duration_ms, 9900);
        assert!(stats.estimated_size_mb > 0.0);
        assert!(stats.estimated_size_mb < 0.1); // 10초 = ~36KB
    }
}
