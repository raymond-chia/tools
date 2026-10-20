use game_core::{
    Command, Game, GridPos, Phase, PowerSource, SkillDef, SkillEffect, Snapshot, Team,
    TerrainEntryRule, TerrainLayer, TerrainPlacement, TerrainTypeDef, UnitView, authoring,
};
use std::collections::HashMap;

pub(super) const ACTOR_ID: i64 = 1;
pub(super) const TARGET_ID: i64 = 2;
pub(super) const BLOCKER_ID: i64 = 3;
pub(super) const TEST_SKILL_ID: &str = "test_skill";

pub(super) fn tank_ai_profile() -> authoring::AiProfile {
    authoring::AiProfile {
        id: "tank".into(),
        distance_preference: authoring::DistancePreference::Near,
        damage_weight: 10,
        healing_weight: 0,
        positioning_weight: 10,
        hit_weight: 0,
        pursuit_weight: 0,
    }
}

// 單技能測試建局資料只透過公開作者格式建立，可供單元與外部整合測試共用。
pub(super) fn skill_test_data(
    min_range: i32,
    max_range: i32,
    effect: SkillEffect,
    target_team: Team,
    diagram: &str,
) -> (authoring::Definitions, authoring::Map) {
    let terrain = serde_json::from_value::<TerrainTypeDef>(serde_json::json!({
        "blocks_sight": false,
        "id": "plain",
        "layer": TerrainLayer::Ground,
        "entry_rule": TerrainEntryRule::Walkable,
        "damage": 0,
        "extra_movement_cost": 0,
        "dodge_penalty": 0,
        "block_penalty": 0
    }))
    .expect("測試地形資料應可載入");
    let layout = ascii_board(diagram);
    let units: Vec<_> = [
        ('A', ACTOR_ID, "test_actor", Team::Player),
        ('T', TARGET_ID, "test_target", target_team),
        (
            'B',
            BLOCKER_ID,
            "test_blocker",
            Team::Enemy("test_enemy".into()),
        ),
    ]
    .into_iter()
    .filter(|(marker, _, _, _)| layout.markers.contains_key(marker))
    .collect();
    let unit_types = units
        .iter()
        .map(|(marker, _, kind, _)| {
            let (_, size) = layout.markers[marker];
            test_unit_type(kind, if *marker == 'A' { 100 } else { 0 }, size)
        })
        .collect();
    let power_source = if matches!(&effect, SkillEffect::Heal { .. }) {
        PowerSource::Magical
    } else {
        PowerSource::Physical
    };
    let mut skill = serde_json::to_value(effect).expect("測試技能效果應可序列化");
    skill["id"] = TEST_SKILL_ID.into();
    skill["power_source"] = serde_json::to_value(power_source).expect("威力來源應可序列化");
    skill["ranged"] = false.into();
    skill["min_range"] = min_range.into();
    skill["max_range"] = max_range.into();
    let skill: SkillDef = serde_json::from_value(skill).expect("測試技能資料應可載入");
    let definitions = authoring::Definitions {
        equipment: Vec::new(),
        default_passive_skills: Vec::new(),
        ai_profiles: vec![tank_ai_profile()],
        terrain_types: vec![
            terrain,
            serde_json::from_value::<TerrainTypeDef>(serde_json::json!({
                "id": "wall",
                "layer": TerrainLayer::Overlay,
                "entry_rule": TerrainEntryRule::Blocked,
                "blocks_sight": true,
                "damage": 0,
                "extra_movement_cost": 0,
                "dodge_penalty": 0,
                "block_penalty": 0
            }))
            .expect("測試地形資料應可載入"),
        ],
        skills: vec![skill],
        unit_types,
    };
    let map = map_from_ascii_board("test_map", &layout, &units);
    (definitions, map)
}

fn test_unit_type(id: &str, initiative: i32, (width, height): (i32, i32)) -> authoring::UnitType {
    authoring::UnitType {
        ai_profile: "tank".into(),
        id: id.into(),
        visual: id.into(),
        width,
        height,
        hp: 100,
        movement: 0,
        initiative,
        dodge: 0,
        hit: 0,
        physical_power: 1,
        magical_power: 1,
        main_hand: String::new(),
        off_hand: String::new(),
        armor: String::new(),
        accessory: String::new(),
        skills: vec![TEST_SKILL_ID.into()],
        passive_skills: Vec::new(),
    }
}

pub(super) struct AsciiBoard {
    pub board: (i32, i32),
    pub markers: HashMap<char, ((i32, i32), (i32, i32))>,
    pub terrains: Vec<TerrainPlacement>,
}

// 從棋盤符號取得左上角座標，單格目的地與多格單位共用同一入口。
pub(super) fn marked_cell(layout: &AsciiBoard, marker: char) -> GridPos {
    let ((x, y), _) = layout.markers.get(&marker).expect("棋盤應包含指定符號");
    GridPos { x: *x, y: *y }
}

// 結束目前行動並推進一次；已結束的回合只推進，不執行 AI。
pub(super) fn end_turn_and_continue(game: &mut Game) -> Snapshot {
    let snapshot = game.snapshot(None);
    if snapshot.turn.phase != Phase::Ended {
        game.command(Command::EndTurn {
            actor: snapshot.turn.actor.expect("進行中的戰鬥應有行動者"),
        })
        .expect("目前單位應可結束回合");
    }
    game.command(Command::Continue)
        .expect("結束回合後應可推進戰鬥")
}

// 共用 ASCII 棋盤：大寫字母標示單位或目的地，同字母填滿矩形表示佔用範圍；# 牆、^ 尖刺、~ 粗糙地面、. 空地。
pub(super) fn ascii_board(diagram: &str) -> AsciiBoard {
    let rows: Vec<_> = diagram.trim().lines().map(str::trim).collect();
    let width = rows[0].len();
    let mut marker_cells: HashMap<char, Vec<(i32, i32)>> = HashMap::new();
    let mut terrains = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        assert_eq!(row.len(), width, "棋盤每列必須等寬");
        for (x, cell) in row.chars().enumerate() {
            let position = (x as i32, y as i32);
            match cell {
                'A'..='Z' => marker_cells.entry(cell).or_default().push(position),
                '#' | '^' | '~' => {
                    let kind = match cell {
                        '#' => "wall",
                        '^' => "spikes",
                        '~' => "rough",
                        _ => unreachable!("已限定地形符號"),
                    };
                    terrains.push(
                        serde_json::from_value::<TerrainPlacement>(serde_json::json!({
                            "x": position.0,
                            "y": position.1,
                            "kind": kind
                        }))
                        .expect("測試地形資料應可載入"),
                    );
                }
                '.' => {}
                _ => panic!("未知棋盤符號：{cell}"),
            }
        }
    }
    let bounds = |cells: &[(i32, i32)]| {
        let min_x = cells
            .iter()
            .map(|cell| cell.0)
            .min()
            .expect("棋盤應包含單位");
        let min_y = cells
            .iter()
            .map(|cell| cell.1)
            .min()
            .expect("棋盤應包含單位");
        let max_x = cells
            .iter()
            .map(|cell| cell.0)
            .max()
            .expect("棋盤應包含單位");
        let max_y = cells
            .iter()
            .map(|cell| cell.1)
            .max()
            .expect("棋盤應包含單位");
        let size = (max_x - min_x + 1, max_y - min_y + 1);
        assert_eq!(
            cells.len(),
            (size.0 * size.1) as usize,
            "單位佔用格必須填滿矩形"
        );
        ((min_x, min_y), size)
    };
    let markers = marker_cells
        .into_iter()
        .map(|(marker, cells)| (marker, bounds(&cells)))
        .collect();
    AsciiBoard {
        board: (width as i32, rows.len() as i32),
        markers,
        terrains,
    }
}

// 將共用棋盤解析結果轉成作者地圖；單位種類及陣營由案例提供。
pub(super) fn ascii_map(
    name: &str,
    diagram: &str,
    units: &[(char, i64, &str, Team)],
) -> authoring::Map {
    map_from_ascii_board(name, &ascii_board(diagram), units)
}

pub(super) fn map_from_ascii_board(
    name: &str,
    layout: &AsciiBoard,
    units: &[(char, i64, &str, Team)],
) -> authoring::Map {
    let AsciiBoard {
        board,
        markers,
        terrains,
    } = layout;
    let placements = units
        .iter()
        .map(|(marker, id, kind, team)| {
            let ((x, y), _) = *markers.get(marker).expect("地圖應包含案例指定的單位符號");
            authoring::UnitPlacement {
                facing: authoring::Facing::Right,
                id: *id,
                min_difficulty: if matches!(team, Team::Enemy(_)) {
                    Some(authoring::Difficulty::Easy)
                } else {
                    None
                },
                unit_type: (*kind).into(),
                team: team.clone(),
                x,
                y,
            }
        })
        .collect();
    authoring::Map {
        victory: authoring::OutcomeGroup {
            mode: authoring::ConditionMode::All,
            conditions: vec![authoring::OutcomeCondition::AllEnemiesDefeated],
        },
        defeat: authoring::OutcomeGroup {
            mode: authoring::ConditionMode::Any,
            conditions: vec![authoring::OutcomeCondition::AllPlayersDefeated],
        },
        cargo: None,
        name: name.into(),
        width: board.0,
        height: board.1,
        terrains: terrains.clone(),
        units: placements,
    }
}

// 從公開快照查詢單位，避免讓行為測試依賴 ECS 元件。
pub(super) fn unit(snapshot: &game_core::Snapshot, id: i64) -> &UnitView {
    snapshot
        .units
        .iter()
        .find(|unit| unit.id == id)
        .expect("快照應包含測試單位")
}

// 驗證公開快照的當前與後續行動者；在遭遇開始或新一輪開始時涵蓋整批參戰者。
pub(super) fn assert_turn_order(game: &Game, markers: &[(char, i64)], expected: &str, name: &str) {
    let snapshot = game.snapshot(None);
    let mut actual: Vec<_> = snapshot
        .turn
        .actor
        .iter()
        .chain(snapshot.turn_order.iter())
        .map(|id| {
            markers
                .iter()
                .find(|(_, unit_id)| unit_id == id)
                .expect("行動單位應有對應的地圖代號")
                .0
        })
        .collect();
    actual.sort();
    let mut expected: Vec<_> = expected.chars().collect();
    expected.sort();
    assert_eq!(actual, expected, "{name}");
}

pub(super) fn sight_game_in_range(
    diagram: &str,
    effect: SkillEffect,
    target_team: Team,
    min_range: i32,
    max_range: i32,
) -> Game {
    let needs_enemy = target_team == Team::Player;
    let (definitions, mut map) =
        skill_test_data(min_range, max_range, effect, target_team, diagram);
    // 治療案例也保留敵人，避免公開命令因已獲勝而略過施放。
    if needs_enemy {
        map.units.push(authoring::UnitPlacement {
            facing: authoring::Facing::Right,
            id: BLOCKER_ID,
            min_difficulty: Some(authoring::Difficulty::Easy),
            unit_type: "test_target".into(),
            team: Team::Enemy("test_enemy".into()),
            x: map.width,
            y: 0,
        });
        map.width += 1;
    }
    Game::from_authoring(definitions, map, game_core::authoring::Difficulty::Easy)
        .expect("視線測試應可載入")
}
