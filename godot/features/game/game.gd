extends Node

const BATTLE_SCENE := preload("res://features/battle/battle.tscn")

func _ready() -> void:
	var data_dir := DataSource.directory()
	var definitions_file := FileAccess.open(data_dir + "definitions.toml", FileAccess.READ)
	if definitions_file == null:
		push_error("無法讀取戰鬥資料：%s" % error_string(FileAccess.get_open_error()))
		return
	var map_file := FileAccess.open(data_dir + "maps/prelude.toml", FileAccess.READ)
	if map_file == null:
		push_error("無法讀取戰鬥資料：%s" % error_string(FileAccess.get_open_error()))
		return
	var battle := BATTLE_SCENE.instantiate()
	battle.configure(definitions_file.get_as_text(), map_file.get_as_text())
	get_tree().call_deferred("change_scene_to_node", battle)
