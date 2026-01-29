extends RefCounted
class_name EventSchemaAdapter

## EventSchemaAdapter
##
## session step API에서 넘어오는 이벤트들을 배치(timeline) 경로에서 사용하는
## 공통 이벤트 스키마로 어댑트하기 위한 유틸리티.
##
## 입력 예시 (session/raw):
##   {
##     "kind": "shot_on_target",
##     "team_id": 0,
##     "player_id": "H9",
##     "player": "조서준",
##     "minute": 64,
##     "t_ms": 3840000,
##     "xg": 0.27
##   }
##
## 출력 예시 (batch/standard):
##   {
##     "minute": 64,
##     "t": 3840.0,
##     "type": "shot_on_target",
##     "team": "Home FC",
##     "is_home_team": true,
##     "player": "조서준",
##     "player_id": "H9",
##     "details": { "xg": 0.27 }
##   }


static func adapt_events(raw_events: Array, t_ms: int, home_team_name: String, away_team_name: String) -> Array:
	var adapted: Array = []
	if raw_events.is_empty():
		return adapted

	var default_t_sec := float(t_ms) / 1000.0

	for ev in raw_events:
		if not (ev is Dictionary):
			continue
		var src: Dictionary = ev

		var t_sec := float(src.get("t", default_t_sec))
		if not src.has("t") and src.has("t_ms"):
			t_sec = float(src.get("t_ms", t_ms)) / 1000.0
		var minute := int(src.get("minute", t_sec / 60.0))

		var event_type := str(src.get("type", src.get("kind", "event")))

		var team_id := int(src.get("team_id", -1))
		var team_name := ""
		var is_home := false
		match team_id:
			0:
				team_name = home_team_name
				is_home = true
			1:
				team_name = away_team_name
				is_home = false
			_:
				team_name = str(src.get("team", ""))
				if team_name == home_team_name:
					is_home = true
				elif team_name == away_team_name:
					is_home = false

		var player_name := str(src.get("player", src.get("player_name", "")))
		var player_id := str(src.get("player_id", ""))

		var details: Dictionary = {}
		if src.has("details") and src.details is Dictionary:
			details = (src.details as Dictionary).duplicate(true)

		for key in ["xg", "distance_m", "speed_mps"]:
			if src.has(key) and not details.has(key):
				details[key] = src.get(key)

		var out := {
			"minute": minute,
			"t": t_sec,
			"type": event_type,
			"team": team_name,
			"is_home_team": is_home,
			"player": player_name,
			"player_id": player_id,
			"details": details,
		}

		adapted.append(out)

	return adapted
