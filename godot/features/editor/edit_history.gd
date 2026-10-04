extends RefCounted

class DocumentState extends RefCounted:
	var state: Dictionary = {}

# 不持有場景節點；試玩返回後仍可使用相同的 UndoRedo 與文件快照。
var undo_redo := UndoRedo.new()
# UndoRedo 只持有文件狀態，不反向持有歷史管理物件，避免參照循環。
var document_state := DocumentState.new()
var state: Dictionary:
	get:
		return document_state.state
var saved_state: Dictionary = {}

func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE: undo_redo.free()

func reset(document: Dictionary) -> void:
	undo_redo.clear_history()
	document_state.state = document.duplicate(true)
	saved_state = document.duplicate(true)

func record(previous: Dictionary, next: Dictionary, label := "編輯資料", merge_mode := UndoRedo.MERGE_DISABLE) -> void:
	if previous == next: return
	undo_redo.create_action(label, merge_mode)
	undo_redo.add_undo_property(document_state, "state", previous.duplicate(true))
	undo_redo.add_do_property(document_state, "state", next.duplicate(true))
	undo_redo.commit_action()
