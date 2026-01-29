extends Control
# PersonalityDemo - PersonAttributes 시스템을 테스트하는 UI 프로토타입

@onready var test_button: Button = $MainContent/VBox/TestSection/VBox/TestButton
@onready var gen_button: Button = $MainContent/VBox/PlayerGenSection/VBox/GenButton
@onready var result_text: RichTextLabel = $MainContent/VBox/ResultsSection/VBox/ResultText
@onready var back_button: Button = $BottomBar/HBox/BackButton

# 성격 원형 버튼들
@onready var leader_btn: Button = $MainContent/VBox/ArchetypeSection/VBox/ArchetypeButtons/LeaderBtn
@onready var genius_btn: Button = $MainContent/VBox/ArchetypeSection/VBox/ArchetypeButtons/GeniusBtn
@onready var workhorse_btn: Button = $MainContent/VBox/ArchetypeSection/VBox/ArchetypeButtons/WorkhorseBtn
@onready var rebel_btn: Button = $MainContent/VBox/ArchetypeSection/VBox/ArchetypeButtons/RebelBtn
@onready var steady_btn: Button = $MainContent/VBox/ArchetypeSection/VBox/ArchetypeButtons/SteadyBtn

var _rust_simulator: RefCounted = null


func _ready() -> void:
	_initialize_rust_connection()
	_update_result_display("PersonAttributes 시스템 테스트에 오신 것을 환영합니다!\n\n위의 버튼들을 클릭하여 시스템을 테스트해보세요.")


## Rust GDExtension 연결 초기화
func _initialize_rust_connection() -> void:
	if ClassDB.class_exists("FootballMatchSimulator"):
		_rust_simulator = ClassDB.instantiate("FootballMatchSimulator")
		if _rust_simulator:
			print("[PersonalityDemo] Rust connection established")
		else:
			print("[PersonalityDemo] Failed to instantiate FootballMatchSimulator")
	else:
		print("[PersonalityDemo] FootballMatchSimulator class not found")


## 전체 시스템 테스트
func _on_test_button_pressed() -> void:
	if not _rust_simulator:
		_update_result_display("[ERROR] Rust 엔진에 연결할 수 없습니다.")
		return

	_update_result_display("🧠 PersonAttributes 시스템 테스트 중...\n")

	var test_result = _rust_simulator.test_personality_system()

	if test_result and test_result != "":
		var json_parser = JSON.new()
		var parse_result = json_parser.parse(test_result)

		if parse_result == OK:
			var data = json_parser.data
			_format_test_results(data)
		else:
			_update_result_display("[ERROR] JSON 파싱 실패: " + json_parser.get_error_message())
	else:
		_update_result_display("[ERROR] Rust 함수 호출 실패")


## 랜덤 선수 생성
func _on_gen_button_pressed() -> void:
	if not _rust_simulator:
		_update_result_display("[ERROR] Rust 엔진에 연결할 수 없습니다.")
		return

	_update_result_display("👤 새로운 선수 생성 중...\n")

	var rng_seed = Time.get_ticks_msec()  # 현재 시간을 시드로 사용
	var player_result = _rust_simulator.generate_random_player(rng_seed)

	if player_result and player_result != "":
		var json_parser = JSON.new()
		var parse_result = json_parser.parse(player_result)

		if parse_result == OK:
			var data = json_parser.data
			_format_player_results(data)
		else:
			_update_result_display("[ERROR] JSON 파싱 실패: " + json_parser.get_error_message())
	else:
		_update_result_display("[ERROR] 선수 생성 실패")


## 특정 성격 원형 테스트
func _on_archetype_button_pressed(archetype_name: String) -> void:
	if not _rust_simulator:
		_update_result_display("[ERROR] Rust 엔진에 연결할 수 없습니다.")
		return

	_update_result_display("🎭 " + archetype_name + " 타입 성격 생성 중...\n")

	var rng_seed = Time.get_ticks_msec()
	var archetype_result = _rust_simulator.get_personality_archetype(archetype_name, rng_seed)

	if archetype_result and archetype_result != "":
		var json_parser = JSON.new()
		var parse_result = json_parser.parse(archetype_result)

		if parse_result == OK:
			var data = json_parser.data
			_format_archetype_results(data, archetype_name)
		else:
			_update_result_display("[ERROR] JSON 파싱 실패: " + json_parser.get_error_message())
	else:
		_update_result_display("[ERROR] " + archetype_name + " 타입 생성 실패")


## 테스트 결과를 보기 좋게 포맷
func _format_test_results(data: Dictionary) -> void:
	var output = "[color=yellow]✅ PersonAttributes 시스템 테스트 완료![/color]\n\n"

	if data.has("archetypes_tested"):
		output += "[color=cyan]🎭 테스트된 성격 원형들:[/color]\n"
		for archetype in data.archetypes_tested:
			output += "  • " + str(archetype) + "\n"
		output += "\n"

	if data.has("sample_personalities"):
		output += "[color=lime]📊 샘플 성격 특성들:[/color]\n"
		for personality in data.sample_personalities:
			output += "  [b]" + str(personality.get("archetype", "Unknown")) + ":[/b]\n"
			if personality.has("attributes"):
				var attrs = personality.attributes
				output += "    적응력: " + str(attrs.get("adaptability", 0)) + "/20\n"
				output += "    야망: " + str(attrs.get("ambition", 0)) + "/20\n"
				output += "    결단력: " + str(attrs.get("determination", 0)) + "/20\n"
				output += "    규율: " + str(attrs.get("discipline", 0)) + "/20\n"
				output += "    충성도: " + str(attrs.get("loyalty", 0)) + "/20\n"
				output += "    압박처리: " + str(attrs.get("pressure", 0)) + "/20\n"
				output += "    프로정신: " + str(attrs.get("professionalism", 0)) + "/20\n"
				output += "    성격: " + str(attrs.get("temperament", 0)) + "/20\n"
			output += "\n"

	if data.has("message"):
		output += "[color=white]💬 시스템 메시지:[/color]\n" + str(data.message) + "\n"

	_update_result_display(output)


## 선수 생성 결과를 보기 좋게 포맷
func _format_player_results(data: Dictionary) -> void:
	var output = "[color=yellow]✅ 새 선수 생성 완료![/color]\n\n"

	if data.has("name"):
		output += "[b]이름:[/b] " + str(data.name) + "\n"

	if data.has("position"):
		output += "[b]포지션:[/b] " + str(data.position) + "\n"

	if data.has("age"):
		output += "[b]나이:[/b] " + str(data.age) + "세\n"

	if data.has("current_ability"):
		output += "[b]현재능력:[/b] " + str(data.current_ability) + "/200\n"

	if data.has("potential_ability"):
		output += "[b]잠재능력:[/b] " + str(data.potential_ability) + "/200\n\n"

	if data.has("personality"):
		var personality = data.personality
		output += "[color=cyan]🧠 성격 특성:[/color]\n"
		if personality.has("archetype"):
			output += "[b]성격 원형:[/b] " + str(personality.archetype) + "\n\n"

		if personality.has("attributes"):
			var attrs = personality.attributes
			output += "[color=lime]📊 성격 수치들:[/color]\n"
			output += "  적응력: " + str(attrs.get("adaptability", 0)) + "/20\n"
			output += "  야망: " + str(attrs.get("ambition", 0)) + "/20\n"
			output += "  결단력: " + str(attrs.get("determination", 0)) + "/20\n"
			output += "  규율: " + str(attrs.get("discipline", 0)) + "/20\n"
			output += "  충성도: " + str(attrs.get("loyalty", 0)) + "/20\n"
			output += "  압박처리: " + str(attrs.get("pressure", 0)) + "/20\n"
			output += "  프로정신: " + str(attrs.get("professionalism", 0)) + "/20\n"
			output += "  성격: " + str(attrs.get("temperament", 0)) + "/20\n"

		if personality.has("effects"):
			var effects = personality.effects
			output += "\n[color=orange]⚡ 게임 효과들:[/color]\n"
			output += "  훈련 효율: " + str(effects.get("training_efficiency", 1.0)) + "x\n"
			output += "  부상 저항력: " + str(effects.get("injury_resistance", 1.0)) + "x\n"
			output += "  압박 대응력: " + str(effects.get("pressure_handling", 1.0)) + "x\n"

	_update_result_display(output)


## 성격 원형 결과를 보기 좋게 포맷
func _format_archetype_results(data: Dictionary, archetype_name: String) -> void:
	var output = "[color=yellow]✅ " + archetype_name + " 타입 성격 생성 완료![/color]\n\n"

	if data.has("archetype"):
		output += "[b]성격 원형:[/b] " + str(data.archetype) + "\n\n"

	if data.has("attributes"):
		var attrs = data.attributes
		output += "[color=lime]📊 성격 특성들:[/color]\n"
		output += "  적응력: " + str(attrs.get("adaptability", 0)) + "/20\n"
		output += "  야망: " + str(attrs.get("ambition", 0)) + "/20\n"
		output += "  결단력: " + str(attrs.get("determination", 0)) + "/20\n"
		output += "  규율: " + str(attrs.get("discipline", 0)) + "/20\n"
		output += "  충성도: " + str(attrs.get("loyalty", 0)) + "/20\n"
		output += "  압박처리: " + str(attrs.get("pressure", 0)) + "/20\n"
		output += "  프로정신: " + str(attrs.get("professionalism", 0)) + "/20\n"
		output += "  성격: " + str(attrs.get("temperament", 0)) + "/20\n"

	if data.has("effects"):
		var effects = data.effects
		output += "\n[color=orange]⚡ 게임 효과들:[/color]\n"
		output += "  훈련 효율: " + str(effects.get("training_efficiency", 1.0)) + "x\n"
		output += "  부상 저항력: " + str(effects.get("injury_resistance", 1.0)) + "x\n"
		output += "  압박 대응력: " + str(effects.get("pressure_handling", 1.0)) + "x\n"

	if data.has("description"):
		output += "\n[color=white]💭 설명:[/color]\n" + str(data.description) + "\n"

	_update_result_display(output)


## 결과 표시 영역 업데이트
func _update_result_display(text: String) -> void:
	result_text.text = text
	# 스크롤을 맨 위로 이동
	var scroll_container = result_text.get_parent().get_parent()
	if scroll_container is ScrollContainer:
		scroll_container.scroll_vertical = 0


## 돌아가기 버튼 처리
func _on_back_button_pressed() -> void:
	get_tree().change_scene_to_file("res://scenes/TitleScreenImproved.tscn")
