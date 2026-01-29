//! Position-specific substates for football player behavior
//!
//! Based on Open Football UML analysis (2024-12-27)
//! Each position has unique behavior states that refine movement and decision-making.

/// Goalkeeper-specific substates (21 states - aligned with Open Football)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum GoalkeeperSubState {
    /// Default: Focused and ready
    #[default]
    Attentive,
    /// Adjusting position in front of goal
    Positioning,
    /// Preparing for incoming shot
    PreparingForSave,
    /// Jumping to save high shots
    Jumping,
    /// Diving to save
    Diving,
    /// Catching the ball
    Catching,
    /// Punching the ball away
    Punching,
    /// Rushing out to sweep (sweeper-keeper)
    Sweeping,
    /// Coming out for 1v1
    ComingOut,
    /// Deciding how to distribute
    Distributing,
    /// Throwing the ball out
    Throwing,
    /// Kicking (goal kick)
    Kicking,
    /// Picking up the ball
    PickingUpBall,
    /// Holding ball (6-second rule)
    HoldingBall,
    /// Saving penalty
    PenaltySave,
    /// Returning to goal position
    ReturningToGoal,
    /// Tackling when rushing out (sweeper-keeper behavior)
    Tackling,
    /// Under pressure - need quick distribution
    UnderPressure,
    /// Shooting (penalty kick taker scenario)
    Shooting,
    /// Running to position
    Running,
    /// Passing to teammate
    Passing,
}

/// Defender-specific substates (22 states - aligned with Open Football + P4.2 Resting + P4.4 Standing/Walking + P5.1 Jogging)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum DefenderSubState {
    /// Default: Maintaining defensive line
    #[default]
    HoldingLine,
    /// Covering space behind teammates
    Covering,
    /// Tight marking an opponent
    MarkingTight,
    /// Pressing the ball carrier
    Pressing,
    /// Sprinting back to defensive position
    TrackingBack,
    /// Clearing the ball from danger
    Clearing,
    /// Contesting aerial ball
    Heading,
    /// Attempting a standing tackle
    Tackling,
    /// Sliding tackle (risky but effective)
    SlidingTackle,
    /// Blocking a shot
    Blocking,
    /// Executing offside trap
    OffsideTrap,
    /// Pushing defensive line higher
    PushingUp,
    /// Intercepting a pass
    Intercepting,
    /// Dribbling with the ball
    Dribbling,
    /// Passing the ball
    Passing,
    /// Running without ball
    Running,
    /// Returning to position
    Returning,
    /// P4.2: Resting to recover stamina (low stamina state)
    Resting,
    /// P4.4: Standing still, observing play (when ball is far and no pressure)
    Standing,
    /// P4.4: Walking slowly (conserving energy when not urgent)
    Walking,
    /// P5.1: Jogging at medium pace (between walking and running)
    Jogging,
}

/// Midfielder-specific substates (22 states - aligned with Open Football + P4.2 Resting + P4.4 Standing/Walking + P5.1 Jogging)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum MidfielderSubState {
    /// Default: Distributing play
    #[default]
    Distributing,
    /// Switching play to opposite side
    SwitchingPlay,
    /// Creating space by movement
    CreatingSpace,
    /// Supporting attack as passing option
    AttackSupporting,
    /// Retaining possession safely
    HoldingPossession,
    /// Pressing opponent with ball
    Pressing,
    /// Tracking a runner into space
    TrackingRunner,
    /// Attempting a shot (mid-range)
    Shooting,
    /// Distance shooting from outside box
    DistanceShooting,
    /// Recycling possession backward
    Recycling,
    /// Intercepting a pass
    Intercepting,
    /// Dribbling with the ball
    Dribbling,
    /// Crossing from wide position
    Crossing,
    /// Tackling opponent
    Tackling,
    /// Running without ball
    Running,
    /// Returning to position
    Returning,
    /// Passing the ball (explicit pass state)
    Passing,
    /// P4.2: Resting to recover stamina (low stamina state)
    Resting,
    /// P4.4: Standing still, observing play (when ball is far and no pressure)
    Standing,
    /// P4.4: Walking slowly (conserving energy when not urgent)
    Walking,
    /// P5.1: Jogging at medium pace (between walking and running)
    Jogging,
}

/// Forward-specific substates (20 states - aligned with Open Football + P4.2 Resting + P4.4 Standing/Walking + P5.1 Jogging)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum ForwardSubState {
    /// Default: Running behind defensive line
    #[default]
    RunningInBehind,
    /// Holding up play with back to goal
    HoldingUpPlay,
    /// Finishing (attempting to score)
    Finishing,
    /// Shooting from distance
    Shooting,
    /// Positioning for cross
    ReceivingCross,
    /// Pressing from the front
    Pressing,
    /// Creating space by dragging defenders
    CreatingSpace,
    /// Breaking offside trap
    OffsideTrapBreaking,
    /// Setting up teammates (assist play)
    Assisting,
    /// Dribbling with the ball
    Dribbling,
    /// Heading the ball
    Heading,
    /// Intercepting passes
    Intercepting,
    /// Tackling opponent
    Tackling,
    /// Running without ball
    Running,
    /// Returning to position
    Returning,
    /// Passing the ball (explicit pass state)
    Passing,
    /// P4.2: Resting to recover stamina (low stamina state)
    Resting,
    /// P4.4: Standing still, observing play (when ball is far and no pressure)
    Standing,
    /// P4.4: Walking slowly (conserving energy when not urgent)
    Walking,
    /// P5.1: Jogging at medium pace (between walking and running)
    Jogging,
}

/// Unified substate enum for storage across all positions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionSubState {
    Goalkeeper(GoalkeeperSubState),
    Defender(DefenderSubState),
    Midfielder(MidfielderSubState),
    Forward(ForwardSubState),
}

impl Default for PositionSubState {
    fn default() -> Self {
        // Default to midfielder distributing (most common)
        PositionSubState::Midfielder(MidfielderSubState::default())
    }
}

impl PositionSubState {
    /// Create default substate for goalkeeper
    pub fn goalkeeper_default() -> Self {
        PositionSubState::Goalkeeper(GoalkeeperSubState::default())
    }

    /// Create default substate for defender
    pub fn defender_default() -> Self {
        PositionSubState::Defender(DefenderSubState::default())
    }

    /// Create default substate for midfielder
    pub fn midfielder_default() -> Self {
        PositionSubState::Midfielder(MidfielderSubState::default())
    }

    /// Create default substate for forward
    pub fn forward_default() -> Self {
        PositionSubState::Forward(ForwardSubState::default())
    }

    /// Check if this is a pressing state (any position)
    pub fn is_pressing(&self) -> bool {
        matches!(
            self,
            PositionSubState::Defender(DefenderSubState::Pressing)
                | PositionSubState::Midfielder(MidfielderSubState::Pressing)
                | PositionSubState::Forward(ForwardSubState::Pressing)
        )
    }

    /// Check if this state involves high-speed movement (sprinting)
    pub fn requires_sprint(&self) -> bool {
        matches!(
            self,
            PositionSubState::Goalkeeper(GoalkeeperSubState::Sweeping)
                | PositionSubState::Defender(DefenderSubState::TrackingBack)
                | PositionSubState::Forward(ForwardSubState::RunningInBehind)
                | PositionSubState::Forward(ForwardSubState::ReceivingCross)
        )
    }

    /// Get a short display name for debugging
    pub fn name(&self) -> &'static str {
        match self {
            PositionSubState::Goalkeeper(gs) => match gs {
                GoalkeeperSubState::Attentive => "GK:Attentive",
                GoalkeeperSubState::Positioning => "GK:Positioning",
                GoalkeeperSubState::PreparingForSave => "GK:PrepSave",
                GoalkeeperSubState::Jumping => "GK:Jumping",
                GoalkeeperSubState::Diving => "GK:Diving",
                GoalkeeperSubState::Catching => "GK:Catching",
                GoalkeeperSubState::Punching => "GK:Punching",
                GoalkeeperSubState::Sweeping => "GK:Sweeping",
                GoalkeeperSubState::ComingOut => "GK:ComingOut",
                GoalkeeperSubState::Distributing => "GK:Distributing",
                GoalkeeperSubState::Throwing => "GK:Throwing",
                GoalkeeperSubState::Kicking => "GK:Kicking",
                GoalkeeperSubState::PickingUpBall => "GK:PickUp",
                GoalkeeperSubState::HoldingBall => "GK:Holding",
                GoalkeeperSubState::PenaltySave => "GK:PenSave",
                GoalkeeperSubState::ReturningToGoal => "GK:Returning",
                GoalkeeperSubState::Tackling => "GK:Tackling",
                GoalkeeperSubState::UnderPressure => "GK:UnderPressure",
                GoalkeeperSubState::Shooting => "GK:Shooting",
                GoalkeeperSubState::Running => "GK:Running",
                GoalkeeperSubState::Passing => "GK:Passing",
            },
            PositionSubState::Defender(ds) => match ds {
                DefenderSubState::HoldingLine => "DEF:HoldLine",
                DefenderSubState::Covering => "DEF:Covering",
                DefenderSubState::MarkingTight => "DEF:Marking",
                DefenderSubState::Pressing => "DEF:Pressing",
                DefenderSubState::TrackingBack => "DEF:TrackBack",
                DefenderSubState::Clearing => "DEF:Clearing",
                DefenderSubState::Heading => "DEF:Heading",
                DefenderSubState::Tackling => "DEF:Tackling",
                DefenderSubState::SlidingTackle => "DEF:Slide",
                DefenderSubState::Blocking => "DEF:Blocking",
                DefenderSubState::OffsideTrap => "DEF:OffsideTrap",
                DefenderSubState::PushingUp => "DEF:PushUp",
                DefenderSubState::Intercepting => "DEF:Intercept",
                DefenderSubState::Dribbling => "DEF:Dribble",
                DefenderSubState::Passing => "DEF:Passing",
                DefenderSubState::Running => "DEF:Running",
                DefenderSubState::Returning => "DEF:Returning",
                DefenderSubState::Resting => "DEF:Resting",
                DefenderSubState::Standing => "DEF:Standing",
                DefenderSubState::Walking => "DEF:Walking",
                DefenderSubState::Jogging => "DEF:Jogging",
            },
            PositionSubState::Midfielder(ms) => match ms {
                MidfielderSubState::Distributing => "MID:Distribute",
                MidfielderSubState::SwitchingPlay => "MID:Switch",
                MidfielderSubState::CreatingSpace => "MID:CreateSpace",
                MidfielderSubState::AttackSupporting => "MID:Support",
                MidfielderSubState::HoldingPossession => "MID:HoldPoss",
                MidfielderSubState::Pressing => "MID:Pressing",
                MidfielderSubState::TrackingRunner => "MID:TrackRun",
                MidfielderSubState::Shooting => "MID:Shooting",
                MidfielderSubState::DistanceShooting => "MID:DistShot",
                MidfielderSubState::Recycling => "MID:Recycle",
                MidfielderSubState::Intercepting => "MID:Intercept",
                MidfielderSubState::Dribbling => "MID:Dribble",
                MidfielderSubState::Crossing => "MID:Crossing",
                MidfielderSubState::Tackling => "MID:Tackling",
                MidfielderSubState::Running => "MID:Running",
                MidfielderSubState::Returning => "MID:Returning",
                MidfielderSubState::Passing => "MID:Passing",
                MidfielderSubState::Resting => "MID:Resting",
                MidfielderSubState::Standing => "MID:Standing",
                MidfielderSubState::Walking => "MID:Walking",
                MidfielderSubState::Jogging => "MID:Jogging",
            },
            PositionSubState::Forward(fs) => match fs {
                ForwardSubState::RunningInBehind => "FWD:RunBehind",
                ForwardSubState::HoldingUpPlay => "FWD:HoldUp",
                ForwardSubState::Finishing => "FWD:Finishing",
                ForwardSubState::Shooting => "FWD:Shooting",
                ForwardSubState::ReceivingCross => "FWD:RecvCross",
                ForwardSubState::Pressing => "FWD:Pressing",
                ForwardSubState::CreatingSpace => "FWD:CreateSpace",
                ForwardSubState::OffsideTrapBreaking => "FWD:BreakTrap",
                ForwardSubState::Assisting => "FWD:Assisting",
                ForwardSubState::Dribbling => "FWD:Dribble",
                ForwardSubState::Heading => "FWD:Heading",
                ForwardSubState::Intercepting => "FWD:Intercept",
                ForwardSubState::Tackling => "FWD:Tackling",
                ForwardSubState::Running => "FWD:Running",
                ForwardSubState::Returning => "FWD:Returning",
                ForwardSubState::Passing => "FWD:Passing",
                ForwardSubState::Resting => "FWD:Resting",
                ForwardSubState::Standing => "FWD:Standing",
                ForwardSubState::Walking => "FWD:Walking",
                ForwardSubState::Jogging => "FWD:Jogging",
            },
        }
    }

    /// Check if this state is a dribbling state
    pub fn is_dribbling(&self) -> bool {
        matches!(
            self,
            PositionSubState::Defender(DefenderSubState::Dribbling)
                | PositionSubState::Midfielder(MidfielderSubState::Dribbling)
                | PositionSubState::Forward(ForwardSubState::Dribbling)
        )
    }

    /// Check if this state is a tackling state
    pub fn is_tackling(&self) -> bool {
        matches!(
            self,
            PositionSubState::Goalkeeper(GoalkeeperSubState::Tackling)
                | PositionSubState::Defender(DefenderSubState::Tackling)
                | PositionSubState::Defender(DefenderSubState::SlidingTackle)
                | PositionSubState::Midfielder(MidfielderSubState::Tackling)
                | PositionSubState::Forward(ForwardSubState::Tackling)
        )
    }

    /// P4.2: Check if this state is a resting state (stamina recovery)
    pub fn is_resting(&self) -> bool {
        matches!(
            self,
            PositionSubState::Defender(DefenderSubState::Resting)
                | PositionSubState::Midfielder(MidfielderSubState::Resting)
                | PositionSubState::Forward(ForwardSubState::Resting)
        )
    }

    /// P4.4: Check if this state is a standing state (observing play)
    pub fn is_standing(&self) -> bool {
        matches!(
            self,
            PositionSubState::Defender(DefenderSubState::Standing)
                | PositionSubState::Midfielder(MidfielderSubState::Standing)
                | PositionSubState::Forward(ForwardSubState::Standing)
        )
    }

    /// P4.4: Check if this state is a walking state (slow movement)
    pub fn is_walking(&self) -> bool {
        matches!(
            self,
            PositionSubState::Defender(DefenderSubState::Walking)
                | PositionSubState::Midfielder(MidfielderSubState::Walking)
                | PositionSubState::Forward(ForwardSubState::Walking)
        )
    }

    /// P5.1: Check if this state is a jogging state (medium pace)
    pub fn is_jogging(&self) -> bool {
        matches!(
            self,
            PositionSubState::Defender(DefenderSubState::Jogging)
                | PositionSubState::Midfielder(MidfielderSubState::Jogging)
                | PositionSubState::Forward(ForwardSubState::Jogging)
        )
    }

    /// P4.4: Check if this is a low-intensity state (standing, walking, jogging, or resting)
    pub fn is_low_intensity(&self) -> bool {
        self.is_standing() || self.is_walking() || self.is_jogging() || self.is_resting()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_substates() {
        assert_eq!(
            PositionSubState::goalkeeper_default(),
            PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive)
        );
        assert_eq!(
            PositionSubState::defender_default(),
            PositionSubState::Defender(DefenderSubState::HoldingLine)
        );
        assert_eq!(
            PositionSubState::midfielder_default(),
            PositionSubState::Midfielder(MidfielderSubState::Distributing)
        );
        assert_eq!(
            PositionSubState::forward_default(),
            PositionSubState::Forward(ForwardSubState::RunningInBehind)
        );
    }

    #[test]
    fn test_is_pressing() {
        assert!(PositionSubState::Defender(DefenderSubState::Pressing).is_pressing());
        assert!(PositionSubState::Midfielder(MidfielderSubState::Pressing).is_pressing());
        assert!(PositionSubState::Forward(ForwardSubState::Pressing).is_pressing());
        assert!(!PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive).is_pressing());
    }

    #[test]
    fn test_requires_sprint() {
        assert!(PositionSubState::Goalkeeper(GoalkeeperSubState::Sweeping).requires_sprint());
        assert!(PositionSubState::Defender(DefenderSubState::TrackingBack).requires_sprint());
        assert!(PositionSubState::Forward(ForwardSubState::RunningInBehind).requires_sprint());
        assert!(!PositionSubState::Midfielder(MidfielderSubState::Distributing).requires_sprint());
    }

    #[test]
    fn test_name() {
        assert_eq!(
            PositionSubState::Goalkeeper(GoalkeeperSubState::Diving).name(),
            "GK:Diving"
        );
        assert_eq!(
            PositionSubState::Defender(DefenderSubState::HoldingLine).name(),
            "DEF:HoldLine"
        );
    }

    // P4.4: Tests for Standing/Walking states

    #[test]
    fn test_is_standing() {
        assert!(PositionSubState::Defender(DefenderSubState::Standing).is_standing());
        assert!(PositionSubState::Midfielder(MidfielderSubState::Standing).is_standing());
        assert!(PositionSubState::Forward(ForwardSubState::Standing).is_standing());
        assert!(!PositionSubState::Defender(DefenderSubState::Running).is_standing());
        assert!(!PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive).is_standing());
    }

    #[test]
    fn test_is_walking() {
        assert!(PositionSubState::Defender(DefenderSubState::Walking).is_walking());
        assert!(PositionSubState::Midfielder(MidfielderSubState::Walking).is_walking());
        assert!(PositionSubState::Forward(ForwardSubState::Walking).is_walking());
        assert!(!PositionSubState::Defender(DefenderSubState::Running).is_walking());
        assert!(!PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive).is_walking());
    }

    #[test]
    fn test_is_jogging() {
        assert!(PositionSubState::Defender(DefenderSubState::Jogging).is_jogging());
        assert!(PositionSubState::Midfielder(MidfielderSubState::Jogging).is_jogging());
        assert!(PositionSubState::Forward(ForwardSubState::Jogging).is_jogging());
        assert!(!PositionSubState::Defender(DefenderSubState::Running).is_jogging());
        assert!(!PositionSubState::Goalkeeper(GoalkeeperSubState::Attentive).is_jogging());
    }

    #[test]
    fn test_is_low_intensity() {
        // Standing is low intensity
        assert!(PositionSubState::Defender(DefenderSubState::Standing).is_low_intensity());
        // Walking is low intensity
        assert!(PositionSubState::Midfielder(MidfielderSubState::Walking).is_low_intensity());
        // Jogging is low intensity
        assert!(PositionSubState::Forward(ForwardSubState::Jogging).is_low_intensity());
        // Resting is low intensity
        assert!(PositionSubState::Forward(ForwardSubState::Resting).is_low_intensity());
        // Running is not low intensity
        assert!(!PositionSubState::Defender(DefenderSubState::Running).is_low_intensity());
        // Pressing is not low intensity
        assert!(!PositionSubState::Forward(ForwardSubState::Pressing).is_low_intensity());
    }

    #[test]
    fn test_standing_walking_names() {
        assert_eq!(
            PositionSubState::Defender(DefenderSubState::Standing).name(),
            "DEF:Standing"
        );
        assert_eq!(
            PositionSubState::Midfielder(MidfielderSubState::Walking).name(),
            "MID:Walking"
        );
        assert_eq!(
            PositionSubState::Forward(ForwardSubState::Standing).name(),
            "FWD:Standing"
        );
    }
}
