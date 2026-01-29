extends Control
class_name StatusScreen

@onready var squad_list_container: VBoxContainer = $HSplitContainer/SquadList/Scroll/VBox
@onready var detail_name: Label = $HSplitContainer/PlayerDetail/VBox/ProfileHeader/NameLabel
@onready var detail_pos: Label = $HSplitContainer/PlayerDetail/VBox/ProfileHeader/PosLabel
@onready var detail_ovr: Label = $HSplitContainer/PlayerDetail/VBox/ProfileHeader/OvrLabel
@onready var hexagon_chart: HexagonChart = $HSplitContainer/PlayerDetail/VBox/HexagonContainer/HexagonChart
@onready var attr_grid: GridContainer = $HSplitContainer/PlayerDetail/VBox/AttributeGrid

const PlayerRowScene = preload("res://scenes/ui/components/PlayerRow.tscn")

# Mock Data (Replace with real Rosters from bridge/autoload)
var _players: Array = []

func _ready() -> void:
	_load_mock_data()
	_populate_list()
	if not _players.is_empty():
		_select_player(_players[0])

func _load_mock_data() -> void:
	# Temporary mock data until SSOT connection
	_players = [
		{"id": "p1", "name": "Son Heung-min", "position": "Fwd", "ca": 88, "condition": 0.95, "role": "Captain", 
		 "stats": {"PACE": 88, "SHOOTING": 90, "PASSING": 82, "TECHNICAL": 86, "DEFENDING": 40, "POWER": 75}},
		{"id": "p2", "name": "James Maddison", "position": "Mid", "ca": 84, "condition": 0.90, "role": "Playmaker",
		 "stats": {"PACE": 74, "SHOOTING": 80, "PASSING": 88, "TECHNICAL": 87, "DEFENDING": 55, "POWER": 65}},
		{"id": "p3", "name": "Cristian Romero", "position": "Def", "ca": 85, "condition": 0.92, "role": "Stopper",
		 "stats": {"PACE": 78, "SHOOTING": 45, "PASSING": 70, "TECHNICAL": 72, "DEFENDING": 89, "POWER": 86}},
		{"id": "p4", "name": "Guglielmo Vicario", "position": "GK", "ca": 83, "condition": 0.98, "role": "Keeper",
		 "stats": {"PACE": 50, "SHOOTING": 20, "PASSING": 75, "TECHNICAL": 60, "DEFENDING": 90, "POWER": 80}},
	]

func _populate_list() -> void:
	for child in squad_list_container.get_children():
		child.queue_free()
		
	for p in _players:
		var row = PlayerRowScene.instantiate()
		squad_list_container.add_child(row)
		row.setup(p)
		row.row_selected.connect(func(pid): _on_player_selected(pid))

func _on_player_selected(pid: String) -> void:
	for p in _players:
		if p["id"] == pid:
			_select_player(p)
			return

func _select_player(player: Dictionary) -> void:
	if detail_name: detail_name.text = player.get("name", "")
	if detail_pos: detail_pos.text = player.get("position", "")
	if detail_ovr: detail_ovr.text = str(player.get("ca", 0))
	
	if hexagon_chart:
		var stats = player.get("stats", {})
		hexagon_chart.set_stats(stats, true)
	
	_update_attributes(player)

func _update_attributes(player: Dictionary) -> void:
	# Populate grid with text attributes or detailed numbers
	# For now, clear and show stats text
	for child in attr_grid.get_children():
		child.queue_free()
	
	var stats = player.get("stats", {})
	for k in stats:
		var lbl_k = Label.new()
		lbl_k.text = k
		lbl_k.add_theme_color_override("font_color", Color.GRAY)
		attr_grid.add_child(lbl_k)
		
		var lbl_v = Label.new()
		lbl_v.text = str(stats[k])
		lbl_v.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		attr_grid.add_child(lbl_v)
		
		var spacer = Control.new() # 3-column layout? Key, Value, Spacer?
		attr_grid.add_child(spacer)
