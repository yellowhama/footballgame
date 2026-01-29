use crate::engine::{ExpConfig, MatchEngine};
use std::{env, fs};

pub(crate) const EXP_CONFIG_PATH_ENV: &str = "OF_EXP_CONFIG_PATH";

pub(crate) fn apply_exp_config_from_env(engine: &mut MatchEngine) -> Result<(), String> {
    let Ok(path) = env::var(EXP_CONFIG_PATH_ENV) else {
        return Ok(());
    };

    let path = path.trim();
    if path.is_empty() {
        return Ok(());
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read exp config file from {EXP_CONFIG_PATH_ENV}='{path}': {e}"))?;

    let config = ExpConfig::from_json(&content)
        .map_err(|e| format!("Failed to parse exp config JSON from {EXP_CONFIG_PATH_ENV}='{path}': {e}"))?;

    config
        .validate()
        .map_err(|e| format!("Invalid exp config from {EXP_CONFIG_PATH_ENV}='{path}': {e}"))?;

    engine.apply_exp_config(&config);
    Ok(())
}

