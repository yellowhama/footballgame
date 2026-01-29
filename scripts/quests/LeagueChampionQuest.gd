extends Quest
class_name LeagueChampionQuest

# Quest to win the youth league in any year.


func _init():
	id = 101  # Unique ID for this quest
	quest_name = "리그 챔피언"
	quest_description = "소속된 유스 리그에서 우승을 차지하여 팀의 위상을 높이세요."
	quest_objective = "시즌 종료 시 리그 순위 1위 달성"


func start(params: Dictionary = {}):
	print("[%s] 퀘스트 시작: %s" % [self.get_class(), quest_name])
	super.start(params)

	# Connect to the season end event
	var subscription := Callable(self, "_on_season_ended")
	if not EventBus.is_connected("season_ended", subscription):
		EventBus.subscribe("season_ended", subscription)

	EventBus.emit("quest_started", {"quest_id": id, "quest_name": quest_name, "quest_type": "seasonal"})


func _on_season_ended(data: Dictionary):
	print("[%s] 시즌 종료 이벤트 수신. 우승 여부 확인..." % self.get_class())
	# data is expected to contain final league table or player's team rank
	var player_team_rank = data.get("player_team_rank", -1)

	if player_team_rank == 1:
		print("[%s] ✅ 리그 우승 확인! 목표 달성." % self.get_class())
		objective_completed = true
		complete()
	else:
		print("[%s] ❌ 리그 우승 실패 (순위: %d). 퀘스트 실패 처리." % [self.get_class(), player_team_rank])
		# Optionally, handle quest failure logic here.


func complete(params: Dictionary = {}):
	super.complete(params)
	print("[%s] 🎉 리그 챔피언 퀘스트 완료!" % self.get_class())

	# Unsubscribe from event to prevent future triggers
	EventBus.unsubscribe("season_ended", Callable(self, "_on_season_ended"))

	# Give rewards
	var rewards = {"ca_bonus": 5, "reputation": 20}
	EventBus.emit("quest_rewards_given", {"quest_id": id, "rewards": rewards})
