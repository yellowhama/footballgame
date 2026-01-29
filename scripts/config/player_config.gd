extends Node

const CONFIG_PATH = "user://player_config.cfg"
const LANGUAGE_SECTION = "language"
const MOBILE_SECTION = "mobile"
const PLAYER_SECTION = "player"

var config_file: ConfigFile


func _init():
	config_file = ConfigFile.new()
	load_config()


func load_config():
	var err = config_file.load(CONFIG_PATH)
	if err != OK:
		print("Config file not found, creating new one")
		save_config()


func save_config():
	config_file.save(CONFIG_PATH)


func has_language_set() -> bool:
	return config_file.has_section_key(LANGUAGE_SECTION, "current")


func get_language() -> String:
	return config_file.get_value(LANGUAGE_SECTION, "current", "en")


func set_language(lang: String):
	config_file.set_value(LANGUAGE_SECTION, "current", lang)
	save_config()
	TranslationServer.set_locale(lang)


func get_touch_sensitivity() -> float:
	return config_file.get_value(MOBILE_SECTION, "touch_sensitivity", 1.0)


func set_touch_sensitivity(value: float):
	config_file.set_value(MOBILE_SECTION, "touch_sensitivity", value)
	save_config()


func get_player_name() -> String:
	return config_file.get_value(PLAYER_SECTION, "name", "Player")


func set_player_name(player_name: String):
	config_file.set_value(PLAYER_SECTION, "name", player_name)
	save_config()
