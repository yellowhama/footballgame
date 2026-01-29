extends Control
# EndingScreen - 5가지 엔딩 시각화 및 표시

@onready var background = $Background
@onready var ending_title = $VBox/EndingTitle
@onready var ending_icon = $VBox/EndingIcon
@onready var ending_description = $VBox/EndingDescription
@onready var player_summary = $VBox/PlayerSummary
@onready var achievements_list = $VBox/AchievementsList
@onready var final_stats = $VBox/FinalStats
@onready var continue_button = $VBox/ContinueButton
@onready var restart_button = $VBox/RestartButton

var ending_type: int = -1
var ending_data: Dictionary = {}
var player_final_stats: Dictionary = {}
var final_achievements: Array = []

# 엔딩별 배경 색상과 이미지
var ending_themes = {
	0: {"color": Color(1.0, 0.8, 0.0, 0.9), "gradient": Color(1.0, 0.6, 0.0, 0.9), "title_color": Color(0.1, 0.1, 0.1)},  # 프로 슈퍼스타 - 골드
	1: {"color": Color(0.2, 0.5, 1.0, 0.9), "gradient": Color(0.1, 0.3, 0.8, 0.9), "title_color": Color.WHITE},  # 해외 유학 - 블루
	2: {"color": Color(0.3, 0.7, 0.3, 0.9), "gradient": Color(0.2, 0.5, 0.2, 0.9), "title_color": Color.WHITE},  # 대학 에이스 - 그린
	3: {"color": Color(0.6, 0.4, 0.2, 0.9), "gradient": Color(0.4, 0.3, 0.1, 0.9), "title_color": Color.WHITE},  # 지도자의 길 - 브라운
	4: {"color": Color(0.8, 0.2, 0.8, 0.9), "gradient": Color(0.6, 0.1, 0.6, 0.9), "title_color": Color.WHITE}  # 히든 레전드 - 퍼플
}


func _ready():
	print("[EndingScreen] Initializing ending screen...")

	# 엔딩 데이터 로드
	_load_ending_data()

	# UI 설정
	_setup_ending_display()
	_connect_signals()

	# 엔딩 애니메이션 시작
	_start_ending_animation()

	print("[EndingScreen] Ending screen ready!")


func _load_ending_data():
	"""엔딩 데이터 로드"""
	# GameData에서 엔딩 정보 가져오기
	if has_node("/root/GameData"):
		var game_data = get_node("/root/GameData")
		ending_type = game_data.get("ending_type") if "ending_type" in game_data else 0
		ending_data = game_data.get("ending_data") if "ending_data" in game_data else {}
		player_final_stats = game_data.get("player_final_stats") if "player_final_stats" in game_data else {}
		final_achievements = game_data.get("final_achievements") if "final_achievements" in game_data else []
	else:
		# Fallback 데이터 (테스트용)
		ending_type = 0  # 프로 슈퍼스타
		ending_data = {"name": "프로 슈퍼스타", "description": "최고의 프로 선수가 되어 월드컵에서 활약한다", "icon": "⭐", "rarity": "S"}
		player_final_stats = {"name": "Test Player", "ca": 150, "pa": 150, "total_matches": 60, "total_goals": 45}
		final_achievements = ["슈퍼스타", "득점왕", "MVP"]


func _setup_ending_display():
	"""엔딩 화면 설정"""
	# 배경 설정
	_setup_background()

	# 엔딩 타이틀 설정
	if ending_title:
		ending_title.text = ending_data.get("name", "Unknown Ending")
		ending_title.add_theme_font_size_override("font_size", 48)
		var theme = ending_themes.get(ending_type, ending_themes[0])
		ending_title.add_theme_color_override("font_color", theme.title_color)

	# 엔딩 아이콘 설정
	if ending_icon:
		ending_icon.text = ending_data.get("icon", "🏆")
		ending_icon.add_theme_font_size_override("font_size", 120)

	# 엔딩 설명 설정
	if ending_description:
		ending_description.text = ending_data.get("description", "")
		ending_description.add_theme_font_size_override("font_size", 20)
		ending_description.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	# 플레이어 요약 설정
	_setup_player_summary()

	# 최종 스탯 설정
	_setup_final_stats()

	# 업적 목록 설정
	_setup_achievements_list()


func _setup_background():
	"""배경 설정"""
	if not background:
		return

	var theme = ending_themes.get(ending_type, ending_themes[0])

	# 그라데이션 배경 생성
	var gradient = Gradient.new()
	gradient.add_point(0.0, theme.color)
	gradient.add_point(1.0, theme.gradient)

	var gradient_texture = GradientTexture2D.new()
	gradient_texture.gradient = gradient
	gradient_texture.fill_from = Vector2(0, 0)
	gradient_texture.fill_to = Vector2(1, 1)

	# 배경에 적용
	if background is TextureRect:
		background.texture = gradient_texture


func _setup_player_summary():
	"""플레이어 요약 설정"""
	if not player_summary:
		return

	var summary_text = (
		"""🏃 최종 플레이어 정보
이름: %s
최종 능력치: %d / %d
총 경기 수: %d경기
총 득점: %d골
성취도: %s급"""
		% [
			player_final_stats.get("name", "Unknown"),
			player_final_stats.get("ca", 0),
			player_final_stats.get("pa", 0),
			player_final_stats.get("total_matches", 0),
			player_final_stats.get("total_goals", 0),
			ending_data.get("rarity", "C")
		]
	)

	player_summary.text = summary_text
	player_summary.add_theme_font_size_override("font_size", 18)


func _setup_final_stats():
	"""최종 스탯 설정"""
	if not final_stats:
		return

	# 최종 스탯 표시 (6각형 능력치)
	var stats_text = (
		"""📊 최종 능력치 분석
⚡ 기술: %d   🏃 속도: %d   💪 근력: %d
🛡️ 수비: %d   🧠 정신: %d   ❤️ 체력: %d"""
		% [
			player_final_stats.get("technical_average", 0),
			player_final_stats.get("pace_average", 0),
			player_final_stats.get("power_average", 0),
			player_final_stats.get("defending_average", 0),
			player_final_stats.get("mental_average", 0),
			player_final_stats.get("physical_average", 0)
		]
	)

	final_stats.text = stats_text
	final_stats.add_theme_font_size_override("font_size", 16)


func _setup_achievements_list():
	"""업적 목록 설정"""
	if not achievements_list:
		return

	if final_achievements.is_empty():
		achievements_list.text = "🏆 획득한 업적이 없습니다"
	else:
		var achievements_text = "🏆 획득 업적 (%d개)\n" % final_achievements.size()
		for i in range(min(final_achievements.size(), 10)):  # 최대 10개까지 표시
			achievements_text += "• %s\n" % final_achievements[i]

		if final_achievements.size() > 10:
			achievements_text += "• ... 그 외 %d개" % (final_achievements.size() - 10)

		achievements_list.text = achievements_text

	achievements_list.add_theme_font_size_override("font_size", 14)


func _start_ending_animation():
	"""엔딩 애니메이션 시작"""
	# 초기 투명도 설정
	modulate = Color(1, 1, 1, 0)

	# 페이드인 애니메이션
	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_CUBIC)

	# 배경 페이드인
	tween.tween_property(self, "modulate", Color.WHITE, 2.0)

	# 엔딩 아이콘 회전 애니메이션
	if ending_icon:
		var icon_tween = create_tween()
		icon_tween.set_loops(1)
		icon_tween.tween_property(ending_icon, "rotation", TAU, 10.0)

	# 텍스트 타이핑 효과 (간단 버전)
	await tween.finished
	_typewriter_effect()


func _typewriter_effect():
	"""타이핑 효과 (간단 버전)"""
	if ending_description:
		var full_text = ending_description.text
		ending_description.text = ""

		for i in range(full_text.length()):
			ending_description.text += full_text[i]
			await get_tree().create_timer(0.05).timeout

	# 버튼 활성화
	if continue_button:
		continue_button.disabled = false
	if restart_button:
		restart_button.disabled = false


func _connect_signals():
	"""시그널 연결"""
	if continue_button:
		continue_button.pressed.connect(_on_continue_pressed)
		continue_button.disabled = true  # 애니메이션 끝날 때까지 비활성화

	if restart_button:
		restart_button.pressed.connect(_on_restart_pressed)
		restart_button.disabled = true


func _on_continue_pressed():
	"""계속하기 버튼 처리"""
	print("[EndingScreen] Continue button pressed")

	# 저장된 게임으로 돌아가거나 크레딧 화면으로
	get_tree().change_scene_to_file("res://scenes/CreditsScreen.tscn")


func _on_restart_pressed():
	"""다시 시작 버튼 처리"""
	print("[EndingScreen] Restart button pressed")

	# 새 게임 시작
	get_tree().change_scene_to_file("res://scenes/TitleScreen.tscn")


# 특별 엔딩별 추가 효과
func _get_special_ending_effects():
	"""특별 엔딩 효과"""
	match ending_type:
		0:  # 프로 슈퍼스타
			_create_star_particles()
		1:  # 해외 유학
			_create_travel_effects()
		4:  # 히든 레전드
			_create_legendary_effects()


func _create_star_particles():
	"""별 파티클 효과 (프로 슈퍼스타용)"""
	# 간단한 별 효과
	for i in range(50):
		await get_tree().create_timer(0.1).timeout
		_create_floating_star()


func _create_floating_star():
	"""떠다니는 별 생성"""
	var star = Label.new()
	star.text = "⭐"
	star.add_theme_font_size_override("font_size", randf_range(20, 40))
	star.position = Vector2(randf_range(0, get_viewport().size.x), get_viewport().size.y)

	add_child(star)

	# 위로 떠오르는 애니메이션
	var tween = create_tween()
	tween.set_parallel(true)
	tween.tween_property(star, "position:y", -100, randf_range(3, 5))
	tween.tween_property(star, "modulate", Color(1, 1, 1, 0), 3.0)

	await tween.finished
	star.queue_free()


func _create_travel_effects():
	"""여행 효과 (해외 유학용)"""
	# 비행기나 지구본 효과 등
	pass


func _create_legendary_effects():
	"""전설 효과 (히든 레전드용)"""
	# 무지개나 왕관 효과 등
	pass
