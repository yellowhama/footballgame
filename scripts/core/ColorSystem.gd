extends Node
# 통합 색상 시스템 - ART_UI_COMPLETE_GUIDE.md 준수

# Kairosoft Style Pastels
const KAIRO_GREEN = Color("#a8e6cf")
const KAIRO_BLUE = Color("#7fcdff")
const KAIRO_YELLOW = Color("#ffd3b6")
const KAIRO_PINK = Color("#ffaaa5")
const KAIRO_PURPLE = Color("#d4a5ff")

# Mobile-friendly High Contrast Colors
const MOBILE_SUCCESS = Color("#2E7D32")
const MOBILE_WARNING = Color("#F57F17")
const MOBILE_DANGER = Color("#C62828")
const MOBILE_INFO = Color("#1565C0")

# Position Colors (FM Mobile Standard)
# GK: Yellow, DF: Blue, MF: Green, FW: Red
const POSITION_GK = Color("#FFD700")  # Goalkeeper - Yellow
const POSITION_DF = Color("#2196F3")  # Defender - Blue
const POSITION_MF = Color("#4CAF50")  # Midfielder - Green
const POSITION_FW = Color("#F44336")  # Forward - Red
# Legacy aliases for compatibility
const POSITION_ST = POSITION_FW
const POSITION_CAM = Color("#8BC34A")  # Attacking Mid - Light Green
const POSITION_CM = POSITION_MF
const POSITION_CB = POSITION_DF

# Condition Colors (Colorblind-friendly)
const CONDITION_PERFECT = Color("#2E7D32")  # Dark Green
const CONDITION_GOOD = Color("#689F38")  # Olive
const CONDITION_NORMAL = Color("#F57F17")  # Orange
const CONDITION_POOR = Color("#E64A19")  # Red-Orange
const CONDITION_TERRIBLE = Color("#C62828")  # Dark Red

# UI Component Colors
const CARD_BACKGROUND = Color(0.15, 0.15, 0.2, 1.0)
const CARD_BORDER = Color(0.3, 0.3, 0.4, 1.0)
const CARD_HOVER = Color(0.2, 0.2, 0.25, 1.0)
const CARD_SELECTED = Color(0.25, 0.25, 0.35, 1.0)

# Background Colors
const BACKGROUND_DARK = Color(0.07, 0.07, 0.1, 1.0)
const BACKGROUND_LIGHT = Color(0.9, 0.9, 0.95, 1.0)

# Text Colors
const TEXT_PRIMARY = Color(1.0, 1.0, 1.0, 1.0)
const TEXT_SECONDARY = Color(0.8, 0.8, 0.8, 1.0)
const TEXT_DISABLED = Color(0.5, 0.5, 0.5, 1.0)

# Colorblind-friendly Colors
const COLORBLIND_SUCCESS = Color(0.0, 0.4, 0.8)  # 파랑 (성공)
const COLORBLIND_WARNING = Color(1.0, 0.667, 0.0)  # 주황 (경고)
const COLORBLIND_DANGER = Color(0.667, 0.0, 0.667)  # 보라 (위험)
const COLORBLIND_NEUTRAL = Color(0.6, 0.6, 0.6)  # 회색 (중립)


static func get_condition_color(level: int) -> Color:
	match level:
		5:
			return CONDITION_PERFECT  # PERFECT
		4:
			return CONDITION_GOOD  # GOOD
		3:
			return CONDITION_NORMAL  # NORMAL
		2:
			return CONDITION_POOR  # POOR
		1:
			return CONDITION_TERRIBLE  # TERRIBLE
		_:
			return CONDITION_NORMAL


static func get_fatigue_color(fatigue_value: float) -> Color:
	if fatigue_value >= 80:
		return MOBILE_DANGER
	elif fatigue_value >= 60:
		return MOBILE_WARNING
	else:
		return MOBILE_SUCCESS


static func get_colorblind_safe_color(type: String) -> Color:
	match type:
		"success":
			return COLORBLIND_SUCCESS
		"warning":
			return COLORBLIND_WARNING
		"danger":
			return COLORBLIND_DANGER
		"neutral":
			return COLORBLIND_NEUTRAL
		_:
			return TEXT_PRIMARY


static func get_position_color(position: String) -> Color:
	## FM Mobile Standard Position Colors
	## GK: Yellow, DF: Blue, MF: Green, FW: Red
	match position.to_upper():
		# Goalkeeper
		"GK":
			return POSITION_GK
		# Defenders
		"CB", "LB", "RB", "LWB", "RWB", "SW":
			return POSITION_DF
		# Midfielders
		"CM", "CDM", "DM", "LM", "RM":
			return POSITION_MF
		# Attacking Midfielders (Lighter Green)
		"CAM", "AM":
			return POSITION_CAM
		# Forwards
		"ST", "CF", "LW", "RW", "SS":
			return POSITION_FW
		_:
			return TEXT_SECONDARY


## Get position category (for grouping)
static func get_position_category(position: String) -> String:
	match position.to_upper():
		"GK":
			return "GK"
		"CB", "LB", "RB", "LWB", "RWB", "SW":
			return "DF"
		"CM", "CDM", "DM", "LM", "RM", "CAM", "AM":
			return "MF"
		"ST", "CF", "LW", "RW", "SS":
			return "FW"
		_:
			return "MF"


## Get position abbreviation color formatted for RichTextLabel
static func get_position_bbcode(position: String) -> String:
	var color := get_position_color(position)
	var hex := "#%02x%02x%02x" % [int(color.r * 255), int(color.g * 255), int(color.b * 255)]
	return "[color=%s]%s[/color]" % [hex, position.to_upper()]


# ============================================
# P0: Mood System (Uma Musume inspired)
# 5-level mood: Awful → Bad → Normal → Good → Great
# Affects training efficiency: 80% → 90% → 100% → 110% → 120%
# ============================================

## Mood level enum
enum MoodLevel { AWFUL = 1, BAD = 2, NORMAL = 3, GOOD = 4, GREAT = 5 }

## Mood colors (visually distinct, colorblind-friendly)
const MOOD_AWFUL_COLOR = Color("#8B0000")  # Dark Red
const MOOD_BAD_COLOR = Color("#CD5C5C")  # Indian Red
const MOOD_NORMAL_COLOR = Color("#808080")  # Gray
const MOOD_GOOD_COLOR = Color("#32CD32")  # Lime Green
const MOOD_GREAT_COLOR = Color("#FFD700")  # Gold

## Mood icons (emoji-style)
const MOOD_AWFUL_ICON = "😢"
const MOOD_BAD_ICON = "😕"
const MOOD_NORMAL_ICON = "😐"
const MOOD_GOOD_ICON = "😊"
const MOOD_GREAT_ICON = "🤩"

## Training efficiency multipliers
const MOOD_AWFUL_MULT = 0.80
const MOOD_BAD_MULT = 0.90
const MOOD_NORMAL_MULT = 1.00
const MOOD_GOOD_MULT = 1.10
const MOOD_GREAT_MULT = 1.20


## Get mood color from level (1-5)
static func get_mood_color(level: int) -> Color:
	match level:
		1:
			return MOOD_AWFUL_COLOR
		2:
			return MOOD_BAD_COLOR
		3:
			return MOOD_NORMAL_COLOR
		4:
			return MOOD_GOOD_COLOR
		5:
			return MOOD_GREAT_COLOR
		_:
			return MOOD_NORMAL_COLOR


## Get mood icon from level (1-5)
static func get_mood_icon(level: int) -> String:
	match level:
		1:
			return MOOD_AWFUL_ICON
		2:
			return MOOD_BAD_ICON
		3:
			return MOOD_NORMAL_ICON
		4:
			return MOOD_GOOD_ICON
		5:
			return MOOD_GREAT_ICON
		_:
			return MOOD_NORMAL_ICON


## Get mood label from level (1-5)
static func get_mood_label(level: int) -> String:
	match level:
		1:
			return "최악"
		2:
			return "나쁨"
		3:
			return "보통"
		4:
			return "좋음"
		5:
			return "최고"
		_:
			return "보통"


## Get training efficiency multiplier from mood level
static func get_mood_multiplier(level: int) -> float:
	match level:
		1:
			return MOOD_AWFUL_MULT
		2:
			return MOOD_BAD_MULT
		3:
			return MOOD_NORMAL_MULT
		4:
			return MOOD_GOOD_MULT
		5:
			return MOOD_GREAT_MULT
		_:
			return MOOD_NORMAL_MULT


## Get formatted mood display string with color
static func get_mood_display(level: int) -> String:
	var icon := get_mood_icon(level)
	var label := get_mood_label(level)
	var color := get_mood_color(level)
	var hex := "#%02x%02x%02x" % [int(color.r * 255), int(color.g * 255), int(color.b * 255)]
	return "%s [color=%s]%s[/color]" % [icon, hex, label]


## Get mood efficiency percentage string (e.g., "110%")
static func get_mood_efficiency_text(level: int) -> String:
	var mult := get_mood_multiplier(level)
	return "%d%%" % int(mult * 100)


## Get BBCode formatted mood bar (visual 5-segment indicator)
static func get_mood_bar_bbcode(level: int) -> String:
	var segments := ""
	for i in range(1, 6):
		var segment_color: Color
		if i <= level:
			segment_color = get_mood_color(level)
		else:
			segment_color = Color(0.3, 0.3, 0.3)  # Dim gray for unfilled
		var hex := (
			"#%02x%02x%02x" % [int(segment_color.r * 255), int(segment_color.g * 255), int(segment_color.b * 255)]
		)
		segments += "[color=%s]■[/color]" % hex
	return segments


# ============================================
# P2: Status Effect System (Uma Musume inspired)
# Negative status effects affecting training/match
# NOTE: "StatusEffect" is different from "Condition" (ConditionSystem.gd)
#   - StatusEffect = 상태이상 (Fatigue, Slump, Injury Risk)
#   - Condition = 컨디션 레벨 (절호조~절부진, handled by ConditionSystem)
# - Fatigue: Training efficiency -20%
# - Slump: Match ability -10%
# - Injury Risk: Injury probability 2x
# ============================================

## Status effect type enum (상태이상)
enum StatusEffect { NONE = 0, FATIGUE = 1, SLUMP = 2, INJURY_RISK = 3 }
## Alias for backward compatibility
const ConditionType = StatusEffect

## Status effect colors (distinct, colorblind-friendly with patterns)
const STATUS_FATIGUE_COLOR = Color("#FF8C00")  # Dark Orange
const STATUS_SLUMP_COLOR = Color("#6A5ACD")  # Slate Blue
const STATUS_INJURY_RISK_COLOR = Color("#DC143C")  # Crimson
const STATUS_NONE_COLOR = Color("#32CD32")  # Lime Green (healthy)

## Status effect icons (emoji-style status indicators)
const STATUS_FATIGUE_ICON = "😴"
const STATUS_SLUMP_ICON = "😞"
const STATUS_INJURY_RISK_ICON = "⚠️"
const STATUS_NONE_ICON = "✓"

## Effect multipliers/modifiers
const STATUS_FATIGUE_TRAINING_MULT = 0.80  # Training efficiency -20%
const STATUS_SLUMP_MATCH_MULT = 0.90  # Match ability -10%
const STATUS_INJURY_RISK_PROBABILITY_MULT = 2.0  # Injury chance x2


## Get status effect color from type
static func get_status_effect_color(effect: int) -> Color:
	match effect:
		StatusEffect.FATIGUE:
			return STATUS_FATIGUE_COLOR
		StatusEffect.SLUMP:
			return STATUS_SLUMP_COLOR
		StatusEffect.INJURY_RISK:
			return STATUS_INJURY_RISK_COLOR
		StatusEffect.NONE, _:
			return STATUS_NONE_COLOR


## Get status effect icon from type
static func get_status_effect_icon(effect: int) -> String:
	match effect:
		StatusEffect.FATIGUE:
			return STATUS_FATIGUE_ICON
		StatusEffect.SLUMP:
			return STATUS_SLUMP_ICON
		StatusEffect.INJURY_RISK:
			return STATUS_INJURY_RISK_ICON
		StatusEffect.NONE, _:
			return STATUS_NONE_ICON


## Get status effect label from type
static func get_status_effect_label(effect: int) -> String:
	match effect:
		StatusEffect.FATIGUE:
			return "피로"
		StatusEffect.SLUMP:
			return "슬럼프"
		StatusEffect.INJURY_RISK:
			return "부상위험"
		StatusEffect.NONE, _:
			return "정상"


## Get status effect description
static func get_status_effect_text(effect: int) -> String:
	match effect:
		StatusEffect.FATIGUE:
			return "훈련 효율 -20%"
		StatusEffect.SLUMP:
			return "경기 능력 -10%"
		StatusEffect.INJURY_RISK:
			return "부상 확률 2배"
		StatusEffect.NONE, _:
			return ""


## Get training efficiency multiplier from status effect
static func get_status_training_mult(effect: int) -> float:
	if effect == StatusEffect.FATIGUE:
		return STATUS_FATIGUE_TRAINING_MULT
	return 1.0


## Get match ability multiplier from status effect
static func get_status_match_mult(effect: int) -> float:
	if effect == StatusEffect.SLUMP:
		return STATUS_SLUMP_MATCH_MULT
	return 1.0


## Get injury probability multiplier from status effect
static func get_status_injury_mult(effect: int) -> float:
	if effect == StatusEffect.INJURY_RISK:
		return STATUS_INJURY_RISK_PROBABILITY_MULT
	return 1.0


## Get formatted status effect display string with color
static func get_status_effect_display(effect: int) -> String:
	var icon := get_status_effect_icon(effect)
	var label := get_status_effect_label(effect)
	var color := get_status_effect_color(effect)
	var hex := "#%02x%02x%02x" % [int(color.r * 255), int(color.g * 255), int(color.b * 255)]
	if effect == StatusEffect.NONE:
		return "%s [color=%s]%s[/color]" % [icon, hex, label]
	var desc := get_status_effect_text(effect)
	return "%s [color=%s]%s[/color] (%s)" % [icon, hex, label, desc]


## Get all active status effects as a list of display strings
static func get_status_effects_display_list(effects: Array) -> Array:
	var result := []
	for e in effects:
		if e != StatusEffect.NONE:
			result.append(get_status_effect_display(e))
	return result


## Check if status effect affects training
static func status_affects_training(effect: int) -> bool:
	return effect == StatusEffect.FATIGUE


## Check if status effect affects match
static func status_affects_match(effect: int) -> bool:
	return effect == StatusEffect.SLUMP


## Check if status effect affects injury risk
static func status_affects_injury(effect: int) -> bool:
	return effect == StatusEffect.INJURY_RISK


## Calculate combined training multiplier from mood and status effects
static func get_combined_training_mult(mood_level: int, effects: Array) -> float:
	var mult := get_mood_multiplier(mood_level)
	for e in effects:
		mult *= get_status_training_mult(e)
	return mult


## Calculate combined match multiplier from mood and status effects
static func get_combined_match_mult(mood_level: int, effects: Array) -> float:
	# Mood affects training primarily, but can also affect match
	var mult := 1.0
	for e in effects:
		mult *= get_status_match_mult(e)
	return mult


## Get BBCode status summary (mood + status effects combined)
static func get_player_status_bbcode(mood_level: int, effects: Array) -> String:
	var parts := []
	# Mood display
	parts.append(get_mood_display(mood_level))
	# Status effects display
	for e in effects:
		if e != StatusEffect.NONE:
			var icon := get_status_effect_icon(e)
			var label := get_status_effect_label(e)
			var color := get_status_effect_color(e)
			var hex := "#%02x%02x%02x" % [int(color.r * 255), int(color.g * 255), int(color.b * 255)]
			parts.append("%s[color=%s]%s[/color]" % [icon, hex, label])
	return " | ".join(parts)


# ============================================
# Backward compatibility aliases (deprecated)
# ============================================
static func get_condition_type_color(c: int) -> Color:
	return get_status_effect_color(c)


static func get_condition_icon(c: int) -> String:
	return get_status_effect_icon(c)


static func get_condition_label(c: int) -> String:
	return get_status_effect_label(c)


static func get_condition_effect_text(c: int) -> String:
	return get_status_effect_text(c)


static func get_condition_training_mult(c: int) -> float:
	return get_status_training_mult(c)


static func get_condition_match_mult(c: int) -> float:
	return get_status_match_mult(c)


static func get_condition_injury_mult(c: int) -> float:
	return get_status_injury_mult(c)


static func get_condition_display(c: int) -> String:
	return get_status_effect_display(c)


static func get_conditions_display_list(arr: Array) -> Array:
	return get_status_effects_display_list(arr)


static func condition_affects_training(c: int) -> bool:
	return status_affects_training(c)


static func condition_affects_match(c: int) -> bool:
	return status_affects_match(c)


static func condition_affects_injury(c: int) -> bool:
	return status_affects_injury(c)


# ============================================
# P2: Intensive Week (Fever/Burst) System
# Kairosoft-style "Aura/Fever" mechanic
# - Focus gauge builds from training/match performance
# - When full, player can activate "Intensive Week"
# - During Intensive Week: +50% training efficiency OR 2x great success chance
# ============================================

## Focus gauge constants
const FOCUS_GAUGE_MAX = 100.0
const FOCUS_GAUGE_TRAINING_GAIN = 10.0  # Per training session
const FOCUS_GAUGE_MATCH_WIN_GAIN = 20.0  # Per match win
const FOCUS_GAUGE_MATCH_DRAW_GAIN = 10.0  # Per match draw
const FOCUS_GAUGE_GREAT_SUCCESS_GAIN = 15.0  # Per training great success
const FOCUS_GAUGE_GOAL_GAIN = 5.0  # Per goal scored in match

## Intensive Week bonuses
const INTENSIVE_WEEK_TRAINING_MULT = 1.50  # +50% training efficiency
const INTENSIVE_WEEK_GREAT_SUCCESS_MULT = 2.0  # 2x great success probability

## Focus gauge colors (gradient from empty to full)
const FOCUS_GAUGE_EMPTY_COLOR = Color("#4A4A4A")  # Dark gray
const FOCUS_GAUGE_LOW_COLOR = Color("#5C8DFF")  # Light blue
const FOCUS_GAUGE_MID_COLOR = Color("#FFB347")  # Orange
const FOCUS_GAUGE_HIGH_COLOR = Color("#FF6B6B")  # Red
const FOCUS_GAUGE_FULL_COLOR = Color("#FFD700")  # Gold (ready to activate)
const INTENSIVE_WEEK_ACTIVE_COLOR = Color("#FF4500")  # Orange-Red (fire)

## Focus gauge icons
const FOCUS_GAUGE_ICON = "🔥"
const INTENSIVE_WEEK_ICON = "⚡"
const FOCUS_GAUGE_READY_ICON = "✨"


## Get focus gauge color based on current value (0-100)
static func get_focus_gauge_color(value: float) -> Color:
	var normalized := clampf(value / FOCUS_GAUGE_MAX, 0.0, 1.0)
	if normalized >= 1.0:
		return FOCUS_GAUGE_FULL_COLOR
	elif normalized >= 0.75:
		return FOCUS_GAUGE_HIGH_COLOR
	elif normalized >= 0.50:
		return FOCUS_GAUGE_MID_COLOR
	elif normalized >= 0.25:
		return FOCUS_GAUGE_LOW_COLOR
	else:
		return FOCUS_GAUGE_EMPTY_COLOR


## Get focus gauge label
static func get_focus_gauge_label(value: float, is_intensive_active: bool = false) -> String:
	if is_intensive_active:
		return "집중 훈련 중!"
	elif value >= FOCUS_GAUGE_MAX:
		return "집중 준비 완료!"
	else:
		return "집중 게이지"


## Get focus gauge percentage string
static func get_focus_gauge_percentage(value: float) -> String:
	return "%d%%" % int(clampf(value, 0.0, FOCUS_GAUGE_MAX))


## Get focus gauge icon based on state
static func get_focus_gauge_icon(value: float, is_intensive_active: bool = false) -> String:
	if is_intensive_active:
		return INTENSIVE_WEEK_ICON
	elif value >= FOCUS_GAUGE_MAX:
		return FOCUS_GAUGE_READY_ICON
	else:
		return FOCUS_GAUGE_ICON


## Get BBCode formatted focus gauge bar (visual indicator)
static func get_focus_gauge_bar_bbcode(value: float, is_intensive_active: bool = false) -> String:
	var segments := ""
	var normalized := clampf(value / FOCUS_GAUGE_MAX, 0.0, 1.0)
	var filled_count := int(normalized * 10)  # 10-segment bar

	var fill_color: Color
	if is_intensive_active:
		fill_color = INTENSIVE_WEEK_ACTIVE_COLOR
	else:
		fill_color = get_focus_gauge_color(value)

	var fill_hex := "#%02x%02x%02x" % [int(fill_color.r * 255), int(fill_color.g * 255), int(fill_color.b * 255)]
	var empty_hex := (
		"#%02x%02x%02x"
		% [
			int(FOCUS_GAUGE_EMPTY_COLOR.r * 255),
			int(FOCUS_GAUGE_EMPTY_COLOR.g * 255),
			int(FOCUS_GAUGE_EMPTY_COLOR.b * 255)
		]
	)

	for i in range(10):
		if i < filled_count:
			segments += "[color=%s]█[/color]" % fill_hex
		else:
			segments += "[color=%s]░[/color]" % empty_hex

	return segments


## Get intensive week display string with icon and status
static func get_intensive_week_display(value: float, is_active: bool = false) -> String:
	var icon := get_focus_gauge_icon(value, is_active)
	var label := get_focus_gauge_label(value, is_active)
	var color := get_focus_gauge_color(value) if not is_active else INTENSIVE_WEEK_ACTIVE_COLOR
	var hex := "#%02x%02x%02x" % [int(color.r * 255), int(color.g * 255), int(color.b * 255)]

	if is_active:
		return "%s [color=%s]%s[/color] (훈련 +50%%)" % [icon, hex, label]
	elif value >= FOCUS_GAUGE_MAX:
		return "%s [color=%s]%s[/color] (발동 가능)" % [icon, hex, label]
	else:
		return "%s [color=%s]%s[/color] (%s)" % [icon, hex, label, get_focus_gauge_percentage(value)]


## Get training multiplier when intensive week is active
static func get_intensive_training_mult(is_active: bool) -> float:
	return INTENSIVE_WEEK_TRAINING_MULT if is_active else 1.0


## Get great success probability multiplier when intensive week is active
static func get_intensive_great_success_mult(is_active: bool) -> float:
	return INTENSIVE_WEEK_GREAT_SUCCESS_MULT if is_active else 1.0


## Calculate focus gauge gain from various sources
static func get_focus_gain_for_event(event_type: String) -> float:
	match event_type:
		"training":
			return FOCUS_GAUGE_TRAINING_GAIN
		"match_win":
			return FOCUS_GAUGE_MATCH_WIN_GAIN
		"match_draw":
			return FOCUS_GAUGE_MATCH_DRAW_GAIN
		"great_success":
			return FOCUS_GAUGE_GREAT_SUCCESS_GAIN
		"goal":
			return FOCUS_GAUGE_GOAL_GAIN
		_:
			return 0.0


## Check if intensive week can be activated
static func can_activate_intensive_week(gauge_value: float, is_already_active: bool) -> bool:
	return gauge_value >= FOCUS_GAUGE_MAX and not is_already_active


## Get full intensive week status for UI
static func get_intensive_week_status(gauge_value: float, is_active: bool, remaining_days: int = 0) -> Dictionary:
	return {
		"gauge": gauge_value,
		"max_gauge": FOCUS_GAUGE_MAX,
		"is_active": is_active,
		"can_activate": can_activate_intensive_week(gauge_value, is_active),
		"remaining_days": remaining_days if is_active else 0,
		"training_bonus": INTENSIVE_WEEK_TRAINING_MULT if is_active else 1.0,
		"great_success_bonus": INTENSIVE_WEEK_GREAT_SUCCESS_MULT if is_active else 1.0,
		"display": get_intensive_week_display(gauge_value, is_active),
		"bar_bbcode": get_focus_gauge_bar_bbcode(gauge_value, is_active),
		"color": get_focus_gauge_color(gauge_value) if not is_active else INTENSIVE_WEEK_ACTIVE_COLOR,
		"icon": get_focus_gauge_icon(gauge_value, is_active)
	}


# ==================== TRAINING CARD COMBO SYSTEM ====================
# Nintendo Pocket Football Club 영감
# 최대 3장 카드 조합 → 특수 효과 발동
# 콤보 발견 시 도감 등록 + 숨겨진 Trait 해금 가능

## Training card types (훈련 카드 종류)
enum TrainingCardType {
	NONE = 0,
	# Physical Training
	SPEED = 1,
	STAMINA = 2,
	STRENGTH = 3,
	AGILITY = 4,
	# Technical Training
	PASSING = 5,
	SHOOTING = 6,
	DRIBBLING = 7,
	BALL_CONTROL = 8,
	# Tactical Training
	POSITIONING = 9,
	VISION = 10,
	TEAMWORK = 11,
	DECISION = 12,
	# Mental Training
	COMPOSURE = 13,
	CONCENTRATION = 14,
	LEADERSHIP = 15,
	DETERMINATION = 16,
	# Special Training
	MINI_GAME = 17,
	MATCH_PREP = 18,
	RECOVERY = 19,
	SCRIMMAGE = 20
}

## Combo rarity levels
enum ComboRarity { COMMON = 0, RARE = 1, LEGENDARY = 2 }  # 2카드 조합, 기본 효과  # 3카드 조합, 향상된 효과  # 특수 3카드 조합, 최고 효과 + Trait 해금

## Combo colors by rarity
const COMBO_COMMON_COLOR = Color("#4A90D9")  # Blue
const COMBO_RARE_COLOR = Color("#9B59B6")  # Purple
const COMBO_LEGENDARY_COLOR = Color("#FFD700")  # Gold

## Combo effect icons
const COMBO_ICON_COMMON = "💫"
const COMBO_ICON_RARE = "✨"
const COMBO_ICON_LEGENDARY = "🌟"

## Pre-defined combo recipes (카드 조합 레시피)
## Format: { "name": String, "cards": Array[TrainingCardType], "rarity": ComboRarity,
##           "effect": String, "stat_bonus": Dictionary, "unlock_trait": String (optional) }
const TRAINING_COMBOS = [
	# === COMMON COMBOS (2 cards) ===
	{
		"id": "speed_burst",
		"name": "Speed Burst",
		"name_kr": "스피드 버스트",
		"cards": [1, 4],  # SPEED + AGILITY
		"rarity": 0,  # COMMON
		"effect": "Acceleration boost",
		"stat_bonus": {"speed": 3, "agility": 2},
		"unlock_trait": ""
	},
	{
		"id": "power_shot",
		"name": "Power Shot",
		"name_kr": "파워 슛",
		"cards": [3, 6],  # STRENGTH + SHOOTING
		"rarity": 0,
		"effect": "Shot power boost",
		"stat_bonus": {"shooting": 3, "strength": 2},
		"unlock_trait": ""
	},
	{
		"id": "precision_pass",
		"name": "Precision Pass",
		"name_kr": "정밀 패스",
		"cards": [5, 10],  # PASSING + VISION
		"rarity": 0,
		"effect": "Pass accuracy boost",
		"stat_bonus": {"passing": 3, "vision": 2},
		"unlock_trait": ""
	},
	{
		"id": "ball_mastery",
		"name": "Ball Mastery",
		"name_kr": "볼 마스터리",
		"cards": [7, 8],  # DRIBBLING + BALL_CONTROL
		"rarity": 0,
		"effect": "Ball handling boost",
		"stat_bonus": {"dribbling": 3, "ball_control": 2},
		"unlock_trait": ""
	},
	{
		"id": "tactical_mind",
		"name": "Tactical Mind",
		"name_kr": "전술적 사고",
		"cards": [9, 12],  # POSITIONING + DECISION
		"rarity": 0,
		"effect": "Decision making boost",
		"stat_bonus": {"positioning": 2, "decision": 3},
		"unlock_trait": ""
	},
	{
		"id": "mental_fortress",
		"name": "Mental Fortress",
		"name_kr": "멘탈 요새",
		"cards": [13, 14],  # COMPOSURE + CONCENTRATION
		"rarity": 0,
		"effect": "Mental stability boost",
		"stat_bonus": {"composure": 3, "concentration": 2},
		"unlock_trait": ""
	},
	# === RARE COMBOS (3 cards) ===
	{
		"id": "flowing_football",
		"name": "Flowing Football",
		"name_kr": "흐르는 축구",
		"cards": [17, 5, 4],  # MINI_GAME + PASSING + AGILITY
		"rarity": 1,  # RARE
		"effect": "Tiki-taka style play",
		"stat_bonus": {"passing": 4, "agility": 3, "vision": 3, "teamwork": 3},
		"unlock_trait": ""
	},
	{
		"id": "counter_master",
		"name": "Counter Master",
		"name_kr": "역습의 달인",
		"cards": [1, 9, 12],  # SPEED + POSITIONING + DECISION
		"rarity": 1,
		"effect": "Counter attack specialist",
		"stat_bonus": {"speed": 4, "positioning": 3, "decision": 3},
		"unlock_trait": ""
	},
	{
		"id": "target_man",
		"name": "Target Man",
		"name_kr": "타겟맨",
		"cards": [3, 8, 6],  # STRENGTH + BALL_CONTROL + SHOOTING
		"rarity": 1,
		"effect": "Hold-up play specialist",
		"stat_bonus": {"strength": 4, "ball_control": 3, "shooting": 3},
		"unlock_trait": ""
	},
	{
		"id": "playmaker_vision",
		"name": "Playmaker Vision",
		"name_kr": "플레이메이커 시야",
		"cards": [10, 5, 14],  # VISION + PASSING + CONCENTRATION
		"rarity": 1,
		"effect": "Creative playmaking",
		"stat_bonus": {"vision": 4, "passing": 4, "concentration": 2},
		"unlock_trait": ""
	},
	{
		"id": "defensive_wall",
		"name": "Defensive Wall",
		"name_kr": "수비 장벽",
		"cards": [9, 13, 11],  # POSITIONING + COMPOSURE + TEAMWORK
		"rarity": 1,
		"effect": "Solid defensive formation",
		"stat_bonus": {"positioning": 4, "composure": 3, "teamwork": 3},
		"unlock_trait": ""
	},
	# === LEGENDARY COMBOS (3 cards, unlocks trait) ===
	{
		"id": "total_football",
		"name": "Total Football",
		"name_kr": "토탈 풋볼",
		"cards": [11, 9, 10],  # TEAMWORK + POSITIONING + VISION
		"rarity": 2,  # LEGENDARY
		"effect": "Position interchangeability",
		"stat_bonus": {"teamwork": 5, "positioning": 4, "vision": 4, "passing": 3},
		"unlock_trait": "versatile"
	},
	{
		"id": "clinical_finisher",
		"name": "Clinical Finisher",
		"name_kr": "임상적 피니셔",
		"cards": [6, 13, 14],  # SHOOTING + COMPOSURE + CONCENTRATION
		"rarity": 2,
		"effect": "Ice cold finishing",
		"stat_bonus": {"shooting": 5, "composure": 4, "concentration": 4},
		"unlock_trait": "clinical"
	},
	{
		"id": "engine_room",
		"name": "Engine Room",
		"name_kr": "엔진룸",
		"cards": [2, 11, 16],  # STAMINA + TEAMWORK + DETERMINATION
		"rarity": 2,
		"effect": "Tireless midfield presence",
		"stat_bonus": {"stamina": 5, "teamwork": 4, "determination": 4},
		"unlock_trait": "tireless"
	},
	{
		"id": "captain_material",
		"name": "Captain Material",
		"name_kr": "캡틴 자질",
		"cards": [15, 13, 16],  # LEADERSHIP + COMPOSURE + DETERMINATION
		"rarity": 2,
		"effect": "Born leader",
		"stat_bonus": {"leadership": 5, "composure": 4, "determination": 4},
		"unlock_trait": "captain"
	},
	{
		"id": "technical_genius",
		"name": "Technical Genius",
		"name_kr": "기술적 천재",
		"cards": [7, 8, 10],  # DRIBBLING + BALL_CONTROL + VISION
		"rarity": 2,
		"effect": "Exceptional technical ability",
		"stat_bonus": {"dribbling": 5, "ball_control": 5, "vision": 3},
		"unlock_trait": "technical_genius"
	}
]


## Get combo color by rarity
static func get_combo_color(rarity: int) -> Color:
	match rarity:
		0:
			return COMBO_COMMON_COLOR
		1:
			return COMBO_RARE_COLOR
		2:
			return COMBO_LEGENDARY_COLOR
		_:
			return COMBO_COMMON_COLOR


## Get combo icon by rarity
static func get_combo_icon(rarity: int) -> String:
	match rarity:
		0:
			return COMBO_ICON_COMMON
		1:
			return COMBO_ICON_RARE
		2:
			return COMBO_ICON_LEGENDARY
		_:
			return COMBO_ICON_COMMON


## Get combo rarity label
static func get_combo_rarity_label(rarity: int) -> String:
	match rarity:
		0:
			return "Common"
		1:
			return "Rare"
		2:
			return "Legendary"
		_:
			return "Unknown"


## Get combo rarity label (Korean)
static func get_combo_rarity_label_kr(rarity: int) -> String:
	match rarity:
		0:
			return "일반"
		1:
			return "레어"
		2:
			return "전설"
		_:
			return "알 수 없음"


## Find matching combo from selected cards
## Returns combo dictionary or empty dict if no match
static func find_combo(selected_cards: Array) -> Dictionary:
	if selected_cards.size() < 2 or selected_cards.size() > 3:
		return {}

	# Sort cards for consistent matching
	var sorted_cards = selected_cards.duplicate()
	sorted_cards.sort()

	for combo in TRAINING_COMBOS:
		var combo_cards = combo["cards"].duplicate()
		combo_cards.sort()

		if sorted_cards == combo_cards:
			return combo

	return {}


## Check if cards can potentially form a combo (partial match)
static func check_partial_combo(selected_cards: Array) -> Array:
	if selected_cards.size() < 1 or selected_cards.size() > 2:
		return []

	var potential_combos = []
	var sorted_selected = selected_cards.duplicate()
	sorted_selected.sort()

	for combo in TRAINING_COMBOS:
		var combo_cards = combo["cards"].duplicate()
		combo_cards.sort()

		# Check if selected cards are subset of combo cards
		var is_subset = true
		for card in sorted_selected:
			if not combo_cards.has(card):
				is_subset = false
				break

		if is_subset:
			potential_combos.append(combo)

	return potential_combos


## Get BBCode formatted combo display
static func get_combo_bbcode(combo: Dictionary) -> String:
	if combo.is_empty():
		return ""

	var color = get_combo_color(combo["rarity"])
	var icon = get_combo_icon(combo["rarity"])
	var rarity_label = get_combo_rarity_label(combo["rarity"])

	return (
		"%s [color=#%s][b]%s[/b][/color] (%s)\n%s"
		% [icon, color.to_html(false), combo["name"], rarity_label, combo["effect"]]
	)


## Get combo discovery popup data
static func get_combo_discovery_display(combo: Dictionary) -> Dictionary:
	if combo.is_empty():
		return {}

	var rarity = combo["rarity"]

	return {
		"title": "🎉 콤보 발견! 🎉" if rarity < 2 else "🌟 전설의 콤보 발견! 🌟",
		"name": combo["name"],
		"name_kr": combo["name_kr"],
		"effect": combo["effect"],
		"icon": get_combo_icon(rarity),
		"color": get_combo_color(rarity),
		"rarity_label": get_combo_rarity_label_kr(rarity),
		"stat_bonus": combo["stat_bonus"],
		"unlock_trait": combo.get("unlock_trait", ""),
		"show_flash": rarity >= 1,  # Rare+ shows screen flash
		"show_particles": rarity >= 2  # Legendary shows particles
	}


## Calculate total stat bonus from combo
static func get_combo_stat_bonus_total(combo: Dictionary) -> int:
	if combo.is_empty() or not combo.has("stat_bonus"):
		return 0

	var total = 0
	for stat in combo["stat_bonus"]:
		total += combo["stat_bonus"][stat]
	return total


## Get all combos by rarity
static func get_combos_by_rarity(rarity: int) -> Array:
	var result = []
	for combo in TRAINING_COMBOS:
		if combo["rarity"] == rarity:
			result.append(combo)
	return result


## Get combo count by rarity (for collection UI)
static func get_combo_count_by_rarity() -> Dictionary:
	var counts = {0: 0, 1: 0, 2: 0}
	for combo in TRAINING_COMBOS:
		counts[combo["rarity"]] += 1
	return counts


## Get training card type name
static func get_training_card_name(card_type: int) -> String:
	match card_type:
		1:
			return "Speed"
		2:
			return "Stamina"
		3:
			return "Strength"
		4:
			return "Agility"
		5:
			return "Passing"
		6:
			return "Shooting"
		7:
			return "Dribbling"
		8:
			return "Ball Control"
		9:
			return "Positioning"
		10:
			return "Vision"
		11:
			return "Teamwork"
		12:
			return "Decision"
		13:
			return "Composure"
		14:
			return "Concentration"
		15:
			return "Leadership"
		16:
			return "Determination"
		17:
			return "Mini Game"
		18:
			return "Match Prep"
		19:
			return "Recovery"
		20:
			return "Scrimmage"
		_:
			return "Unknown"


## Get training card type name (Korean)
static func get_training_card_name_kr(card_type: int) -> String:
	match card_type:
		1:
			return "스피드"
		2:
			return "스태미나"
		3:
			return "근력"
		4:
			return "민첩성"
		5:
			return "패스"
		6:
			return "슈팅"
		7:
			return "드리블"
		8:
			return "볼 컨트롤"
		9:
			return "포지셔닝"
		10:
			return "시야"
		11:
			return "팀워크"
		12:
			return "판단력"
		13:
			return "침착함"
		14:
			return "집중력"
		15:
			return "리더십"
		16:
			return "결단력"
		17:
			return "미니 게임"
		18:
			return "경기 준비"
		19:
			return "휴식"
		20:
			return "연습 경기"
		_:
			return "알 수 없음"


# ==================== PARTNER SYNERGY SYSTEM ====================
# Blue Lock PWC 영감
# 훈련 파트너 선택이 결과에 영향
# 특정 Trait 조합 = 시너지 효과

## Synergy level enum
enum SynergyLevel { NONE = 0, BASIC = 1, GOOD = 2, EXCELLENT = 3 }  # 시너지 없음  # 기본 시너지 (+5%)  # 좋은 시너지 (+10%)  # 최고 시너지 (+20%)

## Synergy colors
const SYNERGY_NONE_COLOR = Color("#808080")  # Gray
const SYNERGY_BASIC_COLOR = Color("#87CEEB")  # Sky Blue
const SYNERGY_GOOD_COLOR = Color("#32CD32")  # Lime Green
const SYNERGY_EXCELLENT_COLOR = Color("#FFD700")  # Gold

## Synergy icons
const SYNERGY_ICON_NONE = "➖"
const SYNERGY_ICON_BASIC = "🤝"
const SYNERGY_ICON_GOOD = "💪"
const SYNERGY_ICON_EXCELLENT = "⚡"

## Synergy training bonus multipliers
const SYNERGY_NONE_MULT = 1.00
const SYNERGY_BASIC_MULT = 1.05
const SYNERGY_GOOD_MULT = 1.10
const SYNERGY_EXCELLENT_MULT = 1.20

## Position synergy rules (같은 포지션 라인 = 기본 시너지)
const POSITION_SYNERGY_RULES = {
	"gk_line": ["GK"],
	"def_line": ["CB", "LB", "RB", "LWB", "RWB"],
	"mid_line": ["CDM", "CM", "CAM", "LM", "RM"],
	"att_line": ["LW", "RW", "CF", "ST"]
}

## Trait synergy pairs (특정 트레이트 조합 = 향상된 시너지)
## Format: [trait1, trait2, synergy_level]
const TRAIT_SYNERGY_PAIRS = [
	# Excellent synergies (특별한 조합)
	["captain", "team_player", 3],  # 캡틴 + 팀플레이어
	["playmaker", "clinical", 3],  # 플레이메이커 + 임상적
	["engine", "tireless", 3],  # 엔진 + 끈질긴
	["mentor", "young_talent", 3],  # 멘토 + 젊은 재능
	# Good synergies (좋은 조합)
	["speedster", "agile", 2],  # 빠른 선수 + 민첩
	["aerial_threat", "target_man", 2],  # 공중전 위협 + 타겟맨
	["ball_winner", "defensive_wall", 2],  # 볼 탈취 + 수비 장벽
	["creative", "vision", 2],  # 창의적 + 시야
	["composed", "clutch", 2],  # 침착한 + 클러치
	# Basic synergies (기본 조합) - 같은 카테고리
	["technical_genius", "ball_control_master", 1],
	["physical_beast", "strength", 1],
	["tactical_mind", "intelligent", 1]
]


## Get synergy level color
static func get_synergy_color(level: int) -> Color:
	match level:
		0:
			return SYNERGY_NONE_COLOR
		1:
			return SYNERGY_BASIC_COLOR
		2:
			return SYNERGY_GOOD_COLOR
		3:
			return SYNERGY_EXCELLENT_COLOR
		_:
			return SYNERGY_NONE_COLOR


## Get synergy icon
static func get_synergy_icon(level: int) -> String:
	match level:
		0:
			return SYNERGY_ICON_NONE
		1:
			return SYNERGY_ICON_BASIC
		2:
			return SYNERGY_ICON_GOOD
		3:
			return SYNERGY_ICON_EXCELLENT
		_:
			return SYNERGY_ICON_NONE


## Get synergy level label
static func get_synergy_label(level: int) -> String:
	match level:
		0:
			return "No Synergy"
		1:
			return "Basic Synergy"
		2:
			return "Good Synergy"
		3:
			return "Excellent Synergy"
		_:
			return "Unknown"


## Get synergy level label (Korean)
static func get_synergy_label_kr(level: int) -> String:
	match level:
		0:
			return "시너지 없음"
		1:
			return "기본 시너지"
		2:
			return "좋은 시너지"
		3:
			return "최고 시너지"
		_:
			return "알 수 없음"


## Get synergy training multiplier
static func get_synergy_mult(level: int) -> float:
	match level:
		0:
			return SYNERGY_NONE_MULT
		1:
			return SYNERGY_BASIC_MULT
		2:
			return SYNERGY_GOOD_MULT
		3:
			return SYNERGY_EXCELLENT_MULT
		_:
			return SYNERGY_NONE_MULT


## Check if two positions are in same line
static func _are_positions_same_line(pos1: String, pos2: String) -> bool:
	for line in POSITION_SYNERGY_RULES.values():
		if pos1 in line and pos2 in line:
			return true
	return false


## Calculate synergy between two players
## player1, player2: Dictionary with "position" and "traits" keys
static func calculate_synergy(player1: Dictionary, player2: Dictionary) -> int:
	var synergy_level = 0

	# Check position synergy (same line = basic synergy)
	var pos1 = player1.get("position", "")
	var pos2 = player2.get("position", "")
	if _are_positions_same_line(pos1, pos2):
		synergy_level = max(synergy_level, 1)

	# Check trait synergies
	var traits1 = player1.get("traits", [])
	var traits2 = player2.get("traits", [])

	for pair in TRAIT_SYNERGY_PAIRS:
		var t1 = pair[0]
		var t2 = pair[1]
		var level = pair[2]

		# Check if traits match (either direction)
		if (t1 in traits1 and t2 in traits2) or (t2 in traits1 and t1 in traits2):
			synergy_level = max(synergy_level, level)

	return synergy_level


## Get synergy display for UI
static func get_synergy_display(player1: Dictionary, player2: Dictionary) -> Dictionary:
	var level = calculate_synergy(player1, player2)

	return {
		"level": level,
		"color": get_synergy_color(level),
		"icon": get_synergy_icon(level),
		"label": get_synergy_label(level),
		"label_kr": get_synergy_label_kr(level),
		"multiplier": get_synergy_mult(level),
		"bonus_percent": int((get_synergy_mult(level) - 1.0) * 100)
	}


## Get synergy BBCode for display
static func get_synergy_bbcode(player1: Dictionary, player2: Dictionary) -> String:
	var display = get_synergy_display(player1, player2)

	if display["level"] == 0:
		return "[color=#808080]%s %s[/color]" % [display["icon"], display["label_kr"]]

	return (
		"[color=#%s]%s %s (+%d%%)[/color]"
		% [display["color"].to_html(false), display["icon"], display["label_kr"], display["bonus_percent"]]
	)


## Get best partner recommendation from a list of players
## Returns array of {player, synergy_level} sorted by synergy (best first)
static func get_best_partners(target_player: Dictionary, available_players: Array) -> Array:
	var results = []

	for player in available_players:
		var level = calculate_synergy(target_player, player)
		results.append({"player": player, "synergy_level": level, "multiplier": get_synergy_mult(level)})

	# Sort by synergy level (descending)
	results.sort_custom(func(a, b): return a["synergy_level"] > b["synergy_level"])

	return results


## Calculate combined training efficiency
## Combines: Base efficiency × Mood × StatusEffect × Synergy × IntensiveWeek
static func get_combined_training_efficiency(
	mood: int, status_effects: Array, synergy_level: int, intensive_week_active: bool
) -> Dictionary:
	var mood_mult = get_mood_multiplier(mood)
	var status_mult = get_combined_training_mult(mood, status_effects)
	var synergy_mult = get_synergy_mult(synergy_level)
	var intensive_mult = INTENSIVE_WEEK_TRAINING_MULT if intensive_week_active else 1.0

	var total = mood_mult * status_mult * synergy_mult * intensive_mult

	return {
		"total": total,
		"mood_mult": mood_mult,
		"status_mult": status_mult,
		"synergy_mult": synergy_mult,
		"intensive_mult": intensive_mult,
		"percent_bonus": int((total - 1.0) * 100)
	}


# ==================== FACILITY UPGRADE SYSTEM ====================
# Kairosoft 영감
# 아카데미 시설 레벨 → 훈련 효율 보너스
# 시각적으로 아카데미 외관 변화 표현

## Facility types (시설 종류)
enum FacilityType {
	TRAINING_GROUND = 0,  # 훈련장 - 기본 훈련 효율
	GYM = 1,  # 체육관 - Physical 훈련
	TECH_CENTER = 2,  # 기술 센터 - Technical 훈련
	TACTICS_ROOM = 3,  # 전술실 - Tactical 훈련
	SPORTS_SCIENCE = 4,  # 스포츠 과학 - 회복 속도
	MEDICAL_CENTER = 5,  # 의료 센터 - 부상 예방/회복
	YOUTH_DORM = 6,  # 유소년 기숙사 - 선수 수용 인원
	SCOUT_NETWORK = 7  # 스카우트 네트워크 - 더 좋은 신입생
}

## Maximum facility level
const FACILITY_MAX_LEVEL = 5

## Facility upgrade costs (레벨별 업그레이드 비용)
const FACILITY_UPGRADE_COSTS = {1: 10000, 2: 25000, 3: 50000, 4: 100000, 5: 200000}  # Level 0 → 1  # Level 1 → 2  # Level 2 → 3  # Level 3 → 4  # Level 4 → 5 (Max)

## Facility colors by level
const FACILITY_LEVEL_COLORS = [
	Color("#808080"), Color("#CD7F32"), Color("#C0C0C0"), Color("#FFD700"), Color("#E5E4E2"), Color("#B9F2FF")  # Level 0 - Gray (기본)  # Level 1 - Bronze  # Level 2 - Silver  # Level 3 - Gold  # Level 4 - Platinum  # Level 5 - Diamond
]

## Facility icons
const FACILITY_ICONS = {0: "⚽", 1: "🏋️", 2: "🎯", 3: "📋", 4: "🔬", 5: "🏥", 6: "🏠", 7: "🔍"}  # TRAINING_GROUND  # GYM  # TECH_CENTER  # TACTICS_ROOM  # SPORTS_SCIENCE  # MEDICAL_CENTER  # YOUTH_DORM  # SCOUT_NETWORK

## Facility training bonuses per level (0-5)
## Format: base_bonus + (level * per_level_bonus)
const FACILITY_BONUSES = {
	0: {"base": 1.00, "per_level": 0.05},  # TRAINING_GROUND: +5% per level
	1: {"base": 1.00, "per_level": 0.08},  # GYM: +8% per level for Physical
	2: {"base": 1.00, "per_level": 0.08},  # TECH_CENTER: +8% per level for Technical
	3: {"base": 1.00, "per_level": 0.08},  # TACTICS_ROOM: +8% per level for Tactical
	4: {"base": 1.00, "per_level": 0.10},  # SPORTS_SCIENCE: +10% recovery per level
	5: {"base": 1.00, "per_level": 0.15},  # MEDICAL_CENTER: -15% injury risk per level
	6: {"base": 10, "per_level": 2},  # YOUTH_DORM: 10 + 2 slots per level
	7: {"base": 1.00, "per_level": 0.10}  # SCOUT_NETWORK: +10% better recruits per level
}


## Get facility name
static func get_facility_name(facility_type: int) -> String:
	match facility_type:
		0:
			return "Training Ground"
		1:
			return "Gym"
		2:
			return "Tech Center"
		3:
			return "Tactics Room"
		4:
			return "Sports Science"
		5:
			return "Medical Center"
		6:
			return "Youth Dorm"
		7:
			return "Scout Network"
		_:
			return "Unknown"


## Get facility name (Korean)
static func get_facility_name_kr(facility_type: int) -> String:
	match facility_type:
		0:
			return "훈련장"
		1:
			return "체육관"
		2:
			return "기술 센터"
		3:
			return "전술실"
		4:
			return "스포츠 과학"
		5:
			return "의료 센터"
		6:
			return "유소년 기숙사"
		7:
			return "스카우트 네트워크"
		_:
			return "알 수 없음"


## Get facility icon
static func get_facility_icon(facility_type: int) -> String:
	return FACILITY_ICONS.get(facility_type, "❓")


## Get facility level color
static func get_facility_level_color(level: int) -> Color:
	level = clamp(level, 0, FACILITY_MAX_LEVEL)
	return FACILITY_LEVEL_COLORS[level]


## Get facility level label
static func get_facility_level_label(level: int) -> String:
	match level:
		0:
			return "Basic"
		1:
			return "Bronze"
		2:
			return "Silver"
		3:
			return "Gold"
		4:
			return "Platinum"
		5:
			return "Diamond"
		_:
			return "Unknown"


## Get facility level label (Korean)
static func get_facility_level_label_kr(level: int) -> String:
	match level:
		0:
			return "기본"
		1:
			return "브론즈"
		2:
			return "실버"
		3:
			return "골드"
		4:
			return "플래티넘"
		5:
			return "다이아몬드"
		_:
			return "알 수 없음"


## Calculate facility bonus multiplier
static func get_facility_bonus(facility_type: int, level: int) -> float:
	level = clamp(level, 0, FACILITY_MAX_LEVEL)
	var bonus_data = FACILITY_BONUSES.get(facility_type, {"base": 1.0, "per_level": 0.0})
	return bonus_data["base"] + (level * bonus_data["per_level"])


## Get upgrade cost for next level
static func get_facility_upgrade_cost(current_level: int) -> int:
	var next_level = current_level + 1
	if next_level > FACILITY_MAX_LEVEL:
		return -1  # Max level reached
	return FACILITY_UPGRADE_COSTS.get(next_level, -1)


## Get facility display info
static func get_facility_display(facility_type: int, level: int) -> Dictionary:
	level = clamp(level, 0, FACILITY_MAX_LEVEL)
	var bonus = get_facility_bonus(facility_type, level)
	var upgrade_cost = get_facility_upgrade_cost(level)

	return {
		"type": facility_type,
		"level": level,
		"name": get_facility_name(facility_type),
		"name_kr": get_facility_name_kr(facility_type),
		"icon": get_facility_icon(facility_type),
		"color": get_facility_level_color(level),
		"level_label": get_facility_level_label(level),
		"level_label_kr": get_facility_level_label_kr(level),
		"bonus": bonus,
		"bonus_percent": int((bonus - 1.0) * 100) if facility_type != 6 else int(bonus),
		"upgrade_cost": upgrade_cost,
		"can_upgrade": level < FACILITY_MAX_LEVEL
	}


## Get facility BBCode display
static func get_facility_bbcode(facility_type: int, level: int) -> String:
	var display = get_facility_display(facility_type, level)
	var color_hex = display["color"].to_html(false)

	if facility_type == 6:  # YOUTH_DORM shows capacity
		return (
			"[color=#%s]%s %s Lv.%d[/color] (%d명)"
			% [color_hex, display["icon"], display["name_kr"], level, int(display["bonus"])]
		)
	else:
		return (
			"[color=#%s]%s %s Lv.%d[/color] (+%d%%)"
			% [color_hex, display["icon"], display["name_kr"], level, display["bonus_percent"]]
		)


## Get all facilities status
static func get_all_facilities_status(facility_levels: Dictionary) -> Array:
	var results = []
	for i in range(8):  # 8 facility types
		var level = facility_levels.get(i, 0)
		results.append(get_facility_display(i, level))
	return results


# ==================== GROWTH RATE (POTENTIAL) SYSTEM ====================
# Uma Musume 영감
# 선수별 성장률이 다름. 높은 성장률 = 훈련 효과 증가

## Potential grade enum (잠재력 등급)
enum PotentialGrade { F = 0, E = 1, D = 2, C = 3, B = 4, A = 5, S = 6 }  # 60-69% 성장률  # 70-79%  # 80-89%  # 90-99%  # 100-109%  # 110-119%  # 120-130%

## Potential colors
const POTENTIAL_COLORS = {
	0: Color("#808080"),  # F - Gray
	1: Color("#964B00"),  # E - Brown
	2: Color("#32CD32"),  # D - Lime
	3: Color("#4A90D9"),  # C - Blue
	4: Color("#9B59B6"),  # B - Purple
	5: Color("#FFD700"),  # A - Gold
	6: Color("#FF6B6B")  # S - Red/Pink
}

## Potential growth multipliers (base range)
const POTENTIAL_GROWTH_RANGE = {
	0: {"min": 0.60, "max": 0.69},  # F
	1: {"min": 0.70, "max": 0.79},  # E
	2: {"min": 0.80, "max": 0.89},  # D
	3: {"min": 0.90, "max": 0.99},  # C
	4: {"min": 1.00, "max": 1.09},  # B
	5: {"min": 1.10, "max": 1.19},  # A
	6: {"min": 1.20, "max": 1.30}  # S
}


## Get potential grade label
static func get_potential_label(grade: int) -> String:
	match grade:
		0:
			return "F"
		1:
			return "E"
		2:
			return "D"
		3:
			return "C"
		4:
			return "B"
		5:
			return "A"
		6:
			return "S"
		_:
			return "?"


## Get potential grade label (Korean)
static func get_potential_label_kr(grade: int) -> String:
	match grade:
		0:
			return "F등급"
		1:
			return "E등급"
		2:
			return "D등급"
		3:
			return "C등급"
		4:
			return "B등급"
		5:
			return "A등급"
		6:
			return "S등급"
		_:
			return "알 수 없음"


## Get potential color
static func get_potential_color(grade: int) -> Color:
	return POTENTIAL_COLORS.get(grade, Color.WHITE)


## Generate random growth rate within potential grade
static func generate_growth_rate(grade: int) -> float:
	var range_data = POTENTIAL_GROWTH_RANGE.get(grade, {"min": 0.9, "max": 1.0})
	return randf_range(range_data["min"], range_data["max"])


## Get potential display
static func get_potential_display(grade: int, growth_rate: float = -1.0) -> Dictionary:
	if growth_rate < 0:
		growth_rate = generate_growth_rate(grade)

	var range_data = POTENTIAL_GROWTH_RANGE.get(grade, {"min": 0.9, "max": 1.0})

	return {
		"grade": grade,
		"label": get_potential_label(grade),
		"label_kr": get_potential_label_kr(grade),
		"color": get_potential_color(grade),
		"growth_rate": growth_rate,
		"growth_percent": int(growth_rate * 100),
		"min_rate": range_data["min"],
		"max_rate": range_data["max"]
	}


## Get potential BBCode
static func get_potential_bbcode(grade: int, growth_rate: float = -1.0) -> String:
	var display = get_potential_display(grade, growth_rate)
	return (
		"[color=#%s][b]%s[/b][/color] (%d%%)"
		% [display["color"].to_html(false), display["label"], display["growth_percent"]]
	)


## Calculate stat-specific potential (개별 스탯별 잠재력)
## Some players might be good at Speed but bad at Passing
static func get_stat_potential_display(stat_potentials: Dictionary) -> Dictionary:
	var result = {}
	for stat in stat_potentials:
		var grade = stat_potentials[stat]
		result[stat] = get_potential_display(grade)
	return result


# ==================== MENTORING SYSTEM ====================
# Uma Musume 인자 계승 영감
# 졸업한 선배(Alumni)가 아카데미에 '멘토'로 방문
# 멘토 지정 시 후배에게 특정 Trait 힌트 또는 스탯 보너스 제공

## Mentor tier enum (멘토 등급 - 졸업생 활약도 기반)
enum MentorTier { AMATEUR = 0, SEMI_PRO = 1, PROFESSIONAL = 2, STAR = 3, LEGEND = 4 }  # 아마추어 리그 활동  # 세미프로 활동  # 프로 리그 활동  # 스타 선수  # 레전드

## Mentor tier colors
const MENTOR_TIER_COLORS = {
	0: Color("#808080"), 1: Color("#32CD32"), 2: Color("#4A90D9"), 3: Color("#FFD700"), 4: Color("#FF6B6B")  # AMATEUR - Gray  # SEMI_PRO - Green  # PROFESSIONAL - Blue  # STAR - Gold  # LEGEND - Red/Pink
}

## Mentor tier icons
const MENTOR_TIER_ICONS = {0: "🌱", 1: "⭐", 2: "🌟", 3: "💫", 4: "👑"}  # AMATEUR  # SEMI_PRO  # PROFESSIONAL  # STAR  # LEGEND

## Mentor training bonus by tier
const MENTOR_TRAINING_BONUS = {0: 0.05, 1: 0.10, 2: 0.15, 3: 0.20, 4: 0.30}  # AMATEUR: +5%  # SEMI_PRO: +10%  # PROFESSIONAL: +15%  # STAR: +20%  # LEGEND: +30%

## Mentor trait hint chance by tier (특정 Trait 힌트 제공 확률)
const MENTOR_TRAIT_HINT_CHANCE = {0: 0.05, 1: 0.10, 2: 0.20, 3: 0.35, 4: 0.50}  # AMATEUR: 5%  # SEMI_PRO: 10%  # PROFESSIONAL: 20%  # STAR: 35%  # LEGEND: 50%


## Get mentor tier label
static func get_mentor_tier_label(tier: int) -> String:
	match tier:
		0:
			return "Amateur"
		1:
			return "Semi-Pro"
		2:
			return "Professional"
		3:
			return "Star"
		4:
			return "Legend"
		_:
			return "Unknown"


## Get mentor tier label (Korean)
static func get_mentor_tier_label_kr(tier: int) -> String:
	match tier:
		0:
			return "아마추어"
		1:
			return "세미프로"
		2:
			return "프로"
		3:
			return "스타"
		4:
			return "레전드"
		_:
			return "알 수 없음"


## Get mentor tier color
static func get_mentor_tier_color(tier: int) -> Color:
	return MENTOR_TIER_COLORS.get(tier, Color.WHITE)


## Get mentor tier icon
static func get_mentor_tier_icon(tier: int) -> String:
	return MENTOR_TIER_ICONS.get(tier, "❓")


## Get mentor training bonus
static func get_mentor_training_bonus(tier: int) -> float:
	return MENTOR_TRAINING_BONUS.get(tier, 0.0)


## Get mentor trait hint chance
static func get_mentor_trait_hint_chance(tier: int) -> float:
	return MENTOR_TRAIT_HINT_CHANCE.get(tier, 0.0)


## Get mentor display
static func get_mentor_display(tier: int, specialty_stat: String = "", specialty_trait: String = "") -> Dictionary:
	return {
		"tier": tier,
		"label": get_mentor_tier_label(tier),
		"label_kr": get_mentor_tier_label_kr(tier),
		"color": get_mentor_tier_color(tier),
		"icon": get_mentor_tier_icon(tier),
		"training_bonus": get_mentor_training_bonus(tier),
		"training_bonus_percent": int(get_mentor_training_bonus(tier) * 100),
		"trait_hint_chance": get_mentor_trait_hint_chance(tier),
		"trait_hint_percent": int(get_mentor_trait_hint_chance(tier) * 100),
		"specialty_stat": specialty_stat,
		"specialty_trait": specialty_trait
	}


## Get mentor BBCode
static func get_mentor_bbcode(mentor_name: String, tier: int, specialty_stat: String = "") -> String:
	var display = get_mentor_display(tier, specialty_stat)
	var specialty_text = ""
	if specialty_stat != "":
		specialty_text = " [%s +%d%%]" % [specialty_stat, display["training_bonus_percent"] + 10]

	return (
		"[color=#%s]%s %s (%s)[/color]%s"
		% [display["color"].to_html(false), display["icon"], mentor_name, display["label_kr"], specialty_text]
	)


## Calculate mentoring effect
## mentor_data: {tier, specialty_stat, specialty_trait}
## training_stat: the stat being trained
static func calculate_mentoring_effect(mentor_data: Dictionary, training_stat: String) -> Dictionary:
	var tier = mentor_data.get("tier", 0)
	var specialty_stat = mentor_data.get("specialty_stat", "")
	var specialty_trait = mentor_data.get("specialty_trait", "")

	var base_bonus = get_mentor_training_bonus(tier)
	var specialty_bonus = 0.10 if training_stat == specialty_stat else 0.0
	var total_bonus = base_bonus + specialty_bonus

	var trait_hint_chance = get_mentor_trait_hint_chance(tier)
	var will_give_hint = randf() < trait_hint_chance

	return {
		"base_bonus": base_bonus,
		"specialty_bonus": specialty_bonus,
		"total_bonus": total_bonus,
		"total_mult": 1.0 + total_bonus,
		"total_percent": int(total_bonus * 100),
		"trait_hint_given": will_give_hint,
		"hinted_trait": specialty_trait if will_give_hint else ""
	}


## Get mentor visit schedule display
## Mentors visit periodically based on tier
static func get_mentor_visit_info(tier: int) -> Dictionary:
	# Higher tier = more frequent visits
	var visit_frequency = {0: 30, 1: 21, 2: 14, 3: 10, 4: 7}  # AMATEUR: every 30 days  # SEMI_PRO: every 21 days  # PROFESSIONAL: every 14 days  # STAR: every 10 days  # LEGEND: every 7 days

	var duration = {0: 3, 1: 4, 2: 5, 3: 6, 4: 7}  # AMATEUR: stays 3 days  # SEMI_PRO: stays 4 days  # PROFESSIONAL: stays 5 days  # STAR: stays 6 days  # LEGEND: stays 7 days

	return {"visit_every_days": visit_frequency.get(tier, 30), "stay_duration_days": duration.get(tier, 3)}


# ==================== LOCKER ROOM ATMOSPHERE SYSTEM ====================
# FM Mobile Dynamics 영감
# 주장(Captain)의 성격이 팀 전체 훈련 효율에 영향

## Atmosphere level enum
enum AtmosphereLevel { TOXIC = 0, POOR = 1, NEUTRAL = 2, GOOD = 3, EXCELLENT = 4 }  # 독성 분위기 (-15%)  # 나쁜 분위기 (-5%)  # 중립 (0%)  # 좋은 분위기 (+5%)  # 최고 분위기 (+15%)

## Atmosphere colors
const ATMOSPHERE_COLORS = {
	0: Color("#8B0000"), 1: Color("#FF8C00"), 2: Color("#808080"), 3: Color("#32CD32"), 4: Color("#FFD700")  # TOXIC - Dark Red  # POOR - Orange  # NEUTRAL - Gray  # GOOD - Green  # EXCELLENT - Gold
}

## Atmosphere icons
const ATMOSPHERE_ICONS = {0: "💀", 1: "😟", 2: "😐", 3: "😊", 4: "🔥"}  # TOXIC  # POOR  # NEUTRAL  # GOOD  # EXCELLENT

## Atmosphere training multipliers
const ATMOSPHERE_TRAINING_MULT = {0: 0.85, 1: 0.95, 2: 1.00, 3: 1.05, 4: 1.15}  # TOXIC: -15%  # POOR: -5%  # NEUTRAL: 0%  # GOOD: +5%  # EXCELLENT: +15%

## Captain personality effects on atmosphere
## Format: personality_trait -> atmosphere_modifier
const CAPTAIN_PERSONALITY_EFFECTS = {
	"leader": 1,  # +1 atmosphere level
	"determined": 1,  # +1 atmosphere level
	"team_player": 1,  # +1 atmosphere level
	"charismatic": 2,  # +2 atmosphere levels (special)
	"lazy": -1,  # -1 atmosphere level
	"selfish": -1,  # -1 atmosphere level
	"temperamental": -1,  # -1 atmosphere level
	"toxic": -2  # -2 atmosphere levels (special bad)
}


## Get atmosphere label
static func get_atmosphere_label(level: int) -> String:
	match level:
		0:
			return "Toxic"
		1:
			return "Poor"
		2:
			return "Neutral"
		3:
			return "Good"
		4:
			return "Excellent"
		_:
			return "Unknown"


## Get atmosphere label (Korean)
static func get_atmosphere_label_kr(level: int) -> String:
	match level:
		0:
			return "독성"
		1:
			return "나쁨"
		2:
			return "중립"
		3:
			return "좋음"
		4:
			return "최고"
		_:
			return "알 수 없음"


## Get atmosphere color
static func get_atmosphere_color(level: int) -> Color:
	return ATMOSPHERE_COLORS.get(level, Color.GRAY)


## Get atmosphere icon
static func get_atmosphere_icon(level: int) -> String:
	return ATMOSPHERE_ICONS.get(level, "❓")


## Get atmosphere training multiplier
static func get_atmosphere_training_mult(level: int) -> float:
	return ATMOSPHERE_TRAINING_MULT.get(level, 1.0)


## Calculate atmosphere from captain traits
static func calculate_atmosphere_from_captain(captain_traits: Array, base_level: int = 2) -> int:
	var modifier = 0
	for t in captain_traits:
		modifier += CAPTAIN_PERSONALITY_EFFECTS.get(t, 0)

	var final_level = clamp(base_level + modifier, 0, 4)
	return final_level


## Get atmosphere display
static func get_atmosphere_display(level: int) -> Dictionary:
	var mult = get_atmosphere_training_mult(level)
	return {
		"level": level,
		"label": get_atmosphere_label(level),
		"label_kr": get_atmosphere_label_kr(level),
		"color": get_atmosphere_color(level),
		"icon": get_atmosphere_icon(level),
		"training_mult": mult,
		"training_bonus_percent": int((mult - 1.0) * 100)
	}


## Get atmosphere BBCode
static func get_atmosphere_bbcode(level: int) -> String:
	var display = get_atmosphere_display(level)
	var bonus_text = ""
	if display["training_bonus_percent"] > 0:
		bonus_text = " (+%d%%)" % display["training_bonus_percent"]
	elif display["training_bonus_percent"] < 0:
		bonus_text = " (%d%%)" % display["training_bonus_percent"]

	return (
		"[color=#%s]%s %s%s[/color]"
		% [display["color"].to_html(false), display["icon"], display["label_kr"], bonus_text]
	)


## Get full academy training efficiency
## Combines all systems: Facility × Mood × StatusEffect × Synergy × IntensiveWeek × Potential × Mentor × Atmosphere
static func get_full_training_efficiency(
	facility_level: int,
	facility_type: int,
	mood: int,
	status_effects: Array,
	synergy_level: int,
	intensive_week_active: bool,
	growth_rate: float,
	mentor_tier: int,
	mentor_specialty_stat: String,
	training_stat: String,
	atmosphere_level: int
) -> Dictionary:
	var facility_mult = get_facility_bonus(facility_type, facility_level)
	var mood_mult = get_mood_multiplier(mood)
	var status_mult = get_combined_training_mult(mood, status_effects)
	var synergy_mult = get_synergy_mult(synergy_level)
	var intensive_mult = INTENSIVE_WEEK_TRAINING_MULT if intensive_week_active else 1.0
	var potential_mult = growth_rate
	var mentor_effect = calculate_mentoring_effect(
		{"tier": mentor_tier, "specialty_stat": mentor_specialty_stat}, training_stat
	)
	var mentor_mult = mentor_effect["total_mult"]
	var atmosphere_mult = get_atmosphere_training_mult(atmosphere_level)

	var total = (
		facility_mult
		* mood_mult
		* status_mult
		* synergy_mult
		* intensive_mult
		* potential_mult
		* mentor_mult
		* atmosphere_mult
	)

	return {
		"total": total,
		"facility_mult": facility_mult,
		"mood_mult": mood_mult,
		"status_mult": status_mult,
		"synergy_mult": synergy_mult,
		"intensive_mult": intensive_mult,
		"potential_mult": potential_mult,
		"mentor_mult": mentor_mult,
		"atmosphere_mult": atmosphere_mult,
		"percent_bonus": int((total - 1.0) * 100),
		"mentor_trait_hint": mentor_effect["hinted_trait"]
	}


# ==================== WEEKLY ACTION POINTS (AP) SYSTEM ====================
# Blue Lock PWC 영감
# 한 주에 행동력 제한 → 훈련/휴식/특별활동 선택의 의미
# 스태미나/컨디션이 선택을 제약

## Weekly AP constants
const WEEKLY_AP_BASE = 4  # 기본 주간 행동력
const WEEKLY_AP_MAX = 6  # 최대 주간 행동력 (시설 보너스 포함)

## Activity AP costs
const AP_COST_TRAINING = 1  # 훈련 1회
const AP_COST_MATCH = 2  # 경기 참가
const AP_COST_REST = 1  # 휴식
const AP_COST_SPECIAL_EVENT = 1  # 특별 이벤트
const AP_COST_MENTOR_SESSION = 1  # 멘토 세션

## AP bonus from facilities
const AP_BONUS_PER_TRAINING_LEVEL = 0.2  # 훈련장 레벨당 +0.2 AP (반올림)

## AP colors based on remaining
const AP_FULL_COLOR = Color("#32CD32")  # 초록 (충분함)
const AP_MEDIUM_COLOR = Color("#FFD700")  # 금색 (보통)
const AP_LOW_COLOR = Color("#FF8C00")  # 주황 (적음)
const AP_EMPTY_COLOR = Color("#DC143C")  # 빨강 (없음)


## Get weekly AP display
static func get_weekly_ap_display(current_ap: int, max_ap: int) -> Dictionary:
	var ratio = float(current_ap) / float(max_ap) if max_ap > 0 else 0.0
	var color: Color
	var status: String
	var status_kr: String

	if ratio >= 0.75:
		color = AP_FULL_COLOR
		status = "Full"
		status_kr = "충분"
	elif ratio >= 0.5:
		color = AP_MEDIUM_COLOR
		status = "Medium"
		status_kr = "보통"
	elif ratio > 0:
		color = AP_LOW_COLOR
		status = "Low"
		status_kr = "적음"
	else:
		color = AP_EMPTY_COLOR
		status = "Empty"
		status_kr = "없음"

	return {
		"current": current_ap,
		"max": max_ap,
		"ratio": ratio,
		"color": color,
		"status": status,
		"status_kr": status_kr,
		"icon": "⚡"
	}


## Get AP BBCode bar
static func get_ap_bar_bbcode(current_ap: int, max_ap: int) -> String:
	var display = get_weekly_ap_display(current_ap, max_ap)
	var filled = "●".repeat(current_ap)
	var empty = "○".repeat(max_ap - current_ap)

	return (
		"[color=#%s]%s %s[/color][color=#808080]%s[/color] (%d/%d)"
		% [display["color"].to_html(false), display["icon"], filled, empty, current_ap, max_ap]
	)


## Calculate max AP with facility bonus
static func calculate_max_ap(training_ground_level: int) -> int:
	var bonus = int(training_ground_level * AP_BONUS_PER_TRAINING_LEVEL)
	return mini(WEEKLY_AP_BASE + bonus, WEEKLY_AP_MAX)


## Check if activity can be performed
static func can_perform_activity(current_ap: int, activity_type: String) -> Dictionary:
	var cost = 0
	match activity_type:
		"training":
			cost = AP_COST_TRAINING
		"match":
			cost = AP_COST_MATCH
		"rest":
			cost = AP_COST_REST
		"special_event":
			cost = AP_COST_SPECIAL_EVENT
		"mentor_session":
			cost = AP_COST_MENTOR_SESSION
		_:
			cost = 1

	var can_afford = current_ap >= cost

	return {
		"can_afford": can_afford,
		"cost": cost,
		"remaining_after": current_ap - cost if can_afford else current_ap,
		"activity": activity_type
	}


# ==================== PLAYER ACHIEVEMENT/MILESTONE SYSTEM ====================
# 선수 개인 업적 및 마일스톤 추적
# 특정 업적 달성 시 보상/Trait 해금

## Achievement categories
enum AchievementCategory { TRAINING = 0, MATCH = 1, GROWTH = 2, SOCIAL = 3, SPECIAL = 4 }  # 훈련 관련  # 경기 관련  # 성장 관련  # 팀/멘토 관련  # 특별 업적

## Achievement tier
enum AchievementTier { BRONZE = 0, SILVER = 1, GOLD = 2, PLATINUM = 3 }  # 브론즈  # 실버  # 골드  # 플래티넘

## Achievement tier colors
const ACHIEVEMENT_TIER_COLORS = {0: Color("#CD7F32"), 1: Color("#C0C0C0"), 2: Color("#FFD700"), 3: Color("#E5E4E2")}  # BRONZE  # SILVER  # GOLD  # PLATINUM

## Achievement tier icons
const ACHIEVEMENT_TIER_ICONS = {0: "🥉", 1: "🥈", 2: "🥇", 3: "🏆"}  # BRONZE  # SILVER  # GOLD  # PLATINUM

## Pre-defined achievements
const ACHIEVEMENTS = [
	# Training achievements
	{
		"id": "first_training",
		"name": "First Steps",
		"name_kr": "첫 걸음",
		"category": 0,
		"tier": 0,
		"condition": "Complete first training",
		"reward_type": "stat_bonus",
		"reward_value": {"any": 1}
	},
	{
		"id": "training_10",
		"name": "Dedicated Trainee",
		"name_kr": "열정적인 훈련생",
		"category": 0,
		"tier": 0,
		"condition": "Complete 10 training sessions",
		"reward_type": "stat_bonus",
		"reward_value": {"stamina": 2}
	},
	{
		"id": "training_50",
		"name": "Training Addict",
		"name_kr": "훈련 중독",
		"category": 0,
		"tier": 1,
		"condition": "Complete 50 training sessions",
		"reward_type": "potential_boost",
		"reward_value": 0.05
	},
	{
		"id": "great_success_5",
		"name": "Lucky Star",
		"name_kr": "행운의 별",
		"category": 0,
		"tier": 1,
		"condition": "Achieve 5 great successes",
		"reward_type": "trait_hint",
		"reward_value": "lucky"
	},
	{
		"id": "combo_discover_3",
		"name": "Combo Master",
		"name_kr": "콤보 마스터",
		"category": 0,
		"tier": 2,
		"condition": "Discover 3 training combos",
		"reward_type": "trait_unlock",
		"reward_value": "versatile"
	},
	# Match achievements
	{
		"id": "first_goal",
		"name": "First Blood",
		"name_kr": "첫 골",
		"category": 1,
		"tier": 0,
		"condition": "Score first goal",
		"reward_type": "stat_bonus",
		"reward_value": {"shooting": 1}
	},
	{
		"id": "goals_10",
		"name": "Rising Striker",
		"name_kr": "떠오르는 스트라이커",
		"category": 1,
		"tier": 1,
		"condition": "Score 10 goals",
		"reward_type": "stat_bonus",
		"reward_value": {"shooting": 3, "composure": 2}
	},
	{
		"id": "goals_50",
		"name": "Golden Boot",
		"name_kr": "골든 부트",
		"category": 1,
		"tier": 2,
		"condition": "Score 50 goals",
		"reward_type": "trait_unlock",
		"reward_value": "clinical"
	},
	{
		"id": "assists_10",
		"name": "Playmaker",
		"name_kr": "플레이메이커",
		"category": 1,
		"tier": 1,
		"condition": "Provide 10 assists",
		"reward_type": "stat_bonus",
		"reward_value": {"passing": 3, "vision": 2}
	},
	{
		"id": "clean_sheet_5",
		"name": "Brick Wall",
		"name_kr": "철벽 수비",
		"category": 1,
		"tier": 1,
		"condition": "Keep 5 clean sheets",
		"reward_type": "stat_bonus",
		"reward_value": {"positioning": 3, "composure": 2}
	},
	{
		"id": "motm_3",
		"name": "MVP Material",
		"name_kr": "MVP 자질",
		"category": 1,
		"tier": 2,
		"condition": "Win Man of the Match 3 times",
		"reward_type": "trait_unlock",
		"reward_value": "big_game_player"
	},
	# Growth achievements
	{
		"id": "stat_70",
		"name": "Well Rounded",
		"name_kr": "올라운더",
		"category": 2,
		"tier": 1,
		"condition": "Reach 70 in any stat",
		"reward_type": "stat_bonus",
		"reward_value": {"any": 2}
	},
	{
		"id": "stat_80",
		"name": "Specialist",
		"name_kr": "스페셜리스트",
		"category": 2,
		"tier": 2,
		"condition": "Reach 80 in any stat",
		"reward_type": "potential_boost",
		"reward_value": 0.05
	},
	{
		"id": "stat_90",
		"name": "World Class",
		"name_kr": "월드클래스",
		"category": 2,
		"tier": 3,
		"condition": "Reach 90 in any stat",
		"reward_type": "trait_unlock",
		"reward_value": "world_class"
	},
	{
		"id": "trait_3",
		"name": "Multi-Talented",
		"name_kr": "다재다능",
		"category": 2,
		"tier": 1,
		"condition": "Unlock 3 traits",
		"reward_type": "stat_bonus",
		"reward_value": {"determination": 3}
	},
	# Social achievements
	{
		"id": "mentor_session_5",
		"name": "Good Student",
		"name_kr": "모범생",
		"category": 3,
		"tier": 1,
		"condition": "Complete 5 mentor sessions",
		"reward_type": "stat_bonus",
		"reward_value": {"teamwork": 2, "leadership": 1}
	},
	{
		"id": "synergy_excellent",
		"name": "Perfect Partner",
		"name_kr": "최고의 파트너",
		"category": 3,
		"tier": 2,
		"condition": "Achieve Excellent synergy with a teammate",
		"reward_type": "trait_unlock",
		"reward_value": "team_player"
	},
	{
		"id": "captain_appointed",
		"name": "Born Leader",
		"name_kr": "타고난 리더",
		"category": 3,
		"tier": 2,
		"condition": "Be appointed as captain",
		"reward_type": "trait_unlock",
		"reward_value": "captain"
	},
	# Special achievements
	{
		"id": "intensive_week_3",
		"name": "On Fire",
		"name_kr": "불타오르다",
		"category": 4,
		"tier": 2,
		"condition": "Complete 3 Intensive Training Weeks",
		"reward_type": "permanent_bonus",
		"reward_value": {"training_mult": 0.05}
	},
	{
		"id": "perfect_mood_week",
		"name": "Positive Vibes",
		"name_kr": "긍정 에너지",
		"category": 4,
		"tier": 1,
		"condition": "Maintain Great mood for entire week",
		"reward_type": "mood_boost",
		"reward_value": 1
	},
	{
		"id": "graduation",
		"name": "Graduated",
		"name_kr": "졸업",
		"category": 4,
		"tier": 3,
		"condition": "Graduate from academy",
		"reward_type": "legacy_unlock",
		"reward_value": "mentor_available"
	}
]


## Get achievement category label
static func get_achievement_category_label(category: int) -> String:
	match category:
		0:
			return "Training"
		1:
			return "Match"
		2:
			return "Growth"
		3:
			return "Social"
		4:
			return "Special"
		_:
			return "Unknown"


## Get achievement category label (Korean)
static func get_achievement_category_label_kr(category: int) -> String:
	match category:
		0:
			return "훈련"
		1:
			return "경기"
		2:
			return "성장"
		3:
			return "관계"
		4:
			return "특별"
		_:
			return "알 수 없음"


## Get achievement tier color
static func get_achievement_tier_color(tier: int) -> Color:
	return ACHIEVEMENT_TIER_COLORS.get(tier, Color.WHITE)


## Get achievement tier icon
static func get_achievement_tier_icon(tier: int) -> String:
	return ACHIEVEMENT_TIER_ICONS.get(tier, "🎖️")


## Get achievement display
static func get_achievement_display(achievement_id: String, is_unlocked: bool = false) -> Dictionary:
	for achievement in ACHIEVEMENTS:
		if achievement["id"] == achievement_id:
			return {
				"id": achievement["id"],
				"name": achievement["name"],
				"name_kr": achievement["name_kr"],
				"category": achievement["category"],
				"category_label": get_achievement_category_label(achievement["category"]),
				"category_label_kr": get_achievement_category_label_kr(achievement["category"]),
				"tier": achievement["tier"],
				"tier_icon": get_achievement_tier_icon(achievement["tier"]),
				"tier_color": get_achievement_tier_color(achievement["tier"]),
				"condition": achievement["condition"],
				"reward_type": achievement["reward_type"],
				"reward_value": achievement["reward_value"],
				"is_unlocked": is_unlocked
			}
	return {}


## Get achievement BBCode
static func get_achievement_bbcode(achievement_id: String, is_unlocked: bool = false) -> String:
	var display = get_achievement_display(achievement_id, is_unlocked)
	if display.is_empty():
		return ""

	var lock_icon = "🔓" if is_unlocked else "🔒"
	var opacity = "" if is_unlocked else "[color=#808080]"
	var end_opacity = "" if is_unlocked else "[/color]"

	return (
		"%s%s [color=#%s]%s[/color] %s%s"
		% [
			opacity,
			display["tier_icon"],
			display["tier_color"].to_html(false),
			display["name_kr"],
			lock_icon,
			end_opacity
		]
	)


## Get all achievements by category
static func get_achievements_by_category(category: int) -> Array:
	var result = []
	for achievement in ACHIEVEMENTS:
		if achievement["category"] == category:
			result.append(achievement)
	return result


## Get achievement progress display
static func get_achievement_progress_display(unlocked_ids: Array) -> Dictionary:
	var total = ACHIEVEMENTS.size()
	var unlocked = unlocked_ids.size()
	var by_tier = {0: 0, 1: 0, 2: 0, 3: 0}
	var by_category = {0: 0, 1: 0, 2: 0, 3: 0, 4: 0}

	for achievement in ACHIEVEMENTS:
		if achievement["id"] in unlocked_ids:
			by_tier[achievement["tier"]] += 1
			by_category[achievement["category"]] += 1

	return {
		"total": total,
		"unlocked": unlocked,
		"percent": int(float(unlocked) / float(total) * 100) if total > 0 else 0,
		"by_tier": by_tier,
		"by_category": by_category
	}


# ==================== MATCH REWARD SYSTEM ====================
# 경기 결과에 따른 보상 계산

## Match result enum
enum MatchResult { LOSS = 0, DRAW = 1, WIN = 2 }

## Base rewards by result
const MATCH_BASE_REWARDS = {
	0: {"money": 1000, "xp": 10, "focus": 0},  # LOSS
	1: {"money": 2500, "xp": 25, "focus": 10},  # DRAW
	2: {"money": 5000, "xp": 50, "focus": 20}  # WIN
}

## Bonus multipliers for goals
const GOAL_BONUS_MONEY = 500
const GOAL_BONUS_XP = 10
const GOAL_BONUS_FOCUS = 5

## Bonus for clean sheet
const CLEAN_SHEET_BONUS = {"money": 1000, "xp": 15, "focus": 5}

## Man of the Match bonus
const MOTM_BONUS = {"money": 2000, "xp": 30, "focus": 10}


## Calculate match rewards
static func calculate_match_rewards(
	result: int, goals_scored: int, goals_conceded: int, player_goals: int, player_assists: int, is_motm: bool
) -> Dictionary:
	# Base reward
	var base = MATCH_BASE_REWARDS.get(result, MATCH_BASE_REWARDS[0])
	var money = base["money"]
	var xp = base["xp"]
	var focus = base["focus"]

	# Goal bonus for team
	money += goals_scored * GOAL_BONUS_MONEY
	xp += goals_scored * GOAL_BONUS_XP

	# Player contribution bonus
	money += player_goals * GOAL_BONUS_MONEY * 2
	xp += player_goals * GOAL_BONUS_XP * 2
	focus += player_goals * GOAL_BONUS_FOCUS

	money += player_assists * int(GOAL_BONUS_MONEY * 0.5)
	xp += player_assists * int(GOAL_BONUS_XP * 0.5)

	# Clean sheet bonus
	if goals_conceded == 0:
		money += CLEAN_SHEET_BONUS["money"]
		xp += CLEAN_SHEET_BONUS["xp"]
		focus += CLEAN_SHEET_BONUS["focus"]

	# MOTM bonus
	if is_motm:
		money += MOTM_BONUS["money"]
		xp += MOTM_BONUS["xp"]
		focus += MOTM_BONUS["focus"]

	return {
		"money": money,
		"xp": xp,
		"focus_gain": focus,
		"result": result,
		"goals_scored": goals_scored,
		"goals_conceded": goals_conceded,
		"player_goals": player_goals,
		"player_assists": player_assists,
		"is_motm": is_motm,
		"clean_sheet": goals_conceded == 0
	}


## Get match reward display
static func get_match_reward_display(rewards: Dictionary) -> Dictionary:
	var result_text = ""
	var result_color: Color

	match rewards["result"]:
		0:
			result_text = "패배"
			result_color = Color("#DC143C")
		1:
			result_text = "무승부"
			result_color = Color("#FFD700")
		2:
			result_text = "승리"
			result_color = Color("#32CD32")

	return {
		"result_text": result_text,
		"result_color": result_color,
		"money_text": "%d 골드" % rewards["money"],
		"xp_text": "+%d XP" % rewards["xp"],
		"focus_text": "+%d 집중 게이지" % rewards["focus_gain"] if rewards["focus_gain"] > 0 else "",
		"highlights": _get_match_highlights(rewards)
	}


## Get match highlights
static func _get_match_highlights(rewards: Dictionary) -> Array:
	var highlights = []

	if rewards["player_goals"] > 0:
		highlights.append("⚽ %d골" % rewards["player_goals"])
	if rewards["player_assists"] > 0:
		highlights.append("👟 %d어시스트" % rewards["player_assists"])
	if rewards["clean_sheet"]:
		highlights.append("🧤 클린시트")
	if rewards["is_motm"]:
		highlights.append("⭐ 맨 오브 더 매치")

	return highlights


## Get match reward BBCode
static func get_match_reward_bbcode(rewards: Dictionary) -> String:
	var display = get_match_reward_display(rewards)

	var result = "[color=#%s][b]%s[/b][/color]\n" % [display["result_color"].to_html(false), display["result_text"]]
	result += "💰 %s | 📊 %s" % [display["money_text"], display["xp_text"]]

	if display["focus_text"] != "":
		result += " | 🔥 %s" % display["focus_text"]

	if display["highlights"].size() > 0:
		result += "\n" + " ".join(display["highlights"])

	return result


# ==================== TRAIT EVOLUTION SYSTEM ====================
# Blue Lock PWC 영감: 중복 획득 → 레벨 캡 상승 / Trait 진화
# Bronze → Silver → Gold 3단계 진화

## Trait tier enum
enum TraitTier { BRONZE = 0, SILVER = 1, GOLD = 2 }

## Tier colors
const TRAIT_TIER_COLORS = {0: Color("#CD7F32"), 1: Color("#C0C0C0"), 2: Color("#FFD700")}  # BRONZE  # SILVER  # GOLD

## Tier icons
const TRAIT_TIER_ICONS = {0: "🥉", 1: "🥈", 2: "🥇"}

## Tier names
const TRAIT_TIER_NAMES = {
	0: {"en": "Bronze", "kr": "브론즈"}, 1: {"en": "Silver", "kr": "실버"}, 2: {"en": "Gold", "kr": "골드"}
}

## Evolution requirements (duplicates needed)
const TRAIT_EVOLUTION_REQUIREMENTS = {0: 1, 1: 3, 2: 7}  # Bronze: 1 (initial)  # Silver: 3 duplicates total  # Gold: 7 duplicates total

## Tier effect multipliers
const TRAIT_TIER_EFFECT_MULT = {0: 1.0, 1: 1.5, 2: 2.0}  # Bronze: base effect  # Silver: +50% effect  # Gold: +100% effect


## Calculate trait tier from duplicate count
static func get_trait_tier_from_duplicates(duplicate_count: int) -> int:
	if duplicate_count >= TRAIT_EVOLUTION_REQUIREMENTS[2]:
		return 2  # GOLD
	elif duplicate_count >= TRAIT_EVOLUTION_REQUIREMENTS[1]:
		return 1  # SILVER
	else:
		return 0  # BRONZE


## Get progress to next tier
static func get_trait_evolution_progress(duplicate_count: int) -> Dictionary:
	var current_tier = get_trait_tier_from_duplicates(duplicate_count)

	if current_tier >= 2:
		return {
			"current_tier": current_tier,
			"next_tier": -1,
			"current_count": duplicate_count,
			"required_count": TRAIT_EVOLUTION_REQUIREMENTS[2],
			"progress_percent": 100,
			"is_max": true
		}

	var next_tier = current_tier + 1
	var current_req = TRAIT_EVOLUTION_REQUIREMENTS[current_tier]
	var next_req = TRAIT_EVOLUTION_REQUIREMENTS[next_tier]
	var progress = float(duplicate_count - current_req) / float(next_req - current_req)

	return {
		"current_tier": current_tier,
		"next_tier": next_tier,
		"current_count": duplicate_count,
		"required_count": next_req,
		"progress_percent": int(progress * 100),
		"is_max": false
	}


## Get trait tier display
static func get_trait_tier_display(tier: int) -> Dictionary:
	return {
		"tier": tier,
		"name": TRAIT_TIER_NAMES.get(tier, TRAIT_TIER_NAMES[0])["en"],
		"name_kr": TRAIT_TIER_NAMES.get(tier, TRAIT_TIER_NAMES[0])["kr"],
		"color": TRAIT_TIER_COLORS.get(tier, TRAIT_TIER_COLORS[0]),
		"icon": TRAIT_TIER_ICONS.get(tier, TRAIT_TIER_ICONS[0]),
		"effect_mult": TRAIT_TIER_EFFECT_MULT.get(tier, 1.0)
	}


## Get trait BBCode with tier
static func get_trait_tier_bbcode(trait_name: String, tier: int) -> String:
	var display = get_trait_tier_display(tier)
	return "[color=#%s]%s %s[/color]" % [display["color"].to_html(false), display["icon"], trait_name]


## Calculate evolved trait effect
static func calculate_evolved_trait_effect(base_effect: float, tier: int) -> float:
	var mult = TRAIT_TIER_EFFECT_MULT.get(tier, 1.0)
	return base_effect * mult


## Get evolution BBCode (for UI display)
static func get_trait_evolution_bbcode(trait_name: String, duplicate_count: int) -> String:
	var progress = get_trait_evolution_progress(duplicate_count)
	var display = get_trait_tier_display(progress["current_tier"])

	var trait_result = "[color=#%s]%s %s[/color]" % [display["color"].to_html(false), display["icon"], trait_name]

	if not progress["is_max"]:
		var next_display = get_trait_tier_display(progress["next_tier"])
		trait_result += (
			" [color=#808080](%d/%d → %s)[/color]"
			% [progress["current_count"], progress["required_count"], next_display["icon"]]
		)
	else:
		trait_result += " [color=#FFD700]★MAX★[/color]"

	return trait_result


# ==================== ENERGY DANGER ZONE SYSTEM ====================
# Uma Musume 영감: 에너지 30% 이하 = 훈련 실패율 급증
# 위험 구간 경고 및 강행 훈련 고위험 고보상

## Energy thresholds
const ENERGY_CRITICAL_THRESHOLD = 0.2  # 20% 이하 = 위험
const ENERGY_WARNING_THRESHOLD = 0.3  # 30% 이하 = 경고
const ENERGY_CAUTION_THRESHOLD = 0.5  # 50% 이하 = 주의

## Energy zone colors
const ENERGY_ZONE_COLORS = {
	"safe": Color("#32CD32"), "caution": Color("#FFD700"), "warning": Color("#FF8C00"), "critical": Color("#DC143C")  # 초록 (50%+)  # 노란 (30-50%)  # 주황 (20-30%)  # 빨강 (20%-)
}

## Training failure rates by zone
const ENERGY_FAILURE_RATES = {"safe": 0.05, "caution": 0.10, "warning": 0.25, "critical": 0.50}  # 5% 실패  # 10% 실패  # 25% 실패  # 50% 실패

## Training injury risk by zone
const ENERGY_INJURY_RISK = {"safe": 0.02, "caution": 0.05, "warning": 0.15, "critical": 0.30}  # 2% 부상  # 5% 부상  # 15% 부상  # 30% 부상

## Forced training bonus (risk-reward)
const ENERGY_FORCED_TRAINING_BONUS = {"safe": 1.0, "caution": 1.1, "warning": 1.25, "critical": 1.5}  # 일반  # +10%  # +25%  # +50% (고위험 고보상)


## Get energy zone from percentage
static func get_energy_zone(energy_percent: float) -> String:
	if energy_percent <= ENERGY_CRITICAL_THRESHOLD:
		return "critical"
	elif energy_percent <= ENERGY_WARNING_THRESHOLD:
		return "warning"
	elif energy_percent <= ENERGY_CAUTION_THRESHOLD:
		return "caution"
	else:
		return "safe"


## Get energy danger display
static func get_energy_danger_display(energy_percent: float) -> Dictionary:
	var zone = get_energy_zone(energy_percent)
	var color = ENERGY_ZONE_COLORS[zone]

	var status: String
	var status_kr: String
	var icon: String

	match zone:
		"critical":
			status = "CRITICAL"
			status_kr = "위험"
			icon = "🚨"
		"warning":
			status = "WARNING"
			status_kr = "경고"
			icon = "⚠️"
		"caution":
			status = "CAUTION"
			status_kr = "주의"
			icon = "⚡"
		_:
			status = "SAFE"
			status_kr = "안전"
			icon = "✅"

	return {
		"zone": zone,
		"color": color,
		"status": status,
		"status_kr": status_kr,
		"icon": icon,
		"failure_rate": ENERGY_FAILURE_RATES[zone],
		"injury_risk": ENERGY_INJURY_RISK[zone],
		"forced_bonus": ENERGY_FORCED_TRAINING_BONUS[zone],
		"percent": int(energy_percent * 100)
	}


## Get energy danger BBCode
static func get_energy_danger_bbcode(energy_percent: float) -> String:
	var display = get_energy_danger_display(energy_percent)

	var energy_result = (
		"[color=#%s]%s %s %d%%[/color]"
		% [display["color"].to_html(false), display["icon"], display["status_kr"], display["percent"]]
	)

	if display["zone"] != "safe":
		energy_result += (
			"\n[color=#808080]실패율 %d%% | 부상위험 %d%%[/color]"
			% [int(display["failure_rate"] * 100), int(display["injury_risk"] * 100)]
		)
		if display["forced_bonus"] > 1.0:
			energy_result += "\n[color=#FFD700]강행 시 효과 +%d%%[/color]" % [int((display["forced_bonus"] - 1.0) * 100)]

	return energy_result


## Check if training should be warned
static func should_warn_training(energy_percent: float) -> bool:
	return energy_percent <= ENERGY_WARNING_THRESHOLD


## Calculate training outcome with energy factor
static func calculate_training_with_energy(
	base_gain: float, energy_percent: float, is_forced: bool = false
) -> Dictionary:
	var zone = get_energy_zone(energy_percent)
	var failure_rate = ENERGY_FAILURE_RATES[zone]
	var injury_risk = ENERGY_INJURY_RISK[zone]
	var bonus = ENERGY_FORCED_TRAINING_BONUS[zone] if is_forced else 1.0

	# Random rolls
	var roll_fail = randf()
	var roll_injury = randf()

	var failed = roll_fail < failure_rate
	var injured = roll_injury < injury_risk

	var actual_gain = 0.0
	if not failed:
		actual_gain = base_gain * bonus

	return {
		"base_gain": base_gain,
		"actual_gain": actual_gain,
		"bonus_mult": bonus,
		"failed": failed,
		"injured": injured,
		"zone": zone,
		"was_forced": is_forced
	}


# ==================== ALUMNI/LEGACY SYSTEM ====================
# Uma Musume 인자계승 영감: 졸업생이 멘토로 돌아옴
# 졸업생 관리 및 재방문 시스템

## Alumni status enum
enum AlumniStatus { ACTIVE = 0, RETIRED = 1, MENTOR = 2, UNAVAILABLE = 3 }  # 현역 프로  # 은퇴  # 멘토 활동 중  # 연락 두절

## Alumni tier based on career success
enum AlumniTier { AMATEUR = 0, LOWER_LEAGUE = 1, MID_LEAGUE = 2, TOP_LEAGUE = 3, LEGEND = 4 }  # 비프로 (졸업만)  # 하위리그  # 중위리그  # 상위리그  # 전설 (국대/챔스)

## Alumni tier colors
const ALUMNI_TIER_COLORS = {
	0: Color("#808080"), 1: Color("#CD7F32"), 2: Color("#C0C0C0"), 3: Color("#FFD700"), 4: Color("#E5E4E2")  # 회색  # 브론즈  # 실버  # 골드  # 플래티넘
}

## Alumni tier icons
const ALUMNI_TIER_ICONS = {0: "🎓", 1: "⚽", 2: "🏅", 3: "🏆", 4: "👑"}

## Mentoring effectiveness by alumni tier
const ALUMNI_MENTOR_EFFECTIVENESS = {0: 0.05, 1: 0.10, 2: 0.15, 3: 0.25, 4: 0.40}  # +5%  # +10%  # +15%  # +25%  # +40%

## Visit frequency (days) by tier
const ALUMNI_VISIT_FREQUENCY = {0: 60, 1: 45, 2: 30, 3: 21, 4: 14}  # 2개월  # 1.5개월  # 1개월  # 3주  # 2주

## Legacy trait chance by tier
const ALUMNI_LEGACY_TRAIT_CHANCE = {0: 0.05, 1: 0.10, 2: 0.20, 3: 0.35, 4: 0.50}  # 5%  # 10%  # 20%  # 35%  # 50%


## Get alumni tier display
static func get_alumni_tier_display(tier: int) -> Dictionary:
	var tier_names = {
		0: {"en": "Amateur", "kr": "아마추어"},
		1: {"en": "Lower League", "kr": "하위리그"},
		2: {"en": "Mid League", "kr": "중위리그"},
		3: {"en": "Top League", "kr": "상위리그"},
		4: {"en": "Legend", "kr": "레전드"}
	}

	return {
		"tier": tier,
		"name": tier_names.get(tier, tier_names[0])["en"],
		"name_kr": tier_names.get(tier, tier_names[0])["kr"],
		"color": ALUMNI_TIER_COLORS.get(tier, ALUMNI_TIER_COLORS[0]),
		"icon": ALUMNI_TIER_ICONS.get(tier, ALUMNI_TIER_ICONS[0]),
		"mentor_effectiveness": ALUMNI_MENTOR_EFFECTIVENESS.get(tier, 0.05),
		"visit_frequency": ALUMNI_VISIT_FREQUENCY.get(tier, 60),
		"legacy_trait_chance": ALUMNI_LEGACY_TRAIT_CHANCE.get(tier, 0.05)
	}


## Get alumni status display
static func get_alumni_status_display(status: int) -> Dictionary:
	var status_info = {
		0: {"en": "Active Pro", "kr": "현역 프로", "icon": "🏃", "color": Color("#32CD32")},
		1: {"en": "Retired", "kr": "은퇴", "icon": "🏠", "color": Color("#808080")},
		2: {"en": "Mentoring", "kr": "멘토 활동", "icon": "📚", "color": Color("#4169E1")},
		3: {"en": "Unavailable", "kr": "연락 두절", "icon": "❓", "color": Color("#DC143C")}
	}

	var info = status_info.get(status, status_info[0])
	return {"status": status, "name": info["en"], "name_kr": info["kr"], "icon": info["icon"], "color": info["color"]}


## Create alumni record from graduating player
static func create_alumni_record(player_data: Dictionary) -> Dictionary:
	# Determine tier based on player stats at graduation
	var avg_stat = 0
	var stat_count = 0
	for key in player_data:
		if key in ["pace", "power", "technical", "shooting", "passing", "defending"]:
			avg_stat += player_data[key]
			stat_count += 1

	if stat_count > 0:
		avg_stat = avg_stat / stat_count

	var tier = 0
	if avg_stat >= 85:
		tier = 4  # LEGEND potential
	elif avg_stat >= 75:
		tier = 3  # TOP_LEAGUE
	elif avg_stat >= 65:
		tier = 2  # MID_LEAGUE
	elif avg_stat >= 55:
		tier = 1  # LOWER_LEAGUE

	return {
		"name": player_data.get("name", "Unknown"),
		"graduation_year": player_data.get("year", 2025),
		"position": player_data.get("position", "MF"),
		"tier": tier,
		"status": 0,  # ACTIVE
		"traits": player_data.get("traits", []),
		"specialty_stats": _get_top_stats(player_data),
		"mentor_visits": 0,
		"last_visit_day": -1,
		"legacy_given": []
	}


## Get top 2 stats for specialty
static func _get_top_stats(player_data: Dictionary) -> Array:
	var stats = []
	for key in ["pace", "power", "technical", "shooting", "passing", "defending"]:
		if player_data.has(key):
			stats.append({"name": key, "value": player_data[key]})

	stats.sort_custom(func(a, b): return a["value"] > b["value"])

	var top_stats = []
	for i in range(mini(2, stats.size())):
		top_stats.append(stats[i]["name"])
	return top_stats


## Calculate legacy effect when alumni mentors
static func calculate_alumni_legacy_effect(alumni_record: Dictionary, _trainee_data: Dictionary) -> Dictionary:
	var tier = alumni_record.get("tier", 0)
	var display = get_alumni_tier_display(tier)

	# Training bonus for specialty stats
	var specialty_bonus = {}
	for stat in alumni_record.get("specialty_stats", []):
		specialty_bonus[stat] = display["mentor_effectiveness"]

	# Trait inheritance chance
	var trait_hint = ""
	var alumni_traits = alumni_record.get("traits", [])
	if alumni_traits.size() > 0 and randf() < display["legacy_trait_chance"]:
		trait_hint = alumni_traits[randi() % alumni_traits.size()]

	return {
		"training_bonus": specialty_bonus,
		"overall_bonus": display["mentor_effectiveness"],
		"trait_hint": trait_hint,
		"alumni_name": alumni_record.get("name", "Unknown"),
		"alumni_tier": tier,
		"tier_display": display
	}


## Get alumni BBCode
static func get_alumni_bbcode(alumni_record: Dictionary) -> String:
	var tier_display = get_alumni_tier_display(alumni_record.get("tier", 0))
	var status_display = get_alumni_status_display(alumni_record.get("status", 0))

	return (
		"[color=#%s]%s %s[/color] [color=#%s](%s)[/color]"
		% [
			tier_display["color"].to_html(false),
			tier_display["icon"],
			alumni_record.get("name", "Unknown"),
			status_display["color"].to_html(false),
			status_display["name_kr"]
		]
	)


## Get alumni card display (for UI)
static func get_alumni_card_display(alumni_record: Dictionary) -> Dictionary:
	var tier_display = get_alumni_tier_display(alumni_record.get("tier", 0))
	var status_display = get_alumni_status_display(alumni_record.get("status", 0))

	return {
		"name": alumni_record.get("name", "Unknown"),
		"position": alumni_record.get("position", "MF"),
		"graduation_year": alumni_record.get("graduation_year", 2025),
		"tier": tier_display,
		"status": status_display,
		"specialty": alumni_record.get("specialty_stats", []),
		"traits": alumni_record.get("traits", []),
		"mentor_visits": alumni_record.get("mentor_visits", 0),
		"can_mentor": alumni_record.get("status", 0) in [0, 1, 2]  # Not unavailable
	}


# ==================== IN-GAME FEEDBACK SYSTEM ====================
# Retro Bowl/New Star Games 영감: 플레이어 피드백 적극 수용
# 인게임 피드백 UI 및 데이터 구조

## Feedback categories
enum FeedbackCategory { BUG = 0, SUGGESTION = 1, BALANCE = 2, UI_UX = 3, OTHER = 4 }  # 버그 리포트  # 기능 제안  # 밸런스 의견  # UI/UX 피드백  # 기타

## Feedback priority
enum FeedbackPriority { LOW = 0, MEDIUM = 1, HIGH = 2, CRITICAL = 3 }

## Feedback category icons and colors
const FEEDBACK_CATEGORY_INFO = {
	0: {"icon": "🐛", "color": Color("#DC143C"), "name": "Bug", "name_kr": "버그"},
	1: {"icon": "💡", "color": Color("#FFD700"), "name": "Suggestion", "name_kr": "제안"},
	2: {"icon": "⚖️", "color": Color("#4169E1"), "name": "Balance", "name_kr": "밸런스"},
	3: {"icon": "🎨", "color": Color("#32CD32"), "name": "UI/UX", "name_kr": "UI/UX"},
	4: {"icon": "📝", "color": Color("#808080"), "name": "Other", "name_kr": "기타"}
}

## Priority colors
const FEEDBACK_PRIORITY_COLORS = {0: Color("#808080"), 1: Color("#FFD700"), 2: Color("#FF8C00"), 3: Color("#DC143C")}  # LOW - 회색  # MEDIUM - 노란  # HIGH - 주황  # CRITICAL - 빨강


## Create feedback record
static func create_feedback_record(
	category: int, title: String, description: String, priority: int = 1, screenshot_path: String = ""
) -> Dictionary:
	return {
		"id": "fb_%d_%d" % [Time.get_unix_time_from_system(), randi() % 1000],
		"category": category,
		"title": title,
		"description": description,
		"priority": priority,
		"screenshot_path": screenshot_path,
		"timestamp": Time.get_datetime_string_from_system(),
		"game_version": "0.1.0",  # TODO: Get from project settings
		"device_info": _get_device_info(),
		"status": "pending"
	}


## Get device info for feedback
static func _get_device_info() -> Dictionary:
	return {
		"os": OS.get_name(),
		"locale": OS.get_locale(),
		"screen_size": "%dx%d" % [DisplayServer.window_get_size().x, DisplayServer.window_get_size().y]
	}


## Get feedback category display
static func get_feedback_category_display(category: int) -> Dictionary:
	var info = FEEDBACK_CATEGORY_INFO.get(category, FEEDBACK_CATEGORY_INFO[4])
	return {
		"category": category,
		"icon": info["icon"],
		"color": info["color"],
		"name": info["name"],
		"name_kr": info["name_kr"]
	}


## Get feedback priority display
static func get_feedback_priority_display(priority: int) -> Dictionary:
	var names = {
		0: {"en": "Low", "kr": "낮음"},
		1: {"en": "Medium", "kr": "보통"},
		2: {"en": "High", "kr": "높음"},
		3: {"en": "Critical", "kr": "긴급"}
	}
	var name_info = names.get(priority, names[1])
	return {
		"priority": priority,
		"name": name_info["en"],
		"name_kr": name_info["kr"],
		"color": FEEDBACK_PRIORITY_COLORS.get(priority, FEEDBACK_PRIORITY_COLORS[1])
	}


## Get feedback BBCode
static func get_feedback_bbcode(feedback: Dictionary) -> String:
	var cat = get_feedback_category_display(feedback.get("category", 4))
	var pri = get_feedback_priority_display(feedback.get("priority", 1))

	return (
		"[color=#%s]%s[/color] [color=#%s][%s][/color] %s"
		% [
			cat["color"].to_html(false),
			cat["icon"],
			pri["color"].to_html(false),
			pri["name_kr"],
			feedback.get("title", "")
		]
	)


# ==================== TILE & CARD UI SYSTEM ====================
# FM Mobile 영감: Tile = 요약, Card = 상세
# 정보 위계를 위한 UI 헬퍼

## Card size types
enum CardSize { MINI = 0, TILE = 1, COMPACT = 2, FULL = 3 }  # 아이콘만  # 타일 (요약)  # 컴팩트 카드  # 풀 카드 (상세)

## Card size dimensions (width x height)
const CARD_DIMENSIONS = {0: Vector2(48, 48), 1: Vector2(120, 80), 2: Vector2(200, 120), 3: Vector2(320, 400)}  # MINI  # TILE  # COMPACT  # FULL


## Player tile data (minimal view)
static func get_player_tile_data(player: Dictionary) -> Dictionary:
	var position = player.get("position", "MF")
	var pos_color = get_position_color(position)

	return {
		"name": player.get("name", "Unknown"),
		"position": position,
		"position_color": pos_color,
		"overall": player.get("overall", 50),
		"mood_icon": get_mood_icon(player.get("mood", 3)),
		"card_size": CardSize.TILE
	}


## Player compact card data
static func get_player_compact_data(player: Dictionary) -> Dictionary:
	var tile = get_player_tile_data(player)
	var mood = get_mood_display(player.get("mood", 3))
	var stamina = player.get("stamina", 100)
	var energy_display = get_energy_danger_display(float(stamina) / 100.0)

	tile["mood"] = mood
	tile["stamina"] = stamina
	tile["energy_zone"] = energy_display
	tile["traits_count"] = player.get("traits", []).size()
	tile["card_size"] = CardSize.COMPACT

	return tile


## Player full card data
static func get_player_full_card_data(player: Dictionary) -> Dictionary:
	var compact = get_player_compact_data(player)

	# Add detailed stats
	compact["stats"] = {
		"pace": player.get("pace", 50),
		"power": player.get("power", 50),
		"technical": player.get("technical", 50),
		"shooting": player.get("shooting", 50),
		"passing": player.get("passing", 50),
		"defending": player.get("defending", 50)
	}

	# Add traits with tiers
	var traits_display = []
	for trait_info in player.get("traits", []):
		if trait_info is Dictionary:
			var tier = trait_info.get("tier", 0)
			traits_display.append(get_trait_tier_display(tier))
		else:
			traits_display.append(get_trait_tier_display(0))
	compact["traits_display"] = traits_display

	# Add potential
	var potential = player.get("potential", 0.9)
	compact["potential"] = get_potential_display(potential)

	# Add achievements summary
	var achievements = player.get("unlocked_achievements", [])
	compact["achievements"] = get_achievement_progress_display(achievements)

	compact["card_size"] = CardSize.FULL

	return compact


## Generate player card BBCode (adaptive to size)
static func get_player_card_bbcode(player: Dictionary, size: int = 1) -> String:
	match size:
		0:  # MINI
			var pos = player.get("position", "MF")
			var pos_color = get_position_color(pos)
			return "[color=#%s]%s[/color]" % [pos_color.to_html(false), pos]

		1:  # TILE
			var data = get_player_tile_data(player)
			return (
				"[color=#%s]%s[/color] %s %s [%d]"
				% [
					data["position_color"].to_html(false),
					data["position"],
					data["name"],
					data["mood_icon"],
					data["overall"]
				]
			)

		2:  # COMPACT
			var data = get_player_compact_data(player)
			var line1 = (
				"[color=#%s]%s[/color] [b]%s[/b] OVR:%d"
				% [data["position_color"].to_html(false), data["position"], data["name"], data["overall"]]
			)
			var line2 = (
				"%s %s | %s %d%%"
				% [data["mood"]["icon"], data["mood"]["name_kr"], data["energy_zone"]["icon"], data["stamina"]]
			)
			return line1 + "\n" + line2

		3, _:  # FULL
			var data = get_player_full_card_data(player)
			var lines = []

			# Header
			lines.append(
				(
					"[color=#%s][b]%s[/b][/color] %s"
					% [data["position_color"].to_html(false), data["position"], data["name"]]
				)
			)
			lines.append("OVR: [b]%d[/b] | %s" % [data["overall"], data["potential"]["grade"]])

			# Stats
			var stats = data["stats"]
			lines.append("PAC:%d POW:%d TEC:%d" % [stats["pace"], stats["power"], stats["technical"]])
			lines.append("SHO:%d PAS:%d DEF:%d" % [stats["shooting"], stats["passing"], stats["defending"]])

			# Condition
			lines.append(
				(
					"%s %s | %s"
					% [
						data["mood"]["icon"],
						data["mood"]["name_kr"],
						get_energy_danger_bbcode(float(data["stamina"]) / 100.0).split("\n")[0]
					]
				)
			)

			# Traits count
			if data["traits_display"].size() > 0:
				lines.append("Traits: %d" % data["traits_display"].size())

			return "\n".join(lines)


## Facility tile data
static func get_facility_tile_data(facility_type: int, level: int) -> Dictionary:
	var display = get_facility_display(facility_type, level)
	return {
		"type": facility_type,
		"level": level,
		"name_kr": display["name_kr"],
		"icon": display["icon"],
		"level_icon": display["level_icon"],
		"color": display["color"],
		"bonus": display["bonus"],
		"card_size": CardSize.TILE
	}


## Match result tile
static func get_match_tile_data(match_data: Dictionary) -> Dictionary:
	var result = match_data.get("result", 0)
	var result_colors = {0: Color("#DC143C"), 1: Color("#FFD700"), 2: Color("#32CD32")}
	var result_icons = {0: "❌", 1: "➖", 2: "✅"}
	var result_names = {0: "패배", 1: "무승부", 2: "승리"}

	return {
		"opponent": match_data.get("opponent", "Unknown"),
		"score_home": match_data.get("score_home", 0),
		"score_away": match_data.get("score_away", 0),
		"result": result,
		"result_icon": result_icons.get(result, "❓"),
		"result_color": result_colors.get(result, Color.WHITE),
		"result_name": result_names.get(result, "?"),
		"card_size": CardSize.TILE
	}


# ==================== UNIFIED TRAINING CALCULATOR ====================
# 모든 시스템을 통합하는 최종 훈련 효율 계산기
# 시설×Mood×StatusEffect×시너지×집중훈련×잠재력×멘토×분위기×에너지×Trait 진화


## Calculate final training efficiency with ALL factors
static func calculate_ultimate_training_efficiency(
	# Base parameters
	base_stat_gain: float,
	target_stat: String,
	# Player state
	player_mood: int,
	player_stamina_percent: float,
	player_potential: float,
	player_status_effects: Array,
	player_trait_tiers: Dictionary,  # {trait_id: tier}
	# Team/Academy state
	facility_levels: Dictionary,
	atmosphere_level: int,
	is_intensive_week: bool,
	intensive_focus_stat: String,
	# Partner/Mentor
	partner_synergy_level: int,
	mentor_tier: int,
	mentor_specialty: Array,
	# Options
	is_forced_training: bool = false
) -> Dictionary:
	var multipliers = {}
	var warnings = []
	var bonuses = []

	# 1. Facility bonus
	var facility_type = _stat_to_facility_type(target_stat)
	var facility_bonus = get_facility_bonus(facility_type, facility_levels.get(facility_type, 0))
	multipliers["facility"] = 1.0 + facility_bonus
	if facility_bonus > 0:
		bonuses.append("시설 +%d%%" % int(facility_bonus * 100))

	# 2. Mood bonus
	var mood_mult = get_mood_multiplier(player_mood)
	multipliers["mood"] = mood_mult
	if mood_mult != 1.0:
		var mood_pct = int((mood_mult - 1.0) * 100)
		if mood_pct > 0:
			bonuses.append("기분 +%d%%" % mood_pct)
		else:
			warnings.append("기분 %d%%" % mood_pct)

	# 3. Status effects
	var status_mult = 1.0
	for effect_val in player_status_effects:
		var effect_int: int = int(effect_val)
		var effect_training_mult: float = get_status_training_mult(effect_int)
		status_mult *= effect_training_mult
		if effect_training_mult < 1.0:
			var effect_label: String = get_status_effect_label(effect_int)
			warnings.append("%s %d%%" % [effect_label, int((effect_training_mult - 1.0) * 100)])
	multipliers["status_effects"] = status_mult

	# 4. Partner synergy
	var synergy_mult = get_synergy_mult(partner_synergy_level)
	multipliers["synergy"] = synergy_mult
	if synergy_mult > 1.0:
		bonuses.append("시너지 +%d%%" % int((synergy_mult - 1.0) * 100))

	# 5. Intensive week
	var intensive_mult = 1.0
	if is_intensive_week:
		intensive_mult = get_intensive_training_mult(true)
		if target_stat != intensive_focus_stat:
			intensive_mult = 1.0  # No bonus for non-focus stats
		if intensive_mult > 1.0:
			bonuses.append("집중훈련 +%d%%" % int((intensive_mult - 1.0) * 100))
	multipliers["intensive"] = intensive_mult

	# 6. Potential
	var potential_display = get_potential_display(player_potential)
	multipliers["potential"] = player_potential
	if player_potential > 1.0:
		bonuses.append("잠재력 +%d%%" % int((player_potential - 1.0) * 100))
	elif player_potential < 1.0:
		warnings.append("잠재력 %d%%" % int((player_potential - 1.0) * 100))

	# 7. Mentor
	var mentor_mult = 1.0
	if mentor_tier >= 0:
		var mentor_display = get_mentor_display(mentor_tier)
		mentor_mult = 1.0 + mentor_display["training_bonus"]
		if target_stat in mentor_specialty:
			mentor_mult += 0.10  # Additional specialty bonus
			bonuses.append("멘토(전문) +%d%%" % int((mentor_mult - 1.0) * 100))
		elif mentor_display["training_bonus"] > 0:
			bonuses.append("멘토 +%d%%" % int(mentor_display["training_bonus"] * 100))
	multipliers["mentor"] = mentor_mult

	# 8. Atmosphere
	var atmo_display = get_atmosphere_display(atmosphere_level)
	multipliers["atmosphere"] = atmo_display["training_mult"]
	if atmo_display["training_mult"] != 1.0:
		var atmo_pct = int((atmo_display["training_mult"] - 1.0) * 100)
		if atmo_pct > 0:
			bonuses.append("분위기 +%d%%" % atmo_pct)
		else:
			warnings.append("분위기 %d%%" % atmo_pct)

	# 9. Energy/Stamina zone
	var energy_result = calculate_training_with_energy(1.0, player_stamina_percent, is_forced_training)
	multipliers["energy"] = energy_result["bonus_mult"]
	if energy_result["zone"] != "safe":
		warnings.append("체력 %s" % energy_result["zone"])
		if is_forced_training and energy_result["bonus_mult"] > 1.0:
			bonuses.append("강행 +%d%%" % int((energy_result["bonus_mult"] - 1.0) * 100))

	# 10. Trait tier bonus (if relevant trait exists)
	var trait_mult = 1.0
	var relevant_trait = _stat_to_relevant_trait(target_stat)
	if relevant_trait != "" and player_trait_tiers.has(relevant_trait):
		var tier = player_trait_tiers[relevant_trait]
		trait_mult = TRAIT_TIER_EFFECT_MULT.get(tier, 1.0)
		if trait_mult > 1.0:
			var tier_display = get_trait_tier_display(tier)
			bonuses.append("Trait(%s) +%d%%" % [tier_display["icon"], int((trait_mult - 1.0) * 100)])
	multipliers["trait"] = trait_mult

	# Calculate total multiplier
	var total_mult = 1.0
	for key in multipliers:
		total_mult *= multipliers[key]

	# Calculate final gain
	var final_gain = base_stat_gain * total_mult

	# Apply failure chance from energy
	var failed = energy_result["failed"]
	var injured = energy_result["injured"]

	if failed:
		final_gain = 0.0
		warnings.append("훈련 실패!")

	return {
		"base_gain": base_stat_gain,
		"final_gain": final_gain,
		"total_multiplier": total_mult,
		"multipliers": multipliers,
		"bonuses": bonuses,
		"warnings": warnings,
		"failed": failed,
		"injured": injured,
		"target_stat": target_stat,
		"percent_bonus": int((total_mult - 1.0) * 100)
	}


## Map stat to facility type
static func _stat_to_facility_type(stat: String) -> int:
	match stat:
		"pace":
			return 1  # GYM
		"power":
			return 1  # GYM
		"technical":
			return 2  # TECH_CENTER
		"shooting":
			return 2  # TECH_CENTER
		"passing":
			return 2  # TECH_CENTER
		"defending":
			return 3  # TACTICS_ROOM
		_:
			return 0  # TRAINING_GROUND


## Map stat to relevant trait
static func _stat_to_relevant_trait(stat: String) -> String:
	match stat:
		"pace":
			return "speed_demon"
		"power":
			return "powerhouse"
		"technical":
			return "technical_master"
		"shooting":
			return "clinical"
		"passing":
			return "playmaker"
		"defending":
			return "rock_solid"
		_:
			return ""


## Get training efficiency BBCode summary
static func get_training_efficiency_bbcode(result: Dictionary) -> String:
	var lines = []

	# Header with final result
	var gain_color = "#32CD32" if not result["failed"] else "#DC143C"
	lines.append(
		(
			"[color=%s][b]+%.1f %s[/b][/color] (%+d%%)"
			% [gain_color, result["final_gain"], result["target_stat"], result["percent_bonus"]]
		)
	)

	# Bonuses
	if result["bonuses"].size() > 0:
		lines.append("[color=#32CD32]" + " | ".join(result["bonuses"]) + "[/color]")

	# Warnings
	if result["warnings"].size() > 0:
		lines.append("[color=#FF8C00]" + " | ".join(result["warnings"]) + "[/color]")

	# Injury warning
	if result["injured"]:
		lines.append("[color=#DC143C]⚠️ 부상 발생![/color]")

	return "\n".join(lines)
