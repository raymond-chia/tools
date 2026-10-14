class_name GameSettings
extends RefCounted

const PATH := "user://settings.cfg"
const LOCALES := ["zh_TW", "en"]
const PLAYBACK_SPEEDS := [1, 2, 4]
const DEFAULT_LOCALE := "zh_TW"
const DEFAULT_DIFFICULTY := "normal"
const DEFAULT_PLAYBACK_SPEED := 1

static func load_locale() -> String:
	return _read("language", "locale", DEFAULT_LOCALE, LOCALES)

static func load_difficulty() -> String:
	return _read("game", "difficulty", DEFAULT_DIFFICULTY, DifficultyOptions.IDS)

static func load_playback_speed() -> int:
	return _read("presentation", "playback_speed", DEFAULT_PLAYBACK_SPEED, PLAYBACK_SPEEDS)

static func save_locale(locale: String) -> bool:
	return _save("language", "locale", locale)

static func save_difficulty(difficulty: String) -> bool:
	return _save("game", "difficulty", difficulty)

static func save_playback_speed(speed: int) -> bool:
	return _save("presentation", "playback_speed", speed)

# 設定檔是偏好的輸入邊界；缺少或無效的值使用集中定義的預設值。
static func _read(section: String, key: String, default_value: Variant, choices: Array) -> Variant:
	var settings := _load_config()
	if settings == null:
		return default_value
	var value: Variant = settings.get_value(section, key, default_value)
	return value if typeof(value) == typeof(default_value) and value in choices else default_value

# 每次修改先載入完整內容，避免保存單項偏好時覆蓋其他設定。
static func _save(section: String, key: String, value: Variant) -> bool:
	var settings := _load_config()
	if settings == null:
		return false
	settings.set_value(section, key, value)
	return _save_config(settings)

static func _save_config(settings: ConfigFile) -> bool:
	var error := settings.save(PATH)
	if error != OK:
		push_error("無法保存玩家設定：%s" % error_string(error))
	return error == OK

static func _load_config() -> ConfigFile:
	var settings := ConfigFile.new()
	var error := settings.load(PATH)
	if error == OK:
		return settings
	if error != ERR_FILE_NOT_FOUND:
		push_error("無法讀取玩家設定：%s" % error_string(error))
		return null
	settings.set_value("language", "locale", DEFAULT_LOCALE)
	settings.set_value("game", "difficulty", DEFAULT_DIFFICULTY)
	settings.set_value("presentation", "playback_speed", DEFAULT_PLAYBACK_SPEED)
	if not _save_config(settings):
		return null
	return settings
