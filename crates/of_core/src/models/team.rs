use super::Player;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub name: String,
    pub formation: Formation,
    pub players: Vec<Player>, // 18 players (11 starting + 7 subs)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Formation {
    #[serde(rename = "4-4-2")]
    F442,
    #[serde(rename = "4-3-3")]
    F433,
    #[serde(rename = "4-4-1-1")]
    F4411,
    #[serde(rename = "4-3-2-1")]
    F4321,
    #[serde(rename = "4-2-2-2")]
    F4222,
    #[serde(rename = "4-5-1")]
    F451,
    #[serde(rename = "3-5-2")]
    F352,
    #[serde(rename = "3-4-2-1")]
    F3421,
    #[serde(rename = "3-4-1-2")]
    F3412,
    #[serde(rename = "5-3-2")]
    F532,
    #[serde(rename = "4-2-3-1")]
    F4231,
    #[serde(rename = "4-1-4-1")]
    F4141,
    #[serde(rename = "3-4-3")]
    F343,
    #[serde(rename = "5-4-1")]
    F541,
}

impl Formation {
    pub fn validate(&self) -> bool {
        // All formations should have 10 outfield players + 1 GK = 11
        match self {
            Formation::F442
            | Formation::F433
            | Formation::F4411
            | Formation::F4321
            | Formation::F4222
            | Formation::F451
            | Formation::F352
            | Formation::F3421
            | Formation::F3412
            | Formation::F532
            | Formation::F4231
            | Formation::F4141
            | Formation::F343
            | Formation::F541 => true,
        }
    }

    pub fn get_positions(&self) -> (u8, u8, u8) {
        // Returns (defenders, midfielders, forwards)
        match self {
            Formation::F442 => (4, 4, 2),
            Formation::F433 => (4, 3, 3),
            Formation::F4411 => (4, 4, 2),
            Formation::F4321 => (4, 3, 3), // 2 behind striker counted as forwards (CF/LW/RW)
            Formation::F4222 => (4, 4, 2),
            Formation::F451 => (4, 5, 1),
            Formation::F352 => (3, 5, 2),
            Formation::F3421 => (3, 4, 3),
            Formation::F3412 => (3, 5, 2), // CAM+2 ST
            Formation::F532 => (5, 3, 2),
            Formation::F4231 => (4, 5, 1), // 2 DM + 3 AM = 5 midfielders
            Formation::F4141 => (4, 5, 1), // 1 DM + 4 M = 5 midfielders
            Formation::F343 => (3, 4, 3),
            Formation::F541 => (5, 4, 1),
        }
    }

    /// Canonical formation code string (e.g., "4-3-3").
    pub fn code(&self) -> &'static str {
        match self {
            Formation::F442 => "4-4-2",
            Formation::F433 => "4-3-3",
            Formation::F4411 => "4-4-1-1",
            Formation::F4321 => "4-3-2-1",
            Formation::F4222 => "4-2-2-2",
            Formation::F451 => "4-5-1",
            Formation::F352 => "3-5-2",
            Formation::F3421 => "3-4-2-1",
            Formation::F3412 => "3-4-1-2",
            Formation::F532 => "5-3-2",
            Formation::F4231 => "4-2-3-1",
            Formation::F4141 => "4-1-4-1",
            Formation::F343 => "3-4-3",
            Formation::F541 => "5-4-1",
        }
    }
}

impl Team {
    pub fn validate(&self) -> Result<(), String> {
        // Must have exactly 18 players
        if self.players.len() != 18 {
            return Err(format!("Team must have exactly 18 players, found {}", self.players.len()));
        }

        // Must have at least 2 goalkeepers (1 starting + 1 sub)
        let gk_count = self.players.iter().filter(|p| p.position.is_goalkeeper()).count();
        if gk_count < 2 {
            return Err(format!("Team must have at least 2 goalkeepers, found {}", gk_count));
        }

        // Formation must be valid
        if !self.formation.validate() {
            return Err("Invalid formation".to_string());
        }

        // Check that we have enough players for each position
        let (def_needed, mid_needed, fwd_needed) = self.formation.get_positions();

        let defenders = self.players.iter().filter(|p| p.position.is_defender()).count() as u8;
        let midfielders = self.players.iter().filter(|p| p.position.is_midfielder()).count() as u8;
        let forwards = self.players.iter().filter(|p| p.position.is_forward()).count() as u8;

        if defenders < def_needed {
            return Err(format!(
                "Not enough defenders for {}: need {}, have {}",
                serde_json::to_string(&self.formation).unwrap_or_default(),
                def_needed,
                defenders
            ));
        }

        if midfielders < mid_needed {
            return Err(format!(
                "Not enough midfielders for formation: need {}, have {}",
                mid_needed, midfielders
            ));
        }

        if forwards < fwd_needed {
            return Err(format!(
                "Not enough forwards for formation: need {}, have {}",
                fwd_needed, forwards
            ));
        }

        Ok(())
    }

    pub fn get_starting_11(&self) -> &[Player] {
        &self.players[..11]
    }

    pub fn get_substitutes(&self) -> &[Player] {
        &self.players[11..]
    }

    pub fn average_overall(&self) -> f32 {
        let starting_11 = self.get_starting_11();
        let sum: u32 = starting_11.iter().map(|p| p.overall as u32).sum();
        sum as f32 / 11.0
    }
}
