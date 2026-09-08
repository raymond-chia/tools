extends Node2D
class_name DamagePopup

const DAMAGE_COLOR := Color("ff625e")
const HEAL_COLOR := Color("66d17a")
const CRITICAL_COLOR := Color("ffd35a")

@onready var label: Label = $Label


func play(amount: int, critical: bool) -> void:
	label.text = "+%d" % amount if amount > 0 else str(amount)
	if critical:
		label.add_theme_color_override(&"font_color", CRITICAL_COLOR)
	elif amount > 0:
		label.add_theme_color_override(&"font_color", HEAL_COLOR)
	else:
		label.add_theme_color_override(&"font_color", DAMAGE_COLOR)

	scale = Vector2.ONE * 1.35
	var tween := create_tween().set_parallel(true)
	tween.tween_property(self, "position:y", position.y - 36.0, 0.65).set_trans(Tween.TRANS_QUAD).set_ease(Tween.EASE_OUT)
	tween.tween_property(self, "scale", Vector2.ONE, 0.18).set_trans(Tween.TRANS_BACK).set_ease(Tween.EASE_OUT)
	tween.tween_property(self, "modulate:a", 0.0, 0.3).set_delay(0.35)
	tween.chain().tween_callback(queue_free)
