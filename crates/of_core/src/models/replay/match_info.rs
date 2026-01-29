use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::types::{MeterPos, Team};

/// Match information and field specifications
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct MatchInfo {
    pub id: String,
    pub seed: i64,
    pub pitch: PitchSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<GoalSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub areas: Option<AreasSpec>,
    pub periods: Vec<Period>,
}

/// Pitch specifications (field dimensions)
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct PitchSpec {
    pub length_m: f64, // 90-120m, default 105
    pub width_m: f64,  // 45-90m, default 68
}

/// Goal specifications
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct GoalSpec {
    pub center_y_m: f64, // 20-48m, default 34.0
    pub width_m: f64,    // 6.0-7.5m, default 7.32
    pub depth_m: f64,    // 1.5-3.0m, default 2.44
}

/// Area specifications (penalty area, goal area, etc.)
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct AreasSpec {
    pub penalty_depth_m: f64,          // 10-20m, default 16.5
    pub penalty_width_m: f64,          // 35-45m, default 40.32
    pub six_depth_m: f64,              // 4.0-7.0m, default 5.5
    pub six_width_m: f64,              // 16.0-20.0m, default 18.32
    pub penalty_spot_from_line_m: f64, // 10.0-12.0m, default 11.0
    pub arc_radius_m: f64,             // 8.0-10.0m, default 9.15
}

/// Match period (half, extra time, etc.)
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct Period {
    pub i: u32,                   // Period number (1, 2, ...)
    pub start_t: f64,             // Start time in seconds
    pub end_t: f64,               // End time in seconds
    pub home_attacks_right: bool, // Field orientation
}

// Implementation methods
impl PitchSpec {
    /// Create standard FIFA pitch dimensions
    pub fn standard() -> Self {
        Self { length_m: 105.0, width_m: 68.0 }
    }

    /// Validate pitch dimensions are within FIFA regulations
    pub fn is_valid(&self) -> bool {
        self.length_m >= 90.0
            && self.length_m <= 120.0
            && self.width_m >= 45.0
            && self.width_m <= 90.0
    }

    /// Get center of the pitch
    pub fn center(&self) -> MeterPos {
        MeterPos { x: self.length_m / 2.0, y: self.width_m / 2.0 }
    }
}

impl GoalSpec {
    /// Create standard FIFA goal dimensions
    pub fn standard(pitch_width: f64) -> Self {
        Self { center_y_m: pitch_width / 2.0, width_m: 7.32, depth_m: 2.44 }
    }

    /// Validate goal dimensions are within FIFA regulations
    pub fn is_valid(&self) -> bool {
        self.width_m >= 6.0 && self.width_m <= 7.5 && self.depth_m >= 1.5 && self.depth_m <= 3.0
    }

    /// Get goal center position (on goal line)
    pub fn center(&self) -> MeterPos {
        MeterPos {
            x: 0.0, // On goal line (will be adjusted for away goal)
            y: self.center_y_m,
        }
    }

    /// Get goal posts positions
    pub fn posts(&self) -> (MeterPos, MeterPos) {
        let left_post = MeterPos { x: 0.0, y: self.center_y_m - self.width_m / 2.0 };
        let right_post = MeterPos { x: 0.0, y: self.center_y_m + self.width_m / 2.0 };
        (left_post, right_post)
    }
}

impl AreasSpec {
    /// Create standard FIFA area dimensions
    pub fn standard() -> Self {
        Self {
            penalty_depth_m: 16.5,
            penalty_width_m: 40.32,
            six_depth_m: 5.5,
            six_width_m: 18.32,
            penalty_spot_from_line_m: 11.0,
            arc_radius_m: 9.15,
        }
    }

    /// Validate area dimensions are within FIFA regulations
    pub fn is_valid(&self) -> bool {
        self.penalty_depth_m >= 10.0
            && self.penalty_depth_m <= 20.0
            && self.penalty_width_m >= 35.0
            && self.penalty_width_m <= 45.0
            && self.six_depth_m >= 4.0
            && self.six_depth_m <= 7.0
            && self.six_width_m >= 16.0
            && self.six_width_m <= 20.0
            && self.penalty_spot_from_line_m >= 10.0
            && self.penalty_spot_from_line_m <= 12.0
            && self.arc_radius_m >= 8.0
            && self.arc_radius_m <= 10.0
    }

    /// Get penalty spot position for given team
    pub fn penalty_spot(&self, team: Team, pitch_length: f64) -> MeterPos {
        match team {
            Team::Home => MeterPos {
                x: self.penalty_spot_from_line_m,
                y: pitch_length / 2.0, // Center of goal
            },
            Team::Away => {
                MeterPos { x: pitch_length - self.penalty_spot_from_line_m, y: pitch_length / 2.0 }
            }
        }
    }
}

impl Period {
    /// Create a standard 90-minute match period
    pub fn standard_match() -> Vec<Self> {
        vec![
            Self {
                i: 1,
                start_t: 0.0,
                end_t: 2700.0, // 45 minutes
                home_attacks_right: true,
            },
            Self {
                i: 2,
                start_t: 2700.0,
                end_t: 5400.0,             // 90 minutes total
                home_attacks_right: false, // Teams switch sides
            },
        ]
    }

    /// Get period duration in seconds
    pub fn duration(&self) -> f64 {
        self.end_t - self.start_t
    }

    /// Check if given time falls within this period
    pub fn contains_time(&self, t: f64) -> bool {
        t >= self.start_t && t <= self.end_t
    }
}

impl MatchInfo {
    /// Create new match info with minimal required data
    pub fn new(id: String, seed: i64) -> Self {
        Self {
            id,
            seed,
            pitch: PitchSpec::standard(),
            goal: Some(GoalSpec::standard(68.0)),
            areas: Some(AreasSpec::standard()),
            periods: Period::standard_match(),
        }
    }

    /// Validate all match info components
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Match ID cannot be empty".to_string());
        }

        if !self.pitch.is_valid() {
            return Err("Invalid pitch dimensions".to_string());
        }

        if let Some(ref goal) = self.goal {
            if !goal.is_valid() {
                return Err("Invalid goal dimensions".to_string());
            }
        }

        if let Some(ref areas) = self.areas {
            if !areas.is_valid() {
                return Err("Invalid area dimensions".to_string());
            }
        }

        if self.periods.is_empty() {
            return Err("At least one period is required".to_string());
        }

        for (i, period) in self.periods.iter().enumerate() {
            if period.start_t < 0.0 {
                return Err(format!("Period {} has negative start time", i + 1));
            }
            if period.end_t <= period.start_t {
                return Err(format!("Period {} end time must be after start time", i + 1));
            }
        }

        Ok(())
    }

    /// Get total match duration
    pub fn total_duration(&self) -> f64 {
        self.periods.iter().map(|p| p.duration()).sum()
    }

    /// Find which period contains the given time
    pub fn find_period(&self, t: f64) -> Option<&Period> {
        self.periods.iter().find(|period| period.contains_time(t))
    }
}

// Position validation helpers that depend on match info
impl MeterPos {
    /// Check if position is in penalty area for given team
    pub fn is_in_penalty_area(&self, areas: &AreasSpec, team: Team, pitch_length: f64) -> bool {
        let half_width = areas.penalty_width_m / 2.0;
        let center_y = pitch_length / 2.0;

        match team {
            Team::Home => {
                self.x <= areas.penalty_depth_m
                    && self.y >= (center_y - half_width)
                    && self.y <= (center_y + half_width)
            }
            Team::Away => {
                self.x >= (pitch_length - areas.penalty_depth_m)
                    && self.y >= (center_y - half_width)
                    && self.y <= (center_y + half_width)
            }
        }
    }

    /// Check if position is in goal area for given team
    pub fn is_in_goal_area(&self, areas: &AreasSpec, team: Team, pitch_length: f64) -> bool {
        let half_width = areas.six_width_m / 2.0;
        let center_y = pitch_length / 2.0;

        match team {
            Team::Home => {
                self.x <= areas.six_depth_m
                    && self.y >= (center_y - half_width)
                    && self.y <= (center_y + half_width)
            }
            Team::Away => {
                self.x >= (pitch_length - areas.six_depth_m)
                    && self.y >= (center_y - half_width)
                    && self.y <= (center_y + half_width)
            }
        }
    }

    /// Check if ball is in goal (considering goal specification)
    pub fn is_in_goal(&self, goal: &GoalSpec, pitch_length: f64) -> bool {
        let goal_y_min = goal.center_y_m - goal.width_m / 2.0;
        let goal_y_max = goal.center_y_m + goal.width_m / 2.0;

        // Home goal (x <= 0) or Away goal (x >= pitch_length)
        (self.x <= 0.0 || self.x >= pitch_length) && self.y >= goal_y_min && self.y <= goal_y_max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_spec_validation() {
        let standard_pitch = PitchSpec::standard();
        assert!(standard_pitch.is_valid());
        assert_eq!(standard_pitch.length_m, 105.0);
        assert_eq!(standard_pitch.width_m, 68.0);

        let invalid_pitch = PitchSpec { length_m: 200.0, width_m: 20.0 };
        assert!(!invalid_pitch.is_valid());
    }

    #[test]
    fn test_goal_spec() {
        let goal = GoalSpec::standard(68.0);
        assert!(goal.is_valid());
        assert_eq!(goal.center_y_m, 34.0);

        let (left_post, right_post) = goal.posts();
        assert_eq!(left_post.y, 34.0 - 7.32 / 2.0);
        assert_eq!(right_post.y, 34.0 + 7.32 / 2.0);
    }

    #[test]
    fn test_areas_spec() {
        let areas = AreasSpec::standard();
        assert!(areas.is_valid());

        let penalty_spot = areas.penalty_spot(Team::Home, 105.0);
        assert_eq!(penalty_spot.x, 11.0);
    }

    #[test]
    fn test_period_functionality() {
        let periods = Period::standard_match();
        assert_eq!(periods.len(), 2);
        assert_eq!(periods[0].duration(), 2700.0);
        assert!(periods[0].contains_time(1000.0));
        assert!(!periods[0].contains_time(3000.0));
    }

    #[test]
    fn test_match_info_validation() {
        let match_info = MatchInfo::new("test-123".to_string(), 42);
        assert!(match_info.validate().is_ok());
        assert_eq!(match_info.total_duration(), 5400.0);

        let period = match_info.find_period(1500.0);
        assert!(period.is_some());
        assert_eq!(period.unwrap().i, 1);
    }
}
