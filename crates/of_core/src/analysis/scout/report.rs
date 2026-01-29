//! # Scout Report Generation
//!
//! Assembles scouted data into presentable reports.
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: SCOUT_REPORT_SYSTEM.md

use super::model::{ScoutLevel, ScoutedValue, QualitativeGrade, ScoutedPlayerAttributes, PlayerAttributeSnapshot};
use super::style_tags::{StyleTag, TacticalRecommendation};

/// A complete scout report for a team.
#[derive(Debug, Clone)]
pub struct TeamScoutReport {
    /// Team identifier
    pub team_id: u32,
    /// Team name
    pub team_name: String,
    /// Scout level used to generate this report
    pub scout_level: ScoutLevel,
    /// When the report was generated
    pub generated_at: u64,
    /// Overall team rating
    pub overall_rating: ScoutedValue<u8>,
    /// Tactical style tags
    pub style_tags: Vec<StyleTag>,
    /// Formation preference
    pub formation: Option<String>,
    /// Strengths summary
    pub strengths: Vec<String>,
    /// Weaknesses summary
    pub weaknesses: Vec<String>,
    /// Key players to watch
    pub key_players: Vec<KeyPlayerInfo>,
    /// Tactical recommendations
    pub recommendations: Vec<TacticalRecommendation>,
    /// Detailed stats (availability depends on scout level)
    pub detailed_stats: Option<TeamDetailedStats>,
}

/// Key player information in scout report.
#[derive(Debug, Clone)]
pub struct KeyPlayerInfo {
    /// Player ID
    pub player_id: u32,
    /// Player name
    pub player_name: String,
    /// Position
    pub position: String,
    /// Overall rating with uncertainty
    pub rating: ScoutedValue<u8>,
    /// Key attribute values (depends on scout level)
    pub attributes: Vec<(String, ScoutedAttribute)>,
    /// Role description
    pub role: String,
    /// Threat level (1-5)
    pub threat_level: u8,
}

/// A scouted attribute with display info.
#[derive(Debug, Clone)]
pub enum ScoutedAttribute {
    /// Exact value with uncertainty (L3+)
    Value(ScoutedValue<u8>),
    /// Qualitative grade (L1-L2)
    Grade(QualitativeGrade),
    /// Range bar (L2)
    Range { low: u8, high: u8 },
    /// Hidden (L0)
    Hidden,
}

/// Detailed team statistics (L2+ only).
#[derive(Debug, Clone, Default)]
pub struct TeamDetailedStats {
    /// Goals per game
    pub goals_per_game: ScoutedValue<f32>,
    /// Goals conceded per game
    pub conceded_per_game: ScoutedValue<f32>,
    /// Possession percentage
    pub possession_pct: ScoutedValue<f32>,
    /// Pass accuracy
    pub pass_accuracy: ScoutedValue<f32>,
    /// Shots per game
    pub shots_per_game: ScoutedValue<f32>,
    /// Shots on target per game
    pub shots_on_target_per_game: ScoutedValue<f32>,
    /// xG per game
    pub xg_per_game: ScoutedValue<f32>,
    /// xGA per game
    pub xga_per_game: ScoutedValue<f32>,
}

// ============================================================================
// Player Scout Report
// ============================================================================

/// A complete scout report for an individual player.
#[derive(Debug, Clone)]
pub struct PlayerScoutReport {
    /// Player ID
    pub player_id: u32,
    /// Player name
    pub player_name: String,
    /// Scout level used to generate this report
    pub scout_level: ScoutLevel,
    /// When the report was generated
    pub generated_at: u64,
    /// Overall rating with uncertainty
    pub overall_rating: ScoutedValue<u8>,
    /// Primary position
    pub position: String,
    /// Secondary positions
    pub secondary_positions: Vec<String>,
    /// Age
    pub age: Option<u8>,
    /// Nationality
    pub nationality: Option<String>,
    /// Scouted attributes (based on level)
    pub attributes: ScoutedPlayerAttributes,
    /// Visible attribute summaries
    pub attribute_summaries: Vec<AttributeSummary>,
    /// Player style tags
    pub style_tags: Vec<String>,
    /// Strengths
    pub strengths: Vec<String>,
    /// Weaknesses
    pub weaknesses: Vec<String>,
    /// Comparable players
    pub comparisons: Vec<String>,
    /// Transfer value estimate
    pub estimated_value: Option<ScoutedValue<f32>>,
}

/// Attribute summary for display.
#[derive(Debug, Clone)]
pub struct AttributeSummary {
    /// Attribute name
    pub name: String,
    /// Category (Physical, Technical, Mental, Hidden)
    pub category: String,
    /// Display value based on scout level
    pub display: ScoutedAttribute,
}

/// Build a player scout report from scouted attributes.
pub fn build_player_report(
    player_id: u32,
    player_name: &str,
    position: &str,
    actual: &PlayerAttributeSnapshot,
    scout_level: ScoutLevel,
    sample_count: u32,
) -> PlayerScoutReport {
    let scouted = ScoutedPlayerAttributes::from_actual(actual, scout_level, sample_count);
    let overall = scouted.overall();

    // Generate attribute summaries based on level
    let visible = scouted.visible_at_level(scout_level);
    let attribute_summaries: Vec<_> = visible
        .into_iter()
        .map(|(name, value)| {
            let display = match scout_level {
                ScoutLevel::Rumor => ScoutedAttribute::Hidden,
                ScoutLevel::Basic => ScoutedAttribute::Grade(
                    QualitativeGrade::from_attribute(value.estimate)
                ),
                ScoutLevel::Report => {
                    let (low, high) = value.range(1.0);
                    ScoutedAttribute::Range { low, high }
                }
                ScoutLevel::Detail | ScoutLevel::Elite => {
                    ScoutedAttribute::Value(value.clone())
                }
            };

            let category = match name {
                "Pace" | "Acceleration" | "Stamina" | "Strength" | "Jumping" => "Physical",
                "Passing" | "Shooting" | "Dribbling" | "First Touch" | "Crossing" | "Heading" | "Tackling" => "Technical",
                "Vision" | "Composure" | "Positioning" | "Decisions" | "Work Rate" => "Mental",
                _ => "Hidden",
            };

            AttributeSummary {
                name: name.to_string(),
                category: category.to_string(),
                display,
            }
        })
        .collect();

    // Detect strengths and weaknesses
    let (strengths, weaknesses) = analyze_player_attributes(actual);

    // Detect style tags
    let style_tags = detect_player_style(actual);

    PlayerScoutReport {
        player_id,
        player_name: player_name.to_string(),
        scout_level,
        generated_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        overall_rating: overall,
        position: position.to_string(),
        secondary_positions: Vec::new(),
        age: None,
        nationality: None,
        attributes: scouted,
        attribute_summaries,
        style_tags,
        strengths,
        weaknesses,
        comparisons: Vec::new(),
        estimated_value: None,
    }
}

/// Analyze player attributes to detect strengths and weaknesses.
fn analyze_player_attributes(attrs: &PlayerAttributeSnapshot) -> (Vec<String>, Vec<String>) {
    let mut strengths = Vec::new();
    let mut weaknesses = Vec::new();

    // Physical
    if attrs.pace >= 85 {
        strengths.push("Rapid pace".to_string());
    } else if attrs.pace <= 55 {
        weaknesses.push("Lacks pace".to_string());
    }

    if attrs.stamina >= 85 {
        strengths.push("Excellent stamina".to_string());
    } else if attrs.stamina <= 55 {
        weaknesses.push("Poor stamina".to_string());
    }

    if attrs.strength >= 85 {
        strengths.push("Physically strong".to_string());
    }

    // Technical
    if attrs.passing >= 85 {
        strengths.push("Elite passer".to_string());
    } else if attrs.passing <= 55 {
        weaknesses.push("Limited passing".to_string());
    }

    if attrs.shooting >= 85 {
        strengths.push("Clinical finisher".to_string());
    } else if attrs.shooting <= 55 {
        weaknesses.push("Poor shooting".to_string());
    }

    if attrs.dribbling >= 85 {
        strengths.push("Exceptional dribbler".to_string());
    }

    if attrs.tackling >= 85 {
        strengths.push("Strong tackler".to_string());
    }

    // Mental
    if attrs.vision >= 85 {
        strengths.push("Great vision".to_string());
    }

    if attrs.composure >= 85 {
        strengths.push("Cool under pressure".to_string());
    } else if attrs.composure <= 55 {
        weaknesses.push("Struggles under pressure".to_string());
    }

    // Hidden
    if attrs.consistency <= 55 {
        weaknesses.push("Inconsistent performer".to_string());
    }

    if attrs.big_games <= 55 {
        weaknesses.push("Struggles in big games".to_string());
    }

    if attrs.injury_prone >= 70 {
        weaknesses.push("Injury prone".to_string());
    }

    (strengths, weaknesses)
}

/// Detect player style tags based on attributes.
fn detect_player_style(attrs: &PlayerAttributeSnapshot) -> Vec<String> {
    let mut tags = Vec::new();

    // Pace-based styles
    if attrs.pace >= 80 && attrs.acceleration >= 80 {
        tags.push("Speedster".to_string());
    }

    // Technical styles
    if attrs.dribbling >= 80 && attrs.first_touch >= 80 {
        tags.push("Ball Wizard".to_string());
    }

    if attrs.passing >= 80 && attrs.vision >= 80 {
        tags.push("Playmaker".to_string());
    }

    if attrs.shooting >= 80 && attrs.composure >= 75 {
        tags.push("Clinical Finisher".to_string());
    }

    // Defensive styles
    if attrs.tackling >= 80 && attrs.positioning >= 75 {
        tags.push("Ball Winner".to_string());
    }

    if attrs.strength >= 80 && attrs.heading >= 75 {
        tags.push("Aerial Threat".to_string());
    }

    // Physical styles
    if attrs.stamina >= 85 && attrs.work_rate >= 80 {
        tags.push("Engine".to_string());
    }

    // Mental styles
    if attrs.decisions >= 80 && attrs.vision >= 80 {
        tags.push("Intelligent".to_string());
    }

    if attrs.big_games >= 80 && attrs.composure >= 80 {
        tags.push("Big Game Player".to_string());
    }

    tags
}

/// Generate a text summary of a player scout report.
pub fn generate_player_text_summary(report: &PlayerScoutReport) -> String {
    let mut lines = Vec::new();

    lines.push(format!("=== Player Scout Report: {} ===", report.player_name));
    lines.push(format!("Position: {}", report.position));
    lines.push(format!("Level: {:?}", report.scout_level));
    lines.push(format!(
        "Overall: {}",
        format_scouted_value(&report.overall_rating, report.scout_level)
    ));

    if !report.style_tags.is_empty() && report.scout_level >= ScoutLevel::Basic {
        lines.push("\nStyle:".to_string());
        for tag in &report.style_tags {
            lines.push(format!("  • {}", tag));
        }
    }

    if !report.attribute_summaries.is_empty() {
        lines.push("\nAttributes:".to_string());
        for attr in &report.attribute_summaries {
            let value_str = match &attr.display {
                ScoutedAttribute::Hidden => "???".to_string(),
                ScoutedAttribute::Grade(g) => g.display().to_string(),
                ScoutedAttribute::Range { low, high } => format!("{}-{}", low, high),
                ScoutedAttribute::Value(v) => format!("{}", v.estimate),
            };
            lines.push(format!("  {} ({}): {}", attr.name, attr.category, value_str));
        }
    }

    if !report.strengths.is_empty() && report.scout_level >= ScoutLevel::Basic {
        lines.push("\nStrengths:".to_string());
        for s in &report.strengths {
            lines.push(format!("  + {}", s));
        }
    }

    if !report.weaknesses.is_empty() && report.scout_level >= ScoutLevel::Report {
        lines.push("\nWeaknesses:".to_string());
        for w in &report.weaknesses {
            lines.push(format!("  - {}", w));
        }
    }

    lines.join("\n")
}

/// Build KeyPlayerInfo from scouted player attributes.
pub fn build_key_player_info(
    player_id: u32,
    player_name: &str,
    position: &str,
    attrs: &ScoutedPlayerAttributes,
    scout_level: ScoutLevel,
) -> KeyPlayerInfo {
    let overall = attrs.overall();

    // Get visible attributes
    let visible = attrs.visible_at_level(scout_level);
    let attributes: Vec<_> = visible
        .into_iter()
        .take(5) // Limit to 5 key attributes
        .map(|(name, value)| {
            let display = match scout_level {
                ScoutLevel::Rumor => ScoutedAttribute::Hidden,
                ScoutLevel::Basic => ScoutedAttribute::Grade(
                    QualitativeGrade::from_attribute(value.estimate)
                ),
                ScoutLevel::Report => {
                    let (low, high) = value.range(1.0);
                    ScoutedAttribute::Range { low, high }
                }
                ScoutLevel::Detail | ScoutLevel::Elite => {
                    ScoutedAttribute::Value(value.clone())
                }
            };
            (name.to_string(), display)
        })
        .collect();

    // Determine threat level based on overall
    let threat_level = match overall.estimate {
        0..=59 => 1,
        60..=69 => 2,
        70..=79 => 3,
        80..=89 => 4,
        _ => 5,
    };

    // Determine role from position and key attributes
    let role = determine_player_role(position, attrs);

    KeyPlayerInfo {
        player_id,
        player_name: player_name.to_string(),
        position: position.to_string(),
        rating: overall,
        attributes,
        role,
        threat_level,
    }
}

/// Determine a player's tactical role based on position and attributes.
fn determine_player_role(position: &str, attrs: &ScoutedPlayerAttributes) -> String {
    let pos = position.to_uppercase();

    if pos.contains("GK") {
        return "Goalkeeper".to_string();
    }

    if pos.contains("CB") || pos.contains("DC") {
        if attrs.passing.estimate >= 70 {
            return "Ball-Playing Defender".to_string();
        }
        return "Center Back".to_string();
    }

    if pos.contains("LB") || pos.contains("RB") || pos.contains("WB") {
        if attrs.pace.estimate >= 80 && attrs.crossing.estimate >= 75 {
            return "Attacking Wingback".to_string();
        }
        return "Full Back".to_string();
    }

    if pos.contains("DM") || pos.contains("CDM") {
        if attrs.tackling.estimate >= 80 {
            return "Defensive Anchor".to_string();
        }
        if attrs.passing.estimate >= 80 {
            return "Deep Playmaker".to_string();
        }
        return "Holding Midfielder".to_string();
    }

    if pos.contains("CM") {
        if attrs.shooting.estimate >= 75 && attrs.stamina.estimate >= 80 {
            return "Box-to-Box Midfielder".to_string();
        }
        if attrs.passing.estimate >= 80 && attrs.vision.estimate >= 78 {
            return "Playmaker".to_string();
        }
        return "Central Midfielder".to_string();
    }

    if pos.contains("AM") || pos.contains("CAM") {
        if attrs.passing.estimate >= 80 {
            return "Trequartista".to_string();
        }
        return "Attacking Midfielder".to_string();
    }

    if pos.contains("LW") || pos.contains("RW") || pos.contains("LM") || pos.contains("RM") {
        if attrs.pace.estimate >= 85 && attrs.dribbling.estimate >= 80 {
            return "Inverted Winger".to_string();
        }
        if attrs.crossing.estimate >= 80 {
            return "Traditional Winger".to_string();
        }
        return "Wide Midfielder".to_string();
    }

    if pos.contains("ST") || pos.contains("CF") {
        if attrs.pace.estimate >= 80 && attrs.dribbling.estimate >= 75 {
            return "Mobile Striker".to_string();
        }
        if attrs.strength.estimate >= 80 && attrs.heading.estimate >= 78 {
            return "Target Man".to_string();
        }
        if attrs.shooting.estimate >= 85 {
            return "Poacher".to_string();
        }
        return "Striker".to_string();
    }

    "Utility Player".to_string()
}

/// Report builder for assembling scout reports.
#[derive(Debug, Default)]
pub struct ScoutReportBuilder {
    team_id: Option<u32>,
    team_name: Option<String>,
    scout_level: ScoutLevel,
    style_tags: Vec<StyleTag>,
    key_players: Vec<KeyPlayerInfo>,
    recommendations: Vec<TacticalRecommendation>,
    detailed_stats: Option<TeamDetailedStats>,
}

impl ScoutReportBuilder {
    /// Create a new report builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the team being scouted.
    pub fn team(mut self, id: u32, name: &str) -> Self {
        self.team_id = Some(id);
        self.team_name = Some(name.to_string());
        self
    }

    /// Set the scout level.
    pub fn level(mut self, level: ScoutLevel) -> Self {
        self.scout_level = level;
        self
    }

    /// Add style tags.
    pub fn style_tags(mut self, tags: Vec<StyleTag>) -> Self {
        self.style_tags = tags;
        self
    }

    /// Add a key player.
    pub fn add_key_player(mut self, player: KeyPlayerInfo) -> Self {
        self.key_players.push(player);
        self
    }

    /// Add tactical recommendations.
    pub fn recommendations(mut self, recs: Vec<TacticalRecommendation>) -> Self {
        self.recommendations = recs;
        self
    }

    /// Add detailed stats (if scout level permits).
    pub fn detailed_stats(mut self, stats: TeamDetailedStats) -> Self {
        if self.scout_level >= ScoutLevel::Report {
            self.detailed_stats = Some(stats);
        }
        self
    }

    /// Build the final report.
    pub fn build(self) -> Option<TeamScoutReport> {
        let team_id = self.team_id?;
        let team_name = self.team_name?;

        // Generate strengths/weaknesses from tags
        let (strengths, weaknesses) = categorize_tags(&self.style_tags);

        // Limit data based on scout level
        let visible_tags: Vec<_> = self.style_tags
            .into_iter()
            .take(self.scout_level.style_tags_count())
            .collect();

        let visible_players: Vec<_> = self.key_players
            .into_iter()
            .take(match self.scout_level {
                ScoutLevel::Rumor => 1,
                ScoutLevel::Basic => 3,
                ScoutLevel::Report => 5,
                ScoutLevel::Detail | ScoutLevel::Elite => 11,
            })
            .collect();

        Some(TeamScoutReport {
            team_id,
            team_name,
            scout_level: self.scout_level,
            generated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            overall_rating: ScoutedValue::new(75, 8.0 * self.scout_level.uncertainty_mult()),
            style_tags: visible_tags,
            formation: None, // TODO: Detect from data
            strengths,
            weaknesses,
            key_players: visible_players,
            recommendations: self.recommendations,
            detailed_stats: self.detailed_stats,
        })
    }
}

/// Categorize style tags into strengths and weaknesses.
fn categorize_tags(tags: &[StyleTag]) -> (Vec<String>, Vec<String>) {
    let mut strengths = Vec::new();
    let mut weaknesses = Vec::new();

    for tag in tags {
        let desc = tag.display().to_string();
        if tag.is_weakness() {
            weaknesses.push(desc);
        } else {
            strengths.push(desc);
        }
    }

    (strengths, weaknesses)
}

/// Format a scouted value for display based on scout level.
pub fn format_scouted_value(value: &ScoutedValue<u8>, level: ScoutLevel) -> String {
    match level {
        ScoutLevel::Rumor => "???".to_string(),
        ScoutLevel::Basic => {
            let grade = QualitativeGrade::from_value(value.estimate as f32, 99.0);
            grade.display().to_string()
        }
        ScoutLevel::Report => {
            let (low, high) = value.range(1.0);
            format!("{}-{}", low, high)
        }
        ScoutLevel::Detail | ScoutLevel::Elite => {
            let (low, high) = value.range(0.5);
            format!("{} (±{})", value.estimate, (high - low) / 2)
        }
    }
}

/// Generate a text summary of the report.
pub fn generate_text_summary(report: &TeamScoutReport) -> String {
    let mut lines = Vec::new();

    lines.push(format!("=== Scout Report: {} ===", report.team_name));
    lines.push(format!("Level: {:?}", report.scout_level));
    lines.push(format!(
        "Overall: {}",
        format_scouted_value(&report.overall_rating, report.scout_level)
    ));

    if !report.style_tags.is_empty() {
        lines.push("\nStyle:".to_string());
        for tag in &report.style_tags {
            lines.push(format!("  • {}", tag.display()));
        }
    }

    if !report.strengths.is_empty() {
        lines.push("\nStrengths:".to_string());
        for s in &report.strengths {
            lines.push(format!("  + {}", s));
        }
    }

    if !report.weaknesses.is_empty() {
        lines.push("\nWeaknesses:".to_string());
        for w in &report.weaknesses {
            lines.push(format!("  - {}", w));
        }
    }

    if !report.key_players.is_empty() {
        lines.push("\nKey Players:".to_string());
        for player in &report.key_players {
            lines.push(format!(
                "  {} ({}) - {} [Threat: {}/5]",
                player.player_name,
                player.position,
                format_scouted_value(&player.rating, report.scout_level),
                player.threat_level
            ));
        }
    }

    if !report.recommendations.is_empty() {
        lines.push("\nTactical Recommendations:".to_string());
        for rec in &report.recommendations {
            lines.push(format!("  {}:", rec.title));
            lines.push(format!("    {}", rec.description));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_builder() {
        let report = ScoutReportBuilder::new()
            .team(1, "Test FC")
            .level(ScoutLevel::Report)
            .style_tags(vec![StyleTag::HighPress, StyleTag::HubDependent])
            .build();

        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report.team_name, "Test FC");
        assert_eq!(report.scout_level, ScoutLevel::Report);
        assert!(!report.weaknesses.is_empty()); // HubDependent is a weakness
    }

    #[test]
    fn test_format_scouted_value() {
        let value = ScoutedValue::new(75u8, 5.0);

        let rumor = format_scouted_value(&value, ScoutLevel::Rumor);
        assert_eq!(rumor, "???");

        let basic = format_scouted_value(&value, ScoutLevel::Basic);
        assert!(basic.len() > 0);

        let report = format_scouted_value(&value, ScoutLevel::Report);
        assert!(report.contains("-")); // Range format
    }

    #[test]
    fn test_text_summary() {
        let report = ScoutReportBuilder::new()
            .team(1, "Test FC")
            .level(ScoutLevel::Basic)
            .style_tags(vec![StyleTag::HighPress])
            .build()
            .unwrap();

        let summary = generate_text_summary(&report);
        assert!(summary.contains("Test FC"));
        assert!(summary.contains("High Press"));
    }

    #[test]
    fn test_build_player_report() {
        let attrs = PlayerAttributeSnapshot {
            pace: 88,
            acceleration: 85,
            stamina: 82,
            strength: 70,
            jumping: 72,
            passing: 78,
            shooting: 90,
            dribbling: 85,
            first_touch: 82,
            crossing: 75,
            heading: 65,
            tackling: 40,
            composure: 85,
            vision: 78,
            positioning: 82,
            decisions: 80,
            work_rate: 75,
            consistency: 80,
            big_games: 88,
            injury_prone: 25,
        };

        let report = build_player_report(
            10,
            "Test Player",
            "ST",
            &attrs,
            ScoutLevel::Detail,
            5,
        );

        assert_eq!(report.player_name, "Test Player");
        assert_eq!(report.position, "ST");
        assert!(report.overall_rating.estimate > 60);
        assert!(!report.strengths.is_empty());
        // High shooting should trigger "Clinical finisher"
        assert!(report.strengths.iter().any(|s| s.contains("finisher")));
        // High big_games + composure should trigger "Big Game Player"
        assert!(report.style_tags.iter().any(|t| t.contains("Big Game")));
    }

    #[test]
    fn test_player_text_summary() {
        let attrs = PlayerAttributeSnapshot {
            pace: 75,
            shooting: 80,
            passing: 85,
            vision: 82,
            composure: 78,
            ..Default::default()
        };

        let report = build_player_report(
            1,
            "Test Midfielder",
            "CM",
            &attrs,
            ScoutLevel::Report,
            3,
        );

        let summary = generate_player_text_summary(&report);
        assert!(summary.contains("Test Midfielder"));
        assert!(summary.contains("CM"));
        assert!(summary.contains("Attributes:"));
    }

    #[test]
    fn test_build_key_player_info() {
        // Create more complete attributes for a realistic overall
        let attrs = ScoutedPlayerAttributes {
            pace: ScoutedValue::new(88u8, 3.0),
            shooting: ScoutedValue::new(90u8, 3.0),
            dribbling: ScoutedValue::new(85u8, 3.0),
            passing: ScoutedValue::new(82u8, 3.0),
            tackling: ScoutedValue::new(50u8, 3.0),
            composure: ScoutedValue::new(85u8, 3.0),
            positioning: ScoutedValue::new(82u8, 3.0),
            stamina: ScoutedValue::new(80u8, 3.0),
            strength: ScoutedValue::new(75u8, 3.0),
            ..Default::default()
        };

        let key_info = build_key_player_info(
            10,
            "Star Player",
            "ST",
            &attrs,
            ScoutLevel::Detail,
        );

        assert_eq!(key_info.player_name, "Star Player");
        assert_eq!(key_info.position, "ST");
        // Should have a role assigned
        assert!(!key_info.role.is_empty());
        // Should have attributes
        assert!(!key_info.attributes.is_empty());
        // Threat level should be at least 1
        assert!(key_info.threat_level >= 1);
    }

    #[test]
    fn test_determine_player_role() {
        // Create a fast striker
        let striker_attrs = ScoutedPlayerAttributes {
            pace: ScoutedValue::new(85u8, 3.0),
            dribbling: ScoutedValue::new(80u8, 3.0),
            shooting: ScoutedValue::new(85u8, 3.0),
            ..Default::default()
        };

        let role = determine_player_role("ST", &striker_attrs);
        // Should be Mobile Striker due to high pace and dribbling
        assert!(role.contains("Mobile") || role.contains("Striker"));

        // Create a playmaker
        let playmaker_attrs = ScoutedPlayerAttributes {
            passing: ScoutedValue::new(88u8, 3.0),
            vision: ScoutedValue::new(85u8, 3.0),
            ..Default::default()
        };

        let role = determine_player_role("CM", &playmaker_attrs);
        assert!(role.contains("Playmaker"));
    }

    #[test]
    fn test_analyze_player_attributes() {
        let fast_player = PlayerAttributeSnapshot {
            pace: 92,
            stamina: 88,
            shooting: 85,
            injury_prone: 80,
            consistency: 50,
            ..Default::default()
        };

        let (strengths, weaknesses) = analyze_player_attributes(&fast_player);

        assert!(strengths.iter().any(|s| s.contains("pace")));
        assert!(strengths.iter().any(|s| s.contains("stamina")));
        assert!(strengths.iter().any(|s| s.contains("finisher")));
        assert!(weaknesses.iter().any(|w| w.contains("Injury")));
        assert!(weaknesses.iter().any(|w| w.contains("Inconsistent")));
    }
}
