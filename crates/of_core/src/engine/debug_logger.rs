//! # Debug Logger (P10-13 Phase 7)
//!
//! AI 의사결정 과정을 기록하고 시각화하는 디버깅 도구.
//!
//! ## 목적
//! - "왜 저렇게 했지?" 질문에 답
//! - 버그 vs 의도된 행동 구분
//! - 밸런스 튜닝 지원
//!
//! ## 사용법
//! ```rust,ignore
//! let mut logger = DebugLogger::new(true);
//! logger.log_decision(decision_log);
//! logger.log_execution(execution_log);
//! println!("{}", logger.summary());
//! ```

use serde::Serialize;

/// Debug logger 활성화 여부 (debug 빌드에서만 활성)
#[cfg(debug_assertions)]
pub const ENABLE_DEBUG_LOGGING: bool = true;

#[cfg(not(debug_assertions))]
pub const ENABLE_DEBUG_LOGGING: bool = false;

// ========== Decision Log Structures ==========

/// 단일 의사결정 기록
#[derive(Debug, Clone, Serialize)]
pub struct DecisionLog {
    /// 결정 시점 (tick)
    pub tick: u32,
    /// 결정 시점 (분:초)
    pub match_time: String,
    /// 선수 인덱스
    pub player_idx: usize,
    /// 선수 이름
    pub player_name: String,
    /// 최종 선택
    pub chosen_action: String,
    /// 각 액션별 평가
    pub action_evaluations: Vec<ActionEvaluation>,
    /// 컨텍스트 정보
    pub context: DecisionContext,
}

/// 개별 액션 평가 기록
#[derive(Debug, Clone, Serialize)]
pub struct ActionEvaluation {
    /// 액션 종류
    pub action: String,
    /// 타겟 (패스면 receiver_idx)
    pub target: Option<usize>,
    /// Rational EV (P12)
    pub rational_ev: f32,
    /// Audacity 보너스 (P14)
    pub audacity_bonus: f32,
    /// 최종 EV
    pub final_ev: f32,
    /// 선택 여부
    pub is_chosen: bool,
    /// 상세 breakdown
    pub breakdown: EvaluationBreakdown,
}

/// EV 계산 상세
#[derive(Debug, Clone, Default, Serialize)]
pub struct EvaluationBreakdown {
    // === Shot ===
    pub xg: Option<f32>,
    pub loss_cost: Option<f32>,

    // === Pass ===
    pub pass_success_prob: Option<f32>,
    pub future_threat: Option<f32>,
    pub fail_cost: Option<f32>,
    pub interception_risk: Option<f32>,

    // === Dribble ===
    pub dribble_success_prob: Option<f32>,
    pub position_improvement: Option<f32>,

    // === Audacity ===
    pub flair: Option<f32>,
    pub audacity: Option<f32>,
    pub desperation: Option<f32>,
    pub glory_bonus: Option<f32>,
    pub risk_dampen: Option<f32>,
    pub alpha: Option<f32>,
}

/// 의사결정 컨텍스트
#[derive(Debug, Clone, Serialize)]
pub struct DecisionContext {
    /// 공 위치 (미터)
    pub ball_position: (f32, f32),
    /// 선수 위치 (미터)
    pub player_position: (f32, f32),
    /// 골대까지 거리
    pub distance_to_goal: f32,
    /// 압박 레벨 (enum 문자열)
    pub pressure_level: String,
    /// 압박 수치 (0-1) - P18: FieldBoard local_pressure
    pub pressure_value: f32,
    /// 현재 스코어 (gf, ga)
    pub score: (u8, u8),
    /// 현재 분
    pub minute: u8,
    /// 선수 체력
    pub stamina: f32,
}

// ========== Execution Log Structures ==========

/// 실행 오차 기록
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionLog {
    pub tick: u32,
    pub match_time: String,
    pub player_idx: usize,
    pub player_name: String,
    pub action: String,
    /// 의도
    pub intent: ExecutionIntent,
    /// 오차
    pub error: ExecutionErrorLog,
    /// 결과
    pub result: ExecutionResultLog,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionIntent {
    /// 의도한 목표 위치
    pub target_position: (f32, f32),
    /// 의도한 높이 (슛/크로스)
    pub target_height: Option<f32>,
    /// 의도한 파워
    pub power: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionErrorLog {
    /// 각도 오차 (도)
    pub angle_error_deg: f32,
    /// 파워 오차 (배율)
    pub power_factor: f32,
    /// 오차 magnitude
    pub magnitude: f32,

    // === 오차 원인 ===
    pub tech_skill: u8,
    pub pressure: f32,
    pub fatigue: f32,
    pub weak_foot: bool,
    pub composure: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResultLog {
    /// 실제 도착 위치
    pub actual_position: (f32, f32),
    /// 결과 타입
    pub result_type: String,
    /// 추가 정보
    pub details: Option<String>,
}

// ========== Logger ==========

/// Debug Logger (개발 모드 전용)
#[derive(Debug)]
pub struct DebugLogger {
    /// Decision logs
    decision_logs: Vec<DecisionLog>,
    /// Execution logs
    execution_logs: Vec<ExecutionLog>,
    /// 활성화 여부
    enabled: bool,
    /// 최대 로그 수 (메모리 제한)
    max_logs: usize,
}

impl DebugLogger {
    pub fn new(enabled: bool) -> Self {
        Self { decision_logs: Vec::new(), execution_logs: Vec::new(), enabled, max_logs: 10000 }
    }

    /// Decision 로그 추가
    pub fn log_decision(&mut self, log: DecisionLog) {
        if !self.enabled {
            return;
        }
        if self.decision_logs.len() >= self.max_logs {
            self.decision_logs.remove(0);
        }
        self.decision_logs.push(log);
    }

    /// Execution 로그 추가
    pub fn log_execution(&mut self, log: ExecutionLog) {
        if !self.enabled {
            return;
        }
        if self.execution_logs.len() >= self.max_logs {
            self.execution_logs.remove(0);
        }
        self.execution_logs.push(log);
    }

    /// 특정 선수의 최근 결정 가져오기
    pub fn get_recent_decisions(&self, player_idx: usize, count: usize) -> Vec<&DecisionLog> {
        self.decision_logs.iter().rev().filter(|l| l.player_idx == player_idx).take(count).collect()
    }

    /// 특정 틱의 모든 결정 가져오기
    pub fn get_decisions_at_tick(&self, tick: u32) -> Vec<&DecisionLog> {
        self.decision_logs.iter().filter(|l| l.tick == tick).collect()
    }

    /// 모든 결정 로그 가져오기
    pub fn get_all_decisions(&self) -> &[DecisionLog] {
        &self.decision_logs
    }

    /// 모든 실행 로그 가져오기
    pub fn get_all_executions(&self) -> &[ExecutionLog] {
        &self.execution_logs
    }

    /// 로그 초기화
    pub fn clear(&mut self) {
        self.decision_logs.clear();
        self.execution_logs.clear();
    }

    /// JSON으로 내보내기
    pub fn export_to_json(&self) -> Result<String, serde_json::Error> {
        let data = serde_json::json!({
            "decisions": self.decision_logs,
            "executions": self.execution_logs,
        });
        serde_json::to_string_pretty(&data)
    }

    /// 통계 요약
    pub fn summary(&self) -> LogSummary {
        let total_decisions = self.decision_logs.len();

        let shots = self.decision_logs.iter().filter(|l| l.chosen_action == "Shoot").count();
        let passes = self.decision_logs.iter().filter(|l| l.chosen_action == "Pass").count();
        let dribbles = self.decision_logs.iter().filter(|l| l.chosen_action == "Dribble").count();
        let holds = self.decision_logs.iter().filter(|l| l.chosen_action == "Hold").count();

        // Audacity 영향 분석
        let audacity_influenced = self
            .decision_logs
            .iter()
            .filter(|l| l.action_evaluations.iter().any(|e| e.audacity_bonus.abs() > 0.1))
            .count();

        // 평균 압박
        let avg_pressure = if self.decision_logs.is_empty() {
            0.0
        } else {
            self.decision_logs.iter().map(|l| l.context.pressure_value).sum::<f32>()
                / self.decision_logs.len() as f32
        };

        LogSummary {
            total_decisions,
            action_distribution: ActionDistribution { shots, passes, dribbles, holds },
            audacity_influenced_decisions: audacity_influenced,
            avg_pressure_at_decision: avg_pressure,
            total_executions: self.execution_logs.len(),
        }
    }
}

impl Default for DebugLogger {
    fn default() -> Self {
        Self::new(ENABLE_DEBUG_LOGGING)
    }
}

/// 로그 통계 요약
#[derive(Debug, Clone)]
pub struct LogSummary {
    pub total_decisions: usize,
    pub action_distribution: ActionDistribution,
    pub audacity_influenced_decisions: usize,
    pub avg_pressure_at_decision: f32,
    pub total_executions: usize,
}

#[derive(Debug, Clone)]
pub struct ActionDistribution {
    pub shots: usize,
    pub passes: usize,
    pub dribbles: usize,
    pub holds: usize,
}

impl std::fmt::Display for LogSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Debug Log Summary ===")?;
        writeln!(f, "Total Decisions: {}", self.total_decisions)?;
        writeln!(f, "Action Distribution:")?;
        writeln!(
            f,
            "  - Shots: {} ({:.1}%)",
            self.action_distribution.shots,
            if self.total_decisions > 0 {
                self.action_distribution.shots as f32 / self.total_decisions as f32 * 100.0
            } else {
                0.0
            }
        )?;
        writeln!(
            f,
            "  - Passes: {} ({:.1}%)",
            self.action_distribution.passes,
            if self.total_decisions > 0 {
                self.action_distribution.passes as f32 / self.total_decisions as f32 * 100.0
            } else {
                0.0
            }
        )?;
        writeln!(
            f,
            "  - Dribbles: {} ({:.1}%)",
            self.action_distribution.dribbles,
            if self.total_decisions > 0 {
                self.action_distribution.dribbles as f32 / self.total_decisions as f32 * 100.0
            } else {
                0.0
            }
        )?;
        writeln!(
            f,
            "  - Holds: {} ({:.1}%)",
            self.action_distribution.holds,
            if self.total_decisions > 0 {
                self.action_distribution.holds as f32 / self.total_decisions as f32 * 100.0
            } else {
                0.0
            }
        )?;
        writeln!(
            f,
            "Audacity-Influenced: {} ({:.1}%)",
            self.audacity_influenced_decisions,
            if self.total_decisions > 0 {
                self.audacity_influenced_decisions as f32 / self.total_decisions as f32 * 100.0
            } else {
                0.0
            }
        )?;
        writeln!(f, "Avg Pressure: {:.2}", self.avg_pressure_at_decision)?;
        writeln!(f, "Total Executions: {}", self.total_executions)?;
        Ok(())
    }
}

// ========== Console Output ==========

impl DecisionLog {
    /// 콘솔 친화적 출력
    pub fn to_console_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("\n=== {} @ {} ===\n", self.player_name, self.match_time));
        s.push_str(&format!(
            "Position: ({:.1}, {:.1}) | To Goal: {:.1}m\n",
            self.context.player_position.0,
            self.context.player_position.1,
            self.context.distance_to_goal
        ));
        s.push_str(&format!(
            "Local Pressure (FieldBoard): {:.2} [{}] | Stamina: {:.0}%\n",
            self.context.pressure_value,
            self.context.pressure_level,
            self.context.stamina * 100.0
        ));
        s.push_str(&format!(
            "Score: {}-{} | Minute: {}\n",
            self.context.score.0, self.context.score.1, self.context.minute
        ));
        s.push_str("\n--- Action Evaluations ---\n");

        for eval in &self.action_evaluations {
            let marker = if eval.is_chosen { ">>>" } else { "   " };
            let action_str = if let Some(target) = eval.target {
                format!("{} to #{}", eval.action, target)
            } else {
                eval.action.clone()
            };

            s.push_str(&format!(
                "{} {:12} | Rational: {:+.3} | Audacity: {:+.3} | Final: {:+.3}\n",
                marker, action_str, eval.rational_ev, eval.audacity_bonus, eval.final_ev
            ));
        }

        s
    }
}

impl ExecutionLog {
    pub fn to_console_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("\n~~~ {} EXECUTION @ {} ~~~\n", self.player_name, self.match_time));
        s.push_str(&format!("Action: {}\n", self.action));
        s.push_str(&format!(
            "Intent: ({:.1}, {:.1})",
            self.intent.target_position.0, self.intent.target_position.1
        ));
        if let Some(h) = self.intent.target_height {
            s.push_str(&format!(" @ {:.1}m height", h));
        }
        s.push('\n');

        s.push_str(&format!(
            "Error: angle {:+.1}° | power x{:.2} | mag: {:.3}\n",
            self.error.angle_error_deg, self.error.power_factor, self.error.magnitude
        ));
        s.push_str(&format!(
            "Factors: Tech {} | Pressure {:.2} | Fatigue {:.2} | WeakFoot: {}\n",
            self.error.tech_skill, self.error.pressure, self.error.fatigue, self.error.weak_foot
        ));

        s.push_str(&format!(
            "Result: ({:.1}, {:.1}) -> {}\n",
            self.result.actual_position.0, self.result.actual_position.1, self.result.result_type
        ));

        s
    }
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_decision_log() -> DecisionLog {
        DecisionLog {
            tick: 1000,
            match_time: "85:30".to_string(),
            player_idx: 9,
            player_name: "Ronaldo".to_string(),
            chosen_action: "Shoot".to_string(),
            action_evaluations: vec![
                ActionEvaluation {
                    action: "Shoot".to_string(),
                    target: None,
                    rational_ev: -0.28,
                    audacity_bonus: 0.52,
                    final_ev: 0.24,
                    is_chosen: true,
                    breakdown: EvaluationBreakdown {
                        xg: Some(0.05),
                        loss_cost: Some(0.35),
                        flair: Some(0.90),
                        audacity: Some(0.65),
                        desperation: Some(0.88),
                        ..Default::default()
                    },
                },
                ActionEvaluation {
                    action: "Pass".to_string(),
                    target: Some(10),
                    rational_ev: 0.18,
                    audacity_bonus: 0.05,
                    final_ev: 0.23,
                    is_chosen: false,
                    breakdown: Default::default(),
                },
            ],
            context: DecisionContext {
                ball_position: (78.5, 34.2),
                player_position: (78.5, 34.2),
                distance_to_goal: 26.5,
                pressure_level: "Heavy".to_string(),
                pressure_value: 0.72,
                score: (0, 1),
                minute: 85,
                stamina: 0.45,
            },
        }
    }

    #[test]
    fn test_logger_creation() {
        let logger = DebugLogger::new(true);
        assert_eq!(logger.decision_logs.len(), 0);
        assert_eq!(logger.execution_logs.len(), 0);
    }

    #[test]
    fn test_log_decision() {
        let mut logger = DebugLogger::new(true);
        let log = create_test_decision_log();

        logger.log_decision(log);
        assert_eq!(logger.decision_logs.len(), 1);
    }

    #[test]
    fn test_disabled_logger() {
        let mut logger = DebugLogger::new(false);
        let log = create_test_decision_log();

        logger.log_decision(log);
        assert_eq!(logger.decision_logs.len(), 0); // 비활성화되어 로그 안 됨
    }

    #[test]
    fn test_max_logs_limit() {
        let mut logger = DebugLogger::new(true);
        logger.max_logs = 5;

        for i in 0..10 {
            let mut log = create_test_decision_log();
            log.tick = i;
            logger.log_decision(log);
        }

        assert_eq!(logger.decision_logs.len(), 5);
        assert_eq!(logger.decision_logs[0].tick, 5); // 첫 5개 삭제됨
    }

    #[test]
    fn test_get_recent_decisions() {
        let mut logger = DebugLogger::new(true);

        for i in 0..5 {
            let mut log = create_test_decision_log();
            log.tick = i;
            log.player_idx = if i % 2 == 0 { 9 } else { 10 };
            logger.log_decision(log);
        }

        let player9_decisions = logger.get_recent_decisions(9, 10);
        assert_eq!(player9_decisions.len(), 3); // 인덱스 0, 2, 4
    }

    #[test]
    fn test_summary() {
        let mut logger = DebugLogger::new(true);

        // Shoot decision
        let mut log1 = create_test_decision_log();
        log1.chosen_action = "Shoot".to_string();
        logger.log_decision(log1);

        // Pass decision
        let mut log2 = create_test_decision_log();
        log2.chosen_action = "Pass".to_string();
        logger.log_decision(log2);

        // Dribble decision
        let mut log3 = create_test_decision_log();
        log3.chosen_action = "Dribble".to_string();
        log3.action_evaluations[0].audacity_bonus = 0.0; // No audacity influence
        logger.log_decision(log3);

        let summary = logger.summary();
        assert_eq!(summary.total_decisions, 3);
        assert_eq!(summary.action_distribution.shots, 1);
        assert_eq!(summary.action_distribution.passes, 1);
        assert_eq!(summary.action_distribution.dribbles, 1);
        assert_eq!(summary.audacity_influenced_decisions, 2); // log1, log2에 audacity 영향
    }

    #[test]
    fn test_console_output() {
        let log = create_test_decision_log();
        let output = log.to_console_string();

        assert!(output.contains("Ronaldo"));
        assert!(output.contains("85:30"));
        assert!(output.contains(">>>"));
        assert!(output.contains("Shoot"));
    }

    #[test]
    fn test_json_export() {
        let mut logger = DebugLogger::new(true);
        logger.log_decision(create_test_decision_log());

        let json = logger.export_to_json().unwrap();
        assert!(json.contains("Ronaldo"));
        assert!(json.contains("decisions"));
    }
}
