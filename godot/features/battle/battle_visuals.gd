class_name BattleVisuals
extends RefCounted

const TILE_SIZE := Vector2i(64, 32)
const GROUND_ART := preload("res://assets/tiles/isometric_ground.svg")
const CARGO_ART := preload("res://assets/cargo.svg")
const BASE_ART := preload("res://assets/units/faction_base.svg")
# 所有地形素材使用相同畫布，格子中心位於圖片的 (32, 48)。
const TERRAIN_ART_ORIGIN := Vector2(32, 48)
const TERRAIN_ART := {
	"tree": preload("res://assets/terrain/tree.svg"),
	"broken_log": preload("res://assets/terrain/broken_log.svg"),
	"wagon": preload("res://assets/terrain/wagon.svg"),
	"spikes": preload("res://assets/terrain/spikes.svg"),
	"mire": preload("res://assets/terrain/mire.svg"),
}
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

static func create_unit_node(unit_id: int, visual_name: String, world_objects: Node2D) -> Node2D:
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
	# 血條固定畫在場景物件上方，彼此仍沿用所屬單位的 Y 排序。
	health.z_as_relative = false
	health.z_index = BattleConfig.HEALTH_BAR_Z_INDEX
	for bar_name in ["Background", "Fill"]:
		var bar := Polygon2D.new()
		bar.name = bar_name
		bar.color = Color("281e25") if bar_name == "Background" else Color("62d27c")
		health.add_child(bar)
	node.add_child(health)
	var cargo_marker := Sprite2D.new()
	cargo_marker.name = "CargoMarker"
	cargo_marker.visible = false
	cargo_marker.texture = CARGO_ART
	health.add_child(cargo_marker)
	world_objects.add_child(node)
	return node

# 編輯器與戰鬥地面共用同一張貨物圖片。
static func draw_ground_cargo(canvas: CanvasItem, point: Vector2) -> void:
	canvas.draw_texture(CARGO_ART, point - CARGO_ART.get_size() * 0.5)

static func update_unit_cargo(node: Node2D, unit: Dictionary) -> void:
	var marker: Sprite2D = node.get_node("Health/CargoMarker")
	var width: float = BattleConfig.LARGE_UNIT_HEALTH_BAR_WIDTH if unit.large else BattleConfig.UNIT_HEALTH_BAR_WIDTH
	marker.position = Vector2(width * 0.5 + 14.0, BattleConfig.HEALTH_BAR_HEIGHT * 0.5)
	marker.visible = unit.carrying_cargo

static func style_unit_node(node: Node2D, large: bool, team: Variant, selected: bool) -> void:
	var base_scale := Vector2.ONE * (BattleConfig.LARGE_UNIT_BASE_SCALE if large else BattleConfig.UNIT_BASE_SCALE)
	var attack_preview_ring: Sprite2D = node.get_node("Visual/AttackPreviewRing")
	attack_preview_ring.scale = base_scale * BattleConfig.ATTACK_PREVIEW_RING_SCALE
	attack_preview_ring.modulate = Color("ffe17a")
	var selection: Sprite2D = node.get_node("Visual/Selection")
	selection.scale = base_scale * BattleConfig.UNIT_SELECTION_RING_SCALE
	selection.modulate = Color("ffe17a")
	selection.visible = selected
	var base: Sprite2D = node.get_node("Visual/Base")
	base.scale = base_scale
	base.modulate = Color("ff6868") if BattleConfig.is_enemy(team) else Color("63a9ff")
	var body: Sprite2D = node.get_node("Visual/Body")
	body.scale = Vector2.ONE * (BattleConfig.LARGE_UNIT_BODY_SCALE if large else BattleConfig.UNIT_BODY_SCALE)
	body.position.y = -body.get_rect().end.y * body.scale.y
	body.modulate = Color.WHITE
	var health: Node2D = node.get_node("Health")
	health.position = Vector2(0, body.position.y + body.get_rect().position.y * body.scale.y - BattleConfig.HEALTH_BAR_HEAD_GAP - BattleConfig.HEALTH_BAR_HEIGHT)

# 只執行核心發出的顯示指令，不從單位狀態重建方向。
static func present_events(unit_nodes: Dictionary, events: Array) -> void:
	for event in events:
		match event.type:
			"unit_faced":
				var body: Sprite2D = unit_nodes[event.unit_id].get_node("Visual/Body")
				body.flip_h = event.facing == "left"

static func face_unit_toward(node: Node2D, destination: Vector2) -> void:
	var horizontal_distance := destination.x - node.position.x
	if is_zero_approx(horizontal_distance):
		return
	var body: Sprite2D = node.get_node("Visual/Body")
	body.flip_h = horizontal_distance < 0.0

static func update_unit_health(node: Node2D, unit: Dictionary) -> void:
	var width: float = BattleConfig.LARGE_UNIT_HEALTH_BAR_WIDTH if unit.large else BattleConfig.UNIT_HEALTH_BAR_WIDTH
	var health: Node2D = node.get_node("Health")
	var background: Polygon2D = health.get_node("Background")
	var fill: Polygon2D = health.get_node("Fill")
	background.polygon = health_bar_polygon(width, width)
	fill.visible = unit.hp > 0
	fill.polygon = health_bar_polygon(width, width * float(unit.hp) / float(unit.max_hp))

static func health_bar_polygon(width: float, filled_width: float) -> PackedVector2Array:
	var left := -width * 0.5
	return PackedVector2Array([Vector2(left, 0), Vector2(left + filled_width, 0), Vector2(left + filled_width, BattleConfig.HEALTH_BAR_HEIGHT), Vector2(left, BattleConfig.HEALTH_BAR_HEIGHT)])

static func diamond(center: Vector2) -> PackedVector2Array:
	var half_size := Vector2(TILE_SIZE) * 0.5
	return PackedVector2Array([center + Vector2(0, -half_size.y), center + Vector2(half_size.x, 0), center + Vector2(0, half_size.y), center + Vector2(-half_size.x, 0)])

static func footprint_center(ground: TileMapLayer, unit: Dictionary) -> Vector2:
	var first := ground.position + ground.map_to_local(Vector2i(unit.x, unit.y))
	var last_cell: Dictionary = unit.occupied_cells[-1]
	var last := ground.position + ground.map_to_local(Vector2i(last_cell.x, last_cell.y))
	return (first + last) * 0.5

# 可通行的 overlay 集中繪製，位於世界物件下方。
static func create_terrain_overlay(parent: CanvasItem, ground: TileMapLayer, effects: Callable) -> Node2D:
	var overlay := Node2D.new()
	overlay.name = "TerrainOverlay"
	overlay.z_index = BattleConfig.OVERLAY_Z_INDEX
	overlay.draw.connect(func(): draw_terrain_effects(overlay, ground, effects.call()))
	parent.add_child(overlay)
	return overlay

static func draw_terrain_effects(canvas: CanvasItem, ground: TileMapLayer, effects: Array) -> void:
	for effect in effects:
		if effect.blocks_movement:
			continue
		var center := ground.position + ground.map_to_local(Vector2i(effect.x, effect.y))
		draw_terrain_effect(canvas, effect.visual, center)

# 每個障礙物使用獨立節點，原點位於格子中心，與單位腳底共用 Y sorting。
static func sync_terrain_objects(world_objects: Node2D, ground: TileMapLayer, effects: Array, nodes: Dictionary) -> void:
	var active := {}
	for effect in effects:
		if not effect.blocks_movement:
			continue
		var key := "%s:%s:%s" % [effect.x, effect.y, effect.effect]
		active[key] = true
		if not nodes.has(key):
			var node := Node2D.new()
			var visual: String = effect.visual
			node.draw.connect(func(): draw_terrain_effect(node, visual, Vector2.ZERO))
			world_objects.add_child(node)
			nodes[key] = node
		var node: Node2D = nodes[key]
		node.position = ground.position + ground.map_to_local(Vector2i(effect.x, effect.y))
	for key in nodes.keys():
		if not active.has(key):
			var node: Node2D = nodes[key]
			world_objects.remove_child(node)
			node.queue_free()
			nodes.erase(key)

# 編輯器與戰鬥共用地面出口光圈，節點原點位於格子中心。
static func sync_exit_markers(world_objects: Node2D, ground: TileMapLayer, exits: Array, nodes: Dictionary) -> void:
	var active := {}
	for exit in exits:
		var cell := Vector2i(exit.x, exit.y)
		active[cell] = true
		if not nodes.has(cell):
			var node := preload("res://features/battle/exit_marker.gd").new()
			# 光圈貼地，固定在角色與場景物件下方。
			node.z_index = -1
			world_objects.add_child(node)
			nodes[cell] = node
		var node: Node2D = nodes[cell]
		node.position = ground.position + ground.map_to_local(cell)
	for cell in nodes.keys():
		if not active.has(cell):
			var node: Node2D = nodes[cell]
			world_objects.remove_child(node)
			node.queue_free()
			nodes.erase(cell)

static func draw_exit_marker(canvas: CanvasItem, progress: float) -> void:
	var ring := diamond(Vector2.ZERO)
	ring.append(ring[0])
	canvas.draw_polyline(ring, Color("258eff30"), 6.0, true)
	canvas.draw_polyline(ring, Color("65baff70"), 1.5, true)
	var trail := PackedVector2Array()
	var colors := PackedColorArray()
	# 尾巴繞滿一圈接回光點；插入轉角，避免線段切過格子內側。
	var start := (progress - 1.0) * 4.0
	var finish := progress * 4.0
	var samples: Array[float] = [start, finish]
	for index in range(int(floor(start)) + 1, int(ceil(finish))):
		samples.append(float(index))
	for index in range(1, 25):
		samples.append(lerpf(start, finish, float(index) / 25.0))
	samples.sort()
	for sample in samples:
		var edge := int(floor(sample))
		trail.append(ring[posmod(edge, 4)].lerp(ring[posmod(edge + 1, 4)], sample - floor(sample)))
		var brightness := (sample - start) / (finish - start)
		var color := BattleConfig.EXIT_TRAIL_COLOR.lerp(BattleConfig.EXIT_HEAD_COLOR, brightness)
		color.a *= lerpf(1.0, brightness * brightness, BattleConfig.EXIT_TRAIL_FADE_STRENGTH)
		colors.append(color)
	canvas.draw_polyline_colors(trail, colors, BattleConfig.EXIT_TRAIL_WIDTH * (10.0 / 3.5), true)
	canvas.draw_polyline_colors(trail, colors, BattleConfig.EXIT_TRAIL_WIDTH, true)
	var head := trail[trail.size() - 1]
	var head_glow := BattleConfig.EXIT_HEAD_COLOR
	head_glow.a *= 0.44
	canvas.draw_circle(head, BattleConfig.EXIT_HEAD_GLOW_WIDTH * 0.5, head_glow, true, -1.0, true)
	canvas.draw_circle(head, BattleConfig.EXIT_HEAD_WIDTH * 0.5, BattleConfig.EXIT_HEAD_COLOR, true, -1.0, true)

static func draw_terrain_effect(canvas: CanvasItem, visual: String, center: Vector2) -> void:
	if TERRAIN_ART.has(visual):
		canvas.draw_texture(TERRAIN_ART[visual], center - TERRAIN_ART_ORIGIN)

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
