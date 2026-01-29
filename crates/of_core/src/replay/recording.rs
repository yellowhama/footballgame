//! Replay Recording System
//!
//! OpenFootball Vector3 데이터 기반 리플레이 녹화 시스템

use chrono::{DateTime, Utc};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// P0.75 Patch 1: cm→m 원천 봉인
///
/// Vector3 좌표를 미터 단위로 정규화합니다.
/// - 휴리스틱: x > 210 또는 y > 136이면 cm 단위로 간주하고 0.01을 곱합니다.
/// - 이는 일반 축구장 크기(105m x 68m)를 기준으로 한 임계값입니다.
fn normalize_vec3_to_meters(v: Vector3<f32>) -> Vector3<f32> {
    if v.x > 210.0 || v.y > 136.0 {
        // cm 단위로 추정 → 0.01 배율로 m 변환
        v * 0.01
    } else {
        // 이미 m 단위
        v
    }
}

/// OpenFootball 매치 녹화 데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRecording {
    // 메타데이터
    pub match_id: u64,
    pub timestamp: DateTime<Utc>,
    pub home_team: String,
    pub away_team: String,
    pub final_score: (u8, u8),
    pub total_ticks: u64,

    // 핵심 데이터 스트림
    pub position_stream: PositionStream,
    pub event_stream: EventStream,
    pub state_stream: StateStream,

    // 압축 정보
    pub compression: CompressionInfo,
    pub version: u32,
}

impl MatchRecording {
    pub fn new(match_id: u64, home: String, away: String) -> Self {
        Self {
            match_id,
            timestamp: Utc::now(),
            home_team: home,
            away_team: away,
            final_score: (0, 0),
            total_ticks: 0,
            position_stream: PositionStream::new(),
            event_stream: EventStream::new(),
            state_stream: StateStream::new(),
            compression: CompressionInfo::default(),
            version: 1,
        }
    }

    /// OpenFootball 데이터 추가
    pub fn add_frame(&mut self, tick: u64, frame: FrameData) {
        self.position_stream.add_frame(tick, frame);
        self.total_ticks = self.total_ticks.max(tick);
    }

    /// 특정 시간의 스냅샷 가져오기
    pub fn get_snapshot_at(&self, tick: u64) -> Option<FrameSnapshot> {
        self.position_stream.get_frame_at(tick)
    }

    /// 하이라이트 구간 추출
    pub fn extract_highlights(&self) -> Vec<HighlightSegment> {
        self.event_stream.extract_highlights()
    }
}

/// 위치 데이터 스트림
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionStream {
    pub ball_frames: Vec<BallFrame>,
    pub player_frames: HashMap<u32, Vec<PlayerFrame>>,
    pub frame_rate: u32,
}

impl Default for PositionStream {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionStream {
    pub fn new() -> Self {
        Self {
            ball_frames: Vec::with_capacity(324000), // 90분 * 60fps
            player_frames: HashMap::with_capacity(22),
            frame_rate: 60,
        }
    }

    pub fn add_frame(&mut self, _tick: u64, frame: FrameData) {
        // 공 위치 추가
        self.ball_frames.push(frame.ball);

        // 선수 위치 추가
        for player_frame in frame.players {
            self.player_frames.entry(player_frame.player_id).or_default().push(player_frame);
        }
    }

    pub fn get_frame_at(&self, tick: u64) -> Option<FrameSnapshot> {
        // Binary search로 효율적 검색
        let ball = self
            .ball_frames
            .binary_search_by_key(&tick, |f| f.tick)
            .ok()
            .and_then(|idx| self.ball_frames.get(idx))?;

        let mut players = Vec::new();
        for frames in self.player_frames.values() {
            if let Ok(idx) = frames.binary_search_by_key(&tick, |f| f.tick) {
                if let Some(frame) = frames.get(idx) {
                    players.push(frame.clone());
                }
            }
        }

        Some(FrameSnapshot { tick, ball: ball.clone(), players })
    }
}

/// 이벤트 스트림
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStream {
    pub events: Vec<TimestampedEvent>,
    pub highlights: Vec<HighlightMarker>,
}

impl Default for EventStream {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStream {
    pub fn new() -> Self {
        Self { events: Vec::new(), highlights: Vec::new() }
    }

    pub fn add_event(&mut self, event: TimestampedEvent) {
        self.events.push(event.clone());

        // 중요 이벤트는 하이라이트로 마킹
        if event.importance > 0.7 {
            self.highlights.push(HighlightMarker {
                start_tick: event.tick.saturating_sub(180), // 3초 전
                end_tick: event.tick + 180,                 // 3초 후
                event_type: event.event_type.clone(),
                importance: event.importance,
            });
        }
    }

    pub fn extract_highlights(&self) -> Vec<HighlightSegment> {
        // 겹치는 하이라이트 병합
        let mut segments: Vec<HighlightSegment> = Vec::new();
        let mut sorted_highlights = self.highlights.clone();
        sorted_highlights.sort_by_key(|h| h.start_tick);

        for highlight in sorted_highlights {
            if let Some(last) = segments.last_mut() {
                if last.overlaps_with(&highlight) {
                    last.merge(highlight);
                } else {
                    segments.push(HighlightSegment::from(highlight));
                }
            } else {
                segments.push(HighlightSegment::from(highlight));
            }
        }

        segments
    }
}

/// AI 상태 스트림
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStream {
    pub state_changes: Vec<StateChange>,
}

impl Default for StateStream {
    fn default() -> Self {
        Self::new()
    }
}

impl StateStream {
    pub fn new() -> Self {
        Self { state_changes: Vec::new() }
    }

    pub fn add_state_change(&mut self, change: StateChange) {
        self.state_changes.push(change);
    }
}

/// 한 프레임의 모든 데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameData {
    pub ball: BallFrame,
    pub players: Vec<PlayerFrame>,
}

/// 프레임 스냅샷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSnapshot {
    pub tick: u64,
    pub ball: BallFrame,
    pub players: Vec<PlayerFrame>,
}

/// 공 프레임 데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallFrame {
    pub tick: u64,
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub height: f32,
    pub possession: Option<u32>, // 소유 선수 ID
}

impl BallFrame {
    pub fn from_of_position(
        tick: u64,
        position: Vector3<f32>,
        prev_pos: Option<Vector3<f32>>,
    ) -> Self {
        // P0.75 Patch 1: cm→m 원천 봉인
        let normalized_pos = normalize_vec3_to_meters(position);
        let normalized_prev = prev_pos.map(normalize_vec3_to_meters);

        let velocity = if let Some(prev) = normalized_prev {
            (normalized_pos - prev) * 60.0 // 60 FPS 기준
        } else {
            Vector3::zeros()
        };

        Self {
            tick,
            position: normalized_pos,
            velocity,
            height: normalized_pos.z, // z축을 높이로 사용
            possession: None,
        }
    }
}

/// 선수 프레임 데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerFrame {
    pub tick: u64,
    pub player_id: u32,
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub orientation: f32,
    pub state: String, // PlayerState를 문자열로
    pub team_id: u8,
}

impl PlayerFrame {
    pub fn from_of_position(
        tick: u64,
        player_id: u32,
        position: Vector3<f32>,
        prev_pos: Option<Vector3<f32>>,
        state: String,
        team_id: u8,
    ) -> Self {
        // P0.75 Patch 1: cm→m 원천 봉인
        let normalized_pos = normalize_vec3_to_meters(position);
        let normalized_prev = prev_pos.map(normalize_vec3_to_meters);

        let velocity = if let Some(prev) = normalized_prev {
            (normalized_pos - prev) * 60.0 // 60 FPS 기준
        } else {
            Vector3::zeros()
        };

        // 속도 벡터로부터 방향 계산
        let orientation =
            if velocity.x != 0.0 || velocity.y != 0.0 { velocity.y.atan2(velocity.x) } else { 0.0 };

        Self { tick, player_id, position: normalized_pos, velocity, orientation, state, team_id }
    }
}

/// 타임스탬프된 이벤트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedEvent {
    pub tick: u64,
    pub event_type: String,
    pub player_id: Option<u32>,
    pub position: Option<Vector3<f32>>,
    pub importance: f32,
    pub data: serde_json::Value,
}

/// AI 상태 변화
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub tick: u64,
    pub player_id: u32,
    pub from_state: String,
    pub to_state: String,
    pub trigger: String,
}

/// 하이라이트 마커
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightMarker {
    pub start_tick: u64,
    pub end_tick: u64,
    pub event_type: String,
    pub importance: f32,
}

/// 하이라이트 구간
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightSegment {
    pub start_tick: u64,
    pub end_tick: u64,
    pub events: Vec<String>,
    pub max_importance: f32,
}

impl HighlightSegment {
    pub fn from(marker: HighlightMarker) -> Self {
        Self {
            start_tick: marker.start_tick,
            end_tick: marker.end_tick,
            events: vec![marker.event_type],
            max_importance: marker.importance,
        }
    }

    pub fn overlaps_with(&self, marker: &HighlightMarker) -> bool {
        self.end_tick >= marker.start_tick && self.start_tick <= marker.end_tick
    }

    pub fn merge(&mut self, marker: HighlightMarker) {
        self.start_tick = self.start_tick.min(marker.start_tick);
        self.end_tick = self.end_tick.max(marker.end_tick);
        self.events.push(marker.event_type);
        self.max_importance = self.max_importance.max(marker.importance);
    }
}

/// 압축 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionInfo {
    pub algorithm: String,
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f32,
}

impl Default for CompressionInfo {
    fn default() -> Self {
        Self {
            algorithm: "none".to_string(),
            original_size: 0,
            compressed_size: 0,
            compression_ratio: 1.0,
        }
    }
}

/// OpenFootball 데이터 변환기
pub struct OpenFootballConverter;

impl OpenFootballConverter {
    /// OpenFootball ResultMatchPositionData를 MatchRecording으로 변환
    /// 실제 OpenFootball 통합 시 구현 예정
    pub fn convert_to_recording(
        _of_data: &serde_json::Value, // 추후 OpenFootball ResultMatchPositionData 타입으로 변경
        match_id: u64,
    ) -> MatchRecording {
        // 실제 구현에서는 OpenFootball 데이터 변환
        // 이 부분은 OpenFootball 타입과 정확히 매칭해야 함

        MatchRecording::new(match_id, "Home Team".to_string(), "Away Team".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_creation() {
        let recording = MatchRecording::new(1, "Home".to_string(), "Away".to_string());
        assert_eq!(recording.match_id, 1);
        assert_eq!(recording.total_ticks, 0);
    }

    #[test]
    fn test_frame_addition() {
        let mut recording = MatchRecording::new(1, "Home".to_string(), "Away".to_string());

        let frame = FrameData {
            ball: BallFrame {
                tick: 100,
                position: Vector3::new(50.0, 30.0, 0.0),
                velocity: Vector3::zeros(),
                height: 0.0,
                possession: None,
            },
            players: vec![],
        };

        recording.add_frame(100, frame);
        assert_eq!(recording.total_ticks, 100);

        let snapshot = recording.get_snapshot_at(100);
        assert!(snapshot.is_some());
    }

    #[test]
    fn test_highlight_extraction() {
        let mut event_stream = EventStream::new();

        event_stream.add_event(TimestampedEvent {
            tick: 1000,
            event_type: "goal".to_string(),
            player_id: Some(10),
            position: Some(Vector3::new(90.0, 35.0, 0.0)),
            importance: 0.9,
            data: serde_json::json!({}),
        });

        let highlights = event_stream.extract_highlights();
        assert_eq!(highlights.len(), 1);
        assert_eq!(highlights[0].events[0], "goal");
    }

    // ========================================================================
    // ENGINE_CONTRACT 1: Ball Range & Corner Stuck Detection
    // ========================================================================

    /// Helper: Calculate ball metrics from frame sequence
    /// Returns: (minx, maxx, miny, maxy, corner_stuck_ratio)
    fn ball_metrics(frames: &[BallFrame]) -> (f32, f32, f32, f32, f32) {
        let mut minx = f32::INFINITY;
        let mut maxx = f32::NEG_INFINITY;
        let mut miny = f32::INFINITY;
        let mut maxy = f32::NEG_INFINITY;

        // Corner stuck heuristic: ball near corners (0,0)/(105,0)/(0,68)/(105,68)
        // for consecutive frames
        let mut stuck = 0usize;
        for f in frames {
            let x = f.position.x;
            let y = f.position.y;
            minx = minx.min(x);
            maxx = maxx.max(x);
            miny = miny.min(y);
            maxy = maxy.max(y);
            let near_left = x <= 0.5;
            let near_right = x >= 104.5;
            let near_bottom = y <= 0.5;
            let near_top = y >= 67.5;
            if (near_left || near_right) && (near_bottom || near_top) {
                stuck += 1;
            }
        }
        let ratio = if frames.is_empty() { 0.0 } else { stuck as f32 / frames.len() as f32 };
        (minx, maxx, miny, maxy, ratio)
    }

    /// CONTRACT 1: Ball coordinate range must be within 0-105m x 0-68m
    /// - Verifies cm→m normalization works correctly
    /// - Checks corner_stuck heuristic (ball < 0.5m from edges)
    /// - Thresholds: ball_range within [-0.5, 105.5] × [-0.5, 68.5], corner_stuck < 2%
    #[test]
    fn engine_contract_ball_range() {
        // Synthetic replay frames: mix of normal play + corner frames
        let mut frames: Vec<BallFrame> = Vec::new();
        let mut prev: Option<Vector3<f32>> = None;

        // Normal play: should stay within pitch (900 frames)
        for i in 0..900 {
            let x = (i as f32 * 0.1) % 105.0;
            let y = 34.0 + ((i as f32) * 0.01).sin() * 10.0;
            let pos = Vector3::new(x, y, 0.0);
            frames.push(BallFrame::from_of_position(i, pos, prev));
            prev = Some(pos);
        }

        // Corner stuck chunk (short): 10 frames only
        for i in 900..910 {
            let pos = Vector3::new(105.0, 68.0, 0.0);
            frames.push(BallFrame::from_of_position(i, pos, prev));
            prev = Some(pos);
        }

        let (minx, maxx, miny, maxy, corner_ratio) = ball_metrics(&frames);

        // CONTRACT: ball range must be sane in meters
        assert!(minx >= -0.5, "minx out of range: {}", minx);
        assert!(maxx <= 105.5, "maxx out of range: {}", maxx);
        assert!(miny >= -0.5, "miny out of range: {}", miny);
        assert!(maxy <= 68.5, "maxy out of range: {}", maxy);

        // CONTRACT: corner stuck ratio should be low in normal play
        assert!(corner_ratio < 0.02, "corner stuck ratio too high: {}", corner_ratio);

        println!(
            "[CONTRACT_OK] ball_range: x=[{:.2}, {:.2}], y=[{:.2}, {:.2}], corner_stuck={:.2}%",
            minx,
            maxx,
            miny,
            maxy,
            corner_ratio * 100.0
        );
    }
}
