extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/battle_test_setup.gd")
const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITIONS := "res://tests/features/battle/data/battle_encounter_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/battle_encounter_map.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(BATTLE_SCENE)
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	var setup_error: String = await BattleTestSetup.load_and_start(battle, runner, TEST_DEFINITIONS, TEST_MAP)
	assert_str(setup_error).override_failure_message(setup_error).is_empty()

func after_test() -> void:
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
