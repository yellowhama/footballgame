extends Quest
class_name FirstTrainingQuest

# First Training Quest - Triggered after manager meeting
# Player must complete their first training session

const TrainingEventPayload := preload("res://scripts/utils/TrainingEventPayload.gd")

var training_completed: bool = false


func _init() -> void:
	id = 1
	quest_name = "첫 번째 훈련"
	quest_description = "박감독님과의 첫 만남 후, 첫 번째 훈련을 완료해야 합니다."
	quest_objective = "훈련 메뉴에서 아무 훈련이나 한 번 완료하기"


func _ready() -> void:
	print("[FirstTrainingQuest] 첫 번째 훈련 퀘스트 초기화")

	# Event Bus를 통해 훈련 완료 이벤트 구독
	EventBus.subscribe("training_completed", _on_training_completed)
	EventBus.subscribe("bridge_training_completed", _on_bridge_training_completed)

	# Quest 신호 연결
	completed.connect(_on_quest_completed)


func start(params: Dictionary = {}) -> void:
	print("[FirstTrainingQuest] 퀘스트 시작: ", quest_name)
	super.start(params)

	# 퀘스트 시작을 Event Bus로 알림
	(
		EventBus
		. emit(
			"quest_started",
			{
				"quest_id": id,
				"quest_name": quest_name,
				"quest_type": "tutorial",
			}
		)
	)


func _on_training_completed(event: Dictionary) -> void:
	"""TrainingManager 신호 payload 처리"""
	if training_completed:
		return
	var payload = TrainingEventPayload.normalize(event)
	var training_name: String = ""
	var normalized_event: Dictionary = event
	if payload and not payload.is_empty():
		training_name = str(payload.training_name)
		normalized_event = payload.raw_event if payload.raw_event else event
	else:
		training_name = str(event.get("training_id", "Unknown"))
	print("[FirstTrainingQuest] 훈련 완료 감지: ", training_name)
	_complete_training_objective(normalized_event)


func _on_bridge_training_completed(data: Dictionary) -> void:
	"""Bridge의 훈련 완료 이벤트 처리"""
	if not training_completed:
		print("[FirstTrainingQuest] Bridge 훈련 완료 감지: ", data.get("training_type", "Unknown"))
		_complete_training_objective(data)


func _complete_training_objective(training_data: Dictionary) -> void:
	"""훈련 목표 완료 처리"""
	training_completed = true
	objective_completed = true

	print("[FirstTrainingQuest] ✅ 첫 번째 훈련 목표 달성!")

	# 퀘스트 완료
	complete()

	# Event Bus로 목표 달성 알림
	(
		EventBus
		. emit(
			"quest_objective_completed",
			{
				"quest_id": id,
				"objective": "first_training",
				"training_data": training_data,
			}
		)
	)


func _on_quest_completed() -> void:
	"""퀘스트 완료 시 호출"""
	print("[FirstTrainingQuest] 🏁 첫 번째 훈련 퀘스트 완료!")

	# 보상 지급
	_give_completion_rewards()

	# 다음 퀘스트 트리거
	_trigger_next_quest()


func _give_completion_rewards() -> void:
	"""완료 보상 지급"""
	var rewards := {
		"experience": 50,
		"confidence": 5,
		"manager_relationship": 5,
	}

	print("[FirstTrainingQuest] 보상 지급: ", rewards)

	# Event Bus로 보상 지급 알림
	(
		EventBus
		. emit(
			"quest_rewards_given",
			{
				"quest_id": id,
				"rewards": rewards,
			}
		)
	)

	# RelationshipSystem에 관계도 보상 적용
	if RelationshipSystem:
		RelationshipSystem.improve_manager_relationship(rewards.manager_relationship)


func _trigger_next_quest() -> void:
	"""다음 퀘스트 트리거"""
	# 다음 퀘스트는 "일주일 동안 꾸준히 훈련하기"
	(
		EventBus
		. emit(
			"quest_trigger",
			{
				"quest_id": "weekly_training_routine",
				"source": "first_training_completed",
				"data":
				{
					"previous_quest": id,
					"unlock_condition": "first_training_complete",
				},
			}
		)
	)


# 퀘스트 상태 체크 함수들


func is_objective_completed() -> bool:
	return objective_completed


func get_progress_text() -> String:
	if objective_completed:
		return "✅ 첫 번째 훈련 완료"
	else:
		return "🟡 첫 번째 훈련 완료하기"


func get_detailed_progress() -> Dictionary:
	return {
		"training_completed": training_completed,
		"progress_percentage": 100 if training_completed else 0,
		"status": "completed" if objective_completed else "in_progress",
	}
