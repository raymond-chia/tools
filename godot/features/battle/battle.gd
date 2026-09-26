extends Node

@onready var world = $World
@onready var ui = $UI

var core
var state: Dictionary = {}
var displayed_state: Dictionary = {}
var pending_display_log: Array = []
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
	var map_info := read_core_response(core.map_to_json(map_text))
	if map_info.is_empty():
		present()
		return
	battle_name = map_info.name
	update_battle_title()
	core.set_random_seed(randi())
	world.setup_map(state)
	send({"type": "start"})

func _on_language_changed() -> void:
	update_battle_title()
	ui.refresh_log_language()
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
	pending_display_log.append_array(value.log)
	status = ""
	present(true)
	advance_automatic_turn()
	return true

func advance_automatic_turn() -> void:
	if state.is_empty() or not state.turn.can_continue:
		return
	if world.is_presenting_combat_events():
		await world.combat_events_finished
	await get_tree().create_timer(BattleConfig.COMBAT_EVENT_PAUSE).timeout
	if is_inside_tree():
		send({"type": "continue"})

func read_core_response(response: String, report_error := true) -> Dictionary:
	var ignored_error_ids: Array = [] if report_error else BattleConfig.PREVIEW_IGNORED_ERROR_IDS
	return CoreResponse.read(response, show_error, ignored_error_ids)

func show_error(message: String) -> void:
	status = message
	ui.present_status(message)

func present(new_snapshot := false) -> void:
	world.present(state, pending_action, inspected_cell, core, new_snapshot)
	if not world.is_presenting_combat_events():
		show_current_state()
	ui.present(displayed_state, pending_action, inspected_cell, inspected_skill, status, selecting_delay)
	displayed_state.log = []

func _on_combat_events_finished() -> void:
	show_current_state()
	ui.present(displayed_state, pending_action, inspected_cell, inspected_skill, status, selecting_delay)
	displayed_state.log = []

func show_current_state() -> void:
	displayed_state = state.duplicate()
	displayed_state.log = pending_display_log
	pending_display_log = []

func input_is_locked() -> bool:
	return (not state.is_empty() and state.turn.can_continue) or world.is_presenting_combat_events()

func select_action(action: String) -> void:
	if input_is_locked():
		return
	selecting_delay = false
	pending_action = action
	status = "請選擇技能目標。"
	present()

func _on_primary_clicked(unit_id: int, cell: Vector2i) -> void:
	if input_is_locked():
		return
	if selecting_delay:
		return
	if pending_action != "":
		use_pending_action(cell)
		return
	if state.is_empty() or state.turn.actor == null or not world.is_cell_on_board(cell):
		return
	if state.battle_mode != "combat" and unit_id != 0:
		for unit in state.units:
			if unit.id == unit_id and unit.team == "player":
				send({"type": "select_unit", "actor": unit_id})
				return
	world.prepare_move_animation(state.turn.actor, cell)
	if not send({"type": "move", "actor": state.turn.actor, "x": cell.x, "y": cell.y}):
		world.cancel_move_animation()

func _on_inspection_clicked(unit_id: int, cell: Vector2i) -> void:
	if input_is_locked():
		return
	if pending_action != "" and (unit_id == 0 or world.unit_occupies_cell_id(unit_id, inspected_cell)):
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

func use_pending_action(cell: Vector2i) -> void:
	if not world.is_cell_on_board(cell):
		status = "請選擇地圖上的格子。"
		present()
		return
	var actor: int = state.turn.actor
	var succeeded := send({"type": "skill", "actor": actor, "x": cell.x, "y": cell.y, "skill": pending_action})
	if succeeded:
		pending_action = ""
		present()

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

func _on_delay_target_selected(unit_id: int) -> void:
	if input_is_locked():
		return
	if not selecting_delay or state.is_empty() or state.turn.actor == null:
		return
	var actor: int = state.turn.actor
	selecting_delay = false
	send({"type": "delay", "actor": actor, "after": unit_id})

func _on_turn_order_focus_requested(unit_id: int) -> void:
	if input_is_locked():
		return
	if not state.is_empty() and state.battle_mode != "combat":
		send({"type": "select_unit", "actor": unit_id})
	world.focus_unit(unit_id)
	if world.unit_id_at_cell(inspected_cell) == 0:
		return
	inspected_cell = world.unit_cell(unit_id)
	inspected_skill = ""
	present()
