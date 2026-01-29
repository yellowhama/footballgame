# GraduationManager - 선수 졸업 시스템
extends Node

# ===== 시그널 =====
signal graduation_completed(success: bool, player_name: String)

# ============================================================
# DUAL ENDING SYSTEM (Phase 1 Upgrade)
# ============================================================

## Career-based ending types (CA-driven)
enum CareerEnding { PRO_SUPERSTAR = 0, OVERSEAS_STUDY = 1, COLLEGE_ACE = 2, COACH_PATH = 3, HIDDEN_LEGEND = 4 }  ## CA 150+ with national championship  ## CA 120+ with international experience  ## CA 100+ with academic balance  ## CA 80+ or default  ## CA 180+ with hidden conditions

## Relationship-based ending types
enum RelationshipEnding { COACH_DISCIPLE = 0, ETERNAL_RIVAL = 1, BEST_TEAMMATE = 2, SOLO_PLAYER = 3 }  ## Coach evaluation 90+  ## Rival affinity 90+  ## Friend affinity 90+  ## Default (no strong relationships)

# ===== 졸업 조건 =====
const GRADUATION_YEAR = 3
const GRADUATION_WEEK = 52

# ===== 상태 관리 =====
var graduation_processed: bool = false
var graduation_data: Dictionary = {}


func _ready():
	print("[GraduationManager] 선수 졸업 시스템 초기화")

	# GameManager 시그널 연결
	if has_node("/root/GameManager"):
		var game_manager = get_node("/root/GameManager")
		game_manager.week_advanced.connect(_on_week_advanced)
		game_manager.season_completed.connect(_on_season_completed)
		print("[GraduationManager] ✅ GameManager 시그널 연결 완료")
	else:
		print("[GraduationManager] ❌ GameManager를 찾을 수 없습니다!")

	# MyTeamManager 시그널 연결 준비
	if has_node("/root/MyTeamManager"):
		print("[GraduationManager] ✅ MyTeamManager 발견")
	else:
		print("[GraduationManager] ❌ MyTeamManager를 찾을 수 없습니다!")


func _on_week_advanced(week: int, year: int):
	"""주차 진행 시 졸업 조건 확인"""
	print("[GraduationManager] Week advanced: %d학년 %d주차" % [year, week])

	# 졸업 조건 확인: 3학년 52주차
	if year == GRADUATION_YEAR and week == GRADUATION_WEEK:
		if not graduation_processed:
			print("[GraduationManager] 🎓 졸업 조건 달성! 졸업 처리 시작...")
			_process_graduation()
		else:
			print("[GraduationManager] 이미 졸업 처리가 완료되었습니다.")


func _on_season_completed(completed_year: int):
	"""학년 완료 시 추가 확인"""
	print("[GraduationManager] Season completed: %d학년 완료" % completed_year)

	if completed_year == GRADUATION_YEAR:
		if not graduation_processed:
			print("[GraduationManager] 🎓 3학년 완료! 졸업 처리 시작...")
			_process_graduation()


func _process_graduation():
	"""실제 졸업 처리 로직"""
	print("[GraduationManager] ===== 졸업 처리 시작 =====")

	# PlayerData에서 현재 선수 데이터 가져오기
	var player_data_node = get_node_or_null("/root/PlayerData")
	if not player_data_node:
		print("[GraduationManager] ❌ PlayerData를 찾을 수 없습니다!")
		graduation_completed.emit(false, "Unknown")
		return

	# 선수 데이터 수집
	var player_data = player_data_node.get_player_data()
	print("[GraduationManager] 선수 정보 수집: %s" % player_data.get("name", "Unknown"))

	# 졸업 데이터 구성
	graduation_data = _create_graduation_data(player_data)

	# MyTeamData에 저장
	var success = _save_to_myteam(graduation_data)

	if success:
		print("[GraduationManager] ✅ 졸업 처리 완료!")
		graduation_processed = true

		# MyTeamManager에 시그널 발송
		if has_node("/root/MyTeamManager"):
			var my_team_manager = get_node("/root/MyTeamManager")
			my_team_manager.emit_signal("player_graduated", graduation_data)

		graduation_completed.emit(true, graduation_data.get("name", "Unknown"))
	else:
		print("[GraduationManager] ❌ 졸업 처리 실패!")
		graduation_completed.emit(false, graduation_data.get("name", "Unknown"))

	print("[GraduationManager] ===== 졸업 처리 종료 =====")


func _create_graduation_data(player_data: Dictionary) -> Dictionary:
	"""PlayerData를 MyTeamData 호환 형식으로 변환"""

	# ===== PHASE 24: Formula-based ending generation =====
	var ending_data = generate_ending()  # NEW: Use formula-based generation

	# Get real career stats from CareerStatisticsManager
	var stats_summary = ending_data.get("stats_summary", {})

	# OLD: Dual ending system (kept for backward compatibility with relationship endings)
	var ending_conditions = check_ending_conditions()

	# 기본 정보 (Phase 22: Enhanced with missing fields)
	var result_data = {
		"name": player_data.get("name", "졸업생"),
		"pseudo_name": player_data.get("name", "졸업생"),  # NEW: Phase 22
		"nationality": _get_nationality(),  # NEW: Phase 22
		"age": player_data.get("age", 21),  # 3년 후 21세
		"position": player_data.get("position", "ST"),
		"positions": [player_data.get("position", "ST")],  # NEW: Phase 22 (array format)
		"overall": player_data.get("overall", 60),
		"ca": player_data.get("overall", 60),  # NEW: Phase 22
		"pa": 199,  # NEW: Phase 22 (max potential after graduation)
		# 42개 스탯 (PlayerData와 MyTeamData 모두 동일한 구조)
		"technical": player_data.get("technical", {}),
		"mental": player_data.get("mental", {}),
		"physical": player_data.get("physical", {}),
		"goalkeeper": player_data.get("goalkeeper", {}),
		# ===== Phase 4: Personality (Phase 22 integration) =====
		"personality": _get_personality_data(player_data),  # NEW: Phase 22
		"personality_archetype": player_data.get("personality_archetype", "Steady"),  # NEW: Phase 22
		# ===== Phase 3: Special Abilities (Phase 22 integration) =====
		"special_abilities": _get_special_abilities(),  # NEW: Phase 22
		# ===== Phase 2: Exclusive Trait (Phase 22 integration) =====
		"exclusive_trait": _get_exclusive_trait(),  # NEW: Phase 22
		# 외형 데이터 (GlobalCharacterData에서 가져오기 시도)
		"appearance": _get_appearance_data(),
		# ===== PHASE 24: Enhanced career stats with real data =====
		"career_stats":
		{
			"seasons_played": 3,
			"total_goals": stats_summary.get("total_goals", 0),  # NEW: Real data
			"total_assists": stats_summary.get("total_assists", 0),  # NEW: Real data
			"total_matches": stats_summary.get("total_matches", 0),  # NEW: Real data
			"average_rating": stats_summary.get("average_rating", 0.0),  # NEW
			"win_rate": stats_summary.get("win_rate", 0.0),  # NEW
			"ca_growth": stats_summary.get("ca_growth", 0),  # NEW
			"final_division": stats_summary.get("final_division", 3),  # NEW
			"final_position": stats_summary.get("final_position", 6),  # NEW
			"ending_type": ending_data.get("korean_title", ending_conditions.combined_title),  # NEW: Formula-based
			"career_ending": ending_data.get("english_title", ending_conditions.career_name),  # NEW
			"relationship_ending": ending_conditions.relationship_name  # OLD: Keep for now
		},
		# ===== PHASE 24: Enhanced ending data =====
		"ending":
		{
			"type": ending_data.get("type", "GENERIC"),
			"korean_title": ending_data.get("korean_title", "졸업생"),
			"english_title": ending_data.get("english_title", "Graduate"),
			"rarity": ending_data.get("rarity", "B"),
			"description": ending_data.get("description", ""),
			"narrative": ending_data.get("narrative", ""),
			"highlights": ending_data.get("highlights", []),  # Top 5-8 decision highlights
			"stats_summary": stats_summary
		},
		# 추가 졸업 메타데이터
		"graduation_info":
		{
			"graduation_year": GRADUATION_YEAR,
			"graduation_week": GRADUATION_WEEK,
			"graduation_date": Time.get_datetime_string_from_system(),
			"final_condition": player_data.get("condition", 3),
			"final_fatigue": player_data.get("fatigue", 0.0),
			"final_ca": player_data.get("overall", 60),  # New: Final CA
			"coach_evaluation": get_coach_evaluation()  # New: Final coach evaluation
		}
	}

	print("[GraduationManager] 졸업 데이터 생성 완료:")
	print("  이름: %s" % result_data.name)
	print("  포지션: %s" % result_data.position)
	print("  종합 능력치: %d" % result_data.overall)
	print("  🎓 Phase 24 엔딩: %s (%s)" % [ending_data.korean_title, ending_data.rarity])
	print(
		(
			"    - Goals: %d, Assists: %d, Matches: %d"
			% [
				stats_summary.get("total_goals", 0),
				stats_summary.get("total_assists", 0),
				stats_summary.get("total_matches", 0)
			]
		)
	)
	print(
		(
			"    - Division: %d, Position: %d"
			% [stats_summary.get("final_division", 3), stats_summary.get("final_position", 6)]
		)
	)
	print("    - Highlights: %d key moments" % ending_data.get("highlights", []).size())

	return result_data


func _get_appearance_data() -> Dictionary:
	"""GlobalCharacterData에서 외형 데이터 가져오기"""
	if has_node("/root/GlobalCharacterData"):
		var char_data = get_node("/root/GlobalCharacterData")
		if char_data.character_data.has("appearance"):
			return char_data.character_data.appearance

	# 기본 외형 데이터
	return {"hair_color": "#8B4513", "skin_color": "#FFDBAC", "eye_color": "#8B4513"}


func _get_nationality() -> String:
	"""
	Phase 22: Get nationality from GlobalCharacterData
	Fallback to KOR if not available
	"""
	if has_node("/root/GlobalCharacterData"):
		var char_data = get_node("/root/GlobalCharacterData")
		if char_data.character_data.has("nationality"):
			return char_data.character_data.nationality
	return "KOR"


func _get_personality_data(player_data: Dictionary) -> Dictionary:
	"""
	Phase 22: Extract personality data (Phase 4 integration)
	Returns 8 personality fields (adaptability, ambition, determination, etc.)
	"""
	if player_data.has("personality"):
		return player_data.personality

	# Fallback: Try to get from PlayerData autoload
	if has_node("/root/PlayerData"):
		var pd = get_node("/root/PlayerData")
		# Build personality dict from individual fields (Phase 4 structure)
		var personality = {
			"adaptability": pd.get("personality_adaptability") if "personality_adaptability" in pd else 50,
			"ambition": pd.get("personality_ambition") if "personality_ambition" in pd else 50,
			"determination": pd.get("personality_determination") if "personality_determination" in pd else 50,
			"discipline": pd.get("personality_discipline") if "personality_discipline" in pd else 50,
			"loyalty": pd.get("personality_loyalty") if "personality_loyalty" in pd else 50,
			"pressure": pd.get("personality_pressure") if "personality_pressure" in pd else 50,
			"professionalism": pd.get("personality_professionalism") if "personality_professionalism" in pd else 50,
			"temperament": pd.get("personality_temperament") if "personality_temperament" in pd else 50
		}
		return personality

	# Last resort: Return neutral personality
	return {
		"adaptability": 50,
		"ambition": 50,
		"determination": 50,
		"discipline": 50,
		"loyalty": 50,
		"pressure": 50,
		"professionalism": 50,
		"temperament": 50
	}


func _get_special_abilities() -> Array:
	"""
	Phase 22: Get special abilities (Phase 3 integration)
	Returns array of ability names/structures
	"""
	if has_node("/root/PlayerData"):
		var pd = get_node("/root/PlayerData")
		if "special_abilities" in pd and pd.special_abilities is Array:
			return pd.special_abilities.duplicate(true)
	return []


func _get_exclusive_trait() -> Variant:
	"""
	Phase 22: Get exclusive trait (Phase 2 integration)
	Returns trait object or null if none
	"""
	if has_node("/root/PlayerData"):
		var pd = get_node("/root/PlayerData")
		if "exclusive_trait" in pd and pd.exclusive_trait != "":
			var trait_data = {"name": pd.exclusive_trait}
			if "exclusive_trait_level" in pd:
				trait_data["level"] = pd.exclusive_trait_level
			return trait_data
	return null


func _save_to_myteam(player_grad_data: Dictionary) -> bool:
	"""MyTeamData에 졸업생 저장"""
	var myteam_data = get_node_or_null("/root/MyTeamData")
	if not myteam_data:
		print("[GraduationManager] ❌ MyTeamData를 찾을 수 없습니다!")
		return false

	# MyTeamData.save_player_from_career() 호출
	var success = myteam_data.save_player_from_career(player_grad_data)

	if success:
		print("[GraduationManager] ✅ MyTeamData에 졸업생 저장 완료")
	else:
		print("[GraduationManager] ❌ MyTeamData 저장 실패 (팀 가득참 등)")

	return success


# ============================================================
# DUAL ENDING SYSTEM - ENDING DETERMINATION
# ============================================================


## Checks and returns dual ending conditions
func check_ending_conditions() -> Dictionary:
	var career = determine_career_ending()
	var relationship = determine_relationship_ending()

	return {
		"career_ending": career,
		"relationship_ending": relationship,
		"combined_title": get_combined_ending_title(career, relationship),
		"career_name": get_career_ending_name(career),
		"relationship_name": get_relationship_ending_name(relationship)
	}


## Determines career ending based on CA and achievements
func determine_career_ending() -> CareerEnding:
	var player_data_node = get_node_or_null("/root/PlayerData")
	if not player_data_node:
		return CareerEnding.COACH_PATH

	var ca = player_data_node.get_current_ability()

	# HIDDEN_LEGEND: CA 180+ with special conditions
	if ca >= 180 and has_hidden_conditions():
		return CareerEnding.HIDDEN_LEGEND

	# PRO_SUPERSTAR: CA 150+ with national championship
	elif ca >= 150 and has_national_championship():
		return CareerEnding.PRO_SUPERSTAR

	# OVERSEAS_STUDY: CA 120+ with international experience
	elif ca >= 120 and has_international_experience():
		return CareerEnding.OVERSEAS_STUDY

	# COLLEGE_ACE: CA 100+ with academic balance
	elif ca >= 100 and has_academic_balance():
		return CareerEnding.COLLEGE_ACE

	# COACH_PATH: Default for CA < 100
	else:
		return CareerEnding.COACH_PATH


## Determines relationship ending based on evaluation and affinities
func determine_relationship_ending() -> RelationshipEnding:
	var coach_eval = get_coach_evaluation()
	var rival_affinity = get_rival_affinity()
	var friend_affinity = get_friend_affinity()

	# COACH_DISCIPLE: Coach evaluation 90+
	if coach_eval >= 90:
		return RelationshipEnding.COACH_DISCIPLE

	# ETERNAL_RIVAL: Rival affinity 90+
	elif rival_affinity >= 90:
		return RelationshipEnding.ETERNAL_RIVAL

	# BEST_TEAMMATE: Friend affinity 90+
	elif friend_affinity >= 90:
		return RelationshipEnding.BEST_TEAMMATE

	# SOLO_PLAYER: Default (no strong relationships)
	else:
		return RelationshipEnding.SOLO_PLAYER


## Returns combined ending title (20 possible combinations)
func get_combined_ending_title(career: CareerEnding, relationship: RelationshipEnding) -> String:
	# Ending title matrix (5 career × 4 relationship = 20 endings)
	var titles = {
		# PRO_SUPERSTAR combinations
		"PRO_SUPERSTAR_COACH_DISCIPLE": "감독의 계승자, 프로의 별",
		"PRO_SUPERSTAR_ETERNAL_RIVAL": "라이벌의 맹세, 프로의 정상에서",
		"PRO_SUPERSTAR_BEST_TEAMMATE": "우정의 프로 선수",
		"PRO_SUPERSTAR_SOLO_PLAYER": "고독한 별",
		# OVERSEAS_STUDY combinations
		"OVERSEAS_STUDY_COACH_DISCIPLE": "유럽행 제자",
		"OVERSEAS_STUDY_ETERNAL_RIVAL": "숙명의 라이벌, 해외로의 도전",
		"OVERSEAS_STUDY_BEST_TEAMMATE": "함께하는 꿈, 해외 유학",
		"OVERSEAS_STUDY_SOLO_PLAYER": "혼자 떠나는 길",
		# COLLEGE_ACE combinations
		"COLLEGE_ACE_COACH_DISCIPLE": "대학의 스승",
		"COLLEGE_ACE_ETERNAL_RIVAL": "캠퍼스 라이벌",
		"COLLEGE_ACE_BEST_TEAMMATE": "우정의 대학 에이스",
		"COLLEGE_ACE_SOLO_PLAYER": "홀로 서는 대학 에이스",
		# COACH_PATH combinations
		"COACH_PATH_COACH_DISCIPLE": "스승의 스승",
		"COACH_PATH_ETERNAL_RIVAL": "코치 라이벌",
		"COACH_PATH_BEST_TEAMMATE": "함께 가르치는 길",
		"COACH_PATH_SOLO_PLAYER": "고독한 지도자",
		# HIDDEN_LEGEND combinations
		"HIDDEN_LEGEND_COACH_DISCIPLE": "전설의 계승",
		"HIDDEN_LEGEND_ETERNAL_RIVAL": "전설의 라이벌",
		"HIDDEN_LEGEND_BEST_TEAMMATE": "전설의 우정",
		"HIDDEN_LEGEND_SOLO_PLAYER": "외로운 전설"
	}

	var key = "%s_%s" % [CareerEnding.keys()[career], RelationshipEnding.keys()[relationship]]

	return titles.get(key, "축구 인생의 새로운 시작")


# ============================================================
# PHASE 24: FORMULA-BASED ENDING GENERATION (SSOT Compliant)
# ============================================================

## Ending Matrix: [League Outcome] + [Role] + [Decision Style] = Career Ending
## SSOT: "Endings are generated from decisions, not selected from a fixed list"
## Total Combinations: 28 (Elite: 8, Pro: 10, College: 7, Amateur: 1, Developing: 1, Fallback: 1)
const ENDING_MATRIX = {
	# Elite tier (Division 1 Champion)
	"Elite_Striker_Aggressive":
	{
		"type": "PRO_STRIKER_ELITE",
		"korean_title": "🔥 공격형 엘리트 스트라이커",
		"english_title": "Elite Aggressive Striker",
		"rarity": "SS",
		"description": "3년간 공격적인 플레이로 Division 1 정상에 올랐습니다."
	},
	"Elite_Striker_Balanced":
	{
		"type": "PRO_STRIKER",
		"korean_title": "⚽ 완성형 프로 스트라이커",
		"english_title": "Complete Professional Striker",
		"rarity": "S",
		"description": "균형잡힌 성장으로 프로 스트라이커의 길을 열었습니다."
	},
	"Elite_Playmaker_Balanced":
	{
		"type": "PRO_PLAYMAKER",
		"korean_title": "🎯 완성형 플레이메이커",
		"english_title": "Complete Playmaker",
		"rarity": "S",
		"description": "뛰어난 시야와 패스로 팀을 이끌었습니다."
	},
	"Elite_Balanced_Balanced":
	{
		"type": "PRO_ALLROUNDER",
		"korean_title": "🌟 완벽한 올라운더",
		"english_title": "Perfect All-rounder",
		"rarity": "S",
		"description": "모든 면에서 균형잡힌 완벽한 선수로 성장했습니다."
	},
	"Elite_Defender_Balanced":
	{
		"type": "PRO_DEFENDER",
		"korean_title": "🛡️ 철벽 수비수",
		"english_title": "Elite Defender",
		"rarity": "S",
		"description": "수비에서 팀을 지키는 든든한 기둥이 되었습니다."
	},
	"Elite_GK_Balanced":
	{
		"type": "PRO_GK",
		"korean_title": "🧤 프로 골키퍼",
		"english_title": "Professional Goalkeeper",
		"rarity": "S",
		"description": "골문을 지키는 마지막 보루가 되었습니다."
	},
	"Elite_Striker_Conservative":
	{
		"type": "PRO_STRIKER_SAFE",
		"korean_title": "🛡️ 안정형 엘리트 스트라이커",
		"english_title": "Conservative Elite Striker",
		"rarity": "S",
		"description": "안정적인 플레이로 꾸준히 정상에 머물렀습니다."
	},
	"Elite_Playmaker_Conservative":
	{
		"type": "PRO_PLAYMAKER_SAFE",
		"korean_title": "🧠 신중한 엘리트 플레이메이커",
		"english_title": "Conservative Elite Playmaker",
		"rarity": "S",
		"description": "위험을 최소화하며 팀을 이끌었습니다."
	},
	# Pro tier (Division 1-2 top 3)
	"Pro_Striker_Aggressive":
	{
		"type": "PRO_STRIKER",
		"korean_title": "⚽ 프로 지망 스트라이커",
		"english_title": "Professional Striker",
		"rarity": "A",
		"description": "프로 무대를 향한 첫 걸음을 내디뎠습니다."
	},
	"Pro_Striker_Balanced":
	{
		"type": "PRO_PROSPECT",
		"korean_title": "⭐ 유망주 공격수",
		"english_title": "Promising Striker",
		"rarity": "A",
		"description": "프로에서 활약할 가능성이 높은 선수입니다."
	},
	"Pro_Playmaker_Balanced":
	{
		"type": "PRO_PLAYMAKER",
		"korean_title": "🎯 프로 플레이메이커",
		"english_title": "Professional Playmaker",
		"rarity": "A",
		"description": "팀을 이끄는 플레이메이커로 성장했습니다."
	},
	"Pro_Balanced_Balanced":
	{
		"type": "PRO_PROSPECT",
		"korean_title": "⭐ 프로 유망주",
		"english_title": "Professional Prospect",
		"rarity": "A",
		"description": "프로에서 성공할 잠재력을 보여주었습니다."
	},
	"Pro_Defender_Balanced":
	{
		"type": "PRO_DEFENDER",
		"korean_title": "🛡️ 프로 지망 수비수",
		"english_title": "Professional Defender",
		"rarity": "A",
		"description": "수비에서 두각을 나타내며 프로 무대를 노립니다."
	},
	"Pro_GK_Balanced":
	{
		"type": "PRO_GK",
		"korean_title": "🧤 프로 지망 골키퍼",
		"english_title": "Professional Goalkeeper",
		"rarity": "A",
		"description": "골대를 지키며 프로 계약을 기대합니다."
	},
	"Pro_Striker_Conservative":
	{
		"type": "PRO_STRIKER_SAFE",
		"korean_title": "⚽ 신중한 프로 지망생",
		"english_title": "Conservative Pro Striker",
		"rarity": "A",
		"description": "안정적인 플레이로 프로 진출을 준비했습니다."
	},
	"Pro_Playmaker_Conservative":
	{
		"type": "PRO_PLAYMAKER_SAFE",
		"korean_title": "🎯 신중한 프로 플레이메이커",
		"english_title": "Conservative Pro Playmaker",
		"rarity": "A",
		"description": "위험을 피하며 팀을 이끄는 플레이메이커입니다."
	},
	"Pro_Defender_Conservative":
	{
		"type": "PRO_DEFENDER_SAFE",
		"korean_title": "🛡️ 신중한 프로 수비수",
		"english_title": "Conservative Pro Defender",
		"rarity": "A",
		"description": "안정적인 수비로 프로를 준비하는 선수입니다."
	},
	"Pro_GK_Conservative":
	{
		"type": "PRO_GK_SAFE",
		"korean_title": "🧤 신중한 프로 골키퍼",
		"english_title": "Conservative Pro Goalkeeper",
		"rarity": "A",
		"description": "안정적인 선방으로 프로에 도전합니다."
	},
	# College tier (Division 2-3 any position)
	"College_Striker_Balanced":
	{
		"type": "COLLEGE_STRIKER",
		"korean_title": "🎓 대학 진학형 공격수",
		"english_title": "College Striker",
		"rarity": "B",
		"description": "대학에서 한 단계 더 성장할 잠재력을 보여주었습니다."
	},
	"College_Playmaker_Balanced":
	{
		"type": "COLLEGE_PLAYMAKER",
		"korean_title": "🎓 대학 진학형 플레이메이커",
		"english_title": "College Playmaker",
		"rarity": "B",
		"description": "대학 무대에서 활약할 수 있는 선수입니다."
	},
	"College_Balanced_Balanced":
	{
		"type": "COLLEGE_PLAYER",
		"korean_title": "🎓 대학 진학형 선수",
		"english_title": "College Player",
		"rarity": "B",
		"description": "대학 축구에서 계속 성장할 수 있는 선수입니다."
	},
	"College_Defender_Balanced":
	{
		"type": "COLLEGE_DEFENDER",
		"korean_title": "🎓 대학 진학형 수비수",
		"english_title": "College Defender",
		"rarity": "B",
		"description": "대학에서 수비의 핵심이 될 수 있습니다."
	},
	"College_GK_Balanced":
	{
		"type": "COLLEGE_GK",
		"korean_title": "🎓 대학 진학형 골키퍼",
		"english_title": "College Goalkeeper",
		"rarity": "B",
		"description": "대학에서 골문을 지킬 실력을 갖췄습니다."
	},
	"College_Striker_Conservative":
	{
		"type": "COLLEGE_STRIKER_SAFE",
		"korean_title": "🎓 신중한 대학 진학형 공격수",
		"english_title": "Conservative College Striker",
		"rarity": "B",
		"description": "안전한 선택으로 대학 진학을 준비했습니다."
	},
	"College_Playmaker_Conservative":
	{
		"type": "COLLEGE_PLAYMAKER_SAFE",
		"korean_title": "🎓 신중한 대학 플레이메이커",
		"english_title": "Conservative College Playmaker",
		"rarity": "B",
		"description": "안정적인 플레이로 대학에서 활약할 수 있습니다."
	},
	# Amateur tier (Division 3 bottom - FAILURE IS VALID!)
	"Amateur_any_any":
	{
		"type": "AMATEUR",
		"korean_title": "⚽ 아마추어 선수",
		"english_title": "Amateur Player",
		"rarity": "C",
		"description": "프로는 어렵지만, 축구를 사랑하는 선수로 성장했습니다."
	},
	# Developing (no decisions tracked yet)
	"Developing_any_any":
	{
		"type": "DEVELOPING",
		"korean_title": "🌱 성장 중인 선수",
		"english_title": "Developing Player",
		"rarity": "C",
		"description": "아직 성장 중인 선수입니다."
	}
}


## Generate career ending from formula (SSOT-compliant)
## Formula: [League Outcome] + [Player Role] + [Decision Patterns] + [Key Moments] = Ending
func generate_ending() -> Dictionary:
	"""
	SSOT-compliant ending generation
	Returns: {
		type: String,
		korean_title: String,
		english_title: String,
		rarity: String (SS/S/A/B/C),
		description: String,
		narrative: String,
		highlights: Array (top 5-8 decisions),
		stats_summary: Dictionary
	}
	"""

	# Safe fallback if systems not ready
	if not CareerStatisticsManager or not DecisionTracker:
		return _get_fallback_ending()

	# 1. Get career summary from CareerStatisticsManager
	var career_stats = CareerStatisticsManager.get_career_summary()

	# 2. Classify league outcome
	var league_tier = _classify_league_outcome(career_stats.league_outcome)

	# 3. Classify player role
	var role = _classify_role(career_stats.player_role)

	# 4. Classify decision style
	var style = _classify_style(career_stats.decision_patterns)

	# 5. Generate ending from matrix
	var ending_key = "%s_%s_%s" % [league_tier, role, style]

	# Check for Amateur tier first (FAILURE IS VALID!)
	if league_tier == "Amateur":
		var ending_data = ENDING_MATRIX.get("Amateur_any_any", _get_fallback_ending())
		ending_data = ending_data.duplicate(true)
		ending_data["narrative"] = _generate_narrative(career_stats, ending_data)
		ending_data["highlights"] = career_stats.get("key_moments", [])
		ending_data["stats_summary"] = _build_stats_summary(career_stats)

		if OS.is_debug_build():
			print("[GraduationManager] Generated ending: %s (%s)" % [ending_data.korean_title, ending_data.type])
			print("  → League: %s (Amateur path)" % league_tier)

		return ending_data

	var ending_data = ENDING_MATRIX.get(ending_key, _get_generic_fallback(league_tier, role))

	# 6. Enrich with narrative
	ending_data = ending_data.duplicate(true)
	ending_data["narrative"] = _generate_narrative(career_stats, ending_data)
	ending_data["highlights"] = career_stats.get("key_moments", [])
	ending_data["stats_summary"] = _build_stats_summary(career_stats)

	if OS.is_debug_build():
		print("[GraduationManager] Generated ending: %s (%s)" % [ending_data.korean_title, ending_data.type])
		print("  → League: %s, Role: %s, Style: %s" % [league_tier, role, style])

	return ending_data


## Classify league outcome: Elite / Pro / College / Amateur
func _classify_league_outcome(league_outcome: Dictionary) -> String:
	var final_div = league_outcome.get("final_division", 3)
	var final_pos = league_outcome.get("final_position", 6)

	# Elite: Division 1 Champion
	if final_div == 1 and final_pos == 1:
		return "Elite"

	# Pro: Division 1 any position (2-6) OR Division 2 top 3
	if final_div == 1:
		return "Pro"  # All Division 1 players → Pro (not College!)
	if final_div == 2 and final_pos <= 3:
		return "Pro"

	# College: Division 2 mid-bottom (4-6) OR Division 3 mid (1-4)
	if final_div == 2:
		return "College"  # Division 2 position 4-6
	if final_div == 3 and final_pos <= 4:
		return "College"

	# Amateur: Division 3 bottom (5-6)
	return "Amateur"  # FAILURE IS VALID!


## Classify player role: Striker / Playmaker / Balanced / Defender / GK / Developing
func _classify_role(player_role: Dictionary) -> String:
	return player_role.get("role_type", "Balanced")


## Classify decision style: Aggressive / Balanced / Conservative
func _classify_style(patterns: Dictionary) -> String:
	var tendency = patterns.get("training_tendency", "Balanced")
	return tendency


## Generate career narrative from statistics
func _generate_narrative(career_stats: Dictionary, ending_data: Dictionary) -> String:
	var league = career_stats.get("league_outcome", {})
	var role = career_stats.get("player_role", {})
	var match_stats = career_stats.get("match_statistics", {})

	var narrative = ""

	# Division progression
	var div_history = league.get("seasons", [])
	if div_history.size() >= 2:
		var start_div = div_history[0].get("division", 3)
		var end_div = div_history[div_history.size() - 1].get("division", 3)

		if end_div < start_div:
			narrative += "Division %d에서 시작해 Division %d까지 승격했습니다. " % [start_div, end_div]
		elif end_div > start_div:
			narrative += "Division %d에서 시작했지만 Division %d로 강등되었습니다. " % [start_div, end_div]
		else:
			narrative += "Division %d에서 3년을 보냈습니다. " % start_div

	# Role-specific narrative
	var goals = role.get("goals", 0)
	var assists = role.get("assists", 0)
	var total_matches = match_stats.get("total_matches", 0)

	if goals > 20:
		narrative += "총 %d골을 기록하며 공격수로서 활약했습니다. " % goals
	elif assists > 15:
		narrative += "총 %d도움을 기록하며 플레이메이커로 성장했습니다. " % assists
	elif goals + assists > 15:
		narrative += "총 %d골 %d도움을 기록했습니다. " % [goals, assists]

	if total_matches > 0:
		var avg_rating = match_stats.get("average_rating", 0.0)
		if avg_rating >= 7.5:
			narrative += "평균 평점 %.1f로 뛰어난 활약을 보여주었습니다." % avg_rating
		elif avg_rating >= 6.5:
			narrative += "평균 평점 %.1f로 안정적인 경기력을 보였습니다." % avg_rating

	return narrative if narrative != "" else ending_data.get("description", "")


## Build stats summary for ending display
func _build_stats_summary(career_stats: Dictionary) -> Dictionary:
	var match_stats = career_stats.get("match_statistics", {})
	var league = career_stats.get("league_outcome", {})
	var role = career_stats.get("player_role", {})

	return {
		"total_matches": match_stats.get("total_matches", 0),
		"total_goals": role.get("goals", 0),
		"total_assists": role.get("assists", 0),
		"average_rating": match_stats.get("average_rating", 0.0),
		"win_rate": match_stats.get("win_rate", 0.0),
		"final_division": league.get("final_division", 3),
		"final_position": league.get("final_position", 6),
		"ca_growth": _calculate_ca_growth(career_stats)
	}


## Calculate CA growth from timeline
func _calculate_ca_growth(career_stats: Dictionary) -> int:
	var ca_timeline = career_stats.get("ca_timeline", [])
	if ca_timeline.size() < 2:
		return 0

	var start_ca = ca_timeline[0].get("ca", 0)
	var end_ca = ca_timeline[ca_timeline.size() - 1].get("ca", 0)
	return end_ca - start_ca


## Get fallback ending when systems not ready
func _get_fallback_ending() -> Dictionary:
	return {
		"type": "GENERIC",
		"korean_title": "🎓 졸업생",
		"english_title": "Graduate",
		"rarity": "B",
		"description": "3년간의 아카데미 생활을 마쳤습니다.",
		"narrative": "축구 선수로서의 첫 여정을 성공적으로 마쳤습니다.",
		"highlights": [],
		"stats_summary": {}
	}


## Get generic fallback for missing ending combinations
func _get_generic_fallback(league_tier: String, role: String) -> Dictionary:
	# Try to find a similar ending
	for key in ENDING_MATRIX.keys():
		if league_tier in key or role in key:
			return ENDING_MATRIX[key].duplicate(true)

	# Ultimate fallback
	return _get_fallback_ending()


# ============================================================
# HELPER FUNCTIONS - CAREER CONDITIONS
# ============================================================


## Checks for hidden legend conditions (perfect stats + achievements)
func has_hidden_conditions() -> bool:
	# TODO: Implement actual hidden conditions
	# Example: All stats 90+, won all tournaments, perfect attendance
	return false


## Checks for national championship achievement
func has_national_championship() -> bool:
	# TODO: Implement actual championship check
	# Example: Check if player won national tournament
	return false


## Checks for international experience
func has_international_experience() -> bool:
	# TODO: Implement actual international experience check
	# Example: Check if player participated in international matches
	return false


## Checks for academic balance (good grades + good football)
func has_academic_balance() -> bool:
	# TODO: Implement actual academic balance check
	# Example: Check if player maintained good grades while training
	return true  # Default to true for now


# ============================================================
# HELPER FUNCTIONS - RELATIONSHIP VALUES
# ============================================================


## Gets current coach evaluation from CoachEvaluationManager
func get_coach_evaluation() -> int:
	var coach_eval_manager = get_node_or_null("/root/CoachEvaluationManager")
	if coach_eval_manager:
		return coach_eval_manager.get_evaluation()
	return 0


## Gets rival affinity (placeholder for future system)
func get_rival_affinity() -> int:
	# TODO: Implement actual rival affinity system
	# For now, return 0 (no rival system yet)
	return 0


## Gets friend affinity (placeholder for future system)
func get_friend_affinity() -> int:
	# TODO: Implement actual friend affinity system
	# For now, return 0 (no friend system yet)
	return 0


# ============================================================
# UI HELPER FUNCTIONS - ENDING NAMES
# ============================================================


## Returns Korean name for career ending
func get_career_ending_name(career: CareerEnding) -> String:
	match career:
		CareerEnding.PRO_SUPERSTAR:
			return "프로 슈퍼스타"
		CareerEnding.OVERSEAS_STUDY:
			return "해외 유학"
		CareerEnding.COLLEGE_ACE:
			return "대학 에이스"
		CareerEnding.COACH_PATH:
			return "코치의 길"
		CareerEnding.HIDDEN_LEGEND:
			return "히든 레전드"
		_:
			return "알 수 없음"


## Returns Korean name for relationship ending
func get_relationship_ending_name(relationship: RelationshipEnding) -> String:
	match relationship:
		RelationshipEnding.COACH_DISCIPLE:
			return "감독의 후계자"
		RelationshipEnding.ETERNAL_RIVAL:
			return "영원한 라이벌"
		RelationshipEnding.BEST_TEAMMATE:
			return "최고의 동료"
		RelationshipEnding.SOLO_PLAYER:
			return "고독한 천재"
		_:
			return "알 수 없음"


# ===== 디버그/테스트 함수 =====
func debug_trigger_graduation():
	"""디버그용 강제 졸업 처리"""
	print("[GraduationManager] 🧪 DEBUG: 강제 졸업 처리")
	graduation_processed = false  # 재처리 허용
	_process_graduation()


func debug_check_conditions() -> Dictionary:
	"""현재 졸업 조건 상태 확인"""
	var game_manager = get_node_or_null("/root/GameManager")
	if not game_manager:
		return {"error": "GameManager not found"}

	var current_week = game_manager.get_current_week()
	var current_year = game_manager.get_current_year()

	return {
		"current_week": current_week,
		"current_year": current_year,
		"graduation_week": GRADUATION_WEEK,
		"graduation_year": GRADUATION_YEAR,
		"conditions_met": current_year == GRADUATION_YEAR and current_week == GRADUATION_WEEK,
		"graduation_processed": graduation_processed
	}


func get_graduation_status() -> Dictionary:
	"""졸업 시스템 상태 반환"""
	return {
		"graduation_processed": graduation_processed,
		"graduation_data": graduation_data,
		"has_graduated_player": graduation_data.size() > 0
	}


func get_graduation_data() -> Dictionary:
	"""졸업 데이터 반환"""
	return graduation_data.duplicate()
