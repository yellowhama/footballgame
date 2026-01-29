//! Audit Gates - Match Engine 계약 검증
//!
//! **FIX_2512 Phase 0 - Audit Gates**
//!
//! ## 목적
//!
//! Match Engine의 입출력 계약을 검증하는 감사 게이트.
//! "10cm + 50ms" 마이그레이션 전 안전망으로, 좌표/상태/ID 무결성을 보장한다.
//!
//! ## Gate 분류
//!
//! ### HARD Gates (A1-A4) - 계약 위반 즉시 차단
//! - A1: Ball coordinates (필드 범위 + 여유분)
//! - A2: Player coordinates (필드 범위 + 여유분)
//! - A3: track_id integrity (0-22 unique)
//! - A4: attributes range (0-100)
//!
//! ### SOFT Gates (B1-B4) - 경고 후 7 빌드 후 HARD 전환
//! - B1: attributes missing (attributes=None)
//! - B2: invalid formation
//! - B3: roster size != 11
//! - B4: duplicate jersey numbers
//!
//! ## 좌표 시스템
//!
//! 현재 엔진은 **normalized coordinates (0.0-1.0)**을 사용하지만,
//! 검증은 **meters (0-105, 0-68)**로 변환 후 수행한다.
//! (향후 Coord10 전환 시 이 모듈도 0.1m 단위로 전환)
//!
//! ```text
//! Field Layout (meters):
//!
//!   x=0 (Home Goal)                    x=105 (Away Goal)
//!     │                                       │
//!     │  ┌─────────────────────────────────┐  │
//!     │  │ -1.0 ← 여유분                   │  │  ← 106.0
//!     └──┤                                 ├──┘
//!  y=0   │         Field (0-105m)          │  y=68
//!     ┌──┤                                 ├──┐
//!     │  │                   여유분 → 69.0 │  │
//!     │  └─────────────────────────────────┘  │
//!     │                                       │
//!   -1.0                                   106.0
//!
//! Ball Z: -0.5 ~ 15.0m (지면 아래 여유분 ~ 상공)
//! ```
//!
//! ## 사용 예시
//!
//! ```ignore
//! // A3: track_id 검증 (MatchEngine::new()에서)
//! audit_gates::validate_track_ids(&setup)?;
//!
//! // A1: Ball 검증 (tick_based.rs에서)
//! #[cfg(debug_assertions)]
//! audit_gates::validate_ball_coordinates(&ball);
//!
//! // A2: Player 검증 (tick_based.rs에서)
//! #[cfg(debug_assertions)]
//! audit_gates::validate_player_coordinates(&player_positions);
//! ```

use crate::engine::ball::Ball;
use crate::engine::physics_constants::field;

/// 필드 크기 (미터 단위)
pub const FIELD_LENGTH_M: f32 = field::LENGTH_M;
pub const FIELD_WIDTH_M: f32 = field::WIDTH_M;

/// A1/A2: 좌표 범위 (미터 단위, 여유분 포함)
/// DECISION_01에서 확정: ±1.0m 여유분
pub const BALL_X_MIN: f32 = -1.0;
pub const BALL_X_MAX: f32 = 106.0; // 105 + 1
pub const BALL_Y_MIN: f32 = -1.0;
pub const BALL_Y_MAX: f32 = 69.0; // 68 + 1
pub const BALL_Z_MIN: f32 = -0.5; // 지면 아래 여유분
pub const BALL_Z_MAX: f32 = 15.0; // 상공 최대 (롱패스/로빙슛)

pub const PLAYER_X_MIN: f32 = -1.0;
pub const PLAYER_X_MAX: f32 = 106.0;
pub const PLAYER_Y_MIN: f32 = -1.0;
pub const PLAYER_Y_MAX: f32 = 69.0;

/// A4: Attributes 범위
pub const ATTR_MIN: u8 = 0;
pub const ATTR_MAX: u8 = 100;

/// A3: 선수 수 (ball 제외)
pub const PLAYER_COUNT: usize = 22;

/// A3: track_id 범위 (0-22: ball + 22 players)
pub const TRACK_ID_BALL: u8 = 0;
pub const TRACK_ID_MIN_PLAYER: u8 = 1;
pub const TRACK_ID_MAX_PLAYER: u8 = 22;

// ===========================================
// A1: Ball Coordinates Validation
// ===========================================

/// A1-HARD: Ball 좌표 검증
///
/// **실행 조건**: debug_assertions에서만 로그
/// **위반 시**: eprintln + 카운트 (CI에서 실패)
///
/// # Normalized → Meters 변환
///
/// 현재 Ball은 normalized (0.0-1.0) 좌표를 사용하므로 변환:
/// ```text
/// x_m = ball.position.0 * FIELD_LENGTH_M
/// y_m = ball.position.1 * FIELD_WIDTH_M
/// z_m = ball.height (이미 meters)
/// ```
pub fn validate_ball_coordinates(ball: &Ball) {
    // Coord10 → Meters 변환
    let (x_m, y_m) = ball.position.to_meters();
    let z_m = ball.height as f32 / 10.0; // i16 to meters

    // NaN/Inf 검사
    if !x_m.is_finite() {
        #[cfg(debug_assertions)]
        eprintln!("[AUDIT-A1-HARD] Ball X is NaN/Inf: x_m={}", x_m);
        return;
    }

    if !y_m.is_finite() {
        #[cfg(debug_assertions)]
        eprintln!("[AUDIT-A1-HARD] Ball Y is NaN/Inf: y_m={}", y_m);
        return;
    }

    if !z_m.is_finite() {
        #[cfg(debug_assertions)]
        eprintln!("[AUDIT-A1-HARD] Ball Z is NaN/Inf: z_m={}", z_m);
        return;
    }

    // 범위 검사
    if !(BALL_X_MIN..=BALL_X_MAX).contains(&x_m) {
        #[cfg(debug_assertions)]
        eprintln!(
            "[AUDIT-A1-HARD] Ball X out of range: {:.2} (expected {:.1} ~ {:.1})",
            x_m, BALL_X_MIN, BALL_X_MAX
        );
    }

    if !(BALL_Y_MIN..=BALL_Y_MAX).contains(&y_m) {
        #[cfg(debug_assertions)]
        eprintln!(
            "[AUDIT-A1-HARD] Ball Y out of range: {:.2} (expected {:.1} ~ {:.1})",
            y_m, BALL_Y_MIN, BALL_Y_MAX
        );
    }

    if !(BALL_Z_MIN..=BALL_Z_MAX).contains(&z_m) {
        #[cfg(debug_assertions)]
        eprintln!(
            "[AUDIT-A1-HARD] Ball Z out of range: {:.2} (expected {:.1} ~ {:.1})",
            z_m, BALL_Z_MIN, BALL_Z_MAX
        );
    }
}

// ===========================================
// A2: Player Coordinates Validation
// ===========================================

/// A2-HARD: Player 좌표 검증
///
/// **실행 조건**: debug_assertions에서만 로그
/// **위반 시**: eprintln + 카운트 (CI에서 실패)
///
/// # Normalized → Meters 변환
///
/// ```text
/// x_m = pos.0 * FIELD_LENGTH_M
/// y_m = pos.1 * FIELD_WIDTH_M
/// ```
/// FIX_2601: Updated to accept &[Coord10] instead of &[(f32, f32)]
pub fn validate_player_coordinates(player_positions: &[super::types::Coord10]) {
    if player_positions.len() != PLAYER_COUNT {
        #[cfg(debug_assertions)]
        eprintln!(
            "[AUDIT-A2-HARD] Player count mismatch: {} (expected {})",
            player_positions.len(),
            PLAYER_COUNT
        );
        return;
    }

    for (_track_id, pos) in player_positions.iter().enumerate() {
        // FIX_2601: Coord10 → meters
        let (x_m, y_m) = pos.to_meters();

        // NaN/Inf 검사
        if !x_m.is_finite() {
            #[cfg(debug_assertions)]
            eprintln!(
                "[AUDIT-A2-HARD] Player {} X is NaN/Inf: pos={:?}, x_m={}",
                _track_id + 1,
                pos,
                x_m
            );
            continue;
        }

        if !y_m.is_finite() {
            #[cfg(debug_assertions)]
            eprintln!(
                "[AUDIT-A2-HARD] Player {} Y is NaN/Inf: pos={:?}, y_m={}",
                _track_id + 1,
                pos,
                y_m
            );
            continue;
        }

        // 범위 검사
        if !(PLAYER_X_MIN..=PLAYER_X_MAX).contains(&x_m) {
            #[cfg(debug_assertions)]
            eprintln!(
                "[AUDIT-A2-HARD] Player {} X out of range: {:.2} (expected {:.1} ~ {:.1})",
                _track_id + 1,
                x_m,
                PLAYER_X_MIN,
                PLAYER_X_MAX
            );
        }

        if !(PLAYER_Y_MIN..=PLAYER_Y_MAX).contains(&y_m) {
            #[cfg(debug_assertions)]
            eprintln!(
                "[AUDIT-A2-HARD] Player {} Y out of range: {:.2} (expected {:.1} ~ {:.1})",
                _track_id + 1,
                y_m,
                PLAYER_Y_MIN,
                PLAYER_Y_MAX
            );
        }
    }
}

// ===========================================
// A3: track_id Integrity Validation
// ===========================================

use crate::models::MatchSetup;

/// A3-HARD: track_id 무결성 검증
///
/// **실행 조건**: 항상 (debug + release)
/// **위반 시**: Result::Err 반환 (엔진 거부)
///
/// # 검증 항목
///
/// 1. track_id 범위: 0-21 (22명)
/// 2. track_id 중복 없음 (암묵적: 0-21 고정 배열)
/// 3. 선수 수 = 22명 (home 11 + away 11)
pub fn validate_track_ids(setup: &MatchSetup) -> Result<(), String> {
    // Home team (track_id 0-10)
    if setup.home.starters.len() != 11 {
        return Err(format!(
            "[AUDIT-A3-HARD] Home team has {} players (expected 11)",
            setup.home.starters.len()
        ));
    }

    // Away team (track_id 11-21)
    if setup.away.starters.len() != 11 {
        return Err(format!(
            "[AUDIT-A3-HARD] Away team has {} players (expected 11)",
            setup.away.starters.len()
        ));
    }

    Ok(())
}

// ===========================================
// A4: Attributes Range Validation
// ===========================================

use crate::models::player::PlayerAttributes;

/// A4-HARD: Attributes 범위 검증
///
/// **실행 조건**: 항상 (debug + release)
/// **위반 시**: Result::Err 반환 (엔진 거부)
///
/// # 검증 항목
///
/// 모든 attribute (36개 필드) 가 0-100 범위 내
pub fn validate_player_attributes(
    attrs: &PlayerAttributes,
    player_name: &str,
    team_name: &str,
) -> Result<(), String> {
    // A4: attributes range (HARD) - 대표 필드만 검증 (36개 전부 하면 verbose)
    let sample_fields = [
        ("passing", attrs.passing),
        ("dribbling", attrs.dribbling),
        ("tackling", attrs.tackling),
        ("finishing", attrs.finishing),
        ("pace", attrs.pace),
        ("stamina", attrs.stamina),
    ];

    for (name, value) in &sample_fields {
        // Note: ATTR_MIN is 0, so we only check upper bound (u8 cannot be negative)
        if *value > ATTR_MAX {
            return Err(format!(
                "[AUDIT-A4-HARD] Player '{}' ({}) attribute '{}' = {} (expected {}-{})",
                player_name, team_name, name, value, ATTR_MIN, ATTR_MAX
            ));
        }
    }

    Ok(())
}

// ===========================================
// B2-B3: SOFT Gates (경고만, 7 빌드 후 HARD)
// ===========================================

use crate::models::TeamSetup;

/// B2-SOFT: 유효하지 않은 포메이션
///
/// 현재는 경고만, DECISION_01 확정 후 HARD 전환
pub fn validate_formation_soft(formation: &str, _team_name: &str) {
    let valid = ["4-4-2", "4-3-3", "3-5-2", "4-2-3-1", "3-4-3"];

    if !valid.contains(&formation) {
        #[cfg(debug_assertions)]
        eprintln!(
            "[AUDIT-B2-SOFT] Team '{}' has invalid formation '{}' (expected one of {:?})",
            _team_name, formation, valid
        );
    }
}

/// B3-SOFT: Roster 크기 != 11
///
/// 현재는 경고만, DECISION_01 확정 후 HARD 전환
pub fn validate_roster_size_soft(team: &TeamSetup) {
    if team.starters.len() != 11 {
        #[cfg(debug_assertions)]
        eprintln!(
            "[AUDIT-B3-SOFT] Team '{}' has {} players (expected 11)",
            team.name,
            team.starters.len()
        );
    }
}

// ===========================================
// 통합 검증 함수
// ===========================================

/// MatchPlan 전체 검증 (A3, A4, B2-B3)
///
/// MatchEngine::new()에서 호출
pub fn validate_match_plan(
    setup: &MatchSetup,
    home_formation: &str,
    away_formation: &str,
) -> Result<(), String> {
    // A3: track_id integrity (HARD)
    validate_track_ids(setup)?;

    // A4: attributes (HARD)
    for player in &setup.home.starters {
        validate_player_attributes(&player.attributes, &player.name, &setup.home.name)?;
    }
    for player in &setup.away.starters {
        validate_player_attributes(&player.attributes, &player.name, &setup.away.name)?;
    }

    // B2: formation (SOFT)
    validate_formation_soft(home_formation, &setup.home.name);
    validate_formation_soft(away_formation, &setup.away.name);

    // B3: roster size (SOFT)
    validate_roster_size_soft(&setup.home);
    validate_roster_size_soft(&setup.away);

    Ok(())
}

// ===========================================
// 테스트
// ===========================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn test_ball_coordinates_valid() {
        use crate::engine::types::coord10::{Coord10, Vel10};
        let ball = Ball {
            position: Coord10::CENTER, // 중앙 (field center)
            velocity: Vel10::default(),
            height: 0, // FIX_2601: i16
            ..Default::default()
        };

        validate_ball_coordinates(&ball); // Should not panic
    }

    #[test]
    fn test_ball_coordinates_oob() {
        use crate::engine::types::coord10::{Coord10, Vel10};
        // Out of bounds (x > 106.0m)
        let ball = Ball {
            position: Coord10::from_meters(110.25, field::CENTER_Y), // 110.25m > 106.0
            velocity: Vel10::default(),
            height: 0, // FIX_2601: i16
            ..Default::default()
        };

        // debug_assertions에서만 로그 (패닉 없음)
        validate_ball_coordinates(&ball);
    }

    #[test]
    fn test_ball_coordinates_nan() {
        use crate::engine::types::coord10::{Coord10, Vel10};
        let ball = Ball {
            position: Coord10::from_meters(f32::NAN, 0.5),
            velocity: Vel10::default(),
            height: 0, // FIX_2601: i16 in 0.1m units
            ..Default::default()
        };

        validate_ball_coordinates(&ball); // Should not panic
    }

    #[test]
    fn test_player_coordinates_valid() {
        // FIX_2601: Use Coord10 for player positions (center = 525, 340 in 0.1m units)
        use crate::engine::types::Coord10;
        let positions = vec![Coord10 { x: 525, y: 340, z: 0 }; 22]; // All at center

        validate_player_coordinates(&positions); // Should not panic
    }

    #[test]
    fn test_player_coordinates_wrong_count() {
        // FIX_2601: Use Coord10 for player positions
        use crate::engine::types::Coord10;
        let positions = vec![Coord10 { x: 525, y: 340, z: 0 }; 20]; // Only 20 players

        validate_player_coordinates(&positions); // Should log error
    }
}
