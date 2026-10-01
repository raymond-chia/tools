extends RefCounted

const BATTLE_SCENE := preload("res://features/battle/battle.tscn")

# 使用各測試指定的專用文件建立尚未進入場景樹的戰鬥場景。
static func create_battle(definitions_path: String, map_path: String) -> Node:
	var battle := BATTLE_SCENE.instantiate()
	battle.configure(FileAccess.get_file_as_string(definitions_path), FileAccess.get_file_as_string(map_path))
	return battle

static func wait_until_idle(battle, runner: GdUnitSceneRunner) -> void:
	while battle.state.turn.can_continue or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)

# 使用專用 TOML 重新建立真實戰鬥場景，供各測試保留自己的操作與斷言。
static func load_and_start(battle, runner: GdUnitSceneRunner, definitions_path: String, map_path: String) -> String:
	await wait_until_idle(battle, runner)
	clear_combat_log(battle)
	for child in battle.world.units_layer.get_children():
		child.free()
	battle.world.unit_nodes.clear()
	var definitions := FileAccess.get_file_as_string(definitions_path)
	var map := FileAccess.get_file_as_string(map_path)
	var loaded: Dictionary = CoreResponse.read(battle.core.load_documents(definitions, map), battle.show_error)
	if loaded.is_empty():
		return "專用 TOML 載入失敗：%s" % battle.status
	battle.state = loaded
	battle.world.setup_map(loaded)
	if not battle.send({"type": "start"}):
		return "專用測試戰鬥啟動失敗：%s" % battle.status
	await wait_until_idle(battle, runner)
	return ""

static func clear_combat_log(battle) -> void:
	battle.pending_display_log.clear()
	battle.ui.presented_log_events.clear()
	battle.ui.log_entry_expanded_states.clear()
	battle.ui.battle_log.text = ""
