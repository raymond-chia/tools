extends CanvasLayer

signal action_selected(action: String)
signal end_turn_requested
signal inspection_closed
signal skill_inspection_requested(skill_id: String)

const TEAM_COLORS := {"player": "#63a9ff", "enemy": "#ff6868"}
const RESULT_STYLE := "[color=#f0c96a]%s[/color]"
const CRITICAL_STYLE := "[color=#ff7043]暴擊[/color]"
const MOVE_COST_POPUP_OFFSET := Vector2(16.0, 16.0)
const ATTACK_PREVIEW_OFFSET := Vector2(18.0, 18.0)

@onready var root: Control = $Root
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
	"power": $Root/InfoPanel/Margin/Content/UnitDetails/Rows/PowerValue,
}
@onready var terrain_name: Label = $Root/InfoPanel/Margin/Content/TerrainRows/TerrainValue
@onready var terrain_cost: Label = $Root/InfoPanel/Margin/Content/TerrainRows/CostValue
@onready var terrain_effect: Label = $Root/InfoPanel/Margin/Content/TerrainRows/EffectValue
@onready var terrain_title: Label = $Root/InfoPanel/Margin/Content/TerrainTitle
@onready var terrain_rows: GridContainer = $Root/InfoPanel/Margin/Content/TerrainRows
@onready var inspected_skill_details: VBoxContainer = $Root/InfoPanel/Margin/Content/SkillDetails
@onready var inspected_skill_name: Label = $Root/InfoPanel/Margin/Content/SkillDetails/Name
@onready var inspected_skill_description: Label = $Root/InfoPanel/Margin/Content/SkillDetails/Details
@onready var actor_name: Label = $Root/BottomBar/Margin/Layout/Actor/Name
@onready var movement: Label = $Root/BottomBar/Margin/Layout/Actor/Movement
@onready var status: Label = $Root/BottomBar/Margin/Layout/Actions/Status
@onready var battle_log: RichTextLabel = $Root/LogPanel/Margin/Content/Entries
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
@onready var action_buttons := {
	"melee_attack": $Root/BottomBar/Margin/Layout/Actions/Buttons/Melee,
	"ranged_attack": $Root/BottomBar/Margin/Layout/Actions/Buttons/Ranged,
	"power_strike": $Root/BottomBar/Margin/Layout/Actions/Buttons/PowerStrike,
	"aimed_shot": $Root/BottomBar/Margin/Layout/Actions/Buttons/AimedShot,
	"shield_bash": $Root/BottomBar/Margin/Layout/Actions/Buttons/ShieldBash,
	"corrosive_mire": $Root/BottomBar/Margin/Layout/Actions/Buttons/CorrosiveMire,
	"heal": $Root/BottomBar/Margin/Layout/Actions/Buttons/Heal,
}
var dragging_info_panel := false
var drag_offset := Vector2.ZERO
var log_entry_expanded_states: Dictionary = {}
var presented_log_events: Array = []
var dragging_battle_log := false
var presented_skills: Array = []
var hovered_skill_id := ""

func _ready() -> void:
	$Root/AttackPreview/Margin/Content/HealthImpactBar.resized.connect(func(): position_health_markers.call_deferred())
	$Root/InfoPanel/Margin/Content/Header.gui_input.connect(_on_header_gui_input)
	$Root/InfoPanel/Margin/Content/Header/Close.pressed.connect(func(): inspection_closed.emit())
	battle_log.meta_clicked.connect(_on_battle_log_meta_clicked)
	battle_log.gui_input.connect(_on_battle_log_gui_input)
	for action in action_buttons:
		action_buttons[action].pressed.connect(_on_action_pressed.bind(action))
		action_buttons[action].mouse_entered.connect(_on_skill_mouse_entered.bind(action))
		action_buttons[action].mouse_exited.connect(_on_skill_mouse_exited.bind(action))
		action_buttons[action].gui_input.connect(_on_skill_gui_input.bind(action))
	$Root/BottomBar/Margin/Layout/EndTurn.pressed.connect(func(): end_turn_requested.emit())

func present(snapshot: Dictionary, pending_action: String, inspected_cell: Vector2i, inspected_skill_id: String, status_text: String) -> void:
	present_status(status_text)
	if snapshot.is_empty():
		actor_name.text = "—"
		movement.text = ""
		info_panel.hide()
		move_cost_popup.hide()
		attack_preview_panel.hide()
		hovered_skill_card.hide()
		for button in action_buttons.values():
			button.disabled = true
		$Root/BottomBar/Margin/Layout/EndTurn.disabled = true
		return
	$Root/BottomBar/Margin/Layout/EndTurn.disabled = not snapshot.turn.can_end_turn
	update_log_entry_states(snapshot.log)
	presented_log_events = snapshot.log.duplicate(true)
	var formatted_log := format_log(presented_log_events)
	if battle_log.text != formatted_log:
		battle_log.text = formatted_log
	var actor := unit_with_id(snapshot.units, snapshot.turn.actor)
	actor_name.text = actor.name if not actor.is_empty() else "—"
	movement.text = "剩餘移動 %d" % int(snapshot.turn.move_remaining)
	presented_skills = snapshot.skill_ranges
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
		info_title.text = "技能"
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
	info_title.text = "詳情"
	inspected_skill_details.visible = false
	terrain_title.visible = true
	terrain_rows.visible = true
	var unit := unit_with_id(snapshot.units, terrain.unit_id)
	unit_details.visible = not unit.is_empty()
	if not unit.is_empty():
		unit_name.text = unit.name
		detail_values.team.text = "我方" if unit.team == "player" else "敵方"
		detail_values.hp.text = "%d / %d" % [int(unit.hp), int(unit.max_hp)]
		detail_values.size.text = "大型" if unit.large else "一般"
		detail_values.movement.text = "%d" % int(unit.movement)
		detail_values.initiative.text = "%d" % int(unit.initiative)
		detail_values.defense.text = "%d / %d" % [int(unit.dodge), int(unit.block)]
		detail_values.attack.text = "%d / %d" % [int(unit.melee), int(unit.ranged)]
		detail_values.power.text = "%d / %d" % [int(unit.damage), int(unit.range)]
	var terrain_names := {"plain": "平地", "rough": "崎嶇地面", "grease": "油膩地面", "spikes": "地刺", "mire": "腐蝕泥沼"}
	terrain_name.text = terrain_names[terrain.kind]
	terrain_cost.text = "%d" % int(terrain.cost)
	terrain_effect.text = localized_detail(terrain.effect_description)

func present_status(message: String) -> void:
	status.text = message

func present_move_cost(total_cost, pointer_position: Vector2) -> void:
	move_cost_popup.visible = total_cost != null
	if total_cost == null:
		return
	move_cost_label.text = "移動消耗 %d" % int(total_cost)
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
		attack_preview_title.text = tr("HEAL_PREVIEW_TITLE") % preview.target
		attack_preview_damage.text = tr("HEAL_PREVIEW_AMOUNT") % int(preview.healing)
		attack_preview_markers.get_node("Hit").text = str(int(preview.remaining_hp))
		present_health_segments(preview.health_segments)
		healing_preview_segment.size_flags_stretch_ratio = float(preview.healing)
		layout_attack_preview(pointer_position)
		return
	attack_preview_title.text = tr("ATTACK_PREVIEW_TITLE") % preview.target
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

func format_log(events: Array) -> String:
	var entries: Array[String] = []
	for index in events.size():
		var event = events[index]
		var expanded: bool = log_entry_expanded_states[index]
		var marker := "▼" if expanded else "▶"
		match event.type:
			"new_round":
				entries.append("[url=log_entry:%d]%s ── 第 %d 輪 ──[/url]" % [index, marker, int(event.round)])
				if expanded:
					for initiative_roll in event.initiative_rolls:
						entries.append("%s：D%d 擲骰 %d + 先攻加值 %d = 先攻總值 %d" % [colored_unit(initiative_roll.unit, initiative_roll.team), int(initiative_roll.die_sides), int(initiative_roll.roll), int(initiative_roll.modifier), int(initiative_roll.total)])
			"skill", "healing":
				entries.append("[url=log_entry:%d]%s %s 使用「%s」影響 %s[/url]" % [index, marker, colored_unit(event.actor, event.actor_team), event.skill, colored_unit(event.target, event.target_team)])
				if not expanded:
					continue
				if event.type == "healing":
					entries.append("結果：%s，回復 %d HP，HP %d/%d" % [RESULT_STYLE % "治療", int(event.healing), int(event.remaining_hp), int(event.max_hp)])
					continue
				var attack_stat_names := {"melee": "近戰", "ranged": "遠程"}
				entries.append("攻擊加值：%s %d%s%s = %d" % [attack_stat_names[event.attack_stat], int(event.attack_stat_modifier), format_modifier_term("技能", int(event.skill_attack_modifier)), format_modifier_term("包抄", int(event.flanking_modifier)), int(event.attack_modifier)])
				entries.append("D%d 擲骰 %d + 攻擊加值 %d = 攻擊總值 %d" % [int(event.die_sides), int(event.roll), int(event.attack_modifier), int(event.attack_total)])
				entries.append("目標防禦：閃避門檻 %d／格擋門檻 %d" % [int(event.dodge_target), int(event.block_target)])
				var result_names := {"dodge": "閃避", "block": "格擋", "hit": "命中"}
				var result: String = RESULT_STYLE % result_names[event.result]
				var critical := "，%s" % CRITICAL_STYLE if event.critical else ""
				if event.result == "block":
					entries.append("結果：%s%s" % [result, critical])
					entries.append("傷害：原始 %d − 格擋 %d = %d，HP %d/%d" % [int(event.raw_damage), int(event.damage_reduction), int(event.damage), int(event.remaining_hp), int(event.max_hp)])
				else:
					entries.append("結果：%s%s，%d 傷害，HP %d/%d" % [result, critical, int(event.damage), int(event.remaining_hp), int(event.max_hp)])
				if event.pushed:
					entries.append("%s 被沿攻擊方向推動 %d 格" % [colored_unit(event.target, event.target_team), int(event.push_distance)])
				elif event.push_blocked:
					entries.append("推擊受阻，%s 額外受到 %d 點碰撞傷害" % [colored_unit(event.target, event.target_team), int(event.collision_damage)])
				if event.downed:
					entries.append("%s 倒下" % colored_unit(event.target, event.target_team))
			"terrain_created":
				var terrain_names := {"mire": "腐蝕泥沼"}
				entries.append("[url=log_entry:%d]%s %s 使用「%s」[/url]" % [index, marker, colored_unit(event.actor, event.actor_team), event.skill])
				if expanded:
					entries.append("產生「%s」" % terrain_names[event.terrain])
			"status_applied":
				var status_names := {"grease": "油脂"}
				entries.append("[url=log_entry:%d]%s 狀態變化[/url]" % [index, marker])
				if expanded:
					entries.append("%s 受到「%s」狀態影響" % [colored_unit(event.target, event.target_team), status_names[event.status]])
			"terrain_damage":
				var terrain_names := {"spikes": "地刺"}
				entries.append("[url=log_entry:%d]%s %s 踩到「%s」[/url]" % [index, marker, colored_unit(event.target, event.target_team), terrain_names[event.terrain]])
				if expanded:
					entries.append("%s 踩到「%s」，受到 %d 點傷害，HP %d/%d" % [colored_unit(event.target, event.target_team), terrain_names[event.terrain], int(event.damage), int(event.remaining_hp), int(event.max_hp)])
					if event.downed:
						entries.append("%s 倒下" % colored_unit(event.target, event.target_team))
	return "\n".join(entries)

func format_modifier_term(label: String, value: int) -> String:
	var operator := " + " if value >= 0 else " − "
	return "%s%s %d" % [operator, label, absi(value)]

func update_log_entry_states(events: Array) -> void:
	var unchanged_count := 0
	var comparable_count: int = mini(events.size(), presented_log_events.size())
	while unchanged_count < comparable_count and events[unchanged_count] == presented_log_events[unchanged_count]:
		unchanged_count += 1
	for index in range(unchanged_count, presented_log_events.size()):
		log_entry_expanded_states.erase(index)
	for index in range(unchanged_count, events.size()):
		if not log_entry_expanded_states.has(index):
			log_entry_expanded_states[index] = events[index].type != "new_round"

func toggle_log_entry(index: int) -> void:
	if not log_entry_expanded_states.has(index):
		return
	log_entry_expanded_states[index] = not log_entry_expanded_states[index]
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

func colored_unit(unit: String, team: String) -> String:
	return "[color=%s]%s[/color]" % [TEAM_COLORS[team], unit]

func _on_action_pressed(action: String) -> void:
	action_selected.emit(action)

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
	return tr(skill.name_key)

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
