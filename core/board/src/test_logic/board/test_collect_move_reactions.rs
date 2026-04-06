//! collect_move_reactions 測試

use crate::domain::alias::ID;
use crate::domain::constants::PLAYER_ALLIANCE_ID;
use crate::domain::core_types::{ReactionTrigger, SkillType, TargetFilter, TriggeringSource};
use crate::ecs_types::components::Position;
use crate::error::Result;
use crate::logic::skill::skill_reaction::{MoveReaction, ReactionUnitInfo, collect_move_reactions};
use crate::test_helpers::level_builder::{LevelBuilder, MarkerEntry};
use std::collections::HashMap;

const ALLY_ALLIANCE: ID = 1;
const ENEMY_ALLIANCE: ID = 2;

const OA11_NAME: &str = "opportunity-attack-1-1";
const OA12_NAME: &str = "opportunity-attack-1-2";
const OA22_NAME: &str = "opportunity-attack-2-2";
const OB11_NAME: &str = "opportunity-buff-1-1";

fn standard_board(
    ascii: &str,
) -> Result<(
    HashMap<String, Vec<Position>>,
    HashMap<String, Vec<MarkerEntry>>,
)> {
    let (_, markers, unit_markers) = LevelBuilder::from_ascii(ascii)
        .unit("S", "mover", PLAYER_ALLIANCE_ID)
        .unit("Ea", "enemy-reactor-a", ENEMY_ALLIANCE)
        .unit("Eb", "enemy-reactor-b", ENEMY_ALLIANCE)
        .unit("Ec", "enemy-reactor-c", ENEMY_ALLIANCE)
        .unit("Aa", "ally-reactor", ALLY_ALLIANCE)
        .to_unit_map()?;
    Ok((markers, unit_markers))
}

/// 從 unit_markers 建立 HashMap<Position, ReactionUnitInfo>
/// 只包含 reaction_configs 中有設定的 marker
fn to_reaction_map<'a>(
    unit_markers: &HashMap<String, Vec<MarkerEntry>>,
    reaction_configs: &'a HashMap<&str, ReactionConfig>,
) -> HashMap<Position, ReactionUnitInfo<'a>> {
    unit_markers
        .iter()
        .filter(|(key, _)| reaction_configs.contains_key(key.as_str()))
        .flat_map(|(key, entries)| {
            let config = reaction_configs
                .get(key.as_str())
                .expect("已經 filter 過，一定有對應的設定");
            entries.iter().map(|entry| {
                (
                    entry.position,
                    ReactionUnitInfo {
                        unit_info: entry.unit_info.clone(),
                        remaining_reactions: config.remaining_reactions,
                        skills: &config.skills,
                    },
                )
            })
        })
        .collect()
}

struct ReactionConfig {
    remaining_reactions: i32,
    skills: Vec<SkillType>,
}

fn reaction_config(remaining: i32, skills: Vec<SkillType>) -> ReactionConfig {
    ReactionConfig {
        remaining_reactions: remaining,
        skills,
    }
}

/// 建立 AttackOfOpportunity 技能
fn reaction_skill(
    name: &str,
    min_range: usize,
    max_range: usize,
    filter: TargetFilter,
) -> SkillType {
    SkillType::Reaction {
        name: name.to_string(),
        tags: Vec::new(),
        cost: 0,
        triggering_unit: TriggeringSource {
            source_range: (min_range, max_range),
            source_filter: filter,
            trigger: ReactionTrigger::AttackOfOpportunity,
        },
        effects: Vec::new(),
    }
}

/// 從 marker 名稱列表解析出 Position 路徑
fn resolve_path(markers: &HashMap<String, Vec<Position>>, names: &[&str]) -> Vec<Position> {
    names.iter().map(|name| markers[*name][0]).collect()
}

/// 驗證反應結果：不在意反應者順序，但每個反應者的技能列表需完全匹配
fn assert_reactions(
    result_reactions: &[MoveReaction],
    expected: &[(&str, &[&str])],
    unit_markers: &HashMap<String, Vec<MarkerEntry>>,
    ascii: &str,
) {
    assert_eq!(
        result_reactions.len(),
        expected.len(),
        "反應者數量不符，棋盤：{ascii}"
    );

    // 建立 expected: HashMap<Occupant, sorted skill names>
    let expected_map: HashMap<_, Vec<&str>> = expected
        .iter()
        .map(|(marker, skill_names)| {
            let occupant = unit_markers[*marker][0].unit_info.occupant;
            let mut names: Vec<&str> = skill_names.to_vec();
            names.sort();
            (occupant, names)
        })
        .collect();

    let actual_map: HashMap<_, Vec<&str>> = result_reactions
        .iter()
        .map(|r| {
            let mut names: Vec<&str> = r.skill_names.iter().map(|s| s.as_str()).collect();
            names.sort();
            (r.occupant, names)
        })
        .collect();

    assert_eq!(actual_map, expected_map, "反應列表不符，棋盤：{ascii}");
}

// ============================================================================
// 測試
// ============================================================================

#[test]
fn collect_move_reactions_cases() {
    let test_data = [
        // 無反應者：移動完成，回傳路徑終點
        (
            "
            .  .  .  S  .  .  .
            .  .  .  T  .  .  .
            .  .  .  .  .  .  .
            ",
            vec!["S", "T"],
            "T",
            vec![],
        ),
        (
            "
            .  .  .  T  .  .  .
            .  .  .  P1 .  .  .
            .  .  .  S  .  .  .
            ",
            vec!["S", "P1", "T"],
            "T",
            vec![],
        ),
        (
            "
            .  .  .  .  .  .  .
            .  S  P1 P2 T  .  .
            .  .  .  .  .  .  .
            ",
            vec!["S", "P1", "P2", "T"],
            "T",
            vec![],
        ),
        (
            "
            .  .  .  .  .  .  .
            .  T  P3 P2 P1 S  .
            .  .  .  .  .  .  .
            ",
            vec!["S", "P1", "P2", "P3", "T"],
            "T",
            vec![],
        ),
        //
        // 路徑中途觸發：敵方反應者在路徑某步 from 的 range 內
        (
            "
            .  .  S  Ec .
            .  Ea P1 .  .
            .  .  P2 .  Eb
            .  .  T  .  .
            .  .  .  .  .
            ",
            vec!["S", "P1", "P2", "T"],
            "P2",
            vec![("Ea", &[OA11_NAME] as &[&str])],
        ),
        (
            "
            .  .  T  .  .
            .  .  P2 .  .
            .  Ea P1 Eb .
            .  Ec S  .  .
            .  .  .  .  .
            ",
            vec!["S", "P1", "P2", "T"],
            "P1",
            vec![("Eb", &[OA12_NAME, OA22_NAME])],
        ),
        // 多個反應者同一步觸發
        (
            "
            .  .  .  .  .
            .  Ec Ea .  .
            S  P1 P2 T  .
            .  .  .  Eb .
            .  .  .  .  .
            ",
            vec!["S", "P1", "P2", "T"],
            "T",
            vec![
                ("Ea", &[OA11_NAME] as &[&str]),
                ("Eb", &[OA12_NAME, OA22_NAME]),
            ],
        ),
        (
            "
            .  .  .  .  .
            .  .  .  .  .
            .  .  Ea .  .
            T  P2 P1 S  .
            .  Ec .  Eb .
            ",
            vec!["S", "P1", "P2", "T"],
            "P1",
            vec![("Eb", &[OA12_NAME] as &[&str])],
        ),
        //
        // 友軍反應者
        (
            "
            Ec .  .  .  .
            S  P1 P2 T  .
            .  Aa .  .  .
            .  .  .  .  .
            .  .  .  .  .
            ",
            vec!["S", "P1", "P2", "T"],
            "P2",
            vec![("Aa", &[OB11_NAME] as &[&str])],
        ),
        (
            "
            .  Ec .  .  .
            S  P1 P2 T  .
            .  .  .  Aa .
            .  .  .  .  .
            .  .  .  .  .
            ",
            vec!["S", "P1", "P2", "T"],
            "T",
            vec![],
        ),
    ];

    let reaction_configs = HashMap::from([
        (
            "Ea",
            reaction_config(
                1,
                vec![
                    reaction_skill(OA11_NAME, 1, 1, TargetFilter::Enemy),
                    reaction_skill(OB11_NAME, 1, 1, TargetFilter::Ally),
                ],
            ),
        ),
        (
            "Eb",
            reaction_config(
                2,
                vec![
                    reaction_skill(OA12_NAME, 1, 2, TargetFilter::Enemy),
                    reaction_skill(OA22_NAME, 2, 2, TargetFilter::Enemy),
                ],
            ),
        ),
        (
            "Ec",
            reaction_config(
                0,
                vec![
                    reaction_skill(OA12_NAME, 1, 2, TargetFilter::Enemy),
                    reaction_skill(OA22_NAME, 2, 2, TargetFilter::Enemy),
                ],
            ),
        ),
        (
            "Aa",
            reaction_config(
                1,
                vec![
                    reaction_skill(OA11_NAME, 1, 1, TargetFilter::Enemy),
                    reaction_skill(OB11_NAME, 1, 1, TargetFilter::Ally),
                ],
            ),
        ),
    ]);

    for (ascii, path_names, expected_stop, expected_reactions) in &test_data {
        let (markers, unit_markers) = standard_board(ascii).expect("建立棋盤失敗");
        let mover = &unit_markers["S"][0].unit_info;
        let path = resolve_path(&markers, path_names);
        let units_on_board = to_reaction_map(&unit_markers, &reaction_configs);

        let result = collect_move_reactions(mover, &path, &units_on_board).expect("collect 失敗");

        assert_eq!(
            result.stop_position, markers[*expected_stop][0],
            "stop_position 不符，棋盤：{ascii}"
        );
        assert_reactions(&result.reactions, expected_reactions, &unit_markers, ascii);
    }
}
