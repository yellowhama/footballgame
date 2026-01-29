# MyTeam 시스템 관리 싱글톤
extends Node

# ===== 팀 기본 정보 =====
var team_name: String = "My FC"
var team_created_at: String = ""
var team_emblem: Dictionary = {"shape": "shield", "color_primary": "#FF0000", "color_secondary": "#FFFFFF"}

# ===== 메인 캐릭터 (플레이어가 생성한 캐릭터) =====
var main_character: Dictionary = {}

# ===== 로스터 관리 =====
var first_team: Array = []  # 1군 (최대 25명)
var reserves: Array = []  # 리저브 (최대 100명)

const MAX_FIRST_TEAM = 25
const MAX_RESERVES = 100

# ===== 덱 시스템 (7칸) =====
var current_deck: Dictionary = {"manager": null, "coaches": [null, null, null], "tactics": [null, null, null]}  # 감독 카드 1장  # 코치 카드 3장  # 전술 카드 3장

# ===== 통계 =====
var total_matches_played: int = 0
var total_wins: int = 0
var total_draws: int = 0
var total_losses: int = 0

# ===== 시그널 =====
signal player_graduated(player_data)
signal roster_updated
signal deck_changed


func _ready():
	print("===== MyTeamManager 초기화 =====")
	team_created_at = Time.get_datetime_string_from_system()

	# Wait for all autoloads to be ready (including StageManager)
	await get_tree().process_frame

	# Try to load player's selected team from StageManager
	var loaded_from_stage = _try_load_from_stage_manager()

	if not loaded_from_stage:
		# Fallback: create default team
		_create_default_team()

	print("✅ MyTeam 시스템 준비 완료!")
	print("   팀명: %s" % team_name)
	print("   1군: %d명" % first_team.size())
	print("   리저브: %d명" % reserves.size())


func _try_load_from_stage_manager() -> bool:
	"""Try to load player's selected team from StageManager"""
	if not has_node("/root/StageManager"):
		print("[MyTeamManager] StageManager not found, using default team")
		return false

	var stage_mgr = get_node("/root/StageManager")
	var player_team_id = stage_mgr.player_team_id

	if player_team_id <= 0:
		print("[MyTeamManager] No player team selected, using default team")
		return false

	# Load player team from StageManager
	if not stage_mgr.current_player_team or stage_mgr.current_player_team.is_empty():
		print("[MyTeamManager] Player team data empty, using default team")
		return false

	var player_team = stage_mgr.current_player_team
	var squad = player_team.get("squad", [])

	if squad.is_empty():
		print("[MyTeamManager] Player team has no squad, using default team")
		return false

	# Load squad into first_team
	first_team.clear()
	team_name = str(player_team.get("club_name", "My FC"))

	for player_data in squad:
		var player = {
			"id": "stage_player_" + str(player_data.get("name", "Unknown")),
			"name": str(player_data.get("name", "Unknown Player")),
			"ca": int(player_data.get("ca", 50)),
			"pa": int(player_data.get("pa", 80)),
			"position": str(player_data.get("position", "CM")),
			"age_months": 192,  # Default 16 years
			"is_default": false
		}
		first_team.append(player)

	print("[MyTeamManager] ✅ Loaded team from StageManager: %s (%d players)" % [team_name, first_team.size()])
	return true


func _create_default_team():
	"""기본 팀 생성 (시작 시 제공되는 선수들)"""
	# 기본 선발 11명 + 벤치 7명 = 18명 생성
	var starting_positions = ["GK", "LB", "CB", "CB", "RB", "DM", "CM", "CM", "RW", "LW", "ST"]
	var bench_positions = ["GK", "CB", "FB", "DM", "CM", "AM", "ST"]

	# 한국 선수 이름 풀
	var korean_surnames = ["김", "이", "박", "최", "정", "강", "조", "윤", "장", "임", "한", "오", "서", "신", "권"]
	var korean_given_names = [
		"민준", "서준", "예준", "도윤", "시우", "주원", "하준", "지호", "지우", "준서", "건우", "우진", "현우", "선우", "연우", "유준", "정우", "승우"
	]

	var all_positions = starting_positions + bench_positions
	var rng = RandomNumberGenerator.new()
	rng.randomize()

	for i in range(18):
		var surname = korean_surnames[rng.randi() % korean_surnames.size()]
		var given_name = korean_given_names[rng.randi() % korean_given_names.size()]
		var full_name = surname + given_name

		var player = {
			"id": "default_player_" + str(i),
			"name": full_name,
			"ca": rng.randi_range(35, 45),  # 낮은 능력치
			"pa": rng.randi_range(75, 85),
			"position": all_positions[i],
			"age_months": 192 + rng.randi_range(0, 24),  # 16-18세
			"is_default": true  # 기본 제공 선수 표시
		}
		first_team.append(player)


# ===== 육성 시스템 연동 =====
func add_graduated_player(player_data: Dictionary) -> bool:
	"""육성 완료된 선수를 리저브에 추가"""
	if reserves.size() >= MAX_RESERVES:
		print("❌ 리저브가 가득 찼습니다! (%d/%d)" % [reserves.size(), MAX_RESERVES])
		return false

	# 플레이어 ID 생성
	if not player_data.has("id"):
		player_data["id"] = "player_" + str(Time.get_unix_time_from_system())

	reserves.append(player_data)
	emit_signal("player_graduated", player_data)
	emit_signal("roster_updated")

	print("✅ 선수 '%s'이(가) 리저브에 추가되었습니다!" % player_data.get("name", "Unknown"))
	print("   현재 리저브: %d/%d" % [reserves.size(), MAX_RESERVES])
	return true


# ===== 로스터 관리 =====
func promote_to_first_team(player_id: String) -> bool:
	"""리저브 → 1군 승격"""
	if first_team.size() >= MAX_FIRST_TEAM:
		print("❌ 1군이 가득 찼습니다! (%d/%d)" % [first_team.size(), MAX_FIRST_TEAM])
		return false

	var player = _find_and_remove_from_reserves(player_id)
	if player:
		first_team.append(player)
		emit_signal("roster_updated")
		print("✅ '%s' 선수가 1군으로 승격했습니다!" % player.get("name", "Unknown"))
		return true

	print("❌ 리저브에서 선수를 찾을 수 없습니다: %s" % player_id)
	return false


func demote_to_reserves(player_id: String) -> bool:
	"""1군 → 리저브 강등"""
	if reserves.size() >= MAX_RESERVES:
		print("❌ 리저브가 가득 찼습니다!")
		return false

	var player = _find_and_remove_from_first_team(player_id)
	if player:
		reserves.append(player)
		emit_signal("roster_updated")
		print("✅ '%s' 선수가 리저브로 이동했습니다." % player.get("name", "Unknown"))
		return true

	print("❌ 1군에서 선수를 찾을 수 없습니다: %s" % player_id)
	return false


func release_player(player_id: String) -> int:
	"""리저브 선수 방출"""
	var player = _find_and_remove_from_reserves(player_id)
	if player:
		var compensation = _calculate_release_compensation(player)
		emit_signal("roster_updated")
		print("✅ '%s' 선수를 방출했습니다. 보상금: %d" % [player.get("name", "Unknown"), compensation])
		return compensation

	print("❌ 방출할 선수를 찾을 수 없습니다: %s" % player_id)
	return 0


# ===== 덱 관리 =====
func set_manager_card(card: Dictionary) -> bool:
	"""감독 카드 설정"""
	if card.get("card_type") != "Manager":
		print("❌ 감독 카드만 설정할 수 있습니다!")
		return false

	current_deck["manager"] = card
	emit_signal("deck_changed")
	print("✅ 감독 카드 설정: %s" % card.get("name", "Unknown"))
	return true


func set_coach_card(slot: int, card: Dictionary) -> bool:
	"""코치 카드 설정 (슬롯 0-2)"""
	if slot < 0 or slot >= 3:
		print("❌ 유효하지 않은 코치 슬롯: %d" % slot)
		return false

	if card.get("card_type") != "Coach":
		print("❌ 코치 카드만 설정할 수 있습니다!")
		return false

	current_deck["coaches"][slot] = card
	emit_signal("deck_changed")
	print("✅ 코치 카드 설정 [슬롯 %d]: %s" % [slot, card.get("name", "Unknown")])
	return true


func set_tactics_card(slot: int, card: Dictionary) -> bool:
	"""전술 카드 설정 (슬롯 0-2)"""
	if slot < 0 or slot >= 3:
		print("❌ 유효하지 않은 전술 슬롯: %d" % slot)
		return false

	if card.get("card_type") != "Tactics":
		print("❌ 전술 카드만 설정할 수 있습니다!")
		return false

	current_deck["tactics"][slot] = card
	emit_signal("deck_changed")
	print("✅ 전술 카드 설정 [슬롯 %d]: %s" % [slot, card.get("name", "Unknown")])
	return true


func get_deck_bonus() -> float:
	"""현재 덱의 총 보너스 계산"""
	var bonus = 1.0

	# 감독 보너스
	if current_deck["manager"]:
		bonus *= 1.0 + (current_deck["manager"].get("rarity", 1) * 0.1)

	# 코치 보너스
	for coach in current_deck["coaches"]:
		if coach:
			bonus *= 1.0 + (coach.get("rarity", 1) * 0.05)

	# 전술 보너스
	for tactics in current_deck["tactics"]:
		if tactics:
			bonus *= 1.0 + (tactics.get("rarity", 1) * 0.05)

	# TODO: 시너지 보너스 계산

	return bonus


# ===== 경기 시스템 연동 =====
func can_start_match() -> bool:
	"""경기 시작 가능 여부 확인"""
	if first_team.size() < 11:
		print("❌ 1군에 최소 11명이 필요합니다! (현재: %d명)" % first_team.size())
		return false
	return true


func get_starting_eleven() -> Array:
	"""선발 11명 반환"""
	if first_team.size() >= 11:
		return first_team.slice(0, 11)
	return []


func get_bench_players() -> Array:
	"""벤치 선수들 반환"""
	if first_team.size() > 11:
		return first_team.slice(11, min(18, first_team.size()))
	return []


## 경기 시뮬레이션용 정규화된 팀 데이터 제공
func get_match_team_data_legacy() -> Dictionary:
	var name_val: String = team_name if typeof(team_name) == TYPE_STRING and team_name != "" else "My FC"
	var formation_val: String = "4-4-2"
	var players: Array = []

	# 우선 1군에서 수집, 부족하면 리저브로 채움
	var pool: Array = []
	for p in first_team:
		pool.append(p)
	if pool.size() < 18:
		for p in reserves:
			pool.append(p)
			if pool.size() >= 18:
				break

	# 최소 스키마(name/position/overall)로 변환
	for i in range(min(18, pool.size())):
		var src: Dictionary = pool[i] as Dictionary
		var pname := String(src.get("name", "Player %02d" % (i + 1)))
		var ppos := String(src.get("position", "MF"))
		var pov := int(src.get("overall", src.get("ca", 60)))
		(
			players
			. append(
				{
					"name": pname,
					"position": ppos,
					"overall": int(clampi(pov, 1, 200)),
				}
			)
		)

	# 18명 미만이면 패딩
	while players.size() < 18:
		var idx := players.size() + 1
		(
			players
			. append(
				{
					"name": "%s Bench %02d" % [name_val, idx],
					"position": "CM",
					"overall": 60,
				}
			)
		)

	return {"name": name_val, "formation": formation_val, "players": players}


func get_match_team_data() -> Dictionary:
	"""매치 시뮬레이션용 팀 데이터 생성 (OpenFootball 형식)"""
	var starting_eleven = get_starting_eleven()
	if starting_eleven.size() < 11:
		print("❌ 선발 11명이 부족합니다!")
		return {}

	var players = []
	for player in starting_eleven:
		players.append(
			{
				"name": player.get("name", "Unknown Player"),
				"position": player.get("position", "MF"),
				"ca": player.get("ca", 50),
				"pa": player.get("pa", 80),
				"age_months": player.get("age_months", 216),  # 18세 기본
				"id": player.get("id", "player_" + str(randi()))
			}
		)

	return {"name": team_name, "players": players, "formation": "4-4-2", "team_bonus": get_deck_bonus()}  # 기본 포메이션


func generate_ai_opponent(difficulty: String = "normal") -> Dictionary:
	"""AI 상대팀 생성 (난이도별)"""
	var base_ca = 50
	var team_name_prefix = "일반"

	match difficulty:
		"easy":
			base_ca = 40
			team_name_prefix = "약한"
		"normal":
			base_ca = 55
			team_name_prefix = "보통"
		"hard":
			base_ca = 70
			team_name_prefix = "강한"
		"legendary":
			base_ca = 85
			team_name_prefix = "전설의"

	var positions = ["GK", "DF", "DF", "DF", "DF", "MF", "MF", "MF", "FW", "FW", "FW"]
	var players = []

	for i in range(11):
		players.append(
			{
				"name": "AI 선수 " + str(i + 1),
				"position": positions[i],
				"ca": base_ca + randi_range(-5, 10),  # 약간의 변동
				"pa": base_ca + 20
			}
		)

	return {"name": team_name_prefix + " FC", "players": players, "formation": "4-3-3"}  # AI는 공격적


# ===== 저장/불러오기 =====
func save_data() -> Dictionary:
	"""MyTeam 데이터 저장"""
	return {
		"team_name": team_name,
		"team_emblem": team_emblem,
		"first_team": first_team,
		"reserves": reserves,
		"current_deck": current_deck,
		"statistics":
		{"matches": total_matches_played, "wins": total_wins, "draws": total_draws, "losses": total_losses},
		"created_at": team_created_at
	}


func load_data(data: Dictionary):
	"""MyTeam 데이터 불러오기"""
	team_name = data.get("team_name", "My FC")
	team_emblem = data.get("team_emblem", team_emblem)
	first_team = data.get("first_team", [])
	reserves = data.get("reserves", [])
	current_deck = data.get("current_deck", current_deck)

	var stats = data.get("statistics", {})
	total_matches_played = stats.get("matches", 0)
	total_wins = stats.get("wins", 0)
	total_draws = stats.get("draws", 0)
	total_losses = stats.get("losses", 0)

	team_created_at = data.get("created_at", team_created_at)
	emit_signal("roster_updated")
	emit_signal("deck_changed")

	print("✅ MyTeam 데이터 로드 완료!")


# ===== 내부 헬퍼 함수 =====
func _find_and_remove_from_reserves(player_id: String):
	"""리저브에서 선수 찾아서 제거"""
	for i in range(reserves.size()):
		if reserves[i].get("id") == player_id:
			return reserves.pop_at(i)
	return null


func _find_and_remove_from_first_team(player_id: String):
	"""1군에서 선수 찾아서 제거"""
	for i in range(first_team.size()):
		if first_team[i].get("id") == player_id:
			return first_team.pop_at(i)
	return null


func _calculate_release_compensation(player: Dictionary) -> int:
	"""방출 보상금 계산"""
	var base_compensation = 100
	var ca = player.get("ca", 40)
	return base_compensation + (ca * 10)


# ===== 디버그 함수 =====
func debug_print_team_status():
	"""팀 상태 디버그 출력"""
	print("\n===== MyTeam 현황 =====")
	print("팀명: %s" % team_name)
	print("1군: %d/%d명" % [first_team.size(), MAX_FIRST_TEAM])
	print("리저브: %d/%d명" % [reserves.size(), MAX_RESERVES])
	print("전적: %d전 %d승 %d무 %d패" % [total_matches_played, total_wins, total_draws, total_losses])

	print("\n덱 구성:")
	print("  감독: %s" % (current_deck["manager"].get("name", "없음") if current_deck["manager"] else "없음"))
	for i in range(3):
		var coach = current_deck["coaches"][i]
		print("  코치%d: %s" % [i + 1, coach.get("name", "없음") if coach else "없음"])
	for i in range(3):
		var tactics = current_deck["tactics"][i]
		print("  전술%d: %s" % [i + 1, tactics.get("name", "없음") if tactics else "없음"])

	print("  총 보너스: x%.2f" % get_deck_bonus())
	print("=========================")
