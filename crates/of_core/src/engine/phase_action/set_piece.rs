//! Set Piece FSM (Corner, Free Kick, Penalty)
//!
//! P9: 세트피스를 Phase-based FSM으로 구현
//!
//! ## 핵심 개념
//! - **SetPieceType**: 세트피스 종류 (Corner, FreeKick, Penalty, ThrowIn, GoalKick)
//! - **SetPiecePhase**: 실행 단계 (Setup → Delivery → Contest → Outcome → Finished)
//! - **SetPieceAction**: FSM 액션
//!
//! ## Phase 설명
//! - **Setup**: 선수들이 위치 잡는 중 (2-4틱)
//! - **Delivery**: 킥/스로 실행 (1-2틱)
//! - **Contest**: 공중볼 경합 또는 수비 반응 (1-3틱)
//! - **Outcome**: 결과 결정 (헤딩/슛/클리어/세이브)
//! - **Finished**: 완료

use crate::engine::physics_constants::skills;
use rand::Rng;
use serde::{Deserialize, Serialize};

// ============================================================================
// Set Piece Types
// ============================================================================

/// 세트피스 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetPieceType {
    /// 코너킥 - 크로스 + 헤딩 기회
    Corner,
    /// 프리킥 (직접) - 직접 슛 가능
    FreeKickDirect,
    /// 프리킥 (간접) - 크로스/패스만 가능
    FreeKickIndirect,
    /// 페널티킥
    Penalty,
    /// 스로인
    ThrowIn,
    /// 골킥
    GoalKick,
}

/// 코너킥 전술
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CornerTactic {
    /// 인스윙 크로스 (골대 방향)
    #[default]
    Inswing,
    /// 아웃스윙 크로스 (골대 반대 방향)
    Outswing,
    /// 숏코너 (가까운 동료에게)
    Short,
    /// 니어 포스트 타겟
    NearPost,
    /// 파 포스트 타겟
    FarPost,
}

/// 프리킥 전술
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FreeKickTactic {
    /// 직접 슛
    #[default]
    DirectShot,
    /// 크로스 (박스 안으로)
    Cross,
    /// 숏패스 (벽패스 등)
    ShortPass,
    /// 로빙 (수비 뒤로)
    LobBehind,
}

// ============================================================================
// Set Piece Phase
// ============================================================================

/// 세트피스 실행 단계
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SetPiecePhase {
    /// 선수 위치 잡는 중
    #[default]
    Setup,
    /// 킥/스로 실행 중
    Delivery,
    /// 공중볼 경합 / 수비 반응
    Contest,
    /// 결과 결정 (헤딩, 슛, 클리어, 세이브)
    Outcome,
    /// 완료
    Finished,
}

// ============================================================================
// Set Piece Result
// ============================================================================

/// 세트피스 결과
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SetPieceResult {
    /// 진행 중
    InProgress,
    /// 골!
    Goal { scorer_idx: usize, scorer_name: String, assist_idx: Option<usize> },
    /// 슛 시도 (온타겟)
    ShotOnTarget { shooter_idx: usize, xg: f32 },
    /// 슛 시도 (오프타겟)
    ShotOffTarget { shooter_idx: usize, xg: f32 },
    /// GK 세이브
    Save { gk_idx: usize, shooter_idx: usize },
    /// 수비 클리어
    Cleared { defender_idx: usize },
    /// 공격팀 소유 유지
    AttackRetain { receiver_idx: usize },
    /// 수비팀 소유 전환
    DefenseWin { receiver_idx: usize },
    /// 아웃
    OutOfPlay,
}

// ============================================================================
// Corner Kick Context
// ============================================================================

/// 코너킥 실행 컨텍스트
#[derive(Debug, Clone)]
pub struct CornerKickContext {
    /// 공격팀이 홈인지
    pub is_home_attacking: bool,
    /// 킥커 인덱스
    pub kicker_idx: usize,
    /// 킥커 이름
    pub kicker_name: String,
    /// 코너킥 위치 (왼쪽/오른쪽)
    pub is_left_corner: bool,
    /// 전술
    pub tactic: CornerTactic,
    /// 킥커 능력치
    pub corners: u8,   // FIX_2601/0123: corners 속성 추가 - 코너킥 전용 delivery quality
    pub crossing: u8,
    pub technique: u8,
    pub vision: u8,
    pub curve: u8,
    /// 헤딩 타겟들
    pub header_targets: Vec<AerialTarget>,
    /// 수비수들
    pub defenders: Vec<AerialDefender>,
}

/// 공중볼 타겟 (공격수)
#[derive(Debug, Clone)]
pub struct AerialTarget {
    pub idx: usize,
    pub name: String,
    pub heading: u8,
    pub jumping: u8,
    pub strength: u8,
    pub bravery: u8,
    pub positioning: u8,
    /// 현재 위치 (정규화)
    pub position: (f32, f32),
    /// 골대와의 거리
    pub distance_to_goal: f32,
    /// Gold AirRaid 트레이트
    pub has_airraid_gold: bool,
}

/// 공중볼 수비수
#[derive(Debug, Clone)]
pub struct AerialDefender {
    pub idx: usize,
    pub heading: u8,
    pub jumping: u8,
    pub strength: u8,
    pub bravery: u8,
    pub positioning: u8,
    pub marking: u8,
    /// 마킹 중인 공격수 인덱스
    pub marking_target: Option<usize>,
    pub position: (f32, f32),
}

// ============================================================================
// Free Kick Context
// ============================================================================

/// 프리킥 실행 컨텍스트
#[derive(Debug, Clone)]
pub struct FreeKickContext {
    pub is_home_attacking: bool,
    pub kicker_idx: usize,
    pub kicker_name: String,
    /// 프리킥 위치 (정규화)
    pub position: (f32, f32),
    /// 골대와의 거리 (미터)
    pub distance_to_goal: f32,
    /// 직접 슛 가능 여부
    pub can_shoot_direct: bool,
    /// 전술
    pub tactic: FreeKickTactic,
    /// 킥커 능력치
    pub free_kicks: u8,
    pub long_shots: u8,
    pub technique: u8,
    pub curve: u8,
    pub shot_power: u8,
    pub composure: u8,
    /// Gold DeadBall 트레이트
    pub has_deadball_gold: bool,
    /// GK 능력치
    pub gk_idx: usize,
    pub gk_reflexes: u8,
    pub gk_positioning: u8,
}

// ============================================================================
// Penalty Context
// ============================================================================

/// 페널티킥 실행 컨텍스트
#[derive(Debug, Clone)]
pub struct PenaltyContext {
    pub is_home_attacking: bool,
    pub kicker_idx: usize,
    pub kicker_name: String,
    /// 킥커 능력치
    pub penalty_taking: u8,
    pub composure: u8,
    pub finishing: u8,
    pub technique: u8,
    /// Gold DeadBall 트레이트
    pub has_deadball_gold: bool,
    /// GK 능력치
    pub gk_idx: usize,
    pub gk_name: String,
    pub gk_reflexes: u8,
    pub gk_diving: u8,
    pub gk_anticipation: u8,
}

// ============================================================================
// Set Piece Action FSM
// ============================================================================

/// 세트피스 액션 FSM
#[derive(Debug, Clone)]
pub struct SetPieceAction {
    /// 고유 ID
    pub id: u64,
    /// 세트피스 종류
    pub set_piece_type: SetPieceType,
    /// 현재 단계
    pub phase: SetPiecePhase,
    /// 공격팀이 홈인지
    pub is_home_attacking: bool,
    /// 킥커/스로어 인덱스
    pub kicker_idx: usize,
    /// 시작 틱
    pub start_tick: u64,
    /// 현재 단계 남은 틱
    pub phase_remaining_ticks: u8,
    /// 결과
    pub result: SetPieceResult,
    /// 코너킥 컨텍스트 (코너킥일 때만)
    pub corner_context: Option<CornerKickContext>,
    /// 프리킥 컨텍스트 (프리킥일 때만)
    pub freekick_context: Option<FreeKickContext>,
    /// 페널티 컨텍스트 (페널티일 때만)
    pub penalty_context: Option<PenaltyContext>,
}

impl SetPieceAction {
    /// 코너킥 FSM 생성
    pub fn new_corner(id: u64, start_tick: u64, ctx: CornerKickContext) -> Self {
        Self {
            id,
            set_piece_type: SetPieceType::Corner,
            phase: SetPiecePhase::Setup,
            is_home_attacking: ctx.is_home_attacking,
            kicker_idx: ctx.kicker_idx,
            start_tick,
            phase_remaining_ticks: CORNER_SETUP_TICKS,
            result: SetPieceResult::InProgress,
            corner_context: Some(ctx),
            freekick_context: None,
            penalty_context: None,
        }
    }

    /// 프리킥 FSM 생성
    pub fn new_freekick(id: u64, start_tick: u64, ctx: FreeKickContext, is_direct: bool) -> Self {
        let set_piece_type =
            if is_direct { SetPieceType::FreeKickDirect } else { SetPieceType::FreeKickIndirect };

        Self {
            id,
            set_piece_type,
            phase: SetPiecePhase::Setup,
            is_home_attacking: ctx.is_home_attacking,
            kicker_idx: ctx.kicker_idx,
            start_tick,
            phase_remaining_ticks: FREEKICK_SETUP_TICKS,
            result: SetPieceResult::InProgress,
            corner_context: None,
            freekick_context: Some(ctx),
            penalty_context: None,
        }
    }

    /// 페널티킥 FSM 생성
    pub fn new_penalty(id: u64, start_tick: u64, ctx: PenaltyContext) -> Self {
        Self {
            id,
            set_piece_type: SetPieceType::Penalty,
            phase: SetPiecePhase::Setup,
            is_home_attacking: ctx.is_home_attacking,
            kicker_idx: ctx.kicker_idx,
            start_tick,
            phase_remaining_ticks: PENALTY_SETUP_TICKS,
            result: SetPieceResult::InProgress,
            corner_context: None,
            freekick_context: None,
            penalty_context: Some(ctx),
        }
    }

    /// FSM tick 업데이트
    pub fn update_tick<R: Rng>(&mut self, _current_tick: u64, rng: &mut R) -> SetPieceResult {
        if self.phase == SetPiecePhase::Finished {
            return self.result.clone();
        }

        // 남은 틱 감소
        if self.phase_remaining_ticks > 0 {
            self.phase_remaining_ticks -= 1;
            if self.phase_remaining_ticks > 0 {
                return SetPieceResult::InProgress;
            }
        }

        // 단계 전이
        match self.set_piece_type {
            SetPieceType::Corner => self.tick_corner(rng),
            SetPieceType::FreeKickDirect | SetPieceType::FreeKickIndirect => {
                self.tick_freekick(rng)
            }
            SetPieceType::Penalty => self.tick_penalty(rng),
            SetPieceType::ThrowIn | SetPieceType::GoalKick => {
                // 스로인/골킥은 간단히 처리
                self.phase = SetPiecePhase::Finished;
                self.result = SetPieceResult::InProgress;
            }
        }

        self.result.clone()
    }

    /// 코너킥 틱 처리
    fn tick_corner<R: Rng>(&mut self, rng: &mut R) {
        let ctx = match &self.corner_context {
            Some(c) => c,
            None => {
                self.phase = SetPiecePhase::Finished;
                return;
            }
        };

        match self.phase {
            SetPiecePhase::Setup => {
                // Setup → Delivery
                self.phase = SetPiecePhase::Delivery;
                self.phase_remaining_ticks = CORNER_DELIVERY_TICKS;
            }
            SetPiecePhase::Delivery => {
                // 숏코너 체크
                if ctx.tactic == CornerTactic::Short {
                    // 숏코너: 가장 가까운 동료에게 패스
                    if let Some(target) = ctx.header_targets.first() {
                        self.result = SetPieceResult::AttackRetain { receiver_idx: target.idx };
                    }
                    self.phase = SetPiecePhase::Finished;
                    return;
                }

                // Delivery → Contest
                self.phase = SetPiecePhase::Contest;
                self.phase_remaining_ticks = CORNER_CONTEST_TICKS;
            }
            SetPiecePhase::Contest => {
                // 크로스 정확도 계산
                // FIX_2601/0123: corners 속성 추가
                let cross_accuracy =
                    calculate_cross_accuracy(ctx.corners, ctx.crossing, ctx.technique, ctx.vision, ctx.tactic);

                let cross_roll: f32 = rng.gen();

                if cross_roll > cross_accuracy {
                    // 크로스 실패 - 수비 클리어
                    if let Some(defender) = ctx.defenders.first() {
                        self.result = SetPieceResult::Cleared { defender_idx: defender.idx };
                    } else {
                        self.result = SetPieceResult::OutOfPlay;
                    }
                    self.phase = SetPiecePhase::Finished;
                    return;
                }

                // 크로스 성공 → 공중볼 경합
                self.phase = SetPiecePhase::Outcome;
                self.phase_remaining_ticks = 1;
            }
            SetPiecePhase::Outcome => {
                // 공중볼 경합 결과
                let result = resolve_aerial_duel(ctx, rng);
                self.result = result;
                self.phase = SetPiecePhase::Finished;
            }
            SetPiecePhase::Finished => {}
        }
    }

    /// 프리킥 틱 처리
    fn tick_freekick<R: Rng>(&mut self, rng: &mut R) {
        let ctx = match &self.freekick_context {
            Some(c) => c.clone(),
            None => {
                self.phase = SetPiecePhase::Finished;
                return;
            }
        };

        match self.phase {
            SetPiecePhase::Setup => {
                self.phase = SetPiecePhase::Delivery;
                self.phase_remaining_ticks = FREEKICK_DELIVERY_TICKS;
            }
            SetPiecePhase::Delivery => {
                match ctx.tactic {
                    FreeKickTactic::DirectShot => {
                        // 직접 슛 결과 계산
                        let result = resolve_direct_freekick(&ctx, rng);
                        self.result = result;
                        self.phase = SetPiecePhase::Finished;
                    }
                    FreeKickTactic::Cross | FreeKickTactic::LobBehind => {
                        // 크로스 → Contest
                        self.phase = SetPiecePhase::Contest;
                        self.phase_remaining_ticks = FREEKICK_CONTEST_TICKS;
                    }
                    FreeKickTactic::ShortPass => {
                        // 숏패스로 연결
                        self.result = SetPieceResult::InProgress;
                        self.phase = SetPiecePhase::Finished;
                    }
                }
            }
            SetPiecePhase::Contest => {
                // 크로스 후 경합 (코너킥과 유사하지만 간소화)
                let success_roll: f32 = rng.gen();
                if success_roll < 0.3 {
                    self.result =
                        SetPieceResult::ShotOnTarget { shooter_idx: ctx.kicker_idx, xg: 0.1 };
                } else if success_roll < 0.5 {
                    self.result = SetPieceResult::Cleared { defender_idx: ctx.gk_idx };
                }
                self.phase = SetPiecePhase::Finished;
            }
            SetPiecePhase::Outcome | SetPiecePhase::Finished => {
                self.phase = SetPiecePhase::Finished;
            }
        }
    }

    /// 페널티킥 틱 처리
    fn tick_penalty<R: Rng>(&mut self, rng: &mut R) {
        let ctx = match &self.penalty_context {
            Some(c) => c.clone(),
            None => {
                self.phase = SetPiecePhase::Finished;
                return;
            }
        };

        match self.phase {
            SetPiecePhase::Setup => {
                self.phase = SetPiecePhase::Delivery;
                self.phase_remaining_ticks = PENALTY_DELIVERY_TICKS;
            }
            SetPiecePhase::Delivery => {
                // 페널티킥 결과 계산
                let result = resolve_penalty(&ctx, rng);
                self.result = result;
                self.phase = SetPiecePhase::Finished;
            }
            _ => {
                self.phase = SetPiecePhase::Finished;
            }
        }
    }

    /// FSM이 완료되었는지
    pub fn is_finished(&self) -> bool {
        self.phase == SetPiecePhase::Finished
    }
}

// ============================================================================
// Duration Constants
// ============================================================================

/// 코너킥 Setup 틱 (선수 위치 잡기)
pub const CORNER_SETUP_TICKS: u8 = 3;
/// 코너킥 Delivery 틱 (크로스)
pub const CORNER_DELIVERY_TICKS: u8 = 2;
/// 코너킥 Contest 틱 (공중볼 경합)
pub const CORNER_CONTEST_TICKS: u8 = 2;

/// 프리킥 Setup 틱
pub const FREEKICK_SETUP_TICKS: u8 = 3;
/// 프리킥 Delivery 틱
pub const FREEKICK_DELIVERY_TICKS: u8 = 2;
/// 프리킥 Contest 틱
pub const FREEKICK_CONTEST_TICKS: u8 = 2;

/// 페널티 Setup 틱
pub const PENALTY_SETUP_TICKS: u8 = 2;
/// 페널티 Delivery 틱
pub const PENALTY_DELIVERY_TICKS: u8 = 1;

// ============================================================================
// Calculation Functions
// ============================================================================

/// 크로스 정확도 계산
///
/// FIX_2601/0123: corners 속성 추가 - 코너킥 전용 delivery quality
/// - corners: 코너킥 특화 능력 (가중치 0.3)
/// - crossing: 일반 크로스 능력 (가중치 0.25)
/// - technique: 기술 (가중치 0.25) - curve/placement에 영향
/// - vision: 시야 (가중치 0.2) - 타겟 선택 정확도
pub fn calculate_cross_accuracy(
    corners: u8,
    crossing: u8,
    technique: u8,
    vision: u8,
    tactic: CornerTactic,
) -> f32 {
    let corner_skill = skills::normalize(corners as f32);
    let cross_skill = skills::normalize(crossing as f32);
    let tech_skill = skills::normalize(technique as f32);
    let vis_skill = skills::normalize(vision as f32);

    // FIX_2601/0123: corners 속성이 가장 중요 (코너킥 특화)
    let base = corner_skill * 0.3 + cross_skill * 0.25 + tech_skill * 0.25 + vis_skill * 0.2;

    // 전술별 보정
    let tactic_mod = match tactic {
        CornerTactic::Short => 0.95, // 숏코너는 거의 성공
        CornerTactic::Inswing | CornerTactic::Outswing => 1.0,
        CornerTactic::NearPost => 0.9, // 니어 포스트는 타이밍이 어려움
        CornerTactic::FarPost => 0.85, // 파 포스트는 거리가 멀어 어려움
    };

    (base * tactic_mod).clamp(0.2, 0.9)
}

/// 공중볼 경합 점수 계산
pub fn calculate_aerial_score(
    heading: u8,
    jumping: u8,
    strength: u8,
    bravery: u8,
    positioning: u8,
    has_gold: bool,
) -> f32 {
    let h = skills::normalize(heading as f32);
    let j = skills::normalize(jumping as f32);
    let s = skills::normalize(strength as f32);
    let b = skills::normalize(bravery as f32);
    let p = skills::normalize(positioning as f32);

    let base = h * 0.35 + j * 0.25 + s * 0.15 + b * 0.1 + p * 0.15;

    if has_gold {
        (base + 0.15).min(0.95)
    } else {
        base
    }
}

/// 공중볼 경합 해결
pub fn resolve_aerial_duel<R: Rng>(ctx: &CornerKickContext, rng: &mut R) -> SetPieceResult {
    if ctx.header_targets.is_empty() {
        return SetPieceResult::OutOfPlay;
    }

    // 최고 공격수 찾기
    let mut best_attacker: Option<&AerialTarget> = None;
    let mut best_attack_score = 0.0f32;

    for target in &ctx.header_targets {
        let score = calculate_aerial_score(
            target.heading,
            target.jumping,
            target.strength,
            target.bravery,
            target.positioning,
            target.has_airraid_gold,
        );
        if score > best_attack_score {
            best_attack_score = score;
            best_attacker = Some(target);
        }
    }

    // 최고 수비수 찾기
    let mut best_defender: Option<&AerialDefender> = None;
    let mut best_defend_score = 0.0f32;

    for defender in &ctx.defenders {
        let score = calculate_aerial_score(
            defender.heading,
            defender.jumping,
            defender.strength,
            defender.bravery,
            defender.positioning,
            false,
        );
        if score > best_defend_score {
            best_defend_score = score;
            best_defender = Some(defender);
        }
    }

    let attacker = match best_attacker {
        Some(a) => a,
        None => return SetPieceResult::OutOfPlay,
    };

    // 경합 결과
    let roll: f32 = rng.gen();
    let attack_advantage = best_attack_score - best_defend_score * 0.8; // 공격 약간 유리

    if roll < (0.4 + attack_advantage).clamp(0.1, 0.7) {
        // 공격수 승리 - 헤딩
        let header_goal_chance = calculate_header_goal_chance(attacker, rng);

        if rng.gen::<f32>() < header_goal_chance {
            // 골!
            SetPieceResult::Goal {
                scorer_idx: attacker.idx,
                scorer_name: attacker.name.clone(),
                assist_idx: Some(ctx.kicker_idx),
            }
        } else {
            // 슛 시도
            let on_target = rng.gen::<f32>() < 0.5;
            if on_target {
                SetPieceResult::ShotOnTarget { shooter_idx: attacker.idx, xg: header_goal_chance }
            } else {
                SetPieceResult::ShotOffTarget { shooter_idx: attacker.idx, xg: header_goal_chance }
            }
        }
    } else if let Some(defender) = best_defender {
        // 수비수 승리 - 클리어
        SetPieceResult::Cleared { defender_idx: defender.idx }
    } else {
        SetPieceResult::OutOfPlay
    }
}

/// 헤딩 골 확률 계산
fn calculate_header_goal_chance<R: Rng>(target: &AerialTarget, rng: &mut R) -> f32 {
    let heading = skills::normalize(target.heading as f32);
    let jumping = skills::normalize(target.jumping as f32);

    let base_skill = heading * 0.6 + jumping * 0.4;

    // 골대와의 거리 보정 (가까울수록 유리)
    let distance_factor = (1.0 - target.distance_to_goal / 20.0).clamp(0.3, 1.0);

    // Gold AirRaid 보너스
    let gold_bonus = if target.has_airraid_gold { 0.15 } else { 0.0 };

    // 랜덤 요소
    let random_factor = 0.9 + rng.gen::<f32>() * 0.2;

    (base_skill * distance_factor * 0.25 + gold_bonus) * random_factor
}

/// 직접 프리킥 해결
pub fn resolve_direct_freekick<R: Rng>(ctx: &FreeKickContext, rng: &mut R) -> SetPieceResult {
    if !ctx.can_shoot_direct || ctx.distance_to_goal > 35.0 {
        return SetPieceResult::OutOfPlay;
    }

    let fk = skills::normalize(ctx.free_kicks as f32);
    let tech = skills::normalize(ctx.technique as f32);
    let curve = skills::normalize(ctx.curve as f32);
    let power = skills::normalize(ctx.shot_power as f32);
    let composure = skills::normalize(ctx.composure as f32);

    // 거리 기반 난이도
    let distance_factor = (1.0 - ctx.distance_to_goal / 40.0).clamp(0.2, 1.0);

    // 기본 골 확률
    let base_chance = (fk * 0.35 + tech * 0.2 + curve * 0.2 + power * 0.15 + composure * 0.1)
        * distance_factor
        * 0.15;

    // Gold DeadBall 보너스
    let goal_chance =
        if ctx.has_deadball_gold { (base_chance * 1.8).min(0.25) } else { base_chance.min(0.15) };

    // GK 세이브 확률
    let gk_reflexes = skills::normalize(ctx.gk_reflexes as f32);
    let gk_pos = skills::normalize(ctx.gk_positioning as f32);
    let save_factor = (gk_reflexes * 0.6 + gk_pos * 0.4) * 0.3;

    let roll: f32 = rng.gen();

    // 온타겟 확률
    let on_target_chance = (fk * 0.4 + tech * 0.3 + composure * 0.3).clamp(0.3, 0.7);

    if roll < goal_chance * (1.0 - save_factor) {
        // 직접 골!
        SetPieceResult::Goal {
            scorer_idx: ctx.kicker_idx,
            scorer_name: ctx.kicker_name.clone(),
            assist_idx: None,
        }
    } else if roll < goal_chance {
        // GK 세이브
        SetPieceResult::Save { gk_idx: ctx.gk_idx, shooter_idx: ctx.kicker_idx }
    } else if roll < on_target_chance {
        // 온타겟 슛
        SetPieceResult::ShotOnTarget { shooter_idx: ctx.kicker_idx, xg: goal_chance }
    } else {
        // 오프타겟
        SetPieceResult::ShotOffTarget { shooter_idx: ctx.kicker_idx, xg: goal_chance }
    }
}

/// 페널티킥 해결
pub fn resolve_penalty<R: Rng>(ctx: &PenaltyContext, rng: &mut R) -> SetPieceResult {
    let pk = skills::normalize(ctx.penalty_taking as f32);
    let composure = skills::normalize(ctx.composure as f32);
    let finishing = skills::normalize(ctx.finishing as f32);
    let technique = skills::normalize(ctx.technique as f32);

    // 기본 성공률: 75~90%
    let base_success = 0.75 + pk * 0.05 + composure * 0.04 + finishing * 0.03 + technique * 0.03;

    // Gold DeadBall 보너스
    let success_rate =
        if ctx.has_deadball_gold { (base_success * 1.1).min(0.95) } else { base_success.min(0.90) };

    // GK 세이브 확률 (최대 ~20%)
    let gk_reflex = skills::normalize(ctx.gk_reflexes as f32);
    let gk_diving = skills::normalize(ctx.gk_diving as f32);
    let gk_antic = skills::normalize(ctx.gk_anticipation as f32);
    let save_prob = (gk_reflex * 0.4 + gk_diving * 0.35 + gk_antic * 0.25) * 0.25;

    let roll: f32 = rng.gen();

    if roll < success_rate * (1.0 - save_prob) {
        // 골!
        SetPieceResult::Goal {
            scorer_idx: ctx.kicker_idx,
            scorer_name: ctx.kicker_name.clone(),
            assist_idx: None,
        }
    } else if roll < success_rate {
        // GK 세이브
        SetPieceResult::Save { gk_idx: ctx.gk_idx, shooter_idx: ctx.kicker_idx }
    } else {
        // 미스 (골대 벗어남)
        SetPieceResult::ShotOffTarget {
            shooter_idx: ctx.kicker_idx,
            xg: 0.76, // 평균 페널티 전환율
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn make_test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn test_cross_accuracy_short_corner() {
        // FIX_2601/0123: corners 속성 추가
        let acc = calculate_cross_accuracy(15, 15, 14, 13, CornerTactic::Short);
        // Short corner tactic_mod = 0.95, but base accuracy depends on skills
        assert!(acc > 0.6, "Short corner should be fairly accurate: {}", acc);
    }

    #[test]
    fn test_cross_accuracy_normal() {
        // FIX_2601/0123: corners 속성 추가
        let acc = calculate_cross_accuracy(15, 15, 14, 13, CornerTactic::Inswing);
        assert!(acc > 0.4 && acc < 0.8, "Normal cross accuracy: {}", acc);
    }

    #[test]
    fn test_cross_accuracy_corners_attribute_impact() {
        // FIX_2601/0123: corners 속성이 크로스 정확도에 영향을 주는지 테스트
        let low_corners = calculate_cross_accuracy(8, 15, 14, 13, CornerTactic::Inswing);
        let high_corners = calculate_cross_accuracy(18, 15, 14, 13, CornerTactic::Inswing);
        assert!(
            high_corners > low_corners,
            "Higher corners attribute should improve accuracy: {} vs {}",
            high_corners,
            low_corners
        );
    }

    #[test]
    fn test_aerial_score() {
        let score = calculate_aerial_score(16, 15, 14, 13, 12, false);
        assert!(score > 0.3 && score < 0.8, "Aerial score: {}", score);
    }

    #[test]
    fn test_aerial_score_gold() {
        let normal = calculate_aerial_score(15, 14, 13, 12, 11, false);
        let gold = calculate_aerial_score(15, 14, 13, 12, 11, true);
        assert!(gold > normal, "Gold should increase score");
    }

    #[test]
    fn test_penalty_success_rate() {
        let ctx = PenaltyContext {
            is_home_attacking: true,
            kicker_idx: 9,
            kicker_name: "Striker".to_string(),
            penalty_taking: 16,
            composure: 15,
            finishing: 17,
            technique: 14,
            has_deadball_gold: false,
            gk_idx: 11,
            gk_name: "GK".to_string(),
            gk_reflexes: 14,
            gk_diving: 15,
            gk_anticipation: 13,
        };

        let mut rng = make_test_rng();
        let mut goals = 0;
        let iterations = 1000;

        for _ in 0..iterations {
            let result = resolve_penalty(&ctx, &mut rng);
            if matches!(result, SetPieceResult::Goal { .. }) {
                goals += 1;
            }
        }

        let rate = goals as f32 / iterations as f32;
        assert!(rate > 0.65 && rate < 0.85, "Penalty success rate: {}", rate);
    }

    #[test]
    fn test_penalty_gold_bonus() {
        let ctx_normal = PenaltyContext {
            is_home_attacking: true,
            kicker_idx: 9,
            kicker_name: "Striker".to_string(),
            penalty_taking: 14,
            composure: 13,
            finishing: 14,
            technique: 12,
            has_deadball_gold: false,
            gk_idx: 11,
            gk_name: "GK".to_string(),
            gk_reflexes: 14,
            gk_diving: 14,
            gk_anticipation: 13,
        };

        let ctx_gold = PenaltyContext { has_deadball_gold: true, ..ctx_normal.clone() };

        let mut rng1 = make_test_rng();
        let mut rng2 = make_test_rng();
        let iterations = 500;

        let mut normal_goals = 0;
        let mut gold_goals = 0;

        for _ in 0..iterations {
            if matches!(resolve_penalty(&ctx_normal, &mut rng1), SetPieceResult::Goal { .. }) {
                normal_goals += 1;
            }
            if matches!(resolve_penalty(&ctx_gold, &mut rng2), SetPieceResult::Goal { .. }) {
                gold_goals += 1;
            }
        }

        assert!(
            gold_goals >= normal_goals,
            "Gold should improve or equal: {} vs {}",
            gold_goals,
            normal_goals
        );
    }

    #[test]
    fn test_direct_freekick_close_range() {
        let ctx = FreeKickContext {
            is_home_attacking: true,
            kicker_idx: 7,
            kicker_name: "Midfielder".to_string(),
            position: (0.5, 0.85),
            distance_to_goal: 18.0, // Close range
            can_shoot_direct: true,
            tactic: FreeKickTactic::DirectShot,
            free_kicks: 17,
            long_shots: 15,
            technique: 16,
            curve: 15,
            shot_power: 14,
            composure: 14,
            has_deadball_gold: true,
            gk_idx: 11,
            gk_reflexes: 14,
            gk_positioning: 13,
        };

        let mut rng = make_test_rng();
        let mut goals = 0;
        let iterations = 500;

        for _ in 0..iterations {
            let result = resolve_direct_freekick(&ctx, &mut rng);
            if matches!(result, SetPieceResult::Goal { .. }) {
                goals += 1;
            }
        }

        let rate = goals as f32 / iterations as f32;
        assert!(rate > 0.05, "Close FK should have some goal chance: {}", rate);
    }

    #[test]
    fn test_corner_kick_fsm_phases() {
        let ctx = CornerKickContext {
            is_home_attacking: true,
            kicker_idx: 3,
            kicker_name: "Winger".to_string(),
            is_left_corner: true,
            tactic: CornerTactic::Inswing,
            corners: 16,  // FIX_2601/0123: corners 속성 추가
            crossing: 16,
            technique: 15,
            vision: 14,
            curve: 15,
            header_targets: vec![AerialTarget {
                idx: 4,
                name: "CB".to_string(),
                heading: 17,
                jumping: 16,
                strength: 15,
                bravery: 14,
                positioning: 13,
                position: (0.9, 0.5),
                distance_to_goal: 8.0,
                has_airraid_gold: false,
            }],
            defenders: vec![AerialDefender {
                idx: 13,
                heading: 14,
                jumping: 13,
                strength: 14,
                bravery: 12,
                positioning: 13,
                marking: 14,
                marking_target: Some(4),
                position: (0.9, 0.5),
            }],
        };

        let mut action = SetPieceAction::new_corner(1, 0, ctx);
        let mut rng = make_test_rng();

        // Phase transitions
        assert_eq!(action.phase, SetPiecePhase::Setup);

        // Run until finished
        for tick in 0..20 {
            let result = action.update_tick(tick, &mut rng);
            if action.is_finished() {
                assert!(!matches!(result, SetPieceResult::InProgress));
                break;
            }
        }

        assert!(action.is_finished(), "Corner should finish within 20 ticks");
    }

    #[test]
    fn test_penalty_fsm_phases() {
        let ctx = PenaltyContext {
            is_home_attacking: true,
            kicker_idx: 9,
            kicker_name: "Striker".to_string(),
            penalty_taking: 15,
            composure: 14,
            finishing: 16,
            technique: 13,
            has_deadball_gold: false,
            gk_idx: 11,
            gk_name: "GK".to_string(),
            gk_reflexes: 14,
            gk_diving: 14,
            gk_anticipation: 13,
        };

        let mut action = SetPieceAction::new_penalty(1, 0, ctx);
        let mut rng = make_test_rng();

        assert_eq!(action.phase, SetPiecePhase::Setup);

        // Run until finished
        for tick in 0..10 {
            let _result = action.update_tick(tick, &mut rng);
            if action.is_finished() {
                break;
            }
        }

        assert!(action.is_finished(), "Penalty should finish within 10 ticks");
        assert!(!matches!(action.result, SetPieceResult::InProgress));
    }
}
