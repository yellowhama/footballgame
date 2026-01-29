//! Anchor Table - Target Statistics from Real Matches
//!
//! Contains reference statistics used as calibration targets.

use std::collections::HashMap;
use super::zone::ZoneId;

/// Statistic with percentiles for distribution matching
#[derive(Debug, Clone)]
pub struct StatWithPercentiles {
    pub mean: f32,
    pub p10: f32,
    pub p25: f32,
    pub p50: f32,
    pub p75: f32,
    pub p90: f32,
    pub std: f32,
}

impl StatWithPercentiles {
    /// Create with just mean (other values derived)
    pub fn from_mean(mean: f32) -> Self {
        Self {
            mean,
            p10: mean * 0.7,
            p25: mean * 0.85,
            p50: mean,
            p75: mean * 1.15,
            p90: mean * 1.3,
            std: mean * 0.2,
        }
    }

    /// Check if a value is within acceptable range (P10-P90)
    pub fn is_in_range(&self, value: f32) -> bool {
        value >= self.p10 && value <= self.p90
    }

    /// Calculate how far a value is from the mean (as ratio)
    pub fn deviation_ratio(&self, value: f32) -> f32 {
        if self.mean == 0.0 {
            return 0.0;
        }
        (value - self.mean).abs() / self.mean
    }
}

/// Pass type shares
#[derive(Debug, Clone)]
pub struct PassTypeShares {
    pub progressive: f32,
    pub key: f32,
    pub cross: f32,
    pub long: f32,
    pub backward: f32,
    pub lateral: f32,
}

impl Default for PassTypeShares {
    fn default() -> Self {
        // EPL 2024-25 averages
        Self {
            progressive: 0.18,
            key: 0.04,
            cross: 0.05,
            long: 0.10,
            backward: 0.25,
            lateral: 0.38,
        }
    }
}

impl PassTypeShares {
    /// Get L1 distance between two distributions
    pub fn l1_distance(&self, other: &PassTypeShares) -> f32 {
        (self.progressive - other.progressive).abs()
            + (self.key - other.key).abs()
            + (self.cross - other.cross).abs()
            + (self.long - other.long).abs()
            + (self.backward - other.backward).abs()
            + (self.lateral - other.lateral).abs()
    }
}

/// Zone distribution shares
pub type ZoneShares = HashMap<ZoneId, f32>;

/// Create default zone shares
pub fn default_zone_shares() -> ZoneShares {
    let mut shares = HashMap::new();
    shares.insert(ZoneId::LDef, 0.12);
    shares.insert(ZoneId::CDef, 0.18);
    shares.insert(ZoneId::RDef, 0.12);
    shares.insert(ZoneId::LAtt, 0.18);
    shares.insert(ZoneId::CAtt, 0.22);
    shares.insert(ZoneId::RAtt, 0.18);
    shares
}

/// Create default xG zone shares
pub fn default_xg_zone_shares() -> ZoneShares {
    let mut shares = HashMap::new();
    shares.insert(ZoneId::LDef, 0.01);
    shares.insert(ZoneId::CDef, 0.02);
    shares.insert(ZoneId::RDef, 0.01);
    shares.insert(ZoneId::LAtt, 0.22);
    shares.insert(ZoneId::CAtt, 0.52);
    shares.insert(ZoneId::RAtt, 0.22);
    shares
}

/// Defensive action statistics
#[derive(Debug, Clone)]
pub struct DefensiveStats {
    pub tackle: StatWithPercentiles,
    pub intercept: StatWithPercentiles,
    pub clear: StatWithPercentiles,
    pub press: StatWithPercentiles,
    pub block: StatWithPercentiles,
}

impl Default for DefensiveStats {
    fn default() -> Self {
        // EPL 2024-25 averages
        Self {
            tackle: StatWithPercentiles {
                mean: 17.5, p10: 12.0, p25: 14.0, p50: 17.0, p75: 21.0, p90: 24.0, std: 4.5,
            },
            intercept: StatWithPercentiles {
                mean: 10.2, p10: 6.0, p25: 8.0, p50: 10.0, p75: 12.0, p90: 15.0, std: 3.2,
            },
            clear: StatWithPercentiles {
                mean: 18.0, p10: 10.0, p25: 13.0, p50: 17.0, p75: 22.0, p90: 30.0, std: 7.0,
            },
            press: StatWithPercentiles {
                mean: 145.0, p10: 110.0, p25: 125.0, p50: 145.0, p75: 165.0, p90: 180.0, std: 25.0,
            },
            block: StatWithPercentiles {
                mean: 12.0, p10: 7.0, p25: 9.0, p50: 12.0, p75: 15.0, p90: 18.0, std: 3.5,
            },
        }
    }
}

/// Pass statistics
#[derive(Debug, Clone)]
pub struct PassStats {
    pub attempt: StatWithPercentiles,
    pub success_rate: StatWithPercentiles,
    pub types_share: PassTypeShares,
}

impl Default for PassStats {
    fn default() -> Self {
        Self {
            attempt: StatWithPercentiles {
                mean: 460.0, p10: 320.0, p25: 380.0, p50: 450.0, p75: 530.0, p90: 620.0, std: 95.0,
            },
            success_rate: StatWithPercentiles {
                mean: 0.83, p10: 0.76, p25: 0.79, p50: 0.83, p75: 0.86, p90: 0.89, std: 0.045,
            },
            types_share: PassTypeShares::default(),
        }
    }
}

/// Shot statistics
#[derive(Debug, Clone)]
pub struct ShotStats {
    pub attempt: StatWithPercentiles,
    pub on_target_rate: StatWithPercentiles,
    pub conversion_rate: StatWithPercentiles,
}

impl Default for ShotStats {
    fn default() -> Self {
        Self {
            attempt: StatWithPercentiles {
                mean: 12.5, p10: 7.0, p25: 9.0, p50: 12.0, p75: 15.0, p90: 19.0, std: 4.2,
            },
            on_target_rate: StatWithPercentiles {
                mean: 0.34, p10: 0.26, p25: 0.30, p50: 0.34, p75: 0.38, p90: 0.44, std: 0.065,
            },
            conversion_rate: StatWithPercentiles {
                mean: 0.10, p10: 0.05, p25: 0.07, p50: 0.10, p75: 0.13, p90: 0.16, std: 0.04,
            },
        }
    }
}

/// xG statistics
#[derive(Debug, Clone)]
pub struct XgStats {
    pub total_mean: f32,
    pub total_p10: f32,
    pub total_p90: f32,
    pub by_zone_share: ZoneShares,
}

impl Default for XgStats {
    fn default() -> Self {
        Self {
            total_mean: 1.45,
            total_p10: 0.8,
            total_p90: 2.2,
            by_zone_share: default_xg_zone_shares(),
        }
    }
}

/// Team statistics (per match)
#[derive(Debug, Clone, Default)]
pub struct TeamStats {
    pub defensive: DefensiveStats,
    pub passes: PassStats,
    pub shots: ShotStats,
    pub xg: XgStats,
    pub touch_share_by_zone: ZoneShares,
    pub shot_share_by_zone: ZoneShares,
}

/// Anchor table scope
#[derive(Debug, Clone)]
pub struct AnchorScope {
    pub competition: String,
    pub season: String,
    pub match_minutes: u32,
    pub tempo_band: String,
    pub data_source: String,
}

impl Default for AnchorScope {
    fn default() -> Self {
        Self {
            competition: "EPL".to_string(),
            season: "2024-2025".to_string(),
            match_minutes: 90,
            tempo_band: "normal".to_string(),
            data_source: "FBRef".to_string(),
        }
    }
}

/// Complete anchor table
#[derive(Debug, Clone)]
pub struct AnchorTable {
    pub version: u32,
    pub scope: AnchorScope,
    pub team_per_match: TeamStats,
}

impl Default for AnchorTable {
    fn default() -> Self {
        let mut team_stats = TeamStats::default();
        team_stats.touch_share_by_zone = default_zone_shares();
        team_stats.shot_share_by_zone = default_xg_zone_shares();

        Self {
            version: 1,
            scope: AnchorScope::default(),
            team_per_match: team_stats,
        }
    }
}

impl AnchorTable {
    /// Get the scope key for this table
    pub fn scope_key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.scope.competition,
            self.scope.season.split('-').next().unwrap_or("2024"),
            self.scope.tempo_band
        )
    }

    /// Load EPL 2024-25 defaults
    pub fn epl_2024() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_anchor_table() {
        let table = AnchorTable::default();
        assert_eq!(table.scope.competition, "EPL");
        assert!((table.team_per_match.passes.types_share.progressive - 0.18).abs() < 0.01);
    }

    #[test]
    fn test_scope_key() {
        let table = AnchorTable::default();
        assert_eq!(table.scope_key(), "EPL|2024|normal");
    }

    #[test]
    fn test_stat_in_range() {
        let stat = StatWithPercentiles::from_mean(100.0);
        assert!(stat.is_in_range(100.0));
        assert!(stat.is_in_range(80.0));
        assert!(!stat.is_in_range(50.0));
    }
}
