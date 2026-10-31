extends Control

const BATTLE_SCENE := preload("res://features/battle/battle.tscn")
enum EditMode { NONE, GROUND, OVERLAY, PLACE_UNIT, MOVE_UNIT, DELETE_UNIT, CARGO }
# 依節點名稱對應，不依分頁順序。
const PALETTE_TABS := {
	"Ground": {"title": "地面", "mode": EditMode.GROUND},
	"Overlay": {"title": "上層", "mode": EditMode.OVERLAY},
	"Units": {"title": "單位"},
	"Outcomes": {"title": "勝敗條件", "mode": EditMode.NONE},
	"Cargo": {"title": "貨物", "mode": EditMode.CARGO},
}
const UNIT_MODES := {"放置單位": EditMode.PLACE_UNIT, "移動單位": EditMode.MOVE_UNIT, "刪除單位": EditMode.DELETE_UNIT}
const BRUSH_LAYERS := {EditMode.GROUND: "ground", EditMode.OVERLAY: "overlay"}
const TILE_PREVIEW := preload("res://features/editor/terrain_preview.gd")

# 勾選項對應作者資料；條件是否成立由核心判定。
const OUTCOME_CONDITIONS: Dictionary = {
	"all_enemies_defeated": {"type": "all_enemies_defeated"},
	"player_defeated": {"type": "team_defeated", "team": "player"},
	"cargo_escaped": {"type": "cargo_escaped"},
	"no_cargo_carried": {"type": "no_cargo_carried"},
}

@onready var map_list: ItemList = $Layout/Pages/MapPage/MapListPanel/Maps
@onready var map_name: LineEdit = $Layout/Pages/MapPage/MapPanel/MapFields/MapName
@onready var width_box: SpinBox = $Layout/Pages/MapPage/MapPanel/MapFields/Width
@onready var height_box: SpinBox = $Layout/Pages/MapPage/MapPanel/MapFields/Height
@onready var mode_list: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/Mode
@onready var palette_tabs: TabContainer = $Layout/Pages/MapPage/Materials/PaletteTabs
@onready var terrain_grids: Array[GridContainer] = [$Layout/Pages/MapPage/Materials/PaletteTabs/Ground/Scroll/Tiles, $Layout/Pages/MapPage/Materials/PaletteTabs/Overlay/Scroll/Tiles]
@onready var outcome_groups: Array[VBoxContainer] = [$Layout/Pages/MapPage/Materials/PaletteTabs/Outcomes/Groups/Victory, $Layout/Pages/MapPage/Materials/PaletteTabs/Outcomes/Groups/Defeat]
@onready var cargo_tool: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Cargo/Tool
@onready var cargo_summary: Label = $Layout/Pages/MapPage/Materials/PaletteTabs/Cargo/Summary
@onready var brush_size: SpinBox = $Layout/Pages/MapPage/Materials/PaletteTabs/Ground/BrushSize
@onready var brush_labels: Array[Label] = [$Layout/Pages/MapPage/Materials/PaletteTabs/Ground/Brush, $Layout/Pages/MapPage/Materials/PaletteTabs/Overlay/Brush]
@onready var unit_list: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/Unit
@onready var placement_difficulty: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/MinimumDifficulty
@onready var placement_facing: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/Facing
@onready var inspected_facing: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/InspectedFacing
@onready var inspected_unit_type: Label = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/InspectedUnitType
@onready var inspected_difficulty: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/InspectedDifficulty
@onready var play_difficulty: OptionButton = $Layout/Toolbar/PlayDifficulty
@onready var team_list: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/Team
@onready var faction_field: LineEdit = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/Faction
@onready var existing_factions: OptionButton = $Layout/Pages/MapPage/Materials/PaletteTabs/Units/Tools/ExistingFactions
@onready var map_view = $Layout/Pages/MapPage/MapPanel/MapViewportContainer/MapViewport/Map
@onready var pages: TabContainer = $Layout/Pages
@onready var page_tabs: TabBar = $Layout/Navigation/PageTabs
@onready var palette_navigation: HFlowContainer = $Layout/Navigation/PaletteTabs
@onready var definition_pages: Array = [$Layout/Pages/UnitPage, $Layout/Pages/SkillPage, $Layout/Pages/TerrainPage, $Layout/Pages/EquipmentPage]
@onready var status_label: Label = $Layout/Status

var cargo_draft := {"positions": [], "exits": []}
var cargo_draft_changed := false

var editing_mode := EditMode.GROUND
var data_dir := DataSource.directory()
var definitions_path := data_dir + "definitions.toml"
var map_dir := data_dir + "maps/"
var terrain_brushes := {"ground": "", "overlay": ""}

var core
var definitions: Dictionary = {}
var map_data: Dictionary = {}
var map_file := ""
var dirty := false
var history = preload("res://features/editor/edit_history.gd").new()
var duplicating_map := false
var refreshing := false
var selected_cell := Vector2i(-1, -1)
var stroke_id := 0
var input_error_count := 0

func _process(delta: float) -> void:
	if pages.current_tab != 0: return
	var focused := get_viewport().gui_get_focus_owner()
	if focused is LineEdit or focused is TextEdit:
		return
	map_view.move_camera(delta)

func _ready() -> void:
	core = TacticalGame.new()
	DifficultyOptions.populate(placement_difficulty, true)
	DifficultyOptions.populate(inspected_difficulty, true)
	DifficultyOptions.populate(play_difficulty)
	inspected_difficulty.item_selected.connect(update_unit_difficulty)
	for options in [placement_facing, inspected_facing]:
		for direction in ["right", "left"]:
			options.add_item("初始朝向：右" if direction == "right" else "初始朝向：左")
			options.set_item_metadata(options.item_count - 1, direction)
	inspected_facing.item_selected.connect(update_unit_facing)
	for title in UNIT_MODES:
		mode_list.add_item(title)
		mode_list.set_item_metadata(mode_list.item_count - 1, UNIT_MODES[title])
	for index in palette_tabs.get_tab_count():
		palette_tabs.set_tab_title(index, PALETTE_TABS[str(palette_tabs.get_tab_control(index).name)].title)
	for tool in ["放置貨物", "刪除貨物", "放置出口", "刪除出口"]:
		cargo_tool.add_item(tool)
	$Layout/Pages/MapPage/Materials/PaletteTabs/Cargo/Apply.pressed.connect(apply_cargo)
	$Layout/Pages/MapPage/Materials/PaletteTabs/Cargo/Reset.pressed.connect(reset_cargo_draft)
	$Layout/Pages/MapPage/Materials/PaletteTabs/Cargo/Remove.pressed.connect(remove_cargo)
	for index in outcome_groups.size():
		var group := outcome_groups[index]
		var key: String = ["victory", "defeat"][index]
		var mode: OptionButton = group.get_node("Mode")
		mode.add_item("全部成立")
		mode.add_item("任一成立")
		mode.item_selected.connect(func(_selected: int): update_outcome_group(key, group))
		var add_team: OptionButton = group.get_node("AddTeam")
		add_team.item_selected.connect(func(selected: int): add_outcome_team(key, add_team.get_item_metadata(selected)))
		for child in group.get_children():
			if child is CheckBox:
				child.toggled.connect(func(_pressed: bool): update_outcome_group(key, group))
	for value in ["player", "enemy"]:
		team_list.add_item(value)
	mode_list.select(0)
	team_list.select(0)
	for index in [0, 1, 2, 3, 4]:
		pages.set_tab_title(index, ["地圖與單位配置", "單位", "技能", "地形", "裝備"][index])
	setup_navigation()
	for page in definition_pages:
		page.field_changed.connect(update_definition)
		page.default_passive_skills_changed.connect(update_default_passive_skills)
		page.move_requested.connect(move_definition)
		page.terrain_effect_confirmed.connect(change_terrain_effect)
		page.create_requested.connect(create_definition)
		page.remove_requested.connect(request_remove_definition)
		page.remove_confirmed.connect(remove_definition)
		page.input_error.connect(show_error)
	map_view.cell_hovered.connect(func(_cell: Vector2i): refresh_brush_preview())
	brush_size.share($Layout/Pages/MapPage/Materials/PaletteTabs/Overlay/BrushSize)
	brush_size.value_changed.connect(func(_value: float): refresh_brush_preview())
	map_view.cell_pressed.connect(edit_cell)
	map_view.inspection_clicked.connect(inspect_unit)
	map_view.stroke_started.connect(func(): stroke_id += 1)
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
	existing_factions.get_popup().about_to_popup.connect(refresh_existing_factions)
	existing_factions.item_selected.connect(func(index: int):
		faction_field.text = existing_factions.get_item_text(index)
		existing_factions.select(0)
	)
	existing_factions.add_item("選擇既有敵方派系")
	existing_factions.set_item_disabled(0, true)
	faction_field.visible = false
	width_box.value_changed.connect(func(_value: float): resize_map())
	height_box.value_changed.connect(func(_value: float): resize_map())
	var source := read_file(definitions_path)
	if source.is_empty():
		return
	var parsed := read_core_response(core.definitions_to_json(source))
	if parsed.is_empty():
		return
	definitions = parsed
	history.reset(document_state())
	refresh_map_list()
	if get_tree().root.has_meta("editor_session"):
		var session: Dictionary = get_tree().root.get_meta("editor_session")
		get_tree().root.remove_meta("editor_session")
		definitions = session.definitions
		map_data = session.map
		reset_cargo_draft()
		map_file = session.file
		history = session.history
		stroke_id = session.stroke_id
		select_map(map_file)
		refresh_ui()
		pages.current_tab = session.get("page", 0)
		var selections: Array = session.get("selections", [])
		for index in selections.size(): definition_pages[index].select_id(selections[index])
		refresh_preview()
		return
	if map_list.item_count > 0:
		open_selected_map()
	else:
		refresh_ui()
		new_map()

# 切換列獨立於內容面板；右側素材分類由右向左排列並自動換行。
func setup_navigation() -> void:
	for index in pages.get_tab_count():
		page_tabs.add_tab(pages.get_tab_title(index))
	page_tabs.current_tab = pages.current_tab
	page_tabs.tab_changed.connect(func(index: int): pages.current_tab = index)
	pages.tab_changed.connect(func(index: int):
		page_tabs.current_tab = index
		palette_navigation.visible = index == 0
	)
	palette_navigation.visible = pages.current_tab == 0
	var group := ButtonGroup.new()
	for index in palette_tabs.get_tab_count():
		var button := Button.new()
		button.text = palette_tabs.get_tab_title(index)
		button.layout_direction = Control.LAYOUT_DIRECTION_LTR
		button.toggle_mode = true
		button.button_group = group
		button.button_pressed = palette_tabs.current_tab == index
		button.pressed.connect(func(): palette_tabs.current_tab = index)
		palette_navigation.add_child(button)
	palette_tabs.tab_changed.connect(func(index: int):
		var button: Button = palette_navigation.get_child(index)
		button.set_pressed_no_signal(true)
	)

func read_file(path: String) -> String:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		show_error("無法讀取 %s：%s" % [path, error_string(FileAccess.get_open_error())])
		return ""
	return file.get_as_text()

func refresh_map_list() -> void:
	var previous := selected_map()
	map_list.clear()
	for filename in DirAccess.get_files_at(map_dir):
		if not filename.ends_with(".toml"): continue
		var path := map_dir + filename
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
	reset_cargo_draft()
	map_view.camera_initialized = false
	map_file = path
	history.reset(document_state())
	selected_cell = Vector2i(-1, -1)
	map_view.inspected_unit = 0
	dirty = false
	refresh_ui()
	refresh_preview()

func new_map() -> void:
	open_map_dialog(false)

func duplicate_map() -> void:
	if map_data.is_empty(): return
	if not commit_pending_fields(): return
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
	var path := map_dir + filename + ".toml"
	if FileAccess.file_exists(path):
		$AddMapDialog/Form/Error.text = "檔案「%s.toml」已存在。" % filename
		return
	if duplicating_map:
		map_data = map_data.duplicate(true)
		map_data.name += " 副本"
	else:
		map_data = {
			"name": filename, "width": 10, "height": 8, "terrains": [], "units": [],
			"victory": {"mode": "all", "conditions": [{"type": "all_enemies_defeated"}]},
			"defeat": {"mode": "any", "conditions": [{"type": "team_defeated", "team": "player"}]},
		}
	reset_cargo_draft()
	map_file = path
	map_view.camera_initialized = false
	history.reset(document_state())
	history.saved_state = {}
	selected_cell = Vector2i(-1, -1)
	map_view.inspected_unit = 0
	dirty = true
	$AddMapDialog.hide()
	refresh_ui()
	refresh_preview()

func save_map() -> void:
	if map_data.is_empty(): return
	if not commit_pending_fields(): return
	var result := serialize_documents()
	if result.is_empty(): return
	var documents := {definitions_path: result.definitions, map_file: result.map}
	for path in documents:
		var file := FileAccess.open(path, FileAccess.WRITE)
		if file == null:
			show_error("無法寫入 %s：%s" % [path, error_string(FileAccess.get_open_error())])
			return
		file.store_string(documents[path])
	history.saved_state = document_state()
	update_history_controls()
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
	reset_cargo_draft()
	history.reset(document_state())
	refresh_map_list()
	if map_list.item_count > 0: open_selected_map()
	else:
		refresh_ui()
		new_map()

func play_map() -> void:
	if map_data.is_empty(): return
	if not commit_pending_fields(): return
	if play_difficulty.selected <= 0:
		show_error("請選擇試玩難度。")
		return
	var result := serialize_documents()
	if result.is_empty():
		return
	get_tree().root.set_meta("editor_session", {"definitions": definitions.duplicate(true), "map": map_data.duplicate(true), "file": map_file, "history": history, "stroke_id": stroke_id, "page": pages.current_tab, "selections": definition_pages.map(func(page): return page.selected_id())})
	var battle := BATTLE_SCENE.instantiate()
	battle.configure(result.definitions, result.map, play_difficulty.get_selected_metadata(), true)
	get_tree().change_scene_to_node(battle)

func serialize_documents() -> Dictionary:
	return read_core_response(core.documents_from_json(JSON.stringify(definitions), JSON.stringify(map_data)))

# 所有文件修改走同一出口：核心驗證通過才寫入歷史，失敗則還原為修改前文件。
func commit(previous: Dictionary, snapshot := {}, label := "編輯資料", merge_mode := UndoRedo.MERGE_DISABLE) -> bool:
	if snapshot.is_empty(): snapshot = preview_snapshot()
	if snapshot.is_empty():
		var error_message := status_label.text
		apply_document(previous)
		refresh_ui()
		refresh_preview()
		status_label.text = error_message
		return false
	history.record(previous, document_state(), label, merge_mode)
	present_preview(snapshot)
	return true

func preview_snapshot() -> Dictionary:
	return read_core_response(core.inspected_preview_from_json(JSON.stringify(definitions), JSON.stringify(map_data), map_view.inspected_unit if map_view.inspected_unit != 0 else null))

# 不修改文件的重新整理，例如開圖、復原與查看單位。
func refresh_preview() -> void:
	var snapshot := preview_snapshot()
	if not snapshot.is_empty(): present_preview(snapshot)

func present_preview(snapshot: Dictionary) -> void:
	update_history_controls()
	map_view.present(map_data, snapshot, selected_cell)
	refresh_cargo_tools()
	refresh_outcome_groups()
	refresh_inspected_unit()
	status_label.text = "資料有效%s" % ("；尚未儲存" if dirty else "")

# 自製編輯器顯示核心原始診斷；正式遊戲維持共用讀取函式的翻譯訊息。
func read_core_response(response: String) -> Dictionary:
	return CoreResponse.read(response, show_error, [], true)

func show_error(message: String) -> void:
	input_error_count += 1
	status_label.text = "錯誤：" + message
	for page in definition_pages: page.show_add_error(message)

# 歷史只接收已通過核心驗證的新舊文件，不記錄未提交的文字。
func document_state() -> Dictionary:
	return {"definitions": definitions.duplicate(true), "map": map_data.duplicate(true), "file": map_file}

func apply_document(state: Dictionary) -> void:
	definitions = state.definitions.duplicate(true)
	map_data = state.map.duplicate(true)
	map_file = state.file
	update_history_controls()

func update_history_controls() -> void:
	dirty = cargo_draft_changed or document_state() != history.saved_state
	$Layout/Toolbar/Undo.disabled = not history.undo_redo.has_undo()
	$Layout/Toolbar/Redo.disabled = not history.undo_redo.has_redo()

func restore_history() -> void:
	for page in definition_pages: page.discard_pending_fields()
	apply_document(history.state)
	reset_cargo_draft()
	refresh_ui()
	refresh_preview()

func undo() -> void:
	if history.undo_redo.undo(): restore_history()

func redo() -> void:
	if history.undo_redo.redo(): restore_history()

func commit_pending_fields() -> bool:
	if not apply_cargo(): return false
	var errors_before_commit := input_error_count
	var pending_name := map_name.text
	for page in definition_pages:
		page.commit_pending_fields(func(): return input_error_count != errors_before_commit)
		if input_error_count != errors_before_commit: return false
	map_name.text = pending_name
	commit_map_name()
	return input_error_count == errors_before_commit

func commit_map_name() -> void:
	if refreshing or map_data.is_empty() or map_data.name == map_name.text: return
	var previous := document_state()
	map_data.name = map_name.text
	commit(previous)
	refresh_map_list()
	select_map(map_file)

func resize_map() -> void:
	if refreshing or map_data.is_empty(): return
	if not apply_cargo():
		refresh_ui()
		return
	var new_width := int(width_box.value)
	var new_height := int(height_box.value)
	if new_width == map_data.width and new_height == map_data.height: return
	var previous := document_state()
	map_data.width = new_width
	map_data.height = new_height
	# 只刪除超出範圍的地形；不處理單位，避免縮小地圖時不小心改到單位，超界單位交由核心驗證回報。
	map_data.terrains = map_data.terrains.filter(func(t: Dictionary): return t.x < new_width and t.y < new_height)
	commit(previous)

func refresh_ui() -> void:
	update_history_controls()
	var has_map := not map_data.is_empty()
	$Layout/Pages/MapPage/MapPanel.visible = has_map
	$Layout/Pages/MapPage/Materials.visible = has_map
	$Layout/Pages/MapPage/MapListPanel/MapActions/Duplicate.disabled = not has_map
	$Layout/Pages/MapPage/MapListPanel/MapActions/Delete.disabled = not has_map
	refreshing = true
	map_name.text = map_data.get("name", "")
	width_box.value = map_data.get("width", 10)
	height_box.value = map_data.get("height", 8)
	refresh_outcome_groups()
	refresh_cargo_tools()
	refreshing = false
	refresh_map_list()
	select_map(map_file)
	refresh_tools()
	refresh_tools_visibility()
	refresh_definitions()

# 介面只編輯作者文件；空條件與貨物需求交由核心既有驗證決定。
func refresh_outcome_groups() -> void:
	if map_data.is_empty(): return
	for index in outcome_groups.size():
		var group := outcome_groups[index]
		var rule: Dictionary = map_data[["victory", "defeat"][index]]
		var mode: OptionButton = group.get_node("Mode")
		mode.select(0 if rule.mode == "all" else 1)
		refresh_outcome_teams(["victory", "defeat"][index], group, rule.conditions)
		for child in group.get_children():
			if child is CheckBox:
				child.set_pressed_no_signal(rule.conditions.any(func(condition: Dictionary): return condition == OUTCOME_CONDITIONS[str(child.name)]))

func update_outcome_group(key: String, group: VBoxContainer) -> void:
	if refreshing or map_data.is_empty(): return
	var conditions: Array = map_data[key].conditions.filter(func(condition: Dictionary): return condition.type == "team_defeated" and condition.team is Dictionary)
	for child in group.get_children():
		if child is CheckBox and child.button_pressed:
			conditions.append(OUTCOME_CONDITIONS[str(child.name)].duplicate(true))
	var mode: OptionButton = group.get_node("Mode")
	var previous := document_state()
	map_data[key] = {"mode": "all" if mode.selected == 0 else "any", "conditions": conditions}
	commit(previous)

# 下拉選單只整理作者文件中的陣營；勝敗是否合法仍由核心驗證。
func refresh_outcome_teams(key: String, group: VBoxContainer, conditions: Array) -> void:
	var rows: VBoxContainer = group.get_node("Teams")
	for child in rows.get_children():
		rows.remove_child(child)
		child.queue_free()
	var selected_teams: Array = []
	for condition: Dictionary in conditions:
		if condition.type != "team_defeated" or not condition.team is Dictionary: continue
		var team: Variant = condition.team
		selected_teams.append(team)
		var checkbox := CheckBox.new()
		checkbox.text = "%s 全滅" % team.enemy
		checkbox.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		checkbox.set_pressed_no_signal(true)
		checkbox.toggled.connect(func(pressed: bool):
			if not pressed: remove_outcome_team(key, team)
		)
		rows.add_child(checkbox)
	var options: OptionButton = group.get_node("AddTeam")
	options.clear()
	options.add_item("加入指定陣營全滅…")
	options.set_item_disabled(0, true)
	var teams: Array = []
	for unit: Dictionary in map_data.units:
		if not unit.team is Dictionary or teams.has(unit.team): continue
		teams.append(unit.team)
		if selected_teams.has(unit.team): continue
		options.add_item(unit.team.enemy)
		options.set_item_metadata(options.item_count - 1, unit.team)
	options.select(0)
	options.disabled = options.item_count == 1

func add_outcome_team(key: String, team: Variant) -> void:
	var previous := document_state()
	map_data[key].conditions.append({"type": "team_defeated", "team": team})
	commit(previous)

func remove_outcome_team(key: String, team: Variant) -> void:
	var previous := document_state()
	map_data[key].conditions = map_data[key].conditions.filter(func(condition: Dictionary): return not (condition.type == "team_defeated" and condition.team is Dictionary and condition.team == team))
	commit(previous)

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
	var tab: Dictionary = PALETTE_TABS[str(palette_tabs.get_current_tab_control().name)]
	editing_mode = tab.get("mode", mode_list.get_selected_metadata())
	refresh_tools_visibility()
	update_brush_label()

func update_brush_label() -> void:
	refresh_brush_preview()
	for index in brush_labels.size():
		var layer := "ground" if index == 0 else "overlay"
		var kind: String = terrain_brushes[layer]
		brush_labels[index].text = "目前筆刷：" + (kind if not kind.is_empty() else "橡皮擦" if index == 1 else "無素材")

func terrain_kinds(layer: String) -> Array:
	var kinds: Array = definitions.terrain_types.filter(func(terrain: Dictionary): return terrain.layer == layer).map(func(terrain: Dictionary): return terrain.id)
	return kinds

func refresh_tools_visibility() -> void:
	# 工具切換只更新介面狀態；資料修改的驗證與預覽由 commit 統一處理。
	var placing := editing_mode == EditMode.PLACE_UNIT
	map_view.drag_paint = BRUSH_LAYERS.has(editing_mode)
	unit_list.visible = placing
	placement_facing.visible = placing
	team_list.visible = placing
	faction_field.visible = placing and team_list.selected != 0
	existing_factions.visible = faction_field.visible
	placement_difficulty.visible = faction_field.visible

func refresh_existing_factions() -> void:
	var result := read_core_response(core.enemy_factions_from_json(JSON.stringify(map_data)))
	if result.is_empty(): return
	var factions: Array = result.factions
	existing_factions.clear()
	existing_factions.add_item("尚無既有敵方派系" if factions.is_empty() else "選擇既有敵方派系")
	existing_factions.set_item_disabled(0, true)
	for faction in factions:
		existing_factions.add_item(faction)

# 預覽只繪製核心回傳的覆蓋格子，不在顯示層重算筆刷範圍。
func refresh_brush_preview() -> void:
	if not BRUSH_LAYERS.has(editing_mode) or map_data.is_empty() or map_view.hovered.x < 0:
		map_view.present_brush([])
		return
	if editing_mode == EditMode.GROUND and terrain_brushes.ground.is_empty():
		map_view.present_brush([])
		return
	var command := {"x": map_view.hovered.x, "y": map_view.hovered.y, "size": int(brush_size.value)}
	var result := read_core_response(core.terrain_brush_cells_from_json(JSON.stringify(map_data), JSON.stringify(command)))
	map_view.present_brush([] if result.is_empty() else result.cells)

# 筆刷只傳遞作者選擇；圖層替換、清除與合法性由 Rust 決定。
func replace_layer(cell: Vector2i, layer: String, kind: String) -> void:
	var command := {"x": cell.x, "y": cell.y, "layer": layer, "kind": kind, "size": int(brush_size.value)}
	var result := read_core_response(core.paint_terrain_from_json(JSON.stringify(definitions), JSON.stringify(map_data), JSON.stringify(command)))
	if result.is_empty() or not result.changed: return
	var previous := document_state()
	map_data = result.map
	CoreResponse.convert_unit_ids(result.snapshot)
	# 筆刷結果不含查看中單位的範圍，查看時改由 commit 重新查詢。
	commit(previous, result.snapshot if map_view.inspected_unit == 0 else {}, "地形筆刷 %s" % stroke_id, UndoRedo.MERGE_ENDS)

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
	refresh_preview()

func edit_cell(cell: Vector2i) -> void:
	if editing_mode == EditMode.NONE: return
	selected_cell = cell
	if editing_mode == EditMode.CARGO:
		edit_cargo_cell(cell)
		return
	var current := unit_at(cell)
	var previous := document_state()
	if BRUSH_LAYERS.has(editing_mode):
		var layer: String = BRUSH_LAYERS[editing_mode]
		var kind: String = terrain_brushes[layer]
		if editing_mode == EditMode.GROUND and kind.is_empty(): return
		replace_layer(cell, layer, kind)
		return
	elif editing_mode == EditMode.PLACE_UNIT:
		if unit_list.item_count == 0: return
		var kind := unit_list.get_item_text(unit_list.selected)
		var id := 1
		while map_data.units.any(func(u: Dictionary): return u.id == id):
			id += 1
		var team: Variant = "player" if team_list.selected == 0 else {"enemy": faction_field.text.strip_edges()}
		var placement := {"id":id,"unit_type":kind,"team":team,"x":cell.x,"y":cell.y,"min_difficulty":null,"facing":placement_facing.get_selected_metadata()}
		if team_list.selected != 0:
			placement.min_difficulty = placement_difficulty.get_selected_metadata()
		map_data.units.append(placement)
	elif editing_mode == EditMode.MOVE_UNIT:
		if current.is_empty(): return
		map_view.begin_unit_drag(current, cell)
		return
	else:
		if current.is_empty(): return
		map_data.units = map_data.units.filter(func(u: Dictionary): return u.id != current.id)
	commit(previous)

func move_unit(id: int, cell: Vector2i) -> void:
	if editing_mode != EditMode.MOVE_UNIT: return
	selected_cell = cell
	for unit in map_data.units:
		if unit.id != id: continue
		if Vector2i(unit.x, unit.y) == cell: return
		var previous := document_state()
		unit.x = cell.x
		unit.y = cell.y
		commit(previous)
		return

func refresh_definitions() -> void:
	var options := read_core_response(core.edit_definition_from_json(JSON.stringify(definitions), "{}", JSON.stringify({"action": "skill_effect_options"})))
	if options.is_empty(): return
	for page in definition_pages: page.present(definitions, options.terrain_ids, options.terrain_entries, options.skill_entries, options.equipment_choices, options.active_skill_ids, options.passive_skill_ids)

func update_default_passive_skills(skills: Array) -> void:
	if definitions.default_passive_skills == skills: return
	if not edit_definition({"action": "update_default_passive_skills", "skills": skills}):
		refresh_definitions()

func find_definition(category: String, id: String) -> Dictionary:
	# 技能頁同時編輯主動與被動技能。
	var category_entries: Array = definitions.active_skills + definitions.passive_skills if category == "skills" else definitions[category]
	for entry in category_entries:
		if entry.id == id: return entry
	return {}

func move_definition(category: String, id: String, offset: int) -> void:
	edit_definition({"action": "move", "category": category, "id": id, "offset": offset})

func update_definition(category: String, id: String, key: String, value: Variant) -> void:
	# ID 建立後不可更動，因此不需要更新既有引用。
	if key == "id": return
	var entry := find_definition(category, id)
	if entry.is_empty(): return
	if entry.has(key) and entry[key] == value:
		for page in definition_pages:
			if page.category == category: page.accept_field(id, key)
		return
	if category == "skills" and key == "effect":
		if value == "mire":
			for page in definition_pages:
				if page.category == "skills": page.choose_effect_terrain(id)
		else:
			if not edit_definition({"action": "change_skill_effect", "id": id, "effect": value}):
				refresh_definitions()
		return
	# 失敗時保留作者輸入，讓儲存前重新提交並阻止寫入舊值。
	if not edit_definition({"action": "update_field", "category": category, "id": id, "key": key, "value": value}):
		refresh_definitions()

# ID 建立後不可更動，因此不需要更新既有引用；刪除檢查仍涵蓋所有地圖。
func edit_definition(command: Dictionary) -> bool:
	var maps := {}
	for filename in DirAccess.get_files_at(map_dir):
		if not filename.ends_with(".toml"): continue
		var path := map_dir + filename
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
	var previous := document_state()
	definitions = result.definitions
	if not commit(previous): return false
	if command.action == "update_field":
		for page in definition_pages:
			if page.category == command.category: page.accept_field(command.id, command.key)
	refresh_ui()
	return true

func show_definition_error(details: Dictionary) -> void:
	var id: String = details.id
	match details.kind:
		"empty_id": show_error("ID 不可為空。")
		"duplicate_id": show_error("ID「%s」已存在。" % id)
		"not_found": show_error("找不到 ID「%s」。" % id)
		"default_ground": show_error("「%s」是核心保留的預設地形 ID，不能刪除。" % id)
		"missing_map": show_error("請先建立地圖，才能修改定義。")
		"referenced":
			var references: Array[String] = []
			for reference in details.references:
				var category: String = {"unit_types": "單位", "skills": "技能", "maps": "地圖", "defaults": "全體預設"}.get(reference.category, reference.category)
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

# 呈現作者設定；未設定的玩家以簡單顯示，不改寫資料。
func refresh_inspected_unit() -> void:
	inspected_unit_type.visible = false
	inspected_facing.visible = false
	inspected_difficulty.visible = false
	for unit in map_data.get("units", []):
		if unit.id != map_view.inspected_unit: continue
		inspected_unit_type.text = "單位：" + unit.unit_type
		inspected_unit_type.visible = true
		inspected_facing.visible = true
		for index in inspected_facing.item_count:
			if inspected_facing.get_item_metadata(index) == unit.facing:
				inspected_facing.select(index)
		inspected_difficulty.visible = true
		var displayed_difficulty: String = "easy" if unit.min_difficulty == null else unit.min_difficulty
		for index in range(1, inspected_difficulty.item_count):
			if inspected_difficulty.get_item_metadata(index) == displayed_difficulty:
				inspected_difficulty.select(index)
				return

func update_unit_difficulty(index: int) -> void:
	if index <= 0: return
	for unit in map_data.units:
		if unit.id != map_view.inspected_unit: continue
		var selected: String = inspected_difficulty.get_item_metadata(index)
		if unit.min_difficulty == selected: return
		var previous := document_state()
		unit.min_difficulty = selected
		commit(previous)
		return

func update_unit_facing(index: int) -> void:
	for unit in map_data.units:
		if unit.id != map_view.inspected_unit: continue
		var selected: String = inspected_facing.get_item_metadata(index)
		if unit.facing == selected: return
		var previous := document_state()
		unit.facing = selected
		commit(previous)
		return

# 貨物與出口一起提交，未完成的作者配置只作為介面草稿，不送入遊戲預覽。
func reset_cargo_draft() -> void:
	var cargo: Variant = map_data.get("cargo")
	cargo_draft = {"positions": [], "exits": []} if cargo == null else cargo.duplicate(true)
	cargo_draft_changed = false
	refresh_cargo_tools()
	update_history_controls()

func refresh_cargo_tools() -> void:
	cargo_summary.text = "貨物：%s 份；出口：%s 個%s" % [cargo_draft.positions.size(), cargo_draft.exits.size(), "（尚未套用）" if cargo_draft_changed else ""]
	$Layout/Pages/MapPage/Materials/PaletteTabs/Cargo/Apply.disabled = not cargo_draft_changed
	$Layout/Pages/MapPage/Materials/PaletteTabs/Cargo/Reset.disabled = not cargo_draft_changed
	map_view.present_cargo(cargo_draft)

func edit_cargo_cell(cell: Vector2i) -> void:
	var key := "positions" if cargo_tool.selected < 2 else "exits"
	var point := {"x": cell.x, "y": cell.y}
	var cells: Array = cargo_draft[key]
	if cargo_tool.selected in [0, 2]:
		if cells.has(point): return
		cells.append(point)
	else:
		cells.erase(point)
	update_cargo_draft()

func update_cargo_draft() -> void:
	var cargo: Variant = map_data.get("cargo")
	var original: Dictionary = {"positions": [], "exits": []} if cargo == null else cargo
	cargo_draft_changed = cargo_draft != original
	refresh_cargo_tools()
	update_history_controls()
	status_label.text = "貨物配置尚未套用；儲存或試玩時會一併套用。" if cargo_draft_changed else "貨物配置未變更。"

func remove_cargo() -> void:
	cargo_draft = {"positions": [], "exits": []}
	update_cargo_draft()

func apply_cargo() -> bool:
	if not cargo_draft_changed: return true
	var previous := document_state()
	map_data.cargo = null if cargo_draft.positions.is_empty() and cargo_draft.exits.is_empty() else cargo_draft.duplicate(true)
	if not commit(previous): return false
	cargo_draft_changed = false
	update_history_controls()
	refresh_cargo_tools()
	return true
