class_name MapNavigation
extends RefCounted

static func movement(delta: float) -> Vector2:
	var direction := Vector2(
		float(Input.is_physical_key_pressed(KEY_D)) - float(Input.is_physical_key_pressed(KEY_A)),
		float(Input.is_physical_key_pressed(KEY_S)) - float(Input.is_physical_key_pressed(KEY_W))
	)
	return direction.normalized() * BattleConfig.CAMERA_MOVE_SPEED * delta
