extends Node

const SETTINGS_PATH := "user://settings.cfg"
const SETTINGS_SECTION := "display"
const LANGUAGE_SECTION := "language"
const DEFAULT_LANGUAGE := "en"
const DEFAULT_RESOLUTION := Vector2i(1280, 720)
const SUPPORTED_RESOLUTIONS: Array[Vector2i] = [
	Vector2i(1280, 720),
	Vector2i(1600, 900),
	Vector2i(1920, 1080),
]

var language: String = DEFAULT_LANGUAGE
var resolution: Vector2i = DEFAULT_RESOLUTION
var fullscreen := false

func _ready() -> void:
	load_settings()
	apply_settings()

func load_settings() -> void:
	var config := ConfigFile.new()
	var error := config.load(SETTINGS_PATH)
	if error == ERR_FILE_NOT_FOUND:
		return
	if error != OK:
		push_error("Failed to load settings: %s" % error_string(error))
		return

	language = _read_language(config.get_value(LANGUAGE_SECTION, "locale", DEFAULT_LANGUAGE))
	resolution = _read_resolution(config.get_value(SETTINGS_SECTION, "resolution", DEFAULT_RESOLUTION))
	fullscreen = bool(config.get_value(SETTINGS_SECTION, "fullscreen", fullscreen))

func save_settings() -> Error:
	var config := ConfigFile.new()
	config.set_value(LANGUAGE_SECTION, "locale", language)
	config.set_value(SETTINGS_SECTION, "resolution", resolution)
	config.set_value(SETTINGS_SECTION, "fullscreen", fullscreen)
	return config.save(SETTINGS_PATH)

func apply_settings() -> void:
	TranslationServer.set_locale(language)
	if fullscreen:
		DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN)
	else:
		DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED)
		DisplayServer.window_set_size(resolution)

func _read_resolution(value: Variant) -> Vector2i:
	if value is Vector2i and value in SUPPORTED_RESOLUTIONS:
		return value
	return DEFAULT_RESOLUTION

func _read_language(value: Variant) -> String:
	var locale := str(value)
	if locale in TranslationServer.get_loaded_locales():
		return locale
	return DEFAULT_LANGUAGE
