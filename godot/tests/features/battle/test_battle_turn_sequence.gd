extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/test_setup.gd")

const ARIA_ID := 1
const WOLF_ID := 2

const TEST_DEFINITIONS := "res://tests/features/battle/data/battle_turn_sequence_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/battle_turn_sequence_map.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(auto_free(BattleTestSetup.create_battle(TEST_DEFINITIONS, TEST_MAP)))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	await BattleTestSetup.wait_until_idle(battle, runner)

# 驗證跨輪先更新順序與敵人名稱，且敵人攻擊動畫期間持續保留其回合直到演出結束。
func test_new_round_order_precedes_enemy_action() -> void:
	assert_int(battle.state.turn.actor).is_equal(ARIA_ID)
	assert_bool(battle.send({"type": "end_turn", "actor": ARIA_ID})).override_failure_message("玩家應可結束回合").is_true()
	assert_int(int(battle.state.round)).is_equal(1)
	while int(battle.state.round) < 2:
		await runner.simulate_frames(1)
	assert_int(battle.state.turn.actor).override_failure_message("新輪第一位應是敵人").is_equal(WOLF_ID)
	assert_str(battle.ui.presented_log_events[-1].type).override_failure_message("敵人行動前應先記錄新輪先攻").is_equal("new_round")
	assert_int(int(battle.displayed_state.round)).override_failure_message("新輪順序應已交給介面顯示").is_equal(2)
	assert_str(battle.ui.actor_name.text).override_failure_message("介面應先顯示新輪的敵方行動者").is_equal(tr("UNIT_NAME_WOLF"))
	assert_int(battle.ui.turn_order.get_child_count()).override_failure_message("介面應顯示下一位玩家").is_equal(1)
	var round_start_log_size: int = battle.ui.presented_log_events.size()
	for _frame in 60:
		if battle.world.is_presenting_combat_events():
			break
		await runner.simulate_frames(1)
	assert_bool(battle.world.is_presenting_combat_events()).override_failure_message("新輪敵人應開始攻擊動畫").is_true()
	assert_int(battle.state.turn.actor).override_failure_message("敵人攻擊動畫期間不可換成下一位").is_equal(WOLF_ID)
	assert_str(battle.state.turn.phase).override_failure_message("敵人攻擊結算後應等待動畫完成才推進回合").is_equal("ended")
	while battle.world.is_presenting_combat_events():
		assert_int(battle.state.turn.actor).override_failure_message("敵人攻擊動畫尚未結束時不可換成下一位").is_equal(WOLF_ID)
		await runner.simulate_frames(1)
	await wait_for_battle_idle()
	assert_str(battle.ui.presented_log_events[round_start_log_size].type).override_failure_message("更新順序後敵人才進行攻擊").is_equal("skill")
	assert_int(battle.state.turn.actor).override_failure_message("敵人動畫結束後應輪到玩家").is_equal(ARIA_ID)

# 驗證敵人自動回合等待期間與攻擊演出期間均阻擋操作，回到玩家後可重新選技能。
func test_enemy_automatic_turn_locks_inputs_before_and_during_animation() -> void:
	runner.set_time_factor(1.0)
	battle.ui.end_turn_requested.emit()
	assert_bool(battle.state.turn.can_continue).is_true()
	assert_locked_operations()
	var saw_animation := false
	for _frame in 240:
		if battle.world.is_presenting_combat_events():
			saw_animation = true
			assert_locked_operations()
			break
		await runner.simulate_frames(1)
	assert_bool(saw_animation).is_true()
	runner.set_time_factor(9.0)
	await wait_for_battle_idle()
	assert_bool(battle.input_is_locked()).is_false()
	battle.ui.action_buttons.melee_attack.pressed.emit()
	assert_str(battle.pending_action).is_equal("melee_attack")

func assert_locked_operations() -> void:
	var before: Dictionary = battle.state.duplicate(true)
	var camera_position: Vector2 = battle.world.camera.position
	for operation: Callable in [
		func(): battle.world.primary_clicked.emit(0, Vector2i(1, 1)),
		func(): battle.world.inspection_clicked.emit(WOLF_ID, Vector2i(2, 0)),
		func(): battle.ui.action_selected.emit("melee_attack"),
		func(): battle.ui.end_turn_requested.emit(),
		func(): battle.ui.delay_selection_requested.emit(),
		func(): battle.ui.delay_target_selected.emit(WOLF_ID),
		func(): battle.ui.turn_order_focus_requested.emit(WOLF_ID),
		func(): battle.ui.skill_inspection_requested.emit("melee_attack"),
		func(): battle.ui.inspection_closed.emit(),
	]:
		operation.call()
		assert_dict(battle.state).is_equal(before)
		assert_str(battle.pending_action).is_empty()
		assert_vector(battle.inspected_cell).is_equal(Vector2i(-1, -1))
		assert_str(battle.inspected_skill).is_empty()
		assert_bool(battle.selecting_delay).is_false()
		assert_vector(battle.world.camera.position).is_equal(camera_position)

func wait_for_battle_idle() -> void:
	while battle.state.turn.can_continue or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)
