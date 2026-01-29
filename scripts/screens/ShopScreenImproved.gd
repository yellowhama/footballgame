extends Control
# ShopScreenImproved - 코치 카드 가챠 상점 화면

# UI 요소들
@onready var back_button: Button = $VBox/Header/BackButton
@onready var banner_container: Control = $VBox/BannerSection/BannerContainer
@onready var single_draw_button: Button = $VBox/DrawSection/HBox/SingleDrawButton
@onready var ten_draw_button: Button = $VBox/DrawSection/HBox/TenDrawButton
@onready var free_gems_label: Label = $VBox/CurrencySection/HBox/FreeGems/Amount
@onready var paid_diamonds_label: Label = $VBox/CurrencySection/HBox/PaidDiamonds/Amount
@onready var pity_counter_label: Label = $VBox/InfoSection/PityCounter
@onready var probability_button: Button = $VBox/InfoSection/ProbabilityButton

# 가챠 관련 변수
var free_gems: int = 3000  # 시작 무료 재화
var paid_diamonds: int = 0  # 유료 재화
var current_banner: String = "standard"  # standard, pickup, guaranteed

# 가챠 비용
const SINGLE_DRAW_COST = 300
const TEN_DRAW_COST = 2700  # 10% 할인

# QuickBar support
var quickbar: QuickBar


func _ready():
	print("[ShopScreenImproved] Initializing gacha shop...")

	# ColorSystem 적용
	# SceneColorUpdater.apply_color_system_to_scene(self)
	print("ColorSystem skipped - static call needed")

	# 반응형 레이아웃 수정
	# ResponsiveLayoutFixer.fix_scene_layout(self)
	print("ResponsiveLayout skipped - static call needed")

	# 터치 피드백 적용
	# TouchFeedback.apply_to_all_buttons(self)
	print("TouchFeedback skipped - static call needed")

	# 버튼 연결
	_connect_buttons()

	# QuickBar 초기화
	_initialize_quickbar()

	# UI 업데이트
	_update_ui()

	# 초기 배너 설정
	_setup_current_banner()


func _connect_buttons():
	"""버튼 신호 연결"""
	if back_button:
		back_button.pressed.connect(_on_back_pressed)

	if single_draw_button:
		single_draw_button.pressed.connect(_on_single_draw_pressed)

	if ten_draw_button:
		ten_draw_button.pressed.connect(_on_ten_draw_pressed)

	if probability_button:
		probability_button.pressed.connect(_on_probability_pressed)


func _initialize_quickbar():
	"""QuickBar 초기화 및 신호 연결"""
	if has_node("QuickBar"):
		quickbar = %QuickBar
		if quickbar:
			print("[ShopScreenImproved] QuickBar found, configuring for shop...")
			# Shop 전용 설정
			var quickbar_vm = {"autoEnabled": false, "currentSpeed": 1, "highlightLevel": 2, "visible": true}
			quickbar.apply_view_model(quickbar_vm)

			# Shop에서는 Skip = 뒤로가기
			quickbar.skip.connect(_on_back_pressed)


func _update_ui():
	"""UI 업데이트"""
	# 재화 표시
	if free_gems_label:
		free_gems_label.text = str(free_gems)

	if paid_diamonds_label:
		paid_diamonds_label.text = str(paid_diamonds)

	# 버튼 활성화 상태
	if single_draw_button:
		single_draw_button.disabled = free_gems < SINGLE_DRAW_COST
		single_draw_button.text = "단차 뽑기\n%d 젬" % SINGLE_DRAW_COST

	if ten_draw_button:
		ten_draw_button.disabled = free_gems < TEN_DRAW_COST
		ten_draw_button.text = "10연차\n%d 젬" % TEN_DRAW_COST

	# 천장 카운터 표시
	_update_pity_counter()


func _get_football_simulator():
	"""FootballMatchSimulator 인스턴스 가져오기"""
	# MatchSimulationManager를 통해 접근
	if get_node_or_null("/root/MatchSimulationManager"):
		var manager = get_node("/root/MatchSimulationManager")
		if manager.match_engine:
			return manager.match_engine
		else:
			# match_engine이 없으면 새로 생성
			if ClassDB.class_exists("FootballSimulator"):
				manager.match_engine = ClassDB.instantiate("FootballSimulator")
				return manager.match_engine
	return null


func _update_pity_counter(counter: int = -1):
	"""천장 카운터 업데이트"""
	var simulator = _get_football_simulator()
	if not simulator:
		if pity_counter_label:
			pity_counter_label.text = "천장: -/100"
		return

	if counter >= 0:
		# 직접 전달된 값 사용
		if pity_counter_label:
			pity_counter_label.text = "천장: %d/100" % counter
	else:
		# OpenFootball에서 가챠 통계 가져오기
		var stats_json = simulator.get_gacha_statistics()
		if stats_json:
			var stats = JSON.parse_string(stats_json)
			if stats and stats.has("pity_counter"):
				var pity = stats.pity_counter
				if pity_counter_label:
					pity_counter_label.text = "천장: %d/100" % pity

					# 천장 임박 시 색상 변경
					if pity >= 90:
						pity_counter_label.modulate = Color(1, 0.8, 0, 1)  # 노랑
					elif pity >= 75:
						pity_counter_label.modulate = Color(1, 0.9, 0.5, 1)  # 연노랑
					else:
						pity_counter_label.modulate = Color.WHITE


func _setup_current_banner():
	"""현재 배너 설정"""
	print("[ShopScreenImproved] Setting up banner: %s" % current_banner)

	# 배너 이미지 로드 (있다면)
	if banner_container:
		# 배너 컨테이너에 이미지나 정보 표시
		var banner_label = Label.new()
		match current_banner:
			"standard":
				banner_label.text = "스탠다드 가챠\n모든 카드 등장!"
			"pickup":
				banner_label.text = "픽업 가챠\n5★ The GOAT Maker 확률 UP!"
			"guaranteed":
				banner_label.text = "확정 가챠\n4★ 이상 1장 확정!"

		banner_label.add_theme_font_size_override("font_size", 20)
		banner_label.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
		banner_container.add_child(banner_label)


func _on_single_draw_pressed():
	"""단차 뽑기"""
	print("[ShopScreenImproved] Single draw pressed")

	# 재화 확인
	if free_gems < SINGLE_DRAW_COST:
		_show_notification("젬이 부족합니다!", Color(1, 0, 0, 1))
		return

	# 재화 차감
	free_gems -= SINGLE_DRAW_COST
	_update_ui()

	# SSOT: use Rust-backed GachaManager
	if not GachaManager:
		_show_dummy_card()
		return

	var response: Dictionary = GachaManager.draw_single(current_banner)
	if response.get("success", false):
		_show_card_result([response.get("card", {})])
		_update_pity_counter(int(response.get("pity_counter", -1)))
		return

	print("[ShopScreenImproved] Single draw failed: ", response.get("error", "Unknown"))
	_show_dummy_card()


func _on_ten_draw_pressed():
	"""10연차 뽑기"""
	print("[ShopScreenImproved] 10x draw pressed")

	# 재화 확인
	if free_gems < TEN_DRAW_COST:
		_show_notification("젬이 부족합니다!", Color(1, 0, 0, 1))
		return

	# 재화 차감
	free_gems -= TEN_DRAW_COST
	_update_ui()

	# SSOT: use Rust-backed GachaManager
	if not GachaManager:
		var dummy_cards = []
		for i in range(10):
			dummy_cards.append(_create_dummy_card())
		_show_card_result(dummy_cards)
		return

	var response: Dictionary = GachaManager.draw_10x(current_banner)
	if response.get("success", false):
		_show_card_result(response.get("cards", []))
		_update_pity_counter(int(response.get("pity_counter", -1)))
		return

	print("[ShopScreenImproved] 10x draw failed: ", response.get("error", "Unknown"))
	var fallback_cards = []
	for i in range(10):
		fallback_cards.append(_create_dummy_card())
	_show_card_result(fallback_cards)


func _show_card_result(cards: Array):
	"""카드 결과 애니메이션 표시"""
	print("[ShopScreenImproved] Showing %d cards with animation" % cards.size())

	# CardReveal 애니메이션 시스템 사용
	if cards.is_empty():
		_show_notification("카드를 획득하지 못했습니다.", Color(1, 0.5, 0, 1))
		return

	# CardReveal 씬 로드 및 생성
	var card_reveal_scene = preload("res://scenes/ui/CardReveal.tscn")
	var card_reveal = card_reveal_scene.instantiate()

	# 현재 씬에 추가 (최상위 레이어)
	add_child(card_reveal)
	card_reveal.z_index = 100

	# 카드 데이터 변환 (OpenFootball 형식 → CardReveal 형식)
	var reveal_cards = []
	for card in cards:
		var reveal_card = {
			"name": card.get("name", "Unknown Card"),
			"rarity": card.get("rarity", 1),
			"card_type": card.get("card_type", "Coach"),
			"specialty": card.get("specialty", "Balanced"),
			"id": card.get("id", ""),
			"description": card.get("description", "")
		}
		reveal_cards.append(reveal_card)

	# 레어도순 정렬 (낮은 것부터 높은 것까지 연출)
	reveal_cards.sort_custom(func(a, b): return a.rarity < b.rarity)

	# 애니메이션 시작
	card_reveal.reveal_cards(reveal_cards)

	# 애니메이션 완료 대기 (안전하게 타임아웃 추가)
	var CARD_REVEAL_TIMEOUT_SECONDS: float = 60.0
	var _reveal_completed := false
	var _on_reveal_complete := func() -> void: _reveal_completed = true
	card_reveal.cards_revealed_complete.connect(_on_reveal_complete, CONNECT_ONE_SHOT)

	var elapsed := 0.0
	while not _reveal_completed and elapsed < CARD_REVEAL_TIMEOUT_SECONDS:
		await get_tree().process_frame
		elapsed += get_process_delta_time()

	# 타임아웃 처리
	if not _reveal_completed:
		push_error(
			"[ShopScreenImproved] Card reveal animation timed out after %.f seconds." % CARD_REVEAL_TIMEOUT_SECONDS
		)

	# 정리 (타임아웃 여부와 관계없이 실행)
	if is_instance_valid(card_reveal):
		card_reveal.queue_free()

	# 천장 카운터 업데이트
	_update_pity_counter()

	# 획득 완료 알림
	var high_rarity_count = 0
	for card in cards:
		if card.get("rarity", 1) >= 4:
			high_rarity_count += 1

	if high_rarity_count > 0:
		_show_notification("⭐4 이상 %d장 획득!" % high_rarity_count, Color(1, 0.8, 0, 1))
	else:
		_show_notification("%d장 획득 완료!" % cards.size(), Color(0, 1, 0, 1))


func _show_dummy_card():
	"""테스트용 더미 카드 생성"""
	var card = _create_dummy_card()
	_show_card_result([card])


func _create_dummy_card() -> Dictionary:
	"""더미 카드 데이터 생성"""
	var rarities = [1, 1, 1, 2, 2, 3]  # 확률 시뮬레이션
	var rarity = rarities[randi() % rarities.size()]

	var names = {
		1: ["Speed Coach", "Power Coach", "Tech Coach"], 2: ["Elite Trainer", "Skill Master"], 3: ["Legend Coach"]
	}

	return {
		"id": "dummy_%d" % randi(),
		"name": names[rarity][randi() % names[rarity].size()],
		"rarity": rarity,
		"card_type": "Coach",
		"specialty": ["Speed", "Power", "Technical"][randi() % 3],
		"level": 1
	}


func _on_probability_pressed():
	"""확률 정보 표시"""
	print("[ShopScreenImproved] Showing probability info")

	var prob_text = """
	[center][b]가챠 확률 정보[/b][/center]

	⭐ 1성: 60%
	⭐⭐ 2성: 25%
	⭐⭐⭐ 3성: 10%
	⭐⭐⭐⭐ 4성: 4%
	⭐⭐⭐⭐⭐ 5성: 1%

	[color=yellow]천장 시스템:[/color]
	• 100회: 4★ 이상 확정
	• 200회: 5★ 확정

	[color=cyan]10연차 보너스:[/color]
	• 3★ 이상 1장 확정
	"""

	_show_popup(prob_text)


func _on_back_pressed():
	"""뒤로가기"""
	print("[ShopScreenImproved] Going back to main menu")
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _show_notification(text: String, color: Color = Color.WHITE):
	"""알림 표시"""
	var notif = Label.new()
	notif.text = text
	notif.add_theme_font_size_override("font_size", 20)
	notif.position = Vector2(get_viewport().size.x / 2 - 100, 100)
	notif.modulate = color
	add_child(notif)

	# 페이드 아웃
	var tween = get_tree().create_tween()
	tween.tween_interval(1.0)
	tween.tween_property(notif, "modulate:a", 0.0, 1.0)
	tween.tween_callback(notif.queue_free)


func _show_popup(text: String):
	"""팝업 표시"""
	var popup = AcceptDialog.new()
	popup.dialog_text = text
	popup.title = "알림"
	popup.add_theme_font_size_override("font_size", 16)
	add_child(popup)
	popup.popup_centered(Vector2(400, 500))
	popup.confirmed.connect(popup.queue_free)


func _input(event):
	"""입력 처리 (디버그용)"""
	if event.is_action_pressed("ui_cancel"):
		_on_back_pressed()

	# 디버그: F1 = 젬 추가
	if event is InputEventKey and event.pressed:
		if event.keycode == KEY_F1:
			free_gems += 3000
			_update_ui()
			_show_notification("+3000 젬 (디버그)", Color(0, 1, 0, 1))
