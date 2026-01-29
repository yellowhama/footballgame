use serde::{Deserialize, Serialize};

use super::rules::{FoulDetails, FoulSeverity, OffsideDetails, RuleId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchEvent {
    pub minute: u8,
    /// Millisecond timestamp for position_data synchronization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_ms: Option<u64>,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub is_home_team: bool,
    // C7: Removed player: Option<String> - use player_track_id exclusively
    /// Event SSOT: track_id of primary actor (0..21). Prefer this over name-based mapping.
    /// C6: Changed from Option<usize> to Option<u8> for storage efficiency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_track_id: Option<u8>,
    /// Event SSOT: track_id of target (e.g., pass receiver) if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_track_id: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<EventDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(strum_macros::EnumIter))]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Match start / restart after goal (ENGINE_CONTRACT Section 1)
    KickOff,
    Goal,
    /// Own goal - scored into own team's net
    OwnGoal,
    Shot,
    ShotOnTarget,
    ShotOffTarget,
    ShotBlocked,
    Save,
    YellowCard,
    RedCard,
    Substitution,
    Injury,
    Corner,
    Freekick,
    Penalty,
    Offside,
    Foul,
    /// Handball - separate from Foul for better event taxonomy (FIX_2601/0123 Phase 6)
    Handball,
    KeyChance,
    Pass,
    Tackle,
    Dribble,
    /// Ball hits goalpost (ENGINE_CONTRACT Section 3.3)
    PostHit,
    /// Ball hits crossbar (ENGINE_CONTRACT Section 3.4)
    BarHit,
    /// Goal kick restart
    GoalKick,
    /// Throw-in restart
    ThrowIn,
    /// Half-time break
    HalfTime,
    /// Full-time whistle
    FullTime,
    /// VAR review (v0: informational, no overturn yet)
    VarReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EventDetails {
    // C7: Removed name-based fields (use track_id instead):
    // - assist_by: Use target_track_id on parent MatchEvent
    // - replaced_player: Use target_track_id on parent MatchEvent
    // - own_goal_by: Use player_track_id on parent MatchEvent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xg_value: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injury_severity: Option<InjurySeverity>,
    /// Ball position when event occurred (Coord10 units + height)
    /// FIX_2601/0113: 좌표계 통일
    /// (x, y, z) where:
    ///   x: 0-1050 (field length in 0.1m units, Coord10)
    ///   y: 0-680 (field width in 0.1m units, Coord10)
    ///   z: height in meters (0.0 = ground)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ball_position: Option<(f32, f32, f32)>,
    /// FIX_2601/1128: 패스의 의도된 타겟 위치 (forward_pass_rate 계산용)
    /// (x, y) where x, y are in Coord10 units (0.1m)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intended_target_pos: Option<(f32, f32)>,
    /// FIX_2601/1129: 패스 결정 시점의 패서 위치 (forward_pass_rate 계산용)
    /// (x, y) where x, y are in Coord10 units (0.1m)
    /// Used with intended_target_pos to calculate relative direction at decision time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intended_passer_pos: Option<(f32, f32)>,
    /// FIX_2601/0123: 패스 결정 시점 기준 전진 패스 여부
    /// QA metrics에서 하프타임 진영 변경과 무관하게 정확한 forward_pass_rate 계산에 사용
    /// true = 공격 방향으로 7m+ 전진하는 패스
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_forward_pass: Option<bool>,
    /// Substitution metadata (names + bench slot).
    ///
    /// Note: Substitution events are relatively rare, so including names here
    /// avoids fragile post-hoc name resolution when pitch-slot assignments change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub substitution: Option<SubstitutionDetails>,
    /// VAR review metadata (reviewed event type, outcome).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var_review: Option<VarReviewDetails>,
    /// Whether advantage was played on a foul (play continues without restart).
    ///
    /// This is set only when advantage is played (`true`). When absent, treat
    /// as "no advantage" (default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advantage_played: Option<bool>,

    // =========================================================================
    // RuleBook System (IFAB Laws of the Game)
    // =========================================================================

    /// IFAB 규칙 ID (어떤 규칙이 적용되었는지)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<RuleId>,

    /// 오프사이드 상세 정보 (Law 11)
    /// 오프사이드 판정의 근거를 설명하기 위한 데이터.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offside_details: Option<OffsideDetails>,

    /// 파울 상세 정보 (Law 12)
    /// 파울 심각도, 유형, DOGSO 여부 등.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foul_details: Option<FoulDetails>,

    /// FIX_2601/0123 Phase 6: 핸드볼 상세 정보
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handball_details: Option<HandballDetails>,
}

/// Handball event details (FIX_2601/0123 Phase 6)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandballDetails {
    /// Whether the handball was deliberate (affects severity)
    pub deliberate: bool,
    /// Type of restart awarded
    pub restart_type: HandballRestartType,
    /// Whether handball occurred in penalty area
    pub in_penalty_area: bool,
}

/// Restart type after handball
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandballRestartType {
    /// Direct free kick
    DirectFreeKick,
    /// Penalty kick (handball in penalty area, or DOGSO)
    Penalty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubstitutionDetails {
    pub player_in_name: String,
    pub player_out_name: String,
    /// Bench slot within `TeamSetup.substitutes` (0..6)
    pub bench_slot: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InjurySeverity {
    pub weeks_out: u8, // 1-4 weeks as per specification
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VarReviewOutcome {
    Upheld,
    Overturned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VarReviewDetails {
    pub reviewed_event_type: EventType,
    pub outcome: VarReviewOutcome,
}

impl MatchEvent {
    /// Create a kickoff event (match start or restart after goal)
    /// Per ENGINE_CONTRACT.md Section 1.2
    /// C5: timestamp_ms is now engine-confirmed
    pub fn kick_off(minute: u8, timestamp_ms: u64, is_home_team: bool) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::KickOff,
            is_home_team,
            player_track_id: None,
            target_track_id: None,
            details: Some(EventDetails {
                ball_position: Some((0.5, 0.5, 0.0)), // Center spot
                ..Default::default()
            }),
        }
    }

    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn goal(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        scorer_track_id: usize,
        assist_track_id: Option<usize>,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Goal,
            is_home_team,
            player_track_id: Some(scorer_track_id as u8),
            target_track_id: assist_track_id.map(|id| id as u8),
            details: None, // C7: assist_by will be removed
        }
    }

    /// Create an own goal event
    /// - `is_home_team`: The team that BENEFITS from the own goal (the scoring team)
    /// - `own_goal_track_id`: The player who scored against their own team (0-21)
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn own_goal(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        own_goal_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::OwnGoal,
            is_home_team,
            player_track_id: Some(own_goal_track_id as u8),
            target_track_id: None,
            details: None, // C7: own_goal_by will be removed
        }
    }

    /// Create an own goal event with ball position
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn own_goal_with_position(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        own_goal_track_id: usize,
        ball_position: (f32, f32, f32),
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::OwnGoal,
            is_home_team,
            player_track_id: Some(own_goal_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails {
                ball_position: Some(ball_position),
                ..Default::default()
            }),
        }
    }

    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn shot(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
        on_target: bool,
        xg: f32,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: if on_target { EventType::ShotOnTarget } else { EventType::ShotOffTarget },
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails { xg_value: Some(xg), ..Default::default() }),
        }
    }

    /// Create a save event (goalkeeper save)
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn save(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        goalkeeper_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Save,
            is_home_team,
            player_track_id: Some(goalkeeper_track_id as u8),
            target_track_id: None,
            details: None,
        }
    }

    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn yellow_card(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::YellowCard,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: None,
        }
    }

    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn red_card(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::RedCard,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: None,
        }
    }

    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn substitution(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_in_track_id: usize,
        player_out_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Substitution,
            is_home_team,
            player_track_id: Some(player_in_track_id as u8),
            target_track_id: Some(player_out_track_id as u8),
            details: None, // C7: replaced_player will be removed
        }
    }

    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn injury(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
        weeks_out: u8,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Injury,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails {
                injury_severity: Some(InjurySeverity {
                    weeks_out,
                    description: match weeks_out {
                        1 => "Minor injury".to_string(),
                        2 => "Moderate injury".to_string(),
                        3 => "Serious injury".to_string(),
                        _ => "Severe injury".to_string(),
                    },
                }),
                ..Default::default()
            }),
        }
    }

    /// Create a goal event with ball position (3D)
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn goal_with_position(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        scorer_track_id: usize,
        assist_track_id: Option<usize>,
        ball_position: (f32, f32, f32),
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Goal,
            is_home_team,
            player_track_id: Some(scorer_track_id as u8),
            target_track_id: assist_track_id.map(|id| id as u8),
            details: Some(EventDetails {
                ball_position: Some(ball_position),
                ..Default::default()
            }),
        }
    }

    /// Create a shot event with ball position (3D)
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn shot_with_position(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
        on_target: bool,
        xg: f32,
        ball_position: (f32, f32, f32),
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: if on_target { EventType::ShotOnTarget } else { EventType::ShotOffTarget },
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails {
                xg_value: Some(xg),
                ball_position: Some(ball_position),
                ..Default::default()
            }),
        }
    }

    /// Add ball position (3D with height) to existing event details
    pub fn with_ball_position(mut self, position: (f32, f32, f32)) -> Self {
        if let Some(ref mut details) = self.details {
            details.ball_position = Some(position);
        } else {
            self.details =
                Some(EventDetails { ball_position: Some(position), ..Default::default() });
        }
        self
    }

    /// Set timestamp_ms for position_data synchronization
    pub fn with_timestamp(mut self, timestamp_ms: u64) -> Self {
        self.timestamp_ms = Some(timestamp_ms);
        self
    }

    /// Set Event SSOT target track_id (secondary actor).
    /// Examples: pass receiver, foul victim, tackle target, shot shooter on save events.
    pub fn with_target_track_id(mut self, target_track_id: Option<usize>) -> Self {
        self.target_track_id = target_track_id.map(|id| id as u8);
        self
    }

    /// FIX_2601/1128: Set intended target position for pass events
    /// Used by advanced_metrics for accurate forward_pass_rate calculation
    pub fn with_intended_target_pos(mut self, pos: Option<(f32, f32)>) -> Self {
        if let Some(intended_pos) = pos {
            if let Some(ref mut details) = self.details {
                details.intended_target_pos = Some(intended_pos);
            } else {
                self.details = Some(EventDetails {
                    intended_target_pos: Some(intended_pos),
                    ..Default::default()
                });
            }
        }
        self
    }

    /// FIX_2601/1129: Set intended passer position for pass events
    /// Used with intended_target_pos for accurate forward_pass_rate calculation
    pub fn with_intended_passer_pos(mut self, pos: Option<(f32, f32)>) -> Self {
        if let Some(passer_pos) = pos {
            if let Some(ref mut details) = self.details {
                details.intended_passer_pos = Some(passer_pos);
            } else {
                self.details = Some(EventDetails {
                    intended_passer_pos: Some(passer_pos),
                    ..Default::default()
                });
            }
        }
        self
    }

    /// FIX_2601/0123: Set is_forward_pass flag for pass events
    /// Computed at decision time using correct attacks_right direction.
    /// QA metrics can use this directly without needing to know halftime direction changes.
    pub fn with_is_forward_pass(mut self, is_forward: Option<bool>) -> Self {
        if let Some(forward) = is_forward {
            if let Some(ref mut details) = self.details {
                details.is_forward_pass = Some(forward);
            } else {
                self.details = Some(EventDetails {
                    is_forward_pass: Some(forward),
                    ..Default::default()
                });
            }
        }
        self
    }

    /// Mark this event as "advantage played" (play-on) when a foul occurs.
    ///
    /// vNext(v2.1): executor records the foul but does not stop play / restart.
    pub fn with_advantage_played(mut self, advantage_played: bool) -> Self {
        if !advantage_played {
            return self;
        }

        if let Some(ref mut details) = self.details {
            details.advantage_played = Some(true);
        } else {
            self.details =
                Some(EventDetails { advantage_played: Some(true), ..Default::default() });
        }

        self
    }

    /// Add offside details to the event (Law 11)
    ///
    /// Phase 2: Engine Integration - attach OffsideDetails with margin, line position,
    /// passer info, involvement type, restart context, and deflection context.
    pub fn with_offside_details(mut self, offside_details: OffsideDetails) -> Self {
        if let Some(ref mut details) = self.details {
            details.offside_details = Some(offside_details);
            details.rule_id = Some(RuleId::OffsidePosition);
        } else {
            self.details = Some(EventDetails {
                offside_details: Some(offside_details),
                rule_id: Some(RuleId::OffsidePosition),
                ..Default::default()
            });
        }
        self
    }

    /// Add foul details to the event (Law 12)
    ///
    /// Phase 3: Foul/Card Integration - attach FoulDetails with severity,
    /// foul type, DOGSO status, and penalty area flag.
    pub fn with_foul_details(mut self, foul_details: FoulDetails) -> Self {
        let rule_id = match foul_details.severity {
            FoulSeverity::Careless => RuleId::FoulCareless,
            FoulSeverity::Reckless => RuleId::FoulReckless,
            FoulSeverity::ExcessiveForce => RuleId::FoulExcessiveForce,
        };

        if let Some(ref mut details) = self.details {
            details.foul_details = Some(foul_details);
            details.rule_id = Some(rule_id);
        } else {
            self.details = Some(EventDetails {
                foul_details: Some(foul_details),
                rule_id: Some(rule_id),
                ..Default::default()
            });
        }
        self
    }

    /// Create a corner kick event
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn corner(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        taker_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Corner,
            is_home_team,
            player_track_id: Some(taker_track_id as u8),
            target_track_id: None,
            details: None,
        }
    }

    /// Create a goal kick event
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn goal_kick(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        taker_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::GoalKick,
            is_home_team,
            player_track_id: Some(taker_track_id as u8),
            target_track_id: None,
            details: None,
        }
    }

    /// Create a throw-in event
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn throw_in(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        taker_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::ThrowIn,
            is_home_team,
            player_track_id: Some(taker_track_id as u8),
            target_track_id: None,
            details: None,
        }
    }

    /// Create a free kick event
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn freekick(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        taker_track_id: usize,
        ball_position: (f32, f32, f32),
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Freekick,
            is_home_team,
            player_track_id: Some(taker_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails {
                ball_position: Some(ball_position),
                ..Default::default()
            }),
        }
    }

    /// Create a penalty kick event
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn penalty(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        taker_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Penalty,
            is_home_team,
            player_track_id: Some(taker_track_id as u8),
            target_track_id: None,
            details: None,
        }
    }

    /// Create a foul event
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn foul(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
        ball_position: (f32, f32, f32),
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Foul,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails {
                ball_position: Some(ball_position),
                ..Default::default()
            }),
        }
    }

    /// Create a handball event (FIX_2601/0123 Phase 6)
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn handball(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
        ball_position: (f32, f32, f32),
        handball_details: HandballDetails,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Handball,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails {
                ball_position: Some(ball_position),
                handball_details: Some(handball_details),
                ..Default::default()
            }),
        }
    }

    /// Create an offside event
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn offside(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Offside,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: None,
        }
    }

    /// Create a pass event
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn pass(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
        ball_position: (f32, f32, f32),
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Pass,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails {
                ball_position: Some(ball_position),
                ..Default::default()
            }),
        }
    }

    /// Create a tackle event
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn tackle(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
        ball_position: (f32, f32, f32),
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Tackle,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails {
                ball_position: Some(ball_position),
                ..Default::default()
            }),
        }
    }

    /// Create a dribble event
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn dribble(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
        ball_position: (f32, f32, f32),
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::Dribble,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: Some(EventDetails {
                ball_position: Some(ball_position),
                ..Default::default()
            }),
        }
    }

    /// P5.2: Create a post hit event (shot hits goalpost)
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn post_hit(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::PostHit,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: None,
        }
    }

    /// P5.2: Create a bar hit event (shot hits crossbar)
    /// C5: timestamp_ms is now engine-confirmed
    /// C6: track_id is now engine-confirmed (0-21)
    pub fn bar_hit(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: usize,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::BarHit,
            is_home_team,
            player_track_id: Some(player_track_id as u8),
            target_track_id: None,
            details: None,
        }
    }

    /// P5.6: Create a half-time event
    /// C5: timestamp_ms is now engine-confirmed
    pub fn half_time(minute: u8, timestamp_ms: u64) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::HalfTime,
            is_home_team: true, // Neutral event
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    /// Create a full-time event
    /// C5: timestamp_ms is now engine-confirmed
    pub fn full_time(minute: u8, timestamp_ms: u64) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::FullTime,
            is_home_team: true, // Neutral event
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    /// Create a VAR review event.
    /// v0: informational only (no overturn yet).
    /// C5: timestamp_ms is now engine-confirmed
    pub fn var_review(
        minute: u8,
        timestamp_ms: u64,
        is_home_team: bool,
        player_track_id: Option<u8>,
        reviewed_event_type: EventType,
        outcome: VarReviewOutcome,
    ) -> Self {
        Self {
            minute,
            timestamp_ms: Some(timestamp_ms),
            event_type: EventType::VarReview,
            is_home_team,
            player_track_id,
            target_track_id: None,
            details: Some(EventDetails {
                var_review: Some(VarReviewDetails {
                    reviewed_event_type,
                    outcome,
                }),
                ..Default::default()
            }),
        }
    }
}
