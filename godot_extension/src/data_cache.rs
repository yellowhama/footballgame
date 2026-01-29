//! Data Cache Module
//!
//! GameCache.gd에서 주입하는 데이터 캐시 관리
//!
//! ## Embedded Data Support (2025-12-05)
//!
//! When compiled with `--features embedded_players`, player cache is embedded
//! in the binary and used as fallback when external file loading fails.

#[cfg(feature = "embedded_players")]
use cache_builder::load_person_cache_embedded;
use cache_builder::{has_embedded_player_cache, load_person_cache, PersonIndex};
use godot::prelude::*;
use of_core::models::person::{Person, PositionRating};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// 기후-스타일 계수
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateCoeff {
    pub country_code: String,
    pub climate_type: String,
    pub avg_possession: f64,
    pub avg_longball: f64,
    pub avg_pass_success: f64,
}

fn primary_position_token(position: &str) -> String {
    let cleaned = position.replace('"', "");
    let first_segment = cleaned.split(',').next().unwrap_or("").trim();
    let base = first_segment.split('(').next().unwrap_or("").trim();
    if base.is_empty() {
        String::from("MF")
    } else {
        base.to_string()
    }
}

fn simplify_position_group(position: &str) -> String {
    let upper = position.trim().to_uppercase();

    match upper.as_str() {
        "GK" | "GKP" => String::from("GK"),
        "D" | "DC" | "DR" | "DL" | "DM" | "WB" | "WBL" | "WBR" | "CB" | "RB" | "LB" | "SW" => {
            String::from("DF")
        }
        "M" | "MC" | "ML" | "MR" | "AM" | "AMC" | "AML" | "AMR" | "CM" | "CDM" | "CAM" | "LM"
        | "RM" => String::from("MF"),
        "ST" | "FW" | "CF" | "SS" => String::from("FW"),
        _ => {
            if upper.starts_with('G') {
                String::from("GK")
            } else if upper.starts_with('D') || upper.starts_with("WB") {
                String::from("DF")
            } else if upper.starts_with('M') || upper.starts_with('A') {
                String::from("MF")
            } else if upper.starts_with('S') || upper.starts_with('F') {
                String::from("FW")
            } else {
                String::from("MF")
            }
        }
    }
}

/// 선수 밸런스 통계
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceStat {
    pub position: String,
    pub median_ca: f64,
    pub p90_ca: f64,
    pub p10_ca: f64,
    pub avg_ca: f64,
    pub player_count: i32,
}

/// 코치 카드
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachCardData {
    pub card_id: String,
    pub name: String,
    pub rating: i32,
    pub tactical_style: String,
    pub attack_bonus: f64,
    pub defense_bonus: f64,
}

/// 훈련 효율성
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingEfficiency {
    pub training_type: String,
    pub avg_ca_gain: f64,
    pub median_ca_gain: f64,
    pub overtraining_rate: f64,
}

/// 전역 데이터 캐시 저장소
#[derive(GodotClass)]
#[class(base=Object)]
pub struct DataCacheStore {
    /// 기후 계수 맵 (country_code -> ClimateCoeff)
    climate_coeffs: HashMap<String, ClimateCoeff>,

    /// 밸런스 통계 맵 (position -> BalanceStat)
    balance_stats: HashMap<String, BalanceStat>,

    /// 코치 카드 맵 (card_id -> CoachCardData)
    coach_cards: HashMap<String, CoachCardData>,

    /// 훈련 효율성 맵 (training_type -> TrainingEfficiency)
    training_efficiency: HashMap<String, TrainingEfficiency>,

    /// 선수 인덱스 (UID → Person) - Phase 3
    player_index: Option<PersonIndex>,

    #[base]
    base: Base<Object>,
}

#[godot_api]
impl IObject for DataCacheStore {
    fn init(base: Base<Object>) -> Self {
        Self {
            climate_coeffs: HashMap::new(),
            balance_stats: HashMap::new(),
            coach_cards: HashMap::new(),
            training_efficiency: HashMap::new(),
            player_index: None,
            base,
        }
    }
}

#[godot_api]
impl DataCacheStore {
    /// GameCache.gd에서 호출: 기후 계수 데이터 주입
    #[func]
    pub fn set_climate_coeffs(&mut self, data: Dictionary) {
        self.climate_coeffs.clear();

        // Dictionary를 순회하며 파싱
        for (key, value) in data.iter_shared() {
            if let (Ok(country_code), Ok(dict)) =
                (key.try_to::<GString>(), value.try_to::<Dictionary>())
            {
                let climate_type = dict
                    .get("climate_type")
                    .and_then(|v| v.try_to::<GString>().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                let avg_possession = dict
                    .get("avg_possession")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);

                let avg_longball = dict
                    .get("avg_longball")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);

                let avg_pass_success = dict
                    .get("avg_pass_success")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);

                self.climate_coeffs.insert(
                    country_code.to_string(),
                    ClimateCoeff {
                        country_code: country_code.to_string(),
                        climate_type,
                        avg_possession,
                        avg_longball,
                        avg_pass_success,
                    },
                );
            }
        }

        godot_print!(
            "✅ Climate coeffs loaded: {} entries",
            self.climate_coeffs.len()
        );
    }

    /// GameCache.gd에서 호출: 밸런스 데이터 주입
    #[func]
    pub fn set_balance_data(&mut self, data: Dictionary) {
        self.balance_stats.clear();

        for (key, value) in data.iter_shared() {
            if let (Ok(position), Ok(dict)) =
                (key.try_to::<GString>(), value.try_to::<Dictionary>())
            {
                let median_ca = dict
                    .get("median_ca")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);
                let p90_ca = dict
                    .get("p90_ca")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);
                let p10_ca = dict
                    .get("p10_ca")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);
                let avg_ca = dict
                    .get("avg_ca")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);
                let player_count = dict
                    .get("player_count")
                    .and_then(|v| v.try_to::<i32>().ok())
                    .unwrap_or(0);

                self.balance_stats.insert(
                    position.to_string(),
                    BalanceStat {
                        position: position.to_string(),
                        median_ca,
                        p90_ca,
                        p10_ca,
                        avg_ca,
                        player_count,
                    },
                );
            }
        }

        godot_print!(
            "✅ Balance data loaded: {} positions",
            self.balance_stats.len()
        );
    }

    /// GameCache.gd에서 호출: 코치 카드 데이터 주입
    #[func]
    pub fn set_coach_cards(&mut self, data: Array<Variant>) {
        self.coach_cards.clear();

        for value in data.iter_shared() {
            if let Ok(dict) = value.try_to::<Dictionary>() {
                let card_id = dict
                    .get("card_id")
                    .and_then(|v| v.try_to::<GString>().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                if card_id.is_empty() {
                    continue;
                }

                let name = dict
                    .get("name")
                    .and_then(|v| v.try_to::<GString>().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let rating = dict
                    .get("rating")
                    .and_then(|v| v.try_to::<i32>().ok())
                    .unwrap_or(0);
                let tactical_style = dict
                    .get("tactical_style")
                    .and_then(|v| v.try_to::<GString>().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let attack_bonus = dict
                    .get("attack_bonus")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);
                let defense_bonus = dict
                    .get("defense_bonus")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);

                self.coach_cards.insert(
                    card_id.clone(),
                    CoachCardData {
                        card_id,
                        name,
                        rating,
                        tactical_style,
                        attack_bonus,
                        defense_bonus,
                    },
                );
            }
        }

        godot_print!("✅ Coach cards loaded: {} cards", self.coach_cards.len());
    }

    /// GameCache.gd에서 호출: 훈련 효율성 데이터 주입
    #[func]
    pub fn set_training_efficiency(&mut self, data: Dictionary) {
        self.training_efficiency.clear();

        for (key, value) in data.iter_shared() {
            if let (Ok(training_type), Ok(dict)) =
                (key.try_to::<GString>(), value.try_to::<Dictionary>())
            {
                let avg_ca_gain = dict
                    .get("avg_ca_gain")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);
                let median_ca_gain = dict
                    .get("median_ca_gain")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);
                let overtraining_rate = dict
                    .get("overtraining_rate")
                    .and_then(|v| v.try_to::<f64>().ok())
                    .unwrap_or(0.0);

                self.training_efficiency.insert(
                    training_type.to_string(),
                    TrainingEfficiency {
                        training_type: training_type.to_string(),
                        avg_ca_gain,
                        median_ca_gain,
                        overtraining_rate,
                    },
                );
            }
        }

        godot_print!(
            "✅ Training efficiency loaded: {} types",
            self.training_efficiency.len()
        );
    }

    /// 기후 계수 조회
    #[func]
    pub fn get_climate_coeff(&self, country_code: GString) -> Dictionary {
        let code = country_code.to_string();

        if let Some(coeff) = self.climate_coeffs.get(&code) {
            let mut dict = Dictionary::new();
            dict.insert("country_code", coeff.country_code.clone());
            dict.insert("climate_type", coeff.climate_type.clone());
            dict.insert("avg_possession", coeff.avg_possession);
            dict.insert("avg_longball", coeff.avg_longball);
            dict.insert("avg_pass_success", coeff.avg_pass_success);
            dict
        } else {
            Dictionary::new()
        }
    }

    /// 밸런스 통계 조회
    #[func]
    pub fn get_balance_stat(&self, position: GString) -> Dictionary {
        let pos = position.to_string();

        if let Some(stat) = self.balance_stats.get(&pos) {
            let mut dict = Dictionary::new();
            dict.insert("position", stat.position.clone());
            dict.insert("median_ca", stat.median_ca);
            dict.insert("p90_ca", stat.p90_ca);
            dict.insert("p10_ca", stat.p10_ca);
            dict.insert("avg_ca", stat.avg_ca);
            dict.insert("player_count", stat.player_count);
            dict
        } else {
            Dictionary::new()
        }
    }

    /// 코치 카드 조회
    #[func]
    pub fn get_coach_card(&self, card_id: GString) -> Dictionary {
        let id = card_id.to_string();

        if let Some(card) = self.coach_cards.get(&id) {
            let mut dict = Dictionary::new();
            dict.insert("card_id", card.card_id.clone());
            dict.insert("name", card.name.clone());
            dict.insert("rating", card.rating);
            dict.insert("tactical_style", card.tactical_style.clone());
            dict.insert("attack_bonus", card.attack_bonus);
            dict.insert("defense_bonus", card.defense_bonus);
            dict
        } else {
            Dictionary::new()
        }
    }

    /// 훈련 효율성 조회
    #[func]
    pub fn get_training_efficiency(&self, training_type: GString) -> Dictionary {
        let t_type = training_type.to_string();

        if let Some(eff) = self.training_efficiency.get(&t_type) {
            let mut dict = Dictionary::new();
            dict.insert("training_type", eff.training_type.clone());
            dict.insert("avg_ca_gain", eff.avg_ca_gain);
            dict.insert("median_ca_gain", eff.median_ca_gain);
            dict.insert("overtraining_rate", eff.overtraining_rate);
            dict
        } else {
            Dictionary::new()
        }
    }

    /// 모든 캐시 통계
    #[func]
    pub fn get_cache_stats(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("climate_coeffs_count", self.climate_coeffs.len() as i32);
        dict.insert("balance_stats_count", self.balance_stats.len() as i32);
        dict.insert("coach_cards_count", self.coach_cards.len() as i32);
        dict.insert(
            "training_efficiency_count",
            self.training_efficiency.len() as i32,
        );
        dict.insert(
            "player_index_count",
            self.player_index
                .as_ref()
                .map(|idx| idx.len() as i32)
                .unwrap_or(0),
        );
        dict
    }

    // ========== Phase 3: Player Cache Functions ==========

    /// Load player cache from binary file (with embedded fallback)
    ///
    /// # Arguments
    /// * `cache_path` - Absolute path to cache file (e.g., "F:/path/to/cache_players.v3.msgpack.lz4")
    ///
    /// # Returns
    /// true if loaded successfully, false otherwise
    ///
    /// # Fallback Behavior (when compiled with --features embedded_players)
    /// If external file loading fails, automatically falls back to embedded data.
    #[func]
    pub fn load_player_cache(&mut self, cache_path: GString) -> bool {
        let path_str = cache_path.to_string();
        let path = Path::new(&path_str);

        godot_print!("🔄 Loading player cache from: {}", path_str);

        // 1차: 외부 파일 시도
        match load_person_cache(path) {
            Ok(index) => {
                let player_count = index.len();
                let schema_version = index.schema_version.clone();

                self.player_index = Some(index);

                godot_print!(
                    "✅ Player cache loaded from file: {} players (schema: {})",
                    player_count,
                    schema_version
                );
                return true;
            }
            Err(e) => {
                godot_warn!("⚠️ External file load failed: {:?}", e);
            }
        }

        // 2차: 임베딩 데이터 폴백 (feature flag 활성화 시)
        #[cfg(feature = "embedded_players")]
        {
            godot_print!("🔄 Falling back to embedded player cache...");
            match load_person_cache_embedded() {
                Ok(index) => {
                    let player_count = index.len();
                    let schema_version = index.schema_version.clone();

                    self.player_index = Some(index);

                    godot_print!(
                        "✅ Embedded player cache loaded: {} players (schema: {})",
                        player_count,
                        schema_version
                    );
                    return true;
                }
                Err(e) => {
                    godot_error!("❌ Embedded cache also failed: {:?}", e);
                }
            }
        }

        #[cfg(not(feature = "embedded_players"))]
        {
            godot_warn!(
                "⚠️ No embedded fallback available (build without embedded_players feature)"
            );
        }

        self.player_index = None;
        false
    }

    /// Load player cache from embedded data only (no file I/O)
    ///
    /// # Returns
    /// true if embedded data loaded successfully, false if not available or failed
    #[func]
    pub fn load_embedded_player_cache(&mut self) -> bool {
        if !has_embedded_player_cache() {
            godot_warn!("⚠️ Embedded player cache not available (compiled without embedded_players feature)");
            return false;
        }

        #[cfg(feature = "embedded_players")]
        {
            godot_print!("🔄 Loading embedded player cache...");
            match load_person_cache_embedded() {
                Ok(index) => {
                    let player_count = index.len();
                    let schema_version = index.schema_version.clone();

                    self.player_index = Some(index);

                    godot_print!(
                        "✅ Embedded player cache loaded: {} players (schema: {})",
                        player_count,
                        schema_version
                    );
                    return true;
                }
                Err(e) => {
                    godot_error!("❌ Failed to load embedded cache: {:?}", e);
                }
            }
        }

        self.player_index = None;
        false
    }

    /// Check if embedded player cache is available
    #[func]
    pub fn has_embedded_players(&self) -> bool {
        has_embedded_player_cache()
    }

    /// Get player by UID
    ///
    /// # Arguments
    /// * `uid` - Player unique ID (1-8053)
    ///
    /// # Returns
    /// Dictionary containing player data, or empty Dictionary if not found
    #[func]
    pub fn get_player(&self, uid: i32) -> Dictionary {
        if let Some(ref index) = self.player_index {
            if let Some(person) = index.get(uid as u32) {
                let mut dict = Dictionary::new();
                dict.insert("uid", person.uid as i32);
                dict.insert("name", person.name.clone());
                dict.insert("nationality", person.nationality.clone());
                dict.insert("team", person.team.clone());
                dict.insert("position", person.position.clone());
                dict.insert("ca", person.ca as i32);
                dict.insert("pa", person.pa as i32);
                dict.insert("age", person.age as i32);
                return dict;
            }
        }

        Dictionary::new()
    }

    /// Search players by name (partial match, case-insensitive)
    ///
    /// # Arguments
    /// * `query` - Search query string
    /// * `use_real_names` - true to search real names, false for pseudonyms
    /// * `max_results` - Maximum number of results to return (0 = all)
    ///
    /// # Returns
    /// Array of player Dictionaries
    #[func]
    pub fn search_players_by_name(
        &self,
        query: GString,
        _use_real_names: bool,
        max_results: i32,
    ) -> Array<Variant> {
        let mut results = Array::new();

        if let Some(ref index) = self.player_index {
            let query_lower = query.to_string().to_lowercase();
            let max = if max_results <= 0 {
                usize::MAX
            } else {
                max_results as usize
            };
            let mut count = 0;

            for person in index.players.values() {
                if count >= max {
                    break;
                }

                let name_to_search = &person.name;

                if name_to_search.to_lowercase().contains(&query_lower) {
                    let mut dict = Dictionary::new();
                    dict.insert("uid", person.uid as i32);
                    dict.insert("name", person.name.clone());
                    dict.insert("nationality", person.nationality.clone());
                    dict.insert("team", person.team.clone());
                    dict.insert("position", person.position.clone());
                    dict.insert("ca", person.ca as i32);
                    dict.insert("pa", person.pa as i32);
                    dict.insert("age", person.age as i32);
                    results.push(&dict.to_variant());
                    count += 1;
                }
            }
        }

        results
    }

    /// Get players by team (exact match, case-sensitive)
    ///
    /// # Arguments
    /// * `team_name` - Team name to filter by
    /// * `use_real_names` - true for real team names, false for pseudonyms
    ///
    /// # Returns
    /// Array of player Dictionaries
    #[func]
    pub fn get_players_by_team(&self, team_name: GString, _use_real_names: bool) -> Array<Variant> {
        let mut results = Array::new();

        if let Some(ref index) = self.player_index {
            let team = team_name.to_string();

            for person in index.players.values() {
                let person_team = &person.team;

                if person_team == &team {
                    let mut dict = Dictionary::new();
                    dict.insert("uid", person.uid as i32);
                    dict.insert("name", person.name.clone());
                    dict.insert("nationality", person.nationality.clone());
                    dict.insert("team", person.team.clone());
                    dict.insert("position", person.position.clone());
                    dict.insert("ca", person.ca as i32);
                    dict.insert("pa", person.pa as i32);
                    dict.insert("age", person.age as i32);
                    results.push(&dict.to_variant());
                }
            }
        }

        results
    }

    /// Get wonderkids (age ≤ 21 and PA - CA ≥ 20)
    ///
    /// # Arguments
    /// * `max_results` - Maximum number of results to return (0 = all)
    ///
    /// # Returns
    /// Array of player Dictionaries
    #[func]
    pub fn get_wonderkids(&self, max_results: i32) -> Array<Variant> {
        let mut results = Array::new();

        if let Some(ref index) = self.player_index {
            let max = if max_results <= 0 {
                usize::MAX
            } else {
                max_results as usize
            };
            let mut count = 0;

            for person in index.players.values() {
                if count >= max {
                    break;
                }

                if person.is_wonderkid() {
                    let mut dict = Dictionary::new();
                    dict.insert("uid", person.uid as i32);
                    dict.insert("name", person.name.clone());
                    dict.insert("nationality", person.nationality.clone());
                    dict.insert("team", person.team.clone());
                    dict.insert("position", person.position.clone());
                    dict.insert("ca", person.ca as i32);
                    dict.insert("pa", person.pa as i32);
                    dict.insert("age", person.age as i32);
                    dict.insert("growth_potential", person.growth_potential() as i32);
                    results.push(&dict.to_variant());
                    count += 1;
                }
            }
        }

        results
    }

    /// Get world-class players (CA ≥ 170)
    ///
    /// # Arguments
    /// * `max_results` - Maximum number of results to return (0 = all)
    ///
    /// # Returns
    /// Array of player Dictionaries
    #[func]
    pub fn get_world_class_players(&self, max_results: i32) -> Array<Variant> {
        let mut results = Array::new();

        if let Some(ref index) = self.player_index {
            let max = if max_results <= 0 {
                usize::MAX
            } else {
                max_results as usize
            };
            let mut count = 0;

            for person in index.players.values() {
                if count >= max {
                    break;
                }

                if person.is_world_class() {
                    let mut dict = Dictionary::new();
                    dict.insert("uid", person.uid as i32);
                    dict.insert("name", person.name.clone());
                    dict.insert("nationality", person.nationality.clone());
                    dict.insert("team", person.team.clone());
                    dict.insert("position", person.position.clone());
                    dict.insert("ca", person.ca as i32);
                    dict.insert("pa", person.pa as i32);
                    dict.insert("age", person.age as i32);
                    results.push(&dict.to_variant());
                    count += 1;
                }
            }
        }

        results
    }

    /// Compute average CA for a given canonical position with optional filters
    /// position: expected canonical code like "GK", "CB", "CM", "ST".
    /// league_filter: currently unused placeholder (for future indexing); pass empty string to ignore.
    /// min_ca: minimum CA threshold to include in averaging (0 to disable).
    #[func]
    pub fn get_position_average(
        &self,
        position: GString,
        _league_filter: GString, // TODO: implement league filtering
        min_ca: i32,
    ) -> Dictionary {
        let mut out = Dictionary::new();

        let pos_str = position.to_string().to_uppercase();
        let group = match pos_str.as_str() {
            "GK" => "GK",
            "CB" => "DF",
            "CM" => "MF",
            "ST" => "FW",
            _ => "MF",
        };

        if let Some(ref index) = self.player_index {
            let mut count: i32 = 0;
            let mut total_ca: i64 = 0;

            for person in index.players.values() {
                let canon = simplify_position_group(&primary_position_token(&person.position));
                if canon != group {
                    continue;
                }
                let ca_i = person.ca as i32;
                if min_ca > 0 && ca_i < min_ca {
                    continue;
                }
                count += 1;
                total_ca += ca_i as i64;
            }

            if count > 0 {
                let avg_ca = (total_ca as f64) / (count as f64);
                out.insert("count", count);
                out.insert("ca", avg_ca.round() as i32);
            }
        }

        out
    }

    /// Generate a balanced roster by position groups using cached player data
    ///
    /// # Arguments
    /// * `min_ca` - Minimum current ability required (0-200)
    /// * `max_results` - Desired roster size (default 18 when <= 0)
    /// * `use_real_names` - Use real names/teams when true, otherwise pseudonyms
    ///
    /// # Returns
    /// Array of player dictionaries suitable for opponent roster generation
    #[func]
    pub fn get_balanced_roster(
        &self,
        min_ca: i32,
        max_results: i32,
        _use_real_names: bool,
    ) -> Array<Variant> {
        let mut results = Array::new();

        let target_total = if max_results <= 0 {
            18usize
        } else {
            max_results.max(0) as usize
        };

        if target_total == 0 {
            return results;
        }

        if let Some(ref index) = self.player_index {
            let min_ca = min_ca.clamp(0, 200) as u8;

            let mut eligible_all: Vec<&Person> = Vec::new();
            let mut goalkeepers: Vec<&Person> = Vec::new();
            let mut defenders: Vec<&Person> = Vec::new();
            let mut midfielders: Vec<&Person> = Vec::new();
            let mut forwards: Vec<&Person> = Vec::new();
            let mut others: Vec<&Person> = Vec::new();

            for person in index.players.values() {
                if person.ca < min_ca {
                    continue;
                }

                let primary = primary_position_token(&person.position);
                let simplified = simplify_position_group(&primary);

                eligible_all.push(person);

                match simplified.as_str() {
                    "GK" => goalkeepers.push(person),
                    "DF" => defenders.push(person),
                    "MF" => midfielders.push(person),
                    "FW" => forwards.push(person),
                    _ => others.push(person),
                }
            }

            if eligible_all.is_empty() {
                return results;
            }

            fn sort_by_ca_desc(list: &mut Vec<&Person>) {
                list.sort_by(|a, b| b.ca.cmp(&a.ca));
            }

            sort_by_ca_desc(&mut eligible_all);
            sort_by_ca_desc(&mut goalkeepers);
            sort_by_ca_desc(&mut defenders);
            sort_by_ca_desc(&mut midfielders);
            sort_by_ca_desc(&mut forwards);
            sort_by_ca_desc(&mut others);

            let mut selected: Vec<&Person> = Vec::new();
            let mut used: HashSet<u32> = HashSet::new();
            let mut remaining_total = target_total;

            macro_rules! take_from {
                ($source:expr, $quota:expr) => {{
                    let mut taken = 0usize;
                    for person in $source.iter() {
                        if taken >= $quota || remaining_total == 0 {
                            break;
                        }
                        if used.insert(person.uid) {
                            selected.push(*person);
                            taken += 1;
                            if remaining_total > 0 {
                                remaining_total -= 1;
                            }
                        }
                    }
                }};
            }

            let quota_gk = 2usize.min(target_total);
            let quota_df = 6usize.min(target_total.saturating_sub(quota_gk));
            let quota_mf = 6usize.min(target_total.saturating_sub(quota_gk + quota_df));
            let quota_fw = 4usize.min(target_total.saturating_sub(quota_gk + quota_df + quota_mf));

            take_from!(&goalkeepers, quota_gk);
            take_from!(&defenders, quota_df);
            take_from!(&midfielders, quota_mf);
            take_from!(&forwards, quota_fw);

            if remaining_total > 0 {
                take_from!(&others, remaining_total);
            }
            if remaining_total > 0 {
                take_from!(&eligible_all, remaining_total);
            }
            if remaining_total > 0 {
                let mut all_players_any: Vec<&Person> = index.players.values().collect();
                sort_by_ca_desc(&mut all_players_any);
                take_from!(&all_players_any, remaining_total);
            }

            selected.sort_by(|a, b| b.ca.cmp(&a.ca));

            for person in selected {
                let primary = primary_position_token(&person.position);
                let simplified = simplify_position_group(&primary);
                let mut dict = Dictionary::new();
                dict.insert("uid", person.uid as i32);
                dict.insert("name", person.name.clone());
                // Note: pseudo_name removed (FM 2023 direct usage)
                dict.insert("display_name", person.name.clone());
                dict.insert("nationality", person.nationality.clone());
                dict.insert("team", person.team.clone());
                // Note: pseudo_team removed (FM 2023 direct usage)
                dict.insert("display_team", person.team.clone());
                dict.insert("position", person.position.clone());
                dict.insert("primary_position", primary);
                dict.insert("position_group", simplified);
                dict.insert("ca", person.ca as i32);
                dict.insert("pa", person.pa as i32);
                dict.insert("age", person.age as i32);
                results.push(&dict.to_variant());
            }
        }

        results
    }

    /// Check if player cache is loaded
    #[func]
    pub fn is_player_cache_loaded(&self) -> bool {
        self.player_index.is_some()
    }

    /// Get player cache schema version
    #[func]
    pub fn get_player_cache_version(&self) -> GString {
        self.player_index
            .as_ref()
            .map(|idx| GString::from(idx.schema_version.as_str()))
            .unwrap_or_else(|| GString::from("none"))
    }

    /// Get total player count in cache
    #[func]
    pub fn get_player_count(&self) -> i32 {
        self.player_index
            .as_ref()
            .map(|idx| idx.len() as i32)
            .unwrap_or(0)
    }

    // ============================================================================
    // Position Ratings API (Phase 3: FM 2023 Integration)
    // ============================================================================

    /// Get all 14 position ratings for a player
    ///
    /// Returns: Dictionary with position keys (GK, DL, DC, DR, WBL, WBR, DM, ML, MC, MR, AML, AMC, AMR, ST)
    /// Returns empty dict if player not found or ratings unavailable
    ///
    /// Example:
    /// ```gdscript
    /// var ratings = DataCacheStore.get_player_position_ratings(12345)
    /// if not ratings.is_empty():
    ///     print("MC rating: ", ratings["MC"])  # e.g., 20
    /// ```
    #[func]
    pub fn get_player_position_ratings(&self, uid: i32) -> Dictionary {
        // 1. Guard: Check cache loaded
        let Some(ref index) = self.player_index else {
            return Dictionary::new();
        };

        // 2. Lookup player by UID
        let Some(person) = index.get(uid as u32) else {
            return Dictionary::new();
        };

        // 3. Parse position ratings string → [u8; 14]
        let Some(ratings) = person.get_position_ratings() else {
            return Dictionary::new();
        };

        // 4. Build dictionary with 14 positions
        let mut dict = Dictionary::new();
        dict.insert("GK", ratings[0] as i32); // Index 0
        dict.insert("DL", ratings[1] as i32); // Index 1
        dict.insert("DC", ratings[2] as i32); // Index 2
        dict.insert("DR", ratings[3] as i32); // Index 3
        dict.insert("WBL", ratings[4] as i32); // Index 4
        dict.insert("WBR", ratings[5] as i32); // Index 5
        dict.insert("DM", ratings[6] as i32); // Index 6
        dict.insert("ML", ratings[7] as i32); // Index 7
        dict.insert("MC", ratings[8] as i32); // Index 8
        dict.insert("MR", ratings[9] as i32); // Index 9
        dict.insert("AML", ratings[10] as i32); // Index 10
        dict.insert("AMC", ratings[11] as i32); // Index 11
        dict.insert("AMR", ratings[12] as i32); // Index 12
        dict.insert("ST", ratings[13] as i32); // Index 13

        dict
    }

    /// Get player's best positions (filtered by minimum rating, sorted descending)
    ///
    /// Returns: Array of Dictionaries [{position: "MC", rating: 20}, ...]
    /// Returns empty array if player not found
    ///
    /// Example:
    /// ```gdscript
    /// var best_positions = DataCacheStore.get_player_best_positions(12345, 15)
    /// for pos_dict in best_positions:
    ///     print(pos_dict["position"], ": ", pos_dict["rating"])
    /// ```
    #[func]
    pub fn get_player_best_positions(&self, uid: i32, min_rating: i32) -> Array<Variant> {
        // 1. Guard: Check cache loaded
        let Some(ref index) = self.player_index else {
            return Array::new();
        };

        // 2. Lookup player
        let Some(person) = index.get(uid as u32) else {
            return Array::new();
        };

        // 3. Use Person's built-in best_positions() method (already sorted and filtered)
        let min = min_rating.clamp(0, 20) as u8; // Clamp to valid range
        let positions = person.best_positions(min);
        let mut result = Array::new();

        // 4. Build array from filtered positions
        for (pos, rating) in positions {
            let mut dict = Dictionary::new();
            dict.insert("position", pos.name().to_string()); // "MC", "ST", etc.
            dict.insert("rating", rating as i32);
            result.push(&dict.to_variant());
        }

        result
    }

    /// Search all players by position rating (e.g., find best strikers)
    ///
    /// Returns: Array of Dictionaries [{uid, name, position_rating, ca, pa}, ...]
    /// Sorted by rating descending, limited to max_results
    ///
    /// Example:
    /// ```gdscript
    /// var best_strikers = DataCacheStore.find_best_players_for_position("ST", 15, 10)
    /// for player in best_strikers:
    ///     print(player["name"], " (", player["position_rating"], ")")
    /// ```
    #[func]
    pub fn find_best_players_for_position(
        &self,
        position: GString,
        min_rating: i32,
        max_results: i32,
    ) -> Array<Variant> {
        // 1. Guard: Check cache loaded
        let Some(ref index) = self.player_index else {
            return Array::new();
        };

        // 2. Parse position string to enum (e.g., "MC" → PositionRating::MC)
        let pos_str = position.to_string();
        let Some(pos_enum) = PositionRating::from_str(&pos_str) else {
            godot_warn!("Invalid position: {}", pos_str);
            return Array::new();
        };

        let min = min_rating.clamp(0, 20) as u8;
        let max = if max_results <= 0 {
            usize::MAX
        } else {
            max_results as usize
        };

        // 3. Collect matching players
        let mut candidates: Vec<(&Person, u8)> = Vec::new();

        for person in index.players.values() {
            if let Some(rating) = person.get_position_rating(pos_enum) {
                if rating >= min {
                    candidates.push((person, rating));
                }
            }
        }

        // 4. Sort by rating descending
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        // 5. Build result array (limit to max_results)
        let mut result = Array::new();
        for (person, rating) in candidates.iter().take(max) {
            let mut dict = Dictionary::new();
            dict.insert("uid", person.uid as i32);
            dict.insert("name", person.name.clone());
            dict.insert("position_rating", *rating as i32);
            dict.insert("ca", person.ca as i32);
            dict.insert("pa", person.pa as i32);
            dict.insert("age", person.age as i32);
            dict.insert("position", person.position.clone()); // Original position
            result.push(&dict.to_variant());
        }

        result
    }
}
