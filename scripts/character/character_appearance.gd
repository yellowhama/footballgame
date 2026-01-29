# character_appearance.gd
# 캐릭터 외형 데이터를 저장하는 리소스
class_name CharacterAppearance
extends Resource

## 헤어스타일 ID
@export_enum("braids", "curly", "medium", "spiky", "afro", "buzz", "mohawk", "wavy") var hair_style: String = "medium"

## 피부색 프리셋: "medium"(기본), "light", "olive", "brown", "dark"
@export_enum("medium", "light", "olive", "brown", "dark") var skin_tone: String = "medium"

## 헤어색 프리셋: "brown"(기본), "black", "blonde", "ginger", "gray"
@export_enum("brown", "black", "blonde", "ginger", "gray") var hair_color: String = "brown"

## 유니폼 몸통색: "red"(기본), "orange", "yellow", "green", "cyan", "blue", "purple", "pink", "white", "black", "gray"
@export_enum("red", "orange", "yellow", "green", "cyan", "blue", "purple", "pink", "white", "black", "gray")
var torso_color: String = "red"

## 유니폼 슬리브색
@export_enum("red", "orange", "yellow", "green", "cyan", "blue", "purple", "pink", "white", "black", "gray")
var sleeve_color: String = "red"

## 현재 방향 (0-7, 45도 단위)
## 0: front, 1: quarter_front, 2: side, 3: quarter_back, 4: back
## 5-7: flip 처리 (5: quarter_back_r, 6: side_r, 7: quarter_front_r)
var facing_direction: int = 0

## 8가지 헤어스타일 (5방향 완전 + 3방향 부분)
const HAIR_STYLES_FULL = ["braids", "curly", "medium", "spiky"]  # 5방향 완전 지원
const HAIR_STYLES_PARTIAL = ["afro", "buzz", "mohawk", "wavy"]  # 3방향만 지원

## 5가지 방향
const DIRECTIONS = ["front", "quarter_front", "side", "quarter_back", "back"]

## 방향별 접미사 (8방향)
const DIRECTION_MAP = {
	0: {"dir": "front", "flip": false},
	1: {"dir": "quarter_front", "flip": false},
	2: {"dir": "side", "flip": false},
	3: {"dir": "quarter_back", "flip": false},
	4: {"dir": "back", "flip": false},
	5: {"dir": "quarter_back", "flip": true},  # 225° - flip quarter_back
	6: {"dir": "side", "flip": true},  # 270° - flip side
	7: {"dir": "quarter_front", "flip": true}  # 315° - flip quarter_front
}


func get_direction_info() -> Dictionary:
	"""현재 facing_direction에 대한 방향 정보 반환"""
	return DIRECTION_MAP[facing_direction % 8]


func is_hair_style_full_direction() -> bool:
	"""현재 헤어스타일이 5방향 완전 지원인지 확인"""
	return hair_style in HAIR_STYLES_FULL


func randomize_appearance() -> void:
	"""랜덤 외형 생성"""
	var all_hair_styles = HAIR_STYLES_FULL + HAIR_STYLES_PARTIAL
	hair_style = all_hair_styles[randi() % all_hair_styles.size()]

	var skin_tones = ["medium", "light", "olive", "brown", "dark"]
	skin_tone = skin_tones[randi() % skin_tones.size()]

	var hair_colors = ["brown", "black", "blonde", "ginger", "gray"]
	hair_color = hair_colors[randi() % hair_colors.size()]

	var uniform_colors = [
		"red", "orange", "yellow", "green", "cyan", "blue", "purple", "pink", "white", "black", "gray"
	]
	torso_color = uniform_colors[randi() % uniform_colors.size()]
	sleeve_color = uniform_colors[randi() % uniform_colors.size()]


func duplicate_appearance() -> Resource:  # CharacterAppearance
	"""외형 복사"""
	var copy: Resource = (load("res://scripts/character/character_appearance.gd") as GDScript).new()
	copy.hair_style = hair_style
	copy.skin_tone = skin_tone
	copy.hair_color = hair_color
	copy.torso_color = torso_color
	copy.sleeve_color = sleeve_color
	copy.facing_direction = facing_direction
	return copy
