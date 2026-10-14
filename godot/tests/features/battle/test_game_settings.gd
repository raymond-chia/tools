extends "res://tests/features/battle/battle_test_suite.gd"

const TEST_DEFINITIONS := "res://tests/features/battle/data/battle_turn_sequence_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/battle_turn_sequence_map.toml"

# 驗證首次讀取建立統一設定，三項偏好都有明確預設值。
func test_defaults_are_saved() -> void:
	assert_str(GameSettings.load_locale()).is_equal("zh_TW")
	assert_str(GameSettings.load_difficulty()).is_equal("normal")
	assert_int(GameSettings.load_playback_speed()).is_equal(1)
	var settings := ConfigFile.new()
	assert_int(settings.load(GameSettings.PATH)).is_equal(OK)
	assert_str(settings.get_value("language", "locale")).is_equal("zh_TW")
	assert_str(settings.get_value("game", "difficulty")).is_equal("normal")
	assert_int(settings.get_value("presentation", "playback_speed")).is_equal(1)

# 驗證真實子選單與速度按鈕保存三項偏好，保留其他項目並於重建場景讀回。
func test_ui_preserves_and_reloads_preferences() -> void:
	var battle = auto_free(create_battle(TEST_DEFINITIONS, TEST_MAP))
	var runner := scene_runner(battle)
	await runner.simulate_frames(1)
	for test_case in [
		{"locale_id": 1, "locale": "en", "difficulty_id": 2, "difficulty": "hard", "speed": 2},
		{"locale_id": 0, "locale": "zh_TW", "difficulty_id": 0, "difficulty": "easy", "speed": 4},
		{"locale_id": 1, "locale": "en", "difficulty_id": 1, "difficulty": "normal", "speed": 1},
	]:
		battle.ui.menu_button.get_popup().get_item_submenu_node(0).id_pressed.emit(test_case.locale_id)
		battle.ui.menu_button.get_popup().get_item_submenu_node(1).id_pressed.emit(test_case.difficulty_id)
		battle.ui.playback_speed_button.pressed.emit()
		var settings := ConfigFile.new()
		assert_int(settings.load(GameSettings.PATH)).is_equal(OK)
		assert_str(settings.get_value("language", "locale")).is_equal(test_case.locale)
		assert_str(settings.get_value("game", "difficulty")).is_equal(test_case.difficulty)
		assert_int(settings.get_value("presentation", "playback_speed")).is_equal(test_case.speed)
		var next_battle = auto_free(create_battle(TEST_DEFINITIONS, TEST_MAP))
		var next_runner := scene_runner(next_battle)
		await next_runner.simulate_frames(1)
		assert_str(TranslationServer.get_locale()).is_equal(test_case.locale)
		assert_str(next_battle.ui.playback_speed_button.text).is_equal("×%d" % test_case.speed)
		assert_float(next_battle.world.playback_speed).is_equal(float(test_case.speed))

# 驗證設定輸入邊界的無效偏好使用各自預設值。
func test_invalid_preferences_use_defaults() -> void:
	var settings := ConfigFile.new()
	settings.set_value("language", "locale", "unknown")
	settings.set_value("game", "difficulty", 42)
	settings.set_value("presentation", "playback_speed", 3)
	assert_int(settings.save(GameSettings.PATH)).is_equal(OK)
	assert_str(GameSettings.load_locale()).is_equal("zh_TW")
	assert_str(GameSettings.load_difficulty()).is_equal("normal")
	assert_int(GameSettings.load_playback_speed()).is_equal(1)
