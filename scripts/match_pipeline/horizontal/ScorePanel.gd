extends Control
class_name ScorePanel
##
## ScorePanel - 스코어 및 시간 표시 패널
##
## 기능:
##   - 홈/어웨이 스코어 표시
##   - 경기 시간 표시
##   - 9-patch 패널 배경
##
## 스펙: docs/spec+@/spec_v4/dev_spec/UI/2025-12-07_SOCCERALIA_HORIZONTAL_VIEW_SPEC.md
##

#region Constants
const HUD_PATH := "res://assets/sprites/socceralia/hud/"
const PANEL_MARGIN := 10
const FONT_SIZE := 16
#endregion

#region Export
@export var home_team_name: String = "HOME"
@export var away_team_name: String = "AWAY"
#endregion

#region State
var _home_score: int = 0
var _away_score: int = 0
var _match_time_ms: int = 0
var _half: int = 1  # 1 or 2
#endregion

#region Node References
var _panel_bg: NinePatchRect = null
var _home_label: Label = null
var _away_label: Label = null
var _score_label: Label = null
var _time_label: Label = null
#endregion


func _ready() -> void:
	_create_ui()
	_update_display()


func _create_ui() -> void:
	## 패널 배치 (화면 상단 중앙)
	set_anchors_preset(Control.PRESET_CENTER_TOP)
	custom_minimum_size = Vector2(200, 50)
	position.y = PANEL_MARGIN

	## 9-patch 배경
	_panel_bg = NinePatchRect.new()
	_panel_bg.name = "PanelBg"
	_panel_bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	_panel_bg.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST

	var panel_texture_path := HUD_PATH + "hud_panel-p9.png"
	if ResourceLoader.exists(panel_texture_path):
		_panel_bg.texture = load(panel_texture_path)
		## 9-patch 마진 설정 (추정, 실제 텍스처에 맞게 조정)
		_panel_bg.patch_margin_left = 4
		_panel_bg.patch_margin_right = 4
		_panel_bg.patch_margin_top = 4
		_panel_bg.patch_margin_bottom = 4
	else:
		## 텍스처 없으면 단색 배경
		var style := StyleBoxFlat.new()
		style.bg_color = Color(0.1, 0.1, 0.1, 0.8)
		style.corner_radius_top_left = 4
		style.corner_radius_top_right = 4
		style.corner_radius_bottom_left = 4
		style.corner_radius_bottom_right = 4

	add_child(_panel_bg)

	## HBox 컨테이너 (팀명 | 스코어 | 팀명)
	var hbox := HBoxContainer.new()
	hbox.name = "ScoreContainer"
	hbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	hbox.alignment = BoxContainer.ALIGNMENT_CENTER
	hbox.add_theme_constant_override("separation", 8)
	add_child(hbox)

	## 홈 팀 이름
	_home_label = _create_label(home_team_name, Color.WHITE)
	_home_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	_home_label.custom_minimum_size.x = 60
	hbox.add_child(_home_label)

	## 스코어
	_score_label = _create_label("0 - 0", Color.YELLOW)
	_score_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_score_label.custom_minimum_size.x = 50
	hbox.add_child(_score_label)

	## 어웨이 팀 이름
	_away_label = _create_label(away_team_name, Color.WHITE)
	_away_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
	_away_label.custom_minimum_size.x = 60
	hbox.add_child(_away_label)

	## 시간 표시 (하단)
	_time_label = _create_label("00:00", Color(0.8, 0.8, 0.8))
	_time_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_time_label.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	_time_label.position.y = -5
	add_child(_time_label)


func _create_label(text: String, color: Color) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", FONT_SIZE)
	label.add_theme_color_override("font_color", color)
	return label


func _update_display() -> void:
	if _score_label:
		_score_label.text = "%d - %d" % [_home_score, _away_score]

	if _time_label:
		var minutes := _match_time_ms / 60000
		var seconds := (_match_time_ms % 60000) / 1000
		_time_label.text = "%02d:%02d" % [minutes, seconds]

	if _home_label:
		_home_label.text = home_team_name

	if _away_label:
		_away_label.text = away_team_name


#region Public API


## 스코어 설정
func set_score(home: int, away: int) -> void:
	_home_score = home
	_away_score = away
	_update_display()


## 홈 스코어 증가
func add_home_goal() -> void:
	_home_score += 1
	_update_display()


## 어웨이 스코어 증가
func add_away_goal() -> void:
	_away_score += 1
	_update_display()


## 경기 시간 설정 (밀리초)
func set_time_ms(time_ms: int) -> void:
	_match_time_ms = time_ms
	_update_display()


## 하프 설정
func set_half(half: int) -> void:
	_half = half


## 팀 이름 설정
func set_team_names(home: String, away: String) -> void:
	home_team_name = home
	away_team_name = away
	_update_display()


## 전체 리셋
func reset() -> void:
	_home_score = 0
	_away_score = 0
	_match_time_ms = 0
	_half = 1
	_update_display()


## 현재 스코어 반환
func get_score() -> Dictionary:
	return {"home": _home_score, "away": _away_score}

#endregion
