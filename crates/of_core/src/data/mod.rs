//! 게임 데이터 모듈
//!
//! 바이너리에 임베딩된 기본 게임 데이터를 제공합니다.
//! - Climate coefficients (국가별 기후 계수)
//! - Game balance (포지션별 밸런스)
//! - Training efficiency (훈련 타입별 효율)
//! - League configuration (리그 설정)
//! - Rules (IFAB Laws of the Game)
//! - RuleBook UI Cards (구조화된 "왜?" 버튼 JSON payload)

pub mod embedded;
pub mod person_cache;
pub mod rules;
pub mod rulebook_ui_cards;
pub mod scale_conversion;

pub use embedded::{
    get_climate_coeffs, get_game_balance, get_league_config, get_training_efficiency, ClimateCoeff,
    GameBalance, LeagueConfig, LeagueTeam, TrainingEfficiency,
};

pub use person_cache::{
    get_person_by_uid, get_person_index, resolve_person_by_player_uid, PersonIndex,
    DEFAULT_PERSON_CACHE_REL_PATH, PERSON_CACHE_ENV,
};

pub use scale_conversion::ScaleConverter;

// RuleBook System (IFAB Laws of the Game)
pub use rules::{
    // Data loading
    get_fouls_rules, get_offside_rules,
    // Basic explanation formatting
    format_foul_explanation, format_offside_explanation,
    format_offside_exception_explanation, format_deflection_explanation,
    format_restart_explanation,
    // Phase 4: UI "Why?" Button - Full explanation generators
    generate_offside_why_explanation, generate_foul_why_explanation,
    should_show_why_button,
    // Raw YAML data
    LAW_11_YAML, LAW_12_YAML, LAW_09_10_13_17_YAML,
    EVENT_TO_RULE_MAP_YAML, EXPLAIN_TEMPLATES_YAML,
};

// RuleBook UI Card System (P1: vNext)
pub use rulebook_ui_cards::{
    generate_ui_card,
    generate_ui_card_from_match_event,
    RulebookUiCard, RulebookUiEvent, RulebookUiRule,
    CardBlock, CardLine, CardRef,
};
