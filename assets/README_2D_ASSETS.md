# 2D Assets Documentation

## Overview
2D isometric quarter-view assets for the football management game, following Kairosoft/Pocket League Story 2 visual style.

**Integration Date**: 2025-10-16
**Status**: ✅ Integrated and tested
**License**: CC0 (Creative Commons Zero) - Free for all use

## Asset Structure

### Characters (`assets/characters/`)

#### Kenney Toon Characters (`2d_sprites/`)
**Source**: Kenney.nl Toon Characters Pack 1
**License**: CC0 (see `2d_sprites/License.txt`)
**Total Size**: ~7.1MB

**Available Characters**:
- **Male person** - 45 poses (walk, run, idle, jump, etc.)
- **Female person** - 45 poses
- **Male adventurer** - 45 poses
- **Female adventurer** - 45 poses
- **Robot** - 45 poses
- **Zombie** - 45 poses

**Format Structure**:
```
Character Name/
├── PNG/
│   ├── Poses/           # Full character sprites (standard resolution)
│   ├── Poses HD/        # High-definition full sprites
│   ├── Parts/           # Modular body parts for custom assembly
│   └── Parts HD/        # HD modular parts
└── Vector/              # SVG source files
```

**Animation Frames**:
- Walk: 8 frames (`walk0.png` - `walk7.png`)
- Run: 3 frames (`run0.png` - `run2.png`)
- Jump: 3 frames (`jump0.png` - `jump2.png`)
- Additional poses: idle, climb, duck, fall, hurt, etc.

**Usage Example**:
```gdscript
# In AnimatedSprite2D
var walk_frames = [
    "res://assets/characters/2d_sprites/Male person/PNG/Poses/character_malePerson_walk0.png",
    # ... walk1-walk7
]
```

### Environment (`assets/environment/`)

#### Field Assets (`field/`)
**Source**: Kenney Sports Pack
**Size**: ~1.3MB

**Contents**:
- Soccer field tiles and tilesheet
- Equipment sprites (footballs, goals, cones)
- Field markers and elements

#### Isometric Assets (`isometric/`)
**Source**: Kenney Isometric Blocks
**Size**: ~2.6MB

**Contents**:
- Isometric building blocks
- Environment decoration
- Quarter-view perspective elements

## Test Scenes

### Character Animation Test
**File**: `scenes/test/2d_character_test.tscn`

**Features**:
- AnimatedSprite2D with Kenney male person walk cycle
- Arrow key movement controls
- Space to toggle animation
- Camera2D with 2x zoom

**How to Run**:
1. Open in Godot 4.4
2. Press F5 or F6 to run test scene
3. Use arrow keys to move character
4. Press Space to toggle walk animation

## Integration with OpenFootball Rust Engine

### 2D Position Adapter
The Rust engine outputs 3D coordinates that need conversion to 2D isometric screen positions:

```gdscript
# Convert Rust Vector3 to 2D isometric screen position
func rust_to_screen(rust_pos: Vector3) -> Vector2:
    # Quarter-view transformation
    var iso_x = (rust_pos.x - rust_pos.y) * cos(PI/4)
    var iso_y = (rust_pos.x + rust_pos.y) * sin(PI/4)
    return Vector2(iso_x, iso_y) * SCALE_FACTOR
```

**Scale Factor**: TBD (adjust based on field size and screen resolution)

### Animation State Mapping
Map Rust player states to sprite animations:

```gdscript
match player_state:
    PlayerState.IDLE: sprite.play("idle")
    PlayerState.WALKING: sprite.play("walk")
    PlayerState.RUNNING: sprite.play("run")
    PlayerState.JUMPING: sprite.play("jump")
```

## Asset Optimization

### Current Status
- **Characters**: ~7.1MB (PNG + SVG)
- **Environment**: ~3.9MB (field + isometric)
- **Total**: ~11MB

### Optimization Plan
- [x] Remove SVG files (keep PNG only) → Save ~30%
- [ ] Compress PNG files (pngquant) → Target 7-8MB total
- [ ] Use texture atlases for animation frames → Improve performance
- [ ] Implement on-demand loading for unused character types

### Performance Targets
- **Load time**: <2 seconds for all character assets
- **Memory usage**: <50MB for loaded sprites
- **FPS**: 60fps stable with 22 animated players + ball

## Credits & Attribution

**Kenney** (www.kenney.nl)
- Toon Characters Pack 1 (2019-09-26)
- Sports Pack
- Isometric Blocks

**License**: CC0 - Public Domain
**Attribution**: Optional but appreciated - "Assets by Kenney (www.kenney.nl)"

## References

- **Design Proposal**: `DESIGN_PROPOSAL_2D_PIVOT.md`
- **Dev Approval**: `DESIGN_RESPONSE_2D_PIVOT.md`
- **Kenney Assets**: https://www.kenney.nl/
- **CC0 License**: http://creativecommons.org/publicdomain/zero/1.0/

---

**Last Updated**: 2025-10-16
**Maintainer**: Development Team (@yellowhama)
