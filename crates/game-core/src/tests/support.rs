use crate::*;

pub(super) const ACTOR_ID: i64 = 1;
pub(super) const TARGET_ID: i64 = 2;
pub(super) const BLOCKER_ID: i64 = 3;

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

pub(super) fn push_collision_game() -> Game {
    game_with_skill_range_and_effect(
        1,
        1,
        SkillEffect::Push {
            attack_bonus: 100,
            power_bonus: 0,
        },
    )
}

pub(super) fn game_with_skill_range_and_effect(
    min_range: i32,
    max_range: i32,
    effect: SkillEffect,
) -> Game {
    game_with_skill_on_ascii_map(
        min_range,
        max_range,
        effect,
        "
        .....
        .ATB.
        ",
    )
}

// 共用技能與移動 fixture 由 ASCII 決定起點，避免測試再手動搬動單位。
pub(super) fn game_with_skill_on_ascii_map(
    min_range: i32,
    max_range: i32,
    effect: SkillEffect,
    diagram: &str,
) -> Game {
    let terrain = TerrainTypeDef {
        id: "plain".into(),
        layer: TerrainLayer::Ground,
        entry_rule: TerrainEntryRule::Walkable,
        damage: 0,
        extra_movement_cost: 0,
        dodge_penalty: 0,
        block_penalty: 0,
    };
    let mut rough = terrain.clone();
    rough.id = "rough".into();
    let definitions = authoring::Definitions {
        equipment: Vec::new(),
        ai_profiles: vec![tank_ai_profile()],
        terrain_types: vec![terrain, rough],
        skills: vec![SkillDef {
            power_source: if matches!(&effect, SkillEffect::Heal { .. }) {
                PowerSource::Magical
            } else {
                PowerSource::Physical
            },
            id: "push".into(),
            ranged: false,
            min_range,
            max_range,
            effect,
        }],
        unit_types: vec![
            push_collision_unit_type("test_player", 100),
            push_collision_unit_type("test_unit", 0),
        ],
    };
    let map = ascii_map(
        "test_map",
        diagram,
        &[
            ('A', ACTOR_ID, "test_player", Team::Player),
            (
                'T',
                TARGET_ID,
                "test_unit",
                Team::Enemy("test_enemy".into()),
            ),
            (
                'B',
                BLOCKER_ID,
                "test_unit",
                Team::Enemy("test_enemy".into()),
            ),
        ],
    );
    match Game::from_authoring(definitions, map) {
        Ok(game) => game,
        Err(error) => panic!("測試戰鬥定義應有效：{}", error.message()),
    }
}

fn push_collision_unit_type(id: &str, initiative: i32) -> authoring::UnitType {
    authoring::UnitType {
        ai_profile: "tank".into(),
        id: id.into(),
        visual: id.into(),
        width: 1,
        height: 1,
        hp: 100,
        movement: 0,
        initiative,
        dodge: 0,
        attack: 0,
        physical_power: 1,
        magical_power: 1,
        main_hand: String::new(),
        off_hand: String::new(),
        armor: String::new(),
        accessory: String::new(),
        skills: vec!["push".into()],
    }
}

pub(super) struct AsciiBoard {
    pub board: (i32, i32),
    pub markers: HashMap<char, ((i32, i32), (i32, i32))>,
    pub terrains: Vec<TerrainPlacement>,
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
                    terrains.push(TerrainPlacement {
                        x: position.0,
                        y: position.1,
                        kind: kind.into(),
                    });
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
    let AsciiBoard {
        board,
        markers,
        terrains,
    } = ascii_board(diagram);
    let placements = units
        .iter()
        .map(|(marker, id, kind, team)| {
            let ((x, y), _) = *markers.get(marker).expect("地圖應包含案例指定的單位符號");
            authoring::UnitPlacement {
                id: *id,
                unit_type: (*kind).into(),
                team: team.clone(),
                x,
                y,
            }
        })
        .collect();
    authoring::Map {
        name: name.into(),
        width: board.0,
        height: board.1,
        terrains,
        units: placements,
    }
}
