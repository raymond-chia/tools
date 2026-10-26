extends "res://tests/features/battle/battle_test_suite.gd"

const TEST_DEFINITIONS := "res://tests/features/battle/data/battle_encounter_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/battle_encounter_map.toml"

var battle
var runner: GdUnitSceneRunner

# 準備本案例的真實戰鬥場景與專用資源。
func setup_test() -> void:
	runner = battle_scene_runner(auto_free(create_battle(TEST_DEFINITIONS, TEST_MAP)))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	await wait_until_idle(battle, runner)

# 清理本案例的專用資源與尚未完成的操作。
func cleanup_test() -> void:
	while battle.state.turn.can_continue or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)

# 驗證探索時切換隊員保留移動額度，預覽完整路徑，走完後才依先攻排入敵人。
func test_move_contact_starts_initiative_immediately() -> void:
	assert_str(battle.state.battle_mode).is_equal("exploring")
	assert_int(battle.state.turn.actor).is_equal(1)
	var preview: Dictionary = JSON.parse_string(battle.core.preview_move(1, 5, 1))
	assert_bool(preview.interrupted).is_false()
	assert_int(int(preview.second[-1].x)).is_equal(5)
	assert_bool(battle.send({"type": "move", "actor": 1, "x": 2, "y": 1})).is_true()
	assert_bool(battle.send({"type": "select_unit", "actor": 2})).is_true()
	assert_int(battle.state.turn.actor).is_equal(2)
	assert_bool(battle.send({"type": "select_unit", "actor": 1})).is_true()
	assert_int(int(battle.state.turn.move_remaining)).is_equal(1)
	assert_bool(battle.send({"type": "move", "actor": 1, "x": 5, "y": 1})).is_true()
	assert_str(battle.state.battle_mode).is_equal("combat")
	for unit in battle.state.units:
		if unit.id == 1:
			assert_int(int(unit.x)).is_equal(5)
	assert_int(battle.state.turn.actor).is_equal(3)
	assert_int(int(battle.state.round)).is_equal(1)

# 驗證位移待出刀時重選自己保留技能，切換隊友則清除選取，讓隊友可正常移動。
func test_switch_player_after_step_clears_skill_selection_and_allows_move() -> void:
	battle.ui.action_buttons.step_through_the_sedge.pressed.emit()
	battle.world.primary_clicked.emit(0, Vector2i(2, 1))
	await wait_until_idle(battle, runner)
	assert_str(battle.state.turn.phase).is_equal("pending_strike")
	assert_str(battle.pending_action).is_equal("step_through_the_sedge")
	battle.ui.turn_order_focus_requested.emit(1)
	assert_str(battle.pending_action).is_equal("step_through_the_sedge")
	battle.ui.turn_order_focus_requested.emit(2)
	assert_int(battle.state.turn.actor).is_equal(2)
	assert_str(battle.pending_action).is_empty()
	battle.world.primary_clicked.emit(0, Vector2i(2, 2))
	await wait_until_idle(battle, runner)
	assert_str(battle.state.turn.phase).is_equal("moving")
	assert_int(int(battle.state.turn.move_remaining)).is_equal(1)
	for unit in battle.state.units:
		if unit.id == 2:
			assert_int(int(unit.x)).is_equal(2)
			assert_int(int(unit.y)).is_equal(2)

# 驗證踏莎行待出刀後切換隊友及切回自己，技能按鈕仍依原本順序排列。
func test_step_preserves_skill_button_order_when_switching_players() -> void:
	var expected_buttons: Array = [battle.ui.action_buttons.long_shot, battle.ui.action_buttons.step_through_the_sedge]
	assert_array(battle.ui.action_buttons_container.get_children()).contains_exactly(expected_buttons)
	battle.ui.action_buttons.step_through_the_sedge.pressed.emit()
	battle.world.primary_clicked.emit(0, Vector2i(2, 1))
	await wait_until_idle(battle, runner)
	assert_str(battle.state.turn.phase).is_equal("pending_strike")
	assert_array(battle.ui.action_buttons_container.get_children()).contains_exactly([battle.ui.action_buttons.step_through_the_sedge])
	for actor_id in [2, 1]:
		battle.ui.turn_order_focus_requested.emit(actor_id)
		await runner.simulate_frames(1)
		assert_int(battle.state.turn.actor).is_equal(actor_id)
		assert_array(battle.ui.action_buttons_container.get_children()).contains_exactly([
			battle.ui.action_buttons.long_shot,
			battle.ui.action_buttons.step_through_the_sedge,
		] if actor_id == 2 else [battle.ui.action_buttons.step_through_the_sedge])

# 驗證遠距攻擊接敵後，其餘玩家完成探索陣營回合，下一輪敵人才進入先攻順序。
func test_attack_contact_waits_for_player_faction() -> void:
	assert_str(battle.state.battle_mode).is_equal("exploring")
	assert_bool(battle.send({"type": "skill", "actor": 1, "x": 13, "y": 1, "skill": "long_shot"})).is_true()
	assert_str(battle.state.battle_mode).is_equal("attack_pending")
	assert_int(battle.state.turn.actor).is_equal(2)
	assert_int(int(battle.state.round)).is_equal(0)
	assert_bool(battle.send({"type": "end_turn", "actor": 2})).is_true()
	assert_str(battle.state.battle_mode).is_equal("combat")
	assert_int(int(battle.state.round)).is_equal(1)
	assert_int(battle.state.turn.actor).is_equal(3)

# 驗證敵方自動回合清除玩家移動預覽，滑鼠移動也不會顯示敵人的路徑與花費。
func test_enemy_turn_hides_mouse_move_preview() -> void:
	battle.world.hovered = Vector2i(2, 1)
	battle.world.update_move_preview()
	assert_bool(battle.world.first_move_path.is_empty()).is_false()
	assert_bool(battle.ui.move_cost_popup.visible).is_true()
	assert_bool(battle.send({"type": "skill", "actor": 1, "x": 13, "y": 1, "skill": "long_shot"})).is_true()
	assert_bool(battle.send({"type": "end_turn", "actor": 2})).is_true()
	assert_bool(battle.state.turn.can_continue).is_true()
	assert_bool(battle.world.first_move_path.is_empty()).is_true()
	assert_bool(battle.ui.move_cost_popup.visible).is_false()
	var target: Vector2 = battle.world.cell_center(Vector2i(12, 1))
	var mouse_motion := InputEventMouseMotion.new()
	mouse_motion.position = battle.world.get_global_transform_with_canvas() * target
	battle.world._unhandled_input(mouse_motion)
	assert_bool(battle.world.first_move_path.is_empty()).is_true()
	assert_bool(battle.world.second_move_path.is_empty()).is_true()
	assert_bool(battle.ui.move_cost_popup.visible).is_false()
