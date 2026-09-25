class_name CoreResponse
extends RefCounted

static func read(response: String, show_error: Callable, ignored_error_ids: Array = []) -> Dictionary:
	var parsed = JSON.parse_string(response)
	if parsed == null:
		show_error.call("ERROR_CORE_RESPONSE_JSON_PARSE")
		return {}
	var value: Dictionary = parsed
	if value.has("error_id"):
		var error_id: String = value.error_id
		if not ignored_error_ids.has(error_id):
			show_error.call("ERROR_" + error_id.to_upper())
		return {}
	return value
