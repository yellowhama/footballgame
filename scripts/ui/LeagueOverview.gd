## LeagueOverview.gd - Phase E League Overview
## Displays current league matches with scores and status

@onready var matches_container: VBoxContainer = $ScrollContainer/MatchesContainer
@onready var refresh_button: Button = $ControlPanel/RefreshButton

var league_matches: Array = []

func _ready():
    if refresh_button:
        refresh_button.pressed.connect(_refresh_matches)

    _load_league_matches()

func _load_league_matches():
    # Placeholder: Load current week's matches
    # In real implementation, this would query the league state
    league_matches = [
        {
            "id": "match_1",
            "home_team": "Team A",
            "away_team": "Team B",
            "home_score": 2,
            "away_score": 1,
            "status": "completed",
            "minute": 90
        },
        {
            "id": "match_2",
            "home_team": "Team C",
            "away_team": "Team D",
            "home_score": 0,
            "away_score": 0,
            "status": "in_progress",
            "minute": 67
        }
    ]
    _display_matches()

func _refresh_matches():
    _load_league_matches()

func _display_matches():
    # Clear existing
    for child in matches_container.get_children():
        child.queue_free()

    for match_info in league_matches:
        var match_panel = _create_match_panel(match_info)
        matches_container.add_child(match_panel)

func _create_match_panel(match_info: Dictionary) -> Panel:
    var panel = Panel.new()
    panel.size = Vector2(600, 80)

    var hbox = HBoxContainer.new()
    hbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    panel.add_child(hbox)

    # Home team
    var home_label = Label.new()
    home_label.text = match_info.get("home_team", "Home")
    home_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    hbox.add_child(home_label)

    # Score
    var score_label = Label.new()
    if match_info.get("status") == "completed":
        score_label.text = "%d - %d" % [match_info.get("home_score", 0), match_info.get("away_score", 0)]
    else:
        score_label.text = "vs"
    hbox.add_child(score_label)

    # Away team
    var away_label = Label.new()
    away_label.text = match_info.get("away_team", "Away")
    away_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    hbox.add_child(away_label)

    # Status
    var status_label = Label.new()
    var status = match_info.get("status", "scheduled")
    var minute = match_info.get("minute", 0)
    if status == "in_progress":
        status_label.text = "%d'" % minute
    elif status == "completed":
        status_label.text = "FT"
    else:
        status_label.text = "Scheduled"
    hbox.add_child(status_label)

    return panel