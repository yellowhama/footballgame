class_name CoachProfile
extends Resource

# 코치 스타일에 따른 주간 팀훈련 분배
@export var name: String = "Default Coach"
@export var style: String = "Balanced"  # Speed/Technical/Defensive/Balanced

# 스탯 포커스 (합이 1.0이 되도록)
@export var weekly_focus: Dictionary = {"pace": 0.2, "stamina": 0.2, "technique": 0.2, "tactics": 0.2, "mentality": 0.2}

@export_range(0.5, 1.5, 0.1) var base_intensity: float = 1.0
@export var affects_morale: float = 0.0  # 주간 사기 보정 (-10 ~ +10)
@export var affects_fatigue: float = 0.0  # 주간 피로 보정 (-5 ~ +5)

# 코치 성격/철학
@export_multiline var philosophy: String = ""


func make_team_plan_for_week(term: String, week_in_term: int) -> Dictionary:
	# 학기/주차에 따라 강도 조절
	var intensity_mod := 1.0

	# 방학엔 강도 낮춤
	if "방학" in term:
		intensity_mod = 0.7

	# 시즌 초반엔 적응 기간
	if week_in_term <= 2:
		intensity_mod *= 0.8

	# 시즌 막바지엔 컨디션 관리
	if week_in_term >= 14:
		intensity_mod *= 0.9

	return {
		"kind": "Team",
		"focus": weekly_focus,
		"intensity": base_intensity * intensity_mod,
		"term": term,
		"week": week_in_term,
		"coach": name,
		"morale_bonus": affects_morale,
		"fatigue_bonus": affects_fatigue
	}


func apply_to_stats(stats: Dictionary, intensity: float = 1.0) -> Dictionary:
	# 팀훈련 결과를 스탯에 적용
	var deltas := {}

	# 포커스에 따라 스탯 증가량 계산
	for focus_area in weekly_focus:
		var value = weekly_focus[focus_area] * intensity * 2.0  # 주간 증가량

		# 포커스 영역을 실제 스탯으로 매핑
		match focus_area:
			"pace":
				deltas["pace"] = value * 0.6
				deltas["acceleration"] = value * 0.4
			"stamina":
				deltas["stamina"] = value * 0.8
				deltas["conditioning"] = value * 0.2
			"technique":
				deltas["first_touch"] = value * 0.3
				deltas["dribbling"] = value * 0.3
				deltas["passing"] = value * 0.4
			"tactics":
				deltas["positioning"] = value * 0.5
				deltas["vision"] = value * 0.5
			"mentality":
				deltas["decision"] = value * 0.5
				deltas["composure"] = value * 0.5

	return deltas


# 프리셋 팩토리 메서드들
static func create_speed_coach() -> CoachProfile:
	var coach = CoachProfile.new()
	coach.name = "김속도 감독"
	coach.style = "Speed"
	coach.weekly_focus = {"pace": 0.35, "stamina": 0.25, "technique": 0.15, "tactics": 0.15, "mentality": 0.10}
	coach.base_intensity = 1.2
	coach.affects_fatigue = 3.0
	coach.affects_morale = -2.0
	coach.philosophy = "빠른 역습과 압박 축구를 추구합니다."
	return coach


static func create_technical_coach() -> CoachProfile:
	var coach = CoachProfile.new()
	coach.name = "박기술 감독"
	coach.style = "Technical"
	coach.weekly_focus = {"pace": 0.10, "stamina": 0.15, "technique": 0.40, "tactics": 0.25, "mentality": 0.10}
	coach.base_intensity = 1.0
	coach.affects_morale = 3.0
	coach.philosophy = "볼 소유와 패스 플레이를 중시합니다."
	return coach


static func create_defensive_coach() -> CoachProfile:
	var coach = CoachProfile.new()
	coach.name = "이수비 감독"
	coach.style = "Defensive"
	coach.weekly_focus = {"pace": 0.15, "stamina": 0.30, "technique": 0.10, "tactics": 0.35, "mentality": 0.10}
	coach.base_intensity = 1.1
	coach.affects_fatigue = 2.0
	coach.affects_morale = 1.0
	coach.philosophy = "견고한 수비와 조직력을 바탕으로 합니다."
	return coach


static func create_balanced_coach() -> CoachProfile:
	var coach = CoachProfile.new()
	coach.name = "정균형 감독"
	coach.style = "Balanced"
	coach.weekly_focus = {"pace": 0.20, "stamina": 0.20, "technique": 0.20, "tactics": 0.20, "mentality": 0.20}
	coach.base_intensity = 1.0
	coach.affects_morale = 2.0
	coach.affects_fatigue = -1.0
	coach.philosophy = "균형잡힌 훈련으로 선수를 성장시킵니다."
	return coach
