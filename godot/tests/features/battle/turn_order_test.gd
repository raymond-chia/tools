extends GdUnitTestSuite

const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITION := "res://tests/features/battle/data/turn_order.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(BATTLE_SCENE)
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	await load_test_definition()

# 驗證延後模式以橫棒標示插入位置，並將目前單位排到所選單位之後。
func test_delay_uses_highlighted_slot_and_reorders_turns() -> void:
	var delayed_actor: String = battle.state.turn.actor
	var target_actor: String = battle.state.turn_order[0]
	var target_slot: VBoxContainer = battle.ui.turn_order.get_child(battle.ui.turn_order.get_child_count() - 1)
	var marker: ColorRect = target_slot.get_child(0)

	assert_bool(marker.color.is_equal_approx(BattleVisualConfig.DELAY_SLOT_COLOR)).override_failure_message("延後位置平時應顯示低亮度橫棒").is_true()
	battle.ui.delay_button.pressed.emit()
	await wait_for_combat_events()
	target_slot = battle.ui.turn_order.get_child(battle.ui.turn_order.get_child_count() - 1)
	marker = target_slot.get_child(0)
	var target_button: Button = target_slot.get_child(1)
	target_button.mouse_entered.emit()
	assert_bool(battle.selecting_delay).override_failure_message("按下延後後應進入位置選擇模式").is_true()
	assert_bool(marker.color.is_equal_approx(BattleVisualConfig.DELAY_SLOT_HIGHLIGHT_COLOR)).override_failure_message("游標指向的延後位置應高亮").is_true()

	battle._on_delay_target_selected(target_actor)
	await wait_for_combat_events()

	assert_str(battle.status).override_failure_message("延後指令應成功").is_empty()
	assert_str(battle.state.turn.actor).override_failure_message("所選單位應成為目前行動者；預期 %s，實際 %s" % [target_actor, battle.state.turn.actor]).is_equal(target_actor)
	assert_str(battle.state.turn_order[0]).override_failure_message("原行動者應排在所選單位之後；預期 %s，實際 %s" % [delayed_actor, battle.state.turn_order[0]]).is_equal(delayed_actor)
	assert_bool(battle.selecting_delay).override_failure_message("完成延後後應離開位置選擇模式").is_false()

# 驗證目前單位一旦移動，本回合便不能再延後。
func test_delay_is_disabled_after_moving() -> void:
	var actor: String = battle.state.turn.actor
	var destination := empty_reachable_cell()

	assert_dict(destination).override_failure_message("測試資料應提供未占用的可達格").is_not_empty()
	var moved: bool = battle.send({"type": "move", "actor": actor, "x": int(destination.x), "y": int(destination.y)})
	assert_bool(moved).override_failure_message("測試單位應能移動到可達格：%s" % battle.status).is_true()
	await wait_for_combat_events()

	assert_bool(battle.state.turn.can_delay).override_failure_message("移動後核心應禁止延後").is_false()
	assert_bool(battle.ui.delay_button.disabled).override_failure_message("移動後延後按鈕應停用").is_true()

func load_test_definition() -> void:
	await wait_for_combat_events()
	for child in battle.world.units_layer.get_children():
		child.free()
	battle.world.unit_nodes.clear()
	battle.pending_action = ""
	battle.selecting_delay = false
	var definition := FileAccess.get_file_as_string(TEST_DEFINITION)
	var loaded = JSON.parse_string(battle.core.load_definition(definition))
	assert_bool(loaded.has("error")).override_failure_message("專用 TOML 應成功載入").is_false()
	if loaded.has("error"):
		return
	battle.state = loaded
	battle.world.setup_map(loaded)
	assert_bool(battle.send({"type": "start"})).override_failure_message("專用測試戰鬥應成功開始").is_true()
	await wait_for_combat_events()

func wait_for_combat_events() -> void:
	while battle.state.turn.auto_step or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)

func empty_reachable_cell() -> Dictionary:
	for reachable in battle.state.reachable:
		for terrain in battle.state.terrain_cells:
			if terrain.x == reachable.x and terrain.y == reachable.y and terrain.unit_id == null:
				return reachable
	return {}
