extends CanvasLayer

signal action_selected(action: String)
signal end_move_requested
signal end_turn_requested
signal inspection_closed

@onready var root: Control = $Root
@onready var info_panel: Panel = $Root/InfoPanel
@onready var unit_details: VBoxContainer = $Root/InfoPanel/Margin/Content/UnitDetails
@onready var unit_name: Label = $Root/InfoPanel/Margin/Content/UnitDetails/UnitName
@onready var detail_values := {
	"team": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/TeamValue,
	"hp": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/HpValue,
	"size": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/SizeValue,
	"movement": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/MovementValue,
	"initiative": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/InitiativeValue,
	"defense": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/DefenseValue,
	"attack": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/AttackValue,
	"power": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/PowerValue,
}
@onready var terrain_name: Label = $Root/InfoPanel/Margin/Content/TerrainRows/TerrainValue
@onready var terrain_cost: Label = $Root/InfoPanel/Margin/Content/TerrainRows/CostValue
@onready var terrain_effect: Label = $Root/InfoPanel/Margin/Content/TerrainRows/EffectValue
@onready var actor_name: Label = $Root/BottomBar/Margin/Layout/Actor/Name
@onready var movement: Label = $Root/BottomBar/Margin/Layout/Actor/Movement
@onready var status: Label = $Root/BottomBar/Margin/Layout/Actions/Status
@onready var action_buttons := {
	"melee_attack": $Root/BottomBar/Margin/Layout/Actions/Buttons/Melee,
	"ranged_attack": $Root/BottomBar/Margin/Layout/Actions/Buttons/Ranged,
	"power_strike": $Root/BottomBar/Margin/Layout/Actions/Buttons/PowerStrike,
	"aimed_shot": $Root/BottomBar/Margin/Layout/Actions/Buttons/AimedShot,
}
var dragging_info_panel := false
var drag_offset := Vector2.ZERO

func _ready() -> void:
	$Root/InfoPanel/Margin/Content/Header.gui_input.connect(_on_header_gui_input)
	$Root/InfoPanel/Margin/Content/Header/Close.pressed.connect(func(): inspection_closed.emit())
	for action in action_buttons:
		action_buttons[action].pressed.connect(_on_action_pressed.bind(action))
	$Root/BottomBar/Margin/Layout/EndMove.pressed.connect(func(): end_move_requested.emit())
	$Root/BottomBar/Margin/Layout/EndTurn.pressed.connect(func(): end_turn_requested.emit())

func present(snapshot: Dictionary, pending_action: String, inspected_cell: Vector2i, status_text: String) -> void:
	status.text = status_text
	if snapshot.is_empty():
		return
	var actor := unit_with_id(snapshot.units, snapshot.turn.actor)
	actor_name.text = actor.name if not actor.is_empty() else "—"
	movement.text = "剩餘移動 %s" % snapshot.turn.move_remaining
	for action in action_buttons:
		action_buttons[action].button_pressed = action == pending_action
	var inspected := is_cell_on_board(snapshot, inspected_cell)
	info_panel.visible = inspected
	if not inspected:
		return
	var unit := unit_at_cell(snapshot.units, inspected_cell)
	unit_details.visible = not unit.is_empty()
	if not unit.is_empty():
		unit_name.text = unit.name
		detail_values.team.text = "我方" if unit.team == "player" else "敵方"
		detail_values.hp.text = "%s / %s" % [unit.hp, unit.max_hp]
		detail_values.size.text = "大型" if unit.width > 1 or unit.height > 1 else "一般"
		detail_values.movement.text = str(unit.movement)
		detail_values.initiative.text = str(unit.initiative)
		detail_values.defense.text = "%s / %s" % [unit.dodge, unit.block]
		detail_values.attack.text = "%s / %s" % [unit.melee, unit.ranged]
		detail_values.power.text = "%s / %s" % [unit.damage, unit.range]
	var terrain := terrain_at(snapshot.terrain_cells, inspected_cell)
	var terrain_names := {"plain": "平地", "rough": "崎嶇地面", "grease": "油膩地面"}
	terrain_name.text = terrain_names[terrain.kind]
	terrain_cost.text = str(terrain.cost)
	terrain_effect.text = terrain.effect

func _on_action_pressed(action: String) -> void:
	action_selected.emit(action)

func _on_header_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		dragging_info_panel = event.pressed
		if dragging_info_panel:
			drag_offset = event.global_position - info_panel.position
		root.accept_event()
	elif event is InputEventMouseMotion and dragging_info_panel:
		var maximum := root.size - info_panel.size
		info_panel.position = (event.global_position - drag_offset).clamp(Vector2.ZERO, maximum)
		root.accept_event()

func unit_with_id(units: Array, id) -> Dictionary:
	for unit in units:
		if unit.id == id:
			return unit
	return {}

func unit_at_cell(units: Array, cell: Vector2i) -> Dictionary:
	for unit in units:
		if cell.x >= unit.x and cell.x < unit.x + unit.width and cell.y >= unit.y and cell.y < unit.y + unit.height:
			return unit
	return {}

func terrain_at(terrains: Array, cell: Vector2i) -> Dictionary:
	for terrain in terrains:
		if terrain.x == cell.x and terrain.y == cell.y:
			return terrain
	return {}

func is_cell_on_board(snapshot: Dictionary, cell: Vector2i) -> bool:
	return cell.x >= 0 and cell.y >= 0 and cell.x < snapshot.width and cell.y < snapshot.height
