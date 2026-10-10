//! Utility AI：先評估第一段移動與技能組合，無法施放時再利用兩段移動接近。
use crate::authoring::{AiProfile, DistancePreference, UnitType};
use crate::error::{self, GameError};
use crate::game::Game;
use crate::model::{
    AttackPreview, Board, Footprint, GridPos, HealingPreview, Hp, Id, MovementTransition, Pos,
    SkillDef, SkillEffect, SkillPreview, Skills, Turn, Unit,
};
use crate::movement::{
    MovementOption, can_move, footprint_cell_distance, footprint_cells, footprint_distance,
    full_board_paths, movement_budgets, movement_options, toward_skill_range, unit_at_cell,
};
use crate::skill::{
    can_use_skill, preview_unit_skill_from_position, validate_cell_skill_from_position,
};
use crate::terrain::{terrain_ends_movement, terrain_type, terrains_at};
use bevy_ecs::prelude::{Component, Entity, Resource, World};
use std::collections::HashMap;

#[derive(Resource)]
pub(crate) struct AiProfiles {
    by_unit_type: HashMap<String, AiProfile>,
}

/// 只在作者輸入邊界解析引用與驗證設定，執行期直接使用已解析的傾向。
pub(crate) fn resolve_profiles(
    profiles: Vec<AiProfile>,
    units: &[UnitType],
) -> Result<AiProfiles, GameError> {
    let mut definitions = HashMap::new();
    for profile in profiles {
        if profile.id.trim().is_empty() {
            return Err(error::invalid_ai_profile(&profile.id));
        }
        let id = profile.id.clone();
        if definitions.insert(id.clone(), profile).is_some() {
            return Err(error::duplicate_ai_profile(&id));
        }
    }
    let mut by_unit_type = HashMap::new();
    for unit in units {
        let profile = definitions
            .get(&unit.ai_profile)
            .ok_or_else(|| error::unknown_ai_profile(&unit.id, &unit.ai_profile))?
            .clone();
        by_unit_type.insert(unit.id.clone(), profile);
    }
    Ok(AiProfiles { by_unit_type })
}

struct SkillPlan {
    position_index: usize,
    skill: SkillDef,
    target_cell: GridPos,
    target_id: i64,
    utility: f64,
}

/// 僅記錄上一個 AI 回合實際攻擊的目標，站位仍逐回合重新評估。
#[derive(Component)]
struct PreviousAttackTarget(i64);

impl Game {
    pub(crate) fn run_ai_turn(&mut self) -> Result<(), GameError> {
        loop {
            let Turn {
                actor,
                phase: _,
                movement_remaining: _,
                movement_segments_used: _,
            } = self.world.resource::<Turn>();
            let actor = actor.ok_or(error::missing_initiative_unit())?;
            let entity = self.entity(actor).ok_or(error::missing_initiative_unit())?;
            let unit = self.world.get::<Unit>(entity).expect("AI 單位應具有 Unit");
            let AiProfiles { by_unit_type } = self.world.resource::<AiProfiles>();
            let profile = by_unit_type
                .get(&unit.unit_type)
                .expect("作者資料已解析所有單位種類的 AI 傾向");
            let Skills { definitions } = self.world.resource::<Skills>();
            let mut skills: Vec<_> = unit
                .skills
                .iter()
                .filter_map(|id| {
                    let skill = definitions.get(id).expect("作者資料已驗證技能引用");
                    if matches!(skill.effect, SkillEffect::Flanking { .. }) {
                        None
                    } else {
                        Some(skill.clone())
                    }
                })
                .collect();
            skills.sort_by(|a, b| a.id.cmp(&b.id));
            let mut positions =
                movement_options(&self.world, entity, self.world.resource::<Turn>(), false);
            let healing_focus = healing_focus(&self.world, entity, profile, &skills);
            let skill_plan =
                self.ai_skill_plan(entity, profile, &skills, &positions, healing_focus);
            let (position_index, action) = match skill_plan {
                Some(SkillPlan {
                    position_index,
                    skill,
                    target_cell,
                    target_id,
                    utility: _,
                }) => (position_index, Some((skill, target_cell, target_id))),
                None => {
                    positions =
                        movement_options(&self.world, entity, self.world.resource::<Turn>(), true);
                    (
                        fallback_position(
                            &self.world,
                            entity,
                            profile,
                            &skills,
                            &positions,
                            healing_focus,
                        ),
                        None,
                    )
                }
            };
            let MovementOption {
                position,
                path,
                cost: _,
            } = &positions[position_index];
            if path.len() > 1 {
                self.movements.push(MovementTransition {
                    unit_id: actor,
                    path: path.clone(),
                    before_log_index: self.world.resource::<crate::model::Log>().0.len(),
                });
                // 與玩家共用移動成本、階段、地形傷害及遭遇啟動流程。
                self.move_along(actor, path.clone())?;
                if self.world.get_entity(entity).is_err() {
                    return Ok(());
                }
                // 尖刺只中斷這次移動；仍有額度且尚未規劃技能時，從實際停點重新評估。
                let footprint = *self
                    .world
                    .get::<Footprint>(entity)
                    .expect("AI 單位應具有佔用尺寸");
                if action.is_none()
                    && terrain_ends_movement(&self.world, *position, footprint)
                    && can_move(self.world.resource::<Turn>())
                {
                    continue;
                }
            }
            // 同回合重新規劃仍沿用上回合目標，確定結束規劃後才清除記憶。
            self.world
                .entity_mut(entity)
                .remove::<PreviousAttackTarget>();
            match action {
                Some((skill, cell, target_id)) if can_use_skill(self.world.resource::<Turn>()) => {
                    let attack = matches!(
                        skill.effect,
                        SkillEffect::Attack { .. } | SkillEffect::Push { .. }
                    );
                    self.use_skill_at_cell(actor, cell, skill)?;
                    if attack && self.world.get_entity(entity).is_ok() {
                        self.world
                            .entity_mut(entity)
                            .insert(PreviousAttackTarget(target_id));
                    }
                    return Ok(());
                }
                _ => {
                    self.finish();
                    return Ok(());
                }
            }
        }
    }

    fn ai_skill_plan(
        &self,
        actor: Entity,
        profile: &AiProfile,
        skills: &[SkillDef],
        positions: &[MovementOption],
        healing_focus: Option<Entity>,
    ) -> Option<SkillPlan> {
        if !can_use_skill(self.world.resource::<Turn>()) {
            return None;
        }
        let mut best: Option<SkillPlan> = None;
        for (
            position_index,
            MovementOption {
                position,
                path: _,
                cost,
            },
        ) in positions.iter().enumerate()
        {
            for skill in skills {
                let preferred_distance = match preferred_skill_distance(
                    profile,
                    skills,
                    matches!(skill.effect, SkillEffect::Heal { .. }),
                ) {
                    Some(distance) => distance,
                    None => continue,
                };
                for target in self
                    .world
                    .iter_entities()
                    .filter(|target| target.contains::<Unit>())
                {
                    let target_entity = target.id();
                    let target_id = target.get::<Id>().expect("戰鬥單位應具有 Id").0;
                    let target_position = if target_entity == actor {
                        *position
                    } else {
                        target.get::<Pos>().expect("目標應具有位置").0
                    };
                    let target_footprint = *target.get::<Footprint>().expect("目標應具有佔用尺寸");
                    // 大型單位可有多個施放格；逐格沿用核心驗證，涵蓋最小射程。
                    for cell in footprint_cells(target_position, target_footprint) {
                        let utility = match skill_utility(
                            &self.world,
                            actor,
                            profile,
                            *position,
                            cell,
                            skill,
                            preferred_distance,
                            healing_focus,
                        ) {
                            Some(utility) => utility,
                            None => continue,
                        };
                        let candidate = SkillPlan {
                            position_index,
                            skill: skill.clone(),
                            target_cell: cell,
                            target_id,
                            utility,
                        };
                        let replace = best.as_ref().is_none_or(|previous| {
                            candidate.utility.total_cmp(&previous.utility).is_gt()
                                || (candidate.utility == previous.utility
                                    && (
                                        *cost, target_id, &skill.id, cell.y, cell.x, position.y,
                                        position.x,
                                    ) < (
                                        positions[previous.position_index].cost,
                                        previous.target_id,
                                        &previous.skill.id,
                                        previous.target_cell.y,
                                        previous.target_cell.x,
                                        positions[previous.position_index].position.y,
                                        positions[previous.position_index].position.x,
                                    ))
                        });
                        if replace {
                            best = Some(candidate);
                        }
                    }
                }
            }
        }
        best
    }
}

/// 評分僅使用核心的唯讀預覽；不同技能的價值集中於此，不依單位種類名稱分支。
fn skill_utility(
    world: &World,
    actor: Entity,
    profile: &AiProfile,
    position: GridPos,
    cell: GridPos,
    skill: &SkillDef,
    preferred_distance: i32,
    healing_focus: Option<Entity>,
) -> Option<f64> {
    let footprint = *world
        .get::<Footprint>(actor)
        .expect("AI 單位應具有佔用尺寸");
    let positioning = -(f64::from(footprint_cell_distance(position, footprint, cell))
        - f64::from(preferred_distance))
    .abs();
    if matches!(skill.effect, SkillEffect::Mire { .. }) {
        if profile.positioning_weight == 0 {
            return None;
        }
        let target = unit_at_cell(world, cell)?;
        let actor_team = &world.get::<Unit>(actor).expect("施放者應具有 Unit").team;
        if &world.get::<Unit>(target).expect("目標應具有 Unit").team == actor_team {
            return None;
        }
        let (terrain, _) =
            validate_cell_skill_from_position(world, actor, position, cell, skill).ok()?;
        if terrains_at(world, cell).contains(&terrain) {
            return None;
        }
        let terrain = terrain_type(world.resource::<Board>(), &terrain);
        let control = f64::from(terrain.extra_movement_cost)
            + f64::from(terrain.dodge_penalty)
            + f64::from(terrain.block_penalty);
        return if control > 0.0 {
            Some((control + positioning) * f64::from(profile.positioning_weight))
        } else {
            None
        };
    }
    let preview = preview_unit_skill_from_position(world, actor, position, cell, skill).ok()?;
    match preview {
        SkillPreview::Attack(preview) => {
            if profile.damage_weight == 0 {
                return None;
            }
            Some(
                if world
                    .get::<PreviousAttackTarget>(actor)
                    .is_some_and(|previous| previous.0 == preview.target)
                {
                    f64::from(profile.pursuit_weight)
                } else {
                    0.0
                } + f64::from(preview.hit_chance) / 100.0 * f64::from(profile.hit_weight)
                    + attack_utility(preview) * f64::from(profile.damage_weight)
                    + positioning * f64::from(profile.positioning_weight),
            )
        }
        SkillPreview::Healing(HealingPreview {
            target: _,
            target_type: _,
            target_hp,
            target_max_hp,
            target_mana: _,
            healing,
            remaining_hp: _,
            missing_hp: _,
            health_segments: _,
        }) => {
            if healing == 0 || profile.healing_weight == 0 {
                return None;
            }
            // 有效治療量優先；同樣能接受完整治療時，以傷勢比例決定優先順序。
            let urgency = 1.0 - f64::from(target_hp) / f64::from(target_max_hp);
            let progress = healing_focus
                .map(|focus| {
                    let start = world.get::<Pos>(actor).expect("AI 單位應具有位置").0;
                    f64::from(
                        target_distance(world, actor, start, focus)
                            - target_distance(world, actor, position, focus),
                    )
                })
                .unwrap_or(0.0);
            Some(
                (f64::from(healing) + urgency) * f64::from(profile.healing_weight)
                    + if healing_focus.is_some_and(|focus| {
                        let start = world.get::<Pos>(actor).expect("AI 單位應具有位置").0;
                        target_distance(world, actor, start, focus) > skill.max_range
                    }) {
                        progress
                    } else {
                        positioning
                    } * f64::from(profile.positioning_weight),
            )
        }
    }
}

fn target_distance(world: &World, actor: Entity, position: GridPos, target: Entity) -> i32 {
    let actor_footprint = *world
        .get::<Footprint>(actor)
        .expect("AI 單位應具有佔用尺寸");
    let target_position = if actor == target {
        position
    } else {
        world.get::<Pos>(target).expect("目標應具有位置").0
    };
    let target_footprint = *world.get::<Footprint>(target).expect("目標應具有佔用尺寸");
    footprint_distance(position, actor_footprint, target_position, target_footprint)
}

fn healing_focus(
    world: &World,
    actor: Entity,
    profile: &AiProfile,
    skills: &[SkillDef],
) -> Option<Entity> {
    if profile.healing_weight == 0
        || !skills
            .iter()
            .any(|skill| matches!(skill.effect, SkillEffect::Heal { .. }))
    {
        return None;
    }
    let team = &world.get::<Unit>(actor).expect("AI 單位應具有 Unit").team;
    world
        .iter_entities()
        .filter(|target| target.get::<Unit>().is_some_and(|unit| &unit.team == team))
        .filter(|target| target.get::<Hp>().is_some_and(|hp| hp.current < hp.maximum))
        .min_by(|a, b| {
            let Hp {
                current: a_hp,
                maximum: a_max,
            } = a.get::<Hp>().expect("隊友應具有 Hp");
            let Hp {
                current: b_hp,
                maximum: b_max,
            } = b.get::<Hp>().expect("隊友應具有 Hp");
            (i64::from(*a_hp) * i64::from(*b_max))
                .cmp(&(i64::from(*b_hp) * i64::from(*a_max)))
                .then_with(|| a_hp.cmp(b_hp))
                .then_with(|| {
                    a.get::<Id>()
                        .expect("隊友應具有 Id")
                        .0
                        .cmp(&b.get::<Id>().expect("隊友應具有 Id").0)
                })
        })
        .map(|target| target.id())
}

fn fallback_position(
    world: &World,
    actor: Entity,
    profile: &AiProfile,
    skills: &[SkillDef],
    positions: &[MovementOption],
    healing_focus: Option<Entity>,
) -> usize {
    if profile.positioning_weight == 0 {
        return 0;
    }
    let start = positions[0].position;
    let team = &world.get::<Unit>(actor).expect("AI 單位應具有 Unit").team;
    let targets = match healing_focus {
        Some(target) => vec![target],
        None => {
            let mut targets: Vec<_> = world
                .iter_entities()
                .filter(|target| target.get::<Unit>().is_some_and(|unit| &unit.team != team))
                .map(|target| target.id())
                .collect();
            targets.sort_by_key(|target| {
                (
                    target_distance(world, actor, start, *target),
                    world.get::<Id>(*target).expect("目標應具有 Id").0,
                )
            });
            targets
        }
    };
    let footprint = *world
        .get::<Footprint>(actor)
        .expect("AI 單位應具有佔用尺寸");
    let allowance = world
        .get::<Unit>(actor)
        .expect("AI 單位應具有 Unit")
        .movement;
    let (first_budget, second_budget) = movement_budgets(world.resource::<Turn>(), allowance);
    let budget = first_budget + second_budget;
    let desired = match preferred_skill_distance(profile, skills, healing_focus.is_some()) {
        Some(distance) => distance,
        None => return 0,
    };
    if targets.is_empty() {
        return 0;
    }
    let search = full_board_paths(world, actor, start, footprint);
    // 依距離與 ID 選擇有合法施放路徑的敵人；治療仍優先接近既定的重傷隊友。
    for target in targets {
        let target_footprint = *world.get::<Footprint>(target).expect("目標應具有佔用尺寸");
        let goal = world.get::<Pos>(target).expect("目標應具有位置").0;
        // 共用跨技能偏好；每條路徑仍須朝該技能可施放的合法射程前進。
        let position = skills
            .iter()
            .filter(|skill| relevant_positioning_skill(profile, skill, healing_focus.is_some()))
            .filter_map(|skill| {
                let skill_distance = desired.clamp(skill.min_range, skill.max_range);
                let path = toward_skill_range(
                    world,
                    &search,
                    start,
                    goal,
                    target_footprint,
                    footprint,
                    budget,
                    skill.min_range,
                    skill.max_range,
                    skill_distance,
                )?;
                // 理想停點可能因第一段危險路徑截斷而不可執行，仍沿規劃路徑前進到最遠合法停點。
                let index = path
                    .iter()
                    .rev()
                    .find_map(|cell| positions.iter().position(|plan| plan.position == *cell))?;
                let MovementOption {
                    position: end,
                    path: _,
                    cost: _,
                } = &positions[index];
                let end = *end;
                let distance = target_distance(world, actor, end, target);
                let range_gap = (skill.min_range - distance)
                    .max(distance - skill.max_range)
                    .max(0);
                Some((
                    (
                        range_gap,
                        (distance - desired).abs(),
                        positions[index].cost,
                        &skill.id,
                    ),
                    index,
                ))
            })
            .min_by(|(a, _), (b, _)| a.cmp(b))
            .map(|(_, index)| index);
        if let Some(index) = position {
            return index;
        }
    }
    0
}

/// 攻擊與治療分別比較可用技能，避免自我治療的零射程影響接敵站位。
fn preferred_skill_distance(
    profile: &AiProfile,
    skills: &[SkillDef],
    healing: bool,
) -> Option<i32> {
    let ranges = skills
        .iter()
        .filter(|skill| relevant_positioning_skill(profile, skill, healing));
    match profile.distance_preference {
        DistancePreference::Near => ranges.map(|skill| skill.min_range).min(),
        DistancePreference::Far => ranges.map(|skill| skill.max_range).max(),
    }
}

fn relevant_positioning_skill(profile: &AiProfile, skill: &SkillDef, healing: bool) -> bool {
    match skill.effect {
        SkillEffect::Heal { .. } => healing && profile.healing_weight > 0,
        SkillEffect::Attack { .. } | SkillEffect::Push { .. } => {
            !healing && profile.damage_weight > 0
        }
        SkillEffect::Mire { .. } => !healing && profile.positioning_weight > 0,
        SkillEffect::Flanking { .. } => false,
    }
}

fn attack_utility(preview: AttackPreview) -> f64 {
    let AttackPreview {
        target: _,
        target_type: _,
        target_hp,
        target_max_hp: _,
        target_mana: _,
        hit_remaining_hp: _,
        block_remaining_hp: _,
        dodge_remaining_hp: _,
        dodge_chance: _,
        block_chance,
        hit_chance,
        critical_chance,
        dodge_damage: _,
        block_damage,
        critical_block_damage,
        hit_damage,
        critical_hit_damage,
        health_segments: _,
    } = preview;
    let (critical_hit, critical_block) = if hit_chance > 0 {
        (critical_chance, 0)
    } else if block_chance > 0 {
        (0, critical_chance)
    } else {
        (0, 0)
    };
    let outcomes = [
        (hit_chance - critical_hit, hit_damage),
        (block_chance - critical_block, block_damage),
        (critical_hit, critical_hit_damage),
        (critical_block, critical_block_damage),
    ];
    outcomes
        .into_iter()
        .map(|(chance, damage)| {
            // 擊倒能移除對手後續行動，收益與傷害一同受 damage_weight 控制。
            f64::from(chance) / 100.0
                * (f64::from(damage.min(target_hp)) + if damage >= target_hp { 100.0 } else { 0.0 })
        })
        .sum()
}
