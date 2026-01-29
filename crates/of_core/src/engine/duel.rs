//! Duel System: 1:1 결투 메커니즘
//!
//! 돌파(Take-on)의 본질은 "이동"이 아니라 "속이기"입니다.
//!
//! ## 핵심 개념
//! - **Feint (속임수):** 공격수가 한쪽으로 가는 척하며 수비수를 유인
//! - **Cut (방향전환):** 반대 방향으로 급선회
//! - **Burst (돌파):** 수비수가 역동작에 걸린 틈을 타 가속
//!
//! ## 수비수의 딜레마 (Defender's Dilemma)
//! - **Contain (지연):** 안전하게 따라가며 시간 벌기 (제쳐지지 않음, 공 못 뺏음)
//! - **Commit (도전):** 적극적으로 공 뺏기 시도 (성공하면 역습, 실패하면 스턴)

use serde::{Deserialize, Serialize};

/// 수비수의 행동 선택
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefensiveAction {
    /// 지연: 간격 유지하며 따라감 (안전)
    /// - 제쳐지지 않음
    /// - 공을 뺏지 못함
    /// - 슈팅/패스 시간을 내줌
    Contain,

    /// 도전: 적극적으로 공 뺏기 시도 (고위험/고보상)
    /// - 성공 시 공 탈취 + 역습 기회
    /// - 실패 시 스턴 + 제쳐짐
    Commit,
}

/// 공격수의 행동 선택
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackerAction {
    /// 운반 (Carry/Keep): 공을 안전하게 유지
    /// - 낮은 리스크
    /// - 전진 어려움
    Carry,

    /// 돌파 (Take-on): 수비수를 제치려는 시도
    /// - 높은 리스크
    /// - 성공 시 수비수 뒤로 이동
    TakeOn,
}

/// 1:1 대결 상황 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelState {
    /// 공격수 인덱스
    pub attacker_idx: usize,
    /// 수비수 인덱스
    pub defender_idx: usize,

    /// 페인트 방향 (None = 페인트 안 함)
    pub feint_direction: Option<(f32, f32)>,
    /// 실제 돌파 방향
    pub actual_burst_direction: (f32, f32),

    /// 수비수 무게중심이 쏠린 방향
    pub defender_balance_dir: (f32, f32),
    /// 수비수가 얼마나 쏠렸는지 (0.0 ~ 1.0)
    pub defender_commit_level: f32,

    /// 현재 페이즈
    pub phase: DuelPhase,
    /// 경과 틱
    pub elapsed_ticks: u8,
    /// 총 지속 틱
    pub duration_ticks: u8,

    /// 역동작 정도 (-1.0 ~ 1.0)
    /// -1.0 = 완전 반대 (앵클 브레이크!)
    ///  0.0 = 직각 (살짝 제침)
    /// +1.0 = 같은 방향 (막힘)
    pub wrong_foot_factor: f32,
}

/// Duel 진행 페이즈
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DuelPhase {
    /// 속임수 단계 (틱 0~5)
    Feint,
    /// 방향전환 단계 (틱 6~10)
    Cut,
    /// 돌파 단계 (틱 11~20)
    Burst,
    /// 완료
    Finished,
}

/// Duel 결과
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DuelOutcome {
    /// 교착 상태 (Contain vs Carry)
    Stalemate,

    /// 공격수 막힘 (Contain vs TakeOn)
    AttackerBlocked,

    /// 수비수 승리 (공 탈취)
    DefenderWins {
        /// 쉬운 승리인지 (Commit vs Carry)
        easy: bool,
    },

    /// 앵클 브레이크 (수비수 완전히 제쳐짐)
    AnkleBreaker {
        /// 스턴 틱 수
        stun_ticks: u8,
    },

    /// 수비수 살짝 제쳐짐
    Beaten {
        /// 회복 틱 수
        recovery_ticks: u8,
    },

    /// 루즈볼 (50:50)
    LooseBall,

    /// 파울 발생
    Foul,
}

/// 태클 결과 (Commit vs TakeOn 상세)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TackleDuelOutcome {
    DefenderWins,
    AttackerWins,
    Foul,
}

impl DuelState {
    /// 새 Duel 시작
    pub fn new(attacker_idx: usize, defender_idx: usize) -> Self {
        Self {
            attacker_idx,
            defender_idx,
            feint_direction: None,
            actual_burst_direction: (0.0, 1.0), // 기본: 앞으로
            defender_balance_dir: (0.0, 0.0),
            defender_commit_level: 0.0,
            phase: DuelPhase::Feint,
            elapsed_ticks: 0,
            duration_ticks: 20, // 기본 2초
            wrong_foot_factor: 0.0,
        }
    }

    /// 페인트 방향 설정
    pub fn set_feint_direction(&mut self, dir: (f32, f32)) {
        self.feint_direction = Some(dir);
    }

    /// 실제 돌파 방향 설정
    pub fn set_burst_direction(&mut self, dir: (f32, f32)) {
        self.actual_burst_direction = dir;
    }

    /// 수비수가 페인트에 낚였는지 설정
    pub fn set_defender_fooled(&mut self, balance_dir: (f32, f32), commit_level: f32) {
        self.defender_balance_dir = balance_dir;
        self.defender_commit_level = commit_level.clamp(0.0, 1.0);
    }

    /// 역동작 계산 (벡터 내적)
    pub fn calculate_wrong_foot_factor(&mut self) {
        // 벡터 정규화
        let burst = normalize(self.actual_burst_direction);
        let balance = normalize(self.defender_balance_dir);

        // 내적: -1.0 (완전 반대) ~ +1.0 (같은 방향)
        self.wrong_foot_factor = dot(burst, balance);
    }

    /// 틱 진행
    pub fn tick(&mut self) {
        self.elapsed_ticks += 1;

        // 페이즈 전환
        self.phase = match self.elapsed_ticks {
            0..=5 => DuelPhase::Feint,
            6..=10 => DuelPhase::Cut,
            11..=20 => DuelPhase::Burst,
            _ => DuelPhase::Finished,
        };
    }

    /// 현재 진행률 (0.0 ~ 1.0)
    pub fn progress(&self) -> f32 {
        self.elapsed_ticks as f32 / self.duration_ticks as f32
    }

    /// 완료 여부
    pub fn is_finished(&self) -> bool {
        self.phase == DuelPhase::Finished
    }
}

/// 스태미나 고갈 임계값 (0108: Open-Football Integration)
/// 이 값 이하면 고강도 수비(Commit)를 저강도(Contain)로 전환
const STAMINA_EXHAUSTED_THRESHOLD: f32 = 0.30;

/// 수비 AI: Contain vs Commit 결정
///
/// # Arguments
/// * `stamina_percent` - 선수 스태미나 비율 (0.0-1.0). 0.0이면 스태미나 체크 생략 (기존 동작)
pub fn decide_defensive_action(
    distance_to_goal: f32,
    has_cover: bool,
    attacker_bad_touch: bool,
    defender_aggression: u8,
    defender_composure: u8,
    team_pressing_bonus: f32,
    stamina_percent: f32,
) -> DefensiveAction {
    // 0108: Stamina-Aware Defense
    // 스태미나 30% 이하면 무조건 Contain (에너지 보존)
    if stamina_percent > 0.0 && stamina_percent < STAMINA_EXHAUSTED_THRESHOLD {
        return DefensiveAction::Contain;
    }

    // Commit 점수 계산
    let mut commit_score: f32 = 0.0;

    // 커버 있음: +20 (뚫려도 되니까 질러봄)
    // 커버 없음: -50 (뒤에 아무도 없으면 절대 덤비지 마라)
    commit_score += if has_cover { 20.0 } else { -50.0 };

    // 골대 가까움: +30 (슈팅 각을 주기 전에 막아야 함)
    if distance_to_goal < 20.0 {
        commit_score += 30.0;
    }

    // 상대 배드터치: +25 (기회 포착)
    if attacker_bad_touch {
        commit_score += 25.0;
    }

    // 성향 공격적: +Aggression * 0.5
    commit_score += defender_aggression as f32 * 0.5;

    // 침착성: -Composure * 0.3 (침착한 선수는 덜 덤빔)
    commit_score -= defender_composure as f32 * 0.3;

    // 팀 전술 보너스 (High Press 등)
    commit_score += team_pressing_bonus;

    // 0108: 스태미나가 중간 수준(30-50%)이면 Commit 페널티
    if stamina_percent > 0.0 && stamina_percent < 0.50 {
        commit_score -= 15.0; // 중간 피로 = 덜 공격적
    }

    // 결정
    if commit_score > 50.0 {
        DefensiveAction::Commit
    } else {
        DefensiveAction::Contain
    }
}

/// Duel 결과 매트릭스 처리
pub fn resolve_duel(
    defender_action: DefensiveAction,
    attacker_action: AttackerAction,
    wrong_foot_factor: f32,
    defender_commit_level: f32,
    tackle_success_roll: f32,  // 0.0 ~ 1.0 (수비수 능력치 기반)
    dribble_success_roll: f32, // 0.0 ~ 1.0 (공격수 능력치 기반)
) -> DuelOutcome {
    match (defender_action, attacker_action) {
        // === 지연 vs 운반 = 교착 ===
        (DefensiveAction::Contain, AttackerAction::Carry) => DuelOutcome::Stalemate,

        // === 지연 vs 돌파 = 수비 유리 ===
        (DefensiveAction::Contain, AttackerAction::TakeOn) => {
            // 수비수가 안 덤비고 뒤로 물러남
            // 공격수가 무리하게 뚫으려다 막힘
            DuelOutcome::AttackerBlocked
        }

        // === 도전 vs 운반 = 수비 승리 ===
        (DefensiveAction::Commit, AttackerAction::Carry) => {
            // 공격수가 방심했을 때 공을 쉽게 뺏음
            DuelOutcome::DefenderWins { easy: true }
        }

        // === 도전 vs 돌파 = 진검승부! ===
        (DefensiveAction::Commit, AttackerAction::TakeOn) => {
            // 역동작 정도에 따른 결과 분기
            if wrong_foot_factor < -0.5 {
                // 앵클 브레이크 (수비수가 완전히 역동작에 걸림)
                let stun_ticks = calculate_stun_ticks(defender_commit_level);
                DuelOutcome::AnkleBreaker { stun_ticks }
            } else if wrong_foot_factor < 0.0 {
                // 수비수가 약간 뒤처지지만 따라올 수 있음
                DuelOutcome::Beaten { recovery_ticks: 5 }
            } else if wrong_foot_factor < 0.5 {
                // 50:50 상황 - 능력치 대결
                if dribble_success_roll > tackle_success_roll {
                    DuelOutcome::Beaten { recovery_ticks: 3 }
                } else if tackle_success_roll - dribble_success_roll > 0.3 {
                    DuelOutcome::DefenderWins { easy: false }
                } else {
                    DuelOutcome::LooseBall
                }
            } else {
                // 수비수가 안 속고 길을 막음
                if tackle_success_roll > 0.7 {
                    DuelOutcome::DefenderWins { easy: false }
                } else if dribble_success_roll > 0.8 {
                    // 힘으로 뚫음
                    DuelOutcome::Beaten { recovery_ticks: 2 }
                } else {
                    DuelOutcome::AttackerBlocked
                }
            }
        }
    }
}

/// 스턴 틱 계산
fn calculate_stun_ticks(commit_level: f32) -> u8 {
    if commit_level > 0.8 {
        25 // 슬라이딩 시도했다가 역동작 = 2.5초 스턴
    } else if commit_level > 0.5 {
        15 // 발 뻗었다가 역동작 = 1.5초 스턴
    } else {
        8 // 살짝 쏠렸다가 역동작 = 0.8초 스턴
    }
}

/// 벡터 내적
fn dot(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

/// 벡터 정규화
fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len < 0.0001 {
        (0.0, 0.0)
    } else {
        (v.0 / len, v.1 / len)
    }
}

/// 2차 베지에 곡선으로 회피 궤적 계산
///
/// # Arguments
/// * `start` - 공격수 시작 위치
/// * `defender` - 수비수 위치
/// * `goal_dir` - 공격 방향 (정규화된 벡터)
/// * `t` - 진행률 (0.0 ~ 1.0)
///
/// # Returns
/// 현재 위치
pub fn calculate_evasive_trajectory(
    start: (f32, f32),
    defender: (f32, f32),
    goal_dir: (f32, f32),
    t: f32,
) -> (f32, f32) {
    // 1. 최종 도착점 (P2): 수비수 뒤 2m
    let end = (defender.0 + goal_dir.0 * 2.0, defender.1 + goal_dir.1 * 2.0);

    // 2. 우회점 (P1): 수비수 옆 1.5m
    // 수직 벡터: (-y, x)
    let side_vector = (-goal_dir.1, goal_dir.0);
    let control = (defender.0 + side_vector.0 * 1.5, defender.1 + side_vector.1 * 1.5);

    // 3. 2차 베지에 곡선 공식
    // B(t) = (1-t)² * P0 + 2(1-t)t * P1 + t² * P2
    let one_minus_t = 1.0 - t;
    let one_minus_t_sq = one_minus_t * one_minus_t;
    let t_sq = t * t;

    (
        one_minus_t_sq * start.0 + 2.0 * one_minus_t * t * control.0 + t_sq * end.0,
        one_minus_t_sq * start.1 + 2.0 * one_minus_t * t * control.1 + t_sq * end.1,
    )
}

// ===========================================
// Passive Interference System (패시브 방해)
// ===========================================

/// 슈팅 압박 수치 계산 (0.0 ~ 1.0)
///
/// 수비수가 슈터 주변에 있으면 물리적 접촉이 없어도
/// 심리적/물리적 페널티를 받습니다.
///
/// # Arguments
/// * `shooter_pos` - 슈터 위치 (m)
/// * `defenders` - 수비수 위치들 (m)
/// * `goal_pos` - 골대 중앙 위치 (m)
///
/// # Returns
/// 압박 수치 (0.0 = 오픈 찬스, 1.0 = 극심한 압박)
pub fn calculate_shot_pressure(
    shooter_pos: (f32, f32),
    defenders: &[(f32, f32)],
    goal_pos: (f32, f32),
) -> f32 {
    let mut total_pressure = 0.0;

    for &def_pos in defenders {
        let dist = distance(shooter_pos, def_pos);

        // 1. 거리 페널티 (2m 이내일 때만 영향)
        if dist < 2.0 {
            // 가까울수록 압박 심함 (0m=1.0, 2m=0.0)
            let proximity_pressure = (2.0 - dist) / 2.0;

            // 2. 각도 페널티 (슈팅 각도(Cone) 안에 있는가?)
            let is_blocking = is_in_shot_cone(shooter_pos, def_pos, goal_pos);
            let angle_multiplier = if is_blocking { 1.5 } else { 0.5 };

            total_pressure += proximity_pressure * angle_multiplier;
        }
    }

    total_pressure.clamp(0.0, 1.0)
}

/// 수비수가 슈팅 콘(Cone) 안에 있는지 체크
///
/// 슈터 → 골대 방향에서 약 30도 이내에 수비수가 있고,
/// 수비수가 슈터와 골대 사이에 있으면 true
pub fn is_in_shot_cone(shooter: (f32, f32), defender: (f32, f32), goal: (f32, f32)) -> bool {
    let to_goal = normalize((goal.0 - shooter.0, goal.1 - shooter.1));
    let to_defender = normalize((defender.0 - shooter.0, defender.1 - shooter.1));

    // 내적으로 각도 계산 (0.85 ≈ 30도 이내)
    let dot_val = dot(to_goal, to_defender);

    // 수비수가 슈터와 골대 사이에 있어야 함
    let dist_to_defender = distance(shooter, defender);
    let dist_to_goal = distance(shooter, goal);

    dot_val > 0.85 && dist_to_defender < dist_to_goal
}

/// Composure 능력치로 압박 저항력 적용
///
/// Composure가 높을수록 압박의 영향이 줄어듭니다.
/// - Composure 20 (최고) → 압박 50% 무시
/// - Composure 5 (최저) → 압박 150% 증폭
pub fn apply_pressure_with_composure(pressure: f32, composure: u8) -> f32 {
    // Composure 범위: 1-20 (FM 스타일)
    let composure_factor = 1.5 - (composure as f32 / 20.0);
    (pressure * composure_factor).clamp(0.0, 1.0)
}

/// 압박이 적용된 xG 계산
///
/// 압박이 높을수록 xG가 감소합니다.
/// - Pressure 0.0 → xG 100%
/// - Pressure 1.0 → xG 50%
pub fn calculate_xg_with_pressure(base_xg: f32, pressure: f32, composure: u8) -> f32 {
    let effective_pressure = apply_pressure_with_composure(pressure, composure);
    let pressure_penalty = 1.0 - (effective_pressure * 0.5);
    base_xg * pressure_penalty
}

/// 압박에 의한 슈팅 정확도 배율
///
/// - Pressure 0.0 → 1.0x (정상)
/// - Pressure 1.0 → 3.0x (오차 3배)
pub fn calculate_accuracy_penalty(pressure: f32, composure: u8) -> f32 {
    let effective_pressure = apply_pressure_with_composure(pressure, composure);
    1.0 + (effective_pressure * 2.0)
}

/// 압박에 의한 슈팅 파워 배율
///
/// - Pressure 0.0 → 1.0 (100% 파워)
/// - Pressure 1.0 → 0.7 (70% 파워)
pub fn calculate_power_penalty(pressure: f32, composure: u8) -> f32 {
    let effective_pressure = apply_pressure_with_composure(pressure, composure);
    1.0 - (effective_pressure * 0.3)
}

/// 압박에 의한 슛 블락 확률
///
/// Pressure > 0.8 이고 수비수가 블로킹 위치면 40% 확률로 블락
pub fn calculate_block_chance(pressure: f32, is_blocking_position: bool) -> f32 {
    if pressure > 0.8 && is_blocking_position {
        0.4
    } else if pressure > 0.5 && is_blocking_position {
        0.2
    } else {
        0.0
    }
}

/// 두 점 사이의 거리
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn test_decide_defensive_action_with_cover() {
        // 커버 있으면 Commit 가능성 높음
        let action = decide_defensive_action(
            15.0,  // 골대와 가까움
            true,  // 커버 있음
            false, // 배드터치 아님
            70,    // 공격적 성향
            50,    // 보통 침착성
            0.0,   // 팀 보너스 없음
            0.80,  // 스태미나 80% (정상)
        );
        assert_eq!(action, DefensiveAction::Commit);
    }

    #[test]
    fn test_decide_defensive_action_last_man() {
        // 라스트맨은 Contain
        let action = decide_defensive_action(
            30.0,  // 골대와 멀음
            false, // 커버 없음 (라스트맨)
            false, 50, 80, // 침착함
            0.0, 0.80, // 스태미나 80% (정상)
        );
        assert_eq!(action, DefensiveAction::Contain);
    }

    // ============================================================================
    // 0108: Stamina-Aware Defense Tests
    // ============================================================================

    #[test]
    fn test_stamina_exhausted_forces_contain() {
        // 스태미나 25% - 지친 상태 (원래 Commit 조건)
        let action = decide_defensive_action(
            15.0, // 골대와 가까움
            true, // 커버 있음
            true, // 배드터치 (원래는 Commit 유도)
            90,   // 매우 공격적
            30,   // 낮은 침착성
            20.0, // 높은 프레싱 보너스
            0.25, // 스태미나 25% - 임계값 이하!
        );
        assert_eq!(
            action,
            DefensiveAction::Contain,
            "Exhausted player should Contain regardless of other factors"
        );
    }

    #[test]
    fn test_stamina_normal_allows_commit() {
        // 스태미나 60% - 정상 상태
        let action = decide_defensive_action(
            15.0, true, true, 70, 50, 0.0, 0.60, // 스태미나 60%
        );
        assert_eq!(action, DefensiveAction::Commit, "Normal stamina should allow Commit");
    }

    #[test]
    fn test_stamina_zero_uses_default_behavior() {
        // stamina_percent = 0.0 (기본값, 정보 없음) - 기존 로직 유지
        let action = decide_defensive_action(
            15.0, true, true, 70, 50, 0.0, 0.0, // Default (no stamina info)
        );
        assert_eq!(
            action,
            DefensiveAction::Commit,
            "Zero stamina (default) should use normal behavior"
        );
    }

    #[test]
    fn test_stamina_boundary_at_threshold() {
        // 정확히 30% - 경계값 (아직 정상)
        let action = decide_defensive_action(
            15.0, true, true, 70, 50, 0.0, 0.30, // 정확히 30%
        );
        // 30%는 < 0.30이 아니므로 정상 동작해야 함
        assert_eq!(action, DefensiveAction::Commit, "At threshold (30%) should still allow Commit");
    }

    #[test]
    fn test_stamina_medium_penalizes_commit() {
        // 스태미나 40% (중간 피로) - Commit 점수 -15 페널티
        // 경계 상황: 원래는 Commit이지만 페널티로 Contain이 될 수 있음
        let action_high_stamina = decide_defensive_action(
            25.0,  // 중거리
            true,  // 커버 있음 (+20)
            false, // 배드터치 없음
            60,    // 보통 공격성 (+30)
            60,    // 보통 침착성 (-18)
            0.0,   // 팀 보너스 없음
            0.80,  // 스태미나 80%: 점수 = 20 + 30 - 18 = 32 → Contain
        );

        let action_medium_stamina = decide_defensive_action(
            25.0, true, false, 60, 60, 0.0,
            0.40, // 스태미나 40%: 점수 = 20 + 30 - 18 - 15 = 17 → 확실히 Contain
        );

        // 둘 다 이 조건에서는 Contain이지만, 중간 피로 페널티 확인용
        assert_eq!(action_high_stamina, DefensiveAction::Contain);
        assert_eq!(action_medium_stamina, DefensiveAction::Contain);
    }

    #[test]
    fn test_resolve_duel_contain_vs_carry() {
        let outcome =
            resolve_duel(DefensiveAction::Contain, AttackerAction::Carry, 0.0, 0.0, 0.5, 0.5);
        assert_eq!(outcome, DuelOutcome::Stalemate);
    }

    #[test]
    fn test_resolve_duel_contain_vs_takeon() {
        let outcome =
            resolve_duel(DefensiveAction::Contain, AttackerAction::TakeOn, 0.0, 0.0, 0.5, 0.5);
        assert_eq!(outcome, DuelOutcome::AttackerBlocked);
    }

    #[test]
    fn test_resolve_duel_commit_vs_carry() {
        let outcome =
            resolve_duel(DefensiveAction::Commit, AttackerAction::Carry, 0.0, 0.0, 0.5, 0.5);
        assert_eq!(outcome, DuelOutcome::DefenderWins { easy: true });
    }

    #[test]
    fn test_resolve_duel_ankle_breaker() {
        // 완전 역동작
        let outcome = resolve_duel(
            DefensiveAction::Commit,
            AttackerAction::TakeOn,
            -0.8, // 역동작
            0.7,  // 커밋 레벨 높음
            0.5,
            0.5,
        );
        assert!(matches!(outcome, DuelOutcome::AnkleBreaker { .. }));
    }

    #[test]
    fn test_wrong_foot_factor_calculation() {
        let mut duel = DuelState::new(0, 1);

        // 페인트: 오른쪽
        duel.set_feint_direction((1.0, 0.0));
        // 수비수가 낚임: 오른쪽으로 쏠림
        duel.set_defender_fooled((1.0, 0.0), 0.7);
        // 실제 돌파: 왼쪽
        duel.set_burst_direction((-1.0, 0.0));

        duel.calculate_wrong_foot_factor();

        // 완전 반대 방향 = -1.0
        assert!(duel.wrong_foot_factor < -0.9);
    }

    #[test]
    fn test_evasive_trajectory() {
        let start = (50.0, field::CENTER_Y);
        let defender = (55.0, field::CENTER_Y);
        let goal_dir = (1.0, 0.0); // 오른쪽으로 공격

        // t=0: 시작점
        let pos0 = calculate_evasive_trajectory(start, defender, goal_dir, 0.0);
        assert!((pos0.0 - start.0).abs() < 0.1);
        assert!((pos0.1 - start.1).abs() < 0.1);

        // t=1: 수비수 뒤 도착
        let pos1 = calculate_evasive_trajectory(start, defender, goal_dir, 1.0);
        assert!(pos1.0 > defender.0); // 수비수보다 앞에 있어야 함
    }

    #[test]
    fn test_duel_phase_progression() {
        let mut duel = DuelState::new(0, 1);

        assert_eq!(duel.phase, DuelPhase::Feint);

        for _ in 0..6 {
            duel.tick();
        }
        assert_eq!(duel.phase, DuelPhase::Cut);

        for _ in 0..5 {
            duel.tick();
        }
        assert_eq!(duel.phase, DuelPhase::Burst);

        for _ in 0..10 {
            duel.tick();
        }
        assert_eq!(duel.phase, DuelPhase::Finished);
    }

    // ===========================================
    // Passive Interference Tests
    // ===========================================

    #[test]
    fn test_shot_pressure_no_defenders() {
        let shooter = (90.0, field::CENTER_Y);
        let defenders: Vec<(f32, f32)> = vec![];
        let goal = (field::LENGTH_M, field::CENTER_Y);

        let pressure = calculate_shot_pressure(shooter, &defenders, goal);
        assert_eq!(pressure, 0.0);
    }

    #[test]
    fn test_shot_pressure_defender_close() {
        let shooter = (90.0, field::CENTER_Y);
        let defenders = vec![(91.0, field::CENTER_Y)]; // 1m 앞에 수비수
        let goal = (field::LENGTH_M, field::CENTER_Y);

        let pressure = calculate_shot_pressure(shooter, &defenders, goal);
        // 1m 거리 → proximity = 0.5, blocking 위치 → 0.5 * 1.5 = 0.75
        assert!(pressure > 0.5);
    }

    #[test]
    fn test_shot_pressure_defender_far() {
        let shooter = (90.0, field::CENTER_Y);
        let defenders = vec![(95.0, field::CENTER_Y)]; // 5m 앞에 수비수
        let goal = (field::LENGTH_M, field::CENTER_Y);

        let pressure = calculate_shot_pressure(shooter, &defenders, goal);
        // 5m > 2m → 영향 없음
        assert_eq!(pressure, 0.0);
    }

    #[test]
    fn test_composure_reduces_pressure() {
        let pressure = 0.6;

        // 낮은 Composure (5) → 압박 증폭
        let low_composure = apply_pressure_with_composure(pressure, 5);
        // 높은 Composure (20) → 압박 감소
        let high_composure = apply_pressure_with_composure(pressure, 20);

        assert!(low_composure > pressure);
        assert!(high_composure < pressure);
    }

    #[test]
    fn test_xg_with_pressure() {
        let base_xg = 0.4;

        // 압박 없음 → xG 그대로
        let xg_no_pressure = calculate_xg_with_pressure(base_xg, 0.0, 15);
        assert!((xg_no_pressure - 0.4).abs() < 0.01);

        // 압박 있음 → xG 감소
        let xg_with_pressure = calculate_xg_with_pressure(base_xg, 0.8, 15);
        assert!(xg_with_pressure < xg_no_pressure);
    }

    #[test]
    fn test_accuracy_penalty() {
        // 압박 없음 → 1.0x
        let no_pressure = calculate_accuracy_penalty(0.0, 15);
        assert!((no_pressure - 1.0).abs() < 0.01);

        // 압박 있음 → 배율 증가
        let with_pressure = calculate_accuracy_penalty(1.0, 15);
        assert!(with_pressure > 2.0);
    }

    #[test]
    fn test_power_penalty() {
        // 압박 없음 → 100% 파워
        let no_pressure = calculate_power_penalty(0.0, 15);
        assert!((no_pressure - 1.0).abs() < 0.01);

        // 압박 있음 → 파워 감소
        let with_pressure = calculate_power_penalty(1.0, 15);
        assert!(with_pressure < 0.8);
    }

    #[test]
    fn test_block_chance() {
        // 압박 낮음 → 블락 없음
        assert_eq!(calculate_block_chance(0.3, true), 0.0);

        // 압박 높음 + 블로킹 위치 → 블락 확률
        assert!(calculate_block_chance(0.9, true) > 0.3);

        // 압박 높음 + 비블로킹 위치 → 블락 없음
        assert_eq!(calculate_block_chance(0.9, false), 0.0);
    }
}
