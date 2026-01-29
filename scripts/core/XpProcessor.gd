## XpProcessor - P0.3 Match Statistics System Integration
## Phase: Game OS v1.1 (Post v1.0)
## Created: 2025-12-19
##
## PURPOSE:
## Convert match events to XP (Experience Points) and apply stat growth to players.
## Implements the Hero Growth system from Rust (HeroXpBucket + HeroMatchGrowth) in GDScript.
##
## DATA FLOW:
## MatchResult.events → convert_events_to_xp() → {attribute: xp_amount}
##                    → calculate_growth() → {attribute: stat_gain}
##                    → apply_growth_to_player() → PlayerData updated
##
## DEPENDENCIES:
## - PlayerData - For reading/writing player attributes
## - MatchEvent from Rust - Event types (goal, pass, tackle, etc.)
##
## REFERENCES:
## - Rust Implementation:
##   - /crates/of_core/src/engine/growth/xp_bucket.rs (XP accumulation)
##   - /crates/of_core/src/engine/growth/match_growth.rs (XP → stat conversion)
##   - /crates/of_core/src/engine/growth/hero_action_tag.rs (Event → XP mapping)
## - Plan: /home/hugh51/.claude/plans/quirky-questing-mountain.md (P0.3)

class_name XpProcessor
extends RefCounted

## ============================================================================
## CONSTANTS - Event → HeroActionTag Mapping
## ============================================================================

## Event type to HeroActionTag mapping
## Based on /crates/of_core/src/engine/growth/hero_action_tag.rs
enum HeroActionTag {
	# Passing types
	SAFE_PASS,  # 안전한 패스 (횡패스, 후방 패스)
	FORWARD_PASS,  # 전진 패스
	THROUGH_PASS,  # 스루 패스 (수비 라인 뚫기)
	LOB_PASS,  # 롱패스 / 크로스
	# Dribbling types
	SAFE_DRIBBLE,  # 안전한 드리블
	DRIBBLE_PAST,  # 1v1 돌파 성공
	# Shooting types
	BOX_SHOT,  # 박스 내 슈팅
	LONG_SHOT,  # 중거리 슛
	HEADER_SHOT,  # 헤딩 슛
	# Defensive types
	INTERCEPTION,  # 패스 차단
	TACKLE,  # 태클 성공
	AERIAL_DUEL,  # 공중볼 경합 승리
}

## Growth thresholds (XP needed for +1 stat)
## From /crates/of_core/src/engine/growth/match_growth.rs
const GROWTH_THRESHOLDS = {
	"0-40": 10.0,
	"41-60": 15.0,
	"61-75": 25.0,
	"76-85": 40.0,
	"86-90": 60.0,
	"91-95": 100.0,
	"96-99": 200.0,
	"99+": 500.0  # Practically impossible
}

## ============================================================================
## PUBLIC API
## ============================================================================


## Convert match events to XP accumulation per attribute
##
## PARAMETERS:
##   events: Array[Dictionary] - MatchEvent array from MatchResult
##   player_uid: String - UID of the player to extract XP for
##   match_setup: MatchSetup - For resolving player names to track_id/UID
##
## RETURNS:
##   Dictionary {attribute_name: xp_amount} - XP accumulated per attribute
##
## ALGORITHM:
##   1. Filter events for specified player_uid
##   2. Map each event type → HeroActionTag
##   3. Calculate XP based on tag, success, pressure, etc.
##   4. Distribute XP to affected attributes based on tag weights
##   5. Return accumulated XP per attribute
static func convert_events_to_xp(events: Array, player_uid: String, match_setup) -> Dictionary:
	var xp_bucket: Dictionary = {}  # {attribute_name: xp_amount}

	if match_setup == null:
		push_error("[XpProcessor] MatchSetup is null")
		return xp_bucket

	# C7: Resolve hero track_id from UID (one-time lookup)
	var hero_track_id: int = -1
	const TOTAL_SLOTS = 22
	for track_id in range(TOTAL_SLOTS):
		var slot = match_setup.get_slot(track_id)
		if slot and slot.player and slot.player.uid == player_uid:
			hero_track_id = track_id
			break

	if hero_track_id == -1:
		return xp_bucket  # Player not in this match

	# Process events
	for event in events:
		if not event is Dictionary:
			continue

		var event_type: String = event.get("type", "")
		var track_id: int = event.get("player_track_id", -1)  # C7: Direct track_id access
		var minute: int = event.get("minute", 0)
		var details: Dictionary = event.get("details", {})

		# C7: Only process events involving our hero player
		if track_id != hero_track_id:
			# Check if hero assisted via target_track_id
			var target_id: int = event.get("target_track_id", -1)
			if target_id != hero_track_id:
				continue  # Not our hero's event

		# Map event type to HeroActionTag
		var tag = _event_to_hero_action_tag(event_type, details)
		if tag == -1:
			continue  # No XP for this event type

		# Calculate XP for this event
		var success: bool = _is_event_successful(event_type, details)
		var base_xp: float = _get_base_xp(tag)
		var total_xp: float = base_xp * (1.5 if success else 0.5)

		# Distribute XP to affected attributes
		var affected_attrs = _get_affected_attributes(tag)
		for attr_data in affected_attrs:
			var attr_name: String = attr_data[0]
			var weight: float = attr_data[1]
			var attr_xp: float = total_xp * weight

			xp_bucket[attr_name] = xp_bucket.get(attr_name, 0.0) + attr_xp

	return xp_bucket


## Calculate stat growth from XP bucket
##
## PARAMETERS:
##   xp_bucket: Dictionary {attribute_name: xp_amount} - From convert_events_to_xp()
##   player_data: PlayerData or Dictionary - Current player stats
##   xp_overflow: Dictionary - Overflow XP from previous match (optional)
##
## RETURNS:
##   Dictionary {
##     "stat_gains": {attribute_name: gain_amount},  # +1, +2, or +3
##     "xp_overflow": {attribute_name: leftover_xp}, # For next match
##     "total_xp": float,
##     "highlights": [[attr_name, gain], ...]
##   }
##
## ALGORITHM (from HeroMatchGrowth.from_bucket):
##   1. Apply overflow XP from previous match
##   2. For each attribute with XP:
##      - Get current stat value
##      - Get growth threshold based on current value
##      - If XP >= threshold: calculate gain (max +3)
##      - Store leftover XP as overflow
##   3. Return gains + overflow
static func calculate_growth(xp_bucket: Dictionary, player_data, xp_overflow: Dictionary = {}) -> Dictionary:
	var stat_gains: Dictionary = {}
	var new_overflow: Dictionary = {}
	var highlights: Array = []
	var total_xp: float = 0.0

	# Apply overflow XP from previous match
	var combined_xp: Dictionary = xp_bucket.duplicate()
	for attr in xp_overflow:
		combined_xp[attr] = combined_xp.get(attr, 0.0) + xp_overflow[attr]

	# Calculate total XP
	for xp in combined_xp.values():
		total_xp += xp

	# Process each attribute
	for attr_name in combined_xp:
		var xp: float = combined_xp[attr_name]

		# Get current stat value
		var current_stat: int = _get_player_attribute(player_data, attr_name)

		# Get growth threshold
		var threshold: float = _get_growth_threshold(current_stat)

		if xp >= threshold:
			# Calculate growth points (max +3)
			var raw_points: int = int(xp / threshold)
			var points: int = mini(raw_points, 3)
			var leftover: float = xp - (points * threshold)

			if points > 0:
				stat_gains[attr_name] = points
				highlights.append([attr_name, points])

			new_overflow[attr_name] = leftover
		else:
			# XP below threshold → carry forward
			new_overflow[attr_name] = xp

	# Sort highlights by gain amount (descending)
	highlights.sort_custom(func(a, b): return a[1] > b[1])

	return {"stat_gains": stat_gains, "xp_overflow": new_overflow, "total_xp": total_xp, "highlights": highlights}


## Apply stat growth to PlayerData instance
##
## PARAMETERS:
##   player_data: PlayerData (Node) - The player to apply growth to
##   growth: Dictionary - Result from calculate_growth()
##
## SIDE EFFECTS:
##   - Updates player_data stats (+1-3 per attribute)
##   - Updates player_data.xp_overflow
##   - Recalculates player_data.current_ca
##   - Emits player_data.stats_changed signal
static func apply_growth_to_player(player_data, growth: Dictionary) -> void:
	if player_data == null:
		push_error("[XpProcessor] PlayerData is null")
		return

	var stat_gains: Dictionary = growth.get("stat_gains", {})
	var xp_overflow: Dictionary = growth.get("xp_overflow", {})

	if stat_gains.is_empty():
		print("[XpProcessor] No stat growth this match (XP below threshold)")
	else:
		print("[XpProcessor] Applying stat growth:")

	# Apply stat gains
	for attr_name in stat_gains:
		var gain: int = stat_gains[attr_name]
		var current: int = _get_player_attribute(player_data, attr_name)
		var new_value: int = current + gain

		# Apply to PlayerData
		if player_data.has_method("_set_stat_value"):
			player_data._set_stat_value(attr_name, new_value)
		else:
			# Fallback: direct stat dictionary access
			_set_player_attribute_direct(player_data, attr_name, new_value)

		print("  %s: %d → %d (+%d)" % [attr_name, current, new_value, gain])

	# Update overflow XP
	if player_data.has("xp_overflow"):
		player_data.xp_overflow = xp_overflow

	# Recalculate CA
	if player_data.has_method("_recalculate_openfootball_ca"):
		player_data._recalculate_openfootball_ca()

	# Emit stats_changed signal
	if player_data.has_signal("stats_changed") and not stat_gains.is_empty():
		player_data.emit_signal("stats_changed", stat_gains)


## ============================================================================
## PRIVATE HELPERS
## ============================================================================


## Map MatchEvent type to HeroActionTag
static func _event_to_hero_action_tag(event_type: String, details: Dictionary) -> int:
	match event_type:
		"goal":
			# Determine shot type from details (box shot vs long shot vs header)
			# TODO: Use ball_position from details to determine box vs long
			return HeroActionTag.BOX_SHOT

		"shot", "shot_on_target":
			return HeroActionTag.BOX_SHOT

		"shot_off_target", "shot_blocked":
			return HeroActionTag.LONG_SHOT

		"pass":
			# TODO: Classify pass type (safe, forward, through, lob)
			return HeroActionTag.FORWARD_PASS

		"key_chance":
			return HeroActionTag.THROUGH_PASS

		"dribble":
			return HeroActionTag.DRIBBLE_PAST

		"tackle":
			return HeroActionTag.TACKLE

		"save":
			# Goalkeeper save - no direct mapping, skip for now
			return -1

		_:
			return -1  # No XP for this event


## Check if event was successful
static func _is_event_successful(event_type: String, details: Dictionary) -> bool:
	match event_type:
		"goal", "shot_on_target", "pass", "tackle", "dribble", "save":
			return true
		"shot_off_target", "shot_blocked":
			return false
		_:
			return true  # Default: success


## Get base XP for HeroActionTag (from hero_action_tag.rs)
static func _get_base_xp(tag: int) -> float:
	match tag:
		HeroActionTag.SAFE_PASS:
			return 1.0
		HeroActionTag.FORWARD_PASS:
			return 2.0
		HeroActionTag.THROUGH_PASS:
			return 5.0
		HeroActionTag.LOB_PASS:
			return 3.0
		HeroActionTag.SAFE_DRIBBLE:
			return 2.0
		HeroActionTag.DRIBBLE_PAST:
			return 6.0
		HeroActionTag.BOX_SHOT:
			return 4.0
		HeroActionTag.LONG_SHOT:
			return 3.0
		HeroActionTag.HEADER_SHOT:
			return 4.0
		HeroActionTag.INTERCEPTION:
			return 4.0
		HeroActionTag.TACKLE:
			return 3.0
		HeroActionTag.AERIAL_DUEL:
			return 3.0
		_:
			return 0.0


## Get affected attributes and weights for HeroActionTag
## Returns: Array of [attribute_name: String, weight: float]
## Weights sum to 1.0
static func _get_affected_attributes(tag: int) -> Array:
	match tag:
		HeroActionTag.SAFE_PASS:
			return [["passing", 0.6], ["composure", 0.4]]
		HeroActionTag.FORWARD_PASS:
			return [["passing", 0.5], ["vision", 0.3], ["decisions", 0.2]]
		HeroActionTag.THROUGH_PASS:
			return [["passing", 0.4], ["vision", 0.4], ["decisions", 0.2]]
		HeroActionTag.LOB_PASS:
			return [["passing", 0.5], ["technique", 0.3], ["vision", 0.2]]
		HeroActionTag.SAFE_DRIBBLE:
			return [["dribbling", 0.5], ["composure", 0.3], ["first_touch", 0.2]]
		HeroActionTag.DRIBBLE_PAST:
			return [["dribbling", 0.5], ["agility", 0.3], ["flair", 0.2]]
		HeroActionTag.BOX_SHOT:
			return [["finishing", 0.6], ["composure", 0.3], ["technique", 0.1]]
		HeroActionTag.LONG_SHOT:
			return [["long_shots", 0.5], ["technique", 0.3], ["composure", 0.2]]
		HeroActionTag.HEADER_SHOT:
			return [["finishing", 0.4], ["jumping", 0.4], ["strength", 0.2]]
		HeroActionTag.INTERCEPTION:
			return [["anticipation", 0.5], ["positioning", 0.3], ["decisions", 0.2]]
		HeroActionTag.TACKLE:
			return [["tackling", 0.6], ["strength", 0.2], ["aggression", 0.2]]
		HeroActionTag.AERIAL_DUEL:
			return [["jumping", 0.5], ["strength", 0.3], ["marking", 0.2]]
		_:
			return []


## Get growth threshold based on current stat value
static func _get_growth_threshold(current_stat: int) -> float:
	if current_stat <= 40:
		return GROWTH_THRESHOLDS["0-40"]
	elif current_stat <= 60:
		return GROWTH_THRESHOLDS["41-60"]
	elif current_stat <= 75:
		return GROWTH_THRESHOLDS["61-75"]
	elif current_stat <= 85:
		return GROWTH_THRESHOLDS["76-85"]
	elif current_stat <= 90:
		return GROWTH_THRESHOLDS["86-90"]
	elif current_stat <= 95:
		return GROWTH_THRESHOLDS["91-95"]
	elif current_stat <= 99:
		return GROWTH_THRESHOLDS["96-99"]
	else:
		return GROWTH_THRESHOLDS["99+"]


## Get player attribute value (handles both PlayerData Node and Dictionary)
static func _get_player_attribute(player_data, attr_name: String) -> int:
	# Try PlayerData node methods first
	if player_data.has_method("get_attribute"):
		return player_data.get_attribute(attr_name)

	# Try direct property access (PlayerData has technical_stats, mental_stats, etc.)
	if player_data.has("technical_stats") and attr_name in player_data.technical_stats:
		return player_data.technical_stats[attr_name]
	if player_data.has("mental_stats") and attr_name in player_data.mental_stats:
		return player_data.mental_stats[attr_name]
	if player_data.has("physical_stats") and attr_name in player_data.physical_stats:
		return player_data.physical_stats[attr_name]
	if player_data.has("goalkeeper_stats") and attr_name in player_data.goalkeeper_stats:
		return player_data.goalkeeper_stats[attr_name]

	# Try dictionary access (for Dictionary player_data)
	if player_data is Dictionary:
		if player_data.has("technical") and attr_name in player_data.technical:
			return player_data.technical[attr_name]
		if player_data.has("mental") and attr_name in player_data.mental:
			return player_data.mental[attr_name]
		if player_data.has("physical") and attr_name in player_data.physical:
			return player_data.physical[attr_name]
		if player_data.has("goalkeeper") and attr_name in player_data.goalkeeper:
			return player_data.goalkeeper[attr_name]

	push_warning("[XpProcessor] Could not find attribute '%s' in player_data" % attr_name)
	return 50  # Default fallback


## Set player attribute (direct access fallback)
static func _set_player_attribute_direct(player_data, attr_name: String, value: int) -> void:
	if player_data.has("technical_stats") and attr_name in player_data.technical_stats:
		player_data.technical_stats[attr_name] = value
	elif player_data.has("mental_stats") and attr_name in player_data.mental_stats:
		player_data.mental_stats[attr_name] = value
	elif player_data.has("physical_stats") and attr_name in player_data.physical_stats:
		player_data.physical_stats[attr_name] = value
	elif player_data.has("goalkeeper_stats") and attr_name in player_data.goalkeeper_stats:
		player_data.goalkeeper_stats[attr_name] = value
	else:
		push_warning("[XpProcessor] Could not set attribute '%s'" % attr_name)
