extends RefCounted

const BATTLE_SCENE := preload("res://features/battle/battle.tscn")
const RANDOM_SEED := 1
# 演出案例在準備完成後重設種子，避免先攻擲骰影響首次攻擊。
const PRESENTATION_RANDOM_SEED := 2

# 使用專用文件與固定種子建立尚未進入場景樹的戰鬥場景，涵蓋初始先攻擲骰。
static func create_battle(definitions_path: String, map_path: String, seed := RANDOM_SEED) -> Node:
	var battle := BATTLE_SCENE.instantiate()
	battle.configure(FileAccess.get_file_as_string(definitions_path), FileAccess.get_file_as_string(map_path), false, seed)
	return battle

# 直接建立核心或準備特定操作時，使用與場景相同的預設種子。
static func reset_random_seed(core, seed := RANDOM_SEED) -> void:
	core.set_random_seed(seed)

static func wait_until_idle(battle, runner: GdUnitSceneRunner) -> void:
	while battle.state.turn.can_continue or battle.world.is_presenting_combat_events() or has_running_unit_tweens(battle.world):
		await runner.simulate_frames(1)

static func has_running_unit_tweens(world) -> bool:
	for tweens in [world.movement_tweens, world.hit_tweens, world.attack_tweens]:
		for tween: Tween in tweens.values():
			if tween.is_valid() and tween.is_running():
				return true
	return false

# 等待舊場景完成後重建真實場景，讓初始化自行建立完整狀態。
static func replace_battle(suite: GdUnitTestSuite, runner: GdUnitSceneRunner, definitions_path: String, map_path: String) -> GdUnitSceneRunner:
	runner.set_time_factor(9.0)
	await wait_until_idle(runner.scene(), runner)
	runner.scene().free()
	var next_runner := suite.scene_runner(suite.auto_free(create_battle(definitions_path, map_path)))
	next_runner.set_time_factor(9.0)
	await next_runner.simulate_frames(1)
	await wait_until_idle(next_runner.scene(), next_runner)
	return next_runner
