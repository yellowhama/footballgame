extends RefCounted
class_name TeamColorManager
##
## TeamColorManager - 팀 컬러 및 선수 외모 관리
##
## 기능:
##   - 팀별 유니폼 색상 프리셋
##   - 선수별 헤어 스타일 배정
##   - ColorSwap 셰이더 적용
##
## 스펙: docs/spec+@/spec_v4/dev_spec/UI/2025-12-07_SOCCERALIA_HORIZONTAL_VIEW_SPEC.md
##

#region Pattern Types
enum PatternType {
	SOLID = 0,  ## 단색
	HOOPS = 1,  ## 가로줄 (Celtic, QPR)
	STRIPES = 2,  ## 세로줄 (AC Milan, Juventus)
	CHECKER = 3,  ## 체크 (Croatia)
	DIAGONAL = 4,  ## 대각선
}
#endregion

#region Team Color Presets (패턴 포함)
const TEAM_COLORS := {
	## 아시아
	"korea": {"primary": Color(0.9, 0.1, 0.1), "secondary": Color.WHITE, "pattern": PatternType.SOLID},
	"japan": {"primary": Color(0.1, 0.2, 0.8), "secondary": Color.WHITE, "pattern": PatternType.SOLID},
	"china": {"primary": Color(0.9, 0.1, 0.1), "secondary": Color(1.0, 0.85, 0.0), "pattern": PatternType.SOLID},
	"australia": {"primary": Color(1.0, 0.85, 0.0), "secondary": Color(0.0, 0.4, 0.2), "pattern": PatternType.SOLID},
	## 유럽 - 클럽
	"ac_milan": {"primary": Color(0.8, 0.1, 0.1), "secondary": Color.BLACK, "pattern": PatternType.STRIPES},
	"inter_milan": {"primary": Color(0.0, 0.2, 0.5), "secondary": Color.BLACK, "pattern": PatternType.STRIPES},
	"juventus": {"primary": Color.WHITE, "secondary": Color.BLACK, "pattern": PatternType.STRIPES},
	"barcelona": {"primary": Color(0.6, 0.0, 0.2), "secondary": Color(0.0, 0.3, 0.6), "pattern": PatternType.STRIPES},
	"celtic": {"primary": Color(0.0, 0.5, 0.2), "secondary": Color.WHITE, "pattern": PatternType.HOOPS},
	"qpr": {"primary": Color(0.0, 0.3, 0.6), "secondary": Color.WHITE, "pattern": PatternType.HOOPS},
	## 유럽 - 국가대표
	"germany": {"primary": Color.WHITE, "secondary": Color.BLACK, "pattern": PatternType.SOLID},
	"france": {"primary": Color(0.1, 0.2, 0.7), "secondary": Color.WHITE, "pattern": PatternType.SOLID},
	"italy": {"primary": Color(0.1, 0.3, 0.8), "secondary": Color.WHITE, "pattern": PatternType.SOLID},
	"spain": {"primary": Color(0.9, 0.1, 0.1), "secondary": Color(1.0, 0.85, 0.0), "pattern": PatternType.SOLID},
	"england": {"primary": Color.WHITE, "secondary": Color(0.1, 0.2, 0.5), "pattern": PatternType.SOLID},
	"netherlands": {"primary": Color(1.0, 0.5, 0.0), "secondary": Color.BLACK, "pattern": PatternType.SOLID},
	"portugal": {"primary": Color(0.6, 0.0, 0.0), "secondary": Color(0.0, 0.5, 0.2), "pattern": PatternType.SOLID},
	"belgium": {"primary": Color(0.9, 0.1, 0.1), "secondary": Color(1.0, 0.85, 0.0), "pattern": PatternType.SOLID},
	"croatia": {"primary": Color(0.9, 0.1, 0.1), "secondary": Color.WHITE, "pattern": PatternType.CHECKER},
	## 남미
	"brazil": {"primary": Color(1.0, 0.85, 0.0), "secondary": Color(0.0, 0.5, 0.2), "pattern": PatternType.SOLID},
	"argentina": {"primary": Color(0.5, 0.7, 1.0), "secondary": Color.WHITE, "pattern": PatternType.STRIPES},
	"colombia": {"primary": Color(1.0, 0.85, 0.0), "secondary": Color(0.1, 0.2, 0.5), "pattern": PatternType.SOLID},
	"uruguay": {"primary": Color(0.5, 0.7, 1.0), "secondary": Color.BLACK, "pattern": PatternType.SOLID},
	## 아프리카
	"nigeria": {"primary": Color(0.0, 0.5, 0.2), "secondary": Color.WHITE, "pattern": PatternType.SOLID},
	"senegal": {"primary": Color.WHITE, "secondary": Color(0.0, 0.5, 0.2), "pattern": PatternType.SOLID},
	"morocco": {"primary": Color(0.6, 0.0, 0.0), "secondary": Color(0.0, 0.5, 0.2), "pattern": PatternType.SOLID},
	## 기본값
	"home": {"primary": Color(0.85, 0.2, 0.2), "secondary": Color.WHITE, "pattern": PatternType.SOLID},
	"away": {"primary": Color(0.2, 0.4, 0.85), "secondary": Color.WHITE, "pattern": PatternType.SOLID},
}

## 원본 스프라이트의 키 색상 (흰색 유니폼 기준)
const SOURCE_KIT_COLOR := Color.WHITE
const SOURCE_HAIR_COLOR := Color(0.5, 0.5, 0.5)  ## 회색 기본 머리색
#endregion

#region Hair Style Distribution
## 포지션별 헤어 스타일 배분
const HAIR_STYLES := ["black", "blonde", "redhead", "other"]

const POSITION_HAIR_PREFERENCE := {
	"GK": "gk",  ## 골키퍼는 전용 스타일
	"CB": "black",
	"LB": "blonde",
	"RB": "black",
	"DM": "black",
	"CM": "blonde",
	"LM": "redhead",
	"RM": "redhead",
	"AM": "blonde",
	"LW": "redhead",
	"RW": "redhead",
	"ST": "black",
	"CF": "blonde",
}
#endregion


## 팀 색상 가져오기
static func get_team_colors(team_id: String) -> Dictionary:
	return TEAM_COLORS.get(team_id.to_lower(), TEAM_COLORS["home"])


## 선수에게 팀 컬러 + 패턴 셰이더 적용
static func apply_team_color_to_player(player: Node2D, team_id: String) -> void:
	var team_data: Dictionary = get_team_colors(team_id)

	# Potagon 지원: SoccerPlayer의 potagon에 색상 직접 적용
	if player.has_method("setup_from_roster"):
		# SoccerPlayer with Potagon
		var roster_data = {
			"kit_primary":
			[int(team_data["primary"].r * 255), int(team_data["primary"].g * 255), int(team_data["primary"].b * 255)],
			"kit_secondary":
			[
				int(team_data["secondary"].r * 255),
				int(team_data["secondary"].g * 255),
				int(team_data["secondary"].b * 255)
			]
		}
		player.setup_from_roster(roster_data)
	elif player.has_method("apply_team_color"):
		# Legacy player (레거시 지원)
		var pattern: int = team_data.get("pattern", PatternType.SOLID)
		if pattern == PatternType.SOLID:
			player.apply_team_color(team_data["primary"], team_data["secondary"], SOURCE_KIT_COLOR)
		else:
			player.apply_kit_pattern(team_data["primary"], team_data["secondary"], pattern, SOURCE_KIT_COLOR)


## 선수 ID 기반 헤어 스타일 결정 (일관성 있는 랜덤)
static func get_hair_style_for_player(player_id: String, position: String = "") -> String:
	## 골키퍼는 항상 GK 스타일
	if position == "GK":
		return "gk"

	## 포지션 기반 선호 스타일 확인
	if position in POSITION_HAIR_PREFERENCE:
		return POSITION_HAIR_PREFERENCE[position]

	## 선수 ID 해시 기반 일관된 스타일
	var hash_value: int = player_id.hash()
	var style_index: int = abs(hash_value) % HAIR_STYLES.size()
	return HAIR_STYLES[style_index]


## 팀 전체 선수에게 색상 및 스타일 적용
static func setup_team_players(players: Array, team_id: String, roster: Array) -> void:
	for i in range(min(players.size(), roster.size())):
		var player: Node2D = players[i]
		var roster_data: Dictionary = roster[i] if roster[i] is Dictionary else {}

		var player_id: String = str(roster_data.get("id", i))
		var position: String = str(roster_data.get("position", ""))
		var jersey_number: int = roster_data.get("jersey_number", i + 1)

		## 헤어 스타일 설정 (레거시만 지원)
		if player.has_method("set_hair_style"):
			player.set_hair_style(get_hair_style_for_player(player_id, position))

		## 팀 컬러 적용
		apply_team_color_to_player(player, team_id)

		## 배번 설정
		player.set_jersey_number(jersey_number)

		## 메타데이터 저장
		player.player_id = player_id


## 홈/어웨이 기본 색상으로 간단히 적용
static func setup_home_away_teams(home_players: Array, away_players: Array) -> void:
	for player in home_players:
		apply_team_color_to_player(player, "home")

	for player in away_players:
		apply_team_color_to_player(player, "away")


## 커스텀 팀 컬러로 선수에게 적용 (MyTeamData 연동용)
## uniform 형식: { "primary": "#FF0000", "secondary": "#FFFFFF", "pattern_type": 0 }
static func apply_custom_team_color(player: Node2D, uniform: Dictionary) -> void:
	var primary_hex: String = uniform.get("primary", "#FF0000")
	var secondary_hex: String = uniform.get("secondary", "#FFFFFF")
	var pattern: int = uniform.get("pattern_type", 0)

	var primary: Color = Color(primary_hex)
	var secondary: Color = Color(secondary_hex)

	# Potagon 지원
	if player.has_method("setup_from_roster"):
		var roster_data = {
			"kit_primary": [int(primary.r * 255), int(primary.g * 255), int(primary.b * 255)],
			"kit_secondary": [int(secondary.r * 255), int(secondary.g * 255), int(secondary.b * 255)]
		}
		player.setup_from_roster(roster_data)
	elif player.has_method("apply_team_color"):
		# Legacy
		if pattern == PatternType.SOLID:
			player.apply_team_color(primary, secondary, SOURCE_KIT_COLOR)
		else:
			player.apply_kit_pattern(primary, secondary, pattern, SOURCE_KIT_COLOR)


## Socceralia 외형 데이터로 팀 전체 설정
## roster 형식: [{ "id": "p1", "position": "ST", "jersey_number": 9, "appearance": { "hair_folder": "black", ... } }, ...]
## team_uniform 형식: { "primary": "#FF0000", "secondary": "#FFFFFF", "pattern_type": 0 }
static func setup_team_with_appearance(players: Array, roster: Array, team_uniform: Dictionary) -> void:
	for i in range(min(players.size(), roster.size())):
		var player: Node2D = players[i]
		var roster_data: Dictionary = roster[i] if roster[i] is Dictionary else {}

		var player_id: String = str(roster_data.get("id", i))
		var position: String = str(roster_data.get("position", ""))
		var jersey_number: int = roster_data.get("jersey_number", i + 1)
		var appearance: Dictionary = roster_data.get("appearance", {})

		## 외형 데이터가 있으면 적용
		if appearance.has("kit_primary"):
			## Rust PlayerAppearanceData 형식 (kit_primary: [r,g,b])
			if player.has_method("setup_from_roster"):
				player.setup_from_roster(appearance)
			else:
				## 수동 적용 (레거시)
				var hair_color: String = appearance.get("hair_color", "black")
				if player.has_method("set_hair_style"):
					player.set_hair_style(_hair_color_to_folder(hair_color))
				_apply_rust_appearance_to_player(player, appearance)
		elif appearance.has("hair_folder"):
			## Socceralia 스키마
			if player.has_method("set_hair_style"):
				player.set_hair_style(appearance.get("hair_folder", "black"))
			apply_custom_team_color(player, team_uniform)
		elif appearance.has("parts_appearance"):
			## 레거시 스키마 - hair_color를 hair_folder로 변환
			var parts: Dictionary = appearance.get("parts_appearance", {})
			var hair_color: String = parts.get("hair_color", "brown")
			var hair_folder: String = _hair_color_to_folder(hair_color)
			if player.has_method("set_hair_style"):
				player.set_hair_style(hair_folder)
			apply_custom_team_color(player, team_uniform)
		else:
			## 외형 데이터 없으면 포지션 기반 결정
			if player.has_method("set_hair_style"):
				player.set_hair_style(get_hair_style_for_player(player_id, position))
			apply_custom_team_color(player, team_uniform)

		## 배번 설정
		player.set_jersey_number(jersey_number)

		## 역할/포지션 설정 (GK 장갑 등) - 2025-12-12
		if player.has_method("set_role"):
			player.set_role(position)

		## 메타데이터 저장
		player.player_id = player_id


## Rust PlayerAppearanceData 형식을 선수에게 직접 적용
## appearance 형식: { "hair_color": "black", "kit_primary": [255,0,0], "kit_secondary": [255,255,255], "kit_pattern": 0 }
static func _apply_rust_appearance_to_player(player: Node2D, appearance: Dictionary) -> void:
	var kit_primary: Array = appearance.get("kit_primary", [255, 0, 0])
	var kit_secondary: Array = appearance.get("kit_secondary", [255, 255, 255])
	var kit_pattern: int = appearance.get("kit_pattern", 0)

	var primary: Color = Color(kit_primary[0] / 255.0, kit_primary[1] / 255.0, kit_primary[2] / 255.0)
	var secondary: Color = Color(kit_secondary[0] / 255.0, kit_secondary[1] / 255.0, kit_secondary[2] / 255.0)

	# Potagon 지원
	if player.has_method("setup_from_roster"):
		var roster_data = {"kit_primary": kit_primary, "kit_secondary": kit_secondary}
		player.setup_from_roster(roster_data)
	elif player.has_method("apply_team_color"):
		# Legacy
		if kit_pattern == PatternType.SOLID:
			player.apply_team_color(primary, secondary, SOURCE_KIT_COLOR)
		else:
			player.apply_kit_pattern(primary, secondary, kit_pattern, SOURCE_KIT_COLOR)


## hair_color → hair_folder 변환 헬퍼
static func _hair_color_to_folder(hair_color: String) -> String:
	match hair_color:
		"black":
			return "black"
		"blonde":
			return "blonde"
		"ginger":
			return "redhead"
		_:
			return "other"
