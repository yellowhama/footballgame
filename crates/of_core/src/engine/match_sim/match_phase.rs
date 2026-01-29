//! Match phase scaffold for extra time / penalty shootout flow.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchPhase {
    FirstHalf,
    HalfTime,
    SecondHalf,
    ExtraTimeFirstHalf,
    ExtraTimeHalfTime,
    ExtraTimeSecondHalf,
    PenaltyShootout,
    Finished,
}

#[derive(Debug, Clone, Copy)]
pub struct PhaseConfig {
    pub allow_extra_time: bool,
    pub allow_penalty_shootout: bool,
    pub extra_time_minutes: u8,
    pub golden_goal: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PhaseSchedule {
    pub second_half_start: u8,
    pub regulation_end: u8,
    pub extra_time_second_half_start: Option<u8>,
    pub extra_time_end: Option<u8>,
}

impl Default for PhaseConfig {
    fn default() -> Self {
        Self {
            allow_extra_time: false,
            allow_penalty_shootout: false,
            extra_time_minutes: 15,
            golden_goal: false,
        }
    }
}

/// Advance to the next match phase after a phase ends.
///
/// `is_draw` should reflect the score at the end of the phase.
pub fn next_phase(current: MatchPhase, config: PhaseConfig, is_draw: bool) -> MatchPhase {
    match current {
        MatchPhase::FirstHalf => MatchPhase::HalfTime,
        MatchPhase::HalfTime => MatchPhase::SecondHalf,
        MatchPhase::SecondHalf => {
            if is_draw && config.allow_extra_time {
                MatchPhase::ExtraTimeFirstHalf
            } else {
                MatchPhase::Finished
            }
        }
        MatchPhase::ExtraTimeFirstHalf => MatchPhase::ExtraTimeHalfTime,
        MatchPhase::ExtraTimeHalfTime => MatchPhase::ExtraTimeSecondHalf,
        MatchPhase::ExtraTimeSecondHalf => {
            if is_draw && config.allow_penalty_shootout {
                MatchPhase::PenaltyShootout
            } else {
                MatchPhase::Finished
            }
        }
        MatchPhase::PenaltyShootout => MatchPhase::Finished,
        MatchPhase::Finished => MatchPhase::Finished,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regulation_flow_draw_to_extra_time() {
        let config = PhaseConfig {
            allow_extra_time: true,
            allow_penalty_shootout: true,
            ..Default::default()
        };

        assert_eq!(
            next_phase(MatchPhase::FirstHalf, config, true),
            MatchPhase::HalfTime
        );
        assert_eq!(
            next_phase(MatchPhase::HalfTime, config, true),
            MatchPhase::SecondHalf
        );
        assert_eq!(
            next_phase(MatchPhase::SecondHalf, config, true),
            MatchPhase::ExtraTimeFirstHalf
        );
    }

    #[test]
    fn test_regulation_flow_decided() {
        let config = PhaseConfig {
            allow_extra_time: true,
            ..Default::default()
        };
        assert_eq!(
            next_phase(MatchPhase::SecondHalf, config, false),
            MatchPhase::Finished
        );
    }

    #[test]
    fn test_extra_time_to_penalties() {
        let config = PhaseConfig {
            allow_extra_time: true,
            allow_penalty_shootout: true,
            ..Default::default()
        };
        assert_eq!(
            next_phase(MatchPhase::ExtraTimeSecondHalf, config, true),
            MatchPhase::PenaltyShootout
        );
        assert_eq!(
            next_phase(MatchPhase::PenaltyShootout, config, true),
            MatchPhase::Finished
        );
    }

    #[test]
    fn test_extra_time_without_penalties() {
        let config = PhaseConfig {
            allow_extra_time: true,
            allow_penalty_shootout: false,
            ..Default::default()
        };
        assert_eq!(
            next_phase(MatchPhase::ExtraTimeSecondHalf, config, true),
            MatchPhase::Finished
        );
    }
}
