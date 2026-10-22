//! 作者輸入的驗證與建局準備；入口共用地圖規則，執行期依賴已建立的不變量。
use crate::error::GameError;
use crate::model::{
    BattleMode, Board, Encounter, Exploration, Footprint, GridPos, Hp, Id, Log, Outcome, Phase,
    Pos, Random, ResultState, SkillDef, SkillEffect, SkillTargetKind, Skills, Team,
    TemporaryTerrains, TerrainLayer, Turn, Unit,
};
use crate::movement::footprint_cells;
use crate::skill::target_kind;
use crate::terrain::fixed_terrains_at;
use crate::{authoring, error, gameplay_config};
use bevy_ecs::prelude::World;
use std::collections::{HashMap, HashSet};

/// 不依賴單位、技能與地形定義的地圖驗證，供文件轉換與正式載入共用。
pub(super) fn validate_map(map: &authoring::Map) -> Result<(), GameError> {
    let authoring::Map {
        name,
        width,
        height,
        terrains,
        units,
        victory,
        defeat,
        cargo,
    } = map;
    if name.trim().is_empty() {
        return Err(error::empty_map_name());
    }
    if *width <= 0
        || *height <= 0
        || width.checked_mul(*height).is_none()
        || width.checked_add(*height).is_none()
    {
        return Err(error::invalid_map_dimensions());
    }
    for group in [victory, defeat] {
        if group.conditions.is_empty() {
            return Err(error::invalid_outcome_conditions());
        }
        if cargo.is_none()
            && group
                .conditions
                .iter()
                .any(|condition| matches!(condition, authoring::OutcomeCondition::CargoEscaped))
        {
            return Err(error::invalid_cargo_objective());
        }
    }
    if let Some(authoring::CargoObjective { positions, exits }) = cargo {
        let inside = |p: crate::GridPos| p.x >= 0 && p.y >= 0 && p.x < *width && p.y < *height;
        if positions.is_empty()
            || positions.iter().any(|position| !inside(*position))
            || exits.is_empty()
            || exits.iter().any(|p| {
                !inside(*p) || (p.x != 0 && p.y != 0 && p.x != *width - 1 && p.y != *height - 1)
            })
        {
            return Err(error::invalid_cargo_objective());
        }
    }
    let mut used_ids = HashSet::new();
    for unit in units {
        if unit.id <= 0 {
            return Err(error::invalid_unit_placement_id());
        }
        if !used_ids.insert(unit.id) {
            return Err(error::duplicate_unit_placement_id(unit.id));
        }
        if let Team::Enemy(faction) = &unit.team {
            if faction.trim().is_empty() {
                return Err(error::empty_enemy_faction());
            }
            if unit.min_difficulty.is_none() {
                return Err(error::missing_enemy_difficulty(unit.id));
            }
        }
    }
    if terrains.iter().any(|terrain| {
        terrain.x < 0 || terrain.y < 0 || terrain.x >= *width || terrain.y >= *height
    }) {
        return Err(error::terrain_out_of_bounds());
    }
    Ok(())
}

pub(crate) struct LoadedMap {
    pub(crate) world: World,
    pub(crate) presentation_events: Vec<crate::model::PresentationEvent>,
}

/// 定義只解析一次，供各地圖驗證與正式建局共用。
pub(crate) struct PreparedDefinitions {
    terrain_types: HashMap<String, crate::model::TerrainTypeDef>,
    skills: HashMap<String, SkillDef>,
    unit_types: HashMap<String, authoring::UnitType>,
    equipment_stats: HashMap<String, crate::equipment::EquipmentStats>,
    ai_profiles: crate::model::AiProfiles,
}

pub(crate) fn prepare_definitions(
    definitions: authoring::Definitions,
) -> Result<PreparedDefinitions, GameError> {
    let authoring::Definitions {
        equipment,
        default_passive_skills,
        ai_profiles,
        terrain_types,
        skills,
        unit_types,
    } = definitions;
    let mut terrains = HashMap::new();
    for terrain in terrain_types {
        if terrains.insert(terrain.id.clone(), terrain).is_some() {
            return Err(error::duplicate_terrain_type_id());
        }
    }
    let terrain_types = terrains;
    // 地形數值不以負值反轉成治療或加成。
    if let Some(kind) = terrain_types
        .iter()
        .filter(|(_, terrain)| {
            terrain.damage < 0 || terrain.dodge_penalty < 0 || terrain.block_penalty < 0
        })
        .map(|(kind, _)| kind)
        .min()
    {
        return Err(error::invalid_terrain_values(kind));
    }
    if terrain_types
        .get(gameplay_config::DEFAULT_GROUND_TERRAIN)
        .is_none_or(|terrain| terrain.layer != TerrainLayer::Ground)
    {
        return Err(error::missing_default_ground_terrain());
    }
    let skill_ids: HashSet<_> = skills.iter().map(|skill| skill.id.as_str()).collect();
    let ai_profiles = crate::ai::resolve_profiles(ai_profiles, &unit_types)?;
    let equipment = crate::equipment::definitions(equipment)?;
    let mut equipment_stats = HashMap::new();
    let mut types = HashMap::new();
    crate::skill::validate_passive_skills(&default_passive_skills, &skills, "全體預設")?;
    for mut kind in unit_types {
        kind.skills = crate::skill::resolve_unit_skills(&kind, &default_passive_skills, &skills)?;
        if kind.id.trim().is_empty() || kind.hp <= 0 || kind.width <= 0 || kind.height <= 0 {
            return Err(error::invalid_unit_type(&kind.id));
        }
        if let Some(unknown) = kind
            .skills
            .iter()
            .find(|skill| !skill_ids.contains(skill.as_str()))
        {
            return Err(error::unknown_unit_skill(&kind.id, unknown));
        }
        let mut unit_skills = HashSet::new();
        if let Some(duplicate) = kind
            .skills
            .iter()
            .find(|skill| !unit_skills.insert(skill.as_str()))
        {
            return Err(error::duplicate_unit_skill(&kind.id, duplicate));
        }
        equipment_stats.insert(
            kind.id.clone(),
            crate::equipment::resolve(&kind, &equipment)?,
        );
        if types.insert(kind.id.clone(), kind).is_some() {
            return Err(error::duplicate_unit_type_id());
        }
    }
    let mut skill_definitions = HashMap::new();
    for skill in skills {
        if matches!(skill.effect, SkillEffect::Flanking { hit_bonus } if hit_bonus < 0) {
            return Err(error::invalid_passive_skills(&skill.id));
        }
        if skill.min_range < 0 || skill.max_range < skill.min_range {
            return Err(error::invalid_skill_range(&skill.id));
        }
        if let SkillEffect::Mire { terrain, duration } = &skill.effect {
            if *duration == 0 {
                return Err(error::invalid_duration(&skill.id));
            }
            let definition = terrain_types
                .get(terrain)
                .ok_or_else(|| error::invalid_skill_terrain(&skill.id))?;
            // 暫時地形疊在既有地形上，不能是 ground，否則同格會有兩個 ground。
            if definition.layer != TerrainLayer::Overlay {
                return Err(error::skill_terrain_not_overlay(&skill.id));
            }
        }
        if skill_definitions.insert(skill.id.clone(), skill).is_some() {
            return Err(error::duplicate_skill_id());
        }
    }
    Ok(PreparedDefinitions {
        terrain_types,
        skills: skill_definitions,
        unit_types: types,
        equipment_stats,
        ai_profiles,
    })
}

/// 驗證完整配置並整理固定地形；不建立 ECS world，也不依難度略過驗證。
pub(crate) fn validate_map_with_definitions(
    definitions: &PreparedDefinitions,
    map: &authoring::Map,
) -> Result<HashMap<GridPos, Vec<String>>, GameError> {
    validate_map(map)?;
    let PreparedDefinitions {
        terrain_types,
        skills,
        unit_types: types,
        equipment_stats,
        ai_profiles: _,
    } = definitions;
    let authoring::Map {
        name: _,
        width,
        height,
        terrains: terrain_placements,
        units: placements,
        victory: _,
        defeat: _,
        cargo: _,
    } = map;
    for placement in placements {
        let kind = types
            .get(&placement.unit_type)
            .ok_or_else(|| error::unknown_unit_type(&placement.unit_type))?;
        // 敵方 AI 依技能決定行動，沒有技能就無法推進回合。
        if matches!(placement.team, Team::Enemy(_))
            && !kind.skills.iter().any(|id| {
                skills
                    .get(id)
                    .is_some_and(|skill| target_kind(&skill.effect) != SkillTargetKind::Passive)
            })
        {
            return Err(error::missing_ai_skill(placement.id));
        }
    }
    validate_numeric_ranges(
        terrain_types,
        skills,
        types,
        equipment_stats,
        *width,
        *height,
    )?;
    for terrain in terrain_placements {
        if !terrain_types.contains_key(&terrain.kind) {
            return Err(error::unknown_terrain_type(&terrain.kind));
        }
    }
    let mut terrain_positions = HashSet::new();
    let mut ground_positions = HashSet::new();
    for terrain in terrain_placements {
        if !terrain_positions.insert((terrain.x, terrain.y, terrain.kind.as_str())) {
            return Err(error::duplicate_terrain(terrain.x, terrain.y));
        }
        if terrain_types[&terrain.kind].layer == TerrainLayer::Ground
            && !ground_positions.insert((terrain.x, terrain.y))
        {
            return Err(error::multiple_ground_terrains(terrain.x, terrain.y));
        }
    }
    let mut terrains: HashMap<GridPos, Vec<String>> = HashMap::new();
    for terrain in terrain_placements {
        terrains
            .entry(GridPos {
                x: terrain.x,
                y: terrain.y,
            })
            .or_default()
            .push(terrain.kind.clone());
    }
    for kinds in terrains.values_mut() {
        kinds.sort();
    }
    let mut occupied = HashSet::new();
    for placement in placements {
        let authoring::UnitPlacement {
            facing: _,
            id,
            min_difficulty: _,
            unit_type,
            team: _,
            x,
            y,
        } = placement;
        let authoring::UnitType {
            id: _,
            ai_profile: _,
            visual: _,
            width: unit_width,
            height: unit_height,
            hp: _,
            movement: _,
            initiative: _,
            dodge: _,
            hit: _,
            physical_power: _,
            magical_power: _,
            main_hand: _,
            off_hand: _,
            armor: _,
            accessory: _,
            skills: _,
            passive_skills: _,
        } = types.get(unit_type).expect("單位配置的類型已在上方驗證");
        if *x < 0
            || *y < 0
            || x.checked_add(*unit_width)
                .is_none_or(|right| right > *width)
            || y.checked_add(*unit_height)
                .is_none_or(|bottom| bottom > *height)
        {
            return Err(error::unit_out_of_bounds(*id));
        }
        let cells = footprint_cells(
            GridPos { x: *x, y: *y },
            Footprint {
                width: *unit_width,
                height: *unit_height,
            },
        );
        if cells.iter().any(|cell| {
            fixed_terrains_at(terrain_types, &terrains, *cell)
                .iter()
                .any(|kind| terrain_types[*kind].entry_rule != crate::TerrainEntryRule::Walkable)
        }) {
            return Err(error::unit_on_impassable(*id));
        }
        if cells.iter().any(|cell| !occupied.insert(*cell)) {
            return Err(error::overlapping_unit(*id));
        }
    }
    Ok(terrains)
}

/// 驗證完成後才建立世界並依難度生成單位。
pub(crate) fn load_map(
    definitions: authoring::Definitions,
    map: authoring::Map,
    difficulty: authoring::Difficulty,
) -> Result<LoadedMap, GameError> {
    let definitions = prepare_definitions(definitions)?;
    let terrains = validate_map_with_definitions(&definitions, &map)?;
    let PreparedDefinitions {
        terrain_types,
        skills: skill_definitions,
        unit_types: types,
        equipment_stats,
        ai_profiles,
    } = definitions;
    let authoring::Map {
        name: _,
        width,
        height,
        terrains: _,
        units: placements,
        victory,
        defeat,
        cargo,
    } = map;
    let mut w = World::new();
    w.insert_resource(Board {
        width,
        height,
        terrains,
        terrain_types,
    });
    w.insert_resource(crate::model::OutcomeRules { victory, defeat });
    if let Some(authoring::CargoObjective { positions, exits }) = cargo {
        w.insert_resource(crate::model::Cargo {
            items: positions
                .into_iter()
                .map(crate::model::CargoState::Ground)
                .collect(),
            exits,
        });
    }
    w.insert_resource(ai_profiles);
    w.insert_resource(Encounter::default());
    w.insert_resource(Exploration {
        mode: BattleMode::Exploring,
        turns: HashMap::new(),
    });
    w.insert_resource(TemporaryTerrains::default());
    w.insert_resource(Turn {
        actor: None,
        phase: Phase::Ended,
        movement_remaining: 0,
        movement_segments_used: 0,
    });
    w.insert_resource(Random(0xc0ffee));
    w.insert_resource(Log::default());
    w.insert_resource(ResultState(Outcome::Ongoing));
    w.insert_resource(Skills {
        definitions: skill_definitions,
    });
    let mut presentation_events = Vec::new();
    for placement in placements {
        let authoring::UnitPlacement {
            facing,
            id,
            min_difficulty,
            unit_type,
            team,
            x,
            y,
        } = placement;
        let authoring::UnitType {
            id: _,
            ai_profile: _,
            visual,
            width,
            height,
            hp: _,
            movement,
            initiative,
            dodge,
            hit,
            physical_power: _,
            magical_power: _,
            main_hand: _,
            off_hand: _,
            armor: _,
            accessory: _,
            skills,
            passive_skills: _,
        } = types
            .get(&unit_type)
            .expect("單位配置的類型已在上方驗證")
            .clone();
        let crate::equipment::EquipmentStats {
            hp,
            physical_power,
            magical_power,
            block,
            block_reduction,
            equipment,
        } = equipment_stats.get(&unit_type).expect("配裝已驗證").clone();
        let f = Footprint { width, height };
        let p = GridPos { x, y };
        if matches!(team, Team::Enemy(_))
            && min_difficulty.expect("作者輸入邊界已確認每隻怪物都有出現難度") > difficulty
        {
            continue;
        }
        presentation_events.push(crate::model::PresentationEvent::UnitFaced {
            unit_id: id,
            facing,
        });
        w.spawn((
            Id(id),
            Pos(p),
            f,
            Hp {
                current: hp,
                maximum: hp,
            },
            Unit {
                unit_type,
                visual,
                team,
                movement,
                initiative,
                dodge,
                block,
                hit,
                physical_power,
                magical_power,
                block_reduction,
                equipment,
                skills,
            },
        ));
    }
    Ok(LoadedMap {
        world: w,
        presentation_events,
    })
}

/// 載入時建立座標、地形疊加與戰鬥數值可安全運算的不變量。
fn validate_numeric_ranges(
    terrains: &HashMap<String, crate::model::TerrainTypeDef>,
    skills: &HashMap<String, SkillDef>,
    units: &HashMap<String, authoring::UnitType>,
    equipment_stats: &HashMap<String, crate::equipment::EquipmentStats>,
    width: i32,
    height: i32,
) -> Result<(), GameError> {
    // 同格最多一種 ground，overlay 與技能建立的地形都按種類去重。
    let maximum_cell_value = |value: fn(&crate::model::TerrainTypeDef) -> i128| {
        let ground = terrains
            .values()
            .filter(|terrain| terrain.layer == TerrainLayer::Ground)
            .map(value)
            .max()
            .unwrap_or(0);
        ground
            + terrains
                .values()
                .filter(|terrain| terrain.layer == TerrainLayer::Overlay)
                .map(value)
                .sum::<i128>()
    };
    let cost = 1 + maximum_cell_value(|terrain| i128::from(terrain.extra_movement_cost));
    let cells = i128::from(width) * i128::from(height);
    if cost * (cells + 1) > i128::from(u32::MAX)
        || maximum_cell_value(|terrain| i128::from(terrain.damage)) > i128::from(i32::MAX)
        || maximum_cell_value(|terrain| i128::from(terrain.dodge_penalty)) > i128::from(i32::MAX)
        || maximum_cell_value(|terrain| i128::from(terrain.block_penalty)) > i128::from(i32::MAX)
    {
        return Err(error::numeric_range("terrain_types"));
    }
    let maximum_hp = equipment_stats
        .values()
        .map(|unit| unit.hp)
        .max()
        .unwrap_or(0);
    let dodge_penalty = maximum_cell_value(|terrain| i128::from(terrain.dodge_penalty));
    let block_penalty = maximum_cell_value(|terrain| i128::from(terrain.block_penalty));
    for unit in units.values() {
        let stats = equipment_stats.get(&unit.id).expect("配裝已驗證");
        if i128::from(unit.dodge) - dodge_penalty < i128::from(i32::MIN)
            || i128::from(stats.block) - block_penalty < i128::from(i32::MIN)
            || i128::from(unit.movement) * 2 + cost > i128::from(u32::MAX)
            || width.checked_add(unit.width).is_none()
            || height.checked_add(unit.height).is_none()
            || unit
                .initiative
                .checked_add(gameplay_config::INITIATIVE_DIE_SIDES as i32)
                .is_none()
            || i64::from(unit.dodge.max(0)) + i64::from(stats.block.max(0)) > i64::from(i32::MAX)
        {
            return Err(error::numeric_range(&unit.id));
        }
        for skill in skills
            .values()
            .filter(|skill| unit.skills.contains(&skill.id))
        {
            let (hit_bonus, power_bonus) = match skill.effect {
                SkillEffect::Attack {
                    hit_bonus,
                    power_bonus,
                } => (hit_bonus, Some(power_bonus)),
                SkillEffect::Push { hit_bonus } => (hit_bonus, None),
                SkillEffect::Heal { power_bonus } => (0, Some(power_bonus)),
                SkillEffect::Mire { .. } | SkillEffect::Flanking { .. } | SkillEffect::Loot => {
                    continue;
                }
            };
            let hit = i64::from(unit.hit) + i64::from(hit_bonus);
            let power = power_bonus.map_or(0, |bonus| {
                i64::from(match skill.power_source {
                    crate::PowerSource::Physical => stats.physical_power,
                    crate::PowerSource::Magical => stats.magical_power,
                }) + i64::from(bonus)
            });
            if hit < i64::from(i32::MIN)
                || hit
                    + i64::from(crate::skill::flanking_hit_bonus(
                        unit.skills.iter().filter_map(|id| skills.get(id)),
                    ))
                    + i64::from(gameplay_config::HIT_DIE_SIDES)
                    > i64::from(i32::MAX)
                || power < i64::from(i32::MIN)
                || power.max(0) * 2 > i64::from(i32::MAX)
                || matches!(skill.effect, SkillEffect::Heal { .. })
                    && power.max(0) + i64::from(maximum_hp) > i64::from(i32::MAX)
            {
                return Err(error::numeric_range(&unit.id));
            }
        }
    }
    Ok(())
}
