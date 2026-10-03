extends Control

var visual := ""
var layer := "ground"

# 重用戰鬥畫面的素材與繪圖，避免縮圖與實際地形有兩套外觀定義。
func _draw() -> void:
	var center := size * 0.5
	var atlas_cell: Vector2i = BattleVisuals.GROUND_TILES.get(visual, BattleVisuals.GROUND_TILES.plain)
	if layer == "ground":
		draw_texture_rect_region(BattleVisuals.GROUND_ART, Rect2(center - Vector2(32, 16), Vector2(64, 32)), Rect2(Vector2(atlas_cell * BattleVisuals.TILE_SIZE), Vector2(BattleVisuals.TILE_SIZE)))
	elif visual.is_empty():
		draw_line(center + Vector2(-12, -12), center + Vector2(12, 12), Color("ffe17a"), 3.0, true)
		draw_line(center + Vector2(-12, 12), center + Vector2(12, -12), Color("ffe17a"), 3.0, true)
	else:
		draw_colored_polygon(BattleVisuals.diamond(center), Color("405149"))
		BattleVisuals.draw_terrain_effect(self, visual, center)
