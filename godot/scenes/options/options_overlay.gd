extends CanvasLayer

var pending_language := ""
var pending_resolution := Vector2i.ZERO
var pending_fullscreen := false

@onready var language_option: OptionButton = %LanguageOption
@onready var resolution_option: OptionButton = %ResolutionOption
@onready var fullscreen_check: CheckButton = %FullscreenCheck
@onready var back_button: Button = %BackButton
@onready var confirm_dialog: ConfirmationDialog = %ConfirmDialog


func _ready() -> void:
	_load_pending_settings()
	_populate_languages()
	_populate_resolutions()
	back_button.grab_focus()

func _input(event: InputEvent) -> void:
	if confirm_dialog.visible:
		return
	if event.is_action_pressed("ui_cancel"):
		get_viewport().set_input_as_handled()
		_on_back_pressed()

func _load_pending_settings() -> void:
	pending_language = SettingsManager.language
	pending_resolution = SettingsManager.resolution
	pending_fullscreen = SettingsManager.fullscreen
	fullscreen_check.button_pressed = pending_fullscreen

func _populate_languages() -> void:
	language_option.clear()
	var locales := TranslationServer.get_loaded_locales()
	for locale in locales:
		var label := TranslationServer.get_locale_name(locale)
		language_option.add_item(label if not label.is_empty() else locale)
		language_option.set_item_metadata(language_option.item_count - 1, locale)
		if locale == pending_language:
			language_option.select(language_option.item_count - 1)

func _populate_resolutions() -> void:
	resolution_option.clear()
	for size in SettingsManager.SUPPORTED_RESOLUTIONS:
		resolution_option.add_item("%d x %d" % [size.x, size.y])
		resolution_option.set_item_metadata(resolution_option.item_count - 1, size)
		if size == pending_resolution:
			resolution_option.select(resolution_option.item_count - 1)

func _on_language_selected(index: int) -> void:
	pending_language = str(language_option.get_item_metadata(index))

func _on_resolution_selected(index: int) -> void:
	pending_resolution = resolution_option.get_item_metadata(index)

func _on_fullscreen_toggled(value: bool) -> void:
	pending_fullscreen = value

func _on_apply_pressed() -> void:
	_apply_pending()
	queue_free()

func _on_back_pressed() -> void:
	if not _has_pending_changes():
		queue_free()
	else:
		confirm_dialog.popup_centered()

func _on_confirm_discard() -> void:
	queue_free()

func _apply_pending() -> void:
	SettingsManager.language = pending_language
	SettingsManager.resolution = pending_resolution
	SettingsManager.fullscreen = pending_fullscreen
	SettingsManager.apply_settings()
	var error := SettingsManager.save_settings()
	if error != OK:
		push_error("Failed to save settings: %s" % error_string(error))

func _has_pending_changes() -> bool:
	return (
		pending_language != SettingsManager.language
		or pending_resolution != SettingsManager.resolution
		or pending_fullscreen != SettingsManager.fullscreen
	)
