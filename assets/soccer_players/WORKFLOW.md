# Soccer Player Character Pipeline

캐릭터 생성부터 게임 에셋까지의 전체 워크플로우.

## Current Status (2025-01-08)

| Stage | 내용 | 상태 | 캐릭터 수 |
|-------|------|------|----------|
| 1 | Image Generation | ✅ 완료 | 20 |
| 2 | TRELLIS 3D | ✅ 완료 | 20 |
| 3 | RigAnything | ✅ 완료 | 20 |
| 4 | Mesh Scale | ✅ 완료 | 8 (body variants) |
| 5 | Asset Organization | ✅ 완료 | 20 |
| 6 | Mobile Optimization | ✅ 완료 | 20 |
| 7 | Godot Integration | 🔜 다음 | - |

**총 캐릭터**: 20개 (lia 1 + NPC 11 + Body Variants 8)

## Pipeline Stages

```
Stage 1: Image Generation (ComfyUI)
    ↓ body_presets.py → prompt modifiers
Stage 2: TRELLIS 3D Conversion
    ↓ 2D → 3D GLB (resolution: 1024_cascade)
Stage 3: RigAnything Auto-Rigging
    ↓ mesh → skeleton (CPU mode for RTX 50)
Stage 4: Mesh Scale Application
    ↓ body preset → x/y/z scale
Stage 5: Asset Organization
    → characters/ + animations/
Stage 6: Mobile Optimization
    → characters_mobile/ (Decimate to ~18K verts)
```

## Mobile Optimization

모바일용 최적화된 메시 (~18K vertices, 62% 감소):

| 항목 | 값 |
|------|-----|
| Target | 15,000 vertices |
| 실제 평균 | ~18,600 vertices |
| 파일 크기 감소 | 4.2MB → 2.7MB (35-40%) |
| 출력 폴더 | `characters_mobile/` |

**최적화 방법**:
1. `remove_doubles` - 중복 vertex 제거 (40-50% 감소)
2. `Decimate COLLAPSE` - 기하 단순화
3. `dissolve_limited` - 평면 병합 (필요시)
4. 반복 decimate (필요시)

## Body Presets

### 키 (Height)
| Preset | Prompt | Scale Z |
|--------|--------|---------|
| `tall` | tall, long legs, elongated | 1.10 |
| `average` | average height, balanced | 1.00 |
| `short` | petite, compact, shorter | 0.92 |

### 체형 (Build)
| Preset | Prompt | Scale X |
|--------|--------|---------|
| `slim` | slim, lean, slender, thin | 0.90 |
| `athletic` | athletic, toned, fit | 1.00 |
| `muscular` | muscular, strong, broad | 1.10 |

### 몸매 (Figure) - Female Only
| Preset | Prompt |
|--------|--------|
| `slender` | slender figure, model-like, elegant |
| `glamorous` | glamorous, curvy, hourglass |
| `standard` | balanced proportions, natural |

## Quick Usage

### 전체 파이프라인 실행
```bash
cd /home/hugh/footballgame_repo/assets/soccer_players
python workflow_pipeline.py --stage all
```

### 단계별 실행
```bash
# 이미지 생성만
python workflow_pipeline.py --stage generate

# TRELLIS 변환만
python workflow_pipeline.py --stage trellis

# 리깅만
python workflow_pipeline.py --stage rig

# 에셋 정리만
python workflow_pipeline.py --stage organize
```

### 프리셋 확인
```bash
python workflow_pipeline.py --list-presets
```

## Character Archetypes

### Male
| Name | Height | Build | 설명 |
|------|--------|-------|------|
| striker_agile | tall | athletic | 빠른 스트라이커 |
| striker_power | tall | muscular | 타겟맨 |
| midfielder_playmaker | average | slim | 창의적 미드필더 |
| midfielder_box2box | average | athletic | 박스투박스 |
| defender_stopper | tall | muscular | 센터백 |
| defender_agile | average | athletic | 풀백 |
| goalkeeper_tall | tall | athletic | 골키퍼 |

### Female
| Name | Height | Build | Figure | 설명 |
|------|--------|-------|--------|------|
| female_striker_fast | tall | slim | slender | 스피드 스트라이커 |
| female_striker_power | tall | athletic | glamorous | 피지컬 스트라이커 |
| female_midfielder | average | athletic | slender | 테크니컬 미드필더 |
| female_defender | tall | athletic | standard | 견고한 수비수 |
| female_goalkeeper | tall | athletic | slender | 민첩한 골키퍼 |

## Mesh Scale Reference

3D 모델에 적용되는 스케일:

```python
MESH_SCALE_PRESETS = {
    "tall_slim":       {"x": 0.95, "y": 1.0, "z": 1.10},
    "tall_athletic":   {"x": 1.00, "y": 1.0, "z": 1.10},
    "tall_muscular":   {"x": 1.10, "y": 1.05, "z": 1.10},
    "average_slim":    {"x": 0.90, "y": 1.0, "z": 1.00},
    "average_athletic":{"x": 1.00, "y": 1.0, "z": 1.00},
    "average_muscular":{"x": 1.10, "y": 1.05, "z": 1.00},
    "short_slim":      {"x": 0.90, "y": 1.0, "z": 0.92},
    "short_athletic":  {"x": 1.00, "y": 1.0, "z": 0.92},
    "short_muscular":  {"x": 1.05, "y": 1.0, "z": 0.92},
}
```

## File Structure

```
soccer_players/
├── body_presets.py          # 체형 프리셋 정의
├── workflow_pipeline.py     # 전체 파이프라인 스크립트
├── WORKFLOW.md              # 이 문서
├── README.md                # Godot 사용법
│
├── characters/              # 완성된 캐릭터
│   ├── lia/
│   │   ├── mesh.glb
│   │   └── textures/
│   ├── male_tall_muscular/
│   │   └── mesh.glb
│   └── ...
│
└── animations/              # 공유 애니메이션
    ├── shared/              # 모든 플레이어용
    ├── field_player/        # 필드 플레이어용
    └── goalkeeper/          # 골키퍼용
```

## Python API

```python
from body_presets import get_body_prompt, get_mesh_scale

# 프롬프트 생성
prompt = get_body_prompt(height="tall", build="athletic", figure="glamorous")
# → {"positive": "tall, long legs, ..., athletic build, ..., glamorous figure, ..."}

# 메시 스케일
scale = get_mesh_scale(height="tall", build="muscular")
# → {"x": 1.1, "y": 1.05, "z": 1.1}
```

## Dependencies

- **ComfyUI**: http://127.0.0.1:8188
- **TRELLIS**: stage3_trellis/TRELLIS/
- **RigAnything**: stage3_trellis/RigAnything/
- **Blender**: /home/hugh/blender/blender

## Notes

- RigAnything는 CPU 모드 실행 (RTX 50 시리즈 호환)
- TRELLIS는 1024x1024 정사각형 이미지 필요 (자동 패딩)
- 모든 캐릭터는 동일한 RigAnything 스켈레톤 (Bone_0~33) 사용
- 애니메이션은 스켈레톤 기반 → 모든 캐릭터에서 공유 가능
