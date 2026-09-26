extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/battle_test_setup.gd")

const ARIA_ID := 1
const WOLF_ID := 2

const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITIONS := "res://tests/features/battle/data/battle_turn_sequence_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/battle_turn_sequence_map.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(BATTLE_SCENE)
	runner.set_time_factor(1.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	var setup_error: String = await BattleTestSetup.load_and_start(battle, runner, TEST_DEFINITIONS, TEST_MAP)
	assert_str(setup_error).override_failure_message(setup_error).is_empty()

# 驗證跨輪後先更新先攻順序與翻譯後的敵人名稱，並在敵人攻擊動畫結束後才交給下一位。
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
	await wait_for_battle_idle()
	assert_str(battle.ui.presented_log_events[round_start_log_size].type).override_failure_message("更新順序後敵人才進行攻擊").is_equal("skill")
	assert_int(battle.state.turn.actor).override_failure_message("敵人動畫結束後應輪到玩家").is_equal(ARIA_ID)

func wait_for_battle_idle() -> void:
	while battle.state.turn.can_continue or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)
