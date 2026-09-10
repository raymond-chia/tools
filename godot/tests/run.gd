extends SceneTree

const TEST_ROOT := "res://tests"

func _init() -> void:
	_run.call_deferred()

func _run() -> void:
	var test_paths := find_test_paths(TEST_ROOT)
	if test_paths.is_empty():
		push_error("找不到 Godot 測試檔案")
		quit(1)
		return

	var all_failures: Array[String] = []
	for test_path in test_paths:
		var test_script = load(test_path)
		var test = test_script.new()
		var test_failures: Array[String] = await test.run(self)
		for failure in test_failures:
			all_failures.append("%s：%s" % [test_path, failure])
		test = null
		await process_frame

	if all_failures.is_empty():
		print("PASS: %s 個 Godot 測試檔案" % test_paths.size())
		quit(0)
	else:
		for failure in all_failures:
			push_error(failure)
		quit(1)

func find_test_paths(directory_path: String) -> Array[String]:
	var paths: Array[String] = []
	for directory_name in DirAccess.get_directories_at(directory_path):
		paths.append_array(find_test_paths(directory_path.path_join(directory_name)))
	for file_name in DirAccess.get_files_at(directory_path):
		if file_name.ends_with("_test.gd"):
			paths.append(directory_path.path_join(file_name))
	paths.sort()
	return paths
