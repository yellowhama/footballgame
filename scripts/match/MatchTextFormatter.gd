class_name MatchTextFormatter
## Helper to convert OpenFootball timeline events into readable text.

const TEAM_NAMES := {0: "HOME", 1: "AWAY"}


static func format_events(events: Array, rosters: Dictionary = {}) -> String:
	if events.is_empty():
		return "이 경기에 기록된 이벤트가 없습니다."

	var player_names := _build_player_name_map(rosters)
	var lines: Array = []
	lines.append("=== 경기 이벤트 로그 ===")

	for idx in range(events.size()):
		var event = events[idx]
		if typeof(event) != TYPE_DICTIONARY:
			continue
		lines.append(_format_event_line(idx, event, player_names, rosters))

	lines.append("")
	lines.append("총 이벤트 수: %d" % events.size())
	return "\n".join(lines)


static func _build_player_name_map(rosters: Dictionary) -> Dictionary:
	var map := {}
	for side_key in ["home", "away"]:
		if not rosters.has(side_key):
			continue
		var roster: Variant = rosters.get(side_key, {})
		if typeof(roster) != TYPE_DICTIONARY:
			continue
		var players_variant: Variant = roster.get("players", [])
		if not (players_variant is Array):
			continue
		for player in players_variant:
			if typeof(player) != TYPE_DICTIONARY:
				continue
			var pid := str(player.get("id", player.get("player_id", "")))
			if pid == "":
				continue
			map[pid] = str(player.get("name", "Player %s" % pid))
	return map


static func _format_event_line(idx: int, event: Dictionary, player_names: Dictionary, rosters: Dictionary) -> String:
	var base_variant: Variant = event.get("base", {})
	var base: Dictionary = base_variant if typeof(base_variant) == TYPE_DICTIONARY else {}
	var minute: float = float(base.get("t", event.get("minute", base.get("minute", 0.0))))
	var team_id: int = int(base.get("team_id", event.get("team_id", base.get("team", 0))))
	var team_label: String = TEAM_NAMES.get(team_id, "TEAM%s" % str(team_id))
	var kind := str(event.get("kind", event.get("type", "이벤트"))).to_lower()

	var player_id := str(event.get("player_id", base.get("player_id", "")))
	var player_name := _resolve_player_name(event, player_id, player_names)

	var text := "[%d] %0.1f' [%s] %s" % [idx + 1, minute, team_label, _humanize_kind(kind)]
	if player_name != "":
		text += " - %s" % player_name

	var extra := _collect_extra_details(event)
	if extra != "":
		text += " (%s)" % extra

	return text


static func _resolve_player_name(event: Dictionary, player_id: String, player_names: Dictionary) -> String:
	if event.has("player_name"):
		return str(event["player_name"])
	if player_id != "" and player_names.has(player_id):
		return player_names[player_id]
	if player_id != "":
		return "선수 %s" % player_id
	return ""


static func _humanize_kind(kind: String) -> String:
	match kind:
		"goal":
			return "골"
		"assist":
			return "도움"
		"shoot":
			return "슈팅"
		"pass", "ball_move":
			return "패스"
		"foul":
			return "파울"
		"kick_off":
			return "킥오프"
		"half_time":
			return "하프타임"
		"full_time":
			return "경기 종료"
		"yellow_card":
			return "경고"
		"red_card":
			return "퇴장"
		_:
			return kind.capitalize()


static func _collect_extra_details(event: Dictionary) -> String:
	var fields := ["result", "to", "from", "assist_player", "target_player"]
	var parts: Array = []
	for field in fields:
		if event.has(field):
			parts.append("%s=%s" % [field, str(event[field])])
	return ", ".join(parts)
