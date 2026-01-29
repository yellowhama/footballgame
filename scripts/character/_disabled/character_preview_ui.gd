# character_preview_ui.gd
# 캐릭터 미리보기 UI 컨트롤러
extends Control

@onready var character = $CharacterContainer/Character
@onready var customizer: CharacterCustomizer = $CharacterContainer/Character/Customizer

@onready var direction_slider: HSlider = $UI/DirectionSlider
@onready var hair_style_option: OptionButton = $UI/HairStyleOption
@onready var hair_color_option: OptionButton = $UI/HairColorOption
@onready var skin_tone_option: OptionButton = $UI/SkinToneOption
@onready var torso_color_option: OptionButton = $UI/TorsoColorOption
@onready var sleeve_color_option: OptionButton = $UI/SleeveColorOption
@onready var random_button: Button = $UI/RandomButton

const HAIR_STYLES = ["braids", "curly", "medium", "spiky", "afro", "buzz", "mohawk", "wavy"]
const HAIR_COLORS = ["brown", "black", "blonde", "ginger", "gray"]
const SKIN_TONES = ["medium", "light", "olive", "brown", "dark"]
const UNIFORM_COLORS = ["red", "orange", "yellow", "green", "cyan", "blue", "purple", "pink", "white", "black", "gray"]


func _ready() -> void:
	_setup_options()
	_connect_signals()


func _setup_options() -> void:
	# Hair styles
	hair_style_option.clear()
	for style in HAIR_STYLES:
		hair_style_option.add_item(style.capitalize())
	hair_style_option.select(HAIR_STYLES.find("medium"))

	# Hair colors
	hair_color_option.clear()
	for color in HAIR_COLORS:
		hair_color_option.add_item(color.capitalize())
	hair_color_option.select(0)  # brown

	# Skin tones
	skin_tone_option.clear()
	for tone in SKIN_TONES:
		skin_tone_option.add_item(tone.capitalize())
	skin_tone_option.select(0)  # medium

	# Torso colors
	torso_color_option.clear()
	for color in UNIFORM_COLORS:
		torso_color_option.add_item(color.capitalize())
	torso_color_option.select(0)  # red

	# Sleeve colors
	sleeve_color_option.clear()
	for color in UNIFORM_COLORS:
		sleeve_color_option.add_item(color.capitalize())
	sleeve_color_option.select(0)  # red


func _connect_signals() -> void:
	direction_slider.value_changed.connect(_on_direction_changed)
	hair_style_option.item_selected.connect(_on_hair_style_changed)
	hair_color_option.item_selected.connect(_on_hair_color_changed)
	skin_tone_option.item_selected.connect(_on_skin_tone_changed)
	torso_color_option.item_selected.connect(_on_torso_color_changed)
	sleeve_color_option.item_selected.connect(_on_sleeve_color_changed)
	random_button.pressed.connect(_on_random_pressed)


func _on_direction_changed(value: float) -> void:
	customizer.set_direction(int(value))


func _on_hair_style_changed(index: int) -> void:
	customizer.set_hair_style(HAIR_STYLES[index])


func _on_hair_color_changed(index: int) -> void:
	customizer.set_hair_color(HAIR_COLORS[index])


func _on_skin_tone_changed(index: int) -> void:
	customizer.set_skin_tone(SKIN_TONES[index])


func _on_torso_color_changed(index: int) -> void:
	customizer.set_uniform_colors(UNIFORM_COLORS[index], customizer.appearance.sleeve_color)


func _on_sleeve_color_changed(index: int) -> void:
	customizer.set_uniform_colors(customizer.appearance.torso_color, UNIFORM_COLORS[index])


func _on_random_pressed() -> void:
	customizer.randomize_character()
	_sync_ui_to_appearance()


func _sync_ui_to_appearance() -> void:
	"""UI를 현재 appearance에 동기화"""
	var app = customizer.appearance

	direction_slider.value = app.facing_direction
	hair_style_option.select(HAIR_STYLES.find(app.hair_style))
	hair_color_option.select(HAIR_COLORS.find(app.hair_color))
	skin_tone_option.select(SKIN_TONES.find(app.skin_tone))
	torso_color_option.select(UNIFORM_COLORS.find(app.torso_color))
	sleeve_color_option.select(UNIFORM_COLORS.find(app.sleeve_color))


func _input(event: InputEvent) -> void:
	# 키보드로 방향 변경 (1-8)
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_1:
				customizer.set_direction(0)
			KEY_2:
				customizer.set_direction(1)
			KEY_3:
				customizer.set_direction(2)
			KEY_4:
				customizer.set_direction(3)
			KEY_5:
				customizer.set_direction(4)
			KEY_6:
				customizer.set_direction(5)
			KEY_7:
				customizer.set_direction(6)
			KEY_8:
				customizer.set_direction(7)
			KEY_R:
				_on_random_pressed()
		_sync_ui_to_appearance()
