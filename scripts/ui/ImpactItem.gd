extends HBoxContainer
class_name ImpactItem

## Individual impact item for ChoiceImpactPopup
## Displays one effect from a player choice (affection, trait, card, ending, stat)

@onready var icon: TextureRect = $Icon
@onready var label: Label = $Label

# Icon textures for different impact types
var icon_map := {
	"affection": "res://assets/ui/icons/heart.png",
	"trait": "res://assets/ui/icons/star.png",
	"card": "res://assets/ui/icons/card.png",
	"ending": "res://assets/ui/icons/trophy.png",
	"stat": "res://assets/ui/icons/arrow_up.png"
}


func set_impact_data(data: Dictionary) -> void:
	"""
	Set impact data and update UI
	@param data: Impact data dictionary with keys:
		- type: "affection", "trait", "card", "ending", "stat"
		- (type-specific fields)
	"""
	var type: String = str(data.get("type", ""))

	# Set icon (fallback to placeholder if texture doesn't exist)
	var icon_path: String = icon_map.get(type, "")
	if icon_path != "" and FileAccess.file_exists(icon_path):
		icon.texture = load(icon_path)
	else:
		# Use colored rect as fallback
		icon.texture = null
		icon.modulate = _get_fallback_color(type)

	# Set label text based on type
	match type:
		"affection":
			_set_affection_text(data)

		"trait":
			_set_trait_text(data)

		"card":
			_set_card_text(data)

		"ending":
			_set_ending_text(data)

		"stat":
			_set_stat_text(data)

		_:
			label.text = "Unknown impact type: %s" % type


func _set_affection_text(data: Dictionary) -> void:
	var char_name: String = str(data.get("character", ""))
	var value: int = data.get("value", 0)
	var from_val: int = data.get("from", 0)
	var to_val: int = data.get("to", 0)

	label.text = "%s 호감도 %+d (%d → %d)" % [_get_character_name(char_name), value, from_val, to_val]


func _set_trait_text(data: Dictionary) -> void:
	var trait_name: String = str(data.get("trait_name", ""))
	var level: int = data.get("level", 1)

	label.text = '"%s Lv.%d" 획득' % [_get_trait_name(trait_name), level]


func _set_card_text(data: Dictionary) -> void:
	var card_id: String = str(data.get("card_id", ""))

	# Try to load card data from DeckManager
	if DeckManager:
		var card_data = DeckManager.load_card_from_json(card_id)
		if card_data and card_data is Dictionary:
			var rarity_str: String = str(card_data.get("rarity")) if card_data.has("rarity") else "?"
			var char_name: String = str(card_data.get("character_name")) if card_data.has("character_name") else card_id
			label.text = "%s %s 카드 획득" % [rarity_str, char_name]
			return

	# Fallback if card not found
	label.text = "%s 카드 획득" % card_id


func _set_ending_text(data: Dictionary) -> void:
	var ending_id: String = str(data.get("ending_id", ""))
	var progress: int = data.get("progress", 0)
	var total: int = data.get("total", 3)

	label.text = "%s 진입 (%d/%d)" % [_get_ending_name(ending_id), progress, total]


func _set_stat_text(data: Dictionary) -> void:
	var stat_name: String = str(data.get("stat_name", ""))
	var value: int = data.get("value", 0)

	label.text = "%s %+d" % [stat_name, value]


func _get_character_name(id: String) -> String:
	"""Get localized character name from ID"""
	var names := {
		"kang_taeyang": "강태양", "park_minjun": "박민준", "coach_kim": "김철수", "lee_seojun": "이서준", "choi_jihoon": "최지훈"
	}
	return names.get(id, id)


func _get_trait_name(id: String) -> String:
	"""Get localized trait name from ID"""
	# Use PlayerData's function if available
	if PlayerData and PlayerData.has_method("get_exclusive_trait_name"):
		return PlayerData.get_exclusive_trait_name()

	# Fallback
	var names := {
		"rival_awakening": "라이벌 각성",
		"team_chemistry": "팀 케미",
		"tactical_understanding": "전술 이해도",
		"leadership": "리더십",
		"iron_defense": "철벽 수비"
	}
	return names.get(id, id)


func _get_ending_name(id: String) -> String:
	"""Get localized ending name from ID"""
	var names := {
		"true_ending": "True Ending",
		"rival_true_ending": "라이벌 True Ending",
		"rival_alt_ending": "라이벌 Alternative Ending",
		"friendship_ending": "우정 Ending",
		"mentor_ending": "멘토 Ending",
		"captain_ending": "주장 Ending",
		"guardian_ending": "수호신 Ending"
	}
	return names.get(id, id)


func _get_fallback_color(type: String) -> Color:
	"""Get fallback color when icon texture is missing"""
	match type:
		"affection":
			return Color.PINK
		"trait":
			return Color.GOLD
		"card":
			return Color.CYAN
		"ending":
			return Color.YELLOW
		"stat":
			return Color.GREEN_YELLOW
		_:
			return Color.WHITE
