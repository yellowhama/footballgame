@tool
extends SceneTree


func _init():
	var path = "F:/Aisaak/Projects/2d_asset/downloaded_asset/potagon/pixel/CharacterSheets/CharacterSheets/Color1/Down.png"
	if FileAccess.file_exists(path):
		var img = Image.load_from_file(path)
		if img:
			print("IMAGE_SIZE: ", img.get_size())
		else:
			print("Failed to load image")
	else:
		print("File not found")
	quit()
