extends RefCounted

const BATTLE_SCENE := preload("res://features/battle/battle.tscn")
const TEST_DEFINITION := "res://tests/features/battle/data/battle_preview.toml"

var failures: Array[String] = []

func run(tree: SceneTree) -> Array[String]:
	failures.clear()
	var battle_scene := BATTLE_SCENE.instantiate()
	tree.root.add_child(battle_scene)
	await tree.process_frame

	var battle = battle_scene.get_node("Content")
	test_skill_range_preview(battle)
	test_two_stage_movement_range_preview(battle)
	test_movement_path_preview(battle)
	test_single_click_movement(battle)

	battle_scene.queue_free()
	await tree.process_frame
	return failures

# 驗證四種技能的施放範圍邊界正確，且選擇技能會隱藏移動範圍並清除路徑預覽。
func test_skill_range_preview(battle) -> void:
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
		expect_false(battle.first_move_path.is_empty(), "%s：選擇技能前應有移動路徑預覽" % test_case.name)

		push_left_click(battle, test_case.rect.get_center())

		var preview_cells := cells_from_values(battle.selected_skill_range())
		expect_true(preview_cells.has(actor_cell + Vector2i(test_case.range, 0)), "%s：射程邊界格應包含在預覽" % test_case.name)
		expect_false(preview_cells.has(actor_cell + Vector2i(test_case.range + 1, 0)), "%s：射程外一格不應包含在預覽" % test_case.name)
		expect_true(battle.first_move_path.is_empty() and battle.second_move_path.is_empty(), "%s：選擇技能後移動路徑應消失" % test_case.name)
		push_mouse_motion(battle, battle.cell_center(actor_cell + Vector2i(2, 0)))
		expect_true(battle.first_move_path.is_empty() and battle.second_move_path.is_empty(), "%s：技能待選時不應重新產生移動路徑" % test_case.name)

# 驗證移動範圍會區分第一段、第二段與兩段外的格子。
func test_two_stage_movement_range_preview(battle) -> void:
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
		expect_equal(first_cells.has(cell), test_case.first, "%s：第一段範圍歸屬應正確" % test_case.name)
		expect_equal(second_cells.has(cell), test_case.second, "%s：第二段範圍歸屬應正確" % test_case.name)

# 驗證滑鼠懸停會產生正確分段的移動路徑，且不可達格不會留下路徑。
func test_movement_path_preview(battle) -> void:
	var test_data := [
		{"name": "第一段路徑", "offset": Vector2i(2, 0), "first": [0, 1, 2], "second": []},
		{"name": "第二段路徑", "offset": Vector2i(3, 0), "first": [0, 1, 2], "second": [2, 3]},
		{"name": "不可達路徑", "offset": Vector2i(5, 0), "first": [], "second": []},
	]
	for test_case in test_data:
		prepare_case(battle)
		var origin := actor_cell(battle)
		push_mouse_motion(battle, battle.cell_center(origin + test_case.offset))
		expect_equal(path_x_offsets(battle.first_move_path, origin), test_case.first, "%s：第一段路徑應正確" % test_case.name)
		expect_equal(path_x_offsets(battle.second_move_path, origin), test_case.second, "%s：第二段路徑應正確" % test_case.name)

# 驗證單次點擊可抵達第一段或第二段目的地，並正確扣除兩段移動力。
func test_single_click_movement(battle) -> void:
	var test_data := [
		{"name": "抵達第一段", "offset": Vector2i(2, 0), "remaining": 0, "phase": "aftermove"},
		{"name": "抵達第二段", "offset": Vector2i(3, 0), "remaining": 1, "phase": "moving"},
	]
	for test_case in test_data:
		prepare_case(battle)
		var destination: Vector2i = actor_cell(battle) + Vector2i(test_case.offset)

		push_left_click(battle, battle.cell_center(destination))

		expect_equal(actor_cell(battle), destination, "%s：角色應以單次點擊抵達目的地" % test_case.name)
		expect_equal(battle.state.turn.move_remaining, test_case.remaining, "%s：剩餘移動力應正確" % test_case.name)
		expect_equal(battle.state.turn.phase, test_case.phase, "%s：移動階段應正確" % test_case.name)

func prepare_case(battle) -> void:
	for child in battle.units_layer.get_children():
		child.free()
	battle.unit_nodes.clear()
	battle.pending_action = ""
	battle.hovered = Vector2i(-1, -1)
	battle.clear_move_preview()

	var definition := FileAccess.get_file_as_string(TEST_DEFINITION)
	var loaded = JSON.parse_string(battle.core.load_definition(definition))
	expect_false(loaded.has("error"), "專用 TOML 應成功載入")
	if loaded.has("error"): return
	battle.state = loaded
	battle.setup_native_tilemap()
	expect_true(battle.send({"type": "start"}), "專用測試戰鬥應成功開始")

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
		expect_equal(cell.y, origin.y, "直線測試路徑不應偏離列")
		offsets.append(cell.x - origin.x)
	return offsets

func push_mouse_motion(battle, local_position: Vector2) -> void:
	var viewport_position: Vector2 = battle.get_global_transform_with_canvas() * local_position
	var event := InputEventMouseMotion.new()
	event.position = viewport_position
	event.global_position = viewport_position
	battle.get_viewport().push_input(event, true)

func push_left_click(battle, local_position: Vector2) -> void:
	var viewport_position: Vector2 = battle.get_global_transform_with_canvas() * local_position
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = true
	event.position = viewport_position
	event.global_position = viewport_position
	battle.get_viewport().push_input(event, true)

func expect_true(actual: bool, message: String) -> void:
	if not actual:
		failures.append(message)

func expect_false(actual: bool, message: String) -> void:
	expect_true(not actual, message)

func expect_equal(actual, expected, message: String) -> void:
	if actual != expected:
		failures.append("%s（預期：%s，實際：%s）" % [message, expected, actual])
