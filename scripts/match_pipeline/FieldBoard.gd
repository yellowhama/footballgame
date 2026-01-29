extends RefCounted
class_name FieldBoard

## FieldBoard OS Layer 0 - Grid + Heatmaps
## Godot-side container for Rust FieldBoard data

const COLS = 28
const ROWS = 18
const FIELD_LENGTH_M = 105.0
const FIELD_WIDTH_M = 68.0

# Computed dimensions
var cell_w_m: float  # ~3.75m
var cell_h_m: float  # ~3.78m

# Heatmaps (28×18 = 504 cells each)
var occupancy_total: Array[int] = []  # [504] - total players per cell
var occupancy_home: Array[int] = []  # [504] - home players per cell
var occupancy_away: Array[int] = []  # [504] - away players per cell
var pressure_against_home: Array[float] = []  # [504] - pressure if home has ball
var pressure_against_away: Array[float] = []  # [504] - pressure if away has ball


func _init():
	cell_w_m = FIELD_LENGTH_M / float(COLS)
	cell_h_m = FIELD_WIDTH_M / float(ROWS)

	# Initialize arrays (504 cells)
	occupancy_total.resize(COLS * ROWS)
	occupancy_home.resize(COLS * ROWS)
	occupancy_away.resize(COLS * ROWS)
	pressure_against_home.resize(COLS * ROWS)
	pressure_against_away.resize(COLS * ROWS)

	# Zero-fill
	for i in range(COLS * ROWS):
		occupancy_total[i] = 0
		occupancy_home[i] = 0
		occupancy_away[i] = 0
		pressure_against_home[i] = 0.0
		pressure_against_away[i] = 0.0


## Core API: Convert meter position to cell index
func cell_of(pos_m: Vector2) -> int:
	var col := int(clamp(pos_m.x / cell_w_m, 0, COLS - 1))
	var row := int(clamp(pos_m.y / cell_h_m, 0, ROWS - 1))
	return row * COLS + col


## Core API: Get cell center in meters
func cell_center(cell_index: int) -> Vector2:
	var col := cell_index % COLS
	var row := cell_index / COLS
	return Vector2((col + 0.5) * cell_w_m, (row + 0.5) * cell_h_m)


## Core API: Get neighbors (Moore8 pattern)
func neighbors(cell_index: int) -> Array[int]:
	var neighbors_list: Array[int] = []
	var col: int = cell_index % COLS
	var row: int = cell_index / COLS

	for dr in [-1, 0, 1]:
		for dc in [-1, 0, 1]:
			if dr == 0 and dc == 0:
				continue
			var nc: int = col + dc
			var nr: int = row + dr
			if nc >= 0 and nc < COLS and nr >= 0 and nr < ROWS:
				neighbors_list.append(nr * COLS + nc)

	return neighbors_list


## Query API: Get pressure at position
func get_pressure(pos_m: Vector2, home_has_ball: bool) -> float:
	var cell := cell_of(pos_m)
	if home_has_ball:
		return pressure_against_home[cell]
	else:
		return pressure_against_away[cell]


## Query API: Get occupancy at position
func get_occupancy(pos_m: Vector2) -> Dictionary:
	var cell := cell_of(pos_m)
	return {"total": occupancy_total[cell], "home": occupancy_home[cell], "away": occupancy_away[cell]}


## Update heatmaps from Rust snapshot (called each tick)
func update_from_rust_snapshot(snapshot: Dictionary) -> void:
	if snapshot.has("occupancy_total"):
		occupancy_total = snapshot["occupancy_total"]
	if snapshot.has("occupancy_home"):
		occupancy_home = snapshot["occupancy_home"]
	if snapshot.has("occupancy_away"):
		occupancy_away = snapshot["occupancy_away"]
	if snapshot.has("pressure_against_home"):
		pressure_against_home = snapshot["pressure_against_home"]
	if snapshot.has("pressure_against_away"):
		pressure_against_away = snapshot["pressure_against_away"]


## Export for BoardOverlay visualization
func to_overlay_snapshot() -> Dictionary:
	return {
		"grid": {"cols": COLS, "rows": ROWS, "cell_w_m": cell_w_m, "cell_h_m": cell_h_m},
		"heatmaps":
		{
			"occupancy_total": occupancy_total,
			"occupancy_home": occupancy_home,
			"occupancy_away": occupancy_away,
			"pressure_for_home": pressure_against_home,
			"pressure_for_away": pressure_against_away
		}
	}
