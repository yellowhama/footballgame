extends Node
class_name TacticalPresets

## Tactical Presets Data
##
## Phase 3.3: UI 프리셋 선택 메뉴
##
## 6개 유명 전술 프리셋 데이터를 정의하고,
## Rust TeamInstructions 구조체로 변환할 수 있도록 제공합니다.
##
## 사용법:
##   var preset = TacticalPresets.get_preset("tiki_taka")
##   var instructions = TacticalPresets.get_instructions("tiki_taka")

## 6개 전술 프리셋 데이터
const PRESETS = {
	"tiki_taka":
	{
		"id": "tiki_taka",
		"name": "티키타카",
		"name_en": "Tiki-Taka",
		"icon": "⚽",
		"color": Color(0.2, 0.6, 1.0),  # Blue
		"description": "짧은 패스 중심의 점유율 축구.\n바르셀로나의 대표 전술.",
		"description_en": "Possession-based football with short passes.\nBarcelona's signature style.",
		"instructions":
		{
			"tempo": "Fast",
			"pressing": "High",
			"width": "Wide",
			"build_up_play": "ShortPassing",
			"defensive_line": "High"
		}
	},
	"gegenpressing":
	{
		"id": "gegenpressing",
		"name": "게겐프레싱",
		"name_en": "Gegenpressing",
		"icon": "⚡",
		"color": Color(1.0, 0.2, 0.2),  # Red
		"description": "볼을 빼앗긴 즉시 강하게 압박.\n리버풀의 클롭 스타일.",
		"description_en": "Immediate high pressing after losing the ball.\nKlopp's Liverpool style.",
		"instructions":
		{
			"tempo": "VeryFast",
			"pressing": "VeryHigh",
			"width": "Medium",
			"build_up_play": "Mixed",
			"defensive_line": "High"
		}
	},
	"catenaccio":
	{
		"id": "catenaccio",
		"name": "카테나치오",
		"name_en": "Catenaccio",
		"icon": "🛡️",
		"color": Color(0.3, 0.3, 0.3),  # Gray
		"description": "수비 중심의 전술.\n이탈리아 전통 수비 전술.",
		"description_en": "Defensive tactics focused on solid structure.\nItalian traditional defense.",
		"instructions":
		{
			"tempo": "Slow",
			"pressing": "Low",
			"width": "Narrow",
			"build_up_play": "DirectPassing",
			"defensive_line": "Low"
		}
	},
	"counter_attack":
	{
		"id": "counter_attack",
		"name": "역습",
		"name_en": "Counter-Attack",
		"icon": "🏃",
		"color": Color(1.0, 0.6, 0.0),  # Orange
		"description": "수비 후 빠른 역습.\n공간을 활용한 빠른 전환.",
		"description_en": "Quick counter-attacks after defending.\nUtilizing space for fast transitions.",
		"instructions":
		{
			"tempo": "Fast",
			"pressing": "Medium",
			"width": "Wide",
			"build_up_play": "DirectPassing",
			"defensive_line": "Medium"
		}
	},
	"high_line":
	{
		"id": "high_line",
		"name": "높은 라인",
		"name_en": "High-Line",
		"icon": "⬆️",
		"color": Color(0.0, 0.8, 0.4),  # Green
		"description": "높은 수비 라인으로 압박.\n오프사이드 트랩 활용.",
		"description_en": "High defensive line with pressing.\nUtilizing offside trap.",
		"instructions":
		{
			"tempo": "Medium",
			"pressing": "High",
			"width": "Wide",
			"build_up_play": "ShortPassing",
			"defensive_line": "VeryHigh"
		}
	},
	"park_the_bus":
	{
		"id": "park_the_bus",
		"name": "극단적 수비",
		"name_en": "Park-the-Bus",
		"icon": "🚌",
		"color": Color(0.5, 0.5, 0.5),  # Dark Gray
		"description": "모든 선수를 수비에 투입.\n극단적 수비 전술.",
		"description_en": "All players defending.\nExtreme defensive tactics.",
		"instructions":
		{
			"tempo": "VerySlow",
			"pressing": "VeryLow",
			"width": "Narrow",
			"build_up_play": "DirectPassing",
			"defensive_line": "VeryLow"
		}
	}
}

## 프리셋 ID 목록 (순서 보장)
const PRESET_IDS = ["tiki_taka", "gegenpressing", "catenaccio", "counter_attack", "high_line", "park_the_bus"]


## 프리셋 데이터 가져오기
static func get_preset(preset_id: String) -> Dictionary:
	"""
	프리셋 ID로 전체 프리셋 데이터 반환

	Args:
		preset_id: 프리셋 ID (예: "tiki_taka")

	Returns:
		Dictionary: 프리셋 데이터 (또는 빈 Dictionary)
	"""
	if PRESETS.has(preset_id):
		return PRESETS[preset_id]
	return {}


## 모든 프리셋 목록 반환
static func get_all_presets() -> Array:
	"""
	모든 프리셋을 배열로 반환 (순서 보장)

	Returns:
		Array: 프리셋 Dictionary 배열
	"""
	var presets = []
	for id in PRESET_IDS:
		presets.append(PRESETS[id])
	return presets


## TeamInstructions Dictionary로 변환
static func get_instructions(preset_id: String) -> Dictionary:
	"""
	프리셋 ID로 TeamInstructions Dictionary 반환

	Rust TeamInstructions 구조체와 호환되는 형식:
	{
		"tempo": "Fast",
		"pressing": "High",
		"width": "Wide",
		"build_up_play": "ShortPassing",
		"defensive_line": "High"
	}

	Args:
		preset_id: 프리셋 ID

	Returns:
		Dictionary: TeamInstructions (또는 빈 Dictionary)
	"""
	var preset = get_preset(preset_id)
	if preset.is_empty():
		return {}
	return preset.get("instructions", {})


## 프리셋 이름 반환 (한국어)
static func get_name(preset_id: String) -> String:
	"""
	프리셋 ID로 한국어 이름 반환

	Args:
		preset_id: 프리셋 ID

	Returns:
		String: 한국어 이름 (또는 빈 문자열)
	"""
	var preset = get_preset(preset_id)
	return preset.get("name", "")


## 프리셋 이름 반환 (영어)
static func get_name_en(preset_id: String) -> String:
	"""
	프리셋 ID로 영어 이름 반환

	Args:
		preset_id: 프리셋 ID

	Returns:
		String: 영어 이름 (또는 빈 문자열)
	"""
	var preset = get_preset(preset_id)
	return preset.get("name_en", "")


## 프리셋 아이콘 반환
static func get_icon(preset_id: String) -> String:
	"""
	프리셋 ID로 아이콘 반환

	Args:
		preset_id: 프리셋 ID

	Returns:
		String: 아이콘 (이모지) (또는 빈 문자열)
	"""
	var preset = get_preset(preset_id)
	return preset.get("icon", "")


## 프리셋 색상 반환
static func get_color(preset_id: String) -> Color:
	"""
	프리셋 ID로 테마 색상 반환

	Args:
		preset_id: 프리셋 ID

	Returns:
		Color: 테마 색상 (또는 White)
	"""
	var preset = get_preset(preset_id)
	return preset.get("color", Color.WHITE)


## 프리셋 설명 반환 (한국어)
static func get_description(preset_id: String) -> String:
	"""
	프리셋 ID로 한국어 설명 반환

	Args:
		preset_id: 프리셋 ID

	Returns:
		String: 한국어 설명 (또는 빈 문자열)
	"""
	var preset = get_preset(preset_id)
	return preset.get("description", "")


## 프리셋 설명 반환 (영어)
static func get_description_en(preset_id: String) -> String:
	"""
	프리셋 ID로 영어 설명 반환

	Args:
		preset_id: 프리셋 ID

	Returns:
		String: 영어 설명 (또는 빈 문자열)
	"""
	var preset = get_preset(preset_id)
	return preset.get("description_en", "")


## 프리셋 존재 여부 확인
static func has_preset(preset_id: String) -> bool:
	"""
	프리셋 ID가 존재하는지 확인

	Args:
		preset_id: 프리셋 ID

	Returns:
		bool: 존재 여부
	"""
	return PRESETS.has(preset_id)


## 프리셋 수 반환
static func get_preset_count() -> int:
	"""
	전체 프리셋 수 반환

	Returns:
		int: 프리셋 수 (6)
	"""
	return PRESET_IDS.size()
