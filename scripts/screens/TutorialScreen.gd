extends Control
## TutorialScreen - 3-Minute Onboarding Flow
## P0 Implementation: Tutorial → Train → Match → Results → Complete

signal tutorial_completed

# UI References
@onready var title_label: Label = $ContentContainer/TitleLabel
@onready var message_label: Label = $ContentContainer/MessageLabel
@onready var description_label: Label = $ContentContainer/DescriptionLabel
@onready var next_button: Button = $ContentContainer/NextButton
@onready var skip_button: Button = $SkipButton
@onready var step_indicator: Label = $StepIndicator

# Tutorial State
enum TutorialStep { WELCOME, TRAINING_INTRO, TRAINING_RESULT, MATCH_INTRO, MATCH_RESULT, COMPLETE }

var current_step: TutorialStep = TutorialStep.WELCOME
var training_result: Dictionary = {}
var match_result: Dictionary = {}


func _ready():
	print("[TutorialScreen] Starting 3-minute tutorial flow")
	Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)

	# Connect buttons
	if next_button:
		next_button.disabled = false
		next_button.focus_mode = Control.FOCUS_ALL
		next_button.mouse_filter = Control.MOUSE_FILTER_STOP
		next_button.pressed.connect(_on_next_pressed)
		next_button.grab_focus()
	if skip_button:
		skip_button.disabled = false
		skip_button.focus_mode = Control.FOCUS_ALL
		skip_button.mouse_filter = Control.MOUSE_FILTER_STOP
		skip_button.pressed.connect(_on_skip_pressed)

	# Show welcome step
	_show_step(TutorialStep.WELCOME)


func _show_step(step: TutorialStep):
	"""Display content for current tutorial step"""
	current_step = step

	match step:
		TutorialStep.WELCOME:
			_show_welcome()

		TutorialStep.TRAINING_INTRO:
			_show_training_intro()

		TutorialStep.TRAINING_RESULT:
			_show_training_result()

		TutorialStep.MATCH_INTRO:
			_show_match_intro()

		TutorialStep.MATCH_RESULT:
			_show_match_result()

		TutorialStep.COMPLETE:
			_show_complete()


func _show_welcome():
	"""Step 1: Welcome message"""
	title_label.text = "⚽ 축구 아카데미에 오신 것을 환영합니다"
	message_label.text = "3년 후, 프로가 될 수 있을까?"
	description_label.text = """훈련하고, 경기하고, 성장하세요.
156주간의 여정이 지금 시작됩니다."""

	next_button.text = "시작하기 ▶"
	step_indicator.text = "Step 1 / 4"

	print("[TutorialScreen] Welcome step displayed")


func _show_training_intro():
	"""Step 2: Training introduction"""
	title_label.text = "💪 훈련으로 실력을 키우세요"
	message_label.text = "매주 훈련 종류를 선택할 수 있습니다"
	description_label.text = """기술 훈련: 슈팅, 패스, 드리블 향상
체력 훈련: 스피드, 스태미나 향상
멘탈 훈련: 집중력, 침착성 향상
전술 훈련: 포지셔닝, 판단력 향상

첫 훈련을 시작해볼까요?"""

	next_button.text = "기술 훈련 시작 ▶"
	step_indicator.text = "Step 2 / 4"

	print("[TutorialScreen] Training intro displayed")


func _show_training_result():
	"""Step 3: Show training results"""
	title_label.text = "✅ 훈련 완료!"
	message_label.text = "능력치가 향상되었습니다"

	var ca_gain = training_result.get("ca_gain", 2)
	var stats_text = "기술 능력 +%d\n피로도 +10" % ca_gain

	description_label.text = (
		"""훌륭해요! 첫 훈련을 마쳤습니다.

%s

매주 훈련을 통해 꾸준히 성장할 수 있습니다.
이제 첫 경기를 준비해볼까요?"""
		% stats_text
	)

	next_button.text = "경기 준비 ▶"
	step_indicator.text = "Step 3 / 4"

	print("[TutorialScreen] Training result displayed (CA +%d)" % ca_gain)


func _show_match_intro():
	"""Step 4: Match introduction"""
	title_label.text = "⚽ 경기에 출전하세요"
	message_label.text = "실전에서 실력을 발휘할 차례입니다"
	description_label.text = """매주 1-2번의 경기가 있습니다.
좋은 활약을 펼칠수록 능력치가 더 빨리 성장합니다.

평점 6.0 이상: 보통
평점 7.0 이상: 좋음
평점 8.0 이상: 훌륭함

첫 경기를 시작해볼까요?"""

	next_button.text = "경기 시작 ▶"
	step_indicator.text = "Step 4 / 4"

	print("[TutorialScreen] Match intro displayed")


func _show_match_result():
	"""Step 5: Show match results"""
	title_label.text = "🎉 첫 경기 완료!"

	var score = match_result.get("final_score", [1, 0])
	var rating = match_result.get("player_rating", 7.0)
	var result = match_result.get("result", "승리")

	var result_icon = "🏆" if result == "승리" else ("🤝" if result == "무승부" else "❌")

	message_label.text = "%s %s (평점 %.1f)" % [result_icon, result, rating]

	description_label.text = (
		"""경기 결과: %d - %d
개인 평점: %.1f / 10.0

훌륭한 출발이에요!
이제 본격적인 3년 여정을 시작할 준비가 되었습니다.

매주 훈련하고 경기하며 프로 선수를 목표로 성장하세요!"""
		% [score[0], score[1], rating]
	)

	next_button.text = "여정 시작하기 ▶"
	step_indicator.text = "튜토리얼 완료"

	print("[TutorialScreen] Match result displayed (%s, %.1f rating)" % [result, rating])


func _show_complete():
	"""Step 6: Tutorial complete"""
	title_label.text = "🎓 튜토리얼 완료!"
	message_label.text = "이제 본격적인 여정이 시작됩니다"
	description_label.text = """156주 동안 훈련하고, 경기하고, 성장하세요.
3년 후 당신은 어떤 선수가 되어 있을까요?

행운을 빕니다! ⚽"""

	next_button.text = "시작하기 ▶"
	step_indicator.text = ""

	print("[TutorialScreen] Tutorial complete screen")


func _on_next_pressed():
	"""Handle next button press"""
	print("[TutorialScreen] Next pressed (current step: %d)" % current_step)

	match current_step:
		TutorialStep.WELCOME:
			_show_step(TutorialStep.TRAINING_INTRO)

		TutorialStep.TRAINING_INTRO:
			_execute_tutorial_training()

		TutorialStep.TRAINING_RESULT:
			_show_step(TutorialStep.MATCH_INTRO)

		TutorialStep.MATCH_INTRO:
			_execute_tutorial_match()

		TutorialStep.MATCH_RESULT:
			_show_step(TutorialStep.COMPLETE)

		TutorialStep.COMPLETE:
			_finish_tutorial()


func _execute_tutorial_training():
	"""Execute tutorial training (auto-select Technical)"""
	print("[TutorialScreen] Executing tutorial training (Technical)")

	# Call TrainingManager to execute training
	if not has_node("/root/TrainingManager"):
		push_error("[TutorialScreen] TrainingManager not found!")
		training_result = {"ca_gain": 2, "fatigue_cost": 10}
		_show_step(TutorialStep.TRAINING_RESULT)
		return

	var training_manager: Node = get_node("/root/TrainingManager")

	# Use base technical program for onboarding (shooting focus)
	var training_id := "shooting"
	var result: Dictionary = training_manager.execute_training(training_id, false)
	var success: bool = result.get("success", false)

	if success:
		var changes_dict: Dictionary = {}
		var changes_variant: Variant = result.get("changes", {})
		if changes_variant is Dictionary:
			changes_dict = changes_variant
		var ca_gain: int = 0
		for change_value_local in changes_dict.values():
			var change_value: int = int(change_value_local)
			ca_gain += change_value
		training_result = {
			"ca_gain": ca_gain,
			"fatigue_cost": float(result.get("condition_cost", 0.0)),
			"changes": changes_dict,
			"message": String(result.get("message", ""))
		}
		print("[TutorialScreen] Training executed: %s | ΔCA ≈ %d" % [training_id, ca_gain])
	else:
		var failure_reason: String = String(result.get("message", "훈련을 실행할 수 없습니다"))
		push_warning("[TutorialScreen] Tutorial training failed: %s" % failure_reason)
		training_result = {"ca_gain": 0, "fatigue_cost": 0, "changes": {}, "message": failure_reason}

	_show_step(TutorialStep.TRAINING_RESULT)


func _execute_tutorial_match():
	"""Execute tutorial match (auto-simulate)"""
	print("[TutorialScreen] Executing tutorial match")

	# Tutorial uses a deterministic mock match result for onboarding.
	match_result = {
		"success": true,
		"result": "승리",
		"final_score": [2, 1],
		"player_rating": 7.5,
		"highlights":
		[{"minute": 23, "event": "goal", "player": "주인공"}, {"minute": 54, "event": "assist", "player": "주인공"}]
	}

	print(
		(
			"[TutorialScreen] Mock match simulated: %s (%.1f rating)"
			% [match_result.get("result", "승리"), match_result.get("player_rating", 7.5)]
		)
	)

	_show_step(TutorialStep.MATCH_RESULT)


func _finish_tutorial():
	"""Complete tutorial and save flag"""
	print("[TutorialScreen] Tutorial finished - saving completion flag")

	# Save tutorial_completed flag
	if has_node("/root/SaveManager"):
		var save_manager = get_node("/root/SaveManager")
		if save_manager.has_method("set_tutorial_completed"):
			save_manager.set_tutorial_completed(true)
			print("[TutorialScreen] Tutorial flag saved via SaveManager")
		else:
			# Fallback: Set global flag
			if has_node("/root/GameManager"):
				var game_manager = get_node("/root/GameManager")
				if "tutorial_completed" in game_manager:
					game_manager.tutorial_completed = true
					print("[TutorialScreen] Tutorial flag saved via GameManager")

	# Emit completion signal
	tutorial_completed.emit()

	# Transition to main game
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _on_skip_pressed():
	"""Handle skip button press"""
	print("[TutorialScreen] Tutorial skipped by user")

	# Confirm skip
	var confirm_popup = AcceptDialog.new()
	confirm_popup.dialog_text = "튜토리얼을 건너뛰시겠습니까?\n(언제든지 다시 볼 수 없습니다)"
	confirm_popup.title = "건너뛰기 확인"
	confirm_popup.confirmed.connect(_finish_tutorial)
	call_deferred("add_child", confirm_popup)
	confirm_popup.popup_centered.call_deferred()
