extends Control

signal cell_pressed(cell: Vector2i)
signal cell_hovered(cell: Vector2i)

@onready var ground: TileMapLayer = $Ground
@onready var units_layer: Node2D = $Units
@onready var camera: Camera2D = get_node("../Camera2D")

var map_data: Dictionary = {}
var definitions: Dictionary = {}
var hovered := Vector2i(-1, -1)
var selected := Vector2i(-1, -1)
var dragging := false
var drag_paint := false
var last_painted := Vector2i(-1, -1)
var camera_bounds := Rect2()
var camera_initialized := false

func _ready() -> void:
	mouse_exited.connect(func():
		hovered = Vector2i(-1, -1)
		dragging = false
		queue_redraw())

func present(map_value: Dictionary, definitions_value: Dictionary, selected_cell: Vector2i) -> void:
	map_data = map_value
	definitions = definitions_value
	selected = selected_cell
	custom_minimum_size = Vector2((map_data.width + map_data.height) * 32 + 96, (map_data.width + map_data.height) * 16 + 140)
	ground.position = Vector2(int(map_data.height) * 32 + 32, 48)
	var terrain_cells: Array = []
	for y in int(map_data.height):
		for x in int(map_data.width):
			var base_kind := "plain"
			for terrain in map_data.terrains:
				if terrain.x == x and terrain.y == y and definitions.terrain_types.has(terrain.kind) and definitions.terrain_types[terrain.kind].visual == "rough":
					base_kind = "rough"
			terrain_cells.append({"x": x, "y": y, "base_kind": base_kind})
	BattleVisuals.setup_ground(ground, terrain_cells)
	update_camera_bounds()
	if not camera_initialized:
		camera.position = center(Vector2i(int(map_data.width / 2), int(map_data.height / 2)))
		camera_initialized = true
	else:
		camera.position = clamp_camera_position(camera.position)
	for child in units_layer.get_children(): child.queue_free()
	for unit in map_data.units:
		var kind := unit_type(unit.unit_type)
		if kind.is_empty(): continue
		var first := center(Vector2i(unit.x, unit.y))
		var last := center(Vector2i(unit.x + kind.get("width", 1) - 1, unit.y + kind.get("height", 1) - 1))
		var point := (first + last) * 0.5
		var large: bool = kind.get("width", 1) > 1 or kind.get("height", 1) > 1
		var node := BattleVisuals.create_unit_node(unit.id, kind.visual, units_layer)
		node.position = point
		node.z_index = int(point.y)
		BattleVisuals.style_unit_node(node, large, unit.team, false)
	queue_redraw()

func move_camera(delta: float) -> void:
	var movement := MapNavigation.movement(delta)
	if not movement.is_zero_approx():
		camera.position = clamp_camera_position(camera.position + movement)

func update_camera_bounds() -> void:
	var minimum := center(Vector2i.ZERO)
	var maximum := minimum
	for cell in [Vector2i(map_data.width - 1, 0), Vector2i(0, map_data.height - 1), Vector2i(map_data.width - 1, map_data.height - 1)]:
		var point := center(cell)
		minimum = minimum.min(point)
		maximum = maximum.max(point)
	camera_bounds = Rect2(minimum, maximum - minimum).grow(BattleConfig.CAMERA_BOUNDS_MARGIN)

func clamp_camera_position(position: Vector2) -> Vector2:
	return position.clamp(camera_bounds.position, camera_bounds.end)

func unit_type(id: String) -> Dictionary:
	for kind in definitions.unit_types:
		if kind.id == id: return kind
	return {}

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
	for terrain in map_data.terrains:
		var cell := Vector2i(terrain.x, terrain.y)
		if not definitions.terrain_types.has(terrain.kind): continue
		BattleVisuals.draw_terrain_effect(self, definitions.terrain_types[terrain.kind].visual, center(cell))
	if selected.x >= 0: marker(selected, Color(1, 0.88, 0.48, 0.16), Color("ffe17a"))
	if hovered.x >= 0: marker(hovered, Color(1, 1, 1, 0.1), Color(1, 1, 1, 0.75))
