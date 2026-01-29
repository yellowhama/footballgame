extends RefCounted
class_name TimelinePlayerMapper


##
## Utility: Build per-track player metadata for Timeline viewers.
##
## Input:
##   match_doc   - MRB0 header JSON (should contain rosters/teams/etc.)
##   position_data - { "players": { track_id: [ {t,x,y}... ], ... }, ... }
##
## Output:
##   Dictionary keyed by track_id(String) -> { "team": "home"|"away", "position": String, "name": String }
##
static func build_player_meta(match_doc: Dictionary, position_data: Dictionary) -> Dictionary:
	var result: Dictionary = {}

	# Extract tracks present in position_data
	var track_ids: Array = []
	if position_data.has("players") and position_data.players is Dictionary:
		for key in position_data.players.keys():
			track_ids.append(str(key))
	track_ids.sort()

	if match_doc.is_empty():
		_assign_fallback_meta(result, track_ids)
		return result

	var rosters: Dictionary = {}
	if match_doc.has("rosters") and match_doc.rosters is Dictionary:
		rosters = match_doc.rosters

	# Import one side (home/away) into result
	_import_side(rosters, result, "home", "home")
	_import_side(rosters, result, "away", "away")

	# If track_ids exist but some IDs are missing in result, fill minimal meta.
	if not track_ids.is_empty():
		if result.is_empty():
			_assign_fallback_meta(result, track_ids)
		else:
			for i in range(track_ids.size()):
				var tid: String = str(track_ids[i])
				if result.has(tid):
					continue
				var team_key: String = "home" if i < 11 else "away"
				result[tid] = {
					"team": team_key,
					"position": "",
					"name": tid,
				}

	return result


static func _import_side(rosters: Dictionary, result: Dictionary, side: String, team_key: String) -> void:
	if not rosters.has(side):
		return
	var side_block = rosters.get(side)
	if not (side_block is Dictionary):
		return
	var players: Variant = side_block.get("players", [])
	if not (players is Array):
		return
	for p in players:
		if not (p is Dictionary):
			continue
		var pid_val: Variant = p.get("id", p.get("player_id", null))
		if pid_val == null:
			continue
		var pid_str := str(pid_val)
		var name_str := str(p.get("name", p.get("display_name", pid_str)))
		var pos_str := str(p.get("position", p.get("role_id", "")))
		result[pid_str] = {
			"team": team_key,
			"position": pos_str,
			"name": name_str,
		}


static func _assign_fallback_meta(result: Dictionary, track_ids: Array) -> void:
	var sorted_ids: Array = track_ids.duplicate()
	sorted_ids.sort()
	for i in range(sorted_ids.size()):
		var tid: String = str(sorted_ids[i])
		var team_key: String = "home" if i < 11 else "away"
		result[tid] = {
			"team": team_key,
			"position": "",
			"name": tid,
		}
