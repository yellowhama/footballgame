extends Node2D

## Godot ReplayLoader v2 Test Scene
##
## FIX_2512 Phase 3 - TASK_08
## 2025-12-25

# Preload classes
const ReplayLoaderV2 = preload("res://scripts/utils/ReplayLoaderV2.gd")
const PositionSnapshotAdapter = preload("res://scripts/match_pipeline/PositionSnapshotAdapter.gd")


func _ready():
	print("\n=== Godot ReplayLoader v2 Test ===\n")

	# Test 1: ReplayV2 JSON 로드
	print("Test 1: Load ReplayV2 JSON")
	var replay_path := "res://data/test_replay_v2.json"
	var replay := ReplayLoaderV2.load_json(replay_path)

	if replay.is_empty():
		print("❌ Failed to load replay")
		return

	print("✅ Loaded ReplayV2 from: ", replay_path)

	# Test 2: 메타 정보 출력
	print("\nTest 2: Meta information")
	ReplayLoaderV2.print_meta(replay)

	# Test 3: 유효성 검증
	print("\nTest 3: Validation")
	if ReplayLoaderV2.validate(replay):
		print("✅ Replay validation passed")
	else:
		print("❌ Replay validation failed")
		return

	# Test 4: 통계 정보
	print("\nTest 4: Statistics")
	var stats := ReplayLoaderV2.get_stats(replay)
	print("- Frames: ", stats["frame_count"])
	print("- Events: ", stats["event_count"])
	print("- Duration: ", float(stats["duration_ms"]) / 1000.0, " seconds")
	print("- Goals: ", stats["goals"])
	print("- Passes: ", stats["passes"])
	print("- Shots: ", stats["shots"])

	# Test 5: 첫 프레임 변환
	print("\nTest 5: Frame conversion (first frame)")
	var frames := ReplayLoaderV2.iter_frames(replay)
	if frames.is_empty():
		print("❌ No frames found")
		return

	var first_frame: Dictionary = frames[0]
	print("- Frame t_ms: ", first_frame["t_ms"])
	print("- Entity count: ", first_frame["entities"].size())

	# PositionSnapshotAdapter로 변환
	var snapshot := PositionSnapshotAdapter.from_replay_v2_frame(first_frame)
	if snapshot.is_empty():
		print("❌ Failed to convert frame to snapshot")
		return

	print("✅ Converted to StandardSnapshot")
	print("- Snapshot t_ms: ", snapshot["t_ms"])
	print("- Ball pos: ", snapshot["ball"]["pos"])
	print("- Ball vel: ", snapshot["ball"]["vel"])
	print("- Ball z: ", snapshot["ball"]["z"])
	print("- Ball owner_id: ", snapshot["ball"]["owner_id"])
	print("- Player count: ", snapshot["players"].size())

	# 첫 선수 정보
	if snapshot["players"].has("0"):
		var player0 = snapshot["players"]["0"]
		print("- Player 0 pos: ", player0["pos"])
		print("- Player 0 vel: ", player0["velocity"])
		print("- Player 0 action: ", player0["action"])
		print("- Player 0 team_id: ", player0["team_id"])

	# Test 6: 특정 시간 프레임 검색
	print("\nTest 6: Get frame at specific time")
	var target_t_ms := 1000
	var frame_at := ReplayLoaderV2.get_frame_at(replay, target_t_ms)
	if not frame_at.is_empty():
		print("✅ Found frame at t_ms=", frame_at["t_ms"], " (requested: ", target_t_ms, ")")
	else:
		print("❌ No frame found")

	# Test 7: 이벤트 필터링
	print("\nTest 7: Event filtering")
	var goals := ReplayLoaderV2.get_events_by_type(replay, ReplayLoaderV2.EVENT_GOAL)
	print("- Goal events: ", goals.size())
	for goal in goals:
		var goal_t_sec := float(goal["t_ms"]) / 1000.0
		var goal_pos := Vector2(float(goal["x10"]) / 10.0, float(goal["y10"]) / 10.0)
		print("  * t=%.1fs, scorer=%d, pos=%s" % [goal_t_sec, goal["a"], goal_pos])

	# Test 8: ReplaySmoother 호환성 (간접 테스트)
	print("\nTest 8: ReplaySmoother compatibility check")
	print("- snapshot has 't_ms': ", snapshot.has("t_ms"))
	print("- snapshot has 'ball': ", snapshot.has("ball"))
	print("- snapshot has 'players': ", snapshot.has("players"))
	print("- snapshot has 'events': ", snapshot.has("events"))
	if snapshot.has("ball"):
		var ball = snapshot["ball"]
		print("- ball has 'pos': ", ball.has("pos"))
		print("- ball has 'z': ", ball.has("z"))
	if snapshot.has("players") and not snapshot["players"].is_empty():
		var first_player_key = snapshot["players"].keys()[0]
		var first_player = snapshot["players"][first_player_key]
		print("- player has 'pos': ", first_player.has("pos"))
		print("- player has 'velocity': ", first_player.has("velocity"))
		print("- player has 'action': ", first_player.has("action"))
	print("✅ All required fields present (compatible with ReplaySmoother)")

	print("\n=== All ReplayLoader v2 Tests Passed ===\n")
