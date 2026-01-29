extends PanelContainer
class_name ChoiceLogEntry

## Individual choice log entry
## Displays one choice with immediate and long-term impacts

@onready var week_label: Label = $MarginContainer/VBox/Header/WeekLabel
@onready var route_label: Label = $MarginContainer/VBox/Header/RouteLabel
@onready var choice_text_label: Label = $MarginContainer/VBox/ChoiceText
@onready var immediate_impacts_list: VBoxContainer = $MarginContainer/VBox/ImmediateImpacts/List
@onready var long_term_impacts_list: VBoxContainer = $MarginContainer/VBox/LongTermImpacts/List
@onready var long_term_section: VBoxContainer = $MarginContainer/VBox/LongTermImpacts


func set_choice_data(data: Dictionary) -> void:
	"""
	Set choice data and populate UI

	@param data: Choice record from EventManager.choice_history
		{
			"week": 75,
			"route": "rival",
			"branch_id": 2,
			"choice": "A",
			"choice_text": "정정당당하게 경쟁하자",
			"immediate_impacts": [...],
			"long_term_impacts": [...]
		}
	"""
	# Set header
	week_label.text = "Week %d" % data.get("week", 0)

	var route: String = str(data.get("route", ""))
	var branch_id: int = data.get("branch_id", 0)
	route_label.text = "%s 루트 분기점 %d" % [_get_route_name(route), branch_id]

	# Set choice text
	var choice: String = str(data.get("choice", ""))
	var choice_text: String = str(data.get("choice_text", ""))
	choice_text_label.text = '선택: %s. "%s"' % [choice, choice_text]

	# Populate immediate impacts
	var immediate_impacts: Array = data.get("immediate_impacts", [])
	_populate_immediate_impacts(immediate_impacts)

	# Populate long-term impacts
	var long_term_impacts: Array = data.get("long_term_impacts", [])
	if long_term_impacts.is_empty():
		long_term_section.visible = false
	else:
		_populate_long_term_impacts(long_term_impacts)


func _populate_immediate_impacts(impacts: Array) -> void:
	"""Populate immediate impacts list"""
	# Clear existing
	for child in immediate_impacts_list.get_children():
		child.queue_free()

	if impacts.is_empty():
		var label := Label.new()
		label.text = "· 효과 없음"
		immediate_impacts_list.add_child(label)
		return

	# Add each impact
	for impact in impacts:
		var label := Label.new()
		label.text = "· " + _format_impact(impact)
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		immediate_impacts_list.add_child(label)


func _populate_long_term_impacts(impacts: Array) -> void:
	"""Populate long-term impacts list"""
	# Clear existing
	for child in long_term_impacts_list.get_children():
		child.queue_free()

	# Add each impact
	for impact_text in impacts:
		var label := Label.new()
		label.text = "· " + str(impact_text)
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		long_term_impacts_list.add_child(label)


func _format_impact(impact: Dictionary) -> String:
	"""Format impact dictionary to readable string"""
	var type: String = str(impact.get("type", ""))

	match type:
		"affection":
			var char_name: String = _get_character_name(str(impact.get("character", "")))
			var value: int = impact.get("value", 0)
			var from_val: int = impact.get("from", 0)
			var to_val: int = impact.get("to", 0)
			return "%s 호감도 %+d (%d → %d)" % [char_name, value, from_val, to_val]

		"trait":
			var trait_name: String = _get_trait_name(str(impact.get("trait_name", "")))
			var level: int = impact.get("level", 1)
			return '"%s Lv.%d" 획득' % [trait_name, level]

		"card":
			var card_id: String = str(impact.get("card_id", ""))
			if DeckManager:
				var card_data = DeckManager.load_card_from_json(card_id)
				if card_data and card_data is Dictionary:
					var rarity_str: String = str(card_data.get("rarity")) if card_data.has("rarity") else ""
					var char_name: String = (
						str(card_data.get("character_name")) if card_data.has("character_name") else ""
					)
					return "%s %s 카드 획득" % [rarity_str, char_name]
			return "%s 카드 획득" % card_id

		"ending":
			var ending_name: String = _get_ending_name(str(impact.get("ending_id", "")))
			var progress: int = impact.get("progress", 0)
			var total: int = impact.get("total", 3)
			return "%s 진입 (%d/%d)" % [ending_name, progress, total]

		"stat":
			var stat_name: String = str(impact.get("stat_name", ""))
			var value: int = impact.get("value", 0)
			return "%s %+d" % [stat_name, value]

		_:
			return "Unknown impact"


func _get_route_name(route_id: String) -> String:
	"""Get localized route name"""
	var names := {"rival": "라이벌", "friendship": "우정", "mentor": "멘토", "captain": "주장", "guardian": "수호신"}
	return names.get(route_id, route_id)


func _get_character_name(char_id: String) -> String:
	"""Get localized character name"""
	var names := {
		"kang_taeyang": "강태양", "park_minjun": "박민준", "coach_kim": "김철수", "lee_seojun": "이서준", "choi_jihoon": "최지훈"
	}
	return names.get(char_id, char_id)


func _get_trait_name(trait_id: String) -> String:
	"""Get localized trait name"""
	if PlayerData and PlayerData.has_method("get_exclusive_trait_name"):
		return PlayerData.get_exclusive_trait_name()

	var names := {
		"rival_awakening": "라이벌 각성",
		"team_chemistry": "팀 케미",
		"tactical_understanding": "전술 이해도",
		"leadership": "리더십",
		"iron_defense": "철벽 수비"
	}
	return names.get(trait_id, trait_id)


func _get_ending_name(ending_id: String) -> String:
	"""Get localized ending name"""
	var names := {
		"rival_true_ending": "라이벌 True Ending",
		"rival_alt_ending": "라이벌 Alternative Ending",
		"friendship_ending": "우정 Ending",
		"mentor_ending": "멘토 Ending",
		"captain_ending": "주장 Ending",
		"guardian_ending": "수호신 Ending"
	}
	return names.get(ending_id, ending_id)
