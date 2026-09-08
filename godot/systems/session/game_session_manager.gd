extends Node

const UNITS_DATA_PATH := "res://data/units.toml"
const SKILLS_DATA_PATH := "res://data/skills.toml"
const EQUIPMENTS_DATA_PATH := "res://data/equipments.toml"
const OBJECTS_DATA_PATH := "res://data/objects.toml"
const LEVEL_DATA_PATH_TEMPLATE := "res://data/levels/%s.toml"
const ERROR_TRANSLATION_PREFIX := "ERROR_"
const ERROR_FALLBACK_TRANSLATION_KEY := "ERROR_UNKNOWN"
const ERROR_CODE_NO_ACTIVE_SESSION := "NoActiveSession"

var _session: GameSession
var _current_level_name := ""


func has_active_session() -> bool:
	return _session != null


func get_level_snapshot() -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("read level state")
	var result := _session.get_level_state()
	if result.get("ok", false) != true:
		_report_error(result, "Failed to read level state")
	return result


func start_new_game() -> Dictionary:
	var units_data := _read_text(UNITS_DATA_PATH)
	if units_data.get("ok", false) != true:
		_report_error(units_data)
		return units_data
	var skills_data := _read_text(SKILLS_DATA_PATH)
	if skills_data.get("ok", false) != true:
		_report_error(skills_data)
		return skills_data
	var equipments_data := _read_text(EQUIPMENTS_DATA_PATH)
	if equipments_data.get("ok", false) != true:
		_report_error(equipments_data)
		return equipments_data
	var objects_data := _read_text(OBJECTS_DATA_PATH)
	if objects_data.get("ok", false) != true:
		_report_error(objects_data)
		return objects_data

	var new_session := GameSession.new()
	var result := new_session.parse_and_insert_game_data(
		units_data["text"],
		skills_data["text"],
		equipments_data["text"],
		objects_data["text"]
	)
	if result.get("ok", false) != true:
		_report_error(result, "Failed to load game data")
		return result

	_session = new_session
	_current_level_name = ""
	return result


func enter_level(level_name: String) -> Dictionary:
	if not has_active_session():
		var error := {
			"ok": false,
			"error_code": ERROR_CODE_NO_ACTIVE_SESSION,
			"error": "Cannot enter level: no active game session",
		}
		_report_error(error)
		return error

	var level_path := LEVEL_DATA_PATH_TEMPLATE % level_name
	var level_data := _read_text(level_path)
	if level_data.get("ok", false) != true:
		_report_error(level_data)
		return level_data
	var snapshot := _session.spawn_level(level_name, level_data["text"])
	if snapshot.get("ok", false) != true:
		_report_error(snapshot, "Failed to load level: %s" % level_name)
		return snapshot

	_current_level_name = level_name
	return snapshot


func execute_skill(skill_name: String, target_positions: Array[Vector2i]) -> Dictionary:
	if not has_active_session():
		var error := {
			"ok": false,
			"error_code": ERROR_CODE_NO_ACTIVE_SESSION,
			"error": "Cannot execute skill: no active game session",
		}
		_report_error(error)
		return error
	var result := _session.execute_skill(skill_name, target_positions)
	if result.get("ok", false) != true:
		_report_error(result, "Skill execution failed")
	return result


func start_battle() -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("start battle")
	var result := _session.start_battle()
	if result.get("ok", false) != true:
		_report_error(result, "Failed to start battle")
	return result


func get_available_skills() -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("query skills")
	var result := _session.get_available_skills()
	if result.get("ok", false) != true:
		_report_error(result, "Failed to query skills")
	return result


func get_remaining_turn_units() -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("query remaining turn units")
	var result: Dictionary = _session.get_remaining_turn_units()
	if result.get("ok", false) != true:
		_report_error(result, "Failed to query remaining turn units")
	return result


func can_delay_current_unit() -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("query whether the current unit can delay")
	var result: Dictionary = _session.can_delay_current_unit()
	if result.get("ok", false) != true:
		_report_error(result, "Failed to query whether the current unit can delay")
	return result


func delay_current_unit(target_index: int) -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("delay current unit")
	var result: Dictionary = _session.delay_current_unit(target_index)
	if result.get("ok", false) != true:
		_report_error(result, "Failed to delay current unit")
	return result


func get_current_unit_reachable_positions() -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("query reachable positions")
	var result := _session.get_current_unit_reachable_positions()
	if result.get("ok", false) != true:
		_report_error(result, "Failed to query reachable positions")
	return result


func get_skill_targetable_positions(skill_name: String) -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("query skill targets")
	var result := _session.get_skill_targetable_positions(skill_name)
	if result.get("ok", false) != true:
		_report_error(result, "Failed to query skill targets")
	return result


func get_skill_affected_positions(skill_name: String, target_position: Vector2i) -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("query skill affected positions")
	var result: Dictionary = _session.get_skill_affected_positions(skill_name, target_position)
	if result.get("ok", false) != true:
		_report_error(result, "Failed to query skill affected positions")
	return result


func move_current_unit(target: Vector2i) -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("move current unit")
	var result := _session.move_current_unit(target)
	if result.get("ok", false) != true:
		_report_error(result, "Failed to move current unit")
	return result


func end_current_turn() -> Dictionary:
	if not has_active_session():
		return _no_active_session_error("end current turn")
	var result := _session.end_current_turn()
	if result.get("ok", false) != true:
		_report_error(result, "Failed to end current turn")
	return result


func end_game() -> void:
	_session = null
	_current_level_name = ""


func _read_text(path: String) -> Dictionary:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		var error := FileAccess.get_open_error()
		return {
			"ok": false,
			"error_code": "FileReadError",
			"error": "Failed to read file %s: %s" % [path, error_string(error)],
		}
	return {
		"ok": true,
		"text": file.get_as_text(),
	}


func _no_active_session_error(action: String) -> Dictionary:
	var error := {
		"ok": false,
		"error_code": ERROR_CODE_NO_ACTIVE_SESSION,
		"error": "Cannot %s: no active game session" % action,
	}
	_report_error(error)
	return error


func _report_error(result: Dictionary, fallback := "Operation failed") -> void:
	push_error(result.get("error", fallback))


func get_error_message(result: Dictionary) -> String:
	if result.get("ok", false) == true:
		return ""
	return get_error_message_for_code(result.get("error_code", ""))


func get_error_message_for_code(error_code: String) -> String:
	if error_code.is_empty():
		return tr(ERROR_FALLBACK_TRANSLATION_KEY)
	var translation_key := ERROR_TRANSLATION_PREFIX + error_code
	var message := tr(translation_key)
	if message == translation_key:
		return tr(ERROR_FALLBACK_TRANSLATION_KEY)
	return message
