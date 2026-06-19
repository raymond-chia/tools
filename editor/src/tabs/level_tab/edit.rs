use super::{BattleAction, DragState, DraggedObject, LevelTabMode, LevelTabUIState, battlefield};
use crate::constants::*;
use crate::generic_editor::MessageState;
use crate::utils::search::{
    combobox_with_dynamic_height, filter_by_search, render_filtered_options, render_search_input,
};
use bevy_ecs::world::World;
use board::domain::alias::{Coord, ID, TypeName};
use board::domain::constants::{PLAYER_ALLIANCE_ID, PLAYER_FACTION_ID};
use board::domain::core_types::SkillType;
use board::ecs_types::components::Position;
use board::ecs_types::resources::Board;
use board::loader_schema::{
    Faction, LevelType, ObjectPlacement, ObjectType, ObjectsToml, SkillsToml, UnitPlacement,
    UnitType, UnitsToml,
};
use std::collections::{HashMap, HashSet};

/// 渲染編輯模式的表單
pub fn render_form(
    ui: &mut egui::Ui,
    level: &mut LevelType,
    ui_state: &mut LevelTabUIState,
    message_state: &mut MessageState,
) {
    // 基本資訊區
    ui.horizontal(|ui| {
        ui.label("名稱：");
        ui.text_edit_singleline(&mut level.name);
    });

    ui.horizontal(|ui| {
        ui.label("棋盤寬度：");
        ui.add(
            egui::DragValue::new(&mut level.board_width)
                .speed(DRAG_VALUE_SPEED)
                .range(1..=Coord::MAX),
        );
        ui.add_space(SPACING_SMALL);
        ui.label("棋盤高度：");
        ui.add(
            egui::DragValue::new(&mut level.board_height)
                .speed(DRAG_VALUE_SPEED)
                .range(1..=Coord::MAX),
        );
    });

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 陣營配置區
    ui.heading("陣營配置");
    render_faction_list(ui, &mut level.factions);

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 部署點配置區
    ui.vertical(|ui| {
        ui.label("玩家人數上限：");
        ui.add(
            egui::DragValue::new(&mut level.max_player_units)
                .speed(DRAG_VALUE_SPEED)
                .range(0..=6),
        );
        ui.add_space(SPACING_SMALL);
        ui.heading("部署點");
        render_deployment_positions_list(ui, &mut level.deployment_positions);
    });

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 單位配置區（可收合，避免新增/複製時清單變長把戰場往下推）
    let unit_names: Vec<TypeName> = ui_state
        .available_units
        .iter()
        .map(|u| u.name.clone())
        .collect();
    egui::CollapsingHeader::new(format!("單位配置（{}）", level.unit_placements.len()))
        .id_salt("unit_placements_header")
        .default_open(false)
        .show(ui, |ui| {
            render_unit_placement_list(
                ui,
                &mut level.unit_placements,
                &level.factions,
                &unit_names,
                &mut ui_state.unit_search_query,
            );
        });

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    // 物件配置區（可收合）
    let object_names: Vec<TypeName> = ui_state
        .available_objects
        .iter()
        .map(|o| o.name.clone())
        .collect();
    egui::CollapsingHeader::new(format!("物件配置（{}）", level.object_placements.len()))
        .id_salt("object_placements_header")
        .default_open(false)
        .show(ui, |ui| {
            render_object_placement_list(
                ui,
                &mut level.object_placements,
                &object_names,
                &mut ui_state.object_search_query,
            );
        });

    ui.add_space(SPACING_MEDIUM);
    ui.separator();

    if ui.button("開始部署").clicked() {
        match initialize_world(
            level,
            &ui_state.available_units,
            &ui_state.available_skills,
            &ui_state.available_objects,
        ) {
            Ok(world) => {
                ui_state.world = world;
                ui_state.selected_left_pos = None;
                ui_state.selected_right_pos = None;
                ui_state.battle_action = BattleAction::Normal;
                ui_state.mode = LevelTabMode::Deploy;
                return;
            }
            Err(msg) => {
                message_state.set_error(format!("進入部署模式失敗：{}", msg));
            }
        }
    }

    // 戰場預覽區
    render_battlefield(ui, level, ui_state, message_state);
}

/// 渲染陣營列表
fn render_faction_list(ui: &mut egui::Ui, factions: &mut Vec<Faction>) {
    if ui.button("新增陣營").clicked() {
        let next_id = factions
            .iter()
            .map(|f| f.id)
            .max()
            .map(|m| m + 1)
            .unwrap_or(PLAYER_FACTION_ID);
        factions.push(Faction {
            id: next_id,
            name: String::new(),
            alliance: PLAYER_ALLIANCE_ID,
            color: [128, 128, 128],
        });
    }

    let mut to_remove = None;
    for (index, faction) in factions.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", index + 1));
                if ui.button("刪除").clicked() {
                    to_remove = Some(index);
                }

                ui.separator();

                ui.label("同盟：");
                ui.add(
                    egui::DragValue::new(&mut faction.alliance)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=ID::MAX),
                );

                ui.label("ID：");
                ui.add(
                    egui::DragValue::new(&mut faction.id)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=ID::MAX),
                );

                ui.label("名稱：");
                ui.text_edit_singleline(&mut faction.name);

                ui.label("顏色：");
                let mut color32 =
                    egui::Color32::from_rgb(faction.color[0], faction.color[1], faction.color[2]);
                if ui.color_edit_button_srgba(&mut color32).changed() {
                    faction.color = [color32.r(), color32.g(), color32.b()];
                }
            });
        });
        ui.add_space(SPACING_SMALL);
    }

    if let Some(index) = to_remove {
        factions.remove(index);
    }
}

/// 渲染部署點列表
fn render_deployment_positions_list(ui: &mut egui::Ui, positions: &mut Vec<Position>) {
    if ui.button("新增放置點").clicked() {
        positions.push(Position::default());
    }

    let mut to_remove = None;
    for (index, position) in positions.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", index + 1));
                if ui.button("刪除").clicked() {
                    to_remove = Some(index);
                }

                ui.separator();

                ui.label("X：");
                ui.add(
                    egui::DragValue::new(&mut position.x)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );
                ui.label("Y：");
                ui.add(
                    egui::DragValue::new(&mut position.y)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );
            });
        });
        ui.add_space(SPACING_SMALL);
    }

    if let Some(index) = to_remove {
        positions.remove(index);
    }
}

/// 渲染單位配置列表
fn render_unit_placement_list(
    ui: &mut egui::Ui,
    placements: &mut Vec<UnitPlacement>,
    factions: &[Faction],
    available_units: &[TypeName],
    unit_search_query: &mut TypeName,
) {
    if ui.button("新增單位").clicked() {
        placements.push(UnitPlacement::default());
    }

    let mut to_remove = None;
    for (index, placement) in placements.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", index + 1));
                if ui.button("刪除").clicked() {
                    to_remove = Some(index);
                }

                ui.separator();

                ui.label("X：");
                ui.add(
                    egui::DragValue::new(&mut placement.position.x)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );
                ui.label("Y：");
                ui.add(
                    egui::DragValue::new(&mut placement.position.y)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );

                ui.separator();

                ui.label("陣營：");
                let selected_name = factions
                    .iter()
                    .find(|f| f.id == placement.faction_id)
                    .map(|f| f.name.as_str())
                    .unwrap_or("（未選擇）");
                combobox_with_dynamic_height(
                    &format!("unit_faction_{}", index),
                    selected_name,
                    factions.len(),
                )
                .show_ui(ui, |ui| {
                    for faction in factions {
                        ui.selectable_value(&mut placement.faction_id, faction.id, &faction.name);
                    }
                });

                ui.separator();

                ui.label("單位類型：");
                if available_units.is_empty() {
                    ui.label("（尚未定義任何單位）");
                } else {
                    let display = if placement.unit_type_name.is_empty() {
                        "選擇單位"
                    } else {
                        &placement.unit_type_name
                    };
                    combobox_with_dynamic_height(
                        &format!("unit_placement_{}", index),
                        display,
                        available_units.len(),
                    )
                    .show_ui(ui, |ui| {
                        let response = render_search_input(ui, unit_search_query);
                        ui.memory_mut(|mem| mem.request_focus(response.id));
                        ui.separator();
                        let visible_units = filter_by_search(available_units, unit_search_query);
                        let hidden_count = available_units.len() - visible_units.len();
                        render_filtered_options(
                            ui,
                            &visible_units,
                            hidden_count,
                            &mut placement.unit_type_name,
                            unit_search_query,
                        );
                    });
                }
            });
        });
        ui.add_space(SPACING_SMALL);
    }

    if let Some(index) = to_remove {
        placements.remove(index);
    }
}

/// 渲染物件配置列表
fn render_object_placement_list(
    ui: &mut egui::Ui,
    placements: &mut Vec<ObjectPlacement>,
    available_objects: &[TypeName],
    object_search_query: &mut TypeName,
) {
    if ui.button("新增物件").clicked() {
        placements.push(ObjectPlacement::default());
    }

    let mut to_remove = None;
    for (index, placement) in placements.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", index + 1));
                if ui.button("刪除").clicked() {
                    to_remove = Some(index);
                }

                ui.separator();

                ui.label("X：");
                ui.add(
                    egui::DragValue::new(&mut placement.position.x)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );
                ui.label("Y：");
                ui.add(
                    egui::DragValue::new(&mut placement.position.y)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=Coord::MAX),
                );

                ui.separator();

                ui.label("物件類型：");
                if available_objects.is_empty() {
                    ui.label("（尚未定義任何物件）");
                } else {
                    let display = if placement.object_type_name.is_empty() {
                        "選擇物件"
                    } else {
                        &placement.object_type_name
                    };
                    combobox_with_dynamic_height(
                        &format!("object_placement_{}", index),
                        display,
                        available_objects.len(),
                    )
                    .show_ui(ui, |ui| {
                        let response = render_search_input(ui, object_search_query);
                        ui.memory_mut(|mem| mem.request_focus(response.id));
                        ui.separator();
                        let visible_objects =
                            filter_by_search(available_objects, object_search_query);
                        let hidden_count = available_objects.len() - visible_objects.len();
                        render_filtered_options(
                            ui,
                            &visible_objects,
                            hidden_count,
                            &mut placement.object_type_name,
                            object_search_query,
                        );
                    });
                }
            });
        });
        ui.add_space(SPACING_SMALL);
    }

    if let Some(index) = to_remove {
        placements.remove(index);
    }
}

/// 渲染戰場預覽，支持拖曳修改位置
fn render_battlefield(
    ui: &mut egui::Ui,
    level: &mut LevelType,
    ui_state: &mut LevelTabUIState,
    message_state: &mut MessageState,
) {
    let board = Board {
        width: level.board_width,
        height: level.board_height,
    };

    ui.heading("戰場預覽");

    let scroll_output = egui::ScrollArea::both()
        .auto_shrink([false; 2])
        // 避免兩個 scroll bar 重疊
        .max_width(ui.available_width() - SPACING_MEDIUM)
        .min_scrolled_height(LIST_PANEL_MIN_HEIGHT)
        .show(ui, |ui: &mut egui::Ui| -> Option<Position> {
            let total_size = battlefield::calculate_grid_dimensions(board);
            let (rect, response) =
                ui.allocate_exact_size(total_size, egui::Sense::click_and_drag());

            let drag_state = update_drag_state(ui_state.drag_state, &response, rect, board, level);
            ui_state.drag_state = drag_state;
            let hovered_pos = battlefield::compute_hover_pos(&response, rect, board);
            let dragged_pos = drag_state.and_then(|_| hovered_pos);
            // 在更新後重新建立 lookup maps
            let (deployment_set, unit_map, object_map) = prepare_lookup_maps(level);

            // 渲染網格
            let get_cell_info_fn =
                get_cell_info(&level.factions, &deployment_set, &unit_map, &object_map);
            let get_cell_highlight_fn = get_cell_highlight(drag_state, dragged_pos);
            battlefield::render_grid(
                ui,
                rect,
                board,
                ui_state.scroll_offset,
                get_cell_info_fn,
                get_cell_highlight_fn,
            );
            if let Some(hovered_pos) = hovered_pos {
                let get_tooltip_info_fn = get_tooltip_info(&deployment_set, &unit_map, &object_map);
                battlefield::render_hover_tooltip(ui, rect, hovered_pos, get_tooltip_info_fn);
            }

            // 把懸停格回傳出去，供閉包外判斷 Ctrl+D / Backspace
            hovered_pos
        });

    // 儲存滾動位置供下一幀使用
    ui_state.scroll_offset = scroll_output.state.offset;

    if let Some(hovered_pos) = scroll_output.inner {
        // Ctrl+D：複製滑鼠懸停那格的單位 / 物件到最近空格
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::D)) {
            try_duplicate(level, hovered_pos, board, message_state);
        }
        // Backspace：刪除滑鼠懸停那格的單位 / 物件
        if ui.input(|i| i.key_pressed(egui::Key::Backspace)) {
            try_delete(level, hovered_pos);
        }
    }

    ui.add_space(SPACING_SMALL);
    battlefield::render_battlefield_legend(ui);

    ui.label("快捷鍵：Ctrl+D 複製懸停格。Backspace 刪除");
}

// ==================== 輔助函數 ====================

/// 序列化資料並初始化 ECS World
fn initialize_world(
    level: &LevelType,
    units: &[UnitType],
    skills: &[SkillType],
    objects: &[ObjectType],
) -> Result<World, String> {
    let units_toml = toml::to_string_pretty(&UnitsToml {
        units: units.to_vec(),
    })
    .map_err(|e| format!("序列化單位資料失敗：{}", e))?;
    let skills_toml = toml::to_string_pretty(&SkillsToml {
        skills: skills.to_vec(),
    })
    .map_err(|e| format!("序列化技能資料失敗：{}", e))?;
    let objects_toml = toml::to_string_pretty(&ObjectsToml {
        objects: objects.to_vec(),
    })
    .map_err(|e| format!("序列化物件資料失敗：{}", e))?;
    let level_toml =
        toml::to_string_pretty(level).map_err(|e| format!("序列化關卡資料失敗：{}", e))?;

    let mut world = World::new();
    board::ecs_logic::loader::parse_and_insert_game_data(
        &mut world,
        &units_toml,
        &skills_toml,
        &objects_toml,
    )
    .map_err(|e| format!("載入遊戲資料失敗：{:?}", e))?;

    board::ecs_logic::spawner::spawn_level(&mut world, &level_toml, &level.name)
        .map_err(|e| format!("生成關卡失敗：{:?}", e))?;

    Ok(world)
}

/// 識別被拖曳的物體及其索引
fn identify_dragged_object(level: &LevelType, pos: &Position) -> Option<DraggedObject> {
    for (idx, deployment) in level.deployment_positions.iter().enumerate() {
        if deployment == pos {
            return Some(DraggedObject::Deployment(idx));
        }
    }
    for (idx, unit) in level.unit_placements.iter().enumerate() {
        if unit.position == *pos {
            return Some(DraggedObject::Unit(idx));
        }
    }
    for (idx, obj) in level.object_placements.iter().enumerate() {
        if obj.position == *pos {
            return Some(DraggedObject::Object(idx));
        }
    }
    None
}

/// 應用拖曳更新
fn apply_drag_update(level: &mut LevelType, state: DragState, new_pos: Position) {
    match state.object {
        DraggedObject::Deployment(idx) => {
            if idx < level.deployment_positions.len() {
                level.deployment_positions[idx] = new_pos;
            }
        }
        DraggedObject::Unit(idx) => {
            if idx < level.unit_placements.len() {
                level.unit_placements[idx].position = new_pos;
            }
        }
        DraggedObject::Object(idx) => {
            if idx < level.object_placements.len() {
                level.object_placements[idx].position = new_pos;
            }
        }
    }
}

/// 更新拖曳狀態：處理拖曳開始與結束，並將位移結果寫入 level
fn update_drag_state(
    drag_state: Option<DragState>,
    response: &egui::Response,
    rect: egui::Rect,
    board: Board,
    level: &mut LevelType,
) -> Option<DragState> {
    // 拖曳開始：找出被點中的物件
    if response.drag_started() {
        return battlefield::compute_hover_pos(response, rect, board)
            .and_then(|pos| identify_dragged_object(level, &pos))
            .map(|dragged| DragState { object: dragged });
    }

    // 拖曳中：保持狀態不變
    if response.dragged() {
        return drag_state;
    }

    // 拖曳結束：套用位移並清除狀態
    let state = match drag_state {
        None => return None,
        Some(s) => s,
    };
    if let Some(new_pos) = battlefield::compute_hover_pos(response, rect, board) {
        apply_drag_update(level, state, new_pos);
    }
    return None;
}

/// 建立查詢表以加速格子內容查詢
fn prepare_lookup_maps(
    level: &LevelType,
) -> (
    HashSet<Position>,
    HashMap<Position, &UnitPlacement>,
    HashMap<Position, &ObjectPlacement>,
) {
    let deployment_set: HashSet<Position> = level.deployment_positions.iter().cloned().collect();
    let unit_map: HashMap<Position, &UnitPlacement> = level
        .unit_placements
        .iter()
        .map(|u| (u.position, u))
        .collect();
    let object_map: HashMap<Position, &ObjectPlacement> = level
        .object_placements
        .iter()
        .map(|o| (o.position, o))
        .collect();
    (deployment_set, unit_map, object_map)
}

fn get_cell_info(
    factions: &[Faction],
    deployment_set: &HashSet<Position>,
    unit_map: &HashMap<Position, &UnitPlacement>,
    object_map: &HashMap<Position, &ObjectPlacement>,
) -> impl Fn(Position) -> (String, egui::Color32, egui::Color32) {
    // cell_text, font_color, bg_color
    |pos: Position| -> (String, egui::Color32, egui::Color32) {
        if deployment_set.contains(&pos) {
            (
                "".to_string(),
                BATTLEFIELD_COLOR_DEPLOYMENT,
                BATTLEFIELD_COLOR_DEPLOYMENT,
            )
        } else if let Some(unit) = unit_map.get(&pos) {
            let faction_color = factions
                .iter()
                .find(|f| f.id == unit.faction_id)
                .map(|f| egui::Color32::from_rgb(f.color[0], f.color[1], f.color[2]))
                .unwrap_or(egui::Color32::BLACK);
            let abbrev: TypeName = unit.unit_type_name.chars().take(2).collect();
            (abbrev, faction_color, BATTLEFIELD_COLOR_UNIT)
        } else if let Some(obj) = object_map.get(&pos) {
            let abbrev: TypeName = obj.object_type_name.chars().take(2).collect();
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

fn get_cell_highlight(
    drag_state: Option<DragState>,
    hovered_in_bounds: Option<Position>,
) -> impl Fn(Position) -> battlefield::CellHighlight {
    move |pos: Position| battlefield::CellHighlight {
        border: (drag_state.is_some() && hovered_in_bounds == Some(pos))
            .then_some(BATTLEFIELD_COLOR_HIGHLIGHT),
        bg: None,
    }
}

fn get_tooltip_info(
    deployment_set: &HashSet<Position>,
    unit_map: &HashMap<Position, &UnitPlacement>,
    object_map: &HashMap<Position, &ObjectPlacement>,
) -> impl Fn(Position) -> String {
    |pos| -> String {
        if deployment_set.contains(&pos) {
            format!("({}, {})\n玩家部署點", pos.x, pos.y)
        } else if let Some(unit) = unit_map.get(&pos) {
            format!("({}, {})\n單位 {}", pos.x, pos.y, unit.unit_type_name)
        } else if let Some(obj) = object_map.get(&pos) {
            format!("({}, {})\n物件 {}", pos.x, pos.y, obj.object_type_name)
        } else {
            format!("({}, {})", pos.x, pos.y)
        }
    }
}

// 找最近空格:以 origin 為中心,曼哈頓距離 1~3 圈往外找,跳過所有已占用格
fn find_nearest_empty(level: &LevelType, origin: Position, board: Board) -> Option<Position> {
    let (deployment_set, unit_map, object_map) = prepare_lookup_maps(level);
    let occupied = |p: &Position| {
        deployment_set.contains(p) || unit_map.contains_key(p) || object_map.contains_key(p)
    };
    for radius in 1..=3 {
        for dx in -(radius as i64)..=(radius as i64) {
            for dy in -(radius as i64)..=(radius as i64) {
                if dx.unsigned_abs() as usize + dy.unsigned_abs() as usize != radius {
                    continue; // 只取剛好等於這圈距離的格子
                }
                let (Some(x), Some(y)) = (
                    origin.x.checked_add_signed(dx as isize),
                    origin.y.checked_add_signed(dy as isize),
                ) else {
                    continue;
                };
                let cand = Position { x, y };
                if board::logic::board::is_valid_position(board, cand) && !occupied(&cand) {
                    return Some(cand);
                }
            }
        }
    }
    None
}

// 複製：依懸停格找出是部署點 / unit / object，在最近空格新增一份
fn try_duplicate(
    level: &mut LevelType,
    origin: Position,
    board: Board,
    message_state: &mut MessageState,
) {
    // 先確認原格有可複製物（空格不處理）
    let dragged = identify_dragged_object(level, &origin);
    if dragged.is_none() {
        return; // 懸停格是空格，靜默不動作
    }

    let Some(new_pos) = find_nearest_empty(level, origin, board) else {
        message_state.set_error("附近三格內沒有空格可放置複製品".to_string());
        return;
    };

    match dragged {
        Some(DraggedObject::Deployment(_)) => {
            level.deployment_positions.push(new_pos);
        }
        Some(DraggedObject::Unit(idx)) => {
            let mut copy = level.unit_placements[idx].clone();
            copy.position = new_pos;
            level.unit_placements.push(copy);
        }
        Some(DraggedObject::Object(idx)) => {
            let mut copy = level.object_placements[idx].clone();
            copy.position = new_pos;
            level.object_placements.push(copy);
        }
        None => {}
    }
}

// 刪除：依懸停格找出是部署點 / unit / object，移除整筆
fn try_delete(level: &mut LevelType, origin: Position) {
    match identify_dragged_object(level, &origin) {
        Some(DraggedObject::Deployment(idx)) => {
            level.deployment_positions.remove(idx);
        }
        Some(DraggedObject::Unit(idx)) => {
            level.unit_placements.remove(idx);
        }
        Some(DraggedObject::Object(idx)) => {
            level.object_placements.remove(idx);
        }
        // 空格：靜默不動作
        None => {}
    }
}
