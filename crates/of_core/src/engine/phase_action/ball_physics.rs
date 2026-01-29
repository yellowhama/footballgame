//! Ball Physics FSM
//!
//! P7 Spec Section 5: Ball State Machine with realistic physics
//!
//! ## Ball State Flow
//! ```text
//! Controlled (드리블) ←─→ Rolling (굴러감) ←─→ Settled (정지)
//!      │                       ↑
//!      │ kick_to()             │ bounces = 0
//!      ▼                       │
//!  InFlight (비행) → Bouncing (바운스)
//!      │
//!      └──→ OutOfPlay (아웃)
//! ```

use serde::{Deserialize, Serialize};
// P0: Core types moved to action_queue
use super::super::action_queue::{PassType, ShotType};
use super::duration::{
    BALL_MIN_VELOCITY, BOUNCE_COEFFICIENT, DRIBBLE_CONTROL_RANGE, DRIBBLE_MAX_SEPARATION,
    DRIBBLE_MIN_SEPARATION, DRIBBLE_TOUCH_DISTANCE, GRASS_FRICTION, GRAVITY, MAX_BOUNCES, TICK_DT,
};
use crate::engine::physics_constants::field;

// ============================================================================
// Ball Physics Constants (P7 Spec Section 5)
// ============================================================================

/// 공 반경 (m)
pub const BALL_RADIUS: f32 = 0.11;

/// 드리블 터치 시 목표 separation
pub const TOUCH_SEPARATION: f32 = DRIBBLE_TOUCH_DISTANCE;

/// 기본 디플렉션 파워
pub const DEFAULT_DEFLECT_POWER: f32 = 5.0;

// ============================================================================
// Height Curve (공 비행 궤적)
// ============================================================================

/// 공 비행 궤적의 높이 곡선
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeightCurve {
    /// 땅볼 (height = 0)
    Ground,
    /// 낮은 탄도 (max height ~1m)
    LowArc,
    /// 중간 탄도 (max height ~3m)
    MediumArc,
    /// 높은 탄도 (max height ~10m)
    HighArc,
    /// 직선 (강슛, height 거의 없음)
    Line,
}

impl HeightCurve {
    /// 진행률(0~1)에 따른 높이 반환
    pub fn height_at(&self, progress: f32) -> f32 {
        // 포물선: h = 4 * max_h * p * (1 - p)
        let max_height = self.max_height();
        4.0 * max_height * progress * (1.0 - progress)
    }

    /// 최대 높이 반환
    pub fn max_height(&self) -> f32 {
        match self {
            HeightCurve::Ground => 0.0,
            HeightCurve::LowArc => 1.0,
            HeightCurve::MediumArc => 3.0,
            HeightCurve::HighArc => 10.0,
            HeightCurve::Line => 0.3,
        }
    }

    /// PassType에서 HeightCurve 결정
    pub fn from_pass_type(pass_type: PassType) -> Self {
        match pass_type {
            PassType::Ground => HeightCurve::Ground,
            PassType::Lofted => HeightCurve::MediumArc,
            PassType::ThroughBall => HeightCurve::Ground,
            PassType::Cross => HeightCurve::HighArc,
            PassType::BackPass => HeightCurve::Ground,
        }
    }

    /// ShotType에서 HeightCurve 결정
    pub fn from_shot_type(shot_type: ShotType) -> Self {
        match shot_type {
            ShotType::Normal => HeightCurve::LowArc,
            ShotType::Finesse => HeightCurve::MediumArc,
            ShotType::Power => HeightCurve::Line,
            ShotType::Chip => HeightCurve::HighArc,
            ShotType::Header => HeightCurve::MediumArc,
            ShotType::Volley => HeightCurve::Line,
            ShotType::OneTouch => HeightCurve::LowArc,
        }
    }
}

// ============================================================================
// Restart Type
// ============================================================================

/// 경기 재시작 유형
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartType {
    KickOff,
    GoalKick,
    Corner,
    ThrowIn,
    FreeKick,
    PenaltyKick,
}

// ============================================================================
// Ball State FSM
// ============================================================================

/// 공의 물리 상태
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BallPhysicsState {
    /// 선수가 컨트롤 중 (드리블)
    Controlled {
        owner_idx: usize,
        owner_team: u32,
        /// 공과 선수 사이 거리 (0.3~2.5m)
        separation: f32,
        /// 공이 선수 기준 어디에 있는지 (forward offset angle)
        offset_angle: f32,
    },

    /// 비행 중 (패스, 슈팅, 클리어)
    InFlight {
        start_pos: (f32, f32),
        end_pos: (f32, f32),
        start_tick: u64,
        arrival_tick: u64,
        height_curve: HeightCurve,
        initial_speed: f32,
    },

    /// 굴러가는 중 (루즈볼, 디플렉션)
    Rolling { velocity: (f32, f32) },

    /// 바운스 중 (높이 있는 상태에서 땅에 닿음)
    Bouncing {
        velocity: (f32, f32, f32), // (vx, vy, vz)
        remaining_bounces: u8,
    },

    /// 정지
    #[default]
    Settled,

    /// 아웃 (재시작 대기)
    OutOfPlay { restart_type: RestartType },
}

/// Internal helper for update_tick to avoid borrow conflicts
enum StateUpdateData {
    Controlled {
        owner_idx: usize,
        separation: f32,
        offset_angle: f32,
    },
    InFlight {
        start_pos: (f32, f32),
        end_pos: (f32, f32),
        start_tick: u64,
        arrival_tick: u64,
        height_curve: HeightCurve,
    },
    Rolling {
        velocity: (f32, f32),
    },
    Bouncing {
        velocity: (f32, f32, f32),
        remaining_bounces: u8,
    },
}

// ============================================================================
// Ball Physics Struct
// ============================================================================

/// 공 물리 구조체 (P7)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallPhysics {
    /// 현재 위치 (x, y) - meters
    pub position: (f32, f32),

    /// 현재 높이 (m)
    pub height: f32,

    /// 물리 상태
    pub state: BallPhysicsState,

    /// 마지막 터치한 선수 인덱스
    pub last_touch_player_idx: Option<usize>,

    /// 마지막 터치한 팀
    pub last_touch_team: Option<u32>,
}

impl Default for BallPhysics {
    fn default() -> Self {
        Self::new()
    }
}

impl BallPhysics {
    /// 새 공 생성 (필드 중앙)
    pub fn new() -> Self {
        Self {
            position: (field::CENTER_X, field::CENTER_Y), // 필드 중앙
            height: 0.0,
            state: BallPhysicsState::Settled,
            last_touch_player_idx: None,
            last_touch_team: None,
        }
    }

    /// 특정 위치에 공 생성
    pub fn at_position(x: f32, y: f32) -> Self {
        Self {
            position: (x, y),
            height: 0.0,
            state: BallPhysicsState::Settled,
            last_touch_player_idx: None,
            last_touch_team: None,
        }
    }

    /// 공 소유자 인덱스 반환
    pub fn owner(&self) -> Option<usize> {
        match &self.state {
            BallPhysicsState::Controlled { owner_idx, .. } => Some(*owner_idx),
            _ => None,
        }
    }

    /// 공 소유 팀 반환
    pub fn owner_team(&self) -> Option<u32> {
        match &self.state {
            BallPhysicsState::Controlled { owner_team, .. } => Some(*owner_team),
            _ => None,
        }
    }

    /// 루즈볼 여부
    pub fn is_loose(&self) -> bool {
        matches!(
            self.state,
            BallPhysicsState::Rolling { .. }
                | BallPhysicsState::Bouncing { .. }
                | BallPhysicsState::Settled
        )
    }

    /// 비행 중 여부
    pub fn is_in_flight(&self) -> bool {
        matches!(self.state, BallPhysicsState::InFlight { .. })
    }

    /// 아웃 여부
    pub fn is_out_of_play(&self) -> bool {
        matches!(self.state, BallPhysicsState::OutOfPlay { .. })
    }

    // ========================================================================
    // Ownership Transfer
    // ========================================================================

    /// 소유권 이전
    pub fn transfer_ownership(&mut self, new_owner_idx: usize, team_id: u32) {
        self.state = BallPhysicsState::Controlled {
            owner_idx: new_owner_idx,
            owner_team: team_id,
            separation: DRIBBLE_MIN_SEPARATION,
            offset_angle: 0.0,
        };
        self.last_touch_player_idx = Some(new_owner_idx);
        self.last_touch_team = Some(team_id);
        self.height = 0.0;
    }

    /// 루즈볼로 전환
    pub fn become_loose(&mut self, kick_velocity: (f32, f32)) {
        self.state = BallPhysicsState::Rolling { velocity: kick_velocity };
    }

    // ========================================================================
    // Tick Update
    // ========================================================================

    /// 매 틱 물리 업데이트
    pub fn update_tick(
        &mut self,
        current_tick: u64,
        player_positions: &[(f32, f32)],
        player_facings: &[f32],
    ) {
        // Use take pattern to move state out, process, then put back
        let state = std::mem::replace(&mut self.state, BallPhysicsState::Settled);

        let new_state = match state {
            BallPhysicsState::Controlled {
                owner_idx,
                owner_team,
                mut separation,
                offset_angle,
            } => self.process_controlled(
                owner_idx,
                owner_team,
                &mut separation,
                offset_angle,
                player_positions,
                player_facings,
            ),
            BallPhysicsState::InFlight {
                start_pos,
                end_pos,
                start_tick,
                arrival_tick,
                height_curve,
                initial_speed,
            } => self.process_in_flight(
                start_pos,
                end_pos,
                start_tick,
                arrival_tick,
                height_curve,
                initial_speed,
                current_tick,
            ),
            BallPhysicsState::Rolling { mut velocity } => self.process_rolling(&mut velocity),
            BallPhysicsState::Bouncing { mut velocity, mut remaining_bounces } => {
                self.process_bouncing(&mut velocity, &mut remaining_bounces)
            }
            BallPhysicsState::Settled => {
                self.height = 0.0;
                BallPhysicsState::Settled
            }
            BallPhysicsState::OutOfPlay { restart_type } => {
                BallPhysicsState::OutOfPlay { restart_type }
            }
        };

        self.state = new_state;
    }

    /// Controlled 상태 처리 - 새 상태 반환
    fn process_controlled(
        &mut self,
        owner_idx: usize,
        owner_team: u32,
        separation: &mut f32,
        offset_angle: f32,
        player_positions: &[(f32, f32)],
        player_facings: &[f32],
    ) -> BallPhysicsState {
        if owner_idx >= player_positions.len() || owner_idx >= player_facings.len() {
            // 잘못된 인덱스: 루즈볼로 전환
            return BallPhysicsState::Rolling { velocity: (0.0, 0.0) };
        }

        let owner_pos = player_positions[owner_idx];
        let owner_facing = player_facings[owner_idx];

        // 공 위치 = 선수 위치 + 전방 오프셋
        let effective_angle = owner_facing + offset_angle;
        let (dx, dy) = (effective_angle.cos() * *separation, effective_angle.sin() * *separation);

        self.position = (owner_pos.0 + dx, owner_pos.1 + dy);
        self.height = 0.0;

        // separation이 너무 커지면 공 놓침 체크
        if *separation > DRIBBLE_CONTROL_RANGE {
            BallPhysicsState::Rolling {
                velocity: (dx * 0.5, dy * 0.5), // 약한 속도로 굴러감
            }
        } else {
            BallPhysicsState::Controlled {
                owner_idx,
                owner_team,
                separation: *separation,
                offset_angle,
            }
        }
    }

    /// Controlled 상태 업데이트 (legacy - dribble_touch/carry에서 사용)
    fn update_controlled(
        &mut self,
        owner_idx: usize,
        separation: &mut f32,
        offset_angle: f32,
        player_positions: &[(f32, f32)],
        player_facings: &[f32],
    ) {
        if owner_idx >= player_positions.len() || owner_idx >= player_facings.len() {
            self.state = BallPhysicsState::Rolling { velocity: (0.0, 0.0) };
            return;
        }

        let owner_pos = player_positions[owner_idx];
        let owner_facing = player_facings[owner_idx];

        let effective_angle = owner_facing + offset_angle;
        let (dx, dy) = (effective_angle.cos() * *separation, effective_angle.sin() * *separation);

        self.position = (owner_pos.0 + dx, owner_pos.1 + dy);
        self.height = 0.0;

        if *separation > DRIBBLE_CONTROL_RANGE {
            self.state = BallPhysicsState::Rolling { velocity: (dx * 0.5, dy * 0.5) };
        }
    }

    /// InFlight 상태 처리 - 새 상태 반환
    fn process_in_flight(
        &mut self,
        start_pos: (f32, f32),
        end_pos: (f32, f32),
        start_tick: u64,
        arrival_tick: u64,
        height_curve: HeightCurve,
        initial_speed: f32,
        current_tick: u64,
    ) -> BallPhysicsState {
        let total_ticks = arrival_tick.saturating_sub(start_tick) as f32;
        let elapsed_ticks = current_tick.saturating_sub(start_tick) as f32;

        if total_ticks == 0.0 {
            // 즉시 도착
            self.position = end_pos;
            self.height = 0.0;
            return BallPhysicsState::Settled;
        }

        let progress = (elapsed_ticks / total_ticks).clamp(0.0, 1.0);

        // 위치 보간
        self.position = (
            start_pos.0 + (end_pos.0 - start_pos.0) * progress,
            start_pos.1 + (end_pos.1 - start_pos.1) * progress,
        );

        // 높이 계산
        self.height = height_curve.height_at(progress);

        // 도착 체크
        if current_tick >= arrival_tick {
            self.position = end_pos;
            let max_height = height_curve.max_height();

            if max_height > 0.5 {
                // 바운스 시작
                let downward_velocity = (max_height * 0.5).sqrt() * 2.0;
                self.height = 0.1;
                BallPhysicsState::Bouncing {
                    velocity: (0.0, 0.0, -downward_velocity),
                    remaining_bounces: MAX_BOUNCES,
                }
            } else {
                // 땅볼: 바로 Rolling으로
                self.height = 0.0;
                BallPhysicsState::Rolling { velocity: (0.0, 0.0) }
            }
        } else {
            // 아직 비행 중
            BallPhysicsState::InFlight {
                start_pos,
                end_pos,
                start_tick,
                arrival_tick,
                height_curve,
                initial_speed,
            }
        }
    }

    /// InFlight 상태 업데이트 (legacy)
    fn update_in_flight(
        &mut self,
        start_pos: (f32, f32),
        end_pos: (f32, f32),
        start_tick: u64,
        arrival_tick: u64,
        height_curve: HeightCurve,
        current_tick: u64,
    ) {
        let total_ticks = arrival_tick.saturating_sub(start_tick) as f32;
        let elapsed_ticks = current_tick.saturating_sub(start_tick) as f32;

        if total_ticks == 0.0 {
            self.position = end_pos;
            self.height = 0.0;
            self.state = BallPhysicsState::Settled;
            return;
        }

        let progress = (elapsed_ticks / total_ticks).clamp(0.0, 1.0);

        self.position = (
            start_pos.0 + (end_pos.0 - start_pos.0) * progress,
            start_pos.1 + (end_pos.1 - start_pos.1) * progress,
        );

        self.height = height_curve.height_at(progress);

        if current_tick >= arrival_tick {
            self.on_flight_arrival(end_pos, height_curve);
        }
    }

    /// 비행 도착 시 처리
    fn on_flight_arrival(&mut self, end_pos: (f32, f32), height_curve: HeightCurve) {
        self.position = end_pos;

        let max_height = height_curve.max_height();

        if max_height > 0.5 {
            // 바운스 시작
            let downward_velocity = (max_height * 0.5).sqrt() * 2.0; // 근사적 낙하 속도
            self.state = BallPhysicsState::Bouncing {
                velocity: (0.0, 0.0, -downward_velocity),
                remaining_bounces: MAX_BOUNCES,
            };
            self.height = 0.1; // 약간의 높이
        } else {
            // 땅볼: 바로 Rolling으로
            self.state = BallPhysicsState::Rolling {
                velocity: (0.0, 0.0), // 정지 상태 (Trap 필요)
            };
            self.height = 0.0;
        }
    }

    /// Rolling 상태 처리 - 새 상태 반환
    fn process_rolling(&mut self, velocity: &mut (f32, f32)) -> BallPhysicsState {
        // 위치 업데이트
        self.position.0 += velocity.0 * TICK_DT;
        self.position.1 += velocity.1 * TICK_DT;
        self.height = 0.0;

        // 마찰에 의한 감속 (GRASS_FRICTION은 틱당 속도 유지율)
        // 매 틱마다 속도를 GRASS_FRICTION 배로 유지
        velocity.0 *= GRASS_FRICTION;
        velocity.1 *= GRASS_FRICTION;

        // 속도가 충분히 작으면 정지
        let speed = (velocity.0 * velocity.0 + velocity.1 * velocity.1).sqrt();
        if speed < BALL_MIN_VELOCITY {
            BallPhysicsState::Settled
        } else {
            BallPhysicsState::Rolling { velocity: *velocity }
        }
    }

    /// Rolling 상태 업데이트 (legacy)
    fn update_rolling(&mut self, velocity: &mut (f32, f32)) {
        self.position.0 += velocity.0 * TICK_DT;
        self.position.1 += velocity.1 * TICK_DT;
        self.height = 0.0;

        let friction_per_tick = GRASS_FRICTION.powf(TICK_DT);
        velocity.0 *= friction_per_tick;
        velocity.1 *= friction_per_tick;

        let speed = (velocity.0 * velocity.0 + velocity.1 * velocity.1).sqrt();
        if speed < BALL_MIN_VELOCITY {
            self.state = BallPhysicsState::Settled;
        }
    }

    /// Bouncing 상태 처리 - 새 상태 반환
    fn process_bouncing(
        &mut self,
        velocity: &mut (f32, f32, f32),
        remaining_bounces: &mut u8,
    ) -> BallPhysicsState {
        // 수평 이동
        self.position.0 += velocity.0 * TICK_DT;
        self.position.1 += velocity.1 * TICK_DT;

        // 수직 이동 (중력 적용)
        self.height += velocity.2 * TICK_DT;
        velocity.2 -= GRAVITY * TICK_DT;

        // 땅에 닿음 체크
        if self.height <= 0.0 {
            self.height = 0.0;

            if *remaining_bounces > 0 {
                // 바운스
                velocity.2 = -velocity.2 * BOUNCE_COEFFICIENT;
                // 수평 속도도 감소
                velocity.0 *= BOUNCE_COEFFICIENT;
                velocity.1 *= BOUNCE_COEFFICIENT;
                *remaining_bounces -= 1;

                BallPhysicsState::Bouncing {
                    velocity: *velocity,
                    remaining_bounces: *remaining_bounces,
                }
            } else {
                // 바운스 끝 → Rolling으로 전환
                BallPhysicsState::Rolling { velocity: (velocity.0, velocity.1) }
            }
        } else {
            // 아직 공중에 있음
            BallPhysicsState::Bouncing {
                velocity: *velocity,
                remaining_bounces: *remaining_bounces,
            }
        }
    }

    /// Bouncing 상태 업데이트 (legacy)
    fn update_bouncing(&mut self, velocity: &mut (f32, f32, f32), remaining_bounces: &mut u8) {
        self.position.0 += velocity.0 * TICK_DT;
        self.position.1 += velocity.1 * TICK_DT;

        self.height += velocity.2 * TICK_DT;
        velocity.2 -= GRAVITY * TICK_DT;

        if self.height <= 0.0 {
            self.height = 0.0;

            if *remaining_bounces > 0 {
                velocity.2 = -velocity.2 * BOUNCE_COEFFICIENT;
                velocity.0 *= BOUNCE_COEFFICIENT;
                velocity.1 *= BOUNCE_COEFFICIENT;
                *remaining_bounces -= 1;
            } else {
                self.state = BallPhysicsState::Rolling { velocity: (velocity.0, velocity.1) };
            }
        }
    }

    // ========================================================================
    // Dribble Functions
    // ========================================================================

    /// 드리블 터치: separation 업데이트
    pub fn dribble_touch(&mut self, touch_direction: (f32, f32)) {
        if let BallPhysicsState::Controlled { separation, offset_angle, .. } = &mut self.state {
            // 터치하면 공이 1.5m 앞으로
            *separation = TOUCH_SEPARATION;
            *offset_angle = touch_direction.1.atan2(touch_direction.0);
        }
    }

    /// Carry 단계: separation이 점점 늘어남
    pub fn dribble_carry(&mut self, dt: f32) {
        if let BallPhysicsState::Controlled { separation, .. } = &mut self.state {
            // 공이 선수보다 빠르게 굴러가서 separation 증가
            *separation += 0.3 * dt; // 초당 0.3m 증가
            *separation = separation.min(DRIBBLE_MAX_SEPARATION);
        }
    }

    // ========================================================================
    // Kick Functions
    // ========================================================================

    /// 특정 위치로 공 차기
    pub fn kick_to(
        &mut self,
        target_pos: (f32, f32),
        speed: f32,
        height_curve: HeightCurve,
        kicker_idx: usize,
        kicker_team: u32,
        current_tick: u64,
    ) {
        let distance = ((target_pos.0 - self.position.0).powi(2)
            + (target_pos.1 - self.position.1).powi(2))
        .sqrt();

        // 도착 틱 계산
        let travel_time = if speed > 0.0 { distance / speed } else { 0.0 };
        let travel_ticks = (travel_time / TICK_DT).ceil() as u64;

        self.state = BallPhysicsState::InFlight {
            start_pos: self.position,
            end_pos: target_pos,
            start_tick: current_tick,
            arrival_tick: current_tick + travel_ticks.max(1),
            height_curve,
            initial_speed: speed,
        };

        self.last_touch_player_idx = Some(kicker_idx);
        self.last_touch_team = Some(kicker_team);
    }

    /// 패스 시작 (PassType 기반)
    pub fn start_pass(
        &mut self,
        target_pos: (f32, f32),
        pass_type: PassType,
        passer_idx: usize,
        passer_team: u32,
        current_tick: u64,
    ) {
        let speed = pass_type.base_speed();
        let height_curve = HeightCurve::from_pass_type(pass_type);
        self.kick_to(target_pos, speed, height_curve, passer_idx, passer_team, current_tick);
    }

    /// 슈팅 시작 (ShotType 기반)
    pub fn start_shot(
        &mut self,
        target_pos: (f32, f32),
        shot_type: ShotType,
        power: f32,
        shooter_idx: usize,
        shooter_team: u32,
        current_tick: u64,
    ) {
        let speed = shot_type.base_speed() * power;
        let height_curve = HeightCurve::from_shot_type(shot_type);
        self.kick_to(target_pos, speed, height_curve, shooter_idx, shooter_team, current_tick);
    }

    /// 디플렉션 (태클/블락에 의해 튕겨나감)
    pub fn deflect(
        &mut self,
        deflect_direction: (f32, f32),
        deflect_power: f32,
        toucher_idx: usize,
        toucher_team: u32,
    ) {
        let norm = (deflect_direction.0.powi(2) + deflect_direction.1.powi(2)).sqrt();
        let (dx, dy) = if norm > 0.001 {
            (deflect_direction.0 / norm * deflect_power, deflect_direction.1 / norm * deflect_power)
        } else {
            (0.0, 0.0)
        };

        self.state = BallPhysicsState::Rolling { velocity: (dx, dy) };

        self.last_touch_player_idx = Some(toucher_idx);
        self.last_touch_team = Some(toucher_team);
        self.height = 0.0;
    }

    // ========================================================================
    // Boundary Check
    // ========================================================================

    /// 경계 체크 (아웃 판정)
    pub fn check_boundary(&mut self, field_width: f32, field_height: f32) -> Option<RestartType> {
        let (x, y) = self.position;

        // 터치라인 (좌우 - y축)
        if y < 0.0 || y > field_height {
            let restart = RestartType::ThrowIn;
            self.state = BallPhysicsState::OutOfPlay { restart_type: restart };
            return Some(restart);
        }

        // 골라인 (상하 - x축, 골대 제외)
        // 골대 영역: 30.34m ~ 37.66m (중앙 34m ± 3.66m = 7.32m 골대)
        let goal_y_min = field_height / 2.0 - 3.66;
        let goal_y_max = field_height / 2.0 + 3.66;
        let in_goal_area = y >= goal_y_min && y <= goal_y_max;

        if x < 0.0 && !in_goal_area {
            // 왼쪽 골라인 밖 (골 영역 제외)
            let restart = if self.last_touch_team == Some(1) {
                RestartType::Corner
            } else {
                RestartType::GoalKick
            };
            self.state = BallPhysicsState::OutOfPlay { restart_type: restart };
            return Some(restart);
        }

        if x > field_width && !in_goal_area {
            // 오른쪽 골라인 밖 (골 영역 제외)
            let restart = if self.last_touch_team == Some(0) {
                RestartType::Corner
            } else {
                RestartType::GoalKick
            };
            self.state = BallPhysicsState::OutOfPlay { restart_type: restart };
            return Some(restart);
        }

        None
    }

    /// 재시작 위치로 리셋
    pub fn reset_for_restart(&mut self, _restart_type: RestartType, position: (f32, f32)) {
        self.position = position;
        self.height = 0.0;
        self.state = BallPhysicsState::Settled;
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ball_creation() {
        let ball = BallPhysics::new();
        assert_eq!(ball.position, (field::CENTER_X, field::CENTER_Y));
        assert_eq!(ball.height, 0.0);
        assert!(matches!(ball.state, BallPhysicsState::Settled));
    }

    #[test]
    fn test_rolling_friction() {
        let mut ball = BallPhysics::new();
        ball.state = BallPhysicsState::Rolling { velocity: (10.0, 0.0) };

        let initial_x = ball.position.0;

        // 여러 틱 업데이트
        for tick in 0..200 {
            ball.update_tick(tick, &[], &[]);
            if matches!(ball.state, BallPhysicsState::Settled) {
                break;
            }
        }

        // 정지 상태로 전환되어야 함
        assert!(matches!(ball.state, BallPhysicsState::Settled));
        // 공이 이동했어야 함
        assert!(ball.position.0 > initial_x);
    }

    #[test]
    fn test_in_flight_progress() {
        let mut ball = BallPhysics::new();
        ball.kick_to((field::CENTER_X, 50.0), 20.0, HeightCurve::MediumArc, 0, 0, 0);

        assert!(matches!(ball.state, BallPhysicsState::InFlight { .. }));

        // 중간 지점에서 높이 확인
        ball.update_tick(1, &[], &[]);
        assert!(ball.height > 0.0);

        // 도착 후
        for tick in 2..30 {
            ball.update_tick(tick, &[], &[]);
        }

        // InFlight이 끝나야 함
        assert!(!matches!(ball.state, BallPhysicsState::InFlight { .. }));
    }

    #[test]
    fn test_height_curve() {
        // Ground
        assert_eq!(HeightCurve::Ground.height_at(0.5), 0.0);

        // MediumArc - 중간에서 최고 높이
        let mid_height = HeightCurve::MediumArc.height_at(0.5);
        assert!((mid_height - 3.0).abs() < 0.01);

        // 시작과 끝에서는 0
        assert!(HeightCurve::MediumArc.height_at(0.0).abs() < 0.001);
        assert!(HeightCurve::MediumArc.height_at(1.0).abs() < 0.001);
    }

    #[test]
    fn test_bounce_decay() {
        let mut ball = BallPhysics::new();
        ball.height = 3.0;
        ball.state = BallPhysicsState::Bouncing { velocity: (5.0, 0.0, 0.0), remaining_bounces: 3 };

        let mut bounce_count = 0;
        let mut prev_bounces = 3;

        for tick in 0..200 {
            if let BallPhysicsState::Bouncing { remaining_bounces, .. } = ball.state {
                if remaining_bounces < prev_bounces {
                    bounce_count += 1;
                    prev_bounces = remaining_bounces;
                }
            }

            ball.update_tick(tick, &[], &[]);

            // Rolling 상태로 전환되면 종료
            if matches!(ball.state, BallPhysicsState::Rolling { .. } | BallPhysicsState::Settled) {
                break;
            }
        }

        assert_eq!(bounce_count, 3);
        assert!(matches!(ball.state, BallPhysicsState::Rolling { .. } | BallPhysicsState::Settled));
    }

    #[test]
    fn test_controlled_separation() {
        let mut ball = BallPhysics::new();
        ball.state = BallPhysicsState::Controlled {
            owner_idx: 0,
            owner_team: 0,
            separation: 0.5,
            offset_angle: 0.0,
        };

        let player_positions = vec![(50.0, field::CENTER_Y)];
        let player_facings = vec![0.0]; // 동쪽

        ball.update_tick(0, &player_positions, &player_facings);

        // 공은 선수 앞에 있어야 함
        assert!(ball.position.0 > 50.0);
        assert!((ball.position.1 - field::CENTER_Y).abs() < 0.01);
    }

    #[test]
    fn test_dribble_touch_and_carry() {
        let mut ball = BallPhysics::new();
        ball.transfer_ownership(0, 0);

        // Touch
        ball.dribble_touch((1.0, 0.0));

        if let BallPhysicsState::Controlled { separation, .. } = ball.state {
            assert!((separation - TOUCH_SEPARATION).abs() < 0.01);
        } else {
            panic!("Expected Controlled state");
        }

        // Carry
        ball.dribble_carry(1.0); // 1초

        if let BallPhysicsState::Controlled { separation, .. } = ball.state {
            assert!(separation > TOUCH_SEPARATION);
        } else {
            panic!("Expected Controlled state");
        }
    }

    #[test]
    fn test_boundary_check_throw_in() {
        let mut ball = BallPhysics::new();
        ball.position = (50.0, -1.0); // 터치라인 밖
        ball.state = BallPhysicsState::Rolling { velocity: (0.0, -1.0) };

        let restart = ball.check_boundary(field::LENGTH_M, field::WIDTH_M);

        assert!(matches!(restart, Some(RestartType::ThrowIn)));
        assert!(matches!(ball.state, BallPhysicsState::OutOfPlay { .. }));
    }

    #[test]
    fn test_boundary_check_corner() {
        let mut ball = BallPhysics::new();
        ball.position = (-1.0, 10.0); // 왼쪽 골라인 밖 (골 영역 외)
        ball.last_touch_team = Some(1); // 원정팀이 마지막 터치
        ball.state = BallPhysicsState::Rolling { velocity: (-1.0, 0.0) };

        let restart = ball.check_boundary(field::LENGTH_M, field::WIDTH_M);

        assert!(matches!(restart, Some(RestartType::Corner)));
    }

    #[test]
    fn test_boundary_check_goal_kick() {
        let mut ball = BallPhysics::new();
        ball.position = (-1.0, 10.0); // 왼쪽 골라인 밖 (골 영역 외)
        ball.last_touch_team = Some(0); // 홈팀이 마지막 터치
        ball.state = BallPhysicsState::Rolling { velocity: (-1.0, 0.0) };

        let restart = ball.check_boundary(field::LENGTH_M, field::WIDTH_M);

        assert!(matches!(restart, Some(RestartType::GoalKick)));
    }

    #[test]
    fn test_deflect() {
        let mut ball = BallPhysics::new();
        ball.transfer_ownership(5, 0);

        ball.deflect((1.0, 1.0), 5.0, 10, 1);

        assert!(matches!(ball.state, BallPhysicsState::Rolling { .. }));
        assert_eq!(ball.last_touch_player_idx, Some(10));
        assert_eq!(ball.last_touch_team, Some(1));
    }

    #[test]
    fn test_pass_type_height_curve() {
        assert_eq!(HeightCurve::from_pass_type(PassType::Ground), HeightCurve::Ground);
        assert_eq!(HeightCurve::from_pass_type(PassType::Cross), HeightCurve::HighArc);
        assert_eq!(HeightCurve::from_pass_type(PassType::Lofted), HeightCurve::MediumArc);
    }

    #[test]
    fn test_shot_type_height_curve() {
        assert_eq!(HeightCurve::from_shot_type(ShotType::Power), HeightCurve::Line);
        assert_eq!(HeightCurve::from_shot_type(ShotType::Chip), HeightCurve::HighArc);
        assert_eq!(HeightCurve::from_shot_type(ShotType::Normal), HeightCurve::LowArc);
    }

    #[test]
    fn test_ownership_transfer() {
        let mut ball = BallPhysics::new();

        ball.transfer_ownership(5, 1);

        assert_eq!(ball.owner(), Some(5));
        assert_eq!(ball.owner_team(), Some(1));
        assert!(!ball.is_loose());
    }

    #[test]
    fn test_become_loose() {
        let mut ball = BallPhysics::new();
        ball.transfer_ownership(5, 1);

        ball.become_loose((3.0, 2.0));

        assert!(ball.is_loose());
        assert_eq!(ball.owner(), None);
    }

    #[test]
    fn test_start_pass() {
        let mut ball = BallPhysics::new();
        ball.start_pass((70.0, 40.0), PassType::Ground, 5, 0, 100);

        assert!(ball.is_in_flight());
        assert_eq!(ball.last_touch_player_idx, Some(5));
        assert_eq!(ball.last_touch_team, Some(0));

        if let BallPhysicsState::InFlight { height_curve, .. } = ball.state {
            assert_eq!(height_curve, HeightCurve::Ground);
        } else {
            panic!("Expected InFlight state");
        }
    }

    #[test]
    fn test_start_shot() {
        let mut ball = BallPhysics::new();
        ball.start_shot((field::LENGTH_M, field::CENTER_Y), ShotType::Power, 1.0, 9, 0, 200);

        assert!(ball.is_in_flight());

        if let BallPhysicsState::InFlight { height_curve, .. } = ball.state {
            assert_eq!(height_curve, HeightCurve::Line);
        } else {
            panic!("Expected InFlight state");
        }
    }
}
