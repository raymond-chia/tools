extends Control

signal cell_pressed(cell: Vector2i)
signal cell_hovered(cell: Vector2i)

@onready var ground: TileMapLayer = $Ground
@onready var units_layer: Node2D = $Units
@onready var camera: MapCamera = get_node("../Camera2D")

var map_data: Dictionary = {}
var snapshot: Dictionary = {}
var hovered := Vector2i(-1, -1)
var selected := Vector2i(-1, -1)
var dragging := false
var drag_paint := false
var last_painted := Vector2i(-1, -1)
var camera_initialized := false

func _ready() -> void:
	mouse_exited.connect(func():
		hovered = Vector2i(-1, -1)
		dragging = false
		queue_redraw())

func present(map_value: Dictionary, snapshot_value: Dictionary, selected_cell: Vector2i) -> void:
	map_data = map_value
	snapshot = snapshot_value
	selected = selected_cell
	custom_minimum_size = Vector2((map_data.width + map_data.height) * 32 + 96, (map_data.width + map_data.height) * 16 + 140)
	ground.position = Vector2(int(map_data.height) * 32 + 32, 48)
	BattleVisuals.setup_ground(ground, snapshot.get("terrain_cells", []))
	camera.configure_board(self, ground, map_data.width, map_data.height)
	if not camera_initialized:
		camera.center_on_board()
		camera_initialized = true
	for child in units_layer.get_children(): child.queue_free()
	for unit in snapshot.get("units", []):
		var point := BattleVisuals.footprint_center(ground, unit)
		var node := BattleVisuals.create_unit_node(unit.id, unit.visual, units_layer)
		node.position = point
		node.z_index = int(point.y)
		BattleVisuals.style_unit_node(node, unit.large, unit.team, false)
	queue_redraw()

func move_camera(delta: float) -> void:
	camera.move_with_input(delta)

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
		var cell := cell_at(event.position)
		if cell != hovered:
			hovered = cell
			cell_hovered.emit(cell)
			queue_redraw()
		if dragging and drag_paint and cell.x >= 0 and cell != last_painted:
			last_painted = cell
			cell_pressed.emit(cell)
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		dragging = event.pressed
		if dragging:
			var cell := cell_at(event.position)
			if cell.x >= 0:
				last_painted = cell
				cell_pressed.emit(cell)
		else:
			last_painted = Vector2i(-1, -1)

func diamond(point: Vector2) -> PackedVector2Array:
	return BattleVisuals.diamond(point)

func marker(cell: Vector2i, fill: Color, edge: Color) -> void:
	var points := diamond(center(cell))
	draw_colored_polygon(points, fill)
	points.append(points[0])
	draw_polyline(points, edge, 2.0, true)

func _draw() -> void:
	if map_data.is_empty(): return
	BattleVisuals.draw_terrain_effects(self, ground, snapshot.get("terrain_effects", []))
	BattleVisuals.draw_unit_health(self, ground, snapshot.get("units", []))
	if selected.x >= 0: marker(selected, Color(1, 0.88, 0.48, 0.16), Color("ffe17a"))
	if hovered.x >= 0: marker(hovered, Color(1, 1, 1, 0.1), Color(1, 1, 1, 0.75))
