extends RefCounted
class_name TrainingEventPayload

const TRAINING_INTENSITY_LABELS := {
	"light": "UI_TRAINING_INTENSITY_LIGHT",
	"normal": "UI_TRAINING_INTENSITY_NORMAL",
	"intense": "UI_TRAINING_INTENSITY_INTENSE"
}

const TRAINING_MODE_LABELS := {
	"team": "UI_TRAINING_MODE_TEAM", "special": "UI_TRAINING_MODE_SPECIAL", "personal": "UI_TRAINING_MODE_PERSONAL"
}


static func normalize(event: Dictionary) -> Dictionary:
	if event == null:
		return {}

	var normalized: Dictionary = event.duplicate(true)
	var result: Dictionary = normalized.get("result", {})
	var payload: Dictionary = {"raw_event": normalized, "result": result.duplicate(true) if result else {}}

	var training_id: String = String(normalized.get("training_id", ""))
	payload["training_id"] = training_id
	payload["training_name"] = String(result.get("training_name", normalized.get("training_name", training_id)))
	payload["program_type"] = String(normalized.get("program_type", result.get("program_type", "")))

	var mode_id: String = String(normalized.get("mode", result.get("mode", "personal")))
	payload["mode_id"] = mode_id
	payload["mode_label"] = _describe_training_mode(mode_id)

	var intensity_id: String = String(normalized.get("intensity", result.get("intensity", "normal")))
	payload["intensity_id"] = intensity_id
	payload["intensity_label"] = _describe_intensity(intensity_id)

	var deck_bonus_raw: Variant = normalized.get("deck_bonus", result.get("deck_bonus", {}))
	var deck_bonus: Dictionary = deck_bonus_raw if deck_bonus_raw is Dictionary else {}
	payload["deck_bonus"] = deck_bonus
	payload["deck_bonus_pct"] = _calculate_deck_bonus_pct(deck_bonus)
	payload["deck_snapshot"] = normalized.get("deck_snapshot", result.get("deck_snapshot", []))

	payload["timestamp"] = int(normalized.get("timestamp", result.get("timestamp", Time.get_unix_time_from_system())))
	payload["changes"] = result.get("changes", {})
	payload["training_load"] = result.get("training_load", {})
	payload["injury_risk"] = float(result.get("injury_risk", -1.0))
	payload["needs_rest_warning"] = bool(result.get("needs_rest_warning", false))
	payload["coach_bonus_log"] = result.get("coach_bonus_log", result.get("coach_bonus_breakdown", []))
	var deck_snapshot: Variant = payload["deck_snapshot"]
	payload["active_deck"] = deck_snapshot if deck_snapshot is Array else []
	payload["ui_note"] = normalized.get("ui_note", result.get("ui_note", ""))
	payload["description"] = normalized.get("description", result.get("description", ""))
	payload["condition_cost"] = float(normalized.get("condition_cost", result.get("condition_cost", 0.0)))

	return payload


static func _describe_training_mode(mode_id: String) -> String:
	var key: String = TRAINING_MODE_LABELS.get(mode_id, TRAINING_MODE_LABELS["personal"])
	return TranslationServer.translate(key)


static func _describe_intensity(intensity_id: String) -> String:
	var key: String = TRAINING_INTENSITY_LABELS.get(intensity_id, "")
	if not key.is_empty():
		return TranslationServer.translate(key)
	return intensity_id.capitalize()


static func _calculate_deck_bonus_pct(deck_bonus: Dictionary) -> int:
	if deck_bonus.is_empty():
		return 0
	var total: float = float(deck_bonus.get("total_bonus", deck_bonus.get("total", 0.0)))
	return int(round(total * 100.0))
