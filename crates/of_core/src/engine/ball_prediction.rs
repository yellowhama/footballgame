//! Ball Prediction System
//!
//! FIX_2601/0112: Google Football 스타일 공 예측 + 캐싱
//!
//! 공의 미래 위치를 예측하여 인터셉트 판단 및 패스 타겟 선택에 활용

use super::physics_constants::{ball, google_football, substep};
use super::types::coord10::{Coord10, Vel10};
use crate::engine::physics_constants::field;

/// 공 예측 캐시 시스템
///
/// - 10ms 간격으로 공 위치 예측
/// - 최대 3초(3000ms) 예측 윈도우
/// - 공 위치/속도 변경 시 캐시 무효화
#[derive(Debug, Clone)]
pub struct BallPrediction {
    /// 예측 위치 (10ms 간격): (x_m, y_m, z_m)
    predictions: Vec<(f32, f32, f32)>,
    /// 캐시 유효 여부
    cache_valid: bool,
    /// 마지막 공 위치 (Coord10)
    last_position: Coord10,
    /// 마지막 공 속도 (Vel10)
    last_velocity: Vel10,
}

impl Default for BallPrediction {
    fn default() -> Self {
        Self::new()
    }
}

impl BallPrediction {
    /// 새 BallPrediction 인스턴스 생성
    pub fn new() -> Self {
        Self {
            predictions: Vec::with_capacity(google_football::CACHED_PREDICTIONS),
            cache_valid: false,
            last_position: Coord10::ZERO,
            last_velocity: Vel10::from_mps(0.0, 0.0),
        }
    }

    /// 공 예측 위치 슬라이스 반환 (캐싱)
    ///
    /// # Arguments
    /// - `ball_pos`: 현재 공 위치 (Coord10)
    /// - `ball_vel`: 현재 공 속도 (Vel10)
    /// - `duration_ms`: 예측 기간 (ms), 최대 3000ms
    ///
    /// # Returns
    /// 10ms 간격의 (x_m, y_m, z_m) 예측 위치 슬라이스
    pub fn predict(
        &mut self,
        ball_pos: Coord10,
        ball_vel: Vel10,
        duration_ms: u32,
    ) -> &[(f32, f32, f32)] {
        let duration_ms = duration_ms.min(google_football::PREDICTION_WINDOW_MS);

        // 캐시 유효성 확인
        if !self.cache_valid || self.last_position != ball_pos || self.last_velocity != ball_vel {
            self.recalculate(ball_pos, ball_vel, duration_ms);
        }

        &self.predictions
    }

    /// 특정 시점의 예측 위치 반환
    ///
    /// # Arguments
    /// - `ball_pos`: 현재 공 위치
    /// - `ball_vel`: 현재 공 속도
    /// - `time_ms`: 예측 시점 (현재로부터 ms)
    ///
    /// # Returns
    /// (x_m, y_m, z_m) 예측 위치
    pub fn position_at(
        &mut self,
        ball_pos: Coord10,
        ball_vel: Vel10,
        time_ms: u32,
    ) -> (f32, f32, f32) {
        let predictions = self.predict(ball_pos, ball_vel, time_ms);
        if predictions.is_empty() {
            let pos_m = ball_pos.to_meters();
            return (pos_m.0, pos_m.1, 0.0);
        }

        let idx = (time_ms / google_football::PREDICTION_STEP_MS).min(predictions.len() as u32 - 1)
            as usize;
        predictions[idx]
    }

    /// 플레이어가 공에 도달 가능한 시점 찾기
    ///
    /// # Arguments
    /// - `ball_pos`: 현재 공 위치
    /// - `ball_vel`: 현재 공 속도
    /// - `player_pos`: 플레이어 위치 (meters)
    /// - `player_speed`: 플레이어 최대 속도 (m/s)
    ///
    /// # Returns
    /// - `Some(intercept_ms)`: 인터셉트 가능 시점 (ms)
    /// - `None`: 인터셉트 불가능
    pub fn find_intercept_time(
        &mut self,
        ball_pos: Coord10,
        ball_vel: Vel10,
        player_pos: (f32, f32),
        player_speed: f32,
    ) -> Option<u32> {
        let predictions = self.predict(ball_pos, ball_vel, google_football::PREDICTION_WINDOW_MS);

        for (idx, &(bx, by, _)) in predictions.iter().enumerate() {
            let time_ms = (idx as u32 + 1) * google_football::PREDICTION_STEP_MS;
            let time_sec = time_ms as f32 / 1000.0;

            // 플레이어가 해당 시점까지 도달 가능한 거리
            let player_reach = player_speed * time_sec;

            // 공까지 거리
            let dx = bx - player_pos.0;
            let dy = by - player_pos.1;
            let ball_dist = (dx * dx + dy * dy).sqrt();

            // Google Football 스타일: 낙관적 반경 사용
            if ball_dist <= player_reach + google_football::RADIUS_OPTIMISTIC_M {
                return Some(time_ms);
            }
        }

        None
    }

    /// 인터셉트 가능 여부 확인 (간단한 버전)
    ///
    /// # Arguments
    /// - `ball_pos`: 현재 공 위치
    /// - `ball_vel`: 현재 공 속도
    /// - `player_pos`: 플레이어 위치 (meters)
    /// - `player_speed`: 플레이어 최대 속도 (m/s)
    /// - `within_ms`: 이 시간 내에 인터셉트 가능한지 (ms)
    pub fn can_intercept(
        &mut self,
        ball_pos: Coord10,
        ball_vel: Vel10,
        player_pos: (f32, f32),
        player_speed: f32,
        within_ms: u32,
    ) -> bool {
        if let Some(intercept_time) =
            self.find_intercept_time(ball_pos, ball_vel, player_pos, player_speed)
        {
            intercept_time <= within_ms
        } else {
            false
        }
    }

    /// 공 정지 예상 위치 반환
    ///
    /// 속도가 MIN_VELOCITY 이하로 떨어지는 시점의 위치
    pub fn rest_position(&mut self, ball_pos: Coord10, ball_vel: Vel10) -> (f32, f32) {
        let predictions = self.predict(ball_pos, ball_vel, google_football::PREDICTION_WINDOW_MS);

        // 마지막 예측 위치 (속도가 충분히 감소했을 것)
        if let Some(&(x, y, _)) = predictions.last() {
            (x, y)
        } else {
            ball_pos.to_meters()
        }
    }

    /// 캐시 무효화
    pub fn invalidate(&mut self) {
        self.cache_valid = false;
    }

    /// 예측 재계산 (내부 함수)
    fn recalculate(&mut self, ball_pos: Coord10, ball_vel: Vel10, duration_ms: u32) {
        self.predictions.clear();

        let mut pos = ball_pos.to_meters();
        let mut vel = ball_vel.to_mps();
        let height = 0.0_f32; // 지상 가정

        let steps = (duration_ms as f32 / substep::SUBSTEP_MS) as usize;

        for _ in 0..steps {
            // 물리 시뮬레이션 (ball_physics.rs 상수 사용)
            let speed = (vel.0.powi(2) + vel.1.powi(2)).sqrt();

            if speed > ball::MIN_VELOCITY {
                // 드래그 + 롤링 저항 감속 (Google Football 스타일 간소화)
                // Google Football: decel = drag * speed + friction
                let drag_decel = google_football::DRAG * speed;
                let friction_decel = google_football::FRICTION;
                let decel = (drag_decel + friction_decel) * substep::SUBSTEP_SEC;
                let new_speed = (speed - decel).max(0.0);

                // 속도 업데이트
                if speed > 0.001 {
                    vel = (vel.0 * new_speed / speed, vel.1 * new_speed / speed);
                }

                // 위치 업데이트 (필드 경계 클램핑)
                pos = (
                    (pos.0 + vel.0 * substep::SUBSTEP_SEC).clamp(0.0, field::LENGTH_M),
                    (pos.1 + vel.1 * substep::SUBSTEP_SEC).clamp(0.0, field::WIDTH_M),
                );
            }

            self.predictions.push((pos.0, pos.1, height));
        }

        self.last_position = ball_pos;
        self.last_velocity = ball_vel;
        self.cache_valid = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::physics_constants::field;

    #[test]
    fn test_ball_prediction_stationary() {
        let mut predictor = BallPrediction::new();
        let pos = Coord10::CENTER;
        let vel = Vel10::from_mps(0.0, 0.0);

        let predictions = predictor.predict(pos, vel, 1000);

        // 정지 상태면 모든 예측이 현재 위치
        for &(x, y, _) in predictions.iter() {
            assert!((x - field::CENTER_X).abs() < 0.1);
            assert!((y - field::CENTER_Y).abs() < 0.1);
        }
    }

    #[test]
    fn test_ball_prediction_moving() {
        let mut predictor = BallPrediction::new();
        let pos = Coord10::from_meters(50.0, field::CENTER_Y);
        let vel = Vel10::from_mps(10.0, 0.0); // 10 m/s 오른쪽

        let predictions = predictor.predict(pos, vel, 1000);

        // 공이 오른쪽으로 이동해야 함
        assert!(!predictions.is_empty());
        let last = predictions.last().unwrap();
        assert!(last.0 > 50.0); // x 증가
    }

    #[test]
    fn test_find_intercept_time() {
        let mut predictor = BallPrediction::new();
        let ball_pos = Coord10::from_meters(50.0, field::CENTER_Y);
        let ball_vel = Vel10::from_mps(5.0, 0.0); // 5 m/s 오른쪽

        // 플레이어가 공 진행 방향에 위치
        let player_pos = (55.0, field::CENTER_Y);
        let player_speed = 7.0; // 7 m/s

        let intercept = predictor.find_intercept_time(ball_pos, ball_vel, player_pos, player_speed);
        assert!(intercept.is_some());
    }

    #[test]
    fn test_cache_invalidation() {
        let mut predictor = BallPrediction::new();
        let pos1 = Coord10::from_meters(50.0, field::CENTER_Y);
        let vel1 = Vel10::from_mps(5.0, 0.0);

        let _ = predictor.predict(pos1, vel1, 1000);
        assert!(predictor.cache_valid);

        // 다른 위치로 호출 → 캐시 재계산
        let pos2 = Coord10::from_meters(60.0, field::CENTER_Y);
        let _ = predictor.predict(pos2, vel1, 1000);
        assert!(predictor.cache_valid);
        assert_eq!(predictor.last_position, pos2);
    }
}
