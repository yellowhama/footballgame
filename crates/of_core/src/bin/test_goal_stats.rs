// Comprehensive match statistics test
// Run with: cargo run --bin test_goal_stats --release
//
// 목적: 양팀 분리 상세 통계 수집 (v11)

use of_core::api::simulate_match_json;
use serde_json::Value;

fn create_test_plan(seed: u64) -> String {
    format!(
        r#"{{
        "schema_version": 1,
        "seed": {},
        "home_team": {{
            "name": "Test Home FC",
            "formation": "4-4-2",
            "players": [
                {{"name": "GK1", "position": "GK", "overall": 75}},
                {{"name": "GK2", "position": "GK", "overall": 70}},
                {{"name": "LB", "position": "LB", "overall": 72}},
                {{"name": "CB1", "position": "CB", "overall": 74}},
                {{"name": "CB2", "position": "CB", "overall": 73}},
                {{"name": "RB", "position": "RB", "overall": 71}},
                {{"name": "LM", "position": "LM", "overall": 76}},
                {{"name": "CM1", "position": "CM", "overall": 78}},
                {{"name": "CM2", "position": "CM", "overall": 75}},
                {{"name": "RM", "position": "RM", "overall": 74}},
                {{"name": "ST1", "position": "ST", "overall": 82}},
                {{"name": "ST2", "position": "ST", "overall": 80}},
                {{"name": "Sub1", "position": "GK", "overall": 68}},
                {{"name": "Sub2", "position": "CB", "overall": 70}},
                {{"name": "Sub3", "position": "CM", "overall": 70}},
                {{"name": "Sub4", "position": "CM", "overall": 69}},
                {{"name": "Sub5", "position": "ST", "overall": 72}},
                {{"name": "Sub6", "position": "ST", "overall": 71}}
            ]
        }},
        "away_team": {{
            "name": "Test Away FC",
            "formation": "4-3-3",
            "players": [
                {{"name": "AGK1", "position": "GK", "overall": 73}},
                {{"name": "AGK2", "position": "GK", "overall": 68}},
                {{"name": "ALB", "position": "LB", "overall": 70}},
                {{"name": "ACB1", "position": "CB", "overall": 72}},
                {{"name": "ACB2", "position": "CB", "overall": 71}},
                {{"name": "ARB", "position": "RB", "overall": 69}},
                {{"name": "ACM1", "position": "CM", "overall": 74}},
                {{"name": "ACM2", "position": "CM", "overall": 73}},
                {{"name": "ACM3", "position": "CM", "overall": 72}},
                {{"name": "ALW", "position": "LW", "overall": 76}},
                {{"name": "AST", "position": "ST", "overall": 78}},
                {{"name": "ARW", "position": "RW", "overall": 75}},
                {{"name": "ASub1", "position": "GK", "overall": 65}},
                {{"name": "ASub2", "position": "CB", "overall": 68}},
                {{"name": "ASub3", "position": "CM", "overall": 68}},
                {{"name": "ASub4", "position": "CM", "overall": 67}},
                {{"name": "ASub5", "position": "ST", "overall": 70}},
                {{"name": "ASub6", "position": "ST", "overall": 69}}
            ]
        }},
        "match_config": {{
            "duration_seconds": 90.0,
            "budget_ops": 50000
        }}
    }}"#,
        seed
    )
}

/// 팀별 상세 통계
#[derive(Default, Clone)]
struct TeamStats {
    goals: u32,
    shots: u32,
    shots_on_target: u32,
    tackles: u32,
    tackle_attempts: u32,
    passes: u32,
    pass_attempts: u32,
    corners: u32,
    freekicks: u32,
    headers: u32,
    header_attempts: u32,
    take_ons: u32,         // 돌파 성공
    take_on_attempts: u32, // 돌파 시도
    possession: f32,
    xg: f32,
}

/// 매치 전체 통계
struct MatchStats {
    home: TeamStats,
    away: TeamStats,
}

fn get_match_stats(result_json: &str) -> MatchStats {
    let parsed: Value = serde_json::from_str(result_json).expect("Failed to parse result");

    let mut home = TeamStats::default();
    let mut away = TeamStats::default();

    // 스코어
    home.goals = parsed["score_home"].as_u64().unwrap_or(0) as u32;
    away.goals = parsed["score_away"].as_u64().unwrap_or(0) as u32;

    // Statistics 객체에서 읽기
    let stats = &parsed["statistics"];

    // 슛
    home.shots = stats["shots_home"].as_u64().unwrap_or(0) as u32;
    away.shots = stats["shots_away"].as_u64().unwrap_or(0) as u32;
    home.shots_on_target = stats["shots_on_target_home"].as_u64().unwrap_or(0) as u32;
    away.shots_on_target = stats["shots_on_target_away"].as_u64().unwrap_or(0) as u32;

    // 태클
    home.tackles = stats["tackles_home"].as_u64().unwrap_or(0) as u32;
    away.tackles = stats["tackles_away"].as_u64().unwrap_or(0) as u32;
    home.tackle_attempts = stats["tackle_attempts_home"].as_u64().unwrap_or(0) as u32;
    away.tackle_attempts = stats["tackle_attempts_away"].as_u64().unwrap_or(0) as u32;

    // 패스
    home.passes = stats["passes_home"].as_u64().unwrap_or(0) as u32;
    away.passes = stats["passes_away"].as_u64().unwrap_or(0) as u32;
    home.pass_attempts = stats["pass_attempts_home"].as_u64().unwrap_or(0) as u32;
    away.pass_attempts = stats["pass_attempts_away"].as_u64().unwrap_or(0) as u32;

    // 코너킥
    home.corners = stats["corners_home"].as_u64().unwrap_or(0) as u32;
    away.corners = stats["corners_away"].as_u64().unwrap_or(0) as u32;

    // 프리킥
    home.freekicks = stats["freekicks_home"].as_u64().unwrap_or(0) as u32;
    away.freekicks = stats["freekicks_away"].as_u64().unwrap_or(0) as u32;

    // 헤딩
    home.headers = stats["headers_home"].as_u64().unwrap_or(0) as u32;
    away.headers = stats["headers_away"].as_u64().unwrap_or(0) as u32;
    home.header_attempts = stats["header_attempts_home"].as_u64().unwrap_or(0) as u32;
    away.header_attempts = stats["header_attempts_away"].as_u64().unwrap_or(0) as u32;

    // 돌파 (TakeOn)
    home.take_ons = stats["take_ons_home"].as_u64().unwrap_or(0) as u32;
    away.take_ons = stats["take_ons_away"].as_u64().unwrap_or(0) as u32;
    home.take_on_attempts = stats["take_on_attempts_home"].as_u64().unwrap_or(0) as u32;
    away.take_on_attempts = stats["take_on_attempts_away"].as_u64().unwrap_or(0) as u32;

    // 점유율
    home.possession = stats["possession_home"].as_f64().unwrap_or(50.0) as f32;
    away.possession = stats["possession_away"].as_f64().unwrap_or(50.0) as f32;

    // xG
    home.xg = stats["xg_home"].as_f64().unwrap_or(0.0) as f32;
    away.xg = stats["xg_away"].as_f64().unwrap_or(0.0) as f32;

    // 이벤트에서 추가 통계 수집 (통계 객체에 없는 경우)
    if let Some(events) = parsed["events"].as_array() {
        for event in events {
            if let Some(event_type) = event["type"].as_str() {
                let is_home = event["is_home_team"].as_bool().unwrap_or(false);

                // 슛 (이벤트 기반 - 통계가 0이면 이벤트에서 수집)
                if home.shots == 0
                    && away.shots == 0
                    && (event_type.starts_with("shot") || event_type == "goal")
                {
                    if is_home {
                        home.shots += 1;
                    } else {
                        away.shots += 1;
                    }
                }

                // 태클 시도 (이벤트 기반)
                if home.tackle_attempts == 0
                    && away.tackle_attempts == 0
                    && event_type.contains("tackle")
                {
                    if is_home {
                        home.tackle_attempts += 1;
                    } else {
                        away.tackle_attempts += 1;
                    }
                }
            }
        }
    }

    MatchStats { home, away }
}

/// 통계 누적
struct AggregatedStats {
    home: TeamStats,
    away: TeamStats,
    match_count: u32,
}

impl AggregatedStats {
    fn new() -> Self {
        Self { home: TeamStats::default(), away: TeamStats::default(), match_count: 0 }
    }

    fn add(&mut self, stats: MatchStats) {
        self.match_count += 1;

        // Home
        self.home.goals += stats.home.goals;
        self.home.shots += stats.home.shots;
        self.home.shots_on_target += stats.home.shots_on_target;
        self.home.tackles += stats.home.tackles;
        self.home.tackle_attempts += stats.home.tackle_attempts;
        self.home.passes += stats.home.passes;
        self.home.pass_attempts += stats.home.pass_attempts;
        self.home.corners += stats.home.corners;
        self.home.freekicks += stats.home.freekicks;
        self.home.headers += stats.home.headers;
        self.home.header_attempts += stats.home.header_attempts;
        self.home.take_ons += stats.home.take_ons;
        self.home.take_on_attempts += stats.home.take_on_attempts;
        self.home.possession += stats.home.possession;
        self.home.xg += stats.home.xg;

        // Away
        self.away.goals += stats.away.goals;
        self.away.shots += stats.away.shots;
        self.away.shots_on_target += stats.away.shots_on_target;
        self.away.tackles += stats.away.tackles;
        self.away.tackle_attempts += stats.away.tackle_attempts;
        self.away.passes += stats.away.passes;
        self.away.pass_attempts += stats.away.pass_attempts;
        self.away.corners += stats.away.corners;
        self.away.freekicks += stats.away.freekicks;
        self.away.headers += stats.away.headers;
        self.away.header_attempts += stats.away.header_attempts;
        self.away.take_ons += stats.away.take_ons;
        self.away.take_on_attempts += stats.away.take_on_attempts;
        self.away.possession += stats.away.possession;
        self.away.xg += stats.away.xg;
    }

    fn avg(&self, val: u32) -> f64 {
        if self.match_count == 0 {
            0.0
        } else {
            val as f64 / self.match_count as f64
        }
    }

    fn avg_f32(&self, val: f32) -> f64 {
        if self.match_count == 0 {
            0.0
        } else {
            val as f64 / self.match_count as f64
        }
    }
}

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║            Comprehensive Match Statistics Test (v11)                   ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝\n");

    let num_matches = 50;
    let mut agg = AggregatedStats::new();

    println!("Running {} matches...", num_matches);

    for i in 0..num_matches {
        let seed = 1000 + i as u64;
        let plan = create_test_plan(seed);

        match simulate_match_json(&plan) {
            Ok(result) => {
                // 첫 번째 경기 통계 JSON 출력 (디버그용)
                if i == 0 {
                    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
                    let stats = &parsed["statistics"];
                    eprintln!("\n=== First Match Statistics (raw JSON) ===");
                    eprintln!("pass_attempts_home: {:?}", stats["pass_attempts_home"]);
                    eprintln!("pass_attempts_away: {:?}", stats["pass_attempts_away"]);
                    eprintln!("passes_home: {:?}", stats["passes_home"]);
                    eprintln!("passes_away: {:?}", stats["passes_away"]);
                    eprintln!("shots_home: {:?}", stats["shots_home"]);
                    eprintln!("shots_on_target_home: {:?}", stats["shots_on_target_home"]);
                    eprintln!("tackle_attempts_home: {:?}", stats["tackle_attempts_home"]);
                    eprintln!("corners_home: {:?}", stats["corners_home"]);
                    eprintln!("freekicks_home: {:?}", stats["freekicks_home"]);
                    eprintln!("header_attempts_home: {:?}", stats["header_attempts_home"]);
                    eprintln!("take_on_attempts_home: {:?}", stats["take_on_attempts_home"]);
                    eprintln!("==========================================\n");
                }

                let stats = get_match_stats(&result);
                agg.add(stats);

                if (i + 1) % 10 == 0 {
                    print!(".");
                    use std::io::Write;
                    std::io::stdout().flush().unwrap();
                }
            }
            Err(e) => {
                eprintln!("\nMatch {} failed: {:?}", i, e);
            }
        }
    }

    println!("\n");

    if agg.match_count == 0 {
        println!("No successful matches!");
        return;
    }

    // 헤더
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "📊 Results ({} matches)                                HOME    AWAY    TOTAL",
        agg.match_count
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 골
    let h_goals = agg.avg(agg.home.goals);
    let a_goals = agg.avg(agg.away.goals);
    println!(
        "  Goals                                              {:>6.2}  {:>6.2}  {:>6.2}",
        h_goals,
        a_goals,
        h_goals + a_goals
    );

    // 슛
    let h_shots = agg.avg(agg.home.shots);
    let a_shots = agg.avg(agg.away.shots);
    println!(
        "  Shots                                              {:>6.2}  {:>6.2}  {:>6.2}",
        h_shots,
        a_shots,
        h_shots + a_shots
    );

    // 유효슛
    let h_sot = agg.avg(agg.home.shots_on_target);
    let a_sot = agg.avg(agg.away.shots_on_target);
    println!(
        "  Shots on Target                                    {:>6.2}  {:>6.2}  {:>6.2}",
        h_sot,
        a_sot,
        h_sot + a_sot
    );

    // 태클 성공
    let h_tackles = agg.avg(agg.home.tackles);
    let a_tackles = agg.avg(agg.away.tackles);
    println!(
        "  Tackles (won)                                      {:>6.2}  {:>6.2}  {:>6.2}",
        h_tackles,
        a_tackles,
        h_tackles + a_tackles
    );

    // 태클 시도
    let h_tackle_att = agg.avg(agg.home.tackle_attempts);
    let a_tackle_att = agg.avg(agg.away.tackle_attempts);
    println!(
        "  Tackle Attempts                                    {:>6.2}  {:>6.2}  {:>6.2}",
        h_tackle_att,
        a_tackle_att,
        h_tackle_att + a_tackle_att
    );

    // 패스 성공
    let h_passes = agg.avg(agg.home.passes);
    let a_passes = agg.avg(agg.away.passes);
    println!(
        "  Passes (completed)                                 {:>6.2}  {:>6.2}  {:>6.2}",
        h_passes,
        a_passes,
        h_passes + a_passes
    );

    // 패스 시도
    let h_pass_att = agg.avg(agg.home.pass_attempts);
    let a_pass_att = agg.avg(agg.away.pass_attempts);
    println!(
        "  Pass Attempts                                      {:>6.2}  {:>6.2}  {:>6.2}",
        h_pass_att,
        a_pass_att,
        h_pass_att + a_pass_att
    );

    // 코너킥
    let h_corners = agg.avg(agg.home.corners);
    let a_corners = agg.avg(agg.away.corners);
    println!(
        "  Corners                                            {:>6.2}  {:>6.2}  {:>6.2}",
        h_corners,
        a_corners,
        h_corners + a_corners
    );

    // 프리킥
    let h_fk = agg.avg(agg.home.freekicks);
    let a_fk = agg.avg(agg.away.freekicks);
    println!(
        "  Free Kicks                                         {:>6.2}  {:>6.2}  {:>6.2}",
        h_fk,
        a_fk,
        h_fk + a_fk
    );

    // 헤딩 성공
    let h_headers = agg.avg(agg.home.headers);
    let a_headers = agg.avg(agg.away.headers);
    println!(
        "  Headers (won)                                      {:>6.2}  {:>6.2}  {:>6.2}",
        h_headers,
        a_headers,
        h_headers + a_headers
    );

    // 헤딩 시도
    let h_header_att = agg.avg(agg.home.header_attempts);
    let a_header_att = agg.avg(agg.away.header_attempts);
    println!(
        "  Header Attempts                                    {:>6.2}  {:>6.2}  {:>6.2}",
        h_header_att,
        a_header_att,
        h_header_att + a_header_att
    );

    // 돌파 성공
    let h_takeons = agg.avg(agg.home.take_ons);
    let a_takeons = agg.avg(agg.away.take_ons);
    println!(
        "  Take-Ons (success)                                 {:>6.2}  {:>6.2}  {:>6.2}",
        h_takeons,
        a_takeons,
        h_takeons + a_takeons
    );

    // 돌파 시도
    let h_takeon_att = agg.avg(agg.home.take_on_attempts);
    let a_takeon_att = agg.avg(agg.away.take_on_attempts);
    println!(
        "  Take-On Attempts                                   {:>6.2}  {:>6.2}  {:>6.2}",
        h_takeon_att,
        a_takeon_att,
        h_takeon_att + a_takeon_att
    );

    // 점유율
    let h_poss = agg.avg_f32(agg.home.possession);
    let a_poss = agg.avg_f32(agg.away.possession);
    println!(
        "  Possession (%)                                     {:>6.2}  {:>6.2}  {:>6.2}",
        h_poss,
        a_poss,
        h_poss + a_poss
    );

    // xG
    let h_xg = agg.avg_f32(agg.home.xg);
    let a_xg = agg.avg_f32(agg.away.xg);
    println!(
        "  xG                                                 {:>6.2}  {:>6.2}  {:>6.2}",
        h_xg,
        a_xg,
        h_xg + a_xg
    );

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 목표 체크
    println!("\n📋 Target Validation:");
    let total_goals = h_goals + a_goals;
    let total_shots = h_shots + a_shots;
    let total_tackles = h_tackles + a_tackles;
    let total_passes = h_passes + a_passes;
    let total_xg = h_xg + a_xg;

    let goals_ok = (1.5..=4.0).contains(&total_goals);
    let shots_ok = (15.0..=30.0).contains(&total_shots);
    let tackles_ok = (20.0..=60.0).contains(&total_tackles);
    let passes_ok = (700.0..=1000.0).contains(&total_passes);
    let poss_ok = (45.0..=55.0).contains(&h_poss);
    let xg_ok = (1.5..=3.5).contains(&total_xg);

    println!(
        "  Goals:      {} ({:.2}, 목표: 1.5-4.0)",
        if goals_ok { "✅" } else { "⚠️" },
        total_goals
    );
    println!(
        "  Shots:      {} ({:.2}, 목표: 15-30)",
        if shots_ok { "✅" } else { "⚠️" },
        total_shots
    );
    println!(
        "  Tackles:    {} ({:.2}, 목표: 20-60)",
        if tackles_ok { "✅" } else { "⚠️" },
        total_tackles
    );
    println!(
        "  Passes:     {} ({:.2}, 목표: 700-1000)",
        if passes_ok { "✅" } else { "⚠️" },
        total_passes
    );
    println!("  Possession: {} ({:.2}%, 목표: 45-55%)", if poss_ok { "✅" } else { "⚠️" }, h_poss);
    println!("  xG:         {} ({:.2}, 목표: 1.5-3.5)", if xg_ok { "✅" } else { "⚠️" }, total_xg);

    let all_ok = goals_ok && shots_ok && tackles_ok && passes_ok && poss_ok && xg_ok;
    if all_ok {
        println!("\n  🎉 All targets met!");
    }

    println!();
}
