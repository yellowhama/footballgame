extends Node2D
class_name NightMatchLighting
##
## NightMatchLighting - 야간 경기 조명 효과
##
## 기능:
##   - CanvasModulate로 전체 화면 어둡게
##   - 공 주변 PointLight2D 스포트라이트
##   - 경기장 조명 배치 (옵션)
##
## Created: 2025-12-11 (Phase 9)
##

#region Constants
## 야간 모드 색상 (어두운 남색)
const NIGHT_COLOR := Color(0.15, 0.15, 0.25, 1.0)
const DAY_COLOR := Color(1.0, 1.0, 1.0, 1.0)

## 공 주변 라이트 설정
const BALL_LIGHT_ENERGY := 1.2
const BALL_LIGHT_SCALE := Vector2(0.8, 0.8)
const BALL_LIGHT_COLOR := Color(1.0, 0.95, 0.8)  ## 따뜻한 조명

## 경기장 조명 위치 (상단에서 비추는 효과)
const STADIUM_LIGHT_POSITIONS := [
	Vector2(262, -50),  ## 왼쪽 상단
	Vector2(525, -50),  ## 중앙 상단
	Vector2(788, -50),  ## 오른쪽 상단
	Vector2(262, 730),  ## 왼쪽 하단
	Vector2(525, 730),  ## 중앙 하단
	Vector2(788, 730),  ## 오른쪽 하단
]
const STADIUM_LIGHT_ENERGY := 0.8
const STADIUM_LIGHT_COLOR := Color(1.0, 1.0, 0.9)
#endregion

#region State
var _enabled: bool = false
var _canvas_modulate: CanvasModulate = null
var _ball_light: PointLight2D = null
var _stadium_lights: Array[PointLight2D] = []
var _ball_target: Node2D = null  ## 추적할 공
#endregion


func _ready() -> void:
	_setup_canvas_modulate()
	_setup_ball_light()


func _process(_delta: float) -> void:
	if _enabled and _ball_target and _ball_light:
		_ball_light.global_position = _ball_target.global_position


## CanvasModulate 설정 (전체 화면 색상 조절)
func _setup_canvas_modulate() -> void:
	_canvas_modulate = CanvasModulate.new()
	_canvas_modulate.name = "NightModulate"
	_canvas_modulate.color = DAY_COLOR  ## 초기값: 낮 (비활성화)
	add_child(_canvas_modulate)


## 공 주변 PointLight2D 설정
func _setup_ball_light() -> void:
	_ball_light = PointLight2D.new()
	_ball_light.name = "BallSpotlight"
	_ball_light.enabled = false
	_ball_light.color = BALL_LIGHT_COLOR
	_ball_light.energy = BALL_LIGHT_ENERGY
	_ball_light.texture_scale = BALL_LIGHT_SCALE.x
	_ball_light.shadow_enabled = false  ## 성능을 위해 그림자 비활성화

	## 기본 라이트 텍스처 사용 (부드러운 원형 그라디언트)
	_ball_light.texture = _create_light_texture()

	add_child(_ball_light)


## 경기장 조명 설정 (옵션)
func setup_stadium_lights() -> void:
	for pos in STADIUM_LIGHT_POSITIONS:
		var light := PointLight2D.new()
		light.position = pos
		light.enabled = _enabled
		light.color = STADIUM_LIGHT_COLOR
		light.energy = STADIUM_LIGHT_ENERGY
		light.texture_scale = 2.0
		light.texture = _create_light_texture()
		light.shadow_enabled = false
		add_child(light)
		_stadium_lights.append(light)


## 간단한 라이트 텍스처 생성 (부드러운 원형)
func _create_light_texture() -> GradientTexture2D:
	var gradient := Gradient.new()
	gradient.set_color(0, Color.WHITE)
	gradient.set_color(1, Color(1, 1, 1, 0))

	var tex := GradientTexture2D.new()
	tex.gradient = gradient
	tex.fill = GradientTexture2D.FILL_RADIAL
	tex.fill_from = Vector2(0.5, 0.5)
	tex.fill_to = Vector2(0.5, 0.0)
	tex.width = 256
	tex.height = 256

	return tex


#region Public API
## Night Match 모드 활성화/비활성화
func set_enabled(enabled: bool) -> void:
	_enabled = enabled

	## CanvasModulate 색상 변경
	if _canvas_modulate:
		if enabled:
			_canvas_modulate.color = NIGHT_COLOR
		else:
			_canvas_modulate.color = DAY_COLOR

	## 공 라이트 활성화
	if _ball_light:
		_ball_light.enabled = enabled

	## 경기장 라이트 활성화
	for light in _stadium_lights:
		light.enabled = enabled


## Night Match 모드 토글
func toggle() -> bool:
	set_enabled(not _enabled)
	return _enabled


## 공 타겟 설정 (라이트가 따라다님)
func set_ball_target(ball: Node2D) -> void:
	_ball_target = ball


## 현재 상태 반환
func is_enabled() -> bool:
	return _enabled


## 조명 강도 조절
func set_light_energy(ball_energy: float = 1.2, stadium_energy: float = 0.8) -> void:
	if _ball_light:
		_ball_light.energy = ball_energy

	for light in _stadium_lights:
		light.energy = stadium_energy


## 야간 색상 조절 (0.0 = 완전 어둠, 1.0 = 낮)
func set_darkness_level(level: float) -> void:
	level = clamp(level, 0.0, 1.0)
	var darkness := 1.0 - level

	if _canvas_modulate:
		_canvas_modulate.color = DAY_COLOR.lerp(NIGHT_COLOR, darkness)
#endregion
