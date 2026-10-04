extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/test_setup.gd")
const TEST_DEFINITIONS := "res://tests/features/battle/data/combat_presentation_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/combat_presentation_spikes_map.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(auto_free(BattleTestSetup.create_battle(TEST_DEFINITIONS, TEST_MAP, BattleTestSetup.PRESENTATION_RANDOM_SEED)))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	await BattleTestSetup.wait_until_idle(battle, runner)
	BattleTestSetup.reset_random_seed(battle.core, BattleTestSetup.PRESENTATION_RANDOM_SEED)

func after_test() -> void:
	runner.set_time_factor(9.0)
	await BattleTestSetup.wait_until_idle(battle, runner)

# 驗證攻擊與死亡演出期間所有戰鬥操作均被阻擋，且演出完成後技能與查看操作恢復。
func test_combat_presentation_locks_inputs_and_restores_them() -> void:
	for skill in ["presentation_push", "presentation_finish"]:
		await reload_battle()
		runner.set_time_factor(1.0)
		battle.ui.action_buttons.presentation_push.pressed.emit()
		battle.world.inspection_clicked.emit(3, Vector2i(2, 1))
		assert_bool(battle.send({"type": "skill", "actor": 1, "x": 2, "y": 1, "skill": skill})).is_true()
		assert_bool(battle.world.is_presenting_combat_events()).is_true()
		assert_inputs_locked()
		if skill == "presentation_finish":
			# 目標節點保留期間持續鎖定操作，不依賴死亡效果的顏色。
			for _frame in 240:
				if not battle.world.unit_nodes.has(3):
					break
				assert_inputs_locked()
				await runner.simulate_frames(1)
			assert_bool(battle.world.unit_nodes.has(3)).is_false()
		runner.set_time_factor(9.0)
		await BattleTestSetup.wait_until_idle(battle, runner)
		assert_bool(battle.input_is_locked()).is_false()
		battle.ui.action_buttons.presentation_heal.pressed.emit()
		assert_str(battle.pending_action).is_equal("presentation_heal")
		battle.world.inspection_clicked.emit(2, Vector2i(0, 3))
		assert_vector(battle.inspected_cell).is_equal(Vector2i(0, 3))

# 驗證地圖外、空地與友方等無效技能目標保留待選技能及戰鬥狀態，之後可正常施放。
func test_failed_skill_keeps_selection_until_valid_target() -> void:
	var cases := [
		{"cell": Vector2i(-1, -1), "status": "請選擇地圖上的格子。"},
		{"cell": Vector2i(1, 0), "status": ""},
		{"cell": Vector2i(0, 3), "status": ""},
	]
	for test_case in cases:
		await reload_battle()
		battle.ui.action_buttons.presentation_push.pressed.emit()
		var before: Dictionary = battle.state.duplicate(true)
		battle.world.primary_clicked.emit(0, test_case.cell)
		assert_str(battle.pending_action).is_equal("presentation_push")
		assert_dict(battle.state).is_equal(before)
		assert_str(battle.status).is_not_empty()
		assert_str(battle.ui.status.text).is_equal(tr(battle.status))
		if not test_case.status.is_empty():
			assert_str(battle.status).is_equal(test_case.status)
		assert_bool(battle.ui.action_buttons.presentation_push.button_pressed).is_true()
		battle.world.primary_clicked.emit(3, Vector2i(2, 1))
		assert_str(battle.pending_action).is_empty()
		await BattleTestSetup.wait_until_idle(battle, runner)
		assert_str(battle.status).is_empty()
		assert_bool(battle.ui.action_buttons.has("presentation_push")).is_false()

# 驗證移動到已占用格失敗後不殘留路徑或 Tween，接著仍可正常移動並抵達節點終點。
func test_failed_move_clears_prepared_animation_and_recovers() -> void:
	var before: Dictionary = battle.state.duplicate(true)
	var node: Node2D = battle.world.unit_nodes[1]
	var original_position := node.position
	battle.world.primary_clicked.emit(3, Vector2i(2, 1))
	assert_dict(battle.state).is_equal(before)
	assert_str(battle.status).is_not_empty()
	assert_int(battle.world.prepared_move_unit_id).is_zero()
	assert_array(battle.world.prepared_move_path).is_empty()
	assert_bool(battle.world.movement_tweens.has(1)).is_false()
	assert_vector(node.position).is_equal(original_position)
	var destination := Vector2i(1, 0)
	battle.world.hovered = destination
	battle.world.update_move_preview()
	assert_array(battle.world.first_move_path).is_not_empty()
	battle.world.primary_clicked.emit(0, destination)
	assert_str(battle.status).is_empty()
	await BattleTestSetup.wait_until_idle(battle, runner)
	assert_vector(node.position).is_equal(battle.world.cell_center(destination))
	assert_int(battle.world.prepared_move_unit_id).is_zero()
	assert_array(battle.world.prepared_move_path).is_empty()

# 驗證行動者切換後按鈕依技能組增刪，移除舊 hover 與查看內容，且重新整理後一次按下只送出一次技能選擇。
func test_actor_change_updates_skill_buttons_and_clears_old_content() -> void:
	assert_array(battle.ui.action_buttons.keys()).contains_exactly(["presentation_push", "presentation_finish"])
	var old_button: Button = battle.ui.action_buttons.presentation_push
	old_button.mouse_entered.emit()
	var right := InputEventMouseButton.new()
	right.button_index = MOUSE_BUTTON_RIGHT
	right.pressed = true
	old_button.gui_input.emit(right)
	assert_bool(battle.ui.hovered_skill_card.visible).is_true()
	assert_bool(battle.ui.info_panel.visible).is_true()
	battle.ui.get_node("Root/ActionBar/Margin/Layout/EndTurn").pressed.emit()
	await BattleTestSetup.wait_until_idle(battle, runner)
	await runner.simulate_frames(1)
	assert_int(battle.state.turn.actor).is_equal(2)
	assert_array(battle.ui.action_buttons.keys()).contains_exactly(["presentation_heal"])
	assert_str(battle.ui.hovered_skill_id).is_empty()
	assert_bool(battle.ui.hovered_skill_card.visible).is_false()
	assert_bool(battle.ui.info_panel.visible).is_false()
	var new_button: Button = battle.ui.action_buttons.presentation_heal
	for _refresh in 3:
		battle.present()
	var selected_actions: Array[String] = []
	battle.ui.action_selected.connect(func(action: String): selected_actions.append(action))
	new_button.pressed.emit()
	assert_array(selected_actions).contains_exactly(["presentation_heal"])
	assert_str(battle.pending_action).is_equal("presentation_heal")
	new_button.mouse_entered.emit()
	assert_bool(battle.ui.hovered_skill_card.visible).is_true()
	assert_str(battle.ui.hovered_skill_title.text).is_equal(tr(BattleConfig.skill_name_key("presentation_heal")))

func assert_inputs_locked() -> void:
	assert_bool(battle.input_is_locked()).is_true()
	var before: Dictionary = battle.state.duplicate(true)
	var pending_action: String = battle.pending_action
	var inspected_cell: Vector2i = battle.inspected_cell
	var inspected_skill: String = battle.inspected_skill
	var selecting_delay: bool = battle.selecting_delay
	var status: String = battle.status
	var camera_position: Vector2 = battle.world.camera.position
	var hovered: Vector2i = battle.world.hovered
	var operations: Array[Callable] = [
		func(): push_map_input(MOUSE_BUTTON_LEFT),
		func(): push_map_input(MOUSE_BUTTON_RIGHT),
		func(): battle.world.primary_clicked.emit(0, Vector2i(1, 0)),
		func(): battle.world.inspection_clicked.emit(2, Vector2i(0, 3)),
		func(): battle.ui.action_selected.emit("presentation_finish"),
		func(): battle.ui.end_turn_requested.emit(),
		func(): battle.ui.delay_selection_requested.emit(),
		func(): battle.ui.delay_target_selected.emit(2),
		func(): battle.ui.turn_order_focus_requested.emit(2),
		func(): battle.ui.inspection_closed.emit(),
		func(): battle.ui.skill_inspection_requested.emit("presentation_finish"),
	]
	for index in operations.size():
		operations[index].call()
		assert_dict(battle.state).override_failure_message("操作 %d 不可送出核心命令" % index).is_equal(before)
		assert_str(battle.pending_action).is_equal(pending_action)
		assert_vector(battle.inspected_cell).is_equal(inspected_cell)
		assert_str(battle.inspected_skill).is_equal(inspected_skill)
		assert_bool(battle.selecting_delay).is_equal(selecting_delay)
		assert_str(battle.status).is_equal(status)
		assert_vector(battle.world.camera.position).is_equal(camera_position)
		assert_vector(battle.world.hovered).is_equal(hovered)

func push_map_input(button: MouseButton) -> void:
	var viewport_position: Vector2 = battle.world.get_global_transform_with_canvas() * battle.world.cell_center(Vector2i(1, 0))
	var motion := InputEventMouseMotion.new()
	motion.position = viewport_position
	battle.get_viewport().push_input(motion, true)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = button
		event.pressed = pressed
		event.position = viewport_position
		battle.get_viewport().push_input(event, true)

func reload_battle() -> void:
	runner = await BattleTestSetup.replace_battle(self, runner, TEST_DEFINITIONS, TEST_MAP)
	battle = runner.scene()
	BattleTestSetup.reset_random_seed(battle.core, BattleTestSetup.PRESENTATION_RANDOM_SEED)
