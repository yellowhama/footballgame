extends PanelContainer
class_name TopBar

@onready var date_label: Label = $HBox/DateLabel
@onready var stamina_bar: ProgressBar = $HBox/StaminaBar

func update_info(date_str: String, stamina_pct: float) -> void:
	if date_label:
		date_label.text = date_str
	if stamina_bar:
		stamina_bar.value = stamina_pct
