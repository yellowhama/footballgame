//! Team Phase State Machine
//!
//! 팀의 현재 경기 페이즈를 관리합니다.
//! - Attack: 공격 조직 완료, 상대 진영에서 플레이
//! - Defense: 수비 조직 완료, 자기 진영 보호
//! - TransitionAttack: 공 탈취 직후, 역습 기회
//! - TransitionDefense: 공 상실 직후, 수비 복귀 중
//!
//! FIX_2601/1128: Attack Sub-Phases (for reciprocity/network improvements)
//! - Circulation: 압박 상황, 전진 불가 → 순환 패스 (backward/lateral 선호)
//! - Progression: 전진 가능 → 전방 패스 허용
//! - Finalization: 슈팅 존 도달 → 마무리 시도

use serde::{Deserialize, Serialize};

/// 팀의 현재 경기 페이즈
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TeamPhase {
    /// 공격 조직 완료 - 상대 진영에서 빌드업/찬스 메이킹
    #[default]
    Attack,
    /// 수비 조직 완료 - 자기 진영 보호, 블록 형성
    Defense,
    /// 공 탈취 직후 - 역습 기회, 빠른 전환
    TransitionAttack,
    /// 공 상실 직후 - 수비 복귀 중, 지연 플레이
    TransitionDefense,
}

impl TeamPhase {
    /// 이 페이즈가 공격적인 페이즈인지
    pub fn is_attacking(&self) -> bool {
        matches!(self, TeamPhase::Attack | TeamPhase::TransitionAttack)
    }

    /// 이 페이즈가 전환 페이즈인지
    pub fn is_transition(&self) -> bool {
        matches!(self, TeamPhase::TransitionAttack | TeamPhase::TransitionDefense)
    }

    /// 이 페이즈에서 선수들이 전방 압박을 해야 하는지
    pub fn should_press(&self) -> bool {
        matches!(self, TeamPhase::TransitionDefense | TeamPhase::Defense)
    }
}

/// FIX_2601/1128: 공격 하위 국면
///
/// Attack 상태에서 세부적인 플레이 스타일을 결정합니다.
/// - Circulation: 압박 상황 → 구조 유지, 전진 제한
/// - Progression: 전진 가능 → 전방 패스 허용
/// - Finalization: 슈팅 존 → 마무리 시도
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AttackSubPhase {
    /// 순환 - 압박 상황에서 구조 유지
    /// 전방 패스 제한, backward/lateral 패스 선호
    /// 이전 패스 상대에게 돌려주는 것이 자연스러움
    #[default]
    Circulation,
    /// 전진 - 전방 옵션이 있을 때 전진 시도
    /// 전방 패스 허용, 라인 돌파 시도
    Progression,
    /// 마무리 - 슈팅 존에서 득점 시도
    /// 슛/크로스 우선, 찬스 메이킹
    Finalization,
}

impl AttackSubPhase {
    /// 이 국면에서 전방 패스를 선호해야 하는지
    pub fn prefers_forward_pass(&self) -> bool {
        matches!(self, AttackSubPhase::Progression)
    }

    /// 이 국면에서 순환 패스를 선호해야 하는지
    pub fn prefers_circulation(&self) -> bool {
        matches!(self, AttackSubPhase::Circulation)
    }

    /// 이 국면에서 슈팅을 우선해야 하는지
    pub fn prefers_shooting(&self) -> bool {
        matches!(self, AttackSubPhase::Finalization)
    }

    /// 전방 패스 가중치 배율 (1.0 = 기본)
    /// FIX_2601/1128: VERY extreme weights to achieve forward_pass_rate 22% target
    pub fn forward_pass_weight_multiplier(&self) -> f32 {
        match self {
            AttackSubPhase::Circulation => 0.02,   // 전방 패스 2%로 극도로 제한 (was 4%)
            AttackSubPhase::Progression => 0.8,    // 전방 패스 20% 감소 (was 1.0)
            AttackSubPhase::Finalization => 0.5,   // 슈팅 존에서는 슛 우선 (was 0.6)
        }
    }

    /// 순환 패스(backward/lateral) 가중치 배율
    /// FIX_2601/1128: VERY strong backward preference to achieve forward_pass_rate 22% target
    pub fn circulation_pass_weight_multiplier(&self) -> f32 {
        match self {
            AttackSubPhase::Circulation => 12.0,   // 순환 패스 1100% 보너스 (was 8.0)
            AttackSubPhase::Progression => 1.2,    // 순환 패스 20% 보너스 (was 0.85) - encourage backward even in Progression
            AttackSubPhase::Finalization => 1.0,   // 기본
        }
    }
}

/// 전환 안정화에 필요한 틱 수 (30틱 = 3초 @ 10 ticks/sec)
const TRANSITION_SETTLE_TICKS: u64 = 30;

/// 팀 페이즈 상태 관리자
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamPhaseState {
    /// 현재 페이즈
    pub phase: TeamPhase,
    /// 현재 페이즈 시작 틱
    pub phase_start_tick: u64,
    /// 마지막 점유 변경 틱
    pub last_possession_change_tick: u64,
    /// 이 팀이 현재 공을 소유하고 있는지
    pub has_possession: bool,
    /// FIX_2601/1128: 공격 하위 국면 (Attack 상태에서만 의미 있음)
    pub attack_sub_phase: AttackSubPhase,
    /// FIX_2601/1128: 연속 전방 패스 실패 횟수
    pub consecutive_forward_failures: u8,
    /// FIX_2601/1128: 현재 압박 수준 (0.0 ~ 1.0)
    pub current_pressure: f32,
}

impl Default for TeamPhaseState {
    fn default() -> Self {
        Self {
            phase: TeamPhase::Defense,
            phase_start_tick: 0,
            last_possession_change_tick: 0,
            has_possession: false,
            attack_sub_phase: AttackSubPhase::default(),
            consecutive_forward_failures: 0,
            current_pressure: 0.0,
        }
    }
}

impl TeamPhaseState {
    /// 새 상태 생성
    pub fn new(initial_phase: TeamPhase) -> Self {
        Self {
            phase: initial_phase,
            phase_start_tick: 0,
            last_possession_change_tick: 0,
            has_possession: initial_phase.is_attacking(),
            attack_sub_phase: if initial_phase.is_attacking() {
                AttackSubPhase::Circulation
            } else {
                AttackSubPhase::default()
            },
            consecutive_forward_failures: 0,
            current_pressure: 0.0,
        }
    }

    /// 공격 시작 상태로 생성 (킥오프 팀용)
    pub fn attacking() -> Self {
        Self::new(TeamPhase::Attack)
    }

    /// 수비 시작 상태로 생성
    pub fn defending() -> Self {
        Self::new(TeamPhase::Defense)
    }

    /// 현재 페이즈에서 경과한 틱 수
    pub fn ticks_in_phase(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.phase_start_tick)
    }

    /// 점유 변경 후 경과한 틱 수
    pub fn ticks_since_possession_change(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.last_possession_change_tick)
    }

    /// 페이즈 업데이트
    ///
    /// # Arguments
    /// * `has_possession` - 이 팀이 현재 공을 소유하고 있는지
    /// * `current_tick` - 현재 시뮬레이션 틱
    ///
    /// # Returns
    /// 페이즈가 변경되었으면 이전 페이즈 반환
    pub fn update(&mut self, has_possession: bool, current_tick: u64) -> Option<TeamPhase> {
        let possession_changed = has_possession != self.has_possession;
        self.has_possession = has_possession;

        if possession_changed {
            self.last_possession_change_tick = current_tick;
        }

        let old_phase = self.phase;
        let new_phase = self.calculate_next_phase(current_tick);

        if new_phase != old_phase {
            self.phase = new_phase;
            self.phase_start_tick = current_tick;
            Some(old_phase)
        } else {
            None
        }
    }

    /// 다음 페이즈 계산
    fn calculate_next_phase(&self, current_tick: u64) -> TeamPhase {
        let ticks_since_change = self.ticks_since_possession_change(current_tick);

        match (self.has_possession, self.phase) {
            // 공 소유 중
            (true, TeamPhase::Defense) => {
                // 수비 → 전환공격
                TeamPhase::TransitionAttack
            }
            (true, TeamPhase::TransitionDefense) => {
                // 전환수비 중 공 탈취 → 전환공격
                TeamPhase::TransitionAttack
            }
            (true, TeamPhase::TransitionAttack) => {
                // 전환공격 → 일정 시간 후 공격 안정화
                if ticks_since_change >= TRANSITION_SETTLE_TICKS {
                    TeamPhase::Attack
                } else {
                    TeamPhase::TransitionAttack
                }
            }
            (true, TeamPhase::Attack) => TeamPhase::Attack,

            // 공 미소유
            (false, TeamPhase::Attack) => {
                // 공격 → 전환수비
                TeamPhase::TransitionDefense
            }
            (false, TeamPhase::TransitionAttack) => {
                // 전환공격 중 공 상실 → 전환수비
                TeamPhase::TransitionDefense
            }
            (false, TeamPhase::TransitionDefense) => {
                // 전환수비 → 일정 시간 후 수비 안정화
                if ticks_since_change >= TRANSITION_SETTLE_TICKS {
                    TeamPhase::Defense
                } else {
                    TeamPhase::TransitionDefense
                }
            }
            (false, TeamPhase::Defense) => TeamPhase::Defense,
        }
    }

    /// 세트피스 등으로 강제 페이즈 설정
    pub fn force_phase(&mut self, phase: TeamPhase, current_tick: u64) {
        self.phase = phase;
        self.phase_start_tick = current_tick;
        self.has_possession = phase.is_attacking();
        // FIX_2601/1128: Reset sub-phase on phase change
        if phase == TeamPhase::Attack {
            self.attack_sub_phase = AttackSubPhase::Circulation; // Start with circulation
        }
    }

    // ====== FIX_2601/1128: Attack Sub-Phase Management ======

    /// 공격 하위 국면 업데이트
    ///
    /// # Arguments
    /// * `pressure` - 현재 압박 수준 (0.0 ~ 1.0)
    /// * `forward_options` - 전방 패스 옵션 수
    /// * `dist_to_goal_m` - 골까지 거리 (미터)
    /// * `forward_pass_succeeded` - 직전 전방 패스 성공 여부 (Some = 패스 발생)
    pub fn update_attack_sub_phase(
        &mut self,
        pressure: f32,
        forward_options: usize,
        dist_to_goal_m: f32,
        forward_pass_succeeded: Option<bool>,
    ) {
        // Only relevant during Attack phase
        if self.phase != TeamPhase::Attack {
            return;
        }

        self.current_pressure = pressure;

        // Track forward pass failures
        if let Some(succeeded) = forward_pass_succeeded {
            if !succeeded {
                self.consecutive_forward_failures = self.consecutive_forward_failures.saturating_add(1);
            } else {
                self.consecutive_forward_failures = 0;
            }
        }

        // Finalization zone check (within 25m of goal)
        if dist_to_goal_m < 25.0 {
            self.attack_sub_phase = AttackSubPhase::Finalization;
            return;
        }

        // FIX_2601/1128: Stricter conditions for Progression (target: 80% Circulation)
        // In real football, most possession is circulation (probing, maintaining shape)
        // Progression only happens when ALL conditions are met:
        // 1. Extremely low pressure (< 0.10) - defender is far away
        // 2. Many forward options (>= 7) - multiple clear passing lanes ahead
        // 3. No recent failures (== 0) - perfect recent execution
        // This makes Circulation the ~80% default mode
        let can_progress =
            pressure < 0.10 // Extremely low pressure required (0.15 → 0.10)
            && forward_options >= 7 // Many forward options required (6 → 7)
            && self.consecutive_forward_failures == 0; // No recent failures allowed (< 2 → == 0)

        if can_progress {
            self.attack_sub_phase = AttackSubPhase::Progression;
        } else {
            self.attack_sub_phase = AttackSubPhase::Circulation;
        }
    }

    /// 현재 공격 하위 국면 조회
    pub fn get_attack_sub_phase(&self) -> AttackSubPhase {
        if self.phase.is_attacking() {
            self.attack_sub_phase
        } else {
            AttackSubPhase::Circulation // Default for non-attack phases
        }
    }

    /// 전방 패스 가중치 배율 조회
    /// FIX_2601/1128: Extended to also apply sub-phase logic in TransitionAttack
    pub fn forward_pass_weight(&self) -> f32 {
        if self.phase == TeamPhase::Attack {
            self.attack_sub_phase.forward_pass_weight_multiplier()
        } else if self.phase == TeamPhase::TransitionAttack {
            // Also apply sub-phase during transitions with stronger effect
            match self.attack_sub_phase {
                AttackSubPhase::Circulation => 0.15, // Strong reduction in Circulation
                AttackSubPhase::Progression => 1.2,  // Counter-attack: favor forward
                AttackSubPhase::Finalization => 0.7, // Near goal: moderate forward
            }
        } else {
            1.0
        }
    }

    /// 순환 패스 가중치 배율 조회
    /// FIX_2601/1128: Extended to also apply sub-phase logic in TransitionAttack
    pub fn circulation_pass_weight(&self) -> f32 {
        if self.phase == TeamPhase::Attack {
            self.attack_sub_phase.circulation_pass_weight_multiplier()
        } else if self.phase == TeamPhase::TransitionAttack {
            // Also apply sub-phase during transitions with stronger effect
            match self.attack_sub_phase {
                AttackSubPhase::Circulation => 3.0,  // Strong backward preference in Circulation
                AttackSubPhase::Progression => 0.85, // Counter-attack: reduce circulation
                AttackSubPhase::Finalization => 1.0, // Near goal: normal
            }
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_phase_transition_attack() {
        let mut state = TeamPhaseState::defending();
        assert_eq!(state.phase, TeamPhase::Defense);
        assert!(!state.has_possession);

        // 공 획득 → 전환공격
        let changed = state.update(true, 10);
        assert_eq!(changed, Some(TeamPhase::Defense));
        assert_eq!(state.phase, TeamPhase::TransitionAttack);

        // 아직 안정화 안됨 (10틱 후)
        state.update(true, 20);
        assert_eq!(state.phase, TeamPhase::TransitionAttack);

        // 안정화 완료 (30틱 후)
        state.update(true, 50);
        assert_eq!(state.phase, TeamPhase::Attack);
    }

    #[test]
    fn test_team_phase_transition_defense() {
        let mut state = TeamPhaseState::attacking();
        assert_eq!(state.phase, TeamPhase::Attack);

        // 공 상실 → 전환수비
        let changed = state.update(false, 10);
        assert_eq!(changed, Some(TeamPhase::Attack));
        assert_eq!(state.phase, TeamPhase::TransitionDefense);

        // 안정화 완료 (30틱 후)
        state.update(false, 50);
        assert_eq!(state.phase, TeamPhase::Defense);
    }

    #[test]
    fn test_quick_possession_change() {
        let mut state = TeamPhaseState::attacking();

        // 공 상실
        state.update(false, 10);
        assert_eq!(state.phase, TeamPhase::TransitionDefense);

        // 즉시 공 탈취 (5틱 후)
        state.update(true, 15);
        assert_eq!(state.phase, TeamPhase::TransitionAttack);
    }

    #[test]
    fn test_phase_is_attacking() {
        assert!(TeamPhase::Attack.is_attacking());
        assert!(TeamPhase::TransitionAttack.is_attacking());
        assert!(!TeamPhase::Defense.is_attacking());
        assert!(!TeamPhase::TransitionDefense.is_attacking());
    }

    #[test]
    fn test_phase_is_transition() {
        assert!(!TeamPhase::Attack.is_transition());
        assert!(TeamPhase::TransitionAttack.is_transition());
        assert!(!TeamPhase::Defense.is_transition());
        assert!(TeamPhase::TransitionDefense.is_transition());
    }
}
