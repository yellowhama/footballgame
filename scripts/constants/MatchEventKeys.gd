class_name MatchEventKeys

# Match OS Event SSOT keys (Event Identity + Time)
#
# Invariants (spec):
# - Key presence is 100% (values may be fallback, e.g. -1).
# - Event identity uses track_id (+ optional target_track_id), not names.

const TYPE := "type"
const T_MS := "t_ms"
const PLAYER_TRACK_ID := "player_track_id"
const TARGET_TRACK_ID := "target_track_id"

# Optional / display-only
const PLAYER_NAME := "player"
