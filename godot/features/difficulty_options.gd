class_name DifficultyOptions
extends RefCounted

# 共用語意 ID 與顯示文字；出現門檻比較仍由核心負責。
const IDS := ["easy", "normal", "hard"]
const LABEL_KEYS := ["DIFFICULTY_EASY", "DIFFICULTY_NORMAL", "DIFFICULTY_HARD"]

static func populate(list: OptionButton, minimum := false) -> void:
	list.clear()
	list.add_item(TranslationServer.translate("DIFFICULTY_SELECT_MINIMUM" if minimum else "DIFFICULTY_SELECT"))
	list.set_item_disabled(0, true)
	for index in IDS.size():
		var label := TranslationServer.translate(LABEL_KEYS[index])
		list.add_item(TranslationServer.translate("DIFFICULTY_MINIMUM") % label if minimum else label)
		list.set_item_metadata(index + 1, IDS[index])
	list.select(0)
