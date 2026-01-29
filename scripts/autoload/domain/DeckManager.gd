extends Node
# Preload to avoid class_name loading order issues
const _CharacterCard = preload("res://scripts/resources/CharacterCard.gd")

## DeckManager.gd
## 파워프로 스타일 캐릭터 덱 시스템
## 작성일: 2025-10-24
## 버전: 1.0

# ============================================
# 시그널
# ============================================

## 덱에 카드 추가 시 발생
signal card_added(card: _CharacterCard)

## 덱에서 카드 제거 시 발생
signal card_removed(card: _CharacterCard)

## 덱 구성 변경 시 발생
signal deck_changed(deck: Array[_CharacterCard])

# ============================================
# 상태 변수
# ============================================

## 현재 덱 (최대 5-6장)
var current_deck: Array[_CharacterCard] = []

## 덱 최대 크기
const MAX_DECK_SIZE: int = 6

## 사용 가능한 모든 카드
var available_cards: Array[_CharacterCard] = []

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	print("[DeckManager] Initialized")
	_load_initial_cards()


func _load_initial_cards() -> void:
	"""초기 카드 로드"""
	# TODO: Week 2에서 실제 카드 데이터 로드
	# 현재는 테스트용 기본 카드 생성

	# 강태양 (라이벌) - SR
	var rival_card = _CharacterCard.new()
	rival_card.character_id = "rival_taeyoung"
	rival_card.character_name = "강태양"
	rival_card.rarity = "SR"
	rival_card.character_type = "rival"
	rival_card.position = "FW"
	rival_card.technical_bonus = 0.15
	rival_card.physical_bonus = 0.10
	rival_card.mental_bonus = 0.05
	available_cards.append(rival_card)

	# 박민준 (친구) - R
	var friend_card = _CharacterCard.new()
	friend_card.character_id = "friend_minjun"
	friend_card.character_name = "박민준"
	friend_card.rarity = "R"
	friend_card.character_type = "friend"
	friend_card.position = "MF"
	friend_card.technical_bonus = 0.10
	friend_card.physical_bonus = 0.05
	friend_card.mental_bonus = 0.10
	available_cards.append(friend_card)

	# 김철수 (코치) - SSR
	var coach_card = _CharacterCard.new()
	coach_card.character_id = "coach_cheolsu"
	coach_card.character_name = "김철수 코치"
	coach_card.rarity = "SSR"
	coach_card.character_type = "coach"
	coach_card.position = "MF"
	coach_card.technical_bonus = 0.20
	coach_card.physical_bonus = 0.15
	coach_card.mental_bonus = 0.20
	available_cards.append(coach_card)

	print("[DeckManager] Loaded ", available_cards.size(), " cards")


# ============================================
# 덱 관리
# ============================================


func add_card_to_deck(card: _CharacterCard) -> bool:
	"""
	덱에 카드 추가

	@param card: 추가할 카드
	@return: 성공 여부
	"""

	# 덱이 가득 찬 경우
	if current_deck.size() >= MAX_DECK_SIZE:
		print("[DeckManager] Deck is full (", MAX_DECK_SIZE, ")")
		return false

	# 이미 덱에 있는 경우
	if has_card_in_deck(card.character_id):
		print("[DeckManager] Card already in deck: ", card.character_name)
		return false

	current_deck.append(card)
	card_added.emit(card)
	deck_changed.emit(current_deck)

	print("[DeckManager] Card added: ", card.character_name)
	return true


func remove_card_from_deck(character_id: String) -> bool:
	"""
	덱에서 카드 제거

	@param character_id: 제거할 캐릭터 ID
	@return: 성공 여부
	"""

	for i in range(current_deck.size()):
		if current_deck[i].character_id == character_id:
			var card = current_deck[i]
			current_deck.remove_at(i)
			card_removed.emit(card)
			deck_changed.emit(current_deck)

			print("[DeckManager] Card removed: ", card.character_name)
			return true

	print("[DeckManager] Card not found in deck: ", character_id)
	return false


func has_card_in_deck(character_id: String) -> bool:
	"""덱에 카드가 있는지 확인"""
	for card in current_deck:
		if card.character_id == character_id:
			return true
	return false


func get_card_from_deck(character_id: String) -> _CharacterCard:
	"""덱에서 특정 카드 조회"""
	for card in current_deck:
		if card.character_id == character_id:
			return card
	return null


func clear_deck() -> void:
	"""덱 비우기"""
	current_deck.clear()
	deck_changed.emit(current_deck)
	print("[DeckManager] Deck cleared")


# ============================================
# 훈련 보너스 계산
# ============================================


func calculate_training_bonus(training_type: String) -> Dictionary:
	"""
	현재 덱 기반 훈련 보너스 계산

	@param training_type: 훈련 타입 ("technical", "physical", "mental")
	@return: 보너스 Dictionary
	"""

	var bonus = {"multiplier": 1.0, "active_characters": [], "total_bonus": 0.0}

	if current_deck.is_empty():
		return bonus

	var total_bonus_value = 0.0

	for card in current_deck:
		var card_bonus = 0.0

		match training_type:
			"technical":
				card_bonus = card.technical_bonus
			"physical":
				card_bonus = card.physical_bonus
			"mental":
				card_bonus = card.mental_bonus

		if card_bonus > 0.0:
			total_bonus_value += card_bonus
			bonus["active_characters"].append(card.character_name)

	bonus["total_bonus"] = total_bonus_value
	bonus["multiplier"] = 1.0 + total_bonus_value

	return bonus


func get_deck_quality_score() -> float:
	"""
	덱 품질 점수 계산 (레어도 기반)

	@return: 품질 점수 (0.0 ~ 1.0)
	"""

	if current_deck.is_empty():
		return 0.0

	var total_stars = 0
	var max_stars = current_deck.size() * 4  # 최대 SSR(4성) 기준

	for card in current_deck:
		total_stars += card.get_rarity_stars()

	return float(total_stars) / float(max_stars)


# ============================================
# 캐릭터 등장 체크
# ============================================


func get_appearing_characters(training_type: String) -> Array[String]:
	"""
	현재 훈련에 등장할 캐릭터 ID 목록 반환

	@param training_type: 훈련 타입
	@return: 등장 캐릭터 ID 배열
	"""

	var appearing = []

	for card in current_deck:
		# 보너스가 있는 캐릭터만 등장
		var has_bonus = false

		match training_type:
			"technical":
				has_bonus = card.technical_bonus > 0.0
			"physical":
				has_bonus = card.physical_bonus > 0.0
			"mental":
				has_bonus = card.mental_bonus > 0.0

		if has_bonus:
			appearing.append(card.character_id)

	return appearing


# ============================================
# 사용 가능한 카드 관리
# ============================================


func get_available_cards() -> Array[_CharacterCard]:
	"""덱에 추가 가능한 카드 목록 반환"""
	var available = []

	for card in available_cards:
		if not has_card_in_deck(card.character_id):
			available.append(card)

	return available


func unlock_card(_character_id: String) -> bool:
	"""
	카드 잠금 해제 (이벤트로 획득)

	@param _character_id: 잠금 해제할 캐릭터 ID (TODO: implement)
	@return: 성공 여부
	"""

	# TODO: Week 2에서 카드 획득 시스템 구현
	# 현재는 이미 로드된 카드만 사용 가능
	return false


# ============================================
# 유틸리티
# ============================================


func get_deck_size() -> int:
	"""현재 덱 크기"""
	return current_deck.size()


func is_deck_full() -> bool:
	"""덱이 가득 찼는지 확인"""
	return current_deck.size() >= MAX_DECK_SIZE


func get_deck_summary() -> String:
	"""덱 요약 문자열 반환"""
	if current_deck.is_empty():
		return "Empty Deck"

	var summary = "Deck (" + str(current_deck.size()) + "/" + str(MAX_DECK_SIZE) + "): "
	for card in current_deck:
		summary += card.character_name + " (" + card.rarity + "), "

	return summary.trim_suffix(", ")


# ============================================
# 세이브/로드
# ============================================


func save_deck_data() -> Dictionary:
	"""덱 데이터 저장용 Dictionary 반환"""
	var deck_ids = []
	for card in current_deck:
		deck_ids.append(card.character_id)

	return {"deck_card_ids": deck_ids, "deck_size": current_deck.size()}


func load_deck_data(data: Dictionary) -> void:
	"""저장된 덱 데이터 로드"""
	clear_deck()

	var deck_ids = data.get("deck_card_ids", [])

	for character_id in deck_ids:
		# available_cards에서 찾아서 추가
		for card in available_cards:
			if card.character_id == character_id:
				add_card_to_deck(card)
				break

	print("[DeckManager] Deck loaded: ", current_deck.size(), " cards")


# ============================================
# 디버그
# ============================================


func _debug_print_deck() -> void:
	"""현재 덱 상태 출력 (디버그용)"""
	print("=== Deck Status ===")
	print("Size: ", current_deck.size(), "/", MAX_DECK_SIZE)
	print("Quality: ", get_deck_quality_score())

	for i in range(current_deck.size()):
		var card = current_deck[i]
		print(i + 1, ". ", card.character_name, " (", card.rarity, ")")
		print("   Tech:", card.technical_bonus, " Phys:", card.physical_bonus, " Ment:", card.mental_bonus)

	print("===================")


# ============================================
# Card Loading from JSON (Phase 1 Task 1.3)
# ============================================


func load_card_from_json(card_id: String) -> _CharacterCard:
	"""
	JSON 파일에서 카드 데이터 로드

	@param card_id: 카드 ID (파일명에서 .json 제외)
	@return: _CharacterCard 인스턴스 (실패 시 null)
	"""

	var json_path = "res://data/cards/rewards/%s.json" % card_id

	if not FileAccess.file_exists(json_path):
		push_error("[DeckManager] Card JSON not found: " + json_path)
		return null

	var file = FileAccess.open(json_path, FileAccess.READ)
	if not file:
		push_error("[DeckManager] Failed to open card JSON: " + json_path)
		return null

	var json_text = file.get_as_text()
	file.close()

	var json = JSON.new()
	var error = json.parse(json_text)

	if error != OK:
		push_error("[DeckManager] Failed to parse card JSON: " + json_path)
		push_error("[DeckManager] Parse error at line %d: %s" % [json.get_error_line(), json.get_error_message()])
		return null

	var data = json.get_data()
	if typeof(data) != TYPE_DICTIONARY:
		push_error("[DeckManager] Card JSON is not a dictionary: " + json_path)
		return null

	var card = _CharacterCard.from_dict(data)

	print("[DeckManager] Loaded card from JSON: %s (%s)" % [card.character_name, card.rarity])

	return card
