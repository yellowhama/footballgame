# Team Uniform Shader System

흰색 유니폼 캐릭터에 팀 색상을 적용하는 셰이더 시스템.

## 파일 구조

```
shaders/
├── team_uniform.gdshader        # 전체 기능 (등번호 포함)
├── team_uniform_simple.gdshader # 간단 버전 (색상만)
└── README.md

scripts/
├── team_colors.gd               # 팀 색상 프리셋
└── player_character.gd          # 캐릭터 컴포넌트
```

## 사용법

### 방법 1: 인스펙터에서 직접 설정

1. MeshInstance3D 선택
2. Surface Material Override → New ShaderMaterial
3. Shader → `team_uniform_simple.gdshader` 로드
4. `team_color` 파라미터 조정

### 방법 2: 스크립트로 적용

```gdscript
# 프리셋 팀 사용
TeamColors.apply_team_color($Player/MeshInstance3D, "liverpool")

# 커스텀 색상 사용
TeamColors.apply_custom_color($Player/MeshInstance3D, Color.BLUE)
```

### 방법 3: PlayerCharacter 컴포넌트

```gdscript
# 씬에 player_character.gd 붙이고
$Player.team_color = Color.RED
$Player.jersey_number = 10

# 또는 프리셋으로
$Player.set_team("manchester_united")
```

## 셰이더 파라미터

### team_uniform_simple.gdshader

| 파라미터 | 타입 | 설명 |
|----------|------|------|
| `team_color` | Color | 팀 색상 |
| `threshold` | float | 흰색 감지 임계값 (0.5-0.95) |
| `albedo_texture` | Texture2D | 원본 텍스처 |

### team_uniform.gdshader (전체 버전)

| 파라미터 | 타입 | 설명 |
|----------|------|------|
| `team_primary_color` | Color | 주 팀 색상 |
| `team_secondary_color` | Color | 보조 색상 (등번호 등) |
| `show_number` | bool | 등번호 표시 여부 |
| `number_texture` | Texture2D | 등번호 텍스처 |
| `number_position` | Vector2 | 등번호 UV 위치 |
| `number_scale` | float | 등번호 크기 |

## 프리셋 팀 색상

```gdscript
TeamColors.TEAMS = {
    "red", "blue", "green", "yellow", "white", "black",
    "manchester_united", "liverpool", "arsenal",
    "chelsea", "manchester_city", "barcelona",
    "real_madrid", "juventus",
    "brazil", "dortmund", "netherlands",
    "orange", "purple", "pink"
}
```

## 등번호 추가 (Decal3D 사용)

```gdscript
# Decal3D 노드로 등번호 붙이기
var decal = Decal3D.new()
decal.texture_albedo = load("res://assets/numbers/10.png")
decal.size = Vector3(0.3, 0.4, 0.1)
decal.position = Vector3(0, 1.2, -0.15)  # 등 위치
$Player.add_child(decal)
```
