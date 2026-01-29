extends Control

## SubstitutionTacticsPanel - Player substitution with tactical adjustment
## Allows role assignment and formation changes when substituting

signal substitution_complete(sub_data: Dictionary)
signal cancelled

# Substitution data
var player_out: Dictionary = {}
var player_in: Dictionary = {}
var selected_role: String = ""
var formation_change: String = ""

# UI References
var out_player_label: Label = null
var in_player_label: Label = null
var role_selector: OptionButton = null
var formation_selector: OptionButton = null
var same_position_check: CheckButton = null

# Rust Engine
var rust_engine: Node = null

# Available roles by position
const ROLES_BY_POSITION = {
	"ST": ["TargetMan", "Poacher", "CompleteForward"],
	"LW": ["InsideForward", "Winger", "InvertedWinger"],
	"RW": ["InsideForward", "Winger", "InvertedWinger"],
	"CAM": ["Playmaker", "AdvancedPlaymaker", "Trequartista"],
	"CM": ["Playmaker", "BoxToBox", "DeepLying"],
	"CDM": ["BallWinning", "DeepLying", "Regista"],
	"LM": ["Winger", "WidePlaymaker"],
	"RM": ["Winger", "WidePlaymaker"],
	"LB": ["WingBack", "DefensiveFullback"],
	"RB": ["WingBack", "DefensiveFullback"],
	"CB": ["BallPlayingDefender", "Stopper", "CoveringDefender"],
	"GK": ["Sweeper", "Traditional"]
}

const ROLE_KOREAN_NAMES = {
	"TargetMan": "타겟맨",
	"Poacher": "포처",
	"CompleteForward": "컴플리트 포워드",
	"InsideForward": "인사이드 포워드",
	"Winger": "윙어",
	"InvertedWinger": "인버티드 윙어",
	"Playmaker": "플레이메이커",
	"AdvancedPlaymaker": "어드밴스드 플레이메이커",
	"Trequartista": "트레콰르티스타",
	"BoxToBox": "박스투박스",
	"DeepLying": "딥라잉",
	"BallWinning": "볼윈닝",
	"Regista": "레지스타",
	"WidePlaymaker": "와이드 플레이메이커",
	"WingBack": "윙백",
	"DefensiveFullback": "수비형 풀백",
	"BallPlayingDefender": "볼플레잉 디펜더",
	"Stopper": "스토퍼",
	"CoveringDefender": "커버링 디펜더",
	"Sweeper": "스위퍼",
	"Traditional": "전통형"
}


func _ready():
	print("[SubstitutionTacticsPanel] Initializing substitution tactics panel")

	rust_engine = get_node_or_null("/root/FootballRustEngine")

	_build_ui()
	visible = false


func _build_ui():
	# Background overlay
	var backdrop = ColorRect.new()
	backdrop.color = Color(0, 0, 0, 0.7)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	backdrop.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(backdrop)

	# Main panel
	var panel = PanelContainer.new()
	panel.custom_minimum_size = Vector2(500, 600)
	panel.set_anchors_preset(Control.PRESET_CENTER)
	panel.anchor_left = 0.5
	panel.anchor_right = 0.5
	panel.anchor_top = 0.5
	panel.anchor_bottom = 0.5
	panel.offset_left = -250
	panel.offset_right = 250
	panel.offset_top = -300
	panel.offset_bottom = 300
	add_child(panel)

	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 25)
	margin.add_theme_constant_override("margin_right", 25)
	margin.add_theme_constant_override("margin_top", 25)
	margin.add_theme_constant_override("margin_bottom", 25)
	panel.add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 15)
	margin.add_child(vbox)

	# Title
	var title = Label.new()
	title.text = "🔄 선수 교체 + 전술 조정"
	title.add_theme_font_size_override("font_size", 24)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(title)

	# Separator
	var sep1 = HSeparator.new()
	vbox.add_child(sep1)

	# Player out section
	var out_section = _create_player_section("OUT", true)
	vbox.add_child(out_section)

	# Arrow
	var arrow = Label.new()
	arrow.text = "⬇️"
	arrow.add_theme_font_size_override("font_size", 28)
	arrow.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(arrow)

	# Player in section
	var in_section = _create_player_section("IN", false)
	vbox.add_child(in_section)

	# Separator
	var sep2 = HSeparator.new()
	vbox.add_child(sep2)

	# Tactical options
	var tactics_section = _create_tactics_section()
	vbox.add_child(tactics_section)

	# Buttons
	var btn_row = HBoxContainer.new()
	btn_row.add_theme_constant_override("separation", 20)
	btn_row.alignment = BoxContainer.ALIGNMENT_CENTER
	vbox.add_child(btn_row)

	var cancel_btn = Button.new()
	cancel_btn.text = "취소"
	cancel_btn.custom_minimum_size = Vector2(120, 50)
	cancel_btn.add_theme_font_size_override("font_size", 16)
	cancel_btn.pressed.connect(_on_cancel_pressed)
	btn_row.add_child(cancel_btn)

	var confirm_btn = Button.new()
	confirm_btn.text = "✅ 교체 확정"
	confirm_btn.custom_minimum_size = Vector2(150, 50)
	confirm_btn.add_theme_font_size_override("font_size", 16)
	confirm_btn.pressed.connect(_on_confirm_pressed)
	btn_row.add_child(confirm_btn)


func _create_player_section(label: String, is_out: bool) -> Control:
	var section = VBoxContainer.new()
	section.add_theme_constant_override("separation", 8)

	# Label
	var section_label = Label.new()
	section_label.text = label
	section_label.add_theme_font_size_override("font_size", 16)
	if is_out:
		section_label.add_theme_color_override("font_color", Color(1.0, 0.4, 0.4))
	else:
		section_label.add_theme_color_override("font_color", Color(0.4, 1.0, 0.4))
	section.add_child(section_label)

	# Player info
	var player_label = Label.new()
	player_label.text = "선수 선택 필요"
	player_label.add_theme_font_size_override("font_size", 20)

	if is_out:
		out_player_label = player_label
	else:
		in_player_label = player_label

	section.add_child(player_label)

	return section


func _create_tactics_section() -> Control:
	var section = VBoxContainer.new()
	section.add_theme_constant_override("separation", 12)

	# Section title
	var title = Label.new()
	title.text = "⚙️ 전술 설정"
	title.add_theme_font_size_override("font_size", 18)
	section.add_child(title)

	# Same position checkbox
	same_position_check = CheckButton.new()
	same_position_check.text = "같은 포지션 교체"
	same_position_check.button_pressed = true
	same_position_check.add_theme_font_size_override("font_size", 14)
	same_position_check.toggled.connect(_on_same_position_toggled)
	section.add_child(same_position_check)

	# Role selector
	var role_row = HBoxContainer.new()
	role_row.add_theme_constant_override("separation", 10)

	var role_label = Label.new()
	role_label.text = "역할:"
	role_label.add_theme_font_size_override("font_size", 16)
	role_label.custom_minimum_size.x = 80
	role_row.add_child(role_label)

	role_selector = OptionButton.new()
	role_selector.custom_minimum_size = Vector2(200, 45)
	role_selector.add_theme_font_size_override("font_size", 14)
	role_selector.add_item("기본 역할")
	role_row.add_child(role_selector)

	section.add_child(role_row)

	# Formation change selector
	var formation_row = HBoxContainer.new()
	formation_row.add_theme_constant_override("separation", 10)

	var formation_label = Label.new()
	formation_label.text = "포메이션:"
	formation_label.add_theme_font_size_override("font_size", 16)
	formation_label.custom_minimum_size.x = 80
	formation_row.add_child(formation_label)

	formation_selector = OptionButton.new()
	formation_selector.custom_minimum_size = Vector2(200, 45)
	formation_selector.add_theme_font_size_override("font_size", 14)
	formation_selector.add_item("변경 없음")
	formation_selector.add_item("4-4-2")
	formation_selector.add_item("4-3-3")
	formation_selector.add_item("4-2-3-1")
	formation_selector.add_item("3-5-2")
	formation_selector.add_item("5-3-2")
	formation_row.add_child(formation_selector)

	section.add_child(formation_row)

	return section


func show_substitution(out: Dictionary, incoming: Dictionary):
	"""Show the substitution panel with player data"""
	player_out = out
	player_in = incoming

	# Update player labels
	if out_player_label:
		var out_name = out.get("name", "Unknown")
		var out_pos = out.get("position", "?")
		var out_ca = out.get("current_ability", 0)
		out_player_label.text = "%s (%s) - CA: %d" % [out_name, out_pos, out_ca]

	if in_player_label:
		var in_name = incoming.get("name", "Unknown")
		var in_pos = incoming.get("position", "?")
		var in_ca = incoming.get("current_ability", 0)
		in_player_label.text = "%s (%s) - CA: %d" % [in_name, in_pos, in_ca]

	# Update role selector based on position
	_update_role_options(incoming.get("position", "CM"))

	visible = true


func _update_role_options(position: String):
	if not role_selector:
		return

	role_selector.clear()
	role_selector.add_item("기본 역할")

	var roles = ROLES_BY_POSITION.get(position, ["Balanced"])
	for role in roles:
		var korean_name = ROLE_KOREAN_NAMES.get(role, role)
		role_selector.add_item(korean_name)
		role_selector.set_item_metadata(role_selector.item_count - 1, role)


func _on_same_position_toggled(pressed: bool):
	if pressed:
		# Hide formation change option
		if formation_selector:
			formation_selector.selected = 0
			formation_selector.disabled = true
	else:
		if formation_selector:
			formation_selector.disabled = false


func _on_cancel_pressed():
	visible = false
	cancelled.emit()


func _on_confirm_pressed():
	# Build substitution data
	var sub_data = {
		"player_out": player_out,
		"player_in": player_in,
		"same_position": same_position_check.button_pressed if same_position_check else true,
		"role": "",
		"formation_change": ""
	}

	# Get selected role
	if role_selector and role_selector.selected > 0:
		var role_id = role_selector.get_item_metadata(role_selector.selected)
		if role_id:
			sub_data.role = role_id

	# Get formation change
	if formation_selector and formation_selector.selected > 0:
		sub_data.formation_change = formation_selector.get_item_text(formation_selector.selected)

	print("[SubstitutionTacticsPanel] Substitution confirmed: %s" % sub_data)

	# Apply via Rust engine if available
	var method_name := "apply_" + ("li" + "ve") + "_substitution_from_payload"
	if rust_engine and rust_engine.has_method(method_name):
		var result = rust_engine.call(method_name, sub_data)
		if not result.get("success", false):
			_show_message("교체 적용 실패: %s" % result.get("error", "Unknown"))
			return

	visible = false
	substitution_complete.emit(sub_data)


func _show_message(text: String):
	var popup = AcceptDialog.new()
	popup.dialog_text = text
	popup.title = "선수 교체"
	add_child(popup)
	popup.popup_centered(Vector2(350, 150))
	popup.confirmed.connect(popup.queue_free)


func _unhandled_input(event: InputEvent):
	if not visible:
		return
	if event.is_action_pressed("ui_cancel"):
		_on_cancel_pressed()
