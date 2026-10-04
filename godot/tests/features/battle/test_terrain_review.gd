extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/test_setup.gd")
const DATA := "res://tests/features/battle/data/terrain_review_"
const DEFINITIONS := DATA + "definitions.toml"
const ACTOR_ID := 1
const TARGET_ID := 2

var battle
var runner: GdUnitSceneRunner
var original_locale: String

func before_test() -> void:
	original_locale = TranslationServer.get_locale()
	TranslationServer.set_locale("zh_TW")
	runner = scene_runner(auto_free(BattleTestSetup.create_battle(DEFINITIONS, DATA + "move_anchor_map.toml")))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	await BattleTestSetup.wait_until_idle(battle, runner)

func after_test() -> void:
	TranslationServer.set_locale(original_locale)

# 驗證大型單位進入地形時，左上角與其他佔用格均應受到相同傷害及防禦懲罰。
func test_large_unit_terrain_damage_uses_occupied_cells() -> void:
	for location in ["anchor", "offset"]:
		load_case("move_" + location)
		var snapshot := dispatch({"type": "move", "actor": ACTOR_ID, "x": 1, "y": 0})
		var actor := unit_with_id(snapshot, ACTOR_ID)
		assert_int(int(actor.dodge)).override_failure_message(location + "：佔用格地形應降低閃避 1").is_equal(9)
		assert_int(int(actor.block)).override_failure_message(location + "：佔用格地形應降低格擋 4").is_equal(6)
		assert_int(int(actor.hp)).override_failure_message(location + "：佔用格地形應造成 3 點傷害").is_equal(97)
		var damage_events: Array = snapshot.log.filter(func(event): return event.type == "terrain_damage")
		assert_int(damage_events.size()).override_failure_message(location + "：應產生一筆地形傷害事件").is_equal(1)

# 驗證大型單位移動一格時，任一佔用格的額外地形成本均應計入預覽及剩餘額度。
func test_large_unit_terrain_cost_uses_occupied_cells() -> void:
	for location in ["anchor", "offset"]:
		load_case("move_" + location)
		var preview := decode(battle.core.preview_move(ACTOR_ID, 1, 0))
		assert_int(int(preview.total_cost)).override_failure_message(location + "：一格基本成本 1 加地形成本 2 應為 3").is_equal(3)
		var snapshot := dispatch({"type": "move", "actor": ACTOR_ID, "x": 1, "y": 0})
		assert_int(int(snapshot.turn.move_remaining)).override_failure_message(location + "：實際移動應扣除同樣的 3 點成本").is_equal(17)

# 驗證大型單位任一佔用格進入傷害地形時，預覽與實際移動均停止於該步。
func test_large_unit_damaging_terrain_stops_movement() -> void:
	for location in ["anchor", "offset"]:
		load_case("move_" + location)
		var preview := decode(battle.core.preview_move(ACTOR_ID, 3, 0))
		assert_bool(preview.interrupted).override_failure_message(location + "：佔用格進入傷害地形應標示中斷").is_true()
		assert_int(int(preview.first.back().x)).override_failure_message(location + "：預覽應停在第一步").is_equal(1)
		var snapshot := dispatch({"type": "move", "actor": ACTOR_ID, "x": 3, "y": 0})
		assert_int(int(unit_with_id(snapshot, ACTOR_ID).x)).override_failure_message(location + "：實際位置應停在第一步").is_equal(1)

# 驗證推動大型單位後，任一佔用格的傷害地形或懸崖均應觸發相應結算。
func test_large_unit_pushed_terrain_uses_occupied_cells() -> void:
	var cases := [
		{"map": "push_mire_anchor", "hp": 97, "downed": false},
		{"map": "push_mire_offset", "hp": 97, "downed": false},
		{"map": "push_cliff_anchor", "hp": 0, "downed": true},
		{"map": "push_cliff_offset", "hp": 0, "downed": true},
	]
	for test_case in cases:
		load_case(test_case.map)
		BattleTestSetup.reset_random_seed(battle.core)
		var snapshot := dispatch({"type": "skill", "actor": ACTOR_ID, "x": 2, "y": 0, "skill": "shield_bash"})
		var skill_events: Array = snapshot.log.filter(func(event): return event.type == "skill")
		assert_int(skill_events.size()).is_equal(1)
		assert_str(skill_events[0].result).override_failure_message(test_case.map + "：固定種子應成功命中").is_equal("hit")
		assert_bool(skill_events[0].pushed).override_failure_message(test_case.map + "：地形應允許推入").is_true()
		var target := unit_with_id(snapshot, TARGET_ID)
		assert_bool(target.is_empty()).override_failure_message(test_case.map + "：懸崖應使大型目標倒下").is_equal(test_case.downed)
		if not target.is_empty():
			assert_int(int(target.x)).is_equal(3)
			assert_int(int(target.hp)).override_failure_message(test_case.map + "：推入地形後 HP 應為 %d" % test_case.hp).is_equal(test_case.hp)
		var damage_events: Array = snapshot.log.filter(func(event): return event.type == "terrain_damage")
		assert_int(damage_events.size()).override_failure_message(test_case.map + "：推入地形應產生一筆傷害事件").is_equal(1)
		if not damage_events.is_empty():
			assert_bool(damage_events[0].instant_down).is_equal(test_case.downed)
			assert_int(int(damage_events[0].remaining_hp)).is_equal(test_case.hp)

# 驗證固定及暫時地形快照與真實詳情面板均保留不同防禦懲罰、成本與傷害，暫時地形另顯示期限。
func test_terrain_description_preserves_each_effect() -> void:
	for temporary in [false, true]:
		var snapshot := load_case("move_anchor")
		var cell := Vector2i(1, 0)
		if temporary:
			cell = Vector2i(4, 0)
			snapshot = dispatch({"type": "skill", "actor": ACTOR_ID, "x": cell.x, "y": cell.y, "skill": "mire"})
		var terrain: Dictionary = snapshot.terrain_cells.filter(func(item): return item.x == cell.x and item.y == cell.y)[0]
		var description: Dictionary = terrain.effect_descriptions.filter(func(item): return item.terrain == "mire")[0]
		var expected := {"dodge_penalty": 1.0, "block_penalty": 4.0, "extra_movement_cost": 2.0, "damage": 3.0}
		if temporary:
			expected.remaining_rounds = 3.0
		assert_dict(description.values).override_failure_message("暫時=%s：快照應保留所有實際效果且不合併防禦懲罰" % temporary).is_equal(expected)
		battle.ui.present(snapshot, "", cell, "", "", false)
		assert_bool(battle.ui.info_panel.visible).is_true()
		# 本案例驗證效果數值完整性；沿用 JSON 數字的既有文字格式。
		var expected_texts := [
			tr("TERRAIN_EFFECT_DODGE").format({"value": 1.0}),
			tr("TERRAIN_EFFECT_BLOCK").format({"value": 4.0}),
			tr("TERRAIN_EFFECT_MOVEMENT").format({"value": 2.0}),
			tr("TERRAIN_EFFECT_DAMAGE").format({"value": 3.0}),
		]
		for text in expected_texts:
			assert_str(battle.ui.terrain_effect.text).override_failure_message("暫時=%s：地形詳情應顯示「%s」" % [temporary, text]).contains(text)
		if temporary:
			assert_str(battle.ui.terrain_effect.text).contains(tr("TERRAIN_EFFECT_DURATION").format({"value": 3.0}))
		else:
			assert_str(battle.ui.terrain_effect.text).not_contains("回合")

func load_case(map_name: String) -> Dictionary:
	decode(battle.core.load_documents(FileAccess.get_file_as_string(DEFINITIONS), FileAccess.get_file_as_string(DATA + map_name + "_map.toml")))
	BattleTestSetup.reset_random_seed(battle.core)
	return dispatch({"type": "start"})

func dispatch(command: Dictionary) -> Dictionary:
	return decode(battle.core.dispatch(JSON.stringify(command)))

func decode(response: String) -> Dictionary:
	var value: Dictionary = JSON.parse_string(response)
	assert_bool(value.has("error_id")).override_failure_message("測試準備或操作應成功：" + response).is_false()
	return CoreResponse.read(response, battle.show_error)

func unit_with_id(snapshot: Dictionary, id: int) -> Dictionary:
	for unit in snapshot.units:
		if unit.id == id:
			return unit
	return {}
