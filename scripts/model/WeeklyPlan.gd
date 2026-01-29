class_name WeeklyPlan
extends Resource

const TrainingPlanScript = preload("res://scripts/model/TrainingPlan.gd")

@export var team_plan: Dictionary = {}
@export var personal_plan: Resource  # TrainingPlan
@export var personal_kind: String = "Rest"
@export var day_entries: Dictionary = {}


func set_rest() -> void:
	personal_kind = "Rest"
	personal_plan = null


func set_training(plan) -> void:  # param is TrainingPlan
	personal_kind = "Training"
	personal_plan = plan


func is_rest() -> bool:
	return personal_kind == "Rest" or personal_plan == null


func get_description() -> String:
	if is_rest():
		return "이미 예정된 주간 휴식"
	else:
		if personal_plan:
			return "개인훈련: %s (강도 %.1f)" % [personal_plan.kind, personal_plan.intensity]
		return "개인훈련 미지정"


func set_day_entry(day_of_week: int, entry: Dictionary) -> void:
	day_entries[day_of_week] = entry.duplicate(true) if entry else {}


func get_day_entry(day_of_week: int) -> Dictionary:
	return day_entries.get(day_of_week, {}).duplicate(true)


func to_dict() -> Dictionary:
	var result: Dictionary = {"team_plan": team_plan.duplicate(true), "personal_kind": personal_kind, "day_entries": {}}
	if personal_plan:
		result["personal_plan"] = personal_plan.to_dict()
	for day in day_entries.keys():
		result["day_entries"][str(day)] = day_entries[day]
	return result


static func from_dict(data: Dictionary) -> Resource:
	var resource: Resource = (load("res://scripts/model/WeeklyPlan.gd") as GDScript).new()
	resource.team_plan = data.get("team_plan", {}).duplicate(true)
	resource.personal_kind = data.get("personal_kind", "Rest")
	if data.has("personal_plan"):
		resource.personal_plan = TrainingPlanScript.from_dict(data["personal_plan"])
	var entries: Dictionary = data.get("day_entries", {})
	for key in entries.keys():
		var day_idx := int(key)
		resource.set_day_entry(day_idx, entries[key])
	return resource
