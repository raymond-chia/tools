extends Control

const TILE_SIZE := Vector2i(64, 32)
const PANEL_X := 700.0
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
var selected := ""
var hovered := Vector2i(-1, -1)
var status := "點角色查看資訊；點亮起的菱形格移動。"
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

func send(command: Dictionary) -> void:
	var value = JSON.parse_string(core.dispatch(JSON.stringify(command)))
	if value.has("error"): status = value.error
	else: state = value; status = ""; sync_unit_sprites()
	queue_redraw()

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
			var base := Sprite2D.new(); base.name = "Base"; base.texture = BASE_ART; node.add_child(base)
			var body := Sprite2D.new(); body.name = "Body"; body.texture = UNIT_ART[unit.id]; node.add_child(body)
			units_layer.add_child(node)
			unit_nodes[unit.id] = node
		else: node = unit_nodes[unit.id]
		var center := footprint_center(unit)
		node.position = center
		node.z_index = int(center.y)
		var large: bool = unit.width > 1
		node.get_node("Base").scale = Vector2(1.7, 1.7) if large else Vector2.ONE
		node.get_node("Base").modulate = Color("63a9ff") if unit.team == "player" else Color("ff6868")
		var body: Sprite2D = node.get_node("Body")
		body.position.y = -55 if large else -43
		body.scale = Vector2(0.88, 0.88) if large else Vector2(0.72, 0.72)
		body.modulate = Color(0.45,0.45,0.48,0.75) if unit.downed else Color.WHITE

func footprint_center(unit: Dictionary) -> Vector2:
	var first := cell_center(Vector2i(unit.x, unit.y))
	var last := cell_center(Vector2i(unit.x + unit.width - 1, unit.y + unit.height - 1))
	return (first + last) * 0.5

func unit_hit_rect(unit: Dictionary) -> Rect2:
	var center := footprint_center(unit)
	var size := Vector2(100, 135) if unit.width > 1 else Vector2(70, 92)
	return Rect2(center - Vector2(size.x * 0.5, size.y - 14), size)

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		hovered = point_to_cell(event.position) if event.position.x < PANEL_X else Vector2i(-1,-1)
		queue_redraw(); return
	if not event is InputEventMouseButton or not event.pressed or event.button_index != MOUSE_BUTTON_LEFT or state.is_empty(): return
	var mouse: Vector2 = event.position
	if mouse.x > PANEL_X: handle_buttons(mouse); return
	var ordered: Array = state.units.duplicate()
	ordered.sort_custom(func(a,b): return footprint_center(a).y > footprint_center(b).y)
	for unit in ordered:
		if unit_hit_rect(unit).has_point(mouse):
			selected = unit.id; status = "已選擇 %s｜HP %s/%s" % [unit.name,unit.hp,unit.max_hp]; queue_redraw(); return
	var cell := point_to_cell(mouse)
	if state.turn.actor != null and cell.x >= 0 and cell.y >= 0 and cell.x < state.width and cell.y < state.height:
		send({"type":"move","actor":state.turn.actor,"x":cell.x,"y":cell.y})

func handle_buttons(mouse: Vector2) -> void:
	if state.turn.actor == null: return
	var actor: String = state.turn.actor
	if Rect2(730,190,250,42).has_point(mouse) and selected != "": send({"type":"attack","actor":actor,"target":selected,"ranged":false})
	elif Rect2(730,240,250,42).has_point(mouse) and selected != "": send({"type":"attack","actor":actor,"target":selected,"ranged":true})
	elif Rect2(730,290,250,42).has_point(mouse) and selected != "": send({"type":"skill","actor":actor,"target":selected,"skill":"power_strike"})
	elif Rect2(730,340,250,42).has_point(mouse) and selected != "": send({"type":"skill","actor":actor,"target":selected,"skill":"aimed_shot"})
	elif Rect2(730,390,250,42).has_point(mouse): send({"type":"end_move","actor":actor})
	elif Rect2(730,440,250,42).has_point(mouse): send({"type":"end_turn","actor":actor})

func diamond(center: Vector2) -> PackedVector2Array:
	return PackedVector2Array([center+Vector2(0,-16),center+Vector2(32,0),center+Vector2(0,16),center+Vector2(-32,0)])

func draw_marker(cell: Vector2i, fill: Color, edge: Color) -> void:
	var points := diamond(cell_center(cell)); draw_colored_polygon(points,fill); points.append(points[0]); draw_polyline(points,edge,2,true)

func _draw() -> void:
	if state.is_empty(): draw_string(ThemeDB.fallback_font,Vector2(30,50),status); return
	draw_string(ThemeDB.fallback_font,Vector2(26,38),"灰燼谷伏擊",HORIZONTAL_ALIGNMENT_LEFT,-1,28,Color("f2d49b"))
	for p in state.reachable: draw_marker(Vector2i(p.x,p.y),Color(0.22,0.9,0.57,0.22),Color(0.35,1,0.69,0.7))
	if hovered.x >= 0 and hovered.y >= 0 and hovered.x < state.width and hovered.y < state.height: draw_marker(hovered,Color(1,1,1,0.08),Color(1,1,1,0.6))
	var grease := cell_center(Vector2i(4,4)); draw_set_transform(grease,0,Vector2(1,0.5)); draw_circle(Vector2.ZERO,20,Color(0.6,0.3,0.85,0.72)); draw_set_transform(Vector2.ZERO)
	for unit in state.units:
		var center := footprint_center(unit)
		var width: float = 96 if unit.width > 1 else 60
		draw_rect(Rect2(center+Vector2(-width*0.5,20),Vector2(width,7)),Color("281e25"))
		draw_rect(Rect2(center+Vector2(-width*0.5,20),Vector2(width*float(unit.hp)/unit.max_hp,7)),Color("62d27c"))
		if unit.id == selected: draw_arc(center,width*0.54,0,TAU,40,Color("ffe17a"),3,true)
	draw_panel()

func draw_panel() -> void:
	draw_rect(Rect2(700,0,340,size.y),Color("1d2939")); draw_line(Vector2(700,0),Vector2(700,size.y),Color("4c6179"),2)
	var actor := "—"
	for unit in state.units:
		if unit.id == state.turn.actor: actor = unit.name
	draw_string(ThemeDB.fallback_font,Vector2(730,62),"行動角色",HORIZONTAL_ALIGNMENT_LEFT,280,14,Color("8fa7c0")); draw_string(ThemeDB.fallback_font,Vector2(730,91),actor,HORIZONTAL_ALIGNMENT_LEFT,280,22)
	draw_string(ThemeDB.fallback_font,Vector2(730,124),"剩餘移動 %s　｜　%s" % [state.turn.move_remaining,state.turn.phase],HORIZONTAL_ALIGNMENT_LEFT,280,15,Color("b6c6d8"))
	button(Rect2(730,190,250,42),"近戰攻擊"); button(Rect2(730,240,250,42),"遠程攻擊"); button(Rect2(730,290,250,42),"強力一擊"); button(Rect2(730,340,250,42),"瞄準射擊"); button(Rect2(730,390,250,42),"結束本次移動"); button(Rect2(730,440,250,42),"結束回合")
	draw_string(ThemeDB.fallback_font,Vector2(730,515),status,HORIZONTAL_ALIGNMENT_LEFT,280,14,Color("f1c982"))
	var yy := 552
	for line in state.log: draw_string(ThemeDB.fallback_font,Vector2(730,yy),line,HORIZONTAL_ALIGNMENT_LEFT,285,13,Color("c8d4e2")); yy += 22

func button(rect: Rect2, label: String) -> void:
	draw_rect(rect,Color("304965")); draw_rect(rect,Color("7291ad"),false,1.5); draw_string(ThemeDB.fallback_font,rect.position+Vector2(18,28),label,HORIZONTAL_ALIGNMENT_LEFT,-1,17)
