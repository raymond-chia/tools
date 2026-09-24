extends GdUnitTestSuite

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
	while battle.state.turn.auto_step or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)
	for child in battle.world.units_layer.get_children():
		child.free()
	battle.world.unit_nodes.clear()
	var definitions := FileAccess.get_file_as_string(TEST_DEFINITIONS)
	var map := FileAccess.get_file_as_string(TEST_MAP)
	var loaded: Dictionary = JSON.parse_string(battle.core.load_documents(definitions, map))
	assert_bool(loaded.has("error")).override_failure_message("專用 TOML 應成功載入").is_false()
	if loaded.has("error"):
		return
	battle.state = loaded
	battle.world.setup_map(loaded)
	assert_bool(battle.send({"type": "start"})).override_failure_message("專用測試戰鬥應成功開始").is_true()
	await wait_for_battle_idle()

# 驗證跨輪後先更新先攻順序，並在敵人攻擊動畫結束後才交給下一位。
func test_new_round_order_precedes_enemy_action() -> void:
	assert_str(battle.state.turn.actor).is_equal("aria")
	assert_bool(battle.send({"type": "end_turn", "actor": "aria"})).override_failure_message("玩家應可結束回合").is_true()
	assert_int(int(battle.state.round)).is_equal(1)
	while int(battle.state.round) < 2:
		await runner.simulate_frames(1)
	assert_str(battle.state.turn.actor).override_failure_message("新輪第一位應是敵人").is_equal("wolf")
	assert_str(battle.state.log[-1].type).override_failure_message("敵人行動前應先記錄新輪先攻").is_equal("new_round")
	assert_int(int(battle.displayed_state.round)).override_failure_message("新輪順序應已交給介面顯示").is_equal(2)
	assert_str(battle.ui.actor_name.text).override_failure_message("介面應先顯示新輪的敵方行動者").is_equal("測試荒原狼")
	assert_int(battle.ui.turn_order.get_child_count()).override_failure_message("介面應顯示下一位玩家").is_equal(1)
	var round_start_log_size: int = battle.state.log.size()
	for _frame in 60:
		if battle.world.is_presenting_combat_events():
			break
		await runner.simulate_frames(1)
	assert_bool(battle.world.is_presenting_combat_events()).override_failure_message("新輪敵人應開始攻擊動畫").is_true()
	assert_str(battle.state.turn.actor).override_failure_message("敵人攻擊動畫期間不可換成下一位").is_equal("wolf")
	assert_str(battle.state.turn.phase).override_failure_message("敵人攻擊結算後應等待動畫完成才推進回合").is_equal("ended")
	await wait_for_battle_idle()
	assert_str(battle.state.log[round_start_log_size].type).override_failure_message("更新順序後敵人才進行攻擊").is_equal("skill")
	assert_str(battle.state.turn.actor).override_failure_message("敵人動畫結束後應輪到玩家").is_equal("aria")

func wait_for_battle_idle() -> void:
	while battle.state.turn.auto_step or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)
