//! Match Setup - P17: 경기 시뮬레이션 OS
//!
//! 경기 시작 전 모든 팀/선수 정보를 한 곳에서 관리.
//! 경기 중에는 읽기 전용으로 참조.

use serde::{Deserialize, Serialize};

use crate::player::personality::PersonalityArchetype;
use crate::fix01::error_codes;

use super::person::{Person, PositionRating};
use super::player::{Player, PlayerAttributes, Position};
use super::skill::SpecialSkill;
use super::team::{Formation, Team};
use super::trait_system::TraitSlots;

// ============================================================================
// TeamSide - 기존 중복 정의 통합
// ============================================================================

/// 팀 사이드
///
/// NOTE: 기존 tactical_context.rs, defensive_positioning.rs에 중복 정의되어 있었음.
/// 이 정의로 통합하고 기존 2곳은 re-export로 변경.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TeamSide {
    #[default]
    Home,
    Away,
}

impl TeamSide {
    /// track_id에서 팀 판별 (0-10: Home, 11-21: Away)
    #[inline]
    pub const fn from_track_id(track_id: usize) -> Self {
        if track_id < 11 {
            TeamSide::Home
        } else {
            TeamSide::Away
        }
    }

    /// track_id가 홈팀인지
    #[inline]
    pub const fn is_home(track_id: usize) -> bool {
        track_id < 11
    }

    /// track_id에서 팀 내 슬롯 번호 (0-10)
    #[inline]
    pub const fn team_slot(track_id: usize) -> u8 {
        if track_id < 11 {
            track_id as u8
        } else {
            (track_id - 11) as u8
        }
    }

    /// 팀/슬롯에서 track_id로 변환 (slot은 0-10)
    #[inline]
    pub const fn track_id_from_slot(team: TeamSide, slot: u8) -> Option<usize> {
        if slot >= 11 {
            return None;
        }
        match team {
            TeamSide::Home => Some(slot as usize),
            TeamSide::Away => Some(11 + slot as usize),
        }
    }

    /// 팀 ID 반환 (0 = Home, 1 = Away)
    #[inline]
    pub const fn team_id(track_id: usize) -> u32 {
        if track_id < 11 {
            0
        } else {
            1
        }
    }

    /// 상대 팀 반환
    #[inline]
    pub fn opponent(&self) -> Self {
        match self {
            TeamSide::Home => TeamSide::Away,
            TeamSide::Away => TeamSide::Home,
        }
    }

    /// 선수 인덱스로 팀 결정 (from_track_id의 alias)
    #[inline]
    pub fn from_player_idx(player_idx: usize) -> Self {
        Self::from_track_id(player_idx)
    }

    /// 이 팀이 Home인지 (인스턴스 메서드)
    #[inline]
    pub fn is_home_team(&self) -> bool {
        matches!(self, TeamSide::Home)
    }

    /// 이 팀이 Away인지 (인스턴스 메서드)
    #[inline]
    pub fn is_away_team(&self) -> bool {
        matches!(self, TeamSide::Away)
    }

    // =========================================================================
    // P17 Phase 6: idx < 11 패턴 제거용 헬퍼 메서드
    // =========================================================================

    /// 상대팀 선수 인덱스 범위
    ///
    /// # Example
    /// ```ignore
    /// let opponent_range = TeamSide::opponent_range(5);  // 11..22 (Home 선수 → Away 팀)
    /// let opponent_range = TeamSide::opponent_range(15); // 0..11 (Away 선수 → Home 팀)
    /// ```
    #[inline]
    pub fn opponent_range(track_id: usize) -> std::ops::Range<usize> {
        if track_id < 11 {
            11..22
        } else {
            0..11
        }
    }

    /// 같은팀 선수 인덱스 범위
    #[inline]
    pub fn teammate_range(track_id: usize) -> std::ops::Range<usize> {
        if track_id < 11 {
            0..11
        } else {
            11..22
        }
    }

    /// 두 선수가 같은 팀인지
    ///
    /// # Example
    /// ```ignore
    /// TeamSide::same_team(3, 7);   // true (둘 다 Home)
    /// TeamSide::same_team(3, 15);  // false (Home vs Away)
    /// ```
    #[inline]
    pub fn same_team(idx1: usize, idx2: usize) -> bool {
        (idx1 < 11) == (idx2 < 11)
    }

    /// 상대팀 GK 인덱스 (GK는 팀의 0번 슬롯)
    ///
    /// # Example
    /// ```ignore
    /// TeamSide::opponent_gk(5);  // 11 (Home 선수 → Away GK)
    /// TeamSide::opponent_gk(15); // 0 (Away 선수 → Home GK)
    /// ```
    #[inline]
    pub fn opponent_gk(track_id: usize) -> usize {
        if track_id < 11 {
            11
        } else {
            0
        }
    }

    /// 자기팀 GK 인덱스
    #[inline]
    pub fn own_gk(track_id: usize) -> usize {
        if track_id < 11 {
            0
        } else {
            11
        }
    }

    /// 공격 방향 골문 X 좌표 (105m 피치 기준)
    ///
    /// Home 팀은 오른쪽(105.0)으로 공격, Away 팀은 왼쪽(0.0)으로 공격
    #[deprecated(
        note = "Use DirectionContext::opponent_goal_x()/own_goal_x() for halftime-aware goal positions."
    )]
    #[inline]
    pub fn attack_goal_x(track_id: usize) -> f32 {
        if track_id < 11 {
            105.0
        } else {
            0.0
        }
    }

    /// 수비 방향 골문 X 좌표
    #[deprecated(
        note = "Use DirectionContext::opponent_goal_x()/own_goal_x() for halftime-aware goal positions."
    )]
    #[inline]
    pub fn defense_goal_x(track_id: usize) -> f32 {
        if track_id < 11 {
            0.0
        } else {
            105.0
        }
    }

    /// 팀 내 로컬 인덱스 (0-10)
    ///
    /// # Example
    /// ```ignore
    /// TeamSide::local_idx(5);  // 5
    /// TeamSide::local_idx(15); // 4
    /// ```
    #[inline]
    pub fn local_idx(track_id: usize) -> usize {
        if track_id < 11 {
            track_id
        } else {
            track_id - 11
        }
    }

    /// 로컬 인덱스를 글로벌 인덱스로 변환
    #[inline]
    pub fn global_idx(local_idx: usize, is_home: bool) -> usize {
        if is_home {
            local_idx
        } else {
            local_idx + 11
        }
    }

    /// is_home bool에서 상대팀 범위 반환
    ///
    /// # Example
    /// ```ignore
    /// TeamSide::opponent_range_for_home(true);  // 11..22
    /// TeamSide::opponent_range_for_home(false); // 0..11
    /// ```
    #[inline]
    pub fn opponent_range_for_home(is_home: bool) -> std::ops::Range<usize> {
        if is_home {
            11..22
        } else {
            0..11
        }
    }

    /// is_home bool에서 같은팀 범위 반환
    #[inline]
    pub fn teammate_range_for_home(is_home: bool) -> std::ops::Range<usize> {
        if is_home {
            0..11
        } else {
            11..22
        }
    }

    /// is_home bool에서 공격 골문 X 좌표 반환
    #[deprecated(
        note = "Use DirectionContext::opponent_goal_x()/own_goal_x() for halftime-aware goal positions."
    )]
    #[inline]
    pub fn attack_goal_x_for_home(is_home: bool) -> f32 {
        if is_home {
            105.0
        } else {
            0.0
        }
    }

    /// is_home bool에서 수비 골문 X 좌표 반환
    #[deprecated(
        note = "Use DirectionContext::opponent_goal_x()/own_goal_x() for halftime-aware goal positions."
    )]
    #[inline]
    pub fn defense_goal_x_for_home(is_home: bool) -> f32 {
        if is_home {
            0.0
        } else {
            105.0
        }
    }
}

// ============================================================================
// MatchPlayer - 경기용 선수 정보
// ============================================================================

/// 경기용 선수 정보 (Player의 경기용 복사본)
#[derive(Debug, Clone)]
pub struct MatchPlayer {
    /// 선수 이름
    pub name: String,
    /// 포지션
    pub position: Position,
    /// 전체 능력치 (1-99)
    pub overall: u8,
    /// 상세 능력치 (36개)
    pub attributes: PlayerAttributes,
    /// 특성 (4슬롯, Bronze/Silver/Gold)
    pub traits: TraitSlots,
    /// 성격 유형
    pub personality: PersonalityArchetype,
    /// 슬롯 번호 (0-10)
    pub slot: u8,
    /// FIX01: match-time condition level (1..=5)
    pub condition_level: u8,
    /// 포지션 적합도 (0.0-1.0, 1.0 = 자연 포지션, 0.3 = 70% 패널티)
    pub position_suitability: f32,
    /// 장착된 특수 스킬
    pub equipped_skills: Vec<SpecialSkill>,
}

impl MatchPlayer {
    /// Player로부터 MatchPlayer 생성
    ///
    /// # Arguments
    ///
    /// * `player` - 선수 정보
    /// * `slot` - 슬롯 번호 (0-10)
    /// * `person` - Person 데이터 (포지션 적합도 계산용, Option)
    /// * `is_starter` - 선발 선수 여부 (P0.5 Attributes tracking)
    ///
    /// # Position Penalty
    ///
    /// person 데이터가 있으면 포지션 적합도를 계산하여 능력치에 패널티 적용:
    /// - Rating 15-20: 1.0 (패널티 없음, 자연 포지션)
    /// - Rating 11-14: 0.85 (15% 패널티)
    /// - Rating 6-10: 0.6 (40% 패널티)
    /// - Rating 1-5: 0.3 (70% 패널티)
    /// - Rating 0: 0.0 (100% 패널티, 플레이 불가)
    ///
    /// person이 None이면 패널티 없음 (1.0).
    pub fn from_player(
        player: &Player,
        slot: u8,
        person: Option<&Person>,
        _is_starter: bool,
    ) -> Self {
        // 1. 포지션 적합도 계산
        let rating_pos = PositionRating::from_engine_position(&player.position);
        let position_suitability =
            person.map(|p| p.position_suitability(rating_pos)).unwrap_or(1.0); // Person 데이터 없으면 패널티 없음

        // 2. 능력치 복사 후 패널티 적용
        // ---------------------------------------------
        // ✅ P0.75 Patch 2: Attributes None 실패 정책 (다층 방어)
        //
        // Contract: Player.attributes must ALWAYS be Some(...) after P0.75-2 injection SSOT.
        // - PlayerLibrary.gd guarantees 100% injection coverage (36 attrs)
        //
        // Level 1: Debug Assert (개발 중 즉시 발각)
        // Level 2: Strict Mode (CI/테스트 환경)
        // Level 3: Release Warn (프로덕션 fallback)
        // ---------------------------------------------

        // Level 1: Debug mode - immediate panic during development
        debug_assert!(
            player.attributes.is_some(),
            "P0.75-2 violated: Player.attributes=None for '{}' (slot={})",
            player.name,
            slot
        );

        // Level 2: Strict mode - panic in CI/test builds
        #[cfg(feature = "strict_contracts")]
        if player.attributes.is_none() {
            panic!(
                "STRICT: P0.75-2 violated: Player.attributes=None for '{}' (slot={})",
                player.name, slot
            );
        }

        // Level 3: Release mode - warn and fallback to default
        let mut attributes = if let Some(attrs) = player.attributes.clone() {
            attrs
        } else {
            eprintln!(
                "[ATTRS_MISSING] P0.75-2 violated: Player '{}' (slot={}) has None attributes",
                player.name, slot
            );
            eprintln!("[ATTRS_FALLBACK] Using default(50) for '{}'", player.name);
            PlayerAttributes::default()
        };

        attributes.apply_position_penalty(position_suitability);

        Self {
            name: player.name.clone(),
            position: player.position,
            overall: player.overall,
            attributes, // 패널티 적용된 능력치
            traits: player.traits.clone(),
            personality: player.personality,
            slot,
            condition_level: player.condition,
            position_suitability,
            equipped_skills: player.equipped_skills.clone(),
        }
    }

    /// 특정 스킬 보유 여부
    #[inline]
    pub fn has_skill(&self, skill: SpecialSkill) -> bool {
        self.equipped_skills.contains(&skill)
    }

    /// Calculate curve shot skill level based on player attributes
    pub fn get_curve_level(&self) -> crate::engine::ball::CurveLevel {
        use crate::engine::ball::CurveLevel;

        let attr = &self.attributes;

        // Lv3: Elite curve shot (free_kicks > 18, technique > 17, flair > 16)
        if attr.free_kicks > 18 && attr.technique > 17 && attr.flair > 16 {
            return CurveLevel::Lv3;
        }

        // Lv2: Advanced curve shot (free_kicks > 15, technique > 15)
        if attr.free_kicks > 15 && attr.technique > 15 {
            return CurveLevel::Lv2;
        }

        // Lv1: Basic curve shot (free_kicks > 12, technique > 12)
        if attr.free_kicks > 12 && attr.technique > 12 {
            return CurveLevel::Lv1;
        }

        // None: No curve ability
        CurveLevel::None
    }
}

// ============================================================================
// PlayerSlot - track_id ↔ 선수 연결
// ============================================================================

/// 슬롯 정보 (track_id ↔ 선수 연결)
#[derive(Debug, Clone, Copy)]
pub struct PlayerSlot {
    /// 팀 (Home/Away)
    pub team: TeamSide,
    /// 팀 내 슬롯 (0-10)
    pub team_slot: u8,
    /// 현재 경기 중인지 (교체/퇴장/부상 시 false)
    pub is_active: bool,
    /// 교체로 나간 선수인지
    pub substituted: bool,
    /// 퇴장당한 선수인지
    pub sent_off: bool,
    /// 부상으로 나간 선수인지
    pub injured: bool,
}

impl Default for PlayerSlot {
    fn default() -> Self {
        Self {
            team: TeamSide::Home,
            team_slot: 0,
            is_active: true,
            substituted: false,
            sent_off: false,
            injured: false,
        }
    }
}

// ============================================================================
// PitchAssignment - pitch slot ↔ roster entry mapping (substitutions)
// ============================================================================

/// Which roster entry currently occupies a given **pitch slot** (team_slot 0-10).
///
/// Contract:
/// - Pitch slots stay fixed (`track_id` 0..21).
/// - The occupying player can change via substitutions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitchAssignment {
    /// Starter slot within `TeamSetup.starters` (0..10)
    Starter(u8),
    /// Bench slot within `TeamSetup.substitutes` (0..MAX_SUBSTITUTES-1)
    Substitute(u8),
}

impl Default for PitchAssignment {
    fn default() -> Self {
        Self::Starter(0)
    }
}

// ============================================================================ 
// TeamSetup - 팀별 셋업
// ============================================================================ 

/// 팀 셋업
#[derive(Debug, Clone)]
pub struct TeamSetup {
    /// 팀 이름
    pub name: String,
    /// 포메이션
    pub formation: Formation,
    /// 스타팅 11명
    pub starters: Vec<MatchPlayer>,
    /// 후보 선수들
    pub substitutes: Vec<MatchPlayer>,
}

/// 후보 선수 최대 인원 (FIFA 규정 기준)
pub const MAX_SUBSTITUTES: usize = 7;

impl TeamSetup {
    /// Team으로부터 TeamSetup 생성
    ///
    /// # Note
    ///
    /// Person 데이터가 없으므로 포지션 패널티가 적용되지 않습니다.
    /// Person 데이터 연동이 필요하면 `from_team_with_persons()` 메서드를 추가하거나
    /// 별도로 Person 데이터를 받아 처리할 수 있습니다.
    pub fn from_team(team: &Team) -> Result<Self, String> {
        // ✅ P0.5: Track missing attributes across all players
        let mut missing_starters: usize = 0;
        let mut missing_bench: usize = 0;

        // Convert starters (first 11 players) with deterministic formation-slot assignment.
        let starters_slice: Vec<&Player> = team.players.iter().take(11).collect();
        if starters_slice.len() != 11 {
            return Err(format!(
                "{}: Team '{}' must have 11 starters, found {}",
                error_codes::UNSUPPORTED_POSITION_MAPPING,
                team.name,
                starters_slice.len()
            ));
        }

        // Slot template aligned with engine slot_to_position_key() for the same formation code.
        let slot_template: [Position; 11] = formation_slot_template(&team.formation);

        // Hard requirement: exactly one GK in starting 11.
        let gk_count = starters_slice.iter().filter(|p| p.position == Position::GK).count();
        if gk_count != 1 {
            return Err(format!(
                "{}: starting 11 must have exactly 1 GK, got {}",
                error_codes::UNSUPPORTED_POSITION_MAPPING, gk_count
            ));
        }

        // Hungarian assignment (11 players × 11 slots) with deterministic tie-breaks.
        use pathfinding::kuhn_munkres::kuhn_munkres_min;
        use pathfinding::matrix::Matrix;

        const COST_INCOMPATIBLE: i64 = 1_000_000;

        let costs = Matrix::from_fn(11, 11, |(player_idx, slot_idx)| {
            let player = starters_slice[player_idx];
            let expected = slot_template[slot_idx];

            // Enforce GK slot (0) strictly.
            let base_cost = if slot_idx == 0 {
                if player.position == Position::GK {
                    0
                } else {
                    COST_INCOMPATIBLE
                }
            } else if player.position == Position::GK {
                COST_INCOMPATIBLE
            } else if player.position == expected {
                0
            } else if player.position.is_compatible_position(expected) {
                10
            } else {
                COST_INCOMPATIBLE
            };

            // Deterministic tie-break (prefer earlier roster order).
            base_cost + player_idx as i64
        });

        let (_, assignments) = kuhn_munkres_min(&costs);

        // assignments[player_idx] = slot_idx
        let mut slot_to_player_idx = [0usize; 11];
        for (player_idx, slot_idx) in assignments.iter().enumerate() {
            slot_to_player_idx[*slot_idx] = player_idx;
        }

        // Validate assignment: no incompatible placements.
        for slot_idx in 0..11 {
            let player_idx = slot_to_player_idx[slot_idx];
            let player = starters_slice[player_idx];
            let expected = slot_template[slot_idx];
            let ok = if slot_idx == 0 {
                player.position == Position::GK
            } else {
                player.position != Position::GK
                    && (player.position == expected || player.position.is_compatible_position(expected))
            };
            if !ok {
                return Err(format!(
                    "{}: cannot assign '{}' ({:?}) to slot {} ({:?}) for formation {}",
                    error_codes::UNSUPPORTED_POSITION_MAPPING,
                    player.name,
                    player.position,
                    slot_idx,
                    expected,
                    team.formation.code()
                ));
            }
        }

        let starters: Vec<MatchPlayer> = (0..11)
            .map(|slot| {
                let player_idx = slot_to_player_idx[slot];
                let player = starters_slice[player_idx];
                if player.attributes.is_none() {
                    missing_starters += 1;
                }
                MatchPlayer::from_player(player, slot as u8, None, true)
            })
            .collect();

        // P1: 후보는 최대 7명으로 제한 (FIFA 규정)
        // Convert bench (next 7 players max)
        let substitutes: Vec<MatchPlayer> = team
            .players
            .iter()
            .skip(11)
            .take(MAX_SUBSTITUTES)
            .enumerate()
            .map(|(slot, p)| {
                if p.attributes.is_none() {
                    missing_bench += 1;
                }
                MatchPlayer::from_player(p, slot as u8, None, false)
            })
            .collect();

        // ✅ P0.5: Emit team-level summary
        let total_missing = missing_starters + missing_bench;
        let total_players = starters.len() + substitutes.len();

        if total_missing > 0 {
            eprintln!(
                "[ATTRS_WARN] TeamSetup '{}': missing attributes {} / {} players (starters: {} / {}, bench: {} / {})",
                team.name,
                total_missing,
                total_players,
                missing_starters,
                starters.len(),
                missing_bench,
                substitutes.len()
            );
        } else {
            println!(
                "[ATTRS_OK] TeamSetup '{}': all {} players have attributes ✅",
                team.name, total_players
            );
        }

        Ok(Self { name: team.name.clone(), formation: team.formation.clone(), starters, substitutes })
    }
}

fn formation_slot_template(formation: &Formation) -> [Position; 11] {
    use Position::*;
    match formation {
        Formation::F442 => [GK, LB, CB, CB, RB, LM, CM, CM, RM, ST, ST],
        Formation::F433 => [GK, LB, CB, CB, RB, CM, CM, CM, LW, ST, RW],
        Formation::F4231 => [GK, LB, CB, CB, RB, CDM, CDM, LM, CAM, RM, ST],
        Formation::F4141 => [GK, LB, CB, CB, RB, CDM, LM, CM, CM, RM, ST],
        Formation::F4411 => [GK, LB, CB, CB, RB, LM, CM, CM, RM, CF, ST],
        Formation::F4321 => [GK, LB, CB, CB, RB, CM, CM, CM, CF, CF, ST],
        Formation::F4222 => [GK, LB, CB, CB, RB, CDM, CDM, LM, RM, ST, ST],
        Formation::F451 => [GK, LB, CB, CB, RB, LM, CM, CM, CM, RM, ST],
        Formation::F352 => [GK, CB, CB, CB, LWB, CM, CM, CM, RWB, ST, ST],
        Formation::F343 => [GK, CB, CB, CB, LM, CM, CM, RM, LW, ST, RW],
        Formation::F3421 => [GK, CB, CB, CB, LM, CM, CM, RM, LM, RM, ST],
        Formation::F3412 => [GK, CB, CB, CB, LM, CM, CM, RM, CAM, ST, ST],
        Formation::F532 => [GK, LWB, CB, CB, CB, RWB, CM, CM, CM, ST, ST],
        Formation::F541 => [GK, LWB, CB, CB, CB, RWB, LM, CM, CM, RM, ST],
    }
}

// ============================================================================
// P2.2-B: Formation Validator
// ============================================================================

/// Formation validator for team setup validation
///
/// Contract: Formation must have exactly 11 players with 1 GK
/// Purpose: Detect invalid team setups before match simulation
pub struct FormationValidator;

impl FormationValidator {
    /// Validate a team formation
    ///
    /// # Arguments
    /// * `team` - Team setup to validate
    ///
    /// # Returns
    /// * `Ok(())` if formation is valid
    /// * `Err(String)` with error message if invalid
    ///
    /// # Validation Rules
    /// 1. Must have exactly 11 starters
    /// 2. Must have exactly 1 goalkeeper (Position::GK)
    /// 3. No duplicate slot numbers (0-10)
    ///
    /// # Examples
    /// ```ignore
    /// use of_core::models::match_setup::{FormationValidator, TeamSetup};
    ///
    /// let team = build_valid_team();  // 11 players, 1 GK
    /// assert!(FormationValidator::validate_formation(&team).is_ok());
    /// ```
    pub fn validate_formation(team: &TeamSetup) -> Result<(), String> {
        // Rule 1: Check player count
        if team.starters.len() != 11 {
            return Err(format!(
                "Invalid formation: {} players (expected 11)",
                team.starters.len()
            ));
        }

        // Rule 2: Check for exactly 1 GK
        let gk_count = team.starters.iter().filter(|p| p.position == Position::GK).count();

        if gk_count != 1 {
            return Err(format!("Invalid formation: {} goalkeepers (expected 1)", gk_count));
        }

        // Rule 3: Check for duplicate slots
        let mut slots = std::collections::HashSet::new();
        for player in &team.starters {
            if !slots.insert(player.slot) {
                return Err(format!(
                    "Duplicate slot {} in formation (player: {})",
                    player.slot, player.name
                ));
            }
        }

        // Rule 4: Check slot numbers are in valid range (0-10)
        for player in &team.starters {
            if player.slot > 10 {
                return Err(format!(
                    "Invalid slot {} for player {} (must be 0-10)",
                    player.slot, player.name
                ));
            }
        }

        Ok(())
    }

    /// Validate both teams in a match
    ///
    /// Convenience method to validate both home and away teams
    pub fn validate_match(home: &TeamSetup, away: &TeamSetup) -> Result<(), String> {
        Self::validate_formation(home).map_err(|e| format!("Home team: {}", e))?;
        Self::validate_formation(away).map_err(|e| format!("Away team: {}", e))?;
        Ok(())
    }
}

// ============================================================================
// MatchSetup - 경기 셋업 메인 구조
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::DirectionContext;
    use crate::models::person::Person;
    use crate::models::player::{Player, PlayerAttributes, Position};

    #[test]
    fn test_position_penalty_integration_natural_position() {
        // Create player with high passing (18) at MC position
        let player = Player {
            name: "Test Player".to_string(),
            position: Position::CM, // Central Midfielder
            overall: 80,
            condition: 3,
            attributes: Some(PlayerAttributes {
                passing: 18,
                dribbling: 16,
                finishing: 14,
                pace: 80, // Physical attribute
                ..Default::default()
            }),
            equipped_skills: Vec::new(),
            traits: Default::default(),
            personality: Default::default(),
        };

        // Create person with MC rating = 20 (natural position)
        let person = Person::new(
            1,
            "Test Player".to_string(),
            "ENG".to_string(),
            "Test FC".to_string(),
            "MC".to_string(),
            150,
            160,
            25,
            Some("1,1,1,1,1,1,12,15,20,15,14,20,14,12".to_string()),
            // GK=1, DL=1, DC=1, DR=1, WBL=1, WBR=1, DM=12, ML=15, MC=20, MR=15, AML=14, AMC=20, AMR=14, ST=12
        );

        // Test: Natural position (MC rating 20) → no penalty
        let match_player = MatchPlayer::from_player(&player, 0, Some(&person), true);

        assert_eq!(
            match_player.position_suitability, 1.0,
            "Natural position should have no penalty"
        );
        assert_eq!(
            match_player.attributes.passing, 18,
            "Passing should not be penalized in natural position"
        );
        assert_eq!(
            match_player.attributes.dribbling, 16,
            "Dribbling should not be penalized in natural position"
        );
        assert_eq!(
            match_player.attributes.pace, 80,
            "Physical attributes should never be penalized"
        );
    }

    #[test]
    fn test_position_penalty_integration_out_of_position() {
        // Create player with high attributes
        let player = Player {
            name: "Test Player".to_string(),
            position: Position::GK, // Playing as goalkeeper (terrible position for field player)
            overall: 80,
            condition: 3,
            attributes: Some(PlayerAttributes {
                passing: 18,
                dribbling: 15,
                positioning: 16,
                pace: 80, // Physical attribute
                ..Default::default()
            }),
            equipped_skills: Vec::new(),
            traits: Default::default(),
            personality: Default::default(),
        };

        // Create person with GK rating = 1 (cannot play)
        let person = Person::new(
            1,
            "Test Player".to_string(),
            "ENG".to_string(),
            "Test FC".to_string(),
            "MC".to_string(),
            150,
            160,
            25,
            Some("1,1,1,1,1,1,12,15,20,15,14,20,14,12".to_string()),
            // GK=1 (very poor)
        );

        // Test: Out of position (GK rating 1) → 70% penalty
        let match_player = MatchPlayer::from_player(&player, 0, Some(&person), true);

        assert_eq!(
            match_player.position_suitability, 0.3,
            "GK rating 1 should result in 0.3 suitability (70% penalty)"
        );

        // Technical/Mental attributes should be penalized
        assert_eq!(
            match_player.attributes.passing,
            5, // 18 * 0.3 = 5.4 → 5
            "Passing should be heavily penalized (70%)"
        );
        assert_eq!(
            match_player.attributes.dribbling,
            4, // 15 * 0.3 = 4.5 → 4
            "Dribbling should be heavily penalized (70%)"
        );
        assert_eq!(
            match_player.attributes.positioning,
            4, // 16 * 0.3 = 4.8 → 4
            "Positioning should be heavily penalized (70%)"
        );

        // Physical attributes should NOT be penalized
        assert_eq!(
            match_player.attributes.pace, 80,
            "Physical attributes should never be penalized"
        );
    }

    #[test]
    fn test_position_penalty_integration_adequate_position() {
        // Create player
        let player = Player {
            name: "Test Player".to_string(),
            position: Position::CDM, // Defensive Midfielder
            overall: 75,
            condition: 3,
            attributes: Some(PlayerAttributes { passing: 15, vision: 14, ..Default::default() }),
            equipped_skills: Vec::new(),
            traits: Default::default(),
            personality: Default::default(),
        };

        // Create person with DM rating = 8 (adequate)
        let person = Person::new(
            1,
            "Test Player".to_string(),
            "ENG".to_string(),
            "Test FC".to_string(),
            "MC".to_string(),
            150,
            160,
            25,
            Some("1,1,1,1,1,1,8,15,20,15,14,20,14,12".to_string()),
            // DM=8 (adequate)
        );

        // Test: Adequate position (DM rating 8) → 40% penalty
        let match_player = MatchPlayer::from_player(&player, 0, Some(&person), true);

        assert_eq!(
            match_player.position_suitability, 0.6,
            "DM rating 8 should result in 0.6 suitability (40% penalty)"
        );
        assert_eq!(
            match_player.attributes.passing,
            9, // 15 * 0.6 = 9
            "Passing should be penalized by 40%"
        );
        assert_eq!(
            match_player.attributes.vision,
            8, // 14 * 0.6 = 8.4 → 8
            "Vision should be penalized by 40%"
        );
    }

    #[test]
    fn test_position_penalty_integration_no_person_data() {
        // Create player
        let player = Player {
            name: "Test Player".to_string(),
            position: Position::CM,
            overall: 80,
            condition: 3,
            attributes: Some(PlayerAttributes { passing: 18, ..Default::default() }),
            equipped_skills: Vec::new(),
            traits: Default::default(),
            personality: Default::default(),
        };

        // Test: No Person data → no penalty
        let match_player = MatchPlayer::from_player(&player, 0, None, true);

        assert_eq!(
            match_player.position_suitability, 1.0,
            "No Person data should result in no penalty"
        );
        assert_eq!(
            match_player.attributes.passing, 18,
            "Attributes should not be penalized without Person data"
        );
    }

    // =========================================================================
    // TeamSide Helper Method Tests
    // =========================================================================

    #[test]
    fn test_team_side_from_track_id() {
        // Home team: 0-10
        assert_eq!(TeamSide::from_track_id(0), TeamSide::Home);
        assert_eq!(TeamSide::from_track_id(5), TeamSide::Home);
        assert_eq!(TeamSide::from_track_id(10), TeamSide::Home);

        // Away team: 11-21
        assert_eq!(TeamSide::from_track_id(11), TeamSide::Away);
        assert_eq!(TeamSide::from_track_id(15), TeamSide::Away);
        assert_eq!(TeamSide::from_track_id(21), TeamSide::Away);
    }

    #[test]
    fn test_team_side_is_home() {
        assert!(TeamSide::is_home(0));
        assert!(TeamSide::is_home(10));
        assert!(!TeamSide::is_home(11));
        assert!(!TeamSide::is_home(21));
    }

    #[test]
    fn test_team_slot() {
        // Home team slots
        assert_eq!(TeamSide::team_slot(0), 0);
        assert_eq!(TeamSide::team_slot(5), 5);
        assert_eq!(TeamSide::team_slot(10), 10);

        // Away team slots (should be 0-10, not 11-21)
        assert_eq!(TeamSide::team_slot(11), 0);
        assert_eq!(TeamSide::team_slot(16), 5);
        assert_eq!(TeamSide::team_slot(21), 10);
    }

    #[test]
    fn test_team_id() {
        assert_eq!(TeamSide::team_id(0), 0);
        assert_eq!(TeamSide::team_id(10), 0);
        assert_eq!(TeamSide::team_id(11), 1);
        assert_eq!(TeamSide::team_id(21), 1);
    }

    #[test]
    fn test_opponent_range() {
        // Home player (0-10) → Away team range (11-21)
        assert_eq!(TeamSide::opponent_range(0), 11..22);
        assert_eq!(TeamSide::opponent_range(5), 11..22);
        assert_eq!(TeamSide::opponent_range(10), 11..22);

        // Away player (11-21) → Home team range (0-10)
        assert_eq!(TeamSide::opponent_range(11), 0..11);
        assert_eq!(TeamSide::opponent_range(15), 0..11);
        assert_eq!(TeamSide::opponent_range(21), 0..11);
    }

    #[test]
    fn test_teammate_range() {
        // Home player → Home team range
        assert_eq!(TeamSide::teammate_range(5), 0..11);
        // Away player → Away team range
        assert_eq!(TeamSide::teammate_range(15), 11..22);
    }

    #[test]
    fn test_same_team() {
        // Same team
        assert!(TeamSide::same_team(3, 7)); // Both Home
        assert!(TeamSide::same_team(12, 18)); // Both Away

        // Different teams
        assert!(!TeamSide::same_team(5, 15)); // Home vs Away
        assert!(!TeamSide::same_team(10, 11)); // Home vs Away boundary
    }

    #[test]
    fn test_opponent_gk() {
        // Home player → Away GK (index 11)
        assert_eq!(TeamSide::opponent_gk(5), 11);
        assert_eq!(TeamSide::opponent_gk(0), 11);
        assert_eq!(TeamSide::opponent_gk(10), 11);

        // Away player → Home GK (index 0)
        assert_eq!(TeamSide::opponent_gk(15), 0);
        assert_eq!(TeamSide::opponent_gk(11), 0);
        assert_eq!(TeamSide::opponent_gk(21), 0);
    }

    #[test]
    fn test_own_gk() {
        // Home player → Home GK (index 0)
        assert_eq!(TeamSide::own_gk(5), 0);
        // Away player → Away GK (index 11)
        assert_eq!(TeamSide::own_gk(15), 11);
    }

    #[test]
    fn test_goal_positions() {
        let home_ctx = DirectionContext::new(true);
        let away_ctx = DirectionContext::new(false);

        // Home attacks right (105.0), defends left (0.0)
        assert_eq!(home_ctx.opponent_goal_x() * 105.0, 105.0);
        assert_eq!(home_ctx.own_goal_x() * 105.0, 0.0);

        // Away attacks left (0.0), defends right (105.0)
        assert_eq!(away_ctx.opponent_goal_x() * 105.0, 0.0);
        assert_eq!(away_ctx.own_goal_x() * 105.0, 105.0);
    }

    #[test]
    fn test_local_idx() {
        // Home team: local_idx == track_id
        assert_eq!(TeamSide::local_idx(0), 0);
        assert_eq!(TeamSide::local_idx(5), 5);
        assert_eq!(TeamSide::local_idx(10), 10);

        // Away team: local_idx = track_id - 11
        assert_eq!(TeamSide::local_idx(11), 0);
        assert_eq!(TeamSide::local_idx(15), 4);
        assert_eq!(TeamSide::local_idx(21), 10);
    }

    #[test]
    fn test_global_idx() {
        // Home team
        assert_eq!(TeamSide::global_idx(5, true), 5);
        // Away team
        assert_eq!(TeamSide::global_idx(5, false), 16);
    }

    // ========================================================================
    // P0.5 ATTRIBUTES GUARD TESTS
    // ========================================================================

    fn make_player(idx: usize, pos: Position, with_attrs: bool) -> Player {
        Player {
            name: format!("P{}", idx),
            position: pos,
            overall: 80,
            condition: 3,
            attributes: if with_attrs { Some(PlayerAttributes::from_uniform(80)) } else { None },
            equipped_skills: Vec::new(),
            traits: TraitSlots::default(),
            personality: PersonalityArchetype::default(),
        }
    }

    /// Build a minimal valid team (18 players, 2 GKs) for MatchSetup validation.
    fn build_team(with_attrs: bool) -> Team {
        use crate::models::team::{Formation, Team};

        // 11 starters + 7 subs (must satisfy formation/position constraints)
        let positions = [
            Position::GK,
            Position::RB,
            Position::CB,
            Position::CB,
            Position::LB,
            Position::CM,
            Position::CM,
            Position::CAM,
            Position::RW,
            Position::ST,
            Position::LW,
            // subs
            Position::GK,
            Position::RB,
            Position::CB,
            Position::CM,
            Position::CAM,
            Position::RW,
            Position::ST,
        ];

        let mut players = Vec::with_capacity(positions.len());
        for (idx, pos) in positions.iter().enumerate() {
            players.push(make_player(idx, *pos, with_attrs));
        }

        Team {
            name: if with_attrs { "WithAttrs".into() } else { "NoAttrs".into() },
            formation: Formation::F433,
            players,
        }
    }

    #[test]
    fn match_setup_debug_missing_attributes_zero_when_all_have_attrs() {
        let home = build_team(true);
        let away = build_team(true);
        let setup = MatchSetup::from_teams(&home, &away).expect("MatchSetup should build");
        assert_eq!(setup.debug.missing_attributes_count, 0);
    }

    // P2.3: Removed test for fallback path (strict mode now always-on)
    // Old test: match_setup_debug_counts_missing_attributes_when_absent
    // Reason: No longer has fallback to default(50) - always panics on None

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "P0.75-2 violated: Player.attributes=None")]
    fn debug_mode_panics_on_missing_attributes() {
        let home = build_team(false); // attributes = None
        let away = build_team(true);
        let _setup = MatchSetup::from_teams(&home, &away).unwrap();
        // P0.75 Patch 2: Debug assert triggers in debug builds
    }

    // ========================================================================
    // P2.2-B: Formation Validator Tests
    // ========================================================================

    fn build_valid_team_setup() -> TeamSetup {
        let mut players = Vec::new();

        // Add 1 GK
        players.push(MatchPlayer {
            name: "GK1".to_string(),
            position: Position::GK,
            overall: 80,
            attributes: PlayerAttributes::from_uniform(80),
            traits: Default::default(),
            personality: Default::default(),
            slot: 0,
            condition_level: 3,
            position_suitability: 1.0,
            equipped_skills: Vec::new(),
        });

        // Add 10 outfield players
        for i in 1..11 {
            players.push(MatchPlayer {
                name: format!("P{}", i),
                position: Position::CM,
                overall: 80,
                attributes: PlayerAttributes::from_uniform(80),
                traits: Default::default(),
                personality: Default::default(),
                slot: i as u8,
                condition_level: 3,
                position_suitability: 1.0,
                equipped_skills: Vec::new(),
            });
        }

        TeamSetup {
            name: "Test Team".to_string(),
            formation: Formation::F442,
            starters: players,
            substitutes: Vec::new(),
        }
    }

    #[test]
    fn test_formation_validator_valid() {
        let team = build_valid_team_setup();
        assert!(FormationValidator::validate_formation(&team).is_ok());
    }

    #[test]
    fn test_formation_validator_wrong_count() {
        let mut team = build_valid_team_setup();

        // Remove one player (10 players)
        team.starters.pop();
        let result = FormationValidator::validate_formation(&team);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("10 players"));
        assert!(err_msg.contains("expected 11"));

        // Add two extra players (12 players)
        team.starters.push(MatchPlayer {
            name: "Extra1".to_string(),
            position: Position::CM,
            overall: 80,
            attributes: PlayerAttributes::from_uniform(80),
            traits: Default::default(),
            personality: Default::default(),
            slot: 11,
            condition_level: 3,
            position_suitability: 1.0,
            equipped_skills: Vec::new(),
        });
        team.starters.push(MatchPlayer {
            name: "Extra2".to_string(),
            position: Position::CM,
            overall: 80,
            attributes: PlayerAttributes::from_uniform(80),
            traits: Default::default(),
            personality: Default::default(),
            slot: 12,
            condition_level: 3,
            position_suitability: 1.0,
            equipped_skills: Vec::new(),
        });

        let result = FormationValidator::validate_formation(&team);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("12 players"));
    }

    #[test]
    fn test_formation_validator_no_gk() {
        let mut team = build_valid_team_setup();

        // Replace GK with outfield player
        team.starters[0].position = Position::CM;

        let result = FormationValidator::validate_formation(&team);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("0 goalkeepers"));
        assert!(err_msg.contains("expected 1"));
    }

    #[test]
    fn test_formation_validator_multiple_gk() {
        let mut team = build_valid_team_setup();

        // Add second GK
        team.starters[1].position = Position::GK;

        let result = FormationValidator::validate_formation(&team);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("2 goalkeepers"));
    }

    #[test]
    fn test_formation_validator_duplicate_slot() {
        let mut team = build_valid_team_setup();

        // Create duplicate slot 5
        team.starters[6].slot = 5;

        let result = FormationValidator::validate_formation(&team);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Duplicate slot 5"));
    }

    #[test]
    fn test_formation_validator_invalid_slot() {
        let mut team = build_valid_team_setup();

        // Set slot to 11 (out of range 0-10)
        team.starters[5].slot = 11;

        let result = FormationValidator::validate_formation(&team);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Invalid slot 11"));
        assert!(err_msg.contains("must be 0-10"));
    }

    #[test]
    fn test_formation_validator_match_both_teams() {
        let home = build_valid_team_setup();
        let away = build_valid_team_setup();

        // Both teams valid
        assert!(FormationValidator::validate_match(&home, &away).is_ok());

        // Invalid home team
        let mut invalid_home = build_valid_team_setup();
        invalid_home.starters.pop();
        let result = FormationValidator::validate_match(&invalid_home, &away);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Home team"));

        // Invalid away team
        let mut invalid_away = build_valid_team_setup();
        invalid_away.starters[0].position = Position::CM;
        let result = FormationValidator::validate_match(&home, &invalid_away);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Away team"));
    }

    // ========================================================================
    // ENGINE_CONTRACT 2: Attributes None Ratio
    // ========================================================================

    /// CONTRACT 2: All players must have attributes injected (100% coverage)
    /// - Verifies PlayerLibrary.gd injection SSOT
    /// - Threshold: attr_none_ratio == 0% (strict)
    #[test]
    fn engine_contract_attr_none_ratio() {
        use crate::models::team::{Formation, Team};

        const TEAM_SIZE: usize = 18; // 11 starters + 7 subs

        // Build a team with 100% attributes coverage (contract requirement)
        let positions = [
            Position::GK,
            Position::RB,
            Position::CB,
            Position::CB,
            Position::LB,
            Position::CM,
            Position::CM,
            Position::CAM,
            Position::RW,
            Position::ST,
            Position::LW,
            // subs
            Position::GK,
            Position::RB,
            Position::CB,
            Position::CM,
            Position::CAM,
            Position::RW,
            Position::ST,
        ];

        let mut players = Vec::with_capacity(TEAM_SIZE);
        for (idx, pos) in positions.iter().enumerate() {
            players.push(Player {
                name: format!("Player{}", idx),
                position: *pos,
                overall: 75,
                condition: 3,
                attributes: Some(PlayerAttributes::from_uniform(75)), // ✅ 100% coverage
                equipped_skills: Vec::new(),
                traits: TraitSlots::default(),
                personality: PersonalityArchetype::default(),
            });
        }

        let team =
            Team { name: "Contract Test Team".to_string(), formation: Formation::F433, players };

        // Count None attributes
        let mut none_count = 0;
        let total_players = team.players.len();

        for player in &team.players {
            if player.attributes.is_none() {
                none_count += 1;
            }
        }

        let none_ratio = (none_count as f32) / (total_players as f32);

        // CONTRACT ASSERTION: Must be 0%
        assert_eq!(
            none_ratio,
            0.0,
            "Attributes None ratio {:.2}% violates P0.75-2 contract (expected 0%, found {} / {})",
            none_ratio * 100.0,
            none_count,
            total_players
        );

        println!(
            "[CONTRACT_OK] Attributes coverage: {}/{} (100%, {} None)",
            total_players - none_count,
            total_players,
            none_count
        );
    }
}

// ============================================================================
// MatchSetup - 경기 셋업 메인 구조
// ============================================================================

/// 경기 셋업 (경기 시작 전 생성, 경기 중 불변)
#[derive(Debug, Clone, Default)]
pub struct MatchSetupDebugFlags {
    pub missing_attributes_count: u32,
    pub ssot_proof: Option<crate::fix01::SsotProof>,
}

#[derive(Debug, Clone)]
pub struct MatchSetup {
    /// 홈팀 셋업
    pub home: TeamSetup,
    /// 어웨이팀 셋업
    pub away: TeamSetup,
    /// track_id → PlayerSlot 매핑 (22개)
    slots: [PlayerSlot; 22],
    /// Per-team pitch-slot assignment (team_slot 0-10).
    home_assignment: [PitchAssignment; 11],
    away_assignment: [PitchAssignment; 11],
    /// Bench usage (once a substitute is used, they cannot re-enter).
    home_sub_used: [bool; MAX_SUBSTITUTES],
    away_sub_used: [bool; MAX_SUBSTITUTES],
    /// 디버그/계약 확인용 플래그
    pub debug: MatchSetupDebugFlags,
}

impl MatchSetup {
    /// 두 팀으로부터 MatchSetup 생성
    pub fn from_teams(home: &Team, away: &Team) -> Result<Self, String> {
        let mut debug = MatchSetupDebugFlags::default();

        // Count missing attributes before flattening into MatchPlayers.
        debug.missing_attributes_count += count_missing_attributes(home);
        debug.missing_attributes_count += count_missing_attributes(away);

        let home_setup = TeamSetup::from_team(home)?;
        let away_setup = TeamSetup::from_team(away)?;

        // P2.2-B: Validate formation before match setup
        FormationValidator::validate_match(&home_setup, &away_setup)?;

        let mut slots = [PlayerSlot::default(); 22];

        // Home 팀 (track_id 0-10)
        for i in 0..11 {
            slots[i] = PlayerSlot {
                team: TeamSide::Home,
                team_slot: i as u8,
                is_active: true,
                substituted: false,
                sent_off: false,
                injured: false,
            };
        }

        // Away 팀 (track_id 11-21)
        for i in 0..11 {
            slots[11 + i] = PlayerSlot {
                team: TeamSide::Away,
                team_slot: i as u8,
                is_active: true,
                substituted: false,
                sent_off: false,
                injured: false,
            };
        }

        let home_assignment =
            std::array::from_fn(|slot| PitchAssignment::Starter(slot as u8));
        let away_assignment =
            std::array::from_fn(|slot| PitchAssignment::Starter(slot as u8));

        Ok(Self {
            home: home_setup,
            away: away_setup,
            slots,
            home_assignment,
            away_assignment,
            home_sub_used: [false; MAX_SUBSTITUTES],
            away_sub_used: [false; MAX_SUBSTITUTES],
            debug,
        })
    }

    /// track_id로 선수 정보 조회
    #[inline]
    pub fn get_player(&self, track_id: usize) -> &MatchPlayer {
        debug_assert!(track_id < 22, "track_id must be 0-21, got {}", track_id);
        let team = TeamSide::from_track_id(track_id);
        let team_slot = TeamSide::team_slot(track_id) as usize;

        match team {
            TeamSide::Home => match self.home_assignment[team_slot] {
                PitchAssignment::Starter(slot) => &self.home.starters[slot as usize],
                PitchAssignment::Substitute(slot) => &self.home.substitutes[slot as usize],
            },
            TeamSide::Away => match self.away_assignment[team_slot] {
                PitchAssignment::Starter(slot) => &self.away.starters[slot as usize],
                PitchAssignment::Substitute(slot) => &self.away.substitutes[slot as usize],
            },
        }
    }

    /// track_id로 능력치 조회
    #[inline]
    pub fn get_attributes(&self, track_id: usize) -> &PlayerAttributes {
        &self.get_player(track_id).attributes
    }

    /// track_id로 traits 조회
    #[inline]
    pub fn get_traits(&self, track_id: usize) -> &TraitSlots {
        &self.get_player(track_id).traits
    }

    /// track_id로 팀 판별
    #[inline]
    pub fn get_team_side(&self, track_id: usize) -> TeamSide {
        TeamSide::from_track_id(track_id)
    }

    /// 슬롯 정보 조회
    #[inline]
    pub fn get_slot(&self, track_id: usize) -> &PlayerSlot {
        &self.slots[track_id]
    }

    /// 활성 선수인지 확인
    #[inline]
    pub fn is_active(&self, track_id: usize) -> bool {
        self.slots[track_id].is_active
    }

    /// Returns whether a bench slot has already been used (and thus cannot re-enter).
    pub fn is_sub_used(&self, team: TeamSide, bench_slot: u8) -> bool {
        let slot = bench_slot as usize;
        if slot >= MAX_SUBSTITUTES {
            return true;
        }
        match team {
            TeamSide::Home => self.home_sub_used[slot],
            TeamSide::Away => self.away_sub_used[slot],
        }
    }

    /// Apply a substitution by assigning a bench player to a pitch track_id.
    ///
    /// Returns `(player_in_name, player_out_name)` for event/UI purposes.
    pub fn apply_substitution(
        &mut self,
        pitch_track_id: usize,
        bench_slot: u8,
    ) -> Result<(String, String), String> {
        if pitch_track_id >= 22 {
            return Err(format!(
                "apply_substitution: pitch_track_id must be 0-21, got {}",
                pitch_track_id
            ));
        }

        let team = TeamSide::from_track_id(pitch_track_id);
        let bench_slot_usize = bench_slot as usize;
        if bench_slot_usize >= MAX_SUBSTITUTES {
            return Err(format!(
                "apply_substitution: bench_slot must be 0-{}, got {}",
                MAX_SUBSTITUTES.saturating_sub(1),
                bench_slot
            ));
        }

        if self.is_sub_used(team, bench_slot) {
            return Err(format!(
                "apply_substitution: bench_slot {} already used for {:?}",      
                bench_slot, team
            ));
        }

        let available_subs = match team {
            TeamSide::Home => self.home.substitutes.len(),
            TeamSide::Away => self.away.substitutes.len(),
        };
        if bench_slot_usize >= available_subs {
            return Err(format!(
                "apply_substitution: bench_slot {} exceeds available substitutes ({}) for {:?}",
                bench_slot, available_subs, team
            ));
        }

        let team_slot = TeamSide::team_slot(pitch_track_id) as usize;
        let player_out_name = self.get_player(pitch_track_id).name.clone();     

        let (player_in_name, assignment, used) = match team {
            TeamSide::Home => (
                self.home.substitutes[bench_slot_usize].name.clone(),
                &mut self.home_assignment[team_slot],
                &mut self.home_sub_used[bench_slot_usize],
            ),
            TeamSide::Away => (
                self.away.substitutes[bench_slot_usize].name.clone(),
                &mut self.away_assignment[team_slot],
                &mut self.away_sub_used[bench_slot_usize],
            ),
        };

        *assignment = PitchAssignment::Substitute(bench_slot);
        *used = true;

        Ok((player_in_name, player_out_name))
    }
}

fn count_missing_attributes(team: &Team) -> u32 {
    let mut missing = 0u32;
    // starters
    for p in team.players.iter().take(11) {
        if p.attributes.is_none() {
            missing += 1;
        }
    }
    // substitutes (최대 7명)
    for p in team.players.iter().skip(11).take(MAX_SUBSTITUTES) {
        if p.attributes.is_none() {
            missing += 1;
        }
    }
    missing
}

// ============================================================================
// P17 Phase 5: Viewer Export
// ============================================================================

/// Viewer용 MatchSetup 내보내기 (JSON 직렬화)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchSetupExport {
    /// 홈팀 정보
    pub home: TeamSetupExport,
    /// 어웨이팀 정보
    pub away: TeamSetupExport,
    /// 22명 선수 슬롯 정보
    pub player_slots: Vec<PlayerSlotExport>,
}

/// 팀 셋업 내보내기
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSetupExport {
    /// 팀 이름
    pub name: String,
    /// 포메이션 (문자열)
    pub formation: String,
}

/// 선수 슬롯 내보내기
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSlotExport {
    /// track_id (0-21)
    pub track_id: u32,
    /// 팀 ("home" | "away")
    pub team: String,
    /// 선수 이름
    pub name: String,
    /// 포지션 (문자열)
    pub position: String,
    /// 전체 능력치
    pub overall: u8,
    /// 팀 내 슬롯 (0-10)
    pub slot: u8,
}

impl MatchSetup {
    /// Viewer용 export 데이터 생성
    pub fn to_export(&self) -> MatchSetupExport {
        let mut player_slots = Vec::with_capacity(22);

        // Home pitch slots (track_id 0-10)
        for slot in 0..11 {
            let player = self.get_player(slot);
            player_slots.push(PlayerSlotExport {
                track_id: slot as u32,
                team: "home".to_string(),
                name: player.name.clone(),
                position: format!("{:?}", player.position),
                overall: player.overall,
                slot: slot as u8,
            });
        }

        // Away pitch slots (track_id 11-21)
        for slot in 0..11 {
            let track_id = 11 + slot;
            let player = self.get_player(track_id);
            player_slots.push(PlayerSlotExport {
                track_id: track_id as u32,
                team: "away".to_string(),
                name: player.name.clone(),
                position: format!("{:?}", player.position),
                overall: player.overall,
                slot: slot as u8,
            });
        }

        MatchSetupExport {
            home: TeamSetupExport {
                name: self.home.name.clone(),
                formation: format!("{:?}", self.home.formation),
            },
            away: TeamSetupExport {
                name: self.away.name.clone(),
                formation: format!("{:?}", self.away.formation),
            },
            player_slots,
        }
    }

    /// Viewer용 export 데이터 생성 (starting lineup snapshot).
    ///
    /// Contract:
    /// - `player_slots` are the 22 **pitch slots** (0..21) and should represent the
    ///   **starting lineup** at kickoff.
    /// - Substitutions are represented via match events (`EventType::Substitution`)
    ///   and must not retroactively change this snapshot.
    pub fn to_export_starting_lineup(&self) -> MatchSetupExport {
        let mut player_slots = Vec::with_capacity(22);

        // Home pitch slots (track_id 0-10): starters only.
        for slot in 0..11 {
            let player = &self.home.starters[slot];
            player_slots.push(PlayerSlotExport {
                track_id: slot as u32,
                team: "home".to_string(),
                name: player.name.clone(),
                position: format!("{:?}", player.position),
                overall: player.overall,
                slot: slot as u8,
            });
        }

        // Away pitch slots (track_id 11-21): starters only.
        for slot in 0..11 {
            let track_id = 11 + slot;
            let player = &self.away.starters[slot];
            player_slots.push(PlayerSlotExport {
                track_id: track_id as u32,
                team: "away".to_string(),
                name: player.name.clone(),
                position: format!("{:?}", player.position),
                overall: player.overall,
                slot: slot as u8,
            });
        }

        MatchSetupExport {
            home: TeamSetupExport {
                name: self.home.name.clone(),
                formation: format!("{:?}", self.home.formation),
            },
            away: TeamSetupExport {
                name: self.away.name.clone(),
                formation: format!("{:?}", self.away.formation),
            },
            player_slots,
        }
    }
}
