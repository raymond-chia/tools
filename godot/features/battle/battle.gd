extends Node

@onready var world = $World
@onready var ui = $UI

var core
var state: Dictionary = {}
var pending_action := ""
var inspected_cell := Vector2i(-1, -1)
var inspected_skill := ""
var status := "左鍵選擇與移動；右鍵查看單位或地面資訊。"

func _ready() -> void:
	world.primary_clicked.connect(_on_primary_clicked)
	world.inspection_clicked.connect(_on_inspection_clicked)
	world.move_preview_changed.connect(ui.present_move_cost)
	world.attack_preview_changed.connect(ui.present_attack_preview)
	world.read_core_response = read_core_response
	ui.action_selected.connect(select_action)
	ui.end_turn_requested.connect(_on_end_turn_requested)
	ui.inspection_closed.connect(_close_inspection)
	ui.skill_inspection_requested.connect(_on_skill_inspection_requested)
	core = TacticalGame.new()
	var file := FileAccess.open("res://data/vertical_slice.toml", FileAccess.READ)
	if file == null:
		show_error("無法讀取 TOML：%s" % error_string(FileAccess.get_open_error()))
		present()
		return
	state = read_core_response(core.load_definition(file.get_as_text()))
	if state.is_empty():
		present()
		return
	core.set_random_seed(randi())
	world.setup_map(state)
	send({"type": "start"})

func send(command: Dictionary) -> bool:
	var value := read_core_response(core.dispatch(JSON.stringify(command)))
	if value.is_empty():
		present()
		return false
	state = value
	status = ""
	present()
	return true

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
	ui.present(state, pending_action, inspected_cell, inspected_skill, status)

func select_action(action: String) -> void:
	pending_action = action
	status = "請選擇施法格子。" if pending_action_targets_cell() else "請選擇技能目標。"
	present()

func _on_primary_clicked(unit_id: String, cell: Vector2i) -> void:
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
	send({"type": "move", "actor": state.turn.actor, "x": cell.x, "y": cell.y})

func _on_inspection_clicked(unit_id: String, cell: Vector2i) -> void:
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
	inspected_cell = Vector2i(-1, -1)
	inspected_skill = ""
	present()

func _on_skill_inspection_requested(skill_id: String) -> void:
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
	if state.is_empty() or state.turn.actor == null:
		return
	pending_action = ""
	send({"type": "end_turn", "actor": state.turn.actor})
