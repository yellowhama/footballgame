extends Control
class_name IdleRewardPopup

## Phase 6.2: Idle Reward Popup UI
## 복귀 시 방치 보상 수령 팝업

signal rewards_claimed(rewards: Dictionary)
signal popup_closed

## UI References
@onready var time_label: Label = %TimeLabel
@onready var gold_label: Label = %GoldLabel
@onready var exp_label: Label = %ExpLabel
@onready var stamina_label: Label = %StaminaLabel
@onready var token_label: Label = %TokenLabel
@onready var multiplier_label: Label = %MultiplierLabel
@onready var claim_button: Button = %ClaimButton
@onready var gold_row: HBoxContainer = %GoldRow
@onready var exp_row: HBoxContainer = %ExpRow
@onready var stamina_row: HBoxContainer = %StaminaRow
@onready var token_row: HBoxContainer = %TokenRow

## State
var _rewards: Dictionary = {}
var _idle_reward_manager: Node = null


func _ready() -> void:
	_idle_reward_manager = get_node_or_null("/root/IdleRewardManager")

	if claim_button:
		claim_button.pressed.connect(_on_claim_pressed)

	hide()


func show_rewards(rewards: Dictionary) -> void:
	"""보상 팝업 표시.

	Args:
		rewards: IdleRewardManager에서 계산된 보상 Dictionary
	"""
	_rewards = rewards

	if rewards.is_empty():
		return

	# 시간 표시
	var elapsed_seconds: int = rewards.get("elapsed_seconds", 0)
	var hours: int = elapsed_seconds / 3600
	var minutes: int = (elapsed_seconds % 3600) / 60

	if hours > 0:
		time_label.text = "%d시간 %d분 동안 방치" % [hours, minutes]
	else:
		time_label.text = "%d분 동안 방치" % minutes

	# 배율 표시
	var multiplier: float = rewards.get("multiplier", 1.0)
	if multiplier > 1.0:
		multiplier_label.text = "보상 배율: x%.1f" % multiplier
		multiplier_label.visible = true
	else:
		multiplier_label.visible = false

	# 각 보상 행 표시
	_update_reward_row(gold_row, gold_label, rewards.get("gold", 0), "+%d")
	_update_reward_row(exp_row, exp_label, rewards.get("coach_exp", 0), "+%d")
	_update_reward_row(stamina_row, stamina_label, rewards.get("stamina", 0), "+%d")
	_update_reward_row(token_row, token_label, rewards.get("gacha_token", 0), "+%d")

	show()
	_play_entrance_animation()


func _update_reward_row(row: HBoxContainer, label: Label, value: int, format: String) -> void:
	"""보상 행 업데이트."""
	if value > 0:
		label.text = format % value
		row.visible = true
	else:
		row.visible = false


func _play_entrance_animation() -> void:
	"""팝업 등장 애니메이션."""
	modulate.a = 0.0
	scale = Vector2(0.9, 0.9)

	var tween: Tween = create_tween()
	tween.set_parallel(true)
	tween.tween_property(self, "modulate:a", 1.0, 0.3).set_ease(Tween.EASE_OUT)
	tween.tween_property(self, "scale", Vector2(1.0, 1.0), 0.3).set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_BACK)


func _on_claim_pressed() -> void:
	"""보상 수령 버튼."""
	if _idle_reward_manager and _idle_reward_manager.has_method("claim_rewards"):
		_idle_reward_manager.claim_rewards(_rewards)

	rewards_claimed.emit(_rewards)
	_play_exit_animation()


func _play_exit_animation() -> void:
	"""팝업 퇴장 애니메이션."""
	var tween: Tween = create_tween()
	tween.set_parallel(true)
	tween.tween_property(self, "modulate:a", 0.0, 0.2).set_ease(Tween.EASE_IN)
	tween.tween_property(self, "scale", Vector2(0.9, 0.9), 0.2).set_ease(Tween.EASE_IN)
	await tween.finished

	hide()
	popup_closed.emit()


## Public API


func is_showing() -> bool:
	"""팝업 표시 중 여부."""
	return visible
