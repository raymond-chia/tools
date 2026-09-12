extends GdUnitTestSuite

const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITION := "res://tests/features/battle/data/right_click_selection.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(BATTLE_SCENE)
	await runner.simulate_frames(1)
	battle = runner.scene()

# 驗證右鍵可查看一般單位、大型單位與空地，並可取消或切換目前查看目標。
func test_inspection_changes_from_right_click() -> void:
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

		push_mouse_button(battle.world, target_point(battle, test_case.click), true)

		assert_inspection(battle, test_case.expected, test_case.name)
		assert_str(battle.pending_action).override_failure_message("%s：不應建立待選技能" % test_case.name).is_empty()
		assert_dict(battle.state).override_failure_message("%s：不應修改核心戰鬥狀態" % test_case.name).is_equal(state_before_input)

# 驗證技能待選時會先查看尚未顯示的單位，只有重複查看或點空地才取消技能。
func test_pending_action_right_click_priority() -> void:
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

		push_mouse_button(battle.world, target_point(battle, test_case.click), true)

		var expected_target: String = test_case.click if test_case.result == "inspect" else test_case.initial
		var expected_action := "melee" if test_case.result == "inspect" else ""
		var expected_status := "請選擇技能目標。" if test_case.result == "inspect" else "已取消技能。"
		assert_inspection(battle, expected_target, test_case.name)
		assert_str(battle.pending_action).override_failure_message("%s：技能狀態應符合右鍵優先規則" % test_case.name).is_equal(expected_action)
		assert_str(battle.status).override_failure_message("%s：提示文字應符合技能狀態" % test_case.name).is_equal(expected_status)
		assert_dict(battle.state).override_failure_message("%s：不應修改核心戰鬥狀態" % test_case.name).is_equal(state_before_input)

# 驗證戰鬥區域外的右鍵與右鍵放開事件不會改變目前查看狀態。
func test_ignored_right_click_inputs() -> void:
	var test_data := [
		{"name": "戰鬥區域外", "input": "outside"},
		{"name": "右鍵放開", "input": "released"},
	]
	for test_case in test_data:
		prepare_case(battle, "unit:aria")
		var state_before_input: Dictionary = battle.state.duplicate(true)

		if test_case.input == "outside":
			push_mouse_button(battle.ui.root, Vector2(10.0, battle.ui.root.size.y - 10.0), true)
		else:
			push_mouse_button(battle.world, target_point(battle, "unit:lyra"), false)

		assert_inspection(battle, "unit:aria", test_case.name)
		assert_str(battle.pending_action).override_failure_message("%s：不應改變技能狀態" % test_case.name).is_empty()
		assert_dict(battle.state).override_failure_message("%s：不應修改核心戰鬥狀態" % test_case.name).is_equal(state_before_input)

# 驗證單位詳情可從標題列以左鍵拖曳，且放開左鍵後即停止移動。
func test_unit_details_can_be_dragged_with_left_mouse_button() -> void:
	prepare_case(battle, "unit:aria")
	var info_panel: Panel = battle.ui.info_panel
	var header: Control = info_panel.get_node("Margin/Content/Header")
	var initial_position := info_panel.position
	var drag_offset := Vector2(-120.0, 80.0)
	var header_center := header.global_position + header.size / 2.0

	push_left_drag(header, header_center, header_center + drag_offset)

	assert_vector(info_panel.position).override_failure_message("左鍵拖曳後詳情面板應跟著移動，實際位置：%s" % info_panel.position).is_equal(initial_position + drag_offset)
	var released_position := info_panel.position
	emit_mouse_motion(header, header_center + drag_offset * 2.0)
	assert_vector(info_panel.position).override_failure_message("放開左鍵後詳情面板應停止移動").is_equal(released_position)

func prepare_case(battle, initial_target: String) -> void:
	load_test_definition(battle)
	battle.inspected_cell = target_cell(battle, initial_target)
	battle.pending_action = ""
	battle.status = ""
	battle.present()

func load_test_definition(battle) -> void:
	for child in battle.world.units_layer.get_children():
		child.free()
	battle.world.unit_nodes.clear()

	var definition := FileAccess.get_file_as_string(TEST_DEFINITION)
	var loaded = JSON.parse_string(battle.core.load_definition(definition))
	assert_bool(loaded.has("error")).override_failure_message("專用 TOML 應成功載入").is_false()
	if loaded.has("error"):
		return

	battle.state = loaded
	battle.world.setup_map(loaded)
	assert_bool(battle.send({"type": "start"})).override_failure_message("專用測試戰鬥應成功開始").is_true()

func target_point(battle, target: String) -> Vector2:
	if target == "empty":
		return battle.world.cell_center(find_empty_cell(battle.state))
	var unit := find_unit(battle.state.units, target.get_slice(":", 1))
	return battle.world.footprint_center(unit)

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
	assert_vector(battle.inspected_cell).override_failure_message("%s：查看格應正確" % case_name).is_equal(target_cell(battle, expected_target))
	var selected_unit_id := expected_target.get_slice(":", 1) if expected_target.begins_with("unit:") else ""
	for unit_id in battle.world.unit_nodes:
		var selection = battle.world.unit_nodes[unit_id].get_node("Selection")
		assert_bool(selection.visible).override_failure_message("%s：%s 的選取圈可見性應正確" % [case_name, unit_id]).is_equal(unit_id == selected_unit_id)

func push_mouse_button(battle, local_position: Vector2, pressed: bool) -> void:
	var screen_position: Vector2 = battle.get_viewport().get_screen_transform() * battle.get_global_transform_with_canvas() * local_position
	runner.set_mouse_position(screen_position)
	if pressed:
		runner.simulate_mouse_button_pressed(MOUSE_BUTTON_RIGHT)
	else:
		runner.simulate_mouse_button_release(MOUSE_BUTTON_RIGHT)

func push_left_drag(control: Control, from: Vector2, to: Vector2) -> void:
	emit_mouse_button(control, from, true)
	emit_mouse_motion(control, to)
	emit_mouse_button(control, to, false)

func emit_mouse_button(control: Control, global_position: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = pressed
	event.global_position = global_position
	control.gui_input.emit(event)

func emit_mouse_motion(control: Control, global_position: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.global_position = global_position
	control.gui_input.emit(event)
