extends HSplitContainer

# 資料頁只負責清單與作者輸入；資料操作由 game-core::editor 處理，遊戲規則由 game-core 驗證。
signal field_changed(category: String, id: String, key: String, value: Variant)
signal create_requested(category: String, id: String, source_id: String)
signal remove_requested(category: String, id: String)
signal remove_confirmed(category: String, id: String)
signal terrain_effect_confirmed(id: String, terrain: String)
signal move_requested(category: String, id: String, offset: int)
signal input_error(message: String)

const TITLES := {"unit_types": "單位", "skills": "技能", "terrain_types": "地形", "equipment": "裝備"}
const FIELD_LABELS := {
	"id": "ID", "visual": "外觀", "width": "佔地寬度", "height": "佔地高度",
	"hp": "生命值", "movement": "移動力", "initiative": "先攻", "dodge": "閃避",
	"block": "格擋", "hit": "命中", "physical_power": "物理威力", "magical_power": "魔法威力",
	"main_hand": "主手", "off_hand": "副手", "armor": "護具", "accessory": "飾品",
	"slot": "裝備種類", "block_reduction": "格擋減傷", "power_source": "威力來源", "active_skills": "主動技能", "passive_skills": "被動技能",
	"ranged": "遠程技能", "min_range": "最小範圍", "max_range": "最大範圍",
	"effect": "效果", "effects": "基礎效果", "hit_bonus": "命中加成", "power_bonus": "威力加成",
	"terrain": "產生地形", "duration": "持續回合", "layer": "圖層",
	"entry_rule": "進入規則", "damage": "傷害", "extra_movement_cost": "額外移動消耗",
	"blocks_sight": "阻擋視線", "dodge_penalty": "閃避減值", "block_penalty": "格擋減值"
}
const PASSIVE_EFFECT_LABELS := {"flanking": "近戰夾擊", "unhealable": "無法接受治癒"}
const CHOICES := {
	"slot": {"one_hand": "單手", "two_hand": "雙手", "armor": "護具", "accessory": "飾品"},
	"power_source": {"physical": "物理", "magical": "魔法"},
	# 效果選單依一般主動、魔法、被動排列，與技能清單一致。
	"effect": {"attack": "攻擊", "sweep": "前方三格橫掃", "loot": "掠奪", "push": "推擊", "move_then_attack": "位移後攻擊", "heal": "治療", "mire": "產生地形", "passive": "被動（組合基礎效果）"},
	"layer": {"ground": "地面", "overlay": "上層"},
	"entry_rule": {"walkable": "可通行", "blocked": "不可通行", "instant_down_when_pushed": "推入時陣亡"}
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
var equipment_choices: Dictionary = {}
var active_skill_ids: Array = []
var passive_skill_ids: Array = []
var field_texts: Dictionary = {}
var field_id := ""
var comparison_id := ""

func _ready() -> void:
	$ListPanel/Title.text = TITLES[category] + "清單"
	entries.item_clicked.connect(compare_clicked_entry)
	$FormPanel/Comparison/Close.pressed.connect(func():
		comparison_id = ""
		refresh_fields()
	)
	entries.item_selected.connect(func(_index: int): refresh_fields())
	entries.set_drag_forwarding(begin_entry_drag, can_drop_entry, drop_entry)
	$ListPanel/Actions/Add.pressed.connect(func(): open_create_dialog(false))
	$ListPanel/Actions/Duplicate.pressed.connect(func(): open_create_dialog(true))
	$EffectDialog.confirmed.connect(func():
		terrain_effect_confirmed.emit(effect_skill_id, $EffectDialog/Form/Terrain.get_selected_metadata()))
	$AddDialog.confirmed.connect(submit_add)
	$AddDialog/Form/ID.text_submitted.connect(func(_text: String): submit_add())
	$ListPanel/Actions/Remove.pressed.connect(func(): remove_requested.emit(category, selected_id()))
	$RemoveDialog.confirmed.connect(func(): remove_confirmed.emit(category, removing_id))

# 清單拖曳只處理介面位置；地形與技能可移動的目標由核心提供。
func begin_entry_drag(at_position: Vector2) -> Variant:
	var index := entries.get_item_at_position(at_position, true)
	if index < 0: return null
	entries.select(index)
	refresh_fields()
	var preview := Label.new()
	preview.text = entries.get_item_text(index)
	entries.set_drag_preview(preview)
	return {"source": entries, "index": index, "id": entries.get_item_metadata(index).id}

func entry_drop_index(at_position: Vector2) -> int:
	if entries.item_count == 0: return -1
	return entries.get_item_at_position(at_position)

func can_drop_entry(at_position: Vector2, data: Variant) -> bool:
	if not data is Dictionary or data.get("source") != entries: return false
	var target := entry_drop_index(at_position)
	if target < 0 or target == data.index: return false
	if category in ["terrain_types", "skills"]:
		var order: Dictionary = entries.get_item_metadata(data.index)
		return order.move_targets.has(entries.get_item_metadata(target).id)
	return true

func drop_entry(at_position: Vector2, data: Variant) -> void:
	move_requested.emit(category, data.id, entry_drop_index(at_position) - data.index)

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
	return entries.get_item_metadata(selected[0]).id if not selected.is_empty() else ""

func select_id(id: String) -> void:
	for index in entries.item_count:
		if entries.get_item_metadata(index).id == id:
			entries.select(index)
			entries.ensure_current_is_visible()
			refresh_fields()
			return
	if entries.item_count > 0:
		entries.select(0)
	refresh_fields()

func present(value: Dictionary, terrain_ids: Array, terrain_entries: Array, skill_entries: Array, loadout_choices: Dictionary, active_ids: Array, passive_ids: Array) -> void:
	equipment_choices = loadout_choices
	active_skill_ids = active_ids
	passive_skill_ids = passive_ids
	effect_terrain_ids = terrain_ids
	var previous := selected_id()
	definitions = value
	entries.clear()
	if category in ["terrain_types", "skills"]:
		for entry in terrain_entries if category == "terrain_types" else skill_entries:
			var index := entries.add_item(entry.id)
			entries.set_item_metadata(index, entry)
	else:
		for entry in definitions[category]:
			var label: String = entry.id
			if category == "unit_types" and entry.visual.is_empty():
				label += "（外觀待選擇）"
			var index := entries.add_item(label)
			entries.set_item_metadata(index, {"id": entry.id})
	select_id(previous)
	$ListPanel/Actions/Remove.disabled = entries.item_count == 0
	$ListPanel/Actions/Duplicate.disabled = entries.item_count == 0

# 右鍵只指定參考資料，不改變左鍵選取的編輯對象。
func compare_clicked_entry(index: int, _position: Vector2, button: int) -> void:
	if button != MOUSE_BUTTON_RIGHT: return
	comparison_id = entries.get_item_metadata(index).id
	refresh_fields()

func selected_definition() -> Dictionary:
	return definition_by_id(selected_id())

func definition_by_id(id: String) -> Dictionary:
	# 技能頁同時呈現主動與被動技能。
	var category_entries: Array = definitions.active_skills + definitions.passive_skills if category == "skills" else definitions.get(category, [])
	for entry in category_entries:
		if entry.id == id: return entry
	return {}

func refresh_fields() -> void:
	rebuilding = true
	var id := selected_id()
	# 切換清單項目沿用原本的取消輸入行為；同一項目的刷新保留待提交文字。
	if field_id != id:
		field_texts.clear()
		field_id = id
	for child in fields.get_children():
		fields.remove_child(child)
		child.queue_free()
	var comparison := definition_by_id(comparison_id).duplicate()
	if comparison.has("effects"): comparison["effect"] = "passive"
	if comparison.is_empty(): comparison_id = ""
	var comparing := not comparison_id.is_empty()
	$FormPanel/Comparison.visible = comparing
	$FormPanel/Comparison/Title.text = "比對：" + comparison_id + "（唯讀）"
	fields.columns = 3 if comparing else 2
	var entry := selected_definition().duplicate()
	if entry.has("effects"): entry["effect"] = "passive"
	for key in entry:
		var label := Label.new()
		label.text = "" if key in ["active_skills", "passive_skills"] else FIELD_LABELS.get(key, key)
		fields.add_child(label)
		var value = entry[key]
		var input: Control
		if key == "effects":
			input = create_passive_effect_editor(id, value)
		elif key == "passive_skills":
			input = create_passive_selector(value, func(v: Variant): field_changed.emit(category, id, key, v))
		elif value is bool:
			var check := CheckBox.new()
			check.button_pressed = value
			check.toggled.connect(func(v: bool): field_changed.emit(category, id, key, v))
			input = check
		elif key == "visual" and category == "unit_types":
			input = create_visual_selector(id, value)
		elif key == "active_skills":
			var skills := VBoxContainer.new()
			for skill_id in active_skill_ids:
				var check := CheckBox.new()
				check.text = skill_id
				check.button_pressed = value.has(skill_id)
				check.toggled.connect(func(enabled: bool): toggle_skill(id, skill_id, enabled))
				skills.add_child(check)
			input = skills
		elif CHOICES.has(key) or key == "terrain" or key in ["main_hand", "off_hand", "armor", "accessory"]:
			var choices: Dictionary = CHOICES.get(key, {})
			if key == "terrain":
				choices = {}
				for terrain in effect_terrain_ids: choices[terrain] = terrain
			if key in ["main_hand", "off_hand", "armor", "accessory"]:
				choices = {"": "無"}
				for equipment_id in equipment_choices[id][key]:
					if not equipment_id.is_empty(): choices[equipment_id] = equipment_id
			var option := OptionButton.new()
			for choice in choices:
				option.add_item(choices[choice])
				option.set_item_metadata(option.item_count - 1, choice)
				if choice == value: option.select(option.item_count - 1)
			option.item_selected.connect(func(index: int): field_changed.emit(category, id, key, option.get_item_metadata(index)))
			input = option
		else:
			var edit := LineEdit.new()
			edit.text = field_texts.get(key, str(value))
			# ID 建立後不可更動，因此不需要更新既有引用；刪除前仍須檢查引用。
			edit.editable = key != "id"
			edit.set_meta("field_key", key)
			edit.set_meta("commit_field", commit_field.bind(id, key, edit, value))
			edit.text_changed.connect(func(text: String): field_texts[key] = text)
			edit.text_submitted.connect(func(_text: String): commit_field(id, key, edit, value))
			edit.focus_exited.connect(func(): commit_field(id, key, edit, value))
			input = edit
		if key in ["active_skills", "passive_skills"]:
			input = create_skill_section(FIELD_LABELS[key], input)
		input.custom_minimum_size.x = 320
		input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		fields.add_child(input)
		if comparing:
			add_comparison_value(key, comparison.get(key))
	# 主動與被動技能的欄位可能不同；參考資料獨有的欄位也要顯示。
	if comparing:
		for key in comparison:
			if entry.has(key): continue
			var label := Label.new()
			label.text = FIELD_LABELS.get(key, key)
			fields.add_child(label)
			var empty := Label.new()
			empty.text = "—"
			fields.add_child(empty)
			add_comparison_value(key, comparison[key])
	rebuilding = false

# 只排版作者資料，不計算數值差異或遊戲規則。
func add_comparison_value(key: String, value: Variant) -> void:
	var label := Label.new()
	label.custom_minimum_size.x = 320
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.text = comparison_text(key, value)
	label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	label.tooltip_text = label.text
	fields.add_child(label)

func comparison_text(key: String, value: Variant) -> String:
	if value == null: return "—"
	if value is bool: return "是" if value else "否"
	if key == "effects":
		var descriptions: Array[String] = []
		for effect in value:
			var text: String = PASSIVE_EFFECT_LABELS[effect.effect]
			for field in effect:
				if field != "effect": text += " %s：%s" % [FIELD_LABELS[field], str(int(effect[field]))]
			descriptions.append(text)
		return "；".join(descriptions) if not descriptions.is_empty() else "無"
	if value is Array:
		if value.is_empty():
			return "無"
		return "、".join(value)
	if CHOICES.has(key): return CHOICES[key][value]
	if str(value).is_empty(): return "待選擇" if key == "visual" else "無"
	return str(value)

func discard_pending_fields() -> void:
	field_texts.clear()

func accept_field(id: String, key: String) -> void:
	if field_id == id: field_texts.erase(key)

func commit_pending_fields(has_error: Callable) -> void:
	# 提交會重建欄位，先保存本次輸入，避免後續欄位的文字被刷新覆蓋。
	var pending: Array[Dictionary] = []
	for child in fields.get_children():
		if child is LineEdit and child.editable:
			pending.append({"key": child.get_meta("field_key"), "text": child.text})
	# SpinBox 的文字可能尚未確認，儲存前先交給控制項提交。
	for number: SpinBox in fields.find_children("*", "SpinBox", true, false):
		number.apply()
		if has_error.call(): return
	for field in pending:
		for child in fields.get_children():
			if child is LineEdit and child.get_meta("field_key") == field.key:
				child.text = field.text
				child.get_meta("commit_field").call()
				if has_error.call(): return
				break

func commit_field(id: String, key: String, edit: LineEdit, previous: Variant) -> void:
	if rebuilding: return
	if edit.text == str(previous):
		accept_field(id, key)
		return
	field_texts[key] = edit.text
	# 型別轉換屬於作者輸入邊界；遊戲規則合法性由 game-core 驗證。
	if previous is int or previous is float:
		if not edit.text.is_valid_float():
			input_error.emit("%s 必須是數字" % FIELD_LABELS.get(key, key))
			return
		field_changed.emit(category, id, key, edit.text.to_float())
	else:
		field_changed.emit(category, id, key, edit.text)

func toggle_skill(id: String, skill_id: String, enabled: bool) -> void:
	var skills: Array = selected_definition().active_skills.duplicate()
	if enabled: skills.append(skill_id)
	else: skills.erase(skill_id)
	field_changed.emit(category, id, "active_skills", skills)

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

# 外觀是顯示素材，選單讀取既有圖片；空值明確表示作者尚未選擇。
func create_visual_selector(id: String, visual: String) -> OptionButton:
	var option := OptionButton.new()
	option.add_theme_constant_override("icon_max_width", 48)
	option.add_icon_item(load(BattleConfig.unit_art_path("")), "待選擇")
	option.set_item_metadata(0, "")
	option.get_popup().set_item_icon_max_width(0, 48)
	# 單位專屬與共用圖片分段列出，避免共用圖混在單位圖之間難以尋找。
	for section in [["單位", BattleConfig.UNIT_ART_DIR], ["共用", BattleConfig.SHARED_ART_DIR]]:
		option.add_separator(section[0])
		for filename in DirAccess.get_files_at(section[1]):
			if filename in ["faction_base.svg", BattleConfig.PENDING_UNIT_ART]: continue
			if filename.get_extension().to_lower() not in ["svg", "png", "jpg", "jpeg", "webp"]: continue
			option.add_icon_item(load(BattleConfig.unit_art_path(filename)), filename)
			var index := option.item_count - 1
			option.set_item_metadata(index, filename)
			option.get_popup().set_item_icon_max_width(index, 48)
			if filename == visual: option.select(index)
	option.item_selected.connect(func(index: int): field_changed.emit(category, id, "visual", option.get_item_metadata(index)))
	return option

# 技能欄位由同一種盒子呈現，標題與清單放在一起，避免兩組看成連續選項。
func create_skill_section(title: String, selector: Control) -> MarginContainer:
	var spacing := MarginContainer.new()
	spacing.add_theme_constant_override("margin_top", 6)
	spacing.add_theme_constant_override("margin_bottom", 6)
	var panel := PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.18, 0.20, 0.24, 0.65)
	style.border_color = Color(0.45, 0.49, 0.56, 0.8)
	style.set_border_width_all(1)
	style.set_content_margin_all(12)
	panel.add_theme_stylebox_override("panel", style)
	spacing.add_child(panel)
	var content := VBoxContainer.new()
	content.add_theme_constant_override("separation", 8)
	panel.add_child(content)
	var heading := Label.new()
	heading.text = title
	content.add_child(heading)
	content.add_child(HSeparator.new())
	content.add_child(selector)
	return spacing

# 選項由核心分類；此處只編輯勾選清單，效果強弱由核心判斷。
func create_passive_selector(value: Array, changed: Callable) -> VBoxContainer:
	var container := VBoxContainer.new()
	for skill_id in passive_skill_ids:
		var check := CheckBox.new()
		check.text = skill_id
		check.button_pressed = value.has(skill_id)
		check.toggled.connect(func(enabled: bool):
			var selected: Array = value.duplicate()
			if enabled: selected.append(skill_id)
			else: selected.erase(skill_id)
			changed.call(selected)
		)
		container.add_child(check)
	return container

# 此處只編輯作者輸入；效果的預設值、重複與數值合法性由核心處理。
func create_passive_effect_editor(id: String, effects: Array) -> VBoxContainer:
	var container := VBoxContainer.new()
	for kind in PASSIVE_EFFECT_LABELS:
		var row := HBoxContainer.new()
		var check := CheckBox.new()
		check.text = PASSIVE_EFFECT_LABELS[kind]
		check.set_meta("effect", kind)
		var selected: Dictionary = {}
		for effect in effects:
			if effect.effect == kind: selected = effect
		check.button_pressed = not selected.is_empty()
		check.toggled.connect(func(enabled: bool): toggle_passive_effect(id, kind, enabled))
		row.add_child(check)
		for field in selected:
			if field == "effect": continue
			var label := Label.new()
			label.text = FIELD_LABELS[field]
			row.add_child(label)
			var number := SpinBox.new()
			number.allow_greater = true
			number.allow_lesser = true
			number.value = selected[field]
			number.set_meta("effect", kind)
			number.set_meta("field", field)
			number.value_changed.connect(func(value: float): update_passive_effect_field(id, kind, field, value))
			row.add_child(number)
		container.add_child(row)
	return container

func toggle_passive_effect(id: String, kind: String, enabled: bool) -> void:
	var effects: Array = selected_definition().effects.duplicate(true)
	if enabled:
		effects.append({"effect": kind})
	else:
		for index in range(effects.size() - 1, -1, -1):
			if effects[index].effect == kind: effects.remove_at(index)
	field_changed.emit(category, id, "effects", effects)

func update_passive_effect_field(id: String, kind: String, field: String, value: float) -> void:
	var effects: Array = selected_definition().effects.duplicate(true)
	for effect in effects:
		if effect.effect == kind: effect[field] = value
	field_changed.emit(category, id, "effects", effects)
