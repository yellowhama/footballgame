class_name CharacterCard
extends Resource

## CharacterCard.gd
## 파워프로 스타일 캐릭터 카드 리소스
## 작성일: 2025-10-24
## 버전: 1.0

# ============================================
# 기본 정보
# ============================================

## 캐릭터 고유 ID
@export var character_id: String = ""

## 캐릭터 이름
@export var character_name: String = ""

## 캐릭터 레어도 (N/R/SR/SSR)
@export_enum("N", "R", "SR", "SSR") var rarity: String = "N"

## 캐릭터 타입 (라이벌/친구/코치/기타)
@export_enum("rival", "friend", "coach", "captain", "gk", "other") var character_type: String = "other"

## 포지션 (FW/MF/DF/GK)
@export_enum("FW", "MF", "DF", "GK") var position: String = "FW"

# ============================================
# 훈련 보너스
# ============================================

## Technical 속성 보너스 (0.0 ~ 1.0)
@export_range(0.0, 1.0, 0.05) var technical_bonus: float = 0.0

## Physical 속성 보너스 (0.0 ~ 1.0)
@export_range(0.0, 1.0, 0.05) var physical_bonus: float = 0.0

## Mental 속성 보너스 (0.0 ~ 1.0)
@export_range(0.0, 1.0, 0.05) var mental_bonus: float = 0.0

# ============================================
# 이벤트
# ============================================

## 연관된 이벤트 ID 목록
@export var events: Array[String] = []

# ============================================
# 메서드
# ============================================


func get_event_count() -> int:
	"""연관 이벤트 개수 반환"""
	return events.size()


func get_rarity_stars() -> int:
	"""레어도에 따른 별 개수 반환"""
	match rarity:
		"N":
			return 1
		"R":
			return 2
		"SR":
			return 3
		"SSR":
			return 4
		_:
			return 1


func get_total_bonus() -> float:
	"""총 보너스 합계 반환"""
	return technical_bonus + physical_bonus + mental_bonus


func has_event(event_id: String) -> bool:
	"""특정 이벤트 보유 여부"""
	return event_id in events


func to_dict() -> Dictionary:
	"""Dictionary로 변환 (저장용)"""
	return {
		"character_id": character_id,
		"character_name": character_name,
		"rarity": rarity,
		"character_type": character_type,
		"position": position,
		"technical_bonus": technical_bonus,
		"physical_bonus": physical_bonus,
		"mental_bonus": mental_bonus,
		"events": events
	}


static func from_dict(data: Dictionary) -> CharacterCard:
	"""Dictionary에서 생성 (로드용)"""
	var script: Script = load("res://scripts/resources/CharacterCard.gd")
	var card: CharacterCard = script.new()
	card.character_id = data.get("character_id", "")
	card.character_name = data.get("character_name", "")
	card.rarity = data.get("rarity", "N")
	card.character_type = data.get("character_type", "other")
	card.position = data.get("position", "FW")
	card.technical_bonus = data.get("technical_bonus", 0.0)
	card.physical_bonus = data.get("physical_bonus", 0.0)
	card.mental_bonus = data.get("mental_bonus", 0.0)
	card.events = data.get("events", [])
	return card
