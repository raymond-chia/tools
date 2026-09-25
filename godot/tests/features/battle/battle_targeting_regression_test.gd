extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/battle_test_setup.gd")

const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITIONS := "res://tests/features/battle/data/battle_targeting_regression_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/battle_targeting_regression_map.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(BATTLE_SCENE)
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()

# 驗證點擊大型單位所占的指定格施放泥沼時，只在該格建立一份地形。
func test_mire_on_large_unit_uses_only_clicked_cell() -> void:
	await load_test_documents()
	var clicked_cell := Vector2i(5, 2)
	battle.select_action("corrosive_mire")

	push_left_click(battle.world.cell_center(clicked_cell))

	var mire_cells: Array[Vector2i] = []
	for effect in battle.state.terrain_effects:
		if effect.effect == "mire":
			mire_cells.append(Vector2i(effect.x, effect.y))
	assert_array(mire_cells).override_failure_message("大型單位上的泥沼應只出現在實際點擊格").is_equal([clicked_cell])
	var ogre := unit_with_id("ogre")
	assert_int(int(ogre.dodge)).override_failure_message("大型單位任一占用格位於泥沼時都應套用閃避減值").is_equal(2)
	assert_int(int(ogre.block)).override_failure_message("大型單位任一占用格位於泥沼時都應套用格擋減值").is_equal(3)

# 驗證查看大型單位時保留實際點擊格，不改寫為 footprint 的儲存座標。
func test_inspection_keeps_clicked_large_unit_cell() -> void:
	await load_test_documents()
	var clicked_cell := Vector2i(5, 2)

	push_mouse_button(battle.world.cell_center(clicked_cell), MOUSE_BUTTON_RIGHT)

	assert_vector(battle.inspected_cell).override_failure_message("查看大型單位應以玩家實際點擊格為準").is_equal(clicked_cell)

# 驗證單體技能射程以大型單位的實際點擊格計算，距離太遠時回報對應錯誤識別碼。
func test_unit_skill_uses_clicked_large_unit_cell() -> void:
	await load_test_documents()
	battle.select_action("shield_bash")
	push_mouse_button(battle.world.cell_center(Vector2i(5, 2)), MOUSE_BUTTON_LEFT)
	assert_str(battle.status).override_failure_message("點擊射程外的占用格應由核心拒絕").is_equal("ERROR_TARGET_TOO_FAR")
	assert_int(int(unit_with_id("ogre").hp)).is_equal(100)

	battle.select_action("shield_bash")
	push_mouse_button(battle.world.cell_center(Vector2i(4, 1)), MOUSE_BUTTON_LEFT)
	assert_str(battle.status).override_failure_message("點擊射程內的占用格應成功結算").is_empty()
	assert_int(int(unit_with_id("ogre").hp)).override_failure_message("同一大型單位的近側占用格應可被命中").is_equal(93)

# 驗證等待自動回合完成後，泥沼增加移動消耗且只維持兩個回合。
func test_mire_movement_cost_and_duration() -> void:
	await load_test_documents()
	var mire_cell := Vector2i(1, 1)
	assert_bool(battle.send({"type": "cell_skill", "actor": "aria", "x": mire_cell.x, "y": mire_cell.y, "skill": "corrosive_mire"})).override_failure_message("泥沼應成功施放").is_true()
	await wait_for_combat_events()
	var terrain := terrain_at(mire_cell)
	assert_int(int(terrain.cost)).override_failure_message("泥沼地格應增加一點移動消耗").is_equal(2)
	var mire_effect: Dictionary = {}
	for effect in battle.state.terrain_effects:
		if effect.x == mire_cell.x and effect.y == mire_cell.y and effect.effect == "mire":
			mire_effect = effect
	assert_dict(mire_effect).override_failure_message("泥沼效果應位於施放格").is_not_empty()
	assert_int(int(mire_effect.remaining_rounds)).override_failure_message("進入下一輪後泥沼應剩餘一輪").is_equal(1)
	assert_bool(battle.send({"type": "end_turn", "actor": "aria"})).override_failure_message("應能結束下一輪以推進泥沼期限").is_true()
	await wait_for_combat_events()
	assert_array(terrain_at(mire_cell).terrains).override_failure_message("兩輪結束後泥沼應消失").is_empty()

# 驗證推擊命中會沿攻擊者到目標的方向移動一格。
func test_push_hit_moves_target_one_cell() -> void:
	await load_test_documents()
	assert_bool(battle.send({"type": "skill", "actor": "aria", "target": "wolf_a", "x": 2, "y": 1, "skill": "shield_bash"})).override_failure_message("推擊應成功命中測試目標").is_true()
	var target := unit_with_id("wolf_a")
	assert_int(int(target.x)).override_failure_message("目標應沿攻擊方向向右移動一格").is_equal(3)
	assert_int(int(target.y)).is_equal(1)
	var event := find_last_event("skill")
	assert_bool(event.pushed).override_failure_message("技能事件應記錄成功推動").is_true()
	assert_int(int(event.collision_damage)).is_zero()

# 驗證大型目標無法被推出地圖時不移動，並受到碰撞傷害。
func test_blocked_push_deals_collision_damage() -> void:
	await load_test_documents()
	assert_bool(battle.send({"type": "skill", "actor": "aria", "target": "ogre", "x": 4, "y": 1, "skill": "shield_bash"})).override_failure_message("對地圖邊界的大型目標推擊應完成結算").is_true()
	var target := unit_with_id("ogre")
	assert_int(int(target.x)).override_failure_message("受阻的大型目標不應移動").is_equal(4)
	assert_int(int(target.hp)).override_failure_message("受阻推擊應造成兩點碰撞傷害及技能傷害").is_equal(93)
	var event := find_last_event("skill")
	assert_bool(event.pushed).is_false()
	assert_int(int(event.collision_damage)).override_failure_message("技能事件應記錄碰撞傷害").is_equal(2)

# 驗證攻擊動畫與自動回合完成後，倒下單位的格子可再次進入。
func test_downed_unit_does_not_block_cell() -> void:
	await load_test_documents()
	assert_bool(battle.send({"type": "skill", "actor": "aria", "target": "wolf_a", "x": 2, "y": 1, "skill": "finishing_strike"})).override_failure_message("終結攻擊應使測試目標倒下").is_true()
	await wait_for_combat_events()
	assert_dict(unit_with_id("wolf_a")).override_failure_message("倒下單位不應出現在 presentation snapshot").is_empty()
	assert_bool(battle.send({"type": "move", "actor": "aria", "x": 2, "y": 1})).override_failure_message("屍體所在格應可進入").is_true()
	var actor := unit_with_id("aria")
	assert_int(int(actor.x)).is_equal(2)
	assert_int(int(actor.y)).is_equal(1)

func load_test_documents() -> void:
	battle.pending_action = ""
	var setup_error: String = await BattleTestSetup.load_and_start(battle, runner, TEST_DEFINITIONS, TEST_MAP)
	assert_str(setup_error).override_failure_message(setup_error).is_empty()

func wait_for_combat_events() -> void:
	while battle.state.turn.auto_step or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)

func push_left_click(local_position: Vector2) -> void:
	push_mouse_button(local_position, MOUSE_BUTTON_LEFT)

func push_mouse_button(local_position: Vector2, button: MouseButton) -> void:
	var screen_position: Vector2 = battle.get_viewport().get_screen_transform() * battle.world.get_global_transform_with_canvas() * local_position
	runner.set_mouse_position(screen_position)
	runner.simulate_mouse_button_pressed(button)

func terrain_at(cell: Vector2i) -> Dictionary:
	for terrain in battle.state.terrain_cells:
		if terrain.x == cell.x and terrain.y == cell.y:
			return terrain
	return {}

func unit_with_id(id: String) -> Dictionary:
	for unit in battle.state.units:
		if unit.id == id:
			return unit
	return {}

func find_last_event(type: String) -> Dictionary:
	for index in range(battle.state.log.size() - 1, -1, -1):
		var event: Dictionary = battle.state.log[index]
		if event.type == type:
			return event
	return {}
