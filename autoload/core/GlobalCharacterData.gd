extends Node
## ============================================================================
## GlobalCharacterData - Character Creation & Persistence
## ============================================================================
##
## PURPOSE: Store character creation data and persist to save files
##
## BOUNDARY CONTRACT:
## - This is the GDScript save/creation representation
## - Not sent directly to Rust; MatchPlayer is created from this data first
## - See docs/spec/03_data_schemas.md "GDScript ↔ Rust Boundary Contract"
##
## RELATED:
## - Rust: crates/of_core/src/models/player.rs (engine simulation)
## - GDScript Match: scripts/core/MatchPlayer.gd (match-time entity)
## ============================================================================

# 전역 캐릭터 생성 데이터
var character_data: Dictionary = {}

# Signal for data changes
signal data_changed(category: String, attribute: String, new_value: int)
signal character_data_changed(old_data: Dictionary, new_data: Dictionary)


func _ready():
	print("[GlobalCharacterData] Initialized")


func clear_data():
	"""데이터 초기화"""
	character_data = {}
	print("[GlobalCharacterData] Data cleared")


func set_appearance(appearance: Dictionary):
	"""외형 데이터 설정"""
	character_data["appearance"] = appearance
	print("[GlobalCharacterData] Appearance set: %s" % appearance)


func set_position(position: String):
	"""포지션 설정"""
	character_data["position"] = position
	print("[GlobalCharacterData] Position set: %s" % position)


func set_abilities(strengths: Array, weaknesses: Array):
	"""능력치 설정"""
	character_data["strengths"] = strengths
	character_data["weaknesses"] = weaknesses
	print("[GlobalCharacterData] Abilities set - Strengths: %s, Weaknesses: %s" % [strengths, weaknesses])


func get_final_character() -> Dictionary:
	"""최종 캐릭터 데이터 반환"""
	return character_data.get("final_character", {})


func set_character_data(data: Dictionary):
	"""전체 캐릭터 데이터 설정 (시그널 발생)"""
	var old_data = character_data.duplicate(true)
	character_data = data
	character_data_changed.emit(old_data, character_data)
	print("[GlobalCharacterData] Character data set with %d keys" % data.size())


func increase_attribute(attribute_name: String, amount: int):
	"""능력치 증가 (훈련용)"""
	if not character_data.has("attributes"):
		character_data["attributes"] = {}

	if not character_data.attributes.has(attribute_name):
		character_data.attributes[attribute_name] = 50  # 기본값

	var old_value = character_data.attributes[attribute_name]
	var new_value = clampi(old_value + amount, 0, 100)
	character_data.attributes[attribute_name] = new_value

	print("[GlobalCharacterData] %s: %d → %d (+%d)" % [attribute_name, old_value, new_value, amount])

	# Emit signal for UI updates
	data_changed.emit("attributes", attribute_name, new_value)


func get_attribute(attribute_name: String) -> int:
	"""능력치 가져오기"""
	if not character_data.has("attributes"):
		return 50  # 기본값

	return character_data.attributes.get(attribute_name, 50)


func save_to_dict() -> Dictionary:
	"""저장용 데이터 반환 (Phase 9 + Phase 20)"""
	var save_data = character_data.duplicate(true)

	# ✅ NEW: Add growth_profile and personality (Phase 20)
	if PlayerData:
		save_data["growth_profile"] = {
			"specialization": _get_player_specializations(PlayerData.position), "growth_rate": 1.0, "injury_prone": 0.1
		}
		save_data["personality"] = PlayerData.get_personality_dict()
		save_data["personality_archetype"] = PlayerData.personality_archetype

	return save_data


func load_from_dict(data: Dictionary):
	"""로드용 데이터 복원 (Phase 9 + Phase 20 migration)"""
	character_data = data.duplicate(true)

	# ✅ NEW: Migrate old saves (backward compatibility) (Phase 20)
	if not data.has("growth_profile"):
		# Generate default growth_profile for old saves
		character_data["growth_profile"] = {"specialization": [], "growth_rate": 1.0, "injury_prone": 0.1}
		print("[GlobalCharacterData] Migrated old save: added default growth_profile")

	if not data.has("personality"):
		# Use PlayerData's personality if available
		if PlayerData:
			character_data["personality"] = PlayerData.get_personality_dict()
		else:
			# Fallback: default personality
			character_data["personality"] = {
				"adaptability": 50,
				"ambition": 50,
				"determination": 50,
				"discipline": 50,
				"loyalty": 50,
				"pressure": 50,
				"professionalism": 50,
				"temperament": 50
			}
		print("[GlobalCharacterData] Migrated old save: added default personality")

	print("[GlobalCharacterData] Data loaded from save file")

	# Emit signals for all attributes to update UI
	if character_data.has("attributes"):
		for attr_name in character_data.attributes:
			data_changed.emit("attributes", attr_name, character_data.attributes[attr_name])

	print("[GlobalCharacterData] Loaded %d keys (Phase 20 migration OK)" % character_data.size())


# ===============================================================================
# Phase 20: Helper Functions
# ===============================================================================


func _get_player_specializations(position: String) -> Array:
	"""Infer specializations from player position (Phase 20)"""
	match position:
		"ST", "CF":
			return ["FINISHING", "PACE"]
		"CM":
			return ["PASSING", "VISION"]
		"CB":
			return ["TACKLING", "MARKING"]
		_:
			return []
