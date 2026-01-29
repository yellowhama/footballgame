extends PanelContainer
class_name PlayerPositionRatingsPanel

## PlayerPositionRatingsPanel: Displays 14 position ratings with color coding
## Shows FM 2023 position suitability ratings in a 2-row grid
## Usage: Call display_position_ratings(player_uid) to populate

@onready var position_grid: GridContainer = $VBoxContainer/PositionGrid
@onready var title_label: Label = $VBoxContainer/TitleLabel

# Position display order
const ROW1_POSITIONS = ["GK", "DL", "DC", "DR", "WBL", "WBR", "DM"]
const ROW2_POSITIONS = ["ML", "MC", "MR", "AML", "AMC", "AMR", "ST"]


func _ready():
	if not position_grid:
		position_grid = GridContainer.new()
		position_grid.name = "PositionGrid"
		position_grid.columns = 7
		$VBoxContainer.add_child(position_grid)

	if title_label:
		title_label.text = "Position Ratings (FM 2023)"


## Display position ratings for a player by UID
## @param player_uid: Player unique ID from cache
func display_position_ratings(player_uid: int) -> void:
	_clear_grid()

	# Query GameCache for position ratings
	var ratings = GameCache.get_player_position_ratings(player_uid)

	if ratings.is_empty():
		_show_no_data_message()
		return

	# Configure grid (7 columns)
	if position_grid:
		position_grid.columns = 7

	# Row 1: GK DL DC DR WBL WBR DM
	for pos in ROW1_POSITIONS:
		var rating = int(ratings.get(pos, 0))
		_add_position_label(pos, rating)

	# Row 2: [spacer] ML MC MR AML AMC AMR ST
	_add_spacer()
	for pos in ROW2_POSITIONS:
		var rating = int(ratings.get(pos, 0))
		_add_position_label(pos, rating)


## Display best positions only (filtered by min_rating)
## @param player_uid: Player unique ID
## @param min_rating: Minimum rating to display (default 15)
func display_best_positions(player_uid: int, min_rating: int = 15) -> void:
	_clear_grid()

	# Query GameCache for best positions
	var best_positions = GameCache.get_player_best_positions(player_uid, min_rating)

	if best_positions.is_empty():
		_show_no_positions_message(min_rating)
		return

	# Display as horizontal list (not grid)
	if position_grid:
		position_grid.columns = best_positions.size()

	for pos_dict in best_positions:
		var position = str(pos_dict.get("position", ""))
		var rating = int(pos_dict.get("rating", 0))
		_add_position_label(position, rating)


## Add a position label with color coding
func _add_position_label(position: String, rating: int) -> void:
	if not position_grid:
		return

	var label = Label.new()
	label.text = "%s:%d" % [position, rating]
	label.custom_minimum_size = Vector2(60, 35)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER

	# Color coding based on rating
	var color = PositionRatingUtils.get_position_rating_color(rating)
	label.add_theme_color_override("font_color", color)

	# Add tooltip with penalty info
	var tooltip = PositionRatingUtils.get_rating_tooltip(rating)
	label.tooltip_text = tooltip

	position_grid.add_child(label)


## Add empty spacer for grid layout
func _add_spacer() -> void:
	if not position_grid:
		return

	var spacer = Control.new()
	spacer.custom_minimum_size = Vector2(60, 35)
	position_grid.add_child(spacer)


## Clear all grid children
func _clear_grid() -> void:
	if not position_grid:
		return

	for child in position_grid.get_children():
		child.queue_free()


## Show "No data available" message
func _show_no_data_message() -> void:
	var label = Label.new()
	label.text = "No position ratings data available"
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))

	if position_grid:
		position_grid.add_child(label)


## Show "No positions meet criteria" message
func _show_no_positions_message(min_rating: int) -> void:
	var label = Label.new()
	label.text = "No positions with rating >= %d" % min_rating
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.add_theme_color_override("font_color", Color(0.5, 0.5, 0.5))

	if position_grid:
		position_grid.add_child(label)


## Set title text
func set_title(title: String) -> void:
	if title_label:
		title_label.text = title


## Helper: Get position rating for specific position (for external use)
## @param player_uid: Player unique ID
## @param position: Position code (e.g., "ST", "MC")
## @return Rating value (0-20) or -1 if not found
func get_position_rating(player_uid: int, position: String) -> int:
	var ratings = GameCache.get_player_position_ratings(player_uid)
	if ratings.is_empty():
		return -1
	return int(ratings.get(position, -1))
