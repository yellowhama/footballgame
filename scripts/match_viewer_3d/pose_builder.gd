extends RefCounted
class_name PoseBuilder

## Dynamic pose detection based on player velocity, action events, and ball position.
## Reference: football-match-viewer/src/features/match/animations/player/PoseBuilder.ts

enum PoseType {
	IDLE,
	WALK,
	RUN,
	SPRINT,
	PASS,
	SHOT,
	HEAD,
	TACKLE,
	DRIBBLE,
	SAVE,  # Goalkeeper save
	THROW_IN,
}

# Speed thresholds (m/s)
const RUN_THRESHOLD := 5.0
const WALK_THRESHOLD := 1.5
const SPRINT_THRESHOLD := 8.0

# Height threshold for header detection (meters)
const HEADER_HEIGHT := 1.5


## Calculate pose type from velocity, action event, and ball height
## velocity: Vector2 in m/s
## action_event: "pass", "shot", "tackle", etc.
## ball_height: Ball height in meters (for header detection)
static func calculate_pose(velocity: Vector2, action_event: String, ball_height: float = 0.0) -> PoseType:
	# 1. Action-based pose (priority)
	if action_event != "":
		var action_lower := action_event.to_lower()
		match action_lower:
			"pass", "long_pass", "cross", "through_ball":
				return PoseType.HEAD if ball_height > HEADER_HEIGHT else PoseType.PASS
			"shot", "strike":
				return PoseType.HEAD if ball_height > HEADER_HEIGHT else PoseType.SHOT
			"tackle", "sliding_tackle":
				return PoseType.TACKLE
			"throw_in":
				return PoseType.THROW_IN
			"save", "dive":
				return PoseType.SAVE
			"dribble", "control":
				return PoseType.DRIBBLE

	# 2. Movement-based pose (fallback)
	var speed := velocity.length()
	if speed > SPRINT_THRESHOLD:
		return PoseType.SPRINT
	elif speed > RUN_THRESHOLD:
		return PoseType.RUN
	elif speed > WALK_THRESHOLD:
		return PoseType.WALK
	return PoseType.IDLE


## Calculate pose from sample dictionary (convenience method)
## sample: {"vx": float, "vy": float, "state": int/string, "action": string}
## ball_height: Ball z position
static func calculate_pose_from_sample(sample: Dictionary, ball_height: float = 0.0) -> PoseType:
	var vx := float(sample.get("vx", 0.0))
	var vy := float(sample.get("vy", 0.0))
	var velocity := Vector2(vx, vy)

	# Check for explicit action
	var action := str(sample.get("action", ""))

	# Also check state field for action hints
	if action == "":
		var state: Variant = sample.get("state", null)
		if state is String:
			action = state

	return calculate_pose(velocity, action, ball_height)


## Convert PoseType enum to string (for backwards compatibility)
static func pose_to_string(pose: PoseType) -> String:
	match pose:
		PoseType.IDLE:
			return "idle"
		PoseType.WALK:
			return "walk"
		PoseType.RUN:
			return "run"
		PoseType.SPRINT:
			return "sprint"
		PoseType.PASS:
			return "pass"
		PoseType.SHOT:
			return "shot"
		PoseType.HEAD:
			return "head"
		PoseType.TACKLE:
			return "tackle"
		PoseType.DRIBBLE:
			return "dribble"
		PoseType.SAVE:
			return "save"
		PoseType.THROW_IN:
			return "throw_in"
		_:
			return "idle"


## Convert string to PoseType enum
static func string_to_pose(pose_str: String) -> PoseType:
	match pose_str.to_lower():
		"idle":
			return PoseType.IDLE
		"walk":
			return PoseType.WALK
		"run":
			return PoseType.RUN
		"sprint":
			return PoseType.SPRINT
		"pass":
			return PoseType.PASS
		"shot":
			return PoseType.SHOT
		"head", "header":
			return PoseType.HEAD
		"tackle":
			return PoseType.TACKLE
		"dribble", "control":
			return PoseType.DRIBBLE
		"save", "dive":
			return PoseType.SAVE
		"throw_in":
			return PoseType.THROW_IN
		_:
			return PoseType.IDLE
