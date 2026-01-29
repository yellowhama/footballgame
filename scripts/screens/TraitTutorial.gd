extends Control
class_name TraitTutorial

## Trait System Tutorial (2025-12-03)
## - Step-by-step guide for the Unified Trait System
## - Shows on first visit to TraitScreen
## - Can be triggered manually via help button

signal tutorial_completed
signal tutorial_skipped

# Tutorial steps
enum TutorialStep { INTRODUCTION, TIERS, CATEGORIES, SLOTS, EQUIPPING, MERGING, ACQUISITION, COMPLETE }  # What are traits?  # Bronze/Silver/Gold explanation  # 6 categories (Shooting, Passing, etc.)  # 4 equipment slots & unlock levels  # How to equip traits  # 3-to-1 merge system  # How to get traits (training, matches, shop)

var current_step: TutorialStep = TutorialStep.INTRODUCTION
var step_count: int = 8  # Total steps

# Tutorial content
const TUTORIAL_CONTENT = {
	TutorialStep.INTRODUCTION:
	{
		"title": "🎯 특성 시스템이란?",
		"message": "선수의 강점을 더욱 특화시키는 특별한 능력입니다",
		"description":
		"""특성(Trait)은 선수에게 추가 능력치 보너스와
특수 효과를 부여합니다.

예시:
- 스나이퍼: 슈팅 정확도 상승
- 탱크: 드리블 시 공 유지력 상승
- 리더: 패스 성공률 상승

각 특성은 훈련과 경기를 통해 획득할 수 있습니다.""",
		"button": "다음 ▶"
	},
	TutorialStep.TIERS:
	{
		"title": "🏅 특성 등급",
		"message": "Bronze → Silver → Gold로 등급이 올라갑니다",
		"description":
		"""🥉 Bronze (브론즈)
  - 기본 능력치 보너스
  - 획득 확률 70%

🥈 Silver (실버)
  - 더 높은 능력치 보너스
  - 획득 확률 25%

🥇 Gold (골드)
  - 최고 능력치 보너스
  - 특수 효과 발동!
  - 획득 확률 5%

Gold 등급에서는 강력한 스페셜 효과가 활성화됩니다!""",
		"button": "다음 ▶"
	},
	TutorialStep.CATEGORIES:
	{
		"title": "📚 특성 카테고리",
		"message": "6가지 카테고리에서 다양한 특성을 선택하세요",
		"description":
		"""⚽ 슈팅 (6개): Sniper, Cannon, LobMaster, Acrobat, Poacher, Finisher

📨 패스 (6개): Architect, Playmaker, Crosser, DirectPasser, ThroughBall, SetPiece

🏃 드리블 (5개): Speedster, Technician, Tank, Magnet, Flair

🛡️ 수비 (6개): Vacuum, Wall, Reader, Bully, Shadow, AirDuels

🧤 골키퍼 (5개): Spider, Sweeper, Commander, Reflexes, Distribution

💪 피지컬 (2개): Engine, Robust

총 30개의 특성이 당신을 기다립니다!""",
		"button": "다음 ▶"
	},
	TutorialStep.SLOTS:
	{
		"title": "🎒 장착 슬롯",
		"message": "최대 4개의 특성을 장착할 수 있습니다",
		"description":
		"""특성 슬롯은 레벨에 따라 해금됩니다:

📦 슬롯 1: Lv.1 (시작부터 사용 가능)
📦 슬롯 2: Lv.10 해금
📦 슬롯 3: Lv.20 해금
📦 슬롯 4: Lv.30 해금

레벨업을 통해 더 많은 특성을 장착하세요!
전략적인 조합이 승리의 열쇠입니다.""",
		"button": "다음 ▶"
	},
	TutorialStep.EQUIPPING:
	{
		"title": "⚙️ 특성 장착",
		"message": "원하는 슬롯을 탭하고 인벤토리에서 선택하세요",
		"description":
		"""특성 장착 방법:

1️⃣ 빈 슬롯을 탭합니다
2️⃣ "인벤토리에서 선택"이 활성화됩니다
3️⃣ 원하는 특성을 탭합니다
4️⃣ 장착 완료!

💡 이미 장착된 특성을 탭하면 해제됩니다
💡 같은 특성을 중복 장착할 수 없습니다""",
		"button": "다음 ▶"
	},
	TutorialStep.MERGING:
	{
		"title": "🔨 특성 합성",
		"message": "같은 특성 3개를 합쳐 등급을 올리세요",
		"description":
		"""합성 시스템:

🥉🥉🥉 → 🥈 (Bronze 3개 → Silver 1개)
🥈🥈🥈 → 🥇 (Silver 3개 → Gold 1개)

💡 합성 버튼은 합성 가능한 특성이 있을 때만 활성화됩니다
💡 합성된 특성은 같은 종류입니다 (Sniper + Sniper + Sniper → Sniper)

중복된 특성을 모아 더 강한 특성을 만드세요!""",
		"button": "다음 ▶"
	},
	TutorialStep.ACQUISITION:
	{
		"title": "🎁 특성 획득",
		"message": "훈련, 경기, 상점에서 특성을 얻을 수 있습니다",
		"description":
		"""특성 획득 방법:

💪 훈련 완료: 8% 확률로 드롭
⚽ 경기 승리: 12% 확률로 드롭
⚽ 경기 무승부: 6% 확률로 드롭
⚽ 경기 패배: 3% 확률로 드롭
🎖️ MOTM 선정: 15% 확률로 드롭

🛒 상점: 코인으로 특성 팩 구매
  - 기본 팩, 프리미엄 팩, 엘리트 팩
  - 카테고리별 특화 팩

좋은 활약을 펼칠수록 특성 획득 확률이 높아집니다!""",
		"button": "다음 ▶"
	},
	TutorialStep.COMPLETE:
	{
		"title": "🎓 튜토리얼 완료!",
		"message": "이제 특성 시스템을 마스터할 준비가 되었습니다",
		"description":
		"""특성 시스템 요약:

✅ 30개의 다양한 특성
✅ 3단계 등급 (Bronze → Silver → Gold)
✅ 4개의 장착 슬롯 (레벨업으로 해금)
✅ 3-to-1 합성으로 등급 업그레이드
✅ 훈련, 경기, 상점에서 획득

전략적으로 특성을 선택하고 조합하여
최강의 선수를 육성하세요!

행운을 빕니다! ⚽""",
		"button": "시작하기 ▶"
	}
}


func _ready():
	_create_ui()
	_show_step(TutorialStep.INTRODUCTION)


func _create_ui():
	# Background overlay
	var bg = ColorRect.new()
	bg.name = "Background"
	bg.color = Color(0, 0, 0, 0.85)
	bg.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(bg)

	# Main container
	var container = PanelContainer.new()
	container.name = "Container"
	container.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	container.custom_minimum_size = Vector2(500, 500)
	add_child(container)

	var vbox = VBoxContainer.new()
	vbox.name = "VBox"
	vbox.add_theme_constant_override("separation", 15)
	container.add_child(vbox)

	# Margin
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 20)
	margin.add_theme_constant_override("margin_top", 20)
	margin.add_theme_constant_override("margin_right", 20)
	margin.add_theme_constant_override("margin_bottom", 20)
	container.add_child(margin)

	var inner_vbox = VBoxContainer.new()
	inner_vbox.name = "InnerVBox"
	inner_vbox.add_theme_constant_override("separation", 15)
	margin.add_child(inner_vbox)

	# Step indicator
	var step_label = Label.new()
	step_label.name = "StepIndicator"
	step_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	step_label.add_theme_font_size_override("font_size", 12)
	step_label.modulate = Color(0.7, 0.7, 0.7)
	inner_vbox.add_child(step_label)

	# Title
	var title = Label.new()
	title.name = "Title"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 24)
	inner_vbox.add_child(title)

	# Message
	var message = Label.new()
	message.name = "Message"
	message.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	message.add_theme_font_size_override("font_size", 16)
	message.modulate = Color(0.9, 0.9, 0.6)
	inner_vbox.add_child(message)

	# Separator
	var sep = HSeparator.new()
	inner_vbox.add_child(sep)

	# Description (ScrollContainer for long text)
	var scroll = ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(460, 250)
	inner_vbox.add_child(scroll)

	var description = Label.new()
	description.name = "Description"
	description.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	description.add_theme_font_size_override("font_size", 14)
	scroll.add_child(description)

	# Spacer
	var spacer = Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	inner_vbox.add_child(spacer)

	# Buttons
	var btn_container = HBoxContainer.new()
	btn_container.alignment = BoxContainer.ALIGNMENT_CENTER
	btn_container.add_theme_constant_override("separation", 20)
	inner_vbox.add_child(btn_container)

	var skip_btn = Button.new()
	skip_btn.name = "SkipButton"
	skip_btn.text = "건너뛰기"
	skip_btn.custom_minimum_size = Vector2(100, 40)
	skip_btn.pressed.connect(_on_skip_pressed)
	btn_container.add_child(skip_btn)

	var next_btn = Button.new()
	next_btn.name = "NextButton"
	next_btn.text = "다음 ▶"
	next_btn.custom_minimum_size = Vector2(150, 40)
	next_btn.pressed.connect(_on_next_pressed)
	btn_container.add_child(next_btn)


func _show_step(step: TutorialStep):
	current_step = step
	var content = TUTORIAL_CONTENT[step]

	# Update UI
	var inner_vbox = (
		get_node("Container/MarginContainer/InnerVBox")
		if has_node("Container/MarginContainer/InnerVBox")
		else _find_node("InnerVBox")
	)

	if inner_vbox:
		var step_indicator = _find_child_by_name(inner_vbox, "StepIndicator")
		var title = _find_child_by_name(inner_vbox, "Title")
		var message = _find_child_by_name(inner_vbox, "Message")
		var description = _find_child_by_name(inner_vbox, "Description")
		var next_btn = _find_child_by_name(inner_vbox, "NextButton")

		if step_indicator:
			step_indicator.text = "Step %d / %d" % [step + 1, step_count]
		if title:
			title.text = content.title
		if message:
			message.text = content.message
		if description:
			description.text = content.description
		if next_btn:
			next_btn.text = content.button

	# Hide skip button on last step
	var skip_btn = _find_node("SkipButton")
	if skip_btn:
		skip_btn.visible = step != TutorialStep.COMPLETE


func _find_node(node_name: String) -> Node:
	return get_node_or_null(NodePath(node_name)) if has_node(node_name) else _find_child_recursive(self, node_name)


func _find_child_recursive(parent: Node, node_name: String) -> Node:
	for child in parent.get_children():
		if child.name == node_name:
			return child
		var found = _find_child_recursive(child, node_name)
		if found:
			return found
	return null


func _find_child_by_name(parent: Node, node_name: String) -> Node:
	for child in parent.get_children():
		if child.name == node_name:
			return child
		# Check nested
		var found = _find_child_by_name(child, node_name)
		if found:
			return found
	return null


func _on_next_pressed():
	var next_step = current_step + 1

	if next_step > TutorialStep.COMPLETE:
		# Tutorial complete
		_mark_completed()
		tutorial_completed.emit()
		queue_free()
	else:
		_show_step(next_step)


func _on_skip_pressed():
	_mark_completed()
	tutorial_skipped.emit()
	queue_free()


func _mark_completed():
	# Save that trait tutorial has been seen
	if has_node("/root/SaveManager"):
		var save_manager = get_node("/root/SaveManager")
		if save_manager.has_method("set_flag"):
			save_manager.set_flag("trait_tutorial_completed", true)


# ============================================================================
# Static factory method
# ============================================================================


static func should_show() -> bool:
	"""Check if tutorial should be shown (first time user)"""
	if Engine.has_singleton("SaveManager"):
		var save_manager = Engine.get_singleton("SaveManager")
		if save_manager.has_method("get_flag"):
			return not save_manager.get_flag("trait_tutorial_completed", false)

	# Fallback: Check using autoload
	var root = Engine.get_main_loop().root if Engine.get_main_loop() else null
	if root and root.has_node("SaveManager"):
		var save_manager = root.get_node("SaveManager")
		if save_manager.has_method("get_flag"):
			return not save_manager.get_flag("trait_tutorial_completed", false)

	return true  # Default to show


static func create_and_show(parent: Node) -> TraitTutorial:
	"""Factory method to create and show tutorial"""
	var tutorial = TraitTutorial.new()
	parent.add_child(tutorial)
	return tutorial
