extends "res://tests/features/battle/battle_test_suite.gd"

const DATA := "res://tests/features/battle/data/difficulty/"
const EDITOR_SCENE := preload("res://features/editor/editor.tscn")

# 驗證單位外觀選單分段列出單位與共用圖片，選擇共用貨車圖後戰鬥單位使用 shared 內的圖片。
func test_unit_selects_shared_art() -> void:
	var editor := EDITOR_SCENE.instantiate()
	editor.data_dir = DATA
	editor.definitions_path = DATA + "definitions.toml"
	editor.map_dir = DATA + "maps/"
	var runner := scene_runner(auto_free(editor))
	await runner.simulate_frames(1)
	var unit_page = editor.definition_pages[0]
	unit_page.select_id("aria")
	var selector := visual_selector(unit_page)
	var sections: Array = []
	var wagon_index := -1
	var wagon_section := ""
	for index in selector.item_count:
		if selector.is_item_separator(index):
			sections.append(selector.get_item_text(index))
		elif selector.get_item_metadata(index) == "wagon.svg":
			wagon_index = index
			wagon_section = sections.back()
	assert_array(sections).is_equal(["單位", "共用"])
	assert_str(wagon_section).is_equal("共用")
	selector.select(wagon_index)
	selector.item_selected.emit(wagon_index)
	var documents: Dictionary = editor.serialize_documents()
	var battle := BATTLE_SCENE.instantiate()
	battle.configure(documents.definitions, documents.map, "easy", false, RANDOM_SEED)
	var battle_runner := battle_scene_runner(auto_free(battle))
	await battle_runner.simulate_frames(1)
	var body: Sprite2D = battle.world.unit_nodes[1].get_node("Visual/Body")
	assert_str(body.texture.resource_path).is_equal("res://assets/shared/wagon.svg")

# 外觀選單緊接在「外觀」標籤之後。
func visual_selector(page: Node) -> OptionButton:
	var children: Array = page.fields.get_children()
	for index in children.size():
		if children[index] is Label and children[index].text == "外觀":
			return children[index + 1]
	return null
