extends CanvasLayer

signal action_selected(action: String)
signal end_turn_requested
signal inspection_closed

const TEAM_COLORS := {"player": "#63a9ff", "enemy": "#ff6868"}
const RESULT_STYLE := "[color=#f0c96a]%s[/color]"
const CRITICAL_STYLE := "[color=#ff7043]暴擊[/color]"
const MOVE_COST_POPUP_OFFSET := Vector2(16.0, 16.0)

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
@onready var battle_log: RichTextLabel = $Root/LogPanel/Margin/Content/Entries
@onready var move_cost_popup: PanelContainer = $Root/MoveCostPopup
@onready var move_cost_label: Label = $Root/MoveCostPopup/Label
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
	$Root/BottomBar/Margin/Layout/EndTurn.pressed.connect(func(): end_turn_requested.emit())

func present(snapshot: Dictionary, pending_action: String, inspected_cell: Vector2i, status_text: String) -> void:
	status.text = status_text
	if snapshot.is_empty():
		return
	var formatted_log := format_log(snapshot.log)
	if battle_log.text != formatted_log:
		battle_log.text = formatted_log
	var actor := unit_with_id(snapshot.units, snapshot.turn.actor)
	actor_name.text = actor.name if not actor.is_empty() else "—"
	movement.text = "剩餘移動 %d" % int(snapshot.turn.move_remaining)
	for action in action_buttons:
		action_buttons[action].button_pressed = action == pending_action
		action_buttons[action].disabled = not snapshot.turn.can_skill
	var inspected := is_cell_on_board(snapshot, inspected_cell)
	info_panel.visible = inspected
	if not inspected:
		return
	var unit := unit_at_cell(snapshot.units, inspected_cell)
	unit_details.visible = not unit.is_empty()
	if not unit.is_empty():
		unit_name.text = unit.name
		detail_values.team.text = "我方" if unit.team == "player" else "敵方"
		detail_values.hp.text = "%d / %d" % [int(unit.hp), int(unit.max_hp)]
		detail_values.size.text = "大型" if unit.width > 1 or unit.height > 1 else "一般"
		detail_values.movement.text = "%d" % int(unit.movement)
		detail_values.initiative.text = "%d" % int(unit.initiative)
		detail_values.defense.text = "%d / %d" % [int(unit.dodge), int(unit.block)]
		detail_values.attack.text = "%d / %d" % [int(unit.melee), int(unit.ranged)]
		detail_values.power.text = "%d / %d" % [int(unit.damage), int(unit.range)]
	var terrain := terrain_at(snapshot.terrain_cells, inspected_cell)
	var terrain_names := {"plain": "平地", "rough": "崎嶇地面", "grease": "油膩地面", "spikes": "地刺"}
	terrain_name.text = terrain_names[terrain.kind]
	terrain_cost.text = "%d" % int(terrain.cost)
	terrain_effect.text = "造成 %d 點傷害" % int(terrain.damage) if terrain.damage > 0 else terrain.effect

func present_move_cost(total_cost, pointer_position: Vector2) -> void:
	move_cost_popup.visible = total_cost != null
	if total_cost == null:
		return
	move_cost_label.text = "移動消耗 %d" % int(total_cost)
	move_cost_popup.reset_size()
	var viewport_rect: Rect2 = get_viewport().get_visible_rect()
	var popup_position: Vector2 = pointer_position + MOVE_COST_POPUP_OFFSET
	if popup_position.x + move_cost_popup.size.x > viewport_rect.end.x:
		popup_position.x = pointer_position.x - MOVE_COST_POPUP_OFFSET.x - move_cost_popup.size.x
	if popup_position.y + move_cost_popup.size.y > viewport_rect.end.y:
		popup_position.y = pointer_position.y - MOVE_COST_POPUP_OFFSET.y - move_cost_popup.size.y
	var maximum_position: Vector2 = viewport_rect.end - move_cost_popup.size
	move_cost_popup.position = popup_position.clamp(viewport_rect.position, maximum_position)

func format_log(events: Array) -> String:
	var entries: Array[String] = []
	for event in events:
		match event.type:
			"new_round":
				entries.append("── 第 %d 輪 ──" % int(event.round))
			"skill":
				entries.append("%s 使用「%s」影響 %s" % [colored_unit(event.actor, event.actor_team), event.skill, colored_unit(event.target, event.target_team)])
				entries.append("D20 擲骰 %d + 攻擊加值 %d = 攻擊總值 %d" % [int(event.roll), int(event.attack_modifier), int(event.attack_total)])
				entries.append("目標防禦：閃避門檻 %d／格擋門檻 %d" % [int(event.dodge_target), int(event.block_target)])
				var result_names := {"dodge": "閃避", "block": "格擋", "hit": "命中"}
				var result: String = RESULT_STYLE % result_names[event.result]
				var critical := "，%s" % CRITICAL_STYLE if event.critical else ""
				entries.append("結果：%s%s，%d 傷害，HP %d/%d" % [result, critical, int(event.damage), int(event.remaining_hp), int(event.max_hp)])
				if event.downed:
					entries.append("%s 倒下" % colored_unit(event.target, event.target_team))
			"status_applied":
				var status_names := {"grease": "油脂"}
				entries.append("%s 受到「%s」狀態影響" % [colored_unit(event.target, event.target_team), status_names[event.status]])
			"terrain_damage":
				var terrain_names := {"spikes": "地刺"}
				entries.append("%s 踩到「%s」，受到 %d 點傷害，HP %d/%d" % [colored_unit(event.target, event.target_team), terrain_names[event.terrain], int(event.damage), int(event.remaining_hp), int(event.max_hp)])
				if event.downed:
					entries.append("%s 倒下" % colored_unit(event.target, event.target_team))
	return "\n".join(entries)

func colored_unit(unit: String, team: String) -> String:
	return "[color=%s]%s[/color]" % [TEAM_COLORS[team], unit]

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
