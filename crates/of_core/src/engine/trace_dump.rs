//! Trace dump utilities for diagnostics and regression analysis.
//!
//! This is a lightweight JSON output that combines MatchResult with
//! Phase0 diagnostics (and optional post-match analysis).

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::engine::match_analysis::{analyze_match, MatchAnalysisReport};
use crate::engine::match_sim::DiagnosticReport;
use crate::models::MatchResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceDump {
    pub result: MatchResult,
    pub diagnostics: DiagnosticReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis: Option<MatchAnalysisReport>,
}

impl TraceDump {
    pub fn new(result: MatchResult, diagnostics: DiagnosticReport, include_analysis: bool) -> Self {
        let analysis = if include_analysis { Some(analyze_match(&result)) } else { None };
        Self { result, diagnostics, analysis }
    }

    pub fn write_json(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let payload = serde_json::to_string_pretty(self)
            .map_err(|err| format!("Failed to serialize TraceDump: {err}"))?;
        std::fs::write(path, payload).map_err(|err| format!("Failed to write TraceDump: {err}"))?;
        Ok(())
    }
}
