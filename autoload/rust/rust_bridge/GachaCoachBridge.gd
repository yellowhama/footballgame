class_name GachaCoachBridge
extends RefCounted
## ============================================================================
## GachaCoachBridge - Gacha/Coach/Deck System API
## ============================================================================
##
## PURPOSE: Bridge for gacha pulls, coach state, and deck management
##
## EXTRACTED FROM: FootballRustEngine.gd (ST-006 God Class refactoring)
##
## RESPONSIBILITIES:
## - Gacha single/ten pulls with pity system
## - Coach state export/import/reset
## - Coach inventory management
## - Deck validation, CRUD operations
## - Deck bonus calculations
##
## DEPENDENCIES:
## - _rust_simulator: GDExtension Rust object
##
## USAGE:
##   var bridge := GachaCoachBridge.new()
##   bridge.initialize(rust_simulator)
##   var result := bridge.gacha_pull_single()
## ============================================================================

var _rust_simulator: Object = null
var _is_ready: bool = false


func initialize(rust_simulator: Object) -> void:
	"""Initialize GachaCoachBridge with Rust simulator reference"""
	_rust_simulator = rust_simulator
	_is_ready = rust_simulator != null


# =============================================================================
# Gacha System API
# =============================================================================

func gacha_pull_single(rng_seed: int = -1) -> Dictionary:
	"""Perform a single gacha pull.
	Returns Dictionary: { cards: Array, is_new: Array, summary: String, new_count: int }
	"""
	if not _is_ready:
		return {"error": true, "message": "Engine not ready"}
	var actual_seed := rng_seed if rng_seed >= 0 else Time.get_ticks_usec()
	if not _rust_simulator.has_method("gacha_pull_single"):
		return {"error": true, "message": "gacha_pull_single not available"}
	return _rust_simulator.gacha_pull_single(actual_seed)


func gacha_pull_ten(rng_seed: int = -1) -> Dictionary:
	"""Perform a 10-pull gacha with guaranteed 3-star+.
	Returns Dictionary: { cards: Array, is_new: Array, summary: String, new_count: int }
	"""
	if not _is_ready:
		return {"error": true, "message": "Engine not ready"}
	var actual_seed := rng_seed if rng_seed >= 0 else Time.get_ticks_usec()
	if not _rust_simulator.has_method("gacha_pull_ten"):
		return {"error": true, "message": "gacha_pull_ten not available"}
	return _rust_simulator.gacha_pull_ten(actual_seed)


func gacha_get_pity_count() -> int:
	"""Get the current pity counter.
	Returns int: Current pity counter (0-100)
	"""
	if not _is_ready:
		return 0
	if not _rust_simulator.has_method("gacha_get_pity_count"):
		return 0
	return _rust_simulator.gacha_get_pity_count()


# =============================================================================
# Coach State API (SSOT: Rust)
# =============================================================================

func coach_export_state() -> Dictionary:
	"""Export coach state (gacha/deck/inventory) for SaveManager (SSOT: Rust).
	@return: Dictionary {success, state} or {success=false, error, error_code}
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("coach_export_state"):
		return {"success": false, "error": "coach_export_state not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.coach_export_state()


func coach_import_state(state: Dictionary) -> Dictionary:
	"""Import coach state (gacha/deck/inventory) from SaveManager (SSOT: Rust).
	@param state: Dictionary returned by coach_export_state().state
	@return: Dictionary {success} or {success=false, error, error_code}
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("coach_import_state"):
		return {"success": false, "error": "coach_import_state not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.coach_import_state(state)


func coach_reset_state() -> Dictionary:
	"""Reset coach state (gacha/deck/inventory) to defaults (SSOT: Rust).
	Used when loading legacy saves without `coach_state`.
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("coach_reset_state"):
		return {"success": false, "error": "coach_reset_state not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.coach_reset_state()


func coach_get_inventory(filter: Dictionary = {}) -> Dictionary:
	"""Get coach inventory (SSOT: Rust).
	@param filter: Dictionary {type, rarity, specialty} (all optional)
	@return: Dictionary {success, cards, total_count, collection_count, capacity}
	"""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("coach_get_inventory"):
		return {"success": false, "error": "coach_get_inventory not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.coach_get_inventory(filter)


func coach_add_cards(card_ids: PackedStringArray) -> Dictionary:
	"""Admin helper: add cards by id (SSOT: Rust)."""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("coach_add_cards"):
		return {"success": false, "error": "coach_add_cards not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.coach_add_cards(card_ids)


# =============================================================================
# Deck API (SSOT: Rust)
# =============================================================================

func deck_validate(deck: Dictionary) -> Dictionary:
	"""Validate deck schema + ownership (SSOT: Rust)."""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("deck_validate"):
		return {"success": false, "error": "deck_validate not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.deck_validate(deck)


func deck_upsert(deck: Dictionary) -> Dictionary:
	"""Upsert (save) deck (SSOT: Rust)."""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("deck_upsert"):
		return {"success": false, "error": "deck_upsert not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.deck_upsert(deck)


func deck_delete(deck_id: String) -> Dictionary:
	"""Delete deck (SSOT: Rust)."""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("deck_delete"):
		return {"success": false, "error": "deck_delete not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.deck_delete(deck_id)


func deck_set_active(deck_id: String) -> Dictionary:
	"""Set active deck (SSOT: Rust)."""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("deck_set_active"):
		return {"success": false, "error": "deck_set_active not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.deck_set_active(deck_id)


func deck_get_active() -> Dictionary:
	"""Get active deck (SSOT: Rust)."""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("deck_get_active"):
		return {"success": false, "error": "deck_get_active not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.deck_get_active()


func deck_calculate_training_bonus(deck: Dictionary, training_type: String) -> Dictionary:
	"""Calculate training bonus (SSOT: Rust)."""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("deck_calculate_training_bonus"):
		return {"success": false, "error": "deck_calculate_training_bonus not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.deck_calculate_training_bonus(deck, training_type)


func deck_calculate_match_modifiers(deck: Dictionary) -> Dictionary:
	"""Calculate match modifiers (SSOT: Rust)."""
	if not _is_ready:
		return {"success": false, "error": "Engine not ready", "error_code": "NOT_READY"}
	if not _rust_simulator.has_method("deck_calculate_match_modifiers"):
		return {"success": false, "error": "deck_calculate_match_modifiers not available", "error_code": "NOT_IMPLEMENTED"}
	return _rust_simulator.deck_calculate_match_modifiers(deck)
