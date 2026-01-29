# GameCore Service - Central scene/save/version hub
# Autoload as: Name=GameCore, Path=res://autoload/core/game_core.gd, Singleton=On

extends Node

signal scene_changed(path: String)

var _current_scene_path: String = ""
var _build_tag: String = "dev-local"  # TODO: fed via vergen or ProjectSettings


func _ready() -> void:
	print("[GameCore] ready; build=", _build_tag)


func set_build_tag(tag: String) -> void:
	_build_tag = tag


func get_build_tag() -> String:
	return _build_tag


func goto(scene_path: String) -> void:
	assert(scene_path != "", "[GameCore] empty scene path")
	var root := get_tree().root
	var packed: PackedScene = load(scene_path)
	if packed == null:
		push_error("[GameCore] failed to load: %s" % scene_path)
		return

	# Swap scene safely
	for child in root.get_children():
		if child != self:  # keep autoloads
			child.queue_free()

	await get_tree().process_frame
	root.add_child(packed.instantiate())
	_set_current_scene(scene_path)
	scene_changed.emit(scene_path)
	print("[GameCore] scene -> ", scene_path)


# Save / Load stubs (expand later)
func save(path: String = "user://save_v2.tres") -> bool:
	print("[GameCore] save ", path)
	return true


func load_save(path: String = "user://save_v2.tres") -> bool:
	print("[GameCore] load ", path)
	return true


func _set_current_scene(p: String) -> void:
	_current_scene_path = p
