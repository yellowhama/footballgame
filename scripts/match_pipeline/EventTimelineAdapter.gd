extends RefCounted
class_name EventTimelineAdapter
##
## EventTimelineAdapter
##
## MatchSessionController / MatchSimulationScreen 에서 전달받은
## "표준 이벤트 스키마" 배열을 타임라인 마커 배열로 변환한다.
##
## 입력 이벤트(예: EventSchemaAdapter.adapt_events() 출력):
## {
##   "minute": int,
##   "t": float,                # 초 단위 시간 (선택)
##   "type": String,            # "goal", "shot_on_target" 등
##   "team": String,            # 팀 이름
##   "is_home_team": bool,      # 홈팀 여부
##   "player": String,          # 선수 이름 (선택)
##   "player_id": String,       # 선수 ID (선택)
##   "details": Dictionary,     # 추가 메타데이터
## }
##
## 출력 마커(MatchTimelineControls 기대 형태):
## {
##   "time_ms": int,
##   "label": String,
##   "team_id": int,      # 0=home, 1=away, -1=neutral
##   "event_type": String # 소문자 타입 ("goal" 등)
## }

const _MatchTimeFormatter = preload("res://scripts/utils/MatchTimeFormatter.gd")

static func events_to_markers(
        events: Array, home_team_name: String, away_team_name: String, existing_markers: Array = []
) -> Array:
	var markers: Array = existing_markers.duplicate(true)

	for ev in events:
		if not (ev is Dictionary):
			continue
		var e: Dictionary = ev

		# 1) 시간 계산: t(초) 우선, 없으면 minute → 초
		var t_sec := -1.0
		if e.has("t"):
			t_sec = float(e.get("t", -1.0))
		if t_sec < 0.0 and e.has("minute"):
			t_sec = float(e.get("minute", 0)) * 60.0
		if t_sec < 0.0:
			# 시간 정보가 없으면 타임라인에 올리지 않는다.
			continue
                var time_ms := int(round(t_sec * 1000.0))

                # 2) 이벤트 타입/라벨
                var event_type: String = _MatchTimeFormatter.normalize_event_kind(
                        str(e.get("type", "event"))
                )
                var team_name := str(e.get("team", ""))
                var player_name := str(e.get("player", ""))

                var event_kind_label := _MatchTimeFormatter.format_event_kind_display(event_type)

                var base_label := ""
                if player_name != "":
                        base_label = player_name
                elif team_name != "":
                        base_label = team_name
                else:
                        base_label = event_kind_label if event_kind_label != "" else event_type.capitalize()

                var label := base_label
                if event_kind_label != "" and base_label != event_kind_label:
                        label = "%s (%s)" % [base_label, event_kind_label]

                # 3) 팀 ID 매핑
                var team_id := -1
                if e.has("is_home_team"):
			var is_home := bool(e.get("is_home_team", false))
			team_id = 0 if is_home else 1
		elif team_name != "":
			if team_name == home_team_name:
				team_id = 0
			elif team_name == away_team_name:
				team_id = 1

		var marker := {
			"time_ms": time_ms,
			"label": label,
			"team_id": team_id,
			"event_type": event_type,
		}
		markers.append(marker)

	return markers
