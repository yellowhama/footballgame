extends Node

# Enhanced Tactical Management System - v2.0
# Integrates Rust GDExtension tactical engine with existing CoachSystem
# Implements manager authority and real-time formation effectiveness

signal formation_changed(new_formation: String, effectiveness: float)
signal tactical_effectiveness_updated(effectiveness_data: Dictionary)
signal manager_authority_confirmed(action: String, allowed: bool)
signal formation_selection_feedback(formation_id: String, feedback_data: Dictionary)

# Manager Authority Levels
enum ManagerAuthority { NONE = 0, LIMITED = 1, FULL = 2 }  # 권한 없음 (선수는 전술 변경 불가)  # 제한적 권한 (일부 전술만 가능)  # 전체 권한 (모든 전술 가능)

# Formation Change Results
enum FormationChangeResult {
	SUCCESS = 0, INSUFFICIENT_AUTHORITY = 1, UNKNOWN_FORMATION = 2, INADEQUATE_SQUAD = 3, COACH_KNOWLEDGE_REQUIRED = 4
}

# Current manager authority level
var manager_authority: ManagerAuthority = ManagerAuthority.FULL

# Rust GDExtension instances
var formation_manager: FormationManager
var tactical_engine: TacticalEngine

# Integration with existing systems
var coach_system: Node = null
var condition_system: Node = null
var player_manager: Node = null

# Current tactical state
var current_formation_id: String = "442_standard"
var current_effectiveness: Dictionary = {}
var formation_adaptation_rates: Dictionary = {}

# Formation change history for learning
var formation_history: Array = []


func _ready():
	print("[TacticalManager] Initializing enhanced tactical management system")

	# Initialize Rust GDExtension components
	_initialize_rust_components()

	# Connect to existing systems
	_connect_to_existing_systems()

	# Load current tactics from CoachSystem
	_sync_with_coach_system()

	print("[TacticalManager] Tactical system ready with manager authority: %s" % _get_authority_name())


func _initialize_rust_components():
	"""Initialize Rust GDExtension tactical components"""
	formation_manager = FormationManager.new()
	tactical_engine = TacticalEngine.new()

	# Load default formations
	_load_default_formations()

	print("[TacticalManager] Rust components initialized")


func _load_default_formations():
	"""Load default formations into FormationManager"""
	# Note: In real implementation, these would be loaded from JSON files
	# For now, we rely on Rust's built-in formation factory
	var available_formations = formation_manager.get_available_formations()
	print("[TacticalManager] Available formations: %s" % str(available_formations))


func _connect_to_existing_systems():
	"""Connect to existing Godot systems"""
	# Find CoachSystem
	coach_system = get_node_or_null("/root/CoachSystem")
	if not coach_system:
		coach_system = get_tree().get_first_node_in_group("coach_system")

	# Find ConditionSystem
	condition_system = get_node_or_null("/root/ConditionSystem")
	if not condition_system:
		condition_system = get_tree().get_first_node_in_group("condition_system")

	# Find PlayerManager
	player_manager = get_node_or_null("/root/PlayerManager")
	if not player_manager:
		player_manager = get_tree().get_first_node_in_group("player_manager")

	print(
		(
			"[TacticalManager] Connected to existing systems - Coach: %s, Condition: %s, Player: %s"
			% [coach_system != null, condition_system != null, player_manager != null]
		)
	)


func _sync_with_coach_system():
	"""Sync current tactics with existing CoachSystem"""
	if not coach_system:
		print("[TacticalManager] Warning: CoachSystem not found, using default formation")
		return

	# Get current tactics from CoachSystem
	if coach_system.has_method("get_current_tactics"):
		var coach_tactics = coach_system.get_current_tactics()
		current_formation_id = _convert_coach_tactics_to_formation_id(coach_tactics)
		print("[TacticalManager] Synced with CoachSystem tactics: %s -> %s" % [coach_tactics, current_formation_id])


func _convert_coach_tactics_to_formation_id(coach_tactics: String) -> String:
	"""Convert CoachSystem tactics format to FormationManager format"""
	match coach_tactics:
		"4-4-2":
			return "442_standard"
		"4-3-3":
			return "433_standard"
		"3-5-2":
			return "352_standard"
		_:
			return "442_standard"  # Default fallback


# ==============================================================================
# FR-001: Manager Authority System - Only manager can select formation
# ==============================================================================


func request_formation_change(formation_id: String, requester_type: String = "manager") -> Dictionary:
	"""Request formation change with authority check (FR-001)"""
	print("[TacticalManager] Formation change requested: %s by %s" % [formation_id, requester_type])

	# Authority check
	var authority_result = _check_manager_authority(requester_type, "formation_change")
	if not authority_result.allowed:
		manager_authority_confirmed.emit("formation_change", false)
		return {
			"success": false,
			"result": FormationChangeResult.INSUFFICIENT_AUTHORITY,
			"message": authority_result.message
		}

	manager_authority_confirmed.emit("formation_change", true)

	# Proceed with formation change
	return _execute_formation_change(formation_id)


func _check_manager_authority(requester_type: String, action: String) -> Dictionary:
	"""Check if requester has authority for tactical actions (FR-001, FR-002)"""
	match requester_type:
		"manager":
			# Manager always has full authority
			return {"allowed": true, "authority_level": manager_authority, "message": "매니저 권한으로 실행됩니다."}
		"player":
			# FR-002: Players cannot override tactical instructions
			return {"allowed": false, "authority_level": ManagerAuthority.NONE, "message": "선수는 전술 지시를 변경할 수 없습니다."}
		"coach":
			# Coaches have limited authority based on relationship
			var coach_authority = _calculate_coach_authority()
			return {
				"allowed": coach_authority >= ManagerAuthority.LIMITED,
				"authority_level": coach_authority,
				"message": "코치 권한으로 제한적 실행이 가능합니다." if coach_authority >= ManagerAuthority.LIMITED else "코치 권한이 부족합니다."
			}
		_:
			return {"allowed": false, "authority_level": ManagerAuthority.NONE, "message": "인식되지 않은 요청자입니다."}


func _calculate_coach_authority() -> ManagerAuthority:
	"""Calculate coach authority based on relationship with manager"""
	if not coach_system:
		return ManagerAuthority.NONE

	if coach_system.has_method("get_coach_info"):
		var head_coach_info = coach_system.get_coach_info(coach_system.CoachType.HEAD_COACH)
		var relationship = head_coach_info.get("relationship", 0.0)

		if relationship >= 0.8:
			return ManagerAuthority.LIMITED
		else:
			return ManagerAuthority.NONE

	return ManagerAuthority.NONE


func _execute_formation_change(formation_id: String) -> Dictionary:
	"""Execute the actual formation change"""
	# Check if formation exists
	var available_formations = formation_manager.get_available_formations()
	if formation_id not in available_formations:
		return {
			"success": false,
			"result": FormationChangeResult.UNKNOWN_FORMATION,
			"message": "알 수 없는 포메이션입니다: %s" % formation_id
		}

	# Check coach knowledge requirement
	if not _check_coach_knowledge_requirement(formation_id):
		return {
			"success": false,
			"result": FormationChangeResult.COACH_KNOWLEDGE_REQUIRED,
			"message": "이 포메이션을 사용하려면 코치의 전술 지식이 필요합니다."
		}

	# Check squad suitability
	var squad_data = _get_current_squad_data()
	var suitability = formation_manager.calculate_squad_suitability(formation_id, squad_data)

	if suitability < 0.3:  # Minimum threshold
		return {
			"success": false,
			"result": FormationChangeResult.INADEQUATE_SQUAD,
			"message": "현재 스쿼드로는 이 포메이션을 효과적으로 운용할 수 없습니다. (적합도: %.1f%%)" % (suitability * 100)
		}

	# Execute formation change
	var old_formation = current_formation_id
	current_formation_id = formation_id

	# Update CoachSystem
	_update_coach_system_tactics(formation_id)

	# Calculate new effectiveness
	var effectiveness_data = _calculate_formation_effectiveness()
	current_effectiveness = effectiveness_data

	# Record formation change
	formation_history.append(
		{
			"timestamp": Time.get_unix_time_from_system(),
			"old_formation": old_formation,
			"new_formation": formation_id,
			"effectiveness": effectiveness_data.get("overall_effectiveness", 0.0),
			"squad_suitability": suitability
		}
	)

	# Emit signals
	formation_changed.emit(formation_id, effectiveness_data.get("overall_effectiveness", 0.0))
	tactical_effectiveness_updated.emit(effectiveness_data)

	print(
		(
			"[TacticalManager] Formation changed successfully: %s -> %s (effectiveness: %.2f)"
			% [old_formation, formation_id, effectiveness_data.get("overall_effectiveness", 0.0)]
		)
	)

	return {
		"success": true,
		"result": FormationChangeResult.SUCCESS,
		"message": "포메이션이 성공적으로 변경되었습니다.",
		"effectiveness": effectiveness_data,
		"squad_suitability": suitability
	}


func _check_coach_knowledge_requirement(formation_id: String) -> bool:
	"""Check if coach has knowledge of the requested formation"""
	if not coach_system:
		return true  # No coach system means no restrictions

	if coach_system.has_method("get_tactical_knowledge"):
		var tactical_knowledge = coach_system.get_tactical_knowledge()
		var coach_formation_name = _convert_formation_id_to_coach_tactics(formation_id)
		return tactical_knowledge.get(coach_formation_name, 0) > 0

	return true


func _convert_formation_id_to_coach_tactics(formation_id: String) -> String:
	"""Convert FormationManager format back to CoachSystem format"""
	match formation_id:
		"442_standard":
			return "4-4-2"
		"433_standard":
			return "4-3-3"
		"352_standard":
			return "3-5-2"
		_:
			return "4-4-2"


func _update_coach_system_tactics(formation_id: String):
	"""Update CoachSystem with new tactics"""
	if coach_system and coach_system.has_method("change_tactics"):
		var coach_tactics = _convert_formation_id_to_coach_tactics(formation_id)
		coach_system.change_tactics(coach_tactics)
		print("[TacticalManager] Updated CoachSystem tactics to: %s" % coach_tactics)


func _get_current_squad_data() -> String:
	"""Get current squad data in JSON format for Rust backend"""
	# TODO: Integrate with actual PlayerManager
	# For now, return mock data
	var mock_squad = {
		"GK1": {"goalkeeping": 85, "reflexes": 80, "positioning": 78},
		"CB1": {"defending": 82, "heading": 85, "positioning": 80, "marking": 79},
		"CB2": {"defending": 78, "heading": 82, "positioning": 77, "marking": 81},
		"LB1": {"defending": 75, "pace": 78, "crossing": 65, "stamina": 80},
		"RB1": {"defending": 76, "pace": 80, "crossing": 70, "stamina": 82},
		"CM1": {"passing": 85, "vision": 80, "stamina": 83, "first_touch": 78},
		"CM2": {"passing": 80, "vision": 75, "stamina": 85, "first_touch": 76},
		"LW1": {"pace": 88, "dribbling": 85, "crossing": 82, "acceleration": 90},
		"RW1": {"pace": 86, "dribbling": 83, "crossing": 80, "acceleration": 88},
		"ST1": {"finishing": 87, "positioning": 85, "first_touch": 80, "composure": 82},
		"ST2": {"finishing": 84, "positioning": 83, "first_touch": 78, "composure": 80}
	}

	return JSON.stringify(mock_squad)


func _calculate_formation_effectiveness() -> Dictionary:
	"""Calculate real-time formation effectiveness using Rust backend"""
	if not tactical_engine:
		return {"overall_effectiveness": 0.5}

	# Get match context
	var match_context = _get_current_match_context()

	# Get player stats
	var player_stats = _get_current_player_stats()

	# Calculate effectiveness against default opponent
	var opponent_formation = "442_standard"  # Default opponent

	var effectiveness_dict = tactical_engine.calculate_formation_effectiveness(
		current_formation_id, opponent_formation, player_stats, match_context
	)

	return effectiveness_dict


func _get_current_match_context() -> String:
	"""Get current match context in JSON format"""
	var weather = "Clear"
	var pitch = "Good"

	if condition_system:
		# TODO: Integrate with actual weather/pitch systems
		pass

	var context = {
		"weather": weather,
		"pitch_condition": pitch,
		"match_importance": 0.5,
		"crowd_support": 0.5,
		"referee_strictness": 0.5
	}

	return JSON.stringify(context)


func _get_current_player_stats() -> String:
	"""Get current player stats in JSON format"""
	# TODO: Integrate with actual PlayerManager
	# For now, return mock data
	var mock_players = []
	for i in range(11):
		mock_players.append(
			{
				"id": "P%d" % i,
				"attributes":
				{
					"passing": 75 + randi() % 20,
					"pace": 70 + randi() % 25,
					"defending": 60 + randi() % 30,
					"shooting": 65 + randi() % 25
				},
				"tactical_understanding": 70 + randi() % 25,
				"current_condition": 0.8 + randf() * 0.2,
				"fatigue": randf() * 0.3,
				"morale": 0.7 + randf() * 0.3
			}
		)

	return JSON.stringify(mock_players)


# ==============================================================================
# Public API Methods
# ==============================================================================


func get_current_formation() -> String:
	"""Get current formation ID"""
	return current_formation_id


func get_formation_effectiveness() -> Dictionary:
	"""Get current formation effectiveness data"""
	return current_effectiveness


func get_available_formations() -> Array:
	"""Get list of available formations"""
	if formation_manager:
		return formation_manager.get_available_formations()
	return []


func get_formation_suitability(formation_id: String) -> float:
	"""Get squad suitability for a specific formation"""
	if not formation_manager:
		return 0.0

	var squad_data = _get_current_squad_data()
	return formation_manager.calculate_squad_suitability(formation_id, squad_data)


func get_manager_authority_level() -> ManagerAuthority:
	"""Get current manager authority level"""
	return manager_authority


func set_manager_authority_level(new_authority: ManagerAuthority):
	"""Set manager authority level"""
	manager_authority = new_authority
	print("[TacticalManager] Manager authority updated to: %s" % _get_authority_name())


func _get_authority_name() -> String:
	"""Get human-readable authority name"""
	match manager_authority:
		ManagerAuthority.NONE:
			return "권한 없음"
		ManagerAuthority.LIMITED:
			return "제한적 권한"
		ManagerAuthority.FULL:
			return "전체 권한"
		_:
			return "알 수 없음"


func get_formation_history(limit: int = 10) -> Array:
	"""Get recent formation change history"""
	var start_index = max(0, formation_history.size() - limit)
	return formation_history.slice(start_index)


# ==============================================================================
# Testing Methods
# ==============================================================================


func test_tactical_system():
	"""Test the tactical management system"""
	print("=== TacticalManager Test ===")

	# Test authority system
	print("1. Testing Manager Authority...")
	var manager_result = request_formation_change("433_standard", "manager")
	print("Manager formation change: %s" % str(manager_result))

	var player_result = request_formation_change("433_standard", "player")
	print("Player formation change: %s" % str(player_result))

	# Test formation effectiveness
	print("2. Testing Formation Effectiveness...")
	var effectiveness = get_formation_effectiveness()
	print("Current effectiveness: %s" % str(effectiveness))

	# Test available formations
	print("3. Testing Available Formations...")
	var formations = get_available_formations()
	print("Available formations: %s" % str(formations))

	# Test squad suitability
	print("4. Testing Squad Suitability...")
	for formation in formations:
		var suitability = get_formation_suitability(formation)
		print("Formation %s suitability: %.2f" % [formation, suitability])

	print("✅ TacticalManager test completed")
