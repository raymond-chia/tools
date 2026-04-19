//! 技能系統 ECS 操作函數

use super::{get_component, get_component_mut};
use crate::domain::alias::{ID, SkillName};
use crate::domain::constants::IMPASSABLE_MOVEMENT_COST;
use crate::domain::core_types::{SkillType, TargetSelection};
use crate::ecs_logic::query::{
    build_faction_alliance_map, find_entity_by_occupant, get_active_skill_data, get_resource,
    get_resource_mut, read_attribute_bundle, resolve_alliance,
};
use crate::ecs_logic::turn::get_current_unit;
use crate::ecs_types::components::{
    ActionState, ContactEffects, CurrentHp, CurrentMp, MovementPoint, Object, ObjectBundle,
    ObjectMovementCost, Occupant, OccupantTypeName, Position, Skills, Unit, UnitFaction,
};
use crate::ecs_types::resources::{Board, GameData, SkillTargeting, TurnOrder};
use crate::error::{BoardError, Result, UnitError};
use crate::logic::id_generator::generate_unique_id;
use crate::logic::skill::skill_execution::{
    CheckTarget, CombatStats, EffectEntry, ObjectOnBoard, ResolvedEffect, resolve_effect_tree,
};
use crate::logic::skill::skill_range::{compute_affected_positions, compute_range_positions};
use crate::logic::skill::skill_target::{validate_filter, validate_skill_targets};
use crate::logic::skill::{CasterInfo, UnitInfo, is_in_filter, manhattan_distance};
use bevy_ecs::prelude::{Entity, With, World};
use rand::RngExt;
use std::collections::{HashMap, HashSet};

/// 可用技能資訊
pub struct AvailableSkill {
    pub name: SkillName,
    pub usable: bool,
}

/// 查詢當前單位是否可使用技能（行動點足夠才可使用）
pub fn can_use_skill_current_unit(world: &mut World) -> Result<bool> {
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_current_unit(turn_order)?;

    let entity = find_entity_by_occupant(world, active_occupant)?;
    let entity_ref = world.entity(entity);
    let action_state = get_component!(entity_ref, ActionState)?;
    let movement_point = get_component!(entity_ref, MovementPoint)?.0;

    Ok(check_action_point(action_state, movement_point).is_ok())
}

/// 取得當前行動單位的所有主動技能及其可用狀態
pub fn get_available_skills(world: &mut World) -> Result<Vec<AvailableSkill>> {
    // 讀取：TurnOrder → active unit
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_current_unit(turn_order)?;

    // 讀取：當前單位的 Skills、CurrentMp、ActionState、MovementPoint
    let entity = find_entity_by_occupant(world, active_occupant)?;
    let entity_ref = world.entity(entity);
    let skills = get_component!(entity_ref, Skills)?;
    let current_mp = get_component!(entity_ref, CurrentMp)?.0;
    let action_state = get_component!(entity_ref, ActionState)?;
    let movement_point = get_component!(entity_ref, MovementPoint)?.0;

    // 讀取：GameData
    let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;

    // 純邏輯：篩選 Active 技能，判定 usable
    let can_act = match action_state {
        ActionState::Done => false,
        ActionState::Moved { cost } => (*cost as i32) <= movement_point,
    };

    let mut result = Vec::new();
    for skill_name in &skills.0 {
        let skill_type =
            game_data
                .skill_map
                .get(skill_name)
                .ok_or_else(|| UnitError::SkillNotFound {
                    skill_name: skill_name.clone(),
                })?;
        match skill_type {
            SkillType::Active { name, cost, .. } => {
                let usable = can_act && current_mp >= *cost as i32;
                result.push(AvailableSkill {
                    name: name.clone(),
                    usable,
                });
            }
            SkillType::Reaction { .. } | SkillType::Passive { .. } => {}
        }
    }

    Ok(result)
}

/// 查詢指定技能的射程內所有可選格子
///
/// 根據技能的 Target.range 與施放者位置，回傳曼哈頓距離在射程內的所有格子
pub fn get_skill_targetable_positions(
    world: &mut World,
    skill_name: &SkillName,
) -> Result<Vec<Position>> {
    // 讀取：TurnOrder → active unit
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_current_unit(turn_order)?;

    // 讀取：當前單位的位置
    let caster_pos = {
        let entity = find_entity_by_occupant(world, active_occupant)?;
        *get_component!(world.entity(entity), Position)?
    };

    // 讀取：GameData
    let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;

    // 讀取：Board
    let board = get_resource::<Board>(world, "請先呼叫 spawn_level")?;

    // 純邏輯：取得技能的 range，計算射程內格子
    let (target, _, _) = get_active_skill_data(game_data, skill_name)?;
    Ok(compute_range_positions(caster_pos, target.range, *board))
}

/// 預覽技能 AOE 影響範圍結果
pub struct PreviewAffectedPositions {
    /// AOE 範圍內所有格子（不管 filter）
    pub all_positions: Vec<Position>,
    /// 過濾後的格子
    pub filtered_positions: Vec<Position>,
}

/// 預覽技能 AOE 影響範圍
///
/// 根據技能的 Area、施放者位置與目標位置，計算影響的所有格子。
/// 同時驗證目標位置在技能射程內。
/// 回傳未過濾和過濾後兩組結果。
pub fn get_skill_affected_positions(
    world: &mut World,
    skill_name: &SkillName,
    target_pos: Position,
) -> Result<PreviewAffectedPositions> {
    // 讀取
    let board = *get_resource::<Board>(world, "請先呼叫 spawn_level")?;

    let target = {
        let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;
        let (target, _, _) = get_active_skill_data(game_data, skill_name)?;
        target.clone()
    };

    let faction_to_alliance = build_faction_alliance_map(world)?;

    // 建立 caster unit info
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_current_unit(turn_order)?;
    let (caster_pos, caster_occupant, caster_faction) = {
        let entity = find_entity_by_occupant(world, active_occupant)?;
        let entity_ref = world.entity(entity);
        let pos = *get_component!(entity_ref, Position)?;
        let occupant = *get_component!(entity_ref, Occupant)?;
        let faction = get_component!(entity_ref, UnitFaction)?.0;
        (pos, occupant, faction)
    };
    let caster_alliance = resolve_alliance(&faction_to_alliance, caster_faction)?;
    let caster_info = UnitInfo {
        occupant: caster_occupant,
        faction_id: caster_faction,
        alliance_id: caster_alliance,
    };

    // 建立場上單位 position → UnitInfo
    let units_on_board = {
        let mut units_on_board: HashMap<Position, UnitInfo> = HashMap::new();
        for (pos, occupant, faction_id) in world
            .query_filtered::<(&Position, &Occupant, &UnitFaction), With<Unit>>()
            .iter(world)
            .map(|(pos, occ, fac)| (*pos, *occ, fac.0))
        {
            let alliance_id = resolve_alliance(&faction_to_alliance, faction_id)?;
            units_on_board.insert(
                pos,
                UnitInfo {
                    occupant,
                    faction_id,
                    alliance_id,
                },
            );
        }
        units_on_board
    };

    // ============================================================================

    // 計算 AOE 範圍
    let (min_range, max_range) = target.range;
    let distance = manhattan_distance(caster_pos, target_pos);
    if distance < min_range || distance > max_range {
        return Err(BoardError::OutOfRange {
            distance,
            min_range,
            max_range,
        }
        .into());
    }
    let all_positions = compute_affected_positions(&target.area, caster_pos, target_pos, board)?;

    let can_target_ground = matches!(target.selection, TargetSelection::Ground);
    let filter = target.selectable_filter;
    let filtered_positions = all_positions
        .iter()
        .filter(|pos| match units_on_board.get(pos) {
            None => can_target_ground,
            Some(unit) => is_in_filter(&caster_info, unit, filter),
        })
        .copied()
        .collect();

    Ok(PreviewAffectedPositions {
        all_positions,
        filtered_positions,
    })
}

/// 開始技能選目標流程：驗證技能存在且當前單位可用，建立空的 SkillTargeting resource
pub fn start_skill_targeting(world: &mut World, skill_name: &SkillName) -> Result<()> {
    // 讀取：TurnOrder → active unit
    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_current_unit(turn_order)?;

    // 讀取：當前單位的 Skills、CurrentMp、ActionState、MovementPoint
    let entity = find_entity_by_occupant(world, active_occupant)?;
    let entity_ref = world.entity(entity);
    let has_skill = get_component!(entity_ref, Skills)?.0.contains(skill_name);
    let current_mp = get_component!(entity_ref, CurrentMp)?.0;
    let action_state = get_component!(entity_ref, ActionState)?.clone();
    let movement_point = get_component!(entity_ref, MovementPoint)?.0;

    // 讀取：GameData → 取得 Active 技能資料（不存在或非 Active 皆回 SkillNotFound）
    let (cost, max_count) = {
        let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;
        if !has_skill {
            return Err(UnitError::SkillNotFound {
                skill_name: skill_name.clone(),
            }
            .into());
        }
        let (target, _, cost) = get_active_skill_data(game_data, skill_name)?;
        (cost, target.count)
    };

    // 檢查行動點與 MP
    check_action_point(&action_state, movement_point)?;
    if current_mp < cost as i32 {
        return Err(UnitError::InsufficientMp {
            cost,
            current: current_mp,
        }
        .into());
    }

    world.insert_resource(SkillTargeting {
        skill_name: skill_name.clone(),
        picked: Vec::new(),
        max_count,
    });
    Ok(())
}

/// 新增一個目標位置到 SkillTargeting
///
/// - 驗證位置在當前技能的 targetable 範圍內
/// - `allow_same_target=false` 且位置已存在 → 忽略（回 Ok，不 push）
/// - 已滿 count → 回 `TargetCountFull` 錯誤
/// - 其餘情況 push
pub fn add_skill_target(world: &mut World, pos: Position) -> Result<()> {
    let skill_name = get_resource::<SkillTargeting>(world, "請先呼叫 start_skill_targeting")?
        .skill_name
        .clone();

    let targetable = get_skill_targetable_positions(world, &skill_name)?;

    let (count, allow_same_target, min_range, max_range, selection, filter) = {
        let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;
        let (target, _, _) = get_active_skill_data(game_data, &skill_name)?;
        (
            target.count,
            target.allow_same_target,
            target.range.0,
            target.range.1,
            target.selection.clone(),
            target.selectable_filter,
        )
    };

    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_current_unit(turn_order)?;

    let faction_to_alliance = build_faction_alliance_map(world)?;

    let mut units_on_board: HashMap<Position, UnitInfo> = HashMap::new();
    for (unit_pos, occupant, faction_id) in world
        .query_filtered::<(&Position, &Occupant, &UnitFaction), With<Unit>>()
        .iter(world)
        .map(|(p, occ, fac)| (*p, *occ, fac.0))
    {
        let alliance_id = resolve_alliance(&faction_to_alliance, faction_id)?;
        units_on_board.insert(
            unit_pos,
            UnitInfo {
                occupant,
                faction_id,
                alliance_id,
            },
        );
    }

    let caster_entity = find_entity_by_occupant(world, active_occupant)?;
    let caster_pos = {
        let entity_ref = world.entity(caster_entity);
        *get_component!(entity_ref, Position)?
    };
    let caster_info = units_on_board
        .get(&caster_pos)
        .ok_or(BoardError::NoActiveUnit)?
        .clone();

    // ========================================================================
    // 純邏輯階段
    // ========================================================================

    if !targetable.contains(&pos) {
        let distance = manhattan_distance(caster_pos, pos);
        return Err(BoardError::OutOfRange {
            distance,
            min_range,
            max_range,
        }
        .into());
    }

    if matches!(selection, TargetSelection::Unit) {
        match units_on_board.get(&pos) {
            None => {
                return Err(BoardError::NoUnitAtTarget { x: pos.x, y: pos.y }.into());
            }
            Some(target_unit) => {
                let caster = CasterInfo {
                    position: caster_pos,
                    unit_info: caster_info,
                };
                validate_filter(&caster, target_unit, pos, filter)?;
            }
        }
    }

    // ========================================================================
    // 寫入階段
    // ========================================================================

    let mut targeting =
        get_resource_mut::<SkillTargeting>(world, "請先呼叫 start_skill_targeting")?;
    if !allow_same_target && targeting.picked.contains(&pos) {
        return Ok(());
    }
    if targeting.picked.len() >= count {
        return Err(BoardError::TargetCountFull { max: count }.into());
    }
    targeting.picked.push(pos);
    Ok(())
}

/// 取消技能選目標流程，移除 SkillTargeting resource
pub fn cancel_skill_targeting(world: &mut World) {
    world.remove_resource::<SkillTargeting>();
}

/// 執行技能，回傳效果條目供演出
pub fn execute_skill(
    world: &mut World,
    skill_name: &SkillName,
    target_positions: &[Position],
) -> Result<Vec<EffectEntry>> {
    let board = *get_resource::<Board>(world, "請先呼叫 spawn_level")?;

    let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
    let active_occupant = get_current_unit(turn_order)?;

    let faction_to_alliance = build_faction_alliance_map(world)?;

    let caster_entity = find_entity_by_occupant(world, active_occupant)?;
    let (
        caster_pos,
        caster_occupant,
        caster_faction,
        caster_mp,
        caster_action_state,
        caster_movement_point,
        caster_attributes,
    ) = {
        let entity_ref = world.entity(caster_entity);
        let pos = *get_component!(entity_ref, Position)?;
        let occupant = *get_component!(entity_ref, Occupant)?;
        let faction = get_component!(entity_ref, UnitFaction)?.0;
        let mp = get_component!(entity_ref, CurrentMp)?.0;
        let action_state = get_component!(entity_ref, ActionState)?.clone();
        let movement_point = get_component!(entity_ref, MovementPoint)?.0;
        let attributes = read_attribute_bundle(&entity_ref)?;
        (
            pos,
            occupant,
            faction,
            mp,
            action_state,
            movement_point,
            attributes,
        )
    };

    check_action_point(&caster_action_state, caster_movement_point)?;

    let caster_alliance = resolve_alliance(&faction_to_alliance, caster_faction)?;
    let caster_info = UnitInfo {
        occupant: caster_occupant,
        faction_id: caster_faction,
        alliance_id: caster_alliance,
    };

    let (target, effects, cost) = {
        let game_data = get_resource::<GameData>(world, "請先呼叫 parse_and_insert_game_data")?;
        let (target, effects, cost) = get_active_skill_data(game_data, skill_name)?;
        (target.clone(), effects.to_vec(), cost)
    };

    if caster_mp < cost as i32 {
        return Err(UnitError::InsufficientMp {
            cost,
            current: caster_mp,
        }
        .into());
    }

    let unit_entities: Vec<Entity> = world
        .query_filtered::<Entity, With<Unit>>()
        .iter(world)
        .collect();
    let mut unit_stats_on_board: HashMap<Position, CombatStats> = HashMap::new();
    for unit_entity in unit_entities {
        let entity_ref = world.entity(unit_entity);
        let pos = *get_component!(entity_ref, Position)?;
        let occupant = *get_component!(entity_ref, Occupant)?;
        let faction_id = get_component!(entity_ref, UnitFaction)?.0;
        let attributes = read_attribute_bundle(&entity_ref)?;

        let alliance_id = resolve_alliance(&faction_to_alliance, faction_id)?;
        unit_stats_on_board.insert(
            pos,
            CombatStats {
                unit_info: UnitInfo {
                    occupant,
                    faction_id,
                    alliance_id,
                },
                attribute: attributes,
            },
        );
    }

    let objects_on_board: HashMap<Position, ObjectOnBoard> = world
        .query_filtered::<(&Position, &Occupant, &ObjectMovementCost), With<Object>>()
        .iter(world)
        .map(|(pos, occ, mc)| {
            (
                *pos,
                ObjectOnBoard {
                    occupant: *occ,
                    occupies_tile: mc.0 >= IMPASSABLE_MOVEMENT_COST,
                },
            )
        })
        .collect();

    let mut used_ids: HashSet<ID> = world
        .query::<&Occupant>()
        .iter(world)
        .map(|occ| match occ {
            Occupant::Unit(id) | Occupant::Object(id) => *id,
        })
        .collect();

    // ========================================================================
    // 純邏輯階段
    // ========================================================================

    let unit_infos_on_board: HashMap<Position, UnitInfo> = unit_stats_on_board
        .iter()
        .map(|(pos, stats)| (*pos, stats.unit_info.clone()))
        .collect();
    validate_skill_targets(
        &CasterInfo {
            position: caster_pos,
            unit_info: caster_info.clone(),
        },
        &target,
        target_positions,
        &unit_infos_on_board,
        board,
    )?;

    let caster_stats = CombatStats {
        unit_info: caster_info,
        attribute: caster_attributes,
    };

    let mut rng = rand::rng();
    let mut all_entries = Vec::new();
    for target_pos in target_positions {
        let entries = resolve_effect_tree(
            &effects,
            &caster_stats,
            caster_pos,
            *target_pos,
            &unit_stats_on_board,
            &objects_on_board,
            board,
            &mut || rng.random_range(1..=100),
        )?;
        all_entries.extend(entries);
    }

    // ========================================================================
    // 寫入階段
    // ========================================================================

    {
        let mut entity_mut = world.entity_mut(caster_entity);
        {
            let mut mp = get_component_mut!(entity_mut, CurrentMp)?;
            mp.0 -= cost as i32;
        }
        {
            let mut action_state = get_component_mut!(entity_mut, ActionState)?;
            *action_state = ActionState::Done;
        }
    }

    for entry in &all_entries {
        match &entry.effect {
            ResolvedEffect::HpChange { final_amount, .. } => {
                let entity = match entry.target {
                    CheckTarget::Unit(id) => find_entity_by_occupant(world, Occupant::Unit(id))?,
                    CheckTarget::Position(_) => unreachable!("HpChange 不應該有 Position 目標"),
                };
                let mut entity_mut = world.entity_mut(entity);
                let mut hp = get_component_mut!(entity_mut, CurrentHp)?;
                hp.0 += final_amount;
            }
            ResolvedEffect::SpawnObject { object_type } => {
                let pos = match entry.target {
                    CheckTarget::Position(pos) => pos,
                    CheckTarget::Unit(_) => unreachable!("SpawnObject 不應該有 Unit 目標"),
                };
                let id = generate_unique_id(&mut used_ids)?;
                // TODO 物件的其他屬性（例如 contact_effects）應該從技能效果定義中讀取，而不是寫死
                world.spawn(ObjectBundle {
                    object: Object,
                    position: pos,
                    occupant: Occupant::Object(id),
                    occupant_type_name: OccupantTypeName(object_type.clone()),
                    terrain_movement_cost: ObjectMovementCost(0),
                    contact_effects: ContactEffects(Vec::new()),
                });
            }
            // TODO 其他效果類型的寫入邏輯
            ResolvedEffect::ApplyBuff(_) | ResolvedEffect::NoEffect => {}
        }
    }

    Ok(all_entries)
}

/// 檢查施放者的行動點是否足夠發動技能
fn check_action_point(action_state: &ActionState, movement_point: i32) -> Result<()> {
    let action_point_max = movement_point * 2;
    match action_state {
        ActionState::Done => Err(UnitError::InsufficientActionPoint {
            used: action_point_max,
            max: action_point_max,
        }
        .into()),
        ActionState::Moved { cost } if (*cost as i32) > movement_point => {
            Err(UnitError::InsufficientActionPoint {
                used: *cost as i32,
                max: action_point_max,
            }
            .into())
        }
        ActionState::Moved { .. } => Ok(()),
    }
}
