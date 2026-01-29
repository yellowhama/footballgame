extends Node2D
class_name PotagonCharacter

@export var manifest_path: String = "res://assets_manifest.json"
@export var autoplay_action: String = "idle"

@onready var _animated_sprite: AnimatedSprite2D = $AnimatedSprite2D

var _manifest: Dictionary = {}


func _ready() -> void:
	_manifest = AssetManifestLoader.load_manifest(manifest_path)
	if _manifest.is_empty():
		push_error("[PotagonCharacter] Asset manifest not available: %s" % manifest_path)
		return

	var paths: Dictionary = _manifest.get("paths", {})
	var sheet_path := str(paths.get("action_sheet", "")).strip_edges()
	if sheet_path == "":
		push_error("[PotagonCharacter] paths.action_sheet missing in manifest")
		return
	if not ResourceLoader.exists(sheet_path):
		push_error("[PotagonCharacter] action_sheet not found: %s" % sheet_path)
		return

	var tex := load(sheet_path) as Texture2D
	if tex == null:
		push_error("[PotagonCharacter] Failed to load action_sheet texture: %s" % sheet_path)
		return

	if not AssetManifestLoader.validate_texture_size(tex, _manifest):
		push_warning("[PotagonCharacter] action_sheet texture size mismatch: %s" % sheet_path)

	var frames := PotagonSpriteFramesBuilder.build_sprite_frames(_manifest, tex)
	if frames == null:
		push_error("[PotagonCharacter] Failed to build SpriteFrames from manifest")
		return

	_animated_sprite.sprite_frames = frames
	play_action(autoplay_action)


func play_action(action_name: String, restart: bool = true) -> void:
	if _animated_sprite == null or _animated_sprite.sprite_frames == null:
		return

	var anim := str(action_name).to_lower().strip_edges()
	if anim == "":
		anim = "idle"

	if not _animated_sprite.sprite_frames.has_animation(anim):
		push_warning("[PotagonCharacter] Unknown action animation: %s" % anim)
		return

	if restart or _animated_sprite.animation != anim:
		_animated_sprite.animation = anim
	_animated_sprite.play()
