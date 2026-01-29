//! FIX_2601/1123: Detail Builder
//!
//! 결정 단계에서 완전한 ActionDetailV2를 생성한다.
//! 모든 선택은 deterministic 함수를 사용하며 RNG를 사용하지 않는다.
//!
//! ## 설계 원칙
//!
//! 1. **RNG 완전 배제**: 모든 선택은 `deterministic_choice` / `deterministic_f32` 사용
//! 2. **정렬 필수**: valid_targets 같은 리스트는 반드시 정렬 후 선택
//! 3. **Self-reference 금지**: owner == target 불가
//! 4. **완전성 보장**: 반환되는 Detail은 항상 모든 필수 필드가 채워져 있음
//!
//! ## 사용 흐름
//!
//! ```text
//! DecisionContext 구성
//!     → Intent 생성 (Option 포함 가능)
//!     → build_*_detail() 호출
//!     → ActionDetailV2 반환 (Option 없음)
//! ```

use super::action_detail_v2::*;
use super::candidate_key::CandidateKey;
use super::deterministic::{
    clamp_f32, deterministic_choice, deterministic_f32, normalize_direction, subcase,
};
use super::tactical_bias::TacticalBias;
use crate::tactics::team_instructions::{TacticalPreset, TeamInstructions};

// ============================================================================
// Decision Context
// ============================================================================

/// 결정 컨텍스트 - Builder에 필요한 모든 정보
///
/// 이 구조체는 결정 시점의 스냅샷 정보를 담는다.
/// Builder 함수들은 이 컨텍스트와 Intent를 받아 완전한 Detail을 생성한다.
///
/// ## FIX_2601/1124 확장
///
/// - `tactical_bias`: 전술 프리셋에서 변환된 행동 bias
/// - `selected_key`: UAE 선택 단계에서 선택된 CandidateKey (Gate A 검증용)
#[derive(Debug, Clone)]
pub struct DecisionContext {
    /// 매치 시드 (결정론의 근원)
    pub seed: u64,
    /// 현재 틱 (시간)
    pub tick: u64,
    /// 행위자 인덱스 (track_id, 0..21)
    pub owner_idx: usize,
    /// 행위자 위치 (정규화 좌표)
    pub owner_pos: (f32, f32),
    /// 볼 위치 (정규화 좌표)
    pub ball_pos: (f32, f32),
    /// Home 팀 여부
    pub is_home: bool,
    /// 팀 동료 목록: (track_id, position) - **정렬되어 있어야 함**
    pub teammates: Vec<(usize, (f32, f32))>,
    /// 상대 팀 목록: (track_id, position) - **정렬되어 있어야 함**
    pub opponents: Vec<(usize, (f32, f32))>,
    /// 골 방향: +1.0 (오른쪽 골문) 또는 -1.0 (왼쪽 골문)
    pub goal_direction: f32,
    /// 오프사이드 라인 x 좌표 (정규화)
    pub offside_line: f32,
    /// FIX_2601/1124: 전술 편향 (TacticalPreset/TeamInstructions에서 변환)
    pub tactical_bias: TacticalBias,
    /// FIX_2601/1124: 선택된 후보 키 (Gate A 검증용)
    pub selected_key: Option<CandidateKey>,
}

impl DecisionContext {
    /// 해당 위치가 오프사이드인지 확인
    #[inline]
    pub fn is_offside(&self, x: f32) -> bool {
        if self.goal_direction > 0.0 {
            // 오른쪽 골문 공격 → x가 오프사이드 라인보다 크면 오프사이드
            x > self.offside_line
        } else {
            // 왼쪽 골문 공격 → x가 오프사이드 라인보다 작으면 오프사이드
            x < self.offside_line
        }
    }

    /// 유효한 패스 타겟 목록 반환
    ///
    /// - owner 제외
    /// - 오프사이드 위치 제외
    /// - track_id 기준 정렬됨
    pub fn valid_pass_targets(&self) -> Vec<(usize, (f32, f32))> {
        let mut targets: Vec<_> = self
            .teammates
            .iter()
            .filter(|(id, _)| *id != self.owner_idx)
            .filter(|(_, pos)| !self.is_offside(pos.0))
            .cloned()
            .collect();
        targets.sort_by_key(|(id, _)| *id);
        targets
    }

    /// 모든 패스 타겟 목록 반환 (오프사이드 무시)
    ///
    /// - owner 제외
    /// - track_id 기준 정렬됨
    pub fn all_pass_targets(&self) -> Vec<(usize, (f32, f32))> {
        let mut targets: Vec<_> = self
            .teammates
            .iter()
            .filter(|(id, _)| *id != self.owner_idx)
            .cloned()
            .collect();
        targets.sort_by_key(|(id, _)| *id);
        targets
    }

    /// 골문 중앙 위치 반환 (정규화 좌표)
    #[inline]
    pub fn goal_center(&self) -> (f32, f32) {
        if self.goal_direction > 0.0 {
            (1.0, 0.5)
        } else {
            (0.0, 0.5)
        }
    }

    // ========================================================================
    // FIX_2601/1124: TacticalBias 기반 생성자
    // ========================================================================

    /// TacticalPreset으로 DecisionContext 생성
    ///
    /// 기존 필드는 인자로 받고, tactical_bias는 프리셋에서 자동 변환
    pub fn with_preset(
        seed: u64,
        tick: u64,
        owner_idx: usize,
        owner_pos: (f32, f32),
        ball_pos: (f32, f32),
        is_home: bool,
        teammates: Vec<(usize, (f32, f32))>,
        opponents: Vec<(usize, (f32, f32))>,
        goal_direction: f32,
        offside_line: f32,
        preset: TacticalPreset,
    ) -> Self {
        Self {
            seed,
            tick,
            owner_idx,
            owner_pos,
            ball_pos,
            is_home,
            teammates,
            opponents,
            goal_direction,
            offside_line,
            tactical_bias: TacticalBias::from_preset(preset),
            selected_key: None,
        }
    }

    /// TeamInstructions로 DecisionContext 생성
    pub fn with_instructions(
        seed: u64,
        tick: u64,
        owner_idx: usize,
        owner_pos: (f32, f32),
        ball_pos: (f32, f32),
        is_home: bool,
        teammates: Vec<(usize, (f32, f32))>,
        opponents: Vec<(usize, (f32, f32))>,
        goal_direction: f32,
        offside_line: f32,
        instructions: &TeamInstructions,
    ) -> Self {
        Self {
            seed,
            tick,
            owner_idx,
            owner_pos,
            ball_pos,
            is_home,
            teammates,
            opponents,
            goal_direction,
            offside_line,
            tactical_bias: TacticalBias::from_instructions(instructions),
            selected_key: None,
        }
    }

    /// 선택된 키 설정
    pub fn with_selected_key(mut self, key: CandidateKey) -> Self {
        self.selected_key = Some(key);
        self
    }

    /// tactical_bias 참조 반환
    pub fn bias(&self) -> &TacticalBias {
        &self.tactical_bias
    }

    // ========================================================================
    // FIX_2601/1124 Phase 2: P16 브릿지 함수
    // ========================================================================

    /// P16 파이프라인에서 호출하기 위한 브릿지 생성자
    ///
    /// P16 DecisionContext(48+ 필드)는 복잡하므로, 필요한 정보만
    /// 외부에서 추출하여 전달받는다.
    ///
    /// ## 사용 예시
    ///
    /// ```ignore
    /// // ev_decision.rs에서:
    /// let builder_ctx = DecisionContext::from_p16_bridge(
    ///     self.seed,
    ///     self.current_tick,
    ///     player_idx,
    ///     player_pos,
    ///     ball_pos,
    ///     is_home,
    ///     teammates,
    ///     opponents,
    ///     goal_direction,
    ///     offside_line,
    ///     &instructions,
    /// );
    /// ```
    ///
    /// ## 매개변수
    ///
    /// - `seed`: 매치 시드 (결정론의 근원)
    /// - `tick`: 현재 틱
    /// - `owner_idx`: 행위자 track_id (0..21)
    /// - `owner_pos`: 행위자 위치 (정규화 좌표 0..1)
    /// - `ball_pos`: 볼 위치 (정규화 좌표 0..1)
    /// - `is_home`: Home 팀 여부
    /// - `teammates`: 팀 동료 (track_id, pos) 목록
    /// - `opponents`: 상대 팀 (track_id, pos) 목록
    /// - `goal_direction`: 골 방향 (+1.0 또는 -1.0)
    /// - `offside_line`: 오프사이드 라인 x 좌표
    /// - `instructions`: 팀 전술 지시
    #[allow(clippy::too_many_arguments)]
    pub fn from_p16_bridge(
        seed: u64,
        tick: u64,
        owner_idx: usize,
        owner_pos: (f32, f32),
        ball_pos: (f32, f32),
        is_home: bool,
        teammates: Vec<(usize, (f32, f32))>,
        opponents: Vec<(usize, (f32, f32))>,
        goal_direction: f32,
        offside_line: f32,
        instructions: &TeamInstructions,
    ) -> Self {
        Self {
            seed,
            tick,
            owner_idx,
            owner_pos,
            ball_pos,
            is_home,
            teammates,
            opponents,
            goal_direction,
            offside_line,
            tactical_bias: TacticalBias::from_instructions(instructions),
            selected_key: None,
        }
    }

    /// P16 브릿지 + 기본 전술 (TeamInstructions 없을 때)
    ///
    /// TeamInstructions가 없는 경우 Balanced 프리셋을 사용한다.
    #[allow(clippy::too_many_arguments)]
    pub fn from_p16_bridge_default(
        seed: u64,
        tick: u64,
        owner_idx: usize,
        owner_pos: (f32, f32),
        ball_pos: (f32, f32),
        is_home: bool,
        teammates: Vec<(usize, (f32, f32))>,
        opponents: Vec<(usize, (f32, f32))>,
        goal_direction: f32,
        offside_line: f32,
    ) -> Self {
        Self {
            seed,
            tick,
            owner_idx,
            owner_pos,
            ball_pos,
            is_home,
            teammates,
            opponents,
            goal_direction,
            offside_line,
            tactical_bias: TacticalBias::default(), // Balanced 프리셋
            selected_key: None,
        }
    }
}

impl Default for DecisionContext {
    /// 기본 DecisionContext 생성 (테스트용)
    fn default() -> Self {
        Self {
            seed: 0,
            tick: 0,
            owner_idx: 0,
            owner_pos: (0.5, 0.5),
            ball_pos: (0.5, 0.5),
            is_home: true,
            teammates: Vec::new(),
            opponents: Vec::new(),
            goal_direction: 1.0,
            offside_line: 1.0,
            tactical_bias: TacticalBias::default(),
            selected_key: None,
        }
    }
}

// ============================================================================
// Pass Builder
// ============================================================================

/// Pass Detail 생성
///
/// Intent의 intended_target이 유효하면 사용하고,
/// 그렇지 않으면 valid_pass_targets에서 deterministic하게 선택한다.
pub fn build_pass_detail(ctx: &DecisionContext, intent: &PassIntent) -> PassDetail {
    let valid_targets = ctx.valid_pass_targets();

    // 타겟 선택
    let target_track_id = if let Some(intended) = intent.intended_target {
        // Intent에 타겟이 있고 유효하면 사용
        if valid_targets.iter().any(|(id, _)| *id == intended) {
            intended as u8
        } else {
            // Intent 타겟이 무효(오프사이드 등)하면 deterministic 재선정
            select_pass_target(ctx, &valid_targets)
        }
    } else {
        // Intent에 타겟 없으면 deterministic 선택
        select_pass_target(ctx, &valid_targets)
    };

    // 파워 결정 (FIX_2601/1124: tactical_bias 사용)
    let power = intent.power.unwrap_or_else(|| {
        let (min_power, max_power) = ctx.tactical_bias.get_pass_power_bounds();
        let base_power = deterministic_f32(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::PASS_POWER,
            min_power,
            max_power,
        );
        clamp_f32(base_power, 0.1, 1.0)
    });

    PassDetail {
        target_track_id,
        pass_kind: intent.pass_kind,
        power,
        intended_point: intent.intended_point,
        // FIX_2601/1129: passer position은 build_action_detail_v2에서 설정됨
        // 여기서는 DecisionContext에 owner_pos가 정규화 좌표이므로 변환이 필요
        // 하지만 이 함수는 테스트 및 레거시용이므로 None으로 설정
        intended_passer_pos: None,
    }
}

/// 패스 타겟 선택 (내부 헬퍼)
fn select_pass_target(ctx: &DecisionContext, valid_targets: &[(usize, (f32, f32))]) -> u8 {
    if valid_targets.is_empty() {
        // 극단적 예외: 유효 타겟 없음 - 오프사이드 무시하고 아무나 선택
        let all_targets = ctx.all_pass_targets();
        if all_targets.is_empty() {
            // 정말 아무도 없음 - 팀 슬롯에서 선택
            let fallback_idx = if ctx.is_home {
                let candidates: Vec<usize> = (0..11).filter(|&i| i != ctx.owner_idx).collect();
                if candidates.is_empty() {
                    0 // 마지막 폴백
                } else {
                    let idx = deterministic_choice(
                        ctx.seed,
                        ctx.tick,
                        ctx.owner_idx,
                        subcase::PASS_TARGET,
                        candidates.len(),
                    );
                    candidates[idx]
                }
            } else {
                let candidates: Vec<usize> = (11..22).filter(|&i| i != ctx.owner_idx).collect();
                if candidates.is_empty() {
                    11 // 마지막 폴백
                } else {
                    let idx = deterministic_choice(
                        ctx.seed,
                        ctx.tick,
                        ctx.owner_idx,
                        subcase::PASS_TARGET,
                        candidates.len(),
                    );
                    candidates[idx]
                }
            };
            return fallback_idx as u8;
        }

        let idx = deterministic_choice(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::PASS_OFFSIDE_REDIRECT,
            all_targets.len(),
        );
        return all_targets[idx].0 as u8;
    }

    let idx = deterministic_choice(
        ctx.seed,
        ctx.tick,
        ctx.owner_idx,
        subcase::PASS_TARGET,
        valid_targets.len(),
    );
    valid_targets[idx].0 as u8
}

// ============================================================================
// Shot Builder
// ============================================================================

/// Shot Detail 생성
///
/// 골문 범위 내에서 목표 지점과 파워를 결정한다.
/// FIX_2601/1124: tactical_bias의 shot_target_y_range, shot_power_range 사용
pub fn build_shot_detail(ctx: &DecisionContext, intent: &ShotIntent) -> ShotDetail {
    // 목표 지점 결정 (FIX_2601/1124: tactical_bias 사용)
    let target_point = intent.target_point.unwrap_or_else(|| {
        let goal_x = if ctx.goal_direction > 0.0 { 1.0 } else { 0.0 };
        // 골문 y 범위: tactical_bias.shot_target_y_range 사용
        let (min_y, max_y) = ctx.tactical_bias.shot_target_y_range;
        let goal_y = deterministic_f32(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::SHOT_TARGET_Y,
            min_y,
            max_y,
        );
        (goal_x, goal_y)
    });

    // 파워 결정 (FIX_2601/1124: tactical_bias 사용)
    let power = intent.power.unwrap_or_else(|| {
        let (min_power, max_power) = ctx.tactical_bias.get_shot_power_bounds();
        deterministic_f32(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::SHOT_POWER,
            min_power,
            max_power,
        )
    });

    ShotDetail {
        target_point,
        power,
        shot_kind: intent.shot_kind.unwrap_or(ShotKind::Normal),
    }
}

// ============================================================================
// Dribble Builder
// ============================================================================

/// Dribble Detail 생성
///
/// 골 방향을 기본으로 하고 약간의 y 편차를 더한다.
/// FIX_2601/1124: tactical_bias의 dribble_speed_range 사용
pub fn build_dribble_detail(ctx: &DecisionContext, intent: &DribbleIntent) -> DribbleDetail {
    // 방향 결정
    let direction = intent.direction.unwrap_or_else(|| {
        // 기본: 골 방향
        let base_x = ctx.goal_direction;
        // Y 편차: -0.3 ~ 0.3
        let y_offset = deterministic_f32(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::DRIBBLE_DIRECTION_Y,
            -0.3,
            0.3,
        );
        let dir = (base_x, y_offset);
        normalize_direction(dir)
    });

    // 속도 계수 결정 (FIX_2601/1124: tactical_bias 사용)
    let speed_factor = intent.speed_factor.unwrap_or_else(|| {
        let (min_speed, max_speed) = ctx.tactical_bias.get_dribble_speed_bounds();
        deterministic_f32(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::DRIBBLE_SPEED,
            min_speed,
            max_speed,
        )
    });

    DribbleDetail {
        direction,
        speed_factor,
    }
}

// ============================================================================
// Tackle Builder
// ============================================================================

/// Tackle Detail 생성
///
/// 상대 팀에서 태클 대상을 선택한다.
pub fn build_tackle_detail(ctx: &DecisionContext, intent: &TackleIntent) -> TackleDetail {
    // 타겟 선택
    let target_track_id = intent.target.unwrap_or_else(|| {
        if ctx.opponents.is_empty() {
            // 상대 없음 - 이론상 발생하면 안 됨
            if ctx.is_home {
                11
            } else {
                0
            }
        } else {
            let idx = deterministic_choice(
                ctx.seed,
                ctx.tick,
                ctx.owner_idx,
                subcase::TACKLE_TARGET,
                ctx.opponents.len(),
            );
            ctx.opponents[idx].0 as u8
        }
    });

    TackleDetail {
        target_track_id,
        tackle_kind: intent.tackle_kind.unwrap_or(TackleKind::Standing),
    }
}

// ============================================================================
// Header Builder
// ============================================================================

/// Header Detail 생성
pub fn build_header_detail(ctx: &DecisionContext, intent: &HeaderIntent) -> HeaderDetail {
    let target = match intent.target_type {
        HeaderTargetType::Shot => {
            let point = intent.shot_point.unwrap_or_else(|| {
                let goal = ctx.goal_center();
                let y_offset = deterministic_f32(
                    ctx.seed,
                    ctx.tick,
                    ctx.owner_idx,
                    subcase::HEADER_TARGET_Y,
                    -0.1,
                    0.1,
                );
                (goal.0, goal.1 + y_offset)
            });
            HeaderTarget::Shot { point }
        }
        HeaderTargetType::Pass => {
            let target_track_id = intent.pass_target.unwrap_or_else(|| {
                let valid_targets = ctx.valid_pass_targets();
                select_pass_target(ctx, &valid_targets)
            });
            HeaderTarget::Pass { target_track_id }
        }
        HeaderTargetType::Clear => {
            let direction = intent.clear_direction.unwrap_or_else(|| {
                // 클리어: 골 반대 방향 + 약간의 y 편차
                let base_x = -ctx.goal_direction;
                let y_offset = deterministic_f32(
                    ctx.seed,
                    ctx.tick,
                    ctx.owner_idx,
                    subcase::HEADER_CLEAR_DIRECTION,
                    -0.5,
                    0.5,
                );
                normalize_direction((base_x, y_offset))
            });
            HeaderTarget::Clear { direction }
        }
    };

    let power = intent.power.unwrap_or_else(|| {
        deterministic_f32(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::HEADER_POWER,
            0.5,
            0.9,
        )
    });

    HeaderDetail { target, power }
}

// ============================================================================
// Cross Builder
// ============================================================================

/// Cross Detail 생성
pub fn build_cross_detail(ctx: &DecisionContext, intent: &CrossIntent) -> CrossDetail {
    let target_point = intent.target_point.unwrap_or_else(|| {
        // 크로스 목표: 페널티 에어리어 부근
        let x = if ctx.goal_direction > 0.0 {
            deterministic_f32(
                ctx.seed,
                ctx.tick,
                ctx.owner_idx,
                subcase::CROSS_TARGET_X,
                0.8,
                0.95,
            )
        } else {
            deterministic_f32(
                ctx.seed,
                ctx.tick,
                ctx.owner_idx,
                subcase::CROSS_TARGET_X,
                0.05,
                0.2,
            )
        };
        let y = deterministic_f32(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::CROSS_TARGET_Y,
            0.3,
            0.7,
        );
        (x, y)
    });

    let power = intent.power.unwrap_or_else(|| {
        deterministic_f32(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::CROSS_POWER,
            0.6,
            0.9,
        )
    });

    CrossDetail {
        target_point,
        cross_kind: intent.cross_kind.unwrap_or(CrossKind::High),
        power,
    }
}

// ============================================================================
// Clearance Builder
// ============================================================================

/// Clearance Detail 생성
pub fn build_clearance_detail(ctx: &DecisionContext, intent: &ClearanceIntent) -> ClearanceDetail {
    let direction = intent.direction.unwrap_or_else(|| {
        // 클리어: 자기 골문 반대 방향 (+ 위 또는 아래로)
        let base_x = -ctx.goal_direction;
        let y = deterministic_f32(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::CLEARANCE_DIRECTION_Y,
            -0.7,
            0.7,
        );
        normalize_direction((base_x, y))
    });

    let power = intent.power.unwrap_or_else(|| {
        deterministic_f32(
            ctx.seed,
            ctx.tick,
            ctx.owner_idx,
            subcase::CLEARANCE_POWER,
            0.7,
            1.0,
        )
    });

    ClearanceDetail { direction, power }
}

// ============================================================================
// Intercept Builder
// ============================================================================

/// Intercept Detail 생성
pub fn build_intercept_detail(ctx: &DecisionContext) -> InterceptDetail {
    // 인터셉트 지점: 볼 위치 기준
    InterceptDetail {
        intercept_point: ctx.ball_pos,
    }
}

// ============================================================================
// Hold Builder
// ============================================================================

/// Hold Detail 생성
pub fn build_hold_detail(ctx: &DecisionContext) -> HoldDetail {
    // 쉴드 방향: 가장 가까운 상대 반대 방향
    let shield_direction = if ctx.opponents.is_empty() {
        // 상대 없으면 골 방향
        (ctx.goal_direction, 0.0)
    } else {
        // 가장 가까운 상대 찾기
        let closest = ctx
            .opponents
            .iter()
            .min_by(|(_, pos_a), (_, pos_b)| {
                let dist_a = (pos_a.0 - ctx.owner_pos.0).powi(2)
                    + (pos_a.1 - ctx.owner_pos.1).powi(2);
                let dist_b = (pos_b.0 - ctx.owner_pos.0).powi(2)
                    + (pos_b.1 - ctx.owner_pos.1).powi(2);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .map(|(_, pos)| *pos)
            .unwrap_or((0.5, 0.5));

        // 상대 반대 방향
        let dir = (ctx.owner_pos.0 - closest.0, ctx.owner_pos.1 - closest.1);
        normalize_direction(dir)
    };

    HoldDetail { shield_direction }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> DecisionContext {
        DecisionContext {
            seed: 12345,
            tick: 100,
            owner_idx: 5,
            owner_pos: (0.5, 0.5),
            ball_pos: (0.5, 0.5),
            is_home: true,
            teammates: vec![
                (0, (0.1, 0.5)),
                (1, (0.2, 0.3)),
                (2, (0.3, 0.7)),
                (3, (0.4, 0.4)),
                (4, (0.4, 0.6)),
            ],
            opponents: vec![(11, (0.6, 0.5)), (12, (0.7, 0.4)), (13, (0.8, 0.6))],
            goal_direction: 1.0,
            offside_line: 0.8,
            tactical_bias: TacticalBias::default(),
            selected_key: None,
        }
    }

    #[test]
    fn test_decision_context_is_offside() {
        let ctx = make_ctx();
        assert!(!ctx.is_offside(0.7)); // offside line 이전
        assert!(ctx.is_offside(0.85)); // offside line 이후
    }

    #[test]
    fn test_decision_context_valid_pass_targets() {
        let ctx = make_ctx();
        let targets = ctx.valid_pass_targets();

        // owner(5) 제외, offside(x > 0.8) 제외
        assert!(!targets.iter().any(|(id, _)| *id == 5));
        assert!(targets.iter().all(|(_, pos)| pos.0 <= 0.8));
    }

    #[test]
    fn test_build_pass_detail_deterministic() {
        let ctx = make_ctx();
        let intent = PassIntent::default();

        let d1 = build_pass_detail(&ctx, &intent);
        let d2 = build_pass_detail(&ctx, &intent);

        assert_eq!(d1.target_track_id, d2.target_track_id);
        assert_eq!(d1.power, d2.power);
        assert_eq!(d1.pass_kind, d2.pass_kind);
    }

    #[test]
    fn test_build_pass_detail_with_intent() {
        let ctx = make_ctx();
        let intent = PassIntent {
            intended_target: Some(2), // 유효한 타겟
            pass_kind: PassKind::Through,
            power: Some(0.6),
            intended_point: None,
        };

        let detail = build_pass_detail(&ctx, &intent);
        assert_eq!(detail.target_track_id, 2);
        assert_eq!(detail.pass_kind, PassKind::Through);
        assert!((detail.power - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_build_shot_detail_deterministic() {
        let ctx = make_ctx();
        let intent = ShotIntent::default();

        let d1 = build_shot_detail(&ctx, &intent);
        let d2 = build_shot_detail(&ctx, &intent);

        assert_eq!(d1.target_point, d2.target_point);
        assert_eq!(d1.power, d2.power);
        assert_eq!(d1.shot_kind, d2.shot_kind);
    }

    #[test]
    fn test_build_shot_detail_target_in_goal_range() {
        let ctx = make_ctx();
        let intent = ShotIntent::default();

        let detail = build_shot_detail(&ctx, &intent);

        // goal_direction > 0 → x = 1.0
        assert!((detail.target_point.0 - 1.0).abs() < 0.01);
        // y는 0.4 ~ 0.6 범위
        assert!(detail.target_point.1 >= 0.4 && detail.target_point.1 < 0.6);
    }

    #[test]
    fn test_build_dribble_detail_deterministic() {
        let ctx = make_ctx();
        let intent = DribbleIntent::default();

        let d1 = build_dribble_detail(&ctx, &intent);
        let d2 = build_dribble_detail(&ctx, &intent);

        assert_eq!(d1.direction, d2.direction);
        assert_eq!(d1.speed_factor, d2.speed_factor);
    }

    #[test]
    fn test_build_dribble_detail_direction_normalized() {
        let ctx = make_ctx();
        let intent = DribbleIntent::default();

        let detail = build_dribble_detail(&ctx, &intent);
        let len = (detail.direction.0.powi(2) + detail.direction.1.powi(2)).sqrt();
        assert!((len - 1.0).abs() < 0.01, "Direction should be normalized");
    }

    #[test]
    fn test_build_tackle_detail_deterministic() {
        let ctx = make_ctx();
        let intent = TackleIntent::default();

        let d1 = build_tackle_detail(&ctx, &intent);
        let d2 = build_tackle_detail(&ctx, &intent);

        assert_eq!(d1.target_track_id, d2.target_track_id);
        assert_eq!(d1.tackle_kind, d2.tackle_kind);
    }

    #[test]
    fn test_build_tackle_detail_targets_opponent() {
        let ctx = make_ctx();
        let intent = TackleIntent::default();

        let detail = build_tackle_detail(&ctx, &intent);

        // 상대 팀 인덱스 (11, 12, 13) 중 하나여야 함
        assert!(
            [11, 12, 13].contains(&detail.target_track_id),
            "Target should be an opponent"
        );
    }

    #[test]
    fn test_build_header_detail_shot() {
        let ctx = make_ctx();
        let intent = HeaderIntent {
            target_type: HeaderTargetType::Shot,
            ..Default::default()
        };

        let detail = build_header_detail(&ctx, &intent);

        match detail.target {
            HeaderTarget::Shot { point } => {
                // 골문 방향에 가까워야 함
                assert!(point.0 > 0.9);
            }
            _ => panic!("Expected HeaderTarget::Shot"),
        }
    }

    #[test]
    fn test_build_cross_detail() {
        let ctx = make_ctx();
        let intent = CrossIntent::default();

        let detail = build_cross_detail(&ctx, &intent);

        // goal_direction > 0 → target x는 0.8 ~ 0.95 범위
        assert!(detail.target_point.0 >= 0.8 && detail.target_point.0 < 0.95);
    }

    #[test]
    fn test_build_clearance_detail() {
        let ctx = make_ctx();
        let intent = ClearanceIntent::default();

        let detail = build_clearance_detail(&ctx, &intent);

        // 클리어 방향은 골 반대 (goal_direction = 1.0 → direction.x < 0)
        assert!(detail.direction.0 < 0.0);
    }

    #[test]
    fn test_build_hold_detail() {
        let ctx = make_ctx();

        let detail = build_hold_detail(&ctx);

        // 방향이 정규화되어 있어야 함
        let len = (detail.shield_direction.0.powi(2) + detail.shield_direction.1.powi(2)).sqrt();
        assert!((len - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_different_ticks_different_results() {
        let mut ctx1 = make_ctx();
        let mut ctx2 = make_ctx();
        ctx2.tick = 200; // 다른 틱

        let intent = PassIntent::default();

        let d1 = build_pass_detail(&ctx1, &intent);
        let d2 = build_pass_detail(&ctx2, &intent);

        // 다른 틱이면 결과가 다를 가능성이 높음
        // (하지만 우연히 같을 수도 있으므로 단순 컴파일 확인)
        let _ = (d1, d2);
    }
}
