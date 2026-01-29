//! Match State Machine
//!
//! FIX_2601/0123/02_MATCH_STATE_MACHINE: Explicit game state machine pattern
//! inspired by basketball RE analysis (GameData.GFPEGOGEAII enum).
//!
//! This module provides:
//! - GameFlowState enum for explicit game states (named to avoid conflict with tactics::GameFlowState)
//! - StateTransition rules for state changes
//! - GameFlowMachine controller for managing transitions

use crate::engine::action_queue::RestartType;

/// Team identifier (home = true, away = false)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TeamId(pub bool);

impl TeamId {
    pub const HOME: Self = Self(true);
    pub const AWAY: Self = Self(false);

    pub fn is_home(self) -> bool {
        self.0
    }

    pub fn opponent(self) -> Self {
        Self(!self.0)
    }
}

/// Player identifier within match context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MatchPlayerId {
    pub team: TeamId,
    pub index: u8, // 0-10 within team
}

impl MatchPlayerId {
    pub fn new(team: TeamId, index: u8) -> Self {
        Self { team, index }
    }

    /// Convert to global player index (0-21)
    pub fn to_global_index(self) -> usize {
        if self.team.is_home() {
            self.index as usize
        } else {
            11 + self.index as usize
        }
    }
}

/// 2D position on the pitch (normalized 0.0-1.0)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MatchPosition {
    pub x: f32,
    pub y: f32,
}

impl MatchPosition {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn from_tuple(pos: (f32, f32)) -> Self {
        Self { x: pos.0, y: pos.1 }
    }
}

/// Match time representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MatchTime {
    pub minute: u8,
    pub second: u8,
    pub stoppage: u8,
}

impl MatchTime {
    pub fn new(minute: u8, second: u8) -> Self {
        Self {
            minute,
            second,
            stoppage: 0,
        }
    }

    pub fn from_tick(tick: u64) -> Self {
        // 240 ticks per minute
        let total_seconds = tick / 4;
        let minute = (total_seconds / 60) as u8;
        let second = (total_seconds % 60) as u8;
        Self {
            minute,
            second,
            stoppage: 0,
        }
    }
}

/// Free kick type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FreeKickType {
    Direct,
    Indirect,
}

/// Corner kick side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CornerSide {
    Left,
    Right,
}

/// Penalty kick phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PenaltyPhase {
    /// Players positioning
    Setup,
    /// Kicker ready, waiting for whistle
    KickerReady,
    /// Whistle blown, can kick
    ReadyToKick,
    /// Kick in progress
    Kicking,
    /// Awaiting result (save/goal/miss)
    AwaitingResult,
}

/// VAR review type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarReviewType {
    Goal,
    Penalty,
    RedCard,
    MistakenIdentity,
}

/// Match state enum - explicit game states
/// Based on basketball GameData.GFPEGOGEAII pattern
#[derive(Debug, Clone, PartialEq)]
pub enum GameFlowState {
    // ========== Pre-Match ==========
    /// Pre-match (team entrance, anthem, etc.)
    PreMatch,

    /// Kickoff ready (players positioned at center circle)
    KickoffReady { restart_team: TeamId },

    // ========== In-Play ==========
    /// Ball in play (normal gameplay)
    InPlay,

    // ========== Dead Ball / Restarts ==========
    /// Generic dead ball (before specific restart setup)
    DeadBall {
        restart_type: RestartType,
        restart_team: TeamId,
        position: MatchPosition,
    },

    /// Free kick setup (wall positioning, etc.)
    FreeKickSetup {
        kick_type: FreeKickType,
        restart_team: TeamId,
        position: MatchPosition,
    },

    /// Corner kick setup
    CornerSetup {
        corner_side: CornerSide,
        restart_team: TeamId,
    },

    /// Throw-in setup
    ThrowInSetup {
        position: MatchPosition,
        restart_team: TeamId,
    },

    /// Goal kick setup
    GoalKickSetup { restart_team: TeamId },

    // ========== Special Situations ==========
    /// Penalty kick
    PenaltyKick {
        kicker: MatchPlayerId,
        phase: PenaltyPhase,
    },

    /// Goal celebration
    GoalCelebration {
        scorer: MatchPlayerId,
        goal_time: MatchTime,
    },

    /// VAR review
    VarReview { review_type: VarReviewType },

    // ========== Stoppages ==========
    /// Injury stoppage
    InjuryStoppage { injured_player: MatchPlayerId },

    /// Drink break (hydration)
    DrinkBreak,

    /// Cooling break (hot weather)
    CoolingBreak,

    // ========== Period Transitions ==========
    /// Half time
    HalfTime,

    /// Extra time break
    ExtraTimeBreak,

    /// Pre-penalty shootout break
    PrePenaltyShootout,

    /// Penalty shootout in progress
    PenaltyShootout {
        round: u8,
        home_score: u8,
        away_score: u8,
    },

    // ========== End ==========
    /// Full time (match ended)
    FullTime { final_score: (u8, u8) },
}

impl GameFlowState {
    /// Is the clock running in this state?
    pub fn is_clock_running(&self) -> bool {
        matches!(self, GameFlowState::InPlay)
    }

    /// Is this a dead ball state?
    pub fn is_dead_ball(&self) -> bool {
        matches!(
            self,
            GameFlowState::DeadBall { .. }
                | GameFlowState::FreeKickSetup { .. }
                | GameFlowState::CornerSetup { .. }
                | GameFlowState::ThrowInSetup { .. }
                | GameFlowState::GoalKickSetup { .. }
                | GameFlowState::PenaltyKick { .. }
                | GameFlowState::GoalCelebration { .. }
                | GameFlowState::VarReview { .. }
                | GameFlowState::InjuryStoppage { .. }
        )
    }

    /// Is a restart pending?
    pub fn is_restart_pending(&self) -> bool {
        matches!(
            self,
            GameFlowState::KickoffReady { .. }
                | GameFlowState::FreeKickSetup { .. }
                | GameFlowState::CornerSetup { .. }
                | GameFlowState::ThrowInSetup { .. }
                | GameFlowState::GoalKickSetup { .. }
        )
    }

    /// Is the match ended?
    pub fn is_match_ended(&self) -> bool {
        matches!(self, GameFlowState::FullTime { .. })
    }

    /// Can players make decisions in this state?
    pub fn allows_player_decisions(&self) -> bool {
        matches!(
            self,
            GameFlowState::InPlay | GameFlowState::PenaltyKick { .. }
        )
    }

    /// Get the restart type if applicable
    pub fn restart_type(&self) -> Option<RestartType> {
        match self {
            GameFlowState::KickoffReady { .. } => Some(RestartType::KickOff),
            GameFlowState::DeadBall { restart_type, .. } => Some(*restart_type),
            GameFlowState::FreeKickSetup { .. } => Some(RestartType::FreeKick),
            GameFlowState::CornerSetup { .. } => Some(RestartType::Corner),
            GameFlowState::ThrowInSetup { .. } => Some(RestartType::ThrowIn),
            GameFlowState::GoalKickSetup { .. } => Some(RestartType::GoalKick),
            GameFlowState::PenaltyKick { .. } => Some(RestartType::Penalty),
            _ => None,
        }
    }

    /// Get the team that will restart play
    pub fn restart_team(&self) -> Option<TeamId> {
        match self {
            GameFlowState::KickoffReady { restart_team }
            | GameFlowState::DeadBall { restart_team, .. }
            | GameFlowState::FreeKickSetup { restart_team, .. }
            | GameFlowState::CornerSetup { restart_team, .. }
            | GameFlowState::ThrowInSetup { restart_team, .. }
            | GameFlowState::GoalKickSetup { restart_team } => Some(*restart_team),
            GameFlowState::PenaltyKick { kicker, .. } => Some(kicker.team),
            _ => None,
        }
    }
}

/// Transition trigger - what caused the state change
#[derive(Debug, Clone)]
pub enum TransitionTrigger {
    /// Match started
    MatchStart,
    /// Kick executed (kickoff, free kick, etc.)
    KickExecuted,
    /// Ball played (any touch after restart)
    BallPlayed,
    /// Goal scored
    GoalScored { scorer: MatchPlayerId },
    /// Foul committed
    FoulCommitted {
        offender: MatchPlayerId,
        position: MatchPosition,
        is_penalty: bool,
        is_indirect: bool,
    },
    /// Ball out of play
    OutOfPlay {
        restart_type: RestartType,
        position: MatchPosition,
        last_touch_team: TeamId,
    },
    /// Time elapsed (for celebration timeout, etc.)
    TimeElapsed { ticks: u64 },
    /// Referee whistle
    RefereeWhistle,
    /// Player injury
    PlayerInjury { player: MatchPlayerId },
    /// Period end (45 min, 90 min, etc.)
    PeriodEnd { half: u8 },
    /// VAR check initiated
    VarCheckInitiated { review_type: VarReviewType },
    /// VAR decision made
    VarDecision { upheld: bool },
}

/// Transition guard - conditions that must be met
#[derive(Debug, Clone)]
pub enum TransitionGuard {
    /// Minimum time elapsed since state entry
    MinTimeElapsed(u64),
    /// Players are in correct positions
    PlayersPositioned,
    /// Ball is in correct position
    BallInPosition,
    /// Referee is ready
    RefereeReady,
}

/// Transition action - side effects of state change
#[derive(Debug, Clone)]
pub enum TransitionAction {
    /// Reposition players for restart
    RepositionPlayers { restart_type: RestartType },
    /// Place ball at position
    PlaceBall { position: MatchPosition },
    /// Start or stop the clock
    SetClockRunning(bool),
    /// Add stoppage time
    AddStoppageTime(u64),
    /// Update score
    UpdateScore { team: TeamId, goals: u8 },
}

/// State transition definition
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: GameFlowState,
    pub to: GameFlowState,
    pub trigger: TransitionTrigger,
    pub guards: Vec<TransitionGuard>,
    pub actions: Vec<TransitionAction>,
}

/// State machine controller
pub struct GameFlowMachine {
    current_state: GameFlowState,
    state_enter_tick: u64,
    transition_history: Vec<(u64, GameFlowState, GameFlowState)>,
}

impl Default for GameFlowMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl GameFlowMachine {
    /// Create a new state machine starting in PreMatch
    pub fn new() -> Self {
        Self {
            current_state: GameFlowState::PreMatch,
            state_enter_tick: 0,
            transition_history: Vec::new(),
        }
    }

    /// Create a state machine starting in a specific state
    pub fn with_state(initial_state: GameFlowState) -> Self {
        Self {
            current_state: initial_state,
            state_enter_tick: 0,
            transition_history: Vec::new(),
        }
    }

    /// Get current state
    pub fn current(&self) -> &GameFlowState {
        &self.current_state
    }

    /// Get tick when current state was entered
    pub fn state_enter_tick(&self) -> u64 {
        self.state_enter_tick
    }

    /// Get ticks elapsed in current state
    pub fn ticks_in_state(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.state_enter_tick)
    }

    /// Get transition history
    pub fn history(&self) -> &[(u64, GameFlowState, GameFlowState)] {
        &self.transition_history
    }

    /// Try to perform a state transition
    pub fn try_transition(
        &mut self,
        trigger: TransitionTrigger,
        current_tick: u64,
    ) -> Option<&GameFlowState> {
        let new_state = self.compute_transition(&trigger, current_tick)?;

        // Record transition
        let old_state = std::mem::replace(&mut self.current_state, new_state);
        self.transition_history
            .push((current_tick, old_state, self.current_state.clone()));
        self.state_enter_tick = current_tick;

        Some(&self.current_state)
    }

    /// Force a state transition (bypass guards)
    pub fn force_transition(&mut self, new_state: GameFlowState, current_tick: u64) {
        let old_state = std::mem::replace(&mut self.current_state, new_state);
        self.transition_history
            .push((current_tick, old_state, self.current_state.clone()));
        self.state_enter_tick = current_tick;
    }

    /// Compute the new state for a trigger (if valid)
    fn compute_transition(
        &self,
        trigger: &TransitionTrigger,
        current_tick: u64,
    ) -> Option<GameFlowState> {
        match (&self.current_state, trigger) {
            // PreMatch -> KickoffReady
            (GameFlowState::PreMatch, TransitionTrigger::MatchStart) => {
                Some(GameFlowState::KickoffReady {
                    restart_team: TeamId::HOME,
                })
            }

            // KickoffReady -> InPlay
            (GameFlowState::KickoffReady { .. }, TransitionTrigger::KickExecuted) => {
                Some(GameFlowState::InPlay)
            }

            // InPlay -> Goal celebration
            (GameFlowState::InPlay, TransitionTrigger::GoalScored { scorer }) => {
                Some(GameFlowState::GoalCelebration {
                    scorer: *scorer,
                    goal_time: MatchTime::from_tick(current_tick),
                })
            }

            // InPlay -> FreeKickSetup (foul)
            (
                GameFlowState::InPlay,
                TransitionTrigger::FoulCommitted {
                    position,
                    is_penalty,
                    is_indirect,
                    offender,
                },
            ) => {
                let restart_team = offender.team.opponent();
                if *is_penalty {
                    Some(GameFlowState::PenaltyKick {
                        kicker: MatchPlayerId::new(restart_team, 0), // Will be set by caller
                        phase: PenaltyPhase::Setup,
                    })
                } else {
                    Some(GameFlowState::FreeKickSetup {
                        kick_type: if *is_indirect {
                            FreeKickType::Indirect
                        } else {
                            FreeKickType::Direct
                        },
                        restart_team,
                        position: *position,
                    })
                }
            }

            // InPlay -> Out of play situations
            (
                GameFlowState::InPlay,
                TransitionTrigger::OutOfPlay {
                    restart_type,
                    position,
                    last_touch_team,
                },
            ) => {
                let restart_team = last_touch_team.opponent();
                match restart_type {
                    RestartType::ThrowIn => Some(GameFlowState::ThrowInSetup {
                        position: *position,
                        restart_team,
                    }),
                    RestartType::Corner => {
                        let corner_side = if position.y < 0.5 {
                            CornerSide::Left
                        } else {
                            CornerSide::Right
                        };
                        Some(GameFlowState::CornerSetup {
                            corner_side,
                            restart_team,
                        })
                    }
                    RestartType::GoalKick => Some(GameFlowState::GoalKickSetup { restart_team }),
                    _ => Some(GameFlowState::DeadBall {
                        restart_type: *restart_type,
                        restart_team,
                        position: *position,
                    }),
                }
            }

            // InPlay -> Injury
            (GameFlowState::InPlay, TransitionTrigger::PlayerInjury { player }) => {
                Some(GameFlowState::InjuryStoppage {
                    injured_player: *player,
                })
            }

            // InPlay -> HalfTime/FullTime
            (GameFlowState::InPlay, TransitionTrigger::PeriodEnd { half }) => match half {
                1 => Some(GameFlowState::HalfTime),
                2 => Some(GameFlowState::FullTime {
                    final_score: (0, 0), // Will be set by caller
                }),
                _ => None,
            },

            // Restart setups -> InPlay
            (GameFlowState::FreeKickSetup { .. }, TransitionTrigger::KickExecuted)
            | (GameFlowState::CornerSetup { .. }, TransitionTrigger::KickExecuted)
            | (GameFlowState::ThrowInSetup { .. }, TransitionTrigger::BallPlayed)
            | (GameFlowState::GoalKickSetup { .. }, TransitionTrigger::KickExecuted)
            | (GameFlowState::DeadBall { .. }, TransitionTrigger::BallPlayed) => {
                Some(GameFlowState::InPlay)
            }

            // GoalCelebration -> KickoffReady (after timeout)
            (GameFlowState::GoalCelebration { scorer, .. }, TransitionTrigger::TimeElapsed { ticks })
                if *ticks >= 720 =>
            {
                // 3 seconds at 240 ticks/min = 720 ticks
                Some(GameFlowState::KickoffReady {
                    restart_team: scorer.team.opponent(),
                })
            }

            // PenaltyKick phase transitions
            (
                GameFlowState::PenaltyKick {
                    kicker,
                    phase: PenaltyPhase::Setup,
                },
                TransitionTrigger::TimeElapsed { .. },
            ) => Some(GameFlowState::PenaltyKick {
                kicker: *kicker,
                phase: PenaltyPhase::KickerReady,
            }),

            (
                GameFlowState::PenaltyKick {
                    kicker,
                    phase: PenaltyPhase::KickerReady,
                },
                TransitionTrigger::RefereeWhistle,
            ) => Some(GameFlowState::PenaltyKick {
                kicker: *kicker,
                phase: PenaltyPhase::ReadyToKick,
            }),

            (
                GameFlowState::PenaltyKick {
                    kicker,
                    phase: PenaltyPhase::ReadyToKick,
                },
                TransitionTrigger::KickExecuted,
            ) => Some(GameFlowState::PenaltyKick {
                kicker: *kicker,
                phase: PenaltyPhase::Kicking,
            }),

            (
                GameFlowState::PenaltyKick {
                    phase: PenaltyPhase::Kicking | PenaltyPhase::AwaitingResult,
                    ..
                },
                TransitionTrigger::GoalScored { scorer },
            ) => Some(GameFlowState::GoalCelebration {
                scorer: *scorer,
                goal_time: MatchTime::from_tick(current_tick),
            }),

            (
                GameFlowState::PenaltyKick {
                    phase: PenaltyPhase::Kicking | PenaltyPhase::AwaitingResult,
                    ..
                },
                TransitionTrigger::BallPlayed,
            ) => {
                // Penalty saved/missed, play continues
                Some(GameFlowState::InPlay)
            }

            // InjuryStoppage -> DeadBall (when ready to resume)
            (
                GameFlowState::InjuryStoppage { .. },
                TransitionTrigger::TimeElapsed { .. },
            ) => Some(GameFlowState::DeadBall {
                restart_type: RestartType::DropBall,
                restart_team: TeamId::HOME, // Will be determined by possession
                position: MatchPosition::new(0.5, 0.5),
            }),

            // HalfTime -> KickoffReady (2nd half)
            (GameFlowState::HalfTime, TransitionTrigger::TimeElapsed { ticks }) if *ticks >= 3600 => {
                // 15 minutes = 3600 ticks
                Some(GameFlowState::KickoffReady {
                    restart_team: TeamId::AWAY, // Away team kicks off 2nd half
                })
            }

            // VAR transitions
            (GameFlowState::InPlay, TransitionTrigger::VarCheckInitiated { review_type }) => {
                Some(GameFlowState::VarReview {
                    review_type: *review_type,
                })
            }

            (GameFlowState::VarReview { .. }, TransitionTrigger::VarDecision { upheld: _ }) => {
                // Return to InPlay (or appropriate state based on decision)
                Some(GameFlowState::InPlay)
            }

            _ => None,
        }
    }

    /// Get pending actions for current state
    pub fn pending_actions(&self) -> Vec<TransitionAction> {
        match &self.current_state {
            GameFlowState::KickoffReady { .. } => {
                vec![
                    TransitionAction::RepositionPlayers {
                        restart_type: RestartType::KickOff,
                    },
                    TransitionAction::PlaceBall {
                        position: MatchPosition::new(0.5, 0.5),
                    },
                    TransitionAction::SetClockRunning(false),
                ]
            }
            GameFlowState::FreeKickSetup { position, .. } => {
                vec![
                    TransitionAction::RepositionPlayers {
                        restart_type: RestartType::FreeKick,
                    },
                    TransitionAction::PlaceBall { position: *position },
                    TransitionAction::SetClockRunning(false),
                ]
            }
            GameFlowState::CornerSetup { corner_side, .. } => {
                let x = if *corner_side == CornerSide::Left {
                    0.0
                } else {
                    1.0
                };
                vec![
                    TransitionAction::RepositionPlayers {
                        restart_type: RestartType::Corner,
                    },
                    TransitionAction::PlaceBall {
                        position: MatchPosition::new(x, 0.0),
                    },
                    TransitionAction::SetClockRunning(false),
                ]
            }
            GameFlowState::ThrowInSetup { position, .. } => {
                vec![
                    TransitionAction::RepositionPlayers {
                        restart_type: RestartType::ThrowIn,
                    },
                    TransitionAction::PlaceBall { position: *position },
                    TransitionAction::SetClockRunning(false),
                ]
            }
            GameFlowState::GoalKickSetup { .. } => {
                vec![
                    TransitionAction::RepositionPlayers {
                        restart_type: RestartType::GoalKick,
                    },
                    TransitionAction::PlaceBall {
                        position: MatchPosition::new(0.5, 0.1), // Goal area
                    },
                    TransitionAction::SetClockRunning(false),
                ]
            }
            GameFlowState::InPlay => {
                vec![TransitionAction::SetClockRunning(true)]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let sm = GameFlowMachine::new();
        assert!(matches!(sm.current(), GameFlowState::PreMatch));
    }

    #[test]
    fn test_match_start_transition() {
        let mut sm = GameFlowMachine::new();
        sm.try_transition(TransitionTrigger::MatchStart, 0);
        assert!(matches!(sm.current(), GameFlowState::KickoffReady { .. }));
    }

    #[test]
    fn test_kickoff_to_inplay() {
        let mut sm = GameFlowMachine::with_state(GameFlowState::KickoffReady {
            restart_team: TeamId::HOME,
        });
        sm.try_transition(TransitionTrigger::KickExecuted, 100);
        assert!(matches!(sm.current(), GameFlowState::InPlay));
    }

    #[test]
    fn test_goal_celebration_flow() {
        let mut sm = GameFlowMachine::with_state(GameFlowState::InPlay);

        // Goal scored
        let scorer = MatchPlayerId::new(TeamId::HOME, 9);
        sm.try_transition(TransitionTrigger::GoalScored { scorer }, 1000);
        assert!(matches!(sm.current(), GameFlowState::GoalCelebration { .. }));

        // After celebration timeout -> kickoff for opponent
        sm.try_transition(TransitionTrigger::TimeElapsed { ticks: 720 }, 1720);
        if let GameFlowState::KickoffReady { restart_team } = sm.current() {
            assert_eq!(*restart_team, TeamId::AWAY);
        } else {
            panic!("Expected KickoffReady state");
        }
    }

    #[test]
    fn test_foul_to_freekick() {
        let mut sm = GameFlowMachine::with_state(GameFlowState::InPlay);

        let offender = MatchPlayerId::new(TeamId::AWAY, 5);
        sm.try_transition(
            TransitionTrigger::FoulCommitted {
                offender,
                position: MatchPosition::new(0.6, 0.4),
                is_penalty: false,
                is_indirect: false,
            },
            500,
        );

        if let GameFlowState::FreeKickSetup {
            kick_type,
            restart_team,
            ..
        } = sm.current()
        {
            assert_eq!(*kick_type, FreeKickType::Direct);
            assert_eq!(*restart_team, TeamId::HOME);
        } else {
            panic!("Expected FreeKickSetup state");
        }
    }

    #[test]
    fn test_penalty_phases() {
        let kicker = MatchPlayerId::new(TeamId::HOME, 10);
        let mut sm = GameFlowMachine::with_state(GameFlowState::PenaltyKick {
            kicker,
            phase: PenaltyPhase::Setup,
        });

        // Setup -> KickerReady
        sm.try_transition(TransitionTrigger::TimeElapsed { ticks: 100 }, 100);
        assert!(matches!(
            sm.current(),
            GameFlowState::PenaltyKick {
                phase: PenaltyPhase::KickerReady,
                ..
            }
        ));

        // KickerReady -> ReadyToKick
        sm.try_transition(TransitionTrigger::RefereeWhistle, 200);
        assert!(matches!(
            sm.current(),
            GameFlowState::PenaltyKick {
                phase: PenaltyPhase::ReadyToKick,
                ..
            }
        ));

        // ReadyToKick -> Kicking
        sm.try_transition(TransitionTrigger::KickExecuted, 250);
        assert!(matches!(
            sm.current(),
            GameFlowState::PenaltyKick {
                phase: PenaltyPhase::Kicking,
                ..
            }
        ));
    }

    #[test]
    fn test_out_of_play_transitions() {
        let mut sm = GameFlowMachine::with_state(GameFlowState::InPlay);

        // Throw-in
        sm.try_transition(
            TransitionTrigger::OutOfPlay {
                restart_type: RestartType::ThrowIn,
                position: MatchPosition::new(0.0, 0.3),
                last_touch_team: TeamId::HOME,
            },
            100,
        );
        if let GameFlowState::ThrowInSetup { restart_team, .. } = sm.current() {
            assert_eq!(*restart_team, TeamId::AWAY);
        } else {
            panic!("Expected ThrowInSetup");
        }

        // Throw-in executed -> InPlay
        sm.try_transition(TransitionTrigger::BallPlayed, 150);
        assert!(matches!(sm.current(), GameFlowState::InPlay));
    }

    #[test]
    fn test_state_metadata() {
        let state = GameFlowState::InPlay;
        assert!(state.is_clock_running());
        assert!(!state.is_dead_ball());
        assert!(state.allows_player_decisions());

        let state = GameFlowState::FreeKickSetup {
            kick_type: FreeKickType::Direct,
            restart_team: TeamId::HOME,
            position: MatchPosition::new(0.5, 0.5),
        };
        assert!(!state.is_clock_running());
        assert!(state.is_dead_ball());
        assert!(state.is_restart_pending());
        assert_eq!(state.restart_type(), Some(RestartType::FreeKick));
    }

    #[test]
    fn test_transition_history() {
        let mut sm = GameFlowMachine::new();

        sm.try_transition(TransitionTrigger::MatchStart, 0);
        sm.try_transition(TransitionTrigger::KickExecuted, 10);

        let history = sm.history();
        assert_eq!(history.len(), 2);
        assert!(matches!(history[0].1, GameFlowState::PreMatch));
        assert!(matches!(history[0].2, GameFlowState::KickoffReady { .. }));
    }
}
