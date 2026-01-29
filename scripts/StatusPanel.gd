## StatusPanel.gd
extends Control
class_name StatusPanel


func _ready() -> void:
	self.theme = preload("res://ui/ui_theme.tres")
	if has_node("Header"):
		var h = %Header
		if h.has_node("BtnBack"):
			h.get_node("BtnBack").icon = load("res://ui/icons/back.svg")
		if h.has_node("BtnSettings"):
			h.get_node("BtnSettings").icon = load("res://ui/icons/settings.svg")
	if has_node("QuickBar"):
		var qb: QuickBar = %QuickBar
		qb.apply_view_model({"autoEnabled": false, "currentSpeed": 1, "highlightLevel": 2, "visible": true})


func apply_view_model(vm: Dictionary) -> void:
	pass
