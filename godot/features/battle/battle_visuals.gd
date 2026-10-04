class_name BattleVisuals
extends RefCounted

const TILE_SIZE := Vector2i(64, 32)
const GROUND_ART := preload("res://assets/tiles/isometric_ground.svg")
const BASE_ART := preload("res://assets/units/faction_base.svg")
# ground 地形 ID 對應到 GROUND_ART 內的地磚座標。
const GROUND_TILES := {"plain": Vector2i(0, 0), "rough": Vector2i(1, 0), "cliff": Vector2i(2, 0), "chasm": Vector2i(3, 0)}

static func setup_ground(ground: TileMapLayer, terrain_cells: Array) -> void:
	var tiles := TileSet.new()
	tiles.tile_size = TILE_SIZE
	tiles.tile_shape = TileSet.TILE_SHAPE_ISOMETRIC
	tiles.tile_layout = TileSet.TILE_LAYOUT_DIAMOND_RIGHT
	var atlas := TileSetAtlasSource.new()
	atlas.texture = GROUND_ART
	atlas.texture_region_size = TILE_SIZE
	for atlas_cell in GROUND_TILES.values():
		atlas.create_tile(atlas_cell)
	tiles.add_source(atlas, 0)
	ground.tile_set = tiles
	paint_ground(ground, terrain_cells)

static func paint_ground(ground: TileMapLayer, terrain_cells: Array) -> void:
	ground.clear()
	for terrain in terrain_cells:
		var atlas_cell: Vector2i = GROUND_TILES.get(terrain.ground_visual, GROUND_TILES.plain)
		ground.set_cell(Vector2i(terrain.x, terrain.y), 0, atlas_cell)

static func create_unit_node(unit_id: int, visual_name: String, units_layer: Node2D) -> Node2D:
	var node := Node2D.new()
	node.name = str(unit_id)
	var visual := Node2D.new()
	visual.name = "Visual"
	node.add_child(visual)
	var attack_preview_ring := Sprite2D.new()
	attack_preview_ring.name = "AttackPreviewRing"
	attack_preview_ring.texture = BASE_ART
	attack_preview_ring.visible = false
	visual.add_child(attack_preview_ring)
	var selection := Sprite2D.new()
	selection.name = "Selection"
	selection.texture = BASE_ART
	visual.add_child(selection)
	var base := Sprite2D.new()
	base.name = "Base"
	base.texture = BASE_ART
	visual.add_child(base)
	var body := Sprite2D.new()
	body.name = "Body"
	body.texture = load(BattleConfig.unit_art_path(visual_name))
	visual.add_child(body)
	var health := Node2D.new()
	health.name = "Health"
	health.position.y = 20
	for bar_name in ["Background", "Fill"]:
		var bar := Polygon2D.new()
		bar.name = bar_name
		bar.color = Color("281e25") if bar_name == "Background" else Color("62d27c")
		health.add_child(bar)
	node.add_child(health)
	units_layer.add_child(node)
	return node

static func style_unit_node(node: Node2D, large: bool, team: Variant, selected: bool) -> void:
	var base_scale := Vector2(1.7, 1.7) if large else Vector2.ONE
	var attack_preview_ring: Sprite2D = node.get_node("Visual/AttackPreviewRing")
	attack_preview_ring.scale = base_scale * BattleConfig.ATTACK_PREVIEW_RING_SCALE
	attack_preview_ring.modulate = Color("ffe17a")
	var selection: Sprite2D = node.get_node("Visual/Selection")
	selection.scale = base_scale * 1.18
	selection.modulate = Color("ffe17a")
	selection.visible = selected
	var base: Sprite2D = node.get_node("Visual/Base")
	base.scale = base_scale
	base.modulate = Color("63a9ff") if team is String else Color("ff6868")
	var body: Sprite2D = node.get_node("Visual/Body")
	body.position.y = -55 if large else -43
	body.scale = Vector2(0.88, 0.88) if large else Vector2(0.72, 0.72)
	body.modulate = Color.WHITE

static func update_unit_health(node: Node2D, unit: Dictionary) -> void:
	var width: float = 96 if unit.large else 60
	var health: Node2D = node.get_node("Health")
	var background: Polygon2D = health.get_node("Background")
	var fill: Polygon2D = health.get_node("Fill")
	background.polygon = health_bar_polygon(width, width)
	fill.visible = unit.hp > 0
	fill.polygon = health_bar_polygon(width, width * float(unit.hp) / float(unit.max_hp))

static func health_bar_polygon(width: float, filled_width: float) -> PackedVector2Array:
	var left := -width * 0.5
	return PackedVector2Array([Vector2(left, 0), Vector2(left + filled_width, 0), Vector2(left + filled_width, 7), Vector2(left, 7)])

static func diamond(center: Vector2) -> PackedVector2Array:
	return PackedVector2Array([center + Vector2(0, -16), center + Vector2(32, 0), center + Vector2(0, 16), center + Vector2(-32, 0)])

static func footprint_center(ground: TileMapLayer, unit: Dictionary) -> Vector2:
	var first := ground.position + ground.map_to_local(Vector2i(unit.x, unit.y))
	var last_cell: Dictionary = unit.occupied_cells[-1]
	var last := ground.position + ground.map_to_local(Vector2i(last_cell.x, last_cell.y))
	return (first + last) * 0.5

static func draw_terrain_effects(canvas: CanvasItem, ground: TileMapLayer, effects: Array) -> void:
	for effect in effects:
		var center := ground.position + ground.map_to_local(Vector2i(effect.x, effect.y))
		draw_terrain_effect(canvas, effect.visual, center)

static func draw_unit_health(canvas: CanvasItem, ground: TileMapLayer, units: Array) -> void:
	for unit in units:
		var center := footprint_center(ground, unit)
		var width: float = 96 if unit.large else 60
		canvas.draw_rect(Rect2(center + Vector2(-width * 0.5, 20), Vector2(width, 7)), Color("281e25"))
		canvas.draw_rect(Rect2(center + Vector2(-width * 0.5, 20), Vector2(width * float(unit.hp) / float(unit.max_hp), 7)), Color("62d27c"))

static func draw_terrain_effect(canvas: CanvasItem, visual: String, center: Vector2) -> void:
	match visual:
		"spikes":
			for offset_x in [-18.0, -6.0, 6.0, 18.0]:
				var base := center + Vector2(offset_x, 5.0)
				canvas.draw_colored_polygon(PackedVector2Array([base + Vector2(-5.0, 0.0), base + Vector2(5.0, 0.0), base + Vector2(0.0, -18.0)]), Color("d9d5ca"))
		"mire":
			canvas.draw_set_transform(center, 0, Vector2(1, 0.5))
			canvas.draw_circle(Vector2.ZERO, 23, Color(0.2, 0.55, 0.28, 0.76))
			canvas.draw_circle(Vector2(-9, 1), 5, Color(0.58, 0.86, 0.38, 0.72))
			canvas.draw_set_transform(Vector2.ZERO)

static func draw_movement_range(canvas: CanvasItem, ground: TileMapLayer, reachable: Array, second_reachable: Array, selected: bool) -> void:
	var first_color: Color = BattleConfig.SELECTED_MOVE_FIRST_COLOR if selected else BattleConfig.FIRST_MOVE_COLOR
	var second_color: Color = BattleConfig.SELECTED_MOVE_SECOND_COLOR if selected else BattleConfig.SECOND_MOVE_COLOR
	for cell in second_reachable:
		draw_movement_marker(canvas, ground, Vector2i(cell.x, cell.y), second_color, selected)
	for cell in reachable:
		draw_movement_marker(canvas, ground, Vector2i(cell.x, cell.y), first_color, selected)

static func draw_movement_marker(canvas: CanvasItem, ground: TileMapLayer, cell: Vector2i, color: Color, selected: bool) -> void:
	if not selected:
		var points := diamond(ground.position + ground.map_to_local(cell))
		canvas.draw_colored_polygon(points, Color(color, BattleConfig.MOVE_FILL_ALPHA))
		points.append(points[0])
		canvas.draw_polyline(points, Color(color, BattleConfig.MOVE_EDGE_ALPHA), BattleConfig.MOVE_RANGE_EDGE_WIDTH, true)
		return
	var center := ground.position + ground.map_to_local(cell)
	var points := diamond(center)
	var direction := Vector2.RIGHT.rotated(deg_to_rad(BattleConfig.SELECTED_MOVE_LINE_ANGLE_DEGREES))
	var normal := direction.orthogonal()
	var line_color := Color(color, BattleConfig.SELECTED_MOVE_LINE_ALPHA)
	# 先依角度建立平行線，再裁切至格子內，避免端點插值左右線條角度。
	for index in range(BattleConfig.SELECTED_MOVE_LINE_COUNT):
		var offset := (float(index) - float(BattleConfig.SELECTED_MOVE_LINE_COUNT - 1) / 2.0) * BattleConfig.SELECTED_MOVE_LINE_SPACING
		var midpoint := center + normal * offset
		var line := PackedVector2Array([midpoint - direction * 64.0, midpoint + direction * 64.0])
		for segment in Geometry2D.intersect_polyline_with_polygon(line, points):
			canvas.draw_polyline(segment, line_color, BattleConfig.SELECTED_MOVE_LINE_WIDTH, true)
	points.append(points[0])
	canvas.draw_polyline(points, Color(color, BattleConfig.SELECTED_MOVE_EDGE_ALPHA), BattleConfig.SELECTED_MOVE_EDGE_WIDTH, true)
