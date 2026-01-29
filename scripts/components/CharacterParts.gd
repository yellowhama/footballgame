extends Node
# 캐릭터 파츠 데이터 관리 시스템

## 머리 파츠 옵션
var head_parts = {
	"hair_styles":
	[
		{"id": "short", "name": "짧은 머리", "icon": "💇", "color_changeable": true},
		{"id": "medium", "name": "중간 머리", "icon": "💇", "color_changeable": true},
		{"id": "long", "name": "긴 머리", "icon": "💇", "color_changeable": true},
		{"id": "mohawk", "name": "모히칸", "icon": "🎸", "color_changeable": true},
		{"id": "bald", "name": "스킨헤드", "icon": "🥚", "color_changeable": false},
		{"id": "afro", "name": "아프로", "icon": "🦱", "color_changeable": true},
		{"id": "spiky", "name": "스파이크", "icon": "⚡", "color_changeable": true},
		{"id": "ponytail", "name": "포니테일", "icon": "🎀", "color_changeable": true}
	],
	"hair_colors":
	[
		{"name": "검정", "color": Color(0.1, 0.1, 0.1)},
		{"name": "갈색", "color": Color(0.4, 0.3, 0.2)},
		{"name": "금발", "color": Color(0.9, 0.8, 0.4)},
		{"name": "빨강", "color": Color(0.8, 0.2, 0.2)},
		{"name": "파랑", "color": Color(0.2, 0.2, 0.8)},
		{"name": "초록", "color": Color(0.2, 0.8, 0.2)},
		{"name": "보라", "color": Color(0.6, 0.2, 0.8)},
		{"name": "회색", "color": Color(0.6, 0.6, 0.6)}
	],
	"face_types":
	[
		{"id": "normal", "name": "기본형", "icon": "😊"},
		{"id": "round", "name": "둥근형", "icon": "🙂"},
		{"id": "sharp", "name": "날카로운형", "icon": "😎"},
		{"id": "cute", "name": "귀여운형", "icon": "😄"},
		{"id": "tough", "name": "강인한형", "icon": "😤"}
	]
}

## 상체 파츠 옵션
var body_parts = {
	"uniforms":
	[
		{"id": "basic", "name": "기본 유니폼", "icon": "👕", "number": true},
		{"id": "striped", "name": "줄무늬", "icon": "🦓", "number": true},
		{"id": "vneck", "name": "V넥", "icon": "✌️", "number": true},
		{"id": "long_sleeve", "name": "긴팔", "icon": "🧥", "number": true},
		{"id": "sleeveless", "name": "민소매", "icon": "💪", "number": true},
		{"id": "retro", "name": "레트로", "icon": "📻", "number": true},
		{"id": "modern", "name": "모던", "icon": "✨", "number": true},
		{"id": "training", "name": "훈련복", "icon": "🏃", "number": false}
	],
	"uniform_colors":
	[
		{"name": "빨강", "primary": Color(0.9, 0.1, 0.1), "secondary": Color.WHITE},
		{"name": "파랑", "primary": Color(0.1, 0.1, 0.9), "secondary": Color.WHITE},
		{"name": "노랑", "primary": Color(0.9, 0.9, 0.1), "secondary": Color.BLACK},
		{"name": "초록", "primary": Color(0.1, 0.7, 0.1), "secondary": Color.WHITE},
		{"name": "검정", "primary": Color(0.1, 0.1, 0.1), "secondary": Color.WHITE},
		{"name": "흰색", "primary": Color.WHITE, "secondary": Color.BLACK},
		{"name": "주황", "primary": Color(0.9, 0.5, 0.1), "secondary": Color.WHITE},
		{"name": "보라", "primary": Color(0.5, 0.1, 0.7), "secondary": Color.WHITE}
	],
	"body_types":
	[
		{"id": "slim", "name": "슬림", "icon": "🏃"},
		{"id": "normal", "name": "보통", "icon": "🚶"},
		{"id": "athletic", "name": "근육질", "icon": "💪"},
		{"id": "bulky", "name": "건장한", "icon": "🏋️"}
	]
}

## 하체 파츠 옵션
var leg_parts = {
	"shorts":
	[
		{"id": "basic", "name": "기본 반바지", "icon": "🩳"},
		{"id": "long", "name": "긴 반바지", "icon": "👖"},
		{"id": "short", "name": "짧은 반바지", "icon": "🩲"},
		{"id": "baggy", "name": "헐렁한", "icon": "🎭"},
		{"id": "tight", "name": "타이트한", "icon": "🏃"}
	],
	"socks":
	[
		{"id": "crew", "name": "무릎 양말", "icon": "🧦", "height": "knee"},
		{"id": "ankle", "name": "발목 양말", "icon": "👟", "height": "ankle"},
		{"id": "long", "name": "긴 양말", "icon": "🦵", "height": "thigh"},
		{"id": "none", "name": "양말 없음", "icon": "🦶", "height": "none"}
	],
	"shoes":
	[
		{"id": "cleats", "name": "축구화", "icon": "⚽", "color_changeable": true},
		{"id": "indoor", "name": "실내화", "icon": "👟", "color_changeable": true},
		{"id": "classic", "name": "클래식", "icon": "👞", "color_changeable": true},
		{"id": "modern", "name": "모던", "icon": "✨", "color_changeable": true},
		{"id": "speed", "name": "스피드", "icon": "⚡", "color_changeable": true}
	],
	"shoe_colors":
	[
		{"name": "검정", "color": Color.BLACK},
		{"name": "흰색", "color": Color.WHITE},
		{"name": "빨강", "color": Color(0.9, 0.1, 0.1)},
		{"name": "파랑", "color": Color(0.1, 0.1, 0.9)},
		{"name": "노랑", "color": Color(0.9, 0.9, 0.1)},
		{"name": "주황", "color": Color(0.9, 0.5, 0.1)},
		{"name": "형광", "color": Color(0.1, 1, 0.4)}
	]
}

## 현재 선택된 파츠
var current_selection = {
	"head": {"hair_style": 0, "hair_color": 0, "face_type": 0},
	"body": {"uniform": 0, "uniform_color": 0, "body_type": 0, "number": 7},  # 등번호
	"legs": {"shorts": 0, "socks": 0, "shoes": 0, "shoe_color": 0}
}


## 파츠 변경 함수
func change_part(category: String, part_type: String, direction: int):
	if not current_selection.has(category):
		return

	var part_data = get_part_data(category, part_type)
	if not part_data:
		return

	var current = current_selection[category][part_type]
	var max_index = part_data.size() - 1

	current += direction
	if current < 0:
		current = max_index
	elif current > max_index:
		current = 0

	current_selection[category][part_type] = current
	return get_current_part_info(category, part_type)


func get_part_data(category: String, part_type: String) -> Array:
	match category:
		"head":
			match part_type:
				"hair_style":
					return head_parts.hair_styles
				"hair_color":
					return head_parts.hair_colors
				"face_type":
					return head_parts.face_types
		"body":
			match part_type:
				"uniform":
					return body_parts.uniforms
				"uniform_color":
					return body_parts.uniform_colors
				"body_type":
					return body_parts.body_types
		"legs":
			match part_type:
				"shorts":
					return leg_parts.shorts
				"socks":
					return leg_parts.socks
				"shoes":
					return leg_parts.shoes
				"shoe_color":
					return leg_parts.shoe_colors
	return []


func get_current_part_info(category: String, part_type: String) -> Dictionary:
	var part_data = get_part_data(category, part_type)
	if part_data.is_empty():
		return {}

	var index = current_selection[category][part_type]
	if index >= 0 and index < part_data.size():
		return part_data[index]
	return {}


func get_character_data() -> Dictionary:
	# 최종 캐릭터 데이터 반환
	return {"appearance": current_selection.duplicate(true), "created_at": Time.get_unix_time_from_system()}


func randomize_character():
	# 랜덤 캐릭터 생성
	current_selection.head.hair_style = randi() % head_parts.hair_styles.size()
	current_selection.head.hair_color = randi() % head_parts.hair_colors.size()
	current_selection.head.face_type = randi() % head_parts.face_types.size()

	current_selection.body.uniform = randi() % body_parts.uniforms.size()
	current_selection.body.uniform_color = randi() % body_parts.uniform_colors.size()
	current_selection.body.body_type = randi() % body_parts.body_types.size()
	current_selection.body.number = randi_range(1, 99)

	current_selection.legs.shorts = randi() % leg_parts.shorts.size()
	current_selection.legs.socks = randi() % leg_parts.socks.size()
	current_selection.legs.shoes = randi() % leg_parts.shoes.size()
	current_selection.legs.shoe_color = randi() % leg_parts.shoe_colors.size()
