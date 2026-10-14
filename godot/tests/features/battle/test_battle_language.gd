extends "res://tests/features/battle/battle_test_suite.gd"

const TEST_DEFINITIONS := "res://tests/features/battle/data/combat_presentation_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/combat_presentation_spikes_map.toml"

var battle
var runner: GdUnitSceneRunner
var test_translations: Array[Translation] = []

# 準備本案例的真實戰鬥場景與專用資源。
func setup_test() -> void:
	for test_case in [
		{"locale": "zh_TW", "actor": "測試戰士", "skill": "測試推擊"},
		{"locale": "en", "actor": "Test Fighter", "skill": "Test Push"},
	]:
		var translation := Translation.new()
		translation.locale = test_case.locale
		translation.add_message(BattleConfig.unit_name_key("presentation_actor"), test_case.actor)
		translation.add_message(BattleConfig.skill_name_key("presentation_push"), test_case.skill)
		TranslationServer.add_translation(translation)
		test_translations.append(translation)
	runner = scene_runner(auto_free(create_battle(TEST_DEFINITIONS, TEST_MAP, PRESENTATION_RANDOM_SEED)))
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()

# 清理本案例的專用資源與尚未完成的操作。
func cleanup_test() -> void:
	for translation in test_translations:
		TranslationServer.remove_translation(translation)
	test_translations.clear()

# 驗證語言子選單切換更新兩列選單文字、日誌與查看內容，保留展開狀態並關閉預覽。
func test_language_menu_refreshes_existing_content() -> void:
	# 先用真實攻擊產生可翻譯的戰鬥日誌，再回到下一輪玩家。
	assert_bool(battle.send({"type": "skill", "actor": 1, "x": 2, "y": 1, "skill": "presentation_push"})).is_true()
	await wait_until_idle(battle, runner)
	assert_bool(battle.send({"type": "end_turn", "actor": 2})).is_true()
	await wait_until_idle(battle, runner)
	battle.ui.battle_log.meta_clicked.emit("log_entry:0")
	var expansion: Dictionary = battle.ui.log_entry_expanded_states.duplicate(true)
	var events: Array = battle.ui.presented_log_events.duplicate(true)
	battle.world.inspection_clicked.emit(1, Vector2i(0, 1))
	var menu: PopupMenu = battle.ui.menu_button.get_popup()
	var popup: PopupMenu = menu.get_item_submenu_node(0)
	var cases := [
		{"id": 1, "locale": "en", "menu": "Language: English", "difficulty_menu": "Difficulty: Normal", "actor": "Test Fighter", "skill": "Test Push"},
		{"id": 0, "locale": "zh_TW", "menu": "語言：繁體中文", "difficulty_menu": "難度：普通", "actor": "測試戰士", "skill": "測試推擊"},
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
		assert_str(menu.get_item_text(0)).is_equal(test_case.menu)
		assert_str(menu.get_item_text(1)).is_equal(test_case.difficulty_menu)
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

# 驗證語言子選單寫入設定，重建真實戰鬥場景時讀回語系與單選標記。
func test_language_preference_is_loaded_by_next_battle() -> void:
	for test_case in [{"id": 1, "locale": "en"}, {"id": 0, "locale": "zh_TW"}]:
		battle.ui.menu_button.get_popup().get_item_submenu_node(0).id_pressed.emit(test_case.id)
		var settings := ConfigFile.new()
		assert_int(settings.load(GameSettings.PATH)).is_equal(OK)
		assert_str(settings.get_value("language", "locale")).is_equal(test_case.locale)
		TranslationServer.set_locale("zh_TW" if test_case.id == 1 else "en")
		var next_battle = auto_free(create_battle(TEST_DEFINITIONS, TEST_MAP, PRESENTATION_RANDOM_SEED))
		var next_runner := scene_runner(next_battle)
		await next_runner.simulate_frames(1)
		assert_str(TranslationServer.get_locale()).is_equal(test_case.locale)
		assert_bool(next_battle.ui.menu_button.get_popup().get_item_submenu_node(0).is_item_checked(test_case.id)).is_true()
