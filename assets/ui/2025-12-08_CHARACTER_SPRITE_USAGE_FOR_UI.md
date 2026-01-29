# 2025-12-08 - 캐릭터 스프라이트 UI 활용 스펙

> 목적: **Socceralia 스프라이트**를 게임 전체에서 통일하여 사용 (캐릭터 생성, 마이팀 설정, 경기 뷰어)
> 작성일: 2025-12-08
> 참조: `docs/spec+@/spec_v4/dev_spec/UI/2025-12-07_SOCCERALIA_HORIZONTAL_VIEW_SPEC.md`
> 통합 문서: `docs/spec+@/spec_v4/dev_spec/UI/2025-12-08_CHARACTER_SPRITE_INTEGRATION_SPEC.md`
>
> **⚠️ 중요 결정 (2025-12-08)**:
> - 게임 전체에서 **Socceralia 16x16 스프라이트만 사용**
> - 고해상도 파츠 시스템 (SkeletonCharacter) 폐기
> - NES 스프라이트 스펙 미채택

---

## 1. 목표

### 1.1 원하는 것

**캐릭터 생성 화면:**
- 내 선수 캐릭터 1명이 화면에 보임
- 애니메이션 재생 (뛰기, 공 차기, 대기 등)
- 헤어 스타일/헤어 색상 선택 시 **즉시 반영**
- 피부색 선택 시 즉시 반영
- 유니폼 색상도 미리보기 가능

**마이팀 설정 화면:**
- 배경에 팀 선수들 여러 명이 돌아다님 (걷기, 뛰기, 공 차기)
- 팀 유니폼 색상 변경 시 **모든 선수 색상 즉시 변경**
- 활기찬 느낌의 배경 역할

### 1.2 핵심 아이디어

```
┌──────────────────────────────────────────────┐
│          [캐릭터 생성 화면]                    │
│                                              │
│    ┌──────────┐     ┌────────────────┐       │
│    │          │     │ 이름: [_____]  │       │
│    │  🏃 ←    │     │ 헤어: [▼ 금발] │       │
│    │ 선수     │     │ 피부: [▼ 밝음] │       │
│    │ 미리보기 │     │ 포지션: [▼ FW]│       │
│    │          │     └────────────────┘       │
│    └──────────┘                              │
│                                              │
│              [다음] [이전]                    │
└──────────────────────────────────────────────┘

┌──────────────────────────────────────────────┐
│          [마이팀 설정 화면]                    │
│                                              │
│  ┌────────────────────────────────────────┐  │
│  │   🏃  🧍  ⚽🏃   🧍      🏃           │  │
│  │     ← 팀 선수들 애니메이션 배경 →      │  │
│  │   🏃       🧍  🏃   ⚽              │  │
│  └────────────────────────────────────────┘  │
│                                              │
│  팀 이름: [FC 서울_______]                   │
│  메인 컬러: [🔴 빨강 ▼]                       │
│  서브 컬러: [⚪ 흰색 ▼]                       │
│  패턴: [세로줄 ▼]                            │
│                                              │
│              [저장] [취소]                    │
└──────────────────────────────────────────────┘
```

---

## 2. 사용할 아셋

### 2.1 경기 뷰어와 동일한 아셋 사용

| 아셋 | 경로 | 용도 |
|------|------|------|
| **Socceralia 선수** | `assets/sprites/socceralia/player/` | 메인 캐릭터 |
| **8x8 Mini Pack 1** | `assets/sprites/socceralia/mini-pack-1/` | 추가 바리에이션 |
| **8x8 Mini Pack 2** | `assets/sprites/socceralia/mini-pack-2/` | 추가 바리에이션 |

### 2.2 Socceralia 선수 스프라이트 상세

**폴더 구조:**
```
assets/sprites/socceralia/player/
├── black/          # 검은 머리
│   ├── player-black-1.png   # 대기 포즈
│   ├── player-black-2.png   # 걷기 1
│   ├── player-black-3.png   # 걷기 2
│   ├── ...
│   └── player-black-19.png
├── blonde/         # 금발
├── redhead/        # 빨간 머리
├── gk/             # 골키퍼 (다른 유니폼)
└── other/          # 기타
```

**프레임 매핑:**
| 프레임 | 동작 | UI 활용 |
|--------|------|---------|
| 1 | 대기 (Idle) | 기본 미리보기 |
| 2~5 | 달리기 (Run) | 배경 애니메이션 |
| 10 | 킥 (Kick) | 공 차는 동작 |
| 14 | 태클 (Tackle) | - |
| 17 | 세레머니 | 선택 완료 시 |

**사이즈:**
- 원본: 16×16 px
- UI에서 사용: 2x 스케일 (32×32) 또는 4x 스케일 (64×64)

### 2.3 헤어 스타일 옵션

| ID | 폴더명 | 설명 | 미리보기용 |
|----|--------|------|-----------|
| `black` | `black/` | 검은 머리 | ✅ |
| `blonde` | `blonde/` | 금발 | ✅ |
| `redhead` | `redhead/` | 빨간 머리 | ✅ |
| `gk` | `gk/` | 골키퍼 스타일 | GK 전용 |
| `other` | `other/` | 기타 | ✅ |

---

## 3. 구현 컴포넌트

### 3.1 CharacterPreviewSprite (캐릭터 미리보기)

**역할:** 단일 캐릭터를 애니메이션과 함께 표시

**위치:** `scripts/ui/components/CharacterPreviewSprite.gd`

```gdscript
class_name CharacterPreviewSprite
extends Node2D

signal appearance_changed

## 외모 설정
@export var hair_style: String = "black":
    set(value):
        hair_style = value
        _update_appearance()

@export var skin_tone: int = 0:  # 0=밝음, 1=중간, 2=어두움
    set(value):
        skin_tone = value
        _update_appearance()

## 팀 컬러
@export var primary_color: Color = Color.RED:
    set(value):
        primary_color = value
        _apply_team_color()

@export var secondary_color: Color = Color.WHITE:
    set(value):
        secondary_color = value
        _apply_team_color()

## 애니메이션 상태
enum AnimState { IDLE, RUN, KICK, CELEBRATE }
var current_anim: AnimState = AnimState.IDLE

## 내부 노드
@onready var sprite: AnimatedSprite2D = $AnimatedSprite2D
@onready var shadow: Sprite2D = $Shadow

## 스케일 (UI용)
const UI_SCALE := Vector2(4.0, 4.0)  # 16px → 64px

func _ready() -> void:
    sprite.scale = UI_SCALE
    shadow.scale = UI_SCALE
    _setup_animations()
    _update_appearance()
    play_animation(AnimState.IDLE)


func _setup_animations() -> void:
    ## SpriteFrames 동적 생성
    var frames := SpriteFrames.new()

    # Idle (프레임 1)
    frames.add_animation("idle")
    frames.set_animation_speed("idle", 1)
    frames.set_animation_loop("idle", true)

    # Run (프레임 2~5)
    frames.add_animation("run")
    frames.set_animation_speed("run", 8)
    frames.set_animation_loop("run", true)

    # Kick (프레임 10)
    frames.add_animation("kick")
    frames.set_animation_speed("kick", 6)
    frames.set_animation_loop("kick", false)

    # Celebrate (프레임 17)
    frames.add_animation("celebrate")
    frames.set_animation_speed("celebrate", 4)
    frames.set_animation_loop("celebrate", true)

    sprite.sprite_frames = frames
    _load_textures_for_hair_style()


func _load_textures_for_hair_style() -> void:
    var frames := sprite.sprite_frames
    var base_path := "res://assets/sprites/socceralia/player/%s/player-%s-" % [hair_style, hair_style]

    # Idle
    frames.clear("idle")
    frames.add_frame("idle", _load_texture(base_path + "1.png"))

    # Run
    frames.clear("run")
    for i in [2, 3, 4, 5]:
        var tex := _load_texture(base_path + "%d.png" % i)
        if tex:
            frames.add_frame("run", tex)

    # Kick
    frames.clear("kick")
    frames.add_frame("kick", _load_texture(base_path + "10.png"))

    # Celebrate
    frames.clear("celebrate")
    frames.add_frame("celebrate", _load_texture(base_path + "17.png"))


func _load_texture(path: String) -> Texture2D:
    if ResourceLoader.exists(path):
        return load(path)
    return null


func _update_appearance() -> void:
    _load_textures_for_hair_style()
    _apply_team_color()
    appearance_changed.emit()


func _apply_team_color() -> void:
    ## 팀 컬러 셰이더 적용
    var mat := ShaderMaterial.new()
    mat.shader = preload("res://assets/shaders/KitPattern.gdshader")
    mat.set_shader_parameter("primary_color", primary_color)
    mat.set_shader_parameter("secondary_color", secondary_color)
    mat.set_shader_parameter("pattern_type", 0)  # 단색
    mat.set_shader_parameter("key_color", Color.WHITE)
    mat.set_shader_parameter("tolerance", 0.15)
    sprite.material = mat


func play_animation(state: AnimState) -> void:
    current_anim = state
    match state:
        AnimState.IDLE:
            sprite.play("idle")
        AnimState.RUN:
            sprite.play("run")
        AnimState.KICK:
            sprite.play("kick")
        AnimState.CELEBRATE:
            sprite.play("celebrate")


## 외부 API
func set_hair_style(style: String) -> void:
    hair_style = style

func set_team_colors(primary: Color, secondary: Color) -> void:
    primary_color = primary
    secondary_color = secondary
```

### 3.2 TeamPreviewBackground (팀 배경 애니메이션)

**역할:** 여러 선수가 돌아다니는 배경

**위치:** `scripts/ui/components/TeamPreviewBackground.gd`

```gdscript
class_name TeamPreviewBackground
extends Control

## 표시할 선수 수
@export var player_count: int = 8

## 팀 컬러
@export var primary_color: Color = Color.RED:
    set(value):
        primary_color = value
        _update_all_players_color()

@export var secondary_color: Color = Color.WHITE:
    set(value):
        secondary_color = value
        _update_all_players_color()

@export var pattern_type: int = 0:  # 0=단색, 1=가로줄, 2=세로줄
    set(value):
        pattern_type = value
        _update_all_players_color()

## 내부
var _players: Array[Node2D] = []
var _ball: Sprite2D = null

const HAIR_STYLES := ["black", "blonde", "redhead", "other"]


func _ready() -> void:
    _spawn_players()
    _spawn_ball()


func _spawn_players() -> void:
    for i in range(player_count):
        var player := _create_player(i)
        add_child(player)
        _players.append(player)
        _start_random_movement(player)


func _create_player(index: int) -> Node2D:
    ## CharacterPreviewSprite 또는 간단한 AnimatedSprite2D 사용
    var player := preload("res://scenes/ui/CharacterPreviewSprite.tscn").instantiate()

    # 랜덤 헤어 스타일
    player.hair_style = HAIR_STYLES[index % HAIR_STYLES.size()]

    # 팀 컬러
    player.primary_color = primary_color
    player.secondary_color = secondary_color

    # 랜덤 시작 위치
    player.position = Vector2(
        randf_range(50, size.x - 50),
        randf_range(50, size.y - 50)
    )

    # 스케일 (배경용이라 작게)
    player.scale = Vector2(0.5, 0.5)

    return player


func _spawn_ball() -> void:
    _ball = Sprite2D.new()
    _ball.texture = preload("res://assets/socceralia/ball-idle.png")
    _ball.scale = Vector2(2.0, 2.0)
    _ball.position = size / 2
    add_child(_ball)


func _start_random_movement(player: Node2D) -> void:
    ## 랜덤하게 움직이는 Tween 생성
    _move_to_random_target(player)


func _move_to_random_target(player: Node2D) -> void:
    var target := Vector2(
        randf_range(30, size.x - 30),
        randf_range(30, size.y - 30)
    )

    var distance := player.position.distance_to(target)
    var duration := distance / 50.0  # 속도

    # 방향에 따라 flip
    player.get_node("AnimatedSprite2D").flip_h = target.x < player.position.x

    # 달리기 애니메이션
    player.play_animation(CharacterPreviewSprite.AnimState.RUN)

    var tween := create_tween()
    tween.tween_property(player, "position", target, duration)
    tween.tween_callback(func():
        # 도착 후 잠시 대기
        player.play_animation(CharacterPreviewSprite.AnimState.IDLE)
        await get_tree().create_timer(randf_range(1.0, 3.0)).timeout
        _move_to_random_target(player)
    )


func _update_all_players_color() -> void:
    for player in _players:
        if player.has_method("set_team_colors"):
            player.set_team_colors(primary_color, secondary_color)
        if player.has_method("set_pattern_type"):
            player.set_pattern_type(pattern_type)


## 외부 API
func set_team_colors(primary: Color, secondary: Color, pattern: int = 0) -> void:
    primary_color = primary
    secondary_color = secondary
    pattern_type = pattern
```

---

## 4. UI 통합

### 4.1 캐릭터 생성 화면 통합

**파일:** `scripts/screens/CharacterCreationController.gd`

```gdscript
## 기존 코드에 추가

@onready var character_preview: CharacterPreviewSprite = $CharacterPreview

func _on_hair_style_selected(style: String) -> void:
    character_preview.set_hair_style(style)

func _on_skin_tone_selected(tone: int) -> void:
    character_preview.skin_tone = tone

func _on_confirm_pressed() -> void:
    ## 선택 완료 애니메이션
    character_preview.play_animation(CharacterPreviewSprite.AnimState.CELEBRATE)
    await get_tree().create_timer(1.5).timeout
    _proceed_to_next_step()
```

### 4.2 마이팀 설정 화면 통합

**파일:** `scripts/screens/MyTeamSetupScreen.gd`

```gdscript
## 기존 코드에 추가

@onready var team_background: TeamPreviewBackground = $TeamPreviewBackground

func _on_primary_color_selected(color: Color) -> void:
    team_background.primary_color = color

func _on_secondary_color_selected(color: Color) -> void:
    team_background.secondary_color = color

func _on_pattern_selected(pattern_id: int) -> void:
    team_background.pattern_type = pattern_id
```

---

## 5. 씬 구조

### 5.1 CharacterPreviewSprite.tscn

```
CharacterPreviewSprite (Node2D)
├── Shadow (Sprite2D)
│   - texture: ball_shadow.png
│   - modulate: (0,0,0,0.3)
│   - position: (2, 4)
└── AnimatedSprite2D
    - sprite_frames: (동적 생성)
    - texture_filter: Nearest
```

### 5.2 TeamPreviewBackground.tscn

```
TeamPreviewBackground (Control)
├── ColorRect (배경색, 옵션)
└── (동적으로 CharacterPreviewSprite 인스턴스들 추가)
```

### 5.3 캐릭터 생성 화면 구조

```
CharacterCreationScreen (Control, 1080x1920)
├── VBoxContainer
│   ├── HeaderPanel (팀 이름/단계 표시)
│   │
│   ├── PreviewContainer (고정 높이 ~400px)
│   │   └── CharacterPreviewSprite (중앙 배치)
│   │
│   ├── OptionsPanel (스크롤 가능)
│   │   ├── HairStyleSelector
│   │   │   └── HBoxContainer [black] [blonde] [redhead] ...
│   │   ├── SkinToneSelector
│   │   │   └── HBoxContainer [밝음] [중간] [어두움]
│   │   └── PositionSelector
│   │       └── GridContainer [GK] [DF] [MF] [FW]
│   │
│   └── BottomButtons
│       └── HBoxContainer [이전] [다음]
```

### 5.4 마이팀 설정 화면 구조

```
MyTeamSetupScreen (Control, 1080x1920)
├── TeamPreviewBackground (배경, 전체 크기의 상단 40%)
│   └── (선수들 애니메이션)
│
├── SetupPanel (하단 60%)
│   ├── TeamNameInput
│   ├── ColorPickerPrimary
│   ├── ColorPickerSecondary
│   ├── PatternSelector
│   └── SaveButton
```

---

## 6. 색상 선택 UI

### 6.1 프리셋 색상

```gdscript
const COLOR_PRESETS := [
    # 빨강 계열
    Color("#FF0000"),  # 빨강
    Color("#8B0000"),  # 다크 레드
    Color("#DC143C"),  # 크림슨

    # 파랑 계열
    Color("#0000FF"),  # 파랑
    Color("#000080"),  # 네이비
    Color("#4169E1"),  # 로열 블루

    # 초록 계열
    Color("#008000"),  # 초록
    Color("#006400"),  # 다크 그린

    # 노랑/주황 계열
    Color("#FFD700"),  # 골드
    Color("#FFA500"),  # 오렌지

    # 흑백
    Color("#FFFFFF"),  # 흰색
    Color("#000000"),  # 검정
    Color("#808080"),  # 회색

    # 기타
    Color("#800080"),  # 퍼플
    Color("#FFC0CB"),  # 핑크
    Color("#00FFFF"),  # 시안
]
```

### 6.2 패턴 옵션

```gdscript
enum PatternType {
    SOLID = 0,      # 단색
    HOOPS = 1,      # 가로줄 (Celtic 스타일)
    STRIPES = 2,    # 세로줄 (AC Milan 스타일)
    CHECKER = 3,    # 체크 (Croatia 스타일)
}

const PATTERN_NAMES := {
    PatternType.SOLID: "단색",
    PatternType.HOOPS: "가로줄",
    PatternType.STRIPES: "세로줄",
    PatternType.CHECKER: "체크",
}
```

---

## 7. 구현 체크리스트

### Phase 1: 컴포넌트 생성
- [ ] `CharacterPreviewSprite.gd` 작성
- [ ] `CharacterPreviewSprite.tscn` 생성
- [ ] `TeamPreviewBackground.gd` 작성
- [ ] `TeamPreviewBackground.tscn` 생성

### Phase 2: 캐릭터 생성 통합
- [ ] `CharacterCreationScreen.tscn`에 PreviewContainer 추가
- [ ] 헤어 스타일 선택 → 미리보기 연동
- [ ] 피부색 선택 → 미리보기 연동
- [ ] 확인 시 세레머니 애니메이션

### Phase 3: 마이팀 설정 통합
- [ ] `MyTeamSetupScreen.tscn`에 TeamPreviewBackground 추가
- [ ] 메인 컬러 선택 → 배경 선수 색상 변경
- [ ] 서브 컬러 선택 → 배경 선수 색상 변경
- [ ] 패턴 선택 → 배경 선수 패턴 변경

### Phase 4: 데이터 저장 연동
- [ ] 선택한 외모 → PlayerData에 저장
- [ ] 선택한 팀 컬러 → TeamData에 저장
- [ ] 경기 뷰어에서 저장된 데이터 로드하여 적용

---

## 8. 기존 스크립트와의 연동

### 8.1 TeamColorManager.gd 재사용

`scripts/replay/horizontal/TeamColorManager.gd`에 이미 팀 컬러 프리셋과 셰이더 적용 로직이 있음.

```gdscript
## UI에서도 동일하게 사용
TeamColorManager.apply_team_color_to_player(character_preview, "korea")
```

### 8.2 SoccerPlayer.gd 참조

`scripts/replay/horizontal/SoccerPlayer.gd`의 구조를 UI용으로 단순화:

| SoccerPlayer (경기용) | CharacterPreviewSprite (UI용) |
|----------------------|------------------------------|
| 좌표 변환 로직 | 불필요 |
| 팀 ID, 배번 | 불필요 |
| 액션 기반 프레임 선택 | 단순 애니메이션만 |
| 스무딩 이동 | 간단한 Tween |

---

## 9. 예상 결과

### 캐릭터 생성 화면
- 화면 중앙에 내 선수 캐릭터가 크게 보임 (64x64)
- 기본 대기 포즈로 서있음
- 헤어 스타일 버튼 클릭 → 즉시 머리 색상/스타일 변경
- "다음" 버튼 클릭 → 세레머니 동작 후 다음 단계

### 마이팀 설정 화면
- 상단 40%에 선수 8명이 무작위로 걷거나 뜀
- 메인 컬러 선택 → 모든 선수 유니폼 즉시 변경
- 패턴 선택 → 줄무늬/체크 등 즉시 반영
- 활기차고 동적인 느낌의 설정 화면

---

## 10. 화면 구성 상세 플랜

### 10.1 잔디밭 배경

**사용할 아셋:** isometric-nature-pack 잔디 타일

```
assets/sprites/grass/
├── grass8.png  # 메인 타일 (깔끔한 잔디) - 70%
├── grass1.png  # 풀잎 많음 - 변화용 10%
├── grass7.png  # 깔끔한 잔디 - 변화용 5%
└── ...
```

**구현 방식:**

| 방식 | 설명 | 선택 |
|------|------|------|
| **A. TileMap** | 타일맵으로 잔디 패턴 배치 | ❌ 과함 |
| **B. 단일 이미지 타일링** | grass8.png를 TextureRect로 반복 | ✅ 추천 |
| **C. ColorRect + 셰이더** | 초록색 + 노이즈 셰이더 | 간단한 대안 |

**추천: 방식 B (TextureRect 타일링)**

```gdscript
## GrassBackground.gd
extends TextureRect

func _ready() -> void:
    texture = preload("res://assets/sprites/grass/grass8.png")
    stretch_mode = TextureRect.STRETCH_TILE
    texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
    
    # 타일 스케일 (2x 확대)
    texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
```

**씬 구조:**
```
GrassBackground (TextureRect)
- texture: grass8.png
- stretch_mode: STRETCH_TILE
- texture_filter: NEAREST
- custom_minimum_size: (1080, 600)
```

---

### 10.2 캐릭터 생성 화면 레이아웃 (1080x1920)

```
┌─────────────────────────────────────────────────────────┐  0px
│                      HEADER (100px)                     │
│                  "캐릭터 생성 - 1/5단계"                  │
├─────────────────────────────────────────────────────────┤  100px
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │           잔디 배경 (GrassBackground)             │   │
│  │                                                   │   │
│  │                    ┌─────────┐                   │   │
│  │                    │         │                   │   │
│  │                    │  🏃     │ ← 캐릭터 64x64    │   │
│  │                    │ 미리보기│   (4x 스케일)     │   │
│  │                    │         │                   │   │
│  │                    └─────────┘                   │   │
│  │                                                   │   │
│  └─────────────────────────────────────────────────┘   │  650px
│                   PREVIEW AREA (550px)                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │  헤어 스타일:  [검정] [금발] [빨강] [기타]       │   │
│  ├─────────────────────────────────────────────────┤   │
│  │  피부색:       [밝음] [중간] [어두움]            │   │
│  ├─────────────────────────────────────────────────┤   │
│  │  이름:         [________________]               │   │
│  ├─────────────────────────────────────────────────┤   │
│  │  포지션:       [GK] [DF] [MF] [FW]              │   │
│  └─────────────────────────────────────────────────┘   │  1550px
│                   OPTIONS AREA (900px)                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│           [◀ 이전]              [다음 ▶]               │
│                                                         │  1920px
│                   BUTTONS (370px)                       │
└─────────────────────────────────────────────────────────┘
```

**픽셀 배분:**
| 영역 | 높이 | 비율 |
|------|------|------|
| Header | 100px | 5% |
| Preview (잔디+캐릭터) | 550px | 29% |
| Options | 900px | 47% |
| Buttons | 370px | 19% |
| **합계** | **1920px** | 100% |

---

### 10.3 마이팀 설정 화면 레이아웃 (1080x1920)

```
┌─────────────────────────────────────────────────────────┐  0px
│                      HEADER (100px)                     │
│                      "마이팀 설정"                       │
├─────────────────────────────────────────────────────────┤  100px
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │           잔디 배경 (GrassBackground)             │   │
│  │  🏃        ⚽       🧍                           │   │
│  │      🏃         🏃        🧍                    │   │
│  │   🧍      🏃          ⚽     🏃                 │   │
│  │        🧍     🏃    🧍        🏃               │   │
│  │                                                   │   │
│  └─────────────────────────────────────────────────┘   │  850px
│              TEAM PREVIEW AREA (750px)                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │  팀 이름:    [FC 서울_______________]           │   │
│  ├─────────────────────────────────────────────────┤   │
│  │  메인 컬러:  🔴🟠🟡🟢🔵🟣⚫⚪ [선택됨: 🔴]    │   │
│  ├─────────────────────────────────────────────────┤   │
│  │  서브 컬러:  🔴🟠🟡🟢🔵🟣⚫⚪ [선택됨: ⚪]    │   │
│  ├─────────────────────────────────────────────────┤   │
│  │  패턴:       [단색] [가로줄] [세로줄] [체크]    │   │
│  └─────────────────────────────────────────────────┘   │  1550px
│                   OPTIONS AREA (700px)                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│                      [저장하기]                         │
│                                                         │  1920px
│                   BUTTONS (370px)                       │
└─────────────────────────────────────────────────────────┘
```

**픽셀 배분:**
| 영역 | 높이 | 비율 |
|------|------|------|
| Header | 100px | 5% |
| Team Preview (잔디+선수들) | 750px | 39% |
| Options | 700px | 37% |
| Buttons | 370px | 19% |
| **합계** | **1920px** | 100% |

---

### 10.4 잔디 배경 + 캐릭터 영역 구현

**PreviewContainer 씬 구조:**
```
PreviewContainer (Control)
├── GrassBackground (TextureRect)
│   - texture: grass8.png
│   - stretch_mode: STRETCH_TILE
│   - anchors: Full Rect
│   - texture_filter: NEAREST
│
├── FieldLines (Node2D) [선택적]
│   - 필드 라인 일부 그리기 (센터 서클 등)
│
├── CharactersLayer (Node2D)
│   ├── CharacterPreviewSprite (캐릭터 생성용 - 1명)
│   │   또는
│   └── TeamPreviewBackground (마이팀용 - 여러 명)
│
└── Vignette (ColorRect) [선택적]
    - 가장자리 어둡게 하는 효과
```

**코드:**
```gdscript
## PreviewContainer.gd
extends Control

@export var show_field_lines: bool = false
@export var vignette_enabled: bool = true

@onready var grass_bg: TextureRect = $GrassBackground
@onready var characters_layer: Node2D = $CharactersLayer

func _ready() -> void:
    _setup_grass()
    if vignette_enabled:
        _setup_vignette()

func _setup_grass() -> void:
    grass_bg.texture = preload("res://assets/sprites/grass/grass8.png")
    grass_bg.stretch_mode = TextureRect.STRETCH_TILE
    grass_bg.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST

func _setup_vignette() -> void:
    ## 가장자리 그라데이션으로 자연스럽게
    var vignette := $Vignette as ColorRect
    var shader := preload("res://assets/shaders/vignette.gdshader")
    vignette.material = ShaderMaterial.new()
    vignette.material.shader = shader
```

---

### 10.5 Vignette 셰이더 (가장자리 어둡게)

**파일:** `assets/shaders/vignette.gdshader`

```glsl
shader_type canvas_item;

uniform float intensity : hint_range(0.0, 1.0) = 0.4;
uniform float softness : hint_range(0.0, 1.0) = 0.5;

void fragment() {
    vec2 uv = UV - 0.5;
    float dist = length(uv) * 2.0;
    float vignette = smoothstep(1.0 - softness, 1.0, dist);
    COLOR = vec4(0.0, 0.0, 0.0, vignette * intensity);
}
```

---

### 10.6 씬 파일 구조 정리

```
scenes/ui/
├── components/
│   ├── PreviewContainer.tscn        # 잔디 배경 + 캐릭터 컨테이너
│   ├── CharacterPreviewSprite.tscn  # 단일 캐릭터 미리보기
│   ├── TeamPreviewBackground.tscn   # 여러 선수 배경
│   ├── ColorPickerGrid.tscn         # 색상 선택 그리드
│   └── PatternSelector.tscn         # 패턴 선택 버튼들
│
├── CharacterCreationScreen.tscn     # 캐릭터 생성 (통합)
└── MyTeamSetupScreen.tscn           # 마이팀 설정 (통합)
```

---

### 10.7 캐릭터 생성 화면 씬 구조

```
CharacterCreationScreen (Control, 1080x1920)
│
├── VBoxContainer (anchors: Full Rect)
│   │
│   ├── HeaderPanel (min_height: 100)
│   │   └── Label "캐릭터 생성 - 1/5단계"
│   │
│   ├── PreviewContainer (min_height: 550)
│   │   ├── GrassBackground
│   │   └── CharactersLayer
│   │       └── CharacterPreviewSprite (position: center)
│   │
│   ├── OptionsScrollContainer (size_flags_vertical: EXPAND)
│   │   └── VBoxContainer
│   │       ├── HairStyleSelector
│   │       │   └── HBoxContainer
│   │       │       ├── Button "검정"
│   │       │       ├── Button "금발"
│   │       │       ├── Button "빨강"
│   │       │       └── Button "기타"
│   │       ├── SkinToneSelector
│   │       ├── NameInput (LineEdit)
│   │       └── PositionSelector
│   │
│   └── ButtonsPanel (min_height: 120)
│       └── HBoxContainer
│           ├── Button "◀ 이전"
│           └── Button "다음 ▶"
```

---

### 10.8 마이팀 설정 화면 씬 구조

```
MyTeamSetupScreen (Control, 1080x1920)
│
├── VBoxContainer (anchors: Full Rect)
│   │
│   ├── HeaderPanel (min_height: 100)
│   │   └── Label "마이팀 설정"
│   │
│   ├── PreviewContainer (min_height: 750)
│   │   ├── GrassBackground
│   │   └── CharactersLayer
│   │       └── TeamPreviewBackground
│   │           └── (선수 8명 동적 생성)
│   │
│   ├── OptionsContainer (size_flags_vertical: EXPAND)
│   │   └── VBoxContainer
│   │       ├── TeamNameInput (LineEdit)
│   │       ├── PrimaryColorPicker (ColorPickerGrid)
│   │       ├── SecondaryColorPicker (ColorPickerGrid)
│   │       └── PatternSelector
│   │
│   └── ButtonsPanel (min_height: 120)
│       └── Button "저장하기"
```

---

### 10.9 구현 순서

#### Step 1: 기본 컴포넌트
1. `GrassBackground` (TextureRect 설정)
2. `vignette.gdshader` 생성
3. `PreviewContainer.tscn` 생성

#### Step 2: 캐릭터 컴포넌트
4. `CharacterPreviewSprite.tscn` 생성
5. `TeamPreviewBackground.gd` 작성

#### Step 3: 선택 UI
6. `ColorPickerGrid.tscn` (색상 버튼 그리드)
7. `PatternSelector.tscn` (패턴 버튼들)

#### Step 4: 화면 통합
8. `CharacterCreationScreen.tscn` 조립
9. `MyTeamSetupScreen.tscn` 조립

#### Step 5: 연결
10. 선택 → 미리보기 연동
11. 데이터 저장 연동

---

## 11. 기존 커스터마이제이션 분석 및 매핑

### 11.1 현재 캐릭터 생성 (Step2_Appearance.gd)

**현재 커스터마이징 옵션:**

| 항목 | 현재 옵션 | 데이터 키 |
|------|----------|-----------|
| 헤어 스타일 | braids, curly, medium, spiky, afro, buzz, mohawk, wavy | `hair_style` |
| 헤어 색상 | brown, black, blonde, ginger, gray | `hair_color` |
| 피부색 | light, medium, olive, brown, dark | `skin_tone` |
| 유니폼 상의 | 11가지 색상 (red, orange, yellow, green, cyan, blue, purple, pink, white, black, gray) | `torso_color` |
| 유니폼 소매 | 11가지 색상 (동일) | `sleeve_color` |

**현재 구현 방식:**
- `SkeletonCharacter` 씬 사용 (2D Skeleton 기반)
- `CharacterCustomizer` 컴포넌트로 외형 변경
- `PlayerAppearanceBridge`로 데이터 직렬화

### 11.2 Socceralia 스프라이트로 변경 시 매핑

**헤어 스타일 매핑 (단순화):**

| 현재 (8가지) | Socceralia (5가지) | 비고 |
|-------------|-------------------|------|
| braids | `other` | |
| curly | `other` | |
| medium | `black` or `blonde` or `redhead` | 헤어 색상에 따라 |
| spiky | `other` | |
| afro | `other` | |
| buzz | `black` | 짧은 머리 |
| mohawk | `other` | |
| wavy | `blonde` or `redhead` | 헤어 색상에 따라 |

**제안: 헤어 스타일 → 헤어 색상 통합**

Socceralia 스프라이트는 **헤어 스타일과 색상이 통합**되어 있으므로:

| 새로운 옵션 | 폴더 | 설명 |
|------------|------|------|
| `black` | `black/` | 검은 머리 (짧은 스타일) |
| `blonde` | `blonde/` | 금발 (중간 길이) |
| `redhead` | `redhead/` | 빨간 머리 |
| `other` | `other/` | 기타 스타일 (갈색, 다양한 스타일) |
| `gk` | `gk/` | 골키퍼 전용 |

**피부색:**
- Socceralia 스프라이트는 피부색이 이미 고정되어 있음
- **셰이더로 피부색 변경 불가** (옷만 key_color 기반으로 변경)
- **해결책:** 피부색 옵션 제거 또는 스프라이트 리컬러링 에셋 추가 필요

**유니폼 색상:**
- `KitPattern.gdshader` 사용하여 `primary_color`, `secondary_color` 적용
- 기존 `torso_color`, `sleeve_color` → `primary_color`, `secondary_color`로 통합
- 패턴 옵션 추가 (단색, 가로줄, 세로줄, 체크)

### 11.3 현재 마이팀 설정 (MyTeamSetupScreen.gd)

**현재 커스터마이징 옵션:**

| 항목 | 현재 구현 | 데이터 위치 |
|------|----------|------------|
| 팀 이름 | LineEdit | `MyTeamData.academy_settings.name` |
| 팀 별명 | LineEdit | `MyTeamData.academy_settings.nickname` |
| 엠블럼 아이콘 | 선택 UI | `emblem.icon` |
| 엠블럼 배경 | 선택 UI | `emblem.background` |
| 메인 컬러 | ColorPicker | `uniform.home.primary` |
| 서브 컬러 | ColorPicker | `uniform.home.secondary` |

**Socceralia로 변경 시:**
- 메인/서브 컬러 → `KitPattern.gdshader`의 `primary_color`, `secondary_color`
- 패턴 선택 추가 필요 (pattern_type: 0~4)
- 배경에 선수들 8명이 돌아다니며 **실시간으로 색상 변경 반영**

### 11.4 데이터 스키마 변경 제안

**기존 appearance:**
```gdscript
{
    "face_preset": 0,
    "hair_style_index": 2,
    "body_type": 1,
    "parts_appearance": {
        "hair_style": "medium",
        "hair_color": "brown",
        "skin_tone": "medium",
        "torso_color": "red",
        "sleeve_color": "white"
    }
}
```

**새로운 appearance (Socceralia용):**
```gdscript
{
    "sprite_type": "socceralia",  # 신규: 스프라이트 타입
    "hair_folder": "black",       # black/blonde/redhead/other/gk
    "skin_tone": "medium",        # 유지 (향후 확장용)
    "uniform": {
        "primary_color": "#FF0000",
        "secondary_color": "#FFFFFF",
        "pattern_type": 0         # 0=단색, 1=가로줄, 2=세로줄, 3=체크
    }
}
```

**기존 uniform (MyTeamData):**
```gdscript
{
    "home": {
        "primary": "#FF0000",
        "secondary": "#FFFFFF"
    },
    "away": {
        "primary": "#0000FF",
        "secondary": "#FFFFFF"
    }
}
```

**새로운 uniform (패턴 추가):**
```gdscript
{
    "home": {
        "primary": "#FF0000",
        "secondary": "#FFFFFF",
        "pattern_type": 2  # 세로줄
    },
    "away": {
        "primary": "#0000FF",
        "secondary": "#FFFFFF",
        "pattern_type": 0  # 단색
    }
}
```

### 11.5 UI 변경 사항 요약

#### 캐릭터 생성 화면

| 현재 | 변경 후 |
|------|--------|
| SkeletonCharacter 미리보기 | CharacterPreviewSprite (Socceralia) |
| 헤어 스타일 8가지 버튼 | 헤어 타입 4가지 버튼 (black/blonde/redhead/other) |
| 헤어 색상 5가지 버튼 | **제거** (헤어 타입에 통합) |
| 피부색 5가지 버튼 | **제거** 또는 유지 (향후 확장) |
| 유니폼 상의/소매 각각 11색 | 메인/서브 컬러 각각 16색 프리셋 |
| - | 패턴 선택 추가 (4가지) |

#### 마이팀 설정 화면

| 현재 | 변경 후 |
|------|--------|
| 엠블럼만 표시 | 잔디밭 + 선수 8명 애니메이션 배경 |
| ColorPicker (연속) | ColorPickerGrid (16색 프리셋) |
| - | 패턴 선택 추가 (4가지) |

### 11.6 호환성 고려사항

1. **기존 저장 데이터:**
   - `sprite_type` 필드가 없으면 기존 SkeletonCharacter 방식 사용
   - 마이그레이션 함수 필요: `migrate_appearance_to_socceralia()`

2. **경기 뷰어와의 연동:**
   - 캐릭터 생성에서 선택한 `hair_folder` → 경기 뷰어의 `SoccerPlayer`에 전달
   - 팀 설정에서 선택한 `uniform` → `TeamColorManager`에 전달

3. **GK 처리:**
   - 포지션이 GK인 경우 자동으로 `hair_folder = "gk"` 사용
   - 또는 GK 전용 유니폼 색상 별도 설정

---

## 12. 구현 우선순위 조정

### Phase 1: 핵심 컴포넌트 ✅ 완료 (2025-12-08)
1. ✅ `CharacterPreviewSprite.gd/tscn` - 단일 캐릭터 미리보기
2. ✅ `TeamPreviewBackground.gd/tscn` - 팀 배경 애니메이션
3. ✅ `PreviewContainer.tscn` - 잔디 배경 컨테이너
4. ✅ `vignette.gdshader` - 가장자리 어둡게 효과

### Phase 2: UI 통합 ✅ 완료 (2025-12-08)
5. ✅ `Step2_Appearance.gd` 수정 - Socceralia 스프라이트 사용
   - `use_socceralia_sprites` export 변수 추가
   - 헤어 폴더 선택 (black/blonde/redhead/other)
   - primary/secondary 컬러 + 패턴 선택
6. ✅ `MyTeamSetupScreen.gd` 수정 - 배경 추가 및 색상 연동
   - `use_team_preview` export 변수 추가
   - TeamPreviewBackground 8명 선수 배경
   - 패턴 선택 UI 동적 추가

### Phase 3: 데이터 연동 ✅ 완료 (2025-12-08)
7. ✅ `PlayerAppearanceBridge` 확장 - Socceralia 스키마 지원
   - `is_socceralia_schema()`, `socceralia_to_legacy()`, `legacy_to_socceralia()` 함수 추가
   - `create_random_socceralia()`, `create_random_socceralia_with_uniform()` 함수 추가
   - `_color_id_to_hex()`, `_hex_to_color_id()` 색상 변환 헬퍼 추가
8. ✅ `MyTeamData` 확장 - pattern_type 저장 지원
   - uniform 구조에 `pattern_type` 필드 추가
   - `get_team_uniform()` 메서드에 pattern_type 반환 추가
9. ✅ 경기 뷰어 연동 완료
   - `SoccerPlayer.gd`: `apply_appearance()`, `apply_legacy_appearance()` 메서드 추가
   - `TeamColorManager.gd`: `apply_custom_team_color()`, `setup_team_with_appearance()` 메서드 추가
   - `HorizontalMatchViewer.gd`: `setup_teams_with_uniform()`, `setup_my_team_as_home()` 메서드 추가

### Phase 4: 엔드투엔드 테스트 및 마무리 (다음 단계)
10. 🔲 실제 경기 시뮬레이션에서 캐릭터 외형 적용 테스트
    - MyTeamData에서 유니폼 로드 → HorizontalMatchViewer에 전달
    - 선수 저장 데이터의 hair_folder → SoccerPlayer에 전달
11. 🔲 UI 테스트 (Step2_Appearance, MyTeamSetupScreen)
    - Socceralia 스프라이트 미리보기 동작 확인
    - 색상/패턴 변경 시 실시간 반영 확인
12. 🔲 저장/로드 테스트
    - 캐릭터 생성 후 저장 → 경기 뷰어에서 로드
    - 마이팀 설정 저장 → 경기 뷰어에서 팀 유니폼 적용

---

## 13. 데이터 플로우 요약

### 13.1 캐릭터 생성 → 경기 뷰어

```
Step2_Appearance.gd (캐릭터 생성)
    ↓ 선택: hair_folder, uniform (primary/secondary/pattern)
PlayerAppearanceBridge.legacy_to_socceralia()
    ↓ 변환
저장: PlayerData.appearance = { "hair_folder": "black", "uniform": {...} }
    ↓
HorizontalMatchViewer.setup_teams_with_uniform()
    ↓
SoccerPlayer.apply_appearance()
```

### 13.2 마이팀 설정 → 경기 뷰어

```
MyTeamSetupScreen.gd (마이팀 설정)
    ↓ 선택: primary_color, secondary_color, pattern_type
저장: MyTeamData.academy_settings.uniform = { "home": {...}, "away": {...} }
    ↓
HorizontalMatchViewer.setup_my_team_as_home(my_team_data, roster, ...)
    ↓
TeamColorManager.setup_team_with_appearance() + apply_custom_team_color()
```

### 13.3 API 사용 예시

```gdscript
## 경기 시작 시 팀 설정 예시

func _setup_match():
    var match_viewer = $HorizontalMatchViewer

    # 마이팀 로스터 (각 선수의 외형 데이터 포함)
    var my_roster = [
        { "id": "p1", "position": "GK", "jersey_number": 1, "appearance": { "hair_folder": "gk" } },
        { "id": "p2", "position": "CB", "jersey_number": 4, "appearance": { "hair_folder": "black" } },
        # ... 11명
    ]

    # 상대팀 로스터 (기본 외형 사용)
    var opponent_roster = [
        { "id": "opp1", "position": "GK", "jersey_number": 1 },
        # ...
    ]

    # MyTeamData autoload
    var my_team_data = get_node("/root/MyTeamData")

    # 팀 설정 적용
    match_viewer.setup_my_team_as_home(my_team_data, my_roster, opponent_roster, "brazil")
```
