//! 沿實際路徑累計藉機攻擊，供所有 AI 移動決策共用。
use super::attack_outcomes;
use crate::model::{GridPos, Hp};
use crate::movement::placement;
use crate::reaction::{
    basic_attack_definition, opportunity_attackers, remaining_opportunity_reactions,
};
use crate::skill::{attack_preview, effective_skill};
use bevy_ecs::prelude::{Entity, World};
use std::collections::{BTreeMap, HashMap};

pub(super) struct MovementRisk {
    pub(super) damage: f64,
    pub(super) death_chance: f64,
}

impl MovementRisk {
    pub(super) fn cost(&self) -> f64 {
        // 與一般傷害評分同尺度；死亡比單純失血更不值得。
        self.damage * 10.0 + self.death_cost()
    }

    pub(super) fn death_cost(&self) -> f64 {
        self.death_chance * 1000.0
    }
}

pub(super) fn movement_risk(world: &World, actor: Entity, path: &[GridPos]) -> MovementRisk {
    let hp = world.get::<Hp>(actor).expect("AI 單位應具有 Hp").current;
    let mut distribution = BTreeMap::from([(0, 1.0)]);
    let mut used = HashMap::<Entity, u32>::new();
    for step in path.windows(2) {
        for (_, attacker, passive) in opportunity_attackers(world, actor, step[0], step[1]) {
            let count = used.entry(attacker).or_default();
            if *count >= remaining_opportunity_reactions(world, attacker) {
                continue;
            }
            *count += 1;
            let skill = effective_skill(world, attacker, &basic_attack_definition(&passive));
            let preview = attack_preview(
                world,
                attacker,
                placement(world, attacker).0,
                actor,
                step[0],
                &skill,
                0,
                Some((crate::PowerSource::Physical, 0)),
            );
            let mut next = BTreeMap::new();
            for (damage, chance) in distribution {
                for (probability, hit_damage) in attack_outcomes(&preview) {
                    if probability > 0 {
                        *next.entry((damage + hit_damage).min(hp)).or_insert(0.0) +=
                            chance * f64::from(probability) / 100.0;
                    }
                }
            }
            distribution = next;
        }
    }
    MovementRisk {
        damage: distribution
            .iter()
            .map(|(damage, chance)| f64::from(*damage) * chance)
            .sum(),
        death_chance: distribution.get(&hp).copied().unwrap_or(0.0),
    }
}
