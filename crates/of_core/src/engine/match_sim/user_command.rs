//! Career Player Mode - User Command Structures
//!
//! This module defines the user command structures for Career Player Mode,
//! allowing Godot to send player control commands to the Rust engine.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::models::TeamSide;

/// User command from Godot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCommand {
    /// 명령 시퀀스 번호 (중복 방지)
    pub seq: u32,

    /// 유저가 컨트롤하는 선수 track_id
    pub controlled_track_id: usize,

    /// 명령 페이로드
    #[serde(flatten)]
    pub payload: UserCommandPayload,
}

/// Controller slot binding for multi-agent control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerSlot {
    pub controller_id: u32,
    pub team_side: TeamSide,
    pub player_slot: u8,
}

/// Multi-agent command envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentCommand {
    pub controller_id: u32,
    pub seq: u32,
    #[serde(flatten)]
    pub payload: UserCommandPayload,
}

/// Multi-agent command batch (optional)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentCommandBatch {
    pub tick: u64,
    pub commands: Vec<MultiAgentCommand>,
}

/// User command payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum UserCommandPayload {
    #[serde(rename = "on_ball_action")]
    OnBallAction {
        action: OnBallAction,
        #[serde(skip_serializing_if = "Option::is_none")]
        variant: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_track_id: Option<usize>,
    },
}

/// On-ball action types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnBallAction {
    Pass,
    Shoot,
    Carry,
    TakeOn,
    Hold,
}

/// User command queue (FIFO)
#[derive(Debug, Default)]
pub struct UserCommandQueue {
    queue: VecDeque<UserCommand>,
}

impl UserCommandQueue {
    /// Create a new empty command queue
    pub fn new() -> Self {
        Self { queue: VecDeque::new() }
    }

    /// Add a command to the queue
    pub fn enqueue(&mut self, cmd: UserCommand) {
        self.queue.push_back(cmd);
    }

    /// Remove and return the first command from the queue
    pub fn pop_front(&mut self) -> Option<UserCommand> {
        self.queue.pop_front()
    }

    /// Get the number of commands in the queue
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Clear all commands from the queue
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_command_queue_new() {
        let queue = UserCommandQueue::new();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_user_command_queue_enqueue_and_pop() {
        let mut queue = UserCommandQueue::new();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());

        let cmd = UserCommand {
            seq: 1,
            controlled_track_id: 9,
            payload: UserCommandPayload::OnBallAction {
                action: OnBallAction::Pass,
                variant: None,
                target_track_id: Some(10),
            },
        };

        queue.enqueue(cmd.clone());
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        let popped = queue.pop_front().unwrap();
        assert_eq!(popped.seq, 1);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_user_command_queue_multiple() {
        let mut queue = UserCommandQueue::new();

        for i in 1..=5 {
            queue.enqueue(UserCommand {
                seq: i,
                controlled_track_id: 9,
                payload: UserCommandPayload::OnBallAction {
                    action: OnBallAction::Hold,
                    variant: None,
                    target_track_id: None,
                },
            });
        }

        assert_eq!(queue.len(), 5);

        // FIFO order
        assert_eq!(queue.pop_front().unwrap().seq, 1);
        assert_eq!(queue.pop_front().unwrap().seq, 2);
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn test_user_command_queue_clear() {
        let mut queue = UserCommandQueue::new();

        queue.enqueue(UserCommand {
            seq: 1,
            controlled_track_id: 9,
            payload: UserCommandPayload::OnBallAction {
                action: OnBallAction::Shoot,
                variant: None,
                target_track_id: None,
            },
        });

        assert_eq!(queue.len(), 1);
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_user_command_serialization_pass() {
        let cmd = UserCommand {
            seq: 5,
            controlled_track_id: 9,
            payload: UserCommandPayload::OnBallAction {
                action: OnBallAction::Pass,
                variant: Some("through".to_string()),
                target_track_id: Some(10),
            },
        };

        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: UserCommand = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.seq, 5);
        assert_eq!(deserialized.controlled_track_id, 9);

        let UserCommandPayload::OnBallAction { action, variant, target_track_id } =
            deserialized.payload;
        assert_eq!(action, OnBallAction::Pass);
        assert_eq!(variant, Some("through".to_string()));
        assert_eq!(target_track_id, Some(10));
    }

    #[test]
    fn test_user_command_serialization_shoot() {
        let cmd = UserCommand {
            seq: 10,
            controlled_track_id: 11,
            payload: UserCommandPayload::OnBallAction {
                action: OnBallAction::Shoot,
                variant: Some("finesse".to_string()),
                target_track_id: None,
            },
        };

        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: UserCommand = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.seq, 10);
        assert_eq!(deserialized.controlled_track_id, 11);

        let UserCommandPayload::OnBallAction { action, variant, target_track_id } =
            deserialized.payload;
        assert_eq!(action, OnBallAction::Shoot);
        assert_eq!(variant, Some("finesse".to_string()));
        assert_eq!(target_track_id, None);
    }

    #[test]
    fn test_on_ball_action_variants() {
        let actions = vec![
            OnBallAction::Pass,
            OnBallAction::Shoot,
            OnBallAction::Carry,
            OnBallAction::TakeOn,
            OnBallAction::Hold,
        ];

        for action in actions {
            let cmd = UserCommand {
                seq: 1,
                controlled_track_id: 0,
                payload: UserCommandPayload::OnBallAction {
                    action: action.clone(),
                    variant: None,
                    target_track_id: None,
                },
            };

            let json = serde_json::to_string(&cmd).unwrap();
            let deserialized: UserCommand = serde_json::from_str(&json).unwrap();

            let UserCommandPayload::OnBallAction { action: deserialized_action, .. } =
                deserialized.payload;
            assert_eq!(deserialized_action, action);
        }
    }

    #[test]
    fn test_multi_agent_command_serialization() {
        let cmd = MultiAgentCommand {
            controller_id: 7,
            seq: 10,
            payload: UserCommandPayload::OnBallAction {
                action: OnBallAction::Pass,
                variant: Some("through".to_string()),
                target_track_id: Some(5),
            },
        };

        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: MultiAgentCommand = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.controller_id, 7);
        assert_eq!(deserialized.seq, 10);

        let UserCommandPayload::OnBallAction { action, variant, target_track_id } =
            deserialized.payload;
        assert_eq!(action, OnBallAction::Pass);
        assert_eq!(variant, Some("through".to_string()));
        assert_eq!(target_track_id, Some(5));
    }

    #[test]
    fn test_controller_slot_serialization() {
        let slot = ControllerSlot { controller_id: 2, team_side: TeamSide::Away, player_slot: 4 };

        let json = serde_json::to_string(&slot).unwrap();
        let deserialized: ControllerSlot = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.controller_id, 2);
        assert_eq!(deserialized.team_side, TeamSide::Away);
        assert_eq!(deserialized.player_slot, 4);
    }
}
