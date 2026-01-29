extends Node2D
class_name FieldBackgroundManager
##
## FieldBackgroundManager - 경기장 배경 요소 관리
##
## 기능:
##   - isometric 잔디 타일 배치
##   - NES 관중석/광고판 배치
##   - Socceralia 골대 스프라이트 배치
##
## 스펙: docs/spec+@/spec_v4/dev_spec/UI/2025-12-07_SOCCERALIA_HORIZONTAL_VIEW_SPEC.md
##

#region Constants
const FIELD_LENGTH_PX: float = 1050.0
const FIELD_WIDTH_PX: float = 680.0

## 잔디 배경색 (선 없는 단색 - FieldLineDrawer가 선을 그림)
const GRASS_COLOR := Color(0.22, 0.45, 0.18)  ## 짙은 녹색

## 관중석/광고판 설정 (비활성화됨)
const CROWD_MARGIN: float = 60.0
const AD_BANNER_HEIGHT: float = 16.0

## NES 8x8 타일 크기
const NES_TILE_SIZE: int = 8
const NES_SCALE: float = 2.0

## NES 스프라이트 시트 경로
const NES_SPRITESHEET_PATH := "res://assets/sprites/nes/Football Full Sprite Sheet.png"
#endregion

#region Grass Background
var _grass_rect: ColorRect = null
#endregion

#region Goal Resources
var _goal_left_texture: Texture2D = null
var _goal_right_texture: Texture2D = null
#endregion

#region NES Assets
var _nes_spritesheet: Texture2D = null
var _crowd_top_texture: AtlasTexture = null
var _crowd_bottom_texture: AtlasTexture = null
var _ad_texture: AtlasTexture = null
#endregion

#region Node References
var _grass_container: Node2D = null
var _goals_container: Node2D = null
var _crowd_container: Node2D = null
#endregion


func _ready() -> void:
	_load_resources()
	_create_containers()
	_setup_grass_background()
	## _setup_crowd()  ## 관중석 비활성화
	_setup_goals()


func _load_resources() -> void:
	## 골대 텍스처 로드
	if ResourceLoader.exists("res://assets/sprites/socceralia/goal/goal-1.png"):
		_goal_left_texture = load("res://assets/sprites/socceralia/goal/goal-1.png")
	if ResourceLoader.exists("res://assets/sprites/socceralia/goal/goal-2.png"):
		_goal_right_texture = load("res://assets/sprites/socceralia/goal/goal-2.png")

	## NES 스프라이트 시트 로드 (관중석 비활성화로 주석 처리)
	## if ResourceLoader.exists(NES_SPRITESHEET_PATH):
	## 	_nes_spritesheet = load(NES_SPRITESHEET_PATH)
	## 	_crowd_top_texture = AtlasTexture.new()
	## 	_crowd_top_texture.atlas = _nes_spritesheet
	## 	_crowd_top_texture.region = CROWD_TOP_REGION
	## 	_crowd_bottom_texture = AtlasTexture.new()
	## 	_crowd_bottom_texture.atlas = _nes_spritesheet
	## 	_crowd_bottom_texture.region = CROWD_BOTTOM_REGION
	## 	_ad_texture = AtlasTexture.new()
	## 	_ad_texture.atlas = _nes_spritesheet
	## 	_ad_texture.region = AD_REGION


func _create_containers() -> void:
	## 잔디 컨테이너 (가장 아래)
	_grass_container = Node2D.new()
	_grass_container.name = "GrassContainer"
	_grass_container.z_index = -10
	add_child(_grass_container)

	## 관중석 컨테이너 (경기장 밖)
	_crowd_container = Node2D.new()
	_crowd_container.name = "CrowdContainer"
	_crowd_container.z_index = -5
	add_child(_crowd_container)

	## 골대 컨테이너
	_goals_container = Node2D.new()
	_goals_container.name = "GoalsContainer"
	_goals_container.z_index = 5  # 선수보다 위
	add_child(_goals_container)


func _setup_grass_background() -> void:
	## 단색 녹색 배경 (FieldLineDrawer가 선을 그림)
	var margin: float = 100.0

	_grass_rect = ColorRect.new()
	_grass_rect.name = "GrassBackground"
	_grass_rect.color = GRASS_COLOR
	_grass_rect.position = Vector2(-margin, -margin)
	_grass_rect.size = Vector2(FIELD_LENGTH_PX + margin * 2, FIELD_WIDTH_PX + margin * 2)
	_grass_container.add_child(_grass_rect)


func _setup_goals() -> void:
	## 골대 위치 (FieldLineDrawer 좌표에 맞춤)
	## goal-1: 64x120 스프라이트 (64=네트 깊이, 120=골대 높이/너비)
	## goal-2: 64x120 스프라이트
	## FIFA 골대 너비: 7.32m = 73.2px
	## 스케일: 73.2 / 120 ≈ 0.61
	## 골라인: x=0 (왼쪽), x=1050 (오른쪽)

	const GOAL_WIDTH_PX: float = 73.2  ## 7.32m * 10
	const SPRITE_HEIGHT: float = 120.0
	var goal_scale_factor: float = GOAL_WIDTH_PX / SPRITE_HEIGHT  ## ≈ 0.61
	var goal_scale := Vector2(goal_scale_factor, goal_scale_factor)

	## 스케일 적용 후 크기: 64*0.61 ≈ 39px (네트 깊이), 120*0.61 ≈ 73px (골대 너비)
	var scaled_net_depth: float = 64.0 * goal_scale_factor  ## ≈ 39px
	var scaled_goal_height: float = SPRITE_HEIGHT * goal_scale_factor  ## ≈ 73px

	## 왼쪽 골대 (홈) - 골 포스트가 x=0 골라인에 위치, 네트는 필드 밖으로
	## goal-1.png: 네트가 왼쪽, 골 포스트(입구)가 오른쪽에 있음
	if _goal_left_texture:
		var goal_left := Sprite2D.new()
		goal_left.name = "GoalLeft"
		goal_left.texture = _goal_left_texture
		goal_left.centered = false  ## 좌상단 기준으로 위치 지정
		## 오른쪽 가장자리(골 포스트)가 x=0에 위치하도록: left_edge = -scaled_net_depth
		## 수직 중앙 정렬: top = (필드높이 - 골대높이) / 2
		goal_left.position = Vector2(-scaled_net_depth, (FIELD_WIDTH_PX - scaled_goal_height) / 2.0)
		goal_left.scale = goal_scale
		goal_left.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
		_goals_container.add_child(goal_left)

	## 오른쪽 골대 (어웨이) - 골 포스트가 x=1050 골라인에 위치, 네트는 필드 밖으로
	## goal-2.png: 네트가 오른쪽, 골 포스트(입구)가 왼쪽에 있음
	if _goal_right_texture:
		var goal_right := Sprite2D.new()
		goal_right.name = "GoalRight"
		goal_right.texture = _goal_right_texture
		goal_right.centered = false  ## 좌상단 기준으로 위치 지정
		## 왼쪽 가장자리(골 포스트)가 x=1050에 위치하도록: left_edge = FIELD_LENGTH_PX
		goal_right.position = Vector2(FIELD_LENGTH_PX, (FIELD_WIDTH_PX - scaled_goal_height) / 2.0)
		goal_right.scale = goal_scale
		goal_right.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
		_goals_container.add_child(goal_right)


## NES 관중석/광고판 구현
func _setup_crowd() -> void:
	if not _nes_spritesheet:
		push_warning("NES spritesheet not loaded - skipping crowd setup")
		return

	## 경기장 경계 (관중석/광고판이 배치될 위치)
	var field_left := 0.0
	var field_right := FIELD_LENGTH_PX
	var field_top := 0.0
	var field_bottom := FIELD_WIDTH_PX

	## 스케일된 타일 크기
	var scaled_tile_height := NES_TILE_SIZE * NES_SCALE  ## 16px
	var scaled_ad_height := AD_BANNER_HEIGHT * NES_SCALE  ## 32px

	## 상단 관중석 (경기장 위쪽)
	_create_tiled_strip(
		_crowd_top_texture, Vector2(field_left, field_top - CROWD_MARGIN), field_right - field_left, CROWD_MARGIN
	)

	## 하단 관중석 (경기장 아래쪽)
	_create_tiled_strip(
		_crowd_bottom_texture, Vector2(field_left, field_bottom), field_right - field_left, CROWD_MARGIN
	)

	## 상단 광고판 (관중석과 경기장 사이)
	_create_tiled_strip(
		_ad_texture, Vector2(field_left, field_top - scaled_ad_height), field_right - field_left, scaled_ad_height
	)

	## 하단 광고판 (경기장과 관중석 사이)
	_create_tiled_strip(_ad_texture, Vector2(field_left, field_bottom), field_right - field_left, scaled_ad_height)


## 타일 패턴으로 스트립 생성
func _create_tiled_strip(texture: AtlasTexture, start_pos: Vector2, width: float, height: float) -> void:
	if not texture:
		return

	var tex_width := texture.region.size.x * NES_SCALE
	var tex_height := texture.region.size.y * NES_SCALE

	## 가로로 타일링
	var tiles_x := int(ceil(width / tex_width)) + 1
	var tiles_y := int(ceil(height / tex_height)) + 1

	for y in range(tiles_y):
		for x in range(tiles_x):
			var sprite := Sprite2D.new()
			sprite.texture = texture
			sprite.centered = false
			sprite.position = Vector2(start_pos.x + x * tex_width, start_pos.y + y * tex_height)
			sprite.scale = Vector2(NES_SCALE, NES_SCALE)
			sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
			_crowd_container.add_child(sprite)
