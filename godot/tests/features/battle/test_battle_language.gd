extends GdUnitTestSuite

const BattleTestSetup := preload("res://tests/features/battle/test_setup.gd")
const TEST_DEFINITIONS := "res://tests/features/battle/data/combat_presentation_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/combat_presentation_spikes_map.toml"
const LANGUAGE_SETTINGS := "user://language.cfg"

var battle
var runner: GdUnitSceneRunner
var original_locale: String
var had_language_settings := false
var original_settings := PackedByteArray()
var test_translations: Array[Translation] = []

func before_test() -> void:
	original_locale = TranslationServer.get_locale()
	had_language_settings = FileAccess.file_exists(LANGUAGE_SETTINGS)
	if had_language_settings:
		original_settings = FileAccess.get_file_as_bytes(LANGUAGE_SETTINGS)
	for test_case in [
		{"locale": "zh_TW", "title": "演出測試戰鬥", "actor": "測試戰士", "skill": "測試推擊"},
		{"locale": "en", "title": "Presentation Test", "actor": "Test Fighter", "skill": "Test Push"},
	]:
		var translation := Translation.new()
		translation.locale = test_case.locale
		translation.add_message("combat_presentation", test_case.title)
		translation.add_message(BattleConfig.unit_name_key("presentation_actor"), test_case.actor)
		translation.add_message(BattleConfig.skill_name_key("presentation_push"), test_case.skill)
		TranslationServer.add_translation(translation)
		test_translations.append(translation)
	runner = scene_runner(auto_free(BattleTestSetup.create_battle(TEST_DEFINITIONS, TEST_MAP, BattleTestSetup.PRESENTATION_RANDOM_SEED)))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()

func after_test() -> void:
	# 保留既有設定的原始位元組，避免測試改寫使用者偏好或遺失設定註解。
	if had_language_settings:
		var file := FileAccess.open(LANGUAGE_SETTINGS, FileAccess.WRITE)
		assert_object(file).is_not_null()
		if file != null:
			file.store_buffer(original_settings)
			file.close()
	else:
		if FileAccess.file_exists(LANGUAGE_SETTINGS):
			assert_int(DirAccess.remove_absolute(ProjectSettings.globalize_path(LANGUAGE_SETTINGS))).is_equal(OK)
	for translation in test_translations:
		TranslationServer.remove_translation(translation)
	test_translations.clear()
	TranslationServer.set_locale(original_locale)

# 驗證選單切換中英文同步更新標題、日誌、技能與單位查看，保留展開狀態並關閉舊預覽。
func test_language_menu_refreshes_existing_content() -> void:
	# 先用真實攻擊產生可翻譯的戰鬥日誌，再回到下一輪玩家。
	assert_bool(battle.send({"type": "skill", "actor": 1, "x": 2, "y": 1, "skill": "presentation_push"})).is_true()
	await BattleTestSetup.wait_until_idle(battle, runner)
	assert_bool(battle.send({"type": "end_turn", "actor": 2})).is_true()
	await BattleTestSetup.wait_until_idle(battle, runner)
	battle.ui.battle_log.meta_clicked.emit("log_entry:0")
	var expansion: Dictionary = battle.ui.log_entry_expanded_states.duplicate(true)
	var events: Array = battle.ui.presented_log_events.duplicate(true)
	battle.world.inspection_clicked.emit(1, Vector2i(0, 1))
	var popup: PopupMenu = battle.ui.menu_button.get_popup()
	var cases := [
		{"id": 1, "locale": "en", "title": "Presentation Test", "actor": "Test Fighter", "skill": "Test Push"},
		{"id": 0, "locale": "zh_TW", "title": "演出測試戰鬥", "actor": "測試戰士", "skill": "測試推擊"},
	]
	for test_case in cases:
		battle.ui.present_move_cost(1, Vector2.ZERO)
		var preview: Dictionary = battle.read_core_response(battle.core.preview_skill(1, 3, 1, "presentation_push"))
		battle.ui.present_attack_preview(preview, Vector2.ZERO)
		assert_bool(battle.ui.move_cost_popup.visible).is_true()
		assert_bool(battle.ui.attack_preview_panel.visible).is_true()
		popup.id_pressed.emit(test_case.id)
		assert_str(TranslationServer.get_locale()).is_equal(test_case.locale)
		assert_bool(popup.is_item_checked(test_case.id)).is_true()
		assert_bool(popup.is_item_checked(1 - test_case.id)).is_false()
		assert_str(battle.ui.get_node("Root/BattleTitle").text).is_equal(test_case.title)
		assert_str(battle.ui.actor_name.text).is_equal(test_case.actor)
		assert_str(battle.ui.unit_name.text).is_equal(test_case.actor)
		assert_str(battle.ui.battle_log.text).contains(test_case.actor)
		assert_str(battle.ui.battle_log.text).contains(test_case.skill)
		assert_dict(battle.ui.log_entry_expanded_states).is_equal(expansion)
		assert_array(battle.ui.presented_log_events).is_equal(events)
		assert_bool(battle.ui.move_cost_popup.visible).is_false()
		assert_bool(battle.ui.attack_preview_panel.visible).is_false()
		battle.ui.action_buttons.presentation_push.mouse_entered.emit()
		assert_str(battle.ui.hovered_skill_title.text).is_equal(test_case.skill)
		battle.ui.skill_inspection_requested.emit("presentation_push")
		assert_str(battle.ui.inspected_skill_name.text).is_equal(test_case.skill)
		battle.ui.inspection_closed.emit()
		battle.world.inspection_clicked.emit(1, Vector2i(0, 1))

# 驗證選單選擇寫入語言設定，重新建立真實戰鬥場景時讀回語系、勾選與標題。
func test_language_preference_is_loaded_by_next_battle() -> void:
	for test_case in [{"id": 1, "locale": "en", "title": "Presentation Test"}, {"id": 0, "locale": "zh_TW", "title": "演出測試戰鬥"}]:
		battle.ui.menu_button.get_popup().id_pressed.emit(test_case.id)
		var settings := ConfigFile.new()
		assert_int(settings.load(LANGUAGE_SETTINGS)).is_equal(OK)
		assert_str(settings.get_value("language", "locale")).is_equal(test_case.locale)
		TranslationServer.set_locale("zh_TW" if test_case.id == 1 else "en")
		var next_battle = auto_free(BattleTestSetup.create_battle(TEST_DEFINITIONS, TEST_MAP, BattleTestSetup.PRESENTATION_RANDOM_SEED))
		var next_runner := scene_runner(next_battle)
		await next_runner.simulate_frames(1)
		assert_str(TranslationServer.get_locale()).is_equal(test_case.locale)
		assert_bool(next_battle.ui.menu_button.get_popup().is_item_checked(test_case.id)).is_true()
		assert_str(next_battle.ui.get_node("Root/BattleTitle").text).is_equal(test_case.title)
