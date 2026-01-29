//! Assist Candidate System
//!
//! FIX_2601/0102: Open-Football 개선사항 도입
//!
//! 패스 성공 시 어시스트 후보를 추적하고, 골 발생 시
//! 10초 윈도우 내 실제 패스한 선수에게 어시스트 부여.

use crate::models::TeamSide;

/// 어시스트 후보 추적
///
/// 패스가 성공할 때마다 업데이트되며, 골 발생 시
/// 윈도우 내에 있으면 어시스트로 인정.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssistCandidate {
    /// 패스한 선수 track_id
    pub passer_idx: usize,
    /// 패스 받은 선수 track_id
    pub receiver_idx: usize,
    /// 팀 사이드
    pub team_side: TeamSide,
    /// 패스 시점 (tick)
    pub tick: u32,
}

impl AssistCandidate {
    /// 어시스트 윈도우: 10초 = 200 ticks (20Hz, 50ms/tick)
    pub const WINDOW_TICKS: u32 = 200;

    /// 새 어시스트 후보 생성
    pub fn new(passer_idx: usize, receiver_idx: usize, team_side: TeamSide, tick: u32) -> Self {
        Self { passer_idx, receiver_idx, team_side, tick }
    }

    /// 어시스트 유효성 검증
    ///
    /// - 10초 윈도우 내
    /// - 득점자가 패스 받은 선수와 동일
    /// - 자기 자신에게 어시스트 불가 (패스한 선수 != 득점자)
    pub fn is_valid(&self, current_tick: u32, scorer_idx: usize) -> bool {
        let within_window = current_tick.saturating_sub(self.tick) <= Self::WINDOW_TICKS;
        let is_receiver = self.receiver_idx == scorer_idx;
        let not_self_assist = self.passer_idx != scorer_idx;

        within_window && is_receiver && not_self_assist
    }

    /// 특정 팀의 어시스트인지 확인
    pub fn is_for_team(&self, team: TeamSide) -> bool {
        self.team_side == team
    }

    /// 어시스트 윈도우가 만료되었는지 확인
    pub fn is_expired(&self, current_tick: u32) -> bool {
        current_tick.saturating_sub(self.tick) > Self::WINDOW_TICKS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assist_candidate_valid() {
        let candidate = AssistCandidate::new(5, 9, TeamSide::Home, 1000);

        // 득점자가 receiver이고 윈도우 내 → 유효
        assert!(candidate.is_valid(1100, 9));

        // 득점자가 passer (자기 어시스트) → 무효
        assert!(!candidate.is_valid(1100, 5));

        // 득점자가 다른 선수 → 무효
        assert!(!candidate.is_valid(1100, 7));
    }

    #[test]
    fn test_assist_candidate_window_expired() {
        let candidate = AssistCandidate::new(5, 9, TeamSide::Home, 1000);

        // 200틱 내 → 유효
        assert!(candidate.is_valid(1200, 9));

        // 201틱 후 → 만료
        assert!(!candidate.is_valid(1201, 9));

        // 300틱 후 → 만료
        assert!(!candidate.is_valid(1300, 9));
    }

    #[test]
    fn test_assist_candidate_team() {
        let candidate = AssistCandidate::new(5, 9, TeamSide::Home, 1000);

        assert!(candidate.is_for_team(TeamSide::Home));
        assert!(!candidate.is_for_team(TeamSide::Away));
    }

    #[test]
    fn test_assist_candidate_expired() {
        let candidate = AssistCandidate::new(5, 9, TeamSide::Home, 1000);

        assert!(!candidate.is_expired(1000));
        assert!(!candidate.is_expired(1200));
        assert!(candidate.is_expired(1201));
    }
}
