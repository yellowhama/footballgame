# Root controller for MatchScreen. Uses Composer DI for service injection.

extends Control

const MatchGatewayFactory = preload("res://bridge/match_gateway_factory.gd")
const Composer = preload("res://di/Composer.gd")

@onready var logger = $Services/MatchLogger
var gateway: Node  # Created by factory

# Injected dependencies
var _core = null
var _player = null
var _ui = null
var _bus = null


func _ready() -> void:
	# Perform dependency injection
	Composer.inject(self)

	logger.start()

	# Create gateway using factory pattern
	gateway = await MatchGatewayFactory.create(self)

	if _bus:
		_bus.pub("match/scene_ready", {})
		_bus.connect("emit", Callable(self, "_on_bus"))

	# quick demo run after gateway is ready
	_call_simulation_demo()


# Dependency injection method called by Composer
func _inject_dependencies(core, player, ui, bus) -> void:
	_core = core
	_player = player
	_ui = ui
	_bus = bus


func _exit_tree() -> void:
	logger.stop()
	logger.save_snapshot()


func _on_bus(topic: String, payload):
	match topic:
		"match/log":
			logger.log(payload)
			# example: update HUD here
			pass
		_:
			pass


func _call_simulation_demo() -> void:
	var plan := {
		"seed": 12345,
		"home": {"name": "Home", "players": []},
		"away": {"name": "Away", "players": []},
		"user_player": {"is_home_team": true, "player_name": "Son ST2", "highlight_level": "medium"}
	}
	var result = gateway.simulate_json(plan, 50)
	# publish all events back to bus so other widgets can react
	if _bus and result.has("events"):
		for ev in result.events:
			_bus.pub("match/log", ev)
