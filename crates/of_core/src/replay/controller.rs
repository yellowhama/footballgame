//! Replay Playback Controller
//!
//! 리플레이 재생 제어 및 카메라 시스템

use super::recording::*;
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 리플레이 재생 컨트롤러
pub struct ReplayController {
    recording: MatchRecording,
    current_tick: u64,
    playback_speed: f32,
    is_playing: bool,
    camera_mode: CameraMode,
    selected_player: Option<u32>,
    loop_mode: LoopMode,
    highlight_only: bool,
}

impl ReplayController {
    pub fn new(recording: MatchRecording) -> Self {
        Self {
            recording,
            current_tick: 0,
            playback_speed: 1.0,
            is_playing: false,
            camera_mode: CameraMode::Broadcast,
            selected_player: None,
            loop_mode: LoopMode::None,
            highlight_only: false,
        }
    }

    /// 재생 시작
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    /// 일시정지
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// 정지 (처음으로)
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_tick = 0;
    }

    /// 프레임 진행
    pub fn advance(&mut self, delta_ms: f32) -> Option<FrameSnapshot> {
        if !self.is_playing {
            return self.get_current_frame();
        }

        // 속도에 따른 틱 계산 (60 FPS 기준)
        let ticks_to_advance = (delta_ms * 0.06 * self.playback_speed) as u64;
        self.current_tick += ticks_to_advance;

        // 하이라이트만 재생 모드
        if self.highlight_only {
            self.skip_to_next_highlight();
        }

        // 루프 처리
        match self.loop_mode {
            LoopMode::None => {
                if self.current_tick >= self.recording.total_ticks {
                    self.stop();
                }
            }
            LoopMode::Full => {
                if self.current_tick >= self.recording.total_ticks {
                    self.current_tick = 0;
                }
            }
            LoopMode::Segment { start, end } => {
                if self.current_tick >= end {
                    self.current_tick = start;
                }
            }
        }

        self.get_current_frame()
    }

    /// 특정 시간으로 이동
    pub fn seek(&mut self, tick: u64) {
        self.current_tick = tick.min(self.recording.total_ticks);
    }

    /// 특정 이벤트로 이동
    pub fn seek_to_event(&mut self, event_type: &str) -> bool {
        for event in &self.recording.event_stream.events {
            if event.event_type == event_type && event.tick > self.current_tick {
                self.current_tick = event.tick;
                return true;
            }
        }
        false
    }

    /// 다음 하이라이트로 스킵
    pub fn skip_to_next_highlight(&mut self) -> bool {
        let highlights = self.recording.extract_highlights();
        for highlight in highlights {
            if highlight.start_tick > self.current_tick {
                self.current_tick = highlight.start_tick;
                return true;
            }
        }
        false
    }

    /// 이전 하이라이트로 스킵
    pub fn skip_to_previous_highlight(&mut self) -> bool {
        let highlights = self.recording.extract_highlights();
        for highlight in highlights.iter().rev() {
            if highlight.end_tick < self.current_tick {
                self.current_tick = highlight.start_tick;
                return true;
            }
        }
        false
    }

    /// 현재 프레임 가져오기
    pub fn get_current_frame(&self) -> Option<FrameSnapshot> {
        self.recording.get_snapshot_at(self.current_tick)
    }

    /// 재생 속도 설정 (0.25x ~ 4x)
    pub fn set_speed(&mut self, speed: f32) {
        self.playback_speed = speed.clamp(0.25, 4.0);
    }

    /// 카메라 모드 설정
    pub fn set_camera_mode(&mut self, mode: CameraMode) {
        self.camera_mode = mode;
    }

    /// 선수 선택 (Player Follow 모드용)
    pub fn select_player(&mut self, player_id: u32) {
        self.selected_player = Some(player_id);
        self.camera_mode = CameraMode::PlayerFollow;
    }

    /// 카메라 위치 계산
    pub fn calculate_camera_position(&self) -> CameraTransform {
        let frame = match self.get_current_frame() {
            Some(f) => f,
            None => return CameraTransform::default(),
        };

        match self.camera_mode {
            CameraMode::Broadcast => {
                // 방송 카메라: 경기장 측면 고정
                CameraTransform {
                    position: Vector3::new(52.5, -30.0, 25.0),
                    target: Vector3::new(52.5, 34.0, 0.0),
                    fov: 60.0,
                }
            }
            CameraMode::Tactical => {
                // 전술 카메라: 위에서 내려다보기
                CameraTransform {
                    position: Vector3::new(52.5, 34.0, 40.0),
                    target: Vector3::new(52.5, 34.0, 0.0),
                    fov: 90.0,
                }
            }
            CameraMode::BallFollow => {
                // 공 추적 카메라
                let ball_pos = frame.ball.position;
                CameraTransform {
                    position: ball_pos + Vector3::new(0.0, -15.0, 10.0),
                    target: ball_pos,
                    fov: 50.0,
                }
            }
            CameraMode::PlayerFollow => {
                // 선수 추적 카메라
                if let Some(player_id) = self.selected_player {
                    if let Some(player) = frame.players.iter().find(|p| p.player_id == player_id) {
                        let player_pos = player.position;
                        CameraTransform {
                            position: player_pos + Vector3::new(-5.0, -8.0, 5.0),
                            target: player_pos,
                            fov: 45.0,
                        }
                    } else {
                        CameraTransform::default()
                    }
                } else {
                    CameraTransform::default()
                }
            }
            CameraMode::GoalCam { is_home } => {
                // 골대 카메라
                let goal_x = if is_home { 0.0 } else { 105.0 };
                CameraTransform {
                    position: Vector3::new(goal_x, 34.0, 2.0),
                    target: Vector3::new(52.5, 34.0, 0.0),
                    fov: 70.0,
                }
            }
            CameraMode::Custom { position, target } => {
                // 커스텀 카메라
                CameraTransform { position, target, fov: 60.0 }
            }
        }
    }

    /// 현재 재생 정보
    pub fn get_playback_info(&self) -> PlaybackInfo {
        PlaybackInfo {
            current_tick: self.current_tick,
            total_ticks: self.recording.total_ticks,
            current_time_seconds: self.tick_to_seconds(self.current_tick),
            total_time_seconds: self.tick_to_seconds(self.recording.total_ticks),
            is_playing: self.is_playing,
            speed: self.playback_speed,
            camera_mode: self.camera_mode.clone(),
        }
    }

    /// 틱을 초로 변환
    fn tick_to_seconds(&self, tick: u64) -> f32 {
        tick as f32 / 60.0 // 60 FPS 기준
    }
}

/// 카메라 모드
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CameraMode {
    Broadcast,                                               // 방송 뷰
    Tactical,                                                // 전술 뷰 (탑다운)
    BallFollow,                                              // 공 추적
    PlayerFollow,                                            // 선수 추적
    GoalCam { is_home: bool },                               // 골대 카메라
    Custom { position: Vector3<f32>, target: Vector3<f32> }, // 커스텀
}

/// 루프 모드
#[derive(Debug, Clone)]
pub enum LoopMode {
    None,                             // 루프 없음
    Full,                             // 전체 반복
    Segment { start: u64, end: u64 }, // 구간 반복
}

/// 카메라 변환 정보
#[derive(Debug, Clone)]
pub struct CameraTransform {
    pub position: Vector3<f32>,
    pub target: Vector3<f32>,
    pub fov: f32,
}

impl Default for CameraTransform {
    fn default() -> Self {
        Self {
            position: Vector3::new(52.5, -30.0, 25.0),
            target: Vector3::new(52.5, 34.0, 0.0),
            fov: 60.0,
        }
    }
}

/// 재생 정보
#[derive(Debug, Clone, Serialize)]
pub struct PlaybackInfo {
    pub current_tick: u64,
    pub total_ticks: u64,
    pub current_time_seconds: f32,
    pub total_time_seconds: f32,
    pub is_playing: bool,
    pub speed: f32,
    pub camera_mode: CameraMode,
}

/// 리플레이 분석기
pub struct ReplayAnalyzer {
    recording: MatchRecording,
}

impl ReplayAnalyzer {
    pub fn new(recording: MatchRecording) -> Self {
        Self { recording }
    }

    /// 히트맵 생성
    pub fn generate_heatmap(&self, player_id: u32) -> HeatMap {
        let mut heatmap = HeatMap::new();

        if let Some(frames) = self.recording.position_stream.player_frames.get(&player_id) {
            for frame in frames {
                heatmap.add_point(frame.position.x, frame.position.y);
            }
        }

        heatmap.normalize();
        heatmap
    }

    /// 패스 네트워크 분석
    pub fn analyze_pass_network(&self) -> PassNetwork {
        let network = PassNetwork::new();

        // 패스 이벤트 분석
        for event in &self.recording.event_stream.events {
            if event.event_type == "pass" {
                // 패스 데이터 추출 및 네트워크 구축
                // 실제 구현 필요
            }
        }

        network
    }

    /// 이동 거리 계산
    pub fn calculate_distance_covered(&self, player_id: u32) -> f32 {
        let mut total_distance = 0.0;

        if let Some(frames) = self.recording.position_stream.player_frames.get(&player_id) {
            for window in frames.windows(2) {
                let dist = (window[1].position - window[0].position).norm();
                total_distance += dist;
            }
        }

        total_distance
    }

    /// 평균 위치 계산
    pub fn calculate_average_position(&self, player_id: u32) -> Option<Vector3<f32>> {
        if let Some(frames) = self.recording.position_stream.player_frames.get(&player_id) {
            if frames.is_empty() {
                return None;
            }

            let sum: Vector3<f32> = frames.iter().map(|f| f.position).sum();
            Some(sum / frames.len() as f32)
        } else {
            None
        }
    }
}

/// 히트맵
#[derive(Debug)]
pub struct HeatMap {
    grid: Vec<Vec<f32>>,
    width: usize,
    height: usize,
}

impl Default for HeatMap {
    fn default() -> Self {
        Self::new()
    }
}

impl HeatMap {
    pub fn new() -> Self {
        // 10x10 그리드로 경기장 분할
        Self { grid: vec![vec![0.0; 10]; 10], width: 10, height: 10 }
    }

    pub fn add_point(&mut self, x: f32, y: f32) {
        // 좌표를 그리드 인덱스로 변환
        let grid_x = ((x / 105.0) * 10.0).clamp(0.0, 9.0) as usize;
        let grid_y = ((y / 68.0) * 10.0).clamp(0.0, 9.0) as usize;

        self.grid[grid_y][grid_x] += 1.0;
    }

    pub fn normalize(&mut self) {
        let max = self.grid.iter().flat_map(|row| row.iter()).fold(0.0f32, |a, &b| a.max(b));

        if max > 0.0 {
            for row in &mut self.grid {
                for cell in row {
                    *cell /= max;
                }
            }
        }
    }
}

/// 패스 네트워크
#[derive(Debug)]
pub struct PassNetwork {
    connections: HashMap<(u32, u32), u32>, // (from, to) -> count
}

impl Default for PassNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl PassNetwork {
    pub fn new() -> Self {
        Self { connections: HashMap::new() }
    }

    pub fn add_pass(&mut self, from: u32, to: u32) {
        *self.connections.entry((from, to)).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_playback() {
        let recording = MatchRecording::new(1, "Home".to_string(), "Away".to_string());
        let mut controller = ReplayController::new(recording);

        controller.play();
        assert!(controller.is_playing);

        controller.pause();
        assert!(!controller.is_playing);
    }

    #[test]
    fn test_speed_control() {
        let recording = MatchRecording::new(1, "Home".to_string(), "Away".to_string());
        let mut controller = ReplayController::new(recording);

        controller.set_speed(2.0);
        assert_eq!(controller.playback_speed, 2.0);

        controller.set_speed(10.0);
        assert_eq!(controller.playback_speed, 4.0); // Clamped to max
    }
}
