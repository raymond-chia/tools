extends Control

const OPTIONS_OVERLAY_SCENE := preload("res://scenes/options/options_overlay.tscn")
const CREDITS_OVERLAY_SCENE := preload("res://scenes/credits/credits_overlay.tscn")

@onready var status_label: Label = %StatusLabel

func _on_placeholder_pressed(menu_key: String) -> void:
	status_label.text = "%s: %s" % [tr("PLACEHOLDER_PREFIX"), tr(menu_key)]

func _on_continue_pressed() -> void:
	_on_placeholder_pressed("MENU_CONTINUE")

func _on_new_game_pressed() -> void:
	var result := GameSessionManager.start_new_game()
	if result.get("ok", false) != true:
		return
	get_tree().change_scene_to_file("res://scenes/level/level_deployment.tscn")

func _on_load_pressed() -> void:
	_on_placeholder_pressed("MENU_LOAD")

func _on_options_pressed() -> void:
	add_child(OPTIONS_OVERLAY_SCENE.instantiate())

func _on_credits_pressed() -> void:
	add_child(CREDITS_OVERLAY_SCENE.instantiate())

func _on_quit_pressed() -> void:
	get_tree().quit()
