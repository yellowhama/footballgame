// crates/of_core/src/tactics/openfootball_bridge.rs
// Bridge between OpenFootball tactical system and our Godot integration
// Provides 14 formations with position mappings and Korean translations

use serde::{Deserialize, Serialize};

/// Re-export OpenFootball types for convenience
/// Note: These would normally be imported from open-football crate
/// For now, we define simplified versions that match OpenFootball's structure

/// OpenFootball's position types (22 positions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerPositionType {
    Goalkeeper,
    Sweeper,
    DefenderLeft,
    DefenderCenterLeft,
    DefenderCenter,
    DefenderCenterRight,
    DefenderRight,
    DefensiveMidfielder,
    MidfielderLeft,
    MidfielderCenterLeft,
    MidfielderCenter,
    MidfielderCenterRight,
    MidfielderRight,
    AttackingMidfielderLeft,
    AttackingMidfielderCenter,
    AttackingMidfielderRight,
    WingbackLeft,
    WingbackRight,
    Striker,
    ForwardLeft,
    ForwardCenter,
    ForwardRight,
}

impl PlayerPositionType {
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Goalkeeper => "GK",
            Self::Sweeper => "SW",
            Self::DefenderLeft => "LB",
            Self::DefenderCenterLeft => "LCB",
            Self::DefenderCenter => "CB",
            Self::DefenderCenterRight => "RCB",
            Self::DefenderRight => "RB",
            Self::DefensiveMidfielder => "CDM",
            Self::MidfielderLeft => "LM",
            Self::MidfielderCenterLeft => "LCM",
            Self::MidfielderCenter => "CM",
            Self::MidfielderCenterRight => "RCM",
            Self::MidfielderRight => "RM",
            Self::AttackingMidfielderLeft => "LAM",
            Self::AttackingMidfielderCenter => "CAM",
            Self::AttackingMidfielderRight => "RAM",
            Self::WingbackLeft => "LWB",
            Self::WingbackRight => "RWB",
            Self::Striker => "ST",
            Self::ForwardLeft => "LW",
            Self::ForwardCenter => "CF",
            Self::ForwardRight => "RW",
        }
    }

    pub fn korean_name(&self) -> &'static str {
        match self {
            Self::Goalkeeper => "골키퍼",
            Self::Sweeper => "스위퍼",
            Self::DefenderLeft => "왼쪽 풀백",
            Self::DefenderCenterLeft => "왼쪽 센터백",
            Self::DefenderCenter => "센터백",
            Self::DefenderCenterRight => "오른쪽 센터백",
            Self::DefenderRight => "오른쪽 풀백",
            Self::DefensiveMidfielder => "수비형 미드필더",
            Self::MidfielderLeft => "왼쪽 미드필더",
            Self::MidfielderCenterLeft => "왼쪽 중앙 미드필더",
            Self::MidfielderCenter => "중앙 미드필더",
            Self::MidfielderCenterRight => "오른쪽 중앙 미드필더",
            Self::MidfielderRight => "오른쪽 미드필더",
            Self::AttackingMidfielderLeft => "왼쪽 공격형 미드필더",
            Self::AttackingMidfielderCenter => "중앙 공격형 미드필더",
            Self::AttackingMidfielderRight => "오른쪽 공격형 미드필더",
            Self::WingbackLeft => "왼쪽 윙백",
            Self::WingbackRight => "오른쪽 윙백",
            Self::Striker => "스트라이커",
            Self::ForwardLeft => "왼쪽 윙어",
            Self::ForwardCenter => "중앙 공격수",
            Self::ForwardRight => "오른쪽 윙어",
        }
    }

    /// Alias for korean_name for API compatibility
    pub fn display_name_ko(&self) -> &'static str {
        self.korean_name()
    }

    /// Alias for short_name for API compatibility
    pub fn get_short_name(&self) -> &'static str {
        self.short_name()
    }
}

/// OpenFootball's formation types (14 formations)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatchTacticType {
    T442,
    T433,
    T451,
    T4231,
    T352,
    T442Diamond,
    T442DiamondWide,
    T442Narrow,
    T4141,
    T4411,
    T343,
    T1333,
    T4312,
    T4222,
}

impl MatchTacticType {
    pub fn all() -> Vec<MatchTacticType> {
        vec![
            Self::T442,
            Self::T433,
            Self::T451,
            Self::T4231,
            Self::T352,
            Self::T442Diamond,
            Self::T442DiamondWide,
            Self::T442Narrow,
            Self::T4141,
            Self::T4411,
            Self::T343,
            Self::T1333,
            Self::T4312,
            Self::T4222,
        ]
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::T442 => "T442",
            Self::T433 => "T433",
            Self::T451 => "T451",
            Self::T4231 => "T4231",
            Self::T352 => "T352",
            Self::T442Diamond => "T442Diamond",
            Self::T442DiamondWide => "T442DiamondWide",
            Self::T442Narrow => "T442Narrow",
            Self::T4141 => "T4141",
            Self::T4411 => "T4411",
            Self::T343 => "T343",
            Self::T1333 => "T1333",
            Self::T4312 => "T4312",
            Self::T4222 => "T4222",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::T442 => "4-4-2",
            Self::T433 => "4-3-3",
            Self::T451 => "4-5-1",
            Self::T4231 => "4-2-3-1",
            Self::T352 => "3-5-2",
            Self::T442Diamond => "4-4-2 Diamond",
            Self::T442DiamondWide => "4-4-2 Diamond Wide",
            Self::T442Narrow => "4-4-2 Narrow",
            Self::T4141 => "4-1-4-1",
            Self::T4411 => "4-4-1-1",
            Self::T343 => "3-4-3",
            Self::T1333 => "1-3-3-3",
            Self::T4312 => "4-3-1-2",
            Self::T4222 => "4-2-2-2",
        }
    }

    pub fn korean_name(&self) -> &'static str {
        match self {
            Self::T442 => "4-4-2",
            Self::T433 => "4-3-3",
            Self::T451 => "4-5-1",
            Self::T4231 => "4-2-3-1",
            Self::T352 => "3-5-2",
            Self::T442Diamond => "4-4-2 다이아몬드",
            Self::T442DiamondWide => "4-4-2 다이아몬드 와이드",
            Self::T442Narrow => "4-4-2 좁게",
            Self::T4141 => "4-1-4-1",
            Self::T4411 => "4-4-1-1",
            Self::T343 => "3-4-3",
            Self::T1333 => "1-3-3-3",
            Self::T4312 => "4-3-1-2",
            Self::T4222 => "4-2-2-2",
        }
    }

    pub fn description_korean(&self) -> &'static str {
        match self {
            Self::T442 => "가장 균형잡힌 기본 포메이션. 수비와 공격의 밸런스가 좋습니다.",
            Self::T433 => "공격적인 포메이션. 3명의 공격수로 측면 공격을 강화합니다.",
            Self::T451 => "수비적인 포메이션. 중원을 두텁게 하여 수비를 안정화합니다.",
            Self::T4231 => "점유율 중심 포메이션. 공격형 미드필더를 활용한 플레이메이킹.",
            Self::T352 => "윙백을 활용한 포메이션. 측면 공격과 수비를 동시에 강화합니다.",
            Self::T442Diamond => "다이아몬드 형태의 중원. 중앙을 장악하는 플레이.",
            Self::T442DiamondWide => "넓은 다이아몬드. 측면과 중앙을 모두 활용합니다.",
            Self::T442Narrow => "좁은 4-4-2. 중앙을 압축하여 수비를 견고하게 합니다.",
            Self::T4141 => "수비형 미드필더 중심. 안정적인 수비 후 역습을 노립니다.",
            Self::T4411 => "세컨드 스트라이커를 활용. 역습형 포메이션입니다.",
            Self::T343 => "매우 공격적인 3백 시스템. 공격에 많은 선수를 투입합니다.",
            Self::T1333 => "실험적인 포메이션. 모든 라인에 3명씩 배치합니다.",
            Self::T4312 => "중앙 공격수 중심. 점유율을 높이고 중앙 돌파를 노립니다.",
            Self::T4222 => "넓은 공격 포메이션. 양쪽 측면 공격을 강화합니다.",
        }
    }

    /// Alias for korean_name for API compatibility
    pub fn display_name_ko(&self) -> &'static str {
        self.korean_name()
    }

    /// Alias for description_korean for API compatibility
    pub fn description_ko(&self) -> &'static str {
        self.description_korean()
    }

    /// Alias for tactical_style for API compatibility
    pub fn default_style(&self) -> TacticalStyle {
        self.tactical_style()
    }

    /// Get strengths of this formation
    pub fn get_strengths(&self) -> Vec<&'static str> {
        match self {
            Self::T442 => vec!["균형감", "간단한 운영", "다재다능"],
            Self::T433 => vec!["측면 공격", "압박 강도", "공격 옵션"],
            Self::T451 => vec!["중원 장악", "수비 안정성", "볼 소유"],
            Self::T4231 => vec!["창의적인 플레이", "수비 밸런스", "역습 효과"],
            Self::T352 => vec!["윙백 활용", "중원 수적 우위", "유연성"],
            Self::T442Diamond => vec!["중앙 지배력", "연결 플레이", "공격형 미드필더 활용"],
            Self::T442DiamondWide => vec!["폭넓은 공격", "중원 장악", "다양한 공격 루트"],
            Self::T442Narrow => vec!["중앙 압박", "견고한 수비", "카운터 어택"],
            Self::T4141 => vec!["수비 안정성", "중원 통제", "역습 효율"],
            Self::T4411 => vec!["세컨드 스트라이커", "빠른 전환", "역습 위협"],
            Self::T343 => vec!["공격력", "폭넓은 공격", "압박 강도"],
            Self::T1333 => vec!["예측 불가", "균형 배치", "실험적 플레이"],
            Self::T4312 => vec!["중앙 돌파", "점유율", "플레이메이킹"],
            Self::T4222 => vec!["측면 공격", "넓은 전개", "공격적 플레이"],
        }
    }

    /// Get weaknesses of this formation
    pub fn get_weaknesses(&self) -> Vec<&'static str> {
        match self {
            Self::T442 => vec!["중원 수적 열세", "측면 취약", "현대 축구에 다소 진부"],
            Self::T433 => vec!["수비 취약", "중원 약함", "측면 수비 공간"],
            Self::T451 => vec!["공격력 부족", "고립된 공격수", "골 결정력"],
            Self::T4231 => vec!["물리적 약함", "측면 수비", "전방 압박 약함"],
            Self::T352 => vec!["측면 수비 공간", "윙백 의존", "전술 이해 필요"],
            Self::T442Diamond => vec!["측면 약함", "전술 이해 필요", "물리적 중원 필요"],
            Self::T442DiamondWide => vec!["중앙 약함", "선수 간 거리", "전환 속도"],
            Self::T442Narrow => vec!["측면 약함", "공격 폭 부족", "크로스 대응"],
            Self::T4141 => vec!["공격력 부족", "창의성 부족", "소극적 플레이"],
            Self::T4411 => vec!["고립된 공격수", "측면 부족", "수비적 플레이"],
            Self::T343 => vec!["수비 취약", "역습 위험", "수비 조직 약함"],
            Self::T1333 => vec!["증명 안됨", "수비 조직", "전술 이해 어려움"],
            Self::T4312 => vec!["측면 약함", "수비 전환", "물리적 약함"],
            Self::T4222 => vec!["중앙 약함", "수비 밸런스", "전환 수비"],
        }
    }
}

/// Tactical style categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TacticalStyle {
    Attacking,
    Defensive,
    Balanced,
    Possession,
    Counterattack,
    WingPlay,
    WidePlay,
    Compact,
    Experimental,
}

impl TacticalStyle {
    pub fn korean_name(&self) -> &'static str {
        match self {
            Self::Attacking => "공격형",
            Self::Defensive => "수비형",
            Self::Balanced => "균형형",
            Self::Possession => "점유율",
            Self::Counterattack => "역습형",
            Self::WingPlay => "측면 플레이",
            Self::WidePlay => "넓은 플레이",
            Self::Compact => "압축형",
            Self::Experimental => "실험형",
        }
    }
}

impl MatchTacticType {
    pub fn tactical_style(&self) -> TacticalStyle {
        match self {
            Self::T442 | Self::T442Diamond | Self::T442DiamondWide => TacticalStyle::Balanced,
            Self::T433 | Self::T343 => TacticalStyle::Attacking,
            Self::T451 | Self::T4141 => TacticalStyle::Defensive,
            Self::T352 => TacticalStyle::WingPlay,
            Self::T4231 | Self::T4312 => TacticalStyle::Possession,
            Self::T442Narrow => TacticalStyle::Compact,
            Self::T4411 => TacticalStyle::Counterattack,
            Self::T1333 => TacticalStyle::Experimental,
            Self::T4222 => TacticalStyle::WidePlay,
        }
    }

    pub fn defender_count(&self) -> usize {
        match self {
            Self::T442
            | Self::T433
            | Self::T451
            | Self::T4231
            | Self::T442Diamond
            | Self::T442DiamondWide
            | Self::T442Narrow
            | Self::T4141
            | Self::T4411
            | Self::T4312
            | Self::T4222 => 4,
            Self::T352 | Self::T343 | Self::T1333 => 3,
        }
    }

    pub fn midfielder_count(&self) -> usize {
        match self {
            Self::T442
            | Self::T442Diamond
            | Self::T442DiamondWide
            | Self::T442Narrow
            | Self::T4411 => 4,
            Self::T433 | Self::T4312 => 3,
            Self::T451 => 5,
            Self::T4231 | Self::T4222 => 2, // Base midfielders only
            Self::T352 | Self::T1333 => 5,
            Self::T4141 => 1, // Just the CDM as base
            Self::T343 => 4,
        }
    }

    pub fn forward_count(&self) -> usize {
        match self {
            Self::T442
            | Self::T442Diamond
            | Self::T442DiamondWide
            | Self::T442Narrow
            | Self::T352
            | Self::T4312
            | Self::T4222 => 2,
            Self::T433 | Self::T343 | Self::T1333 => 3,
            Self::T451 | Self::T4141 | Self::T4411 | Self::T4231 => 1,
        }
    }
}

/// Position with visualization coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionWithCoords {
    pub slot: usize,
    pub position_type: PlayerPositionType,
    pub x: f32, // 0.0 = left touchline, 1.0 = right touchline
    pub y: f32, // 0.0 = own goal, 1.0 = opponent goal
}

impl PositionWithCoords {
    pub fn new(slot: usize, position_type: PlayerPositionType, x: f32, y: f32) -> Self {
        Self { slot, position_type, x: x.clamp(0.0, 1.0), y: y.clamp(0.0, 1.0) }
    }
}

/// Complete formation data with positions and coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationData {
    pub formation_type: MatchTacticType,
    pub positions: Vec<PositionWithCoords>,
}

impl FormationData {
    pub fn new(formation_type: MatchTacticType, positions: Vec<PositionWithCoords>) -> Self {
        Self { formation_type, positions }
    }

    /// Get formation data for all 14 formations
    pub fn all_formations() -> Vec<FormationData> {
        vec![
            Self::create_t442(),
            Self::create_t433(),
            Self::create_t451(),
            Self::create_t4231(),
            Self::create_t352(),
            Self::create_t442_diamond(),
            Self::create_t442_diamond_wide(),
            Self::create_t442_narrow(),
            Self::create_t4141(),
            Self::create_t4411(),
            Self::create_t343(),
            Self::create_t1333(),
            Self::create_t4312(),
            Self::create_t4222(),
        ]
    }

    /// Get formation data for a specific tactic type
    pub fn for_tactic(tactic: MatchTacticType) -> FormationData {
        match tactic {
            MatchTacticType::T442 => Self::create_t442(),
            MatchTacticType::T433 => Self::create_t433(),
            MatchTacticType::T451 => Self::create_t451(),
            MatchTacticType::T4231 => Self::create_t4231(),
            MatchTacticType::T352 => Self::create_t352(),
            MatchTacticType::T442Diamond => Self::create_t442_diamond(),
            MatchTacticType::T442DiamondWide => Self::create_t442_diamond_wide(),
            MatchTacticType::T442Narrow => Self::create_t442_narrow(),
            MatchTacticType::T4141 => Self::create_t4141(),
            MatchTacticType::T4411 => Self::create_t4411(),
            MatchTacticType::T343 => Self::create_t343(),
            MatchTacticType::T1333 => Self::create_t1333(),
            MatchTacticType::T4312 => Self::create_t4312(),
            MatchTacticType::T4222 => Self::create_t4222(),
        }
    }

    // ============================================================================
    // Formation Definitions (14 formations)
    // ============================================================================

    /// 4-4-2 Standard (Balanced)
    fn create_t442() -> FormationData {
        FormationData::new(
            MatchTacticType::T442,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::MidfielderLeft, 0.15, 0.5),
                PositionWithCoords::new(6, PlayerPositionType::MidfielderCenterLeft, 0.4, 0.5),
                PositionWithCoords::new(7, PlayerPositionType::MidfielderCenterRight, 0.6, 0.5),
                PositionWithCoords::new(8, PlayerPositionType::MidfielderRight, 0.85, 0.5),
                PositionWithCoords::new(9, PlayerPositionType::ForwardLeft, 0.35, 0.8),
                PositionWithCoords::new(10, PlayerPositionType::ForwardRight, 0.65, 0.8),
            ],
        )
    }

    /// 4-3-3 (Attacking)
    fn create_t433() -> FormationData {
        FormationData::new(
            MatchTacticType::T433,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::MidfielderCenterLeft, 0.35, 0.45),
                PositionWithCoords::new(6, PlayerPositionType::MidfielderCenter, 0.5, 0.45),
                PositionWithCoords::new(7, PlayerPositionType::MidfielderCenterRight, 0.65, 0.45),
                PositionWithCoords::new(8, PlayerPositionType::ForwardLeft, 0.15, 0.8),
                PositionWithCoords::new(9, PlayerPositionType::ForwardCenter, 0.5, 0.85),
                PositionWithCoords::new(10, PlayerPositionType::ForwardRight, 0.85, 0.8),
            ],
        )
    }

    /// 4-5-1 (Defensive)
    fn create_t451() -> FormationData {
        FormationData::new(
            MatchTacticType::T451,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::MidfielderLeft, 0.15, 0.5),
                PositionWithCoords::new(6, PlayerPositionType::MidfielderCenterLeft, 0.35, 0.5),
                PositionWithCoords::new(7, PlayerPositionType::MidfielderCenter, 0.5, 0.5),
                PositionWithCoords::new(8, PlayerPositionType::MidfielderCenterRight, 0.65, 0.5),
                PositionWithCoords::new(9, PlayerPositionType::MidfielderRight, 0.85, 0.5),
                PositionWithCoords::new(10, PlayerPositionType::Striker, 0.5, 0.8),
            ],
        )
    }

    /// 4-2-3-1 (Possession)
    fn create_t4231() -> FormationData {
        FormationData::new(
            MatchTacticType::T4231,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::DefensiveMidfielder, 0.4, 0.35),
                PositionWithCoords::new(6, PlayerPositionType::MidfielderCenter, 0.6, 0.35),
                PositionWithCoords::new(7, PlayerPositionType::AttackingMidfielderLeft, 0.2, 0.6),
                PositionWithCoords::new(8, PlayerPositionType::AttackingMidfielderCenter, 0.5, 0.6),
                PositionWithCoords::new(9, PlayerPositionType::AttackingMidfielderRight, 0.8, 0.6),
                PositionWithCoords::new(10, PlayerPositionType::Striker, 0.5, 0.85),
            ],
        )
    }

    /// 3-5-2 (Wing Play)
    fn create_t352() -> FormationData {
        FormationData::new(
            MatchTacticType::T352,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderCenterLeft, 0.35, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenter, 0.5, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.65, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::WingbackLeft, 0.1, 0.45),
                PositionWithCoords::new(5, PlayerPositionType::MidfielderCenterLeft, 0.35, 0.5),
                PositionWithCoords::new(6, PlayerPositionType::MidfielderCenter, 0.5, 0.5),
                PositionWithCoords::new(7, PlayerPositionType::MidfielderCenterRight, 0.65, 0.5),
                PositionWithCoords::new(8, PlayerPositionType::WingbackRight, 0.9, 0.45),
                PositionWithCoords::new(9, PlayerPositionType::ForwardLeft, 0.4, 0.8),
                PositionWithCoords::new(10, PlayerPositionType::ForwardRight, 0.6, 0.8),
            ],
        )
    }

    /// 4-4-2 Diamond (Balanced)
    fn create_t442_diamond() -> FormationData {
        FormationData::new(
            MatchTacticType::T442Diamond,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::DefensiveMidfielder, 0.5, 0.35), // Bottom of diamond
                PositionWithCoords::new(6, PlayerPositionType::MidfielderLeft, 0.3, 0.5), // Left of diamond
                PositionWithCoords::new(7, PlayerPositionType::MidfielderRight, 0.7, 0.5), // Right of diamond
                PositionWithCoords::new(
                    8,
                    PlayerPositionType::AttackingMidfielderCenter,
                    0.5,
                    0.65,
                ), // Top of diamond
                PositionWithCoords::new(9, PlayerPositionType::ForwardLeft, 0.4, 0.85),
                PositionWithCoords::new(10, PlayerPositionType::ForwardRight, 0.6, 0.85),
            ],
        )
    }

    /// 4-4-2 Diamond Wide (Wide Play)
    fn create_t442_diamond_wide() -> FormationData {
        FormationData::new(
            MatchTacticType::T442DiamondWide,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::DefensiveMidfielder, 0.5, 0.35),
                PositionWithCoords::new(6, PlayerPositionType::MidfielderLeft, 0.15, 0.5), // Wider
                PositionWithCoords::new(7, PlayerPositionType::MidfielderRight, 0.85, 0.5), // Wider
                PositionWithCoords::new(
                    8,
                    PlayerPositionType::AttackingMidfielderCenter,
                    0.5,
                    0.65,
                ),
                PositionWithCoords::new(9, PlayerPositionType::ForwardLeft, 0.4, 0.85),
                PositionWithCoords::new(10, PlayerPositionType::ForwardRight, 0.6, 0.85),
            ],
        )
    }

    /// 4-4-2 Narrow (Compact)
    fn create_t442_narrow() -> FormationData {
        FormationData::new(
            MatchTacticType::T442Narrow,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::MidfielderCenterLeft, 0.35, 0.5), // Narrower
                PositionWithCoords::new(6, PlayerPositionType::MidfielderCenter, 0.5, 0.45),
                PositionWithCoords::new(7, PlayerPositionType::MidfielderCenterRight, 0.65, 0.5), // Narrower
                PositionWithCoords::new(8, PlayerPositionType::AttackingMidfielderCenter, 0.5, 0.6),
                PositionWithCoords::new(9, PlayerPositionType::ForwardLeft, 0.4, 0.8),
                PositionWithCoords::new(10, PlayerPositionType::ForwardRight, 0.6, 0.8),
            ],
        )
    }

    /// 4-1-4-1 (Defensive)
    fn create_t4141() -> FormationData {
        FormationData::new(
            MatchTacticType::T4141,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::DefensiveMidfielder, 0.5, 0.35), // Single CDM
                PositionWithCoords::new(6, PlayerPositionType::MidfielderLeft, 0.15, 0.55),
                PositionWithCoords::new(7, PlayerPositionType::MidfielderCenterLeft, 0.4, 0.55),
                PositionWithCoords::new(8, PlayerPositionType::MidfielderCenterRight, 0.6, 0.55),
                PositionWithCoords::new(9, PlayerPositionType::MidfielderRight, 0.85, 0.55),
                PositionWithCoords::new(10, PlayerPositionType::Striker, 0.5, 0.85), // Single ST
            ],
        )
    }

    /// 4-4-1-1 (Counterattack)
    fn create_t4411() -> FormationData {
        FormationData::new(
            MatchTacticType::T4411,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::MidfielderLeft, 0.15, 0.5),
                PositionWithCoords::new(6, PlayerPositionType::MidfielderCenterLeft, 0.4, 0.5),
                PositionWithCoords::new(7, PlayerPositionType::MidfielderCenterRight, 0.6, 0.5),
                PositionWithCoords::new(8, PlayerPositionType::MidfielderRight, 0.85, 0.5),
                PositionWithCoords::new(9, PlayerPositionType::AttackingMidfielderCenter, 0.5, 0.7), // Second striker
                PositionWithCoords::new(10, PlayerPositionType::Striker, 0.5, 0.85),
            ],
        )
    }

    /// 3-4-3 (Very Attacking)
    fn create_t343() -> FormationData {
        FormationData::new(
            MatchTacticType::T343,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderCenterLeft, 0.35, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenter, 0.5, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.65, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::WingbackLeft, 0.15, 0.5),
                PositionWithCoords::new(5, PlayerPositionType::MidfielderCenterLeft, 0.4, 0.5),
                PositionWithCoords::new(6, PlayerPositionType::MidfielderCenterRight, 0.6, 0.5),
                PositionWithCoords::new(7, PlayerPositionType::WingbackRight, 0.85, 0.5),
                PositionWithCoords::new(8, PlayerPositionType::ForwardLeft, 0.2, 0.8),
                PositionWithCoords::new(9, PlayerPositionType::ForwardCenter, 0.5, 0.85),
                PositionWithCoords::new(10, PlayerPositionType::ForwardRight, 0.8, 0.8),
            ],
        )
    }

    /// 1-3-3-3 (Experimental)
    fn create_t1333() -> FormationData {
        FormationData::new(
            MatchTacticType::T1333,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderCenterLeft, 0.35, 0.25),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenter, 0.5, 0.25),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.65, 0.25),
                PositionWithCoords::new(4, PlayerPositionType::MidfielderLeft, 0.25, 0.5),
                PositionWithCoords::new(5, PlayerPositionType::MidfielderCenter, 0.5, 0.5),
                PositionWithCoords::new(6, PlayerPositionType::MidfielderRight, 0.75, 0.5),
                PositionWithCoords::new(7, PlayerPositionType::ForwardLeft, 0.25, 0.75),
                PositionWithCoords::new(8, PlayerPositionType::ForwardCenter, 0.5, 0.75),
                PositionWithCoords::new(9, PlayerPositionType::ForwardRight, 0.75, 0.75),
                PositionWithCoords::new(10, PlayerPositionType::Striker, 0.5, 0.9), // Extra forward
            ],
        )
    }

    /// 4-3-1-2 (Possession)
    fn create_t4312() -> FormationData {
        FormationData::new(
            MatchTacticType::T4312,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::MidfielderCenterLeft, 0.35, 0.4),
                PositionWithCoords::new(6, PlayerPositionType::MidfielderCenter, 0.5, 0.4),
                PositionWithCoords::new(7, PlayerPositionType::MidfielderCenterRight, 0.65, 0.4),
                PositionWithCoords::new(
                    8,
                    PlayerPositionType::AttackingMidfielderCenter,
                    0.5,
                    0.65,
                ), // Central CAM
                PositionWithCoords::new(9, PlayerPositionType::ForwardLeft, 0.4, 0.85),
                PositionWithCoords::new(10, PlayerPositionType::ForwardRight, 0.6, 0.85),
            ],
        )
    }

    /// 4-2-2-2 (Wide Play)
    fn create_t4222() -> FormationData {
        FormationData::new(
            MatchTacticType::T4222,
            vec![
                PositionWithCoords::new(0, PlayerPositionType::Goalkeeper, 0.5, 0.05),
                PositionWithCoords::new(1, PlayerPositionType::DefenderLeft, 0.2, 0.2),
                PositionWithCoords::new(2, PlayerPositionType::DefenderCenterLeft, 0.4, 0.2),
                PositionWithCoords::new(3, PlayerPositionType::DefenderCenterRight, 0.6, 0.2),
                PositionWithCoords::new(4, PlayerPositionType::DefenderRight, 0.8, 0.2),
                PositionWithCoords::new(5, PlayerPositionType::DefensiveMidfielder, 0.4, 0.35), // CDM left
                PositionWithCoords::new(6, PlayerPositionType::MidfielderCenter, 0.6, 0.35), // CDM right
                PositionWithCoords::new(7, PlayerPositionType::AttackingMidfielderLeft, 0.25, 0.6), // CAM left
                PositionWithCoords::new(8, PlayerPositionType::AttackingMidfielderRight, 0.75, 0.6), // CAM right
                PositionWithCoords::new(9, PlayerPositionType::ForwardLeft, 0.35, 0.85),
                PositionWithCoords::new(10, PlayerPositionType::ForwardRight, 0.65, 0.85),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_formations_have_11_positions() {
        let formations = FormationData::all_formations();
        assert_eq!(formations.len(), 14, "Should have 14 formations");

        for formation in formations.iter() {
            assert_eq!(
                formation.positions.len(),
                11,
                "Formation {:?} should have 11 positions",
                formation.formation_type
            );
        }
    }

    #[test]
    fn test_position_coordinates_in_range() {
        let formations = FormationData::all_formations();

        for formation in formations.iter() {
            for pos in &formation.positions {
                assert!(
                    pos.x >= 0.0 && pos.x <= 1.0,
                    "Formation {:?} position {} x coordinate out of range: {}",
                    formation.formation_type,
                    pos.slot,
                    pos.x
                );
                assert!(
                    pos.y >= 0.0 && pos.y <= 1.0,
                    "Formation {:?} position {} y coordinate out of range: {}",
                    formation.formation_type,
                    pos.slot,
                    pos.y
                );
            }
        }
    }

    #[test]
    fn test_formation_counts() {
        assert_eq!(MatchTacticType::T442.defender_count(), 4);
        assert_eq!(MatchTacticType::T442.midfielder_count(), 4);
        assert_eq!(MatchTacticType::T442.forward_count(), 2);

        assert_eq!(MatchTacticType::T433.defender_count(), 4);
        assert_eq!(MatchTacticType::T433.midfielder_count(), 3);
        assert_eq!(MatchTacticType::T433.forward_count(), 3);

        assert_eq!(MatchTacticType::T352.defender_count(), 3);
        assert_eq!(MatchTacticType::T352.midfielder_count(), 5);
        assert_eq!(MatchTacticType::T352.forward_count(), 2);
    }

    #[test]
    fn test_tactical_styles() {
        assert_eq!(MatchTacticType::T442.tactical_style(), TacticalStyle::Balanced);
        assert_eq!(MatchTacticType::T433.tactical_style(), TacticalStyle::Attacking);
        assert_eq!(MatchTacticType::T451.tactical_style(), TacticalStyle::Defensive);
        assert_eq!(MatchTacticType::T4231.tactical_style(), TacticalStyle::Possession);
    }

    #[test]
    fn test_korean_translations_exist() {
        for tactic in MatchTacticType::all() {
            assert!(!tactic.korean_name().is_empty());
            assert!(!tactic.description_korean().is_empty());
        }
    }
}
