extends RefCounted
class_name RealInterp

## 리얼한 보간 알고리즘 (Hermite + Dead Reckoning + Error Correction)
##
## 이 클래스는 선수 움직임을 "미학적 스무딩"이 아닌 "리얼한 물리"로 구현합니다.
## 단순한 lerp() 대신 velocity/acceleration/turn rate를 보존하는 알고리즘을 제공합니다.
##
## 사용 예시:
##   var target = RealInterp.hermite(s0.pos, s0.vel, s1.pos, s1.vel, t, dt)
##   var limited = RealInterp.turn_limited(cur_vel, target_vel, 8.0, delta)
##
## @date 2025-12-17
## @phase Phase 2B: Realistic Interpolation

## 회전 제한 (6~10 rad/s 권장, 실제 선수는 UFO처럼 즉시 방향 전환 불가)
const MAX_TURN_RAD_PER_SEC := 8.0

## 가속도 한도 (15~25 m/s² 권장, 실제 선수는 즉시 최고속도 도달 불가)
const MAX_ACCEL_MPS2 := 20.0

## 부드러운 오차 보정 임계값 (0.3~0.5m 권장)
const ERROR_SOFT_M := 0.35

## 텔레포트 임계값 (2.0~3.0m 권장, 이 이상 오차는 순간이동)
const ERROR_HARD_M := 2.50

## 오차 반감기 (0.08~0.2초 권장, 오차가 절반으로 줄어드는 시간)
const ERROR_HALF_LIFE_SEC := 0.12


## Cubic Hermite 보간 (위치 + 속도 동시 보간)
##
## Hermite 보간은 두 점 사이를 부드럽게 연결하되, 각 점에서의 속도(tangent)를 보존합니다.
## 이는 단순 lerp와 달리 가속도를 자연스럽게 표현할 수 있습니다.
##
## 공식:
##   P(t) = h00(t)*P0 + h10(t)*(V0*dt) + h01(t)*P1 + h11(t)*(V1*dt)
##
## Hermite basis functions:
##   h00(t) =  2t³ - 3t² + 1  (시작 위치 가중치)
##   h10(t) =   t³ - 2t² + t  (시작 속도 가중치)
##   h01(t) = -2t³ + 3t²      (끝 위치 가중치)
##   h11(t) =   t³ -  t²      (끝 속도 가중치)
##
## @param p0 시작 위치 (screen pixels)
## @param v0 시작 속도 (screen px/s)
## @param p1 끝 위치 (screen pixels)
## @param v1 끝 속도 (screen px/s)
## @param t01 보간 비율 [0, 1] (0 = p0, 1 = p1, >1 = extrapolation)
## @param dt 구간 시간 (seconds, s1.timestamp - s0.timestamp)
## @return 보간된 위치 (screen pixels)
static func hermite(p0: Vector2, v0: Vector2, p1: Vector2, v1: Vector2, t01: float, dt: float) -> Vector2:
	var t := clampf(t01, 0.0, 1.0)
	var t2 := t * t
	var t3 := t2 * t

	# Hermite basis functions
	var h00 := 2.0 * t3 - 3.0 * t2 + 1.0  # 시작 위치 가중치
	var h10 := t3 - 2.0 * t2 + t  # 시작 속도 가중치
	var h01 := -2.0 * t3 + 3.0 * t2  # 끝 위치 가중치
	var h11 := t3 - t2  # 끝 속도 가중치

	return (h00 * p0) + (h10 * (v0 * dt)) + (h01 * p1) + (h11 * (v1 * dt))


## 회전 제한 (Turn-Rate Limiting)
##
## 현실적인 회전: 선수는 즉시 방향을 바꿀 수 없습니다.
## 최대 각속도(angular velocity)를 제한하여 자연스러운 방향 전환을 구현합니다.
##
## 예시:
##   - 현재: 오른쪽으로 10 m/s
##   - 목표: 위쪽으로 10 m/s (90도 회전)
##   - dt = 0.1초, max_turn = 8 rad/s
##   - 실제 회전: 0.8 rad (45.8도만 회전)
##   - 결과: 오른쪽+위쪽 대각선 방향
##
## @param cur_vel 현재 속도 벡터 (screen px/s)
## @param target_vel 목표 속도 벡터 (screen px/s)
## @param max_turn_rad_per_sec 최대 회전 각속도 (rad/s, 권장: 6~10)
## @param dt 프레임 시간 (seconds)
## @return 회전 제한된 속도 벡터 (screen px/s)
static func turn_limited(cur_vel: Vector2, target_vel: Vector2, max_turn_rad_per_sec: float, dt: float) -> Vector2:
	# 속도가 0이면 즉시 목표로 (서있는 상태에서 출발)
	if cur_vel.length_squared() < 1e-8:
		return target_vel
	if target_vel.length_squared() < 1e-8:
		return Vector2.ZERO

	var cur_dir := cur_vel.normalized()
	var tgt_dir := target_vel.normalized()
	var cur_spd := cur_vel.length()
	var tgt_spd := target_vel.length()

	# 각도 차이 계산 (signed angle, -π ~ π)
	var angle := cur_dir.angle_to(tgt_dir)
	var max_angle := max_turn_rad_per_sec * maxf(dt, 1e-6)
	var clamped := clampf(angle, -max_angle, max_angle)

	# 제한된 각도로 회전
	var new_dir := cur_dir.rotated(clamped).normalized()

	# 속도 크기는 부드럽게 전환 (lerp, 약 12 Hz blend)
	var new_spd := lerpf(cur_spd, tgt_spd, clampf(dt * 12.0, 0.0, 1.0))

	return new_dir * new_spd


## 가속도 제한 (Acceleration Limiting)
##
## 현실적인 가속: 속도가 즉시 바뀔 수 없습니다.
## 최대 가속도(acceleration)를 제한하여 자연스러운 속도 변화를 구현합니다.
##
## 예시:
##   - 현재: 정지 (0 m/s)
##   - 목표: 최고속도 (10 m/s)
##   - dt = 0.1초, max_accel = 20 m/s²
##   - 실제 가속: 2 m/s (0.1 * 20)
##   - 결과: 0 → 2 m/s (10 m/s까지 5프레임 소요)
##
## @param cur_vel 현재 속도 (screen px/s)
## @param desired_vel 목표 속도 (screen px/s)
## @param max_accel 최대 가속도 (screen px/s², 권장: 15~25)
## @param dt 프레임 시간 (seconds)
## @return 가속 제한된 속도 (screen px/s)
static func accel_limited(cur_vel: Vector2, desired_vel: Vector2, max_accel: float, dt: float) -> Vector2:
	var dv := desired_vel - cur_vel
	var max_dv := max_accel * maxf(dt, 1e-6)
	var len := dv.length()

	# 속도 변화량이 max_dv를 초과하면 제한
	if len > max_dv and len > 1e-6:
		dv = dv * (max_dv / len)

	return cur_vel + dv


## 오차 블렌딩 계수 (Error Blend Factor)
##
## 지수 감쇠(exponential decay)를 사용하여 오차를 부드럽게 흡수합니다.
## 오차가 즉시 사라지지 않고, 반감기에 따라 점진적으로 줄어듭니다.
##
## 공식:
##   factor = 1 - 0.5^(dt / half_life)
##
## 예시:
##   - half_life = 0.12초
##   - dt = 0.016초 (60fps)
##   - factor = 1 - 0.5^(0.016/0.12) ≈ 0.09 (9% 보정)
##   - 0.12초 후: 오차가 절반으로 줄어듦
##
## @param half_life_sec 오차가 절반으로 줄어드는 시간 (초)
## @param dt 프레임 시간 (seconds)
## @return 블렌딩 계수 [0, 1] (0 = 보정 안 함, 1 = 즉시 보정)
static func error_blend_factor(half_life_sec: float, dt: float) -> float:
	if half_life_sec <= 1e-6:
		return 1.0
	return 1.0 - pow(0.5, dt / half_life_sec)
