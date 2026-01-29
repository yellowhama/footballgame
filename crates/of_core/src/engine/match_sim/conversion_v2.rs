//! FIX_2601/1123: V2 Conversion Module
//!
//! ActionDetailV2를 ActionType으로 변환한다.
//! 이 모듈의 모든 함수는 RNG를 사용하지 않는다.
//!
//! ## 설계 원칙
//!
//! 1. **RNG 완전 배제**: 모든 값은 ActionDetailV2에서 직접 추출
//! 2. **Fallback 없음**: ActionDetailV2는 이미 완전하므로 보정이 필요 없음
//! 3. **순수 변환**: 입력 → 출력, 부수 효과 없음

use super::action_detail_v2::*;
use crate::engine::action_queue::ActionType;
use crate::engine::types::Coord10;

// ============================================================================
// Main Conversion Function
// ============================================================================

/// ActionDetailV2를 ActionType으로 변환 (RNG 없음)
///
/// ActionDetailV2는 Builder에서 이미 완전한 값으로 채워져 있으므로
/// 단순히 필드를 매핑하기만 한다.
///
/// # Arguments
/// * `detail` - 완전한 액션 상세
/// * `attacks_right` - 이 팀이 오른쪽을 공격하는지 (방향 변환용)
///
/// # Returns
/// 변환된 ActionType
pub fn convert_detail_v2_to_action_type(
    detail: &ActionDetailV2,
    attacks_right: bool,
) -> ActionType {
    match detail {
        ActionDetailV2::Pass(d) => convert_pass(d),
        ActionDetailV2::Shot(d) => convert_shot(d, attacks_right),
        ActionDetailV2::Dribble(d) => convert_dribble(d),
        ActionDetailV2::Tackle(d) => convert_tackle(d),
        ActionDetailV2::Header(d) => convert_header(d, attacks_right),
        ActionDetailV2::Cross(d) => convert_cross(d),
        ActionDetailV2::Clearance(d) => convert_clearance(d, attacks_right),
        ActionDetailV2::Intercept(d) => convert_intercept(d),
        ActionDetailV2::Hold(_) => convert_hold(),
    }
}

// ============================================================================
// Individual Converters
// ============================================================================

fn convert_pass(d: &PassDetail) -> ActionType {
    let is_long = matches!(d.pass_kind, PassKind::Long | PassKind::Lob);
    let is_through = matches!(d.pass_kind, PassKind::Through);

    // FIX_2601/1128: intended_point를 Coord10으로 변환 (forward_pass_rate 측정용)
    // intended_point는 이미 미터 단위 (ev_decision.rs:2393 참조)이므로 직접 변환
    let intended_target_pos = d.intended_point.map(|point| {
        Coord10::from_meters(point.0, point.1)
    });

    // FIX_2601/1129: intended_passer_pos를 Coord10으로 변환 (forward_pass_rate 측정용)
    let intended_passer_pos = d.intended_passer_pos.map(|point| {
        Coord10::from_meters(point.0, point.1)
    });

    ActionType::Pass {
        target_idx: d.target_track_id as usize,
        is_long,
        is_through,
        intended_target_pos,
        intended_passer_pos,
    }
}

fn convert_shot(d: &ShotDetail, _attacks_right: bool) -> ActionType {
    // 정규화 좌표를 Coord10으로 변환
    // 정규화: (0,0) = 왼쪽 상단, (1,1) = 오른쪽 하단
    // Coord10: 미터 단위 (105m x 68m)
    let target_meters = normalized_to_meters(d.target_point);
    let target = Coord10::from_meters(target_meters.0, target_meters.1);

    ActionType::Shot {
        power: d.power,
        target,
    }
}

fn convert_dribble(d: &DribbleDetail) -> ActionType {
    let direction = d.direction;

    // speed_factor > 0.7이면 aggressive로 간주
    let aggressive = d.speed_factor > 0.7;

    ActionType::Dribble {
        direction,
        aggressive,
    }
}

fn convert_tackle(d: &TackleDetail) -> ActionType {
    ActionType::Tackle {
        target_idx: d.target_track_id as usize,
    }
}

fn convert_header(d: &HeaderDetail, _attacks_right: bool) -> ActionType {
    // HeaderTarget에 따라 is_shot 결정
    let (target, is_shot) = match &d.target {
        HeaderTarget::Shot { point } => {
            let meters = normalized_to_meters(*point);
            (Coord10::from_meters(meters.0, meters.1), true)
        }
        HeaderTarget::Pass { target_track_id: _ } => {
            // 패스 헤더: 필드 중앙을 임시 타겟으로 (resolve에서 실제 처리)
            (Coord10::from_meters(52.5, 34.0), false)
        }
        HeaderTarget::Clear { direction } => {
            // 클리어 방향을 목표 지점으로 변환
            let target = direction_to_target(*direction);
            (target, false)
        }
    };

    ActionType::Header { target, is_shot }
}

fn convert_cross(d: &CrossDetail) -> ActionType {
    // 크로스는 Pass의 특수 형태로 처리
    // CrossKind에 따른 is_long 결정
    let is_long = matches!(d.cross_kind, CrossKind::High | CrossKind::Whipped);

    ActionType::Pass {
        target_idx: 0, // 크로스는 특정 타겟 없이 지역으로 보냄 - resolve에서 처리
        is_long,
        is_through: false,
        intended_target_pos: None, // 크로스는 지역 타겟
        intended_passer_pos: None,
    }
}

fn convert_clearance(d: &ClearanceDetail, _attacks_right: bool) -> ActionType {
    // 클리어는 헤더(is_shot=false)로 처리
    let target = direction_to_target(d.direction);

    ActionType::Header {
        target,
        is_shot: false,
    }
}

fn convert_intercept(d: &InterceptDetail) -> ActionType {
    // 인터셉트 지점을 Coord10으로 변환
    let meters = normalized_to_meters(d.intercept_point);
    let ball_position = Coord10::from_meters(meters.0, meters.1);

    ActionType::Intercept { ball_position }
}

fn convert_hold() -> ActionType {
    // Hold는 특별한 ActionType이 없으므로 Dribble로 대체
    // 방향은 제자리 (0, 0)에 가깝게
    ActionType::Dribble {
        direction: (0.0, 0.0),
        aggressive: false,
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 정규화 좌표를 미터 좌표로 변환
fn normalized_to_meters(pos: (f32, f32)) -> (f32, f32) {
    // 정규화: (0,0) ~ (1,1)
    // 미터: (0,0) ~ (105,68)
    (pos.0 * 105.0, pos.1 * 68.0)
}

/// 방향 벡터를 목표 지점(Coord10)으로 변환
fn direction_to_target(direction: (f32, f32)) -> Coord10 {
    // 필드 중앙에서 방향으로 일정 거리 이동한 지점
    let base_x = 52.5;
    let base_y = 34.0;

    let target_x = (base_x + direction.0 * 40.0).clamp(0.0, 105.0);
    let target_y = (base_y + direction.1 * 25.0).clamp(0.0, 68.0);

    Coord10::from_meters(target_x, target_y)
}

// ============================================================================
// Intent Extractors (V1 → V2 Bridge)
// ============================================================================

use crate::engine::action_detail::ActionDetail as ActionDetailV1;

/// V1 ActionDetail에서 PassIntent 추출
pub fn extract_pass_intent(detail: &ActionDetailV1) -> PassIntent {
    use crate::engine::action_detail::PassType;

    PassIntent {
        intended_target: detail.target.as_ref().and_then(|t| t.player_idx()),
        pass_kind: match detail.pass_type {
            Some(PassType::Short) => PassKind::Short,
            Some(PassType::Through) => PassKind::Through,
            Some(PassType::Long) => PassKind::Long,
            Some(PassType::Lob) => PassKind::Lob,
            Some(PassType::Cross) => PassKind::Short, // 크로스는 별도 처리 - 기본은 Short
            Some(PassType::Switch) => PassKind::Long, // 사이드체인지는 Long 계열
            Some(PassType::Cutback) => PassKind::Short, // 컷백은 Short 계열
            Some(PassType::Back) => PassKind::Short, // 백패스는 Short 계열
            Some(PassType::Clear) => PassKind::Long, // 클리어는 Long 계열
            None => PassKind::Short,
        },
        power: detail.power,
        intended_point: detail.target.as_ref().and_then(|t| t.point()),
    }
}

/// V1 ActionDetail에서 ShotIntent 추출
pub fn extract_shot_intent(detail: &ActionDetailV1) -> ShotIntent {
    ShotIntent {
        target_point: detail.target.as_ref().and_then(|t| t.point()),
        power: detail.power,
        shot_kind: match detail.shot_type {
            Some(crate::engine::action_detail::ShotType::Normal) => Some(ShotKind::Normal),
            Some(crate::engine::action_detail::ShotType::Placed) => Some(ShotKind::Finesse),
            Some(crate::engine::action_detail::ShotType::Chip) => Some(ShotKind::Chip),
            Some(crate::engine::action_detail::ShotType::Power) => Some(ShotKind::Power),
            _ => None,
        },
    }
}

/// V1 ActionDetail에서 DribbleIntent 추출
pub fn extract_dribble_intent(detail: &ActionDetailV1) -> DribbleIntent {
    DribbleIntent {
        direction: detail.get_direction(),
        speed_factor: None, // V1에는 speed_factor가 없음
    }
}

/// V1 ActionDetail에서 TackleIntent 추출
pub fn extract_tackle_intent(detail: &ActionDetailV1) -> TackleIntent {
    TackleIntent {
        target: detail.target.as_ref().and_then(|t| t.player_idx()).map(|i| i as u8),
        tackle_kind: None, // V1에는 tackle_kind가 없음
    }
}

/// V1 ActionDetail에서 HeaderIntent 추출
pub fn extract_header_intent(detail: &ActionDetailV1) -> HeaderIntent {
    // V1에서는 헤더 타입을 명시적으로 구분하지 않음
    // target이 있으면 Pass, target_point가 골문 근처면 Shot, 그 외 Clear
    let target_point = detail.target.as_ref().and_then(|t| t.point());
    let target_type = if detail.target.as_ref().and_then(|t| t.player_idx()).is_some() {
        HeaderTargetType::Pass
    } else if let Some(point) = target_point {
        if point.0 > 0.9 || point.0 < 0.1 {
            HeaderTargetType::Shot
        } else {
            HeaderTargetType::Clear
        }
    } else {
        HeaderTargetType::Clear
    };

    HeaderIntent {
        target_type,
        shot_point: target_point,
        pass_target: detail.target.as_ref().and_then(|t| t.player_idx()).map(|i| i as u8),
        clear_direction: detail.get_direction(),
        power: detail.power,
    }
}

/// V1 ActionDetail에서 ClearanceIntent 추출
pub fn extract_clearance_intent(detail: &ActionDetailV1) -> ClearanceIntent {
    ClearanceIntent {
        direction: detail.get_direction(),
        power: detail.power,
    }
}

/// V1 ActionDetail에서 CrossIntent 추출
pub fn extract_cross_intent(detail: &ActionDetailV1) -> CrossIntent {
    CrossIntent {
        target_point: detail.target.as_ref().and_then(|t| t.point()),
        cross_kind: None, // V1에는 cross_kind가 없음
        power: detail.power,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_pass() {
        let detail = PassDetail::new(5, PassKind::Through, 0.7);
        let action = convert_pass(&detail);

        match action {
            ActionType::Pass { target_idx, is_long, is_through, .. } => {
                assert_eq!(target_idx, 5);
                assert!(!is_long);
                assert!(is_through);
            }
            _ => panic!("Expected Pass"),
        }
    }

    #[test]
    fn test_convert_shot() {
        let detail = ShotDetail::new((1.0, 0.5), 0.9, ShotKind::Normal);
        let action = convert_shot(&detail, true);

        match action {
            ActionType::Shot { power, target: _ } => {
                assert!((power - 0.9).abs() < 0.001);
            }
            _ => panic!("Expected Shot"),
        }
    }

    #[test]
    fn test_convert_dribble_aggressive() {
        let detail = DribbleDetail::new((1.0, 0.0), 0.8); // speed > 0.7 → aggressive
        let action = convert_dribble(&detail);

        match action {
            ActionType::Dribble { direction, aggressive } => {
                assert_eq!(direction, (1.0, 0.0));
                assert!(aggressive);
            }
            _ => panic!("Expected Dribble"),
        }
    }

    #[test]
    fn test_convert_dribble_not_aggressive() {
        let detail = DribbleDetail::new((1.0, 0.0), 0.5); // speed <= 0.7 → not aggressive
        let action = convert_dribble(&detail);

        match action {
            ActionType::Dribble { aggressive, .. } => {
                assert!(!aggressive);
            }
            _ => panic!("Expected Dribble"),
        }
    }

    #[test]
    fn test_convert_tackle() {
        let detail = TackleDetail::new(15, TackleKind::Sliding);
        let action = convert_tackle(&detail);

        match action {
            ActionType::Tackle { target_idx } => {
                assert_eq!(target_idx, 15);
            }
            _ => panic!("Expected Tackle"),
        }
    }

    #[test]
    fn test_normalized_to_meters() {
        let result = normalized_to_meters((0.5, 0.5));
        assert!((result.0 - 52.5).abs() < 0.1);
        assert!((result.1 - 34.0).abs() < 0.1);
    }

    #[test]
    fn test_convert_header_shot() {
        let detail = HeaderDetail {
            target: HeaderTarget::Shot { point: (1.0, 0.5) },
            power: 0.8,
        };
        let action = convert_header(&detail, true);

        match action {
            ActionType::Header { is_shot, .. } => {
                assert!(is_shot);
            }
            _ => panic!("Expected Header"),
        }
    }

    #[test]
    fn test_convert_header_clear() {
        let detail = HeaderDetail {
            target: HeaderTarget::Clear { direction: (-1.0, 0.0) },
            power: 0.7,
        };
        let action = convert_header(&detail, true);

        match action {
            ActionType::Header { is_shot, .. } => {
                assert!(!is_shot);
            }
            _ => panic!("Expected Header"),
        }
    }

    #[test]
    fn test_convert_detail_v2_all_types() {
        // 모든 타입이 panic 없이 변환되는지 확인
        let pass = ActionDetailV2::Pass(PassDetail::new(3, PassKind::Short, 0.5));
        let shot = ActionDetailV2::Shot(ShotDetail::new((1.0, 0.5), 0.8, ShotKind::Normal));
        let dribble = ActionDetailV2::Dribble(DribbleDetail::new((1.0, 0.0), 0.6));
        let tackle = ActionDetailV2::Tackle(TackleDetail::new(12, TackleKind::Standing));
        let header = ActionDetailV2::Header(HeaderDetail {
            target: HeaderTarget::Shot { point: (1.0, 0.5) },
            power: 0.7,
        });
        let cross = ActionDetailV2::Cross(CrossDetail {
            target_point: (0.9, 0.5),
            cross_kind: CrossKind::High,
            power: 0.8,
        });
        let clearance = ActionDetailV2::Clearance(ClearanceDetail {
            direction: (-1.0, 0.0),
            power: 0.9,
        });
        let intercept = ActionDetailV2::Intercept(InterceptDetail {
            intercept_point: (0.5, 0.5),
        });
        let hold = ActionDetailV2::Hold(HoldDetail {
            shield_direction: (0.0, 1.0),
        });

        // 모두 변환 가능해야 함
        let _ = convert_detail_v2_to_action_type(&pass, true);
        let _ = convert_detail_v2_to_action_type(&shot, true);
        let _ = convert_detail_v2_to_action_type(&dribble, true);
        let _ = convert_detail_v2_to_action_type(&tackle, true);
        let _ = convert_detail_v2_to_action_type(&header, true);
        let _ = convert_detail_v2_to_action_type(&cross, true);
        let _ = convert_detail_v2_to_action_type(&clearance, true);
        let _ = convert_detail_v2_to_action_type(&intercept, true);
        let _ = convert_detail_v2_to_action_type(&hold, true);
    }
}
