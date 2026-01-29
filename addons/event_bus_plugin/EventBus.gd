extends Node
## EventBus - Global Event Bus Singleton (Stub)
## Provides signal-based event communication between nodes

# Dictionary to store registered signals dynamically
var _signals: Dictionary = {}

func emit(event_name: String, payload: Variant = null) -> void:
	# Compatibility wrapper (some call sites use `EventBus.emit(name, payload)`).
	if payload == null:
		emit_event(event_name)
		return

	# If caller already passes an args array, forward directly.
	if payload is Array:
		emit_event(event_name, payload as Array)
		return

	# Otherwise treat payload as a single argument.
	emit_event(event_name, [payload])

func emit_event(event_name: String, args: Array = []) -> void:
	if has_user_signal(event_name):
		callv("emit_signal", [event_name] + args)

func register_event(event_name: String) -> void:
	if not has_user_signal(event_name):
		add_user_signal(event_name)
		_signals[event_name] = true

func subscribe(event_name: String, callable: Callable) -> void:
	register_event(event_name)
	if not is_connected(event_name, callable):
		connect(event_name, callable)

func unsubscribe(event_name: String, callable: Callable) -> void:
	if has_user_signal(event_name) and is_connected(event_name, callable):
		disconnect(event_name, callable)
