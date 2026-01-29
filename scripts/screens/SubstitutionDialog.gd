extends Window
class_name SubstitutionDialog

signal substitution_selected(payload: Dictionary)

var _home_roster: Dictionary = {}
var _away_roster: Dictionary = {}
var _starters: Array = []
var _bench: Array = []
var _selected_team: String = "home"


func _ready() -> void:
	if not visible:
		popup_centered()

	if $"VBox/Buttons/CancelButton":
		$"VBox/Buttons/CancelButton".pressed.connect(_on_cancel_pressed)
	if $"VBox/Buttons/ConfirmButton":
		$"VBox/Buttons/ConfirmButton".pressed.connect(_on_confirm_pressed)

	# 팀 탭 버튼 연결 (2025-12-09 추가)
	var home_btn := get_node_or_null("VBox/TeamTabBar/HomeButton")
	var away_btn := get_node_or_null("VBox/TeamTabBar/AwayButton")
	if home_btn:
		home_btn.pressed.connect(_on_home_tab_pressed)
	if away_btn:
		away_btn.pressed.connect(_on_away_tab_pressed)


## 기존 API 유지 (단일 로스터 전달)
func setup(home_roster: Dictionary) -> void:
	setup_both_teams(home_roster, {})


## 확장 API: 홈/어웨이 양팀 로스터 전달 (2025-12-09 추가)
func setup_both_teams(home_roster: Dictionary, away_roster: Dictionary) -> void:
	_home_roster = home_roster
	_away_roster = away_roster
	_selected_team = "home"
	_load_roster_for_team("home")
	_update_tab_buttons()


func _populate_lists() -> void:
	var starting_list: ItemList = $"VBox/HSplit/Left/StartingList"
	var bench_list: ItemList = $"VBox/HSplit/Right/BenchList"

	starting_list.clear()
	bench_list.clear()

	for player in _starters:
		var name := str(player.get("name", ""))
		var pos := str(player.get("position", ""))
		var label := name if pos.is_empty() else "%s (%s)" % [name, pos]
		starting_list.add_item(label)

	for player in _bench:
		var name := str(player.get("name", ""))
		var pos := str(player.get("position", ""))
		var label := name if pos.is_empty() else "%s (%s)" % [name, pos]
		bench_list.add_item(label)


func _on_cancel_pressed() -> void:
	queue_free()


func _on_confirm_pressed() -> void:
        var starting_list: ItemList = $"VBox/HSplit/Left/StartingList"
        var bench_list: ItemList = $"VBox/HSplit/Right/BenchList"

        var out_indices: PackedInt32Array = starting_list.get_selected_items()  
        var in_indices: PackedInt32Array = bench_list.get_selected_items()      

        if out_indices.size() == 0 or in_indices.size() == 0:
                return

        var out_pitch_slot: int = int(out_indices[0])  # 0..10
        var in_bench_slot: int = int(in_indices[0])  # 0..6 (per-team)
        if in_bench_slot < 0 or in_bench_slot >= 7:
                push_warning("[SubstitutionDialog] Bench slot out of supported range (0..6): %d" % in_bench_slot)
                return

        var out_track_id: int = out_pitch_slot if _selected_team == "home" else (11 + out_pitch_slot)

        var out_player: Dictionary = _starters[out_pitch_slot]
        var in_player: Dictionary = _bench[in_bench_slot]

        var payload := {
                "team": _selected_team,  # 선택된 팀 사용 (2025-12-09 수정)
                "out_track_id": out_track_id,
                "in_bench_slot": in_bench_slot,
                # Optional UI-only fields (engine uses only track/bench slots)
                "out_name": out_player.get("name", ""),
                "in_name": in_player.get("name", ""),
        }

        substitution_selected.emit(payload)
        queue_free()


#region Team Tab Handlers (2025-12-09 추가)


func _on_home_tab_pressed() -> void:
	if _selected_team == "home":
		return
	_selected_team = "home"
	_load_roster_for_team("home")


func _on_away_tab_pressed() -> void:
	if _selected_team == "away":
		return
	_selected_team = "away"
	_load_roster_for_team("away")


func _load_roster_for_team(team: String) -> void:
	var roster: Dictionary = _home_roster if team == "home" else _away_roster
	_starters = roster.get("starters", [])
	_bench = roster.get("bench", [])
	_populate_lists()

	# 리스트가 비어 있으면 확인 버튼 비활성화
	var confirm_btn := get_node_or_null("VBox/Buttons/ConfirmButton")
	if confirm_btn:
		confirm_btn.disabled = _starters.is_empty() or _bench.is_empty()


func _update_tab_buttons() -> void:
	var home_btn := get_node_or_null("VBox/TeamTabBar/HomeButton") as Button
	var away_btn := get_node_or_null("VBox/TeamTabBar/AwayButton") as Button

	if home_btn:
		home_btn.button_pressed = (_selected_team == "home")
		# 홈 로스터 비어 있으면 비활성화
		home_btn.disabled = _home_roster.is_empty()

	if away_btn:
		away_btn.button_pressed = (_selected_team == "away")
		# 어웨이 로스터 비어 있으면 비활성화
		away_btn.disabled = _away_roster.is_empty()

#endregion
