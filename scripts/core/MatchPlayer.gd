## MatchPlayer - Single player entity in match context
## Part of Game OS (MatchSetup Phase 17)
##
## Represents a player in a match with all necessary attributes.
## Created by MatchSetupBuilder from PlayerLibrary data.
##
## BOUNDARY CONTRACT:
## - This is the GDScript match-time representation
## - Converted to Rust Player via FootballRustEngine._convert_roster_for_rust()
## - See docs/spec/03_data_schemas.md "GDScript ↔ Rust Boundary Contract"
##
## RELATED:
## - Rust: crates/of_core/src/models/player.rs (engine simulation)
## - GDScript Save: autoload/core/GlobalCharacterData.gd (character creation)
##
## Priority: P0 (File 1 of 7)
class_name MatchPlayer
extends RefCounted

## Player Identification
var uid: String = ""  # Format: "csv:1234" or "grad:1234567890_0001"
var name: String = "Player"
var jersey_number: int = 0

## Position & Role
var position: String = "MF"  # GK, DF, MF, FW
var preferred_position: String = "MF"  # Original/best position

## Core Attributes (0-100 scale)
var overall: int = 50
var technical: int = 50
var mental: int = 50
var physical: int = 50

## Detailed Attributes (optional - for graduated players)
var pace: int = 50
var shooting: int = 50
var passing: int = 50
var dribbling: int = 50
var defending: int = 50
var goalkeeping: int = 50

## Match State
var condition: int = 100  # 0-100 (stamina/fitness during match)
var morale: int = 50  # 0-100

## Traits (optional)
var traits: Array = []  # Array of trait names/IDs

## Personality (optional)
var personality: Dictionary = {}


## Create MatchPlayer from PlayerLibrary data
static func from_player_data(data: Dictionary):  # Returns MatchPlayer (self-reference workaround)
	var _Self = preload("res://scripts/core/MatchPlayer.gd")
	var player = _Self.new()

	# Identification
	player.uid = str(data.get("uid", "csv:0"))
	player.name = str(data.get("name", "Player"))
	player.jersey_number = int(data.get("jersey_number", 0))

	# Position
	player.position = str(data.get("position", "MF"))
	player.preferred_position = str(data.get("preferred_position", player.position))

	# Core attributes
	player.overall = int(data.get("overall", 50))
	player.technical = int(data.get("technical", player.overall))
	player.mental = int(data.get("mental", player.overall))
	player.physical = int(data.get("physical", player.overall))

	# Detailed attributes (optional)
	if data.has("pace"):
		player.pace = int(data.get("pace", 50))
	if data.has("shooting"):
		player.shooting = int(data.get("shooting", 50))
	if data.has("passing"):
		player.passing = int(data.get("passing", 50))
	if data.has("dribbling"):
		player.dribbling = int(data.get("dribbling", 50))
	if data.has("defending"):
		player.defending = int(data.get("defending", 50))
	if data.has("goalkeeping"):
		player.goalkeeping = int(data.get("goalkeeping", 50))

	# Match state
	player.condition = int(data.get("condition", 100))
	player.morale = int(data.get("morale", 50))

	# Traits (optional)
	if data.has("traits"):
		player.traits = data.get("traits", []).duplicate()

	# Personality (optional)
	if data.has("personality"):
		player.personality = data.get("personality", {}).duplicate()

	return player


## Create minimal MatchPlayer from CSV ID (for stub PlayerLibrary)
static func from_csv_id(csv_id: int, jersey_num: int = 0):  # Returns MatchPlayer (self-reference workaround)
	var _Self = preload("res://scripts/core/MatchPlayer.gd")
	var player = _Self.new()
	player.uid = "csv:%d" % csv_id
	player.name = "Player %d" % csv_id
	player.jersey_number = jersey_num
	player.position = "MF"
	player.overall = 50 + (csv_id % 30)  # Vary between 50-80
	player.technical = player.overall
	player.mental = player.overall
	player.physical = player.overall
	return player


## Export to dictionary (for engine JSON)
func to_dict() -> Dictionary:
	var result = {
		"uid": uid,
		"name": name,
		"jersey_number": jersey_number,
		"position": position,
		"preferred_position": preferred_position,
		"overall": overall,
		"technical": technical,
		"mental": mental,
		"physical": physical,
		"condition": condition,
		"morale": morale
	}

	# Add detailed attributes if present
	if pace != 50:
		result["pace"] = pace
	if shooting != 50:
		result["shooting"] = shooting
	if passing != 50:
		result["passing"] = passing
	if dribbling != 50:
		result["dribbling"] = dribbling
	if defending != 50:
		result["defending"] = defending
	if goalkeeping != 50:
		result["goalkeeping"] = goalkeeping

	# Add traits if present
	if not traits.is_empty():
		result["traits"] = traits.duplicate()

	# Add personality if present
	if not personality.is_empty():
		result["personality"] = personality.duplicate()

	return result


## Get position penalty (for playing out of position)
## Returns 0.0 (perfect) to 1.0 (max penalty)
func get_position_penalty() -> float:
	if position == preferred_position:
		return 0.0

	# Simple penalty: Different position = 20% penalty
	# TODO: Implement detailed position compatibility matrix
	return 0.2
