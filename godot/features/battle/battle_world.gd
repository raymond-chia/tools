extends Node2D

signal primary_clicked(unit_id: String, cell: Vector2i)
signal inspection_clicked(unit_id: String, cell: Vector2i)
signal move_preview_changed(total_cost, pointer_position: Vector2)

const TILE_SIZE := Vector2i(64, 32)
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
var state: Dictionary = {}
var pending_action := ""
var inspected_cell := Vector2i(-1, -1)
var hovered := Vector2i(-1, -1)
var first_move_path: Array = []
var second_move_path: Array = []
var move_preview_interrupted := false
var move_preview_total_cost = null
var core
var unit_nodes := {}

func setup_map(snapshot: Dictionary) -> void:
	state = snapshot
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

func present(snapshot: Dictionary, action: String, inspected: Vector2i, game_core) -> void:
	state = snapshot
	pending_action = action
	inspected_cell = inspected
	core = game_core
	sync_unit_sprites()
	update_move_preview()
	queue_redraw()

func _unhandled_input(event: InputEvent) -> void:
	if state.is_empty():
		return
	var local_event := make_input_local(event)
	if local_event is InputEventMouseMotion:
		var cell := point_to_cell(local_event.position)
		var next_hovered := cell if is_cell_on_board(cell) else Vector2i(-1, -1)
		if hovered != next_hovered:
			hovered = next_hovered
			update_move_preview()
			queue_redraw()
		else:
			move_preview_changed.emit(move_preview_total_cost, get_viewport().get_mouse_position())
		return
	if not local_event is InputEventMouseButton or not local_event.pressed:
		return
	var unit := unit_at_point(local_event.position)
	var cell := point_to_cell(local_event.position)
	if not unit.is_empty():
		cell = Vector2i(unit.x, unit.y)
	var unit_id: String = unit.id if not unit.is_empty() else ""
	if local_event.button_index == MOUSE_BUTTON_LEFT:
		primary_clicked.emit(unit_id, cell)
		get_viewport().set_input_as_handled()
	elif local_event.button_index == MOUSE_BUTTON_RIGHT:
		inspection_clicked.emit(unit_id, cell)
		get_viewport().set_input_as_handled()

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
		else:
			node = unit_nodes[unit.id]
		var center := footprint_center(unit)
		node.position = center
		node.z_index = int(center.y)
		var large: bool = unit.width > 1
		var base_scale := Vector2(1.7, 1.7) if large else Vector2.ONE
		var selection: Sprite2D = node.get_node("Selection")
		selection.scale = base_scale * 1.18
		selection.modulate = Color("ffe17a")
		selection.visible = unit_occupies_cell(unit, inspected_cell)
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

func unit_at_point(point: Vector2) -> Dictionary:
	var ordered: Array = state.units.duplicate()
	ordered.sort_custom(func(a,b): return footprint_center(a).y > footprint_center(b).y)
	for unit in ordered:
		var center := footprint_center(unit)
		var radius := Vector2(61, 20) if unit.width > 1 else Vector2(36, 12)
		var offset := point - center
		if offset.x * offset.x / (radius.x * radius.x) + offset.y * offset.y / (radius.y * radius.y) <= 1.0:
			return unit
	return {}

func unit_occupies_cell_id(unit_id: String, cell: Vector2i) -> bool:
	for unit in state.units:
		if unit.id == unit_id:
			return unit_occupies_cell(unit, cell)
	return false

func unit_occupies_cell(unit: Dictionary, cell: Vector2i) -> bool:
	return cell.x >= unit.x and cell.x < unit.x + unit.width and cell.y >= unit.y and cell.y < unit.y + unit.height

func is_cell_on_board(cell: Vector2i) -> bool:
	return not state.is_empty() and cell.x >= 0 and cell.y >= 0 and cell.x < state.width and cell.y < state.height

func selected_skill_range() -> Array:
	if pending_action == "":
		return []
	for skill_range in state.skill_ranges:
		if skill_range.id == pending_action:
			return skill_range.cells
	return []

func clear_move_preview() -> void:
	first_move_path.clear()
	second_move_path.clear()
	move_preview_interrupted = false
	move_preview_total_cost = null

func update_move_preview() -> void:
	clear_move_preview()
	if pending_action != "" or core == null or state.is_empty() or state.turn.actor == null or not is_cell_on_board(hovered):
		move_preview_changed.emit(move_preview_total_cost, get_viewport().get_mouse_position())
		return
	var preview = JSON.parse_string(core.preview_move(state.turn.actor, hovered.x, hovered.y))
	if preview.has("error"):
		move_preview_changed.emit(move_preview_total_cost, get_viewport().get_mouse_position())
		return
	first_move_path = preview.first
	second_move_path = preview.second
	move_preview_interrupted = preview.interrupted
	move_preview_total_cost = preview.total_cost
	move_preview_changed.emit(move_preview_total_cost, get_viewport().get_mouse_position())

func diamond(center: Vector2) -> PackedVector2Array:
	return PackedVector2Array([center+Vector2(0,-16),center+Vector2(32,0),center+Vector2(0,16),center+Vector2(-32,0)])

func draw_marker(cell: Vector2i, fill: Color, edge: Color, edge_width := 2.0) -> void:
	var points := diamond(cell_center(cell)); draw_colored_polygon(points,fill); points.append(points[0]); draw_polyline(points,edge,edge_width,true)

func draw_move_path(path: Array, color: Color) -> void:
	for index in range(1, path.size()):
		var from := cell_center(Vector2i(path[index - 1].x, path[index - 1].y))
		var to := cell_center(Vector2i(path[index].x, path[index].y))
		draw_dashed_line(from,to,color,3.0,8.0,true,true)

func draw_spikes(cell: Vector2i) -> void:
	var center := cell_center(cell)
	var spike_color := Color("d9d5ca")
	for offset_x in [-18.0, -6.0, 6.0, 18.0]:
		var base := center + Vector2(offset_x, 5.0)
		draw_colored_polygon(PackedVector2Array([base + Vector2(-5.0, 0.0), base + Vector2(5.0, 0.0), base + Vector2(0.0, -18.0)]), spike_color)

func _draw() -> void:
	if state.is_empty():
		return
	draw_string(UI_FONT,Vector2(26,44),"灰燼谷伏擊",HORIZONTAL_ALIGNMENT_LEFT,-1,36,Color("f2d49b"))
	if pending_action == "":
		for cell in state.second_reachable: draw_marker(Vector2i(cell.x,cell.y),Color(0.04,0.15,0.42,0.38),Color(0.12,0.32,0.72,0.9))
		for cell in state.reachable: draw_marker(Vector2i(cell.x,cell.y),Color(0.12,0.48,0.95,0.3),Color(0.3,0.68,1.0,0.92))
	for cell in selected_skill_range(): draw_marker(Vector2i(cell.x,cell.y),Color(0.72,0.12,0.04,0.38),Color(1.0,0.34,0.12,0.95),3.0)
	var first_path_color := Color("ff5b4d") if move_preview_interrupted else Color("9debff")
	var second_path_color := Color("ff5b4d") if move_preview_interrupted else Color("78a8ff")
	draw_move_path(first_move_path,first_path_color); draw_move_path(second_move_path,second_path_color)
	if move_preview_interrupted:
		var interrupted_path: Array = second_move_path if not second_move_path.is_empty() else first_move_path
		if not interrupted_path.is_empty():
			draw_marker(Vector2i(interrupted_path[-1].x,interrupted_path[-1].y),Color(0.9,0.05,0.02,0.34),Color("ff4938"),4.0)
	if is_cell_on_board(hovered): draw_marker(hovered,Color(1,1,1,0.08),Color(1,1,1,0.6))
	if is_cell_on_board(inspected_cell): draw_marker(inspected_cell,Color(1.0,0.88,0.48,0.16),Color("ffe17a"))
	for effect in state.terrain_effects:
		if effect.effect == "spikes":
			draw_spikes(Vector2i(effect.x, effect.y))
		elif effect.effect == "grease":
			var center := cell_center(Vector2i(effect.x, effect.y)); draw_set_transform(center,0,Vector2(1,0.5)); draw_circle(Vector2.ZERO,20,Color(0.6,0.3,0.85,0.72)); draw_set_transform(Vector2.ZERO)
	for unit in state.units:
		var center := footprint_center(unit)
		var width: float = 96 if unit.width > 1 else 60
		draw_rect(Rect2(center+Vector2(-width*0.5,20),Vector2(width,7)),Color("281e25"))
		draw_rect(Rect2(center+Vector2(-width*0.5,20),Vector2(width*float(unit.hp)/unit.max_hp,7)),Color("62d27c"))
