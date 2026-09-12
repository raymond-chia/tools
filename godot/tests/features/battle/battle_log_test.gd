extends GdUnitTestSuite

const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITION := "res://tests/features/battle/data/battle_log.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(BATTLE_SCENE)
	await runner.simulate_frames(1)
	battle = runner.scene()
	load_test_definition()

# 驗證開始戰鬥會記錄新回合，並以整數顯示回合數。
func test_new_round_log() -> void:
	var event := find_last_event("new_round")
	assert_dict(event).override_failure_message("應產生新回合事件").is_not_empty()
	assert_int(int(event.round)).override_failure_message("新回合事件應記錄目前輪數").is_equal(1)
	assert_str(battle.ui.battle_log.text).override_failure_message("戰鬥紀錄應顯示整數回合數").contains("── 第 1 輪 ──")

# 驗證新回合事件提供已排序的先攻擲骰明細，且總值等於擲骰與加值之和。
func test_new_round_log_contains_initiative_rolls() -> void:
	var event := find_last_event("new_round")
	assert_array(event.initiative_rolls).override_failure_message("新回合事件應提供先攻擲骰明細").is_not_empty()
	assert_int(event.initiative_rolls.size()).is_equal(1)
	var initiative_roll: Dictionary = event.initiative_rolls[0]
	assert_str(initiative_roll.unit).is_equal("測試劍士")
	assert_str(initiative_roll.team).is_equal("player")
	assert_int(int(initiative_roll.modifier)).is_equal(100)
	assert_int(int(initiative_roll.total)).is_equal(int(initiative_roll.roll) + 100)

# 驗證每筆紀錄依事件類型決定預設展開狀態。
func test_log_entries_use_event_default_expansion() -> void:
	assert_bool(battle.ui.log_entry_expanded_states[0]).override_failure_message("新回合與先攻紀錄應預設摺疊").is_false()
	assert_bool(battle.send(skill_command("wolf_a", "precise_strike"))).override_failure_message("測試技能應成功施放").is_true()
	var skill_index := find_last_event_index("skill")
	assert_bool(battle.ui.log_entry_expanded_states[skill_index]).override_failure_message("技能紀錄應預設展開").is_true()

# 驗證切換一筆紀錄只改變該筆狀態，且新增紀錄後保留既有狀態。
func test_log_entries_preserve_independent_expansion_states() -> void:
	battle.ui.toggle_log_entry(0)
	assert_bool(battle.ui.log_entry_expanded_states[0]).override_failure_message("新回合紀錄應可獨立展開").is_true()
	assert_str(battle.ui.battle_log.text).override_failure_message("展開新回合紀錄應顯示先攻明細").contains("先攻總值")
	assert_bool(battle.send(skill_command("wolf_a", "precise_strike"))).override_failure_message("測試技能應成功施放").is_true()
	var skill_index := find_last_event_index("skill")
	assert_bool(battle.ui.log_entry_expanded_states[0]).override_failure_message("新增紀錄後應保留既有展開狀態").is_true()
	battle.ui.toggle_log_entry(skill_index)
	assert_bool(battle.ui.log_entry_expanded_states[0]).override_failure_message("切換技能紀錄不應影響新回合紀錄").is_true()
	assert_bool(battle.ui.log_entry_expanded_states[skill_index]).override_failure_message("技能紀錄應可獨立摺疊").is_false()
	assert_str(battle.ui.battle_log.text).override_failure_message("摺疊技能紀錄應隱藏攻擊判定明細").not_contains("攻擊加值 104")

# 驗證技能紀錄包含判定雙方數值、結果、暴擊、傷害與剩餘生命，且 UI 以整數顯示其意義。
func test_skill_resolution_log() -> void:
	assert_bool(battle.send(skill_command("wolf_a", "precise_strike"))).override_failure_message("測試技能應成功施放").is_true()
	var event := find_last_event("skill")
	assert_dict(event).override_failure_message("應產生技能事件").is_not_empty()
	assert_str(event.actor).is_equal("測試劍士")
	assert_str(event.actor_team).is_equal("player")
	assert_str(event.skill).is_equal("精準斬擊")
	assert_str(event.target).is_equal("測試木樁")
	assert_str(event.target_team).is_equal("enemy")
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
	var log_text: String = battle.ui.battle_log.text
	assert_str(log_text).contains("[color=#63a9ff]測試劍士[/color] 使用「精準斬擊」影響 [color=#ff6868]測試木樁[/color]")
	assert_str(log_text).contains("D20 擲骰 %d + 攻擊加值 104 = 攻擊總值 %d" % [int(event.roll), int(event.attack_total)])
	assert_str(log_text).contains("目標防禦：閃避門檻 12／格擋門檻 15")
	assert_str(log_text).contains("結果：%s%s，%d 傷害，HP %d/10000" % [result_names[event.result], critical_text, expected_damage, 10000 - expected_damage])

# 驗證技能使生命歸零時會記錄倒下狀態與對應顯示文字。
func test_downed_unit_log() -> void:
	assert_bool(battle.send(skill_command("wolf_b", "finishing_strike"))).override_failure_message("終結技能應成功施放").is_true()
	var event := find_last_event("skill")
	assert_bool(event.downed).override_failure_message("生命歸零的目標應標記為倒下").is_true()
	assert_int(int(event.remaining_hp)).override_failure_message("倒下目標的剩餘生命應為零").is_zero()
	assert_str(battle.ui.battle_log.text).override_failure_message("戰鬥紀錄應以敵方顏色顯示倒下單位").contains("[color=#ff6868]脆弱木樁[/color] 倒下")

# 驗證踩到地刺會停止移動、扣除固定傷害，並記錄完整的地形傷害結果。
func test_spikes_damage_log() -> void:
	var terrain := terrain_at(Vector2i(0, 2))
	assert_str(terrain.kind).override_failure_message("測試地格應標記為地刺").is_equal("spikes")
	assert_int(int(terrain.damage)).override_failure_message("地刺資訊應由核心提供固定傷害").is_equal(3)
	assert_bool(battle.send({"type": "move", "actor": "aria", "x": 0, "y": 2})).override_failure_message("移動到地刺地格應成功").is_true()
	var event := find_last_event("terrain_damage")
	assert_dict(event).override_failure_message("應產生地形傷害事件").is_not_empty()
	assert_str(event.target).is_equal("測試劍士")
	assert_str(event.target_team).is_equal("player")
	assert_str(event.terrain).is_equal("spikes")
	assert_int(int(event.damage)).is_equal(3)
	assert_int(int(event.remaining_hp)).is_equal(47)
	assert_int(int(event.max_hp)).is_equal(50)
	assert_bool(event.downed).is_false()
	var actor := unit_with_id("aria")
	assert_int(int(actor.hp)).override_failure_message("踩到地刺後 snapshot 應反映剩餘生命").is_equal(47)
	assert_int(int(actor.x)).is_equal(0)
	assert_int(int(actor.y)).override_failure_message("角色應停在觸發地刺的格子").is_equal(2)
	assert_str(battle.ui.battle_log.text).override_failure_message("戰鬥紀錄應顯示地刺傷害與剩餘生命").contains("[color=#63a9ff]測試劍士[/color] 踩到「地刺」，受到 3 點傷害，HP 47/50")

func load_test_definition() -> void:
	for child in battle.world.units_layer.get_children():
		child.free()
	battle.world.unit_nodes.clear()
	var definition := FileAccess.get_file_as_string(TEST_DEFINITION)
	var loaded = JSON.parse_string(battle.core.load_definition(definition))
	assert_bool(loaded.has("error")).override_failure_message("專用 TOML 應成功載入").is_false()
	if loaded.has("error"):
		return
	battle.state = loaded
	battle.world.setup_map(loaded)
	assert_bool(battle.send({"type": "start"})).override_failure_message("專用測試戰鬥應成功開始").is_true()

func skill_command(target: String, skill: String) -> Dictionary:
	return {"type": "skill", "actor": "aria", "target": target, "skill": skill}

func find_last_event(type: String) -> Dictionary:
	for index in range(battle.state.log.size() - 1, -1, -1):
		var event: Dictionary = battle.state.log[index]
		if event.type == type:
			return event
	return {}

func find_last_event_index(type: String) -> int:
	for index in range(battle.state.log.size() - 1, -1, -1):
		if battle.state.log[index].type == type:
			return index
	return -1

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
