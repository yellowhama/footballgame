extends Control
class_name ActivityVisualFeedback

## ActivityVisualFeedback.gd
## 훈련/휴식/외출 등 활동 시 비주얼 피드백 화면
## 플레이스홀더 이미지 + 대사 표시
## Created: 2025-10-25

# ============================================
# Signals
# ============================================

signal feedback_completed

# ============================================
# UI References
# ============================================

@onready var background_overlay: ColorRect = $BackgroundOverlay
@onready var visual_panel: PanelContainer = $VisualPanel
@onready var illustration_container: PanelContainer = $VisualPanel/VBox/IllustrationContainer
@onready var illustration_placeholder: ColorRect = $VisualPanel/VBox/IllustrationContainer/IllustrationPlaceholder
@onready var activity_title: Label = $VisualPanel/VBox/ActivityTitle
@onready var dialogue_text: RichTextLabel = $VisualPanel/VBox/DialogueText
@onready var continue_button: Button = $VisualPanel/VBox/ContinueButton

# ============================================
# Activity Configuration
# ============================================

# 활동별 색상 매핑
var activity_colors = {
	"technical": Color(0.3, 0.6, 1.0),  # 파랑 - 기술
	"shooting": Color(1.0, 0.4, 0.2),  # 빨강 - 슈팅
	"passing": Color(0.4, 0.8, 0.5),  # 초록 - 패스
	"pace": Color(1.0, 0.8, 0.2),  # 노랑 - 스피드
	"power": Color(0.9, 0.3, 0.3),  # 진홍 - 근력
	"physical": Color(0.5, 0.9, 0.6),  # 연두 - 체력
	"mental": Color(0.7, 0.4, 0.9),  # 보라 - 정신력
	"defending": Color(0.4, 0.5, 0.8),  # 남색 - 수비
	"balanced": Color(0.8, 0.8, 0.8),  # 회색 - 종합
	"rest": Color(0.5, 0.7, 1.0),  # 하늘색 - 휴식
	"go_out": Color(1.0, 0.7, 0.8),  # 분홍 - 외출
}

# 활동별 기본 대사
var activity_dialogues = {
	"technical": ["공을 다루는 감각이 점점 좋아지고 있어.", "세밀한 컨트롤 연습이 실력 향상의 핵심이야.", "오늘 훈련으로 볼 터치가 한층 부드러워졌어."],
	"shooting": ["골대를 향한 정확한 슈팅, 계속 연습하자.", "슈팅 파워와 정확도가 모두 중요해.", "오늘은 특히 슈팅 감각이 좋은 것 같아!"],
	"passing": ["정확한 패스가 팀 플레이의 시작이지.", "시야를 넓게 가지고 동료를 찾는 연습을 했어.", "킬 패스 타이밍을 잡는 게 핵심이야."],
	"pace": ["속도는 곧 무기다. 더 빠르게!", "순간 가속력을 키우는 훈련이었어.", "민첩성과 스피드가 함께 향상되고 있어."],
	"power": ["근력 강화로 몸싸움에서 밀리지 않을 거야.", "점프력과 헤딩 능력이 좋아지고 있어.", "강한 피지컬이 경기에서 큰 도움이 될 거야."],
	"physical": ["지구력은 90분 내내 뛰기 위한 기본이야.", "체력 훈련은 힘들지만 꼭 필요해.", "활동량이 늘어나면 경기 후반에도 지치지 않아."],
	"mental": ["침착함을 유지하는 것이 중요해.", "정신력 훈련으로 집중력이 높아졌어.", "압박 상황에서도 올바른 판단을 내리자."],
	"defending": ["태클 타이밍과 포지셔닝을 배웠어.", "수비는 공격의 시작이야.", "마킹 연습으로 상대를 효과적으로 막을 수 있어."],
	"balanced": ["균형잡힌 훈련으로 전체적인 실력을 키웠어.", "다방면의 능력을 골고루 발전시키는 중이야.", "오늘은 종합적인 성장에 집중했어."],
	"rest": ["충분한 휴식으로 몸과 마음을 재충전했어.", "피로가 풀리니 컨디션이 좋아지는 걸 느껴.", "때로는 쉬는 것도 훈련만큼 중요해."],
	"go_out": ["가벼운 외출로 기분 전환을 했어.", "친구들과 즐거운 시간을 보냈어.", "재미있게 놀면서 스트레스가 풀렸어!"]
}

# ============================================
# Initialization
# ============================================


func _ready() -> void:
	hide()

	if continue_button:
		continue_button.pressed.connect(_on_continue_pressed)


# ============================================
# Main Functions
# ============================================


func show_activity_feedback(activity_type: String, title: String, custom_dialogue: String = "") -> void:
	"""활동 피드백 표시

	Args:
		activity_type: 활동 타입 (technical, rest, etc.)
		title: 활동 제목
		custom_dialogue: 커스텀 대사 (비어있으면 기본 대사 사용)
	"""
	# Setup UI
	if activity_title:
		activity_title.text = title

	# Setup illustration placeholder
	_setup_illustration_placeholder(activity_type)

	# Setup dialogue
	var dialogue = custom_dialogue
	if dialogue.is_empty():
		dialogue = _get_random_dialogue(activity_type)

	if dialogue_text:
		dialogue_text.text = dialogue

	# Show with animation
	show()
	_animate_show()


func _setup_illustration_placeholder(activity_type: String) -> void:
	"""일러스트 플레이스홀더 설정"""
	# Try to load actual illustration
	var illustration_path = "res://assets/illustrations/%s.png" % activity_type

	if ResourceLoader.exists(illustration_path):
		# TODO: Load actual illustration when available
		pass

	# Use color placeholder
	if illustration_placeholder:
		var color = activity_colors.get(activity_type, Color(0.5, 0.5, 0.5))
		illustration_placeholder.color = color


func _get_random_dialogue(activity_type: String) -> String:
	"""활동 타입별 랜덤 대사 가져오기"""
	var dialogues = activity_dialogues.get(activity_type, ["훈련을 완료했습니다."])
	return dialogues[randi() % dialogues.size()]


func _animate_show() -> void:
	"""등장 애니메이션"""
	# Start invisible and scaled down
	modulate.a = 0.0
	if visual_panel:
		visual_panel.scale = Vector2(0.9, 0.9)

	# Animate
	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_CUBIC)
	tween.set_parallel(true)
	tween.tween_property(self, "modulate:a", 1.0, 0.3)

	if visual_panel:
		tween.tween_property(visual_panel, "scale", Vector2(1.0, 1.0), 0.3)


func _on_continue_pressed() -> void:
	"""계속 버튼 클릭"""
	await _animate_hide()
	feedback_completed.emit()


func _animate_hide() -> void:
	"""종료 애니메이션"""
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN)
	tween.set_trans(Tween.TRANS_CUBIC)
	tween.tween_property(self, "modulate:a", 0.0, 0.2)

	await tween.finished
	hide()

	# Reset for next use
	modulate.a = 1.0
	if visual_panel:
		visual_panel.scale = Vector2(1.0, 1.0)
