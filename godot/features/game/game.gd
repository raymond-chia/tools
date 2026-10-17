extends Control

const BATTLE_SCENE := preload("res://features/battle/battle.tscn")

@onready var error_label: Label = $Center/Menu/Error

var data_dir := DataSource.directory()
var map_path := "maps/ch1.toml"

func _ready() -> void:
	start_game.call_deferred()

func start_game() -> void:
	var battle := create_battle()
	if battle != null:
		get_tree().call_deferred("change_scene_to_node", battle)

# 開局與設定讀取集中於同一入口，讀檔失敗留在錯誤畫面。
func create_battle() -> Node:
	var definitions_file := FileAccess.open(data_dir + "definitions.toml", FileAccess.READ)
	if definitions_file == null:
		error_label.text = tr("GAME_DATA_READ_ERROR") % error_string(FileAccess.get_open_error())
		return null
	var map_file := FileAccess.open(data_dir + map_path, FileAccess.READ)
	if map_file == null:
		error_label.text = tr("GAME_DATA_READ_ERROR") % error_string(FileAccess.get_open_error())
		return null
	var battle := BATTLE_SCENE.instantiate()
	battle.configure(definitions_file.get_as_text(), map_file.get_as_text(), GameSettings.load_difficulty())
	return battle
