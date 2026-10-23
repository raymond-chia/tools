extends "res://tests/features/battle/battle_test_suite.gd"

const TEST_DEFINITIONS := "res://tests/features/battle/data/battle_preview_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/battle_preview_map.toml"

var battle
var runner: GdUnitSceneRunner
var test_translations: Array[Translation] = []

# 準備真實戰鬥場景與專用翻譯，使用命中加值的語意 key。
func setup_test() -> void:
	add_test_translation("zh_TW", {
		"SKILL_POWER_STRIKE_NAME": "強力一擊",
		"SKILL_TARGET_ENEMY": "目標：敵方單位",
		"SKILL_TYPE_MELEE": "類型：近戰",
		"SKILL_RANGE": "射程：%d 格",
		"SKILL_HIT_BONUS": "命中加值：%+d",
		"SKILL_POWER_BONUS": "威力加值：%+d",
		"SKILL_POWER_SOURCE_PHYSICAL": "威力來源：物理",
	})
	add_test_translation("en", {
		"SKILL_POWER_STRIKE_NAME": "Power Strike",
		"SKILL_TARGET_ENEMY": "Target: Enemy unit",
		"SKILL_TYPE_MELEE": "Type: Melee",
		"SKILL_RANGE": "Range: %d tiles",
		"SKILL_HIT_BONUS": "Hit modifier: %+d",
		"SKILL_POWER_BONUS": "Power modifier: %+d",
		"SKILL_POWER_SOURCE_PHYSICAL": "Power source: Physical",
	})
	runner = battle_scene_runner(auto_free(create_battle(TEST_DEFINITIONS, TEST_MAP)))
	await runner.simulate_frames(1)
	battle = runner.scene()
	await wait_until_idle(battle, runner)

# 清理本案例的專用資源與尚未完成的操作。
func cleanup_test() -> void:
	for translation in test_translations:
		TranslationServer.remove_translation(translation)
	test_translations.clear()

# 驗證被動不提供可選主動按鈕，但被動技能分頁顯示技能名稱與翻譯後的夾擊命中加值說明，切換無被動單位後顯示無被動技能。
func test_passive_skill_is_inspectable_but_not_selectable() -> void:
	assert_int(battle.state.turn.actor).is_equal(1)
	assert_array(battle.ui.action_buttons.keys()).contains_exactly(["melee_attack", "ranged_attack", "power_strike", "aimed_shot"])
	var passive_label: Label = battle.ui.passive_skills_text
	var cases := [
		# 繁體中文顯示被動名稱、近戰夾擊條件與 4 點加成。
		{"locale": "zh_TW", "text": "狼群夾擊：被動：近戰夾擊時命中加值 +4"},
		# 英文顯示相同技能與特殊加成。
		{"locale": "en", "text": "Pack flanking：Passive: +4 hit modifier when flanking in melee"},
	]
	for test_case in cases:
		TranslationServer.set_locale(test_case.locale)
		battle.world.inspection_clicked.emit(1, Vector2i(1, 3))
		battle.present()
		battle.ui.detail_tabs.current_tab = battle.ui.DetailTab.PASSIVE_SKILLS
		assert_bool(passive_label.is_visible_in_tree()).is_true()
		assert_str(passive_label.text).is_equal(test_case.text)
		battle.world.inspection_clicked.emit(2, Vector2i(3, 1))
		assert_int(battle.ui.detail_tabs.current_tab).is_equal(battle.ui.DetailTab.PASSIVE_SKILLS)
		assert_bool(passive_label.is_visible_in_tree()).is_true()
		assert_str(passive_label.text).is_equal(tr("DETAIL_NO_PASSIVE_SKILLS"))

# 驗證首次查看屬性、依地形、屬性、主動、被動順序隔離各分頁內容，切換單位更新技能，經過空地後恢復單位分頁。
func test_unit_detail_tabs_show_inspected_unit_skills() -> void:
	runner = await replace_battle(runner, "res://tests/features/battle/data/combat_presentation_definitions.toml", "res://tests/features/battle/data/combat_presentation_spikes_map.toml")
	battle = runner.scene()
	battle.world.inspection_clicked.emit(1, Vector2i(0, 1))
	assert_int(battle.ui.detail_tabs.current_tab).is_equal(battle.ui.DetailTab.ATTRIBUTES)
	assert_bool(battle.ui.unit_details.is_visible_in_tree()).is_true()
	battle.ui.detail_tabs.current_tab = battle.ui.DetailTab.ACTIVE_SKILLS
	assert_bool(battle.ui.unit_details.is_visible_in_tree()).is_false()
	assert_bool(battle.ui.active_skills_text.is_visible_in_tree()).is_true()
	assert_bool(battle.ui.passive_skills_text.is_visible_in_tree()).is_false()
	assert_str(battle.ui.active_skills_text.text).contains(tr(BattleConfig.skill_name_key("presentation_push")))
	battle.world.inspection_clicked.emit(2, Vector2i(0, 3))
	assert_int(battle.ui.detail_tabs.current_tab).is_equal(battle.ui.DetailTab.ACTIVE_SKILLS)
	assert_str(battle.ui.active_skills_text.text).is_equal("%s\n%s" % [tr(BattleConfig.skill_name_key("presentation_heal")), "目標：自己或友方單位\n射程：0–6 格\n威力來源：魔法\n威力加值：+3\n效果：依威力回復 HP"])

	battle.world.inspection_clicked.emit(0, Vector2i(0, 2))
	assert_int(battle.ui.detail_tabs.current_tab).is_equal(battle.ui.DetailTab.TERRAIN)
	for index in range(battle.ui.detail_tabs.get_tab_count()):
		assert_bool(battle.ui.detail_tabs.is_tab_hidden(index)).is_equal(index != battle.ui.DetailTab.TERRAIN)
	assert_bool(battle.ui.terrain_effect.is_visible_in_tree()).is_true()
	battle.world.inspection_clicked.emit(1, Vector2i(0, 1))
	assert_int(battle.ui.detail_tabs.current_tab).is_equal(battle.ui.DetailTab.ACTIVE_SKILLS)
	for index in range(battle.ui.detail_tabs.get_tab_count()):
		assert_bool(battle.ui.detail_tabs.is_tab_hidden(index)).is_false()

# 驗證中英文依地形、屬性、主動、被動順序切換分頁時內容及標題留在面板內，長技能列表可捲動且切換單位重設捲動。
func test_detail_tabs_layout_and_scrolling() -> void:
	for locale in ["zh_TW", "en"]:
		TranslationServer.set_locale(locale)
		for unit_case in [
			{"id": 1, "cell": Vector2i(1, 3)},
			{"id": 2, "cell": Vector2i(3, 1)},
		]:
			battle.world.inspection_clicked.emit(unit_case.id, unit_case.cell)
			battle.present()
			for index in [battle.ui.DetailTab.TERRAIN, battle.ui.DetailTab.ATTRIBUTES, battle.ui.DetailTab.ACTIVE_SKILLS, battle.ui.DetailTab.PASSIVE_SKILLS, battle.ui.DetailTab.TERRAIN]:
				battle.ui.detail_tabs.current_tab = index
				await runner.simulate_frames(5)
				var page: ScrollContainer = battle.ui.detail_tabs.get_tab_control(index)
				var label: Label = {
					battle.ui.DetailTab.TERRAIN: battle.ui.terrain_effect,
					battle.ui.DetailTab.ATTRIBUTES: battle.ui.detail_values.hp,
					battle.ui.DetailTab.ACTIVE_SKILLS: battle.ui.active_skills_text,
					battle.ui.DetailTab.PASSIVE_SKILLS: battle.ui.passive_skills_text,
				}[index]
				var context := "%s 單位 %d 分頁 %d" % [locale, unit_case.id, index]
				assert_bool(label.is_visible_in_tree()).override_failure_message(context + "：內容節點應可見").is_true()
				assert_str(label.text).override_failure_message(context + "：內容不可空白").is_not_empty()
				assert_float(page.size.y).override_failure_message(context + "：可視區應至少能顯示一行文字").is_greater_equal(float(label.get_line_height()))
				assert_bool(page.get_global_rect().intersects(label.get_global_rect())).override_failure_message(context + "：文字應與可視區相交").is_true()
				assert_bool(battle.ui.info_panel.get_global_rect().encloses(page.get_global_rect())).override_failure_message(context + "：頁面應留在面板內").is_true()
				var bar: TabBar = battle.ui.detail_tabs.get_tab_bar()
				assert_bool(bar.get_offset_buttons_visible()).override_failure_message(context + "：不應出現左右導覽按鈕").is_false()
				for tab_index in bar.tab_count:
					var tab_rect := bar.get_tab_rect(tab_index)
					tab_rect.position += bar.global_position
					assert_bool(bar.get_global_rect().encloses(tab_rect)).override_failure_message(context + "：標題 %d 應完整顯示" % tab_index).is_true()
					assert_bool(battle.ui.info_panel.get_global_rect().encloses(tab_rect)).override_failure_message(context + "：標題 %d 應留在面板內" % tab_index).is_true()
		battle.world.inspection_clicked.emit(1, Vector2i(1, 3))
		battle.ui.detail_tabs.current_tab = battle.ui.DetailTab.ACTIVE_SKILLS
		var original_panel_size: Vector2 = battle.ui.info_panel.size
		battle.ui.info_panel.size.y = 340
		await runner.simulate_frames(3)
		var skill_page: ScrollContainer = battle.ui.detail_tabs.get_tab_control(battle.ui.DetailTab.ACTIVE_SKILLS)
		assert_float(battle.ui.active_skills_text.size.y).is_greater(skill_page.size.y)
		skill_page.scroll_vertical = 100
		await runner.simulate_frames(1)
		assert_int(skill_page.scroll_vertical).is_greater(0)
		assert_bool(skill_page.get_v_scroll_bar().visible).is_true()
		assert_float(battle.ui.unit_name.global_position.y).is_less(skill_page.global_position.y)
		battle.world.inspection_clicked.emit(2, Vector2i(3, 1))
		assert_int(skill_page.scroll_vertical).is_zero()
		battle.ui.info_panel.size = original_panel_size

# 驗證中英文技能 hover 依威力來源、命中加值、威力加值排序，且標題不包含額外的狀態前綴。
func test_skill_hover_uses_current_locale() -> void:
	var cases := [
		{"locale": "zh_TW", "title": "強力一擊", "details": "目標：敵方單位\n類型：近戰\n射程：1 格\n威力來源：物理\n命中加值：+2\n威力加值：+2"},
		{"locale": "en", "title": "Power Strike", "details": "Target: Enemy unit\nType: Melee\nRange: 1 tiles\nPower source: Physical\nHit modifier: +2\nPower modifier: +2"},
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

	var info_panel: PanelContainer = battle.ui.info_panel
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

# 驗證再次右鍵相同技能或按關閉鈕會取消技能查看，切換技能時更新內容並清除地格查看。
func test_skill_inspection_toggle_switch_and_close() -> void:
	battle.world.inspection_clicked.emit(1, Vector2i(1, 3))
	for test_case in [
		{"skill": "power_strike", "expected": "power_strike"},
		{"skill": "power_strike", "expected": ""},
		{"skill": "power_strike", "expected": "power_strike"},
		{"skill": "aimed_shot", "expected": "aimed_shot"},
	]:
		emit_right_press(battle.ui.action_buttons[test_case.skill])
		assert_str(battle.inspected_skill).is_equal(test_case.expected)
		assert_vector(battle.inspected_cell).is_equal(Vector2i(-1, -1))
		assert_bool(battle.ui.info_panel.visible).is_equal(not test_case.expected.is_empty())
		if not test_case.expected.is_empty():
			assert_str(battle.ui.inspected_skill_name.text).is_equal(tr(BattleConfig.skill_name_key(test_case.expected)))
	battle.ui.info_panel.get_node("Margin/Content/Header/Close").pressed.emit()
	assert_str(battle.inspected_skill).is_empty()
	assert_bool(battle.ui.info_panel.visible).is_false()

func add_test_translation(locale: String, messages: Dictionary) -> void:
	var translation := Translation.new()
	translation.locale = locale
	for message in messages:
		translation.add_message(message, messages[message])
	TranslationServer.add_translation(translation)
	test_translations.append(translation)

func wait_for_combat_events() -> void:
	while battle.state.turn.can_continue or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)

func emit_right_press(control: Control) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_RIGHT
	event.pressed = true
	control.gui_input.emit(event)
