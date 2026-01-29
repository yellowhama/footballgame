class_name TrainingPlan
extends Resource

@export_enum("Physical", "Technique", "Shooting", "Defense", "Rest", "GoOut", "Hospital") var kind: String = "Rest"

@export_range(0.5, 1.5, 0.1) var intensity: float = 1.0


func to_core_dict() -> Dictionary:
	return {"kind": kind, "intensity": intensity}


func to_dict() -> Dictionary:
	return {"kind": kind, "intensity": intensity}


static func from_dict(data: Dictionary) -> Resource:
	var plan: Resource = (load("res://scripts/model/TrainingPlan.gd") as GDScript).new()
	plan.kind = data.get("kind", plan.kind)
	plan.intensity = float(data.get("intensity", plan.intensity))
	return plan
