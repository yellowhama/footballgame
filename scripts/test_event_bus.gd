extends Node
# EventBus 테스트 스크립트 - 기본 기능 확인용


func _ready():
	print("=== Event Bus 테스트 시작 ===")

	# 테스트 이벤트 구독
	EventBus.subscribe("test_training_selected", _on_training_selected)
	EventBus.subscribe("test_training_hovered", _on_training_hovered)

	# 3초 후 테스트 이벤트 발생
	await get_tree().create_timer(3.0).timeout
	test_emit_events()


func test_emit_events():
	print("테스트 이벤트 발생...")

	# 훈련 선택 이벤트 테스트
	var training_data = {"training_type": "Physical_Endurance", "intensity": 1.2, "fatigue_cost": 15}
	EventBus.emit("test_training_selected", training_data)

	# 훈련 호버 이벤트 테스트
	var hover_data = {"training_type": "Technical_Shooting", "description": "슈팅 정확도 향상 훈련"}
	EventBus.emit("test_training_hovered", hover_data)

	print("B키를 눌러 Event Bus 모니터링 UI를 확인하세요!")


func _on_training_selected(data: Dictionary):
	print("✅ training_selected 이벤트 수신: ", data)


func _on_training_hovered(data: Dictionary):
	print("✅ training_hovered 이벤트 수신: ", data)
