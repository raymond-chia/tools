extends "res://tests/features/battle/battle_test_suite.gd"

const TEST_DEFINITIONS := "res://tests/features/battle/data/combat_presentation_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/combat_presentation_exploration_map.toml"

var battle
var runner: GdUnitSceneRunner
var camera: MapCamera

# 準備本案例的真實戰鬥場景與專用資源。
func setup_test() -> void:
	runner = battle_scene_runner(auto_free(create_battle(TEST_DEFINITIONS, TEST_MAP, PRESENTATION_RANDOM_SEED)))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	camera = battle.world.camera
	# 固定 delta 驗證輸入位移，避免場景 _process 同時累加相機位置。
	battle.world.set_process(false)

# 清理本案例的專用資源與尚未完成的操作。
func cleanup_test() -> void:
	for key in [KEY_W, KEY_A, KEY_S, KEY_D]:
		set_key(key, false)

# 驗證 WASD、斜向及反向按鍵的移動方向、固定速度與放開後停止。
func test_keyboard_navigation_uses_normalized_direction() -> void:
	var cases := [
		{"keys": [KEY_W], "direction": Vector2.UP},
		{"keys": [KEY_A], "direction": Vector2.LEFT},
		{"keys": [KEY_S], "direction": Vector2.DOWN},
		{"keys": [KEY_D], "direction": Vector2.RIGHT},
		{"keys": [KEY_W, KEY_D], "direction": Vector2(1, -1).normalized()},
		{"keys": [KEY_A, KEY_D], "direction": Vector2.ZERO},
	]
	for test_case in cases:
		camera.center_on_board()
		var start := camera.position
		for key in test_case.keys:
			set_key(key, true)
		camera.move_with_input(0.01)
		var expected: Vector2 = start + test_case.direction * BattleConfig.CAMERA_MOVE_SPEED * 0.01
		assert_bool(camera.position.is_equal_approx(expected)).override_failure_message("按鍵 %s 應以固定速度移動" % [test_case.keys]).is_true()
		for key in test_case.keys:
			set_key(key, false)
		var stopped_position := camera.position
		camera.move_with_input(0.01)
		assert_vector(camera.position).is_equal(stopped_position)

# 驗證鍵盤移動與聚焦都限制在真實地圖邊界，手動移動會終止既有聚焦 Tween。
func test_camera_bounds_and_manual_focus_interruption() -> void:
	assert_bool(camera.bounds.has_area()).is_true()
	for test_case in [{"key": KEY_A, "edge": camera.bounds.position.x}, {"key": KEY_D, "edge": camera.bounds.end.x}]:
		camera.center_on_board()
		set_key(test_case.key, true)
		camera.move_with_input(100.0)
		set_key(test_case.key, false)
		assert_float(camera.position.x).is_equal(test_case.edge)
	camera.focus_on(camera.bounds.end + Vector2(1000, 1000))
	await camera.focus_tween.finished
	assert_vector(camera.position).is_equal(camera.bounds.end)
	camera.center_on_board()
	camera.focus_on(camera.bounds.end)
	var focus: Tween = camera.focus_tween
	set_key(KEY_A, true)
	camera.move_with_input(0.01)
	set_key(KEY_A, false)
	assert_bool(focus.is_valid()).is_false()
	var manual_position := camera.position
	await runner.simulate_frames(20)
	assert_vector(camera.position).is_equal(manual_position)

# 驗證雙擊目前頭像或其他排序單位，都會選取目標、更新查看標記並將移開的鏡頭聚焦回目標。
func test_double_click_unit_controls_selects_and_focuses_unit() -> void:
	assert_str(battle.state.battle_mode).is_equal("exploring")
	var cases := [
		{"name": "目前單位頭像", "unit_id": 1, "control": "current", "cell": Vector2i(0, 1), "click_ratio": Vector2(0.5, 0.5)},
		# 目前單位區會蓋住排序按鈕下半部，點擊上緣仍可見的圖示區。
		{"name": "其他排序單位", "unit_id": 2, "control": "turn_order", "cell": Vector2i(0, 3), "click_ratio": Vector2(0.5, 0.1)},
	]
	for test_case in cases:
		runner = await replace_battle(runner, TEST_DEFINITIONS, TEST_MAP)
		battle = runner.scene()
		camera = battle.world.camera
		battle.world.set_process(false)
		battle.world.inspection_clicked.emit(1, Vector2i(0, 1))
		var target: Dictionary = battle.world.unit_with_id(battle.state.units, test_case.unit_id)
		var expected: Vector2 = camera.clamp_to_bounds(battle.world.to_global(battle.world.footprint_center(target)))
		camera.position = camera.bounds.end if not camera.bounds.end.is_equal_approx(expected) else camera.bounds.position
		assert_bool(camera.position.is_equal_approx(expected)).override_failure_message(test_case.name).is_false()
		var controls := {"current": battle.ui.actor_portrait, "turn_order": battle.ui.turn_order_slots[2].get_node("Button")}
		var control: Control = controls[test_case.control]
		await double_click_control(control, control.size * test_case.click_ratio)
		assert_int(battle.state.turn.actor).override_failure_message(test_case.name).is_equal(test_case.unit_id)
		assert_vector(battle.inspected_cell).override_failure_message(test_case.name).is_equal(test_case.cell)
		assert_object(camera.focus_tween).override_failure_message(test_case.name).is_not_null()
		await camera.focus_tween.finished
		assert_vector(camera.position).override_failure_message(test_case.name).is_equal(expected)
		for unit_id in [1, 2]:
			assert_bool(battle.world.unit_nodes[unit_id].get_node("Visual/Selection").visible).override_failure_message(test_case.name).is_equal(unit_id == test_case.unit_id)

func double_click_control(control: Control, local_position: Vector2) -> void:
	await runner.simulate_frames(1)
	var screen_position: Vector2 = control.get_viewport().get_screen_transform() * control.get_global_transform_with_canvas() * local_position
	runner.simulate_mouse_move(screen_position)
	await runner.simulate_frames(1)
	runner.simulate_mouse_button_pressed(MOUSE_BUTTON_LEFT)
	runner.simulate_mouse_button_pressed(MOUSE_BUTTON_LEFT, true)

func set_key(key: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = key
	event.physical_keycode = key
	event.pressed = pressed
	Input.parse_input_event(event)
	Input.flush_buffered_events()
