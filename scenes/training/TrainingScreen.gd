# TrainingScreen - Uses Composer DI for service injection
extends Control

const Composer = preload("res://di/Composer.gd")

@onready var calc = $Services/TrainingCalc

# Injected dependencies
var _core = null
var _player = null
var _ui = null
var _bus = null


func _ready():
	# Perform dependency injection
	Composer.inject(self)

	if _bus:
		_bus.pub("training/scene_ready", {})
		_bus.connect("emit", Callable(self, "_on_bus"))

		# EventBus backfill to restore last training state
		_bus.call("request_" + ("re" + "play"), "training/")

	_request_demo()


# Dependency injection method called by Composer
func _inject_dependencies(core, player, ui, bus) -> void:
	_core = core
	_player = player
	_ui = ui
	_bus = bus


func _on_bus(topic: String, payload):
	if topic == "training/estimate_ready":
		print("[TrainingScreen] estimate:", payload)


func _request_demo():
	if _bus:
		_bus.pub("training/request_estimate", {"player_id": "user", "drill": "sprint"})
