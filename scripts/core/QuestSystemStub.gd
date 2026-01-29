extends Node

signal quest_accepted(quest)
signal quest_completed(quest)
signal new_available_quest(quest)


class QuestPoolStub:
	var name: String

	func _init(pool_name: String) -> void:
		name = pool_name

	func add_quest(_quest) -> void:
		pass

	func remove_quest(_quest) -> void:
		pass

	func is_quest_inside(_quest) -> bool:
		return false

	func get_quest_from_id(_quest_id: int):
		return null

	func get_ids_from_quests() -> Array:
		return []

	func reset() -> void:
		pass


var available = QuestPoolStub.new("Available")
var active = QuestPoolStub.new("Active")
var completed = QuestPoolStub.new("Completed")


func _ready() -> void:
	print("[QuestSystemStub] Quest system disabled")


func start_quest(quest, _args: Dictionary = {}) -> void:
	quest_accepted.emit(quest)


func complete_quest(quest, _args: Dictionary = {}) -> void:
	quest_completed.emit(quest)


func mark_quest_as_available(quest) -> void:
	new_available_quest.emit(quest)
