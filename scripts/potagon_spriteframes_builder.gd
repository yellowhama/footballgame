extends RefCounted
class_name PotagonSpriteFramesBuilder


static func build_sprite_frames(manifest: Dictionary, sheet_texture: Texture2D) -> SpriteFrames:
	if manifest.is_empty() or sheet_texture == null:
		return null

	var frame: Dictionary = manifest.get("frame", {})
	var sheet: Dictionary = manifest.get("sheet", {})
	var action_sheet: Dictionary = manifest.get("action_sheet", {})
	var anims: Array = action_sheet.get("animations", [])

	var cell_w := int(frame.get("width", 0))
	var cell_h := int(frame.get("height", 0))
	var cols := int(sheet.get("columns", 0))
	var total_frames := int(sheet.get("total_frames", 0))
	if cell_w <= 0 or cell_h <= 0 or cols <= 0 or total_frames <= 0:
		return null

	var sprite_frames := SpriteFrames.new()

	for entry in anims:
		if not (entry is Dictionary):
			continue

		var name_raw := str(entry.get("name", ""))
		if name_raw.strip_edges() == "":
			continue

		var anim_name := name_raw.to_lower()
		var start := int(entry.get("frame_start", -1))
		var frames := int(entry.get("frames", 0))
		var fps := float(entry.get("fps", 0.0))
		var loop := bool(entry.get("loop", true))

		if start < 0 or frames <= 0 or fps <= 0.0:
			continue

		if not sprite_frames.has_animation(anim_name):
			sprite_frames.add_animation(anim_name)
		sprite_frames.set_animation_speed(anim_name, fps)
		sprite_frames.set_animation_loop(anim_name, loop)

		for i in range(frames):
			var idx := start + i
			var rect := _frame_rect(idx, cell_w, cell_h, cols)
			var atlas := AtlasTexture.new()
			atlas.atlas = sheet_texture
			atlas.region = rect
			sprite_frames.add_frame(anim_name, atlas)

	return sprite_frames


static func build_animation_library(
	manifest: Dictionary, track_paths: Array, animation_name_transform: Callable = Callable()
) -> AnimationLibrary:
	if manifest.is_empty():
		return null

	var sheet: Dictionary = manifest.get("sheet", {})
	var total_frames := int(sheet.get("total_frames", 0))
	var used_frames := int(sheet.get("used_frames", total_frames))
	if used_frames <= 0:
		used_frames = total_frames

	var action_sheet: Dictionary = manifest.get("action_sheet", {})
	var anims: Array = action_sheet.get("animations", [])

	var library := AnimationLibrary.new()

	for entry in anims:
		if not (entry is Dictionary):
			continue

		var name_raw := str(entry.get("name", ""))
		if name_raw.strip_edges() == "":
			continue

		var anim_name := name_raw.to_lower()
		if animation_name_transform.is_valid():
			anim_name = str(animation_name_transform.call(anim_name))

		var start := int(entry.get("frame_start", -1))
		var frames := int(entry.get("frames", 0))
		var fps := float(entry.get("fps", 0.0))
		var loop := bool(entry.get("loop", true))

		if start < 0 or frames <= 0 or fps <= 0.0:
			continue

		var last_idx := start + (frames - 1)
		if last_idx >= used_frames:
			continue

		var anim := Animation.new()
		anim.length = float(frames) / fps
		anim.loop_mode = Animation.LOOP_LINEAR if loop else Animation.LOOP_NONE

		for track_path in track_paths:
			var path: NodePath
			if track_path is NodePath:
				path = track_path
			else:
				path = NodePath(str(track_path))

			var track_id := anim.add_track(Animation.TYPE_VALUE)
			anim.track_set_path(track_id, path)
			anim.track_set_interpolation_type(track_id, Animation.INTERPOLATION_NEAREST)
			# Godot 4.5: track_set_update_mode() removed, INTERPOLATION_NEAREST handles discrete updates

			for i in range(frames):
				var t := float(i) / fps
				anim.track_insert_key(track_id, t, start + i)

		library.add_animation(anim_name, anim)

	return library


static func apply_animation_library(
	anim_player: AnimationPlayer, library: AnimationLibrary, library_name: StringName = ""
) -> bool:
	if anim_player == null or library == null:
		return false

	if anim_player.has_method("remove_animation_library") and anim_player.has_method("add_animation_library"):
		if anim_player.has_method("get_animation_library"):
			var existing: AnimationLibrary = anim_player.get_animation_library(library_name)
			if existing != null:
				anim_player.remove_animation_library(library_name)
		anim_player.add_animation_library(library_name, library)
		return true

	# Fallback: assign via property (Godot 4 stores libraries as a Dictionary)
	if "libraries" in anim_player:
		anim_player.libraries = {library_name: library}
		return true

	return false


static func _frame_rect(idx: int, cell_w: int, cell_h: int, cols: int) -> Rect2:
	var col := idx % cols
	var row := int(idx / cols)
	return Rect2(col * cell_w, row * cell_h, cell_w, cell_h)
