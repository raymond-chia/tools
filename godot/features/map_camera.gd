class_name MapCamera
extends Camera2D

var bounds := Rect2()
var focus_tween: Tween

func _ready() -> void:
	zoom = Vector2.ONE * BattleConfig.MAP_DISPLAY_SCALE

func configure_board(map_root: CanvasItem, ground: TileMapLayer, width: int, height: int) -> void:
	var corners: Array[Vector2i] = [Vector2i.ZERO, Vector2i(width - 1, 0), Vector2i(0, height - 1), Vector2i(width - 1, height - 1)]
	var first: Vector2 = map_root.get_global_transform() * (ground.position + ground.map_to_local(corners[0]))
	var minimum: Vector2 = first
	var maximum: Vector2 = first
	for cell in corners.slice(1):
		var point: Vector2 = map_root.get_global_transform() * (ground.position + ground.map_to_local(cell))
		minimum = minimum.min(point)
		maximum = maximum.max(point)
	bounds = Rect2(minimum, maximum - minimum).grow(BattleConfig.CAMERA_BOUNDS_MARGIN)
	position = clamp_to_bounds(position)

func center_on_board() -> void:
	position = bounds.get_center()

func move_with_input(delta: float) -> void:
	var movement := MapNavigation.movement(delta)
	if movement.is_zero_approx():
		return
	if focus_tween != null and focus_tween.is_valid():
		focus_tween.kill()
	position = clamp_to_bounds(position + movement)

func focus_on(point: Vector2) -> void:
	if focus_tween != null and focus_tween.is_valid():
		focus_tween.kill()
	focus_tween = create_tween().bind_node(self)
	focus_tween.tween_property(self, "position", clamp_to_bounds(point), BattleConfig.CAMERA_FOCUS_DURATION).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)

func clamp_to_bounds(point: Vector2) -> Vector2:
	return point.clamp(bounds.position, bounds.end) if bounds.has_area() else point
