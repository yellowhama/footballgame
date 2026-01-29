# UIService - Centralized UI helpers (toast/modal/hud events)
# Autoload as: Name=UIService, Path=res://autoload/services/ui_service.gd, Singleton=On

extends Node
# Preload to avoid autoload order issues with class_name
const _TrainingEventPayload = preload("res://scripts/utils/TrainingEventPayload.gd")

# UI Event Signals
signal toast(text: String, seconds: float)
signal modal_opened(title: String, body: String, buttons: Array)
signal notification(message: String, type: String)
signal hud_update(element: String, data: Dictionary)

# UI State Management
var _active_modals: Array[String] = []
var _toast_queue: Array[Dictionary] = []
var _hud_elements: Dictionary = {}


func _ready() -> void:
	print("[UIService] ready")
	_connect_training_events()


func _connect_training_events() -> void:
	if has_node("/root/EventBus"):
		var event_bus = get_node("/root/EventBus")
		if event_bus and event_bus.has_method("subscribe"):
			event_bus.subscribe("training_completed", _on_training_completed_event)


# Toast System
func show_toast(text: String, seconds: float = 2.0) -> void:
	print("[UIService] toast: ", text)
	_toast_queue.append({"text": text, "duration": seconds, "timestamp": Time.get_unix_time_from_system()})
	toast.emit(text, seconds)


func show_success_toast(text: String) -> void:
	show_toast("✅ " + text, 3.0)


func show_error_toast(text: String) -> void:
	show_toast("❌ " + text, 4.0)


func show_warning_toast(text: String) -> void:
	show_toast("⚠️ " + text, 3.5)


# Modal System
func open_modal(title: String, body: String, buttons: Array = ["OK"]) -> void:
	print("[UIService] modal: ", title)
	_active_modals.append(title)
	modal_opened.emit(title, body, buttons)


func close_modal(title: String) -> void:
	_active_modals.erase(title)
	print("[UIService] modal closed: ", title)


func is_modal_active(title: String = "") -> bool:
	if title == "":
		return _active_modals.size() > 0
	return title in _active_modals


# Notification System
func show_notification(message: String, type: String = "info") -> void:
	print("[UIService] notification [%s]: %s" % [type, message])
	notification.emit(message, type)


# HUD Management
func update_hud_element(element: String, data: Dictionary) -> void:
	var payload := data.duplicate(true) if data is Dictionary else data
	_hud_elements[element] = payload
	_persist_training_state_if_needed(element, payload)
	hud_update.emit(element, payload)
	print("[UIService] HUD update: %s -> %s" % [element, str(payload)])


func get_hud_element(element: String) -> Dictionary:
	return _hud_elements.get(element, {})


func _persist_training_state_if_needed(element: String, payload):
	if element != "training_last_result":
		return
	if not (payload is Dictionary):
		return
	var store = get_node_or_null("/root/TrainingStateStore")
	if store and store.has_method("set_last_result"):
		store.set_last_result(payload)


# Specialized UI Helpers for Football Game
func show_stat_increase(stat_name: String, old_value: int, new_value: int) -> void:
	var increase = new_value - old_value
	if increase > 0:
		if increase >= 5:
			show_special_growth_effect(stat_name, old_value, new_value, increase)
		else:
			show_success_toast("%s: %d → %d (+%d)" % [stat_name, old_value, new_value, increase])


func show_special_growth_effect(stat_name: String, old_value: int, new_value: int, growth_amount: int) -> void:
	"""Display special effects for significant growth (+5 or more) - FR-013 Implementation"""
	print("[UIService] Special Growth Effect: %s +%d (FR-013)" % [stat_name, growth_amount])

	# Enhanced toast with special formatting
	show_success_toast("⭐ %s: %d → %d (+%d) ⭐" % [stat_name, old_value, new_value, growth_amount])

	# Play special growth sound via SoundManager
	_play_special_growth_sound()

	# Screen pulse effect using Anima (placeholder for now)
	_trigger_screen_pulse_effect()

	# Emit special signal for additional handling
	notification.emit("Special growth achieved: %s +%d" % [stat_name, growth_amount], "special_growth")


func _trigger_screen_pulse_effect() -> void:
	"""Trigger screen pulse effect for special growth using Tween"""
	# Get the current scene root for full screen effect
	var current_scene = get_tree().current_scene
	if not current_scene:
		print("[UIService] No current scene for pulse effect")
		return

	# Create a brief modulation effect using Godot Tween
	# This creates a subtle "flash" effect to highlight special growth
	var tween = get_tree().create_tween()
	tween.tween_property(current_scene, "modulate", Color(1.2, 1.2, 1.0, 1.0), 0.15).set_ease(Tween.EASE_OUT)
	tween.tween_property(current_scene, "modulate", Color.WHITE, 0.15).set_ease(Tween.EASE_IN)

	print("[UIService] Screen pulse effect triggered with Tween")


func _play_special_growth_sound() -> void:
	"""Play special growth sound effect using SoundManager"""
	# Check if SoundManager exists
	if not has_node("/root/SoundManager"):
		print("[UIService] SoundManager not found for growth sound")
		return

	var sound_manager = get_node("/root/SoundManager")

	# Try to load growth sound file
	var growth_sound_path = "res://assets/audio/ui/growth_special.ogg"
	if ResourceLoader.exists(growth_sound_path):
		var growth_sound = load(growth_sound_path)
		if growth_sound is AudioStream:
			sound_manager.ui_sound_effects.play(growth_sound)
			print("[UIService] Special growth sound played")
			return

	# Fallback: try alternative sound paths
	var fallback_paths = [
		"res://assets/audio/ui/success.ogg",
		"res://assets/audio/sfx/level_up.ogg",
		"res://assets/audio/sfx/achievement.ogg"
	]

	for path in fallback_paths:
		if ResourceLoader.exists(path):
			var fallback_sound = load(path)
			if fallback_sound is AudioStream:
				sound_manager.ui_sound_effects.play(fallback_sound)
				print("[UIService] Fallback growth sound played: %s" % path)
				return

	# If no audio files exist, just log
	print("[UIService] No growth sound files found - silent mode")


func show_level_up(player_name: String, level: int) -> void:
	open_modal("Level Up!", "%s reached level %d!" % [player_name, level], ["Continue"])


func show_match_result(home_score: int, away_score: int, is_win: bool) -> void:
	var result_text = "Match Result: %d - %d" % [home_score, away_score]
	var modal_title = "Victory!" if is_win else "Defeat"

	if is_win:
		show_success_toast("Match won %d-%d!" % [home_score, away_score])
	else:
		show_warning_toast("Match lost %d-%d" % [home_score, away_score])

	open_modal(modal_title, result_text, ["Continue"])


func show_training_complete(training_name: String, stats_gained: Dictionary) -> void:
	var gains_text = ""
	for stat in stats_gained:
		gains_text += "%s +%d\n" % [stat, stats_gained[stat]]

	show_success_toast("Training completed: %s" % training_name)
	if not gains_text.is_empty():
		open_modal("Training Complete", "Stats improved:\n%s" % gains_text, ["OK"])


func show_training_complete_with_growth(training_name: String, attribute_growths: Array) -> void:
	"""Enhanced training complete with AttributeGrowth array from OpenFootball - FR-013"""
	if attribute_growths.is_empty():
		show_success_toast("Training completed: %s" % training_name)
		return

	var gains_text = ""
	var has_special_growth = false

	# Process each AttributeGrowth and check for special effects
	for growth_data in attribute_growths:
		# Handle both Dictionary and direct values
		var stat_name = ""
		var growth_amount = 0
		var old_value = 0
		var new_value = 0

		if growth_data is Dictionary:
			stat_name = growth_data.get("attribute_name", "Unknown")
			growth_amount = growth_data.get("growth_amount", 0)
			old_value = growth_data.get("old_value", 0)
			new_value = growth_data.get("new_value", 0)
		else:
			# Handle simple format from existing code
			continue

		gains_text += "%s +%d (%d → %d)\n" % [stat_name, growth_amount, old_value, new_value]

		# Trigger special effects for significant growth (+5 or more) - FR-013
		if growth_amount >= 5:
			has_special_growth = true
			show_special_growth_effect(stat_name, old_value, new_value, growth_amount)

	# Show training completion notification
	if has_special_growth:
		show_success_toast("🌟 Training completed: %s - SPECIAL GROWTH!" % training_name)
	else:
		show_success_toast("Training completed: %s" % training_name)

	# Show detailed modal if there are gains
	if not gains_text.is_empty():
		var modal_title = "🌟 Special Training Complete!" if has_special_growth else "Training Complete"
		open_modal(modal_title, "Stats improved:\n%s" % gains_text, ["OK"])

	print(
		(
			"[UIService] Training complete with %d growths, special effects: %s"
			% [attribute_growths.size(), has_special_growth]
		)
	)


# Debug helpers
func get_ui_state() -> Dictionary:
	return {
		"active_modals": _active_modals.duplicate(),
		"toast_queue_size": _toast_queue.size(),
		"hud_elements": _hud_elements.keys()
	}


func _on_training_completed_event(event: Dictionary) -> void:
	var payload: Dictionary = _TrainingEventPayload.normalize(event)
	if payload.is_empty():
		return

	var summary_parts: PackedStringArray = PackedStringArray()
	summary_parts.append(
		(
			"%s: %s (%s · %s)"
			% [
				tr("UI_TRAINING_RESULT_FIELD_SUMMARY"),
				payload.training_name,
				payload.mode_label,
				payload.intensity_label
			]
		)
	)

	if payload.deck_bonus_pct != 0:
		summary_parts.append("%s +%d%%" % [tr("UI_TRAINING_RESULT_FIELD_DECK_BONUS"), payload.deck_bonus_pct])

	var coach_logs: Array = payload.get("coach_bonus_log", [])
	if coach_logs is Array and coach_logs.size() > 0:
		summary_parts.append("%s %d" % [tr("UI_TRAINING_RESULT_FIELD_COACH_BONUS"), coach_logs.size()])

	if payload.needs_rest_warning:
		summary_parts.append(tr("UI_TRAINING_SUMMARY_REST_WARNING"))

	if payload.injury_risk >= 0.0:
		summary_parts.append("%s %.0f%%" % [tr("UI_TRAINING_RESULT_FIELD_INJURY"), payload.injury_risk * 100.0])

	var note_text: String = String(payload.ui_note)
	if not note_text.is_empty():
		summary_parts.append(note_text)

	show_success_toast(" | ".join(summary_parts))

	var hud_payload: Dictionary = {
		"name": payload.training_name,
		"mode": payload.mode_label,
		"mode_id": payload.mode_id,
		"intensity": payload.intensity_label,
		"intensity_id": payload.intensity_id,
		"deck_bonus": payload.deck_bonus_pct,
		"deck_snapshot": payload.deck_snapshot,
		"timestamp": payload.timestamp,
		"changes": payload.changes,
		"training_load": payload.training_load,
		"injury_risk": payload.injury_risk,
		"coach_bonus_log": payload.coach_bonus_log,
		"deck_bonus_data": payload.deck_bonus,
		"needs_rest_warning": payload.needs_rest_warning,
		"condition_cost": payload.condition_cost,
		"ui_note": payload.ui_note,
		"description": payload.description
	}
	update_hud_element("training_last_result", hud_payload)
