extends Control

# PromotionPopup - 승격 축하 팝업
# MyTeamData의 promote_to_next_squad() 호출 시 EventBus를 통해 표시됨

# UI 노드 참조
@onready var previous_squad_label = $PopupPanel/ContentContainer/SquadLevelContainer/PreviousSquadLabel
@onready var new_squad_label = $PopupPanel/ContentContainer/SquadLevelContainer/NewSquadLabel
@onready var player_info_label = $PopupPanel/ContentContainer/PlayerInfoLabel
@onready var reason1_label = $PopupPanel/ContentContainer/ReasonsContainer/Reason1
@onready var reason2_label = $PopupPanel/ContentContainer/ReasonsContainer/Reason2
@onready var reason3_label = $PopupPanel/ContentContainer/ReasonsContainer/Reason3
@onready var reason4_label = $PopupPanel/ContentContainer/ReasonsContainer/Reason4


func _ready():
	# 초기에는 숨김
	hide()

	# EventBus 구독 - squad_promoted 이벤트 수신
	if has_node("/root/EventBus"):
		var event_bus = get_node("/root/EventBus")
		if event_bus.has_method("sub"):
			event_bus.sub(self, "squad_promoted", "_on_squad_promoted", false)
			print("[PromotionPopup] Subscribed to squad_promoted event")
	else:
		print("[PromotionPopup] ⚠️ EventBus not found - popup will not auto-show")


func _on_squad_promoted(topic: String, payload: Dictionary):
	"""EventBus에서 squad_promoted 이벤트 수신 시 호출됨"""
	print("[PromotionPopup] Received squad_promoted event: ", payload)
	show_promotion(payload)


func show_promotion(promotion_data: Dictionary):
	"""승격 데이터로 팝업 표시

	Args:
		promotion_data: {
			previous_level: int,  # 0=YOUTH, 1=BTEAM, 2=ATEAM
			new_level: int,
			previous_ca: int,
			current_ca: int,
			reasons: Array[String]
		}
	"""
	# 스쿼드 레벨 이름 변환
	var prev_level_name = _get_squad_level_name(promotion_data.get("previous_level", 0))
	var new_level_name = _get_squad_level_name(promotion_data.get("new_level", 1))

	# UI 업데이트
	previous_squad_label.text = prev_level_name
	new_squad_label.text = new_level_name

	var current_ca = promotion_data.get("current_ca", 0)
	player_info_label.text = "Main Player CA: %d" % current_ca

	# 승격 이유 표시
	var reasons = promotion_data.get("reasons", [])
	var reason_labels = [reason1_label, reason2_label, reason3_label, reason4_label]

	# Hard gate 이유 추가
	var hard_gate_text = "✅ Hard Gate: CA ≥ %d" % _get_hard_gate_threshold(promotion_data.get("new_level", 1))
	reason1_label.text = hard_gate_text

	# Soft gate 이유들 표시
	for i in range(min(reasons.size(), 3)):  # 최대 3개의 soft gate 이유
		if i + 1 < reason_labels.size():
			reason_labels[i + 1].text = reasons[i]

	# 남은 레이블 숨기기
	for i in range(reasons.size() + 1, reason_labels.size()):
		reason_labels[i].text = ""

	# 팝업 표시
	show()
	print("[PromotionPopup] Showing promotion: %s → %s" % [prev_level_name, new_level_name])


func _get_squad_level_name(level: int) -> String:
	"""스쿼드 레벨 표시 이름"""
	match level:
		0:
			return "U18 Youth"
		1:
			return "B-Team"
		2:
			return "A-Team"
		_:
			return "Unknown"


func _get_hard_gate_threshold(level: int) -> int:
	"""하드 게이트 임계값"""
	match level:
		1:  # BTEAM
			return 100
		2:  # ATEAM
			return 115
		_:
			return 100


func _on_continue_button_pressed():
	"""계속하기 버튼 클릭"""
	print("[PromotionPopup] Continue button pressed, hiding popup")
	hide()
