extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/test_setup.gd")
const TEST_DEFINITIONS := "res://tests/features/battle/data/equipment_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/equipment_map.toml"
const ACTOR_ID := 1

var battle
var runner: GdUnitSceneRunner
var original_locale: String

func before_test() -> void:
	original_locale = TranslationServer.get_locale()
	runner = scene_runner(auto_free(BattleTestSetup.create_battle(TEST_DEFINITIONS, TEST_MAP)))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()
	await BattleTestSetup.wait_until_idle(battle, runner)
	TranslationServer.set_locale("zh_TW")

func after_test() -> void:
	TranslationServer.set_locale(original_locale)

# 驗證真實戰鬥面板顯示核心加總後的血量、雙威力、格擋減傷與四個配裝欄位。
func test_unit_details_show_equipment_and_resolved_stats() -> void:
	var cases := [
		# 劍盾重甲與戒指的各項能力加總後顯示在戰士詳情。
		{"id": 1, "cell": Vector2i(1, 1), "hp": "16 / 16", "power": "5 / 3", "defense": "2 / 5 / 3", "equipment": ["單手劍", "盾牌", "重甲", "魔力戒指"]},
		# 輕甲法師保留單位閃避，但不具有格擋或減傷。
		{"id": 2, "cell": Vector2i(0, 2), "hp": "14 / 14", "power": "1 / 11", "defense": "2 / 0 / 0", "equipment": ["法杖", "無", "輕甲", "魔力戒指"]},
		# 無裝備單位明確顯示四個空欄位，格擋與減傷為零。
		{"id": 3, "cell": Vector2i(2, 1), "hp": "50 / 50", "power": "0 / 0", "defense": "2 / 0 / 0", "equipment": ["無", "無", "無", "無"]},
	]
	for test_case in cases:
		battle.world.inspection_clicked.emit(test_case.id, test_case.cell)
		await runner.simulate_frames(1)
		assert_bool(battle.ui.info_panel.visible).is_true()
		assert_str(battle.ui.detail_values.hp.text).is_equal(test_case.hp)
		assert_str(battle.ui.detail_values.power.text).is_equal(test_case.power)
		assert_str(battle.ui.detail_values.defense.text).is_equal(test_case.defense)
		for index in 4:
			var slot: String = ["main_hand", "off_hand", "armor", "accessory"][index]
			assert_str(battle.ui.detail_values[slot].text).override_failure_message("單位 %d：%s 配裝應正確顯示" % [test_case.id, slot]).is_equal(test_case.equipment[index])

# 驗證裝備名稱與雙威力／防禦欄位標題隨語系切換，裝備 ID 由顯示層翻譯。
func test_equipment_details_use_current_locale() -> void:
	battle.world.inspection_clicked.emit(ACTOR_ID, Vector2i(1, 1))
	var cases := [
		# 繁體中文顯示裝備名稱與防禦欄位意義。
		{"locale": "zh_TW", "sword": "單手劍", "shield": "盾牌", "armor": "重甲", "ring": "魔力戒指", "power": "物理／魔法威力", "defense": "閃避／格擋／減傷"},
		# 英文顯示同一配裝與欄位語意。
		{"locale": "en", "sword": "Sword", "shield": "Shield", "armor": "Heavy armor", "ring": "Magic ring", "power": "Physical / Magical power", "defense": "Dodge / Block / Reduction"},
	]
	for test_case in cases:
		TranslationServer.set_locale(test_case.locale)
		battle.present()
		assert_str(battle.ui.detail_values.main_hand.text).is_equal(test_case.sword)
		assert_str(battle.ui.detail_values.off_hand.text).is_equal(test_case.shield)
		assert_str(battle.ui.detail_values.armor.text).is_equal(test_case.armor)
		assert_str(battle.ui.detail_values.accessory.text).is_equal(test_case.ring)
		var rows: GridContainer = battle.ui.unit_details.get_node("Rows")
		assert_str(tr(rows.get_node("PowerLabel").text)).is_equal(test_case.power)
		assert_str(tr(rows.get_node("DefenseLabel").text)).is_equal(test_case.defense)

# 驗證近距離物理與魔法技能呈現各自核心傷害，無裝備目標不顯示格擋結果或減傷血條。
func test_melee_and_magic_previews_use_core_equipment_stats() -> void:
	assert_int(battle.state.turn.actor).is_equal(ACTOR_ID)
	var cases := [
		# 普通近戰使用物理威力與單手劍加值。
		{"skill": "melee", "damage": 5, "source": "威力來源：物理"},
		# 相同距離的法術使用魔法威力與戒指加值。
		{"skill": "spell", "damage": 3, "source": "威力來源：魔法"},
	]
	for test_case in cases:
		battle.world.hovered = Vector2i(2, 1)
		battle.select_action(test_case.skill)
		battle.world.update_attack_preview()
		var preview: Dictionary = battle.world.attack_preview
		assert_dict(preview).is_not_empty()
		assert_int(int(preview.hit_damage)).is_equal(test_case.damage)
		assert_int(int(preview.block_chance)).is_zero()
		assert_int(int(preview.block_damage)).is_equal(test_case.damage)
		assert_str(battle.ui.attack_preview_damage.text).is_equal("傷害 %d" % test_case.damage)
		assert_str(battle.ui.attack_preview_result_labels.block.text).is_equal("格擋 0%")
		assert_bool(battle.ui.attack_preview_health_segments.block.visible).is_false()
		battle.ui.action_buttons[test_case.skill].mouse_entered.emit()
		assert_str(battle.ui.hovered_skill_details.text).contains(test_case.source)
