extends Node2D
class_name UnitToken

const UNIT_TEXTURES := {
	"fighter": preload("res://assets/units/fighter.svg"),
	"cleric": preload("res://assets/units/cleric.svg"),
	"archer": preload("res://assets/units/archer.svg"),
	"wolf": preload("res://assets/units/wolf.svg"),
}
const UNIT_NATIVE_FACING := {
	"fighter": 1,
	"cleric": 1,
	"archer": 1,
	"wolf": -1,
}

var unit_id := -1
var unit_data: Dictionary = {}

@onready var faction_base: Sprite2D = $FactionBase
@onready var facing_root: Node2D = $FacingRoot
@onready var visual: Sprite2D = $FacingRoot/VisualRoot/Visual
@onready var hp_bar: ProgressBar = $HpBar
@onready var animation_player: AnimationPlayer = $AnimationPlayer


func _ready() -> void:
	animation_player.play(&"idle")


func setup(unit: Dictionary) -> void:
	var is_new_unit := unit_id == -1
	unit_data = unit
	unit_id = unit["id"]
	visual.texture = UNIT_TEXTURES[unit["name"]]
	visual.flip_h = UNIT_NATIVE_FACING[unit["name"]] == -1
	if is_new_unit:
		_set_facing(UNIT_NATIVE_FACING[unit["name"]])
	var faction_color: Array = unit["faction_color"]
	faction_base.modulate = Color8(faction_color[0], faction_color[1], faction_color[2])
	apply_state(unit)


func apply_state(unit: Dictionary) -> void:
	unit_data = unit
	var attributes: Dictionary = unit["attributes"]
	update_hp(attributes["current_hp"], attributes["max_hp"])


func update_hp(current_hp: int, max_hp: int) -> void:
	hp_bar.max_value = max_hp
	hp_bar.value = current_hp
	var attributes: Dictionary = unit_data["attributes"]
	attributes["current_hp"] = current_hp
	attributes["max_hp"] = max_hp


func apply_hp_change(amount: int) -> void:
	var attributes: Dictionary = unit_data["attributes"]
	var current_hp: int = attributes["current_hp"]
	var max_hp: int = attributes["max_hp"]
	update_hp(clampi(current_hp + amount, 0, max_hp), max_hp)


func play_attack(target_position: Vector2) -> void:
	face_toward(target_position)
	animation_player.play(&"attack")


func face_toward(target_position: Vector2) -> void:
	var horizontal_distance := target_position.x - position.x
	if not is_zero_approx(horizontal_distance):
		_set_facing(signi(horizontal_distance))


func _set_facing(direction: int) -> void:
	facing_root.scale.x = direction


func play_hit() -> void:
	animation_player.play(&"hit")


func play_move() -> void:
	animation_player.play(&"walk")


func stop_move() -> void:
	animation_player.play(&"idle")


func _on_animation_finished(animation_name: StringName) -> void:
	if animation_name != &"idle":
		animation_player.play(&"idle")
