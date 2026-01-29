//! Role Transition Matrix - 역할 기반 패스 전이 매트릭스
//!
//! OpenFootManager의 Zone Transition Matrix 개념을 가져오되,
//! 필드 고정 존이 아닌 **포메이션 역할(Role) 기반 동적 존**으로 구현
//!
//! ## 핵심 차이점
//! - OpenFootManager: 15개 필드 고정 존 (DEF_BOX, MIDFIELD_CENTER 등)
//! - 우리: 포메이션 역할이 존 (GK, LB, CM, ST 등)
//!
//! ## 사용 예시
//! ```ignore
//! let matrix = RoleTransitionMatrix::new_442_balanced();
//! let weight = matrix.get_weight(PositionKey::CM, PositionKey::ST);
//! // CM → ST 패스 선호도 = 1.3 (높음)
//! ```

use std::collections::HashMap;

use super::positioning::PositionKey;
use crate::models::player::PlayerAttributes;

// =============================================================================
// Constants
// =============================================================================

/// 역할 가중치 최소값 (비선호 역할)
pub const MIN_ROLE_WEIGHT: f32 = 0.5;

/// 역할 가중치 최대값 (강력 선호 역할)
pub const MAX_ROLE_WEIGHT: f32 = 1.5;

/// 중립 가중치
pub const NEUTRAL_WEIGHT: f32 = 1.0;

/// 역할 가중치가 최종 점수에 미치는 영향도 (가산 시)
/// 0.3 = 역할 가중치가 최대 ±15% 영향
pub const ROLE_INFLUENCE: f32 = 0.3;

// =============================================================================
// TacticalStyle
// =============================================================================

/// 전술 스타일 - 패스 패턴에 영향
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TacticalStyle {
    /// 균형 잡힌 패스 패턴
    #[default]
    Balanced,

    /// 점유율 중심 - 뒤로 안전한 패스 선호
    Possession,

    /// 역습 중심 - 앞으로 직접 패스 선호
    Counter,

    /// 측면 공격 - 윙어로 패스 선호
    WingPlay,
}

// =============================================================================
// RoleTransitionMatrix
// =============================================================================

/// 역할 간 패스 선호도 매트릭스
///
/// weights[from_role][to_role] = 가중치 (0.5 ~ 1.5)
#[derive(Debug, Clone)]
pub struct RoleTransitionMatrix {
    /// 역할 간 가중치 매트릭스
    weights: HashMap<PositionKey, HashMap<PositionKey, f32>>,

    /// 이 매트릭스의 포메이션
    pub formation: String,

    /// 전술 스타일
    pub style: TacticalStyle,
}

impl Default for RoleTransitionMatrix {
    fn default() -> Self {
        Self::new_442_balanced()
    }
}

impl RoleTransitionMatrix {
    /// 빈 매트릭스 생성
    pub fn empty(formation: &str, style: TacticalStyle) -> Self {
        Self {
            weights: HashMap::new(),
            formation: formation.to_string(),
            style,
        }
    }

    /// 역할 간 가중치 설정
    pub fn set_weight(&mut self, from: PositionKey, to: PositionKey, weight: f32) {
        let clamped = weight.clamp(MIN_ROLE_WEIGHT, MAX_ROLE_WEIGHT);
        self.weights.entry(from).or_default().insert(to, clamped);
    }

    /// 역할 간 가중치 조회 (없으면 중립 반환)
    pub fn get_weight(&self, from: PositionKey, to: PositionKey) -> f32 {
        self.weights
            .get(&from)
            .and_then(|m| m.get(&to))
            .copied()
            .unwrap_or(NEUTRAL_WEIGHT)
    }

    /// 여러 목표 역할에 대한 가중치 일괄 설정
    fn set_weights(&mut self, from: PositionKey, targets: &[(PositionKey, f32)]) {
        for &(to, weight) in targets {
            self.set_weight(from, to, weight);
        }
    }

    // =========================================================================
    // 4-4-2 Formations
    // =========================================================================

    /// 4-4-2 Balanced (균형)
    pub fn new_442_balanced() -> Self {
        let mut m = Self::empty("4-4-2", TacticalStyle::Balanced);

        // GK 패스 패턴
        m.set_weights(PositionKey::GK, &[
            (PositionKey::LCB, 1.3),    // 센터백 선호
            (PositionKey::RCB, 1.3),
            (PositionKey::LB, 1.1),
            (PositionKey::RB, 1.1),
            (PositionKey::CDM, 0.9),    // 중앙 연결 가능
            (PositionKey::LCM, 0.7),    // 롱볼 비선호
            (PositionKey::RCM, 0.7),
        ]);

        // 센터백 (LCB) 패스 패턴
        m.set_weights(PositionKey::LCB, &[
            (PositionKey::GK, 0.8),     // 백패스 가능
            (PositionKey::RCB, 1.1),    // 횡패스
            (PositionKey::LB, 1.2),     // 풀백 연계
            (PositionKey::CDM, 1.3),    // 수비형 미드 선호
            (PositionKey::LCM, 1.2),    // 중앙 전진
            (PositionKey::RCM, 0.9),
            (PositionKey::LM, 1.0),     // 측면
            (PositionKey::LF, 0.6),     // 롱볼 비선호
            (PositionKey::RF, 0.5),
        ]);

        m.set_weights(PositionKey::RCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::LCB, 1.1),
            (PositionKey::RB, 1.2),
            (PositionKey::CDM, 1.3),
            (PositionKey::RCM, 1.2),
            (PositionKey::LCM, 0.9),
            (PositionKey::RM, 1.0),
            (PositionKey::RF, 0.6),
            (PositionKey::LF, 0.5),
        ]);

        // 풀백 (LB) 패스 패턴
        m.set_weights(PositionKey::LB, &[
            (PositionKey::LCB, 1.1),    // 백패스
            (PositionKey::CDM, 1.0),
            (PositionKey::LM, 1.4),     // 측면 연계 강력 선호
            (PositionKey::LCM, 1.2),
            (PositionKey::LW, 1.3),     // 윙어 오버래핑
            (PositionKey::LF, 0.8),
        ]);

        m.set_weights(PositionKey::RB, &[
            (PositionKey::RCB, 1.1),
            (PositionKey::CDM, 1.0),
            (PositionKey::RM, 1.4),
            (PositionKey::RCM, 1.2),
            (PositionKey::RW, 1.3),
            (PositionKey::RF, 0.8),
        ]);

        // 중앙 미드 (LCM, RCM) 패스 패턴
        m.set_weights(PositionKey::LCM, &[
            (PositionKey::RCM, 1.1),    // 횡패스
            (PositionKey::LCB, 0.9),    // 백패스
            (PositionKey::CDM, 1.0),
            (PositionKey::LM, 1.2),     // 측면
            (PositionKey::CAM, 1.3),    // 공격형 미드 (있을 경우)
            (PositionKey::LF, 1.3),     // 스트라이커 선호
            (PositionKey::RF, 1.1),
            (PositionKey::LW, 1.1),
        ]);

        m.set_weights(PositionKey::RCM, &[
            (PositionKey::LCM, 1.1),
            (PositionKey::RCB, 0.9),
            (PositionKey::CDM, 1.0),
            (PositionKey::RM, 1.2),
            (PositionKey::CAM, 1.3),
            (PositionKey::RF, 1.3),
            (PositionKey::LF, 1.1),
            (PositionKey::RW, 1.1),
        ]);

        // 측면 미드 (LM, RM) 패스 패턴
        m.set_weights(PositionKey::LM, &[
            (PositionKey::LB, 1.0),     // 풀백 연계
            (PositionKey::LCM, 1.1),
            (PositionKey::LW, 1.2),     // 윙어
            (PositionKey::LF, 1.3),     // 스트라이커 크로스
            (PositionKey::RF, 1.0),     // 파사이드
            (PositionKey::RM, 0.8),     // 사이드 체인지
        ]);

        m.set_weights(PositionKey::RM, &[
            (PositionKey::RB, 1.0),
            (PositionKey::RCM, 1.1),
            (PositionKey::RW, 1.2),
            (PositionKey::RF, 1.3),
            (PositionKey::LF, 1.0),
            (PositionKey::LM, 0.8),
        ]);

        // 스트라이커 (LF, RF) 패스 패턴 - 4-4-2의 듀얼 스트라이커
        m.set_weights(PositionKey::LF, &[
            (PositionKey::RF, 1.3),     // 스트라이커 연계
            (PositionKey::LCM, 1.0),    // 레이오프
            (PositionKey::RCM, 0.9),
            (PositionKey::LM, 1.1),
            (PositionKey::LW, 1.2),
        ]);

        m.set_weights(PositionKey::RF, &[
            (PositionKey::LF, 1.3),
            (PositionKey::RCM, 1.0),
            (PositionKey::LCM, 0.9),
            (PositionKey::RM, 1.1),
            (PositionKey::RW, 1.2),
        ]);

        m
    }

    /// 4-4-2 Possession (점유율 중심)
    pub fn new_442_possession() -> Self {
        let mut m = Self::new_442_balanced();
        m.style = TacticalStyle::Possession;

        // 백패스 가중치 증가
        m.set_weight(PositionKey::LCM, PositionKey::LCB, 1.2);
        m.set_weight(PositionKey::RCM, PositionKey::RCB, 1.2);
        m.set_weight(PositionKey::LCM, PositionKey::CDM, 1.3);
        m.set_weight(PositionKey::RCM, PositionKey::CDM, 1.3);

        // 롱볼 가중치 감소
        m.set_weight(PositionKey::LCB, PositionKey::LF, 0.5);
        m.set_weight(PositionKey::RCB, PositionKey::RF, 0.5);

        // 횡패스 가중치 증가
        m.set_weight(PositionKey::LCB, PositionKey::RCB, 1.3);
        m.set_weight(PositionKey::RCB, PositionKey::LCB, 1.3);

        m
    }

    /// 4-4-2 Counter (역습 중심)
    pub fn new_442_counter() -> Self {
        let mut m = Self::new_442_balanced();
        m.style = TacticalStyle::Counter;

        // 스트라이커로 직접 패스 증가
        m.set_weight(PositionKey::LCB, PositionKey::LF, 1.0);
        m.set_weight(PositionKey::RCB, PositionKey::RF, 1.0);
        m.set_weight(PositionKey::CDM, PositionKey::LF, 1.2);
        m.set_weight(PositionKey::CDM, PositionKey::RF, 1.2);
        m.set_weight(PositionKey::LCM, PositionKey::LF, 1.4);
        m.set_weight(PositionKey::RCM, PositionKey::RF, 1.4);

        // 백패스 가중치 감소
        m.set_weight(PositionKey::LCM, PositionKey::LCB, 0.6);
        m.set_weight(PositionKey::RCM, PositionKey::RCB, 0.6);
        m.set_weight(PositionKey::LCM, PositionKey::CDM, 0.7);
        m.set_weight(PositionKey::RCM, PositionKey::CDM, 0.7);

        m
    }

    // =========================================================================
    // 4-3-3 Formations
    // =========================================================================

    /// 4-3-3 Balanced
    pub fn new_433_balanced() -> Self {
        let mut m = Self::empty("4-3-3", TacticalStyle::Balanced);

        // GK
        m.set_weights(PositionKey::GK, &[
            (PositionKey::LCB, 1.3),
            (PositionKey::RCB, 1.3),
            (PositionKey::LB, 1.1),
            (PositionKey::RB, 1.1),
            (PositionKey::CM, 0.8),
        ]);

        // 센터백
        m.set_weights(PositionKey::LCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::RCB, 1.1),
            (PositionKey::LB, 1.2),
            (PositionKey::LCM, 1.2),
            (PositionKey::CM, 1.3),      // 앵커 미드 선호
            (PositionKey::RCM, 0.9),
            (PositionKey::LW, 0.7),
            (PositionKey::ST, 0.5),
        ]);

        m.set_weights(PositionKey::RCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::LCB, 1.1),
            (PositionKey::RB, 1.2),
            (PositionKey::RCM, 1.2),
            (PositionKey::CM, 1.3),
            (PositionKey::LCM, 0.9),
            (PositionKey::RW, 0.7),
            (PositionKey::ST, 0.5),
        ]);

        // 풀백
        m.set_weights(PositionKey::LB, &[
            (PositionKey::LCB, 1.0),
            (PositionKey::LCM, 1.2),
            (PositionKey::CM, 1.0),
            (PositionKey::LW, 1.4),      // 윙어 강력 선호
            (PositionKey::ST, 0.7),
        ]);

        m.set_weights(PositionKey::RB, &[
            (PositionKey::RCB, 1.0),
            (PositionKey::RCM, 1.2),
            (PositionKey::CM, 1.0),
            (PositionKey::RW, 1.4),
            (PositionKey::ST, 0.7),
        ]);

        // 미드필드 3인 (LCM, CM, RCM)
        m.set_weights(PositionKey::LCM, &[
            (PositionKey::CM, 1.2),
            (PositionKey::RCM, 1.0),
            (PositionKey::LCB, 0.9),
            (PositionKey::LB, 1.0),
            (PositionKey::LW, 1.3),
            (PositionKey::ST, 1.2),
        ]);

        m.set_weights(PositionKey::CM, &[
            (PositionKey::LCM, 1.1),
            (PositionKey::RCM, 1.1),
            (PositionKey::LCB, 0.9),
            (PositionKey::RCB, 0.9),
            (PositionKey::LW, 1.1),
            (PositionKey::RW, 1.1),
            (PositionKey::ST, 1.3),      // 스트라이커 선호
        ]);

        m.set_weights(PositionKey::RCM, &[
            (PositionKey::CM, 1.2),
            (PositionKey::LCM, 1.0),
            (PositionKey::RCB, 0.9),
            (PositionKey::RB, 1.0),
            (PositionKey::RW, 1.3),
            (PositionKey::ST, 1.2),
        ]);

        // 공격 3인 (LW, ST, RW)
        m.set_weights(PositionKey::LW, &[
            (PositionKey::LB, 0.9),
            (PositionKey::LCM, 1.0),
            (PositionKey::ST, 1.4),      // 스트라이커 크로스
            (PositionKey::RW, 0.8),      // 사이드 체인지
        ]);

        m.set_weights(PositionKey::ST, &[
            (PositionKey::LW, 1.2),
            (PositionKey::RW, 1.2),
            (PositionKey::CM, 1.0),      // 레이오프
            (PositionKey::LCM, 0.9),
            (PositionKey::RCM, 0.9),
        ]);

        m.set_weights(PositionKey::RW, &[
            (PositionKey::RB, 0.9),
            (PositionKey::RCM, 1.0),
            (PositionKey::ST, 1.4),
            (PositionKey::LW, 0.8),
        ]);

        m
    }

    // =========================================================================
    // WingPlay Style Matrices
    // =========================================================================

    /// 4-4-2 WingPlay - 측면 공격 극대화
    pub fn new_442_wingplay() -> Self {
        let mut m = Self::empty("4-4-2", TacticalStyle::WingPlay);

        // GK - 풀백으로 바로 배급
        m.set_weights(PositionKey::GK, &[
            (PositionKey::LCB, 1.1),
            (PositionKey::RCB, 1.1),
            (PositionKey::LB, 1.4),      // 풀백 강화
            (PositionKey::RB, 1.4),
            (PositionKey::CDM, 0.7),
        ]);

        // 센터백 - 풀백 오버래핑 유도
        m.set_weights(PositionKey::LCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::RCB, 1.0),
            (PositionKey::LB, 1.4),
            (PositionKey::LM, 1.2),
            (PositionKey::CDM, 1.0),
            (PositionKey::LF, 0.6),
        ]);

        m.set_weights(PositionKey::RCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::LCB, 1.0),
            (PositionKey::RB, 1.4),
            (PositionKey::RM, 1.2),
            (PositionKey::CDM, 1.0),
            (PositionKey::RF, 0.6),
        ]);

        // 풀백 - 측면 미드/윙어 강력 선호
        m.set_weights(PositionKey::LB, &[
            (PositionKey::LCB, 0.9),
            (PositionKey::LM, 1.5),      // 측면 미드 매우 선호
            (PositionKey::LW, 1.4),
            (PositionKey::LCM, 1.0),
            (PositionKey::LF, 1.2),      // 크로스
        ]);

        m.set_weights(PositionKey::RB, &[
            (PositionKey::RCB, 0.9),
            (PositionKey::RM, 1.5),
            (PositionKey::RW, 1.4),
            (PositionKey::RCM, 1.0),
            (PositionKey::RF, 1.2),
        ]);

        // 중앙 미드 - 측면 분배
        m.set_weights(PositionKey::LCM, &[
            (PositionKey::RCM, 0.9),
            (PositionKey::LM, 1.4),      // 측면 선호
            (PositionKey::LB, 1.2),
            (PositionKey::LF, 1.1),
            (PositionKey::RF, 0.9),
        ]);

        m.set_weights(PositionKey::RCM, &[
            (PositionKey::LCM, 0.9),
            (PositionKey::RM, 1.4),
            (PositionKey::RB, 1.2),
            (PositionKey::RF, 1.1),
            (PositionKey::LF, 0.9),
        ]);

        // 측면 미드 - 크로스/컷인
        m.set_weights(PositionKey::LM, &[
            (PositionKey::LB, 1.1),
            (PositionKey::LCM, 1.0),
            (PositionKey::LF, 1.5),      // 스트라이커 크로스 매우 선호
            (PositionKey::RF, 1.3),      // 파사이드 크로스
        ]);

        m.set_weights(PositionKey::RM, &[
            (PositionKey::RB, 1.1),
            (PositionKey::RCM, 1.0),
            (PositionKey::RF, 1.5),
            (PositionKey::LF, 1.3),
        ]);

        // 스트라이커 - 측면 연계
        m.set_weights(PositionKey::LF, &[
            (PositionKey::RF, 1.2),
            (PositionKey::LM, 1.3),
            (PositionKey::LCM, 0.9),
        ]);

        m.set_weights(PositionKey::RF, &[
            (PositionKey::LF, 1.2),
            (PositionKey::RM, 1.3),
            (PositionKey::RCM, 0.9),
        ]);

        m
    }

    /// 4-3-3 WingPlay - 윙어 중심 공격
    pub fn new_433_wingplay() -> Self {
        let mut m = Self::empty("4-3-3", TacticalStyle::WingPlay);

        // GK
        m.set_weights(PositionKey::GK, &[
            (PositionKey::LCB, 1.2),
            (PositionKey::RCB, 1.2),
            (PositionKey::LB, 1.3),
            (PositionKey::RB, 1.3),
        ]);

        // 센터백 - 풀백/윙어 연계
        m.set_weights(PositionKey::LCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::RCB, 1.0),
            (PositionKey::LB, 1.3),
            (PositionKey::CM, 1.1),
            (PositionKey::LW, 0.9),      // 롱볼 옵션
        ]);

        m.set_weights(PositionKey::RCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::LCB, 1.0),
            (PositionKey::RB, 1.3),
            (PositionKey::CM, 1.1),
            (PositionKey::RW, 0.9),
        ]);

        // 풀백 - 윙어 매우 선호
        m.set_weights(PositionKey::LB, &[
            (PositionKey::LCB, 0.9),
            (PositionKey::LCM, 1.1),
            (PositionKey::LW, 1.5),      // 윙어 매우 선호
            (PositionKey::ST, 0.8),
        ]);

        m.set_weights(PositionKey::RB, &[
            (PositionKey::RCB, 0.9),
            (PositionKey::RCM, 1.1),
            (PositionKey::RW, 1.5),
            (PositionKey::ST, 0.8),
        ]);

        // 미드필드 - 측면 분배
        m.set_weights(PositionKey::LCM, &[
            (PositionKey::CM, 1.1),
            (PositionKey::RCM, 0.9),
            (PositionKey::LB, 1.1),
            (PositionKey::LW, 1.4),      // 윙어 선호
            (PositionKey::ST, 1.1),
        ]);

        m.set_weights(PositionKey::CM, &[
            (PositionKey::LCM, 1.0),
            (PositionKey::RCM, 1.0),
            (PositionKey::LW, 1.3),
            (PositionKey::RW, 1.3),
            (PositionKey::ST, 1.2),
        ]);

        m.set_weights(PositionKey::RCM, &[
            (PositionKey::CM, 1.1),
            (PositionKey::LCM, 0.9),
            (PositionKey::RB, 1.1),
            (PositionKey::RW, 1.4),
            (PositionKey::ST, 1.1),
        ]);

        // 윙어 - 크로스/커팅 인사이드
        m.set_weights(PositionKey::LW, &[
            (PositionKey::LB, 1.0),
            (PositionKey::LCM, 0.9),
            (PositionKey::ST, 1.5),      // 스트라이커 크로스 매우 선호
            (PositionKey::RW, 0.9),      // 사이드 체인지
        ]);

        m.set_weights(PositionKey::RW, &[
            (PositionKey::RB, 1.0),
            (PositionKey::RCM, 0.9),
            (PositionKey::ST, 1.5),
            (PositionKey::LW, 0.9),
        ]);

        // 스트라이커 - 윙어 연계
        m.set_weights(PositionKey::ST, &[
            (PositionKey::LW, 1.3),
            (PositionKey::RW, 1.3),
            (PositionKey::CM, 1.0),
            (PositionKey::LCM, 0.9),
            (PositionKey::RCM, 0.9),
        ]);

        m
    }

    // =========================================================================
    // 3-5-2 Formation
    // =========================================================================

    /// 3-5-2 Balanced 매트릭스
    /// 포지션: GK, LCB, CB, RCB, LWB, RWB, LCM, CM, RCM, LF, RF
    pub fn new_352_balanced() -> Self {
        let mut m = Self::empty("3-5-2", TacticalStyle::Balanced);

        // 3백 (LCB, CB, RCB)
        m.set_weights(PositionKey::LCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::CB, 1.2),
            (PositionKey::LWB, 1.3),    // 윙백 적극 활용
            (PositionKey::LCM, 1.2),
            (PositionKey::CM, 1.0),
            (PositionKey::LF, 0.7),
        ]);

        m.set_weights(PositionKey::CB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::LCB, 1.1),
            (PositionKey::RCB, 1.1),
            (PositionKey::CM, 1.3),     // 중앙 미드 선호
            (PositionKey::LCM, 1.1),
            (PositionKey::RCM, 1.1),
            (PositionKey::LF, 0.6),
            (PositionKey::RF, 0.6),
        ]);

        m.set_weights(PositionKey::RCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::CB, 1.2),
            (PositionKey::RWB, 1.3),
            (PositionKey::RCM, 1.2),
            (PositionKey::CM, 1.0),
            (PositionKey::RF, 0.7),
        ]);

        // 윙백 (LWB, RWB) - 공수 연결
        m.set_weights(PositionKey::LWB, &[
            (PositionKey::LCB, 1.0),
            (PositionKey::LCM, 1.2),
            (PositionKey::CM, 1.0),
            (PositionKey::LF, 1.4),     // 크로스 선호
            (PositionKey::RF, 1.0),
            (PositionKey::RWB, 0.7),    // 사이드 체인지
        ]);

        m.set_weights(PositionKey::RWB, &[
            (PositionKey::RCB, 1.0),
            (PositionKey::RCM, 1.2),
            (PositionKey::CM, 1.0),
            (PositionKey::RF, 1.4),
            (PositionKey::LF, 1.0),
            (PositionKey::LWB, 0.7),
        ]);

        // 미드필드 3인 (LCM, CM, RCM)
        m.set_weights(PositionKey::LCM, &[
            (PositionKey::CM, 1.2),
            (PositionKey::RCM, 1.0),
            (PositionKey::LCB, 0.9),
            (PositionKey::LWB, 1.1),
            (PositionKey::LF, 1.3),
            (PositionKey::RF, 1.1),
        ]);

        m.set_weights(PositionKey::CM, &[
            (PositionKey::LCM, 1.1),
            (PositionKey::RCM, 1.1),
            (PositionKey::CB, 0.9),
            (PositionKey::LWB, 1.0),
            (PositionKey::RWB, 1.0),
            (PositionKey::LF, 1.3),
            (PositionKey::RF, 1.3),
        ]);

        m.set_weights(PositionKey::RCM, &[
            (PositionKey::CM, 1.2),
            (PositionKey::LCM, 1.0),
            (PositionKey::RCB, 0.9),
            (PositionKey::RWB, 1.1),
            (PositionKey::RF, 1.3),
            (PositionKey::LF, 1.1),
        ]);

        // 투톱 (LF, RF) - 파트너십
        m.set_weights(PositionKey::LF, &[
            (PositionKey::RF, 1.3),     // 투톱 연계
            (PositionKey::LCM, 1.0),
            (PositionKey::CM, 0.9),
            (PositionKey::LWB, 1.1),
        ]);

        m.set_weights(PositionKey::RF, &[
            (PositionKey::LF, 1.3),
            (PositionKey::RCM, 1.0),
            (PositionKey::CM, 0.9),
            (PositionKey::RWB, 1.1),
        ]);

        m
    }

    // =========================================================================
    // 4-2-3-1 Formation
    // =========================================================================

    /// 4-2-3-1 Balanced 매트릭스
    /// 포지션: GK, LB, LCB, RCB, RB, LDM, RDM, LAM, CAM, RAM, ST
    pub fn new_4231_balanced() -> Self {
        let mut m = Self::empty("4-2-3-1", TacticalStyle::Balanced);

        // 4백 (LB, LCB, RCB, RB)
        m.set_weights(PositionKey::LCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::RCB, 1.1),
            (PositionKey::LB, 1.2),
            (PositionKey::LDM, 1.3),    // 수비형 미드 선호
            (PositionKey::RDM, 1.0),
            (PositionKey::LAM, 0.6),
            (PositionKey::ST, 0.5),
        ]);

        m.set_weights(PositionKey::RCB, &[
            (PositionKey::GK, 0.8),
            (PositionKey::LCB, 1.1),
            (PositionKey::RB, 1.2),
            (PositionKey::RDM, 1.3),
            (PositionKey::LDM, 1.0),
            (PositionKey::RAM, 0.6),
            (PositionKey::ST, 0.5),
        ]);

        m.set_weights(PositionKey::LB, &[
            (PositionKey::LCB, 1.1),
            (PositionKey::LDM, 1.1),
            (PositionKey::LAM, 1.3),    // 측면 공격 선호
            (PositionKey::CAM, 1.0),
            (PositionKey::ST, 0.8),
        ]);

        m.set_weights(PositionKey::RB, &[
            (PositionKey::RCB, 1.1),
            (PositionKey::RDM, 1.1),
            (PositionKey::RAM, 1.3),
            (PositionKey::CAM, 1.0),
            (PositionKey::ST, 0.8),
        ]);

        // 더블 피벗 (LDM, RDM)
        m.set_weights(PositionKey::LDM, &[
            (PositionKey::RDM, 1.2),
            (PositionKey::LCB, 1.0),
            (PositionKey::LB, 1.1),
            (PositionKey::LAM, 1.3),
            (PositionKey::CAM, 1.4),    // 10번 선수 선호
            (PositionKey::RAM, 1.1),
            (PositionKey::ST, 0.9),
        ]);

        m.set_weights(PositionKey::RDM, &[
            (PositionKey::LDM, 1.2),
            (PositionKey::RCB, 1.0),
            (PositionKey::RB, 1.1),
            (PositionKey::RAM, 1.3),
            (PositionKey::CAM, 1.4),
            (PositionKey::LAM, 1.1),
            (PositionKey::ST, 0.9),
        ]);

        // 공격형 미드 3인 (LAM, CAM, RAM)
        m.set_weights(PositionKey::LAM, &[
            (PositionKey::LB, 0.9),
            (PositionKey::LDM, 1.0),
            (PositionKey::CAM, 1.2),
            (PositionKey::RAM, 1.0),
            (PositionKey::ST, 1.4),     // 스트라이커 선호
        ]);

        m.set_weights(PositionKey::CAM, &[
            (PositionKey::LDM, 0.9),
            (PositionKey::RDM, 0.9),
            (PositionKey::LAM, 1.1),
            (PositionKey::RAM, 1.1),
            (PositionKey::ST, 1.5),     // 스트라이커 강력 선호
        ]);

        m.set_weights(PositionKey::RAM, &[
            (PositionKey::RB, 0.9),
            (PositionKey::RDM, 1.0),
            (PositionKey::CAM, 1.2),
            (PositionKey::LAM, 1.0),
            (PositionKey::ST, 1.4),
        ]);

        // 원톱 (ST) - 레이오프
        m.set_weights(PositionKey::ST, &[
            (PositionKey::LAM, 1.2),
            (PositionKey::CAM, 1.3),    // CAM 레이오프 선호
            (PositionKey::RAM, 1.2),
            (PositionKey::LDM, 0.8),
            (PositionKey::RDM, 0.8),
        ]);

        m
    }

    // =========================================================================
    // 4-5-1 Formation (Defensive)
    // =========================================================================

    /// 4-5-1 Balanced 매트릭스 (수비적)
    pub fn new_451_balanced() -> Self {
        let mut m = Self::empty("4-5-1", TacticalStyle::Balanced);

        // GK 패턴 (CB로)
        m.set_weights(PositionKey::GK, &[
            (PositionKey::LCB, 1.2),
            (PositionKey::RCB, 1.2),
            (PositionKey::CDM, 0.8),
        ]);

        // CB 패턴 (미드필드로)
        m.set_weights(PositionKey::LCB, &[
            (PositionKey::CDM, 1.3),
            (PositionKey::LM, 1.1),
            (PositionKey::RCB, 1.0),
            (PositionKey::LB, 1.0),
        ]);

        m.set_weights(PositionKey::RCB, &[
            (PositionKey::CDM, 1.3),
            (PositionKey::RM, 1.1),
            (PositionKey::LCB, 1.0),
            (PositionKey::RB, 1.0),
        ]);

        // 풀백 패턴
        m.set_weights(PositionKey::LB, &[
            (PositionKey::LM, 1.3),
            (PositionKey::LCB, 1.0),
            (PositionKey::CDM, 0.9),
        ]);

        m.set_weights(PositionKey::RB, &[
            (PositionKey::RM, 1.3),
            (PositionKey::RCB, 1.0),
            (PositionKey::CDM, 0.9),
        ]);

        // CDM 패턴 (핵심 연결고리)
        m.set_weights(PositionKey::CDM, &[
            (PositionKey::LCM, 1.2),
            (PositionKey::RCM, 1.2),
            (PositionKey::LM, 1.1),
            (PositionKey::RM, 1.1),
            (PositionKey::ST, 1.0),
            (PositionKey::LCB, 0.9),
            (PositionKey::RCB, 0.9),
        ]);

        // 중앙 미드필더 패턴
        m.set_weights(PositionKey::LCM, &[
            (PositionKey::RCM, 1.1),
            (PositionKey::LM, 1.2),
            (PositionKey::CDM, 1.0),
            (PositionKey::CAM, 1.3),
            (PositionKey::ST, 1.1),
        ]);

        m.set_weights(PositionKey::RCM, &[
            (PositionKey::LCM, 1.1),
            (PositionKey::RM, 1.2),
            (PositionKey::CDM, 1.0),
            (PositionKey::CAM, 1.3),
            (PositionKey::ST, 1.1),
        ]);

        // 측면 미드필더 패턴
        m.set_weights(PositionKey::LM, &[
            (PositionKey::LCM, 1.1),
            (PositionKey::CAM, 1.2),
            (PositionKey::ST, 1.3),
            (PositionKey::LB, 0.9),
        ]);

        m.set_weights(PositionKey::RM, &[
            (PositionKey::RCM, 1.1),
            (PositionKey::CAM, 1.2),
            (PositionKey::ST, 1.3),
            (PositionKey::RB, 0.9),
        ]);

        // CAM 패턴 (공격 연결)
        m.set_weights(PositionKey::CAM, &[
            (PositionKey::ST, 1.4),
            (PositionKey::LM, 1.1),
            (PositionKey::RM, 1.1),
            (PositionKey::LCM, 0.9),
            (PositionKey::RCM, 0.9),
        ]);

        // ST 패턴 (원톱)
        m.set_weights(PositionKey::ST, &[
            (PositionKey::CAM, 1.0),
            (PositionKey::LM, 0.9),
            (PositionKey::RM, 0.9),
        ]);

        m
    }

    // =========================================================================
    // 3-4-3 Formation (Attacking)
    // =========================================================================

    /// 3-4-3 Balanced 매트릭스 (공격적)
    pub fn new_343_balanced() -> Self {
        let mut m = Self::empty("3-4-3", TacticalStyle::Counter);

        // GK 패턴
        m.set_weights(PositionKey::GK, &[
            (PositionKey::CB, 1.2),
            (PositionKey::LCB, 1.1),
            (PositionKey::RCB, 1.1),
        ]);

        // 3백 패턴
        m.set_weights(PositionKey::LCB, &[
            (PositionKey::CB, 1.0),
            (PositionKey::LM, 1.2),
            (PositionKey::CDM, 1.1),
            (PositionKey::LW, 1.0),
        ]);

        m.set_weights(PositionKey::CB, &[
            (PositionKey::LCB, 1.0),
            (PositionKey::RCB, 1.0),
            (PositionKey::CDM, 1.2),
            (PositionKey::LCM, 1.0),
            (PositionKey::RCM, 1.0),
        ]);

        m.set_weights(PositionKey::RCB, &[
            (PositionKey::CB, 1.0),
            (PositionKey::RM, 1.2),
            (PositionKey::CDM, 1.1),
            (PositionKey::RW, 1.0),
        ]);

        // 4미드 패턴
        m.set_weights(PositionKey::CDM, &[
            (PositionKey::LCM, 1.2),
            (PositionKey::RCM, 1.2),
            (PositionKey::CB, 0.9),
            (PositionKey::CAM, 1.1),
        ]);

        m.set_weights(PositionKey::LM, &[
            (PositionKey::LCM, 1.1),
            (PositionKey::LW, 1.4),
            (PositionKey::ST, 1.2),
            (PositionKey::LCB, 0.8),
        ]);

        m.set_weights(PositionKey::RM, &[
            (PositionKey::RCM, 1.1),
            (PositionKey::RW, 1.4),
            (PositionKey::ST, 1.2),
            (PositionKey::RCB, 0.8),
        ]);

        m.set_weights(PositionKey::LCM, &[
            (PositionKey::RCM, 1.0),
            (PositionKey::LM, 1.1),
            (PositionKey::CAM, 1.2),
            (PositionKey::CDM, 0.9),
            (PositionKey::LW, 1.1),
        ]);

        m.set_weights(PositionKey::RCM, &[
            (PositionKey::LCM, 1.0),
            (PositionKey::RM, 1.1),
            (PositionKey::CAM, 1.2),
            (PositionKey::CDM, 0.9),
            (PositionKey::RW, 1.1),
        ]);

        m.set_weights(PositionKey::CAM, &[
            (PositionKey::ST, 1.4),
            (PositionKey::LW, 1.2),
            (PositionKey::RW, 1.2),
            (PositionKey::LCM, 0.9),
            (PositionKey::RCM, 0.9),
        ]);

        // 3톱 패턴
        m.set_weights(PositionKey::LW, &[
            (PositionKey::ST, 1.4),
            (PositionKey::CAM, 1.1),
            (PositionKey::LM, 0.9),
            (PositionKey::RW, 0.8),
        ]);

        m.set_weights(PositionKey::ST, &[
            (PositionKey::LW, 1.1),
            (PositionKey::RW, 1.1),
            (PositionKey::CAM, 1.0),
        ]);

        m.set_weights(PositionKey::RW, &[
            (PositionKey::ST, 1.4),
            (PositionKey::CAM, 1.1),
            (PositionKey::RM, 0.9),
            (PositionKey::LW, 0.8),
        ]);

        m
    }

    // =========================================================================
    // Direct Formation + Style Factory (for TacticalAdjuster)
    // =========================================================================

    /// 포메이션과 스타일로 직접 생성 (TacticalAdjuster용)
    pub fn from_formation_and_style(formation: &str, style: TacticalStyle) -> Self {
        match (formation, style) {
            // 4-4-2
            ("4-4-2" | "442", TacticalStyle::WingPlay) => Self::new_442_wingplay(),
            ("4-4-2" | "442", TacticalStyle::Possession) => Self::new_442_possession(),
            ("4-4-2" | "442", TacticalStyle::Counter) => Self::new_442_counter(),
            ("4-4-2" | "442", _) => Self::new_442_balanced(),
            // 4-3-3
            ("4-3-3" | "433", TacticalStyle::WingPlay) => Self::new_433_wingplay(),
            ("4-3-3" | "433", _) => Self::new_433_balanced(),
            // 3-5-2
            ("3-5-2" | "352", _) => Self::new_352_balanced(),
            // 4-2-3-1
            ("4-2-3-1" | "4231", _) => Self::new_4231_balanced(),
            // 4-5-1
            ("4-5-1" | "451", _) => Self::new_451_balanced(),
            // 3-4-3
            ("3-4-3" | "343", _) => Self::new_343_balanced(),
            // 기본값
            _ => Self::new_442_balanced(),
        }
    }

    // =========================================================================
    // TeamInstructions Integration
    // =========================================================================

    /// TeamInstructions에서 전술 스타일 추출
    /// - VeryWide team_width → WingPlay
    /// - Direct build_up → Counter
    /// - Short build_up → Possession
    /// - 그 외 → Balanced
    pub fn style_from_instructions(
        instructions: &crate::tactics::team_instructions::TeamInstructions,
    ) -> TacticalStyle {
        use crate::tactics::team_instructions::{BuildUpStyle, TeamWidth};

        // 넓은 포메이션이면 WingPlay 우선
        if matches!(instructions.team_width, TeamWidth::VeryWide | TeamWidth::Wide) {
            return TacticalStyle::WingPlay;
        }

        match instructions.build_up_style {
            BuildUpStyle::Short => TacticalStyle::Possession,
            BuildUpStyle::Direct => TacticalStyle::Counter,
            BuildUpStyle::Mixed => TacticalStyle::Balanced,
        }
    }

    /// 포메이션과 TeamInstructions로 매트릭스 생성
    pub fn from_formation_and_instructions(
        formation: &str,
        instructions: Option<&crate::tactics::team_instructions::TeamInstructions>,
    ) -> Self {
        let style = instructions
            .map(Self::style_from_instructions)
            .unwrap_or_default();

        // from_formation_and_style 호출하여 중복 제거
        Self::from_formation_and_style(formation, style)
    }
}

// =============================================================================
// Player Attribute Adjustments
// =============================================================================

/// 롱패스인지 판정 (수비→공격)
fn is_long_pass(from: PositionKey, to: PositionKey) -> bool {
    let from_line = position_line(from);
    let to_line = position_line(to);

    // 2라인 이상 건너뛰기 (DF→FW)
    to_line.saturating_sub(from_line) >= 2
}

/// 사이드 체인지인지 판정 (좌→우 또는 우→좌)
fn is_switch_play(from: PositionKey, to: PositionKey) -> bool {
    let from_side = position_side(from);
    let to_side = position_side(to);

    // 왼쪽→오른쪽 또는 오른쪽→왼쪽
    (from_side == -1 && to_side == 1) || (from_side == 1 && to_side == -1)
}

/// 포지션의 라인 (0=GK, 1=DF, 2=MF, 3=FW)
fn position_line(key: PositionKey) -> u8 {
    match key {
        PositionKey::GK => 0,
        PositionKey::LB | PositionKey::LCB | PositionKey::CB | PositionKey::RCB | PositionKey::RB
        | PositionKey::LWB | PositionKey::RWB => 1,
        PositionKey::CDM | PositionKey::LDM | PositionKey::RDM | PositionKey::LM
        | PositionKey::LCM | PositionKey::CM | PositionKey::RCM | PositionKey::RM
        | PositionKey::LAM | PositionKey::CAM | PositionKey::RAM => 2,
        PositionKey::LW | PositionKey::RW | PositionKey::LF | PositionKey::CF | PositionKey::RF
        | PositionKey::ST => 3,
    }
}

/// 포지션의 측면 (-1=왼쪽, 0=중앙, 1=오른쪽)
fn position_side(key: PositionKey) -> i8 {
    match key {
        PositionKey::LB | PositionKey::LCB | PositionKey::LWB | PositionKey::LDM
        | PositionKey::LM | PositionKey::LCM | PositionKey::LAM | PositionKey::LW
        | PositionKey::LF => -1,
        PositionKey::RB | PositionKey::RCB | PositionKey::RWB | PositionKey::RDM
        | PositionKey::RM | PositionKey::RCM | PositionKey::RAM | PositionKey::RW
        | PositionKey::RF => 1,
        _ => 0,
    }
}

/// 선수 특성을 반영한 역할 가중치 조정
///
/// # Arguments
/// * `base_weight` - 기본 역할 가중치
/// * `holder_attrs` - 볼 홀더 능력치
/// * `target_attrs` - 패스 타겟 능력치
/// * `holder_role` - 볼 홀더 역할
/// * `target_role` - 패스 타겟 역할
///
/// # Returns
/// 조정된 가중치 (0.5 ~ 1.5)
pub fn get_adjusted_role_weight(
    base_weight: f32,
    holder_attrs: Option<&PlayerAttributes>,
    target_attrs: Option<&PlayerAttributes>,
    holder_role: PositionKey,
    target_role: PositionKey,
) -> f32 {
    let mut weight = base_weight;

    if let Some(attrs) = holder_attrs {
        // 롱볼 능력 보정 (CB/CDM → ST 등)
        if is_long_pass(holder_role, target_role) {
            // long_passing 속성이 있으면 사용, 없으면 passing 사용
            let long_passing = attrs.passing as f32 / 100.0;
            weight *= 0.8 + long_passing * 0.4; // 80%~120%
        }

        // 비전 보정 (사이드 체인지)
        if is_switch_play(holder_role, target_role) {
            let vision = attrs.vision as f32 / 100.0;
            weight *= 0.7 + vision * 0.6; // 70%~130%
        }
    }

    if let Some(attrs) = target_attrs {
        // 타겟 오프더볼 보정
        let off_the_ball = attrs.off_the_ball as f32 / 100.0;
        weight *= 0.9 + off_the_ball * 0.2; // 90%~110%
    }

    weight.clamp(MIN_ROLE_WEIGHT, MAX_ROLE_WEIGHT)
}

/// 기존 점수에 역할 가중치 적용 (가산 후 정규화)
///
/// # Arguments
/// * `base_score` - 기존 패스 타겟 점수 (0.0 ~ 1.0)
/// * `role_weight` - 역할 가중치 (0.5 ~ 1.5)
///
/// # Returns
/// 최종 점수 (0.0 ~ 1.0)
pub fn apply_role_weight_to_score(base_score: f32, role_weight: f32) -> f32 {
    // 가산 방식: score + (weight - 1.0) * ROLE_INFLUENCE
    // weight = 1.5 → +0.15점
    // weight = 0.5 → -0.15점
    let combined = base_score + (role_weight - NEUTRAL_WEIGHT) * ROLE_INFLUENCE;
    combined.clamp(0.0, 1.0)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_442_balanced_weights() {
        let m = RoleTransitionMatrix::new_442_balanced();

        // CM → ST는 높은 가중치
        let cm_to_lf = m.get_weight(PositionKey::LCM, PositionKey::LF);
        assert!(cm_to_lf > NEUTRAL_WEIGHT, "CM→LF should be preferred");

        // CB → CB 횡패스는 중립
        let cb_to_cb = m.get_weight(PositionKey::LCB, PositionKey::RCB);
        assert!(cb_to_cb >= NEUTRAL_WEIGHT, "CB→CB should be neutral or preferred");

        // CB → ST 롱볼은 낮은 가중치
        let cb_to_st = m.get_weight(PositionKey::LCB, PositionKey::LF);
        assert!(cb_to_st < NEUTRAL_WEIGHT, "CB→ST should not be preferred");
    }

    #[test]
    fn test_possession_style_backpass_boost() {
        let balanced = RoleTransitionMatrix::new_442_balanced();
        let possession = RoleTransitionMatrix::new_442_possession();

        // Possession 스타일에서는 백패스 가중치가 높아야 함
        let balanced_backpass = balanced.get_weight(PositionKey::LCM, PositionKey::LCB);
        let possession_backpass = possession.get_weight(PositionKey::LCM, PositionKey::LCB);

        assert!(
            possession_backpass > balanced_backpass,
            "Possession style should prefer backpass"
        );
    }

    #[test]
    fn test_counter_style_longball_boost() {
        let balanced = RoleTransitionMatrix::new_442_balanced();
        let counter = RoleTransitionMatrix::new_442_counter();

        // Counter 스타일에서는 롱볼 가중치가 높아야 함
        let balanced_longball = balanced.get_weight(PositionKey::LCB, PositionKey::LF);
        let counter_longball = counter.get_weight(PositionKey::LCB, PositionKey::LF);

        assert!(
            counter_longball > balanced_longball,
            "Counter style should prefer longball"
        );
    }

    #[test]
    fn test_apply_role_weight() {
        let base_score = 0.5;

        // 높은 가중치 (1.5) → 점수 증가
        let high_weight_score = apply_role_weight_to_score(base_score, 1.5);
        assert!(high_weight_score > base_score);

        // 낮은 가중치 (0.5) → 점수 감소
        let low_weight_score = apply_role_weight_to_score(base_score, 0.5);
        assert!(low_weight_score < base_score);

        // 중립 가중치 (1.0) → 점수 유지
        let neutral_score = apply_role_weight_to_score(base_score, 1.0);
        assert!((neutral_score - base_score).abs() < 0.001);
    }

    #[test]
    fn test_is_long_pass() {
        // DF → FW = 롱패스
        assert!(is_long_pass(PositionKey::LCB, PositionKey::ST));
        assert!(is_long_pass(PositionKey::RCB, PositionKey::LW));

        // MF → FW = 롱패스 아님 (1라인 차이)
        assert!(!is_long_pass(PositionKey::CM, PositionKey::ST));

        // DF → MF = 롱패스 아님
        assert!(!is_long_pass(PositionKey::LCB, PositionKey::CM));
    }

    #[test]
    fn test_is_switch_play() {
        // 좌→우 = 사이드 체인지
        assert!(is_switch_play(PositionKey::LB, PositionKey::RW));
        assert!(is_switch_play(PositionKey::LM, PositionKey::RM));

        // 좌→중앙 = 사이드 체인지 아님
        assert!(!is_switch_play(PositionKey::LB, PositionKey::CM));

        // 중앙→우 = 사이드 체인지 아님
        assert!(!is_switch_play(PositionKey::CM, PositionKey::RW));
    }

    #[test]
    fn test_adjusted_role_weight() {
        let base_weight = 1.0;

        // 속성 없으면 기본 가중치 유지
        let no_attrs = get_adjusted_role_weight(
            base_weight,
            None,
            None,
            PositionKey::CM,
            PositionKey::ST,
        );
        assert!((no_attrs - base_weight).abs() < 0.001);
    }

    #[test]
    fn test_352_balanced_weights() {
        let m = RoleTransitionMatrix::new_352_balanced();

        // 윙백 → 스트라이커 크로스 선호
        let lwb_to_lf = m.get_weight(PositionKey::LWB, PositionKey::LF);
        assert!(lwb_to_lf > NEUTRAL_WEIGHT, "LWB→LF should be preferred: {}", lwb_to_lf);

        // 투톱 연계
        let lf_to_rf = m.get_weight(PositionKey::LF, PositionKey::RF);
        assert!(lf_to_rf > NEUTRAL_WEIGHT, "LF→RF partnership should be preferred: {}", lf_to_rf);

        // CB → CM 선호
        let cb_to_cm = m.get_weight(PositionKey::CB, PositionKey::CM);
        assert!(cb_to_cm > NEUTRAL_WEIGHT, "CB→CM should be preferred: {}", cb_to_cm);
    }

    #[test]
    fn test_4231_balanced_weights() {
        let m = RoleTransitionMatrix::new_4231_balanced();

        // CAM → ST 강력 선호
        let cam_to_st = m.get_weight(PositionKey::CAM, PositionKey::ST);
        assert!(cam_to_st >= 1.5, "CAM→ST should be strongly preferred: {}", cam_to_st);

        // 더블 피벗 → CAM 선호
        let ldm_to_cam = m.get_weight(PositionKey::LDM, PositionKey::CAM);
        assert!(ldm_to_cam > NEUTRAL_WEIGHT, "LDM→CAM should be preferred: {}", ldm_to_cam);

        // ST → CAM 레이오프
        let st_to_cam = m.get_weight(PositionKey::ST, PositionKey::CAM);
        assert!(st_to_cam > NEUTRAL_WEIGHT, "ST→CAM layoff should be preferred: {}", st_to_cam);
    }

    #[test]
    fn test_formation_selection() {
        // 3-5-2 포메이션 선택
        let m352 = RoleTransitionMatrix::from_formation_and_instructions("3-5-2", None);
        assert_eq!(m352.formation, "3-5-2");

        let m352_alt = RoleTransitionMatrix::from_formation_and_instructions("352", None);
        assert_eq!(m352_alt.formation, "3-5-2");

        // 4-2-3-1 포메이션 선택
        let m4231 = RoleTransitionMatrix::from_formation_and_instructions("4-2-3-1", None);
        assert_eq!(m4231.formation, "4-2-3-1");

        let m4231_alt = RoleTransitionMatrix::from_formation_and_instructions("4231", None);
        assert_eq!(m4231_alt.formation, "4-2-3-1");
    }

    #[test]
    fn test_wingplay_style_weights() {
        let m442_wing = RoleTransitionMatrix::new_442_wingplay();
        let m433_wing = RoleTransitionMatrix::new_433_wingplay();

        // 4-4-2 WingPlay: LB → LM 매우 선호
        let lb_to_lm = m442_wing.get_weight(PositionKey::LB, PositionKey::LM);
        assert!(lb_to_lm >= 1.5, "442 WingPlay LB→LM should be very preferred: {}", lb_to_lm);

        // 4-4-2 WingPlay: LM → LF 크로스 매우 선호
        let lm_to_lf = m442_wing.get_weight(PositionKey::LM, PositionKey::LF);
        assert!(lm_to_lf >= 1.5, "442 WingPlay LM→LF cross should be very preferred: {}", lm_to_lf);

        // 4-3-3 WingPlay: LB → LW 매우 선호
        let lb_to_lw = m433_wing.get_weight(PositionKey::LB, PositionKey::LW);
        assert!(lb_to_lw >= 1.5, "433 WingPlay LB→LW should be very preferred: {}", lb_to_lw);

        // 4-3-3 WingPlay: LW → ST 크로스 매우 선호
        let lw_to_st = m433_wing.get_weight(PositionKey::LW, PositionKey::ST);
        assert!(lw_to_st >= 1.5, "433 WingPlay LW→ST cross should be very preferred: {}", lw_to_st);
    }

    #[test]
    fn test_wingplay_triggered_by_wide_width() {
        use crate::tactics::team_instructions::{TeamInstructions, TeamWidth};

        // VeryWide → WingPlay
        let mut instructions = TeamInstructions::default();
        instructions.team_width = TeamWidth::VeryWide;
        let style = RoleTransitionMatrix::style_from_instructions(&instructions);
        assert_eq!(style, TacticalStyle::WingPlay);

        // Wide → WingPlay
        instructions.team_width = TeamWidth::Wide;
        let style = RoleTransitionMatrix::style_from_instructions(&instructions);
        assert_eq!(style, TacticalStyle::WingPlay);

        // Normal → Balanced (default)
        instructions.team_width = TeamWidth::Normal;
        let style = RoleTransitionMatrix::style_from_instructions(&instructions);
        assert_eq!(style, TacticalStyle::Balanced);
    }
}
