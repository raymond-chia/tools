extends Node2D

var progress := 0.0

func _process(delta: float) -> void:
	progress = fposmod(progress + delta / 2.0, 1.0)
	queue_redraw()

func _draw() -> void:
	BattleVisuals.draw_exit_marker(self, progress)
