extends Node2D

const LEVEL_NAME := "lv1"

@onready var board: LevelBoard = $LevelBoard
@onready var victory_label: Label = %VictoryLabel
@onready var defeat_label: Label = %DefeatLabel
@onready var unit_details_label: Label = %UnitDetailsLabel
@onready var details_scroll: ScrollContainer = %DetailsScroll
@onready var info_panel: PanelContainer = %InfoPanel
@onready var unit_panel: PanelContainer = %UnitPanel
@onready var status_label: Label = %StatusLabel

func _ready() -> void:
	unit_panel.hide()
	var snapshot := GameSessionManager.enter_level(LEVEL_NAME)
	if snapshot.get("ok", false) == true:
		board.display_level(snapshot)

func _on_level_loaded(level_state: Dictionary) -> void:
	victory_label.text = _format_outcomes(tr("DEPLOYMENT_VICTORY"), level_state["victory_conditions"])
	defeat_label.text = _format_outcomes(tr("DEPLOYMENT_DEFEAT"), level_state["defeat_conditions"])

func _format_outcomes(heading: String, outcome_keys: Array) -> String:
	var lines := PackedStringArray([heading])
	for key in outcome_keys:
		lines.append("- %s" % tr(key))
	return "\n".join(lines)

func _on_unit_selected(unit: Dictionary) -> void:
	if unit.is_empty():
		_show_info_panel()
		return
	_show_unit_panel(unit)

func _show_info_panel() -> void:
	unit_panel.hide()
	info_panel.show()

func _show_unit_panel(unit: Dictionary) -> void:
	info_panel.hide()
	unit_panel.show()
	unit_details_label.text = _format_unit_details(unit)
	details_scroll.set_deferred(&"scroll_vertical", 0)

func _format_unit_details(unit: Dictionary) -> String:
	var attributes: Dictionary = unit["attributes"]
	return "\n".join([
		"%s: %s" % [tr("DEPLOYMENT_UNIT_NAME"), unit["name"]],
		"%s: %s" % [tr("DEPLOYMENT_FACTION"), unit["faction_name"]],
		"HP: %d / %d" % [attributes["current_hp"], attributes["max_hp"]],
		"MP: %d / %d" % [attributes["current_mp"], attributes["max_mp"]],
		"%s: %d" % [tr("STAT_INITIATIVE"), attributes["initiative"]],
		"%s: %d" % [tr("STAT_PHYSICAL_ATTACK"), attributes["physical_attack"]],
		"%s: %d" % [tr("STAT_MAGICAL_ATTACK"), attributes["magical_attack"]],
		"%s: %d" % [tr("STAT_PHYSICAL_ACCURACY"), attributes["physical_accuracy"]],
		"%s: %d" % [tr("STAT_MAGICAL_ACCURACY"), attributes["magical_accuracy"]],
		"%s: %d" % [tr("STAT_FORTITUDE"), attributes["fortitude"]],
		"%s: %d" % [tr("STAT_AGILITY"), attributes["agility"]],
		"%s: %d" % [tr("STAT_BLOCK"), attributes["block"]],
		"%s: %d" % [tr("STAT_BLOCK_PROTECTION"), attributes["block_protection"]],
		"%s: %d" % [tr("STAT_WILL"), attributes["will"]],
		"%s: %d" % [tr("STAT_MOVEMENT"), attributes["movement_point"]],
		"%s: %d / %d" % [tr("STAT_REACTION"), attributes["reaction_point"], attributes["max_reaction_point"]],
		"%s: %d" % [tr("STAT_FLANKING_ACCURACY"), attributes["flanking_accuracy_bonus"]],
	])

func _on_start_battle_pressed() -> void:
	get_tree().change_scene_to_file("res://scenes/level/level.tscn")

func _on_menu_pressed() -> void:
	status_label.text = "%s: %s" % [tr("PLACEHOLDER_PREFIX"), tr("DEPLOYMENT_MENU")]
