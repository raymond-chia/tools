//! 貨物的目標驗證、掠奪、離場與死亡掉落。
use crate::error::{self, GameError};
use crate::game::Game;
use crate::model::{ActiveSkillDef, Cargo, CargoState, GridPos, Id, Pos, Unit};
use crate::teams::is_player;
use bevy_ecs::prelude::{Entity, World};

pub(crate) fn check_loot_target(
    world: &World,
    actor: Entity,
    target: GridPos,
) -> Result<(), GameError> {
    let unit = world.get::<Unit>(actor).expect("施放者應具有 Unit");
    let id = world.get::<Id>(actor).expect("施放者應具有 Id").0;
    let Cargo { items, exits: _ } = world
        .get_resource::<Cargo>()
        .ok_or_else(error::cannot_loot)?;
    if is_player(&unit.team)
        || !items.contains(&CargoState::Ground(target))
        || items.contains(&CargoState::Carried(id))
    {
        return Err(error::cannot_loot());
    }
    Ok(())
}

impl Game {
    pub(crate) fn loot_cargo(
        &mut self,
        entity: Entity,
        id: i64,
        target: GridPos,
        skill: &ActiveSkillDef,
    ) -> Result<(), GameError> {
        let origin = self.world.get::<Pos>(entity).expect("施放者應具有位置").0;
        crate::skill::validate_loot_from_position(&self.world, entity, origin, target, skill)?;
        let mut cargo = self.world.resource_mut::<Cargo>();
        let item = cargo
            .items
            .iter_mut()
            .find(|item| **item == CargoState::Ground(target))
            .expect("掠奪目標已驗證有貨物");
        *item = CargoState::Carried(id);
        self.escape_with_cargo(entity, id);
        self.finish_turn();
        Ok(())
    }

    pub(crate) fn drop_cargo(&mut self, entity: Entity, id: i64) {
        let position = self
            .world
            .get::<Pos>(entity)
            .expect("移除的單位應具有位置")
            .0;
        if let Some(mut cargo) = self.world.get_resource_mut::<Cargo>() {
            if let Some(item) = cargo
                .items
                .iter_mut()
                .find(|item| **item == CargoState::Carried(id))
            {
                *item = CargoState::Ground(position);
            }
        }
    }

    pub(crate) fn escape_with_cargo(&mut self, entity: Entity, id: i64) -> bool {
        let position = self.world.get::<Pos>(entity).expect("移動者應具有位置").0;
        let escaped = match self.world.get_resource_mut::<Cargo>() {
            Some(mut cargo) if cargo.exits.contains(&position) => {
                match cargo
                    .items
                    .iter_mut()
                    .find(|item| **item == CargoState::Carried(id))
                {
                    Some(item) => {
                        *item = CargoState::Escaped;
                        true
                    }
                    None => false,
                }
            }
            _ => false,
        };
        if escaped {
            // 先轉為已離場，避免共用移除入口將貨物掉在出口。
            self.remove_unit(entity, id);
        }
        escaped
    }
}
