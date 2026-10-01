extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/battle_test_setup.gd")
const TEST_DEFINITIONS := "res://tests/features/battle/data/battle_encounter_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/battle_encounter_map.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(auto_free(BattleTestSetup.create_battle(TEST_DEFINITIONS, TEST_MAP)))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	await BattleTestSetup.wait_until_idle(battle, runner)

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
