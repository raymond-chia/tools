class_name BattleFlowTest
extends GdUnitTestSuite

const LEVEL_SCENE := "res://scenes/level/level.tscn"
const FIXTURE_ROOT := "res://tests/battle/fixtures/"
const SELECTED_TILE_SOURCE_ID := 0
const EMPTY_TILE_SOURCE_ID := -1


func before_test() -> void:
	var session := GameSession.new()
	var load_result := session.parse_and_insert_game_data(
		_read_fixture("battle_flow_units.toml"),
		_read_fixture("battle_flow_skills.toml"),
		_read_fixture("battle_flow_equipments.toml"),
		_read_fixture("battle_flow_objects.toml")
	)
	assert_bool(load_result.get("ok", false)).is_true()
	var level_result := session.spawn_level(
		"battle-flow-test",
		_read_fixture("battle_flow_level.toml")
	)
	assert_bool(level_result.get("ok", false)).is_true()
	GameSessionManager._session = session
	GameSessionManager._current_level_name = "battle-flow-test"


func after_test() -> void:
	GameSessionManager.end_game()


func test_battle_unit_inspection_skill_priority_movement_attack_and_turn_order() -> void:
	var runner := scene_runner(LEVEL_SCENE)
	await runner.simulate_frames(60)
	var level := runner.scene()
	var board: LevelBoard = level.get_node("LevelBoard")
	var camera: Camera2D = board.get_node("Camera2D")
	var selection_layer: TileMapLayer = board.get_node("SelectionLayer")
	var unit_panel: PanelContainer = level.get_node("Hud/Ui/RightSidebar/UnitPanel")
	var details_label: Label = level.get_node("Hud/Ui/RightSidebar/UnitPanel/Content/DetailsScroll/DetailsContent/UnitDetailsLabel")
	var skill_buttons: HBoxContainer = level.get_node("Hud/SkillPanel/Content/SkillButtons")
	var end_turn_button: Button = level.get_node("Hud/SkillPanel/Content/EndTurnButton")
	var current_portrait: TextureButton = level.get_node("Hud/SkillPanel/Content/CurrentUnitPortrait")
	var turn_order_items: VBoxContainer = level.get_node("Hud/TurnOrderScroll/TurnOrderItems")
	var turn_result := GameSessionManager.get_remaining_turn_units()
	assert_bool(turn_result.get("ok", false)).is_true()
	var turn_units: Array = turn_result["units"]
	assert_int(turn_units.size()).is_equal(2)
	var first_unit: Dictionary = turn_units[0]
	var second_unit: Dictionary = turn_units[1]
	assert_int(first_unit["attributes"]["initiative"]).is_greater(
		second_unit["attributes"]["initiative"] + 6
	)
	var first_cell := Vector2i(first_unit["x"], first_unit["y"])
	var second_cell := Vector2i(second_unit["x"], second_unit["y"])

	await _right_click_unit(runner, board, first_unit["id"])
	assert_bool(unit_panel.is_visible_in_tree()).is_true()
	assert_str(details_label.text.get_slice("\n", 0).strip_edges()).is_equal(
		"%s: %s" % [tr("DEPLOYMENT_UNIT_NAME"), first_unit["name"]]
	)
	assert_int(selection_layer.get_cell_source_id(first_cell)).is_equal(SELECTED_TILE_SOURCE_ID)
	if is_failure():
		return

	await _right_click_unit(runner, board, second_unit["id"])
	assert_bool(unit_panel.is_visible_in_tree()).is_true()
	assert_str(details_label.text.get_slice("\n", 0).strip_edges()).is_equal(
		"%s: %s" % [tr("DEPLOYMENT_UNIT_NAME"), second_unit["name"]]
	)
	assert_int(selection_layer.get_cell_source_id(second_cell)).is_equal(SELECTED_TILE_SOURCE_ID)
	if is_failure():
		return

	await _right_click_unit(runner, board, second_unit["id"])
	assert_bool(unit_panel.is_visible_in_tree()).is_false()
	if is_failure():
		return

	await _right_click_unit(runner, board, first_unit["id"])
	assert_bool(unit_panel.is_visible_in_tree()).is_true()
	assert_str(details_label.text.get_slice("\n", 0).strip_edges()).is_equal(
		"%s: %s" % [tr("DEPLOYMENT_UNIT_NAME"), first_unit["name"]]
	)
	if is_failure():
		return

	assert_str(current_portrait.tooltip_text).is_equal(first_unit["name"])
	assert_int(turn_order_items.get_child_count()).is_equal(1)
	var second_portrait: TextureButton = turn_order_items.get_child(0)
	assert_str(second_portrait.tooltip_text).is_equal(second_unit["name"])
	await _left_click(runner, _control_screen_center(second_portrait))
	await runner.simulate_frames(60)
	var second_token := _unit_token(board, second_unit["id"])
	assert_vector(camera.position).is_equal(second_token.position)
	if is_failure():
		return

	var melee_button := _button_with_text(skill_buttons, "melee")
	await _left_click(runner, _control_screen_center(melee_button))
	assert_bool(level._selected_skill == "melee").is_true()
	if is_failure():
		return

	await _right_click_unit(runner, board, first_unit["id"])
	assert_bool(level._selected_skill == "melee").is_true()
	assert_str(details_label.text.get_slice("\n", 0).strip_edges()).is_equal(
		"%s: %s" % [tr("DEPLOYMENT_UNIT_NAME"), first_unit["name"]]
	)
	assert_int(selection_layer.get_cell_source_id(first_cell)).is_equal(SELECTED_TILE_SOURCE_ID)
	if is_failure():
		return

	# 右鍵空白格時優先取消技能；再次右鍵才清除已檢視的單位。
	var blank_cell := Vector2i(first_cell.x, first_cell.y + 1)
	await _right_click_cell(runner, board, blank_cell)
	assert_bool(level._selected_skill.is_empty()).is_true()
	assert_bool(unit_panel.is_visible_in_tree()).is_true()
	if is_failure():
		return

	await _right_click_cell(runner, board, blank_cell)
	assert_bool(unit_panel.is_visible_in_tree()).is_false()
	assert_int(selection_layer.get_cell_source_id(first_cell)).is_equal(EMPTY_TILE_SOURCE_ID)
	if is_failure():
		return

	await _right_click_unit(runner, board, second_unit["id"])
	assert_bool(unit_panel.is_visible_in_tree()).is_true()
	assert_str(details_label.text.get_slice("\n", 0).strip_edges()).is_equal(
		"%s: %s" % [tr("DEPLOYMENT_UNIT_NAME"), second_unit["name"]]
	)
	if is_failure():
		return

	var adjacent_cell := Vector2i(second_cell.x - 1, second_cell.y)
	await _left_click_cell(runner, board, adjacent_cell)
	await runner.simulate_frames(30)
	var moved_first_unit: Variant = _unit_from_snapshot(first_unit["id"])
	assert_bool(moved_first_unit is Dictionary).is_true()
	assert_int(moved_first_unit["x"]).is_equal(adjacent_cell.x)
	assert_int(moved_first_unit["y"]).is_equal(adjacent_cell.y)
	if is_failure():
		return

	melee_button = _button_with_text(skill_buttons, "melee")
	await _left_click(runner, _control_screen_center(melee_button))
	assert_bool(level._selected_skill == "melee").is_true()
	if is_failure():
		return
	var hp_before: int = second_unit["attributes"]["current_hp"]

	await _left_click_cell(runner, board, second_cell)
	await runner.simulate_frames(20)
	var attacked_second_unit: Variant = _unit_from_snapshot(second_unit["id"])
	assert_bool(attacked_second_unit is Dictionary).is_true()
	assert_int(attacked_second_unit["attributes"]["current_hp"]).is_less(hp_before)
	if is_failure():
		return

	await _left_click(runner, _control_screen_center(end_turn_button))
	await runner.simulate_frames(2)
	var next_turn_result := GameSessionManager.get_remaining_turn_units()
	assert_bool(next_turn_result.get("ok", false)).is_true()
	var next_turn_units: Array = next_turn_result["units"]
	assert_int(next_turn_units[0]["id"]).is_equal(second_unit["id"])
	assert_str(current_portrait.tooltip_text).is_equal(second_unit["name"])


func _read_fixture(file_name: String) -> String:
	return FileAccess.get_file_as_string(FIXTURE_ROOT + file_name)


func _unit_token(board: LevelBoard, unit_id: int) -> UnitToken:
	for child in board.get_node("WorldLayer/UnitsLayer").get_children():
		var token := child as UnitToken
		if token != null and token.unit_id == unit_id:
			return token
	return null


func _unit_from_snapshot(unit_id: int) -> Variant:
	var snapshot := GameSessionManager.get_level_snapshot()
	for unit in snapshot["units"]:
		if unit["id"] == unit_id:
			return unit
	return null


func _button_with_text(container: Control, text: String) -> Button:
	for child in container.get_children():
		var button := child as Button
		if button != null and button.text.begins_with(text):
			return button
	return null


func _unit_screen_position(board: LevelBoard, unit_id: int) -> Vector2:
	var token := _unit_token(board, unit_id)
	return (
		board.get_viewport().get_screen_transform()
		* token.get_global_transform_with_canvas()
		* Vector2.ZERO
	)


func _cell_screen_position(board: LevelBoard, cell: Vector2i) -> Vector2:
	var ground_layer: TileMapLayer = board.get_node("GroundLayer")
	return (
		board.get_viewport().get_screen_transform()
		* ground_layer.get_global_transform_with_canvas()
		* ground_layer.map_to_local(cell)
	)


func _control_screen_center(control: Control) -> Vector2:
	# 先在 Control 區域座標取中心，避免最後才除以 2 時連已轉換的全域位移也一起減半。
	return (
		control.get_viewport().get_screen_transform()
		* control.get_global_transform_with_canvas()
		* (control.size / 2.0)
	)


func _right_click_unit(runner: GdUnitSceneRunner, board: LevelBoard, unit_id: int) -> void:
	await _right_click(runner, _unit_screen_position(board, unit_id))


func _right_click_cell(runner: GdUnitSceneRunner, board: LevelBoard, cell: Vector2i) -> void:
	await _right_click(runner, _cell_screen_position(board, cell))


func _left_click_cell(runner: GdUnitSceneRunner, board: LevelBoard, cell: Vector2i) -> void:
	await _left_click(runner, _cell_screen_position(board, cell))


func _right_click(runner: GdUnitSceneRunner, position: Vector2) -> void:
	runner.set_mouse_position(position)
	await runner.simulate_mouse_button_pressed(MOUSE_BUTTON_RIGHT).simulate_frames(2)


func _left_click(runner: GdUnitSceneRunner, position: Vector2) -> void:
	runner.set_mouse_position(position)
	await runner.simulate_mouse_button_pressed(MOUSE_BUTTON_LEFT).simulate_frames(2)
