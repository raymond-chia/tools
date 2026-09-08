extends CanvasLayer
class_name BattleOutcomeOverlay

signal return_to_menu_requested

@onready var title_label: Label = %TitleLabel
@onready var reason_label: Label = %ReasonLabel
@onready var return_to_menu_button: Button = %ReturnToMenuButton


func _ready() -> void:
	return_to_menu_button.grab_focus()


func show_outcome(outcome: Dictionary) -> void:
	match outcome["status"]:
		"victory":
			title_label.text = tr("BATTLE_VICTORY")
		"defeat":
			title_label.text = tr("BATTLE_DEFEAT")
	reason_label.text = tr(outcome["key"])


func _on_return_to_menu_pressed() -> void:
	return_to_menu_requested.emit()
