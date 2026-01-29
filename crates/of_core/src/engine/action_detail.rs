//! P16/P7 ActionDetail - 리팩토링 없이 확장 가능한 액션 세부 파라미터
//!
//! 핵심 원칙:
//! - PlayerAction은 의도 5개만: Shoot | Pass | Dribble | Hold | TakeOn
//! - 세부(패스타입/슈팅타입/타겟/파워/커브/기타)는 모두 ActionDetail에 저장
//! - 실행부는 action으로 큰 분기 → detail로 미세 분기

use serde::{Deserialize, Serialize};

/// P16/P7 호환 + 리팩토링 없이 확장 가능한 설계
///
/// flat + Option 방식을 사용하는 이유:
/// - enum variant 방식: PassDetail/ShotDetail 추가할 때마다 전체 매칭 수정 필요
/// - flat + Option 방식: 새 기능이 생겨도 ActionDetail만 늘리면 되고 기존 코드 안 깨짐
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionDetail {
    // === 공통 필드 (가장 자주 쓰는 것들) ===
    /// 타겟 (누구에게 / 어디로)
    pub target: Option<ActionTarget>,
    /// 파워 (0.0..=1.0, 정규화)
    pub power: Option<f32>,
    /// 커브 (-1.0..=1.0, 좌/우 휘어짐)
    pub curve: Option<f32>,

    // === 액션별 세부 타입 ===
    /// 패스 타입 (Pass 액션일 때)
    pub pass_type: Option<PassType>,
    /// 슛 타입 (Shoot 액션일 때)
    pub shot_type: Option<ShotType>,
    /// 드리블 스타일 (Dribble 액션일 때)
    pub dribble_style: Option<DribbleStyle>,

    // === 확장 슬롯 (리팩토링 방지용) ===
    /// 추가 파라미터들
    pub params: Vec<ActionParam>,
}

impl ActionDetail {
    /// 빈 ActionDetail 생성
    pub fn empty() -> Self {
        Self::default()
    }

    /// Pass용 ActionDetail 생성
    pub fn for_pass(pass_type: PassType, target: ActionTarget, power: f32, curve: f32) -> Self {
        Self {
            target: Some(target),
            power: Some(power),
            curve: Some(curve),
            pass_type: Some(pass_type),
            ..Default::default()
        }
    }

    /// Shot용 ActionDetail 생성
    pub fn for_shot(shot_type: ShotType, target: ActionTarget, power: f32, curve: f32) -> Self {
        Self {
            target: Some(target),
            power: Some(power),
            curve: Some(curve),
            shot_type: Some(shot_type),
            ..Default::default()
        }
    }

    /// Dribble용 ActionDetail 생성
    pub fn for_dribble(style: DribbleStyle, target: Option<ActionTarget>, sprint: bool) -> Self {
        let mut params = Vec::new();
        if sprint {
            params.push(ActionParam::Sprint(true));
        }
        Self { target, dribble_style: Some(style), params, ..Default::default() }
    }

    /// TakeOn용 ActionDetail 생성
    pub fn for_takeon(opponent_idx: usize, direction: (f32, f32), risk: f32) -> Self {
        Self {
            target: Some(ActionTarget::Player(opponent_idx)),
            params: vec![ActionParam::Risk(risk), ActionParam::Direction(direction.0, direction.1)],
            ..Default::default()
        }
    }

    /// 파라미터에서 Sprint 여부 확인
    pub fn is_sprint(&self) -> bool {
        self.params.iter().any(|p| matches!(p, ActionParam::Sprint(true)))
    }

    /// 파라미터에서 Risk 값 가져오기
    pub fn get_risk(&self) -> Option<f32> {
        self.params.iter().find_map(|p| if let ActionParam::Risk(v) = p { Some(*v) } else { None })
    }

    /// 파라미터에서 Direction 가져오기
    pub fn get_direction(&self) -> Option<(f32, f32)> {
        self.params.iter().find_map(|p| {
            if let ActionParam::Direction(x, y) = p {
                Some((*x, *y))
            } else {
                None
            }
        })
    }

    /// 파라미터에서 Height 가져오기
    pub fn get_height(&self) -> Option<f32> {
        self.params.iter().find_map(
            |p| {
                if let ActionParam::Height(v) = p {
                    Some(*v)
                } else {
                    None
                }
            },
        )
    }
}

/// "누구에게 / 어디로" 표현
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ActionTarget {
    /// 특정 선수에게 (가장 흔함)
    Player(usize),

    /// 월드 좌표 지점으로 (미터)
    Point(f32, f32),

    /// 침투 공간으로 (스루패스용)
    Space {
        point: (f32, f32),
        /// 리드 거리 (미터)
        lead: f32,
    },

    /// 골문 지점으로 (슈팅용, 정규화 0~1)
    GoalMouth(f32, f32),
}

impl ActionTarget {
    /// Player variant에서 인덱스 추출
    pub fn player_idx(&self) -> Option<usize> {
        match self {
            ActionTarget::Player(idx) => Some(*idx),
            _ => None,
        }
    }

    /// Point/Space에서 좌표 추출
    pub fn point(&self) -> Option<(f32, f32)> {
        match self {
            ActionTarget::Point(x, y) => Some((*x, *y)),
            ActionTarget::Space { point, .. } => Some(*point),
            ActionTarget::GoalMouth(x, y) => Some((*x, *y)),
            _ => None,
        }
    }
}

/// 패스 타입 (P7/P16 호환 중심)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PassType {
    /// 숏패스 (기본)
    #[default]
    Short,
    /// 스루패스 (침투)
    Through,
    /// 롱패스 / 스위치 계열
    Long,
    /// 로빙 (띄우기)
    Lob,
    /// 크로스
    Cross,
    /// 대각 전환 (사이드체인지)
    Switch,
    /// 컷백
    Cutback,
    /// 백패스
    Back,
    /// 클리어
    Clear,
}

impl PassType {
    /// 롱패스 계열인지 확인
    pub fn is_long(&self) -> bool {
        matches!(
            self,
            PassType::Long | PassType::Lob | PassType::Cross | PassType::Switch | PassType::Clear
        )
    }

    /// 스루패스인지 확인
    pub fn is_through(&self) -> bool {
        matches!(self, PassType::Through)
    }
}

/// 슛 타입
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ShotType {
    /// 일반
    #[default]
    Normal,
    /// 정확/감아차기 (Finesse)
    Placed,
    /// 강슛
    Power,
    /// 칩샷
    Chip,
    /// 로우슛
    Low,
    /// 발리
    Volley,
    /// 헤더
    Header,
    /// 원터치
    FirstTime,
}

/// 드리블 스타일
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DribbleStyle {
    /// 운반 (Carry)
    #[default]
    Carry,
    /// 키핑/턴
    Keep,
    /// 컷인사이드
    CutInside,
    /// 컷아웃사이드
    CutOutside,
    /// 쉴딩
    Shield,
}

/// 미래 기능을 위한 확장 슬롯
/// 새 기능은 여기로 흡수되어 구조가 안 무너짐
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ActionParam {
    // === 자주 쓰는 케이스 (가독성) ===
    /// 킥 높이 (0..1)
    Height(f32),
    /// 로프트 (0..1)
    Loft(f32),
    /// 회전량 (0..1)
    Spin(f32),
    /// 리스크 (0..1)
    Risk(f32),
    /// 가속/속도 편향
    SpeedBias(f32),
    /// 스프린트 여부
    Sprint(bool),
    /// 쉴딩 여부
    Shielding(bool),
    /// 방향 벡터
    Direction(f32, f32),

    // === 기본 스칼라 (라벨 + 값) ===
    /// Float 파라미터 (라벨, 값)
    Float(String, f32),
    /// Int 파라미터 (라벨, 값)
    Int(String, i32),
    /// Bool 파라미터 (라벨, 값)
    Bool(String, bool),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_detail_for_pass() {
        let detail = ActionDetail::for_pass(
            PassType::Through,
            ActionTarget::Space { point: (80.0, 35.0), lead: 5.0 },
            0.7,
            0.2,
        );

        assert_eq!(detail.pass_type, Some(PassType::Through));
        assert_eq!(detail.power, Some(0.7));
        assert_eq!(detail.curve, Some(0.2));
        assert!(matches!(detail.target, Some(ActionTarget::Space { .. })));
    }

    #[test]
    fn test_action_detail_for_shot() {
        let detail =
            ActionDetail::for_shot(ShotType::Placed, ActionTarget::GoalMouth(0.7, 0.8), 0.6, -0.3);

        assert_eq!(detail.shot_type, Some(ShotType::Placed));
        assert_eq!(detail.power, Some(0.6));
        assert_eq!(detail.curve, Some(-0.3));
    }

    #[test]
    fn test_action_detail_for_dribble() {
        let detail = ActionDetail::for_dribble(
            DribbleStyle::Carry,
            Some(ActionTarget::Point(75.0, 40.0)),
            true,
        );

        assert_eq!(detail.dribble_style, Some(DribbleStyle::Carry));
        assert!(detail.is_sprint());
    }

    #[test]
    fn test_action_detail_for_takeon() {
        let detail = ActionDetail::for_takeon(15, (1.0, 0.5), 0.8);

        assert_eq!(detail.target, Some(ActionTarget::Player(15)));
        assert_eq!(detail.get_risk(), Some(0.8));
        assert_eq!(detail.get_direction(), Some((1.0, 0.5)));
    }

    #[test]
    fn test_pass_type_is_long() {
        assert!(!PassType::Short.is_long());
        assert!(!PassType::Through.is_long());
        assert!(PassType::Long.is_long());
        assert!(PassType::Lob.is_long());
        assert!(PassType::Cross.is_long());
        assert!(PassType::Switch.is_long());
        assert!(PassType::Clear.is_long());
    }

    #[test]
    fn test_action_target_player_idx() {
        assert_eq!(ActionTarget::Player(7).player_idx(), Some(7));
        assert_eq!(ActionTarget::Point(10.0, 20.0).player_idx(), None);
    }

    #[test]
    fn test_action_target_point() {
        assert_eq!(ActionTarget::Point(10.0, 20.0).point(), Some((10.0, 20.0)));
        assert_eq!(
            ActionTarget::Space { point: (30.0, 40.0), lead: 5.0 }.point(),
            Some((30.0, 40.0))
        );
        assert_eq!(ActionTarget::GoalMouth(0.5, 0.5).point(), Some((0.5, 0.5)));
    }
}
