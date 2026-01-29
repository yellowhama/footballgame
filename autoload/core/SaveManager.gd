## SaveManager.gd - Singleton Stub
## Manages game save/load functionality
extends Node

signal save_completed(slot_id: String)
signal load_completed(slot_id: String)
signal save_error(error_message: String)

const SAVE_PATH: String = "user://saves/"
const SETTINGS_PATH: String = "user://settings.json"  # P0: Tutorial flag storage
const MAX_SAVE_SLOTS: int = 5
const VERSION: String = "0.9.2"  # Phase 9.2 - Auto-save System
const AUTO_SAVE_SLOT: String = "slot_auto"
const USE_RUST_BINARY_SAVE: bool = true
const DEV_FORCE_LEGACY_SETTING: String = "of/dev_force_legacy_save"
const LEGACY_SAVE_ENV: String = "DEV_LEGACY_SAVE"

var current_save_slot: String = ""
var save_slots: Dictionary = {}

## Phase 9.2: Auto-save Configuration
var auto_save_enabled: bool = true
var auto_save_frequency: int = 1  # Auto-save every N weeks (1 = every week)
var _initial_auto_save_done: bool = false  # First auto-save in this session skips heavy position data

## P0: Tutorial completion flag
var tutorial_completed: bool = false
var view_mode_preference: String = "3d"  # "2d" or "3d"


func _ready() -> void:
	# Create save directory if it doesn't exist
	if not DirAccess.dir_exists_absolute(SAVE_PATH):
		DirAccess.open("user://").make_dir_recursive("saves")

	# Load existing save slot info
	_load_save_slots()

	# P0: Load settings (tutorial flag)
	_load_settings()

	# Phase 9.2: Connect to GameManager for auto-save
	_connect_auto_save_signals()

	print(
		(
			"[SaveManager] Initialized (v%s) - Auto-save: %s, Tutorial: %s"
			% [VERSION, "ON" if auto_save_enabled else "OFF", "COMPLETED" if tutorial_completed else "PENDING"]
		)
	)


func _load_save_slots() -> void:
	# Stub implementation - scan for existing saves
	for i in range(MAX_SAVE_SLOTS):
		var slot_id: String = "slot_%d" % i
		var save_file_path: String = SAVE_PATH + slot_id + ".save"

		if FileAccess.file_exists(save_file_path):
			save_slots[slot_id] = {
				"exists": true, "timestamp": Time.get_ticks_msec(), "player_name": "Player %d" % i, "progress": 0.0  # Stub timestamp
			}
		else:
			save_slots[slot_id] = {"exists": false}


func save_game(slot_id: String, game_data: Dictionary = {}) -> void:
	"""
	Phase 9.1: Unified save function integrating all managers
	Automatically collects data from GlobalCharacterData, TrainingManager, MatchManager
	"""
	# Collect data from all managers
	var unified_data: Dictionary = {
		"version": VERSION,
		"timestamp": Time.get_unix_time_from_system(),
		"player_name": game_data.get("player_name", "Unknown Player"),
		"progress": game_data.get("progress", 0.0)
	}

	# GlobalCharacterData integration
	if GlobalCharacterData:
		unified_data["player"] = GlobalCharacterData.save_to_dict()
		print("[SaveManager] Collected player data (%d keys)" % unified_data["player"].size())

	# TrainingManager integration
	if TrainingManager:
		unified_data["training"] = TrainingManager.save_to_dict()
		print("[SaveManager] Collected training data")

	# MatchManager integration
	if MatchManager:
		unified_data["match"] = MatchManager.save_to_dict()
		print("[SaveManager] Collected match data")

	# TacticsManager integration
	var tactics_mgr := get_node_or_null("/root/TacticsManager")
	if tactics_mgr and tactics_mgr.has_method("get_save_data"):
		var tactics_data: Dictionary = tactics_mgr.get_save_data()
		if not tactics_data.is_empty():
			unified_data["tactics"] = tactics_data
			print("[SaveManager] Collected tactics data")

	# TraitManager integration (Unified Trait System)
	if TraitManager and TraitManager.has_method("save_to_dict"):
		unified_data["trait_manager"] = TraitManager.save_to_dict()
		print("[SaveManager] Collected trait data")

	# NOTE: Deck/Gacha/Inventory are SSOT in Rust (FIX_2601/0109).
	# Do not save any Godot-side deck state here (DeckManager is legacy/UI only).

	# DateManager integration (MVP temporal state)
	if DateManager and DateManager.has_method("save_to_dict"):
		unified_data["date_manager"] = DateManager.save_to_dict()
		print("[SaveManager] Collected DateManager state")
		if DateManager.has_method("get_current_weekly_plan"):
			var plan_resource = DateManager.get_current_weekly_plan()
			if plan_resource and plan_resource.has_method("to_dict"):
				var weekly_plan_dict: Dictionary = plan_resource.to_dict()
				if not weekly_plan_dict.is_empty():
					unified_data["weekly_plan"] = weekly_plan_dict
					print("[SaveManager] Captured weekly plan snapshot")

	# ✅ Phase 23: Division state
	if DivisionManager and DivisionManager.has_method("save_to_dict"):
		unified_data["division_manager"] = DivisionManager.save_to_dict()
		print(
			(
				"[SaveManager] Collected DivisionManager state (Division %d, Season %d)"
				% [DivisionManager.current_division, DivisionManager.current_season]
			)
		)

	# ✅ Phase 24: Decision tracking and career statistics
	if DecisionTracker and DecisionTracker.has_method("save_to_dict"):
		unified_data["decision_tracker"] = DecisionTracker.save_to_dict()
		print(
			"[SaveManager] Collected DecisionTracker state (%d decisions logged)" % DecisionTracker.decision_log.size()
		)

	if CareerStatisticsManager and CareerStatisticsManager.has_method("save_to_dict"):
		unified_data["career_statistics"] = CareerStatisticsManager.save_to_dict()
		print(
			(
				"[SaveManager] Collected CareerStatisticsManager state (%d goals, %d assists, %d matches)"
				% [
					CareerStatisticsManager.total_goals,
					CareerStatisticsManager.total_assists,
					CareerStatisticsManager.total_matches
				]
			)
		)

	# ✅ Phase 22: MyTeamManager integration (graduated players in reserves)
	if MyTeamManager and MyTeamManager.has_method("save_data"):
		unified_data["my_team_manager"] = MyTeamManager.save_data()
		print(
			(
				"[SaveManager] Collected MyTeamManager (%d first team, %d reserves)"
				% [MyTeamManager.first_team.size(), MyTeamManager.reserves.size()]
			)
		)

	# Quest system integration (Phase 5) with error handling
	var quest_bridge = null
	if ClassDB.class_exists("QuestBridge"):
		quest_bridge = ClassDB.instantiate("QuestBridge")
		if not is_instance_valid(quest_bridge):
			push_warning("[SaveManager] Failed to instantiate QuestBridge")
			quest_bridge = null

	if quest_bridge and quest_bridge.has_method("get_all_quests"):
		var quests_json: String = quest_bridge.get_all_quests()
		if not quests_json.is_empty():
			unified_data["quest_state"] = quests_json
			print("[SaveManager] Collected quest state")

	# Story system integration (Phase 5)
	if StoryManager and StoryManager.has_method("save_story"):
		var story_save: Dictionary = StoryManager.save_story()
		if story_save.get("success", false):
			unified_data["story_state"] = story_save.get("save_data", "")
			print("[SaveManager] Collected story state")

	# HUD snapshot (training result, etc.)
	var training_snapshot: Dictionary = _collect_training_snapshot()
	if not training_snapshot.is_empty():
		unified_data["training_snapshot"] = training_snapshot
		unified_data["ui_state"] = {"training_last_result": training_snapshot.duplicate(true)}
		print("[SaveManager] Captured training snapshot for HUD restore")

	var omit_match_position_data: bool = game_data.get("omit_match_position_data", false)
	if omit_match_position_data:
		print("[SaveManager] Skipping match_position_data for slot %s (omit_match_position_data=true)" % slot_id)
	else:
		var match_position_payload: Dictionary = _collect_match_position_data()
		if not match_position_payload.is_empty():
			unified_data["match_position_data"] = match_position_payload
			var ball_samples: int = 0
			var ball_series = match_position_payload.get("ball", [])
			if ball_series is Array:
				ball_samples = (ball_series as Array).size()
			var player_samples: int = 0
			if match_position_payload.has("players") and match_position_payload["players"] is Dictionary:
				for key in match_position_payload["players"]:
					var entries = match_position_payload["players"][key]
					if entries is Array:
						player_samples += entries.size()
			print(
				(
					"[SaveManager] Captured match position data (ball=%d, player_samples=%d)"
					% [ball_samples, player_samples]
				)
			)

	# M1.4: Add MyTeam branding data
	var my_team_data_node = get_node_or_null("/root/MyTeamData")
	if my_team_data_node and my_team_data_node.has_method("get_branding_data"):
		unified_data["my_team_data"] = my_team_data_node.get_branding_data()
		print("[SaveManager] Collected MyTeam branding data")

	# ✅ Phase 22: UID sequence counter persistence
	unified_data["uid_state"] = {
		"sequence_counter": MyTeamData._uid_sequence_counter, "last_timestamp": MyTeamData._last_uid_timestamp
	}
	print(
		(
			"[SaveManager] Saved UID state (counter: %d, timestamp: %d)"
			% [MyTeamData._uid_sequence_counter, MyTeamData._last_uid_timestamp]
		)
	)

	# Add any additional game_data provided
	if game_data.size() > 0:
		unified_data["additional_data"] = game_data

	var save_file_path: String = SAVE_PATH + slot_id + ".save"
	var prefer_binary := _is_binary_save_enabled()

	if not FootballRustEngine or not FootballRustEngine.is_ready():
		save_error.emit("FootballRustEngine not ready - cannot save game")
		push_error("[SaveManager] FootballRustEngine not ready - cannot save game")
		return

	# ✅ FIX_2601/0109: Persist coach_state (gacha/deck/inventory) from Rust SSOT.
	if FootballRustEngine.has_method("coach_export_state"):
		var coach_export: Dictionary = FootballRustEngine.coach_export_state()
		if coach_export.get("success", false) and coach_export.has("state"):
			unified_data["coach_state"] = coach_export["state"]
			print("[SaveManager] Collected coach_state (gacha/deck/inventory)")
		else:
			push_warning("[SaveManager] Failed to export coach_state: %s" % coach_export.get("error", "unknown"))
	else:
		push_warning("[SaveManager] coach_export_state not available; coach_state will not be saved")

	if prefer_binary:
		var binary_payload: PackedByteArray = FootballRustEngine.save_game_binary(unified_data)
		if binary_payload.size() > 0:
			if _write_binary_save(save_file_path, binary_payload):
				_record_save_metadata(slot_id, unified_data, "Binary")
			else:
				save_error.emit("Failed to write binary save file")
				push_error("[SaveManager] Failed to write binary save file.")
		else:
			save_error.emit("Rust save_game_binary returned empty payload")
			push_error("[SaveManager] Rust save_game_binary returned empty payload.")
	else:
		# JSON save through Rust engine
		var json_payload: String = FootballRustEngine.save_game_json(unified_data)
		if json_payload.length() > 0:
			var file := FileAccess.open(save_file_path, FileAccess.WRITE)
			if file:
				file.store_string(json_payload)
				file.close()
				_record_save_metadata(slot_id, unified_data, "JSON")
			else:
				save_error.emit("Failed to create save file")
				push_error("[SaveManager] Unable to open %s for JSON save" % save_file_path)
		else:
			save_error.emit("Rust save_game_json returned empty payload")
			push_error("[SaveManager] Rust save_game_json returned empty payload.")


func _write_binary_save(save_file_path: String, binary_data: PackedByteArray) -> bool:
	var file := FileAccess.open(save_file_path, FileAccess.WRITE)
	if not file:
		save_error.emit("Failed to create binary save file")
		push_error("[SaveManager] Unable to open %s for binary save" % save_file_path)
		return false
	file.store_buffer(binary_data)
	file.close()
	return true


func _record_save_metadata(slot_id: String, unified_data: Dictionary, method_label: String) -> void:
	save_slots[slot_id] = {
		"exists": true,
		"timestamp": unified_data.get("timestamp", Time.get_unix_time_from_system()),
		"player_name": unified_data.get("player_name", "Unknown"),
		"progress": unified_data.get("progress", 0.0),
		"version": VERSION
	}
	current_save_slot = slot_id
	save_completed.emit(slot_id)
	print("[SaveManager] Game saved to slot: %s (version %s, %s)" % [slot_id, VERSION, method_label])


func _legacy_save_override() -> bool:
	if OS.has_environment(LEGACY_SAVE_ENV):
		var raw := OS.get_environment(LEGACY_SAVE_ENV).strip_edges().to_lower()
		if raw in ["1", "true", "yes", "on"]:
			return true
		if raw in ["0", "false", "no", "off"]:
			return false
	if ProjectSettings.has_setting(DEV_FORCE_LEGACY_SETTING):
		return ProjectSettings.get_setting(DEV_FORCE_LEGACY_SETTING)
	return false


func _is_binary_save_enabled() -> bool:
	return USE_RUST_BINARY_SAVE and not _legacy_save_override()


func load_game(slot_id: String) -> Dictionary:
	"""
	Phase 9.1: Unified load function restoring all managers
	Automatically restores data to GlobalCharacterData, TrainingManager, MatchManager
	Supports both binary (MessagePack+LZ4+SHA256) and legacy JSON formats
	"""
	var save_file_path: String = SAVE_PATH + slot_id + ".save"

	if not FileAccess.file_exists(save_file_path):
		save_error.emit("Save file does not exist")
		push_error("[SaveManager] Save file does not exist: %s" % save_file_path)
		return {}

	var file := FileAccess.open(save_file_path, FileAccess.READ)
	if not file:
		save_error.emit("Failed to open save file")
		push_error("[SaveManager] Failed to open save file: %s" % save_file_path)
		return {}

	var binary_data := file.get_buffer(file.get_length())
	file.close()

	if binary_data.is_empty():
		save_error.emit("Save file is empty")
		push_error("[SaveManager] Save file contains no data: %s" % save_file_path)
		return {}

	var save_data: Dictionary = {}
	var attempted_binary := false
	var binary_enabled := _is_binary_save_enabled()

	if binary_enabled:
		if FootballRustEngine and FootballRustEngine.is_ready():
			save_data = FootballRustEngine.load_game_binary(binary_data)
			attempted_binary = true
			if save_data.is_empty():
				push_warning("[SaveManager] Binary load failed, attempting JSON fallback")
		else:
			push_warning("[SaveManager] FootballRustEngine not ready for binary load; trying legacy JSON")

	if save_data.is_empty():
		var json_text: String = binary_data.get_string_from_utf8()
		var json := JSON.new()
		var parse_result := json.parse(json_text)

		if parse_result == OK:
			save_data = json.data
			if attempted_binary:
				push_warning("[SaveManager] Loaded legacy JSON after binary failure")
		else:
			save_error.emit("Failed to parse save file JSON")
			var reason := json.get_error_message()
			if attempted_binary:
				reason += " (binary payload?)"
			push_error("[SaveManager] JSON parse error: %s" % reason)
			return {}

	# Restore manager data (same for both binary and JSON)
	if not save_data.is_empty():
		# Version check
		var saved_version = save_data.get("version", "0.0.0")
		print("[SaveManager] Loading save version: %s (current: %s)" % [saved_version, VERSION])

		# GlobalCharacterData restoration
		if save_data.has("player") and GlobalCharacterData:
			GlobalCharacterData.load_from_dict(save_data["player"])
			print("[SaveManager] Restored player data")

		# TrainingManager restoration
		if save_data.has("training") and TrainingManager:
			TrainingManager.load_from_dict(save_data["training"])
			print("[SaveManager] Restored training data")

		# MatchManager restoration
		if save_data.has("match") and MatchManager:
			MatchManager.load_from_dict(save_data["match"])
			print("[SaveManager] Restored match data")

		# TacticsManager restoration
		var tactics_mgr := get_node_or_null("/root/TacticsManager")
		if save_data.has("tactics") and tactics_mgr and tactics_mgr.has_method("load_save_data"):
			tactics_mgr.load_save_data(save_data["tactics"])
			print("[SaveManager] Restored tactics data")

		# TraitManager restoration (Unified Trait System)
		if save_data.has("trait_manager") and TraitManager and TraitManager.has_method("load_from_dict"):
			TraitManager.load_from_dict(save_data["trait_manager"])
			print("[SaveManager] Restored trait data")

		# ✅ FIX_2601/0109: Coach state restoration (gacha/deck/inventory SSOT)
		if save_data.has("coach_state"):
			if FootballRustEngine and FootballRustEngine.is_ready() and FootballRustEngine.has_method("coach_import_state"):
				var coach_result: Dictionary = FootballRustEngine.coach_import_state(save_data["coach_state"])
				if coach_result.get("success", false):
					print("[SaveManager] Restored coach_state (gacha/deck/inventory)")
				else:
					push_warning("[SaveManager] Failed to restore coach_state: %s" % coach_result.get("error", "unknown"))
			else:
				push_warning("[SaveManager] coach_state present but FootballRustEngine not ready; skipping restoration")
		else:
			# Legacy saves: avoid leaking in-memory coach_state across loads.
			if FootballRustEngine and FootballRustEngine.is_ready() and FootballRustEngine.has_method("coach_reset_state"):
				var reset_result: Dictionary = FootballRustEngine.coach_reset_state()
				if reset_result.get("success", false):
					print("[SaveManager] coach_state missing; reset Rust coach_state to defaults")
				else:
					push_warning("[SaveManager] Failed to reset coach_state: %s" % reset_result.get("error", "unknown"))
			else:
				push_warning("[SaveManager] coach_state missing but FootballRustEngine not ready; skipping reset")

		# DateManager restoration
		if save_data.has("date_manager") and DateManager and DateManager.has_method("load_from_dict"):
			DateManager.load_from_dict(save_data["date_manager"])
			print("[SaveManager] Restored DateManager state")
			if save_data.has("weekly_plan") and DateManager.has_method("apply_weekly_plan_dict"):
				DateManager.apply_weekly_plan_dict(save_data["weekly_plan"])
				print("[SaveManager] Applied weekly plan override")

		# ✅ Phase 23: Division state restoration
		if save_data.has("division_manager") and DivisionManager:
			DivisionManager.load_from_dict(save_data["division_manager"])
			print(
				(
					"[SaveManager] Restored DivisionManager state (Division %d, Season %d, Position %d/6)"
					% [
						DivisionManager.current_division,
						DivisionManager.current_season,
						DivisionManager.player_stats.get("position", 0)
					]
				)
			)

		# ✅ Phase 24: Decision tracking and career statistics restoration
		if save_data.has("decision_tracker") and DecisionTracker and DecisionTracker.has_method("load_from_dict"):
			DecisionTracker.load_from_dict(save_data["decision_tracker"])
			print("[SaveManager] Restored DecisionTracker state (%d decisions)" % DecisionTracker.decision_log.size())
		elif not save_data.has("decision_tracker"):
			print("[SaveManager] No decision_tracker data in save (legacy save or new game)")

		if (
			save_data.has("career_statistics")
			and CareerStatisticsManager
			and CareerStatisticsManager.has_method("load_from_dict")
		):
			CareerStatisticsManager.load_from_dict(save_data["career_statistics"])
			print(
				(
					"[SaveManager] Restored CareerStatisticsManager state (%d goals, %d assists, %d matches)"
					% [
						CareerStatisticsManager.total_goals,
						CareerStatisticsManager.total_assists,
						CareerStatisticsManager.total_matches
					]
				)
			)
		elif not save_data.has("career_statistics"):
			print("[SaveManager] No career_statistics data in save (legacy save or new game)")

		# ✅ Phase 22: MyTeamManager restoration (graduated players in reserves)
		if save_data.has("my_team_manager") and MyTeamManager and MyTeamManager.has_method("load_data"):
			MyTeamManager.load_data(save_data["my_team_manager"])
			print(
				(
					"[SaveManager] Restored MyTeamManager (%d first team, %d reserves)"
					% [MyTeamManager.first_team.size(), MyTeamManager.reserves.size()]
				)
			)
		elif not save_data.has("my_team_manager"):
			print("[SaveManager] No my_team_manager data in save (legacy save or new game)")

		# Legacy: DeckManager restoration (non-SSOT; only when coach_state is missing)
		if not save_data.has("coach_state") and save_data.has("deck_manager") and DeckManager and DeckManager.has_method("load_deck_data"):
			DeckManager.load_deck_data(save_data["deck_manager"])
			print("[SaveManager] Restored legacy deck_manager data (non-SSOT)")

		# Quest system restoration (Phase 5) with error handling
		if save_data.has("quest_state"):
			var quest_bridge = null
			if ClassDB.class_exists("QuestBridge"):
				quest_bridge = ClassDB.instantiate("QuestBridge")
				if not is_instance_valid(quest_bridge):
					push_warning("[SaveManager] Failed to instantiate QuestBridge for restoration")
					quest_bridge = null

			if quest_bridge and quest_bridge.has_method("quest_init"):
				var quests_json: String = save_data["quest_state"]
				# Note: QuestBridge doesn't have a direct load method, so we need to reinitialize
				# The quest state is preserved in the JSON and will be restored on next init
				print("[SaveManager] Quest state available for restoration")

		# Story system restoration (Phase 5)
		if save_data.has("story_state") and StoryManager and StoryManager.has_method("load_story"):
			var result: Dictionary = StoryManager.load_story(save_data["story_state"])
			if result.get("success", false):
				print("[SaveManager] Restored story state")
			else:
				push_warning("[SaveManager] Failed to restore story state: %s" % result.get("error", "unknown"))

		if (
			save_data.has("match_position_data")
			and MatchSimulationManager
			and MatchSimulationManager.has_method("load_position_data_from_save")
		):
			MatchSimulationManager.load_position_data_from_save(save_data["match_position_data"])
			print("[SaveManager] Restored match position data")

		# HUD snapshot restoration
		var snapshot_payload := {}
		if save_data.has("training_snapshot"):
			snapshot_payload = save_data["training_snapshot"]
		elif save_data.has("ui_state"):
			var ui_state: Dictionary = save_data["ui_state"]
			if ui_state.has("training_last_result"):
				snapshot_payload = ui_state["training_last_result"]
		if snapshot_payload is Dictionary and not (snapshot_payload as Dictionary).is_empty():
			_restore_training_snapshot(snapshot_payload)

		# M1.4: Load MyTeam branding data if it exists
		if save_data.has("my_team_data"):
			var my_team_data_node = get_node_or_null("/root/MyTeamData")
			if my_team_data_node and my_team_data_node.has_method("load_branding_data"):
				my_team_data_node.load_branding_data(save_data["my_team_data"])
				print("[SaveManager] Restored MyTeam branding data")

		# ✅ Phase 22: Restore UID sequence counter
		if save_data.has("uid_state"):
			MyTeamData._uid_sequence_counter = save_data.uid_state.get("sequence_counter", 0)
			MyTeamData._last_uid_timestamp = save_data.uid_state.get("last_timestamp", 0)
			print(
				(
					"[SaveManager] Restored UID state (counter: %d, timestamp: %d)"
					% [MyTeamData._uid_sequence_counter, MyTeamData._last_uid_timestamp]
				)
			)
		elif not save_data.has("uid_state"):
			print("[SaveManager] No uid_state in save (legacy save)")

		current_save_slot = slot_id
		load_completed.emit(slot_id)
		print("[SaveManager] Game loaded from slot: %s" % slot_id)

		# Return additional_data if exists, otherwise return save_data for compatibility
		return save_data.get("additional_data", save_data)

	return {}


func _collect_training_snapshot() -> Dictionary:
	var store = _get_training_state_store()
	if store and store.has_method("has_last_result") and store.has_last_result():
		return store.get_last_result()

	if UIService and UIService.has_method("get_hud_element"):
		var hud_snapshot = UIService.get_hud_element("training_last_result")
		if hud_snapshot is Dictionary and not hud_snapshot.is_empty():
			return hud_snapshot.duplicate(true)

	return {}


func _collect_match_position_data() -> Dictionary:
	if MatchSimulationManager and MatchSimulationManager.has_method("get_last_position_data"):
		var payload = MatchSimulationManager.get_last_position_data()
		if payload is Dictionary and not payload.is_empty():
			return payload
	return {}


func _restore_training_snapshot(raw_snapshot) -> void:
	if not (raw_snapshot is Dictionary):
		return
	var snapshot: Dictionary = raw_snapshot.duplicate(true)
	if snapshot.is_empty():
		return

	var hud_refreshed := false
	if UIService and UIService.has_method("update_hud_element"):
		UIService.update_hud_element("training_last_result", snapshot)
		hud_refreshed = true

	if not hud_refreshed:
		var store = _get_training_state_store()
		if store and store.has_method("set_last_result"):
			store.set_last_result(snapshot)


func _get_training_state_store():
	return get_node_or_null("/root/TrainingStateStore")


func delete_save(slot_id: String) -> void:
	var save_file_path: String = SAVE_PATH + slot_id + ".save"
	if FileAccess.file_exists(save_file_path):
		DirAccess.open("user://").remove(save_file_path)
		save_slots[slot_id]["exists"] = false
		print("Save deleted: %s" % slot_id)


func get_save_slots() -> Dictionary:
	return save_slots


func has_save(slot_id: String) -> bool:
	return save_slots.get(slot_id, {}).get("exists", false)


## Phase 9.2: Progress Calculation


func get_game_progress() -> float:
	"""
	Calculate actual game progress percentage based on GameManager state
	Returns percentage (0.0-100.0)
	"""
	# Try GameManager first (has completed_weeks tracking)
	var game_manager = get_node_or_null("/root/GameManager")
	if game_manager and game_manager.has_method("get_progress_percentage"):
		return game_manager.get_progress_percentage()

	# Fallback to DateManager if GameManager unavailable
	var date_manager = get_node_or_null("/root/DateManager")
	if date_manager:
		# Calculate progress based on current day out of 798 total days
		var total_days = 798  # 3 years
		var current_day = date_manager.current_day
		return (float(current_day) / float(total_days)) * 100.0

	# Final fallback: return 0.0 if no managers available
	push_warning("[SaveManager] Cannot calculate progress - no date tracking available")
	return 0.0


## Phase 9.2: Auto-save System


func _connect_auto_save_signals() -> void:
	"""Connect to GameManager signals for auto-save triggers"""
	# Wait for next frame to ensure GameManager is ready
	await get_tree().process_frame

	var game_manager = get_node_or_null("/root/GameManager")
	if game_manager:
		# Connect to week_advanced signal for auto-save trigger
		if not game_manager.week_advanced.is_connected(_on_week_advanced):
			game_manager.week_advanced.connect(_on_week_advanced)
			print("[SaveManager] Connected to GameManager.week_advanced for auto-save")
	else:
		push_warning("[SaveManager] GameManager not found - auto-save disabled")
		auto_save_enabled = false


func _on_week_advanced(week: int, year: int) -> void:
	"""Handle week advancement for auto-save trigger"""
	if not auto_save_enabled:
		return

	# Check if we should auto-save based on frequency
	var total_weeks = (year - 1) * 52 + week
	if total_weeks % auto_save_frequency == 0:
		perform_auto_save()


func perform_auto_save() -> void:
	"""Perform automatic save to reserved auto-save slot"""
	if not auto_save_enabled:
		return

	print("[SaveManager] Performing auto-save...")

	# Collect current game data
	var game_data = {}

	# Get player name
	if GlobalCharacterData and GlobalCharacterData.character_data.has("player_name"):
		game_data["player_name"] = GlobalCharacterData.character_data["player_name"]
	else:
		game_data["player_name"] = "Auto-save"

	# Get game progress
	game_data["progress"] = get_game_progress()

	# For the very first auto-save in this session, omit heavy match_position_data
	# to keep the initial MVP onboarding flow snappy. Subsequent auto-saves
	# will persist full position data for recorded timelines.
	if not _initial_auto_save_done:
		game_data["omit_match_position_data"] = true
		_initial_auto_save_done = true
		print("[SaveManager] Initial auto-save: omitting match_position_data for performance")

	# Save to auto-save slot
	save_game(AUTO_SAVE_SLOT, game_data)

	print("[SaveManager] Auto-save completed")


func has_auto_save() -> bool:
	"""Check if auto-save exists"""
	return has_save(AUTO_SAVE_SLOT)


func load_auto_save() -> Dictionary:
	"""Load from auto-save slot"""
	if not has_auto_save():
		push_error("[SaveManager] No auto-save found")
		return {}

	return load_game(AUTO_SAVE_SLOT)


func set_auto_save_enabled(enabled: bool) -> void:
	"""Enable or disable auto-save"""
	auto_save_enabled = enabled
	print("[SaveManager] Auto-save %s" % ("enabled" if enabled else "disabled"))


func set_auto_save_frequency(weeks: int) -> void:
	"""Set auto-save frequency in weeks"""
	auto_save_frequency = max(1, weeks)
	print("[SaveManager] Auto-save frequency set to every %d week(s)" % auto_save_frequency)


func get_auto_save_config() -> Dictionary:
	"""Get current auto-save configuration"""
	return {"enabled": auto_save_enabled, "frequency": auto_save_frequency, "has_auto_save": has_auto_save()}


## P0: Tutorial Completion Management


func _load_settings() -> void:
	"""Load global settings (tutorial flag, etc.)"""
	if not FileAccess.file_exists(SETTINGS_PATH):
		print("[SaveManager] No settings file found, using defaults")
		return

	var file := FileAccess.open(SETTINGS_PATH, FileAccess.READ)
	if not file:
		push_error("[SaveManager] Failed to open settings file")
		return

	var json_text := file.get_as_text()
	file.close()

	var json := JSON.new()
	var parse_result := json.parse(json_text)

	if parse_result == OK:
		var settings: Dictionary = json.data
		tutorial_completed = settings.get("tutorial_completed", false)
		var saved_view_mode = settings.get("view_mode_preference", view_mode_preference)
		if saved_view_mode in ["2d", "3d"]:
			view_mode_preference = saved_view_mode
		else:
			print("[SaveManager] Ignoring invalid view mode preference: %s" % saved_view_mode)
		print("[SaveManager] Settings loaded: tutorial_completed = %s" % tutorial_completed)
	else:
		push_error("[SaveManager] Failed to parse settings JSON: %s" % json.get_error_message())


func _save_settings() -> void:
	"""Save global settings (tutorial flag, etc.)"""
	var settings := {
		"tutorial_completed": tutorial_completed, "view_mode_preference": view_mode_preference, "version": VERSION
	}

	var file := FileAccess.open(SETTINGS_PATH, FileAccess.WRITE)
	if not file:
		push_error("[SaveManager] Failed to create settings file")
		return

	file.store_string(JSON.stringify(settings, "\t"))
	file.close()

	print("[SaveManager] Settings saved: tutorial_completed = %s" % tutorial_completed)


func set_tutorial_completed(completed: bool) -> void:
	"""Set tutorial completion flag and save"""
	tutorial_completed = completed
	_save_settings()
	print("[SaveManager] Tutorial completion set to: %s" % completed)


func is_tutorial_completed() -> bool:
	"""Check if tutorial has been completed"""
	return tutorial_completed


func set_view_mode_preference(mode: String) -> void:
	"""Save the preferred match view mode ('2d' or '3d')."""
	if mode not in ["2d", "3d"]:
		push_warning("[SaveManager] Ignoring invalid view mode preference: %s" % mode)
		return
	if view_mode_preference == mode:
		return
	view_mode_preference = mode
	_save_settings()
	print("[SaveManager] View mode preference updated: %s" % mode)


func get_view_mode_preference() -> String:
	"""Return the persisted match view mode preference."""
	return view_mode_preference
