extends Quest
class_name CaptainsArmbandQuest

# Quest to be elected as team captain.


func _init():
	id = 103  # Unique ID for this quest
	quest_name = "주장의 품격"
	quest_description = "팀원들과 코칭 스태프의 신임을 얻어 팀의 주장으로 임명되세요."
	quest_objective = "주장으로 임명되기"


func start(params: Dictionary = {}):
	print("[%s] 퀘스트 시작: %s" % [self.get_class(), quest_name])
	super.start(params)

	# Connect to the player became captain event
	var subscription := Callable(self, "_on_player_became_captain")
	if not EventBus.is_connected("player_became_captain", subscription):
		EventBus.subscribe("player_became_captain", subscription)

	EventBus.emit("quest_started", {"quest_id": id, "quest_name": quest_name, "quest_type": "reputation"})


func _on_player_became_captain(data: Dictionary):
	print("[%s] ✅ 주장 임명 확인! 목표 달성." % self.get_class())
	objective_completed = true
	complete()


func complete(params: Dictionary = {}):
	super.complete(params)
	print("[%s] 🎉 주장 완장 퀘스트 완료!" % self.get_class())

	# Unsubscribe from event
	EventBus.unsubscribe("player_became_captain", Callable(self, "_on_player_became_captain"))

	# Give rewards
	var rewards = {"ca_bonus": 3, "reputation": 50, "unlocks": "leadership_special_ability"}
	EventBus.emit("quest_rewards_given", {"quest_id": id, "rewards": rewards})
