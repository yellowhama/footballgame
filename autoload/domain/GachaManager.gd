extends Node

## GachaManager - 가챠/인벤토리/덱 도메인 오케스트레이터
##
## SSOT: Rust(of_core::coach) via FootballRustEngine (GDExtension)
## - Godot는 UI/입력/표시만 담당하고, 확률/피티/보너스 계산은 절대 중복하지 않는다.
##
## NOTE: 일부 API는 Phase 1/2 마이그레이션 동안 구버전(legacy) 경로로 fallback 할 수 있다.
##       최종 목표는 Dict 기반 API만 남기는 것.

signal gacha_draw_completed(result: Dictionary)
signal inventory_updated(cards: Array)
signal deck_saved(deck_id: String)
signal deck_loaded(deck_id: String, deck_data: Dictionary)

# ============================================
# Constants (UI-only)
# ============================================

const PITY_THRESHOLD: int = 100  # 천장

const CARD_TYPES := {"manager": "감독", "coach": "코치", "tactics": "전술"}
const SPECIALTIES := {
	"speed": "스피드",
	"power": "파워",
	"technical": "테크닉",
	"mental": "멘탈",
	"balanced": "밸런스",
}

# ============================================
# Cached state (display-only)
# ============================================

var _cached_pity_counter: int = 0
var _cached_inventory: Array = []
var _current_banner_id: String = ""


func _ready() -> void:
	print("[GachaManager] Initialized")
	_refresh_pity_counter()


# ============================================
# Public API: Gacha
# ============================================


## 단일 뽑기
## @return Dictionary {success, card, is_new, pity_counter, pity_remaining}
func draw_single(banner_id: String = "") -> Dictionary:
	if not _check_rust_engine():
		return _create_error_result("Rust engine not available", "ENGINE_NOT_READY")

	var pull: Dictionary = FootballRustEngine.gacha_pull_single(_generate_seed())
	var normalized := _normalize_gacha_pull_result(pull, 1)
	if not normalized.get("success", false):
		return normalized

	_refresh_pity_counter()

	var card: Dictionary = normalized.get("card", {})
	var result := {
		"success": true,
		"card": card,
		"is_new": normalized.get("is_new", false),
		"pity_counter": _cached_pity_counter,
		"pity_remaining": get_pity_remaining(),
		"banner_id": banner_id if banner_id else _current_banner_id,
	}

	gacha_draw_completed.emit(result)
	print(
		"[GachaManager] Single draw: %s (rarity %d)"
		% [card.get("name", "Unknown"), card.get("rarity", 1)]
	)
	return result


## 10연차 뽑기
## @return Dictionary {success, cards, is_new_list, pity_counter, pity_remaining}
func draw_10x(banner_id: String = "") -> Dictionary:
	if not _check_rust_engine():
		return _create_error_result("Rust engine not available", "ENGINE_NOT_READY")

	var pull: Dictionary = FootballRustEngine.gacha_pull_ten(_generate_seed())
	var normalized := _normalize_gacha_pull_result(pull, 10)
	if not normalized.get("success", false):
		return normalized

	_refresh_pity_counter()

	var cards: Array = normalized.get("cards", [])
	var result := {
		"success": true,
		"cards": cards,
		"is_new_list": normalized.get("is_new_list", []),
		"pity_counter": _cached_pity_counter,
		"pity_remaining": get_pity_remaining(),
		"banner_id": banner_id if banner_id else _current_banner_id,
	}

	gacha_draw_completed.emit(result)
	print("[GachaManager] 10x draw completed: %d cards" % cards.size())
	return result


# ============================================
# Public API: Inventory
# ============================================


## 인벤토리 조회
## @param filter: Dictionary {type, rarity, specialty}
## @return: Dictionary {success, cards, total_count, ...}
func get_inventory(filter: Dictionary = {}) -> Dictionary:
	if not _check_rust_engine():
		return _create_error_result("Rust engine not available", "ENGINE_NOT_READY")

	if not FootballRustEngine.has_method("coach_get_inventory"):
		return _create_error_result("Inventory API not available", "INVENTORY_API_MISSING")

	var result: Dictionary = FootballRustEngine.coach_get_inventory(filter)

	if result.get("success", false):
		_cached_inventory = result.get("cards", [])
		inventory_updated.emit(_cached_inventory)

	return result


func get_card_count() -> int:
	return _cached_inventory.size()


func get_collection_rate() -> Dictionary:
	# TODO: Replace with Rust SSOT collection_count once coach_get_inventory is wired.
	var total_cards = 120  # 감독 20 + 코치 60 + 전술 40
	var collected = 0
	var unique_ids = {}

	for card in _cached_inventory:
		var card_id = card.get("id", "")
		if not unique_ids.has(card_id):
			unique_ids[card_id] = true
			collected += 1

	return {
		"collected": collected,
		"total": total_cards,
		"percentage": (float(collected) / total_cards) * 100 if total_cards > 0 else 0,
	}


# ============================================
# Public API: Deck
# ============================================


## 덱 저장
func save_deck(deck_data: Dictionary) -> Dictionary:
	if not _check_rust_engine():
		return _create_error_result("Rust engine not available", "ENGINE_NOT_READY")

	if not FootballRustEngine.has_method("deck_upsert"):
		return _create_error_result("Deck API not available", "DECK_API_MISSING")

	var result: Dictionary = FootballRustEngine.deck_upsert(deck_data)

	if result.get("success", false):
		deck_saved.emit(deck_data.get("deck_id", ""))
		print("[GachaManager] Deck saved: %s" % deck_data.get("deck_name", "Unknown"))

	return result


## 덱 로드
func load_deck(deck_id: String) -> Dictionary:
	if not _check_rust_engine():
		return _create_error_result("Rust engine not available", "ENGINE_NOT_READY")

	if not FootballRustEngine.has_method("deck_get_active"):
		return _create_error_result("Deck API not available", "DECK_API_MISSING")

	var result: Dictionary = FootballRustEngine.deck_get_active()

	if result.get("success", false):
		var deck = result.get("deck", {})
		deck_loaded.emit(deck_id, deck)
		print("[GachaManager] Deck loaded: %s" % deck_id)

	return result


# ============================================
# Public API: State
# ============================================


func get_pity_remaining() -> int:
	return max(0, PITY_THRESHOLD - get_pity_counter())


func get_pity_counter() -> int:
	_refresh_pity_counter()
	return _cached_pity_counter


func get_total_draws() -> int:
	# Not tracked on Godot side. (SSOT belongs to Rust.)
	return 0


func set_current_banner(banner_id: String) -> void:
	_current_banner_id = banner_id


func get_rarity_stars(rarity: int) -> String:
	return "★".repeat(max(1, min(rarity, 5)))


func get_rarity_color(rarity: int) -> Color:
	match rarity:
		5:
			return Color("#FFD700")  # gold
		4:
			return Color("#9B59B6")  # purple
		3:
			return Color("#3498DB")  # blue
		2:
			return Color("#27AE60")  # green
		_:
			return Color("#95A5A6")  # gray


# ============================================
# Internal helpers
# ============================================


func _check_rust_engine() -> bool:
	return FootballRustEngine and FootballRustEngine.is_ready()


func _generate_seed() -> int:
	return int(Time.get_unix_time_from_system() * 1000) + randi()


func _refresh_pity_counter() -> void:
	if not _check_rust_engine():
		_cached_pity_counter = 0
		return
	if FootballRustEngine.has_method("gacha_get_pity_count"):
		_cached_pity_counter = int(FootballRustEngine.gacha_get_pity_count())


func _create_error_result(message: String, error_code: String = "MANAGER_ERROR") -> Dictionary:
	return {"success": false, "error": message, "error_code": error_code}


func _normalize_gacha_pull_result(pull: Dictionary, expected_card_count: int) -> Dictionary:
	# New schema: {success, card/cards, ...}
	if pull.has("success"):
		if not pull.get("success", false):
			return pull

		var cards_v = pull.get("cards", [])
		var card_v = pull.get("card", {})
		if cards_v is Array and not cards_v.is_empty():
			return {
				"success": true,
				"cards": cards_v,
				"is_new_list": pull.get("is_new_list", []),
				"card": cards_v[0] if not cards_v.is_empty() else {},
				"is_new": bool(pull.get("is_new", false)),
			}
		if card_v is Dictionary and not card_v.is_empty():
			return {"success": true, "cards": [card_v], "card": card_v, "is_new": bool(pull.get("is_new", false))}

		return _create_error_result("Empty gacha result", "EMPTY_GACHA_RESULT")

	# Old schema (current gacha_pull_*): {cards, is_new, summary, new_count}
	if pull.get("error", false):
		return _create_error_result(pull.get("message", "Gacha pull failed"), pull.get("error_code", "GACHA_PULL_FAILED"))

	var cards: Array = pull.get("cards", [])
	if cards.is_empty():
		return _create_error_result("Empty gacha result", "EMPTY_GACHA_RESULT")

	if expected_card_count > 0 and cards.size() != expected_card_count:
		push_warning(
			(
				"[GachaManager] Unexpected gacha card count: got=%d expected=%d"
				% [cards.size(), expected_card_count]
			)
		)

	var is_new_list: Array = pull.get("is_new", [])
	var card: Dictionary = cards[0]
	var is_new: bool = bool(is_new_list[0]) if is_new_list.size() > 0 else false

	return {
		"success": true,
		"cards": cards,
		"is_new_list": is_new_list,
		"card": card,
		"is_new": is_new,
	}
