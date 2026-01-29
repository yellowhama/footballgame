//! FIX_2601/1124: Candidate Key System
//!
//! UAE 파이프라인의 CandidateKey와 ActionDetailV2의 executed key가
//! 일치하는지 검증하기 위한 타입 시스템.
//!
//! ## Gate A 개념
//!
//! ```text
//! UAE Pipeline:
//!     선택 단계 → CandidateKey 생성
//!         ↓
//!     Detail Builder → ActionDetailV2 생성
//!         ↓
//!     Gate A 검증: CandidateKey == ActionDetailV2.to_candidate_key()
//! ```
//!
//! ## 설계 원칙
//!
//! 1. **키 동일성**: 선택한 것과 실행한 것이 같음을 타입으로 보장
//! 2. **버킷팅**: 연속 값(power, position)은 이산 버킷으로 변환하여 비교
//! 3. **결정론적 변환**: 동일 입력 → 동일 키

use serde::{Deserialize, Serialize};
use super::action_detail_v2::*;
use super::decision_topology::{
    FinalAction, FinalActionType, FinalActionParams,
    ShotParams, ShotTechnique, PassParams, PassType,
    DribbleParams, TackleParams, TackleType,
};

// ============================================================================
// Shot Bucket (슛 타겟 Y 위치 버킷)
// ============================================================================

/// 슛 타겟 Y 위치 버킷
///
/// 골문을 3등분하여 분류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShotBucket {
    /// 왼쪽 포스트 방향 (y < 0.4)
    Near,
    /// 중앙 (0.4 <= y <= 0.6)
    Center,
    /// 오른쪽 포스트 방향 (y > 0.6)
    Far,
}

impl ShotBucket {
    /// Y 좌표에서 버킷 생성
    pub fn from_y(y: f32) -> Self {
        if y < 0.4 {
            ShotBucket::Near
        } else if y > 0.6 {
            ShotBucket::Far
        } else {
            ShotBucket::Center
        }
    }
}

// ============================================================================
// Dribble Channel (드리블 방향 채널)
// ============================================================================

/// 드리블 방향 채널
///
/// 진행 방향을 5개 채널로 분류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DribbleChannel {
    /// 왼쪽 측면
    Left,
    /// 왼쪽 중앙
    LeftCenter,
    /// 중앙
    Center,
    /// 오른쪽 중앙
    RightCenter,
    /// 오른쪽 측면
    Right,
}

impl DribbleChannel {
    /// 방향 벡터에서 채널 생성
    ///
    /// y 값 기준으로 분류 (-1.0 ~ 1.0)
    pub fn from_direction(direction: (f32, f32)) -> Self {
        let y = direction.1;
        if y < -0.6 {
            DribbleChannel::Left
        } else if y < -0.2 {
            DribbleChannel::LeftCenter
        } else if y > 0.6 {
            DribbleChannel::Right
        } else if y > 0.2 {
            DribbleChannel::RightCenter
        } else {
            DribbleChannel::Center
        }
    }
}

// ============================================================================
// Speed Bucket (속도 버킷)
// ============================================================================

/// 속도 버킷
///
/// 속도를 3단계로 분류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpeedBucket {
    /// 느림 (< 0.4)
    Slow,
    /// 중간 (0.4 <= x <= 0.7)
    Medium,
    /// 빠름 (> 0.7)
    Fast,
}

impl SpeedBucket {
    /// 속도 값에서 버킷 생성
    pub fn from_speed(speed: f32) -> Self {
        if speed < 0.4 {
            SpeedBucket::Slow
        } else if speed > 0.7 {
            SpeedBucket::Fast
        } else {
            SpeedBucket::Medium
        }
    }
}

// ============================================================================
// Individual Keys
// ============================================================================

/// Pass 키 (패스 종류 + 대상)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PassKey {
    pub pass_kind: PassKind,
    pub target_track_id: u8,
}

/// Shot 키 (슛 종류 + 타겟 버킷)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShotKey {
    pub shot_kind: ShotKind,
    pub target_bucket: ShotBucket,
}

/// Dribble 키 (채널 + 속도 버킷)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DribbleKey {
    pub channel: DribbleChannel,
    pub speed_bucket: SpeedBucket,
}

/// Tackle 키 (대상 + 종류)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TackleKey {
    pub target_track_id: u8,
    pub tackle_kind: TackleKind,
}

/// Header 키 (목표 타입 + 상세)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeaderKey {
    Shot { target_bucket: ShotBucket },
    Pass { target_track_id: u8 },
    Clear { channel: DribbleChannel },
}

/// Cross 키 (종류 + 타겟 버킷)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CrossKey {
    pub cross_kind: CrossKind,
    pub target_bucket: ShotBucket, // 재사용: Near/Center/Far
}

// ============================================================================
// CandidateKey
// ============================================================================

/// 후보 키 - 선택/실행된 액션의 핵심 식별자
///
/// UAE 선택 단계에서 생성되고, 실행 후 ActionDetailV2에서 추출하여 비교한다.
/// Gate A는 이 두 키가 일치하는지 검증한다.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateKey {
    /// 패스: 종류 + 대상
    Pass(PassKey),
    /// 슛: 종류 + 버킷
    Shot(ShotKey),
    /// 드리블: 채널 + 속도
    Dribble(DribbleKey),
    /// 태클: 대상 + 종류
    Tackle(TackleKey),
    /// 헤더: 목표 타입 + 상세
    Header(HeaderKey),
    /// 크로스: 종류 + 버킷
    Cross(CrossKey),
    /// 클리어: 단순 (방향은 상황에 따라 다름)
    Clearance,
    /// 인터셉트: 단순
    Intercept,
    /// 홀드: 단순
    Hold,
}

impl CandidateKey {
    /// 액션 종류 이름 반환
    pub fn kind_name(&self) -> &'static str {
        match self {
            CandidateKey::Pass(_) => "Pass",
            CandidateKey::Shot(_) => "Shot",
            CandidateKey::Dribble(_) => "Dribble",
            CandidateKey::Tackle(_) => "Tackle",
            CandidateKey::Header(_) => "Header",
            CandidateKey::Cross(_) => "Cross",
            CandidateKey::Clearance => "Clearance",
            CandidateKey::Intercept => "Intercept",
            CandidateKey::Hold => "Hold",
        }
    }

    /// FinalAction에서 CandidateKey 생성
    ///
    /// UAE 파이프라인에서 선택된 FinalAction을 Gate A 검증을 위한
    /// CandidateKey로 변환한다.
    ///
    /// ## 매핑 규칙
    ///
    /// | FinalActionType | CandidateKey |
    /// |-----------------|--------------|
    /// | Shot | Shot(ShotKey) |
    /// | Pass | Pass(PassKey) |
    /// | Dribble | Dribble(DribbleKey) |
    /// | Cross | Cross(CrossKey) |
    /// | Tackle | Tackle(TackleKey) |
    /// | Clear | Clearance |
    /// | Block | Intercept |
    /// | Movement | Hold |
    /// | Hold | Hold |
    pub fn from_final_action(action: &FinalAction) -> Self {
        match action.action_type {
            FinalActionType::Shot => {
                // Header technique은 별도의 CandidateKey::Header로 처리
                if let FinalActionParams::Shot(ShotParams { technique: ShotTechnique::Header, .. }) = &action.params {
                    let target_bucket = action
                        .target_pos
                        .map(|(_, y)| ShotBucket::from_y(y))
                        .unwrap_or(ShotBucket::Center);
                    return CandidateKey::Header(HeaderKey::Shot { target_bucket });
                }

                let shot_kind = match &action.params {
                    FinalActionParams::Shot(ShotParams { technique, .. }) => {
                        match technique {
                            ShotTechnique::Normal => ShotKind::Normal,
                            ShotTechnique::Finesse => ShotKind::Finesse,
                            ShotTechnique::Power => ShotKind::Power,
                            ShotTechnique::Chip => ShotKind::Chip,
                            ShotTechnique::Header => unreachable!(), // 위에서 처리됨
                        }
                    }
                    _ => ShotKind::Normal,
                };

                let target_bucket = action
                    .target_pos
                    .map(|(_, y)| ShotBucket::from_y(y))
                    .unwrap_or(ShotBucket::Center);

                CandidateKey::Shot(ShotKey {
                    shot_kind,
                    target_bucket,
                })
            }

            FinalActionType::Pass => {
                // Clear type pass는 실제로 클리어 액션
                if let FinalActionParams::Pass(PassParams { pass_type: PassType::Clear, .. }) = &action.params {
                    return CandidateKey::Clearance;
                }

                let pass_kind = match &action.params {
                    FinalActionParams::Pass(PassParams { pass_type, is_lofted }) => {
                        match (pass_type, is_lofted) {
                            (PassType::Ground, false) => PassKind::Short,
                            (PassType::Ground, true) => PassKind::Lob,
                            (PassType::Through, _) => PassKind::Through,
                            (PassType::Lob, _) => PassKind::Lob,
                            (PassType::Cross, _) => PassKind::Long,
                            (PassType::Clear, _) => unreachable!(), // 위에서 처리됨
                        }
                    }
                    _ => PassKind::Short,
                };

                let target_track_id = action.target_player.unwrap_or(0) as u8;

                CandidateKey::Pass(PassKey {
                    pass_kind,
                    target_track_id,
                })
            }

            FinalActionType::Dribble => {
                let (channel, speed_bucket) = match &action.params {
                    FinalActionParams::Dribble(DribbleParams { direction, is_skill_move }) => {
                        let channel = DribbleChannel::from_direction(*direction);
                        let speed_bucket = if *is_skill_move {
                            SpeedBucket::Slow // 스킬무브는 느림
                        } else {
                            SpeedBucket::from_speed(action.power)
                        };
                        (channel, speed_bucket)
                    }
                    _ => {
                        let direction = action.target_pos.unwrap_or((1.0, 0.0));
                        (
                            DribbleChannel::from_direction(direction),
                            SpeedBucket::from_speed(action.power),
                        )
                    }
                };

                CandidateKey::Dribble(DribbleKey { channel, speed_bucket })
            }

            FinalActionType::Cross => {
                // Cross kind는 power 기반으로 추론
                let cross_kind = if action.power < 0.4 {
                    CrossKind::Low
                } else if action.power > 0.7 {
                    CrossKind::High
                } else {
                    CrossKind::Whipped
                };

                let target_bucket = action
                    .target_pos
                    .map(|(_, y)| ShotBucket::from_y(y))
                    .unwrap_or(ShotBucket::Center);

                CandidateKey::Cross(CrossKey {
                    cross_kind,
                    target_bucket,
                })
            }

            FinalActionType::Tackle => {
                let tackle_kind = match &action.params {
                    FinalActionParams::Tackle(TackleParams { tackle_type, .. }) => {
                        match tackle_type {
                            TackleType::Standing => TackleKind::Standing,
                            TackleType::Sliding => TackleKind::Sliding,
                            TackleType::Shoulder => TackleKind::Shoulder,
                            TackleType::Poke => TackleKind::Standing, // Poke → Standing
                        }
                    }
                    _ => TackleKind::Standing,
                };

                let target_track_id = action.target_player.unwrap_or(0) as u8;

                CandidateKey::Tackle(TackleKey {
                    target_track_id,
                    tackle_kind,
                })
            }

            FinalActionType::Clear => CandidateKey::Clearance,
            FinalActionType::Block => CandidateKey::Intercept,
            FinalActionType::Movement => CandidateKey::Hold,
            FinalActionType::Hold => CandidateKey::Hold,
        }
    }
}

// ============================================================================
// ActionDetailV2 → CandidateKey 변환
// ============================================================================

impl From<&ActionDetailV2> for CandidateKey {
    fn from(detail: &ActionDetailV2) -> Self {
        match detail {
            ActionDetailV2::Pass(d) => CandidateKey::Pass(PassKey {
                pass_kind: d.pass_kind,
                target_track_id: d.target_track_id,
            }),
            ActionDetailV2::Shot(d) => CandidateKey::Shot(ShotKey {
                shot_kind: d.shot_kind,
                target_bucket: ShotBucket::from_y(d.target_point.1),
            }),
            ActionDetailV2::Dribble(d) => CandidateKey::Dribble(DribbleKey {
                channel: DribbleChannel::from_direction(d.direction),
                speed_bucket: SpeedBucket::from_speed(d.speed_factor),
            }),
            ActionDetailV2::Tackle(d) => CandidateKey::Tackle(TackleKey {
                target_track_id: d.target_track_id,
                tackle_kind: d.tackle_kind,
            }),
            ActionDetailV2::Header(d) => match &d.target {
                HeaderTarget::Shot { point } => CandidateKey::Header(HeaderKey::Shot {
                    target_bucket: ShotBucket::from_y(point.1),
                }),
                HeaderTarget::Pass { target_track_id } => CandidateKey::Header(HeaderKey::Pass {
                    target_track_id: *target_track_id,
                }),
                HeaderTarget::Clear { direction } => CandidateKey::Header(HeaderKey::Clear {
                    channel: DribbleChannel::from_direction(*direction),
                }),
            },
            ActionDetailV2::Cross(d) => CandidateKey::Cross(CrossKey {
                cross_kind: d.cross_kind,
                target_bucket: ShotBucket::from_y(d.target_point.1),
            }),
            ActionDetailV2::Clearance(_) => CandidateKey::Clearance,
            ActionDetailV2::Intercept(_) => CandidateKey::Intercept,
            ActionDetailV2::Hold(_) => CandidateKey::Hold,
        }
    }
}

impl ActionDetailV2 {
    /// CandidateKey 추출
    pub fn to_candidate_key(&self) -> CandidateKey {
        CandidateKey::from(self)
    }
}

// ============================================================================
// Gate A Validation
// ============================================================================

/// Gate A 검증 결과
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateAResult {
    /// 통과: 선택 키와 실행 키가 일치
    Pass,
    /// 실패: 키 불일치
    Fail {
        selected: CandidateKey,
        executed: CandidateKey,
    },
}

impl GateAResult {
    /// 통과 여부
    pub fn passed(&self) -> bool {
        matches!(self, GateAResult::Pass)
    }
}

/// Gate A 검증 함수
///
/// 선택 단계에서 생성된 CandidateKey와
/// 실행 후 ActionDetailV2에서 추출한 키가 일치하는지 검증한다.
pub fn validate_gate_a(selected: &CandidateKey, detail: &ActionDetailV2) -> GateAResult {
    let executed = detail.to_candidate_key();

    if selected == &executed {
        GateAResult::Pass
    } else {
        GateAResult::Fail {
            selected: selected.clone(),
            executed,
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
    fn test_shot_bucket_from_y() {
        assert_eq!(ShotBucket::from_y(0.2), ShotBucket::Near);
        assert_eq!(ShotBucket::from_y(0.5), ShotBucket::Center);
        assert_eq!(ShotBucket::from_y(0.8), ShotBucket::Far);
    }

    #[test]
    fn test_dribble_channel_from_direction() {
        assert_eq!(
            DribbleChannel::from_direction((1.0, -0.8)),
            DribbleChannel::Left
        );
        assert_eq!(
            DribbleChannel::from_direction((1.0, 0.0)),
            DribbleChannel::Center
        );
        assert_eq!(
            DribbleChannel::from_direction((1.0, 0.8)),
            DribbleChannel::Right
        );
    }

    #[test]
    fn test_speed_bucket_from_speed() {
        assert_eq!(SpeedBucket::from_speed(0.3), SpeedBucket::Slow);
        assert_eq!(SpeedBucket::from_speed(0.5), SpeedBucket::Medium);
        assert_eq!(SpeedBucket::from_speed(0.9), SpeedBucket::Fast);
    }

    #[test]
    fn test_pass_detail_to_candidate_key() {
        let detail = ActionDetailV2::Pass(PassDetail {
            target_track_id: 5,
            pass_kind: PassKind::Through,
            power: 0.7,
            intended_point: None,
            intended_passer_pos: None,
        });

        let key = detail.to_candidate_key();
        assert_eq!(
            key,
            CandidateKey::Pass(PassKey {
                pass_kind: PassKind::Through,
                target_track_id: 5,
            })
        );
    }

    #[test]
    fn test_shot_detail_to_candidate_key() {
        let detail = ActionDetailV2::Shot(ShotDetail {
            target_point: (1.0, 0.5),
            power: 0.9,
            shot_kind: ShotKind::Finesse,
        });

        let key = detail.to_candidate_key();
        assert_eq!(
            key,
            CandidateKey::Shot(ShotKey {
                shot_kind: ShotKind::Finesse,
                target_bucket: ShotBucket::Center,
            })
        );
    }

    #[test]
    fn test_dribble_detail_to_candidate_key() {
        let detail = ActionDetailV2::Dribble(DribbleDetail {
            direction: (1.0, 0.5),
            speed_factor: 0.8,
        });

        let key = detail.to_candidate_key();
        assert_eq!(
            key,
            CandidateKey::Dribble(DribbleKey {
                channel: DribbleChannel::RightCenter,
                speed_bucket: SpeedBucket::Fast,
            })
        );
    }

    #[test]
    fn test_header_shot_to_candidate_key() {
        let detail = ActionDetailV2::Header(HeaderDetail {
            target: HeaderTarget::Shot { point: (1.0, 0.3) },
            power: 0.8,
        });

        let key = detail.to_candidate_key();
        assert_eq!(
            key,
            CandidateKey::Header(HeaderKey::Shot {
                target_bucket: ShotBucket::Near
            })
        );
    }

    #[test]
    fn test_header_pass_to_candidate_key() {
        let detail = ActionDetailV2::Header(HeaderDetail {
            target: HeaderTarget::Pass { target_track_id: 7 },
            power: 0.6,
        });

        let key = detail.to_candidate_key();
        assert_eq!(
            key,
            CandidateKey::Header(HeaderKey::Pass { target_track_id: 7 })
        );
    }

    #[test]
    fn test_simple_actions_to_candidate_key() {
        let clearance = ActionDetailV2::Clearance(ClearanceDetail {
            direction: (0.0, 1.0),
            power: 0.9,
        });
        assert_eq!(clearance.to_candidate_key(), CandidateKey::Clearance);

        let intercept = ActionDetailV2::Intercept(InterceptDetail {
            intercept_point: (0.5, 0.5),
        });
        assert_eq!(intercept.to_candidate_key(), CandidateKey::Intercept);

        let hold = ActionDetailV2::Hold(HoldDetail {
            shield_direction: (1.0, 0.0),
        });
        assert_eq!(hold.to_candidate_key(), CandidateKey::Hold);
    }

    #[test]
    fn test_gate_a_pass() {
        let selected = CandidateKey::Pass(PassKey {
            pass_kind: PassKind::Short,
            target_track_id: 3,
        });

        let detail = ActionDetailV2::Pass(PassDetail {
            target_track_id: 3,
            pass_kind: PassKind::Short,
            power: 0.5,
            intended_point: None,
            intended_passer_pos: None,
        });

        let result = validate_gate_a(&selected, &detail);
        assert!(result.passed());
    }

    #[test]
    fn test_gate_a_fail_different_target() {
        let selected = CandidateKey::Pass(PassKey {
            pass_kind: PassKind::Short,
            target_track_id: 3,
        });

        let detail = ActionDetailV2::Pass(PassDetail {
            target_track_id: 5, // 다른 타겟!
            pass_kind: PassKind::Short,
            power: 0.5,
            intended_point: None,
            intended_passer_pos: None,
        });

        let result = validate_gate_a(&selected, &detail);
        assert!(!result.passed());
    }

    #[test]
    fn test_gate_a_fail_different_action_type() {
        let selected = CandidateKey::Pass(PassKey {
            pass_kind: PassKind::Short,
            target_track_id: 3,
        });

        let detail = ActionDetailV2::Dribble(DribbleDetail {
            direction: (1.0, 0.0),
            speed_factor: 0.8,
        });

        let result = validate_gate_a(&selected, &detail);
        assert!(!result.passed());

        if let GateAResult::Fail { selected: s, executed: e } = result {
            assert_eq!(s.kind_name(), "Pass");
            assert_eq!(e.kind_name(), "Dribble");
        }
    }

    #[test]
    fn test_shot_bucket_boundary() {
        // 경계값 테스트
        assert_eq!(ShotBucket::from_y(0.39), ShotBucket::Near);
        assert_eq!(ShotBucket::from_y(0.40), ShotBucket::Center);
        assert_eq!(ShotBucket::from_y(0.60), ShotBucket::Center);
        assert_eq!(ShotBucket::from_y(0.61), ShotBucket::Far);
    }

    #[test]
    fn test_candidate_key_kind_name() {
        assert_eq!(
            CandidateKey::Pass(PassKey {
                pass_kind: PassKind::Short,
                target_track_id: 0
            })
            .kind_name(),
            "Pass"
        );
        assert_eq!(
            CandidateKey::Shot(ShotKey {
                shot_kind: ShotKind::Normal,
                target_bucket: ShotBucket::Center
            })
            .kind_name(),
            "Shot"
        );
        assert_eq!(CandidateKey::Clearance.kind_name(), "Clearance");
        assert_eq!(CandidateKey::Intercept.kind_name(), "Intercept");
        assert_eq!(CandidateKey::Hold.kind_name(), "Hold");
    }

    // ========================================================================
    // FinalAction → CandidateKey 변환 테스트
    // ========================================================================

    use super::super::decision_topology::{Foot, MovementParams, MovementType};

    fn make_shot_action(technique: ShotTechnique, target_y: f32) -> FinalAction {
        FinalAction {
            action_type: FinalActionType::Shot,
            target_pos: Some((1.0, target_y)),
            target_player: None,
            power: 0.8,
            curve: 0.0,
            params: FinalActionParams::Shot(ShotParams {
                technique,
                foot: Foot::Right,
            }),
        }
    }

    fn make_pass_action(pass_type: PassType, is_lofted: bool, target: usize) -> FinalAction {
        FinalAction {
            action_type: FinalActionType::Pass,
            target_pos: Some((0.5, 0.5)),
            target_player: Some(target),
            power: 0.6,
            curve: 0.0,
            params: FinalActionParams::Pass(PassParams {
                pass_type,
                is_lofted,
            }),
        }
    }

    fn make_dribble_action(direction: (f32, f32), is_skill: bool, power: f32) -> FinalAction {
        FinalAction {
            action_type: FinalActionType::Dribble,
            target_pos: None,
            target_player: None,
            power,
            curve: 0.0,
            params: FinalActionParams::Dribble(DribbleParams {
                direction,
                is_skill_move: is_skill,
            }),
        }
    }

    fn make_tackle_action(tackle_type: TackleType, target: usize) -> FinalAction {
        FinalAction {
            action_type: FinalActionType::Tackle,
            target_pos: None,
            target_player: Some(target),
            power: 0.7,
            curve: 0.0,
            params: FinalActionParams::Tackle(TackleParams {
                tackle_type,
                commit_level: 0.5,
            }),
        }
    }

    #[test]
    fn test_from_final_action_shot_normal() {
        let action = make_shot_action(ShotTechnique::Normal, 0.5);
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(
            key,
            CandidateKey::Shot(ShotKey {
                shot_kind: ShotKind::Normal,
                target_bucket: ShotBucket::Center,
            })
        );
    }

    #[test]
    fn test_from_final_action_shot_finesse() {
        let action = make_shot_action(ShotTechnique::Finesse, 0.2);
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(
            key,
            CandidateKey::Shot(ShotKey {
                shot_kind: ShotKind::Finesse,
                target_bucket: ShotBucket::Near,
            })
        );
    }

    #[test]
    fn test_from_final_action_shot_header() {
        let action = make_shot_action(ShotTechnique::Header, 0.8);
        let key = CandidateKey::from_final_action(&action);

        // Header technique → CandidateKey::Header
        assert_eq!(
            key,
            CandidateKey::Header(HeaderKey::Shot {
                target_bucket: ShotBucket::Far
            })
        );
    }

    #[test]
    fn test_from_final_action_pass_ground() {
        let action = make_pass_action(PassType::Ground, false, 5);
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(
            key,
            CandidateKey::Pass(PassKey {
                pass_kind: PassKind::Short,
                target_track_id: 5,
            })
        );
    }

    #[test]
    fn test_from_final_action_pass_through() {
        let action = make_pass_action(PassType::Through, false, 7);
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(
            key,
            CandidateKey::Pass(PassKey {
                pass_kind: PassKind::Through,
                target_track_id: 7,
            })
        );
    }

    #[test]
    fn test_from_final_action_pass_lob() {
        let action = make_pass_action(PassType::Lob, true, 3);
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(
            key,
            CandidateKey::Pass(PassKey {
                pass_kind: PassKind::Lob,
                target_track_id: 3,
            })
        );
    }

    #[test]
    fn test_from_final_action_pass_clear() {
        let action = make_pass_action(PassType::Clear, false, 0);
        let key = CandidateKey::from_final_action(&action);

        // Clear type pass → Clearance
        assert_eq!(key, CandidateKey::Clearance);
    }

    #[test]
    fn test_from_final_action_dribble_normal() {
        let action = make_dribble_action((1.0, 0.5), false, 0.6);
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(
            key,
            CandidateKey::Dribble(DribbleKey {
                channel: DribbleChannel::RightCenter,
                speed_bucket: SpeedBucket::Medium,
            })
        );
    }

    #[test]
    fn test_from_final_action_dribble_skill_move() {
        let action = make_dribble_action((1.0, -0.7), true, 0.9);
        let key = CandidateKey::from_final_action(&action);

        // Skill move → Slow speed bucket
        assert_eq!(
            key,
            CandidateKey::Dribble(DribbleKey {
                channel: DribbleChannel::Left,
                speed_bucket: SpeedBucket::Slow,
            })
        );
    }

    #[test]
    fn test_from_final_action_tackle_standing() {
        let action = make_tackle_action(TackleType::Standing, 10);
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(
            key,
            CandidateKey::Tackle(TackleKey {
                target_track_id: 10,
                tackle_kind: TackleKind::Standing,
            })
        );
    }

    #[test]
    fn test_from_final_action_tackle_sliding() {
        let action = make_tackle_action(TackleType::Sliding, 15);
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(
            key,
            CandidateKey::Tackle(TackleKey {
                target_track_id: 15,
                tackle_kind: TackleKind::Sliding,
            })
        );
    }

    #[test]
    fn test_from_final_action_tackle_poke() {
        let action = make_tackle_action(TackleType::Poke, 8);
        let key = CandidateKey::from_final_action(&action);

        // Poke → Standing
        assert_eq!(
            key,
            CandidateKey::Tackle(TackleKey {
                target_track_id: 8,
                tackle_kind: TackleKind::Standing,
            })
        );
    }

    #[test]
    fn test_from_final_action_cross() {
        let action = FinalAction {
            action_type: FinalActionType::Cross,
            target_pos: Some((0.9, 0.3)),
            target_player: None,
            power: 0.5, // Medium power → Whipped
            curve: 0.2,
            params: FinalActionParams::None,
        };
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(
            key,
            CandidateKey::Cross(CrossKey {
                cross_kind: CrossKind::Whipped,
                target_bucket: ShotBucket::Near,
            })
        );
    }

    #[test]
    fn test_from_final_action_cross_high_power() {
        let action = FinalAction {
            action_type: FinalActionType::Cross,
            target_pos: Some((0.9, 0.7)),
            target_player: None,
            power: 0.9, // High power → High cross
            curve: 0.0,
            params: FinalActionParams::None,
        };
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(
            key,
            CandidateKey::Cross(CrossKey {
                cross_kind: CrossKind::High,
                target_bucket: ShotBucket::Far,
            })
        );
    }

    #[test]
    fn test_from_final_action_clear() {
        let action = FinalAction {
            action_type: FinalActionType::Clear,
            target_pos: None,
            target_player: None,
            power: 0.9,
            curve: 0.0,
            params: FinalActionParams::None,
        };
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(key, CandidateKey::Clearance);
    }

    #[test]
    fn test_from_final_action_hold() {
        let action = FinalAction {
            action_type: FinalActionType::Hold,
            target_pos: None,
            target_player: None,
            power: 0.0,
            curve: 0.0,
            params: FinalActionParams::None,
        };
        let key = CandidateKey::from_final_action(&action);

        assert_eq!(key, CandidateKey::Hold);
    }

    #[test]
    fn test_from_final_action_movement() {
        let action = FinalAction {
            action_type: FinalActionType::Movement,
            target_pos: Some((0.5, 0.5)),
            target_player: None,
            power: 0.5,
            curve: 0.0,
            params: FinalActionParams::Movement(MovementParams {
                movement_type: MovementType::Jog,
                speed_factor: 0.5,
            }),
        };
        let key = CandidateKey::from_final_action(&action);

        // Movement → Hold (비볼 액션)
        assert_eq!(key, CandidateKey::Hold);
    }

    #[test]
    fn test_from_final_action_block() {
        let action = FinalAction {
            action_type: FinalActionType::Block,
            target_pos: None,
            target_player: None,
            power: 0.0,
            curve: 0.0,
            params: FinalActionParams::None,
        };
        let key = CandidateKey::from_final_action(&action);

        // Block → Intercept
        assert_eq!(key, CandidateKey::Intercept);
    }
}
