//! FIX_2601/1124: Tactical Bias System
//!
//! TacticalPreset과 TeamInstructions를 Detail Builder가 사용할 수 있는
//! 연속적인 bias 값으로 변환한다.
//!
//! ## 설계 원칙
//!
//! 1. **프리셋 기반 분포**: 각 TacticalPreset이 고유한 행동 분포를 가짐
//! 2. **연속 값 사용**: 0.0-1.0 범위의 bias로 deterministic 선택에 영향
//! 3. **레인지 기반**: power 같은 값은 (min, max) 레인지로 제공
//!
//! ## 사용 흐름
//!
//! ```text
//! TacticalPreset / TeamInstructions
//!     → TacticalBias::from_preset() / from_instructions()
//!     → DecisionContext에 저장
//!     → Builder가 bias 값 참조
//! ```

use serde::{Deserialize, Serialize};
use crate::tactics::team_instructions::{TacticalPreset, TeamInstructions, BuildUpStyle};
use super::action_detail_v2::PassKind;

// ============================================================================
// TacticalBias
// ============================================================================

/// 전술적 편향 - Detail Builder가 사용하는 연속 값들
///
/// TacticalPreset 또는 TeamInstructions에서 변환되어 생성된다.
/// Builder 함수들은 이 값들을 참조하여 결정론적으로 상세 값을 생성한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalBias {
    // ========================================================================
    // Pass Biases
    // ========================================================================
    /// 패스 위험도 편향 (0.0=안전, 1.0=위험)
    /// 높을수록 리스키한 스루패스, 롱패스 선호
    pub pass_risk_bias: f32,

    /// 패스 진행 편향 (0.0=지원, 1.0=진행)
    /// 높을수록 전진 패스 선호
    pub pass_progress_bias: f32,

    /// 패스 파워 범위 (min, max)
    /// Builder가 deterministic_f32로 이 범위 내에서 선택
    pub pass_power_range: (f32, f32),

    /// 패스 종류별 가중치 [Short, Through, Long, Lob]
    /// 합이 1.0이 아니어도 됨 (상대적 비율로 사용)
    pub pass_kind_weights: [f32; 4],

    // ========================================================================
    // Shot Biases
    // ========================================================================
    /// 슛 목표 Y 범위 (min, max, 0.0=왼쪽 포스트, 1.0=오른쪽 포스트)
    /// 예: (0.45, 0.55) = 중앙 집중, (0.3, 0.7) = 넓은 범위
    pub shot_target_y_range: (f32, f32),

    /// 슛 파워 범위 (min, max)
    pub shot_power_range: (f32, f32),

    /// 슛 시도율 편향 (0.0=보수적, 1.0=적극적)
    /// 높을수록 슛 기회에서 슛 선택 확률 증가
    pub shot_take_rate: f32,

    // ========================================================================
    // Dribble Biases
    // ========================================================================
    /// 드리블 속도 범위 (min, max)
    pub dribble_speed_range: (f32, f32),

    /// 드리블 시도율 편향 (0.0=보수적, 1.0=적극적)
    pub dribble_attempt_rate: f32,

    // ========================================================================
    // General Biases
    // ========================================================================
    /// 템포 팩터 (0.0=느림, 1.0=빠름)
    /// 결정 속도와 전반적인 경기 흐름에 영향
    pub tempo_factor: f32,

    /// 압박 팩터 (0.0=낮음, 1.0=높음)
    /// 수비 시 적극성에 영향
    pub pressing_factor: f32,

    /// 크로스 시도율 편향
    pub cross_attempt_rate: f32,

    /// 클리어 파워 범위
    pub clearance_power_range: (f32, f32),
}

impl Default for TacticalBias {
    fn default() -> Self {
        Self::balanced()
    }
}

impl TacticalBias {
    /// Balanced 프리셋 (기본값)
    pub fn balanced() -> Self {
        Self {
            pass_risk_bias: 0.40,
            pass_progress_bias: 0.50,
            pass_power_range: (0.4, 0.8),
            pass_kind_weights: [0.50, 0.20, 0.20, 0.10], // Short 우세

            shot_target_y_range: (0.40, 0.60),
            shot_power_range: (0.70, 1.00),
            shot_take_rate: 0.12,

            dribble_speed_range: (0.60, 0.90),
            dribble_attempt_rate: 0.15,

            tempo_factor: 0.55,
            pressing_factor: 0.50,
            cross_attempt_rate: 0.15,
            clearance_power_range: (0.70, 1.00),
        }
    }

    /// Possession 프리셋
    pub fn possession() -> Self {
        Self {
            pass_risk_bias: 0.25,       // 안전한 패스 선호
            pass_progress_bias: 0.35,   // 지원 패스 선호
            pass_power_range: (0.35, 0.70), // 낮은 파워
            pass_kind_weights: [0.65, 0.10, 0.15, 0.10], // Short 매우 우세

            shot_target_y_range: (0.42, 0.58), // 정확한 중앙
            shot_power_range: (0.65, 0.90),    // 중간 파워 (피네스)
            shot_take_rate: 0.08,              // 보수적 슛 시도

            dribble_speed_range: (0.50, 0.80), // 느린 드리블
            dribble_attempt_rate: 0.12,        // 적은 드리블 시도

            tempo_factor: 0.30,        // 느린 템포
            pressing_factor: 0.45,     // 중간 압박
            cross_attempt_rate: 0.12,
            clearance_power_range: (0.60, 0.85),
        }
    }

    /// Counter 프리셋
    pub fn counter() -> Self {
        Self {
            pass_risk_bias: 0.65,       // 리스키한 패스 선호
            pass_progress_bias: 0.75,   // 전진 패스 선호
            pass_power_range: (0.50, 0.95), // 높은 파워
            pass_kind_weights: [0.30, 0.30, 0.30, 0.10], // Through, Long 증가

            shot_target_y_range: (0.35, 0.65), // 넓은 범위
            shot_power_range: (0.75, 1.00),    // 높은 파워
            shot_take_rate: 0.18,              // 적극적 슛 시도

            dribble_speed_range: (0.70, 1.00), // 빠른 드리블
            dribble_attempt_rate: 0.20,        // 많은 드리블 시도

            tempo_factor: 0.85,        // 빠른 템포
            pressing_factor: 0.35,     // 낮은 압박 (수비 후 역습)
            cross_attempt_rate: 0.18,
            clearance_power_range: (0.80, 1.00),
        }
    }

    /// High Press 프리셋
    pub fn high_press() -> Self {
        Self {
            pass_risk_bias: 0.50,       // 중간 위험도
            pass_progress_bias: 0.60,   // 약간 전진 선호
            pass_power_range: (0.45, 0.85),
            pass_kind_weights: [0.45, 0.25, 0.20, 0.10],

            shot_target_y_range: (0.38, 0.62),
            shot_power_range: (0.72, 1.00),
            shot_take_rate: 0.16,

            dribble_speed_range: (0.65, 0.95),
            dribble_attempt_rate: 0.18,

            tempo_factor: 0.80,        // 빠른 템포
            pressing_factor: 0.90,     // 매우 높은 압박
            cross_attempt_rate: 0.16,
            clearance_power_range: (0.75, 1.00),
        }
    }

    /// Park the Bus (Defensive) 프리셋
    pub fn park_the_bus() -> Self {
        Self {
            pass_risk_bias: 0.20,       // 매우 안전한 패스
            pass_progress_bias: 0.25,   // 지원 패스 선호
            pass_power_range: (0.30, 0.65),
            pass_kind_weights: [0.60, 0.05, 0.25, 0.10], // Short, Long (클리어용)

            shot_target_y_range: (0.45, 0.55), // 정확한 중앙만
            shot_power_range: (0.70, 0.95),
            shot_take_rate: 0.05,              // 매우 보수적

            dribble_speed_range: (0.45, 0.75),
            dribble_attempt_rate: 0.08,        // 거의 드리블 안함

            tempo_factor: 0.35,        // 느린 템포
            pressing_factor: 0.25,     // 낮은 압박
            cross_attempt_rate: 0.08,
            clearance_power_range: (0.80, 1.00), // 강한 클리어
        }
    }

    /// TacticalPreset에서 변환
    pub fn from_preset(preset: TacticalPreset) -> Self {
        match preset {
            TacticalPreset::Possession => Self::possession(),
            TacticalPreset::Counterattack => Self::counter(),
            TacticalPreset::HighPressing => Self::high_press(),
            TacticalPreset::Defensive => Self::park_the_bus(),
            TacticalPreset::Balanced => Self::balanced(),
        }
    }

    /// TeamInstructions에서 변환
    ///
    /// 개별 설정 값들을 조합하여 bias 생성
    pub fn from_instructions(inst: &TeamInstructions) -> Self {
        let tempo = inst.get_tempo_factor();
        let pressing = inst.get_pressing_factor();

        // Build-up style → pass risk/progress
        let (pass_risk, pass_progress, pass_kind_weights) = match inst.build_up_style {
            BuildUpStyle::Short => (0.25, 0.35, [0.65, 0.10, 0.15, 0.10]),
            BuildUpStyle::Mixed => (0.40, 0.50, [0.50, 0.20, 0.20, 0.10]),
            BuildUpStyle::Direct => (0.60, 0.70, [0.25, 0.25, 0.40, 0.10]),
        };

        // Tempo → power ranges
        let pass_power_range = if tempo > 0.7 {
            (0.50, 0.90)
        } else if tempo > 0.4 {
            (0.40, 0.80)
        } else {
            (0.35, 0.70)
        };

        let dribble_speed_range = if tempo > 0.7 {
            (0.70, 1.00)
        } else if tempo > 0.4 {
            (0.60, 0.90)
        } else {
            (0.50, 0.80)
        };

        // Pressing → shot take rate, dribble attempt rate
        let shot_take_rate = 0.08 + pressing * 0.10;
        let dribble_attempt_rate = 0.10 + pressing * 0.08;
        let cross_attempt_rate = 0.12 + pressing * 0.06;

        Self {
            pass_risk_bias: pass_risk,
            pass_progress_bias: pass_progress,
            pass_power_range,
            pass_kind_weights,

            shot_target_y_range: (0.40, 0.60), // 기본값 유지
            shot_power_range: (0.70, 1.00),
            shot_take_rate,

            dribble_speed_range,
            dribble_attempt_rate,

            tempo_factor: tempo,
            pressing_factor: pressing,
            cross_attempt_rate,
            clearance_power_range: (0.70, 1.00),
        }
    }

    /// 패스 종류에 대한 가중치 반환
    pub fn get_pass_kind_weight(&self, kind: PassKind) -> f32 {
        match kind {
            PassKind::Short => self.pass_kind_weights[0],
            PassKind::Through => self.pass_kind_weights[1],
            PassKind::Long => self.pass_kind_weights[2],
            PassKind::Lob => self.pass_kind_weights[3],
        }
    }

    /// 파워 범위를 deterministic_f32에 사용하기 위한 튜플 반환
    pub fn get_pass_power_bounds(&self) -> (f32, f32) {
        self.pass_power_range
    }

    pub fn get_shot_power_bounds(&self) -> (f32, f32) {
        self.shot_power_range
    }

    pub fn get_dribble_speed_bounds(&self) -> (f32, f32) {
        self.dribble_speed_range
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_balanced() {
        let default = TacticalBias::default();
        let balanced = TacticalBias::balanced();

        assert_eq!(default.tempo_factor, balanced.tempo_factor);
        assert_eq!(default.pressing_factor, balanced.pressing_factor);
    }

    #[test]
    fn test_from_preset() {
        let possession = TacticalBias::from_preset(TacticalPreset::Possession);
        assert!(possession.pass_risk_bias < 0.30);
        assert!(possession.tempo_factor < 0.40);

        let counter = TacticalBias::from_preset(TacticalPreset::Counterattack);
        assert!(counter.pass_risk_bias > 0.60);
        assert!(counter.tempo_factor > 0.80);

        let high_press = TacticalBias::from_preset(TacticalPreset::HighPressing);
        assert!(high_press.pressing_factor > 0.85);
    }

    #[test]
    fn test_from_instructions() {
        let inst = TeamInstructions::for_style(TacticalPreset::Possession);
        let bias = TacticalBias::from_instructions(&inst);

        // Possession의 특성이 반영되어야 함
        assert!(bias.tempo_factor < 0.50); // Slow tempo
    }

    #[test]
    fn test_pass_kind_weight() {
        let bias = TacticalBias::possession();

        let short_weight = bias.get_pass_kind_weight(PassKind::Short);
        let through_weight = bias.get_pass_kind_weight(PassKind::Through);

        // Possession은 Short 패스 가중치가 높아야 함
        assert!(short_weight > through_weight);
    }

    #[test]
    fn test_power_bounds() {
        let counter = TacticalBias::counter();

        let (min, max) = counter.get_pass_power_bounds();
        assert!(min < max);
        assert!(min >= 0.0);
        assert!(max <= 1.0);

        let (min, max) = counter.get_dribble_speed_bounds();
        assert!(min < max);
        assert!(min >= 0.0);
        assert!(max <= 1.0);
    }

    #[test]
    fn test_preset_distributions_match_spec() {
        // 1124 스펙 Part 4 기준 검증
        let possession = TacticalBias::from_preset(TacticalPreset::Possession);
        assert!((possession.pass_risk_bias - 0.25).abs() < 0.01);
        assert!((possession.tempo_factor - 0.30).abs() < 0.01);
        assert!((possession.shot_take_rate - 0.08).abs() < 0.01);

        let counter = TacticalBias::from_preset(TacticalPreset::Counterattack);
        assert!((counter.pass_risk_bias - 0.65).abs() < 0.01);
        assert!((counter.tempo_factor - 0.85).abs() < 0.01);
        assert!((counter.shot_take_rate - 0.18).abs() < 0.01);

        let high_press = TacticalBias::from_preset(TacticalPreset::HighPressing);
        assert!((high_press.tempo_factor - 0.80).abs() < 0.01);
        assert!((high_press.shot_take_rate - 0.16).abs() < 0.01);

        let park_bus = TacticalBias::from_preset(TacticalPreset::Defensive);
        assert!((park_bus.pass_risk_bias - 0.20).abs() < 0.01);
        assert!((park_bus.tempo_factor - 0.35).abs() < 0.01);
        assert!((park_bus.shot_take_rate - 0.05).abs() < 0.01);

        let balanced = TacticalBias::from_preset(TacticalPreset::Balanced);
        assert!((balanced.pass_risk_bias - 0.40).abs() < 0.01);
        assert!((balanced.tempo_factor - 0.55).abs() < 0.01);
        assert!((balanced.shot_take_rate - 0.12).abs() < 0.01);
    }
}
