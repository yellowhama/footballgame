## MatchTimeFormatter - Helpers for displaying engine minutes in UI.
##
## Goal: avoid hardcoding 45'/90' across UI screens while keeping display logic
## out of the engine SSOT.
extends RefCounted


static func extract_timeline_events(match_record: Dictionary) -> Array:
	# Prefer normalized engine timeline event arrays if present.
	var direct: Variant = match_record.get("timeline_events", null)
	if direct is Array:
		return direct as Array

	var match_result: Variant = match_record.get("match_result", null)
	if match_result is Dictionary:
		var events_variant: Variant = (match_result as Dictionary).get("events", null)
		if events_variant is Array:
			return events_variant as Array

	var raw_result: Variant = match_record.get("raw_result", null)
	if raw_result is Dictionary:
		var events_variant: Variant = (raw_result as Dictionary).get("events", null)
		if events_variant is Array:
			return events_variant as Array

	return []


static func _event_type_string(ev: Dictionary) -> String:
	if ev.has("type"):
		return str(ev.get("type", "")).to_lower()
	if ev.has("event_type"):
		return str(ev.get("event_type", "")).to_lower()
	if ev.has("kind"):
		return str(ev.get("kind", "")).to_lower()
	return ""


static func _event_minute_int(ev: Dictionary) -> int:
	# Common keys: minute, t
	if ev.has("minute"):
		return int(ev.get("minute", 0))
	if ev.has("t"):
		return int(ev.get("t", 0))
	return 0


static func find_event_minute(events: Array, wanted_types: Array[String], fallback_minute: int) -> int:
	if events.is_empty():
		return fallback_minute

	var wanted := []
	for t in wanted_types:
		wanted.append(str(t).to_lower())

	for entry in events:
		if not (entry is Dictionary):
			continue
		var ev: Dictionary = entry
		var et := _event_type_string(ev)
		if wanted.has(et):
			var minute := _event_minute_int(ev)
			if minute > 0:
				return minute

	return fallback_minute


static func derive_half_time_minute(match_record: Dictionary) -> int:
	var events := extract_timeline_events(match_record)
	return find_event_minute(events, ["half_time", "halftime"], 45)


static func derive_full_time_minute(match_record: Dictionary, half_time_minute: int = 45) -> int:
	var events := extract_timeline_events(match_record)
	var fallback: int = maxi(90, half_time_minute + 45)
	return find_event_minute(events, ["full_time", "fulltime"], fallback)


static func format_minute_label(minute: int, half_time_minute: int = 45) -> String:
	# Engine minutes are elapsed since kickoff (monotonic).
	#
	# UI display prefers football-style labels:
	# - 1H added time: 45+X'
	# - 2H added time: 90+Y'
	#
	# To keep the 2H labels consistent when 1H had added time, we subtract the
	# first-half added minutes from minutes after halftime.
	if minute <= 45:
		return "%d'" % minute

	half_time_minute = max(45, half_time_minute)
	if minute <= half_time_minute:
		return "45+%d'" % (minute - 45)

	var first_half_added := half_time_minute - 45
	var display_minute := minute - first_half_added
	if display_minute <= 90:
		return "%d'" % display_minute
	return "90+%d'" % (display_minute - 90)


static func normalize_event_kind(kind: String) -> String:
	var normalized := kind.strip_edges().to_lower()
	if normalized == "":
		return ""
	normalized = normalized.replace(" ", "_")
	normalized = normalized.replace("-", "_")
	match normalized:
		"varreview":
			return "var_review"
		"halftime":
			return "half_time"
		"fulltime":
			return "full_time"
		"kickoff":
			return "kick_off"
	return normalized


static func format_event_kind_short(kind: String) -> String:
	var normalized := normalize_event_kind(kind)
	match normalized:
		"var_review":
			return "VAR"
		"substitution":
			return "SUB"
		"half_time":
			return "HT"
		"full_time":
			return "FT"
	return kind.strip_edges()


static func format_event_kind_display(kind: String) -> String:
	var normalized := normalize_event_kind(kind)
	match normalized:
		"var_review":
			return "VAR"
		"substitution":
			return "SUB"
		"half_time":
			return "HT"
		"full_time":
			return "FT"
	if normalized == "":
		return ""
	var words := PackedStringArray()
	for part in normalized.split("_", false):
		if part == "":
			continue
		if part == "xg":
			words.append("xG")
		else:
			words.append(part.capitalize())
	return " ".join(words) if not words.is_empty() else kind.strip_edges()


static func extract_penalty_shootout(match_record: Dictionary) -> Dictionary:
	var candidates: Array = []

	# Most common: match_record.match_result.penalty_shootout
	var match_result: Variant = match_record.get("match_result", null)
	if match_result is Dictionary:
		candidates.append((match_result as Dictionary).get("penalty_shootout", null))

	# Some payloads may expose it at top-level.
	candidates.append(match_record.get("penalty_shootout", null))

	# Legacy nesting.
	var raw_result: Variant = match_record.get("raw_result", null)
	if raw_result is Dictionary:
		candidates.append((raw_result as Dictionary).get("penalty_shootout", null))
		var nested: Variant = (raw_result as Dictionary).get("match_result", null)
		if nested is Dictionary:
			candidates.append((nested as Dictionary).get("penalty_shootout", null))

	for cand in candidates:
		if cand is Dictionary:
			return cand as Dictionary
		if cand is String:
			var parsed: Variant = JSON.parse_string(String(cand))
			if parsed is Dictionary:
				return parsed as Dictionary
	return {}


static func format_penalty_shootout_suffix(match_record: Dictionary) -> String:
	var shootout := extract_penalty_shootout(match_record)
	if shootout.is_empty():
		return ""
	var goals_home := int(shootout.get("goals_home", 0))
	var goals_away := int(shootout.get("goals_away", 0))
	return " (PEN %d-%d)" % [goals_home, goals_away]

