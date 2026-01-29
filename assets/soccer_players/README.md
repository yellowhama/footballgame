# Soccer Players Asset Structure

## Character Library (32 Characters)

### Main Character
| Name | Body Type | Description |
|------|-----------|-------------|
| lia | female_tall_slender | 주인공 캐릭터 |

### Body Variant Presets (8)
| Name | Height | Build | Scale (X/Y/Z) |
|------|--------|-------|---------------|
| male_tall_slim | tall | slim | 0.95/1.0/1.10 |
| male_tall_muscular | tall | muscular | 1.10/1.05/1.10 |
| male_short_athletic | short | athletic | 1.00/1.0/0.92 |
| male_average_muscular | average | muscular | 1.10/1.05/1.00 |
| female_tall_slender | tall | slender | 0.90/1.0/1.10 |
| female_tall_glamorous | tall | glamorous | 1.00/1.0/1.08 |
| female_short_athletic | short | athletic | 0.95/1.0/0.92 |
| female_average_slender | average | slender | 0.88/1.0/1.00 |

### NPC Characters - Position Based (11)
| Name | Position | Gender |
|------|----------|--------|
| npc_striker_male | Striker | Male |
| npc_striker_female | Striker | Female |
| npc_midfielder_male | Midfielder | Male |
| npc_midfielder_female | Midfielder | Female |
| npc_midfielder_plain_female | Midfielder | Female |
| npc_defender_male | Defender | Male |
| npc_defender_female | Defender | Female |
| npc_goalkeeper_male | Goalkeeper | Male |
| npc_goalkeeper_female | Goalkeeper | Female |
| npc_reserve_female | Reserve | Female |
| npc_utility_female | Utility | Female |

### Plain NPC Characters - Background (12)
수수한 배경 캐릭터용 NPC (평범한 외형, 배경 선수용)

| Name | Gender | Vertices (Full) | Vertices (Mobile) |
|------|--------|-----------------|-------------------|
| plain_female_01 | Female | ~49K | ~18K |
| plain_female_02 | Female | ~49K | ~18K |
| plain_female_03 | Female | ~49K | ~18K |
| plain_female_04 | Female | ~49K | ~18K |
| plain_female_05 | Female | ~49K | ~18K |
| plain_female_06 | Female | ~49K | ~18K |
| plain_male_01 | Male | ~49K | ~19K |
| plain_male_02 | Male | ~49K | ~19K |
| plain_male_03 | Male | ~49K | ~19K |
| plain_male_04 | Male | ~49K | ~19K |
| plain_male_05 | Male | ~49K | ~19K |
| plain_male_06 | Male | ~49K | ~19K |

## Folder Structure
```
soccer_players/
├── body_presets.py       # 체형 프리셋 정의
├── workflow_pipeline.py  # 파이프라인 자동화
├── WORKFLOW.md           # 워크플로우 문서
├── README.md             # 이 문서
│
├── characters/           # 캐릭터 메시 (~50K verts, 원본)
│   ├── lia/
│   │   ├── mesh.glb     # 리깅된 메시 (RigAnything 스켈레톤)
│   │   └── textures/    # 유니폼 텍스처
│   ├── male_tall_slim/
│   │   └── mesh.glb
│   ├── female_tall_slender/
│   │   └── mesh.glb
│   └── npc_*/
│       └── mesh.glb
│
├── characters_mobile/    # 모바일용 최적화 (~18K verts)
│   ├── lia/
│   │   └── mesh.glb
│   └── [same structure as characters/]
│
└── animations/           # 공유 애니메이션 라이브러리
    ├── shared/          # 모든 플레이어 공용
    │   ├── offensive_idle.fbx
    │   ├── jog_strafe_left.fbx
    │   └── ...
    ├── field_player/    # 필드 플레이어 전용
    │   ├── kick_soccerball.fbx
    │   ├── header_soccerball.fbx
    │   └── ...
    └── goalkeeper/      # 골키퍼 전용
        ├── goalkeeper_idle.fbx
        ├── goalkeeper_catch.fbx
        └── ...
```

## Usage in Godot

### Load Character
```gdscript
# Body preset character (원본 ~50K verts)
var player = load("res://assets/soccer_players/characters/male_tall_muscular/mesh.glb")

# NPC character (원본)
var npc = load("res://assets/soccer_players/characters/npc_striker_male/mesh.glb")

# Mobile optimized (~18K verts)
var player_mobile = load("res://assets/soccer_players/characters_mobile/male_tall_muscular/mesh.glb")
```

### Select by Body Type
```gdscript
# body_presets.py의 프리셋 사용
func get_character_by_preset(height: String, build: String) -> PackedScene:
    var key = "%s_%s" % [height, build]
    var path = "res://assets/soccer_players/characters/%s/mesh.glb" % key
    return load(path)

# Example
var striker = get_character_by_preset("tall", "athletic")
```

### Apply Animation
```gdscript
var anim = load("res://assets/soccer_players/animations/field_player/kick_soccerball.fbx")
$AnimationPlayer.add_animation("kick", anim)
```

## Skeleton Info
- All characters use RigAnything skeleton (Bone_0 ~ Bone_33)
- Animations are retargeted to this skeleton
- Any animation works with any character

## Body Preset System

체형 프리셋은 `body_presets.py`에서 정의됩니다:

### Height (키)
- `tall`: 1.10x Z scale, long legs
- `average`: 1.0x Z scale
- `short`: 0.92x Z scale, compact build

### Build (체형)
- `slim`: 0.9x X scale, lean
- `athletic`: 1.0x X scale, toned
- `muscular`: 1.1x X scale, broad

### Figure (몸매) - Female Only
- `slender`: model-like proportions
- `glamorous`: curvy, hourglass shape
- `standard`: balanced proportions

## Pipeline

전체 파이프라인은 `WORKFLOW.md`를 참조하세요:
1. Image Generation (ComfyUI)
2. TRELLIS 3D Conversion
3. RigAnything Auto-Rigging
4. Mesh Scale Application
5. Asset Organization
