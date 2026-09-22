extends GdUnitTestSuite

const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITION := "res://tests/features/battle/data/battle_preview.toml"

var battle
var runner: GdUnitSceneRunner
var original_locale := ""
var test_translations: Array[Translation] = []

func before_test() -> void:
	original_locale = TranslationServer.get_locale()
	add_test_translation("zh_TW", {
		"SKILL_POWER_STRIKE_NAME": "強力一擊",
		"SKILL_TARGET_ENEMY": "目標：敵方單位",
		"SKILL_TYPE_MELEE": "類型：近戰",
		"SKILL_RANGE": "射程：%d 格",
		"SKILL_ATTACK_BONUS": "攻擊加值：%+d",
		"SKILL_DAMAGE_BONUS": "傷害加值：%+d",
	})
	add_test_translation("en", {
		"SKILL_POWER_STRIKE_NAME": "Power Strike",
		"SKILL_TARGET_ENEMY": "Target: Enemy unit",
		"SKILL_TYPE_MELEE": "Type: Melee",
		"SKILL_RANGE": "Range: %d tiles",
		"SKILL_ATTACK_BONUS": "Attack modifier: %+d",
		"SKILL_DAMAGE_BONUS": "Damage modifier: %+d",
	})
	runner = scene_runner(BATTLE_SCENE)
	await runner.simulate_frames(1)
	battle = runner.scene()
	await wait_for_combat_events()
	load_test_definition()
	await wait_for_combat_events()

func after_test() -> void:
	TranslationServer.set_locale(original_locale)
	for translation in test_translations:
		TranslationServer.remove_translation(translation)
	test_translations.clear()

# 驗證技能 hover 內容會隨目前語系翻譯，且標題不包含額外的狀態前綴。
func test_skill_hover_uses_current_locale() -> void:
	var cases := [
		{"locale": "zh_TW", "title": "強力一擊", "details": "目標：敵方單位\n類型：近戰\n射程：1 格\n攻擊加值：+2\n傷害加值：+2"},
		{"locale": "en", "title": "Power Strike", "details": "Target: Enemy unit\nType: Melee\nRange: 1 tiles\nAttack modifier: +2\nDamage modifier: +2"},
	]
	for test_case in cases:
		TranslationServer.set_locale(test_case.locale)
		battle.ui.action_buttons.power_strike.mouse_entered.emit()

		assert_bool(battle.ui.hovered_skill_card.visible).override_failure_message("%s：hover 應顯示技能資訊" % test_case.locale).is_true()
		assert_str(battle.ui.hovered_skill_title.text).override_failure_message("%s：技能名稱應使用目前語系且沒有狀態前綴" % test_case.locale).is_equal(test_case.title)
		assert_str(battle.ui.hovered_skill_details.text).override_failure_message("%s：技能內容應使用目前語系" % test_case.locale).is_equal(test_case.details)

# 驗證右鍵技能會使用既有 inspect panel，且固定格式永遠先顯示 Header 再顯示技能內容。
func test_right_click_skill_uses_ordered_inspect_panel() -> void:
	TranslationServer.set_locale("zh_TW")
	emit_right_press(battle.ui.action_buttons.power_strike)
	await runner.simulate_frames(1)

	var info_panel: Panel = battle.ui.info_panel
	var content: VBoxContainer = info_panel.get_node("Margin/Content")
	var header: Control = content.get_node("Header")
	var skill_details: Control = content.get_node("SkillDetails")
	assert_str(battle.inspected_skill).override_failure_message("右鍵技能應建立技能查看狀態").is_equal("power_strike")
	assert_bool(info_panel.visible).override_failure_message("右鍵技能應開啟既有 inspect panel").is_true()
	assert_int(header.get_index()).override_failure_message("Header 在場景樹中應排在技能內容之前").is_less(skill_details.get_index())
	assert_float(header.global_position.y).override_failure_message("Header 在畫面上應位於技能內容上方").is_less(skill_details.global_position.y)
	var cases := [
		{"locale": "zh_TW", "name": "強力一擊", "range": "射程：1 格"},
		{"locale": "en", "name": "Power Strike", "range": "Range: 1 tiles"},
	]
	for test_case in cases:
		TranslationServer.set_locale(test_case.locale)
		battle.present()
		assert_str(battle.ui.inspected_skill_name.text).override_failure_message("%s：inspect 技能名稱應使用目前語系" % test_case.locale).is_equal(test_case.name)
		assert_str(battle.ui.inspected_skill_description.text).override_failure_message("%s：inspect 技能內容應使用目前語系" % test_case.locale).contains(test_case.range)

func add_test_translation(locale: String, messages: Dictionary) -> void:
	var translation := Translation.new()
	translation.locale = locale
	for message in messages:
		translation.add_message(message, messages[message])
	TranslationServer.add_translation(translation)
	test_translations.append(translation)

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

func wait_for_combat_events() -> void:
	while battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)

func emit_right_press(control: Control) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_RIGHT
	event.pressed = true
	control.gui_input.emit(event)
