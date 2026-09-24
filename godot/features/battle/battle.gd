extends Node

@onready var world = $World
@onready var ui = $UI

var core
var state: Dictionary = {}
var displayed_state: Dictionary = {}
var pending_action := ""
var inspected_cell := Vector2i(-1, -1)
var inspected_skill := ""
var selecting_delay := false
var status := "左鍵選擇與移動；右鍵查看單位或地面資訊。"
var battle_name := ""
var testing_battle := false
var back_button: Button

func _ready() -> void:
	world.primary_clicked.connect(_on_primary_clicked)
	world.inspection_clicked.connect(_on_inspection_clicked)
	world.move_preview_changed.connect(ui.present_move_cost)
	world.attack_preview_changed.connect(ui.present_attack_preview)
	world.combat_events_finished.connect(_on_combat_events_finished)
	world.read_core_response = read_core_response
	ui.action_selected.connect(select_action)
	ui.end_turn_requested.connect(_on_end_turn_requested)
	ui.delay_selection_requested.connect(_on_delay_selection_requested)
	ui.delay_target_selected.connect(_on_delay_target_selected)
	ui.turn_order_focus_requested.connect(_on_turn_order_focus_requested)
	ui.inspection_closed.connect(_close_inspection)
	ui.skill_inspection_requested.connect(_on_skill_inspection_requested)
	ui.language_changed.connect(_on_language_changed)
	core = TacticalGame.new()
	var testing := get_tree().root.has_meta("test_map")
	testing_battle = testing
	var definitions_text := ""
	var map_text := ""
	if testing:
		definitions_text = get_tree().root.get_meta("test_definitions")
		map_text = get_tree().root.get_meta("test_map")
		back_button = Button.new()
		back_button.text = tr("返回編輯器")
		back_button.position = Vector2(900, 20)
		back_button.pressed.connect(return_to_editor)
		ui.get_node("Root").add_child(back_button)
	else:
		var definitions_file := FileAccess.open("res://data/definitions.toml", FileAccess.READ)
		var map_file := FileAccess.open("res://data/maps/ash_valley.toml", FileAccess.READ)
		if definitions_file == null or map_file == null:
			show_error("無法讀取戰鬥資料：%s" % error_string(FileAccess.get_open_error()))
			present()
			return
		definitions_text = definitions_file.get_as_text()
		map_text = map_file.get_as_text()
	state = read_core_response(core.load_documents(definitions_text, map_text))
	if state.is_empty():
		present()
		return
	var map_info: Dictionary = JSON.parse_string(core.map_to_json(map_text))
	battle_name = map_info.name
	update_battle_title()
	core.set_random_seed(randi())
	world.setup_map(state)
	send({"type": "start"})

func _on_language_changed() -> void:
	update_battle_title()
	if back_button != null:
		back_button.text = tr("返回編輯器")
	ui.present(displayed_state, pending_action, inspected_cell, inspected_skill, status, selecting_delay)

func update_battle_title() -> void:
	$UI/Root/BattleTitle.text = (tr("測試：") if testing_battle else "") + tr(battle_name)

func return_to_editor() -> void:
	get_tree().root.remove_meta("test_definitions")
	get_tree().root.remove_meta("test_map")
	get_tree().change_scene_to_file("res://features/editor/editor.tscn")

func send(command: Dictionary) -> bool:
	var value := read_core_response(core.dispatch(JSON.stringify(command)))
	if value.is_empty():
		present()
		return false
	state = value
	status = ""
	present()
	advance_automatic_turn()
	return true

func advance_automatic_turn() -> void:
	if state.is_empty() or not state.turn.auto_step:
		return
	if world.is_presenting_combat_events():
		await world.combat_events_finished
	await get_tree().create_timer(BattleVisualConfig.COMBAT_EVENT_PAUSE).timeout
	if is_inside_tree():
		send({"type": "auto_step"})

func read_core_response(response: String, report_error := true) -> Dictionary:
	var value: Dictionary = JSON.parse_string(response)
	if value.has("error"):
		if report_error:
			show_error(value.error)
		return {}
	return value

func show_error(message: String) -> void:
	status = message
	ui.present_status(message)

func present() -> void:
	world.present(state, pending_action, inspected_cell, core)
	if not world.is_presenting_combat_events():
		displayed_state = state
	ui.present(displayed_state, pending_action, inspected_cell, inspected_skill, status, selecting_delay)

func _on_combat_events_finished() -> void:
	displayed_state = state
	ui.present(displayed_state, pending_action, inspected_cell, inspected_skill, status, selecting_delay)

func input_is_locked() -> bool:
	return (not state.is_empty() and state.turn.auto_step) or world.is_presenting_combat_events()

func select_action(action: String) -> void:
	if input_is_locked():
		return
	selecting_delay = false
	pending_action = action
	status = "請選擇施法格子。" if pending_action_targets_cell() else "請選擇技能目標。"
	present()

func _on_primary_clicked(unit_id: String, cell: Vector2i) -> void:
	if input_is_locked():
		return
	if selecting_delay:
		return
	if pending_action != "":
		if pending_action_targets_cell():
			use_pending_cell_action(cell)
			return
		if unit_id == "":
			status = "請選擇一個單位作為目標。"
			present()
			return
		use_pending_action(unit_id, cell)
		return
	if state.is_empty() or state.turn.actor == null or not world.is_cell_on_board(cell):
		return
	world.prepare_move_animation(state.turn.actor, cell)
	if not send({"type": "move", "actor": state.turn.actor, "x": cell.x, "y": cell.y}):
		world.cancel_move_animation()

func _on_inspection_clicked(unit_id: String, cell: Vector2i) -> void:
	if input_is_locked():
		return
	if pending_action != "" and (unit_id == "" or world.unit_occupies_cell_id(unit_id, inspected_cell)):
		pending_action = ""
		status = "已取消技能。"
		present()
		return
	if not world.is_cell_on_board(cell):
		return
	inspected_skill = ""
	inspected_cell = Vector2i(-1, -1) if inspected_cell == cell else cell
	present()

func _close_inspection() -> void:
	if input_is_locked():
		return
	inspected_cell = Vector2i(-1, -1)
	inspected_skill = ""
	present()

func _on_skill_inspection_requested(skill_id: String) -> void:
	if input_is_locked():
		return
	inspected_cell = Vector2i(-1, -1)
	inspected_skill = "" if inspected_skill == skill_id else skill_id
	present()

func use_pending_action(target: String, cell: Vector2i) -> void:
	var actor: String = state.turn.actor
	var succeeded := send({"type": "skill", "actor": actor, "target": target, "x": cell.x, "y": cell.y, "skill": pending_action})
	if succeeded:
		pending_action = ""
		present()

func use_pending_cell_action(cell: Vector2i) -> void:
	if not world.is_cell_on_board(cell):
		status = "請選擇地圖上的格子。"
		present()
		return
	var actor: String = state.turn.actor
	var succeeded := send({"type": "cell_skill", "actor": actor, "x": cell.x, "y": cell.y, "skill": pending_action})
	if succeeded:
		pending_action = ""
		present()

func pending_action_targets_cell() -> bool:
	for skill_range in state.skill_ranges:
		if skill_range.id == pending_action:
			return skill_range.cell_targeted
	return false

func _on_end_turn_requested() -> void:
	if input_is_locked():
		return
	if state.is_empty() or state.turn.actor == null:
		return
	pending_action = ""
	selecting_delay = false
	send({"type": "end_turn", "actor": state.turn.actor})

func _on_delay_selection_requested() -> void:
	if input_is_locked():
		return
	if state.is_empty() or not state.turn.can_delay:
		return
	pending_action = ""
	selecting_delay = not selecting_delay
	status = ""
	present()

func _on_delay_target_selected(unit_id: String) -> void:
	if input_is_locked():
		return
	if not selecting_delay or state.is_empty() or state.turn.actor == null:
		return
	var actor: String = state.turn.actor
	selecting_delay = false
	send({"type": "delay", "actor": actor, "after": unit_id})

func _on_turn_order_focus_requested(unit_id: String) -> void:
	if input_is_locked():
		return
	world.focus_unit(unit_id)
	if world.unit_id_at_cell(inspected_cell).is_empty():
		return
	inspected_cell = world.unit_cell(unit_id)
	inspected_skill = ""
	present()
