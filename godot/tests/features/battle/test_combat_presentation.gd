extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/test_setup.gd")
const TEST_DEFINITIONS := "res://tests/features/battle/data/combat_presentation_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/combat_presentation_spikes_map.toml"
const ACTOR_ID := 1
const HEALER_ID := 2
const TARGET_ID := 3

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

# 驗證推擊命中與位移期間血量不變，抵達後才呈現地形傷害，且查詢不重複播放事件。
func test_push_then_terrain_damage_updates_health_in_order() -> void:
	runner.set_time_factor(1.0)
	var node: Node2D = battle.world.unit_nodes[TARGET_ID]
	var start_position := node.position
	var initial_health := health_fraction(node)
	var log_count: int = battle.ui.presented_log_events.size()
	assert_bool(battle.send(skill_command("presentation_push"))).is_true()
	var skill_event := last_event("skill")
	var terrain_event := last_event("terrain_damage")
	var event_count: int = battle.state.log.size()
	assert_dict(skill_event).is_not_empty()
	assert_dict(terrain_event).is_not_empty()
	assert_bool(skill_event.pushed).is_true()
	assert_int(int(skill_event.damage)).is_zero()
	assert_int(int(skill_event.remaining_hp)).is_equal(30)
	assert_bool(battle.world.is_presenting_combat_events()).is_true()
	assert_bool(is_equal_approx(health_fraction(node), initial_health)).override_failure_message("攻擊前衝期間不應提前顯示最終血量").is_true()
	assert_vector(node.position).is_equal(start_position)
	assert_int(battle.ui.presented_log_events.size()).is_equal(log_count)
	var saw_skill_health := false
	var saw_movement_with_skill_health := false
	var saw_terrain_health := false
	var destination: Vector2 = battle.world.cell_center(Vector2i(3, 1))
	for _frame in 240:
		var fraction := health_fraction(node)
		if is_equal_approx(fraction, float(skill_event.remaining_hp) / float(skill_event.max_hp)):
			saw_skill_health = true
			if not node.position.is_equal_approx(start_position):
				saw_movement_with_skill_health = true
		if is_equal_approx(fraction, float(terrain_event.remaining_hp) / float(terrain_event.max_hp)):
			saw_terrain_health = true
			assert_vector(node.position).override_failure_message("抵達地形格後才呈現地形傷害").is_equal(destination)
		battle.present()
		if not battle.world.is_presenting_combat_events():
			break
		await runner.simulate_frames(1, 16)
	assert_bool(saw_skill_health).is_true()
	assert_bool(saw_movement_with_skill_health).is_true()
	assert_bool(saw_terrain_health).is_true()
	assert_bool(battle.world.is_presenting_combat_events()).is_false()
	assert_array(battle.world.combat_event_queue).is_empty()
	assert_int(battle.ui.presented_log_events.size()).is_equal(log_count + event_count)
	assert_bool(battle.world.pending_movement_ids.has(TARGET_ID)).is_false()

# 驗證推擊受阻時，目標與被撞單位各自顯示核心決定的傷害與血條，且不播放位移。
func test_collision_presents_both_units_without_movement() -> void:
	await load_map("collision")
	runner.set_time_factor(1.0)
	var target: Node2D = battle.world.unit_nodes[TARGET_ID]
	var blocker: Node2D = battle.world.unit_nodes[5]
	var start_position := target.position
	assert_bool(battle.send(skill_command("presentation_push"))).is_true()
	var event := last_event("skill")
	assert_bool(event.push_blocked).is_true()
	assert_array(event.collision_units).is_not_empty()
	var saw_collision := false
	for _frame in 180:
		if has_floating_text(blocker, "-%d" % int(event.collision_damage)):
			saw_collision = true
			assert_bool(has_floating_text(target, "-%d" % int(event.collision_damage))).is_true()
			assert_health(target, event.remaining_hp, event.max_hp)
			assert_health(blocker, event.collision_units[0].remaining_hp, event.collision_units[0].max_hp)
			break
		await runner.simulate_frames(1)
	assert_bool(saw_collision).override_failure_message("應看到被撞單位的傷害浮字").is_true()
	await BattleTestSetup.wait_until_idle(battle, runner)
	assert_vector(target.position).is_equal(start_position)

# 驗證普通擊倒與推落懸崖都保留節點到死亡演出結束，之後清除節點並恢復操作。
func test_downed_unit_is_removed_after_death_presentation() -> void:
	var cases := [
		{"map": "spikes", "skill": "presentation_finish"},
		{"map": "cliff", "skill": "presentation_push"},
	]
	for test_case in cases:
		await load_map(test_case.map)
		runner.set_time_factor(1.0)
		var node: Node2D = battle.world.unit_nodes[TARGET_ID]
		assert_bool(battle.send(skill_command(test_case.skill))).is_true()
		assert_bool(battle.world.is_presenting_combat_events()).is_true()
		assert_bool(is_instance_valid(node)).is_true()
		await runner.simulate_frames(1)
		assert_bool(is_instance_valid(node)).override_failure_message("死亡演出不可立即移除節點").is_true()
		runner.set_time_factor(9.0)
		await BattleTestSetup.wait_until_idle(battle, runner)
		await runner.simulate_frames(1)
		assert_bool(is_instance_valid(node)).is_false()
		assert_bool(battle.world.unit_nodes.has(TARGET_ID)).is_false()
		assert_bool(battle.input_is_locked()).is_false()

# 驗證滿血與缺血治療的預覽、綠色浮字、血條及日誌都使用真實核心結果，並能切回攻擊預覽。
func test_healing_preview_and_presentation() -> void:
	for injured in [false, true]:
		await load_map("spikes")
		assert_bool(battle.send({"type": "end_turn", "actor": ACTOR_ID})).is_true()
		await BattleTestSetup.wait_until_idle(battle, runner)
		assert_int(battle.state.turn.actor).is_equal(HEALER_ID)
		var cell := Vector2i(0, 3)
		if injured:
			cell = Vector2i(0, 2)
			assert_bool(battle.send({"type": "move", "actor": HEALER_ID, "x": cell.x, "y": cell.y})).is_true()
			await BattleTestSetup.wait_until_idle(battle, runner)
		battle.world.hovered = cell
		battle.ui.action_buttons.presentation_heal.pressed.emit()
		var preview: Dictionary = battle.world.attack_preview
		assert_dict(preview).contains_keys(["healing", "remaining_hp", "health_segments"])
		assert_bool(battle.ui.attack_preview_panel.visible).is_true()
		assert_bool(battle.ui.healing_preview_segment.visible).is_equal(injured)
		assert_float(battle.ui.healing_preview_segment.size_flags_stretch_ratio).is_equal(float(preview.healing))
		assert_str(battle.ui.attack_preview_damage.text).is_equal(tr("HEAL_PREVIEW_AMOUNT") % int(preview.healing))
		assert_bool(battle.ui.attack_preview_critical.get_parent().visible).is_false()
		runner.set_time_factor(1.0)
		battle.use_pending_action(cell)
		var event := last_event("healing")
		var node: Node2D = battle.world.unit_nodes[HEALER_ID]
		assert_int(int(event.healing)).is_equal(int(preview.healing))
		assert_int(int(event.remaining_hp)).is_equal(int(preview.remaining_hp))
		assert_bool(has_floating_text(node, "+%d" % int(event.healing))).is_true()
		assert_health(node, event.remaining_hp, event.max_hp)
		for child in node.get_children():
			if child is Label and child.text == "+%d" % int(event.healing):
				assert_bool(child.get_theme_color("font_color").is_equal_approx(BattleConfig.HEALING_TEXT_COLOR)).is_true()
		runner.set_time_factor(9.0)
		await BattleTestSetup.wait_until_idle(battle, runner)
		var formatted: String = battle.ui.format_log(battle.ui.presented_log_events)
		assert_str(formatted).contains(tr("結果：%s，回復 %d HP，HP %d/%d") % [battle.ui.RESULT_STYLE % tr("治療"), int(event.healing), int(event.remaining_hp), int(event.max_hp)])
		var attack: Dictionary = battle.read_core_response(battle.core.preview_skill(ACTOR_ID, 2, 1, "presentation_push"))
		battle.ui.present_attack_preview(attack, Vector2.ZERO)
		assert_bool(battle.ui.healing_preview_segment.visible).is_false()
		assert_bool(battle.ui.attack_preview_critical.get_parent().visible).is_true()

func load_map(variant: String) -> void:
	runner = await BattleTestSetup.replace_battle(self, runner, TEST_DEFINITIONS, "res://tests/features/battle/data/combat_presentation_%s_map.toml" % variant)
	battle = runner.scene()
	BattleTestSetup.reset_random_seed(battle.core, BattleTestSetup.PRESENTATION_RANDOM_SEED)

func skill_command(skill: String) -> Dictionary:
	return {"type": "skill", "actor": ACTOR_ID, "x": 2, "y": 1, "skill": skill}

func last_event(type: String) -> Dictionary:
	for event in battle.state.log:
		if event.type == type:
			return event
	return {}

func health_fraction(node: Node2D) -> float:
	var fill: Polygon2D = node.get_node("Health/Fill")
	var background: Polygon2D = node.get_node("Health/Background")
	return (fill.polygon[1].x - fill.polygon[0].x) / (background.polygon[1].x - background.polygon[0].x)

func assert_health(node: Node2D, hp: int, max_hp: int) -> void:
	assert_bool(is_equal_approx(health_fraction(node), float(hp) / float(max_hp))).override_failure_message("血條應呈現核心事件提供的 HP %d/%d" % [hp, max_hp]).is_true()

func has_floating_text(node: Node2D, text: String) -> bool:
	for child in node.get_children():
		if child is Label and child.text == text:
			return true
	return false
