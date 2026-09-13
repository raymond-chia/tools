extends GdUnitTestSuite

const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITION := "res://tests/features/battle/data/battle_preview.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(BATTLE_SCENE)
	await runner.simulate_frames(1)
	battle = runner.scene()

# 驗證四種技能的施放範圍邊界正確，且選擇技能會隱藏移動範圍並清除路徑預覽。
func test_skill_range_preview() -> void:
	var test_data := [
		{"name": "近戰攻擊", "action": "melee_attack", "button": battle.ui.action_buttons.melee_attack, "range": 1},
		{"name": "遠程攻擊", "action": "ranged_attack", "button": battle.ui.action_buttons.ranged_attack, "range": 3},
		{"name": "強力一擊", "action": "power_strike", "button": battle.ui.action_buttons.power_strike, "range": 1},
		{"name": "瞄準射擊", "action": "aimed_shot", "button": battle.ui.action_buttons.aimed_shot, "range": 4},
	]
	for test_case in test_data:
		prepare_case(battle)
		var actor_cell := actor_cell(battle)
		push_mouse_motion(battle.world, battle.world.cell_center(actor_cell + Vector2i(3, 0)))
		assert_bool(battle.world.first_move_path.is_empty()).override_failure_message("%s：選擇技能前應有移動路徑預覽" % test_case.name).is_false()

		push_control_click(test_case.button)

		var preview_cells := cells_from_values(battle.world.selected_skill_range())
		assert_bool(preview_cells.has(actor_cell + Vector2i(test_case.range, 0))).override_failure_message("%s：射程邊界格應包含在預覽" % test_case.name).is_true()
		assert_bool(preview_cells.has(actor_cell + Vector2i(test_case.range + 1, 0))).override_failure_message("%s：射程外一格不應包含在預覽" % test_case.name).is_false()
		assert_bool(battle.world.first_move_path.is_empty() and battle.world.second_move_path.is_empty()).override_failure_message("%s：選擇技能後移動路徑應消失" % test_case.name).is_true()
		push_mouse_motion(battle.world, battle.world.cell_center(actor_cell + Vector2i(2, 0)))
		assert_bool(battle.world.first_move_path.is_empty() and battle.world.second_move_path.is_empty()).override_failure_message("%s：技能待選時不應重新產生移動路徑" % test_case.name).is_true()

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
		push_mouse_motion(battle.world, battle.world.cell_center(origin + test_case.offset))
		assert_array(path_x_offsets(battle.world.first_move_path, origin)).override_failure_message("%s：第一段路徑應正確" % test_case.name).is_equal(test_case.first)
		assert_array(path_x_offsets(battle.world.second_move_path, origin)).override_failure_message("%s：第二段路徑應正確" % test_case.name).is_equal(test_case.second)

# 驗證移動路徑碰到地刺時會在觸發格截斷，並啟用危險路徑警示狀態。
func test_spikes_interrupt_movement_preview() -> void:
	prepare_case(battle)
	var spikes := Vector2i(1, 2)
	var destination := Vector2i(1, 1)
	var preview = JSON.parse_string(battle.core.preview_move(battle.state.turn.actor, destination.x, destination.y))
	push_mouse_motion(battle.world, battle.world.cell_center(destination))

	assert_bool(preview.interrupted).override_failure_message("核心應標記路徑受到地刺中斷").is_true()
	assert_int(preview.first.size()).override_failure_message("預覽路徑應截斷於第一個地刺格").is_equal(2)
	assert_vector(Vector2i(preview.first[-1].x, preview.first[-1].y)).is_equal(spikes)
	assert_bool(battle.world.move_preview_interrupted).override_failure_message("戰鬥畫面應啟用紅色危險路徑狀態").is_true()
	assert_int(battle.world.first_move_path.size()).is_equal(2)
	assert_vector(Vector2i(battle.world.first_move_path[-1].x, battle.world.first_move_path[-1].y)).override_failure_message("危險路徑落點應為地刺格").is_equal(spikes)

# 驗證移動模式懸停可達地格時會在游標旁顯示整條路徑的總消耗，離開移動模式後則隱藏。
func test_hovered_tile_movement_total_cost() -> void:
	var test_data := [
		{"name": "第一段一般地格", "offset": Vector2i(2, 0), "expected": 2},
		{"name": "第二段高消耗地格", "offset": Vector2i(2, 1), "expected": 4},
	]
	for test_case in test_data:
		prepare_case(battle)
		var destination: Vector2i = actor_cell(battle) + test_case.offset
		var preview = JSON.parse_string(battle.core.preview_move(battle.state.turn.actor, destination.x, destination.y))

		push_mouse_motion(battle.world, battle.world.cell_center(destination))

		assert_int(int(preview.get("total_cost"))).override_failure_message("%s：核心預覽應提供移動總消耗" % test_case.name).is_equal(test_case.expected)
		var popup: Control = battle.ui.get_node_or_null("Root/MoveCostPopup")
		assert_that(popup).override_failure_message("%s：應建立游標旁的移動消耗浮動面板" % test_case.name).is_not_null()
		if popup != null:
			assert_bool(popup.visible).override_failure_message("%s：hover tile 應顯示移動總消耗" % test_case.name).is_true()
			assert_str(popup.get_node("Label").text).override_failure_message("%s：浮動文字應顯示整條路徑的總消耗" % test_case.name).is_equal("移動消耗 %d" % test_case.expected)

	prepare_case(battle)
	var destination := actor_cell(battle) + Vector2i(2, 0)
	push_mouse_motion(battle.world, battle.world.cell_center(destination))
	push_control_click(battle.ui.action_buttons.melee_attack)
	var skill_mode_popup: Control = battle.ui.get_node_or_null("Root/MoveCostPopup")
	assert_that(skill_mode_popup).override_failure_message("應建立游標旁的移動消耗浮動面板").is_not_null()
	if skill_mode_popup != null:
		assert_bool(skill_mode_popup.visible).override_failure_message("技能模式不應顯示移動總消耗").is_false()

# 驗證游標靠近邊界時，移動消耗浮動面板會移至指定象限且不超出 viewport。
func test_movement_cost_popup_uses_available_quadrant() -> void:
	prepare_case(battle)
	var popup: Control = battle.ui.get_node_or_null("Root/MoveCostPopup")
	assert_that(popup).override_failure_message("應建立游標旁的移動消耗浮動面板").is_not_null()
	if popup == null:
		return
	var viewport_rect: Rect2 = battle.get_viewport().get_visible_rect()
	var test_data := [
		{"name": "滑鼠位於中央", "pointer": viewport_rect.get_center(), "horizontal": 1, "vertical": 1},
		{"name": "滑鼠位於左上", "pointer": viewport_rect.position + Vector2(1.0, 1.0), "horizontal": 1, "vertical": 1},
		{"name": "滑鼠位於左下", "pointer": Vector2(viewport_rect.position.x + 1.0, viewport_rect.end.y - 1.0), "horizontal": 1, "vertical": - 1},
		{"name": "滑鼠位於右上", "pointer": Vector2(viewport_rect.end.x - 1.0, viewport_rect.position.y + 1.0), "horizontal": - 1, "vertical": 1},
		{"name": "滑鼠位於右下", "pointer": viewport_rect.end - Vector2(1.0, 1.0), "horizontal": - 1, "vertical": - 1},
	]
	for test_case in test_data:
		var pointer_position: Vector2 = test_case.pointer
		battle.ui.present_move_cost(4, pointer_position)

		assert_bool(popup.visible).override_failure_message("%s：邊界附近仍應顯示移動總消耗" % test_case.name).is_true()
		assert_bool((popup.position.x - pointer_position.x) * test_case.horizontal > 0.0).override_failure_message("%s：浮動面板的左右位置應正確" % test_case.name).is_true()
		assert_bool((popup.position.y - pointer_position.y) * test_case.vertical > 0.0).override_failure_message("%s：浮動面板的上下位置應正確" % test_case.name).is_true()
		assert_bool(viewport_rect.encloses(popup.get_rect())).override_failure_message("%s：浮動面板應完整位於 viewport 可視範圍內" % test_case.name).is_true()

# 驗證單次點擊可抵達第一段或第二段目的地，並正確扣除兩段移動力。
func test_single_click_movement() -> void:
	var test_data := [
		{"name": "抵達第一段", "offset": Vector2i(2, 0), "remaining": 0.0, "phase": "aftermove"},
		{"name": "抵達第二段", "offset": Vector2i(3, 0), "remaining": 1.0, "phase": "moving"},
	]
	for test_case in test_data:
		prepare_case(battle)
		var destination: Vector2i = actor_cell(battle) + Vector2i(test_case.offset)

		push_left_click(battle.world, battle.world.cell_center(destination))

		assert_vector(actor_cell(battle)).override_failure_message("%s：角色應以單次點擊抵達目的地" % test_case.name).is_equal(destination)
		assert_float(battle.state.turn.move_remaining).override_failure_message("%s：剩餘移動力應正確" % test_case.name).is_equal(test_case.remaining)
		assert_str(battle.state.turn.phase).override_failure_message("%s：移動階段應正確" % test_case.name).is_equal(test_case.phase)

# 驗證未移動或只完成一段移動時可施放技能，完成兩段移動後則不可施放。
func test_skill_availability_after_movement() -> void:
	var test_data := [
		{"name": "未移動", "destinations": [], "can_skill": true},
		{"name": "完成一段移動", "destinations": [Vector2i(3, 3)], "can_skill": true},
		{"name": "開始第二段移動", "destinations": [Vector2i(3, 3), Vector2i(4, 3)], "can_skill": false},
		{"name": "完成兩段移動", "destinations": [Vector2i(3, 3), Vector2i(5, 3)], "can_skill": false},
		{"name": "單次開始第二段移動", "destinations": [Vector2i(4, 3)], "can_skill": false},
		{"name": "單次走完兩段移動", "destinations": [Vector2i(5, 3)], "can_skill": false},
	]
	for test_case in test_data:
		prepare_case(battle)
		for destination in test_case.destinations:
			assert_bool(battle.send({"type": "move", "actor": "aria", "x": destination.x, "y": destination.y})).override_failure_message("%s：測試移動應成功" % test_case.name).is_true()

		assert_bool(battle.state.turn.can_skill).override_failure_message("%s：技能可用狀態應正確" % test_case.name).is_equal(test_case.can_skill)
		assert_bool(battle.ui.action_buttons.aimed_shot.disabled).override_failure_message("%s：技能按鈕狀態應正確" % test_case.name).is_equal(not test_case.can_skill)
		assert_bool(battle.send({"type": "skill", "actor": "aria", "target": "ogre", "x": 3, "y": 1, "skill": "aimed_shot"})).override_failure_message("%s：技能施放結果應符合移動段數" % test_case.name).is_equal(test_case.can_skill)

func prepare_case(battle) -> void:
	for child in battle.world.units_layer.get_children():
		child.free()
	battle.world.unit_nodes.clear()
	battle.pending_action = ""
	battle.world.hovered = Vector2i(-1, -1)
	battle.world.clear_move_preview()

	var definition := FileAccess.get_file_as_string(TEST_DEFINITION)
	var loaded = JSON.parse_string(battle.core.load_definition(definition))
	assert_bool(loaded.has("error")).override_failure_message("專用 TOML 應成功載入").is_false()
	if loaded.has("error"): return
	battle.state = loaded
	battle.world.setup_map(loaded)
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

func push_control_click(control: Control) -> void:
	control.pressed.emit()
