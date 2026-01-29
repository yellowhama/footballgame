## MultiSpectatorView.gd - Phase E Multi-Spectator Mode
## Manages multiple match viewports for simultaneous viewing

@onready var viewport_container: HBoxContainer = $ViewportContainer
@onready var match_selector: OptionButton = $ControlPanel/MatchSelector
@onready var add_viewport_button: Button = $ControlPanel/AddViewportButton
@onready var remove_viewport_button: Button = $ControlPanel/RemoveViewportButton
@onready var close_button: Button = $ControlPanel/CloseButton

const MAX_VIEWPORTS = 4
const MINI_VIEWPORT_SIZE = Vector2(400, 300)

var active_matches: Array = []  # Array of match IDs
var viewport_nodes: Array = []  # Array of SubViewportContainer nodes

func _ready():
    if match_selector:
        match_selector.item_selected.connect(_on_match_selected)
    if add_viewport_button:
        add_viewport_button.pressed.connect(_add_viewport)
    if remove_viewport_button:
        remove_viewport_button.pressed.connect(_remove_viewport)
    if close_button:
        close_button.pressed.connect(_on_close)

    # Initialize with one viewport
    _add_viewport()

func set_available_matches(matches: Array):
    active_matches = matches
    _update_match_selector()

func _update_match_selector():
    if not match_selector:
        return

    match_selector.clear()
    for match_info in active_matches:
        var match_id = match_info.get("id", "unknown")
        var home_team = match_info.get("home_team", "Home")
        var away_team = match_info.get("away_team", "Away")
        var label = "%s vs %s" % [home_team, away_team]
        match_selector.add_item(label)

func _on_match_selected(index: int):
    if index < 0 or index >= active_matches.size():
        return

    var selected_match = active_matches[index]
    # Update the first viewport with selected match
    if viewport_nodes.size() > 0:
        _update_viewport_match(viewport_nodes[0], selected_match)

func _add_viewport():
    if viewport_nodes.size() >= MAX_VIEWPORTS:
        return

    var sub_viewport = SubViewportContainer.new()
    sub_viewport.size = MINI_VIEWPORT_SIZE
    sub_viewport.stretch = true

    var viewport = SubViewport.new()
    viewport.size = MINI_VIEWPORT_SIZE
    viewport.render_target_update_mode = SubViewport.UPDATE_WHEN_VISIBLE

    # Add a simple match preview scene (placeholder)
    var match_preview = _create_match_preview()
    viewport.add_child(match_preview)

    sub_viewport.add_child(viewport)
    viewport_container.add_child(sub_viewport)

    viewport_nodes.append(sub_viewport)
    _update_control_buttons()

func _remove_viewport():
    if viewport_nodes.size() <= 1:
        return

    var last_viewport = viewport_nodes.pop_back()
    if last_viewport:
        viewport_container.remove_child(last_viewport)
        last_viewport.queue_free()

    _update_control_buttons()

func _create_match_preview() -> Node:
    # Placeholder: Create a simple panel showing match info
    var panel = Panel.new()
    panel.size = MINI_VIEWPORT_SIZE

    var label = Label.new()
    label.text = "Match Preview\n(Not Implemented)"
    label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    panel.add_child(label)

    return panel

func _update_viewport_match(viewport_node: SubViewportContainer, match_info: Dictionary):
    # Update the viewport with match data
    # This would load the actual match scene or update the preview
    pass

func _on_close() -> void:
    queue_free()

func _update_control_buttons():
    if add_viewport_button:
        add_viewport_button.disabled = viewport_nodes.size() >= MAX_VIEWPORTS
    if remove_viewport_button:
        remove_viewport_button.disabled = viewport_nodes.size() <= 1