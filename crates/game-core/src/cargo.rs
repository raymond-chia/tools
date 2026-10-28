//! 貨物狀態的死亡掉落與攜貨離場；拾取技能由 skill 管理。
use crate::game::Game;
use crate::model::{Cargo, CargoState, Id, Pos};
use bevy_ecs::prelude::{Entity, World};

pub(crate) fn is_carrying_cargo(world: &World, entity: Entity) -> bool {
    let id = world.get::<Id>(entity).expect("戰鬥單位應具有 Id").0;
    world
        .get_resource::<Cargo>()
        .is_some_and(|cargo| cargo.items.contains(&CargoState::Carried(id)))
}

impl Game {
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
