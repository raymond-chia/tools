extends CanvasLayer

signal action_selected(action: String)
signal end_turn_requested
signal delay_selection_requested
signal delay_target_selected(unit_id: String)
signal turn_order_focus_requested(unit_id: String)
signal inspection_closed
signal skill_inspection_requested(skill_id: String)
signal language_changed

const TEAM_COLORS := {"player": "#63a9ff", "enemy": "#ff6868"}
const RESULT_STYLE := "[color=#f0c96a]%s[/color]"
const LANGUAGE_SETTINGS := "user://language.cfg"
const MOVE_COST_POPUP_OFFSET := Vector2(16.0, 16.0)
const ATTACK_PREVIEW_OFFSET := Vector2(18.0, 18.0)

@onready var root: Control = $Root
@onready var menu_button: MenuButton = $Root/Menu
@onready var info_panel: Panel = $Root/InfoPanel
@onready var info_title: Label = $Root/InfoPanel/Margin/Content/Header/Title
@onready var unit_details: VBoxContainer = $Root/InfoPanel/Margin/Content/UnitDetails
@onready var unit_name: Label = $Root/InfoPanel/Margin/Content/UnitDetails/UnitName
@onready var detail_values := {
	"team": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/TeamValue,
	"hp": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/HpValue,
	"size": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/SizeValue,
	"movement": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/MovementValue,
	"initiative": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/InitiativeValue,
	"defense": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/DefenseValue,
	"attack": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/AttackValue,
	"damage": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/DamageValue,
}
@onready var terrain_name: Label = $Root/InfoPanel/Margin/Content/TerrainRows/TerrainValue
@onready var terrain_cost: Label = $Root/InfoPanel/Margin/Content/TerrainRows/CostValue
@onready var terrain_effect: Label = $Root/InfoPanel/Margin/Content/TerrainRows/EffectValue
@onready var terrain_title: Label = $Root/InfoPanel/Margin/Content/TerrainTitle
@onready var terrain_rows: GridContainer = $Root/InfoPanel/Margin/Content/TerrainRows
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
var dragging_info_panel := false
var drag_offset := Vector2.ZERO
var log_entry_expanded_states: Dictionary = {}
var presented_log_events: Array = []
var dragging_battle_log := false
var presented_skills: Array = []
var hovered_skill_id := ""

func _ready() -> void:
	var settings := ConfigFile.new()
	if settings.load(LANGUAGE_SETTINGS) == OK:
		TranslationServer.set_locale(settings.get_value("language", "locale", "zh_TW"))
	else:
		TranslationServer.set_locale("zh_TW")
	var popup := menu_button.get_popup()
	popup.add_radio_check_item("繁體中文", 0)
	popup.add_radio_check_item("English", 1)
	popup.id_pressed.connect(_on_language_selected)
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

func _on_language_selected(id: int) -> void:
	TranslationServer.set_locale("zh_TW" if id == 0 else "en")
	var settings := ConfigFile.new()
	settings.set_value("language", "locale", TranslationServer.get_locale())
	settings.save(LANGUAGE_SETTINGS)
	update_language_menu()
	move_cost_popup.hide()
	attack_preview_panel.hide()
	language_changed.emit()

func update_language_menu() -> void:
	var popup := menu_button.get_popup()
	popup.set_item_checked(0, TranslationServer.get_locale().begins_with("zh"))
	popup.set_item_checked(1, TranslationServer.get_locale().begins_with("en"))

func apply_ui_z_order() -> void:
	menu_button.z_index = BattleConfig.MENU_Z_INDEX
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
	movement.text = tr("剩餘移動 %d") % int(snapshot.turn.move_remaining)
	present_turn_order(snapshot, selecting_delay)
	delay_button.disabled = not snapshot.turn.can_delay
	delay_button.text = tr("取消延後") if selecting_delay else tr("延後")
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
			action_buttons[action].disabled = not skill.enabled
		action_buttons[action].button_pressed = action == pending_action
	present_hovered_skill()
	var inspected_skill := skill_with_id(presented_skills, inspected_skill_id)
	if not inspected_skill.is_empty():
		info_panel.visible = true
		info_title.text = tr("技能")
		unit_details.visible = false
		terrain_title.visible = false
		terrain_rows.visible = false
		inspected_skill_details.visible = true
		inspected_skill_name.text = localized_skill_name(inspected_skill)
		inspected_skill_description.text = localized_skill_details(inspected_skill)
		return
	var terrain := terrain_at(snapshot.terrain_cells, inspected_cell)
	var inspected := not terrain.is_empty()
	info_panel.visible = inspected
	if not inspected:
		return
	info_title.text = tr("詳情")
	inspected_skill_details.visible = false
	terrain_title.visible = true
	terrain_rows.visible = true
	var unit := unit_with_id(snapshot.units, terrain.unit_id)
	unit_details.visible = not unit.is_empty()
	if not unit.is_empty():
		unit_name.text = localized_unit_name(unit.unit_type)
		detail_values.team.text = tr("我方") if unit.team is String else tr("敵方") + "（%s）" % unit.team.enemy
		detail_values.hp.text = "%d / %d" % [int(unit.hp), int(unit.max_hp)]
		detail_values.size.text = tr("大型") if unit.large else tr("一般")
		detail_values.movement.text = "%d" % int(unit.movement)
		detail_values.initiative.text = "%d" % int(unit.initiative)
		detail_values.defense.text = "%d / %d" % [int(unit.dodge), int(unit.block)]
		detail_values.attack.text = "%d" % int(unit.attack)
		detail_values.damage.text = "%d" % int(unit.damage)
	var terrain_names: Array[String] = []
	for kind in terrain.terrains:
		terrain_names.append(tr(BattleConfig.terrain_name_key(kind)))
	terrain_name.text = "、".join(terrain_names) if not terrain_names.is_empty() else tr(BattleConfig.terrain_name_key("plain"))
	terrain_cost.text = "%d" % int(terrain.cost) if terrain.passable else tr("無法通行")
	var terrain_descriptions: Array[String] = []
	for description in terrain.effect_descriptions:
		terrain_descriptions.append(localized_detail(description))
	terrain_effect.text = "；".join(terrain_descriptions) if not terrain_descriptions.is_empty() else tr("TERRAIN_EFFECT_NONE")

func present_turn_order(snapshot: Dictionary, selecting_delay: bool) -> void:
	clear_turn_order()
	for index in range(snapshot.turn_order.size() - 1, -1, -1):
		var unit := unit_with_id(snapshot.units, snapshot.turn_order[index])
		if unit.is_empty():
			continue
		var slot := VBoxContainer.new()
		slot.add_theme_constant_override("separation", 2)
		var marker := ColorRect.new()
		marker.custom_minimum_size = Vector2(112.0, 6.0)
		marker.mouse_filter = Control.MOUSE_FILTER_IGNORE
		marker.color = BattleConfig.DELAY_SLOT_COLOR
		slot.add_child(marker)
		var button := Button.new()
		button.custom_minimum_size = Vector2(112.0, 72.0)
		button.icon = load(BattleConfig.unit_art_path(unit.visual))
		button.expand_icon = true
		button.icon_alignment = HORIZONTAL_ALIGNMENT_CENTER
		button.vertical_icon_alignment = VERTICAL_ALIGNMENT_CENTER
		button.tooltip_text = localized_unit_name(unit.unit_type)
		button.mouse_filter = Control.MOUSE_FILTER_STOP
		button.mouse_entered.connect(_on_delay_target_hovered.bind(marker, true, selecting_delay))
		button.mouse_exited.connect(_on_delay_target_hovered.bind(marker, false, selecting_delay))
		button.pressed.connect(_on_turn_order_pressed.bind(unit.id, selecting_delay))
		slot.add_child(button)
		turn_order.add_child(slot)

func clear_turn_order() -> void:
	for child in turn_order.get_children():
		turn_order.remove_child(child)
		child.queue_free()

func _on_turn_order_pressed(unit_id: String, selecting_delay: bool) -> void:
	if selecting_delay:
		delay_target_selected.emit(unit_id)
		return
	turn_order_focus_requested.emit(unit_id)

func _on_delay_target_hovered(marker: ColorRect, highlighted: bool, selecting_delay: bool) -> void:
	marker.color = BattleConfig.DELAY_SLOT_HIGHLIGHT_COLOR if highlighted and selecting_delay else BattleConfig.DELAY_SLOT_COLOR

func present_status(message: String) -> void:
	status.text = tr(message)

func present_move_cost(total_cost, pointer_position: Vector2) -> void:
	move_cost_popup.visible = total_cost != null
	if total_cost == null:
		return
	move_cost_label.text = tr("移動消耗 %d") % int(total_cost)
	move_cost_popup.reset_size()
	position_pointer_popup(move_cost_popup, pointer_position, MOVE_COST_POPUP_OFFSET)

func present_attack_preview(preview: Dictionary, pointer_position: Vector2) -> void:
	attack_preview_panel.visible = not preview.is_empty()
	if preview.is_empty():
		return
	var is_healing := preview.has("healing")
	healing_preview_segment.get_parent().move_child(healing_preview_segment, 1 if is_healing else 4)
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
				entries.append("[url=log_entry:%d]%s %s[/url]" % [index, marker, tr("── 第 %d 輪 ──") % int(event.round)])
				if expanded:
					for initiative_roll in event.initiative_rolls:
						entries.append(tr("%s：D%d 擲骰 %d + 先攻加值 %d = 先攻總值 %d") % [colored_unit(initiative_roll.unit_type, initiative_roll.team), int(initiative_roll.die_sides), int(initiative_roll.roll), int(initiative_roll.modifier), int(initiative_roll.total)])
			"skill", "healing":
				entries.append("[url=log_entry:%d]%s %s[/url]" % [index, marker, tr("%s 使用「%s」影響 %s") % [colored_unit(event.actor_type, event.actor_team), tr(event.skill), colored_unit(event.target_type, event.target_team)]])
				if not expanded:
					continue
				if event.type == "healing":
					entries.append(tr("結果：%s，回復 %d HP，HP %d/%d") % [RESULT_STYLE % tr("治療"), int(event.healing), int(event.remaining_hp), int(event.max_hp)])
					continue
				entries.append(tr("攻擊加值：%d%s%s = %d") % [int(event.attack_stat_modifier), format_modifier_term("技能", int(event.skill_attack_modifier)), format_modifier_term("包抄", int(event.flanking_modifier)), int(event.attack_modifier)])
				entries.append(tr("D%d 擲骰 %d + 攻擊加值 %d = 攻擊總值 %d") % [int(event.die_sides), int(event.roll), int(event.attack_modifier), int(event.attack_total)])
				entries.append(tr("目標防禦：閃避門檻 %d／格擋門檻 %d") % [int(event.dodge_target), int(event.block_target)])
				var result: String = RESULT_STYLE % tr("ATTACK_RESULT_%s" % event.result.to_upper())
				var critical := tr("，%s") % ("[color=#ff7043]%s[/color]" % tr("暴擊")) if event.critical else ""
				if event.result == "block":
					entries.append(tr("結果：%s%s") % [result, critical])
					entries.append(tr("傷害：原始 %d − 格擋 %d = %d，HP %d/%d") % [int(event.raw_damage), int(event.damage_reduction), int(event.damage), int(event.remaining_hp), int(event.max_hp)])
				else:
					entries.append(tr("結果：%s%s，%d 傷害，HP %d/%d") % [result, critical, int(event.damage), int(event.remaining_hp), int(event.max_hp)])
				if event.pushed:
					entries.append(tr("%s 被沿攻擊方向推動 %d 格") % [colored_unit(event.target_type, event.target_team), int(event.push_distance)])
				elif event.push_blocked:
					entries.append(tr("推擊受阻，%s 額外受到 %d 點碰撞傷害") % [colored_unit(event.target_type, event.target_team), int(event.collision_damage)])
					for collision_unit in event.collision_units:
						entries.append(tr("%s 被撞擊，受到 %d 點碰撞傷害，HP %d/%d") % [colored_unit(collision_unit.unit_type, collision_unit.team), int(event.collision_damage), int(collision_unit.remaining_hp), int(collision_unit.max_hp)])
				if event.downed:
					entries.append(tr("%s 倒下") % colored_unit(event.target_type, event.target_team))
				for collision_unit in event.collision_units:
					if collision_unit.downed:
						entries.append(tr("%s 倒下") % colored_unit(collision_unit.unit_type, collision_unit.team))
			"terrain_created":
				entries.append("[url=log_entry:%d]%s %s[/url]" % [index, marker, tr("%s 使用「%s」") % [colored_unit(event.actor_type, event.actor_team), tr(event.skill)]])
				if expanded:
					entries.append(tr("產生「%s」") % tr(BattleConfig.terrain_name_key(event.terrain)))
			"terrain_damage":
				var log_text := tr(event.log_key) % [colored_unit(event.target_type, event.target_team), tr(BattleConfig.terrain_name_key(event.terrain)), int(event.damage), int(event.remaining_hp), int(event.max_hp)]
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
	log_visibility_button.text = tr("顯示紀錄") if hidden else tr("隱藏紀錄")

func localized_unit_name(unit_type: String) -> String:
	return tr(BattleConfig.unit_name_key(unit_type))

func colored_unit(unit_type: String, team: Variant) -> String:
	return "[color=%s]%s[/color]" % [TEAM_COLORS["player" if team is String else "enemy"], localized_unit_name(unit_type)]

func _on_action_pressed(action: String) -> void:
	action_selected.emit(action)

func sync_action_buttons() -> void:
	var current_ids := {}
	for skill in presented_skills:
		current_ids[skill.id] = true
		if action_buttons.has(skill.id):
			continue
		var button := Button.new()
		button.custom_minimum_size = Vector2(100, 48)
		button.toggle_mode = true
		button.pressed.connect(_on_action_pressed.bind(skill.id))
		button.mouse_entered.connect(_on_skill_mouse_entered.bind(skill.id))
		button.mouse_exited.connect(_on_skill_mouse_exited.bind(skill.id))
		button.gui_input.connect(_on_skill_gui_input.bind(skill.id))
		action_buttons_container.add_child(button)
		action_buttons[skill.id] = button
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
	var lines: Array[String] = []
	for detail in skill.details:
		lines.append(localized_detail(detail))
	return "\n".join(lines)

func localized_detail(detail: Dictionary) -> String:
	var text := tr(detail.text_key)
	var values: Array[int] = []
	for value in detail.arguments:
		values.append(int(value))
	return text if values.is_empty() else text % values

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
