#!/usr/bin/env python3
"""
Body Type Preset System for Character Generation
Used with ComfyUI to generate varied character body types
"""

# =============================================================================
# BODY PRESET DEFINITIONS
# =============================================================================

BODY_PRESETS = {
    # HEIGHT PRESETS
    "height": {
        "tall": {
            "prompt": "tall, long legs, elongated proportions",
            "negative": "short, petite, compact",
            "scale_factor": 1.1  # For 3D scaling reference
        },
        "short": {
            "prompt": "petite, compact build, shorter stature",
            "negative": "tall, elongated, long legs",
            "scale_factor": 0.9
        },
        "average": {
            "prompt": "average height, balanced proportions",
            "negative": "",
            "scale_factor": 1.0
        }
    },

    # BUILD PRESETS (체형)
    "build": {
        "slim": {  # 마름
            "prompt": "slim build, lean body, slender frame, thin",
            "negative": "muscular, bulky, thick, chubby",
            "mesh_modifier": "scale_x:0.9"
        },
        "athletic": {  # 탄탄
            "prompt": "athletic build, toned muscles, fit body, defined muscles",
            "negative": "skinny, frail, overweight",
            "mesh_modifier": "scale_x:1.0"
        },
        "muscular": {  # 근육질
            "prompt": "muscular build, strong physique, broad shoulders, powerful",
            "negative": "thin, weak, skinny",
            "mesh_modifier": "scale_x:1.15"
        }
    },

    # FIGURE PRESETS (몸매) - primarily for female characters
    "figure": {
        "slender": {  # 슬렌더
            "prompt": "slender figure, model-like proportions, elegant silhouette",
            "negative": "curvy, voluptuous, thick",
            "applies_to": "female"
        },
        "glamorous": {  # 글래머
            "prompt": "glamorous figure, curvy, feminine curves, hourglass shape",
            "negative": "flat, boyish, straight figure",
            "applies_to": "female"
        },
        "standard": {
            "prompt": "balanced proportions, natural figure",
            "negative": "",
            "applies_to": "all"
        }
    }
}

# =============================================================================
# PRESET COMBINATIONS (Common combinations for soccer players)
# =============================================================================

CHARACTER_ARCHETYPES = {
    # MALE ARCHETYPES
    "striker_agile": {
        "height": "tall",
        "build": "athletic",
        "description": "Fast striker, lean and quick"
    },
    "striker_power": {
        "height": "tall",
        "build": "muscular",
        "description": "Target man, strong in the air"
    },
    "midfielder_playmaker": {
        "height": "average",
        "build": "slim",
        "description": "Creative midfielder, technical player"
    },
    "midfielder_box2box": {
        "height": "average",
        "build": "athletic",
        "description": "All-around midfielder, balanced"
    },
    "defender_stopper": {
        "height": "tall",
        "build": "muscular",
        "description": "Center back, dominant physically"
    },
    "defender_agile": {
        "height": "average",
        "build": "athletic",
        "description": "Full back, fast and agile"
    },
    "goalkeeper_tall": {
        "height": "tall",
        "build": "athletic",
        "description": "Shot stopper, long reach"
    },

    # FEMALE ARCHETYPES
    "female_striker_fast": {
        "height": "tall",
        "build": "slim",
        "figure": "slender",
        "description": "Speed striker"
    },
    "female_striker_power": {
        "height": "tall",
        "build": "athletic",
        "figure": "glamorous",
        "description": "Physical striker"
    },
    "female_midfielder": {
        "height": "average",
        "build": "athletic",
        "figure": "slender",
        "description": "Technical midfielder"
    },
    "female_defender": {
        "height": "tall",
        "build": "athletic",
        "figure": "standard",
        "description": "Solid defender"
    },
    "female_goalkeeper": {
        "height": "tall",
        "build": "athletic",
        "figure": "slender",
        "description": "Agile goalkeeper"
    },

    # PROTAGONIST (LIA)
    "lia_default": {
        "height": "average",
        "build": "athletic",
        "figure": "glamorous",
        "description": "Main character - balanced with presence"
    }
}

# =============================================================================
# HELPER FUNCTIONS
# =============================================================================

def get_body_prompt(height="average", build="athletic", figure=None):
    """Generate combined prompt for body type"""
    prompts = []
    negatives = []

    if height in BODY_PRESETS["height"]:
        h = BODY_PRESETS["height"][height]
        prompts.append(h["prompt"])
        if h["negative"]:
            negatives.append(h["negative"])

    if build in BODY_PRESETS["build"]:
        b = BODY_PRESETS["build"][build]
        prompts.append(b["prompt"])
        if b["negative"]:
            negatives.append(b["negative"])

    if figure and figure in BODY_PRESETS["figure"]:
        f = BODY_PRESETS["figure"][figure]
        prompts.append(f["prompt"])
        if f["negative"]:
            negatives.append(f["negative"])

    return {
        "positive": ", ".join(prompts),
        "negative": ", ".join(negatives)
    }

def get_archetype_prompt(archetype_name):
    """Get full prompt for a character archetype"""
    if archetype_name not in CHARACTER_ARCHETYPES:
        return None

    arch = CHARACTER_ARCHETYPES[archetype_name]
    return get_body_prompt(
        height=arch.get("height", "average"),
        build=arch.get("build", "athletic"),
        figure=arch.get("figure")
    )

def list_archetypes():
    """List all available archetypes"""
    print("\n=== MALE ARCHETYPES ===")
    for name, arch in CHARACTER_ARCHETYPES.items():
        if "female" not in name and "lia" not in name:
            print(f"  {name}: {arch['description']}")

    print("\n=== FEMALE ARCHETYPES ===")
    for name, arch in CHARACTER_ARCHETYPES.items():
        if "female" in name or "lia" in name:
            print(f"  {name}: {arch['description']}")

# =============================================================================
# MESH SCALING REFERENCE (for Blender/Godot)
# =============================================================================

MESH_SCALE_PRESETS = {
    "tall_slim": {"x": 0.95, "y": 1.0, "z": 1.1},
    "tall_athletic": {"x": 1.0, "y": 1.0, "z": 1.1},
    "tall_muscular": {"x": 1.1, "y": 1.05, "z": 1.1},
    "average_slim": {"x": 0.9, "y": 1.0, "z": 1.0},
    "average_athletic": {"x": 1.0, "y": 1.0, "z": 1.0},
    "average_muscular": {"x": 1.1, "y": 1.05, "z": 1.0},
    "short_slim": {"x": 0.9, "y": 1.0, "z": 0.92},
    "short_athletic": {"x": 1.0, "y": 1.0, "z": 0.92},
    "short_muscular": {"x": 1.05, "y": 1.0, "z": 0.92},
}

def get_mesh_scale(height="average", build="athletic"):
    """Get mesh scale factors for body type"""
    key = f"{height}_{build}"
    return MESH_SCALE_PRESETS.get(key, {"x": 1.0, "y": 1.0, "z": 1.0})

# =============================================================================
# TEST
# =============================================================================

if __name__ == "__main__":
    print("=" * 60)
    print("BODY PRESET SYSTEM")
    print("=" * 60)

    list_archetypes()

    print("\n=== EXAMPLE PROMPTS ===")

    # Example: tall athletic female
    result = get_body_prompt("tall", "athletic", "glamorous")
    print(f"\nTall + Athletic + Glamorous:")
    print(f"  Positive: {result['positive']}")
    print(f"  Negative: {result['negative']}")

    # Example: short slim male
    result = get_body_prompt("short", "slim")
    print(f"\nShort + Slim:")
    print(f"  Positive: {result['positive']}")
    print(f"  Negative: {result['negative']}")

    # Archetype example
    print("\n=== ARCHETYPE EXAMPLE ===")
    arch_prompt = get_archetype_prompt("female_striker_power")
    print(f"female_striker_power:")
    print(f"  Positive: {arch_prompt['positive']}")
    print(f"  Negative: {arch_prompt['negative']}")

    print("\n=== MESH SCALES ===")
    for key, scale in MESH_SCALE_PRESETS.items():
        print(f"  {key}: x={scale['x']}, y={scale['y']}, z={scale['z']}")
