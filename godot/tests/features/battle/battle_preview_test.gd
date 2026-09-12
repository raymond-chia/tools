extends GdUnitTestSuite

const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITION := "res://tests/features/battle/data/battle_preview.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(BATTLE_SCENE)
	await runner.simulate_frames(1)
	battle = runner.scene().get_node("Content")

# 驗證四種技能的施放範圍邊界正確，且選擇技能會隱藏移動範圍並清除路徑預覽。
func test_skill_range_preview() -> void:
	var test_data := [
		{"name": "近戰攻擊", "action": "melee_attack", "rect": battle.MELEE_RECT, "range": 1},
		{"name": "遠程攻擊", "action": "ranged_attack", "rect": battle.RANGED_RECT, "range": 3},
		{"name": "強力一擊", "action": "power_strike", "rect": battle.POWER_STRIKE_RECT, "range": 1},
		{"name": "瞄準射擊", "action": "aimed_shot", "rect": battle.AIMED_SHOT_RECT, "range": 4},
	]
	for test_case in test_data:
		prepare_case(battle)
		var actor_cell := actor_cell(battle)
		push_mouse_motion(battle, battle.cell_center(actor_cell + Vector2i(3, 0)))
		assert_bool(battle.first_move_path.is_empty()).override_failure_message("%s：選擇技能前應有移動路徑預覽" % test_case.name).is_false()

		push_left_click(battle, test_case.rect.get_center())

		var preview_cells := cells_from_values(battle.selected_skill_range())
		assert_bool(preview_cells.has(actor_cell + Vector2i(test_case.range, 0))).override_failure_message("%s：射程邊界格應包含在預覽" % test_case.name).is_true()
		assert_bool(preview_cells.has(actor_cell + Vector2i(test_case.range + 1, 0))).override_failure_message("%s：射程外一格不應包含在預覽" % test_case.name).is_false()
		assert_bool(battle.first_move_path.is_empty() and battle.second_move_path.is_empty()).override_failure_message("%s：選擇技能後移動路徑應消失" % test_case.name).is_true()
		push_mouse_motion(battle, battle.cell_center(actor_cell + Vector2i(2, 0)))
		assert_bool(battle.first_move_path.is_empty() and battle.second_move_path.is_empty()).override_failure_message("%s：技能待選時不應重新產生移動路徑" % test_case.name).is_true()

# 驗證移動範圍會區分第一段、第二段與兩段外的格子。
func test_two_stage_movement_range_preview() -> void:
	var test_data := [
		{"name": "第一段邊界", "offset": Vector2i(2, 0), "first": true, "second": false},
		{"name": "第二段範圍", "offset": Vector2i(3, 0), "first": false, "second": true},
		{"name": "兩段範圍外", "offset": Vector2i(5, 0), "first": false, "second": false},
	]
	for test_case in test_data:
		prepare_case(battle)
		var cell: Vector2i = actor_cell(battle) + Vector2i(test_case.offset)
		var first_cells := cells_from_values(battle.state.reachable)
		var second_cells := cells_from_values(battle.state.second_reachable)
		assert_bool(first_cells.has(cell)).override_failure_message("%s：第一段範圍歸屬應正確" % test_case.name).is_equal(test_case.first)
		assert_bool(second_cells.has(cell)).override_failure_message("%s：第二段範圍歸屬應正確" % test_case.name).is_equal(test_case.second)

# 驗證滑鼠懸停會產生正確分段的移動路徑，且不可達格不會留下路徑。
func test_movement_path_preview() -> void:
	var test_data := [
		{"name": "第一段路徑", "offset": Vector2i(2, 0), "first": [0, 1, 2], "second": []},
		{"name": "第二段路徑", "offset": Vector2i(3, 0), "first": [0, 1, 2], "second": [2, 3]},
		{"name": "不可達路徑", "offset": Vector2i(5, 0), "first": [], "second": []},
	]
	for test_case in test_data:
		prepare_case(battle)
		var origin := actor_cell(battle)
		push_mouse_motion(battle, battle.cell_center(origin + test_case.offset))
		assert_array(path_x_offsets(battle.first_move_path, origin)).override_failure_message("%s：第一段路徑應正確" % test_case.name).is_equal(test_case.first)
		assert_array(path_x_offsets(battle.second_move_path, origin)).override_failure_message("%s：第二段路徑應正確" % test_case.name).is_equal(test_case.second)

# 驗證單次點擊可抵達第一段或第二段目的地，並正確扣除兩段移動力。
func test_single_click_movement() -> void:
	var test_data := [
		{"name": "抵達第一段", "offset": Vector2i(2, 0), "remaining": 0.0, "phase": "aftermove"},
		{"name": "抵達第二段", "offset": Vector2i(3, 0), "remaining": 1.0, "phase": "moving"},
	]
	for test_case in test_data:
		prepare_case(battle)
		var destination: Vector2i = actor_cell(battle) + Vector2i(test_case.offset)

		push_left_click(battle, battle.cell_center(destination))

		assert_vector(actor_cell(battle)).override_failure_message("%s：角色應以單次點擊抵達目的地" % test_case.name).is_equal(destination)
		assert_float(battle.state.turn.move_remaining).override_failure_message("%s：剩餘移動力應正確" % test_case.name).is_equal(test_case.remaining)
		assert_str(battle.state.turn.phase).override_failure_message("%s：移動階段應正確" % test_case.name).is_equal(test_case.phase)

func prepare_case(battle) -> void:
	for child in battle.units_layer.get_children():
		child.free()
	battle.unit_nodes.clear()
	battle.pending_action = ""
	battle.hovered = Vector2i(-1, -1)
	battle.clear_move_preview()

	var definition := FileAccess.get_file_as_string(TEST_DEFINITION)
	var loaded = JSON.parse_string(battle.core.load_definition(definition))
	assert_bool(loaded.has("error")).override_failure_message("專用 TOML 應成功載入").is_false()
	if loaded.has("error"): return
	battle.state = loaded
	battle.setup_native_tilemap()
	assert_bool(battle.send({"type": "start"})).override_failure_message("專用測試戰鬥應成功開始").is_true()

func actor_cell(battle) -> Vector2i:
	for unit in battle.state.units:
		if unit.id == battle.state.turn.actor:
			return Vector2i(unit.x, unit.y)
	return Vector2i(-1, -1)

func cells_from_values(values: Array) -> Array[Vector2i]:
	var cells: Array[Vector2i] = []
	for value in values:
		cells.append(Vector2i(value.x, value.y))
	return cells

func path_x_offsets(path: Array, origin: Vector2i) -> Array[int]:
	var offsets: Array[int] = []
	for value in path:
		var cell := Vector2i(value.x, value.y)
		assert_int(cell.y).override_failure_message("直線測試路徑不應偏離列").is_equal(origin.y)
		offsets.append(cell.x - origin.x)
	return offsets

func push_mouse_motion(battle, local_position: Vector2) -> void:
	var screen_position: Vector2 = battle.get_viewport().get_screen_transform() * battle.get_global_transform_with_canvas() * local_position
	runner.simulate_mouse_move(screen_position)

func push_left_click(battle, local_position: Vector2) -> void:
	var screen_position: Vector2 = battle.get_viewport().get_screen_transform() * battle.get_global_transform_with_canvas() * local_position
	runner.set_mouse_position(screen_position)
	runner.simulate_mouse_button_pressed(MOUSE_BUTTON_LEFT)
