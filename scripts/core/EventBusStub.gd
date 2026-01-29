extends Node
class_name EventBusStub

var _listeners: Dictionary = {}


func _ready() -> void:
	print("[EventBusStub] Initialized (plugin disabled)")


func subscribe(event_name: String, callback: Callable) -> void:
	if not callback.is_valid():
		return
	var list: Array = _listeners.get(event_name, [])
	for existing in list:
		if existing == callback:
			return
	list.append(callback)
	_listeners[event_name] = list


func unsubscribe(event_name: String, callback: Callable) -> void:
	if not _listeners.has(event_name):
		return
	var list: Array = _listeners[event_name]
	list = list.filter(func(item): return item != callback)
	if list.is_empty():
		_listeners.erase(event_name)
	else:
		_listeners[event_name] = list


@warning_ignore("native_method_override")
func is_connected(signal_name: StringName, callable: Callable) -> bool:
	if not callable.is_valid():
		return false
	if not _listeners.has(signal_name):
		return false
	for cb in _listeners[signal_name]:
		if cb == callable:
			return true
	return false


func emit(event_name: String, payload: Variant = null) -> void:
	_dispatch(event_name, payload)


func call_deferred_event(event_name: String, payload: Variant = null) -> void:
	call_deferred("_dispatch", event_name, payload)


func _dispatch(event_name: String, payload: Variant = null) -> void:
	if not _listeners.has(event_name):
		return
	var callbacks: Array = _listeners[event_name].duplicate()
	for cb in callbacks:
		if not cb.is_valid():
			continue
		var args: Array = []
		if payload is Array:
			args = payload
		elif payload != null:
			args = [payload]
		cb.callv(args)
