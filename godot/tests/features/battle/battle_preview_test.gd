extends GdUnitTestSuite

const BATTLE_SCENE := "res://features/battle/battle.tscn"
const TEST_DEFINITION := "res://tests/features/battle/data/battle_preview.toml"
const ATTACK_VARIATIONS_DEFINITION := "res://tests/features/battle/data/attack_preview_variations.toml"

var battle
var runner: GdUnitSceneRunner

func before_test() -> void:
	runner = scene_runner(BATTLE_SCENE)
	runner.set_time_factor(9.0)
	await runner.simulate_frames(1)
	battle = runner.scene()

# 驗證懸停敵人的真實核心預覽、資源與傷害排版、金色外環，以及無格擋時的分段顯示。
func test_attack_hit_preview() -> void:
	await prepare_case(battle)
	push_control_click(battle.ui.action_buttons.aimed_shot)
	push_mouse_motion(battle.world, battle.world.cell_center(Vector2i(3, 1)))

	var preview: Dictionary = battle.world.attack_preview
	var target_node: Node2D = battle.world.unit_nodes[battle.world.attack_preview_unit.id]
	var attack_preview_ring: Sprite2D = target_node.get_node("Visual/AttackPreviewRing")
	var target_base: Sprite2D = target_node.get_node("Visual/Base")
	assert_bool(preview.is_empty()).override_failure_message("懸停可攻擊敵人時應取得命中預覽").is_false()
	assert_int(int(preview.dodge_chance)).is_equal(10)
	assert_int(int(preview.block_chance)).is_equal(20)
	assert_int(int(preview.hit_chance)).is_equal(70)
	assert_int(int(preview.critical_chance)).is_equal(5)
	assert_int(int(preview.dodge_damage)).is_zero()
	assert_int(int(preview.block_damage)).is_equal(3)
	assert_int(int(preview.critical_block_damage)).is_equal(6)
	assert_int(int(preview.hit_damage)).is_equal(5)
	assert_int(int(preview.critical_hit_damage)).is_equal(10)
	assert_int(int(preview.target_hp)).is_equal(20)
	assert_int(int(preview.target_max_hp)).is_equal(20)
	assert_int(int(preview.target_mana)).is_equal(1)
	assert_int(int(preview.hit_remaining_hp)).is_equal(15)
	assert_int(int(preview.block_remaining_hp)).is_equal(17)
	assert_int(int(preview.dodge_remaining_hp)).is_equal(20)
	assert_bool(battle.ui.attack_preview_panel.visible).override_failure_message("命中預覽面板應顯示").is_true()
	assert_str(battle.ui.attack_preview_title.text).override_failure_message("面板標題應顯示目標名稱").contains(preview.target)
	assert_str(battle.ui.attack_preview_resources.text).override_failure_message("資源列應只顯示目標 HP 與 MP").is_equal("HP 20 / 20｜MP 1")
	assert_str(battle.ui.attack_preview_damage.text).override_failure_message("獨立傷害欄應顯示正常命中傷害").is_equal("傷害 5")
	assert_str(battle.ui.attack_preview_result_labels.hit.text).override_failure_message("左欄應顯示命中機率").is_equal("命中 70%")
	assert_str(battle.ui.attack_preview_result_labels.block.text).override_failure_message("中欄應顯示格擋機率").is_equal("格擋 20%")
	assert_bool(battle.ui.attack_preview_health_segments.hit.visible).is_true()
	assert_bool(battle.ui.attack_preview_health_segments.block.visible).is_true()
	assert_bool(battle.ui.attack_preview_health_segments.damage.visible).is_true()
	assert_bool(battle.ui.attack_preview_health_segments.missing.visible).is_false()
	assert_float(battle.ui.attack_preview_health_segments.hit.size_flags_stretch_ratio).is_equal(15.0)
	assert_float(battle.ui.attack_preview_health_segments.block.size_flags_stretch_ratio).is_equal(2.0)
	assert_float(battle.ui.attack_preview_health_segments.damage.size_flags_stretch_ratio).is_equal(3.0)
	assert_float(battle.ui.attack_preview_health_segments.missing.size_flags_stretch_ratio).is_zero()
	assert_str(battle.ui.attack_preview_critical.text).override_failure_message("爆擊率應獨立標註且不重複顯示兩倍傷害").is_equal("暴擊率 5%")
	assert_int(battle.ui.get_node("Root/LogPanel").z_index).override_failure_message("戰鬥紀錄應位於 inspect panel 下層").is_less(battle.ui.info_panel.z_index)
	assert_int(battle.ui.attack_preview_panel.z_index).override_failure_message("瞄準預覽應位於 inspect panel 上層").is_greater(battle.ui.info_panel.z_index)
	assert_bool(attack_preview_ring.visible).override_failure_message("命中預覽目標應顯示金色單位外環").is_true()
	assert_bool(attack_preview_ring.scale.x > target_base.scale.x and attack_preview_ring.scale.y > target_base.scale.y).override_failure_message("金色預覽環應套在單位原光環外側").is_true()
	assert_bool(attack_preview_ring.modulate == Color("ffe17a")).override_failure_message("命中預覽目標外環應為金色").is_true()
	var zero_block_preview := query_attack_preview("wolf_b", "preview_5")
	battle.ui.present_attack_preview(zero_block_preview, Vector2.ZERO)
	assert_str(battle.ui.attack_preview_result_labels.block.text).override_failure_message("格擋率為零時中間欄仍應顯示格擋機率").is_equal("格擋 0%")
	assert_bool(battle.ui.attack_preview_health_segments.block.visible).override_failure_message("沒有格擋結果時不應顯示黃色情境").is_false()
	assert_float(battle.ui.attack_preview_health_segments.block.size_flags_stretch_ratio).override_failure_message("沒有格擋結果時黃色情境應歸零").is_zero()

	push_mouse_motion(battle.world, battle.world.cell_center(Vector2i(2, 2)))
	assert_dict(battle.world.attack_preview).override_failure_message("離開敵人後應清除命中預覽").is_empty()
	assert_bool(battle.ui.attack_preview_panel.visible).override_failure_message("離開敵人後應隱藏命中預覽面板").is_false()
	assert_bool(attack_preview_ring.visible).override_failure_message("離開敵人後應隱藏金色單位外環").is_false()

# 驗證專用 TOML 經真實核心產生的各種血條分段、殘餘 HP 端點、傷害排版與數字避讓。
func test_attack_preview_visual_variations() -> void:
	await prepare_case(battle)
	# 本案例呈現另一個真實核心的預覽，停用地圖輸入以免滑鼠事件清除預覽。
	battle.world.set_process_unhandled_input(false)
	var test_data := [
		# 殘餘 HP 端點靠近時，放大的數字應上下錯開而不重疊。
		{"name": "數字靠近", "target": "ogre", "skill": "preview_5", "preparation": "setup_damage_20", "dodge": 10, "block": 20, "hit": 70, "damage": 5, "target_hp": 80, "target_max_hp": 100, "hit_remaining_hp": 75, "block_remaining_hp": 77, "expected_segments": [75, 2, 3, 20]},
		# 四色同時出現時，確認顏色順序與相鄰分段連續。
		{"name": "四色血條", "target": "wolf_a", "skill": "preview_5", "preparation": "setup_damage_4", "dodge": 10, "block": 20, "hit": 70, "damage": 5, "target_hp": 16, "target_max_hp": 20, "hit_remaining_hp": 11, "block_remaining_hp": 13, "expected_segments": [11, 2, 3, 4]},
		# 格擋完全吸收傷害時，黃色代表減免傷害且紅色隱藏。
		{"name": "完全格擋", "target": "lyra", "skill": "preview_2", "preparation": "", "dodge": 15, "block": 50, "hit": 35, "damage": 2, "target_hp": 20, "target_max_hp": 20, "hit_remaining_hp": 18, "block_remaining_hp": 20, "expected_segments": [18, 2, 0, 0]},
		# 沒有格擋率時，紅色顯示普通命中傷害且深色保留既有損失生命。
		{"name": "無格擋的高傷害命中", "target": "wolf_b", "skill": "preview_7", "preparation": "setup_damage_4", "dodge": 40, "block": 0, "hit": 60, "damage": 7, "target_hp": 16, "target_max_hp": 20, "hit_remaining_hp": 9, "block_remaining_hp": 11, "expected_segments": [9, 0, 7, 4]},
		# 致死命中隱藏綠色後，黃色仍須從血條左端開始。
		{"name": "致死命中", "target": "wolf_a", "skill": "preview_16", "preparation": "setup_damage_4", "dodge": 10, "block": 20, "hit": 70, "damage": 16, "target_hp": 16, "target_max_hp": 20, "hit_remaining_hp": 0, "block_remaining_hp": 2, "expected_segments": [0, 2, 14, 4]},
		# 滿血且無傷害時，只剩綠色且仍填滿整條血條。
		{"name": "只有綠色", "target": "wolf_a", "skill": "preview_0", "preparation": "", "dodge": 10, "block": 20, "hit": 70, "damage": 0, "target_hp": 20, "target_max_hp": 20, "hit_remaining_hp": 20, "block_remaining_hp": 20, "expected_segments": [20, 0, 0, 0]},
		# 已隱藏的分段重新出現時，排版與比例仍須正確。
		{"name": "恢復四色血條", "target": "wolf_a", "skill": "preview_5", "preparation": "setup_damage_4", "dodge": 10, "block": 20, "hit": 70, "damage": 5, "target_hp": 16, "target_max_hp": 20, "hit_remaining_hp": 11, "block_remaining_hp": 13, "expected_segments": [11, 2, 3, 4]},
	]
	for test_case in test_data:
		var preview := query_attack_preview(test_case.target, test_case.skill, test_case.preparation)
		battle.ui.present_attack_preview(preview, Vector2.ZERO)
		await runner.simulate_frames(2)
		var expected_segments: Array = test_case.expected_segments
		assert_str(battle.ui.attack_preview_resources.text).override_failure_message("%s：資源列應只顯示 HP 與 MP" % test_case.name).is_equal("HP %d / %d｜MP 1" % [test_case.target_hp, test_case.target_max_hp])
		assert_str(battle.ui.attack_preview_damage.text).override_failure_message("%s：獨立傷害欄應顯示正常命中傷害" % test_case.name).is_equal("傷害 %d" % test_case.damage)
		var resource_rect: Rect2 = battle.ui.attack_preview_resources.get_global_rect()
		var damage_rect: Rect2 = battle.ui.attack_preview_damage.get_global_rect()
		var resource_row: Control = battle.ui.attack_preview_damage.get_parent()
		assert_bool(damage_rect.position.x - resource_rect.end.x >= 24.0).override_failure_message("%s：傷害應與資源保留間距" % test_case.name).is_true()
		assert_bool(absf(damage_rect.end.x - resource_row.get_global_rect().end.x) <= 1.0).override_failure_message("%s：傷害欄應靠資源列右端" % test_case.name).is_true()
		assert_str(battle.ui.attack_preview_result_labels.hit.text).override_failure_message("%s：命中率應正確顯示" % test_case.name).is_equal("命中 %d%%" % test_case.hit)
		assert_str(battle.ui.attack_preview_result_labels.block.text).override_failure_message("%s：格擋率應正確顯示" % test_case.name).is_equal("格擋 %d%%" % test_case.block)
		assert_int(int(preview.dodge_chance)).override_failure_message("%s：閃避率應保留在預覽資料" % test_case.name).is_equal(test_case.dodge)
		for index in battle.ui.attack_preview_health_segments.size():
			var segment: ColorRect = battle.ui.attack_preview_health_segments.values()[index]
			var expected_value: int = expected_segments[index]
			assert_bool(segment.visible).override_failure_message("%s：血條分段可見性應正確" % test_case.name).is_equal(expected_value > 0)
			assert_float(segment.size_flags_stretch_ratio).override_failure_message("%s：血條分段比例應正確" % test_case.name).is_equal(float(expected_value))
		assert_attack_preview_bar(test_case.name, expected_segments, test_case.target_max_hp)
		assert_attack_preview_markers(test_case)

# 檢查真實排版中的數字內容、刻線位置、邊界與單排／雙排高度。
func assert_attack_preview_markers(test_case: Dictionary) -> void:
	var markers: Control = battle.ui.attack_preview_markers
	var hit_label: Label = markers.get_node("Hit")
	var block_label: Label = markers.get_node("Block")
	var hit_tick: Control = markers.get_node("HitTick")
	var block_tick: Control = markers.get_node("BlockTick")
	var bar: Control = battle.ui.get_node("Root/AttackPreview/Margin/Content/HealthImpactBar")
	assert_str(hit_label.text).is_equal(str(test_case.hit_remaining_hp))
	assert_str(block_label.text).is_equal(str(test_case.block_remaining_hp))
	assert_int(hit_label.get_theme_font_size("font_size")).is_equal(20)
	assert_int(block_label.get_theme_font_size("font_size")).is_equal(20)
	var has_distinct_block: bool = test_case.block > 0 and test_case.block_remaining_hp != test_case.hit_remaining_hp
	assert_bool(block_label.visible).override_failure_message("%s：無格擋或血量相同時只顯示一個數字" % test_case.name).is_equal(has_distinct_block)
	assert_bool(block_tick.visible).is_equal(has_distinct_block)
	assert_bool(markers.get_rect().size.x >= hit_label.get_rect().end.x and hit_label.position.x >= 0.0).override_failure_message("%s：命中數字不得超出左右邊界" % test_case.name).is_true()
	var expected_hit_x: float = bar.size.x * float(test_case.hit_remaining_hp) / float(test_case.target_max_hp)
	assert_bool(absf(hit_tick.position.x - expected_hit_x) <= 1.0).override_failure_message("%s：命中刻線應對準殘餘 HP 端點" % test_case.name).is_true()
	if has_distinct_block:
		var expected_block_x: float = bar.size.x * float(test_case.block_remaining_hp) / float(test_case.target_max_hp)
		assert_bool(absf(block_tick.position.x - expected_block_x) <= 1.0).override_failure_message("%s：格擋刻線應對準殘餘 HP 端點" % test_case.name).is_true()
		assert_bool(hit_label.get_rect().intersects(block_label.get_rect())).override_failure_message("%s：殘餘 HP 數字不得重疊" % test_case.name).is_false()
		assert_bool(block_label.position.x >= 0.0 and block_label.get_rect().end.x <= markers.size.x).is_true()
	if test_case.name == "數字靠近":
		assert_float(hit_label.position.y).override_failure_message("靠近時命中數字應移到下一排").is_greater(block_label.position.y)
	elif not has_distinct_block or test_case.name in ["四色血條", "恢復四色血條"]:
		assert_float(hit_label.position.y).override_failure_message("分開後應恢復單排").is_equal(0.0)
	assert_bool(is_equal_approx(markers.custom_minimum_size.y, hit_label.get_rect().end.y)).override_failure_message("%s：標記區不得預留空白排" % test_case.name).is_true()

func assert_attack_preview_bar(case_name: String, expected_segments: Array, max_hp: int) -> void:
	var bar: HBoxContainer = battle.ui.get_node("Root/AttackPreview/Margin/Content/HealthImpactBar")
	var segment_names := ["HitRemaining", "BlockSaved", "Damage", "Missing"]
	var expected_colors := [Color(0.384, 0.824, 0.486, 1), Color(0.94902, 0.721569, 0.294118, 1), Color(0.937255, 0.32549, 0.313725, 1), Color(0.055, 0.067, 0.086, 1)]
	var bar_rect := bar.get_global_rect()
	assert_bool(bar.is_visible_in_tree()).override_failure_message("%s：血條應實際可見" % case_name).is_true()
	assert_float(bar_rect.size.x).override_failure_message("%s：血條應有實際寬度" % case_name).is_greater(0.0)
	assert_float(bar_rect.size.y).override_failure_message("%s：血條應有實際高度" % case_name).is_greater(0.0)
	var next_x := bar_rect.position.x
	for index in segment_names.size():
		var segment: ColorRect = bar.get_node(segment_names[index])
		var message := "%s／%s" % [case_name, segment_names[index]]
		assert_int(segment.get_index()).override_failure_message("%s：節點順序應為綠黃紅深色" % message).is_equal(index)
		assert_bool(segment.color.is_equal_approx(expected_colors[index])).override_failure_message("%s：分段顏色應正確" % message).is_true()
		assert_bool(segment.modulate == Color.WHITE and segment.self_modulate == Color.WHITE).override_failure_message("%s：分段不應被額外染色" % message).is_true()
		assert_bool(segment.is_visible_in_tree()).override_failure_message("%s：零長度分段應隱藏" % message).is_equal(expected_segments[index] > 0)
		if expected_segments[index] == 0:
			continue
		var rect := segment.get_global_rect()
		assert_bool(is_equal_approx(rect.position.x, next_x)).override_failure_message("%s：分段應接續前段，不得有空隙或重疊" % message).is_true()
		assert_bool(is_equal_approx(rect.position.y, bar_rect.position.y) and is_equal_approx(rect.size.y, bar_rect.size.y)).override_failure_message("%s：分段應對齊並填滿血條高度" % message).is_true()
		assert_float(rect.size.x).override_failure_message("%s：可見分段應有實際寬度" % message).is_greater(0.0)
		# 容器以像素分配寬度，比例檢查容許一個像素的取整誤差。
		var expected_width := bar_rect.size.x * float(expected_segments[index]) / float(max_hp)
		assert_bool(absf(rect.size.x - expected_width) <= 1.0).override_failure_message("%s：實際寬度應符合 HP 比例" % message).is_true()
		next_x = rect.end.x
	assert_bool(is_equal_approx(next_x, bar_rect.end.x)).override_failure_message("%s：最後分段應抵達血條右端" % case_name).is_true()

# 驗證四種技能的施放範圍邊界正確，且選擇技能會隱藏移動範圍並清除路徑預覽。
func test_skill_range_preview() -> void:
	var test_data := [
		{"name": "近戰攻擊", "action": "melee_attack", "button": battle.ui.action_buttons.melee_attack, "range": 1},
		{"name": "遠程攻擊", "action": "ranged_attack", "button": battle.ui.action_buttons.ranged_attack, "range": 3},
		{"name": "強力一擊", "action": "power_strike", "button": battle.ui.action_buttons.power_strike, "range": 1},
		{"name": "瞄準射擊", "action": "aimed_shot", "button": battle.ui.action_buttons.aimed_shot, "range": 4},
	]
	for test_case in test_data:
		await prepare_case(battle)
		var actor_cell := actor_cell(battle)
		push_mouse_motion(battle.world, battle.world.cell_center(actor_cell + Vector2i(3, 0)))
		assert_bool(battle.world.first_move_path.is_empty()).override_failure_message("%s：選擇技能前應有移動路徑預覽" % test_case.name).is_false()

		push_control_click(test_case.button)

		var preview_cells := cells_from_values(battle.world.selected_skill_range())
		assert_bool(preview_cells.has(actor_cell + Vector2i(test_case.range, 0))).override_failure_message("%s：射程邊界格應包含在預覽" % test_case.name).is_true()
		assert_bool(preview_cells.has(actor_cell + Vector2i(test_case.range + 1, 0))).override_failure_message("%s：射程外一格不應包含在預覽" % test_case.name).is_false()
		assert_bool(battle.world.first_move_path.is_empty() and battle.world.second_move_path.is_empty()).override_failure_message("%s：選擇技能後移動路徑應消失" % test_case.name).is_true()
		push_mouse_motion(battle.world, battle.world.cell_center(actor_cell + Vector2i(2, 0)))
		assert_bool(battle.world.first_move_path.is_empty() and battle.world.second_move_path.is_empty()).override_failure_message("%s：技能待選時不應重新產生移動路徑" % test_case.name).is_true()

# 驗證移動範圍會區分第一段、第二段與兩段外的格子。
func test_two_stage_movement_range_preview() -> void:
	var test_data := [
		{"name": "第一段邊界", "offset": Vector2i(2, 0), "first": true, "second": false},
		{"name": "第二段範圍", "offset": Vector2i(3, 0), "first": false, "second": true},
		{"name": "兩段範圍外", "offset": Vector2i(5, 0), "first": false, "second": false},
	]
	for test_case in test_data:
		await prepare_case(battle)
		var cell: Vector2i = actor_cell(battle) + Vector2i(test_case.offset)
		var first_cells := cells_from_values(battle.state.reachable)
		var second_cells := cells_from_values(battle.state.second_reachable)
		assert_bool(first_cells.has(cell)).override_failure_message("%s：第一段範圍歸屬應正確" % test_case.name).is_equal(test_case.first)
		assert_bool(second_cells.has(cell)).override_failure_message("%s：第二段範圍歸屬應正確" % test_case.name).is_equal(test_case.second)

# 驗證滑鼠懸停會產生正確分段的移動路徑，且不可達格不會留下路徑。
func test_movement_path_preview() -> void:
	var test_data := [
		{"name": "第一段路徑", "offset": Vector2i(2, 0), "first": [0, 1, 2], "second": []},
		{"name": "第二段路徑", "offset": Vector2i(3, 0), "first": [0, 1, 2], "second": [2, 3]},
		{"name": "不可達路徑", "offset": Vector2i(5, 0), "first": [], "second": []},
	]
	for test_case in test_data:
		await prepare_case(battle)
		var origin := actor_cell(battle)
		push_mouse_motion(battle.world, battle.world.cell_center(origin + test_case.offset))
		assert_array(path_x_offsets(battle.world.first_move_path, origin)).override_failure_message("%s：第一段路徑應正確" % test_case.name).is_equal(test_case.first)
		assert_array(path_x_offsets(battle.world.second_move_path, origin)).override_failure_message("%s：第二段路徑應正確" % test_case.name).is_equal(test_case.second)

# 驗證移動路徑碰到地刺時會在觸發格截斷，並啟用危險路徑警示狀態。
func test_spikes_interrupt_movement_preview() -> void:
	await prepare_case(battle)
	var spikes := Vector2i(1, 2)
	var destination := Vector2i(1, 1)
	var preview = JSON.parse_string(battle.core.preview_move(battle.state.turn.actor, destination.x, destination.y))
	push_mouse_motion(battle.world, battle.world.cell_center(destination))

	assert_bool(preview.interrupted).override_failure_message("核心應標記路徑受到地刺中斷").is_true()
	assert_int(preview.first.size()).override_failure_message("預覽路徑應截斷於第一個地刺格").is_equal(2)
	assert_vector(Vector2i(preview.first[-1].x, preview.first[-1].y)).is_equal(spikes)
	assert_bool(battle.world.move_preview_interrupted).override_failure_message("戰鬥畫面應啟用紅色危險路徑狀態").is_true()
	assert_int(battle.world.first_move_path.size()).is_equal(2)
	assert_vector(Vector2i(battle.world.first_move_path[-1].x, battle.world.first_move_path[-1].y)).override_failure_message("危險路徑落點應為地刺格").is_equal(spikes)

# 驗證移動模式懸停可達地格時會在游標旁顯示整條路徑的總消耗，離開移動模式後則隱藏。
func test_hovered_tile_movement_total_cost() -> void:
	var test_data := [
		{"name": "第一段一般地格", "offset": Vector2i(2, 0), "expected": 2},
		{"name": "第二段高消耗地格", "offset": Vector2i(2, 1), "expected": 4},
	]
	for test_case in test_data:
		await prepare_case(battle)
		var destination: Vector2i = actor_cell(battle) + test_case.offset
		var preview = JSON.parse_string(battle.core.preview_move(battle.state.turn.actor, destination.x, destination.y))

		push_mouse_motion(battle.world, battle.world.cell_center(destination))

		assert_int(int(preview.get("total_cost"))).override_failure_message("%s：核心預覽應提供移動總消耗" % test_case.name).is_equal(test_case.expected)
		var popup: Control = battle.ui.get_node_or_null("Root/MoveCostPopup")
		assert_that(popup).override_failure_message("%s：應建立游標旁的移動消耗浮動面板" % test_case.name).is_not_null()
		if popup != null:
			assert_bool(popup.visible).override_failure_message("%s：hover tile 應顯示移動總消耗" % test_case.name).is_true()
			assert_str(popup.get_node("Label").text).override_failure_message("%s：浮動文字應顯示整條路徑的總消耗" % test_case.name).is_equal("移動消耗 %d" % test_case.expected)

	await prepare_case(battle)
	var destination := actor_cell(battle) + Vector2i(2, 0)
	push_mouse_motion(battle.world, battle.world.cell_center(destination))
	push_control_click(battle.ui.action_buttons.melee_attack)
	var skill_mode_popup: Control = battle.ui.get_node_or_null("Root/MoveCostPopup")
	assert_that(skill_mode_popup).override_failure_message("應建立游標旁的移動消耗浮動面板").is_not_null()
	if skill_mode_popup != null:
		assert_bool(skill_mode_popup.visible).override_failure_message("技能模式不應顯示移動總消耗").is_false()

# 驗證游標靠近邊界時，游標浮動面板會共用象限選擇並保持在 viewport 內。
func test_pointer_popups_use_available_quadrant() -> void:
	await prepare_case(battle)
	push_control_click(battle.ui.action_buttons.aimed_shot)
	push_mouse_motion(battle.world, battle.world.cell_center(Vector2i(3, 1)))
	var preview: Dictionary = battle.world.attack_preview
	var move_cost_popup: Control = battle.ui.get_node_or_null("Root/MoveCostPopup")
	var attack_popup: Control = battle.ui.get_node_or_null("Root/AttackPreview")
	assert_that(move_cost_popup).override_failure_message("應建立游標旁的移動消耗浮動面板").is_not_null()
	assert_that(attack_popup).override_failure_message("應建立游標旁的瞄準預覽浮動面板").is_not_null()
	if move_cost_popup == null or attack_popup == null:
		return
	var viewport_rect: Rect2 = battle.get_viewport().get_visible_rect()
	var test_data := [
		{"name": "滑鼠位於中央", "pointer": viewport_rect.get_center(), "horizontal": 1, "vertical": 1},
		{"name": "滑鼠位於左上", "pointer": viewport_rect.position + Vector2(1.0, 1.0), "horizontal": 1, "vertical": 1},
		{"name": "滑鼠位於左下", "pointer": Vector2(viewport_rect.position.x + 1.0, viewport_rect.end.y - 1.0), "horizontal": 1, "vertical": - 1},
		{"name": "滑鼠位於右上", "pointer": Vector2(viewport_rect.end.x - 1.0, viewport_rect.position.y + 1.0), "horizontal": - 1, "vertical": 1},
		{"name": "滑鼠位於右下", "pointer": viewport_rect.end - Vector2(1.0, 1.0), "horizontal": - 1, "vertical": - 1},
	]
	for test_case in test_data:
		var pointer_position: Vector2 = test_case.pointer
		battle.ui.present_move_cost(4, pointer_position)
		battle.ui.present_attack_preview(preview, pointer_position)

		for popup in [move_cost_popup, attack_popup]:
			assert_bool(popup.visible).override_failure_message("%s：邊界附近仍應顯示游標浮動面板" % test_case.name).is_true()
			assert_bool((popup.position.x - pointer_position.x) * test_case.horizontal > 0.0).override_failure_message("%s：浮動面板的左右位置應正確" % test_case.name).is_true()
			assert_bool((popup.position.y - pointer_position.y) * test_case.vertical > 0.0).override_failure_message("%s：浮動面板的上下位置應正確" % test_case.name).is_true()
			assert_bool(viewport_rect.encloses(popup.get_rect())).override_failure_message("%s：浮動面板應完整位於 viewport 可視範圍內" % test_case.name).is_true()

# 驗證單次點擊可抵達第一段或第二段目的地，並正確扣除兩段移動力。
func test_single_click_movement() -> void:
	var test_data := [
		{"name": "抵達第一段", "offset": Vector2i(2, 0), "remaining": 0.0, "phase": "aftermove"},
		{"name": "抵達第二段", "offset": Vector2i(3, 0), "remaining": 1.0, "phase": "moving"},
	]
	for test_case in test_data:
		await prepare_case(battle)
		var destination: Vector2i = actor_cell(battle) + Vector2i(test_case.offset)

		push_left_click(battle.world, battle.world.cell_center(destination))

		assert_vector(actor_cell(battle)).override_failure_message("%s：角色應以單次點擊抵達目的地" % test_case.name).is_equal(destination)
		assert_float(battle.state.turn.move_remaining).override_failure_message("%s：剩餘移動力應正確" % test_case.name).is_equal(test_case.remaining)
		assert_str(battle.state.turn.phase).override_failure_message("%s：移動階段應正確" % test_case.name).is_equal(test_case.phase)

# 驗證未移動或只完成一段移動時可施放技能，完成兩段移動後則不可施放。
func test_skill_availability_after_movement() -> void:
	var test_data := [
		{"name": "未移動", "destinations": [], "can_skill": true},
		{"name": "完成一段移動", "destinations": [Vector2i(3, 3)], "can_skill": true},
		{"name": "開始第二段移動", "destinations": [Vector2i(3, 3), Vector2i(4, 3)], "can_skill": false},
		{"name": "完成兩段移動", "destinations": [Vector2i(3, 3), Vector2i(5, 3)], "can_skill": false},
		{"name": "單次開始第二段移動", "destinations": [Vector2i(4, 3)], "can_skill": false},
		{"name": "單次走完兩段移動", "destinations": [Vector2i(5, 3)], "can_skill": false},
	]
	for test_case in test_data:
		await prepare_case(battle)
		for destination in test_case.destinations:
			assert_bool(battle.send({"type": "move", "actor": "aria", "x": destination.x, "y": destination.y})).override_failure_message("%s：測試移動應成功" % test_case.name).is_true()
		await wait_for_combat_events(battle)

		assert_bool(battle.state.turn.can_skill).override_failure_message("%s：技能可用狀態應正確" % test_case.name).is_equal(test_case.can_skill)
		assert_bool(battle.ui.action_buttons.aimed_shot.disabled).override_failure_message("%s：技能按鈕狀態應正確" % test_case.name).is_equal(not test_case.can_skill)
		assert_bool(battle.send({"type": "skill", "actor": "aria", "target": "ogre", "x": 3, "y": 1, "skill": "aimed_shot"})).override_failure_message("%s：技能施放結果應符合移動段數" % test_case.name).is_equal(test_case.can_skill)

# 載入專用 TOML，必要時以真實攻擊建立缺血情境，再取得核心的完整預覽。
func query_attack_preview(target_id: String, skill_id: String, preparation_skill := "") -> Dictionary:
	var preview_core := TacticalGame.new()
	var definition := FileAccess.get_file_as_string(ATTACK_VARIATIONS_DEFINITION)
	var loaded: Dictionary = JSON.parse_string(preview_core.load_definition(definition))
	assert_bool(loaded.has("error")).override_failure_message("預覽變化專用 TOML 應成功載入").is_false()
	if loaded.has("error"):
		return {}
	var started: Dictionary = JSON.parse_string(preview_core.dispatch(JSON.stringify({"type": "start"})))
	assert_bool(started.has("error")).override_failure_message("預覽變化測試戰鬥應成功開始").is_false()
	var target: Dictionary = {}
	for unit in loaded.units:
		if unit.id == target_id:
			target = unit
			break
	assert_bool(target.is_empty()).override_failure_message("專用 TOML 應包含預覽目標 %s" % target_id).is_false()
	if target.is_empty() or started.has("error"):
		return {}
	if not preparation_skill.is_empty():
		# 固定亂數種子，讓建立缺血情境的攻擊穩定普通命中。
		preview_core.set_random_seed(1)
		var damaged: Dictionary = JSON.parse_string(preview_core.dispatch(JSON.stringify({"type": "skill", "actor": "aria", "target": target_id, "x": int(target.x), "y": int(target.y), "skill": preparation_skill})))
		assert_bool(damaged.has("error")).override_failure_message("建立缺血情境的真實攻擊應成功").is_false()
		if damaged.has("error"):
			return {}
	var preview: Dictionary = JSON.parse_string(preview_core.preview_skill("aria", target_id, int(target.x), int(target.y), skill_id))
	assert_bool(preview.has("error")).override_failure_message("核心應成功產生 %s 的 %s 預覽" % [target_id, skill_id]).is_false()
	return preview

func prepare_case(battle) -> void:
	await wait_for_combat_events(battle)
	for child in battle.world.units_layer.get_children():
		child.free()
	battle.world.unit_nodes.clear()
	battle.pending_action = ""
	battle.world.hovered = Vector2i(-1, -1)
	battle.world.clear_move_preview()

	var definition := FileAccess.get_file_as_string(TEST_DEFINITION)
	var loaded = JSON.parse_string(battle.core.load_definition(definition))
	assert_bool(loaded.has("error")).override_failure_message("專用 TOML 應成功載入").is_false()
	if loaded.has("error"): return
	battle.state = loaded
	battle.world.setup_map(loaded)
	assert_bool(battle.send({"type": "start"})).override_failure_message("專用測試戰鬥應成功開始").is_true()
	await wait_for_combat_events(battle)

func wait_for_combat_events(target_battle) -> void:
	while target_battle.world.is_presenting_combat_events():
		await runner.simulate_frames(1)

func actor_cell(battle) -> Vector2i:
	for unit in battle.state.units:
		if unit.id == battle.state.turn.actor:
			return Vector2i(unit.x, unit.y)
	return Vector2i(-1, -1)

func cells_from_values(values: Array) -> Array[Vector2i]:
	var cells: Array[Vector2i] = []
	for value in values:
		cells.append(Vector2i(value.x, value.y))
	return cells

func path_x_offsets(path: Array, origin: Vector2i) -> Array[int]:
	var offsets: Array[int] = []
	for value in path:
		var cell := Vector2i(value.x, value.y)
		assert_int(cell.y).override_failure_message("直線測試路徑不應偏離列").is_equal(origin.y)
		offsets.append(cell.x - origin.x)
	return offsets

func push_mouse_motion(battle, local_position: Vector2) -> void:
	var screen_position: Vector2 = battle.get_viewport().get_screen_transform() * battle.get_global_transform_with_canvas() * local_position
	runner.simulate_mouse_move(screen_position)

func push_left_click(battle, local_position: Vector2) -> void:
	var screen_position: Vector2 = battle.get_viewport().get_screen_transform() * battle.get_global_transform_with_canvas() * local_position
	runner.set_mouse_position(screen_position)
	runner.simulate_mouse_button_pressed(MOUSE_BUTTON_LEFT)

func push_control_click(control: Control) -> void:
	control.pressed.emit()
