extends SceneTree


func _init():
	print("=== Button Connection Test ===")

	# HomeImproved 씬 로드
	var scene = load("res://scenes/HomeImproved.tscn")
	if scene == null:
		print("ERROR: HomeImproved.tscn not found!")
		quit()
		return

	var instance = scene.instantiate()
	if instance == null:
		print("ERROR: Failed to instantiate HomeImproved scene!")
		quit()
		return

	# 씬 트리에 추가
	get_root().add_child(instance)

	# 버튼들 찾기
	var training_btn = instance.get_node_or_null("ScrollContainer/MainContent/QuickActions/VBox/Buttons/TrainingButton")
	var rest_btn = instance.get_node_or_null("ScrollContainer/MainContent/QuickActions/VBox/Buttons/RestButton")
	var go_out_btn = instance.get_node_or_null("ScrollContainer/MainContent/QuickActions/VBox/Buttons/GoOutButton")
	var status_btn = instance.get_node_or_null("BottomBar/ButtonContainer/StatusButton")
	var bottom_training_btn = instance.get_node_or_null("BottomBar/ButtonContainer/TrainingButton")
	var advance_btn = instance.get_node_or_null("BottomBar/ButtonContainer/AdvanceButton")
	var save_btn = instance.get_node_or_null("BottomBar/ButtonContainer/SaveButton")

	print("Training button found: ", training_btn != null)
	print("Rest button found: ", rest_btn != null)
	print("Go out button found: ", go_out_btn != null)
	print("Status button found: ", status_btn != null)
	print("Bottom training button found: ", bottom_training_btn != null)
	print("Advance button found: ", advance_btn != null)
	print("Save button found: ", save_btn != null)

	# 버튼 연결 테스트
	if training_btn:
		training_btn.pressed.connect(_on_training_pressed)
		print("Training button connected successfully")

	if rest_btn:
		rest_btn.pressed.connect(_on_rest_pressed)
		print("Rest button connected successfully")

	if go_out_btn:
		go_out_btn.pressed.connect(_on_go_out_pressed)
		print("Go out button connected successfully")

	if status_btn:
		status_btn.pressed.connect(_on_status_pressed)
		print("Status button connected successfully")

	if bottom_training_btn:
		bottom_training_btn.pressed.connect(_on_training_pressed)
		print("Bottom training button connected successfully")

	if advance_btn:
		advance_btn.pressed.connect(_on_advance_pressed)
		print("Advance button connected successfully")

	if save_btn:
		save_btn.pressed.connect(_on_save_pressed)
		print("Save button connected successfully")

	print("=== Test Complete ===")
	quit()


func _on_training_pressed():
	print("TEST: Training button pressed!")


func _on_rest_pressed():
	print("TEST: Rest button pressed!")


func _on_go_out_pressed():
	print("TEST: Go out button pressed!")


func _on_status_pressed():
	print("TEST: Status button pressed!")


func _on_advance_pressed():
	print("TEST: Advance button pressed!")


func _on_save_pressed():
	print("TEST: Save button pressed!")
