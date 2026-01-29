class_name TrainingBridge
extends RefCounted
## ============================================================================
## TrainingBridge - Training System API
## ============================================================================
##
## PURPOSE: Bridge for player training execution via Rust engine
##
## EXTRACTED FROM: FootballRustEngine.gd (ST-006 God Class refactoring)
##
## RESPONSIBILITIES:
## - Execute training with Rust engine
## - Build and normalize training payloads
## - Synthesize attributes from CA
## - Handle player/manager data validation
##
## DEPENDENCIES:
## - _rust_simulator: GDExtension Rust object
##
## USAGE:
##   var bridge := TrainingBridge.new()
##   bridge.initialize(rust_simulator)
##   var result := bridge.execute_training_json(request, player, manager)
## ============================================================================

var _rust_simulator: Object = null
var _is_ready: bool = false


func initialize(rust_simulator: Object) -> void:
	"""Initialize TrainingBridge with Rust simulator reference"""
	_rust_simulator = rust_simulator
	_is_ready = rust_simulator != null


# =============================================================================
# Public API
# =============================================================================

func execute_training_json(
	training_request: Dictionary, player_data: Dictionary, manager_data: Dictionary
) -> Dictionary:
	"""Execute training via Rust engine
	@param training_request: Dictionary containing training request
	@param player_data: Dictionary containing player data
	@param manager_data: Dictionary containing training manager data
	@return: Dictionary with training result
	"""
	if not _is_ready:
		return {
			"error": true,
			"success": false,
			"message": "Engine not ready",
			"error_code": "ENGINE_NOT_READY"
		}

	# Make copies to avoid modifying originals
	var req := training_request.duplicate(true)
	var player := player_data.duplicate(true)
	var manager := manager_data.duplicate(true)

	# Ensure schema_version is present (defensive)
	if not req.has("schema_version"):
		req["schema_version"] = 1

	# Ensure request_type block is present (defensive)
	if not req.has("request_type"):
		var t := str(req.get("training_type", "technical")).to_lower()
		var target := t
		match t:
			"physical":
				target = "power"
			"speed":
				target = "pace"
			"very_light", "light", "moderate", "high", "very_high":
				target = "technical"
			_:
				pass
		var intensity := str(req.get("intensity", "normal"))
		req["request_type"] = {"type": "ExecutePersonalTraining", "target": target, "intensity": intensity}

	# Ensure player_id is present (defensive)
	if not req.has("player_id"):
		var pid := "player_main"
		if player.has("player_id"):
			pid = str(player.get("player_id"))
		elif player.has("uid"):
			pid = str(player.get("uid"))
		elif player.has("name"):
			pid = str(player.get("name")).replace(" ", "_")
		req["player_id"] = pid

	# Ensure player payload contains an 'id' field (defensive)
	if not player.has("id"):
		var pid2 := str(req.get("player_id", ""))
		if pid2 == "":
			if player.has("uid"):
				pid2 = str(player.get("uid"))
			elif player.has("name"):
				pid2 = str(player.get("name")).replace(" ", "_")
			else:
				pid2 = "player_main"
		player["id"] = pid2

	# Ensure a name exists in player payload (Rust schema requires it)
	if not player.has("name"):
		if player.has("display_name"):
			player["name"] = str(player.get("display_name"))
		elif player.has("id"):
			player["name"] = str(player.get("id"))
		elif player.has("player_id"):
			player["name"] = str(player.get("player_id"))
		else:
			player["name"] = "Player"

	# Ensure CA/PA are present for consistency with core schema
	if not player.has("ca") and player.has("overall"):
		player["ca"] = int(player.get("overall", 50))
	if not player.has("ca"):
		var ca_guess := 50
		if player.has("base_ca"):
			ca_guess = int(player.get("base_ca"))
		elif player.has("OpenFootball CA"):
			ca_guess = int(player.get("OpenFootball CA"))
		player["ca"] = int(ca_guess)
	if not player.has("pa"):
		player["pa"] = int(player.get("ca", 50))

	# Ensure/normalize position (Rust training schema requires specific codes)
	var pos := "M"
	if player.has("position"):
		pos = str(player.get("position"))
	elif player.has("position_category"):
		var cat := str(player.get("position_category")).to_lower()
		match cat:
			"골키퍼", "gk", "goalkeeper":
				pos = "GK"
			"수비수", "df", "defender":
				pos = "CB"
			"미드필더", "mf", "midfielder":
				pos = "CM"
			"공격수", "fw", "attacker", "forward":
				pos = "ST"
			_:
				pos = "M"
	player["position"] = _normalize_position_code(pos)

	# Ensure age_months present (Rust schema requires it)
	if not player.has("age_months"):
		var months := 0
		if player.has("age"):
			months = int(player.get("age", 16)) * 12
		else:
			months = 16 * 12
		player["age_months"] = months

	# Ensure seed is present (defensive)
	if not req.has("seed"):
		req["seed"] = int(Time.get_ticks_msec())

	# Normalize/construct manager payload to match Rust TrainingManager schema
	manager = _build_manager_payload(player, manager)

	# Ensure attributes map exists
	if not player.has("attributes") or typeof(player.get("attributes")) != TYPE_DICTIONARY:
		var pos2 := str(player.get("position", "M"))
		var attrs_synth := _synthesize_attributes_from_ca(int(player.get("ca", 50)), pos2)
		player["attributes"] = _coerce_map_to_ints(attrs_synth)

	# Ensure detailed_stats exist (Rust training requires this block)
	if not player.has("detailed_stats") or typeof(player.get("detailed_stats")) != TYPE_DICTIONARY:
		if player.has("attributes") and typeof(player.get("attributes")) == TYPE_DICTIONARY:
			player["detailed_stats"] = _coerce_map_to_ints(player.get("attributes"))
		else:
			player["detailed_stats"] = _coerce_map_to_ints(
				_synthesize_attributes_from_ca(int(player.get("ca", 50)), str(player.get("position", "M")))
			)

	# Convert dictionaries to JSON strings
	var request_json = JSON.stringify(req)
	var player_json = JSON.stringify(player)
	var manager_json = JSON.stringify(manager)

	if request_json == "" or player_json == "" or manager_json == "":
		return {
			"error": true,
			"success": false,
			"message": "Failed to serialize training data",
			"error_code": "SERIALIZATION_ERROR"
		}

	if not _rust_simulator:
		return {
			"error": true,
			"success": false,
			"message": "Rust simulator not initialized",
			"error_code": "RUST_NOT_INITIALIZED"
		}

	if not _rust_simulator.has_method("execute_training_json"):
		return {
			"error": true,
			"success": false,
			"message": "execute_training_json method not found in Rust simulator",
			"error_code": "METHOD_NOT_FOUND"
		}

	# Call Rust training API
	print("[TrainingBridge] Calling Rust execute_training_json...")
	var response_json = _rust_simulator.execute_training_json(request_json, player_json, manager_json)

	if response_json == "":
		return {
			"error": true,
			"success": false,
			"message": "Empty response from Rust training engine",
			"error_code": "EMPTY_RESPONSE"
		}

	# Parse JSON response
	var json_parser = JSON.new()
	var parse_result = json_parser.parse(response_json)

	if parse_result != OK:
		return {
			"error": true,
			"success": false,
			"message": "Failed to parse training response: " + json_parser.get_error_message(),
			"error_code": "PARSE_ERROR"
		}

	return json_parser.data


# =============================================================================
# Private Helpers
# =============================================================================

func _build_manager_payload(player_data: Dictionary, manager_data_in: Variant) -> Dictionary:
	"""Build TrainingManager JSON compatible with Rust schema"""
	var md: Dictionary = {}
	if typeof(manager_data_in) == TYPE_DICTIONARY:
		md = (manager_data_in as Dictionary).duplicate(true)

	var stamina_current := clampi(int(player_data.get("stamina", md.get("stamina", 100))), 0, 100)
	var condition_value := clampi(int(player_data.get("condition", 50)), 0, 100)
	var condition_enum := _resolve_condition_enum(condition_value)

	var out := {
		"stamina_system":
		{
			"current": int(md.get("stamina_system", {}).get("current", stamina_current)),
			"maximum": int(md.get("stamina_system", {}).get("maximum", 100)),
			"recovery_rate": int(md.get("stamina_system", {}).get("recovery_rate", 30))
		},
		"condition": str(md.get("condition", condition_enum)),
		"consecutive_training_days": int(md.get("consecutive_training_days", 0)),
		"consecutive_rest_days": int(md.get("consecutive_rest_days", 0)),
		"week_training_count": int(md.get("week_training_count", 0)),
		"training_history": md.get("training_history", []),
		"active_deck": null
	}
	return out


func _resolve_condition_enum(condition_value: int) -> String:
	"""Map 0-100 condition value to Rust Condition enum tag"""
	if condition_value >= 80:
		return "PerfectForm"
	elif condition_value >= 60:
		return "GoodForm"
	elif condition_value >= 40:
		return "Normal"
	elif condition_value >= 25:
		return "PoorForm"
	else:
		return "TerribleForm"


func _coerce_to_int(v: Variant) -> int:
	"""Coerce various types to int"""
	if typeof(v) == TYPE_INT:
		return int(v)
	if typeof(v) == TYPE_FLOAT:
		return int(round(float(v)))
	if typeof(v) == TYPE_BOOL:
		return 1 if v else 0
	if typeof(v) == TYPE_STRING:
		var s := String(v)
		if s.is_valid_float():
			return int(round(float(s)))
		elif s.is_valid_int():
			return int(s)
	return 0


func _coerce_map_to_ints(src: Dictionary) -> Dictionary:
	"""Coerce all values in a dictionary to ints"""
	var out: Dictionary = {}
	for k in src.keys():
		out[str(k).to_lower()] = _coerce_to_int(src[k])
	return out


func _synthesize_attributes_from_ca(ca_value: int, pos: String) -> Dictionary:
	"""Generate synthetic attributes based on CA and position"""
	var base: int = clampi(ca_value, 30, 90)
	match String(pos).to_upper():
		"GK":
			return {
				"handling": base,
				"reflexes": base,
				"aerial_reach": base - 5,
				"kicking": base - 10,
				"positioning": base - 5,
			}
		"CB":
			return {
				"tackling": base,
				"marking": base - 5,
				"heading": base - 5,
				"strength": base - 10,
				"positioning": base - 5,
			}
		"CM":
			return {
				"passing": base,
				"vision": base - 5,
				"work_rate": base - 10,
				"first_touch": base - 5,
				"decisions": base - 5,
			}
		"ST":
			return {
				"finishing": base,
				"off_the_ball": base - 5,
				"pace": base - 10,
				"acceleration": base - 10,
				"composure": base - 5,
			}
		_:
			return {
				"technique": base - 5,
				"stamina": base - 10,
				"balance": base - 10,
				"agility": base - 10,
				"work_rate": base - 10,
			}


func _normalize_position_code(pos: String) -> String:
	"""Map any incoming position to allowed codes (GK/CB/CM/ST)"""
	var up := pos.to_upper()
	match up:
		"GK", "GKP":
			return "GK"
		"CB", "LCB", "RCB", "DF", "D", "DC", "DR", "DL", "WB", "WBR", "WBL":
			return "CB"
		"CM", "MF", "M", "MC", "CDM", "CAM", "LM", "RM", "AMC":
			return "CM"
		"ST", "CF", "FW", "SS":
			return "ST"
		_:
			if up.begins_with("G"):
				return "GK"
			elif up.begins_with("D") or up.begins_with("WB"):
				return "CB"
			elif up.begins_with("M") or up.begins_with("A") or up.begins_with("C"):
				return "CM"
			elif up.begins_with("S") or up.begins_with("F"):
				return "ST"
			return "CM"
