extends RefCounted

const BATTLE_SCENE := preload("res://features/battle/battle.tscn")
const TEST_DEFINITION := "res://tests/features/battle/data/right_click_selection.toml"

var failures: Array[String] = []

func run(tree: SceneTree) -> Array[String]:
	failures.clear()
	var battle_scene := BATTLE_SCENE.instantiate()
	tree.root.add_child(battle_scene)
	await tree.process_frame

	var battle = battle_scene.get_node("Content")
	test_inspection_changes_from_right_click(battle)
	test_pending_action_right_click_priority(battle)
	test_ignored_right_click_inputs(battle)

	battle_scene.queue_free()
	await tree.process_frame
	return failures

# 驗證右鍵可查看一般單位、大型單位與空地，並可取消或切換目前查看目標。
func test_inspection_changes_from_right_click(battle) -> void:
	var test_data := [
		{"name": "查看一般單位", "initial": "none", "click": "unit:aria", "expected": "unit:aria"},
		{"name": "查看大型單位", "initial": "none", "click": "unit:ogre", "expected": "unit:ogre"},
		{"name": "查看空地", "initial": "none", "click": "empty", "expected": "empty"},
		{"name": "再次查看同一單位", "initial": "unit:aria", "click": "unit:aria", "expected": "none"},
		{"name": "切換查看單位", "initial": "unit:aria", "click": "unit:lyra", "expected": "unit:lyra"},
	]
	for test_case in test_data:
		prepare_case(battle, test_case.initial)
		var state_before_input: Dictionary = battle.state.duplicate(true)

		push_mouse_button(battle, target_point(battle, test_case.click), true)

		assert_inspection(battle, test_case.expected, test_case.name)
		expect_equal(battle.pending_action, "", "%s：不應建立待選技能" % test_case.name)
		expect_equal(battle.state, state_before_input, "%s：不應修改核心戰鬥狀態" % test_case.name)

# 驗證技能待選時會先查看尚未顯示的單位，只有重複查看或點空地才取消技能。
func test_pending_action_right_click_priority(battle) -> void:
	var test_data := [
		{"name": "未查看時先顯示單位", "initial": "none", "click": "unit:aria", "result": "inspect"},
		{"name": "查看空地時先顯示單位", "initial": "empty", "click": "unit:aria", "result": "inspect"},
		{"name": "查看其他單位時切換單位", "initial": "unit:aria", "click": "unit:lyra", "result": "inspect"},
		{"name": "重複查看同一單位時取消技能", "initial": "unit:aria", "click": "unit:aria", "result": "cancel"},
		{"name": "右鍵空地時取消技能", "initial": "unit:aria", "click": "empty", "result": "cancel"},
	]
	for test_case in test_data:
		prepare_case(battle, test_case.initial)
		battle.select_action("melee")
		var state_before_input: Dictionary = battle.state.duplicate(true)

		push_mouse_button(battle, target_point(battle, test_case.click), true)

		var expected_target: String = test_case.click if test_case.result == "inspect" else test_case.initial
		var expected_action := "melee" if test_case.result == "inspect" else ""
		var expected_status := "請選擇技能目標。" if test_case.result == "inspect" else "已取消技能。"
		assert_inspection(battle, expected_target, test_case.name)
		expect_equal(battle.pending_action, expected_action, "%s：技能狀態應符合右鍵優先規則" % test_case.name)
		expect_equal(battle.status, expected_status, "%s：提示文字應符合技能狀態" % test_case.name)
		expect_equal(battle.state, state_before_input, "%s：不應修改核心戰鬥狀態" % test_case.name)

# 驗證戰鬥區域外的右鍵與右鍵放開事件不會改變目前查看狀態。
func test_ignored_right_click_inputs(battle) -> void:
	var test_data := [
		{"name": "戰鬥區域外", "input": "outside"},
		{"name": "右鍵放開", "input": "released"},
	]
	for test_case in test_data:
		prepare_case(battle, "unit:aria")
		var state_before_input: Dictionary = battle.state.duplicate(true)

		if test_case.input == "outside":
			push_mouse_button(battle, Vector2(battle.PANEL_X + 1.0, 1.0), true)
		else:
			push_mouse_button(battle, target_point(battle, "unit:lyra"), false)

		assert_inspection(battle, "unit:aria", test_case.name)
		expect_equal(battle.pending_action, "", "%s：不應改變技能狀態" % test_case.name)
		expect_equal(battle.state, state_before_input, "%s：不應修改核心戰鬥狀態" % test_case.name)

func prepare_case(battle, initial_target: String) -> void:
	load_test_definition(battle)
	battle.inspected_cell = target_cell(battle, initial_target)
	battle.pending_action = ""
	battle.status = ""
	battle.sync_unit_sprites()

func load_test_definition(battle) -> void:
	for child in battle.units_layer.get_children():
		child.free()
	battle.unit_nodes.clear()

	var definition := FileAccess.get_file_as_string(TEST_DEFINITION)
	var loaded = JSON.parse_string(battle.core.load_definition(definition))
	expect_false(loaded.has("error"), "專用 TOML 應成功載入")
	if loaded.has("error"):
		return

	battle.state = loaded
	battle.setup_native_tilemap()
	expect_true(battle.send({"type": "start"}), "專用測試戰鬥應成功開始")

func target_point(battle, target: String) -> Vector2:
	if target == "empty":
		return battle.cell_center(find_empty_cell(battle.state))
	var unit := find_unit(battle.state.units, target.get_slice(":", 1))
	return battle.footprint_center(unit)

func target_cell(battle, target: String) -> Vector2i:
	if target == "none":
		return Vector2i(-1, -1)
	if target == "empty":
		return find_empty_cell(battle.state)
	var unit := find_unit(battle.state.units, target.get_slice(":", 1))
	return Vector2i(unit.x, unit.y)

func find_empty_cell(state: Dictionary) -> Vector2i:
	for y in state.height:
		for x in state.width:
			var cell := Vector2i(x, y)
			var occupied := false
			for unit in state.units:
				if cell.x >= unit.x and cell.x < unit.x + unit.width and cell.y >= unit.y and cell.y < unit.y + unit.height:
					occupied = true
					break
			if not occupied:
				return cell
	return Vector2i(-1, -1)

func find_unit(units: Array, id: String) -> Dictionary:
	for unit in units:
		if unit.id == id:
			return unit
	return {}

func assert_inspection(battle, expected_target: String, case_name: String) -> void:
	expect_equal(battle.inspected_cell, target_cell(battle, expected_target), "%s：查看格應正確" % case_name)
	var selected_unit_id := expected_target.get_slice(":", 1) if expected_target.begins_with("unit:") else ""
	for unit_id in battle.unit_nodes:
		var selection = battle.unit_nodes[unit_id].get_node("Selection")
		expect_equal(selection.visible, unit_id == selected_unit_id, "%s：%s 的選取圈可見性應正確" % [case_name, unit_id])

func push_mouse_button(battle, local_position: Vector2, pressed: bool) -> void:
	var viewport_position: Vector2 = battle.get_global_transform_with_canvas() * local_position
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_RIGHT
	event.pressed = pressed
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
