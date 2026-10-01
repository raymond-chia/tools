extends Node

const BATTLE_SCENE := preload("res://features/battle/battle.tscn")
const DEFINITIONS_PATH := "res://data/definitions.toml"
const MAP_PATH := "res://data/maps/ash_valley.toml"

func _ready() -> void:
	var definitions_file := FileAccess.open(DEFINITIONS_PATH, FileAccess.READ)
	if definitions_file == null:
		push_error("無法讀取戰鬥資料：%s" % error_string(FileAccess.get_open_error()))
		return
	var map_file := FileAccess.open(MAP_PATH, FileAccess.READ)
	if map_file == null:
		push_error("無法讀取戰鬥資料：%s" % error_string(FileAccess.get_open_error()))
		return
	var battle := BATTLE_SCENE.instantiate()
	battle.configure(definitions_file.get_as_text(), map_file.get_as_text())
	get_tree().call_deferred("change_scene_to_node", battle)
