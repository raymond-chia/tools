class_name DataSource
extends RefCounted

const PRIVATE_DIR := "res://data-private/"
const EXAMPLE_DIR := "res://data/"

# 整套資料共用同一個來源；私人目錄存在時，不混用公開範例。
static func directory() -> String:
	return PRIVATE_DIR if DirAccess.dir_exists_absolute(PRIVATE_DIR) else EXAMPLE_DIR
