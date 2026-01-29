//! 임베딩된 게임 데이터
//!
//! `include_str!` 매크로를 사용하여 컴파일 시점에 JSON 데이터를 바이너리에 포함합니다.
//! 런타임에 파일 I/O 없이 즉시 사용 가능합니다.
//!
//! ## 임베딩된 파일 (총 ~25KB)
//! - cache_climate_coeffs.v3.json (~1KB)
//! - cache_game_balance.v3.json (~661B)
//! - cache_training_efficiency.v3.json (~790B)
//! - league_config.json (~20KB)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

// ============================================================================
// 임베딩된 JSON 데이터 (컴파일 시점에 바이너리에 포함)
// ============================================================================

/// 국가별 기후 계수 JSON (~1KB)
pub const CLIMATE_COEFFS_JSON: &str =
    include_str!("../../../../data/exports/cache_climate_coeffs.v3.json");

/// 포지션별 게임 밸런스 JSON (~661B)
pub const GAME_BALANCE_JSON: &str =
    include_str!("../../../../data/exports/cache_game_balance.v3.json");

/// 훈련 타입별 효율 JSON (~790B)
pub const TRAINING_EFFICIENCY_JSON: &str =
    include_str!("../../../../data/exports/cache_training_efficiency.v3.json");

/// 리그 설정 JSON (~20KB)
pub const LEAGUE_CONFIG_JSON: &str = include_str!("../../../../data/league_config.json");

// ============================================================================
// 타입 정의
// ============================================================================

/// 국가별 기후 계수
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateCoeff {
    /// 기후 타입 (Mediterranean, Temperate, Continental, Oceanic)
    pub climate_type: String,
    /// 평균 온도 (°C)
    pub avg_temperature: f32,
    /// 평균 습도 (%)
    pub avg_humidity: f32,
    /// 평균 고도 (m)
    pub avg_altitude: f32,
    /// 체력 modifier (1.0 = 기본)
    pub stamina_modifier: f32,
    /// 부상 위험 modifier (1.0 = 기본)
    pub injury_risk_modifier: f32,
}

/// 포지션별 게임 밸런스
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameBalance {
    /// 포지션 이름 (Forward, Midfielder, Defender, Goalkeeper)
    pub position: String,
    /// 기본 급여
    pub base_salary: u32,
    /// 레이팅당 급여 증가
    pub salary_per_rating: u32,
    /// 훈련 효율 (0.0-1.0)
    pub training_efficiency: f32,
    /// 부상 위험 (0.0-1.0)
    pub injury_risk: f32,
}

/// 훈련 타입별 효율
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingEfficiency {
    /// 훈련 타입 (Technical, Physical, Tactical, Mental)
    pub training_type: String,
    /// 체력 소모량
    pub stamina_cost: u8,
    /// 기본 개선량 (0.0-1.0)
    pub base_improvement: f32,
    /// 최적 나이 최소
    pub optimal_age_min: u8,
    /// 최적 나이 최대
    pub optimal_age_max: u8,
    /// 나이에 따른 감소 계수
    pub decline_factor: f32,
}

/// 리그 내 팀 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeagueTeam {
    /// 팀 ID
    pub team_id: u32,
    /// 클럽 이름
    pub club_name: String,
    /// 평균 CA
    pub avg_ca: f32,
    /// 감독 ID
    #[serde(default)]
    pub manager_id: Option<u32>,
    /// 포메이션
    pub formation: String,
}

/// 리그 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct League {
    /// 리그 ID (1-12, 1이 최상위)
    pub league_id: u8,
    /// 리그 이름
    pub name: String,
    /// CA 범위 [min, max]
    pub ca_range: [u8; 2],
    /// 팀 풀 크기
    pub team_pool_size: u8,
    /// 리그 내 팀들
    pub teams: Vec<LeagueTeam>,
}

/// 전체 리그 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeagueConfig {
    /// 12부제 리그 목록
    pub leagues: Vec<League>,
    /// 설명
    #[serde(default)]
    pub description: Option<String>,
}

// ============================================================================
// 캐싱된 데이터 (한 번만 파싱)
// ============================================================================

static CLIMATE_COEFFS: OnceLock<HashMap<String, ClimateCoeff>> = OnceLock::new();
static GAME_BALANCE: OnceLock<HashMap<String, GameBalance>> = OnceLock::new();
static TRAINING_EFFICIENCY: OnceLock<HashMap<String, TrainingEfficiency>> = OnceLock::new();
static LEAGUE_CONFIG: OnceLock<LeagueConfig> = OnceLock::new();

// ============================================================================
// 공개 API
// ============================================================================

/// 기후 계수 맵 반환 (국가 코드 → ClimateCoeff)
///
/// 첫 호출 시 JSON 파싱, 이후 캐시된 데이터 반환
pub fn get_climate_coeffs() -> &'static HashMap<String, ClimateCoeff> {
    CLIMATE_COEFFS.get_or_init(|| {
        serde_json::from_str(CLIMATE_COEFFS_JSON)
            .expect("Embedded climate coefficients JSON is corrupted")
    })
}

/// 게임 밸런스 맵 반환 (포지션 코드 → GameBalance)
///
/// 첫 호출 시 JSON 파싱, 이후 캐시된 데이터 반환
pub fn get_game_balance() -> &'static HashMap<String, GameBalance> {
    GAME_BALANCE.get_or_init(|| {
        serde_json::from_str(GAME_BALANCE_JSON).expect("Embedded game balance JSON is corrupted")
    })
}

/// 훈련 효율 맵 반환 (훈련 타입 → TrainingEfficiency)
///
/// 첫 호출 시 JSON 파싱, 이후 캐시된 데이터 반환
pub fn get_training_efficiency() -> &'static HashMap<String, TrainingEfficiency> {
    TRAINING_EFFICIENCY.get_or_init(|| {
        serde_json::from_str(TRAINING_EFFICIENCY_JSON)
            .expect("Embedded training efficiency JSON is corrupted")
    })
}

/// 리그 설정 반환
///
/// 첫 호출 시 JSON 파싱, 이후 캐시된 데이터 반환
pub fn get_league_config() -> &'static LeagueConfig {
    LEAGUE_CONFIG.get_or_init(|| {
        serde_json::from_str(LEAGUE_CONFIG_JSON).expect("Embedded league config JSON is corrupted")
    })
}

// ============================================================================
// 편의 함수
// ============================================================================

/// 특정 국가의 기후 계수 조회
pub fn get_climate_coeff(country_code: &str) -> Option<&'static ClimateCoeff> {
    get_climate_coeffs().get(country_code)
}

/// 특정 포지션의 게임 밸런스 조회
pub fn get_position_balance(position: &str) -> Option<&'static GameBalance> {
    get_game_balance().get(position)
}

/// 특정 훈련 타입의 효율 조회
pub fn get_training_type_efficiency(training_type: &str) -> Option<&'static TrainingEfficiency> {
    get_training_efficiency().get(training_type)
}

/// 특정 리그 조회
pub fn get_league(league_id: u8) -> Option<&'static League> {
    get_league_config().leagues.iter().find(|l| l.league_id == league_id)
}

/// 팀 ID로 팀 정보 조회
pub fn get_team_by_id(team_id: u32) -> Option<(&'static League, &'static LeagueTeam)> {
    for league in &get_league_config().leagues {
        if let Some(team) = league.teams.iter().find(|t| t.team_id == team_id) {
            return Some((league, team));
        }
    }
    None
}

// ============================================================================
// 테스트
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_climate_coeffs_loaded() {
        let coeffs = get_climate_coeffs();
        assert!(!coeffs.is_empty(), "Climate coeffs should not be empty");
        assert!(coeffs.contains_key("ESP"), "Should have ESP");
        assert!(coeffs.contains_key("GBR"), "Should have GBR");

        let esp = &coeffs["ESP"];
        assert_eq!(esp.climate_type, "Mediterranean");
        assert!(esp.stamina_modifier > 0.0 && esp.stamina_modifier < 2.0);
    }

    #[test]
    fn test_game_balance_loaded() {
        let balance = get_game_balance();
        assert!(!balance.is_empty(), "Game balance should not be empty");
        assert!(balance.contains_key("FW"), "Should have FW");
        assert!(balance.contains_key("GK"), "Should have GK");

        let fw = &balance["FW"];
        assert_eq!(fw.position, "Forward");
        assert!(fw.base_salary > 0);
    }

    #[test]
    fn test_training_efficiency_loaded() {
        let efficiency = get_training_efficiency();
        assert!(!efficiency.is_empty(), "Training efficiency should not be empty");
        assert!(efficiency.contains_key("technical"), "Should have technical");
        assert!(efficiency.contains_key("physical"), "Should have physical");

        let tech = &efficiency["technical"];
        assert_eq!(tech.training_type, "Technical");
        assert!(tech.stamina_cost > 0);
    }

    #[test]
    fn test_league_config_loaded() {
        let config = get_league_config();
        assert!(!config.leagues.is_empty(), "Leagues should not be empty");

        // 12부제 리그 확인
        let league_ids: Vec<u8> = config.leagues.iter().map(|l| l.league_id).collect();
        assert!(league_ids.contains(&1), "Should have league 1 (top tier)");

        // 1부 리그 팀 확인
        let league1 = get_league(1).expect("League 1 should exist");
        assert!(!league1.teams.is_empty(), "League 1 should have teams");
    }

    #[test]
    fn test_get_team_by_id() {
        // Man Town (team_id: 174)
        let result = get_team_by_id(174);
        assert!(result.is_some(), "Should find team 174");

        let (league, team) = result.unwrap();
        assert_eq!(team.club_name, "Man Town");
        assert_eq!(league.league_id, 1); // 1부 리그
    }

    #[test]
    fn test_data_is_cached() {
        // 같은 참조를 두 번 가져와서 같은 메모리인지 확인
        let coeffs1 = get_climate_coeffs();
        let coeffs2 = get_climate_coeffs();
        assert!(std::ptr::eq(coeffs1, coeffs2), "Should return cached data");
    }
}
