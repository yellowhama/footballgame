/// action_scoring_types.rs
/// ACTION_SCORING_SSOT.yaml 구조체 정의 (serde_yaml)
use serde::Deserialize;
use std::collections::HashMap;

/// 루트 구조체
#[derive(Debug, Clone, Deserialize)]
pub struct ActionScoringSSOT {
    pub version: u32,

    pub stat_scale: StatScale,
    pub normalize: NormalizeSpec,
    pub curves: CurvesSpec,

    pub human_peaks: HumanPeaks,

    pub situational_factors: HashMap<String, FactorRangeSpec>,

    pub action_model: ActionModelSpec,

    /// reusable score blocks (예: scores.passing.intent)
    pub scores: HashMap<String, HashMap<String, ScoreSpec>>,

    /// action definitions (예: actions.pass)
    pub actions: HashMap<String, ActionSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatScale {
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NormalizeSpec {
    pub method: String, // "linear"
    pub clamp: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurvesSpec {
    pub default_gamma: f32,
    pub by_action_override: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HumanPeaks {
    pub player: HumanPlayerPeaks,
    pub ball: HumanBallPeaks,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HumanPlayerPeaks {
    pub v_max_mps: PeakRange,
    pub t_to_vmax_s: PeakRange,
    pub turn_rate_deg_s: PeakRange,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HumanBallPeaks {
    pub kick_speed_mps: PeakRange,
    pub spin_quality: PeakRange,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PeakRange {
    pub min: f32,
    pub max: f32,
    #[serde(default)]
    pub gamma: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FactorRangeSpec {
    pub range: [f32; 2], // [min, max]
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionModelSpec {
    pub globals: ActionModelGlobals,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionModelGlobals {
    pub quality_clamp: [f32; 2],
    pub prob_link: ProbLinkSpec,
    pub error_link: ErrorLinkSpec,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProbLinkSpec {
    pub steep: f32,
    pub mid: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorLinkSpec {
    pub k: f32,
}

/// score spec: stats weights + gamma
#[derive(Debug, Clone, Deserialize)]
pub struct ScoreSpec {
    pub stats: HashMap<String, f32>, // stat_name → weight
    #[serde(default)]
    pub gamma: Option<f32>,
}

/// For ball_outputs.kick_speed_mps.score_combo
#[derive(Debug, Clone, Deserialize)]
pub struct ScoreComboSpec {
    pub stats: HashMap<String, f32>,
    #[serde(default)]
    pub gamma: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionSpec {
    /// 예: move_player는 outputs에 peak 매핑이 있음
    #[serde(default)]
    pub outputs: Option<HashMap<String, OutputSpec>>,

    /// 예: pass/shot/receive는 intent_quality/execution_quality가 있음
    #[serde(default)]
    pub intent_quality: Option<QualitySpec>,
    #[serde(default)]
    pub execution_quality: Option<QualitySpec>,

    /// 공 출력(킥 속도/오차 등)
    #[serde(default)]
    pub ball_outputs: Option<BallOutputsSpec>,

    /// 결과 확률/contest 입력
    #[serde(default)]
    pub outcome: Option<OutcomeSpec>,

    /// move_player 같은 경우 (situational_effects)
    #[serde(default)]
    pub situational_effects: Option<HashMap<String, FormulaSpec>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutputSpec {
    pub from_peak: String, // "human_peaks.player.v_max_mps" 같은 path
    #[serde(default)]
    pub score: Option<String>, // "movement.top_speed"
}

#[derive(Debug, Clone, Deserialize)]
pub struct QualitySpec {
    pub score: String, // "passing.intent"
    #[serde(default)]
    pub situational: Option<SituationalSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SituationalSpec {
    #[serde(default)]
    pub penalties: Option<HashMap<String, f32>>,
    #[serde(default)]
    pub bonuses: Option<HashMap<String, f32>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BallOutputsSpec {
    #[serde(default)]
    pub kick_speed_mps: Option<KickSpeedSpec>,
    #[serde(default)]
    pub error: Option<ErrorSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KickSpeedSpec {
    pub from_peak: String, // "human_peaks.ball.kick_speed_mps"
    #[serde(default)]
    pub score_combo: Option<ScoreComboSpec>,
    #[serde(default)]
    pub score: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorSpec {
    pub base_error_m: f32,
    pub scale_by: String, // "action_model.globals.error_link"
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutcomeSpec {
    #[serde(default)]
    pub success_prob: Option<ProbSpec>,
    #[serde(default)]
    pub on_target_prob: Option<ProbSpec>,
    #[serde(default)]
    pub control_prob: Option<ProbSpec>,
    #[serde(default)]
    pub win_ball_prob: Option<ProbSpec>,
    #[serde(default)]
    pub cut_lane_prob: Option<ProbSpec>,

    #[serde(default)]
    pub contest_inputs: Option<HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProbSpec {
    pub link: String,                      // "action_model.globals.prob_link"
    pub quality_mix: HashMap<String, f32>, // {"intent":0.45, "execution":0.55}
}

/// YAML에서 formula: "1.0 - 0.35*fatigue" 같은 것
/// MVP에서는 파서 없이 코드로 처리 권장
#[derive(Debug, Clone, Deserialize)]
pub struct FormulaSpec {
    pub formula: String,
}

/// SSOT 로딩
pub fn load_action_scoring_ssot(yaml_str: &str) -> Result<ActionScoringSSOT, serde_yaml::Error> {
    serde_yaml::from_str::<ActionScoringSSOT>(yaml_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_minimal_yaml() {
        let yaml = r#"
version: 1
stat_scale:
  min: 1
  max: 100
normalize:
  method: "linear"
  clamp: true
curves:
  default_gamma: 0.85
  by_action_override: false
human_peaks:
  player:
    v_max_mps:
      min: 5.0
      max: 10.44
      gamma: 0.75
    t_to_vmax_s:
      min: 1.2
      max: 3.0
      gamma: 0.85
    turn_rate_deg_s:
      min: 90.0
      max: 360.0
      gamma: 0.90
  ball:
    kick_speed_mps:
      min: 12.0
      max: 32.0
      gamma: 0.80
    spin_quality:
      min: 0.0
      max: 1.0
situational_factors:
  pressure:
    range: [0.0, 1.0]
  fatigue:
    range: [0.0, 1.0]
action_model:
  globals:
    quality_clamp: [0.0, 1.0]
    prob_link:
      steep: 6.0
      mid: 0.5
    error_link:
      k: 2.0
scores: {}
actions: {}
"#;

        let ssot = load_action_scoring_ssot(yaml).unwrap();
        assert_eq!(ssot.version, 1);
        assert_eq!(ssot.stat_scale.min, 1);
        assert_eq!(ssot.stat_scale.max, 100);
        assert_eq!(ssot.human_peaks.player.v_max_mps.min, 5.0);
        assert_eq!(ssot.human_peaks.player.v_max_mps.max, 10.44);
        assert_eq!(ssot.action_model.globals.prob_link.steep, 6.0);
    }
}
