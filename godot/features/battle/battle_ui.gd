extends CanvasLayer

signal action_selected(action: String)
signal end_turn_requested
signal delay_selection_requested
signal delay_target_selected(unit_id: int)
signal turn_order_focus_requested(unit_id: int)
signal current_unit_focus_requested
signal inspection_closed
signal skill_inspection_requested(skill_id: String)
signal language_changed
signal playback_speed_changed(speed: float)

enum DetailTab { TERRAIN, ATTRIBUTES, ACTIVE_SKILLS, PASSIVE_SKILLS }

const TEAM_COLORS := {"player": "#63a9ff", "enemy": "#ff6868"}
const RESULT_STYLE := "[color=#f0c96a]%s[/color]"
const MOVE_COST_POPUP_OFFSET := Vector2(16.0, 16.0)
const ATTACK_PREVIEW_OFFSET := Vector2(18.0, 18.0)
const LANGUAGE_MENU_ID := 0
const DIFFICULTY_MENU_ID := 1
const LANGUAGE_LABELS := ["繁體中文", "English"]

@onready var root: Control = $Root
@onready var playback_speed_button: Button = $Root/PlaybackSpeed
@onready var menu_button: MenuButton = $Root/Menu
@onready var info_panel: PanelContainer = $Root/InfoPanel
@onready var info_title: Label = $Root/InfoPanel/Margin/Content/Header/Title
@onready var detail_tabs: TabContainer = $Root/InfoPanel/Margin/Content/DetailTabs
@onready var active_skills_text: Label = $Root/InfoPanel/Margin/Content/DetailTabs/ActiveSkills/Text
@onready var passive_skills_text: Label = $Root/InfoPanel/Margin/Content/DetailTabs/PassiveSkills/Text
@onready var unit_details: VBoxContainer = $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails
@onready var unit_name: Label = $Root/InfoPanel/Margin/Content/UnitName
@onready var detail_values := {
	"team": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/TeamValue,
	"hp": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/HpValue,
	"size": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/SizeValue,
	"movement": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/MovementValue,
	"initiative": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/InitiativeValue,
	"dodge": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/DodgeValue,
	"block": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/BlockValue,
	"block_reduction": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/BlockReductionValue,
	"hit": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/HitValue,
	"physical_power": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/PhysicalPowerValue,
	"magical_power": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/MagicalPowerValue,
	"main_hand": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/MainHandValue,
	"off_hand": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/OffHandValue,
	"armor": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/ArmorValue,
	"accessory": $Root/InfoPanel/Margin/Content/DetailTabs/Attributes/Contents/UnitDetails/Rows/AccessoryValue,
}
@onready var terrain_name: Label = $Root/InfoPanel/Margin/Content/DetailTabs/Terrain/Contents/TerrainRows/TerrainValue
@onready var terrain_cost: Label = $Root/InfoPanel/Margin/Content/DetailTabs/Terrain/Contents/TerrainRows/CostValue
@onready var terrain_effect: Label = $Root/InfoPanel/Margin/Content/DetailTabs/Terrain/Contents/TerrainRows/EffectValue
@onready var inspected_skill_details: VBoxContainer = $Root/InfoPanel/Margin/Content/SkillDetails
@onready var inspected_skill_name: Label = $Root/InfoPanel/Margin/Content/SkillDetails/Name
@onready var inspected_skill_description: Label = $Root/InfoPanel/Margin/Content/SkillDetails/Details
@onready var actor_name: Label = $Root/LeftColumn/CurrentUnitBar/Margin/Layout/Actor/NameAndMovement/Name
@onready var actor_portrait: TextureRect = $Root/LeftColumn/CurrentUnitBar/Margin/Layout/Actor/Portrait
@onready var movement: Label = $Root/LeftColumn/CurrentUnitBar/Margin/Layout/Actor/NameAndMovement/Movement
@onready var turn_order: VBoxContainer = $Root/LeftColumn/TurnOrder/Margin/Units
@onready var delay_button: Button = $Root/LeftColumn/TurnOrder/Delay
@onready var status: Label = $Root/Status
@onready var log_panel: Panel = $Root/LogPanel
@onready var battle_log: RichTextLabel = $Root/LogPanel/Margin/Content/Entries
@onready var log_visibility_button: Button = $Root/LogVisibilityButton
@onready var move_cost_popup: PanelContainer = $Root/MoveCostPopup
@onready var move_cost_label: Label = $Root/MoveCostPopup/Label
@onready var attack_preview_panel: PanelContainer = $Root/AttackPreview
@onready var attack_preview_title: Label = $Root/AttackPreview/Margin/Content/Title
@onready var attack_preview_resources: Label = $Root/AttackPreview/Margin/Content/ResourceRow/Resources
@onready var attack_preview_damage: Label = $Root/AttackPreview/Margin/Content/ResourceRow/Damage
@onready var attack_preview_markers: Control = $Root/AttackPreview/Margin/Content/HealthMarkers
@onready var attack_preview_result_labels := {
	"hit": $Root/AttackPreview/Margin/Content/Details/Hit,
	"block": $Root/AttackPreview/Margin/Content/Details/Block,
}
@onready var attack_preview_health_segments := {
	"hit": $Root/AttackPreview/Margin/Content/HealthImpactBar/HitRemaining,
	"block": $Root/AttackPreview/Margin/Content/HealthImpactBar/BlockSaved,
	"damage": $Root/AttackPreview/Margin/Content/HealthImpactBar/Damage,
	"missing": $Root/AttackPreview/Margin/Content/HealthImpactBar/Missing,
}
@onready var attack_preview_critical: Label = $Root/AttackPreview/Margin/Content/Details/Critical
@onready var healing_preview_segment: ColorRect = $Root/AttackPreview/Margin/Content/HealthImpactBar/Healing
@onready var hovered_skill_card: PanelContainer = $Root/HoveredSkill
@onready var hovered_skill_title: Label = $Root/HoveredSkill/Margin/Content/Title
@onready var hovered_skill_details: Label = $Root/HoveredSkill/Margin/Content/Details
@onready var action_buttons_container: HBoxContainer = $Root/ActionBar/Margin/Layout/Actions/Buttons
var action_buttons := {}
var turn_order_slots := {}
var turn_order_selecting_delay := false
var inspected_unit_id: Variant = null
var last_unit_detail_tab := DetailTab.ATTRIBUTES
var showing_unit_tabs := false
var dragging_info_panel := false
var drag_offset := Vector2.ZERO
var log_entry_expanded_states: Dictionary = {}
var presented_log_events: Array = []
var dragging_battle_log := false
var presented_skills: Array = []
var hovered_skill_id := ""
var playback_speed_index := 0
var language_menu: PopupMenu
var difficulty_menu: PopupMenu

func _ready() -> void:
	detail_tabs.tab_changed.connect(_on_detail_tab_changed)
	playback_speed_button.pressed.connect(_on_playback_speed_pressed)
	TranslationServer.set_locale(GameSettings.load_locale())
	playback_speed_index = GameSettings.PLAYBACK_SPEEDS.find(GameSettings.load_playback_speed())
	playback_speed_button.text = "×%d" % GameSettings.PLAYBACK_SPEEDS[playback_speed_index]
	var popup := menu_button.get_popup()
	language_menu = PopupMenu.new()
	for index in GameSettings.LOCALES.size():
		language_menu.add_radio_check_item(LANGUAGE_LABELS[index], index)
	popup.add_submenu_node_item("", language_menu, LANGUAGE_MENU_ID)
	difficulty_menu = PopupMenu.new()
	difficulty_menu.add_separator("DIFFICULTY_SETTING", DifficultyOptions.IDS.size())
	for index in DifficultyOptions.IDS.size():
		difficulty_menu.add_radio_check_item(DifficultyOptions.LABEL_KEYS[index], index)
	popup.add_submenu_node_item("", difficulty_menu, DIFFICULTY_MENU_ID)
	language_menu.id_pressed.connect(_on_language_selected)
	difficulty_menu.id_pressed.connect(_on_difficulty_selected)
	update_difficulty_menu()
	update_language_menu()
	apply_ui_z_order()
	$Root/AttackPreview/Margin/Content/HealthImpactBar.resized.connect(func(): position_health_markers.call_deferred())
	for segment in attack_preview_health_segments.values():
		segment.resized.connect(func(): position_health_markers.call_deferred())
	$Root/InfoPanel/Margin/Content/Header.gui_input.connect(_on_header_gui_input)
	$Root/InfoPanel/Margin/Content/Header/Close.pressed.connect(func(): inspection_closed.emit())
	battle_log.meta_clicked.connect(_on_battle_log_meta_clicked)
	battle_log.gui_input.connect(_on_battle_log_gui_input)
	log_visibility_button.toggled.connect(_on_log_visibility_toggled)
	$Root/ActionBar/Margin/Layout/EndTurn.pressed.connect(func(): end_turn_requested.emit())
	delay_button.pressed.connect(func(): delay_selection_requested.emit())

func _on_playback_speed_pressed() -> void:
	var next_index := (playback_speed_index + 1) % GameSettings.PLAYBACK_SPEEDS.size()
	var speed: int = GameSettings.PLAYBACK_SPEEDS[next_index]
	if not GameSettings.save_playback_speed(speed):
		return
	playback_speed_index = next_index
	playback_speed_button.text = "×%d" % speed
	playback_speed_changed.emit(float(speed))

func _on_detail_tab_changed(index: int) -> void:
	if showing_unit_tabs:
		last_unit_detail_tab = index

func _on_difficulty_selected(id: int) -> void:
	if not GameSettings.save_difficulty(DifficultyOptions.IDS[id]):
		return
	update_difficulty_menu()

func update_difficulty_menu() -> void:
	var difficulty := GameSettings.load_difficulty()
	var popup := menu_button.get_popup()
	for index in DifficultyOptions.IDS.size():
		var selected: bool = DifficultyOptions.IDS[index] == difficulty
		difficulty_menu.set_item_checked(difficulty_menu.get_item_index(index), selected)
		if selected:
			popup.set_item_text(popup.get_item_index(DIFFICULTY_MENU_ID), tr("DIFFICULTY_MENU") % tr(DifficultyOptions.LABEL_KEYS[index]))

func _on_language_selected(id: int) -> void:
	var locale: String = GameSettings.LOCALES[id]
	if not GameSettings.save_locale(locale):
		return
	TranslationServer.set_locale(locale)
	update_language_menu()
	update_difficulty_menu()
	move_cost_popup.hide()
	attack_preview_panel.hide()
	language_changed.emit()

func update_language_menu() -> void:
	var popup := menu_button.get_popup()
	for index in GameSettings.LOCALES.size():
		var selected: bool = TranslationServer.get_locale() == GameSettings.LOCALES[index]
		language_menu.set_item_checked(language_menu.get_item_index(index), selected)
		if selected:
			popup.set_item_text(popup.get_item_index(LANGUAGE_MENU_ID), tr("LANGUAGE_MENU") % LANGUAGE_LABELS[index])

func apply_ui_z_order() -> void:
	menu_button.z_index = BattleConfig.MENU_Z_INDEX
	playback_speed_button.z_index = BattleConfig.MENU_Z_INDEX
	$Root/LogPanel.z_index = BattleConfig.UI_Z_BASE
	$Root/LogVisibilityButton.z_index = BattleConfig.UI_Z_BASE
	$Root/LeftColumn.z_index = BattleConfig.UI_Z_BASE
	$Root/LeftColumn/TurnOrder.z_index = BattleConfig.UI_Z_BASE
	$Root/ActionBar.z_index = BattleConfig.UI_Z_BASE
	info_panel.z_index = BattleConfig.UI_Z_INSPECTION
	move_cost_popup.z_index = BattleConfig.UI_Z_TOOLTIP
	attack_preview_panel.z_index = BattleConfig.UI_Z_TOOLTIP
	hovered_skill_card.z_index = BattleConfig.UI_Z_TOOLTIP

func present(snapshot: Dictionary, pending_action: String, inspected_cell: Vector2i, inspected_skill_id: String, status_text: String, selecting_delay: bool) -> void:
	present_status(status_text)
	if snapshot.is_empty():
		actor_name.text = "—"
		actor_portrait.texture = null
		movement.text = ""
		clear_turn_order()
		info_panel.hide()
		move_cost_popup.hide()
		attack_preview_panel.hide()
		hovered_skill_card.hide()
		for button in action_buttons.values():
			button.disabled = true
		$Root/ActionBar/Margin/Layout/EndTurn.disabled = true
		delay_button.disabled = true
		return
	$Root/ActionBar/Margin/Layout/EndTurn.disabled = not snapshot.turn.can_end_turn
	var new_events: Array = snapshot.log
	if not new_events.is_empty():
		var start_index := presented_log_events.size()
		presented_log_events.append_array(new_events)
		for index in range(start_index, presented_log_events.size()):
			log_entry_expanded_states[index] = presented_log_events[index].type != "new_round"
		# append_text 只解析新事件；重設整份文字會隨戰鬥輪數增加造成嚴重效能問題。
		battle_log.append_text(("\n" if start_index > 0 else "") + format_log(new_events, start_index))
	var actor := unit_with_id(snapshot.units, snapshot.turn.actor)
	actor_name.text = localized_unit_name(actor.unit_type) if not actor.is_empty() else "—"
	actor_portrait.texture = load(BattleConfig.unit_art_path(actor.visual)) if not actor.is_empty() else null
	movement.text = tr("BATTLE_MOVEMENT_REMAINING") % int(snapshot.turn.move_remaining)
	present_turn_order(snapshot, selecting_delay)
	delay_button.disabled = not snapshot.turn.can_delay
	delay_button.text = tr("BATTLE_CANCEL_DELAY") if selecting_delay else tr("BATTLE_DELAY")
	presented_skills = snapshot.skill_ranges
	sync_action_buttons()
	if not hovered_skill_id.is_empty() and skill_with_id(presented_skills, hovered_skill_id).is_empty():
		hovered_skill_id = ""
	for action in action_buttons:
		var skill := skill_with_id(presented_skills, action)
		action_buttons[action].visible = not skill.is_empty()
		action_buttons[action].disabled = true
		if not skill.is_empty():
			action_buttons[action].text = localized_skill_name(skill)
			action_buttons[action].disabled = not skill.usable
		action_buttons[action].button_pressed = action == pending_action
	present_hovered_skill()
	var inspected_skill := skill_with_id(presented_skills, inspected_skill_id)
	if not inspected_skill.is_empty():
		info_panel.visible = true
		info_title.text = tr("DETAIL_SKILL")
		detail_tabs.hide()
		unit_name.hide()
		unit_details.visible = false
		inspected_skill_details.visible = true
		inspected_skill_name.text = localized_skill_name(inspected_skill)
		inspected_skill_description.text = localized_skill_details(inspected_skill)
		return
	var terrain := terrain_at(snapshot.terrain_cells, inspected_cell)
	var inspected := not terrain.is_empty()
	info_panel.visible = inspected
	if not inspected:
		return
	info_title.text = tr("DETAIL_TITLE")
	inspected_skill_details.visible = false
	detail_tabs.show()
	var unit := unit_with_id(snapshot.units, terrain.unit_id)
	unit_details.visible = not unit.is_empty()
	unit_name.visible = not unit.is_empty()
	showing_unit_tabs = false
	for index in [DetailTab.ATTRIBUTES, DetailTab.ACTIVE_SKILLS, DetailTab.PASSIVE_SKILLS]:
		detail_tabs.set_tab_hidden(index, unit.is_empty())
	detail_tabs.current_tab = DetailTab.TERRAIN if unit.is_empty() else last_unit_detail_tab
	showing_unit_tabs = not unit.is_empty()
	if inspected_unit_id != terrain.unit_id:
		inspected_unit_id = terrain.unit_id
		for page in detail_tabs.get_children():
			page.scroll_vertical = 0
	if not unit.is_empty():
		unit_name.text = localized_unit_name(unit.unit_type)
		var passive_lines: Array[String] = []
		for passive in unit.passive_skills:
			passive_lines.append("%s：%s" % [tr(BattleConfig.skill_name_key(passive.id)), tr("SKILL_EFFECT_FLANKING") % int(passive.hit_bonus)])
		passive_skills_text.text = "\n\n".join(passive_lines) if not passive_lines.is_empty() else tr("DETAIL_NO_PASSIVE_SKILLS")
		var active_lines: Array[String] = []
		for skill in unit.active_skills:
			active_lines.append("%s\n%s" % [localized_skill_name(skill), localized_skill_details(skill)])
		active_skills_text.text = "\n\n".join(active_lines) if not active_lines.is_empty() else tr("DETAIL_NO_ACTIVE_SKILLS")
		detail_values.team.text = tr("TEAM_ENEMY") + "（%s）" % tr(BattleConfig.team_name_key(unit.team.enemy)) if BattleConfig.is_enemy(unit.team) else tr("TEAM_ALLY")
		detail_values.hp.text = "%d / %d" % [int(unit.hp), int(unit.max_hp)]
		detail_values.size.text = tr("UNIT_SIZE_LARGE") if unit.large else tr("UNIT_SIZE_NORMAL")
		detail_values.movement.text = "%d" % int(unit.movement)
		detail_values.initiative.text = "%d" % int(unit.initiative)
		detail_values.dodge.text = "%d" % int(unit.dodge)
		detail_values.block.text = "%d" % int(unit.block)
		detail_values.block_reduction.text = "%d" % int(unit.block_reduction)
		detail_values.hit.text = "%d" % int(unit.hit)
		detail_values.physical_power.text = "%d" % int(unit.physical_power)
		detail_values.magical_power.text = "%d" % int(unit.magical_power)
		for slot in ["main_hand", "off_hand", "armor", "accessory"]:
			var equipment_id: String = unit.equipment[slot]
			var key := "EQUIPMENT_NAME_%s" % equipment_id.to_upper()
			detail_values[slot].text = tr("EQUIPMENT_NONE") if equipment_id.is_empty() else (equipment_id if tr(key) == key else tr(key))
	var terrain_names: Array[String] = []
	for kind in terrain.terrains:
		terrain_names.append(tr(BattleConfig.terrain_name_key(kind)))
	terrain_name.text = "、".join(terrain_names)
	terrain_cost.text = "%d" % int(terrain.cost) if terrain.passable else tr("TERRAIN_EFFECT_BLOCKED")
	var terrain_descriptions: Array[String] = []
	var terrain_durations: Array[String] = []
	for description in terrain.effect_descriptions:
		var effect_description := localized_terrain_description(description)
		if not effect_description.is_empty():
			terrain_descriptions.append(effect_description)
		if description.values.has("remaining_rounds"):
			terrain_durations.append(tr("TERRAIN_EFFECT_DURATION").format({"value": description.values.remaining_rounds}))
	var terrain_lines: Array[String] = []
	if not terrain_descriptions.is_empty():
		terrain_lines.append("；".join(terrain_descriptions))
	terrain_lines.append_array(terrain_durations)
	terrain_effect.text = "\n".join(terrain_lines) if not terrain_lines.is_empty() else tr("TERRAIN_DESCRIPTION_NONE")

func present_turn_order(snapshot: Dictionary, selecting_delay: bool) -> void:
	turn_order_selecting_delay = selecting_delay
	var current_ids := {}
	var slot_index := 0
	for index in range(snapshot.turn_order.size() - 1, -1, -1):
		var unit := unit_with_id(snapshot.units, snapshot.turn_order[index])
		if unit.is_empty():
			continue
		current_ids[unit.id] = true
		if not turn_order_slots.has(unit.id):
			turn_order_slots[unit.id] = create_turn_order_slot(unit.id)
		var slot: VBoxContainer = turn_order_slots[unit.id]
		var button: Button = slot.get_node("Button")
		button.icon = load(BattleConfig.unit_art_path(unit.visual))
		button.tooltip_text = localized_unit_name(unit.unit_type)
		if slot.get_index() != slot_index:
			turn_order.move_child(slot, slot_index)
		slot_index += 1
		_on_delay_target_hovered(slot.get_node("Marker"), button.is_hovered())
	for unit_id in turn_order_slots.keys():
		if not current_ids.has(unit_id):
			var slot: VBoxContainer = turn_order_slots[unit_id]
			turn_order.remove_child(slot)
			slot.queue_free()
			turn_order_slots.erase(unit_id)

func create_turn_order_slot(unit_id: int) -> VBoxContainer:
	var slot := VBoxContainer.new()
	slot.add_theme_constant_override("separation", 2)
	var marker := ColorRect.new()
	marker.name = "Marker"
	marker.custom_minimum_size = Vector2(112.0, 6.0)
	marker.mouse_filter = Control.MOUSE_FILTER_IGNORE
	marker.color = BattleConfig.DELAY_SLOT_COLOR
	slot.add_child(marker)
	var button := Button.new()
	button.name = "Button"
	button.custom_minimum_size = Vector2(112.0, 72.0)
	button.expand_icon = true
	button.icon_alignment = HORIZONTAL_ALIGNMENT_CENTER
	button.vertical_icon_alignment = VERTICAL_ALIGNMENT_CENTER
	button.mouse_filter = Control.MOUSE_FILTER_STOP
	button.mouse_entered.connect(_on_delay_target_hovered.bind(marker, true))
	button.mouse_exited.connect(_on_delay_target_hovered.bind(marker, false))
	button.pressed.connect(_on_turn_order_pressed.bind(unit_id))
	slot.add_child(button)
	turn_order.add_child(slot)
	return slot

func clear_turn_order() -> void:
	for slot in turn_order_slots.values():
		turn_order.remove_child(slot)
		slot.queue_free()
	turn_order_slots.clear()
	turn_order_selecting_delay = false

func _on_turn_order_pressed(unit_id: int) -> void:
	if turn_order_selecting_delay:
		delay_target_selected.emit(unit_id)
		return
	turn_order_focus_requested.emit(unit_id)

func _on_actor_portrait_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed and event.double_click:
		actor_portrait.accept_event()
		current_unit_focus_requested.emit()

func _on_delay_target_hovered(marker: ColorRect, highlighted: bool) -> void:
	marker.color = BattleConfig.DELAY_SLOT_HIGHLIGHT_COLOR if highlighted and turn_order_selecting_delay else BattleConfig.DELAY_SLOT_COLOR

func present_status(message: String) -> void:
	status.text = tr(message)

func present_move_cost(total_cost, pointer_position: Vector2) -> void:
	move_cost_popup.visible = total_cost != null
	if total_cost == null:
		return
	move_cost_label.text = tr("BATTLE_MOVE_COST") % int(total_cost)
	move_cost_popup.reset_size()
	position_pointer_popup(move_cost_popup, pointer_position, MOVE_COST_POPUP_OFFSET)

func present_attack_preview(preview: Dictionary, pointer_position: Vector2) -> void:
	attack_preview_panel.visible = not preview.is_empty()
	if preview.is_empty():
		return
	var is_healing: bool = preview.kind == "healing"
	healing_preview_segment.visible = is_healing and int(preview.healing) > 0
	attack_preview_critical.get_parent().visible = not is_healing
	attack_preview_resources.text = tr("ATTACK_PREVIEW_RESOURCES") % [int(preview.target_hp), int(preview.target_max_hp), int(preview.target_mana)]
	if is_healing:
		attack_preview_title.text = tr("HEAL_PREVIEW_TITLE") % localized_unit_name(preview.target_type)
		attack_preview_damage.text = tr("HEAL_PREVIEW_AMOUNT") % int(preview.healing)
		attack_preview_markers.get_node("Hit").text = str(int(preview.remaining_hp))
		present_health_segments(preview.health_segments)
		healing_preview_segment.size_flags_stretch_ratio = float(preview.healing)
		layout_attack_preview(pointer_position)
		return
	attack_preview_title.text = tr("ATTACK_PREVIEW_TITLE") % localized_unit_name(preview.target_type)
	attack_preview_damage.text = tr("ATTACK_PREVIEW_DAMAGE") % int(preview.hit_damage)
	attack_preview_result_labels.hit.text = tr("ATTACK_PREVIEW_HIT") % int(preview.hit_chance)
	attack_preview_result_labels.block.text = tr("ATTACK_PREVIEW_BLOCK") % int(preview.block_chance)
	attack_preview_critical.text = tr("ATTACK_PREVIEW_CRITICAL") % int(preview.critical_chance)
	attack_preview_markers.get_node("Hit").text = str(int(preview.hit_remaining_hp))
	attack_preview_markers.get_node("Block").text = str(int(preview.block_remaining_hp))
	present_health_segments(preview.health_segments)
	layout_attack_preview(pointer_position)

func present_health_segments(segment_values: Dictionary) -> void:
	for result in attack_preview_health_segments:
		var segment: ColorRect = attack_preview_health_segments[result]
		segment.visible = segment_values[result] > 0
		segment.size_flags_stretch_ratio = float(segment_values[result])

func layout_attack_preview(pointer_position: Vector2) -> void:
	attack_preview_panel.reset_size()
	position_pointer_popup(attack_preview_panel, pointer_position, ATTACK_PREVIEW_OFFSET)
	position_health_markers.call_deferred()

func position_health_markers() -> void:
	var hit_label: Label = attack_preview_markers.get_node("Hit")
	var block_label: Label = attack_preview_markers.get_node("Block")
	var hit_segment: ColorRect = attack_preview_health_segments.hit
	var block_segment: ColorRect = attack_preview_health_segments.block
	var hit_end := hit_segment.position.x + hit_segment.size.x if hit_segment.visible else 0.0
	if healing_preview_segment.visible:
		hit_end = healing_preview_segment.position.x + healing_preview_segment.size.x
	var hit_color := healing_preview_segment.color if healing_preview_segment.visible else hit_segment.color
	hit_label.add_theme_color_override("font_color", hit_color)
	attack_preview_markers.get_node("HitTick").color = hit_color
	var block_end := block_segment.position.x + block_segment.size.x
	hit_label.reset_size()
	block_label.reset_size()
	hit_label.position = Vector2(clampf(hit_end - hit_label.size.x / 2.0, 0.0, attack_preview_markers.size.x - hit_label.size.x), 0.0)
	block_label.visible = block_segment.visible
	block_label.position = Vector2(clampf(block_end - block_label.size.x / 2.0, 0.0, attack_preview_markers.size.x - block_label.size.x), 0.0)
	var marker_height := maxf(hit_label.size.y, block_label.size.y)
	if block_label.visible and hit_label.get_rect().grow(4.0).intersects(block_label.get_rect()):
		hit_label.position.y = block_label.size.y + 2.0
		marker_height = hit_label.position.y + hit_label.size.y
	attack_preview_markers.custom_minimum_size.y = marker_height
	attack_preview_markers.get_node("HitTick").position = Vector2(hit_end, marker_height)
	attack_preview_markers.get_node("HitTick").size.y = 6.0
	attack_preview_markers.get_node("BlockTick").position = Vector2(block_end, marker_height)
	attack_preview_markers.get_node("BlockTick").size.y = 6.0
	attack_preview_markers.get_node("BlockTick").visible = block_label.visible

func position_pointer_popup(popup: Control, pointer_position: Vector2, pointer_offset: Vector2) -> void:
	var viewport_rect: Rect2 = get_viewport().get_visible_rect()
	var popup_position: Vector2 = pointer_position + pointer_offset
	if popup_position.x + popup.size.x > viewport_rect.end.x:
		popup_position.x = pointer_position.x - pointer_offset.x - popup.size.x
	if popup_position.y + popup.size.y > viewport_rect.end.y:
		popup_position.y = pointer_position.y - pointer_offset.y - popup.size.y
	var maximum_position: Vector2 = viewport_rect.end - popup.size
	popup.position = popup_position.clamp(viewport_rect.position, maximum_position)

func format_log(events: Array, start_index := 0) -> String:
	var entries: Array[String] = []
	for offset in events.size():
		var index: int = start_index + offset
		var event = events[offset]
		var expanded: bool = log_entry_expanded_states[index]
		var marker := "▼" if expanded else "▶"
		match event.type:
			"new_round":
				entries.append("[url=log_entry:%d]%s %s[/url]" % [index, marker, tr("COMBAT_LOG_ROUND") % int(event.round)])
				if expanded:
					for initiative_roll in event.initiative_rolls:
						entries.append(tr("COMBAT_LOG_INITIATIVE_ROLL") % [colored_unit(initiative_roll.unit_type, initiative_roll.team), int(initiative_roll.die_sides), int(initiative_roll.roll), int(initiative_roll.modifier), int(initiative_roll.total)])
			"skill", "healing":
				entries.append("[url=log_entry:%d]%s %s[/url]" % [index, marker, tr("COMBAT_LOG_SKILL_TARGET") % [colored_unit(event.actor_type, event.actor_team), tr(BattleConfig.skill_name_key(event.skill)), colored_unit(event.target_type, event.target_team)]])
				if not expanded:
					continue
				if event.type == "healing":
					entries.append(tr("COMBAT_LOG_HEAL_RESULT") % [RESULT_STYLE % tr("ATTACK_RESULT_HEAL"), int(event.healing), int(event.remaining_hp), int(event.max_hp)])
					continue
				entries.append(tr("COMBAT_LOG_HIT") % [int(event.base_hit), format_modifier_term("DETAIL_SKILL", int(event.skill_hit)), format_modifier_term("COMBAT_LOG_FLANKING_MODIFIER", int(event.flanking_modifier)), int(event.hit)])
				entries.append(tr("COMBAT_LOG_HIT_ROLL") % [int(event.die_sides), int(event.roll), int(event.hit), int(event.hit_total)])
				entries.append(tr("COMBAT_LOG_TARGET_DEFENSE") % [int(event.dodge_target), int(event.block_target)])
				var result: String = RESULT_STYLE % tr("ATTACK_RESULT_%s" % event.result.to_upper())
				var critical := tr("COMBAT_LOG_CRITICAL_SUFFIX") % ("[color=#ff7043]%s[/color]" % tr("ATTACK_RESULT_CRITICAL")) if event.critical else ""
				if event.result == "block":
					entries.append(tr("COMBAT_LOG_ATTACK_RESULT") % [result, critical])
					entries.append(tr("COMBAT_LOG_BLOCK_DAMAGE") % [int(event.raw_damage), int(event.damage_reduction), int(event.damage), int(event.remaining_hp), int(event.max_hp)])
				else:
					entries.append(tr("COMBAT_LOG_DAMAGE_RESULT") % [result, critical, int(event.damage), int(event.remaining_hp), int(event.max_hp)])
				if event.pushed:
					entries.append(tr("COMBAT_LOG_PUSH") % [colored_unit(event.target_type, event.target_team), int(event.push_distance)])
				elif event.push_blocked:
					entries.append(tr("COMBAT_LOG_PUSH_BLOCKED") % [colored_unit(event.target_type, event.target_team), int(event.collision_damage)])
					for collision_unit in event.collision_units:
						entries.append(tr("COMBAT_LOG_COLLISION_DAMAGE") % [colored_unit(collision_unit.unit_type, collision_unit.team), int(event.collision_damage), int(collision_unit.remaining_hp), int(collision_unit.max_hp)])
				if event.downed:
					entries.append(tr("COMBAT_LOG_UNIT_DEFEATED") % colored_unit(event.target_type, event.target_team))
				for collision_unit in event.collision_units:
					if collision_unit.downed:
						entries.append(tr("COMBAT_LOG_UNIT_DEFEATED") % colored_unit(collision_unit.unit_type, collision_unit.team))
			"terrain_created":
				entries.append("[url=log_entry:%d]%s %s[/url]" % [index, marker, tr("COMBAT_LOG_SKILL") % [colored_unit(event.actor_type, event.actor_team), tr(BattleConfig.skill_name_key(event.skill))]])
				if expanded:
					entries.append(tr("COMBAT_LOG_TERRAIN_CREATED") % tr(BattleConfig.terrain_name_key(event.terrain)))
			"terrain_damage":
				var log_key := "COMBAT_LOG_TERRAIN_INSTANT_DOWN" if event.instant_down else "COMBAT_LOG_TERRAIN_DAMAGE"
				var log_text := tr(log_key) % [colored_unit(event.target_type, event.target_team), tr(BattleConfig.terrain_name_key(event.terrain)), int(event.damage), int(event.remaining_hp), int(event.max_hp)]
				entries.append("[url=log_entry:%d]%s %s[/url]" % [index, marker, log_text])
	return "\n".join(entries)

func format_modifier_term(label: String, value: int) -> String:
	if value == 0:
		return ""
	var operator := " + " if value >= 0 else " − "
	return "%s%s %d" % [operator, tr(label), absi(value)]

func toggle_log_entry(index: int) -> void:
	if not log_entry_expanded_states.has(index):
		return
	log_entry_expanded_states[index] = not log_entry_expanded_states[index]
	battle_log.text = format_log(presented_log_events)

func refresh_log_language() -> void:
	battle_log.text = format_log(presented_log_events)

func _on_battle_log_meta_clicked(meta: Variant) -> void:
	var parts := str(meta).split(":")
	if parts.size() == 2 and parts[0] == "log_entry":
		toggle_log_entry(int(parts[1]))

func _on_battle_log_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		dragging_battle_log = event.pressed
	elif event is InputEventMouseMotion and dragging_battle_log:
		var scroll_bar := battle_log.get_v_scroll_bar()
		scroll_bar.value -= event.relative.y
		battle_log.accept_event()

func _on_log_visibility_toggled(hidden: bool) -> void:
	log_panel.visible = not hidden
	log_visibility_button.text = tr("COMBAT_LOG_SHOW") if hidden else tr("COMBAT_LOG_HIDE")

func localized_unit_name(unit_type: String) -> String:
	return tr(BattleConfig.unit_name_key(unit_type))

func colored_unit(unit_type: String, team: Variant) -> String:
	return "[color=%s]%s[/color]" % [TEAM_COLORS["enemy" if BattleConfig.is_enemy(team) else "player"], localized_unit_name(unit_type)]

func _on_action_pressed(action: String) -> void:
	action_selected.emit(action)

func sync_action_buttons() -> void:
	var current_ids := {}
	for index in range(presented_skills.size()):
		var skill: Dictionary = presented_skills[index]
		current_ids[skill.id] = true
		if not action_buttons.has(skill.id):
			var button := Button.new()
			button.custom_minimum_size = Vector2(100, 48)
			button.toggle_mode = true
			button.pressed.connect(_on_action_pressed.bind(skill.id))
			button.mouse_entered.connect(_on_skill_mouse_entered.bind(skill.id))
			button.mouse_exited.connect(_on_skill_mouse_exited.bind(skill.id))
			button.gui_input.connect(_on_skill_gui_input.bind(skill.id))
			action_buttons_container.add_child(button)
			action_buttons[skill.id] = button
		var button: Button = action_buttons[skill.id]
		if button.get_index() != index:
			action_buttons_container.move_child(button, index)
	for action in action_buttons.keys():
		if current_ids.has(action):
			continue
		action_buttons[action].queue_free()
		action_buttons.erase(action)

func _on_skill_mouse_entered(action: String) -> void:
	hovered_skill_id = action
	present_hovered_skill()

func _on_skill_mouse_exited(action: String) -> void:
	if hovered_skill_id == action:
		hovered_skill_id = ""
		present_hovered_skill()

func _on_skill_gui_input(event: InputEvent, action: String) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_RIGHT and event.pressed:
		skill_inspection_requested.emit(action)
		action_buttons[action].accept_event()

func present_hovered_skill() -> void:
	var hovered := skill_with_id(presented_skills, hovered_skill_id)
	hovered_skill_card.visible = not hovered.is_empty()
	if not hovered.is_empty():
		hovered_skill_title.text = localized_skill_name(hovered)
		hovered_skill_details.text = localized_skill_details(hovered)

func localized_skill_name(skill: Dictionary) -> String:
	return tr(BattleConfig.skill_name_key(skill.id))

func localized_skill_details(skill: Dictionary) -> String:
	var details: Dictionary = skill.details
	var lines: Array[String] = []
	lines.append(tr("SKILL_TARGET_%s" % details.target.to_upper()))
	if details.ranged != null:
		lines.append(tr("SKILL_TYPE_RANGED") if details.ranged else tr("SKILL_TYPE_MELEE"))
	if details.min_range != details.max_range:
		lines.append(tr("SKILL_RANGE_INTERVAL") % [int(details.min_range), int(details.max_range)])
	else:
		lines.append(tr("SKILL_RANGE") % int(details.max_range))
	if details.power_source != null:
		lines.append(tr("SKILL_POWER_SOURCE_%s" % details.power_source.to_upper()))
	if details.hit_bonus != null:
		lines.append(tr("SKILL_HIT_BONUS") % int(details.hit_bonus))
	if details.power_bonus != null:
		lines.append(tr("SKILL_POWER_BONUS") % int(details.power_bonus))
	var effect: Dictionary = details.effect
	if effect.kind == "sweep":
		lines.append(tr("SKILL_EFFECT_SWEEP"))
	elif effect.kind == "push":
		lines.append(tr("SKILL_EFFECT_PUSH") % int(effect.distance))
	elif effect.kind == "mire":
		lines.append(tr("SKILL_EFFECT_MIRE") % [int(effect.extra_movement_cost), int(effect.duration)])
	elif effect.kind == "move_then_attack":
		lines.append(tr("SKILL_EFFECT_STEP_THROUGH_THE_SEDGE"))
	elif effect.kind == "heal":
		lines.append(tr("SKILL_EFFECT_HEAL"))
	return "\n".join(lines)

func localized_terrain_description(description: Dictionary) -> String:
	var parts: Array[String] = []
	match description.entry_rule:
		"blocked":
			parts.append(tr("TERRAIN_EFFECT_BLOCKED"))
		"instant_down_when_pushed":
			parts.append(tr("TERRAIN_EFFECT_INSTANT_DOWN"))
	if description.blocks_sight:
		parts.append(tr("TERRAIN_EFFECT_BLOCKS_SIGHT"))
	var value_keys := {
		"dodge_penalty": "TERRAIN_EFFECT_DODGE",
		"block_penalty": "TERRAIN_EFFECT_BLOCK",
		"extra_movement_cost": "TERRAIN_EFFECT_MOVEMENT",
		"damage": "TERRAIN_EFFECT_DAMAGE",
	}
	for key in value_keys:
		if description.values.has(key):
			parts.append(tr(value_keys[key]).format({"value": description.values[key]}))
	return "，".join(parts)

func skill_with_id(skills: Array, skill_id: String) -> Dictionary:
	for skill in skills:
		if skill.id == skill_id:
			return skill
	return {}

func _on_header_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		dragging_info_panel = event.pressed
		if dragging_info_panel:
			drag_offset = event.global_position - info_panel.position
		root.accept_event()
	elif event is InputEventMouseMotion and dragging_info_panel:
		var maximum := root.size - info_panel.size
		info_panel.position = (event.global_position - drag_offset).clamp(Vector2.ZERO, maximum)
		root.accept_event()

func unit_with_id(units: Array, id) -> Dictionary:
	for unit in units:
		if unit.id == id:
			return unit
	return {}

func terrain_at(terrains: Array, cell: Vector2i) -> Dictionary:
	for terrain in terrains:
		if terrain.x == cell.x and terrain.y == cell.y:
			return terrain
	return {}
