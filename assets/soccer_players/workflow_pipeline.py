#!/usr/bin/env python3
"""
Soccer Player Character Generation Pipeline
============================================

Full workflow from body presets to game-ready assets.

Stages:
1. Image Generation (ComfyUI) - body preset → 2D image
2. TRELLIS 3D Conversion - 2D image → 3D GLB
3. RigAnything Auto-Rigging - 3D mesh → rigged skeleton
4. Animation Retargeting - apply shared animations
5. Asset Organization - final folder structure

Usage:
    python workflow_pipeline.py --stage all
    python workflow_pipeline.py --stage generate --preset female_tall_glamorous
    python workflow_pipeline.py --stage trellis
    python workflow_pipeline.py --stage rig
    python workflow_pipeline.py --stage organize
"""

import os
import sys
import json
import time
import shutil
import argparse
import subprocess
import urllib.request

# =============================================================================
# PATHS
# =============================================================================

COMFYUI_URL = "http://127.0.0.1:8188"
BASE_DIR = "/home/hugh/ComfyUI/soccer_game/process"
OUTPUT_DIR = "/home/hugh/footballgame_repo/assets/soccer_players"
BLENDER_PATH = "/home/hugh/blender/blender"
RIGANYTHING_DIR = f"{BASE_DIR}/stage3_trellis/RigAnything"
MIXAMO_DIR = "/home/hugh/ComfyUI/soccer_game/mixamo/fbx/Soccer Game Pack"

STAGE_DIRS = {
    "stage1_image": f"{BASE_DIR}/stage1_body_variants",
    "stage2_trellis": f"{BASE_DIR}/stage2_body_variants",
    "stage3_rigging": f"{BASE_DIR}/stage3_body_variants",
    "stage4_animation": f"{BASE_DIR}/stage4_body_variants",
}

# =============================================================================
# BODY PRESETS (imported from body_presets.py)
# =============================================================================

sys.path.insert(0, OUTPUT_DIR)
try:
    from body_presets import (
        BODY_PRESETS,
        CHARACTER_ARCHETYPES,
        MESH_SCALE_PRESETS,
        get_body_prompt,
        get_mesh_scale
    )
except ImportError:
    print("Warning: body_presets.py not found, using defaults")
    BODY_PRESETS = {}
    CHARACTER_ARCHETYPES = {}
    MESH_SCALE_PRESETS = {}

# =============================================================================
# STAGE 1: IMAGE GENERATION
# =============================================================================

def generate_character_image(name, height="average", build="athletic", figure=None,
                             gender="female", base_prompt=None):
    """Generate character image using ComfyUI with body presets"""

    from body_presets import get_body_prompt

    body_prompts = get_body_prompt(height, build, figure)

    if base_prompt is None:
        if gender == "female":
            base_prompt = "1girl, female soccer player, soccer uniform, standing pose, full body"
        else:
            base_prompt = "1boy, male soccer player, soccer uniform, standing pose, full body"

    full_positive = f"{base_prompt}, {body_prompts['positive']}, high quality, detailed, anime style"
    full_negative = f"nsfw, nude, bad anatomy, bad hands, {body_prompts['negative']}"

    workflow = {
        "3": {
            "class_type": "KSampler",
            "inputs": {
                "cfg": 7, "denoise": 1,
                "latent_image": ["5", 0], "model": ["4", 0],
                "negative": ["7", 0], "positive": ["6", 0],
                "sampler_name": "dpmpp_2m_sde", "scheduler": "karras",
                "seed": int(time.time() * 1000) % (2**32), "steps": 25
            }
        },
        "4": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "bridgeToonsMix_v80.safetensors"}},
        "5": {"class_type": "EmptyLatentImage", "inputs": {"batch_size": 1, "height": 1216, "width": 832}},
        "6": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["4", 1], "text": full_positive}},
        "7": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["4", 1], "text": full_negative}},
        "8": {"class_type": "VAEDecode", "inputs": {"samples": ["3", 0], "vae": ["4", 2]}},
        "9": {"class_type": "SaveImage", "inputs": {"filename_prefix": f"body_{name}", "images": ["8", 0]}}
    }

    # Queue to ComfyUI
    data = json.dumps({"prompt": workflow}).encode('utf-8')
    req = urllib.request.Request(f"{COMFYUI_URL}/prompt", data=data,
                                  headers={"Content-Type": "application/json"})

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            result = json.loads(resp.read().decode('utf-8'))
            prompt_id = result.get("prompt_id")

            # Wait for completion
            for _ in range(150):  # 5 min timeout
                time.sleep(2)
                try:
                    with urllib.request.urlopen(f"{COMFYUI_URL}/history/{prompt_id}") as hist_resp:
                        history = json.loads(hist_resp.read().decode('utf-8'))
                        if prompt_id in history:
                            outputs = history[prompt_id].get("outputs", {})
                            for node_output in outputs.values():
                                if "images" in node_output:
                                    img = node_output["images"][0]
                                    src = f"/home/hugh/ComfyUI/output/{img['filename']}"
                                    dst = f"{STAGE_DIRS['stage1_image']}/{name}.png"
                                    os.makedirs(STAGE_DIRS['stage1_image'], exist_ok=True)
                                    shutil.copy(src, dst)
                                    return dst
                except:
                    pass
    except Exception as e:
        print(f"  Error: {e}")

    return None

# =============================================================================
# STAGE 2: TRELLIS 3D CONVERSION
# =============================================================================

def run_trellis_conversion(image_path, output_name):
    """Convert 2D image to 3D GLB using TRELLIS"""

    os.makedirs(STAGE_DIRS['stage2_trellis'], exist_ok=True)

    # Pad image to square for TRELLIS
    from PIL import Image
    img = Image.open(image_path)

    if img.size != (1024, 1024):
        new_img = Image.new('RGBA', (1024, 1024), (255, 255, 255, 0))
        x = (1024 - img.size[0]) // 2
        y = (1024 - img.size[1]) // 2
        new_img.paste(img, (x, y))
        padded_path = f"{STAGE_DIRS['stage2_trellis']}/{output_name}_padded.png"
        new_img.save(padded_path)
    else:
        padded_path = image_path

    # Run TRELLIS
    trellis_script = f"""
import sys
sys.path.insert(0, '/home/hugh/ComfyUI/soccer_game/process/stage3_trellis/TRELLIS')
from trellis.pipelines import TrellisImageTo3DPipeline
from PIL import Image

pipe = TrellisImageTo3DPipeline.from_pretrained("JeffreyXiang/TRELLIS-image-large")
pipe.cuda()

img = Image.open("{padded_path}")
outputs = pipe.run(img, seed=42)

glb = outputs['gaussian'][0]
glb.save("{STAGE_DIRS['stage2_trellis']}/{output_name}.glb")
print("SUCCESS")
"""

    script_path = f"/tmp/trellis_{output_name}.py"
    with open(script_path, "w") as f:
        f.write(trellis_script)

    result = subprocess.run(
        ["python3", script_path],
        capture_output=True, text=True, timeout=600
    )

    if "SUCCESS" in result.stdout:
        return f"{STAGE_DIRS['stage2_trellis']}/{output_name}.glb"
    return None

# =============================================================================
# STAGE 3: RIGANYTHING AUTO-RIGGING
# =============================================================================

def run_riganything(glb_path, output_name, mesh_scale=None):
    """Auto-rig mesh using RigAnything (CPU mode for RTX 50 series)"""

    os.makedirs(STAGE_DIRS['stage3_rigging'], exist_ok=True)
    output_dir = f"{STAGE_DIRS['stage3_rigging']}/{output_name}"
    os.makedirs(output_dir, exist_ok=True)

    ra_outputs = f"{RIGANYTHING_DIR}/outputs/{output_name}"
    os.makedirs(ra_outputs, exist_ok=True)
    shutil.copy(glb_path, f"{ra_outputs}/{output_name}.glb")

    # Step 1: Simplify
    cmd1 = f'CUDA_VISIBLE_DEVICES="" {BLENDER_PATH} -b --python run_step1_simplify.py -- ' \
           f'--data_path "{ra_outputs}/{output_name}.glb" --mesh_simplify 1 --simplify_count 20000 ' \
           f'--output_path "{ra_outputs}/"'
    subprocess.run(cmd1, shell=True, cwd=RIGANYTHING_DIR, capture_output=True)

    simplified = f"{ra_outputs}/{output_name}_simplified.glb"
    if not os.path.exists(simplified):
        return None

    # Step 2: Inference
    cmd2 = f'CUDA_VISIBLE_DEVICES="" {BLENDER_PATH} -b --python run_step2_inference.py -- ' \
           f'--config config.yaml --load ckpt/riganything_ckpt.pt ' \
           f'-s inference true -s inference_out_dir "{ra_outputs}/" --mesh_path "{simplified}"'
    subprocess.run(cmd2, shell=True, cwd=RIGANYTHING_DIR, capture_output=True)

    npz_file = f"{ra_outputs}/{output_name}_simplified.npz"
    if not os.path.exists(npz_file):
        return None

    # Step 3: Export
    cmd3 = f'CUDA_VISIBLE_DEVICES="" {BLENDER_PATH} -b --python run_step3_vis.py -- ' \
           f'--data_path "{npz_file}" --save_path "{ra_outputs}/" --mesh_path "{simplified}"'
    subprocess.run(cmd3, shell=True, cwd=RIGANYTHING_DIR, capture_output=True)

    rigged = f"{ra_outputs}/{output_name}_simplified_rig.glb"
    if os.path.exists(rigged):
        final = f"{output_dir}/{output_name}_rigged.glb"
        shutil.copy(rigged, final)

        # Apply mesh scale if provided
        if mesh_scale:
            apply_mesh_scale(final, mesh_scale)

        return final
    return None

def apply_mesh_scale(glb_path, scale):
    """Apply body preset scale to rigged mesh using Blender"""

    scale_script = f"""
import bpy

bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete()

bpy.ops.import_scene.gltf(filepath="{glb_path}")

for obj in bpy.data.objects:
    if obj.type in ['MESH', 'ARMATURE']:
        obj.scale.x *= {scale['x']}
        obj.scale.y *= {scale['y']}
        obj.scale.z *= {scale['z']}
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.transform_apply(scale=True)

bpy.ops.export_scene.gltf(filepath="{glb_path}", export_format='GLB')
print("SCALE_APPLIED")
"""

    script_path = "/tmp/apply_scale.py"
    with open(script_path, "w") as f:
        f.write(scale_script)

    subprocess.run(
        [BLENDER_PATH, "-b", "--python", script_path],
        capture_output=True
    )

# =============================================================================
# STAGE 4: ASSET ORGANIZATION
# =============================================================================

def organize_assets():
    """Organize all assets into final game structure"""

    chars_dir = f"{OUTPUT_DIR}/characters"
    anims_dir = f"{OUTPUT_DIR}/animations"

    os.makedirs(chars_dir, exist_ok=True)
    os.makedirs(f"{anims_dir}/shared", exist_ok=True)
    os.makedirs(f"{anims_dir}/field_player", exist_ok=True)
    os.makedirs(f"{anims_dir}/goalkeeper", exist_ok=True)

    # Copy body variant characters
    for name in os.listdir(STAGE_DIRS['stage3_rigging']):
        src_dir = f"{STAGE_DIRS['stage3_rigging']}/{name}"
        if os.path.isdir(src_dir):
            dst_dir = f"{chars_dir}/{name}"
            os.makedirs(f"{dst_dir}/textures", exist_ok=True)

            rigged = f"{src_dir}/{name}_rigged.glb"
            if os.path.exists(rigged):
                shutil.copy(rigged, f"{dst_dir}/mesh.glb")
                print(f"  ✓ {name}/mesh.glb")

    print(f"\nAssets organized to: {OUTPUT_DIR}")

# =============================================================================
# FULL PIPELINE
# =============================================================================

def run_full_pipeline(characters):
    """Run complete pipeline for character list"""

    print("=" * 60)
    print("SOCCER PLAYER CHARACTER PIPELINE")
    print("=" * 60)

    results = []

    for i, char in enumerate(characters, 1):
        name = char['name']
        print(f"\n[{i}/{len(characters)}] {name}")
        print("-" * 40)

        # Stage 1: Generate
        print("  Stage 1: Image generation...")
        img_path = generate_character_image(
            name=name,
            height=char.get('height', 'average'),
            build=char.get('build', 'athletic'),
            figure=char.get('figure'),
            gender=char.get('gender', 'female')
        )

        if not img_path:
            print("  ✗ Image generation failed")
            results.append({"name": name, "status": "failed_stage1"})
            continue
        print(f"  ✓ Image: {os.path.basename(img_path)}")

        # Stage 2: TRELLIS
        print("  Stage 2: TRELLIS 3D conversion...")
        glb_path = run_trellis_conversion(img_path, name)

        if not glb_path:
            print("  ✗ TRELLIS failed")
            results.append({"name": name, "status": "failed_stage2"})
            continue
        print(f"  ✓ GLB: {os.path.basename(glb_path)}")

        # Stage 3: RigAnything
        print("  Stage 3: RigAnything rigging...")
        mesh_scale = get_mesh_scale(char.get('height', 'average'), char.get('build', 'athletic'))
        rigged_path = run_riganything(glb_path, name, mesh_scale)

        if not rigged_path:
            print("  ✗ Rigging failed")
            results.append({"name": name, "status": "failed_stage3"})
            continue
        print(f"  ✓ Rigged: {os.path.basename(rigged_path)}")

        results.append({"name": name, "status": "success", "path": rigged_path})

    # Stage 4: Organize
    print("\n" + "-" * 40)
    print("Stage 4: Organizing assets...")
    organize_assets()

    # Summary
    success = sum(1 for r in results if r["status"] == "success")
    print(f"\n{'=' * 60}")
    print(f"PIPELINE COMPLETE: {success}/{len(characters)}")
    print("=" * 60)

    return results

# =============================================================================
# CLI
# =============================================================================

def main():
    parser = argparse.ArgumentParser(description="Soccer Player Character Pipeline")
    parser.add_argument("--stage", choices=["all", "generate", "trellis", "rig", "organize"],
                        default="all", help="Pipeline stage to run")
    parser.add_argument("--preset", help="Character archetype preset name")
    parser.add_argument("--list-presets", action="store_true", help="List available presets")

    args = parser.parse_args()

    if args.list_presets:
        print("\n=== Available Archetypes ===")
        for name, arch in CHARACTER_ARCHETYPES.items():
            print(f"  {name}: {arch.get('description', '')}")
        return

    # Default characters (body variants)
    default_chars = [
        {"name": "male_tall_muscular", "height": "tall", "build": "muscular", "gender": "male"},
        {"name": "male_tall_slim", "height": "tall", "build": "slim", "gender": "male"},
        {"name": "male_short_athletic", "height": "short", "build": "athletic", "gender": "male"},
        {"name": "female_tall_slender", "height": "tall", "build": "slim", "figure": "slender", "gender": "female"},
        {"name": "female_tall_glamorous", "height": "tall", "build": "athletic", "figure": "glamorous", "gender": "female"},
        {"name": "female_short_athletic", "height": "short", "build": "athletic", "figure": "standard", "gender": "female"},
    ]

    if args.stage == "all":
        run_full_pipeline(default_chars)
    elif args.stage == "generate":
        for char in default_chars:
            generate_character_image(**char)
    elif args.stage == "trellis":
        for f in os.listdir(STAGE_DIRS['stage1_image']):
            if f.endswith('.png'):
                run_trellis_conversion(f"{STAGE_DIRS['stage1_image']}/{f}", f.replace('.png', ''))
    elif args.stage == "rig":
        for f in os.listdir(STAGE_DIRS['stage2_trellis']):
            if f.endswith('.glb') and '_padded' not in f:
                run_riganything(f"{STAGE_DIRS['stage2_trellis']}/{f}", f.replace('.glb', ''))
    elif args.stage == "organize":
        organize_assets()

if __name__ == "__main__":
    main()
