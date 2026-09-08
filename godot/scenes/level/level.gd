extends Node2D

const BATTLE_OUTCOME_OVERLAY_SCENE := preload("res://scenes/level/battle_outcome_overlay/battle_outcome_overlay.tscn")
const TRANSIENT_PROMPT_DURATION := 2.0
const TURN_PORTRAIT_SIZE := Vector2(64, 64)
const TURN_PORTRAIT_SEPARATION := 8
const DELAY_INSERTION_COLOR := Color(0.95, 0.75, 0.2)
const DELAY_BUTTON_HORIZONTAL_GAP := 2.0
const DELAY_BUTTON_VERTICAL_GAP := 4.0
const UNIT_PORTRAITS := {
	"fighter": preload("res://assets/units/fighter.svg"),
	"cleric": preload("res://assets/units/cleric.svg"),
	"archer": preload("res://assets/units/archer.svg"),
	"wolf": preload("res://assets/units/wolf.svg"),
}

@onready var board: LevelBoard = $LevelBoard
@onready var skill_panel: PanelContainer = $Hud/SkillPanel
@onready var current_unit_portrait: TextureButton = %CurrentUnitPortrait
@onready var skill_buttons: HBoxContainer = %SkillButtons
@onready var delay_turn_button: Button = %DelayTurnButton
@onready var end_turn_button: Button = %EndTurnButton
@onready var move_cost_label: Label = %MoveCostLabel
@onready var prompt_label: Label = %PromptLabel
@onready var error_dialog: AcceptDialog = %ErrorDialog
@onready var right_sidebar: VBoxContainer = $Hud/Ui/RightSidebar
@onready var unit_panel: PanelContainer = %UnitPanel
@onready var unit_details_label: Label = %UnitDetailsLabel
@onready var details_scroll: ScrollContainer = %DetailsScroll
@onready var turn_order_scroll: ScrollContainer = %TurnOrderScroll
@onready var turn_order_items: VBoxContainer = %TurnOrderItems

var _selected_skill := ""
var _current_turn_unit_id := -1
var _inspected_unit_id := -1
var _is_resolving_action := false
var _battle_finished := false
var _is_selecting_delay_target := false
var _prompt_tween: Tween


func _ready() -> void:
	right_sidebar.hide()
	prompt_label.hide()
	skill_panel.resized.connect(_position_delay_turn_button)
	get_viewport().size_changed.connect(_position_delay_turn_button)
	call_deferred(&"_position_delay_turn_button")
	current_unit_portrait.pressed.connect(_on_current_unit_portrait_pressed)
	turn_order_scroll.connect("click_released", _on_turn_order_scroll_click_released)
	if not GameSessionManager.has_active_session():
		push_error("Cannot enter battle: no active game session")
		_show_error_popup_message(GameSessionManager.get_error_message_for_code("NoActiveSession"))
		return
	var battle_result := GameSessionManager.start_battle()
	if battle_result.get("ok", false) != true:
		push_error(battle_result.get("error", "Failed to start battle"))
		_show_error_popup(battle_result)
		return
	var level_snapshot := GameSessionManager.get_level_snapshot()
	if level_snapshot.get("ok", false) != true:
		push_error("Cannot enter battle: no level loaded")
		_show_error_popup(level_snapshot)
		return
	board.display_level(level_snapshot)
	_refresh_turn_order()
	_refresh_skill_buttons()
	_refresh_reachable_positions()


func _position_delay_turn_button() -> void:
	delay_turn_button.position = Vector2(
		current_unit_portrait.global_position.x
			+ current_unit_portrait.size.x
			+ DELAY_BUTTON_HORIZONTAL_GAP,
		skill_panel.global_position.y
			- delay_turn_button.size.y
			- DELAY_BUTTON_VERTICAL_GAP
	)


func _process(_delta: float) -> void:
	if not _is_selecting_delay_target:
		return
	var pointer_position := get_viewport().get_mouse_position()
	for child in turn_order_items.get_children():
		if child is TextureButton:
			var marker: ColorRect = child.get_node("DelayInsertionMarker")
			marker.visible = child.get_global_rect().has_point(pointer_position)


func _input(event: InputEvent) -> void:
	if (
		_is_selecting_delay_target
		and event is InputEventMouseButton
		and event.button_index == MOUSE_BUTTON_RIGHT
		and event.pressed
	):
		_cancel_delay_selection()
		_refresh_reachable_positions()
		get_viewport().set_input_as_handled()


func _on_unit_selected(unit: Dictionary) -> void:
	if unit.is_empty():
		_hide_unit_panel()
		return

	var unit_id: int = unit["id"]
	if unit_panel.visible and unit_id == _inspected_unit_id:
		_hide_unit_panel()
		return

	_show_unit_details(unit)


func _show_unit_details(unit: Dictionary) -> void:
	_inspected_unit_id = unit["id"]
	unit_details_label.text = _format_unit_details(unit)
	unit_panel.show()
	right_sidebar.show()
	details_scroll.set_deferred(&"scroll_vertical", 0)


func _hide_unit_panel() -> void:
	_inspected_unit_id = -1
	right_sidebar.hide()


func _refresh_inspected_unit_details(units: Array) -> void:
	if _inspected_unit_id < 0:
		return
	for unit in units:
		if unit["id"] != _inspected_unit_id:
			continue
		_show_unit_details(unit)
		return
	_hide_unit_panel()


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


func _refresh_skill_buttons() -> void:
	_clear_skill_buttons()
	end_turn_button.disabled = _battle_finished or _is_resolving_action
	_refresh_delay_button()
	if _battle_finished:
		return
	var result := GameSessionManager.get_available_skills()
	if result.get("ok", false) != true:
		_show_error_popup(result)
		return
	var skills: Array = result["skills"]
	if skills.is_empty():
		_show_error_popup_message(tr("BATTLE_NO_SKILLS"))
		return
	for skill in skills:
		var button := Button.new()
		button.text = "%s (MP %d)" % [skill["name"], skill["cost"]]
		button.disabled = not skill["usable"]
		button.pressed.connect(_on_skill_pressed.bind(skill["name"]))
		skill_buttons.add_child(button)


func _refresh_turn_order() -> void:
	_cancel_delay_selection()
	_clear_turn_order_items()
	var result := GameSessionManager.get_remaining_turn_units()
	if result.get("ok", false) != true:
		_show_error_popup(result)
		return
	var units: Array = result["units"]
	if units.is_empty():
		_current_turn_unit_id = -1
		current_unit_portrait.texture_normal = null
		current_unit_portrait.tooltip_text = ""
		return

	var current_unit: Dictionary = units[0]
	_current_turn_unit_id = current_unit["id"]
	_set_portrait(current_unit_portrait, current_unit)
	for index in range(units.size() - 1, 0, -1):
		var portrait := TextureButton.new()
		portrait.custom_minimum_size = TURN_PORTRAIT_SIZE
		portrait.ignore_texture_size = true
		portrait.stretch_mode = TextureButton.STRETCH_KEEP_ASPECT_CENTERED
		portrait.mouse_filter = Control.MOUSE_FILTER_IGNORE
		_set_portrait(portrait, units[index])
		portrait.set_meta(&"unit_id", units[index]["id"])
		portrait.set_meta(&"turn_index", index)
		var insertion_marker := ColorRect.new()
		insertion_marker.name = "DelayInsertionMarker"
		insertion_marker.color = DELAY_INSERTION_COLOR
		insertion_marker.mouse_filter = Control.MOUSE_FILTER_IGNORE
		insertion_marker.set_anchors_and_offsets_preset(Control.PRESET_TOP_WIDE)
		insertion_marker.offset_top = 0.0
		insertion_marker.offset_bottom = 4.0
		insertion_marker.visible = false
		portrait.add_child(insertion_marker)
		turn_order_items.add_child(portrait)
	var future_unit_count := units.size() - 1
	turn_order_items.custom_minimum_size = Vector2(
		TURN_PORTRAIT_SIZE.x,
		future_unit_count * (TURN_PORTRAIT_SIZE.y + TURN_PORTRAIT_SEPARATION) - TURN_PORTRAIT_SEPARATION
	)
	call_deferred(&"_scroll_turn_order_to_bottom")


func _set_portrait(portrait: TextureButton, unit: Dictionary) -> void:
	portrait.texture_normal = UNIT_PORTRAITS[unit["name"]]
	portrait.tooltip_text = unit["name"]


func _on_current_unit_portrait_pressed() -> void:
	_on_turn_unit_portrait_pressed(_current_turn_unit_id)


func _on_turn_unit_portrait_pressed(unit_id: int) -> void:
	if unit_id < 0:
		return
	var unit := board.select_unit_by_id(unit_id)
	if unit.is_empty() or not right_sidebar.visible:
		return
	_show_unit_details(unit)


func _on_turn_order_scroll_click_released(global_position: Vector2) -> void:
	for child in turn_order_items.get_children():
		if child is not TextureButton:
			continue
		if not child.get_global_rect().has_point(global_position):
			continue
		if _is_selecting_delay_target:
			_delay_current_unit(child.get_meta(&"turn_index"))
			return
		var unit_id: int = child.get_meta(&"unit_id")
		_on_turn_unit_portrait_pressed(unit_id)
		return


func _clear_turn_order_items() -> void:
	turn_order_items.custom_minimum_size = Vector2.ZERO
	while turn_order_items.get_child_count() > 0:
		var child := turn_order_items.get_child(0)
		turn_order_items.remove_child(child)
		child.queue_free()


func _refresh_delay_button() -> void:
	if _battle_finished or _is_resolving_action or turn_order_items.get_child_count() == 0:
		delay_turn_button.disabled = true
		_cancel_delay_selection()
		return
	var result := GameSessionManager.can_delay_current_unit()
	if result.get("ok", false) != true:
		delay_turn_button.disabled = true
		_cancel_delay_selection()
		_show_error_popup(result)
		return
	delay_turn_button.disabled = not result["can_delay"]
	if delay_turn_button.disabled:
		_cancel_delay_selection()


func _on_delay_turn_toggled(button_pressed: bool) -> void:
	if delay_turn_button.disabled:
		return
	_is_selecting_delay_target = button_pressed
	if button_pressed:
		_selected_skill = ""
		board.clear_skill_targeting()
		_show_persistent_prompt(tr("BATTLE_DELAY_PROMPT"))
	else:
		_hide_delay_insertion_markers()
		_hide_prompt()
		_refresh_reachable_positions()


func _delay_current_unit(target_index: int) -> void:
	if not _is_selecting_delay_target:
		return
	_is_resolving_action = true
	_set_skill_buttons_disabled(true)
	delay_turn_button.disabled = true
	end_turn_button.disabled = true
	var result := GameSessionManager.delay_current_unit(target_index)
	_cancel_delay_selection()
	if result.get("ok", false) != true:
		_show_error_popup(result)
		_is_resolving_action = false
		_refresh_skill_buttons()
		_refresh_reachable_positions()
		return
	board.apply_authoritative_changes(result)
	_refresh_inspected_unit_details(result["units"])
	_is_resolving_action = false
	_refresh_turn_order()
	_refresh_skill_buttons()
	_refresh_reachable_positions()


func _cancel_delay_selection() -> void:
	_is_selecting_delay_target = false
	if is_instance_valid(delay_turn_button):
		delay_turn_button.set_pressed_no_signal(false)
	_hide_delay_insertion_markers()
	_hide_prompt()


func _hide_delay_insertion_markers() -> void:
	for child in turn_order_items.get_children():
		if child is TextureButton and child.has_node("DelayInsertionMarker"):
			child.get_node("DelayInsertionMarker").hide()


func _scroll_turn_order_to_bottom() -> void:
	turn_order_scroll.scroll_vertical = roundi(turn_order_scroll.get_v_scroll_bar().max_value)


func _refresh_reachable_positions() -> void:
	if _battle_finished or not _selected_skill.is_empty():
		return
	var result := GameSessionManager.get_current_unit_reachable_positions()
	if result.get("ok", false) != true:
		_show_error_popup(result)
		return
	board.set_movement_targetable_positions(
		result["moves"],
		result["normal_range_cost"],
		result["movement_cost_used"]
	)


func _clear_skill_buttons() -> void:
	while skill_buttons.get_child_count() > 0:
		var child := skill_buttons.get_child(0)
		skill_buttons.remove_child(child)
		child.queue_free()


func _on_skill_pressed(skill_name: String) -> void:
	if _is_resolving_action:
		return
	_cancel_delay_selection()
	var result := GameSessionManager.get_skill_targetable_positions(skill_name)
	if result.get("ok", false) != true:
		_show_error_popup(result)
		return
	var positions: Array = result["positions"]
	if positions.is_empty():
		_selected_skill = ""
		board.clear_skill_targeting()
		_show_error_popup_message(tr("BATTLE_NO_TARGETS"))
		return
	_selected_skill = skill_name
	board.set_skill_targetable_positions(positions)
	_hide_prompt()


func _on_target_selected(position: Vector2i) -> void:
	if _is_resolving_action:
		return
	if _is_selecting_delay_target:
		return
	if _selected_skill.is_empty():
		_move_current_unit(position)
		return
	_execute_skill_target(position)


func _on_skill_target_hovered(position: Vector2i) -> void:
	if _selected_skill.is_empty() or _is_resolving_action:
		return
	var result := GameSessionManager.get_skill_affected_positions(_selected_skill, position)
	if result.get("ok", false) != true:
		board.set_skill_affected_positions([])
		return
	board.set_skill_affected_positions(result["positions"])


func _on_skill_target_hover_cleared() -> void:
	board.set_skill_affected_positions([])


func _execute_skill_target(position: Vector2i) -> void:
	_is_resolving_action = true
	board.clear_skill_targeting()
	_set_skill_buttons_disabled(true)
	var targets: Array[Vector2i] = [position]
	var result := GameSessionManager.execute_skill(_selected_skill, targets)
	_selected_skill = ""
	if not result.get("ok", false):
		_show_transient_prompt(GameSessionManager.get_error_message(result))
		_is_resolving_action = false
		_refresh_skill_buttons()
		_refresh_reachable_positions()
		return
	await board.play_effect_entries(result["entries"], position)
	board.apply_authoritative_changes(result)
	_refresh_inspected_unit_details(result["units"])
	_show_outcome(result["outcome"])
	_is_resolving_action = false
	_refresh_turn_order()
	_refresh_skill_buttons()
	_refresh_reachable_positions()


func _move_current_unit(position: Vector2i) -> void:
	_is_resolving_action = true
	board.clear_skill_targeting()
	_set_skill_buttons_disabled(true)
	end_turn_button.disabled = true
	var result := GameSessionManager.move_current_unit(position)
	if result.get("ok", false) != true:
		_show_error_popup(result)
		_is_resolving_action = false
		_refresh_skill_buttons()
		_refresh_reachable_positions()
		return
	await board.play_movement_path(_current_turn_unit_id, result["path_walked"])
	board.apply_authoritative_changes(result)
	_refresh_inspected_unit_details(result["units"])
	if result["outcome"]["status"] != "undetermined":
		_show_outcome(result["outcome"])
	_is_resolving_action = false
	_refresh_turn_order()
	_refresh_skill_buttons()
	_refresh_reachable_positions()


func _on_skill_targeting_cancelled() -> void:
	_cancel_delay_selection()
	_selected_skill = ""
	_hide_prompt()
	_refresh_reachable_positions()


func _on_movement_preview_changed(_path: Array, cost: int, pointer_position: Vector2) -> void:
	move_cost_label.text = tr("BATTLE_MOVE_COST") % cost
	move_cost_label.show()
	move_cost_label.reset_size()
	var label_size := move_cost_label.get_combined_minimum_size()
	var viewport_size := get_viewport_rect().size
	var position := pointer_position + Vector2(16, 16)
	if position.x + label_size.x > viewport_size.x - 8:
		position.x = pointer_position.x - label_size.x - 16
	if position.y + label_size.y > viewport_size.y - 8:
		position.y = pointer_position.y - label_size.y - 16
	move_cost_label.position = position.clamp(
		Vector2(8, 8),
		viewport_size - label_size - Vector2(8, 8)
	)


func _on_movement_preview_cleared() -> void:
	move_cost_label.hide()


func _set_skill_buttons_disabled(disabled: bool) -> void:
	for child in skill_buttons.get_children():
		if child is Button:
			child.disabled = disabled


func _on_end_turn_pressed() -> void:
	if _is_resolving_action or _battle_finished:
		return
	_is_resolving_action = true
	_cancel_delay_selection()
	_selected_skill = ""
	board.clear_skill_targeting()
	_set_skill_buttons_disabled(true)
	end_turn_button.disabled = true
	var result := GameSessionManager.end_current_turn()
	if result.get("ok", false) != true:
		_show_error_popup(result)
		_is_resolving_action = false
		_refresh_skill_buttons()
		_refresh_reachable_positions()
		return
	board.apply_authoritative_changes(result)
	_refresh_inspected_unit_details(result["units"])
	if result["outcome"]["status"] != "undetermined":
		_show_outcome(result["outcome"])
	_is_resolving_action = false
	_refresh_turn_order()
	_refresh_skill_buttons()
	_refresh_reachable_positions()


func _show_outcome(outcome: Dictionary) -> void:
	match outcome["status"]:
		"victory":
			_battle_finished = true
			_show_battle_outcome_overlay(outcome)
		"defeat":
			_battle_finished = true
			_show_battle_outcome_overlay(outcome)


func _show_persistent_prompt(message: String) -> void:
	if _prompt_tween != null:
		_prompt_tween.kill()
	prompt_label.modulate = Color.WHITE
	prompt_label.text = message
	prompt_label.show()


func _show_transient_prompt(message: String) -> void:
	_show_persistent_prompt(message)
	_prompt_tween = create_tween()
	_prompt_tween.tween_interval(TRANSIENT_PROMPT_DURATION)
	_prompt_tween.tween_property(prompt_label, ^"modulate:a", 0.0, 0.25)
	_prompt_tween.tween_callback(prompt_label.hide)


func _hide_prompt() -> void:
	if _prompt_tween != null:
		_prompt_tween.kill()
	prompt_label.hide()


func _show_error_popup(result: Dictionary) -> void:
	_show_error_popup_message(GameSessionManager.get_error_message(result))


func _show_error_popup_message(message: String) -> void:
	error_dialog.title = tr("BATTLE_ERROR_TITLE")
	error_dialog.dialog_text = message
	error_dialog.ok_button_text = tr("UI_OK")
	error_dialog.popup_centered()


func _show_battle_outcome_overlay(outcome: Dictionary) -> void:
	var overlay: BattleOutcomeOverlay = BATTLE_OUTCOME_OVERLAY_SCENE.instantiate()
	add_child(overlay)
	overlay.return_to_menu_requested.connect(_on_return_to_menu_requested)
	overlay.show_outcome(outcome)


func _on_return_to_menu_requested() -> void:
	GameSessionManager.end_game()
	get_tree().change_scene_to_file("res://scenes/main_menu/main_menu.tscn")
