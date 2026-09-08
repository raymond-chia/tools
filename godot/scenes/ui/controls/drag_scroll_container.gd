extends ScrollContainer

signal click_released(global_position: Vector2)

const DRAG_START_DISTANCE := 8.0

var _drag_start_position := Vector2.ZERO
var _drag_start_scroll_vertical := 0
var _is_left_button_pressed := false
var _is_dragging := false

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		_handle_left_button(event)
	elif event is InputEventMouseMotion:
		_handle_mouse_motion(event)


func _handle_left_button(event: InputEventMouseButton) -> void:
	if event.pressed:
		_drag_start_position = event.position
		_drag_start_scroll_vertical = scroll_vertical
		_is_left_button_pressed = true
		_is_dragging = false
		accept_event()
		return

	var was_dragging := _is_dragging
	_is_left_button_pressed = false
	_is_dragging = false
	accept_event()
	if not was_dragging:
		click_released.emit(get_global_mouse_position())


func _handle_mouse_motion(event: InputEventMouseMotion) -> void:
	if not _is_left_button_pressed:
		return
	if (event.button_mask & MOUSE_BUTTON_MASK_LEFT) == 0:
		_reset_drag()
		return

	var drag_offset := event.position - _drag_start_position
	if not _is_dragging:
		if drag_offset.length() < DRAG_START_DISTANCE:
			return
		_is_dragging = true

	scroll_vertical = _drag_start_scroll_vertical - roundi(drag_offset.y)
	accept_event()


func _reset_drag() -> void:
	_is_left_button_pressed = false
	_is_dragging = false
