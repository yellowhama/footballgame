extends Node

## 전역 게임 캐시 로더
## 런타임에 바이너리 캐시를 로드하여 게임 엔진에 주입
## MessagePack+LZ4 캐시 파일을 읽어 메모리에 적재

# 캐시 데이터 저장소
var climate_coeffs := {}
var balance_data := {}
var coach_cards := {}
var training_efficiency := {}
var player_cache_loaded := false  # Phase 3

# 로드 상태
var is_loaded := false
var load_errors: Array[String] = []

# Player cache retry state (lazy/late load safety)
var _last_player_cache_attempt_ms: int = -1
const PLAYER_CACHE_RETRY_MS: int = 2000


## 부팅 시 자동 로드
func _ready():
	print("🔄 GameCache: Loading game caches...")
	_load_all_caches()
	_load_player_cache()  # Phase 3

	if is_loaded:
		print("✅ GameCache: All caches loaded successfully")
		_inject_to_engine()
	else:
		push_error("❌ GameCache: Failed to load some caches")
		for error in load_errors:
			push_error("   - %s" % error)


## 모든 캐시 파일 로드
func _load_all_caches() -> void:
	var cache_files := {
		"climate_coeffs": "res://data/exports/cache_climate_coeffs.v3.msgpack.lz4",
		"balance_data": "res://data/exports/cache_game_balance.v3.msgpack.lz4",
		"coach_cards": "res://data/exports/cache_cards_v3.msgpack.lz4",
		"training_efficiency": "res://data/exports/cache_training_efficiency.v3.msgpack.lz4",
	}

	var success_count := 0

	for key in cache_files:
		var path: String = cache_files[key]
		var result = _load_cache(path)

		if result is Dictionary:
			match key:
				"climate_coeffs":
					climate_coeffs = result
				"balance_data":
					balance_data = result
				"coach_cards":
					coach_cards = result
				"training_efficiency":
					training_efficiency = result
			success_count += 1
			print("   ✅ Loaded: %s (%d entries)" % [key, _get_entry_count(result)])
		else:
			var error_msg := "Failed to load %s: %s" % [key, path]
			load_errors.append(error_msg)
			push_warning("   ⚠️ %s" % error_msg)

	is_loaded = success_count == cache_files.size()


## 캐시 파일 로드 (MessagePack+LZ4)
func _load_cache(path: String) -> Variant:
	# Calculate JSON fallback path once to avoid shadowing
	var json_fallback_path := path.replace(".msgpack.lz4", ".json")

	# 파일 존재 확인
	if not FileAccess.file_exists(path):
		push_warning("Cache file not found: %s" % path)

		# Fallback: JSON 형식 시도
		if FileAccess.file_exists(json_fallback_path):
			push_warning("Falling back to JSON: %s" % json_fallback_path)
			return _load_json_fallback(json_fallback_path)

		return null

	var bytes: PackedByteArray = FileAccess.get_file_as_bytes(path)
	if bytes.is_empty():
		push_error("Cache file is empty: %s" % path)
		return null

	# TODO: LZ4 압축 해제 (GDExtension 래퍼 필요)
	# 현재는 JSON fallback 사용
	push_warning("⚠️ MessagePack+LZ4 decompression not yet implemented")
	push_warning("   Using JSON fallback for: %s" % path)

	return _load_json_fallback(json_fallback_path)


## JSON Fallback 로더 (임시)
func _load_json_fallback(path: String) -> Variant:
	if not FileAccess.file_exists(path):
		push_error("JSON fallback file not found: %s" % path)
		return null

	var json_str := FileAccess.get_file_as_string(path)
	var json := JSON.new()
	var error := json.parse(json_str)

	if error != OK:
		push_error("Failed to parse JSON: %s (line %d)" % [json.get_error_message(), json.get_error_line()])
		return null

	return json.data


## Phase 3: Player cache loader
var data_cache_store: Object = null


func _load_player_cache() -> void:
	# Create DataCacheStore instance if it doesn't exist
	if data_cache_store == null:
		if ClassDB.class_exists("DataCacheStore"):
			data_cache_store = ClassDB.instantiate("DataCacheStore")
			if data_cache_store == null:
				push_warning("   ⚠️ Failed to instantiate DataCacheStore")
				return
		else:
			push_warning("   ⚠️ DataCacheStore class not found - skipping player cache")
			return

	# Load player cache (prefer latest schema; fall back if needed)
	var cache_res_paths: Array[String] = [
		"res://data/exports/cache_players.v4.msgpack.lz4",
		"res://data/exports/cache_players.v3.msgpack.lz4",
	]
	var candidate_paths: Array[String] = []

	# Allow override via environment variable for custom workflows
	if OS.has_environment("PLAYER_CACHE_PATH"):
		var override_path := OS.get_environment("PLAYER_CACHE_PATH").strip_edges()
		if override_path != "":
			candidate_paths.append(override_path)

	# Preferred Windows project location (user request) + WSL mount mirrors.
	for res_path in cache_res_paths:
		var res_path_str: String = String(res_path)
		var file_name: String = res_path_str.get_file()
		var windows_path: String = "F:/Aisaak/Projects/football_game_wsl/data/exports/%s" % file_name
		var wsl_path: String = "/mnt/f/Aisaak/Projects/football_game_wsl/data/exports/%s" % file_name

		if not candidate_paths.has(windows_path):
			candidate_paths.append(windows_path)
		if not candidate_paths.has(wsl_path):
			candidate_paths.append(wsl_path)

		# Project-local fallback (res:// → absolute)
		var project_path: String = ProjectSettings.globalize_path(res_path_str)
		if project_path != "" and not candidate_paths.has(project_path):
			candidate_paths.append(project_path)

	var resolved_path: String = ""
	for candidate in candidate_paths:
		var path := String(candidate).strip_edges()
		if path == "":
			continue

		if FileAccess.file_exists(path):
			resolved_path = path
			break

	if resolved_path == "":
		push_warning("   ⚠️ Player cache file not found. Checked paths:")
		for candidate in candidate_paths:
			if String(candidate) != "":
				push_warning("      - %s" % candidate)
		return

	print("   ↪ Loading player cache from: %s" % resolved_path)
	print("   🔍 DEBUG: Calling data_cache_store.load_player_cache()...")
	print("   🔍 DEBUG: data_cache_store = %s" % data_cache_store)
	print(
		(
			"   🔍 DEBUG: data_cache_store has method load_player_cache: %s"
			% data_cache_store.has_method("load_player_cache")
		)
	)
	var success: bool = data_cache_store.load_player_cache(resolved_path)
	print("   🔍 DEBUG: load_player_cache() returned: %s" % success)
	if not success and "cache_players.v4" in resolved_path:
		var fallback_path := resolved_path.replace("cache_players.v4", "cache_players.v3")
		if FileAccess.file_exists(fallback_path):
			push_warning("   ⚠️ v4 player cache failed to load; trying v3: %s" % fallback_path)
			success = data_cache_store.load_player_cache(fallback_path)
			if success:
				resolved_path = fallback_path

	if success:
		player_cache_loaded = true
		var count: int = data_cache_store.get_player_count()
		var version: String = data_cache_store.get_player_cache_version()
		print("   ✅ Loaded: player_cache (%d players, schema: %s)" % [count, version])

		# Inject player cache to Rust PLAYER_LIBRARY
		_inject_player_cache_to_rust(count)
	else:
		push_warning("   ⚠️ Failed to load player cache")


## Public helper: best-effort lazy load for callers that need the player cache.
## This allows match startup to "pull" the cache if GameCache boot order is late.
func ensure_player_cache_loaded() -> bool:
	if is_player_cache_ready():
		return true

	var now := Time.get_ticks_msec()
	if _last_player_cache_attempt_ms >= 0 and now - _last_player_cache_attempt_ms < PLAYER_CACHE_RETRY_MS:
		return false
	_last_player_cache_attempt_ms = now

	_load_player_cache()
	return is_player_cache_ready()


## 플레이어 캐시를 Rust PLAYER_LIBRARY에 주입
func _inject_player_cache_to_rust(player_count: int) -> void:
	if not has_node("/root/FootballRustEngine"):
		push_warning("   ⚠️ FootballRustEngine not found - cannot inject player cache")
		return

	var engine = get_node("/root/FootballRustEngine")
	var simulator = engine.get_simulator()
	if simulator == null or not simulator.has_method("load_csv_players"):
		push_warning("   ⚠️ FootballMatchSimulator.load_csv_players() not found")
		return

	print("   🔄 Injecting %d players to Rust PLAYER_LIBRARY..." % player_count)

	# Build player array from DataCacheStore
	var players := []
	for uid in range(1, player_count + 1):
		var player_dict = data_cache_store.get_player(uid)
		if not player_dict.is_empty():
			# Convert to Person format expected by Rust
			players.append(player_dict)

	# Call load_csv_players with Variant API (direct to simulator)
	var request = {"schema_version": 1, "players": players}

	var result = simulator.load_csv_players(request)
	if result.get("success", false):
		print("   ✅ Injected %d players to Rust PLAYER_LIBRARY" % result.get("loaded_count", 0))
	else:
		push_warning("   ⚠️ Failed to inject players: %s" % result.get("error", "Unknown error"))


## 엔진에 캐시 데이터 주입
func _inject_to_engine() -> void:
	# FootballRustEngine이 존재하면 주입
	if has_node("/root/FootballRustEngine"):
		var engine = get_node("/root/FootballRustEngine")

		if engine.has_method("set_climate_coeffs"):
			engine.set_climate_coeffs(climate_coeffs)
		if engine.has_method("set_balance_data"):
			engine.set_balance_data(balance_data)
		if engine.has_method("set_coach_cards"):
			engine.set_coach_cards(coach_cards)
		if engine.has_method("set_training_efficiency"):
			engine.set_training_efficiency(training_efficiency)

		print("✅ GameCache: Injected to FootballRustEngine")
	else:
		push_warning("⚠️ GameCache: FootballRustEngine not found - skipping injection")


## 엔트리 개수 계산 (디버깅용)
func _get_entry_count(data: Variant) -> int:
	if data is Dictionary:
		return data.size()
	elif data is Array:
		return data.size()
	else:
		return 0


## 외부에서 캐시 데이터 조회
func get_climate_coeff(country_code: String) -> Dictionary:
	if climate_coeffs.has(country_code):
		return climate_coeffs[country_code]
	return {}


func get_balance_stat(position: String) -> Dictionary:
	if balance_data.has(position):
		return balance_data[position]
	return {}


func get_coach_card(card_id: String) -> Dictionary:
	if coach_cards.has(card_id):
		return coach_cards[card_id]
	return {}


func get_training_efficiency(training_type: String) -> Dictionary:
	if training_efficiency.has(training_type):
		return training_efficiency[training_type]
	return {}


## 캐시 다시 로드 (핫 리로드)
func reload_caches() -> void:
	print("🔄 GameCache: Reloading caches...")
	load_errors.clear()
	_load_all_caches()
	_load_player_cache()  # Phase 3

	if is_loaded:
		print("✅ GameCache: Reload complete")
		_inject_to_engine()
	else:
		push_error("❌ GameCache: Reload failed")


## Phase 3: Query player cache
func get_player(uid: int) -> Dictionary:
	if data_cache_store != null and player_cache_loaded:
		return data_cache_store.get_player(uid)
	return {}


func search_players(query: String, use_real_names: bool = false, max_results: int = 50) -> Array:
	if data_cache_store != null and player_cache_loaded:
		return data_cache_store.search_players_by_name(query, use_real_names, max_results)
	return []


func is_player_cache_ready() -> bool:
	return data_cache_store != null and player_cache_loaded


func get_players_by_team(team_name: String, use_real_names: bool = false) -> Array:
	if data_cache_store != null and player_cache_loaded:
		return data_cache_store.get_players_by_team(team_name, use_real_names)
	return []


func get_balanced_roster(min_ca: int, max_players: int = 18, use_real_names: bool = false) -> Array:
	if data_cache_store != null and player_cache_loaded:
		# Check if the method exists in the current DLL version
		if data_cache_store.has_method("get_balanced_roster"):
			return data_cache_store.get_balanced_roster(min_ca, max_players, use_real_names)
		else:
			# Fallback: DLL doesn't have get_balanced_roster, use CSV fallback
			push_warning("[GameCache] get_balanced_roster not available in DLL, falling back to CSV")
			return []
	return []


## Phase 4: Position Ratings API
## Get all 14 position ratings for a player
## @param uid: Player unique ID
## @return Dictionary with position keys (GK, DL, DC, DR, WBL, WBR, DM, ML, MC, MR, AML, AMC, AMR, ST)
##         Returns empty dict if player not found or ratings unavailable
func get_player_position_ratings(uid: int) -> Dictionary:
	if not is_player_cache_ready():
		return {}
	if data_cache_store.has_method("get_player_position_ratings"):
		return data_cache_store.get_player_position_ratings(uid)
	else:
		push_warning("[GameCache] get_player_position_ratings not available in DLL")
		return {}


## Get player's best positions (filtered by minimum rating, sorted descending)
## @param uid: Player unique ID
## @param min_rating: Minimum rating to include (0-20, default 15)
## @return Array of Dictionaries [{position: "MC", rating: 20}, ...]
##         Returns empty array if player not found
func get_player_best_positions(uid: int, min_rating: int = 15) -> Array:
	if not is_player_cache_ready():
		return []
	if data_cache_store.has_method("get_player_best_positions"):
		return data_cache_store.get_player_best_positions(uid, min_rating)
	else:
		push_warning("[GameCache] get_player_best_positions not available in DLL")
		return []


## Search all players by position rating (e.g., find best strikers)
## @param position: Position code ("GK", "DL", "DC", "DR", "WBL", "WBR", "DM", "ML", "MC", "MR", "AML", "AMC", "AMR", "ST")
## @param min_rating: Minimum rating to include (0-20, default 15)
## @param max_results: Maximum number of results (default 50)
## @return Array of Dictionaries [{uid, name, position_rating, ca, pa, age, position}, ...]
##         Sorted by rating descending
func find_best_players_for_position(position: String, min_rating: int = 15, max_results: int = 50) -> Array:
	if not is_player_cache_ready():
		return []
	if data_cache_store.has_method("find_best_players_for_position"):
		return data_cache_store.find_best_players_for_position(position, min_rating, max_results)
	else:
		push_warning("[GameCache] find_best_players_for_position not available in DLL")
		return []


## Compute average attributes for a given position using CSV/DLL player cache
## @param position: String like "GK", "CB", "CM", "ST"
## @param use_real_names: whether to query real-name team rosters
## @return Dictionary: { count:int, ca:int, attributes:Dictionary }
func get_position_average(
	position: String, use_real_names: bool = true, league_filter: String = "", min_ca: int = 0
) -> Dictionary:
	if data_cache_store == null or not player_cache_loaded:
		return {}

	# Prefer native DLL method if available (faster)
	if data_cache_store.has_method("get_position_average"):
		var res: Variant = data_cache_store.get_position_average(position, league_filter, min_ca)
		if typeof(res) == TYPE_DICTIONARY:
			return res

	var pos := String(position).to_upper()
	var team_list := [
		"Man Town",
		"Real Royal",
		"Blue Bridge",
		"Mun Reds",
		"Paris Stars",
		"Turin White",
		"Catalan FC",
		"Mun Blue",
		"North Reds",
		"Milan Black",
		"Milan Red",
		"London Gunners",
		"London Spurs",
		"Amsterdam Ajax",
		"Lisbon Eagles",
		"Dort Blue",
		"Leipzig RB",
		"Porto Dragons",
		"Sevilla Red",
		"Napoli"
	]

	var total_ca := 0
	var count := 0
	var sums: Dictionary = {}

	for team_name in team_list:
		var roster: Array = data_cache_store.get_players_by_team(team_name, use_real_names)
		if typeof(roster) != TYPE_ARRAY:
			continue
		for p in roster:
			if typeof(p) != TYPE_DICTIONARY:
				continue
			var ppos := String(p.get("position", ""))
			var canon := _canonical_position(ppos)
			if not _position_matches(canon, pos):
				continue
			if min_ca > 0 and int(p.get("ca", p.get("overall", 0))) < min_ca:
				continue
			count += 1
			total_ca += int(p.get("ca", p.get("overall", 0)))
			if p.has("attributes") and typeof(p.get("attributes")) == TYPE_DICTIONARY:
				var attrs: Dictionary = p.get("attributes")
				for k in attrs.keys():
					var key := String(k).to_lower()
					var val := int(attrs[k])
					sums[key] = (int(sums.get(key, 0)) + val)

	if count == 0:
		return {}

	var avg: Dictionary = {}
	for k in sums.keys():
		avg[k] = int(round(float(sums[k]) / float(count)))

	return {"count": count, "ca": int(round(float(total_ca) / float(count))), "attributes": avg}


func _canonical_position(pos: String) -> String:
	var up := String(pos).to_upper()
	match up:
		"GK":
			return "GK"
		"CB", "LCB", "RCB", "DF", "SW":
			return "CB"
		"CDM", "CM", "CAM", "LM", "RM", "MF", "LW", "RW":
			return "CM"
		"ST", "CF", "FW":
			return "ST"
		_:
			return up


func _position_matches(canon: String, target: String) -> bool:
	if target == "GK":
		return canon == "GK"
	if target == "CB":
		return canon == "CB"
	if target == "CM":
		return canon == "CM"
	if target == "ST":
		return canon == "ST"
	return canon == target
