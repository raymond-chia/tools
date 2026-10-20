extends "res://tests/features/battle/battle_test_suite.gd"

const DATA := "res://tests/features/battle/data/difficulty/"
const FACING_DEFINITIONS := "res://tests/features/battle/data/combat_presentation_definitions.toml"
const FACING_MAP := "res://tests/features/battle/data/combat_presentation_facing_map.toml"
const ACTOR_ID := 1
const TARGET_ID := 3
const EDITOR_SCENE := preload("res://features/editor/editor.tscn")

# 驗證真實戰鬥場景呈現左右初始朝向，且單位快照不攜帶方向。
func test_battle_presents_map_facing() -> void:
	var battle = auto_free(create_battle(DATA + "definitions.toml", DATA + "maps/prelude.toml"))
	var runner := battle_scene_runner(battle)
	await runner.simulate_frames(1)
	for test_case in [{"id": 1, "flip_h": true}, {"id": 2, "flip_h": false}]:
		var unit: Dictionary = battle.state.units.filter(func(entry: Dictionary): return entry.id == test_case.id)[0]
		assert_bool(unit.has("facing")).is_false()
		var body: Sprite2D = battle.world.unit_nodes[test_case.id].get_node("Visual/Body")
		assert_bool(body.flip_h).is_equal(test_case.flip_h)

# 驗證 TOML 初始朝左，移動轉向後刷新畫面不會重套初始方向。
func test_movement_faces_travel_direction() -> void:
	var battle = auto_free(create_battle(FACING_DEFINITIONS, FACING_MAP, PRESENTATION_RANDOM_SEED))
	var runner := battle_scene_runner(battle)
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	await wait_until_idle(battle, runner)
	reset_random_seed(battle.core, PRESENTATION_RANDOM_SEED)
	runner.set_time_factor(1.0)
	var node: Node2D = battle.world.unit_nodes[ACTOR_ID]
	var body: Sprite2D = node.get_node("Visual/Body")
	assert_bool(body.flip_h).is_true()
	for test_case in [
		{"cell": Vector2i(2, 1), "flip_h": true},
		{"cell": Vector2i(3, 1), "flip_h": false},
	]:
		var destination: Vector2 = battle.world.cell_center(test_case.cell)
		var start_position := node.position
		assert_bool(battle.send({"type": "move", "actor": ACTOR_ID, "x": test_case.cell.x, "y": test_case.cell.y})).is_true()
		for _frame in 10:
			await runner.simulate_frames(1, 16)
			if not node.position.is_equal_approx(start_position):
				break
		assert_bool(battle.world.is_presenting_combat_events()).is_true()
		assert_bool(node.position.is_equal_approx(start_position)).is_false()
		assert_bool(body.flip_h).override_failure_message("移動演出期間應朝向行進方向 %s" % test_case.cell).is_equal(test_case.flip_h)
		await wait_until_idle(battle, runner)
		assert_vector(node.position).is_equal(destination)
		assert_bool(body.flip_h).override_failure_message("移動結束後應保持行進朝向 %s" % test_case.cell).is_equal(test_case.flip_h)
		battle.present()
		assert_bool(body.flip_h).override_failure_message("畫面刷新不得重套初始朝向").is_equal(test_case.flip_h)

# 驗證同一單位先向左施法，正常推進回合後向右施法，兩次演出開始與結束皆朝向目標。
func test_skill_faces_target_direction() -> void:
	var battle = auto_free(create_battle(FACING_DEFINITIONS, FACING_MAP, PRESENTATION_RANDOM_SEED))
	var runner := battle_scene_runner(battle)
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	await wait_until_idle(battle, runner)
	reset_random_seed(battle.core, PRESENTATION_RANDOM_SEED)
	runner.set_time_factor(1.0)
	var node: Node2D = battle.world.unit_nodes[ACTOR_ID]
	var body: Sprite2D = node.get_node("Visual/Body")
	for test_case in [
		{"target": TARGET_ID, "cell": Vector2i(1, 1), "flip_h": true, "direction": "左"},
		{"target": 4, "cell": Vector2i(7, 3), "flip_h": false, "direction": "右"},
	]:
		assert_int(battle.state.turn.actor).is_equal(ACTOR_ID)
		assert_object(battle.world.unit_nodes[ACTOR_ID]).is_same(node)
		var target: Node2D = battle.world.unit_nodes[test_case.target]
		assert_bool(target.position.x < node.position.x).is_equal(test_case.flip_h)
		battle.ui.action_buttons.presentation_push.pressed.emit()
		battle.world.primary_clicked.emit(test_case.target, test_case.cell)
		assert_bool(battle.world.is_presenting_combat_events()).is_true()
		assert_bool(body.flip_h).override_failure_message("向%s施放技能的前衝開始前應已轉向目標" % test_case.direction).is_equal(test_case.flip_h)
		await wait_until_idle(battle, runner)
		assert_bool(body.flip_h).override_failure_message("向%s施放技能結束後應保持朝向" % test_case.direction).is_equal(test_case.flip_h)
		if test_case.flip_h:
			var previous_round: int = battle.state.round
			for _turn in 4:
				battle.ui.end_turn_requested.emit()
				await wait_until_idle(battle, runner)
				if int(battle.state.turn.actor) == ACTOR_ID:
					break
			assert_int(int(battle.state.round)).is_greater(previous_round)
			assert_bool(body.flip_h).override_failure_message("正常推進回合後應保留左側技能留下的朝向").is_true()

# 驗證編輯器修改左右方向、即時預覽、復原重做及 TOML 序列化往返。
func test_editor_changes_and_serializes_facing() -> void:
	var editor := EDITOR_SCENE.instantiate()
	editor.data_dir = DATA
	editor.definitions_path = DATA + "definitions.toml"
	editor.map_dir = DATA + "maps/"
	var runner := scene_runner(auto_free(editor))
	await runner.simulate_frames(1)
	for test_case in [{"cell": Vector2i(0, 0), "unit": 0, "from": "left", "to": "right", "option": 0}, {"cell": Vector2i(1, 0), "unit": 1, "from": "right", "to": "left", "option": 1}]:
		editor.inspect_unit(test_case.cell)
		assert_bool(editor.inspected_facing.visible).is_true()
		assert_str(editor.inspected_facing.get_selected_metadata()).is_equal(test_case.from)
		editor.inspected_facing.select(test_case.option)
		editor.inspected_facing.item_selected.emit(test_case.option)
		assert_str(editor.map_data.units[test_case.unit].facing).is_equal(test_case.to)
		var unit_id: int = editor.map_data.units[test_case.unit].id
		var body: Sprite2D = editor.map_view.unit_nodes[unit_id].get_node("Visual/Body")
		assert_bool(body.flip_h).is_equal(test_case.to == "left")
		editor.undo()
		assert_str(editor.map_data.units[test_case.unit].facing).is_equal(test_case.from)
		assert_str(editor.inspected_facing.get_selected_metadata()).is_equal(test_case.from)
		body = editor.map_view.unit_nodes[unit_id].get_node("Visual/Body")
		assert_bool(body.flip_h).is_equal(test_case.from == "left")
		editor.redo()
		assert_str(editor.map_data.units[test_case.unit].facing).is_equal(test_case.to)
		assert_str(editor.inspected_facing.get_selected_metadata()).is_equal(test_case.to)
		body = editor.map_view.unit_nodes[unit_id].get_node("Visual/Body")
		assert_bool(body.flip_h).is_equal(test_case.to == "left")
	var documents: Dictionary = editor.serialize_documents()
	var roundtrip: Dictionary = editor.read_core_response(editor.core.map_to_json(documents.map))
	assert_str(roundtrip.units[0].facing).is_equal("right")
	assert_str(roundtrip.units[1].facing).is_equal("left")
	var battle := BATTLE_SCENE.instantiate()
	battle.configure(documents.definitions, documents.map, "easy", false, RANDOM_SEED)
	var battle_runner := battle_scene_runner(auto_free(battle))
	await battle_runner.simulate_frames(1)
	assert_bool(battle.world.unit_nodes[1].get_node("Visual/Body").flip_h).is_false()
	assert_bool(battle.world.unit_nodes[2].get_node("Visual/Body").flip_h).is_true()

# 驗證放置選單可建立左右朝向單位，且預覽同步呈現。
func test_editor_places_units_with_selected_facing() -> void:
	var editor := EDITOR_SCENE.instantiate()
	editor.data_dir = DATA
	editor.definitions_path = DATA + "definitions.toml"
	editor.map_dir = DATA + "maps/"
	var runner := scene_runner(auto_free(editor))
	await runner.simulate_frames(1)
	editor.palette_tabs.current_tab = 2
	editor.mode_list.select(0)
	editor.select_text(editor.unit_list, "aria")
	assert_bool(editor.placement_facing.visible).is_true()
	for test_case in [{"cell": Vector2i(0, 1), "option": 1, "facing": "left"}, {"cell": Vector2i(1, 1), "option": 0, "facing": "right"}]:
		editor.placement_facing.select(test_case.option)
		editor.edit_cell(test_case.cell)
		var unit: Dictionary = editor.map_data.units.back()
		assert_str(unit.facing).is_equal(test_case.facing)
		var body: Sprite2D = editor.map_view.unit_nodes[unit.id].get_node("Visual/Body")
		assert_bool(body.flip_h).is_equal(test_case.facing == "left")

# 驗證建圖事件經共用接收入口還原單位 ID，僅包含實際出現單位且查詢與開始回合不重送。
func test_map_creation_emits_facing_events_once() -> void:
	var core := TacticalGame.new()
	var report_error := func(message: String): assert_str(message).is_empty()
	var loaded: Dictionary = CoreResponse.read(core.load_documents(FileAccess.get_file_as_string(DATA + "definitions.toml"), FileAccess.get_file_as_string(DATA + "maps/prelude.toml"), "easy"), report_error)
	assert_array(loaded.presentation_events).is_equal([
		{"type": "unit_faced", "unit_id": 1, "facing": "left"},
		{"type": "unit_faced", "unit_id": 2, "facing": "right"},
	])
	var queried: Dictionary = CoreResponse.read(core.snapshot(null), report_error)
	assert_array(queried.presentation_events).is_empty()
	var started: Dictionary = CoreResponse.read(core.dispatch(JSON.stringify({"type": "start"})), report_error)
	assert_array(started.presentation_events).is_empty()
