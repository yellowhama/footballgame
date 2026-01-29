extends Node
## TraitManager - Unified Trait System (2025-12-03)
## 30 traits × 3 tiers (Bronze/Silver/Gold) × 4 equipment slots
## Part of domain/ autoload layer (Business logic)

signal trait_equipped(player_id: String, slot: int, trait_data: Dictionary)
signal trait_unequipped(player_id: String, slot: int)
signal trait_merged(player_id: String, from_tier: int, to_tier: int, trait_type: String)
signal slot_unlocked(player_id: String, slot: int)
signal trait_acquired(player_id: String, trait_data: Dictionary)

## Tier enum (matches Rust TraitTier)
enum TraitTier { BRONZE = 1, SILVER = 2, GOLD = 3 }

## Category enum (matches Rust TraitCategory)
enum TraitCategory { SHOOTING = 0, PASSING = 1, DRIBBLING = 2, DEFENSE = 3, GOALKEEPER = 4 }

## Slot unlock levels
const SLOT_UNLOCK_LEVELS = [1, 10, 20, 30]
const MAX_SLOTS = 4

## Merge requirements
const MERGE_REQUIREMENT = 3  # 3 same-tier traits → 1 higher tier

## Tier multipliers (matches Rust)
const TIER_STAT_MULTIPLIER = {TraitTier.BRONZE: 1.0, TraitTier.SILVER: 1.5, TraitTier.GOLD: 2.5}

const TIER_ACTIVE_MULTIPLIER = {TraitTier.BRONZE: 1.1, TraitTier.SILVER: 1.3, TraitTier.GOLD: 1.8}

## Tier icons
const TIER_ICONS = {TraitTier.BRONZE: "🟤", TraitTier.SILVER: "⚪", TraitTier.GOLD: "🟡"}

const TIER_COLORS = {
	TraitTier.BRONZE: Color(0.6, 0.4, 0.2), TraitTier.SILVER: Color(0.8, 0.8, 0.8), TraitTier.GOLD: Color(1.0, 0.8, 0.2)
}

const TIER_NAMES_KO = {TraitTier.BRONZE: "동특", TraitTier.SILVER: "은특", TraitTier.GOLD: "금특"}

## All 30 traits data
const TRAIT_DATA = {
	# === Shooting & Scoring (7) ===
	"Sniper":
	{"icon": "🔫", "name_ko": "스나이퍼", "category": TraitCategory.SHOOTING, "stats": {"finishing": 4, "long_shots": 3}},
	"Cannon":
	{"icon": "💣", "name_ko": "캐논 슈터", "category": TraitCategory.SHOOTING, "stats": {"long_shots": 4, "shot_power": 3}},
	"Finesse":
	{"icon": "🎯", "name_ko": "감아차기 장인", "category": TraitCategory.SHOOTING, "stats": {"curve": 4, "finishing": 3}},
	"Poacher":
	{"icon": "👻", "name_ko": "침투왕", "category": TraitCategory.SHOOTING, "stats": {"positioning": 4, "anticipation": 3}},
	"Panenka":
	{"icon": "🥄", "name_ko": "강심장", "category": TraitCategory.SHOOTING, "stats": {"composure": 5, "penalties": 4}},
	"LobMaster":
	{"icon": "🌈", "name_ko": "로빙 슛 장인", "category": TraitCategory.SHOOTING, "stats": {"finishing": 3, "vision": 3}},
	"Acrobat":
	{"icon": "🤸", "name_ko": "곡예사", "category": TraitCategory.SHOOTING, "stats": {"agility": 4, "balance": 3}},
	# === Passing & Playmaking (5) ===
	"Maestro":
	{"icon": "👁️", "name_ko": "마에스트로", "category": TraitCategory.PASSING, "stats": {"vision": 4, "passing": 3}},
	"Crosser":
	{"icon": "📦", "name_ko": "택배 크로스", "category": TraitCategory.PASSING, "stats": {"crossing": 5, "curve": 3}},
	"DeadBall":
	{"icon": "📐", "name_ko": "프리킥 마스터", "category": TraitCategory.PASSING, "stats": {"free_kicks": 5, "corners": 4}},
	"Metronome":
	{"icon": "⏱️", "name_ko": "메트로놈", "category": TraitCategory.PASSING, "stats": {"short_passing": 4, "composure": 3}},
	"Architect":
	{"icon": "🏗️", "name_ko": "건축가", "category": TraitCategory.PASSING, "stats": {"long_passing": 4, "vision": 3}},
	"Speedster":
	{"icon": "⚡", "name_ko": "총알탄", "category": TraitCategory.DRIBBLING, "stats": {"pace": 4, "acceleration": 3}},
	# === Dribbling & Ball Control (6) ===
	"Technician":
	{"icon": "🌪️", "name_ko": "앵클 브레이커", "category": TraitCategory.DRIBBLING, "stats": {"dribbling": 4, "agility": 3}},
	"Tank":
	{"icon": "🛡️", "name_ko": "불도저", "category": TraitCategory.DRIBBLING, "stats": {"strength": 4, "balance": 3}},
	"Magnet":
	{
		"icon": "🧲",
		"name_ko": "자석 터치",
		"category": TraitCategory.DRIBBLING,
		"stats": {"first_touch": 5, "ball_control": 4}
	},
	"Showman": {"icon": "🎪", "name_ko": "쇼맨", "category": TraitCategory.DRIBBLING, "stats": {"flair": 5, "agility": 3}},
	"Unshakable":
	{
		"icon": "🗿",
		"name_ko": "탈압박 장인",
		"category": TraitCategory.DRIBBLING,
		"stats": {"ball_control": 3, "composure": 3}
	},
	# === Defense & Physical (8) ===
	"Vacuum":
	{"icon": "🧹", "name_ko": "진공 청소기", "category": TraitCategory.DEFENSE, "stats": {"tackling": 4, "interceptions": 3}},
	"Wall": {"icon": "🧱", "name_ko": "통곡의 벽", "category": TraitCategory.DEFENSE, "stats": {"marking": 4, "heading": 3}},
	"AirRaid":
	{"icon": "✈️", "name_ko": "폭격기", "category": TraitCategory.DEFENSE, "stats": {"jumping": 4, "heading": 4}},
	"Engine":
	{"icon": "🔋", "name_ko": "무한 동력", "category": TraitCategory.DEFENSE, "stats": {"stamina": 5, "work_rate": 4}},
	"Reader":
	{
		"icon": "🧠",
		"name_ko": "요격기",
		"category": TraitCategory.DEFENSE,
		"stats": {"anticipation": 5, "interceptions": 4}
	},
	"Shadow": {"icon": "👤", "name_ko": "그림자", "category": TraitCategory.DEFENSE, "stats": {"marking": 4, "agility": 3}},
	"Bully":
	{"icon": "💪", "name_ko": "파이터", "category": TraitCategory.DEFENSE, "stats": {"strength": 5, "aggression": 4}},
	"Motor":
	{"icon": "🏃", "name_ko": "모터", "category": TraitCategory.DEFENSE, "stats": {"acceleration": 4, "dribbling": 3}},
	# === Goalkeeper (4) ===
	"Spider":
	{"icon": "🕸️", "name_ko": "거미손", "category": TraitCategory.GOALKEEPER, "stats": {"diving": 4, "handling": 3}},
	"Sweeper":
	{"icon": "🧤", "name_ko": "스위퍼", "category": TraitCategory.GOALKEEPER, "stats": {"speed": 4, "reflexes": 3}},
	"Giant":
	{"icon": "🗼", "name_ko": "제공권 장악", "category": TraitCategory.GOALKEEPER, "stats": {"jumping": 4, "positioning": 3}},
	"Quarterback":
	{"icon": "🎯", "name_ko": "배급자", "category": TraitCategory.GOALKEEPER, "stats": {"kicking": 5, "throwing": 4}}
}

const CATEGORY_NAMES_KO = {
	TraitCategory.SHOOTING: "슈팅",
	TraitCategory.PASSING: "패스",
	TraitCategory.DRIBBLING: "드리블",
	TraitCategory.DEFENSE: "수비",
	TraitCategory.GOALKEEPER: "골키퍼"
}

## Player trait data storage
## Structure: { player_id: { "slots": [null, null, null, null], "inventory": [], "level": 1 } }
var _player_traits: Dictionary = {}


func _ready() -> void:
	print("[TraitManager] Initialized - 30 traits × 3 tiers × 4 slots")


# ============================================================================
# Player Initialization
# ============================================================================


func initialize_player(player_id: String, level: int = 1) -> void:
	if _player_traits.has(player_id):
		return

	_player_traits[player_id] = {"slots": [null, null, null, null], "inventory": [], "level": level}
	print("[TraitManager] Initialized player: %s (level %d)" % [player_id, level])


func get_player_data(player_id: String) -> Dictionary:
	if not _player_traits.has(player_id):
		initialize_player(player_id)
	return _player_traits[player_id]


# ============================================================================
# Slot Management
# ============================================================================


## Get number of unlocked slots based on player level
func get_unlocked_slot_count(player_id: String) -> int:
	var data = get_player_data(player_id)
	var level = data.level
	var count = 0
	for unlock_level in SLOT_UNLOCK_LEVELS:
		if level >= unlock_level:
			count += 1
	return count


## Check if a slot is unlocked
func is_slot_unlocked(player_id: String, slot_index: int) -> bool:
	if slot_index < 0 or slot_index >= MAX_SLOTS:
		return false
	var data = get_player_data(player_id)
	return data.level >= SLOT_UNLOCK_LEVELS[slot_index]


## Equip a trait from inventory to a slot
func equip_trait(player_id: String, slot_index: int, trait_type: String, tier: int) -> bool:
	if not is_slot_unlocked(player_id, slot_index):
		push_error("[TraitManager] Slot %d not unlocked for player %s" % [slot_index, player_id])
		return false

	if not TRAIT_DATA.has(trait_type):
		push_error("[TraitManager] Invalid trait type: %s" % trait_type)
		return false

	var data = get_player_data(player_id)

	# Check inventory has this trait
	var inventory_idx = _find_in_inventory(data.inventory, trait_type, tier)
	if inventory_idx == -1:
		push_error("[TraitManager] Trait %s (%s) not in inventory" % [trait_type, TIER_NAMES_KO[tier]])
		return false

	# Unequip current trait in slot (if any)
	if data.slots[slot_index] != null:
		unequip_trait(player_id, slot_index)

	# Remove from inventory and equip
	var trait_instance = data.inventory[inventory_idx]
	data.inventory.remove_at(inventory_idx)
	data.slots[slot_index] = trait_instance

	trait_equipped.emit(player_id, slot_index, trait_instance)
	print(
		(
			"[TraitManager] Equipped %s (%s) to slot %d for player %s"
			% [trait_type, TIER_NAMES_KO[tier], slot_index, player_id]
		)
	)
	return true


## Unequip a trait from a slot back to inventory
func unequip_trait(player_id: String, slot_index: int) -> bool:
	if slot_index < 0 or slot_index >= MAX_SLOTS:
		return false

	var data = get_player_data(player_id)
	if data.slots[slot_index] == null:
		return false

	# Move back to inventory
	data.inventory.append(data.slots[slot_index])
	data.slots[slot_index] = null

	trait_unequipped.emit(player_id, slot_index)
	print("[TraitManager] Unequipped slot %d for player %s" % [slot_index, player_id])
	return true


## Get equipped traits for match simulation
func get_equipped_traits(player_id: String) -> Array:
	var data = get_player_data(player_id)
	var equipped = []
	for slot in data.slots:
		if slot != null:
			equipped.append(slot)
	return equipped


# ============================================================================
# Inventory Management
# ============================================================================


## Add a trait to player's inventory
func add_trait_to_inventory(player_id: String, trait_type: String, tier: int) -> bool:
	if not TRAIT_DATA.has(trait_type):
		push_error("[TraitManager] Invalid trait type: %s" % trait_type)
		return false

	if tier < TraitTier.BRONZE or tier > TraitTier.GOLD:
		push_error("[TraitManager] Invalid tier: %d" % tier)
		return false

	var data = get_player_data(player_id)
	var trait_instance = {"type": trait_type, "tier": tier, "acquired_at": Time.get_ticks_msec()}
	data.inventory.append(trait_instance)

	trait_acquired.emit(player_id, trait_instance)
	print("[TraitManager] Added %s (%s) to inventory for player %s" % [trait_type, TIER_NAMES_KO[tier], player_id])
	return true


## Get inventory grouped by tier
func get_inventory_by_tier(player_id: String) -> Dictionary:
	var data = get_player_data(player_id)
	var result = {TraitTier.BRONZE: [], TraitTier.SILVER: [], TraitTier.GOLD: []}
	for trait_item in data.inventory:
		result[trait_item.tier].append(trait_item)
	return result


## Get inventory grouped by type (for merge counting)
func get_inventory_by_type(player_id: String) -> Dictionary:
	var data = get_player_data(player_id)
	var result = {}
	for trait_item in data.inventory:
		var key = "%s_%d" % [trait_item.type, trait_item.tier]
		if not result.has(key):
			result[key] = []
		result[key].append(trait_item)
	return result


# ============================================================================
# Merge System (3 → 1 upgrade)
# ============================================================================


## Check if merge is possible for a trait type and tier
func can_merge(player_id: String, trait_type: String, tier: int) -> bool:
	if tier >= TraitTier.GOLD:
		return false  # Gold is max

	var data = get_player_data(player_id)
	var count = _count_in_inventory(data.inventory, trait_type, tier)
	return count >= MERGE_REQUIREMENT


## Get mergeable combinations for a player
func get_mergeable_traits(player_id: String) -> Array:
	var data = get_player_data(player_id)
	var mergeable = []

	# Count each type+tier combination
	var type_tier_counts = {}
	for trait_item in data.inventory:
		var key = "%s_%d" % [trait_item.type, trait_item.tier]
		if not type_tier_counts.has(key):
			type_tier_counts[key] = {"type": trait_item.type, "tier": trait_item.tier, "count": 0}
		type_tier_counts[key].count += 1

	# Find mergeable ones
	for key in type_tier_counts:
		var info = type_tier_counts[key]
		if info.tier < TraitTier.GOLD and info.count >= MERGE_REQUIREMENT:
			mergeable.append(info)

	return mergeable


## Perform merge: 3 same traits → 1 higher tier
func merge_traits(player_id: String, trait_type: String, tier: int) -> bool:
	if not can_merge(player_id, trait_type, tier):
		push_error(
			"[TraitManager] Cannot merge %s (%s) - need %d" % [trait_type, TIER_NAMES_KO[tier], MERGE_REQUIREMENT]
		)
		return false

	var data = get_player_data(player_id)
	var new_tier = tier + 1

	# Remove 3 from inventory
	var removed = 0
	var i = data.inventory.size() - 1
	while i >= 0 and removed < MERGE_REQUIREMENT:
		if data.inventory[i].type == trait_type and data.inventory[i].tier == tier:
			data.inventory.remove_at(i)
			removed += 1
		i -= 1

	# Add 1 higher tier
	add_trait_to_inventory(player_id, trait_type, new_tier)

	trait_merged.emit(player_id, tier, new_tier, trait_type)
	print(
		(
			"[TraitManager] Merged 3× %s (%s) → 1× %s (%s) for player %s"
			% [trait_type, TIER_NAMES_KO[tier], trait_type, TIER_NAMES_KO[new_tier], player_id]
		)
	)
	return true


# ============================================================================
# Level Up & Slot Unlock
# ============================================================================


## Set player level and check for slot unlocks
func set_player_level(player_id: String, new_level: int) -> void:
	var data = get_player_data(player_id)
	var old_level = data.level
	data.level = new_level

	# Check for new slot unlocks
	for i in range(MAX_SLOTS):
		var unlock_level = SLOT_UNLOCK_LEVELS[i]
		if old_level < unlock_level and new_level >= unlock_level:
			slot_unlocked.emit(player_id, i)
			print("[TraitManager] Slot %d unlocked for player %s at level %d" % [i, player_id, new_level])


# ============================================================================
# UI Helper Functions
# ============================================================================


## Get display info for a trait
func get_trait_display(trait_type: String, tier: int) -> Dictionary:
	if not TRAIT_DATA.has(trait_type):
		return {}

	var base = TRAIT_DATA[trait_type]
	var stat_mult = TIER_STAT_MULTIPLIER[tier]

	# Calculate scaled stats
	var scaled_stats = {}
	for stat_name in base.stats:
		scaled_stats[stat_name] = int(base.stats[stat_name] * stat_mult)

	return {
		"type": trait_type,
		"tier": tier,
		"icon": base.icon,
		"name_ko": base.name_ko,
		"tier_icon": TIER_ICONS[tier],
		"tier_name": TIER_NAMES_KO[tier],
		"tier_color": TIER_COLORS[tier],
		"category": base.category,
		"category_name": CATEGORY_NAMES_KO[base.category],
		"base_stats": base.stats,
		"scaled_stats": scaled_stats,
		"stat_multiplier": stat_mult,
		"active_multiplier": TIER_ACTIVE_MULTIPLIER[tier]
	}


## Get all traits by category
func get_traits_by_category(category: int) -> Array:
	var result = []
	for trait_type in TRAIT_DATA:
		if TRAIT_DATA[trait_type].category == category:
			result.append(trait_type)
	return result


## Get category icon
func get_category_icon(category: int) -> String:
	match category:
		TraitCategory.SHOOTING:
			return "⚽"
		TraitCategory.PASSING:
			return "📤"
		TraitCategory.DRIBBLING:
			return "🦶"
		TraitCategory.DEFENSE:
			return "🛡️"
		TraitCategory.GOALKEEPER:
			return "🧤"
		_:
			return "❓"


# ============================================================================
# Save/Load (SaveManager Integration)
# ============================================================================


## Save all trait data to dictionary (called by SaveManager)
func save_to_dict() -> Dictionary:
	return {"version": 1, "player_traits": _player_traits.duplicate(true)}


## Load trait data from dictionary (called by SaveManager)
func load_from_dict(data: Dictionary) -> void:
	if not data.has("player_traits"):
		print("[TraitManager] No trait data in save")
		return

	_player_traits = data["player_traits"].duplicate(true)
	print("[TraitManager] Loaded trait data for %d players" % _player_traits.size())

	# Log summary
	for player_id in _player_traits:
		var pdata = _player_traits[player_id]
		var equipped_count = 0
		for slot in pdata.slots:
			if slot != null:
				equipped_count += 1
		print(
			(
				"[TraitManager] Player %s: %d equipped, %d in inventory"
				% [player_id, equipped_count, pdata.inventory.size()]
			)
		)


## Export single player's data
func export_player_data(player_id: String) -> Dictionary:
	if not _player_traits.has(player_id):
		return {}
	return _player_traits[player_id].duplicate(true)


## Import single player's data
func import_player_data(player_id: String, data: Dictionary) -> void:
	_player_traits[player_id] = data.duplicate(true)
	print("[TraitManager] Imported data for player: %s" % player_id)


## Clear all data
func clear_all_data() -> void:
	_player_traits.clear()
	print("[TraitManager] Cleared all player trait data")


# ============================================================================
# Rust Integration (for match simulation)
# ============================================================================


## Convert equipped traits to JSON for Rust match engine
func get_traits_for_match_json(player_id: String) -> String:
	var equipped = get_equipped_traits(player_id)
	var traits_array = []

	for trait_item in equipped:
		traits_array.append({"id": trait_item.type, "tier": trait_item.tier})

	return JSON.stringify({"traits": traits_array})


# ============================================================================
# Private Helpers
# ============================================================================


func _find_in_inventory(inventory: Array, trait_type: String, tier: int) -> int:
	for i in range(inventory.size()):
		if inventory[i].type == trait_type and inventory[i].tier == tier:
			return i
	return -1


func _count_in_inventory(inventory: Array, trait_type: String, tier: int) -> int:
	var count = 0
	for item in inventory:
		if item.type == trait_type and item.tier == tier:
			count += 1
	return count


# ============================================================================
# Testing
# ============================================================================


func create_test_player(player_id: String) -> void:
	initialize_player(player_id, 25)  # Level 25 = 3 slots unlocked

	# Add some test traits
	add_trait_to_inventory(player_id, "Sniper", TraitTier.BRONZE)
	add_trait_to_inventory(player_id, "Sniper", TraitTier.BRONZE)
	add_trait_to_inventory(player_id, "Sniper", TraitTier.BRONZE)
	add_trait_to_inventory(player_id, "Maestro", TraitTier.SILVER)
	add_trait_to_inventory(player_id, "Vacuum", TraitTier.GOLD)
	add_trait_to_inventory(player_id, "Speedster", TraitTier.BRONZE)

	# Equip some
	equip_trait(player_id, 0, "Maestro", TraitTier.SILVER)
	equip_trait(player_id, 1, "Vacuum", TraitTier.GOLD)

	print("[TraitManager] Created test player: %s" % player_id)
	print_player_traits(player_id)


func print_player_traits(player_id: String) -> void:
	var data = get_player_data(player_id)
	print("========== Player %s Traits ==========" % player_id)
	print("Level: %d (Slots: %d/%d)" % [data.level, get_unlocked_slot_count(player_id), MAX_SLOTS])

	print("Equipped Slots:")
	for i in range(MAX_SLOTS):
		var slot = data.slots[i]
		var locked = not is_slot_unlocked(player_id, i)
		if locked:
			print("  [%d] 🔒 (Unlock at Lv.%d)" % [i, SLOT_UNLOCK_LEVELS[i]])
		elif slot == null:
			print("  [%d] Empty" % i)
		else:
			var display = get_trait_display(slot.type, slot.tier)
			print("  [%d] %s %s %s" % [i, display.tier_icon, display.icon, display.name_ko])

	print("Inventory: %d traits" % data.inventory.size())
	for item in data.inventory:
		var display = get_trait_display(item.type, item.tier)
		print("  - %s %s %s" % [display.tier_icon, display.icon, display.name_ko])

	print("Mergeable:")
	var mergeable = get_mergeable_traits(player_id)
	for m in mergeable:
		print("  - %s (%s): %d → can merge!" % [m.type, TIER_NAMES_KO[m.tier], m.count])
	print("==========================================")
