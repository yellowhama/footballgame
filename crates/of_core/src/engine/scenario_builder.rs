//! ScenarioBuilder - Declarative Scenario Creation API
//!
//! Google Football 스타일의 선언적 시나리오 생성기.
//!
//! ## 사용 예시
//!
//! ```rust,ignore
//! let scenario = MatchScenarioBuilder::new("3v1_counter")
//!     .set_duration(400)
//!     .set_deterministic(false)
//!     .set_end_on_score(true)
//!     .set_ball_position(0.62, 0.0)
//!     .set_team(TeamSide::Home)
//!     .add_player(-1.0, 0.0, Position::GK)
//!     .add_player(0.6, 0.0, Position::CM)
//!     .set_team(TeamSide::Away)
//!     .add_player(-1.0, 0.0, Position::GK)
//!     .build()?;
//! ```
//!
//! ## 좌표계
//!
//! Google Football 정규화 좌표 (빌더 입력):
//! - x: [-1, 1] (왼쪽 골라인 → 오른쪽 골라인)
//! - y: [-0.42, 0.42] (아래 터치라인 → 위 터치라인)
//!
//! of_core 미터 좌표 (내부 변환):
//! - x: [0, 105] meters
//! - y: [0, 68] meters

use crate::models::player::Position;
use crate::models::TeamSide;

use super::scenario_loader::{
    ScenarioBall, ScenarioPlayer, ScenarioSide, ScenarioSpec, ScenarioStartMode, ScenarioTeam,
};

// ============================================================================
// Error Types
// ============================================================================

/// 시나리오 빌더 에러
#[derive(Debug, Clone, PartialEq)]
pub enum ScenarioError {
    /// 팀이 설정되지 않음
    NoTeamSelected,
    /// 홈팀 누락
    MissingHomeTeam,
    /// 어웨이팀 누락
    MissingAwayTeam,
    /// 골키퍼 누락
    MissingGoalkeeper { team: TeamSide },
    /// 좌표 범위 초과
    CoordinateOutOfBounds { field: &'static str, value: f32, min: f32, max: f32 },
    /// 선수 수 초과
    TooManyPlayers { team: TeamSide, count: usize },
    /// 중복 ID
    DuplicateId { id: String },
}

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoTeamSelected => write!(f, "No team selected. Call set_team() first."),
            Self::MissingHomeTeam => write!(f, "Home team is required."),
            Self::MissingAwayTeam => write!(f, "Away team is required."),
            Self::MissingGoalkeeper { team } => {
                write!(f, "{:?} team must have a goalkeeper.", team)
            }
            Self::CoordinateOutOfBounds { field, value, min, max } => {
                write!(f, "{} coordinate {} is out of bounds [{}, {}]", field, value, min, max)
            }
            Self::TooManyPlayers { team, count } => {
                write!(f, "{:?} team has {} players (max 11).", team, count)
            }
            Self::DuplicateId { id } => write!(f, "Duplicate scenario ID: {}", id),
        }
    }
}

impl std::error::Error for ScenarioError {}

// ============================================================================
// Coordinate Conversion
// ============================================================================

/// 정규화 좌표 범위 (Google Football)
pub const NORM_X_MIN: f32 = -1.0;
pub const NORM_X_MAX: f32 = 1.0;
pub const NORM_Y_MIN: f32 = -0.42;
pub const NORM_Y_MAX: f32 = 0.42;

/// 미터 좌표 범위 (of_core)
pub const METERS_X_MIN: f32 = 0.0;
pub const METERS_X_MAX: f32 = 105.0;
pub const METERS_Y_MIN: f32 = 0.0;
pub const METERS_Y_MAX: f32 = 68.0;

/// 정규화 좌표 → 미터 좌표 변환
///
/// Google Football 좌표계:
/// - x: [-1, 1] → [0, 105]
/// - y: [-0.42, 0.42] → [0, 68]
pub fn normalize_to_meters(x: f32, y: f32) -> (f32, f32) {
    let x_m = (x - NORM_X_MIN) / (NORM_X_MAX - NORM_X_MIN) * (METERS_X_MAX - METERS_X_MIN);
    let y_m = (y - NORM_Y_MIN) / (NORM_Y_MAX - NORM_Y_MIN) * (METERS_Y_MAX - METERS_Y_MIN);
    (x_m, y_m)
}

/// 미터 좌표 → 정규화 좌표 변환
///
/// of_core 좌표계:
/// - x: [0, 105] → [-1, 1]
/// - y: [0, 68] → [-0.42, 0.42]
pub fn meters_to_normalize(x: f32, y: f32) -> (f32, f32) {
    let x_n = (x - METERS_X_MIN) / (METERS_X_MAX - METERS_X_MIN) * (NORM_X_MAX - NORM_X_MIN)
        + NORM_X_MIN;
    let y_n = (y - METERS_Y_MIN) / (METERS_Y_MAX - METERS_Y_MIN) * (NORM_Y_MAX - NORM_Y_MIN)
        + NORM_Y_MIN;
    (x_n, y_n)
}

fn validate_normalized_coords(x: f32, y: f32, field: &'static str) -> Result<(), ScenarioError> {
    if x < NORM_X_MIN || x > NORM_X_MAX {
        return Err(ScenarioError::CoordinateOutOfBounds {
            field,
            value: x,
            min: NORM_X_MIN,
            max: NORM_X_MAX,
        });
    }
    if y < NORM_Y_MIN || y > NORM_Y_MAX {
        return Err(ScenarioError::CoordinateOutOfBounds {
            field,
            value: y,
            min: NORM_Y_MIN,
            max: NORM_Y_MAX,
        });
    }
    Ok(())
}

// ============================================================================
// ScenarioBuilder Trait
// ============================================================================

/// 시나리오 빌더 트레이트
///
/// Google Football 스타일의 선언적 시나리오 생성 API.
/// 소비 패턴(consuming pattern)을 사용하여 메서드 체이닝 지원.
pub trait ScenarioBuilder: Sized {
    /// 경기 시간 설정 (초)
    fn set_duration(self, seconds: u32) -> Self;

    /// 결정론적 모드 설정
    fn set_deterministic(self, deterministic: bool) -> Self;

    /// 시작 모드 설정 (킥오프, 골킥 등)
    fn set_start_mode(self, mode: ScenarioStartMode) -> Self;

    /// 골 득점 시 에피소드 종료
    fn set_end_on_score(self, enabled: bool) -> Self;

    /// 점유 전환 시 에피소드 종료
    fn set_end_on_possession_change(self, enabled: bool) -> Self;

    /// 공 위치 설정 (정규화 좌표)
    ///
    /// x: [-1, 1], y: [-0.42, 0.42]
    fn set_ball_position(self, x: f32, y: f32) -> Self;

    /// 공 속도 설정 (m/s)
    fn set_ball_velocity(self, vx: f32, vy: f32, vz: f32) -> Self;

    /// 현재 팀 설정
    fn set_team(self, side: TeamSide) -> Self;

    /// 선수 추가 (정규화 좌표)
    ///
    /// x: [-1, 1], y: [-0.42, 0.42]
    fn add_player(self, x: f32, y: f32, role: Position) -> Self;

    /// 선수 추가 (능력치 포함)
    fn add_player_with_overall(self, x: f32, y: f32, role: Position, overall: u8) -> Self;

    /// 시나리오 빌드
    fn build(self) -> Result<ScenarioSpec, ScenarioError>;
}

// ============================================================================
// MatchScenarioBuilder
// ============================================================================

/// 경기 시나리오 빌더
///
/// Google Football 스타일의 메서드 체이닝으로 시나리오 생성.
#[derive(Debug, Clone)]
pub struct MatchScenarioBuilder {
    id: String,
    description: Option<String>,
    seed: u64,
    deterministic: bool,
    game_duration_s: Option<u32>,
    start_mode: Option<ScenarioStartMode>,
    end_on_score: bool,
    end_on_possession_change: bool,

    // Ball
    ball_pos: Option<(f32, f32)>, // 정규화 좌표
    ball_vel: Option<(f32, f32, f32)>,

    // Teams
    current_team: Option<TeamSide>,
    home_players: Vec<PlayerEntry>,
    away_players: Vec<PlayerEntry>,
}

#[derive(Debug, Clone)]
struct PlayerEntry {
    x: f32, // 정규화 좌표
    y: f32,
    role: Position,
    overall: u8,
}

impl MatchScenarioBuilder {
    /// 새 빌더 생성
    ///
    /// # Arguments
    /// * `id` - 시나리오 고유 ID
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            description: None,
            seed: 42,
            deterministic: false,
            game_duration_s: None,
            start_mode: None,
            end_on_score: false,
            end_on_possession_change: false,
            ball_pos: None,
            ball_vel: None,
            current_team: None,
            home_players: Vec::new(),
            away_players: Vec::new(),
        }
    }

    /// 설명 설정
    pub fn set_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// 시드 설정
    pub fn set_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// 시뮬레이션 틱 수 설정
    pub fn set_simulate_ticks(self, _ticks: u32) -> Self {
        // ScenarioSpec에 simulate_ticks 필드가 있지만 이 빌더에서는 직접 설정하지 않음
        // 대신 game_duration_s로 제어
        self
    }

    fn players_for_team(&mut self) -> Option<&mut Vec<PlayerEntry>> {
        match self.current_team {
            Some(TeamSide::Home) => Some(&mut self.home_players),
            Some(TeamSide::Away) => Some(&mut self.away_players),
            None => None,
        }
    }

    fn build_team(&self, side: TeamSide, players: &[PlayerEntry]) -> ScenarioTeam {
        let scenario_side = match side {
            TeamSide::Home => ScenarioSide::Home,
            TeamSide::Away => ScenarioSide::Away,
        };

        let scenario_players: Vec<ScenarioPlayer> = players
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let (x_m, y_m) = normalize_to_meters(p.x, p.y);
                ScenarioPlayer {
                    role: p.role,
                    pos_m: [x_m, y_m],
                    slot: Some(i as u8),
                    lazy: false,
                    overall: Some(p.overall),
                    attributes: None,
                    name: None,
                }
            })
            .collect();

        ScenarioTeam {
            side: scenario_side,
            difficulty: None,
            formation: None,
            players: scenario_players,
        }
    }
}

impl ScenarioBuilder for MatchScenarioBuilder {
    fn set_duration(mut self, seconds: u32) -> Self {
        self.game_duration_s = Some(seconds);
        self
    }

    fn set_deterministic(mut self, deterministic: bool) -> Self {
        self.deterministic = deterministic;
        self
    }

    fn set_start_mode(mut self, mode: ScenarioStartMode) -> Self {
        self.start_mode = Some(mode);
        self
    }

    fn set_end_on_score(mut self, enabled: bool) -> Self {
        self.end_on_score = enabled;
        self
    }

    fn set_end_on_possession_change(mut self, enabled: bool) -> Self {
        self.end_on_possession_change = enabled;
        self
    }

    fn set_ball_position(mut self, x: f32, y: f32) -> Self {
        self.ball_pos = Some((x, y));
        self
    }

    fn set_ball_velocity(mut self, vx: f32, vy: f32, vz: f32) -> Self {
        self.ball_vel = Some((vx, vy, vz));
        self
    }

    fn set_team(mut self, side: TeamSide) -> Self {
        self.current_team = Some(side);
        self
    }

    fn add_player(self, x: f32, y: f32, role: Position) -> Self {
        self.add_player_with_overall(x, y, role, 80)
    }

    fn add_player_with_overall(mut self, x: f32, y: f32, role: Position, overall: u8) -> Self {
        if let Some(players) = self.players_for_team() {
            players.push(PlayerEntry { x, y, role, overall: overall.clamp(1, 99) });
        }
        self
    }

    fn build(self) -> Result<ScenarioSpec, ScenarioError> {
        // Validate teams
        if self.home_players.is_empty() {
            return Err(ScenarioError::MissingHomeTeam);
        }
        if self.away_players.is_empty() {
            return Err(ScenarioError::MissingAwayTeam);
        }

        // Validate player counts
        if self.home_players.len() > 11 {
            return Err(ScenarioError::TooManyPlayers {
                team: TeamSide::Home,
                count: self.home_players.len(),
            });
        }
        if self.away_players.len() > 11 {
            return Err(ScenarioError::TooManyPlayers {
                team: TeamSide::Away,
                count: self.away_players.len(),
            });
        }

        // Validate coordinates
        for p in &self.home_players {
            validate_normalized_coords(p.x, p.y, "home_player")?;
        }
        for p in &self.away_players {
            validate_normalized_coords(p.x, p.y, "away_player")?;
        }
        if let Some((bx, by)) = self.ball_pos {
            validate_normalized_coords(bx, by, "ball")?;
        }

        // Build teams
        let home_team = self.build_team(TeamSide::Home, &self.home_players);
        let away_team = self.build_team(TeamSide::Away, &self.away_players);

        // Build ball
        let ball = self.ball_pos.map(|(x, y)| {
            let (x_m, y_m) = normalize_to_meters(x, y);
            ScenarioBall {
                pos_m: [x_m, y_m, 0.0],
                vel_mps: self.ball_vel.map(|(vx, vy, vz)| [vx, vy, vz]),
                owner: None,
                state: None,
            }
        });

        Ok(ScenarioSpec {
            id: self.id,
            description: self.description,
            seed: self.seed,
            deterministic: self.deterministic,
            game_duration_s: self.game_duration_s,
            second_half_s: None,
            start_mode: self.start_mode,
            start_team: None,
            simulate_ticks: None,
            home_attacks_right: None,
            teams: vec![home_team, away_team],
            ball,
            actions: Vec::new(),
            assertions: Vec::new(),
            state_assertions: Vec::new(),
        })
    }
}

// ============================================================================
// Preset Scenarios
// ============================================================================

impl MatchScenarioBuilder {
    /// 3v1 역습 시나리오 (Google Football Academy 스타일)
    ///
    /// Home: GK + 3 attackers
    /// Away: GK + 1 defender
    pub fn academy_3v1_with_keeper() -> Self {
        Self::new("academy_3v1_with_keeper")
            .set_description("3v1 with keeper - Academy scenario")
            .set_duration(400)
            .set_deterministic(false)
            .set_end_on_score(true)
            .set_end_on_possession_change(true)
            .set_ball_position(0.62, 0.0)
            // Home team (attackers)
            .set_team(TeamSide::Home)
            .add_player(-1.0, 0.0, Position::GK) // GK at goal line
            .add_player(0.6, 0.0, Position::CM) // Ball carrier
            .add_player(0.7, 0.2, Position::CM) // Support right
            .add_player(0.7, -0.2, Position::CM) // Support left
            // Away team (defenders)
            .set_team(TeamSide::Away)
            .add_player(1.0, 0.0, Position::GK) // GK at goal line
            .add_player(0.75, 0.0, Position::CB) // Single defender
    }

    /// 1v1 상황 (공격자 vs 골키퍼)
    pub fn academy_1v1_vs_keeper() -> Self {
        Self::new("academy_1v1_vs_keeper")
            .set_description("1v1 vs keeper - Finishing drill")
            .set_duration(200)
            .set_deterministic(false)
            .set_end_on_score(true)
            .set_ball_position(0.5, 0.0)
            // Home team
            .set_team(TeamSide::Home)
            .add_player(-1.0, 0.0, Position::GK)
            .add_player(0.5, 0.0, Position::ST)
            // Away team
            .set_team(TeamSide::Away)
            .add_player(1.0, 0.0, Position::GK)
    }

    /// 코너킥 시나리오
    pub fn academy_corner() -> Self {
        Self::new("academy_corner")
            .set_description("Corner kick situation")
            .set_duration(300)
            .set_start_mode(ScenarioStartMode::Corner)
            .set_end_on_score(true)
            .set_ball_position(0.95, 0.35) // Near corner flag
            // Home team (attacking)
            .set_team(TeamSide::Home)
            .add_player(-1.0, 0.0, Position::GK)
            .add_player(0.95, 0.35, Position::CM) // Corner taker
            .add_player(0.85, 0.1, Position::ST) // Near post
            .add_player(0.85, -0.1, Position::ST) // Far post
            .add_player(0.8, 0.0, Position::CM) // Edge of box
            // Away team (defending)
            .set_team(TeamSide::Away)
            .add_player(1.0, 0.0, Position::GK)
            .add_player(0.9, 0.0, Position::CB)
            .add_player(0.88, 0.1, Position::CB)
            .add_player(0.88, -0.1, Position::CB)
    }

    /// 11v11 킥오프 시나리오
    pub fn full_match_kickoff() -> Self {
        Self::new("full_match_kickoff")
            .set_description("Full 11v11 kickoff")
            .set_start_mode(ScenarioStartMode::KickOff)
            .set_ball_position(0.0, 0.0) // Center
            // Home team (4-4-2)
            .set_team(TeamSide::Home)
            .add_player(-1.0, 0.0, Position::GK)
            .add_player(-0.7, -0.3, Position::LB)
            .add_player(-0.7, -0.1, Position::CB)
            .add_player(-0.7, 0.1, Position::CB)
            .add_player(-0.7, 0.3, Position::RB)
            .add_player(-0.4, -0.3, Position::LM)
            .add_player(-0.4, -0.1, Position::CM)
            .add_player(-0.4, 0.1, Position::CM)
            .add_player(-0.4, 0.3, Position::RM)
            .add_player(-0.1, -0.1, Position::ST)
            .add_player(-0.1, 0.1, Position::ST)
            // Away team (4-4-2)
            .set_team(TeamSide::Away)
            .add_player(1.0, 0.0, Position::GK)
            .add_player(0.7, 0.3, Position::LB)
            .add_player(0.7, 0.1, Position::CB)
            .add_player(0.7, -0.1, Position::CB)
            .add_player(0.7, -0.3, Position::RB)
            .add_player(0.4, 0.3, Position::LM)
            .add_player(0.4, 0.1, Position::CM)
            .add_player(0.4, -0.1, Position::CM)
            .add_player(0.4, -0.3, Position::RM)
            .add_player(0.1, 0.1, Position::ST)
            .add_player(0.1, -0.1, Position::ST)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_conversion() {
        // Center of field
        let (x, y) = normalize_to_meters(0.0, 0.0);
        assert!((x - 52.5).abs() < 0.1);
        assert!((y - 34.0).abs() < 0.1);

        // Left goal line center
        let (x, y) = normalize_to_meters(-1.0, 0.0);
        assert!((x - 0.0).abs() < 0.1);
        assert!((y - 34.0).abs() < 0.1);

        // Right goal line center
        let (x, y) = normalize_to_meters(1.0, 0.0);
        assert!((x - 105.0).abs() < 0.1);
        assert!((y - 34.0).abs() < 0.1);

        // Top left corner
        let (x, y) = normalize_to_meters(-1.0, 0.42);
        assert!((x - 0.0).abs() < 0.1);
        assert!((y - 68.0).abs() < 0.1);

        // Bottom right corner
        let (x, y) = normalize_to_meters(1.0, -0.42);
        assert!((x - 105.0).abs() < 0.1);
        assert!((y - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_inverse_conversion() {
        // Test round-trip
        let original = (0.5, 0.2);
        let (mx, my) = normalize_to_meters(original.0, original.1);
        let (nx, ny) = meters_to_normalize(mx, my);
        assert!((nx - original.0).abs() < 0.001);
        assert!((ny - original.1).abs() < 0.001);
    }

    #[test]
    fn test_basic_build() {
        let scenario = MatchScenarioBuilder::new("test")
            .set_duration(400)
            .set_ball_position(0.0, 0.0)
            .set_team(TeamSide::Home)
            .add_player(-1.0, 0.0, Position::GK)
            .add_player(0.5, 0.0, Position::CM)
            .set_team(TeamSide::Away)
            .add_player(1.0, 0.0, Position::GK)
            .add_player(0.5, 0.0, Position::CM)
            .build();

        assert!(scenario.is_ok());
        let spec = scenario.unwrap();
        assert_eq!(spec.id, "test");
        assert_eq!(spec.game_duration_s, Some(400));
        assert_eq!(spec.teams.len(), 2);
    }

    #[test]
    fn test_missing_home_team() {
        let scenario = MatchScenarioBuilder::new("test")
            .set_team(TeamSide::Away)
            .add_player(1.0, 0.0, Position::GK)
            .build();

        assert!(matches!(scenario, Err(ScenarioError::MissingHomeTeam)));
    }

    #[test]
    fn test_missing_away_team() {
        let scenario = MatchScenarioBuilder::new("test")
            .set_team(TeamSide::Home)
            .add_player(-1.0, 0.0, Position::GK)
            .build();

        assert!(matches!(scenario, Err(ScenarioError::MissingAwayTeam)));
    }

    #[test]
    fn test_coordinate_out_of_bounds() {
        let scenario = MatchScenarioBuilder::new("test")
            .set_team(TeamSide::Home)
            .add_player(-1.5, 0.0, Position::GK) // x out of bounds
            .set_team(TeamSide::Away)
            .add_player(1.0, 0.0, Position::GK)
            .build();

        assert!(matches!(scenario, Err(ScenarioError::CoordinateOutOfBounds { .. })));
    }

    #[test]
    fn test_too_many_players() {
        // Use fold to add 12 players with consuming pattern
        let builder = (0..12).fold(
            MatchScenarioBuilder::new("test").set_team(TeamSide::Home),
            |b, i| b.add_player(-0.5, (i as f32 - 5.5) * 0.07, Position::CM),
        );

        let scenario = builder
            .set_team(TeamSide::Away)
            .add_player(1.0, 0.0, Position::GK)
            .build();

        assert!(matches!(
            scenario,
            Err(ScenarioError::TooManyPlayers { team: TeamSide::Home, count: 12 })
        ));
    }

    #[test]
    fn test_preset_3v1() {
        let builder = MatchScenarioBuilder::academy_3v1_with_keeper();
        let scenario = builder.build();

        assert!(scenario.is_ok());
        let spec = scenario.unwrap();
        assert_eq!(spec.id, "academy_3v1_with_keeper");

        // Check ball position
        let ball = spec.ball.expect("Ball should be set");
        assert!((ball.pos_m[0] - 84.63).abs() < 1.0); // 0.62 normalized = ~84.63m

        // Check player counts
        let home = spec.teams.iter().find(|t| matches!(t.side, ScenarioSide::Home)).unwrap();
        let away = spec.teams.iter().find(|t| matches!(t.side, ScenarioSide::Away)).unwrap();
        assert_eq!(home.players.len(), 4); // GK + 3 attackers
        assert_eq!(away.players.len(), 2); // GK + 1 defender
    }

    #[test]
    fn test_preset_full_match() {
        let builder = MatchScenarioBuilder::full_match_kickoff();
        let scenario = builder.build();

        assert!(scenario.is_ok());
        let spec = scenario.unwrap();

        let home = spec.teams.iter().find(|t| matches!(t.side, ScenarioSide::Home)).unwrap();
        let away = spec.teams.iter().find(|t| matches!(t.side, ScenarioSide::Away)).unwrap();
        assert_eq!(home.players.len(), 11);
        assert_eq!(away.players.len(), 11);
    }
}
