extends Node
## ThemeManager - DPI-aware theme management + color constants
## Phase 7A: Added cross-platform theme switching
## Phase 1: UI_UX_Design_Specification.md 컬러 시스템 통합 (2025-12-03)
## Prevents fractional scaling blur by using integer-scaled themes

# ============================================================================
# 2.1 기본 팔레트 (GitHub Dark Style)
# ============================================================================

# 배경 (Background)
const BG_PRIMARY = Color("0D1117")  # 메인 배경
const BG_SECONDARY = Color("161B22")  # 카드/패널 배경
const BG_TERTIARY = Color("21262D")  # 호버/선택 상태
const BACKGROUND = BG_PRIMARY  # 별칭
const SURFACE = BG_SECONDARY  # 별칭
const BG_SURFACE = BG_SECONDARY  # 별칭
const BG_SURFACE_VARIANT = BG_TERTIARY  # 별칭
const BG_MEDIUM = BG_TERTIARY  # 별칭

# 텍스트 (Text)
const TEXT_PRIMARY = Color("E6EDF3")  # 주요 텍스트 (95% 밝기)
const TEXT_SECONDARY = Color("8B949E")  # 보조 텍스트 (60% 밝기)
const TEXT_DISABLED = Color("484F58")  # 비활성 텍스트
const TEXT_HIGHLIGHT = Color("FFD700")  # 강조 텍스트

# 액센트 (Accent)
const SUCCESS = Color("238636")  # 성공/확인 (Green)
const INFO = Color("1F6FEB")  # 정보/링크 (Blue)
const WARNING = Color("D29922")  # 경고 (Amber)
const DANGER = Color("F85149")  # 위험/에러 (Red)
const PRIMARY = INFO  # 별칭
const SECONDARY = TEXT_SECONDARY  # 별칭
const ACCENT = Color("58A6FF")  # 하이라이트 (Light Blue)

# ============================================================================
# 2.2 포지션별 컬러코드 (FIFA/FM 스타일)
# ============================================================================
const POS_GOALKEEPER = Color("FFD700")  # 골키퍼 - 노랑
const POS_DEFENDER = Color("1E90FF")  # 수비수 - 파랑
const POS_MIDFIELDER = Color("32CD32")  # 미드필더 - 초록
const POS_FORWARD = Color("FF4444")  # 공격수 - 빨강

# ============================================================================
# 2.3 능력치 시각화 컬러
# ============================================================================
const STAT_LEGENDARY = Color("FF8C00")  # 90+ (오렌지)
const STAT_EXCELLENT = Color("FFD700")  # 80-89 (골드)
const STAT_GOOD = Color("32CD32")  # 70-79 (그린)
const STAT_AVERAGE = Color("87CEEB")  # 60-69 (스카이블루)
const STAT_BELOW = Color("FFFFFF")  # 50-59 (화이트)
const STAT_POOR = Color("FF6B6B")  # 50 미만 (레드)

# Pastel 색상들 (호환성)
const PASTEL_YELLOW = Color("FFFACD")
const PASTEL_GREEN = Color("98FB98")
const PASTEL_BLUE = Color("ADD8E6")
const PASTEL_PURPLE = Color("DDA0DD")
const PASTEL_ORANGE = Color("FFDAB9")

# 그림자 색상
const SHADOW_COLOR = Color(0.0, 0.0, 0.0, 0.3)

# ============================================================================
# 3. 타이포그래피 스케일 (Typography Scale)
# ============================================================================
const FONT_H1 = 28  # H1 (화면 제목) Bold
const FONT_H2 = 24  # H2 (섹션 제목) SemiBold
const FONT_H3 = 20  # H3 (카드 제목) Medium
const FONT_BODY = 16  # Body (본문) Regular
const FONT_CAPTION = 14  # Caption (설명) Regular
const FONT_MICRO = 12  # Micro (레이블) Medium

# 폰트 크기 별칭 (호환성)
const FONT_SIZE_TITLE = FONT_H1
const FONT_SIZE_XLARGE = FONT_H2
const FONT_SIZE_LARGE = FONT_H3
const FONT_SIZE_MEDIUM = FONT_BODY
const FONT_SIZE_SMALL = FONT_MICRO

# ============================================================================
# 4. 스페이싱 시스템 (4px Grid)
# ============================================================================
const SPACE_XS = 4  # 아이콘 내부
const SPACE_SM = 8  # 요소 내부 패딩
const SPACE_MD = 16  # 카드 패딩, 요소 간격
const SPACE_LG = 24  # 섹션 간격
const SPACE_XL = 32  # 화면 마진
const SPACE_XXL = 48  # 대형 섹션 분리

# 여백 상수 별칭 (호환성)
const MARGIN_SMALL = SPACE_SM
const MARGIN_MEDIUM = SPACE_MD
const MARGIN_LARGE = SPACE_LG

# 모서리 반지름 상수
const CORNER_RADIUS_SMALL = 4
const CORNER_RADIUS_MEDIUM = 8
const CORNER_RADIUS_LARGE = 12

# ============================================================================
# 5. 버튼 컴포넌트 표준
# ============================================================================
const BUTTON_MIN_WIDTH = 100  # 버튼 최소 너비
const BUTTON_HEIGHT = 56  # 버튼 표준 높이
const BUTTON_COMPACT_HEIGHT = 44  # 컴팩트 버튼 높이
const TOUCH_MIN = 44  # 최소 터치 영역
const TOUCH_COMFORT = 48  # 권장 터치 영역

# 모바일 관련 상수
const MOBILE_TOUCH_SIZE = TOUCH_MIN
const MOBILE_MARGIN = SPACE_MD
const MOBILE_PADDING = SPACE_SM

## DPI-aware theme resources (Phase 7A)
## NOTE: These theme files need to be created in res://themes/
## For now, we'll use runtime theme generation until theme files exist
var current_theme_key: String = "mobile"

signal theme_changed(theme_key: String)


func _ready():
	print("[ThemeManager] Initializing...")

	# 폰트 로드
	load_fonts()

	# Wait for PlatformManager to be ready
	if PlatformManager:
		PlatformManager.platform_changed.connect(_apply_theme)
		PlatformManager.viewport_resized.connect(_on_viewport_changed)
		await get_tree().process_frame  # Wait one frame for platform detection
		_apply_theme()
	else:
		push_warning("[ThemeManager] PlatformManager not found, skipping DPI-aware theme switching")

	# 공통 스타일 프리캐싱
	precache_common_styles()

	print("[ThemeManager] Initialized - Theme: %s" % current_theme_key)


func _apply_theme(_new_platform = null):
	var theme_key = _determine_theme_key()

	if theme_key == current_theme_key:
		return  # No change needed

	# For now, just update theme key without actual theme switching
	# TODO Phase 7A: Create actual theme .tres files and load them
	current_theme_key = theme_key
	theme_changed.emit(theme_key)

	print(
		(
			"[ThemeManager] Theme changed to: %s (Platform: %s, DPI: %d)"
			% [theme_key, PlatformManager.get_platform_name(), PlatformManager.dpi]
		)
	)


func _determine_theme_key() -> String:
	if not PlatformManager:
		return "mobile"

	var dpi = PlatformManager.dpi
	var platform = PlatformManager.current_platform
	var viewport_width = PlatformManager.viewport_size.x

	# High DPI desktop (1440p+, >150 DPI, >2000px width)
	if platform == PlatformManager.Platform.DESKTOP and dpi > 150 and viewport_width >= 2000:
		return "desktop_hd"

	# Standard DPI desktop (96-150 DPI, >1280px width)
	elif platform == PlatformManager.Platform.DESKTOP:
		return "desktop"

	# Tablet (7-12 inch screens, 163-264 DPI)
	elif platform == PlatformManager.Platform.TABLET:
		return "tablet"

	# Mobile (< 7 inch screens, 160-460 DPI)
	else:
		return "mobile"


func _on_viewport_changed(_new_size: Vector2i):
	# Re-evaluate theme on viewport size changes (e.g., window resize on desktop)
	_apply_theme()


## Public API for theme queries
func get_current_theme_key() -> String:
	return current_theme_key


func get_base_font_size() -> int:
	match current_theme_key:
		"mobile":
			return 14
		"tablet":
			return 16
		"desktop":
			return 16
		"desktop_hd":
			return 18
		_:
			return 14


func get_title_font_size() -> int:
	match current_theme_key:
		"mobile":
			return 20
		"tablet":
			return 22
		"desktop":
			return 24
		"desktop_hd":
			return 26
		_:
			return 20


func get_margin_size() -> int:
	match current_theme_key:
		"mobile":
			return 16
		"tablet":
			return 24
		"desktop":
			return 32
		"desktop_hd":
			return 40
		_:
			return 16


func get_spacing_size() -> int:
	match current_theme_key:
		"mobile":
			return 12
		"tablet":
			return 16
		"desktop":
			return 20
		"desktop_hd":
			return 24
		_:
			return 12


# 스탯 색상 계산 함수 (스펙 2.3 능력치 시각화 컬러)
func get_stat_color(value: float, max_value: float = 100.0) -> Color:
	var normalized = (value / max_value) * 100.0
	if normalized >= 90.0:
		return STAT_LEGENDARY  # 90+ 오렌지
	elif normalized >= 80.0:
		return STAT_EXCELLENT  # 80-89 골드
	elif normalized >= 70.0:
		return STAT_GOOD  # 70-79 그린
	elif normalized >= 60.0:
		return STAT_AVERAGE  # 60-69 스카이블루
	elif normalized >= 50.0:
		return STAT_BELOW  # 50-59 화이트
	else:
		return STAT_POOR  # <50 레드


# 컨디션 색상 함수
func get_condition_color(condition: int) -> Color:
	match condition:
		5:
			return SUCCESS
		4:
			return PASTEL_GREEN
		3:
			return PASTEL_YELLOW
		2:
			return WARNING
		1:
			return DANGER
		_:
			return TEXT_SECONDARY


# 스킬 등급 배경색 함수
func get_skill_grade_background(grade: String) -> Color:
	match grade:
		"S":
			return SUCCESS
		"A":
			return PASTEL_GREEN
		"B":
			return PASTEL_BLUE
		"C":
			return PASTEL_YELLOW
		"D":
			return PASTEL_ORANGE
		"F":
			return DANGER
		_:
			return BG_SURFACE


# 포지션 색상 함수 (스펙 2.2 포지션별 컬러코드)
func get_position_color(position: String) -> Color:
	var pos_upper = position.to_upper()
	if pos_upper in ["GK", "G", "GOALKEEPER"]:
		return POS_GOALKEEPER
	elif pos_upper in ["DF", "D", "CB", "LB", "RB", "LWB", "RWB", "DEFENDER"]:
		return POS_DEFENDER
	elif pos_upper in ["MF", "M", "CM", "DM", "AM", "LM", "RM", "CDM", "CAM", "MIDFIELDER"]:
		return POS_MIDFIELDER
	elif pos_upper in ["FW", "F", "ST", "CF", "LW", "RW", "SS", "FORWARD"]:
		return POS_FORWARD
	else:
		return TEXT_SECONDARY


# 그라데이션 생성 함수
func create_gradient(color1: Color, color2: Color) -> Gradient:
	var gradient = Gradient.new()
	gradient.add_point(0.0, color1)
	gradient.add_point(1.0, color2)
	return gradient


# ============================================================================
# 6. 버튼 스타일 팩토리 함수 (스펙 5. 버튼 컴포넌트 표준)
# ============================================================================


## 기본 버튼 스타일 생성
func create_button_stylebox(bg_color: Color, border_color: Color = Color.TRANSPARENT) -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = bg_color
	style.set_corner_radius_all(CORNER_RADIUS_MEDIUM)
	style.content_margin_left = SPACE_MD
	style.content_margin_right = SPACE_MD
	style.content_margin_top = SPACE_SM
	style.content_margin_bottom = SPACE_SM
	if border_color != Color.TRANSPARENT:
		style.set_border_width_all(1)
		style.border_color = border_color
	return style


## Primary 버튼 스타일 (파란색 배경)
func create_button_style_primary() -> Dictionary:
	return {
		"normal": create_button_stylebox(INFO),
		"hover": create_button_stylebox(ACCENT),
		"pressed": create_button_stylebox(INFO.darkened(0.2)),
		"disabled": create_button_stylebox(BG_TERTIARY),
		"font_color": TEXT_PRIMARY,
		"font_hover_color": BG_PRIMARY,
		"font_pressed_color": TEXT_PRIMARY,
		"font_disabled_color": TEXT_DISABLED
	}


## Secondary 버튼 스타일 (테두리만)
func create_button_style_secondary() -> Dictionary:
	return {
		"normal": create_button_stylebox(BG_SECONDARY, BG_TERTIARY),
		"hover": create_button_stylebox(BG_TERTIARY, ACCENT),
		"pressed": create_button_stylebox(BG_PRIMARY, ACCENT),
		"disabled": create_button_stylebox(BG_SECONDARY, BG_TERTIARY),
		"font_color": TEXT_PRIMARY,
		"font_hover_color": ACCENT,
		"font_pressed_color": TEXT_PRIMARY,
		"font_disabled_color": TEXT_DISABLED
	}


## Success 버튼 스타일 (초록색)
func create_button_style_success() -> Dictionary:
	return {
		"normal": create_button_stylebox(SUCCESS),
		"hover": create_button_stylebox(SUCCESS.lightened(0.1)),
		"pressed": create_button_stylebox(SUCCESS.darkened(0.2)),
		"disabled": create_button_stylebox(BG_TERTIARY),
		"font_color": TEXT_PRIMARY,
		"font_hover_color": TEXT_PRIMARY,
		"font_pressed_color": TEXT_PRIMARY,
		"font_disabled_color": TEXT_DISABLED
	}


## Danger 버튼 스타일 (빨간색)
func create_button_style_danger() -> Dictionary:
	return {
		"normal": create_button_stylebox(DANGER),
		"hover": create_button_stylebox(DANGER.lightened(0.1)),
		"pressed": create_button_stylebox(DANGER.darkened(0.2)),
		"disabled": create_button_stylebox(BG_TERTIARY),
		"font_color": TEXT_PRIMARY,
		"font_hover_color": TEXT_PRIMARY,
		"font_pressed_color": TEXT_PRIMARY,
		"font_disabled_color": TEXT_DISABLED
	}


## Warning 버튼 스타일 (노란색)
func create_button_style_warning() -> Dictionary:
	return {
		"normal": create_button_stylebox(WARNING),
		"hover": create_button_stylebox(WARNING.lightened(0.1)),
		"pressed": create_button_stylebox(WARNING.darkened(0.2)),
		"disabled": create_button_stylebox(BG_TERTIARY),
		"font_color": BG_PRIMARY,
		"font_hover_color": BG_PRIMARY,
		"font_pressed_color": BG_PRIMARY,
		"font_disabled_color": TEXT_DISABLED
	}


## 버튼에 스타일 일괄 적용
func apply_button_style(button: Button, style_dict: Dictionary) -> void:
	button.add_theme_stylebox_override("normal", style_dict.normal)
	button.add_theme_stylebox_override("hover", style_dict.hover)
	button.add_theme_stylebox_override("pressed", style_dict.pressed)
	button.add_theme_stylebox_override("disabled", style_dict.disabled)
	button.add_theme_color_override("font_color", style_dict.font_color)
	button.add_theme_color_override("font_hover_color", style_dict.font_hover_color)
	button.add_theme_color_override("font_pressed_color", style_dict.font_pressed_color)
	button.add_theme_color_override("font_disabled_color", style_dict.font_disabled_color)
	# 최소 크기 적용
	button.custom_minimum_size.x = max(button.custom_minimum_size.x, BUTTON_MIN_WIDTH)
	button.custom_minimum_size.y = max(button.custom_minimum_size.y, TOUCH_MIN)


## 버튼 variant 이름으로 스타일 가져오기
func get_button_style(variant: String) -> Dictionary:
	match variant.to_lower():
		"primary":
			return create_button_style_primary()
		"success":
			return create_button_style_success()
		"danger":
			return create_button_style_danger()
		"warning":
			return create_button_style_warning()
		_:
			return create_button_style_secondary()


# ============================================================================
# 7. 네비게이션 바 스타일 (하단 네비게이션 표준)
# ============================================================================


## 하단 네비게이션 바 스타일 (다크 테마)
func create_navbar_style() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = BG_SECONDARY
	style.border_width_top = 1
	style.border_color = BG_TERTIARY
	style.content_margin_left = SPACE_MD
	style.content_margin_right = SPACE_MD
	style.content_margin_top = SPACE_SM
	style.content_margin_bottom = SPACE_SM
	return style


## 헤더 바 스타일 (상단 네비게이션)
func create_header_style() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = BG_SECONDARY
	style.border_width_bottom = 1
	style.border_color = BG_TERTIARY
	style.content_margin_left = SPACE_LG
	style.content_margin_right = SPACE_LG
	style.content_margin_top = SPACE_SM
	style.content_margin_bottom = SPACE_SM
	return style


## 네비게이션 버튼 스타일 (투명 배경)
func create_nav_button_style() -> Dictionary:
	var normal = StyleBoxFlat.new()
	normal.bg_color = Color.TRANSPARENT
	normal.content_margin_left = SPACE_SM
	normal.content_margin_right = SPACE_SM
	normal.content_margin_top = SPACE_XS
	normal.content_margin_bottom = SPACE_XS

	var hover = StyleBoxFlat.new()
	hover.bg_color = BG_TERTIARY
	hover.set_corner_radius_all(CORNER_RADIUS_SMALL)
	hover.content_margin_left = SPACE_SM
	hover.content_margin_right = SPACE_SM
	hover.content_margin_top = SPACE_XS
	hover.content_margin_bottom = SPACE_XS

	var pressed = StyleBoxFlat.new()
	pressed.bg_color = BG_PRIMARY
	pressed.set_corner_radius_all(CORNER_RADIUS_SMALL)
	pressed.content_margin_left = SPACE_SM
	pressed.content_margin_right = SPACE_SM
	pressed.content_margin_top = SPACE_XS
	pressed.content_margin_bottom = SPACE_XS

	return {
		"normal": normal,
		"hover": hover,
		"pressed": pressed,
		"disabled": normal,
		"font_color": TEXT_SECONDARY,
		"font_hover_color": TEXT_PRIMARY,
		"font_pressed_color": ACCENT,
		"font_disabled_color": TEXT_DISABLED
	}


## 활성화된 네비게이션 버튼 스타일
func create_nav_button_active_style() -> Dictionary:
	var base_style = create_nav_button_style()
	base_style.font_color = ACCENT
	return base_style


## Panel에 네비게이션 바 스타일 적용
func apply_navbar_style(panel: Panel) -> void:
	var style = create_navbar_style()
	panel.add_theme_stylebox_override("panel", style)


## Panel에 헤더 스타일 적용
func apply_header_style(panel: Panel) -> void:
	var style = create_header_style()
	panel.add_theme_stylebox_override("panel", style)


## 카드 스타일 생성
func create_card_style() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = BG_SECONDARY
	style.set_border_width_all(1)
	style.border_color = BG_TERTIARY
	style.set_corner_radius_all(CORNER_RADIUS_MEDIUM)
	style.content_margin_left = SPACE_MD
	style.content_margin_right = SPACE_MD
	style.content_margin_top = SPACE_MD
	style.content_margin_bottom = SPACE_MD
	return style


## 카드 호버 스타일 생성
func create_card_hover_style() -> StyleBoxFlat:
	var style = create_card_style()
	style.border_color = ACCENT
	return style


# ============================================================================
# 8. 선수 카드 컴포넌트 스타일 (Phase 2: 선수 카드 표준화)
# ============================================================================


## 선수 카드 스타일 생성 (포지션 기반 액센트)
func create_player_card_style(position: String) -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = BG_SECONDARY
	var pos_color = get_position_color(position)
	style.border_width_left = 4  # 왼쪽에 포지션 색상 바
	style.border_color = pos_color
	style.set_corner_radius_all(CORNER_RADIUS_MEDIUM)
	style.content_margin_left = SPACE_MD
	style.content_margin_right = SPACE_MD
	style.content_margin_top = SPACE_SM
	style.content_margin_bottom = SPACE_SM
	return style


## 선수 카드 호버 스타일 생성
func create_player_card_hover_style(position: String) -> StyleBoxFlat:
	var style = create_player_card_style(position)
	style.bg_color = BG_TERTIARY
	style.border_width_right = 1
	style.border_width_top = 1
	style.border_width_bottom = 1
	return style


## 선수 카드 선택 스타일 생성
func create_player_card_selected_style(position: String) -> StyleBoxFlat:
	var style = create_player_card_style(position)
	style.bg_color = BG_TERTIARY
	style.set_border_width_all(2)
	style.border_color = ACCENT
	return style


## OVR 기반 별 등급 반환 (Uma Musume 스타일)
func get_star_rating(overall: int) -> String:
	if overall >= 90:
		return "★★★★★"
	elif overall >= 80:
		return "★★★★☆"
	elif overall >= 70:
		return "★★★☆☆"
	elif overall >= 60:
		return "★★☆☆☆"
	else:
		return "★☆☆☆☆"


## OVR 기반 별 개수 반환
func get_star_count(overall: int) -> int:
	if overall >= 90:
		return 5
	elif overall >= 80:
		return 4
	elif overall >= 70:
		return 3
	elif overall >= 60:
		return 2
	else:
		return 1


## 선수 카드에 스타일 일괄 적용 (PanelContainer용)
func apply_player_card_style(panel: PanelContainer, position: String, is_selected: bool = false) -> void:
	var style: StyleBoxFlat
	if is_selected:
		style = create_player_card_selected_style(position)
	else:
		style = create_player_card_style(position)
	panel.add_theme_stylebox_override("panel", style)


## 선수 카드에 버튼 스타일 적용 (Button용)
func apply_player_card_button_style(button: Button, position: String) -> void:
	var normal = create_player_card_style(position)
	var hover = create_player_card_hover_style(position)
	var pressed = create_player_card_selected_style(position)

	button.add_theme_stylebox_override("normal", normal)
	button.add_theme_stylebox_override("hover", hover)
	button.add_theme_stylebox_override("pressed", pressed)
	button.add_theme_stylebox_override("disabled", normal)
	button.add_theme_color_override("font_color", TEXT_PRIMARY)
	button.add_theme_color_override("font_hover_color", TEXT_PRIMARY)
	button.add_theme_color_override("font_pressed_color", TEXT_HIGHLIGHT)
	button.add_theme_font_size_override("font_size", FONT_BODY)


# ============================================================================
# 9. 경기 이벤트 색상 (Phase 3: 이벤트 로그 시각화)
# ============================================================================

## 이벤트 타입별 색상
const EVENT_GOAL = Color("FFD700")  # 골 - 골드
const EVENT_ASSIST = Color("87CEEB")  # 어시스트 - 스카이블루
const EVENT_SHOT = Color("1E90FF")  # 슈팅 - 파랑
const EVENT_SAVE = Color("32CD32")  # 세이브 - 초록
const EVENT_FOUL = Color("FF4444")  # 파울 - 빨강
const EVENT_YELLOW_CARD = Color("FFD700")  # 옐로카드 - 노랑
const EVENT_RED_CARD = Color("FF0000")  # 레드카드 - 빨강
const EVENT_SUBSTITUTION = Color("9370DB")  # 교체 - 보라
const EVENT_INJURY = Color("FF6B6B")  # 부상 - 연빨강
const EVENT_OFFSIDE = Color("FFA500")  # 오프사이드 - 오렌지
const EVENT_CORNER = Color("87CEEB")  # 코너킥 - 스카이블루
const EVENT_FREE_KICK = Color("ADD8E6")  # 프리킥 - 라이트블루
const EVENT_PENALTY = Color("FF8C00")  # 페널티 - 오렌지
const EVENT_KICKOFF = Color("FFFFFF")  # 킥오프 - 화이트
const EVENT_WHISTLE = Color("AAAAAA")  # 휘슬 - 회색


## 이벤트 타입으로 색상 가져오기
func get_event_color(event_type: String) -> Color:
	match event_type.to_lower():
		"goal":
			return EVENT_GOAL
		"assist":
			return EVENT_ASSIST
		"shot", "shot_on_target", "shot_off_target":
			return EVENT_SHOT
		"save", "goalkeeper_save":
			return EVENT_SAVE
		"foul":
			return EVENT_FOUL
		"yellow_card", "yellowcard":
			return EVENT_YELLOW_CARD
		"red_card", "redcard":
			return EVENT_RED_CARD
		"substitution", "sub":
			return EVENT_SUBSTITUTION
		"injury":
			return EVENT_INJURY
		"offside":
			return EVENT_OFFSIDE
		"corner", "corner_kick":
			return EVENT_CORNER
		"free_kick", "freekick":
			return EVENT_FREE_KICK
		"penalty":
			return EVENT_PENALTY
		"kickoff", "kick_off":
			return EVENT_KICKOFF
		"whistle", "half_time", "full_time":
			return EVENT_WHISTLE
		_:
			return TEXT_SECONDARY


## 이벤트 아이콘 가져오기
func get_event_icon(event_type: String) -> String:
	match event_type.to_lower():
		"goal":
			return "⚽"
		"assist":
			return "👟"
		"shot", "shot_on_target", "shot_off_target":
			return "💨"
		"save", "goalkeeper_save":
			return "🧤"
		"foul":
			return "⛔"
		"yellow_card", "yellowcard":
			return "🟨"
		"red_card", "redcard":
			return "🟥"
		"substitution", "sub":
			return "🔄"
		"injury":
			return "🏥"
		"offside":
			return "🚩"
		"corner", "corner_kick":
			return "📐"
		"free_kick", "freekick":
			return "🎯"
		"penalty":
			return "⚠️"
		"kickoff", "kick_off":
			return "🏁"
		"whistle", "half_time", "full_time":
			return "📯"
		_:
			return "•"


# ============================================================================
# 10. 미니맵 스타일 (Phase 3: 미니맵 + MatchPlayer 통합)
# ============================================================================

## 미니맵 필드 색상 (다크 테마)
const MINIMAP_FIELD = Color(0.15, 0.35, 0.15, 1.0)  # 어두운 녹색
const MINIMAP_LINE = Color(0.5, 0.5, 0.5, 0.8)  # 회색 라인
const MINIMAP_BALL = Color("FFD700")  # 볼 - 골드
const MINIMAP_HOME_DEFAULT = Color("1E90FF")  # 홈팀 기본 - 파랑
const MINIMAP_AWAY_DEFAULT = Color("FF4444")  # 어웨이팀 기본 - 빨강

## 이벤트 궤적 색상
const MINIMAP_PASS_TRAIL = Color(0.2, 0.8, 0.2, 0.8)  # 패스 - 초록
const MINIMAP_SHOT_TRAIL = Color(0.9, 0.2, 0.2, 0.9)  # 슈팅 - 빨강
const MINIMAP_SHOT_MISS = Color(0.9, 0.6, 0.2, 0.8)  # 슛 실패 - 주황
const MINIMAP_DRIBBLE_TRAIL = Color(0.6, 0.3, 0.9, 0.7)  # 드리블 - 보라
const MINIMAP_TACKLE_EFFECT = Color(1.0, 0.5, 0.0, 0.9)  # 태클 - 주황


## 선수 도트 스타일 생성
func create_player_dot_style(team_color: Color, dot_size: float) -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = team_color
	var radius = int(dot_size * 0.5)
	style.corner_radius_top_left = radius
	style.corner_radius_top_right = radius
	style.corner_radius_bottom_left = radius
	style.corner_radius_bottom_right = radius
	style.set_border_width_all(1)
	style.border_color = Color(0, 0, 0, 0.5)
	return style


## 선수 도트 스타일 (포지션 기반)
func create_position_dot_style(position: String, dot_size: float) -> StyleBoxFlat:
	var pos_color = get_position_color(position)
	return create_player_dot_style(pos_color, dot_size)


## 볼 도트 스타일 생성
func create_ball_dot_style(dot_size: float) -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = MINIMAP_BALL
	var radius = int(dot_size * 0.5)
	style.corner_radius_top_left = radius
	style.corner_radius_top_right = radius
	style.corner_radius_bottom_left = radius
	style.corner_radius_bottom_right = radius
	style.set_border_width_all(2)
	style.border_color = Color.WHITE
	return style


## 하이라이트된 선수 도트 스타일 (공 소유자)
func create_ball_holder_dot_style(team_color: Color, dot_size: float) -> StyleBoxFlat:
	var style = create_player_dot_style(team_color, dot_size)
	style.border_color = MINIMAP_BALL
	style.set_border_width_all(3)
	return style


# ============================================================================
# 11. MatchPlayer 라벨 스타일 (Phase 3: MatchPlayer 통합)
# ============================================================================


## 선수 이름 라벨 스타일 적용
func apply_player_name_label_style(label: Label) -> void:
	label.add_theme_color_override("font_color", TEXT_PRIMARY)
	label.add_theme_font_size_override("font_size", FONT_MICRO)
	label.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.8))
	label.add_theme_constant_override("shadow_offset_x", 1)
	label.add_theme_constant_override("shadow_offset_y", 1)


## 선수 번호 라벨 스타일 적용
func apply_player_number_label_style(label: Label) -> void:
	label.add_theme_color_override("font_color", TEXT_HIGHLIGHT)
	label.add_theme_font_size_override("font_size", FONT_BODY)
	label.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.9))
	label.add_theme_constant_override("shadow_offset_x", 1)
	label.add_theme_constant_override("shadow_offset_y", 1)


## 선수 이름 라벨 스타일 (포지션 색상 강조)
func apply_player_name_label_with_position_style(label: Label, position: String) -> void:
	var pos_color = get_position_color(position)
	label.add_theme_color_override("font_color", pos_color)
	label.add_theme_font_size_override("font_size", FONT_MICRO)
	label.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.8))
	label.add_theme_constant_override("shadow_offset_x", 1)
	label.add_theme_constant_override("shadow_offset_y", 1)


# ============================================================================
# 12. 애니메이션 시스템 (Phase 4: 폴리싱)
# ============================================================================

## 지속시간 상수
const DURATION_FAST = 0.1  # 100ms - 호버, 버튼 프레스
const DURATION_NORMAL = 0.2  # 200ms - 기본 전환
const DURATION_SLOW = 0.3  # 300ms - 화면 전환
const DURATION_COMPLEX = 0.5  # 500ms - 복잡한 애니메이션

## 타이밍 함수 (Tween.TransitionType, EaseType)
# ease-out: TRANS_QUAD, EASE_OUT (진입)
# ease-in: TRANS_QUAD, EASE_IN (퇴장)
# ease-in-out: TRANS_QUAD, EASE_IN_OUT (일반)
# bounce: TRANS_BACK, EASE_OUT (강조)


## 버튼 프레스 애니메이션
func animate_button_press(button: Control, on_complete: Callable = Callable()) -> Tween:
	var tween = button.create_tween()
	tween.set_trans(Tween.TRANS_QUAD)
	tween.set_ease(Tween.EASE_OUT)
	tween.tween_property(button, "scale", Vector2(0.96, 0.96), DURATION_FAST)
	tween.tween_property(button, "scale", Vector2.ONE, DURATION_FAST)
	if on_complete.is_valid():
		tween.tween_callback(on_complete)
	return tween


## 카드 호버 애니메이션
func animate_card_hover(card: Control, hover_in: bool) -> Tween:
	var tween = card.create_tween()
	tween.set_trans(Tween.TRANS_QUAD)
	tween.set_ease(Tween.EASE_OUT)
	if hover_in:
		tween.tween_property(card, "position:y", card.position.y - 4.0, DURATION_NORMAL)
	else:
		tween.tween_property(card, "position:y", card.position.y + 4.0, DURATION_NORMAL)
	return tween


## 페이드 인 애니메이션
func animate_fade_in(control: Control, duration: float = DURATION_NORMAL) -> Tween:
	control.modulate.a = 0.0
	var tween = control.create_tween()
	tween.set_trans(Tween.TRANS_QUAD)
	tween.set_ease(Tween.EASE_OUT)
	tween.tween_property(control, "modulate:a", 1.0, duration)
	return tween


## 페이드 아웃 애니메이션
func animate_fade_out(control: Control, duration: float = DURATION_NORMAL, free_on_complete: bool = false) -> Tween:
	var tween = control.create_tween()
	tween.set_trans(Tween.TRANS_QUAD)
	tween.set_ease(Tween.EASE_IN)
	tween.tween_property(control, "modulate:a", 0.0, duration)
	if free_on_complete:
		tween.tween_callback(control.queue_free)
	return tween


## 스케일 팝업 애니메이션 (모달, 팝업)
func animate_scale_popup(control: Control, duration: float = DURATION_NORMAL) -> Tween:
	control.scale = Vector2(0.9, 0.9)
	control.modulate.a = 0.0
	var tween = control.create_tween()
	tween.set_trans(Tween.TRANS_BACK)
	tween.set_ease(Tween.EASE_OUT)
	tween.set_parallel(true)
	tween.tween_property(control, "scale", Vector2.ONE, duration)
	tween.tween_property(control, "modulate:a", 1.0, duration * 0.7)
	return tween


## 스케일 팝다운 애니메이션 (모달 닫기)
func animate_scale_popdown(control: Control, duration: float = DURATION_FAST, free_on_complete: bool = true) -> Tween:
	var tween = control.create_tween()
	tween.set_trans(Tween.TRANS_QUAD)
	tween.set_ease(Tween.EASE_IN)
	tween.set_parallel(true)
	tween.tween_property(control, "scale", Vector2(0.9, 0.9), duration)
	tween.tween_property(control, "modulate:a", 0.0, duration)
	if free_on_complete:
		tween.tween_callback(control.queue_free)
	return tween


## 슬라이드 인 애니메이션 (화면 전환)
func animate_slide_in(control: Control, from_direction: String = "right", duration: float = DURATION_SLOW) -> Tween:
	var start_offset = Vector2.ZERO
	match from_direction.to_lower():
		"left":
			start_offset = Vector2(-control.size.x, 0)
		"right":
			start_offset = Vector2(control.size.x, 0)
		"up":
			start_offset = Vector2(0, -control.size.y)
		"down":
			start_offset = Vector2(0, control.size.y)

	var target_pos = control.position
	control.position = target_pos + start_offset
	control.modulate.a = 0.0

	var tween = control.create_tween()
	tween.set_trans(Tween.TRANS_QUAD)
	tween.set_ease(Tween.EASE_OUT)
	tween.set_parallel(true)
	tween.tween_property(control, "position", target_pos, duration)
	tween.tween_property(control, "modulate:a", 1.0, duration * 0.5)
	return tween


## 슬라이드 아웃 애니메이션
func animate_slide_out(
	control: Control, to_direction: String = "left", duration: float = DURATION_SLOW, free_on_complete: bool = false
) -> Tween:
	var end_offset = Vector2.ZERO
	match to_direction.to_lower():
		"left":
			end_offset = Vector2(-control.size.x, 0)
		"right":
			end_offset = Vector2(control.size.x, 0)
		"up":
			end_offset = Vector2(0, -control.size.y)
		"down":
			end_offset = Vector2(0, control.size.y)

	var tween = control.create_tween()
	tween.set_trans(Tween.TRANS_QUAD)
	tween.set_ease(Tween.EASE_IN)
	tween.set_parallel(true)
	tween.tween_property(control, "position", control.position + end_offset, duration)
	tween.tween_property(control, "modulate:a", 0.0, duration * 0.5)
	if free_on_complete:
		tween.tween_callback(control.queue_free)
	return tween


## 바운스 강조 애니메이션
func animate_bounce(control: Control, scale_factor: float = 1.1, duration: float = DURATION_NORMAL) -> Tween:
	var tween = control.create_tween()
	tween.set_trans(Tween.TRANS_BACK)
	tween.set_ease(Tween.EASE_OUT)
	tween.tween_property(control, "scale", Vector2(scale_factor, scale_factor), duration * 0.5)
	tween.tween_property(control, "scale", Vector2.ONE, duration * 0.5)
	return tween


## 펄스 애니메이션 (성공 피드백)
func animate_pulse(control: Control, color: Color = SUCCESS, duration: float = DURATION_SLOW) -> Tween:
	var original_modulate = control.modulate
	var tween = control.create_tween()
	tween.set_trans(Tween.TRANS_QUAD)
	tween.set_ease(Tween.EASE_OUT)
	tween.tween_property(control, "modulate", color, duration * 0.3)
	tween.tween_property(control, "modulate", original_modulate, duration * 0.7)
	return tween


## 화면 흔들림 애니메이션 (골 이벤트)
func animate_screen_shake(control: Control, intensity: float = 10.0, duration: float = DURATION_SLOW) -> Tween:
	var original_pos = control.position
	var tween = control.create_tween()

	var shake_count = int(duration / 0.05)
	for i in range(shake_count):
		var offset = Vector2(randf_range(-intensity, intensity), randf_range(-intensity, intensity))
		tween.tween_property(control, "position", original_pos + offset, 0.05)

	tween.tween_property(control, "position", original_pos, 0.05)
	return tween


## 순차적 리스트 애니메이션 (staggered)
func animate_list_staggered(controls: Array, delay_per_item: float = 0.05, animation_type: String = "fade") -> void:
	for i in range(controls.size()):
		var control = controls[i] as Control
		if not control:
			continue

		# 초기 상태 설정
		control.modulate.a = 0.0
		if animation_type == "slide":
			control.position.x += 30

		# 딜레이 후 애니메이션 시작
		var delay_time = i * delay_per_item
		var timer = control.get_tree().create_timer(delay_time)
		await timer.timeout

		var tween = control.create_tween()
		tween.set_trans(Tween.TRANS_QUAD)
		tween.set_ease(Tween.EASE_OUT)
		tween.set_parallel(true)
		tween.tween_property(control, "modulate:a", 1.0, DURATION_NORMAL)
		if animation_type == "slide":
			tween.tween_property(control, "position:x", control.position.x - 30, DURATION_NORMAL)


## 컬러 전환 애니메이션
func animate_color_transition(control: Control, target_color: Color, duration: float = DURATION_NORMAL) -> Tween:
	var tween = control.create_tween()
	tween.set_trans(Tween.TRANS_QUAD)
	tween.set_ease(Tween.EASE_IN_OUT)
	tween.tween_property(control, "modulate", target_color, duration)
	return tween


# ============================================================================
# 13. 로딩/에러 상태 스타일 (Phase 4: 폴리싱)
# ============================================================================


## 로딩 스피너 생성
func create_loading_spinner(parent: Control, size: float = 32.0) -> Control:
	var spinner_container = Control.new()
	spinner_container.custom_minimum_size = Vector2(size, size)
	spinner_container.set_anchors_preset(Control.PRESET_CENTER)

	# 스피너 도트들
	var dot_count = 8
	var dot_size = size * 0.15
	var radius = size * 0.35

	for i in range(dot_count):
		var dot = ColorRect.new()
		dot.color = ACCENT
		dot.size = Vector2(dot_size, dot_size)
		dot.modulate.a = 0.3 + (0.7 * (i / float(dot_count)))

		var angle = (TAU * float(i) / float(dot_count)) - PI / 2.0
		var pos = Vector2(cos(angle), sin(angle)) * radius
		dot.position = (
			pos + Vector2(float(size) / 2.0 - float(dot_size) / 2.0, float(size) / 2.0 - float(dot_size) / 2.0)
		)

		spinner_container.add_child(dot)

	# 회전 애니메이션 시작
	_start_spinner_animation(spinner_container)

	parent.add_child(spinner_container)
	return spinner_container


func _start_spinner_animation(spinner: Control) -> void:
	var tween = spinner.create_tween()
	tween.set_loops()
	tween.tween_property(spinner, "rotation", TAU, 1.0)


## 로딩 오버레이 생성
func create_loading_overlay(parent: Control, message: String = "로딩 중...") -> Control:
	var overlay = ColorRect.new()
	overlay.color = Color(BG_PRIMARY.r, BG_PRIMARY.g, BG_PRIMARY.b, 0.8)
	overlay.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	overlay.mouse_filter = Control.MOUSE_FILTER_STOP

	var center = VBoxContainer.new()
	center.set_anchors_preset(Control.PRESET_CENTER)
	center.alignment = BoxContainer.ALIGNMENT_CENTER
	overlay.add_child(center)

	# 스피너
	var spinner_holder = Control.new()
	spinner_holder.custom_minimum_size = Vector2(48, 48)
	center.add_child(spinner_holder)
	create_loading_spinner(spinner_holder, 48.0)

	# 메시지
	var label = Label.new()
	label.text = message
	label.add_theme_color_override("font_color", TEXT_SECONDARY)
	label.add_theme_font_size_override("font_size", FONT_BODY)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	center.add_child(label)

	parent.add_child(overlay)
	animate_fade_in(overlay, DURATION_FAST)
	return overlay


## 로딩 오버레이 제거
func remove_loading_overlay(overlay: Control) -> void:
	if overlay and is_instance_valid(overlay):
		animate_fade_out(overlay, DURATION_FAST, true)


## 에러 상태 패널 생성
func create_error_panel(message: String, on_retry: Callable = Callable()) -> PanelContainer:
	var panel = PanelContainer.new()
	var style = create_card_style()
	style.border_color = DANGER
	style.set_border_width_all(2)
	panel.add_theme_stylebox_override("panel", style)

	var content = VBoxContainer.new()
	content.add_theme_constant_override("separation", SPACE_MD)
	panel.add_child(content)

	# 아이콘 + 제목
	var header = HBoxContainer.new()
	header.add_theme_constant_override("separation", SPACE_SM)
	content.add_child(header)

	var icon = Label.new()
	icon.text = "⚠"
	icon.add_theme_color_override("font_color", DANGER)
	icon.add_theme_font_size_override("font_size", FONT_H2)
	header.add_child(icon)

	var title = Label.new()
	title.text = "오류 발생"
	title.add_theme_color_override("font_color", DANGER)
	title.add_theme_font_size_override("font_size", FONT_H3)
	header.add_child(title)

	# 에러 메시지
	var msg_label = Label.new()
	msg_label.text = message
	msg_label.add_theme_color_override("font_color", TEXT_SECONDARY)
	msg_label.add_theme_font_size_override("font_size", FONT_BODY)
	msg_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	content.add_child(msg_label)

	# 재시도 버튼 (콜백이 있을 경우)
	if on_retry.is_valid():
		var retry_btn = Button.new()
		retry_btn.text = "다시 시도"
		apply_button_style(retry_btn, get_button_style("danger"))
		retry_btn.pressed.connect(on_retry)
		content.add_child(retry_btn)

	return panel


## 빈 상태 패널 생성
func create_empty_state_panel(
	message: String, icon_text: String = "📭", action_text: String = "", on_action: Callable = Callable()
) -> PanelContainer:
	var panel = PanelContainer.new()
	var style = create_card_style()
	panel.add_theme_stylebox_override("panel", style)

	var content = VBoxContainer.new()
	content.alignment = BoxContainer.ALIGNMENT_CENTER
	content.add_theme_constant_override("separation", SPACE_MD)
	panel.add_child(content)

	# 아이콘
	var icon = Label.new()
	icon.text = icon_text
	icon.add_theme_font_size_override("font_size", 48)
	icon.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	content.add_child(icon)

	# 메시지
	var msg_label = Label.new()
	msg_label.text = message
	msg_label.add_theme_color_override("font_color", TEXT_SECONDARY)
	msg_label.add_theme_font_size_override("font_size", FONT_BODY)
	msg_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	msg_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	content.add_child(msg_label)

	# 액션 버튼 (있을 경우)
	if not action_text.is_empty() and on_action.is_valid():
		var action_btn = Button.new()
		action_btn.text = action_text
		apply_button_style(action_btn, get_button_style("primary"))
		action_btn.pressed.connect(on_action)
		content.add_child(action_btn)

	return panel


## 성공 토스트 메시지
func show_toast(parent: Control, message: String, type: String = "info", duration: float = 2.0) -> void:
	var toast = PanelContainer.new()
	var style = create_card_style()

	match type.to_lower():
		"success":
			style.border_color = SUCCESS
		"error":
			style.border_color = DANGER
		"warning":
			style.border_color = WARNING
		_:
			style.border_color = INFO

	style.set_border_width_all(2)
	toast.add_theme_stylebox_override("panel", style)

	var label = Label.new()
	label.text = message
	label.add_theme_color_override("font_color", TEXT_PRIMARY)
	label.add_theme_font_size_override("font_size", FONT_BODY)
	toast.add_child(label)

	# 위치 설정 (상단 중앙)
	toast.set_anchors_preset(Control.PRESET_CENTER_TOP)
	toast.position.y = SPACE_LG

	parent.add_child(toast)

	# 애니메이션
	animate_slide_in(toast, "up", DURATION_NORMAL)

	# 자동 제거
	var timer = parent.get_tree().create_timer(duration)
	await timer.timeout
	animate_slide_out(toast, "up", DURATION_NORMAL, true)


## 스켈레톤 로딩 스타일 생성
func create_skeleton_style() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = BG_TERTIARY
	style.set_corner_radius_all(CORNER_RADIUS_SMALL)
	return style


## 스켈레톤 로딩 애니메이션 적용
func apply_skeleton_animation(control: Control) -> Tween:
	var tween = control.create_tween()
	tween.set_loops()
	tween.tween_property(control, "modulate:a", 0.5, 0.8)
	tween.tween_property(control, "modulate:a", 1.0, 0.8)
	return tween


## 진행률 표시 바 스타일
func create_progress_bar_style() -> Dictionary:
	var bg_style = StyleBoxFlat.new()
	bg_style.bg_color = BG_TERTIARY
	bg_style.set_corner_radius_all(CORNER_RADIUS_SMALL)

	var fill_style = StyleBoxFlat.new()
	fill_style.bg_color = SUCCESS
	fill_style.set_corner_radius_all(CORNER_RADIUS_SMALL)

	return {"background": bg_style, "fill": fill_style}


## 진행률 바에 스타일 적용
func apply_progress_bar_style(progress_bar: ProgressBar, color: Color = SUCCESS) -> void:
	var bg_style = StyleBoxFlat.new()
	bg_style.bg_color = BG_TERTIARY
	bg_style.set_corner_radius_all(CORNER_RADIUS_SMALL)

	var fill_style = StyleBoxFlat.new()
	fill_style.bg_color = color
	fill_style.set_corner_radius_all(CORNER_RADIUS_SMALL)

	progress_bar.add_theme_stylebox_override("background", bg_style)
	progress_bar.add_theme_stylebox_override("fill", fill_style)


# ============================================================================
# 14. 접근성 (Accessibility) - Phase 4: 폴리싱
# ============================================================================

## 접근성 상수
const MIN_CONTRAST_RATIO = 4.5  # WCAG AA 기준
const LARGE_TEXT_CONTRAST = 3.0  # 큰 텍스트용
const TOUCH_TARGET_MIN = 44.0  # 최소 터치 타겟 (iOS HIG)


## 색상 대비 계산 (상대적 휘도 기반)
func get_luminance(color: Color) -> float:
	var r = color.r if color.r <= 0.03928 else pow((color.r + 0.055) / 1.055, 2.4)
	var g = color.g if color.g <= 0.03928 else pow((color.g + 0.055) / 1.055, 2.4)
	var b = color.b if color.b <= 0.03928 else pow((color.b + 0.055) / 1.055, 2.4)
	return 0.2126 * r + 0.7152 * g + 0.0722 * b


func get_contrast_ratio(color1: Color, color2: Color) -> float:
	var l1 = get_luminance(color1)
	var l2 = get_luminance(color2)
	var lighter = max(l1, l2)
	var darker = min(l1, l2)
	return (lighter + 0.05) / (darker + 0.05)


## 대비 검사 (WCAG AA 기준)
func check_contrast(foreground: Color, background: Color, large_text: bool = false) -> bool:
	var ratio = get_contrast_ratio(foreground, background)
	var required = LARGE_TEXT_CONTRAST if large_text else MIN_CONTRAST_RATIO
	return ratio >= required


## 포커스 링 스타일 생성
func create_focus_ring_style() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = Color.TRANSPARENT
	style.set_border_width_all(2)
	style.border_color = ACCENT
	style.set_corner_radius_all(CORNER_RADIUS_MEDIUM)
	# 외곽 offset으로 포커스 링 효과
	style.content_margin_left = 4
	style.content_margin_right = 4
	style.content_margin_top = 4
	style.content_margin_bottom = 4
	return style


## 버튼에 포커스 스타일 적용
func apply_focus_style(button: Button) -> void:
	var focus_style = create_focus_ring_style()
	button.add_theme_stylebox_override("focus", focus_style)


## 터치 타겟 크기 확인 및 조정
func ensure_touch_target_size(control: Control) -> void:
	if control.custom_minimum_size.x < TOUCH_TARGET_MIN:
		control.custom_minimum_size.x = TOUCH_TARGET_MIN
	if control.custom_minimum_size.y < TOUCH_TARGET_MIN:
		control.custom_minimum_size.y = TOUCH_TARGET_MIN


## 키보드 네비게이션을 위한 포커스 모드 설정
func setup_keyboard_navigation(controls: Array) -> void:
	for i in range(controls.size()):
		var control = controls[i] as Control
		if not control:
			continue

		control.focus_mode = Control.FOCUS_ALL

		# 이전/다음 연결
		if i > 0:
			control.focus_neighbor_top = controls[i - 1].get_path()
			control.focus_previous = controls[i - 1].get_path()
		if i < controls.size() - 1:
			control.focus_neighbor_bottom = controls[i + 1].get_path()
			control.focus_next = controls[i + 1].get_path()


## 색맹 친화적 대체 색상
const COLORBLIND_SAFE_SUCCESS = Color("009E73")  # 청록
const COLORBLIND_SAFE_WARNING = Color("E69F00")  # 주황
const COLORBLIND_SAFE_DANGER = Color("D55E00")  # 빨강/주황
const COLORBLIND_SAFE_INFO = Color("0072B2")  # 파랑


## 색맹 모드 색상 가져오기
func get_colorblind_safe_color(type: String) -> Color:
	match type.to_lower():
		"success":
			return COLORBLIND_SAFE_SUCCESS
		"warning":
			return COLORBLIND_SAFE_WARNING
		"danger":
			return COLORBLIND_SAFE_DANGER
		"info":
			return COLORBLIND_SAFE_INFO
		_:
			return TEXT_PRIMARY


## 스크린 리더 힌트 설정 (tooltip으로 대체)
func set_accessibility_hint(control: Control, hint: String) -> void:
	control.tooltip_text = hint
	# Godot 4에서는 직접적인 스크린 리더 API가 없어 tooltip 활용


## 애니메이션 축소 모드 (모션 민감성 대응)
var reduce_motion: bool = false


func set_reduce_motion(enabled: bool) -> void:
	reduce_motion = enabled


func get_animation_duration(base_duration: float) -> float:
	if reduce_motion:
		return 0.0  # 애니메이션 즉시 완료
	return base_duration


## 고대비 모드 색상
const HIGH_CONTRAST_BG = Color("#000000")
const HIGH_CONTRAST_FG = Color("#FFFFFF")
const HIGH_CONTRAST_ACCENT = Color("#FFFF00")

var high_contrast_mode: bool = false


func set_high_contrast_mode(enabled: bool) -> void:
	high_contrast_mode = enabled


func get_accessible_bg_color() -> Color:
	return HIGH_CONTRAST_BG if high_contrast_mode else BG_PRIMARY


func get_accessible_fg_color() -> Color:
	return HIGH_CONTRAST_FG if high_contrast_mode else TEXT_PRIMARY


func get_accessible_accent_color() -> Color:
	return HIGH_CONTRAST_ACCENT if high_contrast_mode else ACCENT


# ============================================================================
# 15. 성능 최적화 (Performance) - Phase 4: 폴리싱
# ============================================================================

## 스타일 캐시 (StyleBoxFlat 재사용)
var _style_cache: Dictionary = {}


## 캐시된 스타일 가져오기
func get_cached_style(key: String, create_func: Callable) -> StyleBoxFlat:
	if not _style_cache.has(key):
		_style_cache[key] = create_func.call()
	return _style_cache[key]


## 스타일 캐시 클리어
func clear_style_cache() -> void:
	_style_cache.clear()


## 자주 사용되는 스타일 사전 생성
func precache_common_styles() -> void:
	# 카드 스타일
	_style_cache["card_normal"] = create_card_style()
	_style_cache["card_hover"] = create_card_hover_style()

	# 버튼 스타일
	_style_cache["btn_primary"] = create_button_style_primary()
	_style_cache["btn_secondary"] = create_button_style_secondary()
	_style_cache["btn_success"] = create_button_style_success()
	_style_cache["btn_danger"] = create_button_style_danger()

	# 네비게이션 스타일
	_style_cache["navbar"] = create_navbar_style()
	_style_cache["header"] = create_header_style()

	print("[ThemeManager] Common styles precached")


## 오브젝트 풀링 - 재사용 가능한 컨트롤 풀
var _control_pool: Dictionary = {}  # type_name → Array[Control]


func get_pooled_control(type_name: String, create_func: Callable) -> Control:
	if not _control_pool.has(type_name):
		_control_pool[type_name] = []

	var pool: Array = _control_pool[type_name]
	if pool.size() > 0:
		var control = pool.pop_back()
		control.visible = true
		return control

	# 풀이 비어있으면 새로 생성
	return create_func.call()


func return_to_pool(type_name: String, control: Control) -> void:
	if not _control_pool.has(type_name):
		_control_pool[type_name] = []

	control.visible = false
	control.get_parent().remove_child(control)
	_control_pool[type_name].append(control)


func clear_pool(type_name: String = "") -> void:
	if type_name.is_empty():
		for pool in _control_pool.values():
			for control in pool:
				control.queue_free()
		_control_pool.clear()
	elif _control_pool.has(type_name):
		for control in _control_pool[type_name]:
			control.queue_free()
		_control_pool[type_name].clear()


## 지연 로딩 헬퍼
func deferred_call(callable: Callable, delay: float = 0.0) -> void:
	if delay <= 0.0:
		callable.call_deferred()
	else:
		var timer = get_tree().create_timer(delay)
		await timer.timeout
		callable.call()


## 배치 스타일 적용 (다수 컨트롤에 동일 스타일 적용)
func batch_apply_style(controls: Array, style_func: Callable) -> void:
	for control in controls:
		if control is Control:
			style_func.call(control)


## 조건부 렌더링 (뷰포트 밖 컨트롤 숨기기)
func setup_visibility_culling(scroll_container: ScrollContainer, items: Array) -> void:
	var viewport_rect = scroll_container.get_viewport_rect()

	for item in items:
		if item is Control:
			var item_rect = item.get_global_rect()
			item.visible = viewport_rect.intersects(item_rect)


## 대형 리스트 가상 스크롤 설정
func create_virtual_scroll_list(
	container: Control, item_height: float, total_items: int, render_callback: Callable
) -> Dictionary:
	var visible_count = int(container.size.y / item_height) + 2  # 버퍼 포함
	var scroll_data = {
		"container": container,
		"item_height": item_height,
		"total_items": total_items,
		"visible_count": visible_count,
		"render_callback": render_callback,
		"current_offset": 0,
		"rendered_items": []
	}

	# 초기 렌더링
	_render_virtual_items(scroll_data, 0)

	return scroll_data


func _render_virtual_items(scroll_data: Dictionary, scroll_offset: int) -> void:
	var start_index = max(0, scroll_offset)
	var end_index = min(scroll_data.total_items, start_index + scroll_data.visible_count)

	# 기존 아이템 정리
	for item in scroll_data.rendered_items:
		if is_instance_valid(item):
			item.queue_free()
	scroll_data.rendered_items.clear()

	# 새 아이템 렌더링
	var container = scroll_data.container as Control
	for i in range(start_index, end_index):
		var item = scroll_data.render_callback.call(i)
		if item is Control:
			item.position.y = (i - start_index) * scroll_data.item_height
			container.add_child(item)
			scroll_data.rendered_items.append(item)

	scroll_data.current_offset = scroll_offset


func update_virtual_scroll(scroll_data: Dictionary, scroll_position: float) -> void:
	var new_offset = int(scroll_position / scroll_data.item_height)
	if new_offset != scroll_data.current_offset:
		_render_virtual_items(scroll_data, new_offset)


# ============================================================================
# 16. 햅틱 피드백 시스템 (Haptic Feedback) - 피드백 반영
# ============================================================================

## 햅틱 강도 상수
enum HapticIntensity { LIGHT, MEDIUM, HEAVY, SUCCESS, ERROR, WARNING }  # 가벼운 터치 (버튼 호버, 스크롤)  # 중간 (버튼 프레스, 선택)  # 강한 (성공, 경고)  # 성공 패턴 (골!)  # 에러 패턴  # 경고 패턴

## 햅틱 지속시간 (밀리초)
const HAPTIC_LIGHT_MS = 10
const HAPTIC_MEDIUM_MS = 25
const HAPTIC_HEAVY_MS = 50

## 햅틱 활성화 여부
var haptic_enabled: bool = true


func set_haptic_enabled(enabled: bool) -> void:
	haptic_enabled = enabled


## 햅틱 피드백 트리거
func trigger_haptic(intensity: HapticIntensity) -> void:
	if not haptic_enabled:
		return

	# Godot 4에서는 Input.vibrate_handheld() 사용
	# 강도에 따른 진동 시간 조절
	var duration_ms: int
	match intensity:
		HapticIntensity.LIGHT:
			duration_ms = HAPTIC_LIGHT_MS
		HapticIntensity.MEDIUM:
			duration_ms = HAPTIC_MEDIUM_MS
		HapticIntensity.HEAVY:
			duration_ms = HAPTIC_HEAVY_MS
		HapticIntensity.SUCCESS:
			# 성공 패턴: 짧음-긴
			_play_haptic_pattern([HAPTIC_LIGHT_MS, 50, HAPTIC_HEAVY_MS])
			return
		HapticIntensity.ERROR:
			# 에러 패턴: 강함-강함-강함
			_play_haptic_pattern([HAPTIC_HEAVY_MS, 100, HAPTIC_HEAVY_MS, 100, HAPTIC_HEAVY_MS])
			return
		HapticIntensity.WARNING:
			# 경고 패턴: 중간-중간
			_play_haptic_pattern([HAPTIC_MEDIUM_MS, 80, HAPTIC_MEDIUM_MS])
			return
		_:
			duration_ms = HAPTIC_MEDIUM_MS

	Input.vibrate_handheld(duration_ms)


## 햅틱 패턴 재생 (진동-대기-진동...)
func _play_haptic_pattern(pattern: Array) -> void:
	for i in range(pattern.size()):
		if i % 2 == 0:
			# 진동
			Input.vibrate_handheld(pattern[i])
		else:
			# 대기
			await get_tree().create_timer(pattern[i] / 1000.0).timeout


## 버튼 프레스 햅틱 (가장 자주 사용)
func haptic_button_press() -> void:
	trigger_haptic(HapticIntensity.MEDIUM)


## 선택 변경 햅틱
func haptic_selection() -> void:
	trigger_haptic(HapticIntensity.LIGHT)


## 성공 햅틱 (골, 저장 완료 등)
func haptic_success() -> void:
	trigger_haptic(HapticIntensity.SUCCESS)


## 에러 햅틱
func haptic_error() -> void:
	trigger_haptic(HapticIntensity.ERROR)


## 경고 햅틱 (옐로카드 등)
func haptic_warning() -> void:
	trigger_haptic(HapticIntensity.WARNING)


## 골 이벤트 햅틱 (특별 패턴)
func haptic_goal() -> void:
	# 강렬한 골 이벤트 패턴
	_play_haptic_pattern([HAPTIC_HEAVY_MS, 50, HAPTIC_HEAVY_MS, 50, HAPTIC_HEAVY_MS, 100, HAPTIC_HEAVY_MS * 2])


# ============================================================================
# 17. 야외 시인성 / 고대비 모드 개선 - 피드백 반영
# ============================================================================

## 야외 모드 색상 (밝은 배경)
const OUTDOOR_BG_PRIMARY = Color("#F5F5F5")
const OUTDOOR_BG_SECONDARY = Color("#FFFFFF")
const OUTDOOR_BG_TERTIARY = Color("#E0E0E0")
const OUTDOOR_TEXT_PRIMARY = Color("#212121")
const OUTDOOR_TEXT_SECONDARY = Color("#757575")

var outdoor_mode: bool = false


func set_outdoor_mode(enabled: bool) -> void:
	outdoor_mode = enabled


func get_adaptive_bg_color() -> Color:
	if high_contrast_mode:
		return HIGH_CONTRAST_BG
	elif outdoor_mode:
		return OUTDOOR_BG_PRIMARY
	return BG_PRIMARY


func get_adaptive_surface_color() -> Color:
	if high_contrast_mode:
		return HIGH_CONTRAST_BG
	elif outdoor_mode:
		return OUTDOOR_BG_SECONDARY
	return BG_SECONDARY


func get_adaptive_text_color() -> Color:
	if high_contrast_mode:
		return HIGH_CONTRAST_FG
	elif outdoor_mode:
		return OUTDOOR_TEXT_PRIMARY
	return TEXT_PRIMARY


func get_adaptive_text_secondary_color() -> Color:
	if high_contrast_mode:
		return HIGH_CONTRAST_FG
	elif outdoor_mode:
		return OUTDOOR_TEXT_SECONDARY
	return TEXT_SECONDARY


## 자동 밝기 감지 (시스템 설정 기반, 향후 확장용)
func detect_ambient_brightness() -> String:
	# TODO: 시스템 밝기 센서 연동 (플랫폼별 구현 필요)
	# 현재는 수동 설정만 지원
	return "normal"  # "dark", "normal", "bright"


# ============================================================================
# 18. 타이포그래피 확장 (Typography Extended) - Phase 5
# ============================================================================

## 폰트 무게 상수 (CSS font-weight 기준)
const FONT_WEIGHT_REGULAR = 400
const FONT_WEIGHT_MEDIUM = 500
const FONT_WEIGHT_SEMIBOLD = 600
const FONT_WEIGHT_BOLD = 700

## 라인 높이 상수 (스펙 3.1 기준)
const LINE_HEIGHT_H1 = 1.2  # H1 (화면 제목)
const LINE_HEIGHT_H2 = 1.3  # H2 (섹션 제목)
const LINE_HEIGHT_H3 = 1.4  # H3 (카드 제목)
const LINE_HEIGHT_BODY = 1.5  # Body (본문)
const LINE_HEIGHT_CAPTION = 1.4  # Caption (설명)
const LINE_HEIGHT_MICRO = 1.3  # Micro (레이블)

## 폰트 스타일 조합 (용도별)
const TYPOGRAPHY_H1 = {"size": FONT_H1, "weight": FONT_WEIGHT_BOLD, "line_height": LINE_HEIGHT_H1}
const TYPOGRAPHY_H2 = {"size": FONT_H2, "weight": FONT_WEIGHT_SEMIBOLD, "line_height": LINE_HEIGHT_H2}
const TYPOGRAPHY_H3 = {"size": FONT_H3, "weight": FONT_WEIGHT_MEDIUM, "line_height": LINE_HEIGHT_H3}
const TYPOGRAPHY_BODY = {"size": FONT_BODY, "weight": FONT_WEIGHT_REGULAR, "line_height": LINE_HEIGHT_BODY}
const TYPOGRAPHY_CAPTION = {"size": FONT_CAPTION, "weight": FONT_WEIGHT_REGULAR, "line_height": LINE_HEIGHT_CAPTION}
const TYPOGRAPHY_MICRO = {"size": FONT_MICRO, "weight": FONT_WEIGHT_MEDIUM, "line_height": LINE_HEIGHT_MICRO}


## 라벨에 타이포그래피 스타일 적용
func apply_typography(label: Label, style: Dictionary) -> void:
	label.add_theme_font_size_override("font_size", style.size)
	# Godot 4에서 line_height는 Theme에서 설정 필요 (런타임 제한적)
	# label.line_spacing은 추가 간격만 조절 가능


## 숫자 전용 라벨 스타일 (고정폭 폰트 필요)
func apply_number_style(label: Label, size: int = FONT_BODY) -> void:
	label.add_theme_font_size_override("font_size", size)
	if _mono_font:
		label.add_theme_font_override("font", _mono_font)


# =============================================================================
# 18.1 폰트 리소스 관리 (Font Resource Management)
# =============================================================================

## 폰트 경로 상수
const FONT_PATH_DEFAULT = "res://assets/fonts/OpenSans-VariableFont_wdth,wght.ttf"
const FONT_PATH_MONO = "res://assets/fonts/monogram.ttf"
# 추가 폰트 (향후 다운로드/설치 시)
const FONT_PATH_PRETENDARD = "res://assets/fonts/Pretendard-Regular.ttf"
const FONT_PATH_ROBOTO_MONO = "res://assets/fonts/RobotoMono-Regular.ttf"

## 로드된 폰트 캐시
var _default_font: Font = null
var _mono_font: Font = null
var _fonts_loaded: bool = false


## 폰트 초기화 (앱 시작시 호출)
func load_fonts() -> void:
	if _fonts_loaded:
		return

	# 기본 폰트 로드
	if ResourceLoader.exists(FONT_PATH_DEFAULT):
		_default_font = load(FONT_PATH_DEFAULT)
		print("[ThemeManager] Default font loaded: %s" % FONT_PATH_DEFAULT)

	# 고정폭 폰트 로드
	if ResourceLoader.exists(FONT_PATH_MONO):
		_mono_font = load(FONT_PATH_MONO)
		print("[ThemeManager] Mono font loaded: %s" % FONT_PATH_MONO)
	elif ResourceLoader.exists(FONT_PATH_ROBOTO_MONO):
		_mono_font = load(FONT_PATH_ROBOTO_MONO)
		print("[ThemeManager] Mono font loaded: %s" % FONT_PATH_ROBOTO_MONO)

	_fonts_loaded = true


## 폰트 가져오기
func get_default_font() -> Font:
	if not _fonts_loaded:
		load_fonts()
	return _default_font


func get_mono_font() -> Font:
	if not _fonts_loaded:
		load_fonts()
	return _mono_font


## 라벨에 기본 폰트 적용
func apply_default_font(label: Label) -> void:
	if _default_font:
		label.add_theme_font_override("font", _default_font)


## 라벨에 고정폭 폰트 적용
func apply_mono_font(label: Label) -> void:
	if _mono_font:
		label.add_theme_font_override("font", _mono_font)


# =============================================================================
# 18.2 테마 파일 관리 (Theme File Management)
# =============================================================================

## 테마 파일 경로
const THEME_MOBILE = "res://themes/MobileTheme.tres"
const THEME_TABLET = "res://themes/TabletTheme.tres"
const THEME_DESKTOP = "res://themes/DesktopTheme.tres"
const THEME_DESKTOP_HD = "res://themes/DesktopHDTheme.tres"

## 현재 로드된 테마
var _current_theme: Theme = null


## 테마 로드
func load_theme(theme_key: String) -> Theme:
	var path: String
	match theme_key:
		"mobile":
			path = THEME_MOBILE
		"tablet":
			path = THEME_TABLET
		"desktop":
			path = THEME_DESKTOP
		"desktop_hd":
			path = THEME_DESKTOP_HD
		_:
			path = THEME_MOBILE

	if ResourceLoader.exists(path):
		_current_theme = load(path)
		print("[ThemeManager] Theme loaded: %s" % path)
		return _current_theme
	else:
		push_warning("[ThemeManager] Theme file not found: %s" % path)
		return null


## 현재 테마 가져오기
func get_current_theme() -> Theme:
	if not _current_theme:
		load_theme(current_theme_key)
	return _current_theme


## Control에 현재 테마 적용
func apply_theme_to_control(control: Control) -> void:
	var theme = get_current_theme()
	if theme:
		control.theme = theme


# ============================================================================
# 19. 컴포넌트 사이즈 상수 (Component Sizes) - Phase 5
# ============================================================================

## 카드 사이즈 (스펙 6.2 기준) - Vector2(width, height)
const CARD_SM = Vector2(100, 140)  # 목록용
const CARD_MD = Vector2(160, 220)  # 그리드용
const CARD_LG = Vector2(280, 380)  # 상세용
const CARD_XL = Vector2(320, 440)  # 풀스크린

## 위젯 사이즈 (스펙 7.1 홈 화면 위젯)
const WIDGET_HALF = Vector2(160, 120)  # 2열 위젯
const WIDGET_FULL = Vector2(336, 120)  # 1열 위젯 (전체 너비)
const WIDGET_PADDING = SPACE_MD  # 16px

## 아이콘 사이즈
const ICON_XS = 16  # 인라인 아이콘
const ICON_SM = 20  # 버튼 내 아이콘
const ICON_MD = 24  # 네비게이션 아이콘
const ICON_LG = 32  # 강조 아이콘
const ICON_XL = 48  # 빈 상태 아이콘

## 아바타/캐릭터 사이즈
const AVATAR_SM = Vector2(40, 40)  # 리스트 아이템
const AVATAR_MD = Vector2(64, 64)  # 카드 내
const AVATAR_LG = Vector2(120, 180)  # 프로필 대형


## 카드 컨테이너에 사이즈 적용
func apply_card_size(control: Control, size_type: String) -> void:
	match size_type.to_lower():
		"sm":
			control.custom_minimum_size = CARD_SM
		"md":
			control.custom_minimum_size = CARD_MD
		"lg":
			control.custom_minimum_size = CARD_LG
		"xl":
			control.custom_minimum_size = CARD_XL
		_:
			control.custom_minimum_size = CARD_MD


# ============================================================================
# 20. 레이아웃 상수 (Layout Constants) - Phase 5
# ============================================================================

## SafeArea 상수 (스펙 5.1 기준)
const SAFE_AREA_TOP = 48  # 노치/상태바 (iPhone 기준)
const SAFE_AREA_BOTTOM = 34  # 홈 인디케이터

## 헤더/네비게이션 높이
const HEADER_HEIGHT_MIN = 80
const HEADER_HEIGHT_MAX = 120
const NAVBAR_HEIGHT = 80  # 하단 네비게이션
const NAVBAR_HEIGHT_WITH_SAFE = NAVBAR_HEIGHT + SAFE_AREA_BOTTOM  # 114px

## 뷰포트 기준 (1080x1920)
const VIEWPORT_WIDTH = 1080
const VIEWPORT_HEIGHT = 1920

## Thumb Zone 영역 (스펙 5.2)
const THUMB_EASY_HEIGHT = 300  # 하단 - Easy to Reach
const THUMB_REACH_HEIGHT = 600  # 중앙 - Reachable
const THUMB_HARD_HEIGHT = 1020  # 상단 - Hard to Reach (나머지)


## 컨텐츠 영역 계산 (SafeArea 제외)
func get_content_area_height() -> float:
	var viewport_height = get_viewport().get_visible_rect().size.y
	return viewport_height - SAFE_AREA_TOP - NAVBAR_HEIGHT_WITH_SAFE


## Thumb Zone 체크 (y 좌표가 Easy Zone인지)
func is_in_thumb_easy_zone(y_position: float) -> bool:
	var viewport_height = get_viewport().get_visible_rect().size.y
	return y_position > (viewport_height - THUMB_EASY_HEIGHT - SAFE_AREA_BOTTOM)


## 화면 마진 가져오기 (플랫폼별)
func get_screen_margin() -> int:
	match current_theme_key:
		"mobile":
			return SPACE_MD  # 16px
		"tablet":
			return SPACE_LG  # 24px
		"desktop":
			return SPACE_XL  # 32px
		"desktop_hd":
			return SPACE_XXL  # 48px
		_:
			return SPACE_MD


# ============================================================================
# 21. 데이터 시각화 스타일 (Data Visualization) - Phase 5
# ============================================================================

## 헥사곤 차트 색상 (6개 카테고리)
const HEXAGON_PACE = Color("FF6B6B")  # 속도 - 빨강
const HEXAGON_SHOOTING = Color("FFD93D")  # 슈팅 - 노랑
const HEXAGON_PASSING = Color("6BCB77")  # 패스 - 초록
const HEXAGON_DRIBBLING = Color("4D96FF")  # 드리블 - 파랑
const HEXAGON_DEFENDING = Color("9B59B6")  # 수비 - 보라
const HEXAGON_PHYSICAL = Color("FF9F43")  # 피지컬 - 오렌지

## 헥사곤 차트 스타일 상수
const HEXAGON_FILL_ALPHA = 0.3  # 채우기 투명도
const HEXAGON_STROKE_WIDTH = 2.0  # 외곽선 두께
const HEXAGON_GRID_COLOR = Color(0.3, 0.3, 0.3, 0.5)  # 격자 색상
const HEXAGON_GRID_LEVELS = 5  # 격자 단계 (20, 40, 60, 80, 100)


## 카테고리별 색상 가져오기
func get_hexagon_category_color(category: String) -> Color:
	match category.to_lower():
		"pace", "speed", "속도":
			return HEXAGON_PACE
		"shooting", "슈팅":
			return HEXAGON_SHOOTING
		"passing", "패스":
			return HEXAGON_PASSING
		"dribbling", "드리블":
			return HEXAGON_DRIBBLING
		"defending", "defense", "수비":
			return HEXAGON_DEFENDING
		"physical", "피지컬":
			return HEXAGON_PHYSICAL
		_:
			return TEXT_SECONDARY


## 헥사곤 차트 데이터 포맷 (6각형)
func create_hexagon_data(
	pace: float, shooting: float, passing: float, dribbling: float, defending: float, physical: float
) -> Array:
	return [
		{"label": "PAC", "value": pace, "color": HEXAGON_PACE},
		{"label": "SHO", "value": shooting, "color": HEXAGON_SHOOTING},
		{"label": "PAS", "value": passing, "color": HEXAGON_PASSING},
		{"label": "DRI", "value": dribbling, "color": HEXAGON_DRIBBLING},
		{"label": "DEF", "value": defending, "color": HEXAGON_DEFENDING},
		{"label": "PHY", "value": physical, "color": HEXAGON_PHYSICAL}
	]


## 막대 차트 스타일
const BAR_HEIGHT = 8  # 막대 높이
const BAR_CORNER_RADIUS = 4  # 모서리 반경
const BAR_BG_COLOR = BG_TERTIARY  # 배경색
const BAR_ANIMATION_DURATION = 0.5  # 채우기 애니메이션


## 막대 차트 StyleBox 생성
func create_stat_bar_background() -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = BAR_BG_COLOR
	style.set_corner_radius_all(BAR_CORNER_RADIUS)
	return style


func create_stat_bar_fill(value: float, max_value: float = 100.0) -> StyleBoxFlat:
	var style = StyleBoxFlat.new()
	style.bg_color = get_stat_color(value, max_value)
	style.set_corner_radius_all(BAR_CORNER_RADIUS)
	return style


## 원형 진행률 스타일
const CIRCULAR_STROKE_WIDTH = 8.0
const CIRCULAR_BG_COLOR = BG_TERTIARY
const CIRCULAR_START_ANGLE = -90.0  # 12시 방향에서 시작


## 원형 진행률 그리기 파라미터
func get_circular_progress_params(value: float, max_value: float = 100.0) -> Dictionary:
	var ratio = clamp(value / max_value, 0.0, 1.0)
	return {
		"start_angle": deg_to_rad(CIRCULAR_START_ANGLE),
		"end_angle": deg_to_rad(CIRCULAR_START_ANGLE + (360.0 * ratio)),
		"stroke_width": CIRCULAR_STROKE_WIDTH,
		"bg_color": CIRCULAR_BG_COLOR,
		"fill_color": get_stat_color(value, max_value)
	}
