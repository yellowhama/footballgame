class_name CAValidator
extends RefCounted

## CA 계산 검증 시스템
## OpenFootball 엔진과 100% 일치하는 CA 계산 구현


## OpenFootball 정확한 CA 계산 (포지션 모디파이어 포함)
static func calculate_ca_openfootball_accurate(attributes: Dictionary, position_category: String) -> Dictionary:
	var technical_sum = 0
	var mental_sum = 0
	var physical_sum = 0
	var gk_sum = 0

	# Technical attributes (14개)
	var technical_attrs = [
		"corners",
		"crossing",
		"dribbling",
		"finishing",
		"first_touch",
		"free_kicks",
		"heading",
		"long_shots",
		"passing",
		"shooting",
		"ball_control",
		"technique",
		"penalties",
		"throw_ins"
	]

	for attr in technical_attrs:
		if attributes.has(attr):
			technical_sum += attributes[attr]

	# Mental attributes (14개)
	var mental_attrs = [
		"decisions",
		"concentration",
		"leadership",
		"vision",
		"teamwork",
		"work_rate",
		"positioning",
		"anticipation",
		"composure",
		"bravery",
		"determination",
		"flair",
		"off_the_ball",
		"aggression"
	]

	for attr in mental_attrs:
		if attributes.has(attr):
			mental_sum += attributes[attr]

	# Physical attributes (8개) - 2배 가중치
	var physical_attrs = [
		"speed", "stamina", "strength", "agility", "balance", "jumping", "natural_fitness", "acceleration"
	]

	for attr in physical_attrs:
		if attributes.has(attr):
			physical_sum += attributes[attr]

	# GK attributes (6개)
	var gk_attrs = ["reflexes", "handling", "aerial_ability", "command_of_area", "communication", "kicking"]

	for attr in gk_attrs:
		if attributes.has(attr):
			gk_sum += attributes[attr]

	# OpenFootball 정확한 공식
	var total_units = technical_sum + mental_sum + (physical_sum * 2) + gk_sum

	var base_ca = 0.0
	if total_units >= 1000:
		base_ca = (total_units - 1000) / 20.0
	else:
		base_ca = total_units / 40.0

	# 포지션 모디파이어 계산 (OpenFootball ca_calculator.rs:66-93 기반)
	var position_modifier = _calculate_position_modifier(attributes, position_category)

	# 최종 CA 계산
	var final_ca = int(roundf(base_ca * position_modifier))
	final_ca = min(200, max(0, final_ca))  # 0-200 범위 제한

	return {
		"ca": final_ca,
		"base_ca": int(base_ca),
		"position_modifier": position_modifier,
		"total_units": total_units,
		"technical_sum": technical_sum,
		"mental_sum": mental_sum,
		"physical_sum": physical_sum,
		"gk_sum": gk_sum,
		"breakdown": {"technical": technical_sum, "mental": mental_sum, "physical": physical_sum * 2, "gk": gk_sum}  # 실제 가중 적용된 값
	}


## 포지션별 모디파이어 계산 (OpenFootball 정확한 공식)
static func _calculate_position_modifier(attributes: Dictionary, position_category: String) -> float:
	var relevant_avg = 0.0

	match position_category:
		"공격수":  # Forward
			# 공격수 핵심 능력치: shooting, finishing, speed, acceleration, dribbling
			var forward_attrs = ["shooting", "finishing", "speed", "acceleration", "dribbling"]
			var sum = 0
			var count = 0
			for attr in forward_attrs:
				if attributes.has(attr):
					sum += attributes[attr]
					count += 1
			relevant_avg = float(sum) / max(1, count)

		"미드필더":  # Midfielder
			# 미드필더 핵심 능력치: passing, vision, technique, ball_control, teamwork
			var midfielder_attrs = ["passing", "vision", "technique", "ball_control", "teamwork"]
			var sum = 0
			var count = 0
			for attr in midfielder_attrs:
				if attributes.has(attr):
					sum += attributes[attr]
					count += 1
			relevant_avg = float(sum) / max(1, count)

		"수비수":  # Defender
			# 수비수 핵심 능력치: positioning, anticipation, strength, heading, work_rate
			var defender_attrs = ["positioning", "anticipation", "strength", "heading", "work_rate"]
			var sum = 0
			var count = 0
			for attr in defender_attrs:
				if attributes.has(attr):
					sum += attributes[attr]
					count += 1
			relevant_avg = float(sum) / max(1, count)

		_:
			relevant_avg = 50.0  # 기본값

	# OpenFootball 공식: (0.8 + (relevant_avg - 45.0) * 0.008).clamp(0.8, 1.2)
	var modifier = 0.8 + (relevant_avg - 45.0) * 0.008
	modifier = clampf(modifier, 0.8, 1.2)

	return modifier


## Godot 현재 방식과 OpenFootball 방식 비교 검증
static func validate_ca_calculation(character_data: Dictionary) -> Dictionary:
	if not character_data.has("detailed_attributes"):
		return {"error": "detailed_attributes not found", "is_valid": false}

	var attributes = character_data.detailed_attributes
	var position_category = character_data.basic_info.get("position_category", "공격수")

	# 1. Godot 현재 방식 (position_modifier = 1.0 고정)
	var godot_result = _calculate_ca_godot_style(attributes)

	# 2. OpenFootball 정확한 방식 (position_modifier 포함)
	var openfootball_result = calculate_ca_openfootball_accurate(attributes, position_category)

	# 3. 결과 비교
	var ca_difference = abs(godot_result.ca - openfootball_result.ca)
	var is_close = ca_difference <= 2  # 2점 오차 허용

	return {
		"is_valid": is_close,
		"ca_difference": ca_difference,
		"godot_ca": godot_result.ca,
		"openfootball_ca": openfootball_result.ca,
		"position_modifier": openfootball_result.position_modifier,
		"position_category": position_category,
		"recommendation": _get_ca_recommendation(godot_result, openfootball_result),
		"detailed_breakdown": {"godot": godot_result, "openfootball": openfootball_result}
	}


## Godot 현재 방식 CA 계산 (참고용)
static func _calculate_ca_godot_style(attributes: Dictionary) -> Dictionary:
	var technical_sum = 0
	var mental_sum = 0
	var physical_sum = 0
	var gk_sum = 0

	# 기존 방식과 동일한 계산
	var technical_attrs = [
		"dribbling",
		"passing",
		"shooting",
		"crossing",
		"first_touch",
		"ball_control",
		"technique",
		"heading",
		"finishing",
		"long_shots",
		"free_kicks",
		"penalties",
		"corners",
		"throw_ins"
	]

	for attr in technical_attrs:
		if attributes.has(attr):
			technical_sum += attributes[attr]

	var mental_attrs = [
		"decisions",
		"concentration",
		"leadership",
		"vision",
		"teamwork",
		"work_rate",
		"positioning",
		"anticipation",
		"composure",
		"bravery",
		"determination",
		"flair",
		"off_the_ball",
		"aggression"
	]

	for attr in mental_attrs:
		if attributes.has(attr):
			mental_sum += attributes[attr]

	var physical_attrs = [
		"speed", "stamina", "strength", "agility", "balance", "jumping", "natural_fitness", "acceleration"
	]

	for attr in physical_attrs:
		if attributes.has(attr):
			physical_sum += attributes[attr]

	var gk_attrs = ["reflexes", "handling", "aerial_ability", "command_of_area", "communication", "kicking"]

	for attr in gk_attrs:
		if attributes.has(attr):
			gk_sum += attributes[attr]

	var total_units = technical_sum + mental_sum + (physical_sum * 2) + gk_sum
	var base_ca = (total_units - 1000) / 20.0 if total_units >= 1000 else total_units / 40.0

	# Godot 방식: position_modifier = 1.0 고정
	var final_ca = int(base_ca)

	return {
		"ca": final_ca,
		"base_ca": int(base_ca),
		"position_modifier": 1.0,
		"total_units": total_units,
		"technical_sum": technical_sum,
		"mental_sum": mental_sum,
		"physical_sum": physical_sum,
		"gk_sum": gk_sum
	}


## CA 차이에 대한 권장사항
static func _get_ca_recommendation(godot_result: Dictionary, openfootball_result: Dictionary) -> String:
	var difference = abs(godot_result.ca - openfootball_result.ca)

	if difference <= 1:
		return "✅ 완벽! CA 계산이 정확합니다."
	elif difference <= 2:
		return "⚠️ 양호. 포지션 모디파이어로 인한 미미한 차이입니다."
	elif difference <= 5:
		return "❌ 주의. 포지션 모디파이어 검토가 필요합니다."
	else:
		return "🚨 오류. CA 계산 공식을 확인해주세요."


## 실시간 CA 미리보기 (UI에서 사용)
static func get_ca_preview(attributes: Dictionary, position_category: String) -> Dictionary:
	var result = calculate_ca_openfootball_accurate(attributes, position_category)

	return {
		"current_ca": result.ca,
		"position_modifier": result.position_modifier,
		"is_optimized": result.position_modifier >= 1.0,
		"optimization_tip": _get_optimization_tip(position_category, result.position_modifier)
	}


## 포지션 최적화 팁
static func _get_optimization_tip(position_category: String, modifier: float) -> String:
	if modifier >= 1.1:
		return "🌟 포지션에 최적화된 능력치입니다!"
	elif modifier >= 1.0:
		return "✅ 포지션에 적합한 능력치입니다."
	elif modifier >= 0.9:
		return "⚠️ 포지션 특화 능력치를 더 높여보세요."
	else:
		match position_category:
			"공격수":
				return "💡 팁: 슛팅, 피니싱, 스피드, 드리블을 높이면 CA가 증가합니다."
			"미드필더":
				return "💡 팁: 패싱, 비전, 테크닉, 볼 컨트롤을 높이면 CA가 증가합니다."
			"수비수":
				return "💡 팁: 위치선정, 예측력, 힘, 헤딩을 높이면 CA가 증가합니다."
			_:
				return "💡 포지션에 맞는 핵심 능력치를 높여보세요."
