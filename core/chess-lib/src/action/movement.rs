//! movement.rs：
//! - 負責單位移動相關邏輯（如移動規則、移動點消耗、特殊移動技能等）。
//! - 僅處理移動本身，不負責戰鬥判定、AI 決策或棋盤初始化。
//! - 移動相關的資料結構與輔助函式應集中於此。
use crate::*;
use skills_lib::*;
use std::collections::{BTreeMap, HashMap};

const FIRST_MOVEMENT_COLOR: RGBA = (50, 100, 255, 150); // 淺藍
const SECOND_MOVEMENT_COLOR: RGBA = (50, 50, 255, 150); // 深藍
const FIRST_PATH_COLOR: RGBA = (125, 0, 125, 150); // 淺紫
const SECOND_PATH_COLOR: RGBA = (100, 0, 100, 150); // 深紫

/// 檢查位置的地形和物件是否可通行（不處理單位阻擋）
pub fn is_tile_passable(board: &Board, pos: Pos) -> bool {
    match board.get_tile(pos) {
        None => false, // 地形不存在 -> 不可通行
        Some(tile) => {
            if let Some(object) = &tile.object {
                object.is_passable() // 物件可通行
            } else {
                true // 無物件 -> 可通行
            }
        }
    }
}

/// 提供移動邏輯用的棋盤視圖，實作 PathfindingBoard 供路徑搜尋演算法使用
struct MovableBoardView<'a> {
    board: &'a Board,
    move_points: MovementCost,
    moved_distance: MovementCost,
}

impl<'a> PathfindingBoard for MovableBoardView<'a> {
    /// 判斷座標是否合法
    fn is_valid(&self, pos: Pos) -> bool {
        self.board.get_tile(pos).is_some()
    }

    /// 判斷座標是否可通行（不可超越兩倍移動力，不可穿越敵軍，不可穿越障礙物）
    fn is_passable(&self, active_unit_pos: Pos, pos: Pos, total: MovementCost) -> bool {
        // 不能超越兩倍移動力
        if total > self.move_points * 2 - self.moved_distance {
            return false;
        }
        // 檢查地形和物件阻擋
        if !is_tile_passable(self.board, pos) {
            return false;
        }
        let Some(unit_id) = self.board.pos_to_unit(active_unit_pos) else {
            // 不合理
            return false;
        };
        let Some(active_team) = self.board.units.get(&unit_id).map(|unit| &unit.team) else {
            // 不合理
            return false;
        };
        match self.board.pos_to_unit(pos) {
            None => true, // 目標位置無單位
            Some(unit_id) => {
                // 不能穿越敵軍
                let target_team = self.board.units.get(&unit_id).map_or("", |unit| &unit.team);
                active_team == target_team
            }
        }
    }

    /// 取得座標移動成本
    fn get_cost(&self, pos: Pos) -> MovementCost {
        self.board
            .get_tile(pos)
            .map(|t| movement_cost(t.terrain))
            .unwrap_or(MAX_MOVEMENT_COST)
    }

    /// 取得鄰近座標（上下左右）
    fn get_neighbors(&self, pos: Pos) -> Vec<Pos> {
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        dirs.into_iter()
            .map(|(dx, dy)| (dx + pos.x as isize, dy + pos.y as isize))
            .filter_map(|(x, y)| {
                if x >= 0 && y >= 0 {
                    Some((x as usize, y as usize))
                } else {
                    None
                }
            })
            .map(|(x, y)| Pos { x, y })
            .collect()
    }
}

/// 計算指定單位的可移動範圍（回傳所有可達座標與路徑成本）
/// board: 棋盤物件，from: 單位座標
pub fn movable_area(
    board: &Board,
    from: Pos,
    // 檢測
    skills_map: &BTreeMap<String, Skill>,
) -> HashMap<Pos, (MovementCost, Pos)> {
    let unit_id = match board.pos_to_unit(from) {
        Some(id) => id,
        None => return HashMap::new(),
    };
    let unit = match board.units.get(&unit_id) {
        Some(u) => u,
        None => return HashMap::new(),
    };
    if unit.has_cast_skill_this_turn && !has_hit_and_run_skill(unit, skills_map) {
        // 若無「打帶跑」則禁止移動，回傳現有錯誤型別
        return HashMap::new();
    }
    let view = MovableBoardView {
        board,
        move_points: unit.move_points,
        moved_distance: unit.moved,
    };
    dijkstra(&view, from)
}

pub fn reconstruct_path(
    map: &HashMap<Pos, (MovementCost, Pos)>,
    from: Pos,
    to: Pos,
) -> Result<Vec<Pos>, Error> {
    let func = "reconstruct_path";

    let mut path = Vec::new();
    let mut current = to;
    while current != from {
        let Some((_, prev)) = map.get(&current) else {
            return Err(Error::NotReachable { func, pos: to });
        };
        path.push(current);
        current = *prev;
    }
    path.push(from);
    path.reverse();
    Ok(path)
}

pub fn move_unit_along_path(
    board: &mut Board,
    path: Vec<Pos>,
    // 檢測
    skills_map: &BTreeMap<String, Skill>,
) -> Result<(), Error> {
    let func = "move_unit_along_path";

    let actor = path.get(0).ok_or(Error::InvalidParameter {
        func,
        detail: "actor position not found".to_string(),
    })?;
    let unit_id = match board.pos_to_unit(*actor) {
        Some(id) => id,
        None => return Err(Error::NoUnitAtPos { func, pos: *actor }),
    };
    let unit = match board.units.get(&unit_id) {
        Some(u) => u,
        None => return Err(Error::NoUnitAtPos { func, pos: *actor }),
    };
    if unit.has_cast_skill_this_turn && !has_hit_and_run_skill(unit, skills_map) {
        // 若無「打帶跑」則禁止移動
        return Err(Error::NotEnoughAP { func });
    }
    let mut actor = *actor;
    for next in path {
        let result = move_unit(board, actor, next);
        match result {
            Ok(_) => {
                actor = next;
            }
            Err(Error::AlliedUnitAtPos { .. }) => {}
            Err(e) => {
                return Err(Error::Wrap {
                    func,
                    source: Box::new(e),
                });
            }
        }
    }
    Ok(())
}

pub fn movement_tile_color(
    board: &Board,
    movable: &HashMap<Pos, (MovementCost, Pos)>,
    active_unit_id: &UnitID,
    path: &[Pos],
    pos: Pos,
) -> Result<RGBA, Error> {
    let func = "movement_tile_color";

    if board.pos_to_unit(pos).is_some() {
        return Err(Error::NotReachable { func, pos });
    }
    let Some((cost, _)) = movable.get(&pos) else {
        return Err(Error::NotReachable { func, pos });
    };
    let cost = *cost;

    let (move_points, moved) = board
        .units
        .get(active_unit_id)
        .map(|u| (u.move_points, u.moved))
        .unwrap_or((0, 0));
    let is_first = moved + cost <= move_points;
    // 顯示可移動範圍
    let color = if is_first {
        FIRST_MOVEMENT_COLOR // 淺藍
    } else {
        SECOND_MOVEMENT_COLOR // 深藍
    };
    // 顯示移動路徑
    if !path.contains(&pos) {
        // 不在移動路徑
        return Ok(color);
    }
    if is_first {
        Ok(FIRST_PATH_COLOR) // 淺紫
    } else {
        Ok(SECOND_PATH_COLOR) // 深紫
    }
}

use inner::*;
mod inner {
    use super::*;

    /// 將 actor 位置的單位移動到 to 位置
    pub fn move_unit(board: &mut Board, actor: Pos, to: Pos) -> Result<(), Error> {
        let func = "move_unit";

        if actor == to {
            return Ok(()); // 不需要移動
        }
        // to 應該在棋盤上
        let Some(tile) = board.get_tile(to) else {
            return Err(Error::NoTileAtPos { func, pos: to });
        };
        let terrain = tile.terrain;
        // 檢查 from 位置有無單位
        let unit_id = match board.pos_to_unit(actor) {
            Some(id) => id,
            None => return Err(Error::NoUnitAtPos { func, pos: actor }),
        };
        let Some(active_unit) = board.units.get(&unit_id) else {
            return Err(Error::NoUnitAtPos { func, pos: actor });
        };
        // 檢查 to 位置是否已有單位
        let result = if let Some(unit_id) = board.pos_to_unit(to) {
            let Some(target_unit) = board.units.get(&unit_id) else {
                return Err(Error::NoUnitAtPos { func, pos: to });
            };
            if active_unit.team != target_unit.team {
                return Err(Error::HostileUnitAtPos { func, pos: to });
            }
            Err(Error::AlliedUnitAtPos { func, pos: to })
        } else {
            Ok(())
        };
        let Some(active_unit) = board.units.get_mut(&unit_id) else {
            return Err(Error::NoUnitAtPos { func, pos: actor });
        };
        let cost = movement_cost(terrain);
        if active_unit.moved + cost > active_unit.move_points * 2 {
            return Err(Error::NotEnoughAP { func });
        }
        active_unit.moved += cost;
        if result.is_ok() {
            board
                .unit_map
                .move_unit(unit_id, actor, to)
                .map_err(|e| Error::Wrap {
                    func,
                    source: Box::new(e),
                })?;
        }
        result
    }

    pub fn has_hit_and_run_skill(unit: &Unit, skills_map: &BTreeMap<String, Skill>) -> bool {
        for skill_id in &unit.skills {
            if let Some(skill) = skills_map.get(skill_id) {
                if skill
                    .effects
                    .iter()
                    .any(|e| matches!(e, Effect::HitAndRun { .. }))
                {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn skills_map() -> BTreeMap<String, Skill> {
        let sprint_data = include_str!("../../tests/skill_sprint.json");
        let sprint_skill: Skill = serde_json::from_str(sprint_data).unwrap();
        let slash_data = include_str!("../../tests/skill_slash.json");
        let slash_skill: Skill = serde_json::from_str(slash_data).unwrap();
        BTreeMap::from([
            ("sprint".to_string(), sprint_skill),
            ("slash".to_string(), slash_skill),
        ])
    }

    fn basic_board_and_unit(start: Pos, ally: Pos, enemy: Pos) -> (Board, UnitID) {
        // 建立地形
        let tiles = vec![
            vec![
                Tile {
                    terrain: Terrain::Plain,
                    object: None,
                };
                3
            ],
            vec![
                Tile {
                    terrain: Terrain::Plain,
                    object: None,
                };
                3
            ],
            vec![
                Tile {
                    terrain: Terrain::Plain,
                    object: None
                };
                3
            ],
        ];

        // 載入單位 & 技能
        let data = include_str!("../../tests/unit.json");
        let v: serde_json::Value = serde_json::from_str(data).unwrap();
        let template: UnitTemplate = serde_json::from_value(v["UnitTemplate"].clone()).unwrap();
        let marker: UnitMarker = serde_json::from_value(v["UnitMarker"].clone()).unwrap();
        let team: Team = serde_json::from_value(v["Team"].clone()).unwrap();
        let teams = HashMap::from([(team.id.clone(), team.clone())]);
        let skills_map = skills_map();
        let unit_active = {
            let mut unit_active = Unit::from_template(&marker, &template, &skills_map).unwrap();
            // 避免走完所有格子
            unit_active.moved = unit_active.move_points;
            unit_active
        };
        let unit_id = unit_active.id;
        let unit_ally = {
            let mut unit_ally = Unit::from_template(&marker, &template, &skills_map).unwrap();
            unit_ally.id = unit_id + 1;
            unit_ally
        };
        let mut marker = marker;
        marker.team = team.id.clone() + "-enemy";
        let marker = marker;
        let unit_enemy = {
            let mut unit_enemy = Unit::from_template(&marker, &template, &skills_map).unwrap();
            unit_enemy.id = unit_id + 2;
            unit_enemy
        };
        let mut unit_map = UnitMap::default();
        let pos_to_unit = vec![
            (start, unit_id),
            (ally, unit_ally.id),
            (enemy, unit_enemy.id),
        ];
        for (pos, id) in pos_to_unit {
            unit_map.insert(id, pos);
        }
        let units = HashMap::from([
            (unit_id, unit_active),
            (unit_ally.id, unit_ally),
            (unit_enemy.id, unit_enemy),
        ]);

        let board = Board {
            tiles,
            teams,
            units,
            unit_map,
        };
        (board, unit_id)
    }

    #[test]
    fn test_movable_area_blocked_by_unit() {
        let start = Pos { x: 0, y: 0 };
        let ally = Pos { x: 0, y: 1 };
        let enemy = Pos { x: 1, y: 0 };
        let expect = BTreeSet::from([
            Pos { x: 0, y: 0 },
            Pos { x: 0, y: 1 },
            Pos { x: 0, y: 2 },
            Pos { x: 1, y: 1 },
            Pos { x: 1, y: 2 },
            Pos { x: 2, y: 1 },
            // 1,0 被敵人擋住
            // 2,0 因為敵人所以距離太遠
            // 2,2 距離太遠
        ]);
        let (board, _) = basic_board_and_unit(start, ally, enemy);
        let area = movable_area(&board, start, &skills_map());
        let area = area.keys().cloned().collect::<BTreeSet<_>>();
        assert_eq!(area, expect);
    }

    #[test]
    fn test_reconstruct_path() {
        let start = Pos { x: 0, y: 0 };
        let ally = Pos { x: 0, y: 1 };
        let enemy = Pos { x: 1, y: 0 };
        let (board, _) = basic_board_and_unit(start, ally, enemy);
        let area = movable_area(&board, start, &skills_map());
        let path = reconstruct_path(&area, start, Pos { x: 2, y: 1 }).unwrap();
        assert_eq!(
            path,
            vec![
                Pos { x: 0, y: 0 },
                Pos { x: 0, y: 1 },
                Pos { x: 1, y: 1 },
                Pos { x: 2, y: 1 }
            ]
        );
    }

    #[test]
    fn test_move_unit() {
        // 測試時 skills_map 取自 basic_board_and_unit 內部
        let skills_map = skills_map();

        let start = Pos { x: 0, y: 0 };
        let ally = Pos { x: 0, y: 1 };
        let enemy = Pos { x: 1, y: 0 };
        let test_data = [
            (false, vec![Pos { x: 0, y: 0 }, Pos { x: 1, y: 0 }]), // 不能經過敵人
            (
                true,
                vec![Pos { x: 0, y: 0 }, Pos { x: 0, y: 1 }, Pos { x: 0, y: 2 }],
            ),
            (
                // 不能經過敵人
                false,
                vec![Pos { x: 0, y: 0 }, Pos { x: 1, y: 0 }, Pos { x: 2, y: 0 }],
            ),
            (
                true,
                vec![
                    Pos { x: 0, y: 0 },
                    Pos { x: 0, y: 1 },
                    Pos { x: 1, y: 1 },
                    Pos { x: 2, y: 1 },
                ],
            ),
        ];
        for (is_ok, path) in test_data {
            let to = path.last().cloned().unwrap();
            let (mut board, unit_id) = basic_board_and_unit(start, ally, enemy);
            let res = move_unit_along_path(&mut board, path, &skills_map);
            assert_eq!(res.is_ok(), is_ok, "移動到 {:?} 應該成功 ? {}", to, is_ok);
            if is_ok {
                assert_eq!(board.pos_to_unit(to), Some(unit_id));
                assert!(board.pos_to_unit(start).is_none());
            }
        }
    }

    #[test]
    fn test_move_unit_hit_and_run() {
        let start = Pos { x: 0, y: 0 };
        let ally = Pos { x: 0, y: 1 };
        let enemy = Pos { x: 1, y: 0 };
        let path = vec![start, Pos { x: 0, y: 2 }];

        // 新增一個打帶跑技能
        let hit_and_run_skill = Skill {
            effects: vec![Effect::HitAndRun {
                target_type: TargetType::Caster,
                shape: Shape::Point,
                duration: -1,
            }],
            ..Default::default()
        };
        let skills_map = BTreeMap::from([("hit_and_run".to_string(), hit_and_run_skill)]);

        // 無打帶跑技能時禁止移動
        {
            let (mut board, unit_id) = basic_board_and_unit(start, ally, enemy);
            if let Some(unit) = board.units.get_mut(&unit_id) {
                unit.has_cast_skill_this_turn = true;
                unit.skills.clear();
            }

            let res = move_unit_along_path(&mut board, path.clone(), &skills_map);
            assert!(
                matches!(
                    res,
                    Err(Error::NotEnoughAP {
                        func: "move_unit_along_path"
                    })
                ),
                "無打帶跑技能時應禁止移動"
            );
        }

        // 有打帶跑技能時可移動
        {
            let (mut board, unit_id) = basic_board_and_unit(start, ally, enemy);
            if let Some(unit) = board.units.get_mut(&unit_id) {
                unit.has_cast_skill_this_turn = true;
                unit.skills.clear();
                unit.skills.insert("hit_and_run".to_string());
            }

            let res = move_unit_along_path(&mut board, path, &skills_map);
            assert!(res.is_ok(), "有打帶跑技能時應可移動");
        }
    }

    #[test]
    fn test_movement_tile_color() {
        let start = Pos { x: 0, y: 0 };
        let ally = Pos { x: 0, y: 1 };
        let enemy = Pos { x: 1, y: 0 };
        let (board, active_unit_id) = basic_board_and_unit(start, ally, enemy);
        let movable = movable_area(&board, start, &skills_map());
        let path = reconstruct_path(&movable, start, Pos { x: 0, y: 2 }).unwrap();

        // 寫死每一格的預期結果
        let expected = [
            // x=0
            [Err(()), Err(()), Ok(SECOND_PATH_COLOR)],
            // x=1
            [
                Err(()),
                Ok(SECOND_MOVEMENT_COLOR),
                Ok(SECOND_MOVEMENT_COLOR),
            ],
            // x=2
            [Err(()), Ok(SECOND_MOVEMENT_COLOR), Err(())],
        ];

        for x in 0..3 {
            for y in 0..3 {
                let pos = Pos { x, y };
                let result = movement_tile_color(&board, &movable, &active_unit_id, &path, pos);
                match expected[x][y] {
                    Ok(color) => assert_eq!(result.unwrap(), color, "({},{}) 顏色不符", x, y),
                    Err(()) => assert!(result.is_err(), "({},{}) 應回錯誤", x, y),
                }
            }
        }
    }

    #[test]
    fn test_movable_area_blocked_by_tree() {
        let start = Pos { x: 0, y: 0 };
        let ally = Pos { x: 0, y: 1 };
        let enemy = Pos { x: 1, y: 0 };
        let mut board = basic_board_and_unit(start, ally, enemy).0;

        // 在 (1,1) 放置樹木
        if let Some(tile) = board.get_tile_mut(Pos { x: 1, y: 1 }) {
            tile.object = Some(Object::Tree);
        }

        let expect = BTreeSet::from([
            Pos { x: 0, y: 0 },
            Pos { x: 0, y: 1 },
            Pos { x: 0, y: 2 },
            Pos { x: 1, y: 2 },
            // (1,1) 被樹木擋住，所以無法到達 (1,1), (2,1), (2,2)
            // 1,0 被敵人擋住
            // 2,0 因為敵人所以距離太遠
        ]);
        let area = movable_area(&board, start, &skills_map());
        let area = area.keys().cloned().collect::<BTreeSet<_>>();
        assert_eq!(area, expect);
    }
}
