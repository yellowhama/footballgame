//! 선수의 전술적 목표 결정 시스템 (Tactical Brain)
//!
//! 선수가 "무엇을 할지" (What to do)를 결정하는 뇌(Brain) 역할
//! - 공격 시: 골문 향해 달릴지, 공간을 찾을지, 동료 지원할지
//! - 수비 시: 압박할지, 마크할지, 길목 차단할지
//!
//! 36개 능력치(멘탈 중심) × 포지션 가중치 = 최종 판단

use crate::models::match_setup::MatchPlayer;
use crate::models::player::{Player, Position};
use crate::player::personality::DecisionModifiers;

// =============================================================================
// FIX_2601/0109: Pressing Efficiency State (Open-Football Integration)
// =============================================================================

/// Tracks pressing state for a player to determine when to continue/stop pressing
///
/// Open-Football concept: Pressing should be efficient - stop if:
/// - Pressing for too long (energy waste)
/// - Better positioned teammates available (let them press)
#[derive(Debug, Clone, Copy, Default)]
pub struct PressingState {
    /// Tick when pressing started
    pub started_tick: u32,
    /// Last tick when efficiency was checked
    pub last_check_tick: u32,
}

impl PressingState {
    /// Maximum duration of pressing before forced recovery (~30 seconds at 4 ticks/sec)
    pub const MAX_PRESSING_TICKS: u32 = 120;
    /// Interval between efficiency checks
    pub const CHECK_INTERVAL: u32 = 30;

    /// Create new pressing state starting at given tick
    pub fn new(current_tick: u32) -> Self {
        Self { started_tick: current_tick, last_check_tick: current_tick }
    }

    /// Check if player should continue pressing
    ///
    /// Returns false if:
    /// - Pressing duration exceeded maximum
    /// - 2+ teammates are closer to the ball (let them press)
    pub fn should_continue(
        &self,
        current_tick: u32,
        my_distance_to_ball: f32,
        teammate_distances: &[f32],
    ) -> bool {
        // Check max duration
        if current_tick.saturating_sub(self.started_tick) > Self::MAX_PRESSING_TICKS {
            return false;
        }

        // Count teammates better positioned (closer by 20%+)
        let threshold = my_distance_to_ball * 0.8;
        let better_positioned_count = teammate_distances.iter().filter(|&&d| d < threshold).count();

        // If 2+ teammates are closer, let them press
        better_positioned_count < 2
    }

    /// Update last check tick
    pub fn update_check(&mut self, current_tick: u32) {
        self.last_check_tick = current_tick;
    }

    /// Check if efficiency check is due
    pub fn needs_efficiency_check(&self, current_tick: u32) -> bool {
        current_tick.saturating_sub(self.last_check_tick) >= Self::CHECK_INTERVAL
    }
}

// =============================================================================
// FIX_2601/0109: Space Creation Types (Open-Football Integration)
// =============================================================================

/// Types of space creation movements in Open-Football tactical system
///
/// Each type represents a different way to find/create space on the pitch:
/// - HalfSpace: Move into half-space zones (1/3, 2/3 width corridors)
/// - BetweenLines: Find gaps between opponent's midfield and defense lines
/// - WideOverload: Create numerical advantage on one wing
/// - DeepPocket: Find low congestion areas deep in opponent half
/// - ThirdManRun: Make run beyond the immediate pass recipient
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SpaceCreationType {
    /// Half-space zones - 1/3, 2/3 width corridors between center and wing
    #[default]
    HalfSpace,
    /// Gap between opponent MF and DEF lines
    BetweenLines,
    /// Numerical advantage on wing (overload)
    WideOverload,
    /// Low congestion area in attacking third
    DeepPocket,
    /// Run beyond next pass recipient
    ThirdManRun,
}

impl SpaceCreationType {
    /// Determine best space creation type based on context
    ///
    /// # Arguments
    /// * `player_x` - Player's lateral position (0.0 = left, 1.0 = right)
    /// * `ball_x` - Ball's lateral position
    /// * `player_y` - Player's vertical position (0.0 = own goal, 1.0 = opponent goal)
    /// * `is_forward` - Whether player is a forward/attacker
    pub fn determine(player_x: f32, ball_x: f32, player_y: f32, is_forward: bool) -> Self {
        // Forwards in attacking third prefer deep runs
        if is_forward && player_y > 0.65 {
            // Wide players prefer overload, central prefer third man
            if !(0.25..=0.75).contains(&player_x) {
                return SpaceCreationType::WideOverload;
            } else {
                return SpaceCreationType::ThirdManRun;
            }
        }

        // Midfielders prefer between-lines or half-space
        if player_y > 0.35 && player_y < 0.65 {
            // If ball is central, move to half-space
            if ball_x > 0.35 && ball_x < 0.65 {
                return SpaceCreationType::HalfSpace;
            } else {
                return SpaceCreationType::BetweenLines;
            }
        }

        // Attacking third - find pockets
        if player_y > 0.55 {
            return SpaceCreationType::DeepPocket;
        }

        // Default to half-space for general movement
        SpaceCreationType::HalfSpace
    }
}

// =============================================================================
// Tactical Goals
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OffensiveGoal {
    /// 공을 받으러 이동 (직접 관여)
    MoveToBall,

    /// 골문을 향해 침투 (득점 노림)
    AttackGoal,

    /// 빈 공간 찾아 들어가기 (기회 창출)
    FindSpace,

    /// 동료 근처로 가서 패스 선택지 제공 (연계)
    SupportTeammate,

    /// 현재 위치 사수 (빌드업 안정성/체력 안배)
    HoldPosition,
}

/// 수비 시 선수가 가질 수 있는 전술적 목표
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefensiveGoal {
    /// 공 가진 상대를 압박 (직접 탈취 시도)
    PressOpponent,

    /// 수비 진영으로 전력 질주 (복귀)
    TrackBack,

    /// 특정 상대를 맨마킹 (대인 방어)
    MarkPlayer,

    /// 패스 경로를 예측하여 차단 (지능적 수비)
    BlockPassingLane,

    /// 지역 방어 유지 (Zone Defense)
    HoldPosition,
}

impl Player {
    /// 포지션에 따른 공격 목표 가중치 반환
    ///
    /// 반환 순서: (MoveToBall, AttackGoal, FindSpace, Support, Hold)
    fn get_offensive_bias(&self) -> (f32, f32, f32, f32, f32) {
        match self.position {
            // 공격수 (ST/CF): 골 넣는 게 직업
            Position::ST | Position::CF => (1.2, 3.0, 2.0, 0.8, 0.1),

            // 윙어 (LW/RW): 공간 침투와 지원
            Position::LW | Position::RW => (1.0, 1.5, 2.5, 1.5, 0.5),

            // 공격형 미드 (CAM): 공간 찾기와 킬패스 각(Support)
            Position::CAM => (1.5, 1.2, 2.0, 2.0, 0.5),

            // 중앙 미드 (CM/CDM): 연결고리 및 밸런스 유지
            Position::CM | Position::CDM => (1.5, 0.5, 1.0, 3.0, 2.0),

            // 측면 미드 (LM/RM): 윙어와 비슷하지만 더 안정적
            Position::LM | Position::RM => (1.2, 1.0, 2.0, 2.0, 1.0),

            // 풀백 (LB/RB/LWB/RWB): 올라갈 땐 올라가되 기본은 수비
            Position::LB | Position::RB | Position::LWB | Position::RWB => {
                (0.5, 0.2, 1.0, 2.0, 3.0)
            }

            // 센터백 (CB): 절대 가출 금지 (Hold 최우선)
            Position::CB => (0.2, 0.1, 0.5, 1.5, 4.0),

            // 골키퍼: 골대 사수
            Position::GK => (0.0, 0.0, 0.0, 1.0, 10.0),

            // 제너릭 포지션
            Position::FW => (1.2, 2.5, 2.0, 1.0, 0.3),
            Position::MF => (1.5, 0.8, 1.5, 2.5, 1.5),
            Position::DF => (0.3, 0.1, 0.8, 1.8, 3.5),
        }
    }

    /// 포지션에 따른 수비 목표 가중치 반환
    ///
    /// 반환 순서: (Press, TrackBack, Mark, Block, Hold)
    fn get_defensive_bias(&self) -> (f32, f32, f32, f32, f32) {
        match self.position {
            // 공격수: 전방 압박(Press) 위주, 수비 복귀는 잘 안 함
            Position::ST | Position::CF => (2.5, 0.5, 0.5, 1.5, 1.0),

            // 윙어: 측면 수비 가담 필요
            Position::LW | Position::RW => (1.5, 1.5, 1.0, 1.5, 1.0),

            // 공격형 미드: 역습 막기 위해 복귀도 중요
            Position::CAM => (1.5, 2.0, 1.5, 2.0, 1.5),

            // 중앙 미드: 허리 싸움 (압박 + 길목 차단)
            Position::CM => (2.0, 2.0, 2.0, 2.5, 2.0),

            // 수비형 미드: 수비수처럼 지역 방어 중요
            Position::CDM => (1.5, 1.5, 2.5, 3.0, 3.0),

            // 측면 미드: 자기 라인 복귀 필수
            Position::LM | Position::RM => (1.5, 2.5, 2.0, 2.0, 2.0),

            // 풀백: 윙어 마크하며 라인 유지
            Position::LB | Position::RB | Position::LWB | Position::RWB => {
                (1.0, 1.0, 3.0, 2.5, 3.5)
            }

            // 센터백: 지역 방어(Hold)와 대인 마크(Mark)가 생명
            // 섣불리 튀어나가면(Press) 안 됨
            Position::CB => (0.8, 0.5, 3.0, 3.0, 4.0),

            // 골키퍼: 골대만 지키면 됨
            Position::GK => (0.0, 0.0, 0.0, 0.0, 10.0),

            // 제너릭 포지션
            Position::FW => (2.0, 1.0, 0.8, 1.5, 1.0),
            Position::MF => (1.8, 2.0, 2.0, 2.3, 2.0),
            Position::DF => (0.9, 0.8, 3.0, 2.8, 3.8),
        }
    }

    /// 공격 상황에서 전술적 목표 결정
    ///
    /// # Arguments
    /// * `ball_pos` - 현재 공의 위치 (향후 확장용)
    /// * `is_user_player` - 주인공 선수인지 여부
    ///
    /// # Returns
    /// 선수가 추구할 공격 목표
    pub fn decide_offensive_goal(
        &self,
        _ball_pos: (f32, f32),
        is_user_player: bool,
    ) -> OffensiveGoal {
        use super::physics_constants;

        let attr = match &self.attributes {
            Some(a) => a,
            None => return OffensiveGoal::HoldPosition, // 능력치 없으면 유지
        };

        let (w_ball, w_goal, w_space, w_support, w_hold) = self.get_offensive_bias();
        let mods: DecisionModifiers = self.decision_modifiers();

        // 1. MoveToBall: 적극성, 활동량
        let mut s_ball =
            (attr.aggression as f32 + attr.work_rate as f32) * w_ball * mods.move_to_ball;

        // 2. AttackGoal: 오프더볼, 공격성, 결정력
        let mut s_goal = (attr.off_the_ball as f32 * 0.5
            + attr.aggression as f32 * 0.3
            + attr.finishing as f32 * 0.2)
            * w_goal
            * mods.attack_goal;

        // 3. FindSpace: 시야, 오프더볼, 천재성
        let s_space =
            (attr.vision as f32 * 0.3 + attr.off_the_ball as f32 * 0.4 + attr.flair as f32 * 0.3)
                * w_space
                * mods.find_space;

        // 4. Support: 팀워크, 시야, 패스
        let s_support =
            (attr.teamwork as f32 * 0.4 + attr.vision as f32 * 0.3 + attr.passing as f32 * 0.3)
                * w_support
                * mods.support;

        // 5. Hold: 판단력, 위치선정, (반비례)공격성
        // 공격성이 낮을수록 자리를 지키려는 경향
        let calmness = 20.0 - (attr.aggression as f32 / 10.0);
        let mut s_hold =
            (attr.decisions as f32 * 0.4 + attr.positioning as f32 * 0.4 + calmness * 0.2)
                * w_hold
                * mods.hold_offense;

        // 🔥 B3: Hero's Will - "나에게 공을 다오"
        // 주인공은 더 적극적으로 움직인다
        if is_user_player {
            s_ball *= physics_constants::hero_gravity::MOVE_TO_BALL_MULTIPLIER; // 1.5x
            s_goal *= physics_constants::hero_gravity::ATTACK_GOAL_MULTIPLIER; // 1.2x
            s_hold *= physics_constants::hero_gravity::HOLD_BALL_MULTIPLIER; // 0.5x (덜 소극적)
        }

        // 최종 선택 (가장 높은 점수)
        let max_score = s_ball.max(s_goal).max(s_space).max(s_support).max(s_hold);

        if (s_goal - max_score).abs() < 0.01 {
            OffensiveGoal::AttackGoal
        } else if (s_space - max_score).abs() < 0.01 {
            OffensiveGoal::FindSpace
        } else if (s_support - max_score).abs() < 0.01 {
            OffensiveGoal::SupportTeammate
        } else if (s_ball - max_score).abs() < 0.01 {
            OffensiveGoal::MoveToBall
        } else {
            OffensiveGoal::HoldPosition
        }
    }

    /// 수비 상황에서 전술적 목표 결정
    ///
    /// # Arguments
    /// * `distance_to_ball` - 공까지의 거리 (m)
    ///
    /// # Returns
    /// 선수가 추구할 수비 목표
    pub fn decide_defensive_goal(&self, distance_to_ball: f32) -> DefensiveGoal {
        let attr = match &self.attributes {
            Some(a) => a,
            None => return DefensiveGoal::HoldPosition, // 능력치 없으면 유지
        };

        let (w_press, w_track, w_mark, w_block, w_hold) = self.get_defensive_bias();
        let mods: DecisionModifiers = self.decision_modifiers();

        // 거리 기반 압박 보정 (가까우면 압박 본능 발동)
        let dist_factor = if distance_to_ball < 10.0 {
            2.5
        } else if distance_to_ball < 20.0 {
            1.5
        } else {
            1.0
        };

        // 1. Press: 공격성, 용기, 활동량
        let s_press = (attr.aggression as f32 * 0.5
            + attr.bravery as f32 * 0.3
            + attr.work_rate as f32 * 0.2)
            * w_press
            * dist_factor
            * mods.press;

        // 2. TrackBack: 활동량, 팀워크, 속도
        let s_track =
            (attr.work_rate as f32 * 0.4 + attr.teamwork as f32 * 0.4 + attr.pace as f32 * 0.2)
                * w_track
                * mods.track_back;

        // 3. Mark: 마킹, 힘, 집중력
        let s_mark = (attr.marking as f32 * 0.5
            + attr.strength as f32 * 0.3
            + attr.concentration as f32 * 0.2)
            * w_mark
            * mods.mark;

        // 4. Block: 예측력, 위치선정 (지능 수비)
        let s_block =
            (attr.anticipation as f32 * 0.5 + attr.positioning as f32 * 0.5) * w_block * mods.block;

        // 5. Hold: 위치선정, 침착성, 집중력
        let s_hold = (attr.positioning as f32 * 0.4
            + attr.composure as f32 * 0.3
            + attr.concentration as f32 * 0.3)
            * w_hold
            * mods.hold_defense;

        // 최종 선택
        let max_score = s_press.max(s_track).max(s_mark).max(s_block).max(s_hold);

        if (s_press - max_score).abs() < 0.01 {
            DefensiveGoal::PressOpponent
        } else if (s_mark - max_score).abs() < 0.01 {
            DefensiveGoal::MarkPlayer
        } else if (s_block - max_score).abs() < 0.01 {
            DefensiveGoal::BlockPassingLane
        } else if (s_track - max_score).abs() < 0.01 {
            DefensiveGoal::TrackBack
        } else {
            DefensiveGoal::HoldPosition
        }
    }

    /// Calculate curve shot skill level based on player attributes
    pub fn get_curve_level(&self) -> super::ball::CurveLevel {
        let attr = match &self.attributes {
            Some(a) => a,
            None => return super::ball::CurveLevel::None,
        };

        // Lv3: Elite curve shot (free_kicks > 18, technique > 17, flair > 16)
        if attr.free_kicks > 18 && attr.technique > 17 && attr.flair > 16 {
            return super::ball::CurveLevel::Lv3;
        }

        // Lv2: Advanced curve shot (free_kicks > 15, technique > 15)
        if attr.free_kicks > 15 && attr.technique > 15 {
            return super::ball::CurveLevel::Lv2;
        }

        // Lv1: Basic curve shot (free_kicks > 12, technique > 12)
        if attr.free_kicks > 12 && attr.technique > 12 {
            return super::ball::CurveLevel::Lv1;
        }

        // None: No curve ability
        super::ball::CurveLevel::None
    }
}

impl MatchPlayer {
    /// 포지션에 따른 공격 목표 가중치 반환
    ///
    /// 반환 순서: (MoveToBall, AttackGoal, FindSpace, Support, Hold)
    fn get_offensive_bias(&self) -> (f32, f32, f32, f32, f32) {
        match self.position {
            // 공격수 (ST/CF): 골 넣는 게 직업
            Position::ST | Position::CF => (1.2, 3.0, 2.0, 0.8, 0.1),

            // 윙어 (LW/RW): 공간 침투와 지원
            Position::LW | Position::RW => (1.0, 1.5, 2.5, 1.5, 0.5),

            // 공격형 미드 (CAM): 공간 찾기와 킬패스 각(Support)
            Position::CAM => (1.5, 1.2, 2.0, 2.0, 0.5),

            // 중앙 미드 (CM/CDM): 연결고리 및 밸런스 유지
            Position::CM | Position::CDM => (1.5, 0.5, 1.0, 3.0, 2.0),

            // 측면 미드 (LM/RM): 윙어와 비슷하지만 더 안정적
            Position::LM | Position::RM => (1.2, 1.0, 2.0, 2.0, 1.0),

            // 풀백 (LB/RB/LWB/RWB): 올라갈 땐 올라가되 기본은 수비
            Position::LB | Position::RB | Position::LWB | Position::RWB => (0.5, 0.2, 1.0, 2.0, 3.0),

            // 센터백 (CB): 절대 가출 금지 (Hold 최우선)
            Position::CB => (0.2, 0.1, 0.5, 1.5, 4.0),

            // 골키퍼: 골대 사수
            Position::GK => (0.0, 0.0, 0.0, 1.0, 10.0),

            // 제너릭 포지션
            Position::FW => (1.2, 2.5, 2.0, 1.0, 0.3),
            Position::MF => (1.5, 0.8, 1.5, 2.5, 1.5),
            Position::DF => (0.3, 0.1, 0.8, 1.8, 3.5),
        }
    }

    /// 포지션에 따른 수비 목표 가중치 반환
    ///
    /// 반환 순서: (Press, TrackBack, Mark, Block, Hold)
    fn get_defensive_bias(&self) -> (f32, f32, f32, f32, f32) {
        match self.position {
            // 공격수: 전방 압박(Press) 위주, 수비 복귀는 잘 안 함
            Position::ST | Position::CF => (2.5, 0.5, 0.5, 1.5, 1.0),

            // 윙어: 측면 수비 가담 필요
            Position::LW | Position::RW => (1.5, 1.5, 1.0, 1.5, 1.0),

            // 공격형 미드: 역습 막기 위해 복귀도 중요
            Position::CAM => (1.5, 2.0, 1.5, 2.0, 1.5),

            // 중앙 미드: 허리 싸움 (압박 + 길목 차단)
            Position::CM => (2.0, 2.0, 2.0, 2.5, 2.0),

            // 수비형 미드: 수비수처럼 지역 방어 중요
            Position::CDM => (1.5, 1.5, 2.5, 3.0, 3.0),

            // 측면 미드: 자기 라인 복귀 필수
            Position::LM | Position::RM => (1.5, 2.5, 2.0, 2.0, 2.0),

            // 풀백: 윙어 마크하며 라인 유지
            Position::LB | Position::RB | Position::LWB | Position::RWB => (1.0, 1.0, 3.0, 2.5, 3.5),

            // 센터백: 지역 방어(Hold)와 대인 마크(Mark)가 생명
            // 섣불리 튀어나가면(Press) 안 됨
            Position::CB => (0.8, 0.5, 3.0, 3.0, 4.0),

            // 골키퍼: 골대만 지키면 됨
            Position::GK => (0.0, 0.0, 0.0, 0.0, 10.0),

            // 제너릭 포지션
            Position::FW => (2.0, 1.0, 0.8, 1.5, 1.0),
            Position::MF => (1.8, 2.0, 2.0, 2.3, 2.0),
            Position::DF => (0.9, 0.8, 3.0, 2.8, 3.8),
        }
    }

    /// 공격 상황에서 전술적 목표 결정
    ///
    /// # Arguments
    /// * `ball_pos` - 현재 공의 위치 (향후 확장용)
    /// * `is_user_player` - 주인공 선수인지 여부
    ///
    /// # Returns
    /// 선수가 추구할 공격 목표
    pub fn decide_offensive_goal(
        &self,
        _ball_pos: (f32, f32),
        is_user_player: bool,
    ) -> OffensiveGoal {
        use super::physics_constants;

        let attr = &self.attributes;
        let (w_ball, w_goal, w_space, w_support, w_hold) = self.get_offensive_bias();
        let mods: DecisionModifiers = self.personality.decision_modifiers();

        // 1. MoveToBall: 적극성, 활동량
        let mut s_ball =
            (attr.aggression as f32 + attr.work_rate as f32) * w_ball * mods.move_to_ball;

        // 2. AttackGoal: 오프더볼, 공격성, 결정력
        let mut s_goal = (attr.off_the_ball as f32 * 0.5
            + attr.aggression as f32 * 0.3
            + attr.finishing as f32 * 0.2)
            * w_goal
            * mods.attack_goal;

        // 3. FindSpace: 시야, 오프더볼, 천재성
        let s_space =
            (attr.vision as f32 * 0.3 + attr.off_the_ball as f32 * 0.4 + attr.flair as f32 * 0.3)
                * w_space
                * mods.find_space;

        // 4. Support: 팀워크, 시야, 패스
        let s_support =
            (attr.teamwork as f32 * 0.4 + attr.vision as f32 * 0.3 + attr.passing as f32 * 0.3)
                * w_support
                * mods.support;

        // 5. Hold: 판단력, 위치선정, (반비례)공격성
        // 공격성이 낮을수록 자리를 지키려는 경향
        let calmness = 20.0 - (attr.aggression as f32 / 10.0);
        let mut s_hold = (attr.decisions as f32 * 0.4
            + attr.positioning as f32 * 0.4
            + calmness * 0.2)
            * w_hold
            * mods.hold_offense;

        // 🔥 B3: Hero's Will - "나에게 공을 다오"
        // 주인공은 더 적극적으로 움직인다
        if is_user_player {
            s_ball *= physics_constants::hero_gravity::MOVE_TO_BALL_MULTIPLIER; // 1.5x
            s_goal *= physics_constants::hero_gravity::ATTACK_GOAL_MULTIPLIER; // 1.2x
            s_hold *= physics_constants::hero_gravity::HOLD_BALL_MULTIPLIER; // 0.5x (덜 소극적)
        }

        // 최종 선택 (가장 높은 점수)
        let max_score = s_ball.max(s_goal).max(s_space).max(s_support).max(s_hold);

        if (s_goal - max_score).abs() < 0.01 {
            OffensiveGoal::AttackGoal
        } else if (s_space - max_score).abs() < 0.01 {
            OffensiveGoal::FindSpace
        } else if (s_support - max_score).abs() < 0.01 {
            OffensiveGoal::SupportTeammate
        } else if (s_ball - max_score).abs() < 0.01 {
            OffensiveGoal::MoveToBall
        } else {
            OffensiveGoal::HoldPosition
        }
    }

    /// 수비 상황에서 전술적 목표 결정
    ///
    /// # Arguments
    /// * `distance_to_ball` - 공까지의 거리 (m)
    ///
    /// # Returns
    /// 선수가 추구할 수비 목표
    pub fn decide_defensive_goal(&self, distance_to_ball: f32) -> DefensiveGoal {
        let attr = &self.attributes;
        let (w_press, w_track, w_mark, w_block, w_hold) = self.get_defensive_bias();
        let mods: DecisionModifiers = self.personality.decision_modifiers();

        // 거리 기반 압박 보정 (가까우면 압박 본능 발동)
        let dist_factor = if distance_to_ball < 10.0 {
            2.5
        } else if distance_to_ball < 20.0 {
            1.5
        } else {
            1.0
        };

        // 1. Press: 공격성, 용기, 활동량
        let s_press = (attr.aggression as f32 * 0.5
            + attr.bravery as f32 * 0.3
            + attr.work_rate as f32 * 0.2)
            * w_press
            * dist_factor
            * mods.press;

        // 2. TrackBack: 활동량, 팀워크, 속도
        let s_track =
            (attr.work_rate as f32 * 0.4 + attr.teamwork as f32 * 0.4 + attr.pace as f32 * 0.2)
                * w_track
                * mods.track_back;

        // 3. Mark: 마킹, 힘, 집중력
        let s_mark = (attr.marking as f32 * 0.5
            + attr.strength as f32 * 0.3
            + attr.concentration as f32 * 0.2)
            * w_mark
            * mods.mark;

        // 4. Block: 예측력, 위치선정 (지능 수비)
        let s_block =
            (attr.anticipation as f32 * 0.5 + attr.positioning as f32 * 0.5) * w_block * mods.block;

        // 5. Hold: 위치선정, 침착성, 집중력
        let s_hold = (attr.positioning as f32 * 0.4
            + attr.composure as f32 * 0.3
            + attr.concentration as f32 * 0.3)
            * w_hold
            * mods.hold_defense;

        // 최종 선택
        let max_score = s_press.max(s_track).max(s_mark).max(s_block).max(s_hold);

        if (s_press - max_score).abs() < 0.01 {
            DefensiveGoal::PressOpponent
        } else if (s_mark - max_score).abs() < 0.01 {
            DefensiveGoal::MarkPlayer
        } else if (s_block - max_score).abs() < 0.01 {
            DefensiveGoal::BlockPassingLane
        } else if (s_track - max_score).abs() < 0.01 {
            DefensiveGoal::TrackBack
        } else {
            DefensiveGoal::HoldPosition
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::player::PlayerAttributes;
    use crate::models::trait_system::TraitSlots;
    use crate::player::personality::PersonalityArchetype;

    #[test]
    fn test_striker_offensive_goal() {
        let mut striker = Player {
            name: "Test Striker".to_string(),
            position: Position::ST,
            overall: 80,
            condition: 3,
            attributes: Some(PlayerAttributes::default()),
            equipped_skills: Vec::new(),
            traits: TraitSlots::new(),
            personality: PersonalityArchetype::Steady,
        };

        // 공격적인 공격수
        if let Some(ref mut attr) = striker.attributes {
            attr.aggression = 90;
            attr.off_the_ball = 95;
            attr.finishing = 90;
        }

        let goal = striker.decide_offensive_goal((0.5, 0.5), false);
        // 공격수라면 AttackGoal이 가장 높은 가중치를 받음
        assert!(matches!(goal, OffensiveGoal::AttackGoal));
    }

    #[test]
    fn test_centerback_defensive_goal() {
        let mut cb = Player {
            name: "Test CB".to_string(),
            position: Position::CB,
            overall: 80,
            condition: 3,
            attributes: Some(PlayerAttributes::default()),
            equipped_skills: Vec::new(),
            traits: TraitSlots::new(),
            personality: PersonalityArchetype::Steady,
        };

        // 침착한 수비수
        if let Some(ref mut attr) = cb.attributes {
            attr.positioning = 95;
            attr.concentration = 90;
            attr.aggression = 50; // 낮은 공격성
        }

        let goal = cb.decide_defensive_goal(30.0); // 멀리 있음
                                                   // CB는 Hold나 Mark를 선택할 확률이 높음
        assert!(matches!(goal, DefensiveGoal::HoldPosition | DefensiveGoal::MarkPlayer));
    }

    #[test]
    fn test_pressing_forward() {
        let mut fw = Player {
            name: "Pressing FW".to_string(),
            position: Position::ST,
            overall: 80,
            condition: 3,
            attributes: Some(PlayerAttributes::default()),
            equipped_skills: Vec::new(),
            traits: TraitSlots::new(),
            personality: PersonalityArchetype::Steady,
        };

        // 전방 압박형 공격수
        if let Some(ref mut attr) = fw.attributes {
            attr.aggression = 95;
            attr.work_rate = 95;
            attr.bravery = 90;
        }

        let goal = fw.decide_defensive_goal(8.0); // 가까이 있음
                                                  // 공 가까이 있으면 Press 확률 높음
        assert!(matches!(goal, DefensiveGoal::PressOpponent));
    }

    // =========================================================================
    // FIX_2601/0109: PressingState Tests
    // =========================================================================

    #[test]
    fn test_pressing_state_max_duration() {
        let state = PressingState::new(100);

        // Within limit - should continue
        assert!(state.should_continue(150, 10.0, &[]));

        // At limit - should continue
        assert!(state.should_continue(220, 10.0, &[]));

        // Past limit - should stop
        assert!(!state.should_continue(221, 10.0, &[]));
    }

    #[test]
    fn test_pressing_state_better_teammates() {
        let state = PressingState::new(100);

        // No teammates closer - continue
        assert!(state.should_continue(150, 10.0, &[12.0, 15.0, 20.0]));

        // One teammate closer (7.9 < 10 * 0.8 = 8) - continue
        assert!(state.should_continue(150, 10.0, &[7.9, 15.0, 20.0]));

        // Two teammates closer - stop (let them press)
        assert!(!state.should_continue(150, 10.0, &[6.0, 7.5, 20.0]));
    }

    #[test]
    fn test_pressing_state_efficiency_check() {
        let mut state = PressingState::new(100);

        // Not due yet
        assert!(!state.needs_efficiency_check(110));

        // Due at interval
        assert!(state.needs_efficiency_check(130));

        // Update check
        state.update_check(130);
        assert!(!state.needs_efficiency_check(140));
        assert!(state.needs_efficiency_check(160));
    }
}
