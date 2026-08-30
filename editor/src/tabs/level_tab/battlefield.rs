//! 關卡編輯器的戰場網格渲染

use crate::constants::*;
use board::domain::alias::Coord;
use board::ecs_types::components::Position;
use board::ecs_types::resources::Board;

// ==================== 資料型別 ====================

/// 棋盤可見範圍（格子索引，包含兩端）
#[derive(Debug, Clone, Copy)]
pub struct VisibleGridRange {
    pub min: Position,
    pub max: Position,
}

/// 單一格子的高亮資訊
#[derive(Debug, Clone, Copy, Default)]
pub struct CellHighlight {
    pub border: Option<egui::Color32>,
    pub bg: Option<egui::Color32>,
}

// ==================== 座標轉換 ====================

/// 計算棋盤預覽的總尺寸
pub fn calculate_grid_dimensions(board: Board) -> egui::Vec2 {
    let cell_stride = BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING;
    let width = board.width as f32 * cell_stride - BATTLEFIELD_GRID_SPACING;
    let height = board.height as f32 * cell_stride - BATTLEFIELD_GRID_SPACING;
    egui::vec2(width, height)
}

/// 計算可見範圍內的格子索引（用於視口裁剪）
pub fn calculate_visible_range(
    scroll_offset: egui::Vec2,
    viewport_size: egui::Vec2,
    board: Board,
) -> VisibleGridRange {
    let cell_stride = BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING;

    let x_min = (scroll_offset.x / cell_stride).floor().max(0.0) as Coord;
    let y_min = (scroll_offset.y / cell_stride).floor().max(0.0) as Coord;
    let x_max = ((scroll_offset.x + viewport_size.x) / cell_stride)
        .ceil()
        .min(board.width as f32) as Coord;
    let y_max = ((scroll_offset.y + viewport_size.y) / cell_stride)
        .ceil()
        .min(board.height as f32) as Coord;

    VisibleGridRange {
        min: Position { x: x_min, y: y_min },
        max: Position { x: x_max, y: y_max },
    }
}

/// 將螢幕座標轉換為棋盤座標
pub fn screen_to_board_pos(
    screen_pos: egui::Pos2,
    rect: egui::Rect,
    board: Board,
) -> Option<Position> {
    let cell_stride = BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING;
    let relative = screen_pos - rect.min;

    if relative.x < 0.0 || relative.y < 0.0 {
        return None; // 點擊在棋盤外
    }
    let pos = relative / cell_stride;
    let x = pos.x as Coord;
    let y = pos.y as Coord;
    let pos = Position { x, y };
    board::logic::board::is_valid_position(board, pos).then_some(pos)
}

pub fn compute_hover_pos(
    response: &egui::Response,
    rect: egui::Rect,
    board: Board,
) -> Option<Position> {
    response
        .hover_pos()
        // and_then 拆 option，避免 nested option
        .and_then(|p| screen_to_board_pos(p, rect, board))
}

// ==================== 渲染層 ====================

/// 繪製編輯模式棋盤格子，支持拖曳預覽與背景高亮
pub fn render_grid(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    board: Board,
    scroll_offset: egui::Vec2,
    get_cell_info: impl Fn(Position) -> (String, egui::Color32, egui::Color32),
    get_cell_highlight: impl Fn(Position) -> CellHighlight,
) {
    let cell_stride = BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING;

    // 計算可見範圍（視口裁剪優化）
    let viewport_size = ui.clip_rect().size();
    let visible_range = calculate_visible_range(scroll_offset, viewport_size, board);

    let painter = ui.painter();
    for y in visible_range.min.y..visible_range.max.y {
        for x in visible_range.min.x..visible_range.max.x {
            let pos = Position { x, y };

            // 計算每個格子的左上角座標
            let cell_x = rect.min.x + x as f32 * cell_stride;
            let cell_y = rect.min.y + y as f32 * cell_stride;
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(cell_x, cell_y),
                egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE),
            );

            // 決定格子內容與背景顏色
            let (cell_text, font_color, bg_color) = get_cell_info(pos);
            let highlight = get_cell_highlight(pos);

            // 背景高亮覆蓋層
            let bg_color = highlight.bg.unwrap_or(bg_color);
            // 格子背景
            painter.rect_filled(cell_rect, 0.0, bg_color);

            // 文字
            painter.text(
                cell_rect.center(),
                egui::Align2::CENTER_CENTER,
                cell_text,
                egui::FontId::proportional(BATTLEFIELD_TEXT_SIZE),
                font_color,
            );

            if let Some(border_color) = highlight.border {
                painter.rect_stroke(
                    cell_rect,
                    0.0,
                    egui::Stroke::new(STROKE_WIDTH, border_color),
                    egui::epaint::StrokeKind::Inside,
                );
            }
        }
    }
}

/// 渲染懸停提示
pub fn render_hover_tooltip(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    hovered_pos: Position,
    get_tooltip_info: impl Fn(Position) -> String,
) {
    let cell_stride = BATTLEFIELD_CELL_SIZE + BATTLEFIELD_GRID_SPACING;
    let hover_text = get_tooltip_info(hovered_pos);
    let hovered_pos = egui::vec2(hovered_pos.x as f32, hovered_pos.y as f32);
    let hovered_pos = rect.min
        + hovered_pos * cell_stride
        + egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE) / 2.0;

    // 計算文本寬度
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(hover_text.clone(), font_id, egui::Color32::BLACK);
    let text_size = galley.size();
    let viewport_rect = ui.ctx().viewport_rect();
    let tooltip_x = if hovered_pos.x + text_size.x + SPACING_MEDIUM > viewport_rect.right() {
        // 右邊太窄，顯示在左邊
        hovered_pos.x - text_size.x - SPACING_MEDIUM
    } else {
        // 右邊有足夠空間，顯示在右邊
        hovered_pos.x + SPACING_MEDIUM
    };

    let tooltip_pos = egui::pos2(tooltip_x, hovered_pos.y);
    let tooltip_rect = egui::Rect::from_min_size(tooltip_pos, text_size);
    let tooltip_layer = egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("battlefield_hover_tooltip_layer"),
    );
    let tooltip_painter = ui.ctx().layer_painter(tooltip_layer);
    tooltip_painter.rect_filled(tooltip_rect, 0.0, egui::Color32::GRAY);
    tooltip_painter.galley(tooltip_pos, galley, egui::Color32::BLACK);
}

/// 渲染戰場圖例
pub fn render_battlefield_legend(ui: &mut egui::Ui) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label("圖例：");

            // 玩家放置點
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE),
                egui::Sense::empty(),
            );
            ui.painter()
                .rect_filled(rect, 0.0, BATTLEFIELD_COLOR_DEPLOYMENT);
            ui.label("部署點");

            ui.label("｜");

            // 敵人單位
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE),
                egui::Sense::empty(),
            );
            ui.painter().rect_filled(rect, 0.0, BATTLEFIELD_COLOR_UNIT);
            ui.label("單位");

            ui.label("｜");

            // 物件
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(BATTLEFIELD_CELL_SIZE, BATTLEFIELD_CELL_SIZE),
                egui::Sense::empty(),
            );
            ui.painter()
                .rect_filled(rect, 0.0, BATTLEFIELD_COLOR_OBJECT);
            ui.label("物件");
        });
    });
}
