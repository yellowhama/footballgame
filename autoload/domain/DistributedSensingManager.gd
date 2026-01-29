extends Node
## DistributedSensingManager - Distributed Sensing Analytics (DSA) v1
##
## Read-only analytics layer:
## - Consumes `UnifiedFramePipeline.snapshot_ready(t_ms, snapshot)` (StandardSnapshot SSOT)
## - Produces derived telemetry only (InsightFrame + optional match report)
## - MUST NOT mutate match state, events, or existing analytics SSOTs
##
## Spec: docs/specs/fix_2601/0114/DSA_V1_SPEC_AND_CODE_AUDIT.md

signal insight_frame_ready(frame: Dictionary)
signal match_report_ready(report: Dictionary)

const FIELD_LENGTH_M := 105.0
const FIELD_WIDTH_M := 68.0

const PRESSURE_COLS := 28
const PRESSURE_ROWS := 18
const PRESSURE_CELL_W_M := FIELD_LENGTH_M / float(PRESSURE_COLS)
const PRESSURE_CELL_H_M := FIELD_WIDTH_M / float(PRESSURE_ROWS)
const PRESSURE_NORM_CAP := 3.0

const ZONE_LANES := 5
const ZONE_QUARTERS := 4
const ZONE_COUNT := 20

const RING_BUFFER_WINDOW_MS := 60_000
const LANE_LOAD_WINDOW_MS := 1_000

# Transition heuristic tuning (v1)
const PASS_HEURISTIC_MAX_MS := 500

# Hub score weights (v1 proxy only; authoritative post-match lives in Rust metrics)
const HUB_W_OWNER_TIME_S := 1.0
const HUB_W_RECV := 2.0
const HUB_W_REL := 2.0

var _pipeline: Node = null

# Timeline guards
var _last_t_ms: int = -1
var _last_ball_pos: Vector2 = Vector2.ZERO
var _last_ball_zone_id: int = -1
var _last_owner_track_id: int = -2

# Live outputs
var _insight_frames: Array = []  # [{...InsightFrame...}]
var _lane_events: Array = []  # [{t_ms, lane, kind, team_id}]

# Transition matrices (20x20 flattened)
var _home_zone_transitions: PackedInt32Array = PackedInt32Array()
var _away_zone_transitions: PackedInt32Array = PackedInt32Array()
var _last_transition: Dictionary = {}

# Hub tracking (22 players)
var _owner_time_s: Array = []  # float[22]
var _recv_count: Array = []  # int[22]
var _rel_count: Array = []  # int[22]

# Zone dwell (per team, seconds)
var _home_zone_dwell_s: Array = []  # float[20]
var _away_zone_dwell_s: Array = []  # float[20]

# Minute-level aggregates
var _minute_aggs: Dictionary = {}  # minute_idx -> {samples, pressure_sum, ball_speed_sum, transitions}
var _match_report_emitted: bool = false


func _ready() -> void:
        _reset_all()
        call_deferred("_connect_pipeline")


func _exit_tree() -> void:
        if _pipeline and _pipeline.has_signal("snapshot_ready"):
                var c := Callable(self, "_on_snapshot_ready")
                if _pipeline.snapshot_ready.is_connected(c):
                        _pipeline.snapshot_ready.disconnect(c)


func _connect_pipeline() -> void:
        if _pipeline != null:
                return
        if not has_node("/root/UnifiedFramePipeline"):
                if OS.is_debug_build():
                        push_warning("[DSA] UnifiedFramePipeline not found; DSA disabled")
                return

        _pipeline = get_node("/root/UnifiedFramePipeline")
        _pipeline.snapshot_ready.connect(_on_snapshot_ready)

        if OS.is_debug_build():
                print("[DSA] Connected to UnifiedFramePipeline.snapshot_ready")


func _reset_all() -> void:
        _last_t_ms = -1
        _last_ball_pos = Vector2.ZERO
        _last_ball_zone_id = -1
        _last_owner_track_id = -2
        _insight_frames.clear()
        _lane_events.clear()
        _minute_aggs.clear()
        _match_report_emitted = false
        _last_transition = {}

        _home_zone_transitions = PackedInt32Array()
        _home_zone_transitions.resize(ZONE_COUNT * ZONE_COUNT)
        _away_zone_transitions = PackedInt32Array()
        _away_zone_transitions.resize(ZONE_COUNT * ZONE_COUNT)
        for i in range(ZONE_COUNT * ZONE_COUNT):
                _home_zone_transitions[i] = 0
                _away_zone_transitions[i] = 0

        _owner_time_s = []
        _recv_count = []
        _rel_count = []
        _owner_time_s.resize(22)
        _recv_count.resize(22)
        _rel_count.resize(22)
        for i in range(22):
                _owner_time_s[i] = 0.0
                _recv_count[i] = 0
                _rel_count[i] = 0

        _home_zone_dwell_s = []
        _away_zone_dwell_s = []
        _home_zone_dwell_s.resize(ZONE_COUNT)
        _away_zone_dwell_s.resize(ZONE_COUNT)
        for i in range(ZONE_COUNT):
                _home_zone_dwell_s[i] = 0.0
                _away_zone_dwell_s[i] = 0.0


## Public: Get last N seconds of insight frames (UI consumers can pull)
func get_recent_insight_frames() -> Array:
        return _insight_frames.duplicate(true)


## Public: Get minute aggregates (for post-match screen glue)
func get_minute_aggregates() -> Dictionary:
        return _minute_aggs.duplicate(true)


## Public: Get UI-ready minute series arrays (derived from minute aggregates).
## Returns {} when no aggregates exist yet.
## Shape: {"pressure":[0..1], "tempo":[m/s], "transitions":[count]}
func get_minute_series(duration_minutes: int) -> Dictionary:
        if _minute_aggs.is_empty():
                return {}

        var duration: int = maxi(0, int(duration_minutes))
        var pressure_by_minute: Array = []
        var tempo_by_minute: Array = []
        var transitions_by_minute: Array = []

        pressure_by_minute.resize(duration + 1)
        tempo_by_minute.resize(duration + 1)
        transitions_by_minute.resize(duration + 1)
        for i in range(duration + 1):
                pressure_by_minute[i] = 0.0
                tempo_by_minute[i] = 0.0
                transitions_by_minute[i] = 0.0

        for minute_idx in range(duration + 1):
                var agg: Dictionary = _minute_aggs.get(minute_idx, {})
                if agg.is_empty():
                        continue

                var samples: int = int(agg.get("samples", 0))
                if samples > 0:
                        pressure_by_minute[minute_idx] = float(agg.get("pressure_sum", 0.0)) / float(samples)
                        tempo_by_minute[minute_idx] = float(agg.get("ball_speed_sum", 0.0)) / float(samples)
                        transitions_by_minute[minute_idx] = float(agg.get("transitions", 0))

        return {
                "pressure": pressure_by_minute,
                "tempo": tempo_by_minute,
                "transitions": transitions_by_minute,
        }


func _on_snapshot_ready(t_ms: int, snapshot: Dictionary) -> void:
        if snapshot == null or snapshot.is_empty():
                return

        # Guard against replay scrubs / non-monotonic playhead
        if _last_t_ms >= 0 and t_ms < _last_t_ms:
                _reset_all()

        var dt_ms := 0
        if _last_t_ms >= 0:
                dt_ms = t_ms - _last_t_ms
        var dt_s: float = maxf(0.0, float(dt_ms) / 1000.0)

        var ball: Dictionary = snapshot.get("ball", {})
        var ball_pos: Vector2 = ball.get("pos", Vector2.ZERO)
        var owner_tid: int = int(ball.get("owner_id", -1))

        var players: Dictionary = snapshot.get("players", {})
        var owner_pos: Vector2 = ball_pos
        if owner_tid >= 0 and players.has(str(owner_tid)):
                var owner_meta: Dictionary = players.get(str(owner_tid), {})
                owner_pos = owner_meta.get("pos", ball_pos)

        var home_has_ball := _team_id_for_track(owner_tid) == 0

        var pressure_norm := _compute_local_pressure_norm(snapshot, owner_pos, owner_tid, home_has_ball)

        var ball_zone_id := pos_to_posplay_zone_id_world(ball_pos)
        var ball_lane_id := _lane_of_zone_id(ball_zone_id)

        var ball_speed := 0.0
        if dt_s > 0.0001 and _last_ball_pos != Vector2.ZERO:
                ball_speed = ball_pos.distance_to(_last_ball_pos) / dt_s

        # Hub + dwell update
        if dt_s > 0.0 and owner_tid >= 0 and owner_tid < 22:
                _owner_time_s[owner_tid] += dt_s
                var team_id := _team_id_for_track(owner_tid)
                if team_id == 0:
                        _home_zone_dwell_s[ball_zone_id] += dt_s
                elif team_id == 1:
                        _away_zone_dwell_s[ball_zone_id] += dt_s

        # Transition update (heuristic: zone change + owner change classification)
        var transition_kind := ""
        var transition_team_id := -1

        if _last_ball_zone_id >= 0 and ball_zone_id != _last_ball_zone_id:
                if owner_tid == _last_owner_track_id and owner_tid >= 0:
                        transition_kind = "carry"
                        transition_team_id = _team_id_for_track(owner_tid)
                elif owner_tid != _last_owner_track_id and owner_tid >= 0 and _last_owner_track_id >= 0:
                        # Prefer event-confirmed pass if available (survives DeltaFilter cadence)
                        var pass_confident := _has_pass_event(snapshot, _last_owner_track_id, owner_tid)
                        var team_from := _team_id_for_track(_last_owner_track_id)
                        var team_to := _team_id_for_track(owner_tid)
                        if team_from == team_to and team_from >= 0 and (pass_confident or dt_ms <= PASS_HEURISTIC_MAX_MS):
                                transition_kind = "pass"
                                transition_team_id = team_from
                        else:
                                transition_kind = "unknown"
                                transition_team_id = -1
                else:
                        transition_kind = "unknown"
                        transition_team_id = -1

                _last_transition = {"from": _last_ball_zone_id, "to": ball_zone_id, "kind": transition_kind}

                if transition_team_id >= 0:
                        _bump_transition_matrix(transition_team_id, _last_ball_zone_id, ball_zone_id)

                _lane_events.append(
                        {"t_ms": t_ms, "lane": ball_lane_id, "kind": transition_kind, "team_id": transition_team_id}
                )

        # Owner change proxy (hub counts)
        if owner_tid != _last_owner_track_id and owner_tid >= 0 and _last_owner_track_id >= 0:
                if owner_tid < 22:
                        _recv_count[owner_tid] += 1
                if _last_owner_track_id < 22:
                        _rel_count[_last_owner_track_id] += 1

        var lane_load_5 := _compute_lane_load_5(t_ms)
        var hub_top3 := _compute_hub_top3()
        var hub_gini_proxy := _compute_hub_gini_proxy()

        var qa_flags := _compute_qa_flags(hub_gini_proxy, lane_load_5, ball_zone_id, owner_tid)

        # Minute aggregates (very cheap)
        _update_minute_aggs(t_ms, pressure_norm, ball_speed, (transition_kind != ""))

        var frame: Dictionary = {
                "t_ms": t_ms,
                "ball_owner_track_id": owner_tid,
                "ball_zone_id": ball_zone_id,
                "ball_lane_id": ball_lane_id,
                "local_pressure": pressure_norm,
                "lane_load_5": lane_load_5,
                "zone_transition_last": _last_transition,
                "hub_top3": hub_top3,
                "hub_gini_proxy": hub_gini_proxy,
                "qa_flags": qa_flags
        }

        _push_frame_and_trim(t_ms, frame)
        insight_frame_ready.emit(frame)

        # Fulltime report hook (optional v1)
        if not _match_report_emitted and _has_fulltime_event(snapshot):
                _match_report_emitted = true
                match_report_ready.emit(_build_match_report())

        _last_t_ms = t_ms
        _last_ball_pos = ball_pos
        _last_ball_zone_id = ball_zone_id
        _last_owner_track_id = owner_tid


func _compute_local_pressure_norm(
        snapshot: Dictionary, sample_pos: Vector2, owner_tid: int, home_has_ball: bool
) -> float:
        var p_home := _sample_pressure(snapshot.get("pressure_against_home", null), sample_pos)
        var p_away := _sample_pressure(snapshot.get("pressure_against_away", null), sample_pos)

        var owner_team := _team_id_for_track(owner_tid)
        var p := 0.0
        if owner_team == 0:
                p = p_home
        elif owner_team == 1:
                p = p_away
        else:
                # Loose ball: best-effort max
                p = max(p_home, p_away)

        return clamp(p / PRESSURE_NORM_CAP, 0.0, 1.0)


func _sample_pressure(pressure_map: Variant, pos_m: Vector2) -> float:
        if pressure_map == null:
                return 0.0
        var size_ok := false
        if pressure_map is PackedFloat32Array:
                size_ok = (pressure_map as PackedFloat32Array).size() >= PRESSURE_COLS * PRESSURE_ROWS
        elif pressure_map is Array:
                size_ok = (pressure_map as Array).size() >= PRESSURE_COLS * PRESSURE_ROWS
        if not size_ok:
                return 0.0

        var col := int(clamp(pos_m.x / PRESSURE_CELL_W_M, 0.0, float(PRESSURE_COLS - 1)))
        var row := int(clamp(pos_m.y / PRESSURE_CELL_H_M, 0.0, float(PRESSURE_ROWS - 1)))
        var idx := row * PRESSURE_COLS + col
        return float(pressure_map[idx])


func _has_pass_event(snapshot: Dictionary, from_tid: int, to_tid: int) -> bool:
        var events: Array = snapshot.get("events", [])
        if events.is_empty():
                return false
        for ev in events:
                if not (ev is Dictionary):
                        continue
                var typ := str(ev.get("type", ev.get("kind", ""))).strip_edges().to_lower()
                if typ not in ["pass", "cross", "switch_play"]:
                        continue
                var actor := int(ev.get("player_track_id", ev.get("track_id", -1)))
                var target := int(ev.get("target_track_id", -1))
                if actor == from_tid and (target < 0 or target == to_tid):
                        return true
        return false


func _has_fulltime_event(snapshot: Dictionary) -> bool:
        var events: Array = snapshot.get("events", [])
        if events.is_empty():
                return false
        for ev in events:
                if not (ev is Dictionary):
                        continue
                var typ := str(ev.get("type", ev.get("kind", ""))).strip_edges().to_lower()
                if typ == "fulltime":
                        return true
        return false


func _push_frame_and_trim(t_ms: int, frame: Dictionary) -> void:
        _insight_frames.append(frame)
        var min_t := t_ms - RING_BUFFER_WINDOW_MS
        while _insight_frames.size() > 0 and int(_insight_frames[0].get("t_ms", 0)) < min_t:
                _insight_frames.pop_front()


func _compute_lane_load_5(t_ms: int) -> Array:
        var min_t := t_ms - LANE_LOAD_WINDOW_MS
        while _lane_events.size() > 0 and int(_lane_events[0].get("t_ms", 0)) < min_t:
                _lane_events.pop_front()

        var counts := [0, 0, 0, 0, 0]
        for e in _lane_events:
                var lane := int(e.get("lane", -1))
                if lane >= 0 and lane < 5:
                        counts[lane] += 1

        var total := 0
        for c in counts:
                total += int(c)
        if total <= 0:
                return [0.0, 0.0, 0.0, 0.0, 0.0]

        return [
                float(counts[0]) / float(total),
                float(counts[1]) / float(total),
                float(counts[2]) / float(total),
                float(counts[3]) / float(total),
                float(counts[4]) / float(total)
        ]


func _compute_hub_scores() -> Array:
        var hubs: Array = []
        hubs.resize(22)
        for tid in range(22):
                var score := HUB_W_OWNER_TIME_S * float(_owner_time_s[tid]) + HUB_W_RECV * float(_recv_count[tid]) + HUB_W_REL * float(_rel_count[tid])
                hubs[tid] = score
        return hubs


func _compute_hub_top3() -> Array:
        var hubs := _compute_hub_scores()
        var pairs: Array = []
        for tid in range(22):
                pairs.append({"track_id": tid, "score": hubs[tid]})
        pairs.sort_custom(func(a, b): return float(a.score) > float(b.score))
        return pairs.slice(0, 3)


func _compute_hub_gini_proxy() -> float:
        var hubs := _compute_hub_scores()
        var values: Array = []
        for v in hubs:
                values.append(max(0.0, float(v)))
        return gini(values)


func _compute_qa_flags(hub_gini: float, lane_load_5: Array, ball_zone_id: int, owner_tid: int) -> Array:
        var flags: Array = []

        if hub_gini >= 0.65:
                flags.append("HUB_OVERHEAT_WARN")

        var max_lane := 0.0
        for v in lane_load_5:
                max_lane = max(max_lane, float(v))
        if max_lane >= 0.45:
                flags.append("LANE_IMBALANCE_WARN")

        # Simple “zone dwell skew” proxy: if ball lives in BOX zones too long, flag.
        # (v1: purely heuristic; post-match uses Rust analysis)
        if owner_tid >= 0:
                var quarter := int(ball_zone_id / 5)
                if quarter == 3 and _time_in_box_share(_team_id_for_track(owner_tid)) >= 0.35:
                        flags.append("BOX_DWELL_WARN")

        return flags


func _time_in_box_share(team_id: int) -> float:
        var dwell: Array = _home_zone_dwell_s if team_id == 0 else _away_zone_dwell_s
        var total := 0.0
        for v in dwell:
                total += float(v)
        if total <= 0.00001:
                return 0.0
        var box_sum := 0.0
        for zone_id in range(15, 20):
                box_sum += float(dwell[zone_id])
        return box_sum / total


func _update_minute_aggs(t_ms: int, pressure_norm: float, ball_speed: float, had_transition: bool) -> void:
        var minute_idx := int(t_ms / 60000)
        var agg: Dictionary = _minute_aggs.get(minute_idx, {"samples": 0, "pressure_sum": 0.0, "ball_speed_sum": 0.0, "transitions": 0})
        agg.samples = int(agg.samples) + 1
        agg.pressure_sum = float(agg.pressure_sum) + pressure_norm
        agg.ball_speed_sum = float(agg.ball_speed_sum) + ball_speed
        if had_transition:
                agg.transitions = int(agg.transitions) + 1
        _minute_aggs[minute_idx] = agg


func _build_match_report() -> Dictionary:
        # v1: Godot-only lightweight report (UI glue can merge with Rust match_analysis externally)
        return {
                "minute_aggs": get_minute_aggregates(),
                "home_zone_transitions": _home_zone_transitions,
                "away_zone_transitions": _away_zone_transitions,
                "hub_gini_proxy": _compute_hub_gini_proxy(),
                "hub_top3": _compute_hub_top3()
        }


func _bump_transition_matrix(team_id: int, from_zone: int, to_zone: int) -> void:
        if from_zone < 0 or from_zone >= ZONE_COUNT or to_zone < 0 or to_zone >= ZONE_COUNT:
                return
        var idx := from_zone * ZONE_COUNT + to_zone
        if team_id == 0:
                _home_zone_transitions[idx] += 1
        elif team_id == 1:
                _away_zone_transitions[idx] += 1


static func _team_id_for_track(track_id: int) -> int:
        if track_id < 0:
                return -1
        return 0 if track_id < 11 else 1


static func _lane_of_zone_id(zone_id: int) -> int:
        return zone_id % 5 if zone_id >= 0 else -1


## SSOT-compatible 20-zone mapping (world-space v1, no mirroring).
## Index order matches Rust PosPlayZoneId.index(): quarter*5 + lane.
static func pos_to_posplay_zone_id_world(pos_m: Vector2) -> int:
        var x01: float = clampf(pos_m.x / FIELD_LENGTH_M, 0.0, 0.999999)
        var y01: float = clampf(pos_m.y / FIELD_WIDTH_M, 0.0, 0.999999)

        var lane := 4
        if y01 < 0.2:
                lane = 0
        elif y01 < 0.4:
                lane = 1
        elif y01 < 0.6:
                lane = 2
        elif y01 < 0.8:
                lane = 3
        else:
                lane = 4

        var quarter := 3
        if x01 < 0.25:
                quarter = 0
        elif x01 < 0.50:
                quarter = 1
        elif x01 < 0.75:
                quarter = 2
        else:
                quarter = 3

        return quarter * 5 + lane


## Gini coefficient (0..1). O(n log n) implementation.
static func gini(values: Array) -> float:
        var n := values.size()
        if n <= 0:
                return 0.0
        var xs: Array = values.duplicate()
        xs.sort()
        var sum := 0.0
        for v in xs:
                sum += max(0.0, float(v))
        if sum <= 0.000001:
                return 0.0
        var cum := 0.0
        for i in range(n):
                cum += float(i + 1) * max(0.0, float(xs[i]))
        var g := (2.0 * cum) / (float(n) * sum) - (float(n + 1) / float(n))
        return clamp(g, 0.0, 1.0)
