//! Tactical Adjuster - 경기 중 전술 조정 시스템
//!
//! Open-Football의 `TacticsSelector`를 참고하여 구현.
//! 점수차/시간/상대 포메이션에 따라 전술을 동적으로 조정.
//!
//! ## 주요 기능
//! - Phase 4D-1: 기본 구조 (TacticalAdjuster, TacticalAdjustmentReason)
//! - Phase 4D-2: 점수/시간 기반 변경 트리거
//! - Phase 4D-3: 상대 포메이션 대응 (Counter-Tactic)
//! - Phase 4D-4: 감독 속성 연동

use crate::engine::positioning::PositionKey;
use crate::engine::role_transition::{RoleTransitionMatrix, TacticalStyle};
use crate::engine::movement::slot_to_position_key;

// ============================================================================
// Constants
// ============================================================================

/// 전술 변경 시 포메이션 적합도 페널티 (0.8 = 20% 감소)
pub const TACTICAL_CHANGE_PENALTY: f32 = 0.8;

/// 최소 전술 변경 간격 (분)
pub const MIN_ADJUSTMENT_INTERVAL_MINUTES: u8 = 10;

/// 점수차 임계값: 크게 지고 있음
pub const LOSING_BADLY_THRESHOLD: i8 = -2;

/// 점수차 임계값: 지고 있음
pub const LOSING_THRESHOLD: i8 = -1;

/// 점수차 임계값: 이기고 있음
pub const WINNING_THRESHOLD: i8 = 1;

/// 점수차 임계값: 크게 이기고 있음
pub const WINNING_COMFORTABLY_THRESHOLD: i8 = 2;

/// 복잡한 포메이션(3-4-3, 4-2-3-1)에 필요한 최소 전술 지식
pub const COMPLEX_FORMATION_MIN_KNOWLEDGE: u8 = 15;

/// 매우 복잡한 포메이션에 필요한 전술 지식
pub const VERY_COMPLEX_FORMATION_MIN_KNOWLEDGE: u8 = 18;

// ============================================================================
// CoachProfile - 감독 전술 프로필
// ============================================================================

/// 감독의 전술 능력 프로필
///
/// CardRarity와 level을 기반으로 전술 지식을 계산:
/// - tactical_knowledge = (rarity * 4) + (level - 1)
/// - 예: 5성 레벨 10 = 5*4 + 9 = 29
/// - 예: 1성 레벨 1 = 1*4 + 0 = 4
#[derive(Debug, Clone, Copy, Default)]
pub struct CoachProfile {
    /// 전술 지식 (0-30)
    /// - 0-10: 초급 (기본 포메이션만)
    /// - 11-15: 중급 (일반 포메이션)
    /// - 16-20: 고급 (복잡한 포메이션)
    /// - 21+: 전문가 (모든 포메이션)
    pub tactical_knowledge: u8,

    /// 공격 성향 (0.0 = 수비, 1.0 = 공격)
    pub attacking_tendency: f32,

    /// 위험 감수 성향 (0.0 = 보수적, 1.0 = 과감)
    pub risk_tolerance: f32,
}

impl CoachProfile {
    /// 새 감독 프로필 생성
    pub fn new(tactical_knowledge: u8, attacking_tendency: f32, risk_tolerance: f32) -> Self {
        Self {
            tactical_knowledge: tactical_knowledge.min(30),
            attacking_tendency: attacking_tendency.clamp(0.0, 1.0),
            risk_tolerance: risk_tolerance.clamp(0.0, 1.0),
        }
    }

    /// CoachCard에서 프로필 생성
    ///
    /// # Arguments
    /// * `rarity` - 카드 레어도 (1-5)
    /// * `level` - 카드 레벨 (1-10)
    pub fn from_card(rarity: u8, level: u8) -> Self {
        let tactical_knowledge = (rarity.min(5) * 4) + (level.saturating_sub(1)).min(9);

        // 레어도별 기본 성향
        let (attacking, risk) = match rarity {
            1 => (0.5, 0.3), // 보수적
            2 => (0.5, 0.4),
            3 => (0.5, 0.5), // 균형
            4 => (0.6, 0.6),
            5 => (0.6, 0.7), // 과감
            _ => (0.5, 0.5),
        };

        Self::new(tactical_knowledge, attacking, risk)
    }

    /// 기본 프로필 (중급 감독)
    pub fn default_coach() -> Self {
        Self::new(12, 0.5, 0.5)
    }

    /// 포메이션 사용 가능 여부
    pub fn can_use_formation(&self, formation: &str) -> bool {
        let required = Self::formation_complexity(formation);
        self.tactical_knowledge >= required
    }

    /// 포메이션 복잡도 반환
    pub fn formation_complexity(formation: &str) -> u8 {
        match formation {
            // 기본 포메이션 (모든 감독 사용 가능)
            "4-4-2" | "442" | "4-5-1" | "451" => 0,

            // 일반 포메이션 (중급 이상)
            "4-3-3" | "433" | "3-5-2" | "352" => 10,

            // 복잡한 포메이션 (고급 이상)
            "4-2-3-1" | "4231" | "4-3-1-2" | "4312" => COMPLEX_FORMATION_MIN_KNOWLEDGE,

            // 매우 복잡한 포메이션 (전문가)
            "3-4-3" | "343" => VERY_COMPLEX_FORMATION_MIN_KNOWLEDGE,

            _ => 0,
        }
    }

    /// 공격적 조정 가능 여부 (위험 감수 성향 기반)
    pub fn allows_aggressive_change(&self, score_diff: i8) -> bool {
        // 크게 지고 있으면 위험 감수 성향에 따라 공격적 변경 허용
        if score_diff <= LOSING_BADLY_THRESHOLD {
            self.risk_tolerance > 0.5
        } else {
            true
        }
    }
}

// ============================================================================
// TacticalAdjustmentReason - 전술 조정 이유
// ============================================================================

/// 전술 조정이 발생한 이유
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TacticalAdjustmentReason {
    /// 초기 설정 (경기 시작 시)
    Initial,

    /// 점수 상황에 따른 조정 (지고 있을 때 공격적으로, 이길 때 수비적으로)
    ScoreSituation,

    /// 경기 시간에 따른 조정 (후반 막판 등)
    TimeSituation,

    /// 상대 포메이션 대응 (Counter-Tactic)
    OpponentCounter,

    /// 감독 선호도
    CoachPreference,

    /// 선수 구성에 따른 조정
    TeamComposition,
}

// ============================================================================
// SuggestedFormation - 추천 포메이션
// ============================================================================

/// 추천된 포메이션 변경
#[derive(Debug, Clone)]
pub struct SuggestedFormation {
    /// 추천 포메이션 (예: "4-3-3", "4-5-1")
    pub formation: String,

    /// 추천 전술 스타일
    pub style: TacticalStyle,

    /// 추천 이유
    pub reason: TacticalAdjustmentReason,

    /// 신뢰도 (0.0 ~ 1.0)
    pub confidence: f32,
}

impl SuggestedFormation {
    pub fn new(
        formation: impl Into<String>,
        style: TacticalStyle,
        reason: TacticalAdjustmentReason,
        confidence: f32,
    ) -> Self {
        Self {
            formation: formation.into(),
            style,
            reason,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

// ============================================================================
// TacticalAdjuster - 전술 조정기
// ============================================================================

/// 경기 중 전술 조정을 담당하는 구조체
#[derive(Debug, Clone)]
pub struct TacticalAdjuster {
    /// 현재 포메이션
    current_formation: String,

    /// 현재 전술 스타일
    current_style: TacticalStyle,

    /// 마지막 조정 시간 (분)
    last_adjustment_minute: u8,

    /// 조정 횟수
    adjustment_count: u8,

    /// 홈팀 여부
    is_home: bool,

    /// 감독 프로필 (Phase 4D-4)
    coach_profile: CoachProfile,
}

impl TacticalAdjuster {
    /// 새 TacticalAdjuster 생성 (기본 감독 프로필 사용)
    pub fn new(formation: &str, style: TacticalStyle, is_home: bool) -> Self {
        Self::with_coach(formation, style, is_home, CoachProfile::default_coach())
    }

    /// 감독 프로필과 함께 생성
    pub fn with_coach(
        formation: &str,
        style: TacticalStyle,
        is_home: bool,
        coach_profile: CoachProfile,
    ) -> Self {
        Self {
            current_formation: formation.to_string(),
            current_style: style,
            last_adjustment_minute: 0,
            adjustment_count: 0,
            is_home,
            coach_profile,
        }
    }

    /// CoachCard에서 생성
    pub fn from_card(
        formation: &str,
        style: TacticalStyle,
        is_home: bool,
        card_rarity: u8,
        card_level: u8,
    ) -> Self {
        Self::with_coach(
            formation,
            style,
            is_home,
            CoachProfile::from_card(card_rarity, card_level),
        )
    }

    /// 현재 포메이션 반환
    pub fn current_formation(&self) -> &str {
        &self.current_formation
    }

    /// 현재 전술 스타일 반환
    pub fn current_style(&self) -> TacticalStyle {
        self.current_style
    }

    /// 조정 횟수 반환
    pub fn adjustment_count(&self) -> u8 {
        self.adjustment_count
    }

    /// 감독 프로필 반환
    pub fn coach_profile(&self) -> &CoachProfile {
        &self.coach_profile
    }

    /// 전술 조정이 필요한지 확인
    ///
    /// # Arguments
    /// * `minute` - 현재 경기 시간 (분)
    /// * `score_diff` - 점수차 (홈 - 어웨이, 홈팀 기준 양수면 이기고 있음)
    /// * `opponent_formation` - 상대 포메이션 (Counter-Tactic용)
    ///
    /// # Returns
    /// 전술 변경이 추천되면 `Some(SuggestedFormation)`, 아니면 `None`
    pub fn check_adjustment_needed(
        &self,
        minute: u8,
        score_diff: i8,
        opponent_formation: Option<&str>,
    ) -> Option<SuggestedFormation> {
        // 최소 간격 체크
        if minute.saturating_sub(self.last_adjustment_minute) < MIN_ADJUSTMENT_INTERVAL_MINUTES {
            return None;
        }

        // Phase 4D-2: 점수/시간 기반 조정
        if let Some(suggestion) = self.check_score_time_adjustment(minute, score_diff) {
            // Phase 4D-4: 감독 능력에 따른 필터링
            if self.coach_can_use_suggestion(&suggestion, score_diff) {
                return Some(suggestion);
            }
            // 감독이 사용 불가 → 대안 포메이션 시도
            if let Some(fallback) = self.find_fallback_formation(&suggestion) {
                return Some(fallback);
            }
        }

        // Phase 4D-3: 상대 포메이션 대응
        if let Some(opp) = opponent_formation {
            if let Some(suggestion) = self.check_counter_tactic(opp) {
                // Phase 4D-4: 감독 능력에 따른 필터링
                if self.coach_can_use_suggestion(&suggestion, score_diff) {
                    return Some(suggestion);
                }
                if let Some(fallback) = self.find_fallback_formation(&suggestion) {
                    return Some(fallback);
                }
            }
        }

        None
    }

    /// 감독이 추천 포메이션을 사용할 수 있는지 확인
    fn coach_can_use_suggestion(&self, suggestion: &SuggestedFormation, score_diff: i8) -> bool {
        // 포메이션 복잡도 체크
        if !self.coach_profile.can_use_formation(&suggestion.formation) {
            return false;
        }

        // 공격적 변경 체크 (3-4-3 등 공격형 포메이션)
        if suggestion.formation == "3-4-3" || suggestion.formation == "343" {
            return self.coach_profile.allows_aggressive_change(score_diff);
        }

        true
    }

    /// 감독이 사용 가능한 대안 포메이션 찾기
    fn find_fallback_formation(&self, original: &SuggestedFormation) -> Option<SuggestedFormation> {
        // 원본 포메이션이 너무 복잡하면 단순한 대안 제시
        let fallbacks: &[(&str, TacticalStyle)] = match original.formation.as_str() {
            // 3-4-3 → 4-3-3 또는 4-4-2
            "3-4-3" | "343" => &[("4-3-3", TacticalStyle::WingPlay), ("4-4-2", TacticalStyle::Counter)],
            // 4-2-3-1 → 4-4-2
            "4-2-3-1" | "4231" => &[("4-4-2", TacticalStyle::Balanced)],
            // 4-3-3 → 4-4-2
            "4-3-3" | "433" => &[("4-4-2", TacticalStyle::WingPlay)],
            _ => return None,
        };

        for (formation, style) in fallbacks {
            if self.coach_profile.can_use_formation(formation) {
                return Some(SuggestedFormation::new(
                    *formation,
                    *style,
                    original.reason,
                    original.confidence * 0.8, // 대안은 신뢰도 감소
                ));
            }
        }

        None
    }

    /// 전술 조정 적용
    ///
    /// # Arguments
    /// * `suggestion` - 적용할 추천 포메이션
    /// * `minute` - 현재 경기 시간
    ///
    /// # Returns
    /// 새로운 RoleTransitionMatrix와 player_roles
    pub fn apply_adjustment(
        &mut self,
        suggestion: &SuggestedFormation,
        minute: u8,
    ) -> (RoleTransitionMatrix, [PositionKey; 22]) {
        // 상태 업데이트
        self.current_formation = suggestion.formation.clone();
        self.current_style = suggestion.style;
        self.last_adjustment_minute = minute;
        self.adjustment_count += 1;

        // 새 매트릭스 생성
        let matrix = RoleTransitionMatrix::from_formation_and_style(
            &self.current_formation,
            self.current_style,
        );

        // 새 player_roles 생성 (22명)
        let player_roles: [PositionKey; 22] = std::array::from_fn(|i| {
            if self.is_home {
                if i < 11 {
                    slot_to_position_key(i, &self.current_formation)
                } else {
                    // 상대팀은 변경하지 않음 - 기본값 사용
                    PositionKey::CM
                }
            } else {
                if i >= 11 {
                    slot_to_position_key(i - 11, &self.current_formation)
                } else {
                    // 상대팀은 변경하지 않음 - 기본값 사용
                    PositionKey::CM
                }
            }
        });

        (matrix, player_roles)
    }

    // ========================================================================
    // Phase 4D-2: 점수/시간 기반 조정
    // ========================================================================

    /// 점수와 시간에 따른 전술 조정 확인
    fn check_score_time_adjustment(&self, minute: u8, score_diff: i8) -> Option<SuggestedFormation> {
        // 실제 점수차 (어웨이팀이면 부호 반전)
        let effective_diff = if self.is_home { score_diff } else { -score_diff };

        match (effective_diff, minute) {
            // 크게 지고 있고 후반 막판 → 매우 공격적 (3-4-3)
            (diff, min) if diff <= LOSING_BADLY_THRESHOLD && min >= 75 => {
                Some(SuggestedFormation::new(
                    "3-4-3",
                    TacticalStyle::Counter, // 빠른 역습
                    TacticalAdjustmentReason::ScoreSituation,
                    0.9,
                ))
            }

            // 지고 있고 후반 → 공격적 (4-3-3)
            (diff, min) if diff <= LOSING_THRESHOLD && min >= 70 => {
                Some(SuggestedFormation::new(
                    "4-3-3",
                    TacticalStyle::WingPlay,
                    TacticalAdjustmentReason::ScoreSituation,
                    0.8,
                ))
            }

            // 크게 이기고 있고 후반 막판 → 매우 수비적 (4-5-1)
            (diff, min) if diff >= WINNING_COMFORTABLY_THRESHOLD && min >= 80 => {
                Some(SuggestedFormation::new(
                    "4-5-1",
                    TacticalStyle::Possession, // 볼 유지
                    TacticalAdjustmentReason::ScoreSituation,
                    0.9,
                ))
            }

            // 이기고 있고 후반 → 수비적 (4-4-2 수비형)
            (diff, min) if diff >= WINNING_THRESHOLD && min >= 75 => {
                Some(SuggestedFormation::new(
                    "4-4-2",
                    TacticalStyle::Possession,
                    TacticalAdjustmentReason::ScoreSituation,
                    0.7,
                ))
            }

            // 전반에 크게 지고 있고 홈팀 → 점유율 강화 (4-2-3-1)
            (diff, min) if diff <= LOSING_BADLY_THRESHOLD && min < 30 && self.is_home => {
                Some(SuggestedFormation::new(
                    "4-2-3-1",
                    TacticalStyle::Possession,
                    TacticalAdjustmentReason::ScoreSituation,
                    0.6,
                ))
            }

            _ => None,
        }
    }

    // ========================================================================
    // Phase 4D-3: 상대 포메이션 대응 (Counter-Tactic)
    // ========================================================================

    /// 상대 포메이션에 대응하는 전술 확인
    fn check_counter_tactic(&self, opponent_formation: &str) -> Option<SuggestedFormation> {
        // 이미 대응 포메이션이면 스킵
        let counter = Self::get_counter_formation(opponent_formation)?;

        if self.current_formation == counter.0 {
            return None;
        }

        Some(SuggestedFormation::new(
            counter.0,
            counter.1,
            TacticalAdjustmentReason::OpponentCounter,
            0.6, // 상대 대응은 신뢰도 중간
        ))
    }

    /// 상대 포메이션에 대응하는 포메이션 반환
    ///
    /// Open-Football의 `select_counter_tactic` 참고
    fn get_counter_formation(opponent: &str) -> Option<(&'static str, TacticalStyle)> {
        match opponent {
            // 공격형 포메이션 → 수비형으로 대응
            "4-3-3" | "433" | "3-4-3" | "343" => Some(("4-5-1", TacticalStyle::Balanced)),

            // 수비형 포메이션 → 공격형으로 압박
            "4-5-1" | "451" | "4-1-4-1" | "4141" => Some(("4-3-3", TacticalStyle::WingPlay)),

            // 점유형 포메이션 → 프레싱으로 대응
            "4-2-3-1" | "4231" | "4-3-1-2" | "4312" => Some(("4-4-2", TacticalStyle::Counter)),

            // 좁은 포메이션 → 측면 활용
            "4-4-2 narrow" | "442 narrow" => Some(("4-4-2", TacticalStyle::WingPlay)),

            // 3백 → 측면 공격
            "3-5-2" | "352" => Some(("4-3-3", TacticalStyle::WingPlay)),

            _ => None,
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
    fn test_tactical_adjuster_creation() {
        let adjuster = TacticalAdjuster::new("4-4-2", TacticalStyle::Balanced, true);
        assert_eq!(adjuster.current_formation(), "4-4-2");
        assert_eq!(adjuster.current_style(), TacticalStyle::Balanced);
        assert_eq!(adjuster.adjustment_count(), 0);
    }

    #[test]
    fn test_losing_badly_late_game_suggests_attacking() {
        // 기본 감독(tactical_knowledge=12)은 3-4-3 사용 불가 → 4-3-3 대안
        let adjuster = TacticalAdjuster::new("4-4-2", TacticalStyle::Balanced, true);

        // 75분에 2골 뒤지고 있음
        let suggestion = adjuster.check_adjustment_needed(75, -2, None);

        assert!(suggestion.is_some());
        let s = suggestion.unwrap();
        // 기본 감독은 4-3-3로 대체 (3-4-3은 전문가만)
        assert!(s.formation == "4-3-3" || s.formation == "4-4-2", "Got: {}", s.formation);
        assert_eq!(s.reason, TacticalAdjustmentReason::ScoreSituation);
    }

    #[test]
    fn test_expert_coach_uses_343() {
        // 전문가 감독(5성 레벨 10)은 3-4-3 사용 가능
        let adjuster = TacticalAdjuster::from_card(
            "4-4-2",
            TacticalStyle::Balanced,
            true,
            5,  // rarity
            10, // level
        );

        // 75분에 2골 뒤지고 있음
        let suggestion = adjuster.check_adjustment_needed(75, -2, None);

        assert!(suggestion.is_some());
        let s = suggestion.unwrap();
        assert_eq!(s.formation, "3-4-3");
        assert_eq!(s.reason, TacticalAdjustmentReason::ScoreSituation);
    }

    #[test]
    fn test_winning_late_game_suggests_defensive() {
        let adjuster = TacticalAdjuster::new("4-4-2", TacticalStyle::Balanced, true);

        // 80분에 2골 앞서고 있음
        let suggestion = adjuster.check_adjustment_needed(80, 2, None);

        assert!(suggestion.is_some());
        let s = suggestion.unwrap();
        assert_eq!(s.formation, "4-5-1");
        assert_eq!(s.style, TacticalStyle::Possession);
    }

    #[test]
    fn test_no_adjustment_before_minimum_interval() {
        let mut adjuster = TacticalAdjuster::new("4-4-2", TacticalStyle::Balanced, true);
        adjuster.last_adjustment_minute = 70;

        // 75분에 체크 (5분 후) - 최소 간격 10분 미만
        let suggestion = adjuster.check_adjustment_needed(75, -2, None);
        assert!(suggestion.is_none());
    }

    #[test]
    fn test_counter_tactic_against_433() {
        let adjuster = TacticalAdjuster::new("4-4-2", TacticalStyle::Balanced, true);

        // 상대가 4-3-3 → 4-5-1로 대응
        let suggestion = adjuster.check_adjustment_needed(30, 0, Some("4-3-3"));

        assert!(suggestion.is_some());
        let s = suggestion.unwrap();
        assert_eq!(s.formation, "4-5-1");
        assert_eq!(s.reason, TacticalAdjustmentReason::OpponentCounter);
    }

    #[test]
    fn test_apply_adjustment() {
        let mut adjuster = TacticalAdjuster::new("4-4-2", TacticalStyle::Balanced, true);

        let suggestion = SuggestedFormation::new(
            "4-3-3",
            TacticalStyle::WingPlay,
            TacticalAdjustmentReason::ScoreSituation,
            0.8,
        );

        let (matrix, _roles) = adjuster.apply_adjustment(&suggestion, 70);

        assert_eq!(adjuster.current_formation(), "4-3-3");
        assert_eq!(adjuster.current_style(), TacticalStyle::WingPlay);
        assert_eq!(adjuster.adjustment_count(), 1);
        assert_eq!(matrix.formation, "4-3-3");
    }

    #[test]
    fn test_451_matrix_weights() {
        let m = RoleTransitionMatrix::new_451_balanced();
        assert_eq!(m.formation, "4-5-1");

        // CDM → ST 롱볼 연결 확인
        let w = m.get_weight(PositionKey::CDM, PositionKey::ST);
        assert!(w >= 1.0, "CDM→ST should be preferred: {}", w);
    }

    #[test]
    fn test_343_matrix_weights() {
        let m = RoleTransitionMatrix::new_343_balanced();
        assert_eq!(m.formation, "3-4-3");

        // LW → ST 연결 확인
        let w = m.get_weight(PositionKey::LW, PositionKey::ST);
        assert!(w >= 1.3, "LW→ST should be strongly preferred: {}", w);
    }

    #[test]
    fn test_away_team_score_inversion() {
        // 어웨이팀은 점수차 부호가 반전됨
        let adjuster = TacticalAdjuster::new("4-4-2", TacticalStyle::Balanced, false);

        // score_diff = 2 (홈이 이김) → 어웨이는 지고 있음
        let suggestion = adjuster.check_adjustment_needed(75, 2, None);

        assert!(suggestion.is_some());
        let s = suggestion.unwrap();
        // 어웨이가 지고 있으니 공격적으로
        assert!(s.formation == "4-3-3" || s.formation == "3-4-3");
    }

    // ========================================================================
    // Phase 4D-4: Coach Profile Tests
    // ========================================================================

    #[test]
    fn test_coach_profile_from_card() {
        // 1성 레벨 1 = 4 + 0 = 4
        let profile = CoachProfile::from_card(1, 1);
        assert_eq!(profile.tactical_knowledge, 4);

        // 5성 레벨 10 = 20 + 9 = 29
        let profile = CoachProfile::from_card(5, 10);
        assert_eq!(profile.tactical_knowledge, 29);

        // 3성 레벨 5 = 12 + 4 = 16
        let profile = CoachProfile::from_card(3, 5);
        assert_eq!(profile.tactical_knowledge, 16);
    }

    #[test]
    fn test_coach_formation_complexity() {
        // 기본 포메이션
        assert_eq!(CoachProfile::formation_complexity("4-4-2"), 0);
        assert_eq!(CoachProfile::formation_complexity("4-5-1"), 0);

        // 일반 포메이션
        assert_eq!(CoachProfile::formation_complexity("4-3-3"), 10);
        assert_eq!(CoachProfile::formation_complexity("3-5-2"), 10);

        // 복잡한 포메이션
        assert_eq!(CoachProfile::formation_complexity("4-2-3-1"), COMPLEX_FORMATION_MIN_KNOWLEDGE);

        // 매우 복잡한 포메이션
        assert_eq!(CoachProfile::formation_complexity("3-4-3"), VERY_COMPLEX_FORMATION_MIN_KNOWLEDGE);
    }

    #[test]
    fn test_beginner_coach_limited_formations() {
        // 1성 레벨 1 감독 (tactical_knowledge = 4)
        let adjuster = TacticalAdjuster::from_card(
            "4-4-2",
            TacticalStyle::Balanced,
            true,
            1, // rarity
            1, // level
        );

        assert_eq!(adjuster.coach_profile().tactical_knowledge, 4);

        // 4-4-2, 4-5-1만 사용 가능
        assert!(adjuster.coach_profile().can_use_formation("4-4-2"));
        assert!(adjuster.coach_profile().can_use_formation("4-5-1"));

        // 4-3-3, 3-4-3 사용 불가
        assert!(!adjuster.coach_profile().can_use_formation("4-3-3"));
        assert!(!adjuster.coach_profile().can_use_formation("3-4-3"));
    }

    #[test]
    fn test_expert_coach_all_formations() {
        // 5성 레벨 10 감독 (tactical_knowledge = 29)
        let adjuster = TacticalAdjuster::from_card(
            "4-4-2",
            TacticalStyle::Balanced,
            true,
            5, // rarity
            10, // level
        );

        assert_eq!(adjuster.coach_profile().tactical_knowledge, 29);

        // 모든 포메이션 사용 가능
        assert!(adjuster.coach_profile().can_use_formation("4-4-2"));
        assert!(adjuster.coach_profile().can_use_formation("4-3-3"));
        assert!(adjuster.coach_profile().can_use_formation("4-2-3-1"));
        assert!(adjuster.coach_profile().can_use_formation("3-4-3"));
    }

    #[test]
    fn test_coach_fallback_formation() {
        // 1성 레벨 1 감독 → 3-4-3 사용 불가 → 4-4-2로 대체
        let adjuster = TacticalAdjuster::from_card(
            "4-4-2",
            TacticalStyle::Balanced,
            true,
            1, // rarity
            1, // level
        );

        // 75분에 2골 뒤지고 있음 → 3-4-3 추천되지만 감독 능력 부족
        let suggestion = adjuster.check_adjustment_needed(75, -2, None);

        assert!(suggestion.is_some());
        let s = suggestion.unwrap();
        // 대안 포메이션 (4-4-2 Counter)으로 대체됨
        assert_eq!(s.formation, "4-4-2");
        assert_eq!(s.style, TacticalStyle::Counter);
    }

    #[test]
    fn test_coach_with_card_constructor() {
        let adjuster = TacticalAdjuster::from_card(
            "4-4-2",
            TacticalStyle::Balanced,
            true,
            3, // 3성
            5, // 레벨 5
        );

        // tactical_knowledge = 12 + 4 = 16
        assert_eq!(adjuster.coach_profile().tactical_knowledge, 16);

        // 4-2-3-1 사용 가능 (15 이상)
        assert!(adjuster.coach_profile().can_use_formation("4-2-3-1"));

        // 3-4-3 사용 불가 (18 필요)
        assert!(!adjuster.coach_profile().can_use_formation("3-4-3"));
    }
}
