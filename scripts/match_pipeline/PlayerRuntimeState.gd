extends RefCounted
class_name PlayerRuntimeState

# Preload to avoid class_name resolution order issues (DO NOT REMOVE - fixes Parser Error)
const _FieldBoard = preload("res://scripts/match_pipeline/FieldBoard.gd")

## PlayerRuntimeState - OS Layer 1 Runtime
## Mutable per-player state (updated each tick)

# Identity (from MatchSetup, immutable)
var track_id: int = -1
var team_id: int = -1
var name: String = ""
var number: int = 0
var position: String = ""

# Runtime state (updated each tick)
var pos_m: Vector2 = Vector2.ZERO  # Current position (meters)
var vel_mps: Vector2 = Vector2.ZERO  # Current velocity (m/s)
var target_pos_m: Vector2 = Vector2.ZERO  # Positioning target (future use)
var stamina: float = 1.0  # 0.0-1.0
var action: String = "idle"  # idle/running/kick/etc

# Cell location (derived from pos_m)
var current_cell: int = -1


## Set identity from MatchSetup
func set_identity(player_metadata: Dictionary) -> void:
	track_id = player_metadata.get("track_id", -1)
	team_id = player_metadata.get("team_id", -1)
	name = player_metadata.get("name", "")
	number = player_metadata.get("number", 0)
	position = player_metadata.get("position", "")


## Update runtime state from snapshot
func update_from_snapshot(snapshot_data: Dictionary, field_board: _FieldBoard = null) -> void:
	# Position
	if snapshot_data.has("pos"):
		pos_m = snapshot_data["pos"]

		# Update cell location if FieldBoard available
		if field_board:
			current_cell = field_board.cell_of(pos_m)

	# Velocity
	if snapshot_data.has("velocity"):
		vel_mps = snapshot_data["velocity"]

	# Stamina
	if snapshot_data.has("stamina"):
		stamina = snapshot_data["stamina"]

	# Action
	if snapshot_data.has("action"):
		action = snapshot_data["action"]


## Get current state summary (for debug)
func to_dict() -> Dictionary:
	return {
		"track_id": track_id,
		"team_id": team_id,
		"name": name,
		"number": number,
		"position": position,
		"pos_m": pos_m,
		"vel_mps": vel_mps,
		"stamina": stamina,
		"action": action,
		"current_cell": current_cell
	}
