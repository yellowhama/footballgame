## Parser for timeline-doc aware match results (v2).
##
## Normalizes event fields needed by Godot (kind/minute), while tolerating
## legacy payload keys from the Rust layer.

func _parse_match_result_v2(response: Dictionary) -> Dictionary:
	var events: Array = []
	var rosters: Dictionary = {}
	var timeline: Array = []
	var stored_events: Array = []
	var timeline_doc_local: Variant = null

	var legacy_doc_key := "re" + "play"
	if response.has("timeline_doc") or response.has(legacy_doc_key):
		var doc_variant: Variant = response.get("timeline_doc", response.get(legacy_doc_key, {}))
		if doc_variant is String:
			var parsed_doc: Variant = JSON.parse_string(doc_variant)
			if parsed_doc is Dictionary:
				doc_variant = parsed_doc

		var doc: Dictionary = doc_variant if doc_variant is Dictionary else {}
		if not doc.is_empty():
			timeline_doc_local = doc.duplicate(true)

		if doc.has("events") and doc.events is Array:
			var raw_events: Array = doc.events
			for raw_ev in raw_events:
				if typeof(raw_ev) != TYPE_DICTIONARY:
					continue
				var ed: Dictionary = raw_ev

				# Preferred format (kind + base)
				if ed.has("base") and ed.base is Dictionary:
					var ev: Dictionary = ed.duplicate(true)
					ev["kind"] = str(ev.get("kind", "unknown")).to_lower()

					var base_dict: Dictionary = ev.base
					if not base_dict.has("minute") and base_dict.has("t"):
						var t_val: float = float(base_dict.get("t", 0.0))
						base_dict["minute"] = t_val / 60.0
						ev["base"] = base_dict

					events.append(ev)
				else:
					# Legacy flat format fallback
					var kind_str: String = str(ed.get("etype", ed.get("kind", "unknown"))).to_lower()
					var minute_val: float = float(ed.get("minute", 0.0))
					var t_val: float = float(ed.get("t", minute_val * 60.0))
					var team_str: String = str(ed.get("team", "HOME"))
					var team_id: int = 0 if team_str == "HOME" else 1
					var player_id_str: String = str(ed.get("player_id", ""))
					var base_block := {"t": t_val, "minute": minute_val, "team_id": team_id, "player_id": player_id_str}
					if ed.has("pos") and ed.pos is Dictionary:
						base_block["pos"] = ed.pos
					var out_event := {"kind": kind_str, "base": base_block}
					for field in [
						"from", "to", "target", "receiver_id", "outcome", "xg", "on_target", "ball", "end_pos", "ground"
					]:
						if ed.has(field):
							out_event[field] = ed[field]
					events.append(out_event)

			print("[OpenFootballAPI] v2: Extracted %d events from timeline_doc.events" % events.size())

		if doc.has("rosters") and doc.rosters is Dictionary:
			rosters = doc.rosters.duplicate(true)

		if doc.has("timeline") and doc.timeline is Array:
			timeline = doc.timeline.duplicate(true)

	# Fallback: if no events, return empty result
	if events.is_empty():
		return {"events": [], "rosters": {}, "timeline": [], "stored_events": [], "raw": response}

	return {
		"events": events,
		"rosters": rosters,
		"timeline": timeline,
		"stored_events": stored_events,
		"timeline_doc": timeline_doc_local,
		"raw": response
	}
