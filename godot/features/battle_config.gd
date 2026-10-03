class_name BattleConfig
extends RefCounted

const UNIT_ART_DIR := "res://assets/units/"
const PENDING_UNIT_ART := "pending.svg"

static func unit_art_path(visual: String) -> String:
	return UNIT_ART_DIR + (PENDING_UNIT_ART if visual.is_empty() else visual)

static func unit_name_key(unit_type: String) -> String:
	return "UNIT_NAME_%s" % unit_type.to_upper()

static func team_name_key(team_name: String) -> String:
	return "TEAM_NAME_%s" % team_name.to_upper()

static func skill_name_key(skill_id: String) -> String:
	return "SKILL_NAME_%s" % skill_id.to_upper()

static func terrain_name_key(terrain_id: String) -> String:
	return "TERRAIN_NAME_%s" % terrain_id.to_upper()

static func terrain_description_key(terrain_id: String) -> String:
	return "TERRAIN_DESCRIPTION_%s" % terrain_id.to_upper()

const PREVIEW_IGNORED_ERROR_IDS := [
	"cannot_move",
	"cannot_use_skill",
	"unreachable_destination",
	"target_too_close",
	"target_too_far",
	"heal_allies_only",
	"cannot_attack_ally",
]

const UI_Z_BASE := 0
const UI_Z_INSPECTION := 10
const UI_Z_TOOLTIP := 20
const MENU_Z_INDEX := 30

# 目前行動單位的移動範圍填色與外框。
const FIRST_MOVE_COLOR := Color(0.5, 0.7, 1.0)
const SECOND_MOVE_COLOR := Color(0.35, 0.5, 1.0)
const MOVE_FILL_ALPHA := 0.3
const MOVE_EDGE_ALPHA := 1.0
const MOVE_RANGE_EDGE_WIDTH := 2.0
# 選取查看單位的兩段顏色、外框與線條透明度，可獨立於行動單位調整。
const SELECTED_MOVE_FIRST_COLOR := Color(0.5, 0.7, 1.0)
const SELECTED_MOVE_SECOND_COLOR := Color(0.35, 0.5, 1.0)
const SELECTED_MOVE_EDGE_ALPHA := 0.7
const SELECTED_MOVE_EDGE_WIDTH := 2.0
const SELECTED_MOVE_LINE_ALPHA := 1.0
# 角度以畫面右方為 0 度，負值朝右上；數量為每格斜線數，間距與粗細為繪圖像素。
const SELECTED_MOVE_LINE_ANGLE_DEGREES := -90.0
const SELECTED_MOVE_LINE_COUNT := 3
const SELECTED_MOVE_LINE_SPACING := 10.0
const SELECTED_MOVE_LINE_WIDTH := 1.0

const MAP_DISPLAY_SCALE := 1.5
const ATTACK_PREVIEW_RING_SCALE := 1.20
const DELAY_SLOT_COLOR := Color(0.35, 0.45, 0.58, 0.65)
const DELAY_SLOT_HIGHLIGHT_COLOR := Color(1.0, 0.8, 0.25, 1.0)
const UNIT_MOVE_STEP_DURATION := 0.10
const CAMERA_FOCUS_DURATION := 0.28
const CAMERA_MOVE_SPEED := 700.0
const CAMERA_BOUNDS_MARGIN := 320.0
const ATTACK_LUNGE_DURATION := 0.09
const ATTACK_LUNGE_DISTANCE := 14.0
const HIT_FLASH_DURATION := 0.08
const HIT_SHAKE_DISTANCE := 6.0
const DEATH_FADE_DURATION := 0.28
const COMBAT_RESULT_HOLD := 0.45
const COMBAT_EVENT_PAUSE := 0.18
const FLOATING_TEXT_DURATION := 0.8
const FLOATING_TEXT_RISE := 42.0
const DAMAGE_TEXT_COLOR := Color(1.0, 0.32, 0.28)
const BLOCK_TEXT_COLOR := Color(1.0, 0.76, 0.28)
const DODGE_TEXT_COLOR := Color(0.45, 0.85, 1.0)
const HEALING_TEXT_COLOR := Color(0.35, 1.0, 0.58)
