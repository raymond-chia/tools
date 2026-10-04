class_name CoreResponse
extends RefCounted

static func read(response: String, show_error: Callable, ignored_error_ids: Array = [], detailed_errors: bool = false) -> Dictionary:
	var parsed = JSON.parse_string(response)
	if parsed == null:
		if detailed_errors:
			var parser := JSON.new()
			parser.parse(response)
			show_error.call("核心回應 JSON 解析失敗（第 %s 行）：%s" % [parser.get_error_line(), parser.get_error_message()])
		else:
			show_error.call("ERROR_CORE_RESPONSE_JSON_PARSE")
		return {}
	var value: Dictionary = parsed
	if value.has("error_id"):
		var error_id: String = value.error_id
		if not ignored_error_ids.has(error_id):
			if detailed_errors:
				show_error.call("%s: %s" % [error_id, value.error])
			else:
				show_error.call("ERROR_" + error_id.to_upper())
		return {}
	convert_unit_ids(value)
	return value

# Godot 會把 JSON 數字解析成 float；在接收核心資料時還原單位實例 ID 的整數型別。
static func convert_unit_ids(value: Dictionary) -> void:
	for unit in value.get("units", []):
		unit.id = int(unit.id)
	for terrain in value.get("terrain_cells", []):
		if terrain.unit_id != null:
			terrain.unit_id = int(terrain.unit_id)
	if value.has("turn") and value.turn.actor != null:
		value.turn.actor = int(value.turn.actor)
	for index in value.get("turn_order", []).size():
		value.turn_order[index] = int(value.turn_order[index])
	for movement in value.get("movements", []):
		movement.unit_id = int(movement.unit_id)
	for event in value.get("log", []):
		for key in ["actor", "target"]:
			if event.has(key):
				event[key] = int(event[key])
		for roll in event.get("initiative_rolls", []):
			roll.unit = int(roll.unit)
		for collision in event.get("collision_units", []):
			collision.unit = int(collision.unit)
	if value.has("target_type") and value.has("target"):
		value.target = int(value.target)
