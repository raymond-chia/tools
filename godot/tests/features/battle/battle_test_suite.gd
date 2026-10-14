extends GdUnitTestSuite

const BATTLE_SCENE := preload("res://features/battle/battle.tscn")
const RANDOM_SEED := 1
# 演出案例在準備完成後重設種子，避免先攻擲骰影響首次攻擊。
const PRESENTATION_RANDOM_SEED := 2

var _original_locale: String
var _original_settings: Variant

# 每個案例備份並隔離統一設定檔，再以繁體中文執行測試初始化。
func before_test() -> void:
	_original_locale = TranslationServer.get_locale()
	TranslationServer.set_locale("zh_TW")
	_original_settings = FileAccess.get_file_as_bytes(GameSettings.PATH) if FileAccess.file_exists(GameSettings.PATH) else null
	if FileAccess.file_exists(GameSettings.PATH):
		assert_int(DirAccess.remove_absolute(ProjectSettings.globalize_path(GameSettings.PATH))).is_equal(OK)
	await setup_test()

# 完成案例專用清理後，還原統一設定檔的原始位元組與語系。
func after_test() -> void:
	await cleanup_test()
	if _original_settings != null:
		var file := FileAccess.open(GameSettings.PATH, FileAccess.WRITE)
		assert_object(file).is_not_null()
		if file != null:
			file.store_buffer(_original_settings)
			file.close()
	elif FileAccess.file_exists(GameSettings.PATH):
		assert_int(DirAccess.remove_absolute(ProjectSettings.globalize_path(GameSettings.PATH))).is_equal(OK)
	_original_settings = null
	TranslationServer.set_locale(_original_locale)

# 子類別提供案例專用初始化，語系生命週期由共用基底負責。
func setup_test() -> void:
	pass

# 子類別只處理案例專用資源的清理。
func cleanup_test() -> void:
	pass

# 真實場景完成初始化後恢復測試語系，同步刷新兩列設定選單與動態文字。
func battle_scene_runner(scene: Node) -> GdUnitSceneRunner:
	var locale := TranslationServer.get_locale()
	var next_runner := scene_runner(scene)
	TranslationServer.set_locale(locale)
	next_runner.scene().ui.update_language_menu()
	next_runner.scene().ui.update_difficulty_menu()
	next_runner.scene().ui.language_changed.emit()
	return next_runner

# 使用專用文件與固定種子建立尚未進入場景樹的戰鬥場景，涵蓋初始先攻擲骰。
func create_battle(definitions_path: String, map_path: String, seed := RANDOM_SEED) -> Node:
	var battle := BATTLE_SCENE.instantiate()
	battle.configure(FileAccess.get_file_as_string(definitions_path), FileAccess.get_file_as_string(map_path), "easy", false, seed)
	return battle

# 直接建立核心或準備特定操作時，使用與場景相同的預設種子。
func reset_random_seed(core, seed := RANDOM_SEED) -> void:
	core.set_random_seed(seed)

func wait_until_idle(battle, runner: GdUnitSceneRunner) -> void:
	while battle.state.turn.can_continue or battle.world.is_presenting_combat_events() or has_running_unit_tweens(battle.world):
		await runner.simulate_frames(1)

func has_running_unit_tweens(world) -> bool:
	for tweens in [world.movement_tweens, world.hit_tweens, world.attack_tweens]:
		for tween: Tween in tweens.values():
			if tween.is_valid() and tween.is_running():
				return true
	return false

# 等待舊場景完成後重建真實場景，保留測試當下語系並讓初始化建立完整狀態。
func replace_battle(runner: GdUnitSceneRunner, definitions_path: String, map_path: String) -> GdUnitSceneRunner:
	runner.set_time_factor(9.0)
	await wait_until_idle(runner.scene(), runner)
	runner.scene().free()
	var next_runner := battle_scene_runner(auto_free(create_battle(definitions_path, map_path)))
	next_runner.set_time_factor(9.0)
	await next_runner.simulate_frames(1)
	await wait_until_idle(next_runner.scene(), next_runner)
	return next_runner
