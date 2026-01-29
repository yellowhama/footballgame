class_name AttributeConverter
extends RefCounted

## 포지션/속성 변환기
## Godot 캐릭터 생성 → OpenFootball 호환 형식 변환


## 한국어 포지션 → OpenFootball Position enum 변환
static func convert_position_to_openfootball(korean_category: String) -> String:
	match korean_category:
		"공격수":
			return "FW"
		"미드필더":
			return "MF"
		"수비수":
			return "DF"
		"골키퍼":
			return "GK"
		_:
			print("[AttributeConverter] Warning: Unknown position category: ", korean_category)
			return "FW"  # 기본값


## OpenFootball Position enum → 한국어 포지션 변환 (역변환)
static func convert_openfootball_to_korean(openfootball_position: String) -> String:
	match openfootball_position:
		"FW", "ST", "CF", "LW", "RW":
			return "공격수"
		"MF", "CAM", "CM", "CDM", "LM", "RM":
			return "미드필더"
		"DF", "CB", "LB", "RB", "LWB", "RWB":
			return "수비수"
		"GK":
			return "골키퍼"
		_:
			print("[AttributeConverter] Warning: Unknown OpenFootball position: ", openfootball_position)
			return "공격수"


## 포지션 카테고리별 세부 포지션 목록
static func get_detailed_positions(korean_category: String) -> Array:
	match korean_category:
		"공격수":
			return ["ST", "CF", "LW", "RW"]
		"미드필더":
			return ["CAM", "CM", "CDM", "LM", "RM"]
		"수비수":
			return ["CB", "LB", "RB", "LWB", "RWB"]
		"골키퍼":
			return ["GK"]
		_:
			return ["ST"]


## 포지션별 추천 세부 포지션
static func get_recommended_position(korean_category: String, style: String = "") -> String:
	match korean_category:
		"공격수":
			match style:
				"기술형", "창의형":
					return "CF"  # Centre Forward
				"속도형", "윙어형":
					return "LW"  # Left Winger (또는 RW)
				_:
					return "ST"  # Striker
		"미드필더":
			match style:
				"공격형", "창의형":
					return "CAM"  # Attacking Midfielder
				"수비형", "안정형":
					return "CDM"  # Defensive Midfielder
				_:
					return "CM"  # Central Midfielder
		"수비수":
			match style:
				"안정형", "리더형":
					return "CB"  # Centre Back
				"속도형", "공격형":
					return "LB"  # Left Back (또는 RB)
				_:
					return "CB"
		"골키퍼":
			return "GK"
		_:
			return "ST"


## 42개 능력치를 OpenFootball PlayerAttributes 형식으로 변환
static func convert_attributes_to_openfootball(character_data: Dictionary) -> Dictionary:
	if not character_data.has("detailed_attributes"):
		print("[AttributeConverter] Error: detailed_attributes not found")
		return {}

	var attrs = character_data.detailed_attributes

	# OpenFootball PlayerAttributes 구조에 정확히 맞춘 변환
	var openfootball_attrs = {
		# Technical attributes (14개) - OpenFootball 순서 그대로
		"corners": attrs.get("corners", 50),
		"crossing": attrs.get("crossing", 50),
		"dribbling": attrs.get("dribbling", 50),
		"finishing": attrs.get("finishing", 50),
		"first_touch": attrs.get("first_touch", 50),
		"free_kicks": attrs.get("free_kicks", 50),
		"heading": attrs.get("heading", 50),
		"long_shots": attrs.get("long_shots", 50),
		# Support both our legacy keys (throw_ins) and template keys (long_throws)
		"long_throws": attrs.get("long_throws", attrs.get("throw_ins", 50)),
		"marking": attrs.get("marking", 50),  # 기본값 또는 제공값 사용
		"passing": attrs.get("passing", 50),
		# Support both penalties and penalty_kicks
		"penalty_taking": attrs.get("penalty_kicks", attrs.get("penalties", 50)),
		"tackling": attrs.get("tackling", 50),  # 기본값 또는 제공값 사용
		"technique": attrs.get("technique", 50),
		# 추가 Technical attributes (우리 시스템에만 있음)
		"ball_control": attrs.get("ball_control", 50),
		"shooting": attrs.get("shooting", 50),
		# Mental attributes (14개) - OpenFootball 순서 그대로
		"aggression": attrs.get("aggression", 50),
		"anticipation": attrs.get("anticipation", 50),
		"bravery": attrs.get("bravery", 50),
		"composure": attrs.get("composure", 50),
		"concentration": attrs.get("concentration", 50),
		"decisions": attrs.get("decisions", 50),
		"determination": attrs.get("determination", 50),
		"flair": attrs.get("flair", 50),
		"leadership": attrs.get("leadership", 50),
		"off_the_ball": attrs.get("off_the_ball", 50),
		"positioning": attrs.get("positioning", 50),
		"teamwork": attrs.get("teamwork", 50),
		"vision": attrs.get("vision", 50),
		"work_rate": attrs.get("work_rate", 50),
		# Physical attributes (8개) - OpenFootball 순서 그대로
		"acceleration": attrs.get("acceleration", 50),
		"agility": attrs.get("agility", 50),
		"balance": attrs.get("balance", 50),
		"jumping": attrs.get("jumping", 50),
		"natural_fitness": attrs.get("natural_fitness", 50),
		"pace": attrs.get("pace", attrs.get("speed", 50)),  # speed/pace 동시 지원
		"stamina": attrs.get("stamina", 50),
		"strength": attrs.get("strength", 50),
		# GK attributes (6개)
		"aerial_ability": attrs.get("aerial_ability", attrs.get("aerial_reach", 50)),
		"command_of_area": attrs.get("command_of_area", 50),
		"communication": attrs.get("communication", 50),
		"handling": attrs.get("handling", 50),
		"kicking": attrs.get("kicking", 50),
		"reflexes": attrs.get("reflexes", 50)
	}

	return openfootball_attrs


## 캐릭터 생성 데이터를 완전한 OpenFootball 플레이어 생성 요청으로 변환
static func convert_character_to_openfootball_request(character_data: Dictionary) -> Dictionary:
	var basic_info = character_data.get("basic_info", {})
	var position_category = basic_info.get("position_category", "공격수")
	var openfootball_position = convert_position_to_openfootball(position_category)
	var openfootball_attrs = convert_attributes_to_openfootball(character_data)

	# CA 계산 (최종 완성된 CA 사용)
	var final_ca = character_data.get("final_ca", 80)

	# OpenFootball PlayerCreationRequest 형식
	var request = {
		"schema_version": "v1",
		"name": basic_info.get("name", "New Player"),
		"position": openfootball_position,
		"age_months": 18.0 * 12,  # 18세로 고정
		"current_ability": final_ca,
		"potential_ability": final_ca + 20,  # CA + 20으로 설정
		"attributes": openfootball_attrs,
		# 추가 메타데이터
		"metadata":
		{
			"created_by": "GodotCharacterCreation",
			"korean_position": position_category,
			"selected_category": character_data.get("selected_category", ""),
			"selected_specialties": character_data.get("selected_specialties", []),
			"jersey_number": basic_info.get("number", 7),
			"creation_timestamp": Time.get_unix_time_from_system()
		}
	}

	return request


## OpenFootball 응답을 Godot 형식으로 역변환
static func convert_openfootball_response_to_godot(openfootball_player: Dictionary) -> Dictionary:
	var attrs = openfootball_player.get("attributes", {})

	# Godot detailed_attributes 형식으로 역변환
	var godot_attrs = {
		# Technical
		"corners": attrs.get("corners", 50),
		"crossing": attrs.get("crossing", 50),
		"dribbling": attrs.get("dribbling", 50),
		"finishing": attrs.get("finishing", 50),
		"first_touch": attrs.get("first_touch", 50),
		"free_kicks": attrs.get("free_kicks", 50),
		"heading": attrs.get("heading", 50),
		"long_shots": attrs.get("long_shots", 50),
		"passing": attrs.get("passing", 50),
		"shooting": attrs.get("shooting", 50),
		"ball_control": attrs.get("ball_control", 50),
		"technique": attrs.get("technique", 50),
		# Preserve synonyms to make validation round-trip clean
		"penalties": attrs.get("penalty_taking", 50),
		"penalty_kicks": attrs.get("penalty_taking", 50),
		"throw_ins": attrs.get("long_throws", 50),
		"long_throws": attrs.get("long_throws", 50),
		# Include OpenFootball-only technical keys to preserve round-trip
		"marking": attrs.get("marking", 50),
		"tackling": attrs.get("tackling", 50),
		# Mental
		"aggression": attrs.get("aggression", 50),
		"anticipation": attrs.get("anticipation", 50),
		"bravery": attrs.get("bravery", 50),
		"composure": attrs.get("composure", 50),
		"concentration": attrs.get("concentration", 50),
		"decisions": attrs.get("decisions", 50),
		"determination": attrs.get("determination", 50),
		"flair": attrs.get("flair", 50),
		"leadership": attrs.get("leadership", 50),
		"off_the_ball": attrs.get("off_the_ball", 50),
		"positioning": attrs.get("positioning", 50),
		"teamwork": attrs.get("teamwork", 50),
		"vision": attrs.get("vision", 50),
		"work_rate": attrs.get("work_rate", 50),
		# Physical
		"acceleration": attrs.get("acceleration", 50),
		"agility": attrs.get("agility", 50),
		"balance": attrs.get("balance", 50),
		"jumping": attrs.get("jumping", 50),
		"natural_fitness": attrs.get("natural_fitness", 50),
		"speed": attrs.get("pace", 50),  # pace → speed 역매핑
		"pace": attrs.get("pace", 50),  # 동의어 유지
		"stamina": attrs.get("stamina", 50),
		"strength": attrs.get("strength", 50),
		# GK
		"aerial_ability": attrs.get("aerial_ability", 50),
		"aerial_reach": attrs.get("aerial_ability", 50),  # 템플릿 키 보존
		"command_of_area": attrs.get("command_of_area", 50),
		"communication": attrs.get("communication", 50),
		"eccentricity": attrs.get("eccentricity", 50),  # 템플릿 키 보존 (없으면 기본)
		"handling": attrs.get("handling", 50),
		"kicking": attrs.get("kicking", 50),
		"reflexes": attrs.get("reflexes", 50)
	}

	var godot_player = {
		"basic_info":
		{
			"name": openfootball_player.get("name", "Player"),
			"position_category": convert_openfootball_to_korean(openfootball_player.get("position", "FW")),
			"number": openfootball_player.get("metadata", {}).get("jersey_number", 7)
		},
		"detailed_attributes": godot_attrs,
		"final_ca": openfootball_player.get("current_ability", 80),
		"openfootball_id": openfootball_player.get("id", ""),
		"openfootball_data": openfootball_player
	}

	return godot_player


## 속성 매핑 검증 (개발/디버그용)
static func validate_attribute_mapping(character_data: Dictionary) -> Dictionary:
	var conversion_result = convert_attributes_to_openfootball(character_data)
	var reverse_result = convert_openfootball_response_to_godot({"attributes": conversion_result})

	var original_attrs = character_data.get("detailed_attributes", {})
	var final_attrs = reverse_result.get("detailed_attributes", {})

	var differences = []
	# Known synonym pairs that should be treated as equivalent in validation
	var synonyms := {"penalty_kicks": "penalties", "long_throws": "throw_ins", "pace": "speed"}
	# Attributes excluded from strict round-trip validation (not exported forward to OF)
	var exclude_from_rt := {"eccentricity": true}
	for attr in original_attrs:
		var original_val = original_attrs[attr]
		if exclude_from_rt.has(attr):
			# Skip attributes that are not carried through OF payloads
			continue
		var final_val = final_attrs.get(attr, null)
		if final_val == null and synonyms.has(attr):
			final_val = final_attrs.get(synonyms[attr], null)
		if final_val == null:
			final_val = -1
		# 타입 안전성 검사: 둘 다 숫자인지 확인
		if typeof(original_val) == TYPE_INT or typeof(original_val) == TYPE_FLOAT:
			if typeof(final_val) == TYPE_INT or typeof(final_val) == TYPE_FLOAT:
				if original_val != final_val:
					differences.append(
						{
							"attribute": attr,
							"original": original_val,
							"final": final_val,
							"difference": final_val - original_val
						}
					)
			else:
				# 타입 불일치 - 경고만 출력
				print(
					"[AttributeConverter] Warning: Type mismatch for attribute ",
					attr,
					": original=",
					typeof(original_val),
					" final=",
					typeof(final_val)
				)
		else:
			# original_val이 숫자가 아님 - 경고만 출력
			print("[AttributeConverter] Warning: Original value for ", attr, " is not numeric: ", typeof(original_val))

	return {
		"is_perfect": differences.size() == 0,
		"differences": differences,
		"total_attributes": original_attrs.size(),
		"preserved_attributes": original_attrs.size() - differences.size()
	}


## 포지션 호환성 검증
static func validate_position_conversion(korean_position: String) -> Dictionary:
	var openfootball_pos = convert_position_to_openfootball(korean_position)
	var back_to_korean = convert_openfootball_to_korean(openfootball_pos)

	return {
		"is_valid": korean_position == back_to_korean,
		"original": korean_position,
		"openfootball": openfootball_pos,
		"back_converted": back_to_korean,
		"detailed_positions": get_detailed_positions(korean_position)
	}


## 전체 변환 테스트 (디버그용)
static func test_full_conversion(character_data: Dictionary) -> Dictionary:
	print("[AttributeConverter] Testing full conversion...")

	# 1. 포지션 변환 테스트
	var position_test = validate_position_conversion(character_data.basic_info.get("position_category", "공격수"))

	# 2. 속성 변환 테스트
	var attribute_test = validate_attribute_mapping(character_data)

	# 3. 전체 요청 생성 테스트
	var openfootball_request = convert_character_to_openfootball_request(character_data)

	return {
		"position_conversion": position_test,
		"attribute_mapping": attribute_test,
		"openfootball_request": openfootball_request,
		"success": position_test.is_valid and attribute_test.is_perfect
	}
