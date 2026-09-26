class_name BattleVisuals
extends RefCounted

const TILE_SIZE := Vector2i(64, 32)
const GROUND_ART := preload("res://assets/tiles/isometric_ground.svg")
const BASE_ART := preload("res://assets/units/faction_base.svg")

static func setup_ground(ground: TileMapLayer, terrain_cells: Array) -> void:
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
	ground.clear()
	for terrain in terrain_cells:
		var atlas_cell := Vector2i(1, 0) if terrain.base_kind == "rough" else Vector2i.ZERO
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

static func diamond(center: Vector2) -> PackedVector2Array:
	return PackedVector2Array([center + Vector2(0, -16), center + Vector2(32, 0), center + Vector2(0, 16), center + Vector2(-32, 0)])

static func draw_terrain_effect(canvas: CanvasItem, visual: String, center: Vector2) -> void:
	match visual:
		"cliff":
			canvas.draw_colored_polygon(diamond(center), Color("59636c"))
			canvas.draw_polyline(PackedVector2Array([center + Vector2(-32, 0), center + Vector2(0, 16), center + Vector2(32, 0)]), Color("303840"), 5.0)
			canvas.draw_colored_polygon(PackedVector2Array([center + Vector2(-22, -2), center + Vector2(-8, -13), center + Vector2(3, 1)]), Color("818b91"))
		"chasm":
			canvas.draw_colored_polygon(diamond(center), Color("11131d"))
			canvas.draw_polyline(PackedVector2Array([center + Vector2(-32, 0), center + Vector2(0, -16), center + Vector2(32, 0)]), Color("b7774b"), 4.0)
		"spikes":
			for offset_x in [-18.0, -6.0, 6.0, 18.0]:
				var base := center + Vector2(offset_x, 5.0)
				canvas.draw_colored_polygon(PackedVector2Array([base + Vector2(-5.0, 0.0), base + Vector2(5.0, 0.0), base + Vector2(0.0, -18.0)]), Color("d9d5ca"))
		"grease":
			canvas.draw_set_transform(center, 0, Vector2(1, 0.5))
			canvas.draw_circle(Vector2.ZERO, 20, Color(0.6, 0.3, 0.85, 0.72))
			canvas.draw_set_transform(Vector2.ZERO)
		"mire":
			canvas.draw_set_transform(center, 0, Vector2(1, 0.5))
			canvas.draw_circle(Vector2.ZERO, 23, Color(0.2, 0.55, 0.28, 0.76))
			canvas.draw_circle(Vector2(-9, 1), 5, Color(0.58, 0.86, 0.38, 0.72))
			canvas.draw_set_transform(Vector2.ZERO)
