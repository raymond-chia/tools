//! 戰場共用邏輯：網格渲染、快照查詢、詳情面板

use crate::constants::*;
use bevy_ecs::world::World;
use board::domain::alias::{Coord, ID};
use board::domain::constants::PLAYER_ALLIANCE_ID;
use board::ecs_logic::query::ObjectQueryResult;
use board::ecs_types::components::{Position, UnitBundle};
use board::ecs_types::resources::{Board, LevelConfig};
use board::error::Result as CResult;
use board::loader_schema::Faction;
use std::collections::{HashMap, HashSet};

// ==================== 資料型別 ====================

/// 棋盤可見範圍（格子索引，包含兩端）
#[derive(Debug, Clone, Copy)]
pub struct VisibleGridRange {
    pub min: Position,
    pub max: Position,
}

/// 戰場模式所需的所有關卡查詢結果
pub struct Snapshot {
    pub board: Board,
    pub max_player_units: usize,
    pub deployment_set: HashSet<Position>,
    pub deployment_positions: Vec<Position>,
    pub level_config: LevelConfig,
    pub unit_map: HashMap<Position, UnitBundle>,
    pub object_map: HashMap<Position, ObjectQueryResult>,
}

// ==================== 快照查詢 ====================

/// 一次查詢部署/戰鬥模式所需的所有關卡資料
pub fn query_snapshot(world: &mut World) -> CResult<Snapshot> {
    let unit_map = board::ecs_logic::query::get_all_units(world)?;
    let object_map = board::ecs_logic::query::get_all_objects(world)?;
    let board = board::ecs_logic::query::get_board(world)?;
    let deployment_config = board::ecs_logic::query::get_deployment_config(world)?;
    let level_config = board::ecs_logic::query::get_level_config(world)?;
    Ok(Snapshot {
        board,
        max_player_units: deployment_config.max_player_units,
        deployment_set: deployment_config
            .deployment_positions
            .iter()
            .cloned()
            .collect(),
        deployment_positions: deployment_config.deployment_positions,
        level_config,
        unit_map,
        object_map,
    })
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

// ==================== 格子資訊 ====================

pub fn get_cell_info(
    snapshot: &Snapshot,
) -> impl Fn(Position) -> (String, egui::Color32, egui::Color32) {
    |pos: Position| -> (String, egui::Color32, egui::Color32) {
        if snapshot.deployment_set.contains(&pos) {
            if let Some(bundle) = snapshot.unit_map.get(&pos) {
                let faction_color =
                    get_faction_color(&snapshot.level_config.factions, bundle.faction.0);
                let abbrev = get_unit_abbr(&bundle.occupant_type_name.0);
                (abbrev, faction_color, BATTLEFIELD_COLOR_DEPLOYMENT)
            } else {
                (
                    "".to_string(),
                    BATTLEFIELD_COLOR_DEPLOYMENT,
                    BATTLEFIELD_COLOR_DEPLOYMENT,
                )
            }
        } else if let Some(bundle) = snapshot.unit_map.get(&pos) {
            let faction_color =
                get_faction_color(&snapshot.level_config.factions, bundle.faction.0);
            let abbrev = get_unit_abbr(&bundle.occupant_type_name.0);
            (abbrev, faction_color, BATTLEFIELD_COLOR_UNIT)
        } else if let Some(obj) = snapshot.object_map.get(&pos) {
            let abbrev = get_unit_abbr(&obj.bundle.occupant_type_name.0);
            (abbrev, egui::Color32::BLACK, BATTLEFIELD_COLOR_OBJECT)
        } else {
            (
                "".to_string(),
                BATTLEFIELD_COLOR_EMPTY,
                BATTLEFIELD_COLOR_EMPTY,
            )
        }
    }
}

pub fn is_highlight(highlight_pos: Option<Position>) -> impl Fn(Position) -> bool {
    move |pos: Position| highlight_pos == Some(pos)
}

pub fn get_tooltip_info(snapshot: &Snapshot) -> impl Fn(Position) -> String {
    |pos| -> String {
        if snapshot.deployment_set.contains(&pos) {
            if let Some(bundle) = snapshot.unit_map.get(&pos) {
                format!(
                    "({}, {})\n部署點：{}",
                    pos.x, pos.y, bundle.occupant_type_name.0
                )
            } else {
                format!("({}, {})\n空部署點", pos.x, pos.y)
            }
        } else if let Some(bundle) = snapshot.unit_map.get(&pos) {
            format!(
                "({}, {})\n單位 {}",
                pos.x, pos.y, bundle.occupant_type_name.0
            )
        } else if let Some(obj) = snapshot.object_map.get(&pos) {
            format!(
                "({}, {})\n物件 {}",
                pos.x, pos.y, obj.bundle.occupant_type_name.0
            )
        } else {
            format!("({}, {})", pos.x, pos.y)
        }
    }
}

// ==================== 渲染層 ====================

/// 繪製編輯模式棋盤格子，支持拖曳預覽
pub fn render_grid(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    board: Board,
    scroll_offset: egui::Vec2,
    get_cell_info: impl Fn(Position) -> (String, egui::Color32, egui::Color32),
    is_highlight: impl Fn(Position) -> bool,
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

            painter.rect_filled(cell_rect, 0.0, bg_color);
            painter.text(
                cell_rect.center(),
                egui::Align2::CENTER_CENTER,
                cell_text,
                egui::FontId::proportional(BATTLEFIELD_TEXT_SIZE),
                font_color,
            );

            // 拖曳預覽：外框高亮目標位置
            if is_highlight(pos) {
                painter.rect_stroke(
                    cell_rect,
                    0.0,
                    egui::Stroke::new(STROKE_WIDTH, BATTLEFIELD_COLOR_HIGHLIGHT),
                    egui::epaint::StrokeKind::Outside,
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

pub fn render_details_panel(ui: &mut egui::Ui, pos: Position, snapshot: &Snapshot) {
    ui.heading(format!("詳情 ({}, {})", pos.x, pos.y));
    ui.add_space(SPACING_SMALL);

    if let Some(bundle) = snapshot.unit_map.get(&pos) {
        render_unit_details(ui, bundle, &snapshot.level_config.factions);
    }

    ui.add_space(SPACING_MEDIUM);
    ui.separator();
    ui.add_space(SPACING_MEDIUM);

    if let Some(obj) = snapshot.object_map.get(&pos) {
        render_object_details(ui, obj);
    }
}

fn render_unit_details(ui: &mut egui::Ui, bundle: &UnitBundle, factions: &[Faction]) {
    ui.label(format!("類型：單位"));
    ui.label(format!("名稱：{}", bundle.occupant_type_name.0));

    let faction_name = factions
        .iter()
        .find(|f| f.id == bundle.faction.0)
        .map(|f| f.name.as_str())
        .unwrap_or("未知");
    ui.label(format!("陣營：{}", faction_name));

    ui.add_space(SPACING_SMALL);
    ui.separator();
    ui.label("屬性：");

    let attrs = &bundle.attributes;
    ui.label(format!("HP：{} / {}", attrs.current_hp.0, attrs.max_hp.0));
    ui.label(format!("MP：{} / {}", attrs.current_mp.0, attrs.max_mp.0));
    ui.label(format!("先攻：{}", attrs.initiative.0));
    ui.label(format!("移動：{}", attrs.movement.0));
    ui.label(format!("物攻：{}", attrs.physical_attack.0));
    ui.label(format!("魔攻：{}", attrs.magical_attack.0));
    ui.label(format!("命中：{}", attrs.hit.0));
    ui.label(format!("迴避：{}", attrs.evasion.0));
    ui.label(format!("格擋：{}", attrs.block.0));
    ui.label(format!("格擋減傷：{}", attrs.block_protection.0));
    ui.label(format!("魔法DC：{}", attrs.magical_dc.0));
    ui.label(format!("剛毅：{}", attrs.fortitude.0));
    ui.label(format!("反射：{}", attrs.reflex.0));
    ui.label(format!("意志：{}", attrs.will.0));
    ui.label(format!("反應：{}", attrs.reaction.0));

    if !bundle.skills.0.is_empty() {
        ui.add_space(SPACING_SMALL);
        ui.separator();
        ui.label("技能：");
        for skill in &bundle.skills.0 {
            ui.label(format!("  • {}", skill));
        }
    }
}

fn render_object_details(ui: &mut egui::Ui, obj: &ObjectQueryResult) {
    ui.label(format!("類型：物件"));
    ui.label(format!("名稱：{}", obj.bundle.occupant_type_name.0));

    ui.add_space(SPACING_SMALL);
    ui.separator();

    ui.label(format!("移動花費：{}", obj.bundle.terrain_movement_cost.0));
    ui.label(format!("HP 修正：{}", obj.bundle.hp_modify.0));
    ui.label(format!("阻擋視線：{}", obj.blocks_sight));
    ui.label(format!("阻擋聲音：{}", obj.blocks_sound));
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

// ==================== 輔助函數 ====================

/// 取得敵方單位
pub fn enemy_units(snapshot: &Snapshot) -> impl Iterator<Item = &UnitBundle> {
    let enemy_faction_ids: HashSet<ID> = snapshot
        .level_config
        .factions
        .iter()
        .filter(|f| f.alliance != PLAYER_ALLIANCE_ID)
        .map(|f| f.id)
        .collect();
    snapshot
        .unit_map
        .values()
        .filter(move |bundle| enemy_faction_ids.contains(&bundle.faction.0))
}

pub fn get_faction_color(factions: &[Faction], unit_faction_id: ID) -> egui::Color32 {
    factions
        .iter()
        .find(|f| f.id == unit_faction_id)
        .map(|f| egui::Color32::from_rgb(f.color[0], f.color[1], f.color[2]))
        .unwrap_or(egui::Color32::BLACK)
}

pub fn get_unit_abbr(unit_name: &str) -> String {
    unit_name.chars().take(2).collect()
}
