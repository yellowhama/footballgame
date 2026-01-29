extends Control
class_name DraggableSlot

## Draggable slot for player swapping via drag & drop
## Implements Godot 4's native drag & drop system

var slot_index: int = -1
var is_bench: bool = false
var parent_screen = null  # Reference to MyTeamScreen


func setup(index: int, bench: bool, screen):
	"""Initialize the draggable slot"""
	slot_index = index
	is_bench = bench
	parent_screen = screen
	print("[DraggableSlot] Setup slot %d (bench: %s)" % [slot_index, is_bench])


func _get_drag_data(_at_position: Vector2) -> Variant:
	"""Called when user starts dragging this slot"""
	if not MyTeamData or slot_index < 0:
		return null

	# Check if there's a player in this slot
	if slot_index >= MyTeamData.current_team.players.size():
		return null

	var player_id = MyTeamData.current_team.players[slot_index]
	if player_id == "":
		return null  # Empty slot, can't drag

	var player_data = MyTeamData.get_player_by_id(player_id)
	if player_data.size() == 0:
		return null

	print("[DraggableSlot] Starting drag from slot %d" % slot_index)

	# Create drag preview (visual feedback)
	var preview = _create_drag_preview(player_data)
	set_drag_preview(preview)

	# Return drag data (Dictionary with slot index and player info)
	return {
		"slot_index": slot_index,
		"player_id": player_id,
		"player_name": player_data.get("name", "Unknown"),
		"is_bench": is_bench
	}


func _can_drop_data(_at_position: Vector2, data: Variant) -> bool:
	"""Check if dropped data can be accepted at this slot"""
	if not data is Dictionary:
		return false

	if not data.has("slot_index") or not data.has("player_id"):
		return false

	var source_slot = data.get("slot_index", -1)

	# Can't drop on itself
	if source_slot == slot_index:
		return false

	print("[DraggableSlot] Can drop at slot %d from slot %d" % [slot_index, source_slot])
	return true


func _drop_data(_at_position: Vector2, data: Variant) -> void:
	"""Handle the actual drop (swap players)"""
	if not data is Dictionary:
		return

	var source_slot = data.get("slot_index", -1)
	if source_slot < 0 or slot_index < 0:
		return

	print("[DraggableSlot] Dropping at slot %d from slot %d" % [slot_index, source_slot])

	# Swap players via MyTeamData
	if MyTeamData:
		if MyTeamData.swap_players(source_slot, slot_index):
			print("[DraggableSlot] ✅ Swapped players: slot %d <-> slot %d" % [source_slot, slot_index])

			# Reload formation and instructions on parent screen
			if parent_screen and parent_screen.has_method("_load_formation"):
				parent_screen._load_formation()
			if parent_screen and parent_screen.has_method("_load_instructions"):
				parent_screen._load_instructions()
		else:
			print("[DraggableSlot] ❌ Failed to swap players")


func _create_drag_preview(player_data: Dictionary) -> Control:
	"""Create visual drag preview"""
	var preview = PanelContainer.new()
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.2, 0.8, 1.0, 0.7)  # Blue semi-transparent
	style.corner_radius_top_left = 5
	style.corner_radius_top_right = 5
	style.corner_radius_bottom_left = 5
	style.corner_radius_bottom_right = 5
	preview.add_theme_stylebox_override("panel", style)

	var label = Label.new()
	label.text = "%s\nOVR %d" % [player_data.get("name", "Unknown").split(" ")[0], player_data.get("overall", 0)]
	label.add_theme_font_size_override("font_size", 16)
	label.add_theme_color_override("font_color", Color.WHITE)
	preview.add_child(label)

	preview.custom_minimum_size = Vector2(80, 60)

	return preview
