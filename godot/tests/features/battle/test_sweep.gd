extends "res://tests/features/battle/battle_test_suite.gd"

const TEST_DEFINITIONS := "res://tests/features/battle/data/sweep_definitions.toml"
const TEST_MAP := "res://tests/features/battle/data/sweep_map.toml"

var battle
var runner: GdUnitSceneRunner

# 使用真實核心與戰鬥場景，三個目標包含一名友軍。
func setup_test() -> void:
	runner = battle_scene_runner(auto_free(create_battle(TEST_DEFINITIONS, TEST_MAP)))
	await runner.simulate_frames(1)
	battle = runner.scene()
	await wait_until_idle(battle, runner)

# 驗證敵我目標外環會顯示，移到空範圍後會隱藏外環並清除預覽。
func test_sweep_preview_shows_and_clears_target_rings() -> void:
	battle.ui.action_selected.emit("sweep")
	battle.world.hovered = Vector2i(2, 1)
	battle.world.update_attack_preview()
	for id in [2, 3, 4]:
		assert_bool(battle.world.unit_nodes[id].get_node("Visual/AttackPreviewRing").visible).is_true()
	battle.world.hovered = Vector2i(2, 3)
	battle.world.update_attack_preview()
	assert_array(battle.world.attack_preview_targets).is_empty()
	assert_dict(battle.world.attack_preview).is_empty()
	for id in [2, 3, 4]:
		assert_bool(battle.world.unit_nodes[id].get_node("Visual/AttackPreviewRing").visible).is_false()

# 驗證橫掃說明在中英文都明確告知前方三格與友軍傷害。
func test_sweep_description_explains_friendly_fire() -> void:
	for locale in ["zh_TW", "en"]:
		TranslationServer.set_locale(locale)
		var skill: Dictionary = battle.state.skill_ranges[0]
		assert_str(battle.ui.localized_skill_details(skill)).contains(tr("SKILL_EFFECT_SWEEP"))
		assert_str(tr("SKILL_NAME_SWEEP")).is_not_equal("SKILL_NAME_SWEEP")
