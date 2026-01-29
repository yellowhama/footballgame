use crate::engine::position_behavior::PositionContext;
use crate::engine::position_substates::{GoalkeeperSubState, PositionSubState};

pub fn preempt_transition(
    ctx: &PositionContext,
    current_state: GoalkeeperSubState,
) -> Option<PositionSubState> {
    use GoalkeeperSubState::*;

    if ctx.shot_incoming
        && current_state != PreparingForSave
        && current_state != Diving
        && current_state != Catching
    {
        return Some(PositionSubState::Goalkeeper(PreparingForSave));
    }

    if ctx.player_has_ball && current_state != HoldingBall && current_state != Distributing {
        return Some(PositionSubState::Goalkeeper(HoldingBall));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preempts_shot_incoming() {
        let mut ctx = PositionContext::default();
        ctx.shot_incoming = true;
        let next = preempt_transition(&ctx, GoalkeeperSubState::Attentive);
        assert_eq!(
            next,
            Some(PositionSubState::Goalkeeper(GoalkeeperSubState::PreparingForSave))
        );
    }

    #[test]
    fn skips_preempt_when_already_saving() {
        let mut ctx = PositionContext::default();
        ctx.shot_incoming = true;
        let next = preempt_transition(&ctx, GoalkeeperSubState::Diving);
        assert_eq!(next, None);
    }

    #[test]
    fn preempts_holding_ball() {
        let mut ctx = PositionContext::default();
        ctx.player_has_ball = true;
        let next = preempt_transition(&ctx, GoalkeeperSubState::Attentive);
        assert_eq!(
            next,
            Some(PositionSubState::Goalkeeper(GoalkeeperSubState::HoldingBall))
        );
    }
}
