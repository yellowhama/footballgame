//! PepGrid System
//!
//! 펩 과르디올라 스타일의 5채널 포지셔닝 시스템.
//! 필드를 5개 수직 채널로 나누고 선수 과밀화를 방지합니다.
//!
//! ## 채널 배치 (왼쪽 → 오른쪽)
//! - LeftWing: 0~13.6m (터치라인 ~ 하프스페이스 경계)
//! - LeftHalfSpace: 13.6~27.2m
//! - Center: 27.2~40.8m
//! - RightHalfSpace: 40.8~54.4m
//! - RightWing: 54.4~68m

use serde::{Deserialize, Serialize};
use crate::engine::physics_constants::field;

/// 필드 채널 (수직 레인)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    LeftWing,
    LeftHalfSpace,
    Center,
    RightHalfSpace,
    RightWing,
}

impl Channel {
    /// 채널 인덱스 (0-4)
    pub fn index(&self) -> usize {
        match self {
            Channel::LeftWing => 0,
            Channel::LeftHalfSpace => 1,
            Channel::Center => 2,
            Channel::RightHalfSpace => 3,
            Channel::RightWing => 4,
        }
    }

    /// 인덱스에서 채널
    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Channel::LeftWing),
            1 => Some(Channel::LeftHalfSpace),
            2 => Some(Channel::Center),
            3 => Some(Channel::RightHalfSpace),
            4 => Some(Channel::RightWing),
            _ => None,
        }
    }

    /// Y좌표에서 채널 결정 (필드 폭 68m 기준)
    pub fn from_y_position(y: f32) -> Self {
        let channel_width = field::WIDTH_M / 5.0; // 13.6m
        let idx = ((y / channel_width) as usize).min(4);
        Self::from_index(idx).unwrap_or(Channel::Center)
    }

    /// 채널의 Y좌표 중심
    pub fn center_y(&self) -> f32 {
        let channel_width = field::WIDTH_M / 5.0;
        self.index() as f32 * channel_width + channel_width / 2.0
    }

    /// 인접 채널 (왼쪽, 오른쪽)
    pub fn adjacent(&self) -> (Option<Channel>, Option<Channel>) {
        let idx = self.index();
        let left = if idx > 0 { Channel::from_index(idx - 1) } else { None };
        let right = Channel::from_index(idx + 1);
        (left, right)
    }

    /// 하프스페이스 채널인지
    pub fn is_half_space(&self) -> bool {
        matches!(self, Channel::LeftHalfSpace | Channel::RightHalfSpace)
    }

    /// 측면 채널인지
    pub fn is_wing(&self) -> bool {
        matches!(self, Channel::LeftWing | Channel::RightWing)
    }
}

/// 필드 깊이 (수평 존)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoneDepth {
    /// 수비 지역 (0~26.25m)
    Defense,
    /// 중앙 하위 지역 (1/4~중앙선)
    MidfieldLow,
    /// 중앙 상위 지역 (중앙선~3/4)
    MidfieldHigh,
    /// 공격 지역 (78.75~105m)
    Attack,
}

impl ZoneDepth {
    /// X좌표에서 깊이 결정 (필드 길이 105m 기준, 홈팀 관점)
    pub fn from_x_position(x: f32, is_home_team: bool) -> Self {
        // 어웨이팀은 좌표 반전
        let x = if is_home_team { x } else { field::LENGTH_M - x };

        let q1 = field::LENGTH_M * 0.25;
        let q2 = field::CENTER_X;
        let q3 = field::LENGTH_M * 0.75;

        if x < q1 {
            ZoneDepth::Defense
        } else if x < q2 {
            ZoneDepth::MidfieldLow
        } else if x < q3 {
            ZoneDepth::MidfieldHigh
        } else {
            ZoneDepth::Attack
        }
    }

    /// 존의 X좌표 중심 (홈팀 기준)
    pub fn center_x(&self) -> f32 {
        match self {
            ZoneDepth::Defense => 13.125,
            ZoneDepth::MidfieldLow => 39.375,
            ZoneDepth::MidfieldHigh => 65.625,
            ZoneDepth::Attack => 91.875,
        }
    }

    /// 공격적인 존인지
    pub fn is_attacking(&self) -> bool {
        matches!(self, ZoneDepth::MidfieldHigh | ZoneDepth::Attack)
    }
}

/// PepGrid - 5채널 포지셔닝 관리자
#[derive(Debug, Clone, Default)]
pub struct PepGrid {
    /// 채널별 선수 인덱스 리스트
    occupants: [Vec<usize>; 5],
    /// 채널당 최대 인원
    max_per_channel: usize,
    /// 마지막 업데이트 틱
    last_update_tick: u64,
}

impl PepGrid {
    /// 새 PepGrid 생성
    pub fn new() -> Self {
        Self { occupants: Default::default(), max_per_channel: 2, last_update_tick: 0 }
    }

    /// 채널당 최대 인원 설정
    pub fn with_max_per_channel(mut self, max: usize) -> Self {
        self.max_per_channel = max;
        self
    }

    /// 선수 위치로 그리드 업데이트
    pub fn update_from_positions(
        &mut self,
        positions: &[(f32, f32)],
        player_indices: &[usize],
        current_tick: u64,
    ) {
        // 채널 초기화
        for channel in &mut self.occupants {
            channel.clear();
        }

        // 각 선수를 채널에 할당
        for &idx in player_indices {
            if idx < positions.len() {
                let (_, y) = positions[idx];
                let channel = Channel::from_y_position(y);
                self.occupants[channel.index()].push(idx);
            }
        }

        self.last_update_tick = current_tick;
    }

    /// 특정 채널의 점유자 수
    pub fn channel_count(&self, channel: Channel) -> usize {
        self.occupants[channel.index()].len()
    }

    /// 특정 채널의 점유자들
    pub fn channel_occupants(&self, channel: Channel) -> &[usize] {
        &self.occupants[channel.index()]
    }

    /// 채널이 과밀한지 확인
    pub fn is_overcrowded(&self, channel: Channel) -> bool {
        self.channel_count(channel) > self.max_per_channel
    }

    /// 과밀화 해결 - 초과 선수를 인접 채널로 이동 추천
    ///
    /// # Returns
    /// (선수 인덱스, 현재 채널, 추천 채널) 튜플 리스트
    pub fn resolve_overcrowding(&self) -> Vec<(usize, Channel, Channel)> {
        let mut moves = Vec::new();

        for channel_idx in 0..5 {
            let channel = Channel::from_index(channel_idx).unwrap();
            let occupants = &self.occupants[channel_idx];

            if occupants.len() <= self.max_per_channel {
                continue;
            }

            // 초과 인원 계산
            let excess = occupants.len() - self.max_per_channel;
            let (left, right) = channel.adjacent();

            // 인접 채널 중 여유 있는 곳 찾기
            let left_space = left
                .map(|c| self.max_per_channel.saturating_sub(self.channel_count(c)))
                .unwrap_or(0);
            let right_space = right
                .map(|c| self.max_per_channel.saturating_sub(self.channel_count(c)))
                .unwrap_or(0);

            // 초과 선수를 인접 채널로 할당
            let mut to_left = 0;
            let mut to_right = 0;

            for (i, &player_idx) in occupants.iter().rev().take(excess).enumerate() {
                // 번갈아가며 좌우로 분배 (더 여유 있는 쪽 우선)
                if (i % 2 == 0 && left_space > to_left) || right_space <= to_right {
                    if let Some(target) = left {
                        if to_left < left_space {
                            moves.push((player_idx, channel, target));
                            to_left += 1;
                            continue;
                        }
                    }
                }

                if let Some(target) = right {
                    if to_right < right_space {
                        moves.push((player_idx, channel, target));
                        to_right += 1;
                    }
                }
            }
        }

        moves
    }

    /// 전체 그리드 상태 출력 (디버깅용)
    pub fn debug_state(&self) -> String {
        let channels = ["LW", "LHS", "C", "RHS", "RW"];
        channels
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let count = self.occupants[i].len();
                let players: Vec<String> =
                    self.occupants[i].iter().map(|p| p.to_string()).collect();
                format!("{}: {} [{}]", name, count, players.join(", "))
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// 특정 위치에서 가장 가까운 빈 채널 찾기
    pub fn find_nearest_free_channel(&self, current_channel: Channel) -> Option<Channel> {
        let current_idx = current_channel.index() as i32;

        // 가까운 순서로 채널 확인
        for distance in 1..=4i32 {
            // 왼쪽 확인
            let left_idx = current_idx - distance;
            if left_idx >= 0 {
                if let Some(channel) = Channel::from_index(left_idx as usize) {
                    if !self.is_overcrowded(channel) {
                        return Some(channel);
                    }
                }
            }

            // 오른쪽 확인
            let right_idx = current_idx + distance;
            if let Some(channel) = Channel::from_index(right_idx as usize) {
                if !self.is_overcrowded(channel) {
                    return Some(channel);
                }
            }
        }

        None
    }
}

/// 그리드 셀 (채널 + 깊이)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCell {
    pub channel: Channel,
    pub depth: ZoneDepth,
}

impl GridCell {
    pub fn new(channel: Channel, depth: ZoneDepth) -> Self {
        Self { channel, depth }
    }

    /// 좌표에서 셀 결정
    pub fn from_position(x: f32, y: f32, is_home_team: bool) -> Self {
        Self {
            channel: Channel::from_y_position(y),
            depth: ZoneDepth::from_x_position(x, is_home_team),
        }
    }

    /// 셀의 중심 좌표
    pub fn center(&self, is_home_team: bool) -> (f32, f32) {
        let x =
            if is_home_team { self.depth.center_x() } else { field::LENGTH_M - self.depth.center_x() };
        let y = self.channel.center_y();
        (x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const CY: f32 = field::CENTER_Y;

    #[test]
    fn test_channel_from_y_position() {
        assert_eq!(Channel::from_y_position(0.0), Channel::LeftWing);
        assert_eq!(Channel::from_y_position(6.8), Channel::LeftWing);
        assert_eq!(Channel::from_y_position(13.6), Channel::LeftHalfSpace);
        assert_eq!(Channel::from_y_position(CY), Channel::Center);
        // 54.4 / 13.6 = 4.0 → 인덱스 4 = RightWing
        assert_eq!(Channel::from_y_position(54.3), Channel::RightHalfSpace); // 54.4 바로 아래
        assert_eq!(Channel::from_y_position(field::WIDTH_M), Channel::RightWing);
    }

    #[test]
    fn test_channel_adjacent() {
        let (left, right) = Channel::Center.adjacent();
        assert_eq!(left, Some(Channel::LeftHalfSpace));
        assert_eq!(right, Some(Channel::RightHalfSpace));

        let (left, right) = Channel::LeftWing.adjacent();
        assert_eq!(left, None);
        assert_eq!(right, Some(Channel::LeftHalfSpace));

        let (left, right) = Channel::RightWing.adjacent();
        assert_eq!(left, Some(Channel::RightHalfSpace));
        assert_eq!(right, None);
    }

    #[test]
    fn test_zone_depth_from_position() {
        // 홈팀 기준
        assert_eq!(ZoneDepth::from_x_position(10.0, true), ZoneDepth::Defense);
        assert_eq!(ZoneDepth::from_x_position(40.0, true), ZoneDepth::MidfieldLow);
        assert_eq!(ZoneDepth::from_x_position(60.0, true), ZoneDepth::MidfieldHigh);
        assert_eq!(ZoneDepth::from_x_position(90.0, true), ZoneDepth::Attack);

        // 어웨이팀 기준 (좌표 반전)
        assert_eq!(ZoneDepth::from_x_position(10.0, false), ZoneDepth::Attack);
        assert_eq!(ZoneDepth::from_x_position(90.0, false), ZoneDepth::Defense);
    }

    #[test]
    fn test_pep_grid_update() {
        let mut grid = PepGrid::new();

        // 선수 5명 배치
        let positions = vec![
            (50.0, 5.0),  // 선수 0 - LeftWing
            (50.0, CY), // 선수 1 - Center
            (50.0, 35.0), // 선수 2 - Center
            (50.0, 36.0), // 선수 3 - Center (과밀!)
            (50.0, 60.0), // 선수 4 - RightWing
        ];
        let indices: Vec<usize> = (0..5).collect();

        grid.update_from_positions(&positions, &indices, 100);

        assert_eq!(grid.channel_count(Channel::LeftWing), 1);
        assert_eq!(grid.channel_count(Channel::Center), 3);
        assert_eq!(grid.channel_count(Channel::RightWing), 1);

        // Center가 과밀 (3명 > 2명 max)
        assert!(grid.is_overcrowded(Channel::Center));
    }

    #[test]
    fn test_channel_overcrowding() {
        // PEP-01: 한 채널 3명 → 1명 인접 채널 이동
        let mut grid = PepGrid::new().with_max_per_channel(2);

        let positions = vec![
            (50.0, CY), // 선수 0 - Center
            (50.0, 35.0), // 선수 1 - Center
            (50.0, 36.0), // 선수 2 - Center (초과)
            (50.0, 5.0),  // 선수 3 - LeftWing (여유)
        ];
        let indices: Vec<usize> = (0..4).collect();

        grid.update_from_positions(&positions, &indices, 100);

        let moves = grid.resolve_overcrowding();

        // 1명이 이동해야 함
        assert_eq!(moves.len(), 1);

        // Center에서 인접 채널로 이동
        let (_, from, to) = moves[0];
        assert_eq!(from, Channel::Center);
        assert!(to == Channel::LeftHalfSpace || to == Channel::RightHalfSpace);
    }

    #[test]
    fn test_find_nearest_free_channel() {
        let mut grid = PepGrid::new().with_max_per_channel(1);

        // Center 가득, LeftHalfSpace 가득, RightHalfSpace 비어있음
        let positions = vec![
            (50.0, CY), // 선수 0 - Center
            (50.0, 20.0), // 선수 1 - LeftHalfSpace
        ];
        let indices: Vec<usize> = (0..2).collect();

        grid.update_from_positions(&positions, &indices, 100);

        // Center에서 가장 가까운 빈 채널 찾기
        // 왼쪽을 먼저 체크하므로 LeftHalfSpace가 가득 차면 RightHalfSpace가 선택됨
        // 하지만 distance=1에서 LeftHalfSpace(가득)와 RightHalfSpace(비어있음)를 동시에 체크
        // 왼쪽을 먼저 체크하므로 LeftHalfSpace가 overcrowded인지 확인 → 넘어감 → RightHalfSpace 체크
        let free = grid.find_nearest_free_channel(Channel::Center);
        // find_nearest_free_channel은 왼쪽부터 체크하므로, LeftHalfSpace가 overcrowded면 넘어가고
        // RightHalfSpace가 비어있으면 반환
        // 실제로는 LeftHalfSpace에 1명 있고 max=1이므로 overcrowded는 아님 (count > max가 아니라 count <= max)
        // 따라서 LeftHalfSpace가 먼저 반환됨
        assert_eq!(free, Some(Channel::LeftHalfSpace));
    }

    #[test]
    fn test_grid_cell() {
        let cell = GridCell::from_position(60.0, CY, true);
        assert_eq!(cell.channel, Channel::Center);
        assert_eq!(cell.depth, ZoneDepth::MidfieldHigh);

        let center = cell.center(true);
        assert!((center.0 - 65.625).abs() < 0.1);
        assert!((center.1 - CY).abs() < 1.0);
    }

    #[test]
    fn test_channel_properties() {
        assert!(Channel::LeftWing.is_wing());
        assert!(Channel::RightWing.is_wing());
        assert!(!Channel::Center.is_wing());

        assert!(Channel::LeftHalfSpace.is_half_space());
        assert!(Channel::RightHalfSpace.is_half_space());
        assert!(!Channel::Center.is_half_space());
    }
}
