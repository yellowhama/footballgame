extends Control
class_name EmblemPreview

## EmblemPreview: Composable emblem system
## Layers: Background shape (primary color) + Icon symbol (secondary color)

@onready var background: Control = $Background
@onready var icon: Control = $Icon

# Emblem composition data
var current_icon: int = 0
var current_background: int = 0
var primary_color: Color = Color.RED
var secondary_color: Color = Color.WHITE

# Placeholder shapes (will be replaced with SVG from game-icons.net)
const BACKGROUND_SHAPES = ["Shield", "Circle", "CoatOfArms", "Pentagon", "Diamond", "CrossShield"]  # 0: 방패형 (기본)  # 1: 원형  # 2: 문장 방패  # 3: 오각형  # 4: 다이아몬드  # 5: 십자가 방패

# Placeholder icons (will be replaced with SVG from game-icons.net - CC BY 3.0)
# Source: https://game-icons.net (Medieval/Fantasy theme)
const ICON_SYMBOLS = [
	"🦁", "🦅", "🐉", "🐺", "💀", "👑", "⭐", "⚔️", "🛡️", "⚡", "🔥", "🦴", "🐕", "🐀", "🦇", "🕷️", "🐍", "🐻", "🪽", "✝️"  # 0: Lion (사자)  # 1: Eagle (독수리)  # 2: Dragon (용)  # 3: Wolf (늑대)  # 4: Skull (해골)  # 5: Crown (왕관)  # 6: Star (별)  # 7: Sword (칼)  # 8: Shield (방패)  # 9: Lightning (번개)  # 10: Fire (불꽃)  # 11: Bone (뼈)  # 12: Dog (개)  # 13: Rat (쥐)  # 14: Bat (박쥐)  # 15: Spider (거미)  # 16: Snake (뱀)  # 17: Bear (곰)  # 18: Wing (날개)  # 19: Cross (십자가)
]


func _ready():
	_update_emblem()


func set_emblem(icon_id: int, background_id: int, p_color: Color, s_color: Color):
	"""Set emblem composition and update display"""
	current_icon = clamp(icon_id, 0, ICON_SYMBOLS.size() - 1)
	current_background = clamp(background_id, 0, BACKGROUND_SHAPES.size() - 1)
	primary_color = p_color
	secondary_color = s_color

	_update_emblem()


func _update_emblem():
	"""Update emblem visual composition"""
	if not background or not icon:
		return

	# Update background
	_update_background()

	# Update icon
	_update_icon()


func _update_background():
	"""Update background shape and color"""
	# For now, use ColorRect as placeholder
	# Later: Replace with TextureRect + SVG shapes

	if background is ColorRect:
		background.color = primary_color

		# Shape visualization (placeholder)
		match current_background:
			0:  # Circle
				background.size = Vector2(100, 100)
			1:  # Shield
				background.size = Vector2(90, 110)
			2:  # Hexagon
				background.size = Vector2(100, 100)
			3:  # Square
				background.size = Vector2(100, 100)
			4:  # Diamond
				background.size = Vector2(90, 110)
			5:  # Pentagon
				background.size = Vector2(100, 100)


func _update_icon():
	"""Update icon symbol and color"""
	# For now, use Label with emoji
	# Later: Replace with TextureRect + SVG icons

	if icon is Label:
		icon.text = ICON_SYMBOLS[current_icon]
		icon.add_theme_color_override("font_color", secondary_color)
		icon.add_theme_font_size_override("font_size", 48)


func get_background_name(bg_id: int) -> String:
	"""Get background shape name"""
	if bg_id >= 0 and bg_id < BACKGROUND_SHAPES.size():
		return BACKGROUND_SHAPES[bg_id]
	return "Unknown"


func get_icon_symbol(icon_id: int) -> String:
	"""Get icon symbol (emoji placeholder)"""
	if icon_id >= 0 and icon_id < ICON_SYMBOLS.size():
		return ICON_SYMBOLS[icon_id]
	return "?"
