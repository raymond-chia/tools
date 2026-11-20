//! 被動反應共用普攻、射程與每輪次數；不消耗目前行動者的一般行動。
use crate::game::Game;
use crate::model::{
    ActiveSkillDef, ActiveSkillEffect, Encounter, GridPos, Id, PassiveSkillEffect, ReactionState,
    TerrainGrid, Unit,
};
use crate::movement::{footprint_distance, placement};
use crate::skill::{UnitSkillEffect, effective_skill, unit_passive_effects};
use crate::teams::are_allied;
use bevy_ecs::prelude::{Entity, World};

pub(crate) fn basic_attack_definition(id: &str) -> ActiveSkillDef {
    ActiveSkillDef {
        id: id.into(),
        min_range: 1,
        max_range: 1,
        effect: ActiveSkillEffect::Attack {
            power_source: crate::PowerSource::Physical,
            ranged: false,
            hit_bonus: 0,
            power_bonus: 0,
        },
    }
}

fn state(world: &World, entity: Entity) -> ReactionState {
    let round = world.resource::<Encounter>().round;
    match world.get::<ReactionState>(entity) {
        Some(current) if current.round == round => *current,
        _ => ReactionState {
            round,
            action_reactions_used: 0,
            collision_reactions_used: 0,
        },
    }
}

fn in_basic_attack_range(
    world: &World,
    attacker: Entity,
    target: Entity,
    target_position: GridPos,
) -> bool {
    let (origin, footprint) = placement(world, attacker);
    let (_, target_size) = placement(world, target);
    let skill = effective_skill(world, attacker, &basic_attack_definition(""));
    crate::sight::sees_footprint_in_range(
        world.resource::<TerrainGrid>(),
        origin,
        footprint,
        target_position,
        target_size,
        skill.min_range,
        skill.max_range,
    )
}

/// 選路預估與實際結算共用觸發條件，額度由呼叫端依路徑累計。
pub(crate) fn opportunity_attackers(
    world: &World,
    mover: Entity,
    origin: GridPos,
    destination: GridPos,
) -> Vec<(i64, Entity, String)> {
    let team = world
        .get::<Unit>(mover)
        .expect("移動者應有 Unit")
        .team
        .clone();
    let mut attackers: Vec<_> = world
        .iter_entities()
        .filter_map(|entity| {
            let unit = entity.get::<Unit>()?;
            if are_allied(world, &team, &unit.team) {
                return None;
            }
            let (id, quota) = unit_passive_effects(world, entity.id())
                .into_iter()
                .find_map(|(id, effect)| match effect {
                    PassiveSkillEffect::OpportunityAttack {
                        reactions_per_round,
                    } => Some((id.to_string(), reactions_per_round)),
                    _ => None,
                })?;
            if state(world, entity.id()).action_reactions_used >= quota
                || !in_basic_attack_range(world, entity.id(), mover, origin)
                || {
                    let (source, size) = placement(world, entity.id());
                    let (_, mover_size) = placement(world, mover);
                    let skill = effective_skill(world, entity.id(), &basic_attack_definition(""));
                    footprint_distance(source, size, destination, mover_size) <= skill.max_range
                }
            {
                return None;
            }
            Some((
                entity.get::<Id>().expect("戰鬥單位應有 Id").0,
                entity.id(),
                id,
            ))
        })
        .collect();
    attackers.sort_by_key(|(id, _, _)| *id);
    attackers
}

pub(crate) fn remaining_opportunity_reactions(world: &World, entity: Entity) -> u32 {
    let quota = unit_passive_effects(world, entity)
        .into_iter()
        .find_map(|(_, effect)| match effect {
            PassiveSkillEffect::OpportunityAttack {
                reactions_per_round,
            } => Some(reactions_per_round),
            _ => None,
        })
        .expect("藉機攻擊者應具有對應被動技能");
    quota.saturating_sub(state(world, entity).action_reactions_used)
}

impl Game {
    /// 在跨出射程的那一步之前結算；推擊不呼叫此入口。
    pub(crate) fn opportunity_attacks_before_step(&mut self, mover: Entity, destination: GridPos) {
        let (origin, _) = placement(&self.world, mover);
        let attackers = opportunity_attackers(&self.world, mover, origin, destination);
        for (_, attacker, skill_id) in attackers {
            if self.world.get_entity(mover).is_err() {
                break;
            }
            let mut usage = state(&self.world, attacker);
            usage.action_reactions_used += 1;
            self.world.entity_mut(attacker).insert(usage);
            self.reaction_attack(attacker, mover, &skill_id);
        }
    }

    pub(crate) fn collision_counter_target(
        &self,
        actor: Entity,
        opponents: &[Entity],
    ) -> Option<Entity> {
        let unit = self.world.get::<Unit>(actor).expect("碰撞單位應有 Unit");
        opponents
            .iter()
            .copied()
            .filter(|target| {
                let other = self.world.get::<Unit>(*target).expect("碰撞對象應有 Unit");
                !are_allied(&self.world, &unit.team, &other.team)
                    && in_basic_attack_range(
                        &self.world,
                        actor,
                        *target,
                        placement(&self.world, *target).0,
                    )
            })
            .min_by_key(|target| self.world.get::<Id>(*target).expect("碰撞對象應有 Id").0)
    }

    /// 每次反應消耗一次額度；撞牆或沒有合法反擊對象也會消耗。
    pub(crate) fn take_collision_counter(&mut self, entity: Entity) -> Option<String> {
        let (id, quota) = unit_passive_effects(&self.world, entity)
            .into_iter()
            .find_map(|(id, effect)| match effect {
                PassiveSkillEffect::CollisionCounter {
                    reactions_per_round,
                } => Some((id.to_string(), reactions_per_round)),
                _ => None,
            })?;
        let mut usage = state(&self.world, entity);
        if usage.collision_reactions_used >= quota {
            return None;
        }
        usage.collision_reactions_used += 1;
        self.world.entity_mut(entity).insert(usage);
        Some(id)
    }

    pub(crate) fn reaction_attack(&mut self, attacker: Entity, target: Entity, passive_id: &str) {
        // 先前的傷害可能已讓其中一方退場，反應不追打已倒下的單位。
        let (actor, victim) = match (
            self.world.get::<Unit>(attacker),
            self.world.get::<Unit>(target),
        ) {
            (Some(actor), Some(victim)) => (actor, victim),
            _ => return,
        };
        if are_allied(&self.world, &actor.team, &victim.team) {
            return;
        }
        let (position, _) = placement(&self.world, target);
        if !in_basic_attack_range(&self.world, attacker, target, position) {
            return;
        }
        let actor_id = self.world.get::<Id>(attacker).expect("反擊者應有 Id").0;
        let skill = effective_skill(&self.world, attacker, &basic_attack_definition(passive_id));
        self.resolve_unit_skill(
            attacker,
            actor_id,
            target,
            position,
            &skill,
            UnitSkillEffect::Attack {
                hit_bonus: 0,
                power: Some((crate::PowerSource::Physical, 0)),
                push: false,
            },
        );
    }
}
