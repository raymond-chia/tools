//! 技能驗證、預覽、效果與戰鬥計算。
use crate::error::{self, GameError};
use crate::game::{DamageResult, Game, die};
use crate::gameplay_config;
use crate::model::{
    ActiveSkillDef, ActiveSkillDetailEffect, ActiveSkillDetailsView, ActiveSkillEffect,
    ActiveSkillPreview, ActiveSkillRangeView, ActiveSkillTargetKind, ActiveSkills, AttackPreview,
    AttackResult, Board, CollisionUnitLog, CombatLogEvent, Encounter, Footprint, GridPos,
    HealingPreview, HealthSegmentsView, Hp, Id, Log, PassiveSkillDef, PassiveSkillEffect,
    PassiveSkills, PendingStrike, Phase, Pos, RollDegree, TerrainGrid, Turn, Unit,
};
use crate::movement::{
    PositionChangeResult, fits, footprint_cell_distance, footprint_cells, overlap, unit_at_cell,
};
use crate::terrain::{
    TerrainEntry, add_temporary_terrain, footprint_blocks_push, footprint_on_impassable,
    footprint_terrain_penalty, terrain_type,
};
use bevy_ecs::prelude::{Entity, World};

/// 待出刀時，同一技能改用普通近戰；其他技能不能打斷這段行動。
pub(crate) fn skill_for_current_stage(
    world: &World,
    actor: Entity,
    skill: &ActiveSkillDef,
) -> Result<ActiveSkillDef, GameError> {
    if let Some(PendingStrike { skill: pending }) = world.get::<PendingStrike>(actor) {
        if pending != &skill.id {
            return Err(error::cannot_use_skill());
        }
        return Ok(ActiveSkillDef {
            id: skill.id.clone(),
            min_range: 1,
            max_range: 1,
            effect: ActiveSkillEffect::Attack {
                power_source: crate::PowerSource::Physical,
                ranged: false,
                hit_bonus: 0,
                power_bonus: 0,
            },
        });
    }
    Ok(skill.clone())
}

fn validate_step_destination(
    world: &World,
    actor: Entity,
    destination: GridPos,
    skill: &ActiveSkillDef,
) -> Result<(), GameError> {
    let unit = world.get::<Unit>(actor).expect("施放者應具有單位資料");
    if !unit.active_skills.contains(&skill.id) {
        return Err(error::unit_cannot_use_skill());
    }
    let origin = world.get::<Pos>(actor).expect("施放者應具有位置").0;
    let footprint = *world.get::<Footprint>(actor).expect("施放者應具有尺寸");
    let grid = world.resource::<TerrainGrid>();
    if !fits(grid.width, grid.height, destination, footprint) {
        return Err(error::target_cell_out_of_bounds());
    }
    if destination == origin
        || footprint_on_impassable(grid, destination, footprint)
        || world.iter_entities().any(|entity| {
            entity.id() != actor
                && entity
                    .get::<Pos>()
                    .zip(entity.get::<Footprint>())
                    .is_some_and(|(position, other)| {
                        overlap(destination, footprint, position.0, *other)
                    })
        })
    {
        return Err(error::target_cell_impassable());
    }
    crate::sight::check_sight_in_range(
        grid,
        origin,
        footprint,
        destination,
        skill.min_range,
        skill.max_range,
    )
}

impl Game {
    fn move_then_attack(
        &mut self,
        entity: Entity,
        actor: i64,
        destination: GridPos,
        skill: &ActiveSkillDef,
    ) -> Result<(), GameError> {
        validate_step_destination(&self.world, entity, destination, skill)?;
        let origin = self.world.get::<Pos>(entity).expect("施放者應具有位置").0;
        let PositionChangeResult {
            movement_cost: _,
            unit_remains,
        } = self.change_position(
            entity,
            actor,
            &[origin, destination],
            TerrainEntry::Movement,
        );
        if unit_remains {
            self.world.entity_mut(entity).insert(PendingStrike {
                skill: skill.id.clone(),
            });
            self.world.resource_mut::<Turn>().phase = Phase::PendingStrike;
        }
        Ok(())
    }
    /// 預覽與施放共用：確認輪到該單位且本回合仍可使用技能，回傳施放者。
    fn skill_caster(&self, actor: i64) -> Result<Entity, GameError> {
        self.ensure(actor)?;
        if !can_use_skill(self.world.resource::<Turn>()) {
            return Err(error::cannot_use_skill());
        }
        self.entity(actor).ok_or(error::missing_actor())
    }
    pub fn preview_skill(
        &self,
        actor: i64,
        target_cell: GridPos,
        skill_id: &str,
    ) -> Result<ActiveSkillPreview, GameError> {
        let attacker = self.skill_caster(actor)?;
        let ActiveSkills { definitions } = self.world.resource::<ActiveSkills>();
        let skill = definitions
            .get(skill_id)
            .ok_or_else(|| error::unknown_skill(skill_id))?;
        let position = self.world.get::<Pos>(attacker).expect("施放者應具有位置").0;
        let skill = skill_for_current_stage(&self.world, attacker, skill)?;
        preview_unit_skill_from_position(&self.world, attacker, position, target_cell, &skill)
    }
    pub(crate) fn use_skill_at_cell(
        &mut self,
        actor: i64,
        position: GridPos,
        skill: ActiveSkillDef,
    ) -> Result<(), GameError> {
        let ae = self.skill_caster(actor)?;
        let skill = skill_for_current_stage(&self.world, ae, &skill)?;
        if matches!(skill.effect, ActiveSkillEffect::MoveThenAttack) {
            return self.move_then_attack(ae, actor, position, &skill);
        }
        if matches!(skill.effect, ActiveSkillEffect::Loot) {
            return self.loot_cargo(ae, actor, position, &skill);
        }
        if target_kind(&skill.effect) == ActiveSkillTargetKind::Cell {
            return self.use_cell_skill(ae, actor, position, skill);
        }
        let a = actor;
        let target_cell = position;
        let (te, effect) = validate_unit_skill_target(&self.world, ae, target_cell, &skill)?;
        let target = self
            .world
            .get::<Id>(te)
            .expect("所選格上的戰鬥單位應具有 Id 元件")
            .0;
        let Unit {
            unit_type: actor_type,
            visual: _,
            team: actor_team,
            movement: _,
            initiative: _,
            dodge: _,
            block: _,
            hit: _,
            physical_power: _,
            magical_power: _,
            block_reduction: _,
            equipment: _,
            active_skills: _,
            passive_skills: _,
        } = self
            .world
            .get::<Unit>(ae)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .clone();
        let Unit {
            unit_type: target_type,
            visual: _,
            team: target_team,
            movement: _,
            initiative: _,
            dodge: _,
            block: _,
            hit: _,
            physical_power: _,
            magical_power: _,
            block_reduction: _,
            equipment: _,
            active_skills: _,
            passive_skills: _,
        } = self
            .world
            .get::<Unit>(te)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .clone();
        let (hit_bonus, power, push) = match effect {
            UnitSkillEffect::Attack {
                hit_bonus,
                power,
                push,
            } => (hit_bonus, power, push),
            UnitSkillEffect::Heal {
                power_source,
                power_bonus,
            } => {
                let HealingPreview {
                    target: target_id,
                    target_type,
                    target_hp: _,
                    target_max_hp: max_hp,
                    target_mana: _,
                    healing,
                    remaining_hp,
                    missing_hp: _,
                    health_segments: _,
                } = healing_preview(&self.world, ae, te, power_bonus, power_source);
                self.world
                    .get_mut::<Hp>(te)
                    .expect("已建立的戰鬥單位應具有 Hp 元件")
                    .current = remaining_hp;
                self.world
                    .resource_mut::<Log>()
                    .0
                    .push(CombatLogEvent::Healing {
                        actor: a,
                        actor_type,
                        actor_team,
                        skill: skill.id,
                        target: target_id,
                        target_type,
                        target_team,
                        healing,
                        remaining_hp,
                        max_hp,
                    });
                self.finish();
                return Ok(());
            }
        };
        let HitBreakdown {
            base_hit,
            skill_hit,
            flanking_modifier,
            total: modifier,
        } = hit_breakdown(&self.world, ae, te, &skill, hit_bonus);
        let natural = die(&mut self.world, gameplay_config::HIT_DIE_SIDES) as i32;
        let target_dodge = effective_dodge(&self.world, te);
        let target_block = effective_block(&self.world, te);
        let degree = degree(natural, modifier, target_dodge);
        let result = attack_result(natural, modifier, target_dodge, target_dodge + target_block);
        let base_damage = direct_damage(&self.world, ae, power);
        let critical = degree == RollDegree::CriticalSuccess;
        let raw_damage = attack_damage(
            base_damage,
            AttackResult::Hit,
            block_reduction(&self.world, te),
            critical,
        );
        let damage = attack_damage(
            base_damage,
            result,
            block_reduction(&self.world, te),
            critical,
        );
        let damage_reduction = raw_damage - damage;
        let DamageResult {
            mut remaining_hp,
            mut max_hp,
            mut downed,
        } = self.apply_damage(te, target, damage);
        let mut push_path = None;
        let mut push_blocked = false;
        let mut collision_damage = 0;
        let mut collision_units = Vec::new();
        if push && result == AttackResult::Hit && !downed {
            let direction = push_direction(&self.world, ae, target_cell);
            let current = self
                .world
                .get::<Pos>(te)
                .expect("已建立的戰鬥單位應具有 Pos 元件")
                .0;
            let footprint = *self
                .world
                .get::<Footprint>(te)
                .expect("已建立的戰鬥單位應具有 Footprint 元件");
            let destination = GridPos {
                x: current.x + direction.x * gameplay_config::PUSH_DISTANCE,
                y: current.y + direction.y * gameplay_config::PUSH_DISTANCE,
            };
            let Board {
                width,
                height,
                terrains: _,
                terrain_types: _,
            } = self.world.resource::<Board>();
            let terrain_allows_push = fits(*width, *height, destination, footprint)
                && !footprint_blocks_push(&self.world, destination, footprint);
            let blocking_units: Vec<Entity> = if terrain_allows_push {
                self.world
                    .iter_entities()
                    .filter(|entity| {
                        entity.id() != te
                            && entity
                                .get::<Pos>()
                                .zip(entity.get::<Footprint>())
                                .is_some_and(|(position, other_footprint)| {
                                    overlap(destination, footprint, position.0, *other_footprint)
                                })
                    })
                    .map(|entity| entity.id())
                    .collect()
            } else {
                Vec::new()
            };
            let can_push = terrain_allows_push && blocking_units.is_empty();
            if can_push {
                push_path = Some([current, destination]);
            } else {
                push_blocked = true;
                collision_damage = gameplay_config::COLLISION_DAMAGE;
                DamageResult {
                    remaining_hp,
                    max_hp,
                    downed,
                } = self.apply_damage(te, target, collision_damage);
                for blocking_entity in blocking_units {
                    let Unit {
                        unit_type,
                        visual: _,
                        team,
                        movement: _,
                        initiative: _,
                        dodge: _,
                        block: _,
                        hit: _,
                        physical_power: _,
                        magical_power: _,
                        block_reduction: _,
                        equipment: _,
                        active_skills: _,
                        passive_skills: _,
                    } = self
                        .world
                        .get::<Unit>(blocking_entity)
                        .expect("佔用格子的戰鬥單位應具有 Unit 元件")
                        .clone();
                    let id = self
                        .world
                        .get::<Id>(blocking_entity)
                        .expect("佔用格子的戰鬥單位應具有 Id 元件")
                        .0
                        .clone();
                    let DamageResult {
                        remaining_hp,
                        max_hp,
                        downed,
                    } = self.apply_damage(blocking_entity, id, collision_damage);
                    collision_units.push(CollisionUnitLog {
                        unit: id,
                        unit_type,
                        team,
                        remaining_hp,
                        max_hp,
                        downed,
                    });
                }
            }
        }
        let pushed = push_path.is_some();
        self.world
            .resource_mut::<Log>()
            .0
            .push(CombatLogEvent::Skill {
                actor: a,
                actor_type,
                actor_team,
                skill: skill.id,
                target,
                target_type,
                target_team,
                roll: natural,
                die_sides: gameplay_config::HIT_DIE_SIDES,
                base_hit,
                skill_hit,
                flanking_modifier,
                hit: modifier,
                hit_total: natural + modifier,
                dodge_target: target_dodge,
                block_target: target_dodge + target_block,
                result,
                critical,
                raw_damage,
                damage_reduction,
                damage,
                remaining_hp,
                max_hp,
                downed,
                pushed,
                push_blocked,
                push_distance: if pushed {
                    gameplay_config::PUSH_DISTANCE
                } else {
                    0
                },
                collision_damage,
                collision_units,
            });
        if let Some(path) = push_path {
            // 技能結果先入日誌，位置變更入口再記錄推移與地形效果的呈現順序。
            self.change_position(te, target, &path, TerrainEntry::Push);
        }
        self.finish();
        Ok(())
    }
    fn use_cell_skill(
        &mut self,
        entity: Entity,
        actor: i64,
        position: GridPos,
        skill: ActiveSkillDef,
    ) -> Result<(), GameError> {
        let origin = self.world.get::<Pos>(entity).expect("施放者應具有位置").0;
        let (terrain, duration) =
            validate_cell_skill_from_position(&self.world, entity, origin, position, &skill)?;
        let unit = self.world.get::<Unit>(entity).expect("施放者應具有 Unit");
        let actor_type = unit.unit_type.clone();
        let actor_team = unit.team.clone();
        let ActiveSkillDef {
            id,
            min_range: _,
            max_range: _,
            effect: _,
        } = skill;
        let Encounter {
            participants: _,
            order: _,
            cursor: _,
            round: current_round,
        } = self.world.resource::<Encounter>();
        let expires_after_round = *current_round + duration - 1;
        add_temporary_terrain(
            &mut self.world,
            position,
            terrain.clone(),
            expires_after_round,
        );
        self.world
            .resource_mut::<Log>()
            .0
            .push(CombatLogEvent::TerrainCreated {
                actor,
                actor_type,
                actor_team,
                skill: id,
                terrain,
            });
        self.finish();
        Ok(())
    }
}

/// 施放與 AI 預覽共用地形技能合法性；作者引用與數值已於載入時驗證。
pub(crate) fn validate_cell_skill_from_position(
    world: &World,
    actor: Entity,
    origin: GridPos,
    cell: GridPos,
    skill: &ActiveSkillDef,
) -> Result<(String, u32), GameError> {
    let unit = world.get::<Unit>(actor).expect("施放者應具有 Unit");
    if !unit.active_skills.contains(&skill.id) {
        return Err(error::unit_cannot_use_skill());
    }
    let (terrain, duration) = match &skill.effect {
        ActiveSkillEffect::Mire { terrain, duration } => (terrain.clone(), *duration),
        _ => return Err(error::unit_cannot_use_skill()),
    };
    let grid = world.resource::<TerrainGrid>();
    if !fits(
        grid.width,
        grid.height,
        cell,
        Footprint {
            width: 1,
            height: 1,
        },
    ) {
        return Err(error::target_cell_out_of_bounds());
    }
    if footprint_on_impassable(
        grid,
        cell,
        Footprint {
            width: 1,
            height: 1,
        },
    ) {
        return Err(error::target_cell_impassable());
    }
    let footprint = *world.get::<Footprint>(actor).expect("施放者應具有佔用尺寸");
    crate::sight::check_sight_in_range(
        grid,
        origin,
        footprint,
        cell,
        skill.min_range,
        skill.max_range,
    )?;
    Ok((terrain, duration))
}

/// AI 與一般預覽共用同一計算；假想位置只用於查詢，不改動 ECS。
pub(crate) fn preview_unit_skill_from_position(
    world: &World,
    attacker: Entity,
    position: GridPos,
    target_cell: GridPos,
    skill: &ActiveSkillDef,
) -> Result<ActiveSkillPreview, GameError> {
    let (target_entity, effect) =
        validate_unit_skill_target_from_position(world, attacker, position, target_cell, skill)?;
    let (hit_bonus, power) = match effect {
        UnitSkillEffect::Heal {
            power_source,
            power_bonus,
        } => {
            return Ok(ActiveSkillPreview::Healing(healing_preview(
                world,
                attacker,
                target_entity,
                power_bonus,
                power_source,
            )));
        }
        UnitSkillEffect::Attack {
            hit_bonus,
            power,
            push: _,
        } => (hit_bonus, power),
    };

    let Unit {
        unit_type: _,
        visual: _,
        team: _,
        movement: _,
        initiative: _,
        dodge: _,
        block: _,
        hit: _,
        physical_power: _,
        magical_power: _,
        block_reduction: _,
        equipment: _,
        active_skills: _,
        passive_skills: _,
    } = world
        .get::<Unit>(attacker)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    let Unit {
        unit_type: target_type,
        visual: _,
        team: _,
        movement: _,
        initiative: _,
        dodge: _,
        block: _,
        hit: _,
        physical_power: _,
        magical_power: _,
        block_reduction: _,
        equipment: _,
        active_skills: _,
        passive_skills: _,
    } = world
        .get::<Unit>(target_entity)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    let target = world.get::<Id>(target_entity).expect("目標單位應具有 Id").0;
    let Hp {
        current: target_hp,
        maximum: target_max_hp,
    } = world
        .get::<Hp>(target_entity)
        .expect("已建立的戰鬥單位應具有 Hp 元件");
    let HitBreakdown {
        base_hit: _,
        skill_hit: _,
        flanking_modifier: _,
        total: modifier,
    } = hit_breakdown_from_position(world, attacker, position, target_entity, skill, hit_bonus);
    let dodge_target = effective_dodge(world, target_entity);
    let block_target = dodge_target + effective_block(world, target_entity);
    let mut dodge_count = 0;
    let mut block_count = 0;
    let mut hit_count = 0;
    for natural in 1..=gameplay_config::HIT_DIE_SIDES as i32 {
        match attack_result(natural, modifier, dodge_target, block_target) {
            AttackResult::Dodge => dodge_count += 1,
            AttackResult::Block => block_count += 1,
            AttackResult::Hit => hit_count += 1,
        }
    }
    let hit_damage = direct_damage(world, attacker, power);
    let block_damage = attack_damage(
        hit_damage,
        AttackResult::Block,
        block_reduction(world, target_entity),
        false,
    );
    let hit_remaining_hp = (*target_hp - hit_damage).max(0);
    let block_remaining_hp = (*target_hp - block_damage).max(0);
    let block_segment = if block_count > 0 {
        block_remaining_hp - hit_remaining_hp
    } else {
        0
    };
    Ok(ActiveSkillPreview::Attack(AttackPreview {
        target,
        target_type: target_type.clone(),
        target_hp: *target_hp,
        target_max_hp: *target_max_hp,
        target_mana: gameplay_config::DEFAULT_MANA,
        hit_remaining_hp,
        block_remaining_hp,
        dodge_remaining_hp: *target_hp,
        dodge_chance: dodge_count * 100 / gameplay_config::HIT_DIE_SIDES,
        block_chance: block_count * 100 / gameplay_config::HIT_DIE_SIDES,
        hit_chance: hit_count * 100 / gameplay_config::HIT_DIE_SIDES,
        critical_chance: 100 / gameplay_config::HIT_DIE_SIDES,
        dodge_damage: 0,
        block_damage,
        critical_block_damage: attack_damage(
            hit_damage,
            AttackResult::Block,
            block_reduction(world, target_entity),
            true,
        ),
        hit_damage,
        critical_hit_damage: attack_damage(
            hit_damage,
            AttackResult::Hit,
            block_reduction(world, target_entity),
            true,
        ),
        health_segments: HealthSegmentsView {
            hit: hit_remaining_hp,
            block: block_segment,
            damage: *target_hp - hit_remaining_hp - block_segment,
            missing: *target_max_hp - *target_hp,
        },
    }))
}

fn healing_preview(
    world: &World,
    actor: Entity,
    target: Entity,
    power_bonus: i32,
    source: crate::PowerSource,
) -> HealingPreview {
    let Hp { current, maximum } = world
        .get::<Hp>(target)
        .expect("已建立的戰鬥單位應具有 Hp 元件");
    let power = unit_power(world, actor, source);
    let remaining_hp = (*current + skill_power(power, power_bonus)).min(*maximum);
    HealingPreview {
        target: world
            .get::<Id>(target)
            .expect("已建立的戰鬥單位應具有 Id 元件")
            .0
            .clone(),
        target_type: world
            .get::<Unit>(target)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .unit_type
            .clone(),
        target_hp: *current,
        target_max_hp: *maximum,
        target_mana: gameplay_config::DEFAULT_MANA,
        healing: remaining_hp - *current,
        remaining_hp,
        missing_hp: *maximum - remaining_hp,
        health_segments: HealthSegmentsView {
            hit: *current,
            block: 0,
            damage: 0,
            missing: *maximum - remaining_hp,
        },
    }
}

// 推擊不具有直接傷害；預覽與結算共用此判斷。
fn direct_damage(world: &World, actor: Entity, power: Option<(crate::PowerSource, i32)>) -> i32 {
    match power {
        Some((source, bonus)) => skill_power(unit_power(world, actor, source), bonus),
        None => 0,
    }
}

/// 技能造成的基礎傷害或治療量；加值為負時最低為 0，不會反轉成治療或傷害。
fn skill_power(power: i32, power_bonus: i32) -> i32 {
    (power + power_bonus).max(0)
}

/// 技能指定的目標種類；以技能種類決定行為時一律經過此函式。
pub(crate) fn target_kind(effect: &ActiveSkillEffect) -> ActiveSkillTargetKind {
    match effect {
        ActiveSkillEffect::Attack { .. } | ActiveSkillEffect::Push { .. } => {
            ActiveSkillTargetKind::Enemy
        }
        ActiveSkillEffect::Heal { .. } => ActiveSkillTargetKind::Ally,
        ActiveSkillEffect::Mire { .. }
        | ActiveSkillEffect::Loot
        | ActiveSkillEffect::MoveThenAttack => ActiveSkillTargetKind::Cell,
    }
}

/// 以單位為目標的技能效果，只能由 validate_unit_skill_target 驗證後取得。
enum UnitSkillEffect {
    Attack {
        hit_bonus: i32,
        /// 推擊沒有直接傷害，因此不具威力來源與加值。
        power: Option<(crate::PowerSource, i32)>,
        push: bool,
    },
    Heal {
        power_source: crate::PowerSource,
        power_bonus: i32,
    },
}

// 從所選格解析目標並驗證技能；呼叫端不再分別傳入目標 ID 與格子。
fn validate_unit_skill_target(
    world: &World,
    attacker: Entity,
    target_cell: GridPos,
    skill: &ActiveSkillDef,
) -> Result<(Entity, UnitSkillEffect), GameError> {
    let position = world.get::<Pos>(attacker).expect("施放者應具有位置").0;
    validate_unit_skill_target_from_position(world, attacker, position, target_cell, skill)
}

fn validate_unit_skill_target_from_position(
    world: &World,
    attacker: Entity,
    attacker_position: GridPos,
    target_cell: GridPos,
    skill: &ActiveSkillDef,
) -> Result<(Entity, UnitSkillEffect), GameError> {
    let ActiveSkillDef {
        id,
        min_range,
        max_range,
        effect: skill_effect,
    } = skill;
    let attacker_footprint = *world
        .get::<Footprint>(attacker)
        .expect("施放者應具有佔用尺寸");
    let target = if footprint_cells(attacker_position, attacker_footprint).contains(&target_cell) {
        attacker
    } else {
        unit_at_cell(world, target_cell).ok_or(error::missing_target())?
    };
    let Unit {
        unit_type: _,
        visual: _,
        team: attacker_team,
        movement: _,
        initiative: _,
        dodge: _,
        block: _,
        hit: _,
        physical_power: _,
        magical_power: _,
        block_reduction: _,
        equipment: _,
        active_skills,
        passive_skills: _,
    } = world
        .get::<Unit>(attacker)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    let Unit {
        unit_type: _,
        visual: _,
        team: target_team,
        movement: _,
        initiative: _,
        dodge: _,
        block: _,
        hit: _,
        physical_power: _,
        magical_power: _,
        block_reduction: _,
        equipment: _,
        active_skills: _,
        passive_skills: _,
    } = world
        .get::<Unit>(target)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    if !active_skills.contains(id) {
        return Err(error::unit_cannot_use_skill());
    }
    let effect = match skill_effect {
        ActiveSkillEffect::Attack {
            power_source,
            ranged: _,
            hit_bonus,
            power_bonus,
        } => UnitSkillEffect::Attack {
            hit_bonus: *hit_bonus,
            power: Some((*power_source, *power_bonus)),
            push: false,
        },
        ActiveSkillEffect::Push {
            ranged: _,
            hit_bonus,
        } => UnitSkillEffect::Attack {
            hit_bonus: *hit_bonus,
            power: None,
            push: true,
        },
        ActiveSkillEffect::Heal {
            power_source,
            power_bonus,
        } => UnitSkillEffect::Heal {
            power_source: *power_source,
            power_bonus: *power_bonus,
        },
        ActiveSkillEffect::Mire { .. }
        | ActiveSkillEffect::Loot
        | ActiveSkillEffect::MoveThenAttack => {
            return Err(error::unit_cannot_use_skill());
        }
    };
    match effect {
        UnitSkillEffect::Heal { .. }
            if !crate::teams::are_allied(world, attacker_team, target_team) =>
        {
            return Err(error::heal_allies_only());
        }
        UnitSkillEffect::Attack { .. }
            if crate::teams::are_allied(world, attacker_team, target_team) =>
        {
            return Err(error::cannot_attack_ally());
        }
        UnitSkillEffect::Heal { .. } | UnitSkillEffect::Attack { .. } => {}
    }
    crate::sight::check_sight_in_range(
        world.resource::<TerrainGrid>(),
        attacker_position,
        attacker_footprint,
        target_cell,
        *min_range,
        *max_range,
    )?;
    Ok((target, effect))
}

fn attack_result(
    natural: i32,
    modifier: i32,
    dodge_target: i32,
    block_target: i32,
) -> AttackResult {
    let roll_degree = degree(natural, modifier, dodge_target);
    if matches!(
        roll_degree,
        RollDegree::Failure | RollDegree::CriticalFailure
    ) {
        AttackResult::Dodge
    } else if natural + modifier < block_target {
        AttackResult::Block
    } else {
        AttackResult::Hit
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TargetSide {
    Left,
    Right,
    Above,
    Below,
}

struct HitBreakdown {
    base_hit: i32,
    skill_hit: i32,
    flanking_modifier: i32,
    total: i32,
}

#[cfg(test)]
pub(crate) fn hit(
    world: &World,
    attacker: Entity,
    target: Entity,
    skill: &ActiveSkillDef,
    hit_bonus: i32,
) -> i32 {
    let HitBreakdown {
        base_hit: _,
        skill_hit: _,
        flanking_modifier: _,
        total,
    } = hit_breakdown(world, attacker, target, skill, hit_bonus);
    total
}

fn hit_breakdown(
    world: &World,
    attacker: Entity,
    target: Entity,
    skill: &ActiveSkillDef,
    hit_bonus: i32,
) -> HitBreakdown {
    let position = world.get::<Pos>(attacker).expect("攻擊者應具有位置").0;
    hit_breakdown_from_position(world, attacker, position, target, skill, hit_bonus)
}

fn hit_breakdown_from_position(
    world: &World,
    attacker: Entity,
    position: GridPos,
    target: Entity,
    skill: &ActiveSkillDef,
    hit_bonus: i32,
) -> HitBreakdown {
    let unit = world
        .get::<Unit>(attacker)
        .expect("可發動攻擊的單位應具有 Unit");
    let base_hit = unit.hit;
    let skill_hit = hit_bonus;
    let flanking_modifier = flanking_bonus(world, attacker, position, target, skill);
    HitBreakdown {
        base_hit,
        skill_hit,
        flanking_modifier,
        total: base_hit + skill_hit + flanking_modifier,
    }
}

fn flanking_bonus(
    world: &World,
    attacker: Entity,
    position: GridPos,
    target: Entity,
    skill: &ActiveSkillDef,
) -> i32 {
    let ActiveSkillDef {
        id: _,
        min_range,
        max_range,
        effect,
    } = skill;
    if !melee_enemy_skill(effect) {
        return 0;
    }
    let attacker_side = match target_side_from_position(
        world, attacker, position, target, *min_range, *max_range,
    ) {
        Some(side) => side,
        None => return 0,
    };
    let attacker_team = world
        .get::<Unit>(attacker)
        .expect("可發動攻擊的單位應具有 Unit")
        .team
        .clone();
    let ActiveSkills { definitions } = world.resource::<ActiveSkills>();
    let has_supporter = world.iter_entities().any(|entity| {
        if entity.id() == attacker || entity.id() == target {
            return false;
        }
        let Unit {
            unit_type: _,
            visual: _,
            team,
            movement: _,
            initiative: _,
            dodge: _,
            block: _,
            hit: _,
            physical_power: _,
            magical_power: _,
            block_reduction: _,
            equipment: _,
            active_skills,
            passive_skills: _,
        } = match entity.get::<Unit>() {
            Some(unit) => unit,
            None => return false,
        };
        if !crate::teams::are_allied(world, team, &attacker_team) {
            return false;
        }
        active_skills
            .iter()
            .filter_map(|skill_id| definitions.get(skill_id))
            .filter(|support_skill| melee_enemy_skill(&support_skill.effect))
            .any(|support_skill| {
                target_side_within_range(
                    world,
                    entity.id(),
                    target,
                    support_skill.min_range,
                    support_skill.max_range,
                )
                .is_some_and(|side| sides_are_opposite(attacker_side, side))
            })
    });
    if has_supporter {
        let unit = world.get::<Unit>(attacker).expect("戰鬥單位應具有 Unit");
        let PassiveSkills { definitions } = world.resource::<PassiveSkills>();
        flanking_hit_bonus(
            unit.passive_skills
                .iter()
                .map(|id| definitions.get(id).expect("單位被動技能已驗證")),
        )
    } else {
        0
    }
}

/// 夾擊只計入近戰的攻擊與推擊。
fn melee_enemy_skill(effect: &ActiveSkillEffect) -> bool {
    matches!(
        effect,
        ActiveSkillEffect::Attack { ranged: false, .. }
            | ActiveSkillEffect::Push { ranged: false, .. }
    )
}

fn target_side_within_range(
    world: &World,
    unit: Entity,
    target: Entity,
    min_range: i32,
    max_range: i32,
) -> Option<TargetSide> {
    let unit_position = world.get::<Pos>(unit).expect("參與包夾的單位應具有 Pos").0;
    target_side_from_position(world, unit, unit_position, target, min_range, max_range)
}

fn target_side_from_position(
    world: &World,
    unit: Entity,
    unit_position: GridPos,
    target: Entity,
    min_range: i32,
    max_range: i32,
) -> Option<TargetSide> {
    let unit_footprint @ Footprint {
        width: unit_width,
        height: unit_height,
    } = *world
        .get::<Footprint>(unit)
        .expect("參與包夾的單位應具有 Footprint");
    let target_position = world.get::<Pos>(target).expect("包夾目標應具有 Pos").0;
    let target_footprint @ Footprint {
        width: target_width,
        height: target_height,
    } = *world
        .get::<Footprint>(target)
        .expect("包夾目標應具有 Footprint");
    if !footprint_cells(target_position, target_footprint)
        .into_iter()
        .any(|cell| {
            crate::sight::check_sight_in_range(
                world.resource::<TerrainGrid>(),
                unit_position,
                unit_footprint,
                cell,
                min_range,
                max_range,
            )
            .is_ok()
        })
    {
        return None;
    }

    let unit_right = unit_position.x + unit_width - 1;
    let unit_bottom = unit_position.y + unit_height - 1;
    let target_right = target_position.x + target_width - 1;
    let target_bottom = target_position.y + target_height - 1;
    let rows_overlap = unit_position.y <= target_bottom && unit_bottom >= target_position.y;
    let columns_overlap = unit_position.x <= target_right && unit_right >= target_position.x;
    if rows_overlap && unit_right < target_position.x {
        Some(TargetSide::Left)
    } else if rows_overlap && unit_position.x > target_right {
        Some(TargetSide::Right)
    } else if columns_overlap && unit_bottom < target_position.y {
        Some(TargetSide::Above)
    } else if columns_overlap && unit_position.y > target_bottom {
        Some(TargetSide::Below)
    } else {
        None
    }
}

fn sides_are_opposite(first: TargetSide, second: TargetSide) -> bool {
    matches!(
        (first, second),
        (TargetSide::Left, TargetSide::Right)
            | (TargetSide::Right, TargetSide::Left)
            | (TargetSide::Above, TargetSide::Below)
            | (TargetSide::Below, TargetSide::Above)
    )
}

pub(crate) fn attack_damage(
    base_damage: i32,
    result: AttackResult,
    block_damage_reduction: i32,
    critical: bool,
) -> i32 {
    let result_damage = match result {
        AttackResult::Dodge => 0,
        AttackResult::Block => (base_damage - block_damage_reduction).max(0),
        AttackResult::Hit => base_damage,
    };
    result_damage * if critical { 2 } else { 1 }
}

pub(crate) fn can_use_skill(turn: &Turn) -> bool {
    let Turn {
        actor: _,
        phase,
        movement_remaining: _,
        movement_segments_used,
    } = turn;
    matches!(phase, Phase::PendingStrike)
        || matches!(phase, Phase::Ready | Phase::Moving) && *movement_segments_used == 0
        || matches!(phase, Phase::AfterMove) && *movement_segments_used == 1
}
pub fn degree(n: i32, m: i32, t: i32) -> RollDegree {
    if n == 1 {
        RollDegree::CriticalFailure
    } else if n == gameplay_config::HIT_DIE_SIDES as i32 {
        RollDegree::CriticalSuccess
    } else if n + m >= t {
        RollDegree::Success
    } else {
        RollDegree::Failure
    }
}

pub(crate) fn effective_dodge(w: &World, entity: Entity) -> i32 {
    let unit = w
        .get::<Unit>(entity)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    let position = w
        .get::<Pos>(entity)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let footprint = *w
        .get::<Footprint>(entity)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let penalty =
        footprint_terrain_penalty(w.resource::<TerrainGrid>(), position, footprint, |cell| {
            cell.dodge_penalty
        });
    (unit.dodge - penalty).max(0)
}

pub(crate) fn effective_block(w: &World, entity: Entity) -> i32 {
    let unit = w
        .get::<Unit>(entity)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    let position = w
        .get::<Pos>(entity)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let footprint = *w
        .get::<Footprint>(entity)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let penalty =
        footprint_terrain_penalty(w.resource::<TerrainGrid>(), position, footprint, |cell| {
            cell.block_penalty
        });
    (unit.block - penalty).max(0)
}

fn push_direction(w: &World, attacker: Entity, target_cell: GridPos) -> GridPos {
    let attacker_position = w
        .get::<Pos>(attacker)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let attacker_footprint = *w
        .get::<Footprint>(attacker)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    if attacker_position.x + attacker_footprint.width <= target_cell.x {
        GridPos { x: 1, y: 0 }
    } else if target_cell.x < attacker_position.x {
        GridPos { x: -1, y: 0 }
    } else if attacker_position.y + attacker_footprint.height <= target_cell.y {
        GridPos { x: 0, y: 1 }
    } else {
        GridPos { x: 0, y: -1 }
    }
}

pub(crate) fn skill_ranges(w: &World, e: Entity) -> Vec<ActiveSkillRangeView> {
    let board @ Board {
        width,
        height,
        terrains: _,
        terrain_types: _,
    } = w.resource::<Board>();
    let position = w.get::<Pos>(e).expect("已建立的戰鬥單位應具有 Pos 元件").0;
    let footprint = *w
        .get::<Footprint>(e)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let Unit {
        unit_type: _,
        visual: _,
        team: _,
        movement: _,
        initiative: _,
        dodge: _,
        block: _,
        hit: _,
        physical_power: _,
        magical_power: _,
        block_reduction: _,
        equipment: _,
        active_skills,
        passive_skills: _,
    } = w.get::<Unit>(e).expect("已建立的戰鬥單位應具有 Unit 元件");
    let ActiveSkills { definitions } = w.resource::<ActiveSkills>();
    let ranges: Vec<_> = active_skills
        .iter()
        .map(|skill_id| definitions.get(skill_id).expect("單位技能引用已驗證"))
        .filter_map(|skill| skill_for_current_stage(w, e, skill).ok())
        .map(|skill| {
            let skill = &skill;
            let ActiveSkillDef {
                id,
                min_range,
                max_range,
                effect,
            } = skill;
            let mut cells = Vec::new();
            // 只掃描最大射程涵蓋的方框，方框外的格子必然超出射程。
            let top = position.y.saturating_sub(*max_range).max(0);
            let bottom = (position.y + footprint.height)
                .saturating_add(*max_range)
                .min(*height);
            let left = position.x.saturating_sub(*max_range).max(0);
            let right = (position.x + footprint.width)
                .saturating_add(*max_range)
                .min(*width);
            for y in top..bottom {
                for x in left..right {
                    let cell = GridPos { x, y };
                    let cell_distance = footprint_cell_distance(position, footprint, cell);
                    let in_range = if matches!(effect, ActiveSkillEffect::MoveThenAttack) {
                        validate_step_destination(w, e, cell, skill).is_ok()
                    } else {
                        crate::sight::check_sight_in_range(
                            w.resource::<TerrainGrid>(),
                            position,
                            footprint,
                            cell,
                            *min_range,
                            *max_range,
                        )
                        .is_ok()
                    };
                    if in_range
                        && (target_kind(effect) != ActiveSkillTargetKind::Enemy
                            || cell_distance > 0)
                    {
                        cells.push(cell);
                    }
                }
            }
            ActiveSkillRangeView {
                id: id.clone(),
                details: skill_details(skill, board),
                usable: can_use_skill(w.resource::<Turn>())
                    && (!matches!(effect, ActiveSkillEffect::Loot)
                        || w.get_resource::<crate::model::Cargo>()
                            .is_some_and(|cargo| {
                                cargo.items.iter().any(|item| match item {
                                    crate::model::CargoState::Ground(target) => {
                                        crate::cargo::validate_loot(w, e, position, *target, skill)
                                            .is_ok()
                                    }
                                    crate::model::CargoState::Carried(_)
                                    | crate::model::CargoState::Escaped => false,
                                })
                            })),
                cells,
            }
        })
        .collect();
    ranges
}

fn skill_details(skill: &ActiveSkillDef, board: &Board) -> ActiveSkillDetailsView {
    let ActiveSkillDef {
        id: _,
        min_range,
        max_range,
        effect: skill_effect,
    } = skill;
    let target = target_kind(skill_effect);
    let (power_source, ranged, hit_bonus, power_bonus, effect) = match skill_effect {
        ActiveSkillEffect::Attack {
            power_source,
            ranged,
            hit_bonus,
            power_bonus,
        } => (
            Some(*power_source),
            Some(*ranged),
            Some(*hit_bonus),
            Some(*power_bonus),
            ActiveSkillDetailEffect::Attack,
        ),
        ActiveSkillEffect::Push { ranged, hit_bonus } => (
            None,
            Some(*ranged),
            Some(*hit_bonus),
            None,
            ActiveSkillDetailEffect::Push {
                distance: gameplay_config::PUSH_DISTANCE,
            },
        ),
        ActiveSkillEffect::Mire { terrain, duration } => (
            None,
            None,
            None,
            None,
            ActiveSkillDetailEffect::Mire {
                extra_movement_cost: terrain_type(board, terrain).extra_movement_cost as i32,
                duration: *duration,
            },
        ),
        ActiveSkillEffect::Heal {
            power_source,
            power_bonus,
        } => (
            Some(*power_source),
            None,
            None,
            Some(*power_bonus),
            ActiveSkillDetailEffect::Heal,
        ),
        ActiveSkillEffect::MoveThenAttack => (
            None,
            None,
            None,
            None,
            ActiveSkillDetailEffect::MoveThenAttack,
        ),
        ActiveSkillEffect::Loot => (None, None, None, None, ActiveSkillDetailEffect::Loot),
    };
    ActiveSkillDetailsView {
        power_source,
        target,
        ranged,
        min_range: *min_range,
        max_range: *max_range,
        hit_bonus,
        power_bonus,
        effect,
    }
}

fn unit_power(world: &World, entity: Entity, source: crate::PowerSource) -> i32 {
    let unit = world.get::<Unit>(entity).expect("施放者應具有 Unit 元件");
    match source {
        crate::PowerSource::Physical => unit.physical_power,
        crate::PowerSource::Magical => unit.magical_power,
    }
}

fn block_reduction(world: &World, entity: Entity) -> i32 {
    world
        .get::<Unit>(entity)
        .expect("目標應具有 Unit 元件")
        .block_reduction
}

/// 作者輸入邊界只允許已定義的被動技能，且同一效果只能選一個版本。
pub(crate) fn validate_passive_skills(
    ids: &[String],
    passives: &[PassiveSkillDef],
    owner: &str,
) -> Result<(), GameError> {
    let valid = ids
        .iter()
        .all(|id| passives.iter().any(|passive| passive.id == *id));
    if ids.len() > 1 || !valid {
        return Err(error::invalid_passive_skills(owner));
    }
    Ok(())
}

pub(crate) fn flanking_hit_bonus<'a>(
    passives: impl IntoIterator<Item = &'a PassiveSkillDef>,
) -> i32 {
    passives
        .into_iter()
        .map(|passive| match passive.effect {
            PassiveSkillEffect::Flanking { hit_bonus } => hit_bonus,
        })
        .next()
        .unwrap_or(0)
}

/// 單位查看只提供技能說明，不計算操作可用性與射程地格。
pub(crate) fn active_skill_views(
    world: &World,
    entity: Entity,
) -> Vec<crate::model::ActiveSkillView> {
    let unit = world.get::<Unit>(entity).expect("戰鬥單位應具有 Unit");
    let ActiveSkills { definitions } = world.resource::<ActiveSkills>();
    let board = world.resource::<Board>();
    unit.active_skills
        .iter()
        .map(|id| definitions.get(id).expect("單位技能已驗證"))
        .map(|skill| crate::model::ActiveSkillView {
            id: skill.id.clone(),
            details: skill_details(skill, board),
        })
        .collect()
}

pub(crate) fn passive_skill_views(
    world: &World,
    entity: Entity,
) -> Vec<crate::model::PassiveSkillView> {
    let unit = world.get::<Unit>(entity).expect("戰鬥單位應具有 Unit");
    let PassiveSkills { definitions } = world.resource::<PassiveSkills>();
    unit.passive_skills
        .iter()
        .map(|id| definitions.get(id).expect("單位被動技能已驗證"))
        .map(|passive| match passive.effect {
            PassiveSkillEffect::Flanking { hit_bonus } => crate::model::PassiveSkillView {
                id: passive.id.clone(),
                hit_bonus,
            },
        })
        .collect()
}

/// 載入與編輯器共用被動技能分配；同效果由單位版本覆蓋全體預設。
pub(crate) fn resolve_passive_skills(
    unit: &crate::authoring::UnitType,
    defaults: &[String],
    passives: &[PassiveSkillDef],
) -> Result<Vec<String>, GameError> {
    validate_passive_skills(&unit.passive_skills, passives, &unit.id)?;
    let mut resolved: Vec<String> = Vec::new();
    for id in defaults.iter().chain(&unit.passive_skills) {
        let passive = passives
            .iter()
            .find(|passive| passive.id == *id)
            .expect("預設與單位被動技能已在作者輸入邊界驗證");
        let effect = std::mem::discriminant(&passive.effect);
        match resolved.iter().position(|existing| {
            let definition = passives
                .iter()
                .find(|skill| skill.id == *existing)
                .expect("已解析的被動技能應存在");
            std::mem::discriminant(&definition.effect) == effect
        }) {
            Some(index) => resolved[index] = id.clone(),
            None => resolved.push(id.clone()),
        }
    }
    Ok(resolved)
}
