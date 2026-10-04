extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/test_setup.gd")
const TEST_DEFINITIONS := "res://tests/features/battle/data/combat_presentation_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/combat_presentation_exploration_map.toml"

var battle
var runner: GdUnitSceneRunner
var camera: MapCamera

func before_test() -> void:
	runner = scene_runner(auto_free(BattleTestSetup.create_battle(TEST_DEFINITIONS, TEST_MAP, BattleTestSetup.PRESENTATION_RANDOM_SEED)))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	camera = battle.world.camera
	# 固定 delta 驗證輸入位移，避免場景 _process 同時累加相機位置。
	battle.world.set_process(false)

func after_test() -> void:
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

# 驗證點擊探索回合列切換隊員、移動查看標記並聚焦到所選單位。
func test_turn_order_button_selects_and_focuses_unit() -> void:
	assert_str(battle.state.battle_mode).is_equal("exploring")
	battle.world.inspection_clicked.emit(1, Vector2i(0, 1))
	var target: Dictionary = {}
	for unit in battle.state.units:
		if unit.id == 2:
			target = unit
	assert_dict(target).is_not_empty()
	var button: Button
	for candidate in battle.ui.turn_order.find_children("*", "Button", true, false):
		if candidate.tooltip_text == tr(BattleConfig.unit_name_key(target.unit_type)):
			button = candidate
	assert_object(button).is_not_null()
	var expected: Vector2 = battle.world.to_global(battle.world.footprint_center(target))
	button.pressed.emit()
	assert_int(battle.state.turn.actor).is_equal(2)
	assert_vector(battle.inspected_cell).is_equal(Vector2i(0, 3))
	await camera.focus_tween.finished
	assert_vector(camera.position).is_equal(camera.clamp_to_bounds(expected))
	assert_bool(battle.world.unit_nodes[2].get_node("Visual/Selection").visible).is_true()
	assert_bool(battle.world.unit_nodes[1].get_node("Visual/Selection").visible).is_false()

func set_key(key: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = key
	event.physical_keycode = key
	event.pressed = pressed
	Input.parse_input_event(event)
	Input.flush_buffered_events()
