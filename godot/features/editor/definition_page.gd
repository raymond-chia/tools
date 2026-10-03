extends HSplitContainer

# 資料頁只負責清單與作者輸入；資料操作由 game-core::editor 處理，遊戲規則由 game-core 驗證。
signal field_changed(category: String, id: String, key: String, value: Variant)
signal create_requested(category: String, id: String, source_id: String)
signal remove_requested(category: String, id: String)
signal remove_confirmed(category: String, id: String)
signal terrain_effect_confirmed(id: String, terrain: String)
signal move_requested(category: String, id: String, offset: int)
signal input_error(message: String)

const TITLES := {"unit_types": "單位", "skills": "技能", "terrain_types": "地形"}
const FIELD_LABELS := {
	"id": "ID", "visual": "外觀", "width": "佔地寬度", "height": "佔地高度",
	"hp": "生命值", "movement": "移動力", "initiative": "先攻", "dodge": "閃避",
	"block": "格擋", "attack": "命中", "power": "威力", "skills": "可用技能",
	"ranged": "遠程技能", "min_range": "最小範圍", "max_range": "最大範圍",
	"effect": "效果", "attack_bonus": "命中加成", "power_bonus": "威力加成",
	"terrain": "產生地形", "duration": "持續回合", "layer": "圖層",
	"entry_rule": "進入規則", "damage": "傷害", "extra_movement_cost": "額外移動消耗",
	"dodge_penalty": "閃避減值", "block_penalty": "格擋減值"
}
const CHOICES := {
	"effect": {"attack": "攻擊", "push": "推擊", "mire": "產生地形", "heal": "治療"},
	"layer": {"ground": "地面", "overlay": "上層"},
	"entry_rule": {"walkable": "可通行", "blocked": "不可通行", "instant_down_when_pushed": "推入時倒下"}
}

@export var category := "unit_types"
@onready var entries: ItemList = $ListPanel/Entries
@onready var fields: GridContainer = $FormPanel/Scroll/Fields
var definitions: Dictionary = {}
var rebuilding := false
var removing_id := ""
var source_id := ""
var effect_skill_id := ""
var effect_terrain_ids: Array = []

func _ready() -> void:
	$ListPanel/Title.text = TITLES[category] + "清單"
	entries.item_selected.connect(func(_index: int): refresh_fields())
	$ListPanel/Order/Up.pressed.connect(func(): move_requested.emit(category, selected_id(), -1))
	$ListPanel/Order/Down.pressed.connect(func(): move_requested.emit(category, selected_id(), 1))
	$ListPanel/Actions/Add.pressed.connect(func(): open_create_dialog(false))
	$ListPanel/Actions/Duplicate.pressed.connect(func(): open_create_dialog(true))
	$EffectDialog.confirmed.connect(func():
		terrain_effect_confirmed.emit(effect_skill_id, $EffectDialog/Form/Terrain.get_selected_metadata()))
	$AddDialog.confirmed.connect(submit_add)
	$AddDialog/Form/ID.text_submitted.connect(func(_text: String): submit_add())
	$ListPanel/Actions/Remove.pressed.connect(func(): remove_requested.emit(category, selected_id()))
	$RemoveDialog.confirmed.connect(func(): remove_confirmed.emit(category, removing_id))

func confirm_remove(id: String) -> void:
	removing_id = id
	$RemoveDialog.dialog_text = "確定刪除%s「%s」？" % [TITLES[category], id]
	$RemoveDialog.popup_centered()

func show_remove_error(message: String) -> void:
	$RemoveErrorDialog.dialog_text = message
	$RemoveErrorDialog.popup_centered(Vector2i(680, 320))

func open_create_dialog(duplicate: bool) -> void:
	source_id = selected_id() if duplicate else ""
	$AddDialog.title = ("複製" if duplicate else "新增") + TITLES[category]
	$AddDialog.ok_button_text = "複製" if duplicate else "新增"
	$AddDialog/Form/ID.text = source_id + "_copy" if duplicate else ""
	$AddDialog/Form/Error.text = ""
	$AddDialog.popup_centered()
	$AddDialog/Form/ID.grab_focus()
	$AddDialog/Form/ID.select_all()

func submit_add() -> void:
	create_requested.emit(category, $AddDialog/Form/ID.text.strip_edges(), source_id)

func finish_add() -> void:
	$AddDialog.hide()

func show_add_error(message: String) -> void:
	if $AddDialog.visible: $AddDialog/Form/Error.text = message
	if $EffectDialog.visible: $EffectDialog/Form/Error.text = message

func selected_id() -> String:
	var selected := entries.get_selected_items()
	return entries.get_item_text(selected[0]) if not selected.is_empty() else ""

func select_id(id: String) -> void:
	for index in entries.item_count:
		if entries.get_item_text(index) == id:
			entries.select(index)
			entries.ensure_current_is_visible()
			refresh_fields()
			return
	if entries.item_count > 0:
		entries.select(0)
	refresh_fields()

func present(value: Dictionary, terrain_ids: Array, terrain_entries: Array) -> void:
	effect_terrain_ids = terrain_ids
	var previous := selected_id()
	definitions = value
	entries.clear()
	if category == "terrain_types":
		for entry in terrain_entries:
			var index := entries.add_item(entry.id)
			entries.set_item_metadata(index, entry)
	else:
		for entry in definitions[category]: entries.add_item(entry.id)
	select_id(previous)
	$ListPanel/Actions/Remove.disabled = entries.item_count == 0
	$ListPanel/Actions/Duplicate.disabled = entries.item_count == 0

func selected_definition() -> Dictionary:
	var id := selected_id()
	for entry in definitions.get(category, []):
		if entry.id == id: return entry
	return {}

func refresh_fields() -> void:
	var selected := entries.get_selected_items()
	$ListPanel/Order/Up.disabled = selected.is_empty() or selected[0] == 0
	$ListPanel/Order/Down.disabled = selected.is_empty() or selected[0] == entries.item_count - 1
	if category == "terrain_types" and not selected.is_empty():
		var order: Dictionary = entries.get_item_metadata(selected[0])
		$ListPanel/Order/Up.disabled = not order.can_move_up
		$ListPanel/Order/Down.disabled = not order.can_move_down
	rebuilding = true
	for child in fields.get_children():
		fields.remove_child(child)
		child.queue_free()
	var entry := selected_definition().duplicate()
	var id := selected_id()
	for key in entry:
		var label := Label.new()
		label.text = FIELD_LABELS.get(key, key)
		fields.add_child(label)
		var value = entry[key]
		var input: Control
		if value is bool:
			var check := CheckBox.new()
			check.button_pressed = value
			check.toggled.connect(func(v: bool): field_changed.emit(category, id, key, v))
			input = check
		elif key == "skills":
			var skills := VBoxContainer.new()
			for skill in definitions.skills:
				var check := CheckBox.new()
				check.text = skill.id
				check.button_pressed = value.has(skill.id)
				check.toggled.connect(func(enabled: bool): toggle_skill(id, skill.id, enabled))
				skills.add_child(check)
			input = skills
		elif CHOICES.has(key) or key == "terrain":
			var choices: Dictionary = CHOICES.get(key, {})
			if key == "terrain":
				choices = {}
				for terrain in effect_terrain_ids: choices[terrain] = terrain
			var option := OptionButton.new()
			for choice in choices:
				option.add_item(choices[choice])
				option.set_item_metadata(option.item_count - 1, choice)
				if choice == value: option.select(option.item_count - 1)
			option.item_selected.connect(func(index: int): field_changed.emit(category, id, key, option.get_item_metadata(index)))
			input = option
		else:
			var edit := LineEdit.new()
			edit.text = str(value)
			# ID 建立後不可更動，因此不需要更新既有引用；刪除前仍須檢查引用。
			edit.editable = key != "id"
			edit.text_submitted.connect(func(_text: String): commit_field(id, key, edit, value))
			edit.focus_exited.connect(func(): commit_field(id, key, edit, value))
			input = edit
		input.custom_minimum_size.x = 320
		input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		fields.add_child(input)
	rebuilding = false

func commit_field(id: String, key: String, edit: LineEdit, previous: Variant) -> void:
	if rebuilding or edit.text == str(previous): return
	# 型別轉換屬於作者輸入邊界；遊戲規則合法性由 game-core 驗證。
	if previous is int:
		if not edit.text.is_valid_int():
			edit.text = str(selected_definition().get(key, previous))
			input_error.emit("%s 必須是整數" % FIELD_LABELS.get(key, key))
			return
		field_changed.emit(category, id, key, int(edit.text))
	else:
		field_changed.emit(category, id, key, edit.text)

func toggle_skill(id: String, skill_id: String, enabled: bool) -> void:
	var skills: Array = selected_definition().skills.duplicate()
	if enabled: skills.append(skill_id)
	else: skills.erase(skill_id)
	field_changed.emit(category, id, "skills", skills)

func choose_effect_terrain(id: String) -> void:
	# 尚未提交切換，先恢復目前效果的顯示；取消時資料與畫面維持一致。
	refresh_fields()
	if effect_terrain_ids.is_empty():
		input_error.emit("沒有可用的上層地形，請先建立上層地形。")
		return
	effect_skill_id = id
	$EffectDialog/Form/Terrain.clear()
	for terrain in effect_terrain_ids:
		$EffectDialog/Form/Terrain.add_item(terrain)
		$EffectDialog/Form/Terrain.set_item_metadata($EffectDialog/Form/Terrain.item_count - 1, terrain)
	$EffectDialog/Form/Error.text = ""
	$EffectDialog.popup_centered()

func finish_effect_change() -> void:
	$EffectDialog.hide()
