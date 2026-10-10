//! 技能驗證、預覽、效果與戰鬥計算。
use crate::error::{self, GameError};
use crate::game::{DamageResult, Game, die};
use crate::gameplay_config;
use crate::model::{
    AttackPreview, AttackResult, Board, CollisionUnitLog, CombatLogEvent, Encounter, Footprint,
    GridPos, HealingPreview, HealthSegmentsView, Hp, Id, Log, MovementTransition, Phase, Pos,
    RollDegree, SkillDef, SkillDetailEffect, SkillDetailsView, SkillEffect, SkillPreview,
    SkillRangeView, SkillTargetKind, Skills, TemporaryTerrain, TemporaryTerrains, Turn, Unit,
};
use crate::movement::{fits, footprint_cell_distance, footprint_cells, overlap, unit_at_cell};
use crate::terrain::{
    TerrainEntry, footprint_blocks_push, footprint_on_impassable, footprint_terrain_penalty,
    terrain_type,
};
use bevy_ecs::prelude::{Entity, World};

impl Game {
    pub fn preview_skill(
        &self,
        actor: i64,
        target_cell: GridPos,
        skill_id: &str,
    ) -> Result<SkillPreview, GameError> {
        self.ensure(actor)?;
        if !can_use_skill(self.world.resource::<Turn>()) {
            return Err(error::cannot_use_skill());
        }
        let attacker = self.entity(actor).ok_or(error::missing_attacker())?;
        let Skills { definitions } = self.world.resource::<Skills>();
        let skill = definitions
            .get(skill_id)
            .ok_or_else(|| error::unknown_skill(skill_id))?;
        let position = self.world.get::<Pos>(attacker).expect("施放者應具有位置").0;
        preview_unit_skill_from_position(&self.world, attacker, position, target_cell, skill)
    }
    pub(crate) fn use_skill_at_cell(
        &mut self,
        actor: i64,
        position: GridPos,
        skill: SkillDef,
    ) -> Result<(), GameError> {
        if target_kind(&skill.effect) == SkillTargetKind::Cell {
            return self.use_cell_skill(actor, position, skill);
        }
        let a = actor;
        let target_cell = position;
        self.ensure(a)?;
        let turn = self.world.resource::<Turn>();
        if !can_use_skill(turn) {
            return Err(error::cannot_use_skill());
        }
        let ae = self.entity(a).ok_or(error::missing_attacker())?;
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
            attack: _,
            physical_power: _,
            magical_power: _,
            block_reduction: _,
            equipment: _,
            skills: _,
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
            attack: _,
            physical_power: _,
            magical_power: _,
            block_reduction: _,
            equipment: _,
            skills: _,
        } = self
            .world
            .get::<Unit>(te)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .clone();
        let (attack_bonus, power_bonus, push) = match effect {
            UnitSkillEffect::Attack {
                attack_bonus,
                power_bonus,
                push,
            } => (attack_bonus, power_bonus, push),
            UnitSkillEffect::Heal { power_bonus } => {
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
                } = healing_preview(&self.world, ae, te, power_bonus, skill.power_source);
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
        let AttackModifierBreakdown {
            attack_stat_modifier,
            skill_attack_modifier,
            flanking_modifier,
            total: modifier,
        } = attack_modifier_breakdown(&self.world, ae, te, &skill, attack_bonus);
        let natural = die(&mut self.world, gameplay_config::ATTACK_DIE_SIDES) as i32;
        let target_dodge = effective_dodge(&self.world, te);
        let target_block = effective_block(&self.world, te);
        let degree = degree(
            natural,
            modifier,
            gameplay_config::BASE_DEFENSE + target_dodge,
        );
        let result = attack_result(
            natural,
            modifier,
            gameplay_config::BASE_DEFENSE + target_dodge,
            gameplay_config::BASE_DEFENSE + target_dodge + target_block,
        );
        let base_damage = direct_damage(&self.world, ae, skill.power_source, power_bonus);
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
        let mut pushed = false;
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
            let terrain_allows_push = fits(self.world.resource::<Board>(), destination, footprint)
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
                self.world
                    .get_mut::<Pos>(te)
                    .expect("已建立的戰鬥單位應具有 Pos 元件")
                    .0 = destination;
                // 推移在技能結果之後、進入地形之前呈現，包含推入後陣亡的單位。
                self.movements.push(MovementTransition {
                    unit_id: target,
                    path: vec![current, destination],
                    before_log_index: self.world.resource::<Log>().0.len() + 1,
                });
                pushed = true;
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
                        attack: _,
                        physical_power: _,
                        magical_power: _,
                        block_reduction: _,
                        equipment: _,
                        skills: _,
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
                die_sides: gameplay_config::ATTACK_DIE_SIDES,
                attack_stat_modifier,
                skill_attack_modifier,
                flanking_modifier,
                attack_modifier: modifier,
                attack_total: natural + modifier,
                dodge_target: gameplay_config::BASE_DEFENSE + target_dodge,
                block_target: gameplay_config::BASE_DEFENSE + target_dodge + target_block,
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
        if pushed {
            self.apply_terrain_entry(te, target, TerrainEntry::Push);
        }
        self.finish();
        Ok(())
    }
    fn use_cell_skill(
        &mut self,
        actor: i64,
        position: GridPos,
        skill: SkillDef,
    ) -> Result<(), GameError> {
        self.ensure(actor)?;
        if !can_use_skill(self.world.resource::<Turn>()) {
            return Err(error::cannot_use_skill());
        }
        let entity = self.entity(actor).ok_or(error::missing_actor())?;
        let origin = self.world.get::<Pos>(entity).expect("施放者應具有位置").0;
        let (terrain, duration) =
            validate_cell_skill_from_position(&self.world, entity, origin, position, &skill)?;
        let unit = self.world.get::<Unit>(entity).expect("施放者應具有 Unit");
        let actor_type = unit.unit_type.clone();
        let actor_team = unit.team.clone();
        let SkillDef {
            power_source: _,
            id,
            ranged: _,
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
        let current_round = *current_round;
        self.world
            .resource_mut::<TemporaryTerrains>()
            .0
            .entry(position)
            .or_default()
            .insert(
                terrain.clone(),
                TemporaryTerrain {
                    expires_after_round: current_round + duration - 1,
                },
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
    skill: &SkillDef,
) -> Result<(String, u32), GameError> {
    let unit = world.get::<Unit>(actor).expect("施放者應具有 Unit");
    if !unit.skills.contains(&skill.id) {
        return Err(error::unit_cannot_use_skill());
    }
    let (terrain, duration) = match &skill.effect {
        SkillEffect::Mire { terrain, duration } => (terrain.clone(), *duration),
        _ => return Err(error::unit_cannot_use_skill()),
    };
    if !fits(
        world.resource::<Board>(),
        cell,
        Footprint {
            width: 1,
            height: 1,
        },
    ) {
        return Err(error::target_cell_out_of_bounds());
    }
    if footprint_on_impassable(
        world,
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
        world,
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
    skill: &SkillDef,
) -> Result<SkillPreview, GameError> {
    let (target_entity, effect) =
        validate_unit_skill_target_from_position(world, attacker, position, target_cell, skill)?;
    let (attack_bonus, power_bonus) = match effect {
        UnitSkillEffect::Heal { power_bonus } => {
            return Ok(SkillPreview::Healing(healing_preview(
                world,
                attacker,
                target_entity,
                power_bonus,
                skill.power_source,
            )));
        }
        UnitSkillEffect::Attack {
            attack_bonus,
            power_bonus,
            push: _,
        } => (attack_bonus, power_bonus),
    };

    let Unit {
        unit_type: _,
        visual: _,
        team: _,
        movement: _,
        initiative: _,
        dodge: _,
        block: _,
        attack: _,
        physical_power: _,
        magical_power: _,
        block_reduction: _,
        equipment: _,
        skills: _,
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
        attack: _,
        physical_power: _,
        magical_power: _,
        block_reduction: _,
        equipment: _,
        skills: _,
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
    let AttackModifierBreakdown {
        attack_stat_modifier: _,
        skill_attack_modifier: _,
        flanking_modifier: _,
        total: modifier,
    } = attack_modifier_breakdown_from_position(
        world,
        attacker,
        position,
        target_entity,
        skill,
        attack_bonus,
    );
    let dodge_target = gameplay_config::BASE_DEFENSE + effective_dodge(world, target_entity);
    let block_target = dodge_target + effective_block(world, target_entity);
    let mut dodge_count = 0;
    let mut block_count = 0;
    let mut hit_count = 0;
    for natural in 1..=gameplay_config::ATTACK_DIE_SIDES as i32 {
        match attack_result(natural, modifier, dodge_target, block_target) {
            AttackResult::Dodge => dodge_count += 1,
            AttackResult::Block => block_count += 1,
            AttackResult::Hit => hit_count += 1,
        }
    }
    let hit_damage = direct_damage(world, attacker, skill.power_source, power_bonus);
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
    Ok(SkillPreview::Attack(AttackPreview {
        target,
        target_type: target_type.clone(),
        target_hp: *target_hp,
        target_max_hp: *target_max_hp,
        target_mana: gameplay_config::DEFAULT_MANA,
        hit_remaining_hp,
        block_remaining_hp,
        dodge_remaining_hp: *target_hp,
        dodge_chance: dodge_count * 100 / gameplay_config::ATTACK_DIE_SIDES,
        block_chance: block_count * 100 / gameplay_config::ATTACK_DIE_SIDES,
        hit_chance: hit_count * 100 / gameplay_config::ATTACK_DIE_SIDES,
        critical_chance: 100 / gameplay_config::ATTACK_DIE_SIDES,
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
fn direct_damage(
    world: &World,
    actor: Entity,
    source: crate::PowerSource,
    power_bonus: Option<i32>,
) -> i32 {
    match power_bonus {
        Some(bonus) => skill_power(unit_power(world, actor, source), bonus),
        None => 0,
    }
}

/// 技能造成的基礎傷害或治療量；加值為負時最低為 0，不會反轉成治療或傷害。
fn skill_power(power: i32, power_bonus: i32) -> i32 {
    (power + power_bonus).max(0)
}

/// 技能指定的目標種類；以技能種類決定行為時一律經過此函式。
pub(crate) fn target_kind(effect: &SkillEffect) -> SkillTargetKind {
    match effect {
        SkillEffect::Attack { .. } | SkillEffect::Push { .. } => SkillTargetKind::Enemy,
        SkillEffect::Heal { .. } => SkillTargetKind::Ally,
        SkillEffect::Mire { .. } => SkillTargetKind::Cell,
        SkillEffect::Flanking { .. } => SkillTargetKind::Passive,
    }
}

/// 以單位為目標的技能效果，只能由 validate_unit_skill_target 驗證後取得。
enum UnitSkillEffect {
    Attack {
        attack_bonus: i32,
        power_bonus: Option<i32>,
        push: bool,
    },
    Heal {
        power_bonus: i32,
    },
}

// 從所選格解析目標並驗證技能；呼叫端不再分別傳入目標 ID 與格子。
fn validate_unit_skill_target(
    world: &World,
    attacker: Entity,
    target_cell: GridPos,
    skill: &SkillDef,
) -> Result<(Entity, UnitSkillEffect), GameError> {
    let position = world.get::<Pos>(attacker).expect("施放者應具有位置").0;
    validate_unit_skill_target_from_position(world, attacker, position, target_cell, skill)
}

fn validate_unit_skill_target_from_position(
    world: &World,
    attacker: Entity,
    attacker_position: GridPos,
    target_cell: GridPos,
    skill: &SkillDef,
) -> Result<(Entity, UnitSkillEffect), GameError> {
    let SkillDef {
        power_source: _,
        id,
        ranged: _,
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
        attack: _,
        physical_power: _,
        magical_power: _,
        block_reduction: _,
        equipment: _,
        skills,
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
        attack: _,
        physical_power: _,
        magical_power: _,
        block_reduction: _,
        equipment: _,
        skills: _,
    } = world
        .get::<Unit>(target)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    if !skills.contains(id) {
        return Err(error::unit_cannot_use_skill());
    }
    let effect = match skill_effect {
        SkillEffect::Attack {
            attack_bonus,
            power_bonus,
        } => UnitSkillEffect::Attack {
            attack_bonus: *attack_bonus,
            power_bonus: Some(*power_bonus),
            push: false,
        },
        SkillEffect::Push { attack_bonus } => UnitSkillEffect::Attack {
            attack_bonus: *attack_bonus,
            power_bonus: None,
            push: true,
        },
        SkillEffect::Heal { power_bonus } => UnitSkillEffect::Heal {
            power_bonus: *power_bonus,
        },
        SkillEffect::Mire { .. } | SkillEffect::Flanking { .. } => {
            return Err(error::unit_cannot_use_skill());
        }
    };
    match effect {
        UnitSkillEffect::Heal { .. } if attacker_team != target_team => {
            return Err(error::heal_allies_only());
        }
        UnitSkillEffect::Attack { .. } if attacker_team == target_team => {
            return Err(error::cannot_attack_ally());
        }
        UnitSkillEffect::Heal { .. } | UnitSkillEffect::Attack { .. } => {}
    }
    crate::sight::check_sight_in_range(
        world,
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

struct AttackModifierBreakdown {
    attack_stat_modifier: i32,
    skill_attack_modifier: i32,
    flanking_modifier: i32,
    total: i32,
}

#[cfg(test)]
pub(crate) fn attack_modifier(
    world: &World,
    attacker: Entity,
    target: Entity,
    skill: &SkillDef,
    attack_bonus: i32,
) -> i32 {
    let AttackModifierBreakdown {
        attack_stat_modifier: _,
        skill_attack_modifier: _,
        flanking_modifier: _,
        total,
    } = attack_modifier_breakdown(world, attacker, target, skill, attack_bonus);
    total
}

fn attack_modifier_breakdown(
    world: &World,
    attacker: Entity,
    target: Entity,
    skill: &SkillDef,
    attack_bonus: i32,
) -> AttackModifierBreakdown {
    let position = world.get::<Pos>(attacker).expect("攻擊者應具有位置").0;
    attack_modifier_breakdown_from_position(world, attacker, position, target, skill, attack_bonus)
}

fn attack_modifier_breakdown_from_position(
    world: &World,
    attacker: Entity,
    position: GridPos,
    target: Entity,
    skill: &SkillDef,
    attack_bonus: i32,
) -> AttackModifierBreakdown {
    let unit = world
        .get::<Unit>(attacker)
        .expect("可發動攻擊的單位應具有 Unit");
    let attack_stat_modifier = unit.attack;
    let skill_attack_modifier = attack_bonus;
    let flanking_modifier = flanking_bonus(world, attacker, position, target, skill);
    AttackModifierBreakdown {
        attack_stat_modifier,
        skill_attack_modifier,
        flanking_modifier,
        total: attack_stat_modifier + skill_attack_modifier + flanking_modifier,
    }
}

fn flanking_bonus(
    world: &World,
    attacker: Entity,
    position: GridPos,
    target: Entity,
    skill: &SkillDef,
) -> i32 {
    let SkillDef {
        power_source: _,
        id: _,
        ranged,
        min_range,
        max_range,
        effect: _,
    } = skill;
    if *ranged {
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
    let Skills { definitions } = world.resource::<Skills>();
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
            attack: _,
            physical_power: _,
            magical_power: _,
            block_reduction: _,
            equipment: _,
            skills,
        } = match entity.get::<Unit>() {
            Some(unit) => unit,
            None => return false,
        };
        if *team != attacker_team {
            return false;
        }
        skills
            .iter()
            .filter_map(|skill_id| definitions.get(skill_id))
            .filter(|support_skill| {
                !support_skill.ranged
                    && target_kind(&support_skill.effect) == SkillTargetKind::Enemy
            })
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
        let unit = world.get::<Unit>(attacker).expect("攻擊者應具有 Unit");
        flanking_attack_bonus(
            unit.skills
                .iter()
                .map(|id| definitions.get(id).expect("單位技能引用已驗證")),
        )
    } else {
        0
    }
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
                world,
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
    matches!(phase, Phase::Ready | Phase::Moving) && *movement_segments_used == 0
        || matches!(phase, Phase::AfterMove) && *movement_segments_used == 1
}
pub fn degree(n: i32, m: i32, t: i32) -> RollDegree {
    if n == 1 {
        RollDegree::CriticalFailure
    } else if n == gameplay_config::ATTACK_DIE_SIDES as i32 {
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
        footprint_terrain_penalty(w, position, footprint, |terrain| terrain.dodge_penalty);
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
        footprint_terrain_penalty(w, position, footprint, |terrain| terrain.block_penalty);
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

pub(crate) fn skill_ranges(w: &World, e: Entity) -> Vec<SkillRangeView> {
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
        attack: _,
        physical_power: _,
        magical_power: _,
        block_reduction: _,
        equipment: _,
        skills,
    } = w.get::<Unit>(e).expect("已建立的戰鬥單位應具有 Unit 元件");
    let Skills { definitions } = w.resource::<Skills>();
    let ranges: Vec<_> = skills
        .iter()
        .filter_map(|skill_id| definitions.get(skill_id))
        .filter(|skill| !matches!(skill.effect, SkillEffect::Flanking { .. }))
        .map(|skill| {
            let SkillDef {
                power_source: _,
                id,
                ranged: _,
                min_range,
                max_range,
                effect,
            } = skill;
            let mut cells = Vec::new();
            for y in 0..*height {
                for x in 0..*width {
                    let cell = GridPos { x, y };
                    let cell_distance = footprint_cell_distance(position, footprint, cell);
                    if crate::sight::check_sight_in_range(
                        w, position, footprint, cell, *min_range, *max_range,
                    )
                    .is_ok()
                        && (target_kind(effect) != SkillTargetKind::Enemy || cell_distance > 0)
                    {
                        cells.push(cell);
                    }
                }
            }
            SkillRangeView {
                id: id.clone(),
                details: skill_details(skill, board),
                usable: can_use_skill(w.resource::<Turn>()),
                cells,
            }
        })
        .collect();
    ranges
}

fn skill_details(skill: &SkillDef, board: &Board) -> SkillDetailsView {
    let SkillDef {
        power_source,
        id: _,
        ranged,
        min_range,
        max_range,
        effect: skill_effect,
    } = skill;
    let target = target_kind(skill_effect);
    let (attack_bonus, power_bonus, effect) = match skill_effect {
        SkillEffect::Attack {
            attack_bonus,
            power_bonus,
        } => (
            Some(*attack_bonus),
            Some(*power_bonus),
            SkillDetailEffect::Attack,
        ),
        SkillEffect::Push { attack_bonus } => (
            Some(*attack_bonus),
            None,
            SkillDetailEffect::Push {
                distance: gameplay_config::PUSH_DISTANCE,
            },
        ),
        SkillEffect::Mire { terrain, duration } => (
            None,
            None,
            SkillDetailEffect::Mire {
                extra_movement_cost: terrain_type(board, terrain).extra_movement_cost as i32,
                duration: *duration,
            },
        ),
        SkillEffect::Heal { power_bonus } => (None, Some(*power_bonus), SkillDetailEffect::Heal),
        SkillEffect::Flanking { .. } => unreachable!("被動技能不建立主動技能範圍"),
    };
    SkillDetailsView {
        power_source: *power_source,
        target,
        ranged: *ranged,
        min_range: *min_range,
        max_range: *max_range,
        attack_bonus,
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
    skills: &[SkillDef],
    owner: &str,
) -> Result<(), GameError> {
    let valid = ids.iter().all(|id| {
        skills.iter().any(|skill| {
            skill.id == *id && matches!(skill.effect, SkillEffect::Flanking { attack_bonus } if attack_bonus >= 0)
        })
    });
    if ids.len() > 1 || !valid {
        return Err(error::invalid_passive_skills(owner));
    }
    Ok(())
}

pub(crate) fn flanking_attack_bonus<'a>(skills: impl Iterator<Item = &'a SkillDef>) -> i32 {
    skills
        .filter_map(|skill| match skill.effect {
            SkillEffect::Flanking { attack_bonus } => Some(attack_bonus),
            _ => None,
        })
        .next()
        .unwrap_or(0)
}

pub(crate) fn passive_skill_views(
    world: &World,
    entity: Entity,
) -> Vec<crate::model::PassiveSkillView> {
    let unit = world.get::<Unit>(entity).expect("戰鬥單位應具有 Unit");
    let Skills { definitions } = world.resource::<Skills>();
    unit.skills
        .iter()
        .filter_map(|id| {
            let skill = definitions.get(id).expect("單位技能已驗證");
            match skill.effect {
                SkillEffect::Flanking { attack_bonus } => Some(crate::model::PassiveSkillView {
                    id: id.clone(),
                    attack_bonus,
                }),
                _ => None,
            }
        })
        .collect()
}

/// 載入與編輯器共用技能分配，執行期只保存已決定的完整技能清單。
pub(crate) fn resolve_unit_skills(
    unit: &crate::authoring::UnitType,
    defaults: &[String],
    skills: &[SkillDef],
) -> Result<Vec<String>, GameError> {
    validate_passive_skills(&unit.passive_skills, skills, &unit.id)?;
    let mut passives: Vec<&SkillDef> = Vec::new();
    for id in defaults.iter().chain(&unit.passive_skills) {
        let skill = skills
            .iter()
            .find(|skill| skill.id == *id)
            .expect("預設與單位被動技能已在作者輸入邊界驗證");
        let effect = std::mem::discriminant(&skill.effect);
        match passives
            .iter()
            .position(|existing| std::mem::discriminant(&existing.effect) == effect)
        {
            Some(index) => passives[index] = skill,
            None => passives.push(skill),
        }
    }
    if unit.skills.iter().any(|id| {
        skills
            .iter()
            .any(|skill| skill.id == *id && matches!(skill.effect, SkillEffect::Flanking { .. }))
    }) {
        return Err(error::invalid_passive_skills(&unit.id));
    }
    Ok(unit
        .skills
        .iter()
        .chain(passives.iter().map(|skill| &skill.id))
        .cloned()
        .collect())
}
