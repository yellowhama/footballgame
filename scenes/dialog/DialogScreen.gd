# DialogScreen - Uses Composer DI for service injection
extends Control

const Composer = preload("res://di/Composer.gd")

@onready var runtime = $Services/DialogRuntime

# Injected dependencies
var _core = null
var _player = null
var _ui = null
var _bus = null


func _ready():
	# Perform dependency injection
	Composer.inject(self)

	runtime.connect("line", Callable(self, "_on_line"))
	(
		runtime
		. load_script(
			[
				"코치: 오늘은 패싱 훈련이다.",
				"선수: 네!",
			]
		)
	)
	runtime.next()

	# Notify scene ready through EventBus
	if _bus:
		_bus.pub("dialog/scene_ready", {})


# Dependency injection method called by Composer
func _inject_dependencies(core, player, ui, bus) -> void:
	_core = core
	_player = player
	_ui = ui
	_bus = bus


func _on_line(t: String) -> void:
	print("[Dialog] ", t)
	if t != "<end>":
		runtime.next()
