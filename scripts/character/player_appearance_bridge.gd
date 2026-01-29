# player_appearance_bridge.gd
# Dictionary ↔ CharacterAppearance Resource 변환 유틸리티
# Socceralia 스프라이트 스키마 지원 (2025-12-08)
#
# 참조: assets/ui/2025-12-08_CHARACTER_SPRITE_USAGE_FOR_UI.md
class_name PlayerAppearanceBridge
extends RefCounted

const _CharacterAppearance := preload("res://scripts/character/character_appearance.gd")

## 헤어스타일 매핑 (인덱스 → 이름) - 레거시
const HAIR_STYLES = ["braids", "curly", "medium", "spiky", "afro", "buzz", "mohawk", "wavy"]

## 헤어색 목록 - 레거시
const HAIR_COLORS = ["brown", "black", "blonde", "ginger", "gray"]

## 피부톤 목록
const SKIN_TONES = ["medium", "light", "olive", "brown", "dark"]

## 유니폼 색상 목록
const UNIFORM_COLORS = ["red", "orange", "yellow", "green", "cyan", "blue", "purple", "pink", "white", "black", "gray"]

## === Socceralia 스프라이트용 상수 ===
## 헤어 폴더 (Socceralia 스프라이트)
const HAIR_FOLDERS = ["black", "blonde", "redhead", "other", "gk"]

## 패턴 타입
const PATTERN_TYPES = {0: "solid", 1: "hoops", 2: "stripes", 3: "checker", 4: "diagonal"}  # 단색  # 가로줄  # 세로줄  # 체크  # 대각선


## Dictionary → CharacterAppearance 변환
static func from_dict(data: Dictionary) -> Resource:  # CharacterAppearance
	var appearance = _CharacterAppearance.new()

	if data.has("parts_appearance"):
		var parts = data.parts_appearance
		appearance.hair_style = parts.get("hair_style", "medium")
		appearance.hair_color = parts.get("hair_color", "brown")
		appearance.skin_tone = parts.get("skin_tone", "medium")
		appearance.torso_color = parts.get("torso_color", "red")
		appearance.sleeve_color = parts.get("sleeve_color", "red")
	elif data.has("hair_style_index"):
		# 레거시 형식 지원
		var hair_index = data.get("hair_style_index", 2)
		appearance.hair_style = HAIR_STYLES[hair_index % HAIR_STYLES.size()]
		appearance.hair_color = "brown"
		appearance.skin_tone = "medium"
		appearance.torso_color = "red"
		appearance.sleeve_color = "red"

	return appearance


## CharacterAppearance → Dictionary 변환
static func to_dict(appearance: Resource) -> Dictionary:  # CharacterAppearance
	return {
		"parts_appearance":
		{
			"hair_style": appearance.hair_style,
			"hair_color": appearance.hair_color,
			"skin_tone": appearance.skin_tone,
			"torso_color": appearance.torso_color,
			"sleeve_color": appearance.sleeve_color
		},
		# 레거시 호환성
		"hair_style_index": HAIR_STYLES.find(appearance.hair_style),
		"body_type": 1,
		"face_preset": 0
	}


## 랜덤 외형 생성
static func create_random() -> Dictionary:
	return {
		"parts_appearance":
		{
			"hair_style": HAIR_STYLES[randi() % HAIR_STYLES.size()],
			"hair_color": HAIR_COLORS[randi() % HAIR_COLORS.size()],
			"skin_tone": SKIN_TONES[randi() % SKIN_TONES.size()],
			"torso_color": UNIFORM_COLORS[randi() % UNIFORM_COLORS.size()],
			"sleeve_color": UNIFORM_COLORS[randi() % UNIFORM_COLORS.size()]
		}
	}


## 팀 유니폼 적용
static func apply_team_uniform(appearance_dict: Dictionary, primary: String, secondary: String) -> Dictionary:
	var result = appearance_dict.duplicate(true)
	if not result.has("parts_appearance"):
		result["parts_appearance"] = {}
	result.parts_appearance["torso_color"] = primary
	result.parts_appearance["sleeve_color"] = secondary
	return result


## 마이팀 유니폼으로 랜덤 외형 생성
static func create_random_with_team_uniform() -> Dictionary:
	var base = create_random()

	# MyTeamData에서 팀 유니폼 가져오기
	if Engine.has_singleton("MyTeamData"):
		var my_team = Engine.get_singleton("MyTeamData")

		if my_team and my_team.has_method("get_team_uniform"):
			var uniform = my_team.get_team_uniform(true)  # 홈 유니폼
			base.parts_appearance["torso_color"] = uniform.get("primary", "red")
			base.parts_appearance["sleeve_color"] = uniform.get("secondary", "white")

	return base


## 마이팀 유니폼 색상 직접 지정하여 랜덤 생성
static func create_random_with_uniform(primary: String, secondary: String) -> Dictionary:
	var base = create_random()
	base.parts_appearance["torso_color"] = primary
	base.parts_appearance["sleeve_color"] = secondary
	return base


## 기존 세이브 데이터 마이그레이션
static func migrate_legacy(old_data: Dictionary) -> Dictionary:
	if old_data.has("parts_appearance"):
		return old_data  # 이미 신규 형식

	# 레거시 형식 변환
	var hair_index = old_data.get("hair_style_index", 2)
	var hair_style = HAIR_STYLES[hair_index % HAIR_STYLES.size()]

	return {
		"parts_appearance":
		{
			"hair_style": hair_style,
			"hair_color": "brown",
			"skin_tone": "medium",
			"torso_color": "red",
			"sleeve_color": "red"
		},
		"hair_style_index": hair_index,
		"body_type": old_data.get("body_type", 1),
		"face_preset": old_data.get("face_preset", 0)
	}


## === Socceralia 스프라이트 스키마 지원 ===


## Socceralia 스키마인지 확인
static func is_socceralia_schema(data: Dictionary) -> bool:
	return data.get("sprite_type", "") == "socceralia" or data.has("hair_folder")


## Socceralia 스키마 → 레거시 스키마 변환 (호환성용)
static func socceralia_to_legacy(data: Dictionary) -> Dictionary:
	if not is_socceralia_schema(data):
		return data

	var hair_folder = data.get("hair_folder", "black")
	var uniform = data.get("uniform", {})

	# hair_folder를 가장 가까운 hair_style로 매핑
	var hair_style = "medium"
	var hair_color = "brown"
	match hair_folder:
		"black":
			hair_style = "buzz"
			hair_color = "black"
		"blonde":
			hair_style = "medium"
			hair_color = "blonde"
		"redhead":
			hair_style = "wavy"
			hair_color = "ginger"
		"other":
			hair_style = "curly"
			hair_color = "brown"
		"gk":
			hair_style = "buzz"
			hair_color = "black"

	# uniform 컬러를 color_id로 변환
	var primary_hex = uniform.get("primary_color", "#FF0000")
	var secondary_hex = uniform.get("secondary_color", "#FFFFFF")
	var torso = _hex_to_color_id(primary_hex)
	var sleeve = _hex_to_color_id(secondary_hex)

	return {
		"parts_appearance":
		{
			"hair_style": hair_style,
			"hair_color": hair_color,
			"skin_tone": data.get("skin_tone", "medium"),
			"torso_color": torso,
			"sleeve_color": sleeve
		},
		"hair_style_index": HAIR_STYLES.find(hair_style),
		"body_type": 1,
		"face_preset": 0,
		# Socceralia 원본 데이터도 보존
		"_socceralia": data.duplicate(true)
	}


## 레거시 스키마 → Socceralia 스키마 변환
static func legacy_to_socceralia(data: Dictionary) -> Dictionary:
	# 이미 Socceralia면 그대로 반환
	if is_socceralia_schema(data):
		return data

	var parts = data.get("parts_appearance", {})
	var hair_color = parts.get("hair_color", "brown")
	var torso_color = parts.get("torso_color", "red")
	var sleeve_color = parts.get("sleeve_color", "white")

	# hair_color를 hair_folder로 매핑
	var hair_folder = "other"
	match hair_color:
		"black":
			hair_folder = "black"
		"blonde":
			hair_folder = "blonde"
		"ginger":
			hair_folder = "redhead"
		_:
			hair_folder = "other"

	return {
		"sprite_type": "socceralia",
		"hair_folder": hair_folder,
		"skin_tone": parts.get("skin_tone", "medium"),
		"uniform":
		{
			"primary_color": _color_id_to_hex(torso_color),
			"secondary_color": _color_id_to_hex(sleeve_color),
			"pattern_type": 0
		}
	}


## 랜덤 Socceralia 외형 생성
static func create_random_socceralia() -> Dictionary:
	var hair_folders_no_gk = ["black", "blonde", "redhead", "other"]
	var primary = UNIFORM_COLORS[randi() % UNIFORM_COLORS.size()]
	var secondary = UNIFORM_COLORS[randi() % UNIFORM_COLORS.size()]

	return {
		"sprite_type": "socceralia",
		"hair_folder": hair_folders_no_gk[randi() % hair_folders_no_gk.size()],
		"skin_tone": SKIN_TONES[randi() % SKIN_TONES.size()],
		"uniform":
		{
			"primary_color": _color_id_to_hex(primary),
			"secondary_color": _color_id_to_hex(secondary),
			"pattern_type": randi() % 4
		}
	}


## 팀 유니폼으로 Socceralia 외형 생성
static func create_random_socceralia_with_uniform(
	primary_hex: String, secondary_hex: String, pattern: int = 0
) -> Dictionary:
	var hair_folders_no_gk = ["black", "blonde", "redhead", "other"]

	return {
		"sprite_type": "socceralia",
		"hair_folder": hair_folders_no_gk[randi() % hair_folders_no_gk.size()],
		"skin_tone": SKIN_TONES[randi() % SKIN_TONES.size()],
		"uniform": {"primary_color": primary_hex, "secondary_color": secondary_hex, "pattern_type": pattern}
	}


## 색상 ID → HEX 변환
static func _color_id_to_hex(color_id: String) -> String:
	var color_map = {
		"red": "#E63333",
		"orange": "#FF8000",
		"yellow": "#FFE633",
		"green": "#33B34D",
		"cyan": "#00CCCC",
		"blue": "#3366E6",
		"purple": "#994DCC",
		"pink": "#FF80B3",
		"white": "#F2F2F2",
		"black": "#262626",
		"gray": "#808080"
	}
	return color_map.get(color_id, "#FF0000")


## HEX → 색상 ID 변환 (가장 가까운 색상)
static func _hex_to_color_id(hex_color: String) -> String:
	var target = Color(hex_color)
	var best_match = "red"
	var best_dist = 999.0

	var color_map = {
		"red": Color("#E63333"),
		"orange": Color("#FF8000"),
		"yellow": Color("#FFE633"),
		"green": Color("#33B34D"),
		"cyan": Color("#00CCCC"),
		"blue": Color("#3366E6"),
		"purple": Color("#994DCC"),
		"pink": Color("#FF80B3"),
		"white": Color("#F2F2F2"),
		"black": Color("#262626"),
		"gray": Color("#808080")
	}

	for color_id in color_map:
		var c = color_map[color_id]
		var dist = sqrt(pow(target.r - c.r, 2) + pow(target.g - c.g, 2) + pow(target.b - c.b, 2))
		if dist < best_dist:
			best_dist = dist
			best_match = color_id

	return best_match
