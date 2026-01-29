@tool
extends Node

# SPUM Asset Exporter
# Renders assembled components (Body, Hair, Cloth) into static PNGs
# Usage: Run this scene in Godot Editor.

@onready var viewport: SubViewport = $SubViewport
@onready var spum_preview: SPUMPreviewSprite = $SubViewport/SPUMPreviewSprite

const OUTPUT_BASE = "F:/Aisaak/Projects/2d_asset/soonsoon/png_out/PNG"


func _ready():
	# Wait for first frame
	await get_tree().process_frame
	await get_tree().process_frame

	_export_naked_bodies()
	_export_hair_sets()
	_export_cloth_sets()

	print("Export Complete!")
	# get_tree().quit() # Uncomment to auto-close


func _export_naked_bodies():
	print("Exporting Bodies...")
	var bodies = _scan_files("bases/BodySource")
	for body_id in bodies:
		# Setup: Body ONLY
		spum_preview.set_appearance_data(
			{"body": body_id, "hair": "", "face_hair": "", "cloth": "", "pant": "", "helmet": "", "back": ""}
		)
		await _save_screenshot("1_Bodies", body_id)


func _export_hair_sets():
	print("Exporting Hair...")
	var hairs = _scan_files("items/hair/0_Hair")
	for hair_id in hairs:
		# Setup: Hair ONLY (Invisible Body?)
		# SPUMPreview doesn't support hiding body yet, need to add feature or just set empty
		spum_preview.set_appearance_data(
			{"body": "", "hair": hair_id, "face_hair": "", "cloth": "", "pant": "", "helmet": "", "back": ""}  # Empty body
		)
		await _save_screenshot("2_Hairs", hair_id)


func _export_cloth_sets():
	print("Exporting Cloths...")
	var cloths = _scan_files("items/cloth/2_Cloth")
	for cloth_id in cloths:
		spum_preview.set_appearance_data(
			{"body": "", "hair": "", "face_hair": "", "cloth": cloth_id, "pant": "", "helmet": "", "back": ""}  # Separate pants? User asked for "Clothes" generally. Doing Tops first.
		)
		await _save_screenshot("3_Clothes_Top", cloth_id)


func _scan_files(folder):
	# (Reuse scan logic from Step2_Appearance or simplify)
	var list = []
	var path = "res://assets/sprites/spum_modern/" + folder
	var dir = DirAccess.open(path)
	if dir:
		dir.list_dir_begin()
		var n = dir.get_next()
		while n != "":
			if not dir.current_is_dir() and n.ends_with(".png"):
				list.append(n.get_basename())
			n = dir.get_next()
	return list


func _save_screenshot(folder_name, file_name):
	await get_tree().process_frame
	await get_tree().process_frame  # Wait for render

	var img = viewport.get_texture().get_image()
	var dir = DirAccess.open(OUTPUT_BASE)
	if not dir.dir_exists(folder_name):
		dir.make_dir(folder_name)

	var path = "%s/%s/%s.png" % [OUTPUT_BASE, folder_name, file_name]
	img.save_png(path)
	print("Saved: ", path)
