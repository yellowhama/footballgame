# Scene-local training calculator. No global state; reacts to EventBus.

extends Node
class_name TrainingCalc

var _bus := get_node_or_null("/root/EventBus")


func _ready():
	if _bus:
		_bus.connect("emit", Callable(self, "_on_bus"))


func estimate_gain(profile: Dictionary, drill: String) -> Dictionary:
	var mult := 1.0
	match drill:
		"sprint":
			mult = 1.2
		"pass":
			mult = 1.1
		_:
			mult = 1.0
	return {"xp": int(25 * mult), "stamina_cost": int(10 * mult)}


func _on_bus(topic: String, payload):
	if topic == "training/request_estimate":
		var player = payload.get("player_id", "user")
		var ps = get_node("/root/PlayerService")
		var prof = ps.get_profile(player)
		var out = estimate_gain(prof, payload.get("drill", "sprint"))
		_bus.pub("training/estimate_ready", out)
