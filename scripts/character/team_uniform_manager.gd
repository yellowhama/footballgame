# team_uniform_manager.gd
# 팀 유니폼 색상 관리 유틸리티
class_name TeamUniformManager
extends RefCounted

const _PlayerAppearanceBridge := preload("res://scripts/character/player_appearance_bridge.gd")

## 기본 팀 유니폼 프리셋
const TEAM_PRESETS = {
	# 실제 팀 프리셋 (K리그 등)
	"fc_seoul": {"home": {"primary": "red", "secondary": "black"}, "away": {"primary": "white", "secondary": "red"}},
	"suwon_bluewings":
	{"home": {"primary": "blue", "secondary": "white"}, "away": {"primary": "white", "secondary": "blue"}},
	"jeonbuk": {"home": {"primary": "green", "secondary": "white"}, "away": {"primary": "white", "secondary": "green"}},
	"ulsan": {"home": {"primary": "cyan", "secondary": "orange"}, "away": {"primary": "orange", "secondary": "cyan"}},
	"pohang": {"home": {"primary": "red", "secondary": "black"}, "away": {"primary": "black", "secondary": "red"}},
	"daegu": {"home": {"primary": "blue", "secondary": "white"}, "away": {"primary": "white", "secondary": "blue"}},
	"gangwon":
	{"home": {"primary": "orange", "secondary": "black"}, "away": {"primary": "white", "secondary": "orange"}},
	"incheon": {"home": {"primary": "blue", "secondary": "black"}, "away": {"primary": "white", "secondary": "blue"}},
	# 유소년/학교 팀 프리셋
	"school_red": {"home": {"primary": "red", "secondary": "white"}, "away": {"primary": "white", "secondary": "red"}},
	"school_blue":
	{"home": {"primary": "blue", "secondary": "white"}, "away": {"primary": "white", "secondary": "blue"}},
	"school_green":
	{"home": {"primary": "green", "secondary": "white"}, "away": {"primary": "white", "secondary": "green"}},
	"school_yellow":
	{"home": {"primary": "yellow", "secondary": "black"}, "away": {"primary": "black", "secondary": "yellow"}},
	"school_black":
	{"home": {"primary": "black", "secondary": "white"}, "away": {"primary": "white", "secondary": "black"}},
	# 기본
	"default": {"home": {"primary": "red", "secondary": "red"}, "away": {"primary": "blue", "secondary": "blue"}}
}

## 색상 유사도 체크 (충돌 방지용)
const COLOR_SIMILARITY = {
	"red": ["red", "orange", "pink"],
	"orange": ["orange", "red", "yellow"],
	"yellow": ["yellow", "orange", "green"],
	"green": ["green", "yellow", "cyan"],
	"cyan": ["cyan", "green", "blue"],
	"blue": ["blue", "cyan", "purple"],
	"purple": ["purple", "blue", "pink"],
	"pink": ["pink", "purple", "red"],
	"white": ["white", "gray"],
	"black": ["black", "gray"],
	"gray": ["gray", "black", "white"]
}


## 팀 유니폼 가져오기
static func get_team_uniform(team_id: String, is_home: bool = true) -> Dictionary:
	# 마이팀(플레이어 팀) 체크 - MyTeamData에서 커스텀 유니폼 가져오기
	if _is_my_team(team_id):
		var my_uniform = _get_my_team_uniform(is_home)
		if not my_uniform.is_empty():
			return my_uniform

	var preset = TEAM_PRESETS.get(team_id, TEAM_PRESETS["default"])
	return preset["home"] if is_home else preset["away"]


## 마이팀 여부 확인
static func _is_my_team(team_id: String) -> bool:
	return team_id in ["my_team", "player_team", "myteam", "user_team"]


## MyTeamData에서 유니폼 가져오기
static func _get_my_team_uniform(is_home: bool) -> Dictionary:
	var tree = Engine.get_main_loop()
	if tree and tree.root and tree.root.has_node("MyTeamData"):
		var my_team = tree.root.get_node("MyTeamData")
		if my_team and my_team.has_method("get_team_uniform"):
			return my_team.get_team_uniform(is_home)
	return {}


## 두 팀 간 유니폼 충돌 체크
static func check_uniform_conflict(home_uniform: Dictionary, away_uniform: Dictionary) -> bool:
	var home_primary = home_uniform.get("primary", "red")
	var away_primary = away_uniform.get("primary", "blue")

	# 동일 색상
	if home_primary == away_primary:
		return true

	# 유사 색상
	var similar_colors = COLOR_SIMILARITY.get(home_primary, [home_primary])
	if away_primary in similar_colors:
		return true

	return false


## 경기 유니폼 결정 (충돌 방지)
static func determine_match_uniforms(home_team_id: String, away_team_id: String) -> Dictionary:
	var home_uniform = get_team_uniform(home_team_id, true)
	var away_uniform = get_team_uniform(away_team_id, false)

	# 충돌 체크
	if check_uniform_conflict(home_uniform, away_uniform):
		# 원정팀 홈 유니폼으로 시도
		var away_home = get_team_uniform(away_team_id, true)
		if not check_uniform_conflict(home_uniform, away_home):
			away_uniform = away_home
		else:
			# 여전히 충돌하면 기본 대비 색상 사용
			away_uniform = _get_contrast_uniform(home_uniform)

	return {"home": home_uniform, "away": away_uniform}


## 대비 색상 유니폼 생성
static func _get_contrast_uniform(base_uniform: Dictionary) -> Dictionary:
	var base_primary = base_uniform.get("primary", "red")

	# 대비 색상 매핑
	var contrast_map = {
		"red": "blue",
		"orange": "blue",
		"yellow": "purple",
		"green": "purple",
		"cyan": "red",
		"blue": "orange",
		"purple": "yellow",
		"pink": "green",
		"white": "black",
		"black": "white",
		"gray": "blue"
	}

	var contrast_primary = contrast_map.get(base_primary, "white")
	return {"primary": contrast_primary, "secondary": contrast_primary}


## 랜덤 팀 유니폼 생성
static func create_random_uniform() -> Dictionary:
	var colors: Array = _PlayerAppearanceBridge.UNIFORM_COLORS
	var primary = colors[randi() % colors.size()]
	var secondary = colors[randi() % colors.size()]
	return {"home": {"primary": primary, "secondary": secondary}, "away": {"primary": secondary, "secondary": primary}}


## 커스텀 유니폼 생성
static func create_custom_uniform(
	home_primary: String, home_secondary: String, away_primary: String, away_secondary: String
) -> Dictionary:
	return {
		"home": {"primary": home_primary, "secondary": home_secondary},
		"away": {"primary": away_primary, "secondary": away_secondary}
	}
