extends Control

const TILE_SIZE := Vector2i(64, 32)
const PANEL_X := 940.0
const BOTTOM_Y := 570.0
const CLOSE_INFO_RECT := Rect2(1220, 18, 38, 38)
const MELEE_RECT := Rect2(300, 592, 145, 42)
const RANGED_RECT := Rect2(455, 592, 145, 42)
const POWER_STRIKE_RECT := Rect2(610, 592, 145, 42)
const AIMED_SHOT_RECT := Rect2(765, 592, 145, 42)
const END_MOVE_RECT := Rect2(940, 592, 145, 42)
const END_TURN_RECT := Rect2(1100, 652, 150, 48)
const UI_FONT := preload("res://assets/fonts/NotoSans.ttf")
const GROUND_ART := preload("res://assets/tiles/isometric_ground.svg")
const BASE_ART := preload("res://assets/units/faction_base.svg")
const UNIT_ART := {
	"aria": preload("res://assets/units/fighter.svg"), "lyra": preload("res://assets/units/archer.svg"),
	"wolf_a": preload("res://assets/units/wolf.svg"), "wolf_b": preload("res://assets/units/wolf.svg"),
	"ogre": preload("res://assets/units/ogre.svg"),
}

@onready var ground: TileMapLayer = $Ground
@onready var units_layer: Node2D = $Units
var core
var state: Dictionary = {}
var pending_action := ""
var inspected_cell := Vector2i(-1, -1)
var hovered := Vector2i(-1, -1)
var status := "左鍵選擇與移動；右鍵查看單位或地面資訊。"
var unit_nodes := {}

func _ready() -> void:
	core = TacticalGame.new()
	var file := FileAccess.open("res://data/vertical_slice.toml", FileAccess.READ)
	if file == null: status = "無法讀取 TOML"; queue_redraw(); return
	state = JSON.parse_string(core.load_definition(file.get_as_text()))
	setup_native_tilemap()
	send({"type":"start"})

func setup_native_tilemap() -> void:
	var tiles := TileSet.new()
	tiles.tile_size = TILE_SIZE
	tiles.tile_shape = TileSet.TILE_SHAPE_ISOMETRIC
	tiles.tile_layout = TileSet.TILE_LAYOUT_DIAMOND_RIGHT
	var atlas := TileSetAtlasSource.new()
	atlas.texture = GROUND_ART
	atlas.texture_region_size = TILE_SIZE
	atlas.create_tile(Vector2i(0, 0))
	atlas.create_tile(Vector2i(1, 0))
	tiles.add_source(atlas, 0)
	ground.tile_set = tiles
	for y in state.height:
		for x in state.width:
			var atlas_cell := Vector2i(1, 0) if state.costs[y * state.width + x] > 1 else Vector2i.ZERO
			ground.set_cell(Vector2i(x, y), 0, atlas_cell)

func send(command: Dictionary) -> bool:
	var value = JSON.parse_string(core.dispatch(JSON.stringify(command)))
	if value.has("error"):
		status = value.error
		queue_redraw()
		return false
	state = value; status = ""; sync_unit_sprites()
	queue_redraw()
	return true

func cell_center(cell: Vector2i) -> Vector2:
	return ground.position + ground.map_to_local(cell)

func point_to_cell(point: Vector2) -> Vector2i:
	return ground.local_to_map(point - ground.position)

func sync_unit_sprites() -> void:
	for unit in state.units:
		var node: Node2D
		if not unit_nodes.has(unit.id):
			node = Node2D.new()
			node.name = unit.id
			var selection := Sprite2D.new(); selection.name = "Selection"; selection.texture = BASE_ART; node.add_child(selection)
			var base := Sprite2D.new(); base.name = "Base"; base.texture = BASE_ART; node.add_child(base)
			var body := Sprite2D.new(); body.name = "Body"; body.texture = UNIT_ART[unit.id]; node.add_child(body)
			units_layer.add_child(node)
			unit_nodes[unit.id] = node
		else: node = unit_nodes[unit.id]
		var center := footprint_center(unit)
		node.position = center
		node.z_index = int(center.y)
		var large: bool = unit.width > 1
		var base_scale := Vector2(1.7, 1.7) if large else Vector2.ONE
		var selection: Sprite2D = node.get_node("Selection")
		selection.scale = base_scale * 1.18
		selection.modulate = Color("ffe17a")
		selection.visible = is_inspecting() and unit_occupies_cell(unit, inspected_cell)
		node.get_node("Base").scale = base_scale
		node.get_node("Base").modulate = Color("63a9ff") if unit.team == "player" else Color("ff6868")
		var body: Sprite2D = node.get_node("Body")
		body.position.y = -55 if large else -43
		body.scale = Vector2(0.88, 0.88) if large else Vector2(0.72, 0.72)
		body.modulate = Color(0.45,0.45,0.48,0.75) if unit.downed else Color.WHITE

func footprint_center(unit: Dictionary) -> Vector2:
	var first := cell_center(Vector2i(unit.x, unit.y))
	var last := cell_center(Vector2i(unit.x + unit.width - 1, unit.y + unit.height - 1))
	return (first + last) * 0.5

func unit_base_has_point(unit: Dictionary, point: Vector2) -> bool:
	var center := footprint_center(unit)
	var radius := Vector2(61, 20) if unit.width > 1 else Vector2(36, 12)
	var offset := point - center
	return offset.x * offset.x / (radius.x * radius.x) + offset.y * offset.y / (radius.y * radius.y) <= 1.0

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		hovered = point_to_cell(event.position) if event.position.x < PANEL_X and event.position.y < BOTTOM_Y else Vector2i(-1,-1)
		queue_redraw(); return
	if not event is InputEventMouseButton or not event.pressed or state.is_empty(): return
	var mouse: Vector2 = event.position
	if event.button_index == MOUSE_BUTTON_RIGHT:
		if pending_action != "":
			pending_action = ""
			status = "已取消技能。"
			queue_redraw()
		else:
			inspect_at(mouse)
		accept_event()
		return
	if event.button_index != MOUSE_BUTTON_LEFT: return
	if CLOSE_INFO_RECT.has_point(mouse) and is_inspecting():
		inspected_cell = Vector2i(-1, -1)
		sync_unit_sprites()
		queue_redraw()
		accept_event()
		return
	if mouse.y >= BOTTOM_Y:
		handle_buttons(mouse)
		return
	if mouse.x >= PANEL_X: return
	var ordered: Array = state.units.duplicate()
	ordered.sort_custom(func(a,b): return footprint_center(a).y > footprint_center(b).y)
	for unit in ordered:
		if unit_base_has_point(unit, mouse):
			if pending_action != "": use_pending_action(unit.id)
			return
	if pending_action != "":
		status = "請選擇一個單位作為目標。"
		queue_redraw()
		return
	var cell := point_to_cell(mouse)
	if state.turn.actor != null and cell.x >= 0 and cell.y >= 0 and cell.x < state.width and cell.y < state.height:
		send({"type":"move","actor":state.turn.actor,"x":cell.x,"y":cell.y})

func inspect_at(mouse: Vector2) -> void:
	if mouse.x >= PANEL_X or mouse.y >= BOTTOM_Y: return
	var cell := point_to_cell(mouse)
	var ordered: Array = state.units.duplicate()
	ordered.sort_custom(func(a,b): return footprint_center(a).y > footprint_center(b).y)
	for unit in ordered:
		if unit_base_has_point(unit, mouse):
			cell = Vector2i(unit.x, unit.y)
			break
	if not is_cell_on_board(cell): return
	if inspected_cell == cell:
		inspected_cell = Vector2i(-1, -1)
	else:
		inspected_cell = cell
	sync_unit_sprites()
	queue_redraw()

func is_cell_on_board(cell: Vector2i) -> bool:
	return cell.x >= 0 and cell.y >= 0 and cell.x < state.width and cell.y < state.height

func is_inspecting() -> bool:
	return is_cell_on_board(inspected_cell)

func handle_buttons(mouse: Vector2) -> void:
	if state.turn.actor == null: return
	var actor: String = state.turn.actor
	if MELEE_RECT.has_point(mouse): select_action("melee")
	elif RANGED_RECT.has_point(mouse): select_action("ranged")
	elif POWER_STRIKE_RECT.has_point(mouse): select_action("power_strike")
	elif AIMED_SHOT_RECT.has_point(mouse): select_action("aimed_shot")
	elif END_MOVE_RECT.has_point(mouse): pending_action = ""; send({"type":"end_move","actor":actor})
	elif END_TURN_RECT.has_point(mouse): pending_action = ""; send({"type":"end_turn","actor":actor})

func select_action(action: String) -> void:
	pending_action = action
	status = "請選擇技能目標。"
	queue_redraw()

func use_pending_action(target: String) -> void:
	var actor: String = state.turn.actor
	var succeeded := false
	if pending_action == "melee": succeeded = send({"type":"attack","actor":actor,"target":target,"ranged":false})
	elif pending_action == "ranged": succeeded = send({"type":"attack","actor":actor,"target":target,"ranged":true})
	elif pending_action == "power_strike": succeeded = send({"type":"skill","actor":actor,"target":target,"skill":"power_strike"})
	elif pending_action == "aimed_shot": succeeded = send({"type":"skill","actor":actor,"target":target,"skill":"aimed_shot"})
	if succeeded:
		pending_action = ""
		queue_redraw()

func diamond(center: Vector2) -> PackedVector2Array:
	return PackedVector2Array([center+Vector2(0,-16),center+Vector2(32,0),center+Vector2(0,16),center+Vector2(-32,0)])

func draw_marker(cell: Vector2i, fill: Color, edge: Color) -> void:
	var points := diamond(cell_center(cell)); draw_colored_polygon(points,fill); points.append(points[0]); draw_polyline(points,edge,2,true)

func _draw() -> void:
	if state.is_empty(): draw_string(UI_FONT,Vector2(30,50),status,HORIZONTAL_ALIGNMENT_LEFT,-1,20); return
	draw_string(UI_FONT,Vector2(26,44),"灰燼谷伏擊",HORIZONTAL_ALIGNMENT_LEFT,-1,36,Color("f2d49b"))
	for p in state.reachable: draw_marker(Vector2i(p.x,p.y),Color(0.22,0.9,0.57,0.22),Color(0.35,1,0.69,0.7))
	if hovered.x >= 0 and hovered.y >= 0 and hovered.x < state.width and hovered.y < state.height: draw_marker(hovered,Color(1,1,1,0.08),Color(1,1,1,0.6))
	if is_inspecting(): draw_marker(inspected_cell,Color(1.0,0.88,0.48,0.16),Color("ffe17a"))
	var grease := cell_center(Vector2i(4,4)); draw_set_transform(grease,0,Vector2(1,0.5)); draw_circle(Vector2.ZERO,20,Color(0.6,0.3,0.85,0.72)); draw_set_transform(Vector2.ZERO)
	for unit in state.units:
		var center := footprint_center(unit)
		var width: float = 96 if unit.width > 1 else 60
		draw_rect(Rect2(center+Vector2(-width*0.5,20),Vector2(width,7)),Color("281e25"))
		draw_rect(Rect2(center+Vector2(-width*0.5,20),Vector2(width*float(unit.hp)/unit.max_hp,7)),Color("62d27c"))
	draw_bottom_bar()
	draw_info_panel()

func draw_bottom_bar() -> void:
	draw_rect(Rect2(0,BOTTOM_Y,size.x,size.y-BOTTOM_Y),Color("172333")); draw_line(Vector2(0,BOTTOM_Y),Vector2(size.x,BOTTOM_Y),Color("4c6179"),2)
	var actor := "—"
	for unit in state.units:
		if unit.id == state.turn.actor: actor = unit.name
	draw_string(UI_FONT,Vector2(26,605),"行動角色",HORIZONTAL_ALIGNMENT_LEFT,250,17,Color("8fa7c0"))
	draw_string(UI_FONT,Vector2(26,639),actor,HORIZONTAL_ALIGNMENT_LEFT,250,27)
	draw_string(UI_FONT,Vector2(26,672),"剩餘移動 %s" % state.turn.move_remaining,HORIZONTAL_ALIGNMENT_LEFT,250,18,Color("b6c6d8"))
	draw_string(UI_FONT,Vector2(300,579),"攻擊與技能",HORIZONTAL_ALIGNMENT_LEFT,610,16,Color("8fa7c0"))
	action_button(MELEE_RECT,"近戰攻擊","melee"); action_button(RANGED_RECT,"遠程攻擊","ranged"); action_button(POWER_STRIKE_RECT,"強力一擊","power_strike"); action_button(AIMED_SHOT_RECT,"瞄準射擊","aimed_shot")
	button(END_MOVE_RECT,"結束移動"); button(END_TURN_RECT,"結束回合")
	draw_string(UI_FONT,Vector2(300,680),status,HORIZONTAL_ALIGNMENT_LEFT,770,17,Color("f1c982"))

func draw_info_panel() -> void:
	if not is_inspecting(): return
	draw_rect(Rect2(PANEL_X,0,size.x-PANEL_X,BOTTOM_Y),Color("1d2939")); draw_line(Vector2(PANEL_X,0),Vector2(PANEL_X,BOTTOM_Y),Color("4c6179"),2)
	draw_rect(CLOSE_INFO_RECT,Color("304965")); draw_rect(CLOSE_INFO_RECT,Color("7291ad"),false,1.5)
	draw_string(UI_FONT,CLOSE_INFO_RECT.position+Vector2(11,28),"×",HORIZONTAL_ALIGNMENT_LEFT,-1,24)
	var unit := unit_at_cell(inspected_cell)
	var y := 54.0
	if not unit.is_empty():
		draw_string(UI_FONT,Vector2(970,y),unit.name,HORIZONTAL_ALIGNMENT_LEFT,230,28,Color("f2d49b")); y += 38
		draw_info_line("陣營", "我方" if unit.team == "player" else "敵方", y); y += 28
		draw_info_line("生命", "%s / %s" % [unit.hp,unit.max_hp], y); y += 28
		draw_info_line("體型", "大型" if unit.width > 1 or unit.height > 1 else "一般", y); y += 28
		draw_info_line("移動", unit.movement, y); y += 28
		draw_info_line("先攻", unit.initiative, y); y += 28
		draw_info_line("閃避／格擋", "%s / %s" % [unit.dodge,unit.block], y); y += 28
		draw_info_line("近戰／遠程", "%s / %s" % [unit.melee,unit.ranged], y); y += 28
		draw_info_line("傷害／射程", "%s / %s" % [unit.damage,unit.range], y); y += 42
	draw_string(UI_FONT,Vector2(970,y),"地面",HORIZONTAL_ALIGNMENT_LEFT,260,21,Color("8fa7c0")); y += 30
	draw_info_line("地形", terrain_name(inspected_cell), y); y += 28
	draw_info_line("移動消耗", terrain_cost(inspected_cell), y); y += 28
	var effect := terrain_effect(inspected_cell)
	if effect != "": draw_info_line("地面效果", effect, y)

func draw_info_line(label: String, value, y: float) -> void:
	draw_string(UI_FONT,Vector2(970,y),label,HORIZONTAL_ALIGNMENT_LEFT,120,17,Color("8fa7c0"))
	draw_string(UI_FONT,Vector2(1090,y),str(value),HORIZONTAL_ALIGNMENT_LEFT,160,18,Color("dce6f0"))

func unit_at_cell(cell: Vector2i) -> Dictionary:
	for unit in state.units:
		if unit_occupies_cell(unit, cell):
			return unit
	return {}

func unit_occupies_cell(unit: Dictionary, cell: Vector2i) -> bool:
	return cell.x >= unit.x and cell.x < unit.x + unit.width and cell.y >= unit.y and cell.y < unit.y + unit.height

func terrain_cost(cell: Vector2i) -> int:
	return state.costs[cell.y * state.width + cell.x]

func terrain_effect(cell: Vector2i) -> String:
	for effect in state.terrain_effects:
		if effect.x == cell.x and effect.y == cell.y: return effect.effect
	return ""

func terrain_name(cell: Vector2i) -> String:
	if terrain_effect(cell) == "grease": return "油膩地面"
	if terrain_cost(cell) > 1: return "崎嶇地面"
	return "平地"

func button(rect: Rect2, label: String) -> void:
	draw_rect(rect,Color("304965")); draw_rect(rect,Color("7291ad"),false,1.5); draw_string(UI_FONT,rect.position+Vector2(18,29),label,HORIZONTAL_ALIGNMENT_LEFT,-1,21)

func action_button(rect: Rect2, label: String, action: String) -> void:
	button(rect, label)
	if pending_action == action: draw_rect(rect,Color("ffe17a"),false,3.0)
