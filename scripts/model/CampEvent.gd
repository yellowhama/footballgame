class_name CampEvent
extends Resource

# 간단한 타임라인: 대사/화면 텍스트/효과
@export var title: String = ""
@export var term: String = "summer"  # "summer" | "winter"
@export var timeline: Array[Dictionary] = []
# 예: [{"type":"text","speaker":"코치","content":"이번 여름 합숙은 피지컬이다."},
#      {"type":"buff","effect":{"fatigue":+5,"morale":+3}},
#      {"type":"choice","content":"추가 세션?","options":[{"label":"스프린트","value":"sprint"},{"label":"근력","value":"strength"}]}]


func create_default_summer() -> CampEvent:
	var event = CampEvent.new()
	event.title = "여름 합숙 시작"
	event.term = "summer"
	event.timeline = [
		{"type": "text", "speaker": "감독", "content": "이번 여름 합숙은 피지컬 강화에 집중한다."},
		{"type": "text", "speaker": "감독", "content": "2주간 고강도 훈련이 진행될 예정이다."},
		{"type": "buff", "effect": {"morale": 3}},
		{
			"type": "choice",
			"content": "추가 훈련을 선택하시겠습니까?",
			"options":
			[
				{"label": "스프린트 강화", "value": "sprint"},
				{"label": "근력 훈련", "value": "strength"},
				{"label": "휴식", "value": "rest"}
			]
		}
	]
	return event


func create_default_winter() -> CampEvent:
	var event = CampEvent.new()
	event.title = "겨울 합숙 시작"
	event.term = "winter"
	event.timeline = [
		{"type": "text", "speaker": "감독", "content": "겨울 합숙은 전술 이해도와 멘탈 강화에 중점을 둔다."},
		{"type": "text", "speaker": "감독", "content": "영상 분석과 전술 훈련이 주가 될 것이다."},
		{"type": "buff", "effect": {"morale": 2, "fatigue": -5}},
		{"type": "text", "speaker": "", "content": "합숙 일정이 시작되었습니다."}
	]
	return event
