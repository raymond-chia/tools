class_name BattleCameraTest
extends GdUnitTestSuite

const LEVEL_BOARD_SCENE := "res://scenes/level/level_board.tscn"
const CAMERA_PATH := "Camera2D"
const INITIAL_ZOOM := Vector2(1.5, 1.5)
const MIN_ZOOM := Vector2(0.45, 0.45)
const MAX_ZOOM := Vector2(2.0, 2.0)
const TEST_LEVEL_SNAPSHOT := {
	"ok": true,
	"board_width": 10,
	"board_height": 10,
	"objects": [],
	"units": [],
}


func test_display_level_initializes_camera_zoom() -> void:
	var runner := scene_runner(LEVEL_BOARD_SCENE)
	var board: LevelBoard = runner.scene()
	var camera: Camera2D = board.get_node(CAMERA_PATH)

	board.display_level(TEST_LEVEL_SNAPSHOT)

	assert_vector(camera.zoom).is_equal(INITIAL_ZOOM)


func test_wasd_moves_camera_in_each_direction() -> void:
	var runner := scene_runner(LEVEL_BOARD_SCENE)
	var board: LevelBoard = runner.scene()
	var camera: Camera2D = board.get_node(CAMERA_PATH)
	board.display_level(TEST_LEVEL_SNAPSHOT)
	var initial_position := camera.position

	runner.simulate_action_press("camera_left")
	await runner.simulate_frames(2)
	runner.simulate_action_release("camera_left")
	assert_float(camera.position.x).is_less(initial_position.x)

	camera.position = initial_position
	runner.simulate_action_press("camera_right")
	await runner.simulate_frames(2)
	runner.simulate_action_release("camera_right")
	assert_float(camera.position.x).is_greater(initial_position.x)

	camera.position = initial_position
	runner.simulate_action_press("camera_up")
	await runner.simulate_frames(2)
	runner.simulate_action_release("camera_up")
	assert_float(camera.position.y).is_less(initial_position.y)

	camera.position = initial_position
	runner.simulate_action_press("camera_down")
	await runner.simulate_frames(2)
	runner.simulate_action_release("camera_down")
	assert_float(camera.position.y).is_greater(initial_position.y)


func test_mouse_wheel_zooms_camera() -> void:
	var runner := scene_runner(LEVEL_BOARD_SCENE)
	var board: LevelBoard = runner.scene()
	var camera: Camera2D = board.get_node(CAMERA_PATH)
	board.display_level(TEST_LEVEL_SNAPSHOT)

	await runner.simulate_mouse_button_pressed(MOUSE_BUTTON_WHEEL_UP).simulate_frames(1)
	assert_float(camera.zoom.x).is_greater(INITIAL_ZOOM.x)

	camera.zoom = INITIAL_ZOOM
	await runner.simulate_mouse_button_pressed(MOUSE_BUTTON_WHEEL_DOWN).simulate_frames(1)
	assert_float(camera.zoom.x).is_less(INITIAL_ZOOM.x)


func test_mouse_wheel_zoom_stays_within_limits() -> void:
	var runner := scene_runner(LEVEL_BOARD_SCENE)
	var board: LevelBoard = runner.scene()
	var camera: Camera2D = board.get_node(CAMERA_PATH)
	board.display_level(TEST_LEVEL_SNAPSHOT)

	for _index in 20:
		await runner.simulate_mouse_button_pressed(MOUSE_BUTTON_WHEEL_UP).simulate_frames(1)
	assert_vector(camera.zoom).is_equal(MAX_ZOOM)

	for _index in 40:
		await runner.simulate_mouse_button_pressed(MOUSE_BUTTON_WHEEL_DOWN).simulate_frames(1)
	assert_vector(camera.zoom).is_equal(MIN_ZOOM)
