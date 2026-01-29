//! FIX_2601/1123: ActionDetailV2 - 완전성이 타입으로 보장되는 액션 상세
//!
//! 이 모듈의 모든 타입은 Option 필드 없이 필수 값만 포함한다.
//! Conversion 단계에서 RNG fallback이 구조적으로 불가능해진다.
//!
//! ## 설계 원칙
//!
//! 1. **Option 필드 금지**: 필수 필드는 타입으로 강제
//! 2. **액션별 분리**: 각 액션이 필요로 하는 필드만 포함
//! 3. **Intent 분리**: Builder 입력용 타입은 Option 허용 (Builder가 채움)
//!
//! ## 사용 흐름
//!
//! ```text
//! select_best_action_with_detail()
//!     → Intent (Option 포함)
//!     → Builder (deterministic 채움)
//!     → ActionDetailV2 (Option 없음)
//!     → Conversion (RNG 없음)
//!     → ActionType
//! ```

use serde::{Deserialize, Serialize};

// ============================================================================
// Main Enum
// ============================================================================

/// 액션별 완전한 상세 정보
///
/// 모든 variant의 필수 필드는 None이 될 수 없다.
/// Conversion 단계에서 RNG fallback이 구조적으로 불가능.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionDetailV2 {
    Pass(PassDetail),
    Shot(ShotDetail),
    Dribble(DribbleDetail),
    Tackle(TackleDetail),
    Header(HeaderDetail),
    Cross(CrossDetail),
    Clearance(ClearanceDetail),
    Intercept(InterceptDetail),
    Hold(HoldDetail),
}

// ============================================================================
// Pass
// ============================================================================

/// Pass 액션 상세 (Short/Through/Long/Lob 포함)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassDetail {
    /// 필수: 패스 대상 선수 track_id (0..21)
    pub target_track_id: u8,
    /// 필수: 패스 종류
    pub pass_kind: PassKind,
    /// 필수: 패스 파워 (0.0..1.0 정규화)
    pub power: f32,
    /// 선택: 의도된 도착 지점 (있으면 사용, 없으면 target 위치로 계산)
    pub intended_point: Option<(f32, f32)>,
    /// FIX_2601/1129: 선택 시점의 패서 위치 (forward_pass_rate 측정용)
    pub intended_passer_pos: Option<(f32, f32)>,
}

/// 패스 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PassKind {
    /// 짧은 지상 패스
    Short,
    /// 수비 라인 뒤로 보내는 스루 패스
    Through,
    /// 긴 지상/공중 패스
    Long,
    /// 로브 패스 (높은 포물선)
    Lob,
}

impl Default for PassKind {
    fn default() -> Self {
        PassKind::Short
    }
}

// ============================================================================
// Shot
// ============================================================================

/// Shot 액션 상세
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShotDetail {
    /// 필수: 슛 목표 지점 (정규화 좌표, 골문 기준)
    pub target_point: (f32, f32),
    /// 필수: 슛 파워 (0.0..1.0 정규화)
    pub power: f32,
    /// 필수: 슛 종류
    pub shot_kind: ShotKind,
}

/// 슛 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShotKind {
    /// 일반 슛
    Normal,
    /// 커브를 건 피네스 슛
    Finesse,
    /// 골키퍼 넘기는 칩 슛
    Chip,
    /// 강슛
    Power,
}

impl Default for ShotKind {
    fn default() -> Self {
        ShotKind::Normal
    }
}

// ============================================================================
// Dribble
// ============================================================================

/// Dribble 액션 상세
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DribbleDetail {
    /// 필수: 드리블 방향 (normalized, 길이 1.0)
    pub direction: (f32, f32),
    /// 필수: 드리블 속도 계수 (0.0..1.0)
    pub speed_factor: f32,
}

// ============================================================================
// Tackle
// ============================================================================

/// Tackle 액션 상세
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TackleDetail {
    /// 필수: 태클 대상 선수 track_id
    pub target_track_id: u8,
    /// 필수: 태클 종류
    pub tackle_kind: TackleKind,
}

/// 태클 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TackleKind {
    /// 서서 하는 태클
    Standing,
    /// 슬라이딩 태클
    Sliding,
    /// 어깨 태클 (밀어내기)
    Shoulder,
}

impl Default for TackleKind {
    fn default() -> Self {
        TackleKind::Standing
    }
}

// ============================================================================
// Header
// ============================================================================

/// Header 액션 상세
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDetail {
    /// 필수: 헤더 목표
    pub target: HeaderTarget,
    /// 필수: 헤더 파워 (0.0..1.0)
    pub power: f32,
}

/// 헤더 목표 종류
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeaderTarget {
    /// 슛 헤더: 골문 목표 지점
    Shot { point: (f32, f32) },
    /// 패스 헤더: 대상 선수
    Pass { target_track_id: u8 },
    /// 클리어 헤더: 방향
    Clear { direction: (f32, f32) },
}

// ============================================================================
// Cross
// ============================================================================

/// Cross 액션 상세
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDetail {
    /// 필수: 크로스 목표 지점 (정규화 좌표)
    pub target_point: (f32, f32),
    /// 필수: 크로스 종류
    pub cross_kind: CrossKind,
    /// 필수: 크로스 파워 (0.0..1.0)
    pub power: f32,
}

/// 크로스 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrossKind {
    /// 낮은 크로스 (땅볼/낮은 공중볼)
    Low,
    /// 높은 크로스 (일반적인 공중볼)
    High,
    /// 빠르고 낮은 드리븐 크로스
    Driven,
    /// 강하게 휘어지는 크로스
    Whipped,
}

impl Default for CrossKind {
    fn default() -> Self {
        CrossKind::High
    }
}

// ============================================================================
// Clearance
// ============================================================================

/// Clearance 액션 상세
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearanceDetail {
    /// 필수: 클리어 방향 (normalized)
    pub direction: (f32, f32),
    /// 필수: 클리어 파워 (0.0..1.0)
    pub power: f32,
}

// ============================================================================
// Intercept
// ============================================================================

/// Intercept 액션 상세
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptDetail {
    /// 필수: 인터셉트 시도 위치 (정규화 좌표)
    pub intercept_point: (f32, f32),
}

// ============================================================================
// Hold
// ============================================================================

/// Hold 액션 상세 (볼 보유/쉴딩)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldDetail {
    /// 필수: 보호 방향 (상대방 반대쪽)
    pub shield_direction: (f32, f32),
}

// ============================================================================
// Intent Types (Builder 입력용)
// ============================================================================

/// Pass 의도 - Builder 입력용 (일부 필드는 Builder가 채움)
#[derive(Debug, Clone, Default)]
pub struct PassIntent {
    /// 의도한 패스 대상 (None이면 Builder가 선택)
    pub intended_target: Option<usize>,
    /// 패스 종류
    pub pass_kind: PassKind,
    /// 패스 파워 (None이면 Builder가 계산)
    pub power: Option<f32>,
    /// 의도한 도착 지점
    pub intended_point: Option<(f32, f32)>,
}

/// Shot 의도 - Builder 입력용
#[derive(Debug, Clone, Default)]
pub struct ShotIntent {
    /// 슛 목표 지점 (None이면 Builder가 계산)
    pub target_point: Option<(f32, f32)>,
    /// 슛 파워 (None이면 Builder가 계산)
    pub power: Option<f32>,
    /// 슛 종류 (None이면 Normal)
    pub shot_kind: Option<ShotKind>,
}

/// Dribble 의도 - Builder 입력용
#[derive(Debug, Clone, Default)]
pub struct DribbleIntent {
    /// 드리블 방향 (None이면 Builder가 계산)
    pub direction: Option<(f32, f32)>,
    /// 속도 계수 (None이면 Builder가 계산)
    pub speed_factor: Option<f32>,
}

/// Tackle 의도 - Builder 입력용
#[derive(Debug, Clone, Default)]
pub struct TackleIntent {
    /// 태클 대상 (None이면 Builder가 선택)
    pub target: Option<u8>,
    /// 태클 종류 (None이면 Standing)
    pub tackle_kind: Option<TackleKind>,
}

/// Header 의도 - Builder 입력용
#[derive(Debug, Clone)]
pub struct HeaderIntent {
    /// 헤더 목표 타입
    pub target_type: HeaderTargetType,
    /// 슛 헤더인 경우 목표 지점
    pub shot_point: Option<(f32, f32)>,
    /// 패스 헤더인 경우 대상
    pub pass_target: Option<u8>,
    /// 클리어 헤더인 경우 방향
    pub clear_direction: Option<(f32, f32)>,
    /// 헤더 파워
    pub power: Option<f32>,
}

/// 헤더 목표 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderTargetType {
    Shot,
    Pass,
    Clear,
}

impl Default for HeaderIntent {
    fn default() -> Self {
        Self {
            target_type: HeaderTargetType::Clear,
            shot_point: None,
            pass_target: None,
            clear_direction: None,
            power: None,
        }
    }
}

/// Cross 의도 - Builder 입력용
#[derive(Debug, Clone, Default)]
pub struct CrossIntent {
    /// 크로스 목표 지점
    pub target_point: Option<(f32, f32)>,
    /// 크로스 종류
    pub cross_kind: Option<CrossKind>,
    /// 크로스 파워
    pub power: Option<f32>,
}

/// Clearance 의도 - Builder 입력용
#[derive(Debug, Clone, Default)]
pub struct ClearanceIntent {
    /// 클리어 방향
    pub direction: Option<(f32, f32)>,
    /// 클리어 파워
    pub power: Option<f32>,
}

// ============================================================================
// Helper Implementations
// ============================================================================

impl ActionDetailV2 {
    /// 액션 종류 이름 반환 (디버깅용)
    pub fn kind_name(&self) -> &'static str {
        match self {
            ActionDetailV2::Pass(_) => "Pass",
            ActionDetailV2::Shot(_) => "Shot",
            ActionDetailV2::Dribble(_) => "Dribble",
            ActionDetailV2::Tackle(_) => "Tackle",
            ActionDetailV2::Header(_) => "Header",
            ActionDetailV2::Cross(_) => "Cross",
            ActionDetailV2::Clearance(_) => "Clearance",
            ActionDetailV2::Intercept(_) => "Intercept",
            ActionDetailV2::Hold(_) => "Hold",
        }
    }
}

impl PassDetail {
    /// 새 PassDetail 생성
    pub fn new(target_track_id: u8, pass_kind: PassKind, power: f32) -> Self {
        Self {
            target_track_id,
            pass_kind,
            power,
            intended_point: None,
            intended_passer_pos: None,
        }
    }
}

impl ShotDetail {
    /// 새 ShotDetail 생성
    pub fn new(target_point: (f32, f32), power: f32, shot_kind: ShotKind) -> Self {
        Self {
            target_point,
            power,
            shot_kind,
        }
    }
}

impl DribbleDetail {
    /// 새 DribbleDetail 생성
    pub fn new(direction: (f32, f32), speed_factor: f32) -> Self {
        Self {
            direction,
            speed_factor,
        }
    }
}

impl TackleDetail {
    /// 새 TackleDetail 생성
    pub fn new(target_track_id: u8, tackle_kind: TackleKind) -> Self {
        Self {
            target_track_id,
            tackle_kind,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pass_detail_creation() {
        let detail = PassDetail::new(5, PassKind::Through, 0.7);
        assert_eq!(detail.target_track_id, 5);
        assert_eq!(detail.pass_kind, PassKind::Through);
        assert!((detail.power - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_shot_detail_creation() {
        let detail = ShotDetail::new((1.0, 0.5), 0.9, ShotKind::Finesse);
        assert_eq!(detail.target_point, (1.0, 0.5));
        assert_eq!(detail.shot_kind, ShotKind::Finesse);
    }

    #[test]
    fn test_action_detail_v2_kind_name() {
        let pass = ActionDetailV2::Pass(PassDetail::new(3, PassKind::Short, 0.5));
        assert_eq!(pass.kind_name(), "Pass");

        let shot = ActionDetailV2::Shot(ShotDetail::new((1.0, 0.5), 0.8, ShotKind::Normal));
        assert_eq!(shot.kind_name(), "Shot");
    }

    #[test]
    fn test_intent_defaults() {
        let pass_intent = PassIntent::default();
        assert!(pass_intent.intended_target.is_none());
        assert_eq!(pass_intent.pass_kind, PassKind::Short);

        let shot_intent = ShotIntent::default();
        assert!(shot_intent.target_point.is_none());
        assert!(shot_intent.shot_kind.is_none());
    }

    #[test]
    fn test_header_target_variants() {
        let shot_header = HeaderTarget::Shot { point: (1.0, 0.5) };
        let pass_header = HeaderTarget::Pass { target_track_id: 7 };
        let clear_header = HeaderTarget::Clear { direction: (0.0, 1.0) };

        // Just verify they compile and can be matched
        match shot_header {
            HeaderTarget::Shot { point } => assert_eq!(point, (1.0, 0.5)),
            _ => panic!("Wrong variant"),
        }
        match pass_header {
            HeaderTarget::Pass { target_track_id } => assert_eq!(target_track_id, 7),
            _ => panic!("Wrong variant"),
        }
        match clear_header {
            HeaderTarget::Clear { direction } => assert_eq!(direction, (0.0, 1.0)),
            _ => panic!("Wrong variant"),
        }
    }
}
