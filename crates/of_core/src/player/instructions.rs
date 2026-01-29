//! Player Instructions System
//!
//! 개인 선수 전술 지시 시스템 - Role(Preset) + Instructions(Custom)
//! - Role: 미리 정의된 Instructions 조합 (예: Target Man, Playmaker)
//! - Instructions: 세부 전술 항목 (유저가 개별 조정 가능)

use crate::models::player::PlayerAttributes;
use crate::models::Position;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// 개인 선수 전술 지시 (8개 주요 항목)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerInstructions {
    /// 공격 성향 (얼마나 전진하는가)
    pub mentality: Mentality,

    /// 좌우 포지셔닝 (측면 vs 중앙)
    pub width: Width,

    /// 전후 포지셔닝 (수비 vs 공격)
    pub depth: Depth,

    /// 패싱 스타일
    pub passing: PassingStyle,

    /// 드리블 빈도
    pub dribbling: DribblingFrequency,

    /// 슈팅 성향
    pub shooting: ShootingTendency,

    /// 수비 기여도
    pub defensive_work: DefensiveWork,

    /// 압박 강도
    pub pressing: PressingIntensity,
}

impl Default for PlayerInstructions {
    fn default() -> Self {
        Self {
            mentality: Mentality::Balanced,
            width: Width::Normal,
            depth: Depth::Balanced,
            passing: PassingStyle::Mixed,
            dribbling: DribblingFrequency::Normal,
            shooting: ShootingTendency::Normal,
            defensive_work: DefensiveWork::Normal,
            pressing: PressingIntensity::Medium,
        }
    }
}

/// 공격 성향 (전진 빈도)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Mentality {
    #[serde(rename = "conservative")]
    Conservative, // 20% 전진, 수비 중시
    #[serde(rename = "balanced")]
    Balanced, // 50% 전진, 균형
    #[serde(rename = "aggressive")]
    Aggressive, // 80% 전진, 공격 중시
}

/// 좌우 포지셔닝
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Width {
    #[serde(rename = "stay_wide")]
    StayWide, // 측면 유지
    #[serde(rename = "normal")]
    Normal, // 보통
    #[serde(rename = "cut_inside")]
    CutInside, // 중앙으로 이동
    #[serde(rename = "roam")]
    Roam, // 자유롭게
}

/// 전후 포지셔닝
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Depth {
    #[serde(rename = "stay_back")]
    StayBack, // 후방 유지
    #[serde(rename = "balanced")]
    Balanced, // 균형
    #[serde(rename = "get_forward")]
    GetForward, // 전방 이동
}

/// 패싱 스타일
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PassingStyle {
    #[serde(rename = "short")]
    Short, // 짧은 패스 위주
    #[serde(rename = "mixed")]
    Mixed, // 혼합
    #[serde(rename = "direct")]
    Direct, // 직접적인 롱패스
}

/// 드리블 빈도
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DribblingFrequency {
    #[serde(rename = "rarely")]
    Rarely, // 거의 안함
    #[serde(rename = "normal")]
    Normal, // 보통
    #[serde(rename = "often")]
    Often, // 자주
}

/// 슈팅 성향
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ShootingTendency {
    #[serde(rename = "conservative")]
    Conservative, // 신중하게
    #[serde(rename = "normal")]
    Normal, // 보통
    #[serde(rename = "shoot_on_sight")]
    ShootOnSight, // 보이면 쏜다
}

/// 수비 기여도
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DefensiveWork {
    #[serde(rename = "minimal")]
    Minimal, // 최소한
    #[serde(rename = "normal")]
    Normal, // 보통
    #[serde(rename = "high")]
    High, // 적극적
}

/// 압박 강도
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PressingIntensity {
    #[serde(rename = "low")]
    Low, // 낮음
    #[serde(rename = "medium")]
    Medium, // 중간
    #[serde(rename = "high")]
    High, // 높음
}

/// Role = Instructions의 Preset
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PlayerRole {
    // ===== Forward Roles =====
    #[serde(rename = "target_man")]
    TargetMan, // 타겟맨: 강력한 피지컬, 헤딩 중심
    #[serde(rename = "poacher")]
    Poacher, // 포처: 골만 노리는 순수 스트라이커
    #[serde(rename = "complete_forward")]
    CompleteForward, // 컴플리트 포워드: 균형잡힌 공격수

    // ===== Midfielder Roles =====
    #[serde(rename = "playmaker")]
    Playmaker, // 플레이메이커: 패싱과 빌드업 중심
    #[serde(rename = "box_to_box")]
    BoxToBox, // 박스투박스: 공수 균형형 미드필더
    #[serde(rename = "ball_winning")]
    BallWinning, // 볼 위닝: 수비형 미드필더

    // ===== Defender Roles =====
    #[serde(rename = "ball_playing_defender")]
    BallPlayingDefender, // 빌드업형 수비수
    #[serde(rename = "stopper")]
    Stopper, // 태클 중심 수비수
    #[serde(rename = "covering_defender")]
    CoveringDefender, // 커버링 중심 수비수
}

impl PlayerRole {
    /// Role에 해당하는 기본 Instructions 반환 (Preset)
    pub fn default_instructions(&self) -> PlayerInstructions {
        match self {
            // ===== Forward Roles =====
            PlayerRole::TargetMan => PlayerInstructions {
                mentality: Mentality::Aggressive,
                width: Width::Normal,
                depth: Depth::GetForward,
                passing: PassingStyle::Short, // 받아서 연결
                dribbling: DribblingFrequency::Rarely,
                shooting: ShootingTendency::ShootOnSight,
                defensive_work: DefensiveWork::Minimal,
                pressing: PressingIntensity::Low,
            },

            PlayerRole::Poacher => PlayerInstructions {
                mentality: Mentality::Aggressive,
                width: Width::Roam, // 공간 찾아 이동
                depth: Depth::GetForward,
                passing: PassingStyle::Short,
                dribbling: DribblingFrequency::Rarely,
                shooting: ShootingTendency::ShootOnSight,
                defensive_work: DefensiveWork::Minimal,
                pressing: PressingIntensity::Low,
            },

            PlayerRole::CompleteForward => PlayerInstructions {
                mentality: Mentality::Aggressive,
                width: Width::Normal,
                depth: Depth::GetForward,
                passing: PassingStyle::Mixed,
                dribbling: DribblingFrequency::Normal,
                shooting: ShootingTendency::Normal,
                defensive_work: DefensiveWork::Normal,
                pressing: PressingIntensity::Medium,
            },

            // ===== Midfielder Roles =====
            PlayerRole::Playmaker => PlayerInstructions {
                mentality: Mentality::Balanced,
                width: Width::Normal,
                depth: Depth::Balanced,
                passing: PassingStyle::Mixed,
                dribbling: DribblingFrequency::Normal,
                shooting: ShootingTendency::Conservative,
                defensive_work: DefensiveWork::Normal,
                pressing: PressingIntensity::Medium,
            },

            PlayerRole::BoxToBox => PlayerInstructions {
                mentality: Mentality::Balanced,
                width: Width::Normal,
                depth: Depth::Balanced,
                passing: PassingStyle::Mixed,
                dribbling: DribblingFrequency::Normal,
                shooting: ShootingTendency::Normal,
                defensive_work: DefensiveWork::High,
                pressing: PressingIntensity::High,
            },

            PlayerRole::BallWinning => PlayerInstructions {
                mentality: Mentality::Conservative,
                width: Width::Normal,
                depth: Depth::StayBack,
                passing: PassingStyle::Short,
                dribbling: DribblingFrequency::Rarely,
                shooting: ShootingTendency::Conservative,
                defensive_work: DefensiveWork::High,
                pressing: PressingIntensity::High,
            },

            // ===== Defender Roles =====
            PlayerRole::BallPlayingDefender => PlayerInstructions {
                mentality: Mentality::Conservative,
                width: Width::Normal,
                depth: Depth::StayBack,
                passing: PassingStyle::Mixed,
                dribbling: DribblingFrequency::Normal,
                shooting: ShootingTendency::Conservative,
                defensive_work: DefensiveWork::High,
                pressing: PressingIntensity::Medium,
            },

            PlayerRole::Stopper => PlayerInstructions {
                mentality: Mentality::Conservative,
                width: Width::Normal,
                depth: Depth::StayBack,
                passing: PassingStyle::Short,
                dribbling: DribblingFrequency::Rarely,
                shooting: ShootingTendency::Conservative,
                defensive_work: DefensiveWork::High,
                pressing: PressingIntensity::High,
            },

            PlayerRole::CoveringDefender => PlayerInstructions {
                mentality: Mentality::Conservative,
                width: Width::Normal,
                depth: Depth::StayBack,
                passing: PassingStyle::Short,
                dribbling: DribblingFrequency::Rarely,
                shooting: ShootingTendency::Conservative,
                defensive_work: DefensiveWork::High,
                pressing: PressingIntensity::Low, // 커버링 중심
            },
        }
    }

    /// Role 이름 (한글)
    pub fn display_name_ko(&self) -> &'static str {
        match self {
            PlayerRole::TargetMan => "타겟맨",
            PlayerRole::Poacher => "포처",
            PlayerRole::CompleteForward => "컴플리트 포워드",
            PlayerRole::Playmaker => "플레이메이커",
            PlayerRole::BoxToBox => "박스투박스",
            PlayerRole::BallWinning => "볼위닝 미드필더",
            PlayerRole::BallPlayingDefender => "빌드업형 수비수",
            PlayerRole::Stopper => "스토퍼",
            PlayerRole::CoveringDefender => "커버링 수비수",
        }
    }

    /// Role 설명 (한글)
    pub fn description_ko(&self) -> &'static str {
        match self {
            PlayerRole::TargetMan => "강력한 피지컬과 헤딩으로 공을 받아 연결하는 타겟맨",
            PlayerRole::Poacher => "골 결정력에 집중하는 순수 스트라이커",
            PlayerRole::CompleteForward => "공격의 모든 면에서 균형잡힌 포워드",
            PlayerRole::Playmaker => "패싱과 비전으로 공격을 조율하는 플레이메이커",
            PlayerRole::BoxToBox => "공수 양쪽에서 활약하는 만능 미드필더",
            PlayerRole::BallWinning => "수비에 집중하며 볼을 탈취하는 미드필더",
            PlayerRole::BallPlayingDefender => "패싱으로 빌드업을 시작하는 수비수",
            PlayerRole::Stopper => "적극적인 태클로 공격을 차단하는 수비수",
            PlayerRole::CoveringDefender => "포지셔닝으로 공간을 커버하는 수비수",
        }
    }

    /// 이 Role이 적합한 포지션들 반환
    pub fn suitable_positions(&self) -> Vec<Position> {
        match self {
            // Forward roles - 공격수 전용
            PlayerRole::TargetMan | PlayerRole::Poacher | PlayerRole::CompleteForward => {
                vec![Position::ST]
            }

            // Midfielder roles - 미드필더용
            PlayerRole::Playmaker => {
                vec![Position::CAM, Position::CM, Position::RM, Position::LM]
            }
            PlayerRole::BoxToBox => {
                vec![Position::CM, Position::CDM]
            }
            PlayerRole::BallWinning => {
                vec![Position::CDM, Position::CM]
            }

            // Defender roles - 수비수용
            PlayerRole::BallPlayingDefender
            | PlayerRole::Stopper
            | PlayerRole::CoveringDefender => {
                vec![Position::CB, Position::LB, Position::RB, Position::LWB, Position::RWB]
            }
        }
    }

    /// 주어진 포지션에 적합한지 확인
    pub fn is_suitable_for(&self, position: &Position) -> bool {
        self.suitable_positions().contains(position)
    }

    /// 특정 포지션에 사용 가능한 모든 Role 반환
    pub fn available_for_position(position: &Position) -> Vec<PlayerRole> {
        let all_roles = vec![
            PlayerRole::TargetMan,
            PlayerRole::Poacher,
            PlayerRole::CompleteForward,
            PlayerRole::Playmaker,
            PlayerRole::BoxToBox,
            PlayerRole::BallWinning,
            PlayerRole::BallPlayingDefender,
            PlayerRole::Stopper,
            PlayerRole::CoveringDefender,
        ];

        all_roles.into_iter().filter(|role| role.is_suitable_for(position)).collect()
    }
}

/// Instructions를 PlayerAttributes modifier로 변환
pub fn apply_instructions_modifiers(
    base_attrs: &PlayerAttributes,
    instructions: &PlayerInstructions,
) -> PlayerAttributes {
    let mut attrs = base_attrs.clone();

    // 1. Mentality (공격 성향)
    match instructions.mentality {
        Mentality::Aggressive => {
            attrs.off_the_ball = (attrs.off_the_ball as i16 + 2).clamp(1, 20) as u8;
            attrs.positioning = (attrs.positioning as i16 - 1).clamp(1, 20) as u8;
            attrs.work_rate = (attrs.work_rate as i16 + 1).clamp(1, 20) as u8;
        }
        Mentality::Conservative => {
            attrs.off_the_ball = (attrs.off_the_ball as i16 - 1).clamp(1, 20) as u8;
            attrs.positioning = (attrs.positioning as i16 + 2).clamp(1, 20) as u8;
            attrs.concentration = (attrs.concentration as i16 + 1).clamp(1, 20) as u8;
        }
        Mentality::Balanced => {}
    }

    // 2. Width (좌우 포지셔닝)
    match instructions.width {
        Width::StayWide => {
            attrs.crossing = (attrs.crossing as i16 + 2).clamp(1, 20) as u8;
            attrs.dribbling = (attrs.dribbling as i16 + 1).clamp(1, 20) as u8;
        }
        Width::CutInside => {
            attrs.dribbling = (attrs.dribbling as i16 + 2).clamp(1, 20) as u8;
            attrs.long_shots = (attrs.long_shots as i16 + 1).clamp(1, 20) as u8;
        }
        Width::Roam => {
            attrs.off_the_ball = (attrs.off_the_ball as i16 + 1).clamp(1, 20) as u8;
            attrs.anticipation = (attrs.anticipation as i16 + 1).clamp(1, 20) as u8;
        }
        Width::Normal => {}
    }

    // 3. Depth (전후 포지셔닝)
    match instructions.depth {
        Depth::GetForward => {
            attrs.off_the_ball = (attrs.off_the_ball as i16 + 2).clamp(1, 20) as u8;
            attrs.stamina = (attrs.stamina as i16 - 1).clamp(1, 20) as u8;
        }
        Depth::StayBack => {
            attrs.positioning = (attrs.positioning as i16 + 2).clamp(1, 20) as u8;
            attrs.tackling = (attrs.tackling as i16 + 1).clamp(1, 20) as u8;
        }
        Depth::Balanced => {}
    }

    // 4. Passing (패싱 스타일)
    match instructions.passing {
        PassingStyle::Short => {
            attrs.first_touch = (attrs.first_touch as i16 + 2).clamp(1, 20) as u8;
            attrs.passing = (attrs.passing as i16 + 1).clamp(1, 20) as u8;
        }
        PassingStyle::Direct => {
            attrs.vision = (attrs.vision as i16 + 2).clamp(1, 20) as u8;
            attrs.passing = (attrs.passing as i16 + 1).clamp(1, 20) as u8;
        }
        PassingStyle::Mixed => {}
    }

    // 5. Dribbling (드리블 빈도)
    match instructions.dribbling {
        DribblingFrequency::Often => {
            attrs.dribbling = (attrs.dribbling as i16 + 3).clamp(1, 20) as u8;
            attrs.flair = (attrs.flair as i16 + 2).clamp(1, 20) as u8;
            attrs.technique = (attrs.technique as i16 + 1).clamp(1, 20) as u8;
        }
        DribblingFrequency::Rarely => {
            attrs.passing = (attrs.passing as i16 + 1).clamp(1, 20) as u8;
        }
        DribblingFrequency::Normal => {}
    }

    // 6. Shooting (슈팅 성향)
    match instructions.shooting {
        ShootingTendency::ShootOnSight => {
            attrs.long_shots = (attrs.long_shots as i16 + 2).clamp(1, 20) as u8;
            attrs.finishing = (attrs.finishing as i16 + 1).clamp(1, 20) as u8;
        }
        ShootingTendency::Conservative => {
            attrs.composure = (attrs.composure as i16 + 1).clamp(1, 20) as u8;
            attrs.decisions = (attrs.decisions as i16 + 1).clamp(1, 20) as u8;
        }
        ShootingTendency::Normal => {}
    }

    // 7. Defensive Work (수비 기여도)
    match instructions.defensive_work {
        DefensiveWork::High => {
            attrs.tackling = (attrs.tackling as i16 + 2).clamp(1, 20) as u8;
            attrs.marking = (attrs.marking as i16 + 2).clamp(1, 20) as u8;
            attrs.work_rate = (attrs.work_rate as i16 + 2).clamp(1, 20) as u8;
            attrs.stamina = (attrs.stamina as i16 - 1).clamp(1, 20) as u8;
        }
        DefensiveWork::Minimal => {
            attrs.off_the_ball = (attrs.off_the_ball as i16 + 1).clamp(1, 20) as u8;
        }
        DefensiveWork::Normal => {}
    }

    // 8. Pressing (압박 강도)
    match instructions.pressing {
        PressingIntensity::High => {
            attrs.work_rate = (attrs.work_rate as i16 + 2).clamp(1, 20) as u8;
            attrs.aggression = (attrs.aggression as i16 + 2).clamp(1, 20) as u8;
            attrs.stamina = (attrs.stamina as i16 - 1).clamp(1, 20) as u8;
        }
        PressingIntensity::Low => {
            attrs.positioning = (attrs.positioning as i16 + 1).clamp(1, 20) as u8;
        }
        PressingIntensity::Medium => {}
    }

    attrs
}

// FromStr implementations for all enums

impl FromStr for Mentality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "conservative" => Ok(Mentality::Conservative),
            "balanced" => Ok(Mentality::Balanced),
            "aggressive" => Ok(Mentality::Aggressive),
            _ => Err(format!("Invalid Mentality: {}", s)),
        }
    }
}

impl FromStr for Width {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stay_wide" => Ok(Width::StayWide),
            "normal" => Ok(Width::Normal),
            "cut_inside" => Ok(Width::CutInside),
            "roam" => Ok(Width::Roam),
            _ => Err(format!("Invalid Width: {}", s)),
        }
    }
}

impl FromStr for Depth {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stay_back" => Ok(Depth::StayBack),
            "balanced" => Ok(Depth::Balanced),
            "get_forward" => Ok(Depth::GetForward),
            _ => Err(format!("Invalid Depth: {}", s)),
        }
    }
}

impl FromStr for PassingStyle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "short" => Ok(PassingStyle::Short),
            "mixed" => Ok(PassingStyle::Mixed),
            "direct" => Ok(PassingStyle::Direct),
            _ => Err(format!("Invalid PassingStyle: {}", s)),
        }
    }
}

impl FromStr for DribblingFrequency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rarely" => Ok(DribblingFrequency::Rarely),
            "normal" => Ok(DribblingFrequency::Normal),
            "often" => Ok(DribblingFrequency::Often),
            _ => Err(format!("Invalid DribblingFrequency: {}", s)),
        }
    }
}

impl FromStr for ShootingTendency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "conservative" => Ok(ShootingTendency::Conservative),
            "normal" => Ok(ShootingTendency::Normal),
            "shoot_on_sight" => Ok(ShootingTendency::ShootOnSight),
            _ => Err(format!("Invalid ShootingTendency: {}", s)),
        }
    }
}

impl FromStr for DefensiveWork {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "minimal" => Ok(DefensiveWork::Minimal),
            "normal" => Ok(DefensiveWork::Normal),
            "high" => Ok(DefensiveWork::High),
            _ => Err(format!("Invalid DefensiveWork: {}", s)),
        }
    }
}

impl FromStr for PressingIntensity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(PressingIntensity::Low),
            "medium" => Ok(PressingIntensity::Medium),
            "high" => Ok(PressingIntensity::High),
            _ => Err(format!("Invalid PressingIntensity: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_preset() {
        let target_man = PlayerRole::TargetMan;
        let instructions = target_man.default_instructions();

        assert_eq!(instructions.mentality, Mentality::Aggressive);
        assert_eq!(instructions.depth, Depth::GetForward);
        assert_eq!(instructions.shooting, ShootingTendency::ShootOnSight);
    }

    #[test]
    fn test_instructions_modifier() {
        let base = PlayerAttributes { off_the_ball: 10, positioning: 10, ..Default::default() };

        let instructions =
            PlayerInstructions { mentality: Mentality::Aggressive, ..Default::default() };

        let modified = apply_instructions_modifiers(&base, &instructions);

        assert_eq!(modified.off_the_ball, 12); // +2
        assert_eq!(modified.positioning, 9); // -1
    }

    #[test]
    fn test_role_names() {
        assert_eq!(PlayerRole::TargetMan.display_name_ko(), "타겟맨");
        assert!(PlayerRole::TargetMan.description_ko().contains("타겟맨"));
    }
}
