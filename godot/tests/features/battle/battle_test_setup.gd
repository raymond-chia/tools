extends RefCounted

# 使用專用 TOML 重新建立真實戰鬥場景，供各測試保留自己的操作與斷言。
static func load_and_start(battle, runner: GdUnitSceneRunner, definitions_path: String, map_path: String) -> String:
	while battle.state.turn.auto_step or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)
	for child in battle.world.units_layer.get_children():
		child.free()
	battle.world.unit_nodes.clear()
	var definitions := FileAccess.get_file_as_string(definitions_path)
	var map := FileAccess.get_file_as_string(map_path)
	var loaded: Dictionary = JSON.parse_string(battle.core.load_documents(definitions, map))
	if loaded.has("error"):
		return "專用 TOML 載入失敗：%s" % loaded.error
	battle.state = loaded
	battle.world.setup_map(loaded)
	if not battle.send({"type": "start"}):
		return "專用測試戰鬥啟動失敗：%s" % battle.status
	while battle.state.turn.auto_step or battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)
	return ""
