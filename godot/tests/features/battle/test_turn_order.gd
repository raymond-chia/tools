extends "res://tests/features/battle/battle_test_suite.gd"

const TEST_DEFINITIONS := "res://tests/features/battle/data/turn_order_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/turn_order_map.toml"

var battle
var runner: GdUnitSceneRunner

# 準備本案例的真實戰鬥場景與專用資源。
func setup_test() -> void:
	runner = battle_scene_runner(auto_free(create_battle(TEST_DEFINITIONS, TEST_MAP)))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	await wait_until_idle(battle, runner)

# 驗證延後模式以橫棒標示插入位置，並將目前單位排到所選單位之後。
func test_delay_uses_highlighted_slot_and_reorders_turns() -> void:
	var delayed_actor: int = battle.state.turn.actor
	var target_actor: int = battle.state.turn_order[0]
	var target_slot: VBoxContainer = battle.ui.turn_order.get_child(battle.ui.turn_order.get_child_count() - 1)
	var marker: ColorRect = target_slot.get_child(0)

	assert_bool(marker.color.is_equal_approx(BattleConfig.DELAY_SLOT_COLOR)).override_failure_message("延後位置平時應顯示低亮度橫棒").is_true()
	battle.ui.delay_button.pressed.emit()
	await wait_for_combat_events()
	target_slot = battle.ui.turn_order.get_child(battle.ui.turn_order.get_child_count() - 1)
	marker = target_slot.get_child(0)
	var target_button: Button = target_slot.get_child(1)
	target_button.mouse_entered.emit()
	assert_bool(battle.selecting_delay).override_failure_message("按下延後後應進入位置選擇模式").is_true()
	assert_bool(marker.color.is_equal_approx(BattleConfig.DELAY_SLOT_HIGHLIGHT_COLOR)).override_failure_message("游標指向的延後位置應高亮").is_true()

	battle._on_delay_target_selected(target_actor)
	await wait_for_combat_events()

	assert_str(battle.status).override_failure_message("延後指令應成功").is_empty()
	assert_int(battle.state.turn.actor).override_failure_message("所選單位應成為目前行動者；預期 %s，實際 %s" % [target_actor, battle.state.turn.actor]).is_equal(target_actor)
	assert_int(battle.state.turn_order[0]).override_failure_message("原行動者應排在所選單位之後；預期 %s，實際 %s" % [delayed_actor, battle.state.turn_order[0]]).is_equal(delayed_actor)
	assert_bool(battle.selecting_delay).override_failure_message("完成延後後應離開位置選擇模式").is_false()

# 驗證目前單位一旦移動，本回合便不能再延後。
func test_delay_is_disabled_after_moving() -> void:
	var actor: int = battle.state.turn.actor
	var destination := empty_reachable_cell()

	assert_dict(destination).override_failure_message("測試資料應提供未占用的可達格").is_not_empty()
	var moved: bool = battle.send({"type": "move", "actor": actor, "x": int(destination.x), "y": int(destination.y)})
	assert_bool(moved).override_failure_message("測試單位應能移動到可達格：%s" % battle.status).is_true()
	await wait_for_combat_events()

	assert_bool(battle.state.turn.can_delay).override_failure_message("移動後核心應禁止延後").is_false()
	assert_bool(battle.ui.delay_button.disabled).override_failure_message("移動後延後按鈕應停用").is_true()

# 驗證順序列右鍵開啟、切換與重複查看詳情，且不切換操作單位或誤觸延後。
func test_right_click_turn_order_opens_details_without_changing_turn() -> void:
	var cases := [
		{"name": "開啟詳情", "unit_id": 2, "delay": false},
		{"name": "切換詳情", "unit_id": 3, "delay": false},
		{"name": "重複查看保持開啟", "unit_id": 3, "delay": false},
		{"name": "延後模式只查看", "unit_id": 2, "delay": true},
	]
	assert_bool(battle.ui.info_panel.visible).is_false()
	for test_case in cases:
		if test_case.delay:
			battle.ui.delay_button.pressed.emit()
		var original_actor: int = battle.state.turn.actor
		var original_order: Array = battle.state.turn_order.duplicate()
		var button: Button = battle.ui.turn_order_slots[test_case.unit_id].get_node("Button")
		var screen_position := button.get_viewport().get_screen_transform() * button.get_global_transform_with_canvas() * (button.size / 2.0)
		runner.set_mouse_position(screen_position)
		runner.simulate_mouse_button_pressed(MOUSE_BUTTON_RIGHT)
		runner.simulate_mouse_button_release(MOUSE_BUTTON_RIGHT)

		assert_bool(battle.ui.info_panel.is_visible_in_tree()).override_failure_message(test_case.name).is_true()
		assert_bool(battle.ui.unit_details.is_visible_in_tree()).override_failure_message(test_case.name).is_true()
		assert_int(battle.ui.inspected_unit_id).override_failure_message(test_case.name).is_equal(test_case.unit_id)
		assert_vector(battle.inspected_cell).override_failure_message(test_case.name).is_equal(battle.world.unit_cell(test_case.unit_id))
		assert_int(battle.state.turn.actor).override_failure_message("%s：不切換操作單位" % test_case.name).is_equal(original_actor)
		assert_array(battle.state.turn_order).override_failure_message("%s：不改變行動順序" % test_case.name).is_equal(original_order)
		assert_bool(battle.selecting_delay).override_failure_message("%s：維持延後模式" % test_case.name).is_equal(test_case.delay)

func wait_for_combat_events() -> void:
	while battle.state.turn.can_continue or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)

func empty_reachable_cell() -> Dictionary:
	for reachable in battle.state.reachable:
		for terrain in battle.state.terrain_cells:
			if terrain.x == reachable.x and terrain.y == reachable.y and terrain.unit_id == null:
				return reachable
	return {}
