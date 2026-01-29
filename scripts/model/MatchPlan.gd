class_name MatchPlan
extends Resource

@export var opponent_name: String = "Friendly FC"
@export var venue: String = "HOME"  # "HOME" | "AWAY" | "NEUTRAL"
@export_range(0.0, 2.0, 0.1) var aggression: float = 1.0
@export_range(0.0, 2.0, 0.1) var tempo: float = 1.0
@export var tactics: Dictionary = {"pressing": 1.0, "width": 1.0, "line_height": 1.0}  # 간단 전술 키-값


func to_core_dict() -> Dictionary:
	return {
		"opponent_name": opponent_name, "venue": venue, "aggression": aggression, "tempo": tempo, "tactics": tactics
	}
