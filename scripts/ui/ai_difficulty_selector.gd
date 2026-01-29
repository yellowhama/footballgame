extends OptionButton
class_name AIDifficultySelector

## AI 난이도 선택 컴포넌트
##
## Phase 3.2: AI Tactical Manager UI Integration
##
## 사용법:
##   var selector = AIDifficultySelector.new()
##   var difficulty = selector.get_selected_difficulty()
##   # Returns: 0=Easy, 1=Medium, 2=Hard, 3=Expert

## AI 난이도 열거형
## Rust AIDifficulty enum과 동일한 순서
enum AIDifficulty { EASY = 0, MEDIUM = 1, HARD = 2, EXPERT = 3 }  ## AI가 전술을 변경하지 않음  ## 5분마다 30% 확률로 전술 변경  ## 5분마다 80% 확률로 전술 변경  ## 항상 최적의 전술 선택 (100%)

## 선택 변경 시그널
signal difficulty_changed(difficulty: int)


func _ready():
	setup_options()
	select_default()

	# 선택 변경 시 시그널 발생
	item_selected.connect(_on_item_selected)


## 난이도 옵션 설정
func setup_options():
	clear()

	# Easy - 초보자용
	add_item("쉬움 🟢", AIDifficulty.EASY)
	set_item_metadata(
		0,
		{
			"name": "Easy",
			"name_ko": "쉬움",
			"description": "AI가 전술을 변경하지 않습니다.\n초보자에게 추천합니다.",
			"description_en": "AI will not change tactics.\nRecommended for beginners.",
			"update_chance": 0,
			"color": Color(0.3, 0.8, 0.3),  # Green
			"icon": "🟢"
		}
	)

	# Medium - 기본
	add_item("보통 🟡", AIDifficulty.MEDIUM)
	set_item_metadata(
		1,
		{
			"name": "Medium",
			"name_ko": "보통",
			"description": "AI가 기본적인 상황에 대응합니다.\n5분마다 30% 확률로 전술을 변경합니다.",
			"description_en": "AI responds to basic situations.\n30% chance to change tactics every 5 minutes.",
			"update_chance": 30,
			"color": Color(0.9, 0.9, 0.2),  # Yellow
			"icon": "🟡"
		}
	)

	# Hard - 도전적
	add_item("어려움 🟠", AIDifficulty.HARD)
	set_item_metadata(
		2,
		{
			"name": "Hard",
			"name_ko": "어려움",
			"description": "AI가 정교하게 판단합니다.\n5분마다 80% 확률로 전술을 변경합니다.",
			"description_en": "AI makes sophisticated decisions.\n80% chance to change tactics every 5 minutes.",
			"update_chance": 80,
			"color": Color(1.0, 0.6, 0.0),  # Orange
			"icon": "🟠"
		}
	)

	# Expert - 전문가용
	add_item("전문가 🔴", AIDifficulty.EXPERT)
	set_item_metadata(
		3,
		{
			"name": "Expert",
			"name_ko": "전문가",
			"description": "AI가 완벽하게 운영합니다.\n항상 최적의 전술을 선택합니다.",
			"description_en": "AI plays perfectly.\nAlways selects optimal tactics.",
			"update_chance": 100,
			"color": Color(1.0, 0.2, 0.2),  # Red
			"icon": "🔴"
		}
	)


## 기본값 선택 (Medium)
func select_default():
	selected = AIDifficulty.MEDIUM


## 선택된 난이도 반환 (0-3)
func get_selected_difficulty() -> int:
	return selected


## 선택된 난이도의 메타데이터 반환
func get_selected_metadata() -> Dictionary:
	if selected >= 0 and selected < item_count:
		return get_item_metadata(selected)
	return {}


## 툴팁 표시용 설명 텍스트 반환
func get_description() -> String:
	var meta = get_selected_metadata()
	if meta.has("description"):
		return meta.description
	return ""


## 난이도 색상 반환 (UI 표시용)
func get_difficulty_color() -> Color:
	var meta = get_selected_metadata()
	if meta.has("color"):
		return meta.color
	return Color.WHITE


## 난이도 아이콘 반환
func get_difficulty_icon() -> String:
	var meta = get_selected_metadata()
	if meta.has("icon"):
		return meta.icon
	return ""


## 난이도 이름 반환 (한국어)
func get_difficulty_name_ko() -> String:
	var meta = get_selected_metadata()
	if meta.has("name_ko"):
		return meta.name_ko
	return ""


## 난이도 이름 반환 (영어)
func get_difficulty_name_en() -> String:
	var meta = get_selected_metadata()
	if meta.has("name"):
		return meta.name
	return ""


## 업데이트 확률 반환 (0-100)
func get_update_chance() -> int:
	var meta = get_selected_metadata()
	if meta.has("update_chance"):
		return meta.update_chance
	return 0


## 프로그래밍 방식으로 난이도 설정
func set_difficulty(difficulty: AIDifficulty):
	if difficulty >= AIDifficulty.EASY and difficulty <= AIDifficulty.EXPERT:
		selected = difficulty
		difficulty_changed.emit(difficulty)


## 선택 변경 이벤트 핸들러
func _on_item_selected(index: int):
	difficulty_changed.emit(index)


## 난이도 설명 포맷팅 (RichTextLabel용)
func get_formatted_description() -> String:
	var meta = get_selected_metadata()
	if meta.is_empty():
		return ""

	var formatted = "[b]%s %s[/b]\n\n" % [meta.icon, meta.name_ko]
	formatted += "%s\n\n" % meta.description
	formatted += "[color=gray]업데이트 확률: %d%%[/color]" % meta.update_chance

	return formatted
