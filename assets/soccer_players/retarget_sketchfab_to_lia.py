"""
Retarget Sketchfab Soccer Animations to LIA Character
Rigify Metarig -> RigAnything bone mapping
"""
import bpy
import os

# Paths
LIA_FBX = "/home/hugh/ComfyUI/soccer_game/process/stage4_rigging/lia_spread_legs_rigged_clean.fbx"
OUTPUT_DIR = "/home/hugh/ComfyUI/soccer_game/process/stage5_animation"

# Rigify -> RigAnything bone mapping
BONE_MAP = {
    # Core spine
    "Bone_0": "spine",           # Hips/Root
    "Bone_1": "spine.001",       # Spine lower
    "Bone_10": "spine.002",      # Spine mid (chest)

    # Neck/Head
    "Bone_3": "spine.004",       # Neck
    "Bone_12": "spine.006",      # Head

    # Left Leg
    "Bone_4": "thigh.L",         # LeftUpLeg
    "Bone_13": "shin.L",         # LeftLeg
    "Bone_20": "foot.L",         # LeftFoot
    "Bone_25": "toe.L",          # LeftToeBase

    # Right Leg
    "Bone_5": "thigh.R",         # RightUpLeg
    "Bone_14": "shin.R",         # RightLeg
    "Bone_21": "foot.R",         # RightFoot
    "Bone_26": "toe.R",          # RightToeBase

    # Left Arm
    "Bone_6": "shoulder.L",      # LeftShoulder
    "Bone_15": "upper_arm.L",    # LeftArm
    "Bone_22": "forearm.L",      # LeftForeArm
    "Bone_27": "hand.L",         # LeftHand

    # Right Arm
    "Bone_7": "shoulder.R",      # RightShoulder
    "Bone_16": "upper_arm.R",    # RightArm
    "Bone_23": "forearm.R",      # RightForeArm
    "Bone_28": "hand.R",         # RightHand
}

# Animations to retarget
ANIMATIONS = [
    "Soccer Idle",
    "Soccer Running",
    "Soccer Kick",
    "Soccer Pass",
    "Soccer Goalkeeper Idle",
    "Soccer Goalkeeper Catch",
]

def setup_retarget_constraints(lia_armature, source_armature):
    """Set up copy rotation constraints from source to LIA"""
    bpy.context.view_layer.objects.active = lia_armature
    bpy.ops.object.mode_set(mode='POSE')

    constraints_created = 0
    for lia_bone, source_bone in BONE_MAP.items():
        if lia_bone in lia_armature.pose.bones and source_bone in source_armature.pose.bones:
            lia_pbone = lia_armature.pose.bones[lia_bone]

            # Clear existing constraints
            for c in lia_pbone.constraints:
                lia_pbone.constraints.remove(c)

            # Add Copy Rotation
            constraint = lia_pbone.constraints.new('COPY_ROTATION')
            constraint.name = "Retarget_Rotation"
            constraint.target = source_armature
            constraint.subtarget = source_bone
            constraint.mix_mode = 'REPLACE'
            constraint.target_space = 'LOCAL'
            constraint.owner_space = 'LOCAL'

            constraints_created += 1
            print(f"  {lia_bone} <- {source_bone}")

    bpy.ops.object.mode_set(mode='OBJECT')
    return constraints_created

def bake_animation(lia_armature, frame_start, frame_end):
    """Bake animation to LIA armature"""
    bpy.context.scene.frame_start = frame_start
    bpy.context.scene.frame_end = frame_end

    bpy.ops.object.select_all(action='DESELECT')
    lia_armature.select_set(True)
    bpy.context.view_layer.objects.active = lia_armature
    bpy.ops.object.mode_set(mode='POSE')
    bpy.ops.pose.select_all(action='SELECT')

    bpy.ops.nla.bake(
        frame_start=frame_start,
        frame_end=frame_end,
        only_selected=True,
        visual_keying=True,
        clear_constraints=True,
        use_current_action=False,
        bake_types={'POSE'}
    )

    bpy.ops.object.mode_set(mode='OBJECT')

def export_fbx(lia_armature, lia_mesh, output_path):
    """Export LIA with baked animation"""
    bpy.ops.object.select_all(action='DESELECT')
    if lia_mesh:
        lia_mesh.select_set(True)
    lia_armature.select_set(True)
    bpy.context.view_layer.objects.active = lia_armature

    bpy.ops.export_scene.fbx(
        filepath=output_path,
        use_selection=True,
        add_leaf_bones=False,
        bake_anim=True,
        bake_anim_use_all_bones=True,
        bake_anim_use_nla_strips=False,
        bake_anim_use_all_actions=False,
        path_mode='COPY',
        embed_textures=True
    )

def retarget_animation(source_armature, action_name, output_name):
    """Retarget a single animation"""
    print(f"\n{'='*60}")
    print(f"RETARGETING: {action_name}")
    print(f"{'='*60}")

    # Import fresh LIA
    bpy.ops.import_scene.fbx(filepath=LIA_FBX)

    lia_armature = None
    lia_mesh = None
    for obj in bpy.data.objects:
        if obj.type == 'ARMATURE' and 'Metarig' not in obj.name:
            if obj.name != source_armature.name:
                lia_armature = obj
        elif obj.type == 'MESH' and 'Soccer' not in obj.name:
            lia_mesh = obj

    if not lia_armature:
        print(f"  [ERROR] LIA armature not found!")
        return False

    print(f"  LIA Armature: {lia_armature.name}")

    # Set action on source armature
    action = bpy.data.actions.get(action_name)
    if not action:
        print(f"  [ERROR] Action not found: {action_name}")
        return False

    source_armature.animation_data_create()
    source_armature.animation_data.action = action

    frame_start = int(action.frame_range[0])
    frame_end = int(action.frame_range[1])
    print(f"  Frames: {frame_start} - {frame_end}")

    # Position armatures together
    lia_armature.location = source_armature.location

    # Setup constraints
    print("\n  Setting up constraints...")
    constraints = setup_retarget_constraints(lia_armature, source_armature)
    print(f"  Total constraints: {constraints}")

    # Bake
    print("\n  Baking animation...")
    bake_animation(lia_armature, frame_start, frame_end)

    # Export
    output_path = os.path.join(OUTPUT_DIR, output_name)
    print(f"\n  Exporting: {output_path}")
    export_fbx(lia_armature, lia_mesh, output_path)

    # Cleanup LIA objects for next iteration
    bpy.data.objects.remove(lia_armature, do_unlink=True)
    if lia_mesh:
        bpy.data.objects.remove(lia_mesh, do_unlink=True)

    return True

# Main
print("="*60)
print("SKETCHFAB -> LIA RETARGETING")
print("="*60)

os.makedirs(OUTPUT_DIR, exist_ok=True)

# Get first source armature
source_armature = None
for obj in bpy.data.objects:
    if obj.type == 'ARMATURE' and 'Metarig' in obj.name:
        source_armature = obj
        break

if not source_armature:
    print("[ERROR] No source armature found!")
else:
    print(f"Source Armature: {source_armature.name}")

    successful = 0
    for anim_name in ANIMATIONS:
        output_name = f"lia_{anim_name.lower().replace(' ', '_')}.fbx"
        if retarget_animation(source_armature, anim_name, output_name):
            successful += 1
            print(f"  [OK] {output_name}")

    print(f"\n{'='*60}")
    print(f"COMPLETE: {successful}/{len(ANIMATIONS)} animations")
    print(f"Output: {OUTPUT_DIR}")
    print(f"{'='*60}")
