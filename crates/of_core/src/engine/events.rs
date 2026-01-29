use rand::Rng;
use rand_chacha::ChaCha8Rng;

use super::HighlightLevel;
use crate::models::{EventType, MatchEvent};

pub struct EventGenerator {
    // Configuration for event generation
    pub min_events: usize,
    pub max_events: usize,
    pub enhanced_for_user: bool,
    pub user_player_name: Option<String>,
    pub highlight_level: Option<HighlightLevel>,
}

impl EventGenerator {
    pub fn new() -> Self {
        Self {
            min_events: 20,
            max_events: 30,
            enhanced_for_user: false,
            user_player_name: None,
            highlight_level: None,
        }
    }

    pub fn with_user_config(mut self, player_name: &str, highlight_level: HighlightLevel) -> Self {
        self.enhanced_for_user = true;
        self.user_player_name = Some(player_name.to_string());
        self.highlight_level = Some(highlight_level);
        // Adjust event counts based on highlight level
        self.min_events = highlight_level.min_events();
        self.max_events = highlight_level.max_events();
        self
    }

    pub fn generate_key_chance(&self, minute: u8, is_home: bool, _player: String) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: None,
            event_type: EventType::KeyChance,
            is_home_team: is_home,
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    pub fn generate_corner(&self, minute: u8, is_home: bool) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: None,
            event_type: EventType::Corner,
            is_home_team: is_home,
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    pub fn generate_freekick(&self, minute: u8, is_home: bool, _player: String) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: None,
            event_type: EventType::Freekick,
            is_home_team: is_home,
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    pub fn generate_save(&self, minute: u8, is_home_keeper: bool, _shooter: String) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: None,
            event_type: EventType::Save,
            is_home_team: is_home_keeper,
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    pub fn generate_offside(&self, minute: u8, is_home: bool, _player: String) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: None,
            event_type: EventType::Offside,
            is_home_team: is_home,
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    pub fn generate_foul(&self, minute: u8, is_home: bool, _player: String) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: None,
            event_type: EventType::Foul,
            is_home_team: is_home,
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    pub fn should_generate_additional_events(&self, current_count: usize) -> bool {
        // Ensure we have between min_events and max_events
        current_count < self.min_events
    }

    pub fn generate_pass(&self, minute: u8, is_home: bool, _player: String) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: None,
            event_type: EventType::Pass,
            is_home_team: is_home,
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    pub fn generate_tackle(&self, minute: u8, is_home: bool, _player: String) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: None,
            event_type: EventType::Tackle,
            is_home_team: is_home,
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    pub fn generate_dribble(&self, minute: u8, is_home: bool, _player: String) -> MatchEvent {
        MatchEvent {
            minute,
            timestamp_ms: None,
            event_type: EventType::Dribble,
            is_home_team: is_home,
            player_track_id: None,
            target_track_id: None,
            details: None,
        }
    }

    pub fn fill_additional_events(&self, rng: &mut ChaCha8Rng, is_home: bool) -> Vec<MatchEvent> {
        let mut events = Vec::new();

        // Generate some corners (2-8 per team typical)
        let corners = rng.gen_range(1..=4);
        for _ in 0..corners {
            let minute = rng.gen_range(1..=90);
            events.push(self.generate_corner(minute, is_home));
        }

        // Generate some offsides (1-3 per team typical)
        let offsides = rng.gen_range(0..=2);
        for _ in 0..offsides {
            let minute = rng.gen_range(1..=90);
            events.push(self.generate_offside(
                minute,
                is_home,
                format!("Forward {}", rng.gen_range(1..=3)),
            ));
        }

        events
    }
}

impl Default for EventGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 공통 HighlightLevel 정책에 따라 이벤트를 필터링한다.
///
/// - `level`        : 현재 하이라이트 레벨
/// - `my_player_track_id` : 주인공 선수 track_id (0..21). 없으면 MyPlayer 규칙에서만 무시
pub fn filter_events_by_highlight_level(
    events: &[MatchEvent],
    level: HighlightLevel,
    my_player_track_id: Option<u8>,
) -> Vec<MatchEvent> {
    events.iter().filter(|e| level.allows(e, my_player_track_id)).cloned().collect()
}
