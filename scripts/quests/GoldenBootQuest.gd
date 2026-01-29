extends Quest
class_name GoldenBootQuest

# Quest to become the top scorer in the league.


func _init():
	id = 102  # Unique ID for this quest
	quest_name = "골든 부츠"
	quest_description = "리그 최고의 골잡이가 되어 당신의 이름을 증명하세요."
	quest_objective = "시즌 종료 시 리그 득점 순위 1위 달성"


func start(params: Dictionary = {}):
	print("[%s] 퀘스트 시작: %s" % [self.get_class(), quest_name])
	super.start(params)

	# Connect to the season end event
	var subscription := Callable(self, "_on_season_ended")
	if not EventBus.is_connected("season_ended", subscription):
		EventBus.subscribe("season_ended", subscription)

	EventBus.emit("quest_started", {"quest_id": id, "quest_name": quest_name, "quest_type": "seasonal_performance"})


func _on_season_ended(data: Dictionary):
	print("[%s] 시즌 종료 이벤트 수신. 득점왕 여부 확인..." % self.get_class())
	# data is expected to contain player_stats and league_top_scorer info
	var player_goals = data.get("player_goals", 0)
	var top_scorer_goals = data.get("league_top_scorer_goals", -1)

	if player_goals >= top_scorer_goals and player_goals > 0:
		print("[%s] ✅ 득점왕 달성 확인! (득점: %d). 목표 달성." % [self.get_class(), player_goals])
		objective_completed = true
		complete()
	else:
		print("[%s] ❌ 득점왕 달성 실패 (득점: %d, 1위: %d)." % [self.get_class(), player_goals, top_scorer_goals])


func complete(params: Dictionary = {}):
	super.complete(params)
	print("[%s] 🎉 골든 부츠 퀘스트 완료!" % self.get_class())

	# Unsubscribe from event
	EventBus.unsubscribe("season_ended", Callable(self, "_on_season_ended"))

	# Give rewards
	var rewards = {"ca_bonus": 7, "reputation": 30, "unlocks": "finishing_special_ability"}  # Higher bonus for individual achievement
	EventBus.emit("quest_rewards_given", {"quest_id": id, "rewards": rewards})
