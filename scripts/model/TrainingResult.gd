class_name TrainingResult
extends Resource

@export var deltas: Dictionary = {}  # { stat_name: float }
@export var fatigue_delta: float = 0.0
@export var injury: bool = false
@export var message: String = ""
@export var applied: bool = true


static func from_core_dict(d: Dictionary) -> TrainingResult:
	var r := TrainingResult.new()
	r.deltas = d.get("deltas", {})
	r.fatigue_delta = float(d.get("fatigue_delta", 0.0))
	r.injury = bool(d.get("injury", false))
	r.message = str(d.get("message", ""))
	r.applied = bool(d.get("applied", true))
	return r
