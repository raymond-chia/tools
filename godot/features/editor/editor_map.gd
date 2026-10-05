extends Control

signal cell_pressed(cell: Vector2i)
signal inspection_clicked(cell: Vector2i)
signal cell_hovered(cell: Vector2i)
signal stroke_started()
signal unit_dropped(id: int, cell: Vector2i)

@onready var ground: TileMapLayer = $Ground
@onready var units_layer: Node2D = $Units
@onready var camera: MapCamera = get_node("../Camera2D")

var map_data: Dictionary = {}
var snapshot: Dictionary = {}
var hovered := Vector2i(-1, -1)
var inspected_unit := 0
var selected := Vector2i(-1, -1)
var dragging := false
var drag_paint := false
var last_painted := Vector2i(-1, -1)
var camera_initialized := false
var presented_units: Array = []
var unit_nodes: Dictionary = {}
var dragged_node: Node2D
var drag_origin := Vector2.ZERO
var drag_pointer_origin := Vector2.ZERO
var drag_visual_offset := Vector2.ZERO
var dragged_unit := 0
var drag_offset := Vector2i.ZERO

func _ready() -> void:
	mouse_exited.connect(func():
		hovered = Vector2i(-1, -1)
		finish_pointer(Vector2i(-1, -1))
		queue_redraw())

func present(map_value: Dictionary, snapshot_value: Dictionary, selected_cell: Vector2i) -> void:
	map_data = map_value
	snapshot = snapshot_value
	selected = selected_cell
	custom_minimum_size = Vector2((map_data.width + map_data.height) * 32 + 96, (map_data.width + map_data.height) * 16 + 140)
	# DIAMOND_RIGHT 向右上與右下延伸，保留邊距並將最高的格子放在 Control 範圍內。
	var ground_position := Vector2(16, (int(map_data.width) - 1) * 16 + 48)
	var ground_moved := ground.position != ground_position
	ground.position = ground_position
	if ground.tile_set == null:
		BattleVisuals.setup_ground(ground, snapshot.get("terrain_cells", []))
	else:
		BattleVisuals.paint_ground(ground, snapshot.get("terrain_cells", []))
	camera.configure_board(self, ground, map_data.width, map_data.height)
	if not camera_initialized:
		camera.center_on_board()
		camera_initialized = true
	var units: Array = snapshot.get("units", [])
	if ground_moved or units != presented_units:
		unit_nodes.clear()
		for child in units_layer.get_children(): child.queue_free()
		for unit in units:
			var point := BattleVisuals.footprint_center(ground, unit)
			var node := BattleVisuals.create_unit_node(unit.id, unit.visual, units_layer)
			unit_nodes[unit.id] = node
			node.position = point
		presented_units = units.duplicate(true)
	if not units.any(func(unit: Dictionary): return unit.id == inspected_unit):
		inspected_unit = 0
	for unit in units:
		BattleVisuals.style_unit_node(unit_nodes[unit.id], unit.large, unit.team, unit.id == inspected_unit)
		BattleVisuals.update_unit_health(unit_nodes[unit.id], unit)
	queue_redraw()

func move_camera(delta: float) -> void:
	camera.move_with_input(delta)
	if dragged_unit != 0: update_unit_drag(get_local_mouse_position())

func center(cell: Vector2i) -> Vector2:
	return ground.position + ground.map_to_local(cell)

func cell_at(point: Vector2) -> Vector2i:
	var cell := ground.local_to_map(point - ground.position)
	if cell.x >= 0 and cell.y >= 0 and cell.x < map_data.width and cell.y < map_data.height:
		return cell
	return Vector2i(-1, -1)

func _gui_input(event: InputEvent) -> void:
	if map_data.is_empty(): return
	if event is InputEventMouseMotion:
		if dragged_unit != 0: update_unit_drag(event.position)
		var cell := cell_at(event.position)
		if cell != hovered:
			hovered = cell
			cell_hovered.emit(cell)
			queue_redraw()
		if dragging and drag_paint and cell.x >= 0 and cell != last_painted:
			last_painted = cell
			cell_pressed.emit(cell)
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_RIGHT:
		if event.pressed:
			inspection_clicked.emit(cell_at(event.position))
		accept_event()
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		dragging = event.pressed
		if dragging:
			drag_pointer_origin = event.position
			stroke_started.emit()
			var cell := cell_at(event.position)
			if cell.x >= 0:
				last_painted = cell
				cell_pressed.emit(cell)
		else:
			var cell := cell_at(event.position) if Rect2(Vector2.ZERO, size).has_point(event.position) else Vector2i(-1, -1)
			finish_pointer(cell)

func begin_unit_drag(unit: Dictionary, cell: Vector2i) -> void:
	dragged_unit = unit.id
	dragged_node = unit_nodes[unit.id]
	drag_origin = dragged_node.position
	drag_visual_offset = Vector2.ZERO
	drag_offset = cell - Vector2i(unit.x, unit.y)
	selected = Vector2i(unit.x, unit.y)
	queue_redraw()

# 這只是拖曳中的顯示位移，不修改 snapshot，也不推導移動是否合法。
func update_unit_drag(point: Vector2) -> void:
	drag_visual_offset = point - drag_pointer_origin
	dragged_node.position = drag_origin + drag_visual_offset
	queue_redraw()

func finish_pointer(cell: Vector2i) -> void:
	var id := dragged_unit
	if id != 0: dragged_node.position = drag_origin
	dragged_node = null
	drag_visual_offset = Vector2.ZERO
	dragged_unit = 0
	dragging = false
	last_painted = Vector2i(-1, -1)
	selected = Vector2i(-1, -1)
	if id != 0 and cell.x >= 0:
		unit_dropped.emit(id, cell - drag_offset)
	queue_redraw()

func diamond(point: Vector2) -> PackedVector2Array:
	return BattleVisuals.diamond(point)

func marker(cell: Vector2i, fill: Color, edge: Color) -> void:
	var points := diamond(center(cell))
	draw_colored_polygon(points, fill)
	points.append(points[0])
	draw_polyline(points, edge, 2.0, true)

func _draw() -> void:
	if map_data.is_empty(): return
	BattleVisuals.draw_movement_range(self, ground, snapshot.get("inspected_reachable", []), snapshot.get("inspected_second_reachable", []), true)
	BattleVisuals.draw_terrain_effects(self, ground, snapshot.get("terrain_effects", []))
	if selected.x >= 0: marker(selected, Color(1, 0.88, 0.48, 0.16), Color("ffe17a"))
	if dragged_unit != 0 and hovered.x >= 0:
		marker(hovered - drag_offset, Color(1, 0.88, 0.48, 0.25), Color("ffe17a"))
	if hovered.x >= 0: marker(hovered, Color(1, 1, 1, 0.1), Color(1, 1, 1, 0.75))
