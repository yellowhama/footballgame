//! FIX_2601/1129: 팀 단위 공격 국면 (AttackPhase)
//!
//! forward_pass_rate, reciprocity, density 지표 정상화를 위한 팀 상태 모델.
//!
//! ## 핵심 원리
//!
//! 현실 축구에서 전진은 "가능해서" 하는 게 아니라 "조건이 충족될 때만" 한다.
//! - 압박이 강하면: 유지(Circulation)
//! - 라인 정렬되면: 지공(Positional)
//! - 공 탈취 직후: 속공(Transition)
//!
//! ## 지표 영향
//!
//! - Circulation → reciprocity/density 자연 상승 (왕복/횡패스 발생)
//! - Positional → forward_pass_rate 적정 범위 (조건부 전진)
//! - Transition → forward_pass_rate 상승 허용 (속공 합법화)

use serde::{Deserialize, Serialize};

/// 팀 단위 공격 국면 (SSOT)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AttackPhase {
    /// 점유 유지 (순환)
    ///
    /// **언제**: 압박이 강하거나 전방 옵션 불확실
    /// **행동**: 횡패스/백패스/왕복 우선
    /// **효과**: forward 제한, reciprocity 자연 발생
    #[default]
    Circulation,

    /// 지공 (점진적 전진)
    ///
    /// **언제**: 라인 정렬 완료, 전방 옵션 일부 열림
    /// **행동**: 조건부 전진 허용
    /// **효과**: forward_pass_rate 20-30% 범위
    Positional,

    /// 속공 (빠른 전환)
    ///
    /// **언제**: 공 탈취 직후, 수적 우위
    /// **행동**: 전진/직결 허용
    /// **효과**: forward 높아도 정상
    Transition,
}

impl AttackPhase {
    /// 전진 패스 가중치 배율
    ///
    /// Circulation에서는 전진이 거의 금지되고,
    /// Transition에서는 전진이 우대된다.
    pub fn progression_weight_multiplier(&self) -> f32 {
        match self {
            AttackPhase::Circulation => 0.1,  // 전진 거의 금지
            AttackPhase::Positional => 0.6,   // 조건부 전진
            AttackPhase::Transition => 1.5,   // 전진 우대
        }
    }

    /// backward/lateral 패스 가중치 배율
    pub fn circulation_weight_multiplier(&self) -> f32 {
        match self {
            AttackPhase::Circulation => 1.5,  // 순환 우대
            AttackPhase::Positional => 1.0,   // 기본
            AttackPhase::Transition => 0.5,   // 순환 지양
        }
    }

    /// 전진이 허용되는 조건인지 (강제 제한)
    pub fn allows_forward_pass(&self) -> bool {
        match self {
            AttackPhase::Circulation => false, // 전진 금지
            AttackPhase::Positional => true,   // 조건부 허용
            AttackPhase::Transition => true,   // 허용
        }
    }
}

/// 팀 상태
#[derive(Debug, Clone)]
pub struct TeamAttackState {
    /// 현재 공격 국면
    pub phase: AttackPhase,
    /// 현재 phase 시작 틱
    pub phase_since_tick: u64,
    /// 이전 phase (디버깅/분석용)
    pub last_phase: AttackPhase,
    /// 최근 전진 패스 실패 횟수
    pub recent_forward_failures: u32,
    /// 공 탈취 후 경과 틱
    pub ticks_since_turnover: u64,
}

impl Default for TeamAttackState {
    fn default() -> Self {
        Self {
            phase: AttackPhase::Circulation,
            phase_since_tick: 0,
            last_phase: AttackPhase::Circulation,
            recent_forward_failures: 0,
            ticks_since_turnover: u64::MAX, // 아직 탈취 없음
        }
    }
}

impl TeamAttackState {
    /// 새 상태 생성
    pub fn new() -> Self {
        Self::default()
    }

    /// phase 변경
    pub fn transition_to(&mut self, new_phase: AttackPhase, current_tick: u64) {
        if new_phase != self.phase {
            self.last_phase = self.phase;
            self.phase = new_phase;
            self.phase_since_tick = current_tick;

            // Transition으로 진입하면 실패 카운트 리셋
            if new_phase == AttackPhase::Transition {
                self.recent_forward_failures = 0;
            }
        }
    }

    /// 점유 상실 시 초기화
    pub fn on_possession_lost(&mut self, current_tick: u64) {
        self.transition_to(AttackPhase::Circulation, current_tick);
        self.recent_forward_failures = 0;
        self.ticks_since_turnover = u64::MAX;
    }

    /// 공 탈취 시
    pub fn on_turnover(&mut self, current_tick: u64) {
        self.ticks_since_turnover = 0;
    }

    /// 틱 업데이트
    pub fn tick(&mut self) {
        if self.ticks_since_turnover < u64::MAX {
            self.ticks_since_turnover += 1;
        }
    }

    /// 전진 패스 실패 기록
    pub fn record_forward_failure(&mut self) {
        self.recent_forward_failures = self.recent_forward_failures.saturating_add(1);
    }

    /// 전진 패스 성공 시 실패 카운트 리셋
    pub fn record_forward_success(&mut self) {
        self.recent_forward_failures = 0;
    }
}

/// 상태 전이 판단을 위한 컨텍스트
#[derive(Debug, Clone)]
pub struct PhaseTransitionContext {
    /// 전방 패스 옵션 수
    pub forward_options: usize,
    /// 팀 라인 길이 (수비-공격 간격, 미터)
    pub team_length_m: f32,
    /// 볼 캐리어 근처 압박 수준 (0.0-1.0)
    pub local_pressure: f32,
    /// 전방 공격수 수
    pub attackers_ahead: usize,
    /// 전방 수비수 수
    pub defenders_ahead: usize,
    /// 전방 공간이 열렸는지
    pub forward_space_open: bool,
    /// 현재 틱
    pub current_tick: u64,
}

/// 상태 전이 상수
pub mod transition_constants {
    /// 최소 phase 유지 시간 (틱)
    pub const MIN_PHASE_DURATION_TICKS: u64 = 10;
    /// 속공 타임아웃 (틱)
    pub const TRANSITION_TIMEOUT_TICKS: u64 = 30;
    /// 속공 진입 조건: 탈취 후 최대 틱
    pub const TRANSITION_TURNOVER_WINDOW: u64 = 5;
    /// Positional 진입 조건: 최소 전방 옵션 수
    pub const POSITIONAL_MIN_FORWARD_OPTIONS: usize = 2;
    /// Positional 진입 조건: 최대 팀 길이
    pub const POSITIONAL_MAX_TEAM_LENGTH_M: f32 = 40.0;
    /// Positional 진입 조건: 최대 압박 수준
    pub const POSITIONAL_MAX_PRESSURE: f32 = 0.5;
    /// Circulation 복귀 조건: 최대 전방 옵션
    pub const CIRCULATION_MAX_FORWARD_OPTIONS: usize = 1;
    /// Circulation 복귀 조건: 압박 임계값
    pub const CIRCULATION_PRESSURE_THRESHOLD: f32 = 0.7;
    /// Circulation 복귀 조건: 최대 전진 실패 횟수
    pub const CIRCULATION_MAX_FORWARD_FAILURES: u32 = 2;
}

/// 상태 전이 로직
pub fn determine_phase(
    state: &TeamAttackState,
    ctx: &PhaseTransitionContext,
) -> AttackPhase {
    use transition_constants::*;

    // 최소 유지 시간 체크
    if ctx.current_tick.saturating_sub(state.phase_since_tick) < MIN_PHASE_DURATION_TICKS {
        return state.phase;
    }

    // 1. Transition 진입 조건 (최우선)
    if is_transition_opportunity(state, ctx) {
        return AttackPhase::Transition;
    }

    // 2. Transition 만료
    if state.phase == AttackPhase::Transition {
        if ctx.current_tick.saturating_sub(state.phase_since_tick) > TRANSITION_TIMEOUT_TICKS {
            return AttackPhase::Positional;
        }
        return AttackPhase::Transition; // 유지
    }

    // 3. Circulation 복귀 조건
    if should_fall_back_to_circulation(state, ctx) {
        return AttackPhase::Circulation;
    }

    // 4. Positional 진입 조건
    if can_transition_to_positional(ctx) {
        return AttackPhase::Positional;
    }

    // 5. 현재 상태 유지
    state.phase
}

fn is_transition_opportunity(state: &TeamAttackState, ctx: &PhaseTransitionContext) -> bool {
    use transition_constants::*;

    // 공 탈취 직후
    state.ticks_since_turnover < TRANSITION_TURNOVER_WINDOW
        // 수적 우위
        && ctx.attackers_ahead > ctx.defenders_ahead
        // 전방 공간 열림
        && ctx.forward_space_open
}

fn can_transition_to_positional(ctx: &PhaseTransitionContext) -> bool {
    use transition_constants::*;

    // 전방 패스 옵션이 충분
    ctx.forward_options >= POSITIONAL_MIN_FORWARD_OPTIONS
        // 팀 라인 간격 안정
        && ctx.team_length_m < POSITIONAL_MAX_TEAM_LENGTH_M
        // 압박 밀도 낮음
        && ctx.local_pressure < POSITIONAL_MAX_PRESSURE
}

fn should_fall_back_to_circulation(
    state: &TeamAttackState,
    ctx: &PhaseTransitionContext,
) -> bool {
    use transition_constants::*;

    // Circulation 상태면 복귀 불필요
    if state.phase == AttackPhase::Circulation {
        return false;
    }

    // 전방 옵션 부족
    ctx.forward_options < CIRCULATION_MAX_FORWARD_OPTIONS
        // 또는 압박 증가
        || ctx.local_pressure > CIRCULATION_PRESSURE_THRESHOLD
        // 또는 전진 패스 연속 실패
        || state.recent_forward_failures >= CIRCULATION_MAX_FORWARD_FAILURES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_phase_is_circulation() {
        let state = TeamAttackState::new();
        assert_eq!(state.phase, AttackPhase::Circulation);
    }

    #[test]
    fn test_phase_transition() {
        let mut state = TeamAttackState::new();
        state.transition_to(AttackPhase::Positional, 100);
        assert_eq!(state.phase, AttackPhase::Positional);
        assert_eq!(state.last_phase, AttackPhase::Circulation);
        assert_eq!(state.phase_since_tick, 100);
    }

    #[test]
    fn test_circulation_weights() {
        assert!(AttackPhase::Circulation.progression_weight_multiplier() < 0.5);
        assert!(AttackPhase::Circulation.circulation_weight_multiplier() > 1.0);
    }

    #[test]
    fn test_transition_weights() {
        assert!(AttackPhase::Transition.progression_weight_multiplier() > 1.0);
        assert!(AttackPhase::Transition.circulation_weight_multiplier() < 1.0);
    }

    #[test]
    fn test_transition_opportunity() {
        let mut state = TeamAttackState::new();
        state.ticks_since_turnover = 2; // 탈취 직후

        let ctx = PhaseTransitionContext {
            forward_options: 3,
            team_length_m: 35.0,
            local_pressure: 0.3,
            attackers_ahead: 3,
            defenders_ahead: 2, // 수적 우위
            forward_space_open: true,
            current_tick: 100,
        };

        let phase = determine_phase(&state, &ctx);
        assert_eq!(phase, AttackPhase::Transition);
    }

    #[test]
    fn test_positional_transition() {
        let mut state = TeamAttackState::new();
        state.phase_since_tick = 0;

        let ctx = PhaseTransitionContext {
            forward_options: 3,
            team_length_m: 35.0,
            local_pressure: 0.3,
            attackers_ahead: 2,
            defenders_ahead: 3,
            forward_space_open: false,
            current_tick: 100, // MIN_PHASE_DURATION 경과
        };

        let phase = determine_phase(&state, &ctx);
        assert_eq!(phase, AttackPhase::Positional);
    }

    #[test]
    fn test_circulation_fallback() {
        let mut state = TeamAttackState::new();
        state.phase = AttackPhase::Positional;
        state.phase_since_tick = 0;
        state.recent_forward_failures = 3;

        let ctx = PhaseTransitionContext {
            forward_options: 0, // 전방 옵션 없음
            team_length_m: 45.0,
            local_pressure: 0.8, // 압박 높음
            attackers_ahead: 1,
            defenders_ahead: 4,
            forward_space_open: false,
            current_tick: 100,
        };

        let phase = determine_phase(&state, &ctx);
        assert_eq!(phase, AttackPhase::Circulation);
    }
}
