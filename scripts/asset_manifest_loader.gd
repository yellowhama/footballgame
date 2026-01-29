extends Node
class_name AssetManifestLoader

const DEFAULT_MANIFEST_PATH := "res://assets_manifest.json"

static var _cache: Dictionary = {}


static func load_manifest(path: String = DEFAULT_MANIFEST_PATH, force_reload: bool = false) -> Dictionary:
	if not force_reload and _cache.has(path):
		return _cache[path]

	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		push_error("[AssetManifestLoader] Failed to open manifest: %s" % path)
		return {}

	var text := file.get_as_text()
	file.close()

	var json := JSON.new()
	var parse_result := json.parse(text)
	if parse_result != OK:
		push_error("[AssetManifestLoader] JSON parse error (%s): %s" % [path, str(json.get_error_message())])
		return {}

	if not (json.data is Dictionary):
		push_error("[AssetManifestLoader] Manifest root must be an object/dictionary: %s" % path)
		return {}

	var manifest: Dictionary = json.data
	var errors: Array = validate_manifest(manifest)
	if not errors.is_empty():
		push_error("[AssetManifestLoader] Manifest validation failed (%s):\n- %s" % [path, "\n- ".join(errors)])
		return {}

	_cache[path] = manifest
	return manifest


static func validate_manifest(manifest: Dictionary) -> Array:
	var errors: Array = []

	_require_int(manifest, "version", errors)
	_require_dict(manifest, "frame", errors)
	_require_dict(manifest, "sheet", errors)
	_require_array(manifest, "directions", errors)
	_require_dict(manifest, "paths", errors)
	_require_dict(manifest, "action_sheet", errors)

	var frame: Dictionary = manifest.get("frame", {})
	_require_int(frame, "width", errors, "frame.width")
	_require_int(frame, "height", errors, "frame.height")

	var sheet: Dictionary = manifest.get("sheet", {})
	_require_int(sheet, "columns", errors, "sheet.columns")
	_require_int(sheet, "rows", errors, "sheet.rows")
	_require_int(sheet, "total_frames", errors, "sheet.total_frames")
	_require_int(sheet, "used_frames", errors, "sheet.used_frames")

	if frame.get("width", 0) <= 0:
		errors.append("frame.width must be > 0")
	if frame.get("height", 0) <= 0:
		errors.append("frame.height must be > 0")
	if sheet.get("columns", 0) <= 0:
		errors.append("sheet.columns must be > 0")
	if sheet.get("rows", 0) <= 0:
		errors.append("sheet.rows must be > 0")
	if sheet.get("total_frames", 0) <= 0:
		errors.append("sheet.total_frames must be > 0")
	if sheet.get("used_frames", 0) <= 0:
		errors.append("sheet.used_frames must be > 0")
	if int(sheet.get("used_frames", 0)) > int(sheet.get("total_frames", 0)):
		errors.append("sheet.used_frames must be <= sheet.total_frames")

	var directions: Array = manifest.get("directions", [])
	if directions.size() != 8:
		errors.append("directions must have exactly 8 entries (got %d)" % directions.size())
	else:
		var seen := {}
		for d in directions:
			if not (d is String):
				errors.append("directions entries must be strings")
				break
			if seen.has(d):
				errors.append("directions contains duplicate entry: %s" % str(d))
			seen[d] = true

	var paths: Dictionary = manifest.get("paths", {})
	_require_string(paths, "action_sheet", errors, "paths.action_sheet")
	if paths.has("action_sheet") and str(paths.get("action_sheet")) != "":
		var action_sheet_path := str(paths.get("action_sheet"))
		if not ResourceLoader.exists(action_sheet_path):
			errors.append("paths.action_sheet not found (missing resource): %s" % action_sheet_path)

	var action_sheet: Dictionary = manifest.get("action_sheet", {})
	_require_array(action_sheet, "animations", errors, "action_sheet.animations")
	if action_sheet.has("animations") and action_sheet.animations is Array:
		_validate_animations(action_sheet.animations, sheet, errors)

	return errors


static func validate_texture_size(texture: Texture2D, manifest: Dictionary) -> bool:
	if texture == null:
		return false
	if manifest.is_empty():
		return false

	var frame: Dictionary = manifest.get("frame", {})
	var sheet: Dictionary = manifest.get("sheet", {})

	var w := int(frame.get("width", 0))
	var h := int(frame.get("height", 0))
	var cols := int(sheet.get("columns", 0))
	var rows := int(sheet.get("rows", 0))
	if w <= 0 or h <= 0 or cols <= 0 or rows <= 0:
		return false

	var expected_w := w * cols
	var expected_h := h * rows
	var actual := texture.get_size()
	return int(actual.x) == expected_w and int(actual.y) == expected_h


static func _validate_animations(anims: Array, sheet: Dictionary, errors: Array) -> void:
	var total_frames := int(sheet.get("total_frames", 0))
	var used_frames := int(sheet.get("used_frames", total_frames))

	var seen := {}
	for i in range(anims.size()):
		var entry = anims[i]
		if not (entry is Dictionary):
			errors.append("action_sheet.animations[%d] must be an object" % i)
			continue

		var name_key := "name"
		if not entry.has(name_key) or not (entry[name_key] is String) or str(entry[name_key]).strip_edges() == "":
			errors.append("action_sheet.animations[%d].name must be a non-empty string" % i)
		else:
			var name_norm := str(entry[name_key]).to_lower()
			if seen.has(name_norm):
				errors.append("action_sheet.animations has duplicate name: %s" % name_norm)
			seen[name_norm] = true

		var frame_start := int(entry.get("frame_start", -1))
		var frames := int(entry.get("frames", 0))
		var fps := float(entry.get("fps", 0.0))
		if frame_start < 0:
			errors.append("action_sheet.animations[%d].frame_start must be >= 0" % i)
		if frames <= 0:
			errors.append("action_sheet.animations[%d].frames must be > 0" % i)
		if fps <= 0.0:
			errors.append("action_sheet.animations[%d].fps must be > 0" % i)

		if frame_start >= 0 and frames > 0:
			var last_idx := frame_start + (frames - 1)
			if last_idx >= total_frames:
				errors.append(
					(
						"action_sheet.animations[%d] frame range exceeds sheet.total_frames (%d >= %d)"
						% [i, last_idx, total_frames]
					)
				)
			if last_idx >= used_frames:
				errors.append(
					(
						"action_sheet.animations[%d] frame range exceeds sheet.used_frames (%d >= %d)"
						% [i, last_idx, used_frames]
					)
				)


static func _require_dict(src: Dictionary, key: String, errors: Array, label: String = "") -> void:
	var field_name := label if label != "" else key
	if not src.has(key) or not (src[key] is Dictionary):
		errors.append("%s must be an object/dictionary" % field_name)


static func _require_array(src: Dictionary, key: String, errors: Array, label: String = "") -> void:
	var field_name := label if label != "" else key
	if not src.has(key) or not (src[key] is Array):
		errors.append("%s must be an array" % field_name)


static func _require_int(src: Dictionary, key: String, errors: Array, label: String = "") -> void:
	var field_name := label if label != "" else key
	if not src.has(key) or not (src[key] is int or src[key] is float):
		errors.append("%s must be an integer" % field_name)
		return

	var v = src[key]
	if v is float and v != floor(v):
		errors.append("%s must be an integer (got non-integer float)" % field_name)


static func _require_string(src: Dictionary, key: String, errors: Array, label: String = "") -> void:
	var field_name := label if label != "" else key
	if not src.has(key) or not (src[key] is String):
		errors.append("%s must be a string" % field_name)
