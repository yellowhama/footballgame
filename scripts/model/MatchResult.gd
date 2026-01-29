class_name MatchResult
extends Resource

@export var home_team: String = ""
@export var away_team: String = ""
@export var home_score: int = 0
@export var away_score: int = 0
@export var events: Array = []  # [{minute:int, type:String, player:String, note:String}, ...]
@export var ratings: Dictionary = {}  # {player_name: float}
@export var fatigue_delta: float = 0.0
@export var injury_list: Array = []  # [player_name, ...]
@export var message: String = ""


static func from_core_dict(d: Dictionary) -> MatchResult:
	var r := MatchResult.new()
	r.home_team = str(d.get("home_team", ""))
	r.away_team = str(d.get("away_team", ""))
	r.home_score = int(d.get("home_score", 0))
	r.away_score = int(d.get("away_score", 0))
	r.events = d.get("events", [])
	r.ratings = d.get("ratings", {})
	r.fatigue_delta = float(d.get("fatigue_delta", 0.0))
	r.injury_list = d.get("injury_list", [])
	r.message = str(d.get("message", ""))
	return r
