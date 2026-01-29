# character_customizer.gd
# 외형 설정에 따라 스프라이트를 로드/교체하는 컴포넌트
class_name CharacterCustomizer
extends Node

signal appearance_changed

@export var appearance: CharacterAppearance:
	set(value):
		appearance = value
		if is_inside_tree():
			apply_appearance()

## 파츠별 Sprite2D 참조 (씬에서 설정)
@export var torso_sprite: Sprite2D
@export var arm_left_sprite: Sprite2D
@export var arm_right_sprite: Sprite2D
@export var forearm_left_sprite: Sprite2D
@export var forearm_right_sprite: Sprite2D
@export var leg_left_sprite: Sprite2D
@export var leg_right_sprite: Sprite2D
@export var head_sprite: Sprite2D
@export var hair_sprite: Sprite2D

## 에셋 경로
const BASE_PATH = "res://assets/nanobanana_works/character/"
const QUANTIZED_PATH = BASE_PATH + "final_set_quantized/"
const VARIANTS_PATH = BASE_PATH + "final_set_variants/"

## 텍스처 캐시
var _texture_cache: Dictionary = {}


func _ready() -> void:
	if appearance == null:
		appearance = CharacterAppearance.new()
	apply_appearance()


func apply_appearance() -> void:
	"""현재 appearance 설정에 따라 모든 스프라이트 갱신"""
	if appearance == null:
		return

	var dir_info = appearance.get_direction_info()
	var direction = dir_info["dir"]
	var should_flip = dir_info["flip"]

	# 1. 몸통 (Torso) 로드
	_load_torso_sprite(direction, should_flip)

	# 2. 슬리브 (Arm/Forearm) 로드
	_load_sleeve_sprites(direction, should_flip)

	# 3. 다리 로드 (피부색 적용)
	_load_leg_sprites(direction, should_flip)

	# 4. 머리 로드 (피부색 적용)
	_load_head_sprite(direction, should_flip)

	# 5. 헤어 로드
	_load_hair_sprite(direction, should_flip)

	# Z-Order 업데이트
	_update_z_order(direction)

	appearance_changed.emit()


func _load_torso_sprite(direction: String, flip: bool) -> void:
	var path: String
	if appearance.torso_color == "red":
		# 기본 색상 - 원본 사용
		path = QUANTIZED_PATH + "body/" + direction + "/torso.png"
	else:
		# 색상 변형 프리셋
		path = VARIANTS_PATH + "torso_" + appearance.torso_color + "/" + direction + "/torso.png"
	_apply_sprite(torso_sprite, path, flip)


func _load_sleeve_sprites(direction: String, flip: bool) -> void:
	var parts = {
		"arm_left": arm_left_sprite,
		"arm_right": arm_right_sprite,
		"forearm_left": forearm_left_sprite,
		"forearm_right": forearm_right_sprite
	}

	for part_name in parts:
		var sprite = parts[part_name]
		if sprite == null:
			continue

		var path: String
		if appearance.sleeve_color == "red":
			path = QUANTIZED_PATH + "body/" + direction + "/" + part_name + ".png"
		else:
			path = VARIANTS_PATH + "sleeve_" + appearance.sleeve_color + "/" + direction + "/" + part_name + ".png"

		# flip 시 좌우 파츠 교체
		var actual_sprite = sprite
		if flip:
			if part_name.ends_with("_left"):
				actual_sprite = parts[part_name.replace("_left", "_right")]
			elif part_name.ends_with("_right"):
				actual_sprite = parts[part_name.replace("_right", "_left")]

		_apply_sprite(actual_sprite, path, flip)


func _load_leg_sprites(direction: String, flip: bool) -> void:
	var parts = {"leg_left": leg_left_sprite, "leg_right": leg_right_sprite}

	for part_name in parts:
		var sprite = parts[part_name]
		if sprite == null:
			continue

		var path: String
		if appearance.skin_tone == "medium":
			path = QUANTIZED_PATH + "body/" + direction + "/" + part_name + ".png"
		else:
			path = VARIANTS_PATH + "skin_" + appearance.skin_tone + "/" + direction + "/" + part_name + ".png"

		# flip 시 좌우 파츠 교체
		var actual_sprite = sprite
		if flip:
			if part_name == "leg_left":
				actual_sprite = leg_right_sprite
			else:
				actual_sprite = leg_left_sprite

		_apply_sprite(actual_sprite, path, flip)


func _load_head_sprite(direction: String, flip: bool) -> void:
	if head_sprite == null:
		return

	var path: String
	if appearance.skin_tone == "medium":
		path = QUANTIZED_PATH + "body/" + direction + "/head.png"
	else:
		path = VARIANTS_PATH + "skin_" + appearance.skin_tone + "/" + direction + "/head.png"
	_apply_sprite(head_sprite, path, flip)


func _load_hair_sprite(direction: String, flip: bool) -> void:
	if hair_sprite == null:
		return

	# 부분 지원 헤어스타일 방향 제한 처리
	var actual_direction = direction
	if not appearance.is_hair_style_full_direction():
		# 부분 지원 스타일은 front, side, back만 지원
		match direction:
			"quarter_front":
				actual_direction = "front"  # 대체
			"quarter_back":
				actual_direction = "back"  # 대체

	var path: String
	if appearance.hair_color == "brown":
		path = QUANTIZED_PATH + "hair/" + appearance.hair_style + "/hair_" + actual_direction + ".png"
	else:
		path = (
			VARIANTS_PATH
			+ "hair_"
			+ appearance.hair_color
			+ "/"
			+ appearance.hair_style
			+ "/hair_"
			+ actual_direction
			+ ".png"
		)
	_apply_sprite(hair_sprite, path, flip)


func _apply_sprite(sprite: Sprite2D, path: String, flip: bool) -> void:
	if sprite == null:
		return

	var texture = _get_cached_texture(path)
	if texture != null:
		sprite.texture = texture
		sprite.flip_h = flip
	else:
		push_warning("Missing sprite: " + path)
		sprite.texture = null


func _get_cached_texture(path: String) -> Texture2D:
	"""텍스처 캐싱"""
	if _texture_cache.has(path):
		return _texture_cache[path]

	if ResourceLoader.exists(path):
		var tex = load(path)
		_texture_cache[path] = tex
		return tex

	return null


func _update_z_order(direction: String) -> void:
	"""방향에 따른 Z-Order 업데이트"""
	var z_order: Dictionary

	match direction:
		"front", "quarter_front":
			z_order = {
				"leg_left": -2,
				"leg_right": -2,
				"torso": 0,
				"arm_left": 1,
				"arm_right": 1,
				"forearm_left": 2,
				"forearm_right": 2,
				"head": 3,
				"hair": 4
			}
		"back", "quarter_back":
			z_order = {
				"hair": -2,
				"head": -1,
				"arm_left": 0,
				"arm_right": 0,
				"forearm_left": 1,
				"forearm_right": 1,
				"torso": 2,
				"leg_left": 3,
				"leg_right": 3
			}
		"side":
			z_order = {
				"leg_right": -2,
				"leg_left": -1,
				"arm_right": 0,
				"forearm_right": 1,
				"torso": 2,
				"arm_left": 3,
				"forearm_left": 4,
				"head": 5,
				"hair": 6
			}

	_apply_z_order(z_order)


func _apply_z_order(z_order: Dictionary) -> void:
	var sprite_map = {
		"torso": torso_sprite,
		"arm_left": arm_left_sprite,
		"arm_right": arm_right_sprite,
		"forearm_left": forearm_left_sprite,
		"forearm_right": forearm_right_sprite,
		"leg_left": leg_left_sprite,
		"leg_right": leg_right_sprite,
		"head": head_sprite,
		"hair": hair_sprite
	}

	for part_name in z_order:
		var sprite = sprite_map.get(part_name)
		if sprite != null:
			sprite.z_index = z_order[part_name]


# === Public API ===


func set_direction(dir: int) -> void:
	"""방향 변경 (0-7)"""
	appearance.facing_direction = dir % 8
	apply_appearance()


func set_uniform_colors(torso: String, sleeve: String) -> void:
	"""유니폼 색상 변경"""
	appearance.torso_color = torso
	appearance.sleeve_color = sleeve
	apply_appearance()


func set_hair_style(style: String) -> void:
	"""헤어스타일 변경"""
	if style in CharacterAppearance.HAIR_STYLES_FULL or style in CharacterAppearance.HAIR_STYLES_PARTIAL:
		appearance.hair_style = style
		apply_appearance()


func set_hair_color(color: String) -> void:
	"""헤어색 변경"""
	appearance.hair_color = color
	apply_appearance()


func set_skin_tone(tone: String) -> void:
	"""피부색 변경"""
	appearance.skin_tone = tone
	apply_appearance()


func randomize_character() -> void:
	"""랜덤 캐릭터 생성"""
	appearance.randomize_appearance()
	apply_appearance()


func clear_cache() -> void:
	"""텍스처 캐시 초기화"""
	_texture_cache.clear()
