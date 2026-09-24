extends Control

const DEFINITIONS_PATH := "res://data/definitions.toml"
const MAP_DIR := "res://data/maps/"
const BATTLE_SCENE := "res://features/battle/battle.tscn"
const MODES := ["地形筆刷", "放置單位", "移動單位", "刪除"]
const CATEGORIES := ["unit_types", "skills", "terrain_types"]

@onready var map_list: OptionButton = $Layout/Toolbar/Maps
@onready var map_name: LineEdit = $Layout/Body/MapPanel/MapFields/MapName
@onready var width_box: SpinBox = $Layout/Body/MapPanel/MapFields/Width
@onready var height_box: SpinBox = $Layout/Body/MapPanel/MapFields/Height
@onready var mode_list: OptionButton = $Layout/Body/MapPanel/Tools/Mode
@onready var terrain_list: OptionButton = $Layout/Body/MapPanel/Tools/Terrain
@onready var unit_list: OptionButton = $Layout/Body/MapPanel/Tools/Unit
@onready var team_list: OptionButton = $Layout/Body/MapPanel/Tools/Team
@onready var faction_field: LineEdit = $Layout/Body/MapPanel/Tools/Faction
@onready var grid: GridContainer = $Layout/Body/MapPanel/GridScroll/Grid
@onready var category_list: OptionButton = $Layout/Body/SidePanel/Category
@onready var definition_list: OptionButton = $Layout/Body/SidePanel/Definition
@onready var fields: GridContainer = $Layout/Body/SidePanel/InspectorScroll/Fields
@onready var status_label: Label = $Layout/Status

var core
var definitions: Dictionary = {}
var map_data: Dictionary = {}
var map_file := ""
var dirty := false
var selected_unit := ""
var history: Array = []
var future: Array = []
var refreshing := false

func _ready() -> void:
	core = TacticalGame.new()
	for i in MODES.size():
		mode_list.add_item(MODES[i])
	for value in ["player", "enemy"]:
		team_list.add_item(value)
	for value in ["單位", "技能", "地形"]:
		category_list.add_item(value)
	mode_list.select(0)
	team_list.select(0)
	category_list.select(0)
	$Layout/Toolbar/New.pressed.connect(new_map)
	$Layout/Toolbar/Open.pressed.connect(open_selected_map)
	$Layout/Toolbar/Save.pressed.connect(save_map)
	$Layout/Toolbar/Duplicate.pressed.connect(duplicate_map)
	$Layout/Toolbar/Undo.pressed.connect(undo)
	$Layout/Toolbar/Redo.pressed.connect(redo)
	$Layout/Toolbar/Play.pressed.connect(play_map)
	$Layout/Body/SidePanel/DefinitionActions/AddDefinition.pressed.connect(add_definition)
	$Layout/Body/SidePanel/DefinitionActions/RemoveDefinition.pressed.connect(remove_definition)
	category_list.item_selected.connect(func(_index: int): refresh_definitions())
	definition_list.item_selected.connect(func(_index: int): refresh_fields())
	map_name.text_submitted.connect(func(_value: String): commit_map_name())
	map_name.focus_exited.connect(commit_map_name)
	team_list.item_selected.connect(func(index: int): faction_field.visible = index != 0)
	faction_field.visible = false
	width_box.value_changed.connect(func(_value: float): resize_map())
	height_box.value_changed.connect(func(_value: float): resize_map())
	var source := read_file(DEFINITIONS_PATH)
	if source.is_empty():
		return
	var parsed: Dictionary = JSON.parse_string(core.definitions_to_json(source))
	if parsed.has("error"):
		show_error(parsed.error)
		return
	definitions = parsed
	refresh_map_list()
	if get_tree().root.has_meta("editor_session"):
		var session: Dictionary = get_tree().root.get_meta("editor_session")
		get_tree().root.remove_meta("editor_session")
		definitions = session.definitions
		map_data = session.map
		map_file = session.file
		dirty = session.dirty
		history = session.history
		future = session.future
		for i in map_list.item_count:
			if map_list.get_item_text(i) == map_file.get_file().trim_suffix(".toml"):
				map_list.select(i)
		refresh_ui()
		validate_current()
		return
	if map_list.item_count > 0:
		open_selected_map()
	else:
		new_map()

func read_file(path: String) -> String:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		show_error("無法讀取 %s：%s" % [path, error_string(FileAccess.get_open_error())])
		return ""
	return file.get_as_text()

func refresh_map_list() -> void:
	map_list.clear()
	for filename in DirAccess.get_files_at(MAP_DIR):
		if filename.ends_with(".toml"):
			map_list.add_item(filename.trim_suffix(".toml"))
	if map_list.item_count > 0: map_list.select(0)

func open_selected_map() -> void:
	if dirty:
		show_error("請先儲存目前修改，再開啟另一張地圖。")
		return
	if map_list.item_count == 0:
		return
	var path := MAP_DIR + map_list.get_item_text(map_list.selected) + ".toml"
	var source := read_file(path)
	if source.is_empty():
		return
	var parsed: Dictionary = JSON.parse_string(core.map_to_json(source))
	if parsed.has("error"):
		show_error(parsed.error)
		return
	map_data = parsed
	map_file = path
	history.clear()
	future.clear()
	selected_unit = ""
	dirty = false
	refresh_ui()
	validate_current()

func new_map() -> void:
	if dirty:
		show_error("請先儲存目前修改，再建立地圖。")
		return
	var costs := []
	costs.resize(80)
	costs.fill(1)
	map_data = {"name": "新地圖", "width": 10, "height": 8, "costs": costs, "triggers": [], "units": []}
	map_file = unique_map_path("new_map")
	history.clear()
	future.clear()
	selected_unit = ""
	dirty = true
	refresh_ui()
	validate_current()

func duplicate_map() -> void:
	if map_data.is_empty():
		return
	checkpoint()
	map_data = map_data.duplicate(true)
	map_data.name += " 副本"
	map_file = unique_map_path("map_copy")
	dirty = true
	refresh_ui()
	validate_current()

func unique_map_path(base: String) -> String:
	var index := 1
	while FileAccess.file_exists(MAP_DIR + base + "_%d.toml" % index):
		index += 1
	return MAP_DIR + base + "_%d.toml" % index

func save_map() -> void:
	var result := serialize_documents()
	if result.is_empty():
		return
	for path in [DEFINITIONS_PATH, map_file]:
		var file := FileAccess.open(path, FileAccess.WRITE)
		if file == null:
			show_error("無法寫入 %s：%s" % [path, error_string(FileAccess.get_open_error())])
			return
		file.store_string(result.definitions if path == DEFINITIONS_PATH else result.map)
	dirty = false
	refresh_map_list()
	for i in map_list.item_count:
		if map_list.get_item_text(i) == map_file.get_file().trim_suffix(".toml"):
			map_list.select(i)
	status_label.text = "已儲存 %s" % map_file

func play_map() -> void:
	var result := serialize_documents()
	if result.is_empty():
		return
	var players := 0
	var enemies := 0
	for unit in map_data.units:
		if unit.team is String: players += 1
		else: enemies += 1
	if players == 0 or enemies == 0:
		show_error("試玩需要至少一名玩家與一名敵方單位。")
		return
	get_tree().root.set_meta("editor_session", {"definitions": definitions.duplicate(true), "map": map_data.duplicate(true), "file": map_file, "dirty": dirty, "history": history.duplicate(true), "future": future.duplicate(true)})
	get_tree().root.set_meta("test_definitions", result.definitions)
	get_tree().root.set_meta("test_map", result.map)
	get_tree().change_scene_to_file(BATTLE_SCENE)

func serialize_documents() -> Dictionary:
	var parsed: Dictionary = JSON.parse_string(core.documents_from_json(JSON.stringify(definitions), JSON.stringify(map_data)))
	if parsed.has("error"):
		show_error(parsed.error)
		return {}
	return parsed

func validate_current() -> void:
	if serialize_documents().is_empty():
		return
	status_label.text = "資料有效%s" % ("；尚未儲存" if dirty else "")

func show_error(message: String) -> void:
	status_label.text = "錯誤：" + message

func checkpoint() -> void:
	history.append({"definitions": definitions.duplicate(true), "map": map_data.duplicate(true), "file": map_file})
	future.clear()
	dirty = true

func restore(snapshot: Dictionary) -> void:
	definitions = snapshot.definitions
	map_data = snapshot.map
	map_file = snapshot.file
	dirty = true
	refresh_ui()
	validate_current()

func undo() -> void:
	if history.is_empty(): return
	future.append({"definitions": definitions.duplicate(true), "map": map_data.duplicate(true), "file": map_file})
	restore(history.pop_back())

func redo() -> void:
	if future.is_empty(): return
	history.append({"definitions": definitions.duplicate(true), "map": map_data.duplicate(true), "file": map_file})
	restore(future.pop_back())

func commit_map_name() -> void:
	if refreshing or map_data.is_empty() or map_data.name == map_name.text: return
	checkpoint()
	map_data.name = map_name.text
	validate_current()

func resize_map() -> void:
	if refreshing or map_data.is_empty(): return
	var new_width := int(width_box.value)
	var new_height := int(height_box.value)
	if new_width == map_data.width and new_height == map_data.height: return
	checkpoint()
	var costs := []
	for y in new_height:
		for x in new_width:
			costs.append(map_data.costs[y * map_data.width + x] if x < map_data.width and y < map_data.height else 1)
	map_data.width = new_width
	map_data.height = new_height
	map_data.costs = costs
	map_data.triggers = map_data.triggers.filter(func(t: Dictionary): return t.x < new_width and t.y < new_height)
	map_data.units = map_data.units.filter(func(u: Dictionary): return u.x < new_width and u.y < new_height)
	refresh_grid()
	validate_current()

func refresh_ui() -> void:
	refreshing = true
	map_name.text = map_data.name
	width_box.value = map_data.width
	height_box.value = map_data.height
	refreshing = false
	refresh_tools()
	refresh_grid()
	refresh_definitions()

func refresh_tools() -> void:
	var previous := terrain_list.get_item_text(terrain_list.selected) if terrain_list.item_count else "plain"
	terrain_list.clear()
	for kind in ["plain", "rough"]:
		terrain_list.add_item(kind)
	var keys: Array = definitions.terrain_types.keys()
	keys.sort()
	for kind in keys:
		if kind not in ["plain", "rough"]: terrain_list.add_item(kind)
	for i in terrain_list.item_count:
		if terrain_list.get_item_text(i) == previous: terrain_list.select(i)
	if terrain_list.selected < 0: terrain_list.select(0)
	unit_list.clear()
	for kind in definitions.unit_types:
		unit_list.add_item(kind.id)
	if unit_list.item_count > 0: unit_list.select(0)

func refresh_grid() -> void:
	for child in grid.get_children(): child.queue_free()
	grid.columns = int(map_data.width)
	for y in int(map_data.height):
		for x in int(map_data.width):
			var cell := Vector2i(x, y)
			var kind := terrain_at(cell)
			var unit := unit_at(cell)
			var button := Button.new()
			button.custom_minimum_size = Vector2(74, 56)
			button.text = "%d,%d\n%s%s" % [x, y, kind, "\n" + unit.id if not unit.is_empty() else ""]
			button.tooltip_text = "移動消耗 %d；%s" % [map_data.costs[y * map_data.width + x], kind]
			button.modulate = Color("c8b9a4") if kind == "rough" else Color.WHITE
			if kind not in ["plain", "rough"]: button.modulate = Color("e98c82")
			if not unit.is_empty(): button.modulate = Color("79b9f3") if unit.team is String else Color("ee9b94")
			button.pressed.connect(func(): edit_cell(cell))
			button.mouse_entered.connect(func():
				if Input.is_mouse_button_pressed(MOUSE_BUTTON_LEFT) and mode_list.selected == 0: edit_cell(cell))
			grid.add_child(button)

func terrain_at(cell: Vector2i) -> String:
	for t in map_data.triggers:
		if t.x == cell.x and t.y == cell.y: return t.kind
	return "rough" if map_data.costs[cell.y * map_data.width + cell.x] > 1 else "plain"

func unit_at(cell: Vector2i) -> Dictionary:
	for unit in map_data.units:
		var kind := unit_type(unit.unit_type)
		if cell.x >= unit.x and cell.x < unit.x + kind.get("width", 1) and cell.y >= unit.y and cell.y < unit.y + kind.get("height", 1): return unit
	return {}

func unit_type(id: String) -> Dictionary:
	for kind in definitions.unit_types:
		if kind.id == id: return kind
	return {}

func edit_cell(cell: Vector2i) -> void:
	var mode := mode_list.selected
	var current := unit_at(cell)
	if mode == 0:
		var kind := terrain_list.get_item_text(terrain_list.selected)
		if terrain_at(cell) == kind: return
		checkpoint()
		map_data.triggers = map_data.triggers.filter(func(t: Dictionary): return t.x != cell.x or t.y != cell.y)
		map_data.costs[cell.y * map_data.width + cell.x] = 2 if kind == "rough" else 1
		if kind not in ["plain", "rough"]: map_data.triggers.append({"x":cell.x,"y":cell.y,"kind":kind})
	elif mode == 1:
		if unit_list.item_count == 0: return
		checkpoint()
		var kind := unit_list.get_item_text(unit_list.selected)
		var id := kind + "_1"
		var number := 1
		while map_data.units.any(func(u: Dictionary): return u.id == id):
			number += 1
			id = kind + "_%d" % number
		var team: Variant = "player" if team_list.selected == 0 else {"enemy": faction_field.text.strip_edges()}
		map_data.units.append({"id":id,"unit_type":kind,"team":team,"x":cell.x,"y":cell.y})
	elif mode == 2:
		if selected_unit.is_empty():
			if current.is_empty(): return
			selected_unit = current.id
			status_label.text = "選取 %s；點擊目標格移動。" % selected_unit
			return
		checkpoint()
		for unit in map_data.units:
			if unit.id == selected_unit:
				unit.x = cell.x
				unit.y = cell.y
		selected_unit = ""
	else:
		if current.is_empty() and terrain_at(cell) == "plain": return
		checkpoint()
		if not current.is_empty():
			map_data.units = map_data.units.filter(func(u: Dictionary): return u.id != current.id)
		else:
			map_data.triggers = map_data.triggers.filter(func(t: Dictionary): return t.x != cell.x or t.y != cell.y)
			map_data.costs[cell.y * map_data.width + cell.x] = 1
	refresh_grid()
	validate_current()

func category_key() -> String:
	return CATEGORIES[category_list.selected]

func refresh_definitions() -> void:
	definition_list.clear()
	var key := category_key()
	if key == "terrain_types":
		var names: Array = definitions.terrain_types.keys()
		names.sort()
		for name in names: definition_list.add_item(name)
	else:
		for entry in definitions[key]: definition_list.add_item(entry.id)
	if definition_list.item_count > 0: definition_list.select(0)
	refresh_fields()

func selected_definition() -> Dictionary:
	if definition_list.item_count == 0: return {}
	var id := definition_list.get_item_text(definition_list.selected)
	if category_key() == "terrain_types": return definitions.terrain_types[id]
	for entry in definitions[category_key()]:
		if entry.id == id: return entry
	return {}

func refresh_fields() -> void:
	for child in fields.get_children(): child.queue_free()
	var entry := selected_definition()
	if entry.is_empty(): return
	var display := entry.duplicate()
	if category_key() == "skills":
		for optional in {"duration": 2, "heal_amount": 5, "terrain": "mire"}:
			if not display.has(optional): display[optional] = {"duration": 2, "heal_amount": 5, "terrain": "mire"}[optional]
	for key in display.keys():
		var label := Label.new()
		label.text = key
		fields.add_child(label)
		var value = display[key]
		if value is bool:
			var check := CheckBox.new()
			check.button_pressed = value
			check.toggled.connect(func(v: bool): update_definition(key, v))
			fields.add_child(check)
		else:
			var edit := LineEdit.new()
			edit.custom_minimum_size.x = 180
			edit.text = ",".join(PackedStringArray(value)) if value is Array else str(value)
			edit.editable = key != "id"
			edit.text_submitted.connect(func(_v: String): commit_field(key, edit.text, value))
			edit.focus_exited.connect(func(): commit_field(key, edit.text, value))
			fields.add_child(edit)

func commit_field(key: String, text: String, previous) -> void:
	var value = text
	if previous is int:
		if not text.is_valid_int():
			show_error("%s 必須是整數" % key)
			return
		value = int(text)
	elif previous is Array:
		value = Array(text.split(",", false)).map(func(s: String): return s.strip_edges())
	update_definition(key, value)

func update_definition(key: String, value) -> void:
	var entry := selected_definition()
	if entry.is_empty() or (entry.has(key) and entry[key] == value): return
	checkpoint()
	entry[key] = value
	refresh_tools()
	refresh_grid()
	validate_current()

func add_definition() -> void:
	var category := category_key()
	var base_id := "new_unit" if category == "unit_types" else "new_skill" if category == "skills" else "new_terrain"
	var id := base_id
	var count := 1
	while definition_list_has(id):
		count += 1
		id = base_id + "_%d" % count
	checkpoint()
	if category == "unit_types":
		definitions.unit_types.append({"id":id,"name":id,"visual":"res://assets/units/fighter.svg","width":1,"height":1,"hp":10,"movement":5,"initiative":0,"dodge":2,"block":2,"attack":3,"damage":3,"skills":["melee_attack"]})
	elif category == "skills":
		definitions.skills.append({"id":id,"name":id,"ranged":false,"attack_bonus":0,"damage_bonus":0,"min_range":1,"max_range":1,"effect":"attack","ai_default":false})
	else:
		definitions.terrain_types[id] = {"name_key":"TERRAIN_PLAIN","visual":"plain","passable":true,"ends_movement":false,"damage":0,"movement_cost_bonus":0,"dodge_penalty":0,"block_penalty":0,"forced_entry":"none","effect_key":"TERRAIN_EFFECT_NONE"}
	refresh_tools()
	refresh_definitions()
	definition_list.select(definition_list.item_count - 1)
	refresh_fields()
	validate_current()

func definition_list_has(id: String) -> bool:
	for i in definition_list.item_count:
		if definition_list.get_item_text(i) == id: return true
	return false

func remove_definition() -> void:
	if definition_list.item_count == 0: return
	var id := definition_list.get_item_text(definition_list.selected)
	checkpoint()
	if category_key() == "terrain_types":
		definitions.terrain_types.erase(id)
	else:
		definitions[category_key()] = definitions[category_key()].filter(func(entry: Dictionary): return entry.id != id)
	refresh_tools()
	refresh_definitions()
	refresh_grid()
	validate_current()
