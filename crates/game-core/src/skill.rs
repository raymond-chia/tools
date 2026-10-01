//! 技能驗證、預覽、效果與戰鬥計算。
use crate::error::{self, GameError};
use crate::game::{DamageResult, Game, die};
use crate::gameplay_config;
use crate::model::{
    AttackPreview, AttackResult, Board, CollisionUnitLog, CombatLogEvent, Encounter, Footprint,
    GridPos, HealingPreview, HealthSegmentsView, Hp, Id, Log, Phase, Pos, RollDegree, SkillDef,
    SkillDetailEffect, SkillDetailsView, SkillEffect, SkillPreview, SkillRangeView,
    SkillTargetKind, Skills, TemporaryTerrain, TemporaryTerrains, TerrainEntryRule, TerrainTypeDef,
    Turn, Unit,
};
use crate::movement::{
    fits, footprint_blocks_push, footprint_cells, footprint_distance, overlap, terrain_type,
    terrains_at, unit_at_cell,
};
use bevy_ecs::prelude::{Entity, World};
use std::collections::HashSet;

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
        let target_entity = unit_at_cell(&self.world, target_cell).ok_or(error::missing_target())?;
        let skill = self
            .world
            .resource::<Skills>()
            .definitions
            .get(skill_id)
            .ok_or_else(|| error::unknown_skill(skill_id))?;
        let (attack_bonus, power_bonus) = match validate_unit_skill_target(
            &self.world,
            attacker,
            target_entity,
            target_cell,
            skill,
        )? {
            UnitSkillEffect::Heal { power_bonus } => {
                return Ok(SkillPreview::Healing(healing_preview(
                    &self.world,
                    attacker,
                    target_entity,
                    power_bonus,
                )));
            }
            UnitSkillEffect::Attack {
                attack_bonus,
                power_bonus,
                push: _,
            } => (attack_bonus, power_bonus),
        };

        let attacker_unit = self
            .world
            .get::<Unit>(attacker)
            .expect("已建立的戰鬥單位應具有 Unit 元件");
        let target_unit = self
            .world
            .get::<Unit>(target_entity)
            .expect("已建立的戰鬥單位應具有 Unit 元件");
        let target = self
            .world
            .get::<Id>(target_entity)
            .expect("目標單位應具有 Id")
            .0;
        let target_hp = self
            .world
            .get::<Hp>(target_entity)
            .expect("已建立的戰鬥單位應具有 Hp 元件");
        let modifier = attack_modifier(&self.world, attacker, target_entity, skill, attack_bonus);
        let dodge_target =
            gameplay_config::BASE_DEFENSE + effective_dodge(&self.world, target_entity);
        let block_target = dodge_target + effective_block(&self.world, target_entity);
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
        let hit_damage = skill_power(attacker_unit.power, power_bonus);
        let block_damage = attack_damage(
            hit_damage,
            AttackResult::Block,
            gameplay_config::BLOCK_DAMAGE_REDUCTION,
            false,
        );
        let hit_remaining_hp = (target_hp.current - hit_damage).max(0);
        let block_remaining_hp = (target_hp.current - block_damage).max(0);
        let block_segment = if block_count > 0 {
            block_remaining_hp - hit_remaining_hp
        } else {
            0
        };
        Ok(SkillPreview::Attack(AttackPreview {
            target,
            target_type: target_unit.unit_type.clone(),
            target_hp: target_hp.current,
            target_max_hp: target_hp.maximum,
            target_mana: gameplay_config::DEFAULT_MANA,
            hit_remaining_hp,
            block_remaining_hp,
            dodge_remaining_hp: target_hp.current,
            dodge_chance: dodge_count * 100 / gameplay_config::ATTACK_DIE_SIDES,
            block_chance: block_count * 100 / gameplay_config::ATTACK_DIE_SIDES,
            hit_chance: hit_count * 100 / gameplay_config::ATTACK_DIE_SIDES,
            critical_chance: 100 / gameplay_config::ATTACK_DIE_SIDES,
            dodge_damage: 0,
            block_damage,
            critical_block_damage: attack_damage(
                hit_damage,
                AttackResult::Block,
                gameplay_config::BLOCK_DAMAGE_REDUCTION,
                true,
            ),
            hit_damage,
            critical_hit_damage: attack_damage(
                hit_damage,
                AttackResult::Hit,
                gameplay_config::BLOCK_DAMAGE_REDUCTION,
                true,
            ),
            health_segments: HealthSegmentsView {
                hit: hit_remaining_hp,
                block: block_segment,
                damage: target_hp.current - hit_remaining_hp - block_segment,
                missing: target_hp.maximum - target_hp.current,
            },
        }))
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
        let target = unit_at_cell(&self.world, position)
            .and_then(|entity| self.world.get::<Id>(entity).map(|id| id.0))
            .ok_or(error::missing_target())?;
        self.use_skill(actor, target, position, skill)
    }
    pub(crate) fn use_skill(
        &mut self,
        a: i64,
        target: i64,
        target_cell: GridPos,
        skill: SkillDef,
    ) -> Result<(), GameError> {
        self.ensure(a)?;
        let turn = self.world.resource::<Turn>();
        if !can_use_skill(turn) {
            return Err(error::cannot_use_skill());
        }
        let ae = self.entity(a).ok_or(error::missing_attacker())?;
        let te = self.entity(target).ok_or(error::missing_target())?;
        let effect = validate_unit_skill_target(&self.world, ae, te, target_cell, &skill)?;
        let attacker_unit = self
            .world
            .get::<Unit>(ae)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .clone();
        let target_unit = self
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
                } = healing_preview(&self.world, ae, te, power_bonus);
                self.world
                    .get_mut::<Hp>(te)
                    .expect("已建立的戰鬥單位應具有 Hp 元件")
                    .current = remaining_hp;
                self.world
                    .resource_mut::<Log>()
                    .0
                    .push(CombatLogEvent::Healing {
                        actor: a,
                        actor_type: attacker_unit.unit_type,
                        actor_team: attacker_unit.team.clone(),
                        skill: skill.id,
                        target: target_id,
                        target_type,
                        target_team: target_unit.team.clone(),
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
        let base_damage = skill_power(attacker_unit.power, power_bonus);
        let critical = degree == RollDegree::CriticalSuccess;
        let raw_damage = attack_damage(
            base_damage,
            AttackResult::Hit,
            gameplay_config::BLOCK_DAMAGE_REDUCTION,
            critical,
        );
        let damage = attack_damage(
            base_damage,
            result,
            gameplay_config::BLOCK_DAMAGE_REDUCTION,
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
                && !footprint_blocks_push(self.world.resource::<Board>(), destination, footprint);
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
                    let unit = self
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
                        unit_type: unit.unit_type,
                        team: unit.team.clone(),
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
                actor_type: attacker_unit.unit_type,
                actor_team: attacker_unit.team.clone(),
                skill: skill.id,
                target,
                target_type: target_unit.unit_type,
                target_team: target_unit.team.clone(),
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
            self.apply_pushed_terrain(te, target);
        }
        self.finish();
        Ok(())
    }
    fn apply_pushed_terrain(&mut self, entity: Entity, id: i64) {
        let position = self
            .world
            .get::<Pos>(entity)
            .expect("已建立的戰鬥單位應具有 Pos 元件")
            .0;
        for terrain in terrains_at(&self.world, position) {
            let terrain_definition = terrain_type(self.world.resource::<Board>(), &terrain);
            let instant_down =
                terrain_definition.entry_rule == TerrainEntryRule::InstantDownWhenPushed;
            let damage = if instant_down {
                self.world
                    .get::<Hp>(entity)
                    .expect("被推動的單位應具有 Hp")
                    .current
            } else {
                terrain_definition.damage
            };
            if damage == 0 {
                continue;
            }
            let unit = self
                .world
                .get::<Unit>(entity)
                .expect("已建立的戰鬥單位應具有 Unit 元件");
            let target_type = unit.unit_type.clone();
            let target_team = unit.team.clone();
            let DamageResult {
                remaining_hp,
                max_hp,
                downed,
            } = self.apply_damage(entity, id, damage);
            self.world
                .resource_mut::<Log>()
                .0
                .push(CombatLogEvent::TerrainDamage {
                    target: id,
                    target_type,
                    target_team,
                    terrain,
                    instant_down,
                    damage,
                    remaining_hp,
                    max_hp,
                    downed,
                });
            if downed {
                break;
            }
        }
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
        let unit = self
            .world
            .get::<Unit>(entity)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .clone();
        if !unit.skills.contains(&skill.id) {
            return Err(error::unit_cannot_use_skill());
        }
        let (terrain, duration) = match &skill.effect {
            SkillEffect::Mire { terrain, duration } => (terrain.clone(), *duration),
            SkillEffect::Attack { .. } | SkillEffect::Push { .. } | SkillEffect::Heal { .. } => {
                return Err(error::unit_cannot_use_skill());
            }
        };
        let board = self.world.resource::<Board>();
        if !fits(
            board,
            position,
            Footprint {
                width: 1,
                height: 1,
            },
        ) {
            return Err(error::target_cell_out_of_bounds());
        }
        let origin = self
            .world
            .get::<Pos>(entity)
            .expect("已建立的戰鬥單位應具有 Pos 元件")
            .0;
        let footprint = *self
            .world
            .get::<Footprint>(entity)
            .expect("已建立的戰鬥單位應具有 Footprint 元件");
        let target_distance = footprint_distance(
            origin,
            footprint,
            position,
            Footprint {
                width: 1,
                height: 1,
            },
        );
        if target_distance < skill.min_range {
            return Err(error::target_too_close());
        }
        if target_distance > skill.max_range {
            return Err(error::target_too_far());
        }
        let current_round = self.world.resource::<Encounter>().round;
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
                actor_type: unit.unit_type,
                actor_team: unit.team.clone(),
                skill: skill.id,
                terrain,
            });
        self.finish();
        Ok(())
    }
}

fn healing_preview(
    world: &World,
    actor: Entity,
    target: Entity,
    power_bonus: i32,
) -> HealingPreview {
    let hp = world
        .get::<Hp>(target)
        .expect("已建立的戰鬥單位應具有 Hp 元件");
    let power = world
        .get::<Unit>(actor)
        .expect("施放者應具有 Unit 元件")
        .power;
    let remaining_hp = (hp.current + skill_power(power, power_bonus)).min(hp.maximum);
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
        target_hp: hp.current,
        target_max_hp: hp.maximum,
        target_mana: gameplay_config::DEFAULT_MANA,
        healing: remaining_hp - hp.current,
        remaining_hp,
        missing_hp: hp.maximum - remaining_hp,
        health_segments: HealthSegmentsView {
            hit: hp.current,
            block: 0,
            damage: 0,
            missing: hp.maximum - remaining_hp,
        },
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
    }
}

/// 以單位為目標的技能效果，只能由 validate_unit_skill_target 驗證後取得。
enum UnitSkillEffect {
    Attack {
        attack_bonus: i32,
        power_bonus: i32,
        push: bool,
    },
    Heal {
        power_bonus: i32,
    },
}

fn validate_unit_skill_target(
    world: &World,
    attacker: Entity,
    target: Entity,
    target_cell: GridPos,
    skill: &SkillDef,
) -> Result<UnitSkillEffect, GameError> {
    let target_position = world
        .get::<Pos>(target)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let target_footprint = *world
        .get::<Footprint>(target)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    if !overlap(
        target_cell,
        Footprint {
            width: 1,
            height: 1,
        },
        target_position,
        target_footprint,
    ) {
        return Err(error::cell_not_on_target());
    }
    let attacker_unit = world
        .get::<Unit>(attacker)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    let target_unit = world
        .get::<Unit>(target)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    if !attacker_unit.skills.contains(&skill.id) {
        return Err(error::unit_cannot_use_skill());
    }
    let effect = match &skill.effect {
        SkillEffect::Attack {
            attack_bonus,
            power_bonus,
        } => UnitSkillEffect::Attack {
            attack_bonus: *attack_bonus,
            power_bonus: *power_bonus,
            push: false,
        },
        SkillEffect::Push {
            attack_bonus,
            power_bonus,
        } => UnitSkillEffect::Attack {
            attack_bonus: *attack_bonus,
            power_bonus: *power_bonus,
            push: true,
        },
        SkillEffect::Heal { power_bonus } => UnitSkillEffect::Heal {
            power_bonus: *power_bonus,
        },
        SkillEffect::Mire { .. } => return Err(error::unit_cannot_use_skill()),
    };
    match effect {
        UnitSkillEffect::Heal { .. } if attacker_unit.team != target_unit.team => {
            return Err(error::heal_allies_only());
        }
        UnitSkillEffect::Attack { .. } if attacker_unit.team == target_unit.team => {
            return Err(error::cannot_attack_ally());
        }
        UnitSkillEffect::Heal { .. } | UnitSkillEffect::Attack { .. } => {}
    }
    let attacker_position = world
        .get::<Pos>(attacker)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let attacker_footprint = *world
        .get::<Footprint>(attacker)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let target_distance = footprint_distance(
        attacker_position,
        attacker_footprint,
        target_cell,
        Footprint {
            width: 1,
            height: 1,
        },
    );
    if target_distance < skill.min_range {
        return Err(error::target_too_close());
    }
    if target_distance > skill.max_range {
        return Err(error::target_too_far());
    }
    Ok(effect)
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

pub(crate) fn attack_modifier(
    world: &World,
    attacker: Entity,
    target: Entity,
    skill: &SkillDef,
    attack_bonus: i32,
) -> i32 {
    attack_modifier_breakdown(world, attacker, target, skill, attack_bonus).total
}

fn attack_modifier_breakdown(
    world: &World,
    attacker: Entity,
    target: Entity,
    skill: &SkillDef,
    attack_bonus: i32,
) -> AttackModifierBreakdown {
    let unit = world
        .get::<Unit>(attacker)
        .expect("可發動攻擊的單位應具有 Unit");
    let attack_stat_modifier = unit.attack;
    let skill_attack_modifier = attack_bonus;
    let flanking_modifier = flanking_bonus(world, attacker, target, skill);
    AttackModifierBreakdown {
        attack_stat_modifier,
        skill_attack_modifier,
        flanking_modifier,
        total: attack_stat_modifier + skill_attack_modifier + flanking_modifier,
    }
}

fn flanking_bonus(world: &World, attacker: Entity, target: Entity, skill: &SkillDef) -> i32 {
    if skill.ranged {
        return 0;
    }
    let attacker_side =
        match target_side_within_range(world, attacker, target, skill.min_range, skill.max_range) {
            Some(side) => side,
            None => return 0,
        };
    let attacker_team = world
        .get::<Unit>(attacker)
        .expect("可發動攻擊的單位應具有 Unit")
        .team
        .clone();
    let skills = world.resource::<Skills>();
    let has_supporter = world.iter_entities().any(|entity| {
        if entity.id() == attacker || entity.id() == target {
            return false;
        }
        let unit = match entity.get::<Unit>() {
            Some(unit) => unit,
            None => return false,
        };
        if unit.team != attacker_team {
            return false;
        }
        unit.skills
            .iter()
            .filter_map(|skill_id| skills.definitions.get(skill_id))
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
        gameplay_config::FLANKING_ATTACK_BONUS
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
    let unit_footprint = *world
        .get::<Footprint>(unit)
        .expect("參與包夾的單位應具有 Footprint");
    let target_position = world.get::<Pos>(target).expect("包夾目標應具有 Pos").0;
    let target_footprint = *world
        .get::<Footprint>(target)
        .expect("包夾目標應具有 Footprint");
    let target_distance = footprint_distance(
        unit_position,
        unit_footprint,
        target_position,
        target_footprint,
    );
    if target_distance < min_range || target_distance > max_range {
        return None;
    }

    let unit_right = unit_position.x + unit_footprint.width - 1;
    let unit_bottom = unit_position.y + unit_footprint.height - 1;
    let target_right = target_position.x + target_footprint.width - 1;
    let target_bottom = target_position.y + target_footprint.height - 1;
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
    matches!(turn.phase, Phase::Ready | Phase::Moving) && turn.movement_segments_used == 0
        || matches!(turn.phase, Phase::AfterMove) && turn.movement_segments_used == 1
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
    let penalty = footprint_terrain_penalty(
        w.resource::<Board>(),
        w.resource::<TemporaryTerrains>(),
        position,
        footprint,
        |terrain| terrain.dodge_penalty,
    );
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
    let penalty = footprint_terrain_penalty(
        w.resource::<Board>(),
        w.resource::<TemporaryTerrains>(),
        position,
        footprint,
        |terrain| terrain.block_penalty,
    );
    (unit.block - penalty).max(0)
}

fn footprint_terrain_penalty(
    board: &Board,
    temporary: &TemporaryTerrains,
    position: GridPos,
    footprint: Footprint,
    penalty: impl Fn(&TerrainTypeDef) -> i32,
) -> i32 {
    (position.y..position.y + footprint.height)
        .flat_map(|y| (position.x..position.x + footprint.width).map(move |x| GridPos { x, y }))
        .map(|cell| {
            let mut seen = HashSet::new();
            let fixed = board.terrains.get(&cell).into_iter().flatten();
            let dynamic = temporary
                .0
                .get(&cell)
                .into_iter()
                .flat_map(|items| items.keys());
            fixed
                .chain(dynamic)
                .filter(|kind| seen.insert(kind.as_str()))
                .map(|kind| penalty(terrain_type(board, kind)))
                .sum::<i32>()
        })
        .max()
        .unwrap_or(0)
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

pub(crate) fn closest_occupied_cell(w: &World, attacker: Entity, target: Entity) -> GridPos {
    let attacker_position = w
        .get::<Pos>(attacker)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let attacker_footprint = *w
        .get::<Footprint>(attacker)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let target_position = w
        .get::<Pos>(target)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let target_footprint = *w
        .get::<Footprint>(target)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    footprint_cells(target_position, target_footprint)
        .into_iter()
        .min_by_key(|cell| {
            footprint_distance(
                attacker_position,
                attacker_footprint,
                *cell,
                Footprint {
                    width: 1,
                    height: 1,
                },
            )
        })
        .expect("載入時已驗證單位佔用尺寸為正值，應至少佔用一格")
}

pub(crate) fn skill_ranges(w: &World, e: Entity) -> Vec<SkillRangeView> {
    let board = w.resource::<Board>();
    let position = w.get::<Pos>(e).expect("已建立的戰鬥單位應具有 Pos 元件").0;
    let footprint = *w
        .get::<Footprint>(e)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let unit = w.get::<Unit>(e).expect("已建立的戰鬥單位應具有 Unit 元件");
    let skills = w.resource::<Skills>();
    let ranges: Vec<_> = unit
        .skills
        .iter()
        .filter_map(|skill_id| skills.definitions.get(skill_id))
        .map(|skill| {
            let mut cells = Vec::new();
            for y in 0..board.height {
                for x in 0..board.width {
                    let cell = GridPos { x, y };
                    let cell_distance = footprint_distance(
                        position,
                        footprint,
                        cell,
                        Footprint {
                            width: 1,
                            height: 1,
                        },
                    );
                    if (skill.min_range..=skill.max_range).contains(&cell_distance)
                        && (target_kind(&skill.effect) != SkillTargetKind::Enemy
                            || cell_distance > 0)
                    {
                        cells.push(cell);
                    }
                }
            }
            SkillRangeView {
                id: skill.id.clone(),
                details: skill_details(skill, board),
                usable: can_use_skill(w.resource::<Turn>()),
                cells,
            }
        })
        .collect();
    ranges
}

fn skill_details(skill: &SkillDef, board: &Board) -> SkillDetailsView {
    let target = target_kind(&skill.effect);
    let (attack_bonus, power_bonus, effect) = match &skill.effect {
        SkillEffect::Attack {
            attack_bonus,
            power_bonus,
        } => (
            Some(*attack_bonus),
            Some(*power_bonus),
            SkillDetailEffect::Attack,
        ),
        SkillEffect::Push {
            attack_bonus,
            power_bonus,
        } => (
            Some(*attack_bonus),
            Some(*power_bonus),
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
    };
    SkillDetailsView {
        target,
        ranged: skill.ranged,
        min_range: skill.min_range,
        max_range: skill.max_range,
        attack_bonus,
        power_bonus,
        effect,
    }
}
