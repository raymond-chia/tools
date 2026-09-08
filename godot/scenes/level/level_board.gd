extends Node2D
class_name LevelBoard

signal unit_selected(unit: Dictionary)
signal level_loaded(level_snapshot: Dictionary)
signal target_selected(position: Vector2i)
signal skill_targeting_cancelled
signal skill_target_hovered(position: Vector2i)
signal skill_target_hover_cleared
signal movement_preview_changed(path: Array, cost: int, pointer_position: Vector2)
signal movement_preview_cleared

const UNIT_TOKEN_SCENE := preload("res://scenes/level/unit_token/unit_token.tscn")
const DAMAGE_POPUP_SCENE := preload("res://scenes/level/damage_popup/damage_popup.tscn")
# 相機設定
const MIN_ZOOM := 0.45
const MAX_ZOOM := 2.0
const ZOOM_STEP := 1.05
const CAMERA_SPEED := 500.0
const INITIAL_ZOOM := 1.5
# 圖格設定
const TILE_SOURCE_ID := 0
const GROUND_TILE := Vector2i(0, 0)
const OBJECT_TILE := Vector2i(1, 0)
const SKILL_TARGETABLE_TILE := Vector2i(2, 0)
const NORMAL_MOVE_TILE := SKILL_TARGETABLE_TILE
const DOUBLE_MOVE_TILE := Vector2i(3, 0)
const MOVEMENT_PATH_TILE := Vector2i(4, 0)
const SKILL_AFFECTED_TILE := Vector2i(5, 0)
const SELECTION_TILE := Vector2i(6, 0)

var _level_snapshot: Dictionary = {}
var _units_by_cell: Dictionary = {}
var _unit_tokens_by_id: Dictionary = {}
var _objects_by_id: Dictionary = {}
var _targetable_cells: Dictionary = {}
var _movement_reachable: Dictionary = {}
var _is_movement_targeting := false
var _hovered_skill_target: Vector2i
var _has_hovered_skill_target := false
var _selected_cell: Vector2i
var _has_selection := false
@onready var camera: Camera2D = $Camera2D
@onready var ground_layer: TileMapLayer = $GroundLayer
@onready var content_layer: TileMapLayer = $WorldLayer/ContentLayer
@onready var units_layer: Node2D = $WorldLayer/UnitsLayer
@onready var target_layer: TileMapLayer = $TargetLayer
@onready var skill_affected_layer: TileMapLayer = $SkillAffectedLayer
@onready var movement_path_layer: TileMapLayer = $MovementPathLayer
@onready var selection_layer: TileMapLayer = $SelectionLayer
@onready var effects_layer: Node2D = $EffectsLayer

func _ready() -> void:
	set_process(false)

func display_level(snapshot: Dictionary) -> void:
	_level_snapshot = snapshot
	_clear_layers()
	_setup_camera()
	_populate_layers()
	_index_units_by_cell()
	set_process(true)
	level_loaded.emit(_level_snapshot)

func _is_level_loaded() -> bool:
	return _level_snapshot.get("ok", false)

func _process(delta: float) -> void:
	if not _is_level_loaded():
		return
	var direction := Input.get_vector("camera_left", "camera_right", "camera_up", "camera_down")
	if direction != Vector2.ZERO:
		camera.position += direction * CAMERA_SPEED * delta / camera.zoom.x

func _populate_layers() -> void:
	var width: int = _level_snapshot["board_width"]
	var height: int = _level_snapshot["board_height"]
	for y in height:
		for x in width:
			ground_layer.set_cell(Vector2i(x, y), TILE_SOURCE_ID, GROUND_TILE)
	for object in _level_snapshot["objects"]:
		var cell := Vector2i(object["x"], object["y"])
		content_layer.set_cell(cell, TILE_SOURCE_ID, OBJECT_TILE)
		_objects_by_id[object["id"]] = cell
	for unit in _level_snapshot["units"]:
		var token: UnitToken = UNIT_TOKEN_SCENE.instantiate()
		units_layer.add_child(token)
		var cell := Vector2i(unit["x"], unit["y"])
		token.position = ground_layer.map_to_local(cell)
		token.setup(unit)
		_unit_tokens_by_id[unit["id"]] = token

func _clear_layers() -> void:
	ground_layer.clear()
	content_layer.clear()
	target_layer.clear()
	skill_affected_layer.clear()
	movement_path_layer.clear()
	selection_layer.clear()
	_unit_tokens_by_id.clear()
	_objects_by_id.clear()
	_targetable_cells.clear()
	_movement_reachable.clear()
	_is_movement_targeting = false
	_has_hovered_skill_target = false
	for child in units_layer.get_children():
		child.queue_free()
	for child in effects_layer.get_children():
		child.queue_free()
	_has_selection = false

func _index_units_by_cell() -> void:
	_units_by_cell.clear()
	for unit in _level_snapshot["units"]:
		_units_by_cell[Vector2i(unit["x"], unit["y"])] = unit

func _setup_camera() -> void:
	var width: int = _level_snapshot["board_width"]
	var height: int = _level_snapshot["board_height"]
	camera.position = ground_layer.map_to_local(Vector2i(width / 2, height / 2))
	camera.zoom = Vector2.ONE * INITIAL_ZOOM


func focus_unit(unit_id: int) -> void:
	var token: UnitToken = _unit_tokens_by_id.get(unit_id)
	if token == null:
		return
	camera.global_position = token.global_position


func select_unit_by_id(unit_id: int) -> Dictionary:
	for cell in _units_by_cell:
		var unit: Dictionary = _units_by_cell[cell]
		if unit["id"] != unit_id:
			continue
		focus_unit(unit_id)
		_set_selected_unit(cell)
		return unit
	return {}

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_update_skill_target_hover(get_global_mouse_position())
		_update_movement_hover(get_global_mouse_position(), event.position)
		return
	if event is not InputEventMouseButton or not event.pressed:
		return
	match event.button_index:
		MOUSE_BUTTON_WHEEL_UP:
			_zoom_camera(ZOOM_STEP)
		MOUSE_BUTTON_WHEEL_DOWN:
			_zoom_camera(1.0 / ZOOM_STEP)
		MOUSE_BUTTON_RIGHT:
			_handle_right_click(get_global_mouse_position())
		MOUSE_BUTTON_LEFT:
			if not _select_target_at(get_global_mouse_position()):
				return
		_:
			return
	get_viewport().set_input_as_handled()

func _zoom_camera(factor: float) -> void:
	camera.zoom = (camera.zoom * factor).clamp(Vector2.ONE * MIN_ZOOM, Vector2.ONE * MAX_ZOOM)


func _handle_right_click(world_position: Vector2) -> void:
	if _has_unit_at(world_position):
		_select_unit_at(world_position)
		return
	if _cancel_skill_targeting():
		return
	_select_unit_at(world_position)


func _has_unit_at(world_position: Vector2) -> bool:
	if not _is_level_loaded():
		return false
	var cell := content_layer.local_to_map(content_layer.to_local(world_position))
	return _units_by_cell.has(cell)


func _select_unit_at(world_position: Vector2) -> bool:
	if not _is_level_loaded():
		return false
	var cell := content_layer.local_to_map(content_layer.to_local(world_position))
	var unit: Dictionary = _units_by_cell.get(cell, {})
	if unit.is_empty():
		_clear_selection()
		unit_selected.emit({})
		return false
	_set_selected_unit(cell)
	unit_selected.emit(unit)
	return true


func _set_selected_unit(cell: Vector2i) -> void:
	_clear_selection()
	selection_layer.set_cell(cell, TILE_SOURCE_ID, SELECTION_TILE)
	_selected_cell = cell
	_has_selection = true


func _clear_selection() -> void:
	if not _has_selection:
		return
	selection_layer.erase_cell(_selected_cell)
	_has_selection = false

func _select_target_at(world_position: Vector2) -> bool:
	if _targetable_cells.is_empty():
		return false
	var cell := content_layer.local_to_map(content_layer.to_local(world_position))
	if not _targetable_cells.has(cell):
		return false
	target_selected.emit(cell)
	return true

func _cancel_skill_targeting() -> bool:
	if _is_movement_targeting or _targetable_cells.is_empty():
		return false
	clear_skill_targeting()
	skill_targeting_cancelled.emit()
	return true

func set_skill_targetable_positions(positions: Array) -> void:
	target_layer.clear()
	_targetable_cells.clear()
	_movement_reachable.clear()
	_is_movement_targeting = false
	_clear_skill_affected_positions()
	_clear_movement_preview()
	for position in positions:
		var cell: Vector2i = position
		_targetable_cells[cell] = true
		target_layer.set_cell(cell, TILE_SOURCE_ID, SKILL_TARGETABLE_TILE)

func set_movement_targetable_positions(
	moves: Array,
	normal_range_cost: int,
	movement_cost_used: int
) -> void:
	target_layer.clear()
	_targetable_cells.clear()
	_movement_reachable.clear()
	_is_movement_targeting = true
	_clear_skill_affected_positions()
	_clear_movement_preview()
	for move in moves:
		var cell: Vector2i = move["position"]
		_movement_reachable[cell] = move
		if move["passthrough_only"]:
			continue
		_targetable_cells[cell] = true
		var tile := NORMAL_MOVE_TILE
		if movement_cost_used + move["cost"] > normal_range_cost:
			tile = DOUBLE_MOVE_TILE
		target_layer.set_cell(cell, TILE_SOURCE_ID, tile)

func clear_skill_targeting() -> void:
	target_layer.clear()
	_targetable_cells.clear()
	_movement_reachable.clear()
	_is_movement_targeting = false
	_clear_skill_affected_positions()
	_clear_movement_preview()

func set_skill_affected_positions(positions: Array) -> void:
	skill_affected_layer.clear()
	for position in positions:
		var cell: Vector2i = position
		skill_affected_layer.set_cell(cell, TILE_SOURCE_ID, SKILL_AFFECTED_TILE)

func _update_skill_target_hover(world_position: Vector2) -> void:
	if _is_movement_targeting or _targetable_cells.is_empty():
		_clear_skill_affected_positions()
		return
	var cell := content_layer.local_to_map(content_layer.to_local(world_position))
	if not _targetable_cells.has(cell):
		_clear_skill_affected_positions()
		return
	if _has_hovered_skill_target and _hovered_skill_target == cell:
		return
	_hovered_skill_target = cell
	_has_hovered_skill_target = true
	skill_target_hovered.emit(cell)

func _clear_skill_affected_positions() -> void:
	skill_affected_layer.clear()
	if not _has_hovered_skill_target:
		return
	_has_hovered_skill_target = false
	skill_target_hover_cleared.emit()

func _update_movement_hover(world_position: Vector2, pointer_position: Vector2) -> void:
	if not _is_movement_targeting:
		return
	var cell := content_layer.local_to_map(content_layer.to_local(world_position))
	var move: Dictionary = _movement_reachable.get(cell, {})
	if move.is_empty():
		_clear_movement_preview()
		return
	var path := _reconstruct_movement_path(cell)
	movement_path_layer.clear()
	for path_cell in path:
		movement_path_layer.set_cell(path_cell, TILE_SOURCE_ID, MOVEMENT_PATH_TILE)
	movement_preview_changed.emit(path, move["cost"], pointer_position)

func _reconstruct_movement_path(target: Vector2i) -> Array[Vector2i]:
	var path: Array[Vector2i] = [target]
	var current := target
	while _movement_reachable.has(current):
		var move: Dictionary = _movement_reachable[current]
		current = move["previous"]
		path.append(current)
	path.reverse()
	return path

func _clear_movement_preview() -> void:
	movement_path_layer.clear()
	movement_preview_cleared.emit()

func play_movement_path(unit_id: int, path: Array) -> void:
	if path.size() < 2:
		return
	var token: UnitToken = _unit_tokens_by_id.get(unit_id)
	if token == null:
		return
	var movement_tween := token.create_tween()
	token.play_move()
	for index in range(1, path.size()):
		var cell: Vector2i = path[index]
		var target_position := ground_layer.map_to_local(cell)
		movement_tween.tween_callback(token.face_toward.bind(target_position))
		movement_tween.tween_property(
			token,
			^"position",
			target_position,
			0.12
		).set_trans(Tween.TRANS_LINEAR)
	await movement_tween.finished
	token.stop_move()

func play_effect_entries(entries: Array, target_cell: Vector2i) -> void:
	if entries.is_empty():
		return
	var caster_id: int = entries[0]["caster_id"]
	var caster: UnitToken = _unit_tokens_by_id.get(caster_id)
	if caster != null:
		caster.play_attack(ground_layer.map_to_local(target_cell))
	await get_tree().create_timer(0.18).timeout
	for entry in entries:
		_play_effect_entry(entry)

func _play_effect_entry(entry: Dictionary) -> void:
	if entry["effect_type"] != "hp_change" or entry["target_type"] != "unit":
		return
	var target_id: int = entry["target_id"]
	var target: UnitToken = _unit_tokens_by_id.get(target_id)
	if target == null:
		return
	var amount: int = entry["final_amount"]
	if amount < 0:
		target.play_hit()
	target.apply_hp_change(amount)
	var popup: DamagePopup = DAMAGE_POPUP_SCENE.instantiate()
	effects_layer.add_child(popup)
	popup.position = target.position + Vector2(0, -92)
	popup.play(amount, entry["critical"])

# result["units"] 是全場單位的完整名單而非差分，因此索引一律整批重建，
# 不以個別單位的舊座標做增量增刪，避免單位互換位置時彼此覆蓋。
func apply_authoritative_changes(result: Dictionary) -> void:
	for unit_id in result["removed_unit_ids"]:
		_remove_unit(unit_id)
	for unit in result["units"]:
		_apply_unit_state(unit)
	_rebuild_units_by_cell(result["units"])
	for object in result["objects"]:
		_apply_object_state(object)

func _rebuild_units_by_cell(units: Array) -> void:
	_units_by_cell.clear()
	for unit in units:
		_units_by_cell[Vector2i(unit["x"], unit["y"])] = unit

func _apply_unit_state(unit: Dictionary) -> void:
	var unit_id: int = unit["id"]
	var token: UnitToken = _unit_tokens_by_id.get(unit_id)
	if token == null:
		token = UNIT_TOKEN_SCENE.instantiate()
		units_layer.add_child(token)
		_unit_tokens_by_id[unit_id] = token
	var cell := Vector2i(unit["x"], unit["y"])
	token.position = ground_layer.map_to_local(cell)
	token.setup(unit)

func _remove_unit(unit_id: int) -> void:
	var token: UnitToken = _unit_tokens_by_id.get(unit_id)
	if token == null:
		return
	var cell := Vector2i(token.unit_data["x"], token.unit_data["y"])
	_unit_tokens_by_id.erase(unit_id)
	if _has_selection and _selected_cell == cell:
		selection_layer.erase_cell(_selected_cell)
		_has_selection = false
		unit_selected.emit({})
	token.queue_free()

func _apply_object_state(object: Dictionary) -> void:
	var object_id: int = object["id"]
	if _objects_by_id.has(object_id):
		content_layer.erase_cell(_objects_by_id[object_id])
	var cell := Vector2i(object["x"], object["y"])
	content_layer.set_cell(cell, TILE_SOURCE_ID, OBJECT_TILE)
	_objects_by_id[object_id] = cell
