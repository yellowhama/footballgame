//! # Style Tags
//!
//! Team tactical style classification system.
//!
//! ## Reference
//! - FIX_2601/NEW_FUNC: SCOUT_REPORT_SYSTEM.md

use crate::analysis::metrics::gini::GiniMetrics;
use crate::analysis::metrics::shape::TeamShapeMetrics;
use crate::analysis::metrics::movement::OccupancyEntropy;
use crate::analysis::events::carry_extractor::TeamCarryStats;
use crate::analysis::events::sprint_extractor::TeamMovementMetrics;
use crate::analysis::events::run_extractor::TeamRunStats;

/// Team tactical style tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleTag {
    // Possession & Build-up
    /// Heavy reliance on single playmaker (Gini >= 0.35)
    HubDependent,
    /// High forward pass rate (progressive >= 20%)
    DirectProgression,
    /// Patient build-up play
    PossessionBased,
    /// Quick vertical passing
    DirectPlay,

    // Width & Shape
    /// Narrow team shape (width < 38m)
    CentralCongestion,
    /// Wide attacking approach
    WingFocused,
    /// Balanced width across zones
    FlexibleWidth,

    // Tempo & Intensity
    /// Maintains pressure in final third
    HighPress,
    /// Compact defensive block
    LowBlock,
    /// Variable tempo based on situation
    TempoControl,

    // Transitions
    /// Quick attack after winning ball
    CounterAttacking,
    /// Organized positional build-up
    StructuredBuild,

    // Weaknesses
    /// Performance drops in second half
    SecondHalfWeakness,
    /// First half performance issues
    SlowStarters,

    // Ball Progression
    /// Heavy reliance on passes (carry < 15%)
    PassDependent,
    /// Heavy reliance on dribbling (carry > 50%)
    DribbleDependent,
    /// Balanced passing/dribbling mix
    BalancedProgression,

    // Set Pieces
    /// Strong at set pieces
    SetPieceStrength,
    /// Weak at defending set pieces
    SetPieceVulnerable,

    // Miscellaneous
    /// Tactically unpredictable
    Chaotic,
    /// Well-drilled system
    Systematic,
}

impl StyleTag {
    /// Display name for UI.
    pub fn display(&self) -> &'static str {
        match self {
            StyleTag::HubDependent => "Hub Dependent",
            StyleTag::DirectProgression => "Direct Progression",
            StyleTag::PossessionBased => "Possession Based",
            StyleTag::DirectPlay => "Direct Play",
            StyleTag::CentralCongestion => "Central Congestion",
            StyleTag::WingFocused => "Wing Focused",
            StyleTag::FlexibleWidth => "Flexible Width",
            StyleTag::HighPress => "High Press",
            StyleTag::LowBlock => "Low Block",
            StyleTag::TempoControl => "Tempo Control",
            StyleTag::CounterAttacking => "Counter Attacking",
            StyleTag::StructuredBuild => "Structured Build",
            StyleTag::SecondHalfWeakness => "Second Half Weakness",
            StyleTag::SlowStarters => "Slow Starters",
            StyleTag::PassDependent => "Pass Dependent",
            StyleTag::DribbleDependent => "Dribble Dependent",
            StyleTag::BalancedProgression => "Balanced Progression",
            StyleTag::SetPieceStrength => "Set Piece Strength",
            StyleTag::SetPieceVulnerable => "Set Piece Vulnerable",
            StyleTag::Chaotic => "Chaotic",
            StyleTag::Systematic => "Systematic",
        }
    }

    /// Short description for tooltips.
    pub fn description(&self) -> &'static str {
        match self {
            StyleTag::HubDependent => "Relies heavily on a single playmaker for ball distribution",
            StyleTag::DirectProgression => "Frequently plays forward passes to advance quickly",
            StyleTag::PossessionBased => "Prioritizes keeping the ball over direct attacks",
            StyleTag::DirectPlay => "Prefers quick, vertical passes over patient build-up",
            StyleTag::CentralCongestion => "Players tend to occupy central areas",
            StyleTag::WingFocused => "Attacks primarily through wide positions",
            StyleTag::FlexibleWidth => "Uses the full width of the pitch adaptively",
            StyleTag::HighPress => "Applies pressure high up the pitch",
            StyleTag::LowBlock => "Defends deep with compact shape",
            StyleTag::TempoControl => "Varies pace of play strategically",
            StyleTag::CounterAttacking => "Dangerous on quick transitions",
            StyleTag::StructuredBuild => "Methodical positional play in build-up",
            StyleTag::SecondHalfWeakness => "Performance tends to drop after halftime",
            StyleTag::SlowStarters => "Takes time to get into the game",
            StyleTag::PassDependent => "Progresses mainly through passing",
            StyleTag::DribbleDependent => "Relies heavily on dribbling to advance",
            StyleTag::BalancedProgression => "Good mix of passing and dribbling",
            StyleTag::SetPieceStrength => "Effective at scoring from set pieces",
            StyleTag::SetPieceVulnerable => "Concedes often from set pieces",
            StyleTag::Chaotic => "Unpredictable but potentially disorganized",
            StyleTag::Systematic => "Well-organized tactical approach",
        }
    }

    /// Whether this is a weakness tag.
    pub fn is_weakness(&self) -> bool {
        matches!(
            self,
            StyleTag::HubDependent
            | StyleTag::CentralCongestion
            | StyleTag::SecondHalfWeakness
            | StyleTag::SlowStarters
            | StyleTag::PassDependent
            | StyleTag::DribbleDependent
            | StyleTag::SetPieceVulnerable
            | StyleTag::Chaotic
        )
    }
}

/// Input metrics for style tag generation.
#[derive(Debug, Clone, Default)]
pub struct TeamMetrics {
    /// Gini metrics for distribution patterns
    pub gini: GiniMetrics,
    /// Progressive pass rate (progressive / total)
    pub progressive_rate: f32,
    /// Average team width in meters
    pub avg_width_m: f32,
    /// Average team depth in meters
    pub avg_depth_m: f32,
    /// Carry share (carry distance / total progression)
    pub carry_share: f32,
    /// First half goals scored
    pub first_half_goals: u32,
    /// Second half goals scored
    pub second_half_goals: u32,
    /// First half goals conceded
    pub first_half_conceded: u32,
    /// Second half goals conceded
    pub second_half_conceded: u32,
    /// Set piece goals scored
    pub set_piece_goals: u32,
    /// Set piece goals conceded
    pub set_piece_conceded: u32,
    /// High press success rate
    pub press_success_rate: f32,
    /// PPDA (passes allowed per defensive action)
    pub ppda: f32,
}

/// Builder for constructing TeamMetrics from raw analysis data.
#[derive(Debug, Default)]
pub struct TeamMetricsBuilder {
    gini: Option<GiniMetrics>,
    shape: Option<TeamShapeMetrics>,
    movement: Option<TeamMovementMetrics>,
    carries: Option<TeamCarryStats>,
    runs: Option<TeamRunStats>,
    entropy: Option<OccupancyEntropy>,
    // Game stats
    first_half_goals: u32,
    second_half_goals: u32,
    first_half_conceded: u32,
    second_half_conceded: u32,
    set_piece_goals: u32,
    set_piece_conceded: u32,
    total_passes: u32,
    progressive_passes: u32,
    ppda: f32,
}

impl TeamMetricsBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set Gini metrics.
    pub fn with_gini(mut self, gini: GiniMetrics) -> Self {
        self.gini = Some(gini);
        self
    }

    /// Set team shape metrics.
    pub fn with_shape(mut self, shape: TeamShapeMetrics) -> Self {
        self.shape = Some(shape);
        self
    }

    /// Set team movement metrics.
    pub fn with_movement(mut self, movement: TeamMovementMetrics) -> Self {
        self.movement = Some(movement);
        self
    }

    /// Set carry stats.
    pub fn with_carries(mut self, carries: TeamCarryStats) -> Self {
        self.carries = Some(carries);
        self
    }

    /// Set run stats.
    pub fn with_runs(mut self, runs: TeamRunStats) -> Self {
        self.runs = Some(runs);
        self
    }

    /// Set occupancy entropy.
    pub fn with_entropy(mut self, entropy: OccupancyEntropy) -> Self {
        self.entropy = Some(entropy);
        self
    }

    /// Set goal statistics.
    pub fn with_goals(mut self, first_half: u32, second_half: u32) -> Self {
        self.first_half_goals = first_half;
        self.second_half_goals = second_half;
        self
    }

    /// Set conceded statistics.
    pub fn with_conceded(mut self, first_half: u32, second_half: u32) -> Self {
        self.first_half_conceded = first_half;
        self.second_half_conceded = second_half;
        self
    }

    /// Set set piece statistics.
    pub fn with_set_pieces(mut self, scored: u32, conceded: u32) -> Self {
        self.set_piece_goals = scored;
        self.set_piece_conceded = conceded;
        self
    }

    /// Set pass statistics.
    pub fn with_passes(mut self, total: u32, progressive: u32) -> Self {
        self.total_passes = total;
        self.progressive_passes = progressive;
        self
    }

    /// Set PPDA.
    pub fn with_ppda(mut self, ppda: f32) -> Self {
        self.ppda = ppda;
        self
    }

    /// Build the TeamMetrics.
    pub fn build(self) -> TeamMetrics {
        let gini = self.gini.unwrap_or_default();
        let shape = self.shape.unwrap_or_default();
        let carries = self.carries.unwrap_or_default();

        // Calculate progressive rate
        let progressive_rate = if self.total_passes > 0 {
            self.progressive_passes as f32 / self.total_passes as f32
        } else {
            0.0
        };

        // Calculate carry share (carry distance / total progression distance)
        let total_progression = carries.progressive_distance_m + self.progressive_passes as f32 * 8.0; // Approximate 8m per progressive pass
        let carry_share = if total_progression > 0.0 {
            carries.progressive_distance_m / total_progression
        } else {
            0.0
        };

        TeamMetrics {
            gini,
            progressive_rate,
            avg_width_m: shape.avg_width,
            avg_depth_m: shape.avg_depth,
            carry_share,
            first_half_goals: self.first_half_goals,
            second_half_goals: self.second_half_goals,
            first_half_conceded: self.first_half_conceded,
            second_half_conceded: self.second_half_conceded,
            set_piece_goals: self.set_piece_goals,
            set_piece_conceded: self.set_piece_conceded,
            press_success_rate: 0.0, // TODO: Calculate from pressing data
            ppda: self.ppda,
        }
    }
}

/// Generate style tags from team metrics.
pub fn generate_style_tags(metrics: &TeamMetrics) -> Vec<StyleTag> {
    let mut tags = Vec::new();

    // Hub dependency (high Gini)
    if metrics.gini.touch_gini >= 0.35 || metrics.gini.pass_recv_gini >= 0.38 {
        tags.push(StyleTag::HubDependent);
    }

    // Direct progression
    if metrics.progressive_rate >= 0.20 {
        tags.push(StyleTag::DirectProgression);
    } else if metrics.progressive_rate < 0.10 {
        tags.push(StyleTag::PossessionBased);
    }

    // Width patterns
    if metrics.avg_width_m < 38.0 {
        tags.push(StyleTag::CentralCongestion);
    } else if metrics.avg_width_m > 50.0 {
        tags.push(StyleTag::WingFocused);
    } else {
        tags.push(StyleTag::FlexibleWidth);
    }

    // Ball progression style
    if metrics.carry_share < 0.15 {
        tags.push(StyleTag::PassDependent);
    } else if metrics.carry_share > 0.50 {
        tags.push(StyleTag::DribbleDependent);
    } else {
        tags.push(StyleTag::BalancedProgression);
    }

    // Half performance
    let total_goals = metrics.first_half_goals + metrics.second_half_goals;
    if total_goals > 0 {
        let second_half_ratio = metrics.second_half_goals as f32 / total_goals as f32;
        if second_half_ratio < 0.35 {
            tags.push(StyleTag::SecondHalfWeakness);
        }
        let first_half_ratio = metrics.first_half_goals as f32 / total_goals as f32;
        if first_half_ratio < 0.35 {
            tags.push(StyleTag::SlowStarters);
        }
    }

    // Pressing style
    if metrics.ppda < 8.0 {
        tags.push(StyleTag::HighPress);
    } else if metrics.ppda > 15.0 {
        tags.push(StyleTag::LowBlock);
    }

    // Set pieces
    if metrics.set_piece_goals >= 3 {
        tags.push(StyleTag::SetPieceStrength);
    }
    if metrics.set_piece_conceded >= 3 {
        tags.push(StyleTag::SetPieceVulnerable);
    }

    tags
}

/// Extended metrics for style tag generation.
#[derive(Debug, Clone, Default)]
pub struct ExtendedTeamMetrics {
    /// Base team metrics
    pub base: TeamMetrics,
    /// Sprint metrics
    pub movement: TeamMovementMetrics,
    /// Occupancy entropy
    pub entropy: OccupancyEntropy,
    /// Carry stats
    pub carries: TeamCarryStats,
    /// Run stats
    pub runs: TeamRunStats,
}

/// Generate style tags from extended metrics with all available data.
pub fn generate_style_tags_extended(metrics: &ExtendedTeamMetrics) -> Vec<StyleTag> {
    let mut tags = generate_style_tags(&metrics.base);

    // Sprint-based tags
    if metrics.movement.sprint_ratio > 0.15 {
        // High sprint ratio = counter-attacking potential
        if !tags.contains(&StyleTag::CounterAttacking) {
            tags.push(StyleTag::CounterAttacking);
        }
    }

    // Entropy-based tags
    if metrics.entropy.team_avg_entropy > 0.7 {
        // High entropy = flexible positioning
        if !tags.contains(&StyleTag::FlexibleWidth) {
            tags.push(StyleTag::FlexibleWidth);
        }
    } else if metrics.entropy.team_avg_entropy < 0.4 {
        // Low entropy = rigid positioning
        if !tags.contains(&StyleTag::Systematic) {
            tags.push(StyleTag::Systematic);
        }
    }

    // Entropy variance check
    if metrics.entropy.team_entropy_variance > 0.05 {
        // High variance = some players static, others dynamic
        if !tags.contains(&StyleTag::HubDependent) {
            tags.push(StyleTag::HubDependent);
        }
    }

    // Carry-based tags
    if metrics.carries.total_carries > 0 {
        let progressive_rate = metrics.carries.progressive_carries as f32 / metrics.carries.total_carries as f32;
        if progressive_rate > 0.4 {
            // High progressive carry rate = direct play
            if !tags.contains(&StyleTag::DirectPlay) {
                tags.push(StyleTag::DirectPlay);
            }
        }

        // Dispossession rate
        let dispossession_rate = metrics.carries.dispossessions as f32 / metrics.carries.total_carries as f32;
        if dispossession_rate > 0.3 {
            // Losing ball often = risky dribbling
            if !tags.contains(&StyleTag::Chaotic) {
                tags.push(StyleTag::Chaotic);
            }
        }
    }

    // Run-based tags (if we have run stats)
    if metrics.runs.total_runs > 0 {
        let runs_in_behind_ratio = metrics.runs.runs_in_behind as f32 / metrics.runs.total_runs as f32;
        if runs_in_behind_ratio > 0.25 {
            tags.push(StyleTag::DirectProgression);
        }
    }

    tags
}

/// Tactical recommendation based on opponent's style tags.
#[derive(Debug, Clone)]
pub struct TacticalRecommendation {
    /// Short title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Priority (higher = more important)
    pub priority: u8,
}

/// Generate counter-tactics based on opponent's style tags.
pub fn generate_counter_tactics(tags: &[StyleTag]) -> Vec<TacticalRecommendation> {
    let mut recommendations = Vec::new();

    for tag in tags {
        match tag {
            StyleTag::HubDependent => {
                recommendations.push(TacticalRecommendation {
                    title: "Mark the Hub".to_string(),
                    description: "Assign a man-marker to their key playmaker to disrupt distribution".to_string(),
                    priority: 9,
                });
                recommendations.push(TacticalRecommendation {
                    title: "Press Triggers".to_string(),
                    description: "Initiate press when the ball goes to the hub player".to_string(),
                    priority: 7,
                });
            }
            StyleTag::CentralCongestion => {
                recommendations.push(TacticalRecommendation {
                    title: "Use the Flanks".to_string(),
                    description: "Switch play to wide areas to exploit their narrow shape".to_string(),
                    priority: 8,
                });
            }
            StyleTag::SecondHalfWeakness => {
                recommendations.push(TacticalRecommendation {
                    title: "Defensive First Half".to_string(),
                    description: "Stay compact in the first half, push for goals after 60 minutes".to_string(),
                    priority: 7,
                });
            }
            StyleTag::HighPress => {
                recommendations.push(TacticalRecommendation {
                    title: "Play Out from Back".to_string(),
                    description: "Draw them in then exploit space behind with long balls".to_string(),
                    priority: 6,
                });
            }
            StyleTag::LowBlock => {
                recommendations.push(TacticalRecommendation {
                    title: "Patient Build-up".to_string(),
                    description: "Circulate the ball to create gaps, avoid rushed attacks".to_string(),
                    priority: 6,
                });
            }
            StyleTag::CounterAttacking => {
                recommendations.push(TacticalRecommendation {
                    title: "Controlled Possession".to_string(),
                    description: "Don't lose the ball cheaply, manage transitions carefully".to_string(),
                    priority: 8,
                });
            }
            _ => {}
        }
    }

    // Sort by priority
    recommendations.sort_by(|a, b| b.priority.cmp(&a.priority));
    recommendations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_style_tags() {
        let metrics = TeamMetrics {
            gini: GiniMetrics {
                touch_gini: 0.40,
                pass_sent_gini: 0.35,
                pass_recv_gini: 0.42,
                progressive_gini: 0.30,
            },
            progressive_rate: 0.12,
            avg_width_m: 35.0,
            carry_share: 0.20,
            ..Default::default()
        };

        let tags = generate_style_tags(&metrics);

        assert!(tags.contains(&StyleTag::HubDependent));
        assert!(tags.contains(&StyleTag::CentralCongestion));
        assert!(tags.contains(&StyleTag::BalancedProgression));
    }

    #[test]
    fn test_counter_tactics() {
        let tags = vec![StyleTag::HubDependent, StyleTag::CentralCongestion];
        let recommendations = generate_counter_tactics(&tags);

        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.title.contains("Hub")));
        assert!(recommendations.iter().any(|r| r.title.contains("Flanks")));
    }

    #[test]
    fn test_team_metrics_builder() {
        let gini = GiniMetrics {
            touch_gini: 0.32,
            pass_sent_gini: 0.28,
            pass_recv_gini: 0.30,
            progressive_gini: 0.25,
        };

        let shape = TeamShapeMetrics {
            avg_width: 45.0,
            avg_depth: 35.0,
            ..Default::default()
        };

        let metrics = TeamMetricsBuilder::new()
            .with_gini(gini.clone())
            .with_shape(shape)
            .with_goals(1, 2)
            .with_conceded(1, 0)
            .with_passes(500, 80)
            .with_ppda(9.5)
            .build();

        assert_eq!(metrics.gini.touch_gini, gini.touch_gini);
        assert_eq!(metrics.avg_width_m, 45.0);
        assert_eq!(metrics.first_half_goals, 1);
        assert_eq!(metrics.second_half_goals, 2);
        assert!(metrics.progressive_rate > 0.15); // 80/500 = 0.16
    }

    #[test]
    fn test_generate_style_tags_extended() {
        let base = TeamMetrics {
            gini: GiniMetrics::default(),
            progressive_rate: 0.15,
            avg_width_m: 42.0,
            carry_share: 0.25,
            ..Default::default()
        };

        let movement = TeamMovementMetrics {
            sprint_ratio: 0.18,
            ..Default::default()
        };

        let entropy = OccupancyEntropy {
            team_avg_entropy: 0.35,
            team_entropy_variance: 0.02,
            ..Default::default()
        };

        let extended = ExtendedTeamMetrics {
            base,
            movement,
            entropy,
            ..Default::default()
        };

        let tags = generate_style_tags_extended(&extended);

        // High sprint ratio should add CounterAttacking
        assert!(tags.contains(&StyleTag::CounterAttacking));
        // Low entropy should add Systematic
        assert!(tags.contains(&StyleTag::Systematic));
    }

    #[test]
    fn test_style_tag_display() {
        assert_eq!(StyleTag::HighPress.display(), "High Press");
        assert_eq!(StyleTag::HubDependent.display(), "Hub Dependent");
        assert!(StyleTag::HubDependent.is_weakness());
        assert!(!StyleTag::HighPress.is_weakness());
    }
}
