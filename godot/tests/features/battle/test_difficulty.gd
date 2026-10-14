extends "res://tests/features/battle/battle_test_suite.gd"

const DATA := "res://tests/features/battle/data/difficulty/"
const EDITOR_SCENE := preload("res://features/editor/editor.tscn")
const GAME_SCENE := preload("res://features/game/game.tscn")

# 驗證真實戰鬥場景把所選難度交給核心，僅生成該難度的怪物。
func test_battle_uses_selected_difficulty() -> void:
	for test_case in [{"difficulty": "easy", "ids": [1, 2]}, {"difficulty": "normal", "ids": [1, 2, 3]}, {"difficulty": "hard", "ids": [1, 2, 3, 4]}]:
		var battle := BATTLE_SCENE.instantiate()
		battle.configure(FileAccess.get_file_as_string(DATA + "definitions.toml"), FileAccess.get_file_as_string(DATA + "maps/prelude.toml"), test_case.difficulty, false, RANDOM_SEED)
		var runner := battle_scene_runner(auto_free(battle))
		await runner.simulate_frames(1)
		var ids: Array = battle.state.units.map(func(unit: Dictionary): return unit.id)
		ids.sort()
		assert_array(ids).is_equal(test_case.ids)

# 驗證 menu 保存各難度，開局從設定讀取並交給真實戰鬥場景。
func test_game_reads_difficulty_setting() -> void:
	var battle = auto_free(create_battle(DATA + "definitions.toml", DATA + "maps/prelude.toml"))
	var runner := battle_scene_runner(battle)
	await runner.simulate_frames(1)
	for test_case in [{"id": 2, "difficulty": "easy"}, {"id": 3, "difficulty": "normal"}, {"id": 4, "difficulty": "hard"}]:
		battle.ui.menu_button.get_popup().id_pressed.emit(test_case.id)
		var settings := ConfigFile.new()
		assert_int(settings.load(GameSettings.PATH)).is_equal(OK)
		assert_str(settings.get_value("game", "difficulty")).is_equal(test_case.difficulty)
		assert_bool(battle.ui.menu_button.get_popup().is_item_checked(battle.ui.menu_button.get_popup().get_item_index(test_case.id))).is_true()
		var game = auto_free(GAME_SCENE.instantiate())
		game.data_dir = DATA
		var next_battle = auto_free(game.create_battle())
		assert_str(next_battle.difficulty).is_equal(test_case.difficulty)

# 驗證編輯器顯示全部怪物，並正確修改、復原與重做最低出現難度。
func test_editor_changes_and_undoes_enemy_difficulty() -> void:
	var editor := EDITOR_SCENE.instantiate()
	editor.data_dir = DATA
	editor.definitions_path = DATA + "definitions.toml"
	editor.map_dir = DATA + "maps/"
	var runner := scene_runner(auto_free(editor))
	await runner.simulate_frames(1)
	assert_int(editor.map_view.snapshot.units.size()).is_equal(4)
	editor.inspect_unit(Vector2i(2, 0))
	assert_bool(editor.inspected_difficulty.visible).is_true()
	editor.inspected_difficulty.select(3)
	editor.inspected_difficulty.item_selected.emit(3)
	assert_str(editor.map_data.units[2].min_difficulty).is_equal("hard")
	editor.undo()
	assert_str(editor.map_data.units[2].min_difficulty).is_equal("normal")
	editor.redo()
	assert_str(editor.map_data.units[2].min_difficulty).is_equal("hard")

# 驗證放置新怪物前必須選擇出現難度，不會偷偷補上簡單設定。
func test_editor_requires_difficulty_before_enemy_placement() -> void:
	var editor := EDITOR_SCENE.instantiate()
	editor.data_dir = DATA
	editor.definitions_path = DATA + "definitions.toml"
	editor.map_dir = DATA + "maps/"
	var runner := scene_runner(auto_free(editor))
	await runner.simulate_frames(1)
	editor.palette_tabs.current_tab = 2
	editor.mode_list.select(0)
	editor.team_list.select(1)
	editor.faction_field.text = "test_enemy"
	editor.refresh_tools_visibility()
	editor.select_text(editor.unit_list, "wolf")
	editor.edit_cell(Vector2i(0, 1))
	assert_int(editor.map_data.units.size()).is_equal(4)
	assert_str(editor.status_label.text).contains("難度")
	editor.placement_difficulty.select(1)
	editor.edit_cell(Vector2i(0, 1))
	assert_int(editor.map_data.units.size()).is_equal(5)
	assert_str(editor.map_data.units[4].min_difficulty).is_equal("easy")
