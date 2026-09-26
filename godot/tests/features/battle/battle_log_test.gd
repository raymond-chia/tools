extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/battle_test_setup.gd")

const ARIA_ID := 1
const WOLF_A_ID := 2
const WOLF_B_ID := 3

const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITIONS := "res://tests/features/battle/data/battle_log_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/battle_log_map.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(BATTLE_SCENE)
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	await load_test_documents()

# 驗證開始戰鬥會記錄新回合，並以整數顯示回合數。
func test_new_round_log() -> void:
	var event: Dictionary = battle.ui.presented_log_events[0]
	assert_str(event.type).is_equal("new_round")
	assert_dict(event).override_failure_message("應產生新回合事件").is_not_empty()
	assert_int(int(event.round)).override_failure_message("新回合事件應記錄目前輪數").is_equal(1)
	assert_str(formatted_log()).override_failure_message("戰鬥紀錄應顯示整數回合數").contains("── 第 1 輪 ──")

# 驗證新回合事件提供單位實例 ID 與已排序的先攻擲骰明細，且總值等於擲骰與加值之和。
func test_new_round_log_contains_initiative_rolls() -> void:
	var event: Dictionary = battle.ui.presented_log_events[0]
	assert_str(event.type).is_equal("new_round")
	assert_array(event.initiative_rolls).override_failure_message("新回合事件應提供先攻擲骰明細").is_not_empty()
	assert_int(event.initiative_rolls.size()).is_equal(3)
	var initiative_roll: Dictionary = event.initiative_rolls[0]
	assert_int(initiative_roll.unit).is_equal(ARIA_ID)
	assert_str(initiative_roll.team).is_equal("player")
	assert_int(int(initiative_roll.modifier)).is_equal(100)
	var previous_total := 1000
	for roll in event.initiative_rolls:
		assert_int(int(roll.total)).is_equal(int(roll.roll) + int(roll.modifier))
		assert_int(int(roll.total)).is_less_equal(previous_total)
		previous_total = int(roll.total)

# 驗證每筆紀錄依事件類型決定預設展開狀態。
func test_log_entries_use_event_default_expansion() -> void:
	assert_bool(battle.ui.log_entry_expanded_states[0]).override_failure_message("新回合與先攻紀錄應預設摺疊").is_false()
	assert_bool(battle.send(skill_command(WOLF_A_ID, "precise_strike"))).override_failure_message("測試技能應成功施放").is_true()
	await wait_for_combat_events()
	var skill_index := find_last_event_index("skill", ARIA_ID)
	assert_bool(battle.ui.log_entry_expanded_states[skill_index]).override_failure_message("技能紀錄應預設展開").is_true()

# 驗證切換一筆紀錄只改變該筆狀態，且新增紀錄後保留既有狀態。
func test_log_entries_preserve_independent_expansion_states() -> void:
	battle.ui.toggle_log_entry(0)
	assert_bool(battle.ui.log_entry_expanded_states[0]).override_failure_message("新回合紀錄應可獨立展開").is_true()
	assert_str(formatted_log()).override_failure_message("展開新回合紀錄應顯示先攻明細").contains("先攻總值")
	assert_bool(battle.send(skill_command(WOLF_A_ID, "precise_strike"))).override_failure_message("測試技能應成功施放").is_true()
	await wait_for_combat_events()
	var skill_index := find_last_event_index("skill", ARIA_ID)
	assert_bool(battle.ui.log_entry_expanded_states[0]).override_failure_message("新增紀錄後應保留既有展開狀態").is_true()
	battle.ui.toggle_log_entry(skill_index)
	assert_bool(battle.ui.log_entry_expanded_states[0]).override_failure_message("切換技能紀錄不應影響新回合紀錄").is_true()
	assert_bool(battle.ui.log_entry_expanded_states[skill_index]).override_failure_message("技能紀錄應可獨立摺疊").is_false()
	assert_str(formatted_log()).override_failure_message("摺疊技能紀錄應隱藏攻擊判定明細").not_contains("攻擊加值 104")

# 驗證技能紀錄，且 UI 依技能 ID 翻譯名稱並顯示數值。
func test_skill_resolution_log() -> void:
	assert_bool(battle.send(skill_command(WOLF_A_ID, "precise_strike"))).override_failure_message("測試技能應成功施放").is_true()
	await wait_for_combat_events()
	var event := find_last_event("skill", ARIA_ID)
	assert_dict(event).override_failure_message("應產生技能事件").is_not_empty()
	assert_int(event.actor).is_equal(ARIA_ID)
	assert_str(event.actor_team).is_equal("player")
	assert_str(event.skill).is_equal("precise_strike")
	assert_int(event.target).is_equal(WOLF_A_ID)
	assert_dict(event.target_team).contains_key_value("enemy", "targets")
	assert_int(int(event.attack_modifier)).is_equal(104)
	assert_int(int(event.attack_total)).is_equal(int(event.roll) + 104)
	assert_int(int(event.dodge_target)).is_equal(12)
	assert_int(int(event.block_target)).is_equal(15)
	assert_bool(event.critical).is_equal(int(event.roll) == 20)
	var base_damage: int = 6 if event.critical else 3
	var expected_damage: int = int({"dodge": 0, "block": max(base_damage - 2, 0), "hit": base_damage}[event.result])
	assert_int(int(event.damage)).is_equal(expected_damage)
	assert_int(int(event.remaining_hp)).is_equal(10000 - expected_damage)
	assert_int(int(event.max_hp)).is_equal(10000)
	var result_names := {"dodge": "[color=#f0c96a]閃避[/color]", "block": "[color=#f0c96a]格擋[/color]", "hit": "[color=#f0c96a]命中[/color]"}
	var critical_text := "，暴擊" if event.critical else ""
	var log_text: String = formatted_log()
	assert_str(log_text).contains("[color=#63a9ff]%s[/color] 使用「%s」影響 [color=#ff6868]%s[/color]" % [tr("UNIT_NAME_ARIA"), tr(BattleConfig.skill_name_key("precise_strike")), tr("UNIT_NAME_WOLF_A")])
	assert_str(log_text).contains("D20 擲骰 %d + 攻擊加值 104 = 攻擊總值 %d" % [int(event.roll), int(event.attack_total)])
	assert_str(log_text).contains("目標防禦：閃避門檻 12／格擋門檻 15")
	assert_str(log_text).contains("結果：%s%s，%d 傷害，HP %d/10000" % [result_names[event.result], critical_text, expected_damage, 10000 - expected_damage])

# 驗證技能使生命歸零時會記錄倒下狀態，並翻譯倒下單位名稱。
func test_downed_unit_log() -> void:
	assert_bool(battle.send(skill_command(WOLF_B_ID, "finishing_strike"))).override_failure_message("終結技能應成功施放").is_true()
	await wait_for_combat_events()
	var event := find_last_event("skill", ARIA_ID)
	assert_bool(event.downed).override_failure_message("生命歸零的目標應標記為倒下").is_true()
	assert_int(int(event.remaining_hp)).override_failure_message("倒下目標的剩餘生命應為零").is_zero()
	assert_str(formatted_log()).override_failure_message("戰鬥紀錄應以敵方顏色顯示倒下單位").contains("[color=#ff6868]%s[/color] 倒下" % tr("UNIT_NAME_WOLF_B"))

# 驗證踩到地刺會停止移動、扣除固定傷害，並以實例 ID 記錄與顯示結果。
func test_spikes_damage_log() -> void:
	var terrain := terrain_at(Vector2i(0, 2))
	assert_array(terrain.terrains).override_failure_message("測試地格應包含地刺").contains("spikes")
	assert_int(int(terrain.damage)).override_failure_message("地刺資訊應由核心提供固定傷害").is_equal(3)
	assert_bool(battle.send({"type": "move", "actor": ARIA_ID, "x": 0, "y": 2})).override_failure_message("移動到地刺地格應成功").is_true()
	await wait_for_combat_events()
	var event := find_last_event("terrain_damage")
	assert_dict(event).override_failure_message("應產生地形傷害事件").is_not_empty()
	assert_int(event.target).is_equal(ARIA_ID)
	assert_str(event.target_team).is_equal("player")
	assert_str(event.terrain).is_equal("spikes")
	assert_int(int(event.damage)).is_equal(3)
	assert_int(int(event.remaining_hp)).is_equal(47)
	assert_int(int(event.max_hp)).is_equal(50)
	assert_bool(event.downed).is_false()
	var actor := unit_with_id(ARIA_ID)
	assert_int(int(actor.hp)).override_failure_message("踩到地刺後 snapshot 應反映剩餘生命").is_equal(47)
	assert_int(int(actor.x)).is_equal(0)
	assert_int(int(actor.y)).override_failure_message("角色應停在觸發地刺的格子").is_equal(2)
	assert_str(formatted_log()).override_failure_message("戰鬥紀錄應顯示地刺傷害與剩餘生命").contains("[color=#63a9ff]%s[/color] 踩到「地刺」，受到 3 點傷害，HP 47/50" % tr("UNIT_NAME_ARIA"))

# 驗證十二次真實攻擊與自動回合完成後，長戰鬥紀錄仍可拖曳捲動。
func test_battle_log_can_be_dragged_to_scroll() -> void:
	# 準備長紀錄時不需反覆繪製面板，完成後再顯示並檢查捲動。
	# 如果不隱藏，會嚴重拖延測試時間
	battle.ui.log_panel.hide()
	for index in 12:
		assert_bool(battle.send(skill_command(WOLF_A_ID, "precise_strike"))).override_failure_message("第 %d 次測試攻擊應成功" % index).is_true()
		await wait_for_combat_events()
	battle.ui.log_panel.show()
	await runner.simulate_frames(2)
	var scroll_bar: VScrollBar = battle.ui.battle_log.get_v_scroll_bar()
	assert_bool(scroll_bar.max_value > scroll_bar.page).override_failure_message("十二次攻擊後的紀錄應超出面板並可捲動").is_true()
	scroll_bar.value = scroll_bar.max_value
	var initial_value: float = scroll_bar.value
	var press := InputEventMouseButton.new()
	press.button_index = MOUSE_BUTTON_LEFT
	press.pressed = true
	battle.ui.battle_log.gui_input.emit(press)
	var motion := InputEventMouseMotion.new()
	motion.relative = Vector2(0.0, 80.0)
	battle.ui.battle_log.gui_input.emit(motion)
	var release := InputEventMouseButton.new()
	release.button_index = MOUSE_BUTTON_LEFT
	release.pressed = false
	battle.ui.battle_log.gui_input.emit(release)
	assert_float(scroll_bar.value).override_failure_message("向下拖曳紀錄內容應向上捲動").is_less(initial_value)

# 驗證無斷點的長紀錄仍會換行，且紀錄控制項不超出所屬面板。
func test_battle_log_content_stays_inside_panel_width() -> void:
	battle.ui.battle_log.text = "很長的戰鬥紀錄".repeat(100)
	await runner.simulate_frames(2)
	var panel: Control = battle.ui.get_node("Root/LogPanel")
	assert_int(battle.ui.battle_log.get_content_width()).override_failure_message("戰鬥紀錄內容寬度不應超出控制項").is_less_equal(int(battle.ui.battle_log.size.x))
	assert_bool(panel.get_global_rect().encloses(battle.ui.battle_log.get_global_rect())).override_failure_message("戰鬥紀錄控制項應完整位於面板內").is_true()

func load_test_documents() -> void:
	var setup_error: String = await BattleTestSetup.load_and_start(battle, runner, TEST_DEFINITIONS, TEST_MAP)
	assert_str(setup_error).override_failure_message(setup_error).is_empty()

func wait_for_combat_events() -> void:
	while battle.state.turn.can_continue or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)

func formatted_log() -> String:
	return battle.ui.format_log(battle.ui.presented_log_events)

func skill_command(target: int, skill: String) -> Dictionary:
	var unit := unit_with_id(target)
	return {"type": "skill", "actor": ARIA_ID, "x": int(unit.x), "y": int(unit.y), "skill": skill}

func find_last_event(type: String, actor := 0) -> Dictionary:
	for index in range(battle.ui.presented_log_events.size() - 1, -1, -1):
		var event: Dictionary = battle.ui.presented_log_events[index]
		if event.type == type and (actor == 0 or event.get("actor") == actor):
			return event
	return {}

func find_last_event_index(type: String, actor := 0) -> int:
	for index in range(battle.ui.presented_log_events.size() - 1, -1, -1):
		var event: Dictionary = battle.ui.presented_log_events[index]
		if event.type == type and (actor == 0 or event.get("actor") == actor):
			return index
	return -1

func terrain_at(cell: Vector2i) -> Dictionary:
	for terrain in battle.state.terrain_cells:
		if terrain.x == cell.x and terrain.y == cell.y:
			return terrain
	return {}

func unit_with_id(id: int) -> Dictionary:
	for unit in battle.state.units:
		if unit.id == id:
			return unit
	return {}
