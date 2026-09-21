extends Node2D

signal primary_clicked(unit_id: String, cell: Vector2i)
signal inspection_clicked(unit_id: String, cell: Vector2i)
signal move_preview_changed(total_cost, pointer_position: Vector2)
signal attack_preview_changed(preview: Dictionary, pointer_position: Vector2)
signal combat_events_finished

const TILE_SIZE := Vector2i(64, 32)
const UI_FONT := preload("res://assets/fonts/NotoSans.ttf")
const GROUND_ART := preload("res://assets/tiles/isometric_ground.svg")
const BASE_ART := preload("res://assets/units/faction_base.svg")

@onready var ground: TileMapLayer = $Ground
@onready var units_layer: Node2D = $Units
@onready var camera: Camera2D = get_node("../Camera2D")
var state: Dictionary = {}
var pending_action := ""
var inspected_cell := Vector2i(-1, -1)
var hovered := Vector2i(-1, -1)
var first_move_path: Array = []
var second_move_path: Array = []
var move_preview_interrupted := false
var move_preview_total_cost = null
var attack_preview: Dictionary = {}
var attack_preview_unit: Dictionary = {}
var core
var read_core_response: Callable
var unit_nodes := {}
var unit_ids_by_name := {}
var movement_tweens := {}
var hit_tweens := {}
var attack_tweens := {}
var prepared_move_unit_id := ""
var prepared_move_path: Array[Vector2i] = []
var combat_event_queue: Array[Dictionary] = []
var playing_combat_events := false
var pending_death_ids := {}
var pending_movement_ids := {}
var camera_tween: Tween
var camera_bounds := Rect2()

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
	for terrain in state.terrain_cells:
		var atlas_cell := Vector2i(1, 0) if terrain.base_kind == "rough" else Vector2i.ZERO
		ground.set_cell(Vector2i(terrain.x, terrain.y), 0, atlas_cell)
	update_camera_bounds()

func _process(delta: float) -> void:
	var direction := Vector2(
		float(Input.is_physical_key_pressed(KEY_D)) - float(Input.is_physical_key_pressed(KEY_A)),
		float(Input.is_physical_key_pressed(KEY_S)) - float(Input.is_physical_key_pressed(KEY_W))
	)
	if direction.is_zero_approx():
		return
	if camera_tween != null and camera_tween.is_valid():
		camera_tween.kill()
	camera.position = clamp_camera_position(camera.position + direction.normalized() * BattleVisualConfig.CAMERA_MOVE_SPEED * delta)

func present(snapshot: Dictionary, action: String, inspected: Vector2i, game_core) -> void:
	var previous_state := state
	remember_unit_names(previous_state.get("units", []))
	remember_unit_names(snapshot.get("units", []))
	state = snapshot
	pending_action = action
	inspected_cell = inspected
	core = game_core
	if not state.is_empty():
		queue_new_combat_events(previous_state)
		sync_unit_sprites(previous_state)
		start_combat_event_queue()
	update_move_preview()
	update_attack_preview()
	queue_redraw()

func _unhandled_input(event: InputEvent) -> void:
	if state.is_empty() or is_presenting_combat_events():
		return
	var local_event := make_input_local(event)
	if local_event is InputEventMouseMotion:
		var cell := point_to_cell(local_event.position)
		var next_hovered := cell if is_cell_on_board(cell) else Vector2i(-1, -1)
		if hovered != next_hovered:
			hovered = next_hovered
			update_move_preview()
			update_attack_preview()
			queue_redraw()
		else:
			move_preview_changed.emit(move_preview_total_cost, get_viewport().get_mouse_position())
			attack_preview_changed.emit(attack_preview, get_viewport().get_mouse_position())
		return
	if not local_event is InputEventMouseButton or not local_event.pressed:
		return
	var cell := point_to_cell(local_event.position)
	var unit := unit_at_cell(cell)
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

func focus_unit(unit_id: String) -> void:
	var unit := unit_with_id(state.get("units", []), unit_id)
	if unit.is_empty():
		return
	if camera_tween != null and camera_tween.is_valid():
		camera_tween.kill()
	camera_tween = create_tween().bind_node(camera)
	var destination := clamp_camera_position(to_global(footprint_center(unit)))
	camera_tween.tween_property(camera, "position", destination, BattleVisualConfig.CAMERA_FOCUS_DURATION).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)

func update_camera_bounds() -> void:
	if state.terrain_cells.is_empty():
		camera_bounds = Rect2()
		return
	var first_terrain: Dictionary = state.terrain_cells[0]
	var first_center := to_global(cell_center(Vector2i(first_terrain.x, first_terrain.y)))
	var minimum := first_center
	var maximum := first_center
	for terrain in state.terrain_cells.slice(1):
		var center := to_global(cell_center(Vector2i(terrain.x, terrain.y)))
		minimum = minimum.min(center)
		maximum = maximum.max(center)
	camera_bounds = Rect2(minimum, maximum - minimum).grow(BattleVisualConfig.CAMERA_BOUNDS_MARGIN)

func clamp_camera_position(position: Vector2) -> Vector2:
	if camera_bounds.has_area():
		return position.clamp(camera_bounds.position, camera_bounds.end)
	return position

func unit_id_at_cell(cell: Vector2i) -> String:
	var unit := unit_at_cell(cell)
	return unit.id if not unit.is_empty() else ""

func unit_cell(unit_id: String) -> Vector2i:
	var unit := unit_with_id(state.get("units", []), unit_id)
	if unit.is_empty():
		return Vector2i(-1, -1)
	return Vector2i(unit.x, unit.y)

func sync_unit_sprites(previous_state: Dictionary = {}) -> void:
	var current_unit_ids := {}
	for unit in state.units:
		current_unit_ids[unit.id] = true
	for unit_id in pending_death_ids:
		current_unit_ids[unit_id] = true
	for unit_id in unit_nodes.keys():
		if not current_unit_ids.has(unit_id):
			stop_unit_tweens(unit_id)
			unit_nodes[unit_id].queue_free()
			unit_nodes.erase(unit_id)
	for unit in state.units:
		var node: Node2D
		var is_new := not unit_nodes.has(unit.id)
		if is_new:
			node = Node2D.new()
			node.name = unit.id
			var visual := Node2D.new(); visual.name = "Visual"; node.add_child(visual)
			var new_attack_preview_ring := Sprite2D.new(); new_attack_preview_ring.name = "AttackPreviewRing"; new_attack_preview_ring.texture = BASE_ART; new_attack_preview_ring.visible = false; visual.add_child(new_attack_preview_ring)
			var selection := Sprite2D.new(); selection.name = "Selection"; selection.texture = BASE_ART; visual.add_child(selection)
			var base := Sprite2D.new(); base.name = "Base"; base.texture = BASE_ART; visual.add_child(base)
			var body := Sprite2D.new(); body.name = "Body"; body.texture = BattleVisualConfig.UNIT_ART[unit.id]; visual.add_child(body)
			units_layer.add_child(node)
			unit_nodes[unit.id] = node
		else:
			node = unit_nodes[unit.id]
		var center := footprint_center(unit)
		var previous_unit := unit_with_id(previous_state.get("units", []), unit.id)
		var moved: bool = not previous_unit.is_empty() and (previous_unit.x != unit.x or previous_unit.y != unit.y)
		if is_new:
			node.position = center
		elif pending_movement_ids.has(unit.id):
			pass
		elif not moved and not has_active_tween(movement_tweens, unit.id):
			node.position = center
		elif moved and unit_has_movement_transition(unit.id):
			pending_movement_ids[unit.id] = true
		else:
			animate_unit_movement(unit.id, node, center, unit)
		node.z_index = int(center.y)
		var large: bool = unit.large
		var base_scale := Vector2(1.7, 1.7) if large else Vector2.ONE
		var attack_preview_ring: Sprite2D = node.get_node("Visual/AttackPreviewRing")
		attack_preview_ring.scale = base_scale * BattleVisualConfig.ATTACK_PREVIEW_RING_SCALE
		attack_preview_ring.modulate = Color("ffe17a")
		var selection: Sprite2D = node.get_node("Visual/Selection")
		selection.scale = base_scale * 1.18
		selection.modulate = Color("ffe17a")
		selection.visible = unit_occupies_cell(unit, inspected_cell)
		node.get_node("Visual/Base").scale = base_scale
		node.get_node("Visual/Base").modulate = Color("63a9ff") if unit.team == "player" else Color("ff6868")
		var body: Sprite2D = node.get_node("Visual/Body")
		body.position.y = -55 if large else -43
		body.scale = Vector2(0.88, 0.88) if large else Vector2(0.72, 0.72)
		body.modulate = Color.WHITE

func prepare_move_animation(unit_id: String, destination: Vector2i) -> void:
	prepared_move_unit_id = unit_id
	prepared_move_path.clear()
	for cell in first_move_path:
		prepared_move_path.append(Vector2i(cell.x, cell.y))
	for cell in second_move_path:
		var position := Vector2i(cell.x, cell.y)
		if prepared_move_path.is_empty() or prepared_move_path[-1] != position:
			prepared_move_path.append(position)
	if prepared_move_path.is_empty() or prepared_move_path[-1] != destination:
		cancel_move_animation()

func cancel_move_animation() -> void:
	prepared_move_unit_id = ""
	prepared_move_path.clear()

func animate_unit_movement(unit_id: String, node: Node2D, destination: Vector2, unit: Dictionary) -> void:
	stop_tween(movement_tweens, unit_id)
	var tween := create_tween().bind_node(node)
	if unit_id == prepared_move_unit_id and not prepared_move_path.is_empty():
		for cell in prepared_move_path.slice(1):
			var path_cell := Vector2i(cell)
			var cell_center_position := cell_center(path_cell)
			if unit.large:
				var last_cell: Vector2i = path_cell + Vector2i(unit.width - 1, unit.height - 1)
				cell_center_position = (cell_center_position + cell_center(last_cell)) * 0.5
			tween.tween_property(node, "position", cell_center_position, BattleVisualConfig.UNIT_MOVE_STEP_DURATION).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)
		cancel_move_animation()
	else:
		tween.tween_property(node, "position", destination, BattleVisualConfig.UNIT_MOVE_STEP_DURATION).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)
	movement_tweens[unit_id] = tween

func queue_new_combat_events(previous_state: Dictionary) -> void:
	var previous_log_size: int = previous_state.get("log", []).size()
	var movements_changed: bool = previous_state.get("movements", []) != state.get("movements", [])
	for index in range(previous_log_size, state.log.size()):
		queue_movements_before_log(index)
		var event: Dictionary = state.log[index]
		if event.type in ["skill", "healing", "terrain_damage"]:
			combat_event_queue.append(event)
			if event.get("downed", false):
				var unit_id := unit_id_with_name(event.target)
				if not unit_id.is_empty():
					pending_death_ids[unit_id] = true
	if previous_log_size < state.log.size() or movements_changed:
		queue_movements_before_log(state.log.size())

func queue_movements_before_log(log_index: int) -> void:
	for movement in state.get("movements", []):
		if int(movement.before_log_index) == log_index:
			combat_event_queue.append({
				"type": "movement",
				"unit_id": movement.unit_id,
				"path": movement.path,
			})

func start_combat_event_queue() -> void:
	if not playing_combat_events and not combat_event_queue.is_empty():
		play_combat_event_queue()

func play_combat_event_queue() -> void:
	playing_combat_events = true
	while not combat_event_queue.is_empty():
		var event: Dictionary = combat_event_queue.pop_front()
		match event.type:
			"movement":
				await animate_movement_event(event)
			"skill":
				await present_skill_result(event)
			"healing":
				present_unit_text(event.target, "+%d" % int(event.healing), BattleVisualConfig.HEALING_TEXT_COLOR)
				await get_tree().create_timer(BattleVisualConfig.COMBAT_RESULT_HOLD).timeout
			"terrain_damage":
				await wait_for_unit_movement(event.target)
				present_unit_text(event.target, "-%d" % int(event.damage), BattleVisualConfig.DAMAGE_TEXT_COLOR)
				var hit_tween := animate_unit_hit(event.target)
				if hit_tween != null:
					await hit_tween.finished
				await get_tree().create_timer(BattleVisualConfig.COMBAT_RESULT_HOLD).timeout
				if event.downed:
					await animate_unit_death(event.target)
		await get_tree().create_timer(BattleVisualConfig.COMBAT_EVENT_PAUSE).timeout
	playing_combat_events = false
	queue_redraw()
	combat_events_finished.emit()

func is_presenting_combat_events() -> bool:
	return playing_combat_events or not combat_event_queue.is_empty()

func present_skill_result(event: Dictionary) -> void:
	var attack_tween := animate_unit_attack(event.actor, event.target)
	if attack_tween != null:
		await attack_tween.finished
	var hit_tween: Tween
	match event.result:
		"dodge":
			present_unit_text(event.target, "閃避", BattleVisualConfig.DODGE_TEXT_COLOR)
		"block":
			var block_text := "格擋" if int(event.damage) == 0 else "格擋 -%d" % int(event.damage)
			present_unit_text(event.target, block_text, BattleVisualConfig.BLOCK_TEXT_COLOR)
			hit_tween = animate_unit_hit(event.target)
		"hit":
			present_unit_text(event.target, "-%d" % int(event.damage), BattleVisualConfig.DAMAGE_TEXT_COLOR)
			hit_tween = animate_unit_hit(event.target)
	if int(event.collision_damage) > 0:
		present_unit_text(event.target, "-%d" % int(event.collision_damage), BattleVisualConfig.DAMAGE_TEXT_COLOR, 18.0)
	if hit_tween != null:
		await hit_tween.finished
	else:
		await get_tree().create_timer(BattleVisualConfig.HIT_FLASH_DURATION * 2.0).timeout
	await get_tree().create_timer(BattleVisualConfig.COMBAT_RESULT_HOLD).timeout
	if event.downed:
		await animate_unit_death(event.target)

func present_unit_text(unit_name: String, text: String, color: Color, horizontal_offset := 0.0) -> void:
	var unit_id := unit_id_with_name(unit_name)
	if unit_id.is_empty() or not unit_nodes.has(unit_id):
		return
	var label := Label.new()
	label.text = text
	label.position = Vector2(horizontal_offset - 80.0, -105.0)
	label.size = Vector2(160.0, 42.0)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.add_theme_font_override("font", UI_FONT)
	label.add_theme_font_size_override("font_size", 28)
	label.add_theme_color_override("font_color", color)
	label.add_theme_color_override("font_outline_color", Color(0.03, 0.04, 0.06, 0.95))
	label.add_theme_constant_override("outline_size", 6)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	unit_nodes[unit_id].add_child(label)
	var tween := create_tween().bind_node(label).set_parallel(true)
	tween.tween_property(label, "position:y", label.position.y - BattleVisualConfig.FLOATING_TEXT_RISE, BattleVisualConfig.FLOATING_TEXT_DURATION).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)
	tween.tween_property(label, "modulate:a", 0.0, BattleVisualConfig.FLOATING_TEXT_DURATION).set_delay(BattleVisualConfig.FLOATING_TEXT_DURATION * 0.45)
	tween.chain().tween_callback(label.queue_free)

func animate_unit_hit(unit_name: String) -> Tween:
	var unit_id := unit_id_with_name(unit_name)
	if unit_id.is_empty() or not unit_nodes.has(unit_id):
		return null
	var visual: Node2D = unit_nodes[unit_id].get_node("Visual")
	stop_tween(hit_tweens, unit_id)
	visual.position = Vector2.ZERO
	visual.modulate = Color.WHITE
	var tween := create_tween().bind_node(visual).set_parallel(true)
	tween.tween_property(visual, "modulate", Color(2.4, 2.4, 2.4, 1.0), BattleVisualConfig.HIT_FLASH_DURATION)
	tween.tween_property(visual, "position:x", BattleVisualConfig.HIT_SHAKE_DISTANCE, BattleVisualConfig.HIT_FLASH_DURATION)
	tween.chain().tween_property(visual, "modulate", Color.WHITE, BattleVisualConfig.HIT_FLASH_DURATION)
	tween.parallel().tween_property(visual, "position:x", -BattleVisualConfig.HIT_SHAKE_DISTANCE, BattleVisualConfig.HIT_FLASH_DURATION)
	tween.chain().tween_property(visual, "position:x", 0.0, BattleVisualConfig.HIT_FLASH_DURATION)
	hit_tweens[unit_id] = tween
	return tween

func animate_unit_attack(actor_name: String, target_name: String) -> Tween:
	var actor_id := unit_id_with_name(actor_name)
	var target_id := unit_id_with_name(target_name)
	if actor_id.is_empty() or target_id.is_empty() or not unit_nodes.has(actor_id) or not unit_nodes.has(target_id):
		return null
	var actor_visual: Node2D = unit_nodes[actor_id].get_node("Visual")
	var direction: Vector2 = unit_nodes[actor_id].position.direction_to(unit_nodes[target_id].position)
	stop_tween(attack_tweens, actor_id)
	actor_visual.position = Vector2.ZERO
	actor_visual.modulate = Color.WHITE
	var tween := create_tween().bind_node(actor_visual).set_parallel(true)
	tween.tween_property(actor_visual, "position", direction * BattleVisualConfig.ATTACK_LUNGE_DISTANCE, BattleVisualConfig.ATTACK_LUNGE_DURATION).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)
	tween.tween_property(actor_visual, "modulate", Color(1.45, 1.3, 0.85, 1.0), BattleVisualConfig.ATTACK_LUNGE_DURATION)
	tween.chain().tween_property(actor_visual, "position", Vector2.ZERO, BattleVisualConfig.ATTACK_LUNGE_DURATION).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_IN)
	tween.parallel().tween_property(actor_visual, "modulate", Color.WHITE, BattleVisualConfig.ATTACK_LUNGE_DURATION)
	attack_tweens[actor_id] = tween
	return tween

func animate_unit_death(unit_name: String) -> void:
	var unit_id := unit_id_with_name(unit_name)
	if unit_id.is_empty() or not unit_nodes.has(unit_id):
		return
	var body: Sprite2D = unit_nodes[unit_id].get_node("Visual/Body")
	var tween := create_tween().bind_node(body).set_parallel(true)
	tween.tween_property(body, "modulate", Color(0.45, 0.45, 0.48, 0.75), BattleVisualConfig.DEATH_FADE_DURATION)
	tween.tween_property(body, "position:y", body.position.y + 8.0, BattleVisualConfig.DEATH_FADE_DURATION).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_IN)
	await tween.finished
	pending_death_ids.erase(unit_id)
	stop_unit_tweens(unit_id)
	unit_nodes[unit_id].queue_free()
	unit_nodes.erase(unit_id)

func wait_for_unit_movement(unit_name: String) -> void:
	var unit_id := unit_id_with_name(unit_name)
	if unit_id.is_empty() or not movement_tweens.has(unit_id):
		return
	var tween: Tween = movement_tweens[unit_id]
	if tween.is_valid() and tween.is_running():
		await tween.finished

func unit_has_movement_transition(unit_id: String) -> bool:
	for movement in state.get("movements", []):
		if movement.unit_id == unit_id:
			return true
	return false

func animate_movement_event(event: Dictionary) -> void:
	var unit_id: String = event.unit_id
	if not unit_nodes.has(unit_id):
		return
	var unit := unit_with_id(state.units, unit_id)
	if unit.is_empty():
		return
	var node: Node2D = unit_nodes[unit_id]
	stop_tween(movement_tweens, unit_id)
	var tween := create_tween().bind_node(node)
	for cell_value in event.path.slice(1):
		var path_cell := Vector2i(int(cell_value.x), int(cell_value.y))
		var destination := cell_center(path_cell)
		if unit.large:
			var last_cell := path_cell + Vector2i(unit.width - 1, unit.height - 1)
			destination = (destination + cell_center(last_cell)) * 0.5
		tween.tween_property(node, "position", destination, BattleVisualConfig.UNIT_MOVE_STEP_DURATION).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)
	pending_movement_ids.erase(unit_id)
	movement_tweens[unit_id] = tween
	await tween.finished

func stop_unit_tweens(unit_id: String) -> void:
	stop_tween(movement_tweens, unit_id)
	stop_tween(hit_tweens, unit_id)
	stop_tween(attack_tweens, unit_id)

func has_active_tween(tweens: Dictionary, unit_id: String) -> bool:
	return tweens.has(unit_id) and tweens[unit_id].is_valid()

func stop_tween(tweens: Dictionary, unit_id: String) -> void:
	if tweens.has(unit_id):
		var tween: Tween = tweens[unit_id]
		if tween.is_valid():
			tween.kill()
		tweens.erase(unit_id)

func unit_with_id(units: Array, unit_id: String) -> Dictionary:
	for unit in units:
		if unit.id == unit_id:
			return unit
	return {}

func remember_unit_names(units: Array) -> void:
	for unit in units:
		unit_ids_by_name[unit.name] = unit.id

func unit_id_with_name(unit_name: String) -> String:
	return unit_ids_by_name.get(unit_name, "")

func footprint_center(unit: Dictionary) -> Vector2:
	var first := cell_center(Vector2i(unit.x, unit.y))
	var last := cell_center(Vector2i(unit.x + unit.width - 1, unit.y + unit.height - 1))
	return (first + last) * 0.5

func unit_at_cell(cell: Vector2i) -> Dictionary:
	var terrain := terrain_at_cell(cell)
	if terrain.is_empty() or terrain.unit_id == null:
		return {}
	for unit in state.units:
		if unit.id == terrain.unit_id:
			return unit
	return {}

func unit_occupies_cell_id(unit_id: String, cell: Vector2i) -> bool:
	var terrain := terrain_at_cell(cell)
	return not terrain.is_empty() and terrain.unit_id == unit_id

func unit_occupies_cell(unit: Dictionary, cell: Vector2i) -> bool:
	return unit_occupies_cell_id(unit.id, cell)

func terrain_at_cell(cell: Vector2i) -> Dictionary:
	if state.is_empty():
		return {}
	for terrain in state.terrain_cells:
		if terrain.x == cell.x and terrain.y == cell.y:
			return terrain
	return {}

func is_cell_on_board(cell: Vector2i) -> bool:
	return not terrain_at_cell(cell).is_empty()

func selected_skill_range() -> Array:
	if pending_action == "":
		return []
	for skill_range in state.skill_ranges:
		if skill_range.id == pending_action:
			return skill_range.cells
	return []

func pending_action_targets_cell() -> bool:
	for skill_range in state.skill_ranges:
		if skill_range.id == pending_action:
			return skill_range.cell_targeted
	return false

func clear_move_preview() -> void:
	first_move_path.clear()
	second_move_path.clear()
	move_preview_interrupted = false
	move_preview_total_cost = null

func update_move_preview() -> void:
	clear_move_preview()
	if is_presenting_combat_events() or pending_action != "" or core == null or state.is_empty() or state.turn.actor == null or not is_cell_on_board(hovered):
		move_preview_changed.emit(move_preview_total_cost, get_viewport().get_mouse_position())
		return
	var preview: Dictionary = read_core_response.call(core.preview_move(state.turn.actor, hovered.x, hovered.y), false)
	if preview.is_empty():
		move_preview_changed.emit(move_preview_total_cost, get_viewport().get_mouse_position())
		return
	first_move_path = preview.first
	second_move_path = preview.second
	move_preview_interrupted = preview.interrupted
	move_preview_total_cost = preview.total_cost
	move_preview_changed.emit(move_preview_total_cost, get_viewport().get_mouse_position())

func update_attack_preview() -> void:
	attack_preview = {}
	attack_preview_unit = {}
	sync_attack_preview_ring()
	if is_presenting_combat_events() or pending_action == "" or pending_action_targets_cell() or core == null or state.is_empty() or state.turn.actor == null:
		attack_preview_changed.emit(attack_preview, get_viewport().get_mouse_position())
		return
	var target := unit_at_cell(hovered)
	if target.is_empty():
		attack_preview_changed.emit(attack_preview, get_viewport().get_mouse_position())
		return
	var value: Dictionary = read_core_response.call(core.preview_skill(state.turn.actor, target.id, hovered.x, hovered.y, pending_action), false)
	if value.is_empty():
		attack_preview_changed.emit(attack_preview, get_viewport().get_mouse_position())
		return
	attack_preview = value
	attack_preview_unit = target
	sync_attack_preview_ring()
	attack_preview_changed.emit(attack_preview, get_viewport().get_mouse_position())

func sync_attack_preview_ring() -> void:
	var preview_unit_id: String = "" if attack_preview_unit.is_empty() else attack_preview_unit.id
	for unit_id in unit_nodes:
		var attack_preview_ring: Sprite2D = unit_nodes[unit_id].get_node("Visual/AttackPreviewRing")
		attack_preview_ring.visible = unit_id == preview_unit_id

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

func draw_cliff(cell: Vector2i) -> void:
	var center := cell_center(cell)
	var top := diamond(center)
	draw_colored_polygon(top, Color("59636c"))
	draw_polyline(PackedVector2Array([center + Vector2(-32, 0), center + Vector2(0, 16), center + Vector2(32, 0)]), Color("303840"), 5.0)
	draw_colored_polygon(PackedVector2Array([center + Vector2(-22, -2), center + Vector2(-8, -13), center + Vector2(3, 1)]), Color("818b91"))

func draw_chasm(cell: Vector2i) -> void:
	var center := cell_center(cell)
	draw_colored_polygon(diamond(center), Color("11131d"))
	draw_polyline(PackedVector2Array([center + Vector2(-32, 0), center + Vector2(0, -16), center + Vector2(32, 0)]), Color("b7774b"), 4.0)

func _draw() -> void:
	if state.is_empty():
		return
	if pending_action == "" and not is_presenting_combat_events():
		for cell in state.second_reachable: draw_marker(Vector2i(cell.x,cell.y),Color(0.04,0.15,0.42,0.38),Color(0.12,0.32,0.72,0.9))
		for cell in state.reachable: draw_marker(Vector2i(cell.x,cell.y),Color(0.12,0.48,0.95,0.3),Color(0.3,0.68,1.0,0.92))
	for cell in selected_skill_range(): draw_marker(Vector2i(cell.x,cell.y),Color(0.72,0.12,0.04,0.38),Color(1.0,0.34,0.12,0.95),3.0)
	if not attack_preview_unit.is_empty():
		for cell in attack_preview_unit.occupied_cells:
			draw_marker(Vector2i(cell.x, cell.y),Color(1.0,0.72,0.08,0.42),Color("ffe17a"),4.0)
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
		if effect.effect == "cliff":
			draw_cliff(Vector2i(effect.x, effect.y))
		elif effect.effect == "chasm":
			draw_chasm(Vector2i(effect.x, effect.y))
		elif effect.effect == "spikes":
			draw_spikes(Vector2i(effect.x, effect.y))
		elif effect.effect == "grease":
			var center := cell_center(Vector2i(effect.x, effect.y)); draw_set_transform(center,0,Vector2(1,0.5)); draw_circle(Vector2.ZERO,20,Color(0.6,0.3,0.85,0.72)); draw_set_transform(Vector2.ZERO)
		elif effect.effect == "mire":
			var center := cell_center(Vector2i(effect.x, effect.y)); draw_set_transform(center,0,Vector2(1,0.5)); draw_circle(Vector2.ZERO,23,Color(0.2,0.55,0.28,0.76)); draw_circle(Vector2(-9,1),5,Color(0.58,0.86,0.38,0.72)); draw_set_transform(Vector2.ZERO)
	for unit in state.units:
		var center := footprint_center(unit)
		var width: float = 96 if unit.large else 60
		draw_rect(Rect2(center+Vector2(-width*0.5,20),Vector2(width,7)),Color("281e25"))
		draw_rect(Rect2(center+Vector2(-width*0.5,20),Vector2(width*unit.health_ratio,7)),Color("62d27c"))
