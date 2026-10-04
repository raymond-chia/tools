extends Control

const DEFINITIONS_PATH := "res://data/definitions.toml"
const MAP_DIR := "res://data/maps/"
const BATTLE_SCENE := preload("res://features/battle/battle.tscn")
const MODES := ["繪製地面", "繪製上層", "放置單位", "移動單位", "刪除單位"]
const TILE_PREVIEW := preload("res://features/editor/terrain_preview.gd")

@onready var map_list: ItemList = $Layout/Pages/MapPage/MapListPanel/Maps
@onready var map_name: LineEdit = $Layout/Pages/MapPage/MapPanel/MapFields/MapName
@onready var width_box: SpinBox = $Layout/Pages/MapPage/MapPanel/MapFields/Width
@onready var height_box: SpinBox = $Layout/Pages/MapPage/MapPanel/MapFields/Height
@onready var mode_list: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/Mode
@onready var palette_tabs: TabContainer = $Layout/Pages/MapPage/Materials/PaletteTabs
@onready var terrain_grids: Array[GridContainer] = [$Layout/Pages/MapPage/Materials/PaletteTabs/Ground/Tiles, $Layout/Pages/MapPage/Materials/PaletteTabs/Overlay/Tiles]
@onready var brush_label: Label = $Layout/Pages/MapPage/Materials/Brush
@onready var unit_list: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/Unit
@onready var team_list: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/Team
@onready var faction_field: LineEdit = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/Faction
@onready var map_view = $Layout/Pages/MapPage/MapPanel/MapViewportContainer/MapViewport/Map
@onready var pages: TabContainer = $Layout/Pages
@onready var definition_pages: Array = [$Layout/Pages/UnitPage, $Layout/Pages/SkillPage, $Layout/Pages/TerrainPage]
@onready var status_label: Label = $Layout/Status

var editing_mode := 0
var terrain_brushes := {"ground": "", "overlay": ""}

var core
var definitions: Dictionary = {}
var map_data: Dictionary = {}
var map_file := ""
var dirty := false
var pending_edit: Dictionary = {}
var history: Array = []
var future: Array = []
var duplicating_map := false
var refreshing := false
var selected_cell := Vector2i(-1, -1)
var stroke_checkpointed := false
var input_error_count := 0

func _process(delta: float) -> void:
	if pages.current_tab != 0: return
	var focused := get_viewport().gui_get_focus_owner()
	if focused is LineEdit or focused is TextEdit:
		return
	map_view.move_camera(delta)

func _ready() -> void:
	core = TacticalGame.new()
	for i in range(2, MODES.size()):
		mode_list.add_item(MODES[i])
	for index in 3:
		palette_tabs.set_tab_title(index, ["地面", "上層", "單位"][index])
	for value in ["player", "enemy"]:
		team_list.add_item(value)
	mode_list.select(0)
	team_list.select(0)
	for index in [0, 1, 2, 3]:
		pages.set_tab_title(index, ["地圖與單位配置", "單位", "技能", "地形"][index])
	for page in definition_pages:
		page.field_changed.connect(update_definition)
		page.move_requested.connect(move_definition)
		page.terrain_effect_confirmed.connect(change_terrain_effect)
		page.create_requested.connect(create_definition)
		page.remove_requested.connect(request_remove_definition)
		page.remove_confirmed.connect(remove_definition)
		page.input_error.connect(show_error)
	map_view.cell_pressed.connect(edit_cell)
	map_view.inspection_clicked.connect(inspect_unit)
	map_view.stroke_started.connect(func(): stroke_checkpointed = false)
	map_view.unit_dropped.connect(move_unit)
	mode_list.item_selected.connect(func(_index: int): update_editing_mode())
	palette_tabs.tab_changed.connect(func(_index: int): update_editing_mode())
	map_list.item_selected.connect(func(_index: int): open_selected_map())
	$Layout/Pages/MapPage/MapListPanel/MapActions/New.pressed.connect(new_map)
	$Layout/Toolbar/Save.pressed.connect(save_map)
	$Layout/Pages/MapPage/MapListPanel/MapActions/Duplicate.pressed.connect(duplicate_map)
	$AddMapDialog.confirmed.connect(submit_map)
	$AddMapDialog/Form/Filename.text_submitted.connect(func(_text: String): submit_map())
	$Layout/Pages/MapPage/MapListPanel/MapActions/Delete.pressed.connect(request_delete_map)
	$DeleteMapDialog.confirmed.connect(delete_map)
	$Layout/Toolbar/Undo.pressed.connect(undo)
	$Layout/Toolbar/Redo.pressed.connect(redo)
	$Layout/Toolbar/Play.pressed.connect(play_map)
	map_name.text_submitted.connect(func(_value: String): commit_map_name())
	map_name.focus_exited.connect(commit_map_name)
	team_list.item_selected.connect(func(_index: int): refresh_tools_visibility())
	faction_field.visible = false
	width_box.value_changed.connect(func(_value: float): resize_map())
	height_box.value_changed.connect(func(_value: float): resize_map())
	var source := read_file(DEFINITIONS_PATH)
	if source.is_empty():
		return
	var parsed := read_core_response(core.definitions_to_json(source))
	if parsed.is_empty():
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
		select_map(map_file)
		refresh_ui()
		pages.current_tab = session.get("page", 0)
		var selections: Array = session.get("selections", [])
		for index in selections.size(): definition_pages[index].select_id(selections[index])
		validate_current()
		return
	if map_list.item_count > 0:
		open_selected_map()
	else:
		refresh_ui()
		new_map()

func read_file(path: String) -> String:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		show_error("無法讀取 %s：%s" % [path, error_string(FileAccess.get_open_error())])
		return ""
	return file.get_as_text()

func refresh_map_list() -> void:
	var previous := selected_map()
	map_list.clear()
	for filename in DirAccess.get_files_at(MAP_DIR):
		if not filename.ends_with(".toml"): continue
		var path := MAP_DIR + filename
		var index := map_list.add_item(filename)
		map_list.set_item_metadata(index, path)
	if not map_file.is_empty() and not FileAccess.file_exists(map_file):
		var index := map_list.add_item(map_file.get_file())
		map_list.set_item_metadata(index, map_file)
	select_map(previous)
	if selected_map().is_empty() and map_list.item_count > 0: map_list.select(0)

func selected_map() -> String:
	var selected := map_list.get_selected_items()
	return map_list.get_item_metadata(selected[0]) if not selected.is_empty() else ""

func select_map(path: String) -> void:
	map_list.deselect_all()
	for index in map_list.item_count:
		if map_list.get_item_metadata(index) == path:
			map_list.select(index)
			map_list.ensure_current_is_visible()
			return

func selected_text(list: OptionButton) -> String:
	return list.get_item_text(list.selected) if list.selected >= 0 else ""

# 重建選單後選回同名項目；找不到時選第一項。
func select_text(list: OptionButton, text: String) -> void:
	for i in list.item_count:
		if list.get_item_text(i) == text:
			list.select(i)
			return
	if list.item_count > 0: list.select(0)

func open_selected_map() -> void:
	if selected_map() == map_file: return
	if dirty:
		select_map(map_file)
		show_error("請先儲存目前修改，再開啟另一張地圖。")
		return
	if map_list.item_count == 0:
		return
	var path := selected_map()
	var source := read_file(path)
	if source.is_empty():
		return
	var parsed := read_core_response(core.map_to_json(source))
	if parsed.is_empty():
		return
	map_data = parsed
	map_view.camera_initialized = false
	map_file = path
	history.clear()
	future.clear()
	selected_cell = Vector2i(-1, -1)
	map_view.inspected_unit = 0
	dirty = false
	refresh_ui()
	validate_current()

func new_map() -> void:
	open_map_dialog(false)

func duplicate_map() -> void:
	if map_data.is_empty(): return
	open_map_dialog(true)

func open_map_dialog(duplicate: bool) -> void:
	if dirty and not duplicate:
		show_error("請先儲存目前修改，再新增地圖。")
		return
	duplicating_map = duplicate
	$AddMapDialog.title = "複製地圖" if duplicate else "新增地圖"
	$AddMapDialog.ok_button_text = "複製" if duplicate else "新增"
	$AddMapDialog/Form/Filename.text = map_file.get_file().get_basename() + "_copy" if duplicate else ""
	$AddMapDialog/Form/Error.text = ""
	$AddMapDialog.popup_centered()
	$AddMapDialog/Form/Filename.grab_focus()
	$AddMapDialog/Form/Filename.select_all()

func submit_map() -> void:
	var filename: String = $AddMapDialog/Form/Filename.text.strip_edges()
	if filename.ends_with(".toml"): filename = filename.trim_suffix(".toml")
	if not filename.is_valid_filename() or filename in [".", ".."]:
		$AddMapDialog/Form/Error.text = "請輸入有效的檔案名稱，不可包含路徑或檔名禁用字元。"
		return
	var path := MAP_DIR + filename + ".toml"
	if FileAccess.file_exists(path):
		$AddMapDialog/Form/Error.text = "檔案「%s.toml」已存在。" % filename
		return
	if duplicating_map:
		map_data = map_data.duplicate(true)
		map_data.name += " 副本"
	else:
		map_data = {"name": filename, "width": 10, "height": 8, "terrains": [], "units": []}
	map_file = path
	map_view.camera_initialized = false
	history.clear()
	future.clear()
	selected_cell = Vector2i(-1, -1)
	map_view.inspected_unit = 0
	dirty = true
	$AddMapDialog.hide()
	refresh_ui()
	validate_current()

func save_map() -> void:
	if map_data.is_empty(): return
	var errors_before_commit := input_error_count
	for page in definition_pages:
		page.commit_pending_fields(func(): return input_error_count != errors_before_commit)
		if input_error_count != errors_before_commit: return
	var result := serialize_documents()
	if result.is_empty(): return
	var documents := {DEFINITIONS_PATH: result.definitions, map_file: result.map}
	for path in documents:
		var file := FileAccess.open(path, FileAccess.WRITE)
		if file == null:
			show_error("無法寫入 %s：%s" % [path, error_string(FileAccess.get_open_error())])
			return
		file.store_string(documents[path])
	dirty = false
	refresh_map_list()
	select_map(map_file)
	status_label.text = "已儲存 %s" % map_file

func request_delete_map() -> void:
	if map_data.is_empty(): return
	if dirty:
		show_error("請先儲存目前修改，再刪除地圖。")
		return
	$DeleteMapDialog.dialog_text = "確定刪除地圖「%s」？此操作會刪除地圖檔案。" % map_file.get_file()
	$DeleteMapDialog.popup_centered()

func delete_map() -> void:
	var error := DirAccess.remove_absolute(map_file)
	if error != OK:
		show_error("無法刪除 %s：%s" % [map_file, error_string(error)])
		return
	map_file = ""
	map_data = {}
	history.clear()
	future.clear()
	refresh_map_list()
	if map_list.item_count > 0: open_selected_map()
	else:
		refresh_ui()
		new_map()

func play_map() -> void:
	if map_data.is_empty(): return
	var result := serialize_documents()
	if result.is_empty():
		return
	get_tree().root.set_meta("editor_session", {"definitions": definitions.duplicate(true), "map": map_data.duplicate(true), "file": map_file, "dirty": dirty, "history": history.duplicate(true), "future": future.duplicate(true), "page": pages.current_tab, "selections": definition_pages.map(func(page): return page.selected_id())})
	var battle := BATTLE_SCENE.instantiate()
	battle.configure(result.definitions, result.map, true)
	get_tree().change_scene_to_node(battle)

func serialize_documents() -> Dictionary:
	return read_core_response(core.documents_from_json(JSON.stringify(definitions), JSON.stringify(map_data)))

func validate_current() -> void:
	var snapshot := read_core_response(core.inspected_preview_from_json(JSON.stringify(definitions), JSON.stringify(map_data), map_view.inspected_unit if map_view.inspected_unit != 0 else null))
	if snapshot.is_empty():
		if not pending_edit.is_empty():
			var error_message := status_label.text
			definitions = pending_edit.definitions
			map_data = pending_edit.map
			map_file = pending_edit.file
			dirty = pending_edit.dirty
			future = pending_edit.future
			history.resize(pending_edit.history_size)
			stroke_checkpointed = pending_edit.stroke_checkpointed
			pending_edit = {}
			refresh_ui()
			refresh_grid()
			status_label.text = error_message
		return
	pending_edit = {}
	map_view.present(map_data, snapshot, selected_cell)
	status_label.text = "資料有效%s" % ("；尚未儲存" if dirty else "")

# 自製編輯器顯示核心原始診斷；正式遊戲維持共用讀取函式的翻譯訊息。
func read_core_response(response: String) -> Dictionary:
	return CoreResponse.read(response, show_error, [], true)

func show_error(message: String) -> void:
	input_error_count += 1
	status_label.text = "錯誤：" + message
	for page in definition_pages: page.show_add_error(message)

# 保存每次輸入前的狀態；無效輸入不消耗復原紀錄，也不清除重做紀錄。
func begin_edit() -> void:
	pending_edit = {"definitions": definitions.duplicate(true), "map": map_data.duplicate(true), "file": map_file, "dirty": dirty, "future": future.duplicate(true), "history_size": history.size(), "stroke_checkpointed": stroke_checkpointed}

func checkpoint() -> void:
	begin_edit()
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
	refresh_map_list()
	select_map(map_file)

func resize_map() -> void:
	if refreshing or map_data.is_empty(): return
	var new_width := int(width_box.value)
	var new_height := int(height_box.value)
	if new_width == map_data.width and new_height == map_data.height: return
	checkpoint()
	map_data.width = new_width
	map_data.height = new_height
	# 只刪除超出範圍的地形；不處理單位，避免縮小地圖時不小心改到單位，超界單位交由核心驗證回報。
	map_data.terrains = map_data.terrains.filter(func(t: Dictionary): return t.x < new_width and t.y < new_height)
	validate_current()

func refresh_ui() -> void:
	var has_map := not map_data.is_empty()
	$Layout/Pages/MapPage/MapPanel.visible = has_map
	$Layout/Pages/MapPage/Materials.visible = has_map
	$Layout/Pages/MapPage/MapListPanel/MapActions/Duplicate.disabled = not has_map
	$Layout/Pages/MapPage/MapListPanel/MapActions/Delete.disabled = not has_map
	refreshing = true
	map_name.text = map_data.get("name", "")
	width_box.value = map_data.get("width", 10)
	height_box.value = map_data.get("height", 8)
	refreshing = false
	refresh_map_list()
	select_map(map_file)
	refresh_tools()
	refresh_tools_visibility()
	refresh_definitions()

func refresh_tools() -> void:
	refresh_terrain_list()
	var previous := selected_text(unit_list)
	unit_list.clear()
	for kind in definitions.unit_types:
		unit_list.add_item(kind.id)
	select_text(unit_list, previous)

# 素材面板只選擇作者要寫入的地形 ID；地形規則與預覽仍由核心決定。
func refresh_terrain_list() -> void:
	for index in terrain_grids.size():
		var layer := "ground" if index == 0 else "overlay"
		var grid := terrain_grids[index]
		for child in grid.get_children():
			grid.remove_child(child)
			child.queue_free()
		var kinds := terrain_kinds(layer)
		if layer == "overlay": kinds.push_front("")
		if not kinds.has(terrain_brushes[layer]):
			terrain_brushes[layer] = kinds[0] if not kinds.is_empty() else ""
		var group := ButtonGroup.new()
		for kind in kinds:
			var button := Button.new()
			button.custom_minimum_size = Vector2(120, 90)
			button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			button.toggle_mode = true
			button.button_group = group
			button.button_pressed = terrain_brushes[layer] == kind
			button.tooltip_text = kind if not kind.is_empty() else "清除上層"
			var layout := VBoxContainer.new()
			layout.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
			layout.mouse_filter = Control.MOUSE_FILTER_IGNORE
			button.add_child(layout)
			var preview := Control.new()
			preview.set_script(TILE_PREVIEW)
			preview.visual = kind if not kind.is_empty() else ""
			preview.layer = layer
			preview.custom_minimum_size = Vector2(64, 52)
			preview.mouse_filter = Control.MOUSE_FILTER_IGNORE
			layout.add_child(preview)
			var label := Label.new()
			label.text = kind if not kind.is_empty() else "橡皮擦"
			label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			label.mouse_filter = Control.MOUSE_FILTER_IGNORE
			layout.add_child(label)
			button.pressed.connect(func():
				terrain_brushes[layer] = kind
				update_brush_label())
			grid.add_child(button)
	update_brush_label()

func update_editing_mode() -> void:
	editing_mode = palette_tabs.current_tab if palette_tabs.current_tab < 2 else mode_list.selected + 2
	refresh_tools_visibility()
	update_brush_label()

func update_brush_label() -> void:
	if editing_mode > 1:
		brush_label.text = MODES[editing_mode]
		return
	var layer := "ground" if editing_mode == 0 else "overlay"
	var kind: String = terrain_brushes[layer]
	brush_label.text = "目前筆刷：" + (kind if not kind.is_empty() else "橡皮擦" if editing_mode == 1 else "無素材")

func terrain_kinds(layer: String) -> Array:
	var kinds: Array = definitions.terrain_types.filter(func(terrain: Dictionary): return terrain.layer == layer).map(func(terrain: Dictionary): return terrain.id)
	return kinds

func refresh_grid() -> void:
	var snapshot := read_core_response(core.inspected_preview_from_json(JSON.stringify(definitions), JSON.stringify(map_data), map_view.inspected_unit if map_view.inspected_unit != 0 else null))
	if snapshot.is_empty(): return
	map_view.present(map_data, snapshot, selected_cell)

func refresh_tools_visibility() -> void:
	# 工具切換只更新介面狀態；資料修改的驗證與預覽由 validate_current 統一處理。
	var mode := editing_mode
	map_view.drag_paint = mode <= 1
	unit_list.visible = mode == 2
	team_list.visible = mode == 2
	faction_field.visible = mode == 2 and team_list.selected != 0

# 筆刷只傳遞作者選擇；圖層替換、清除與合法性由 Rust 決定。
func replace_layer(cell: Vector2i, layer: String, kind: String) -> void:
	var command := {"x": cell.x, "y": cell.y, "layer": layer, "kind": kind}
	var result := read_core_response(core.paint_terrain_from_json(JSON.stringify(definitions), JSON.stringify(map_data), JSON.stringify(command)))
	if result.is_empty() or not result.changed: return
	checkpoint_stroke()
	map_data = result.map
	pending_edit = {}
	CoreResponse.convert_unit_ids(result.snapshot)
	if map_view.inspected_unit != 0:
		refresh_grid()
	else:
		map_view.present(map_data, result.snapshot, selected_cell)
	status_label.text = "資料有效；尚未儲存"

func unit_at(cell: Vector2i) -> Dictionary:
	# 佔用格由核心決定；編輯器只選取 snapshot 中被點擊的單位。
	for unit in map_view.snapshot.get("units", []):
		for occupied in unit.occupied_cells:
			if Vector2i(occupied.x, occupied.y) == cell: return unit
	return {}

func inspect_unit(cell: Vector2i) -> void:
	var unit := unit_at(cell)
	var unit_id: int = 0 if unit.is_empty() else unit.id
	map_view.inspected_unit = 0 if unit_id == map_view.inspected_unit else unit_id
	refresh_grid()

func edit_cell(cell: Vector2i) -> void:
	selected_cell = cell
	var mode := editing_mode
	var current := unit_at(cell)
	if mode <= 1:
		var layer := "ground" if mode == 0 else "overlay"
		var kind: String = terrain_brushes[layer]
		if mode == 0 and kind.is_empty(): return
		replace_layer(cell, layer, kind)
		return
	elif mode == 2:
		if unit_list.item_count == 0: return
		checkpoint()
		var kind := unit_list.get_item_text(unit_list.selected)
		var id := 1
		while map_data.units.any(func(u: Dictionary): return u.id == id):
			id += 1
		var team: Variant = "player" if team_list.selected == 0 else {"enemy": faction_field.text.strip_edges()}
		map_data.units.append({"id":id,"unit_type":kind,"team":team,"x":cell.x,"y":cell.y})
	elif mode == 3:
		if current.is_empty(): return
		map_view.begin_unit_drag(current, cell)
		return
	else:
		if current.is_empty(): return
		checkpoint()
		map_data.units = map_data.units.filter(func(u: Dictionary): return u.id != current.id)
	validate_current()

func move_unit(id: int, cell: Vector2i) -> void:
	selected_cell = cell
	for unit in map_data.units:
		if unit.id != id: continue
		if Vector2i(unit.x, unit.y) == cell: return
		checkpoint()
		unit.x = cell.x
		unit.y = cell.y
		validate_current()
		return

func checkpoint_stroke() -> void:
	if stroke_checkpointed:
		begin_edit()
		return
	checkpoint()
	stroke_checkpointed = true

func refresh_definitions() -> void:
	var options := read_core_response(core.edit_definition_from_json(JSON.stringify(definitions), "{}", JSON.stringify({"action": "skill_effect_options"})))
	if options.is_empty(): return
	for page in definition_pages: page.present(definitions, options.terrain_ids, options.terrain_entries)

func find_definition(category: String, id: String) -> Dictionary:
	for entry in definitions[category]:
		if entry.id == id: return entry
	return {}

func move_definition(category: String, id: String, offset: int) -> void:
	edit_definition({"action": "move", "category": category, "id": id, "offset": offset})

func update_definition(category: String, id: String, key: String, value: Variant) -> void:
	# ID 建立後不可更動，因此不需要更新既有引用。
	if key == "id": return
	var entry := find_definition(category, id)
	if entry.is_empty() or (entry.has(key) and entry[key] == value): return
	if category == "skills" and key == "effect":
		if value == "mire":
			for page in definition_pages:
				if page.category == "skills": page.choose_effect_terrain(id)
		else:
			if not edit_definition({"action": "change_skill_effect", "id": id, "effect": value}):
				refresh_definitions()
		return
	# 失敗時保留作者輸入，讓儲存前重新提交並阻止寫入舊值。
	edit_definition({"action": "update_field", "category": category, "id": id, "key": key, "value": value})

# ID 建立後不可更動，因此不需要更新既有引用；刪除檢查仍涵蓋所有地圖。
func edit_definition(command: Dictionary) -> bool:
	var maps := {}
	for filename in DirAccess.get_files_at(MAP_DIR):
		if not filename.ends_with(".toml"): continue
		var path := MAP_DIR + filename
		if path == map_file: continue
		var parsed := read_core_response(core.map_to_json(read_file(path)))
		if parsed.is_empty(): return false
		maps[path] = parsed
	if not map_file.is_empty(): maps[map_file] = map_data
	var response: String = core.edit_definition_from_json(JSON.stringify(definitions), JSON.stringify(maps), JSON.stringify(command))
	var parsed = JSON.parse_string(response)
	if parsed is Dictionary and parsed.has("map_path"):
		show_error("地圖「%s」：%s: %s" % [str(parsed.map_path).get_file(), parsed.error_id, parsed.error])
		return false
	if parsed is Dictionary and parsed.get("error_id") == "authoring_edit":
		show_definition_error(parsed.error_details)
		if command.action in ["check_remove", "remove"]:
			for page in definition_pages:
				if page.category == command.category: page.show_remove_error(status_label.text)
		return false
	var result := read_core_response(response)
	if result.is_empty(): return false
	if command.action == "check_remove": return true
	checkpoint()
	definitions = result.definitions
	refresh_ui()
	validate_current()
	return true

func show_definition_error(details: Dictionary) -> void:
	var id: String = details.id
	match details.kind:
		"empty_id": show_error("ID 不可為空。")
		"duplicate_id": show_error("ID「%s」已存在。" % id)
		"not_found": show_error("找不到 ID「%s」。" % id)
		"default_ground": show_error("「%s」是核心保留的預設地形 ID，不能刪除。" % id)
		"referenced":
			var references: Array[String] = []
			for reference in details.references:
				var category: String = {"unit_types": "單位", "skills": "技能", "maps": "地圖"}[reference.category]
				var reference_id: String = reference.id
				if reference.category == "maps": reference_id = reference_id.get_file()
				references.append("• %s「%s」" % [category, reference_id])
			show_error("ID「%s」仍被以下資料引用，不能刪除：\n%s\n請先移除引用。" % [id, "\n".join(references)])
		_: show_error("無法處理資料操作：%s" % id)

func create_definition(category: String, id: String, source_id: String) -> void:
	var command := {"action": "add", "category": category, "id": id}
	if not source_id.is_empty():
		command.action = "duplicate"
		command.source_id = source_id
	if not edit_definition(command): return
	for page in definition_pages:
		if page.category == category:
			page.finish_add()
			page.select_id(id)

func request_remove_definition(category: String, id: String) -> void:
	if id.is_empty(): return
	if not edit_definition({"action": "check_remove", "category": category, "id": id}): return
	for page in definition_pages:
		if page.category == category: page.confirm_remove(id)

func remove_definition(category: String, id: String) -> void:
	if id.is_empty(): return
	edit_definition({"action": "remove", "category": category, "id": id})

func change_terrain_effect(id: String, terrain: String) -> void:
	if not edit_definition({"action": "change_skill_effect", "id": id, "effect": "mire", "terrain": terrain}): return
	for page in definition_pages:
		if page.category == "skills": page.finish_effect_change()
