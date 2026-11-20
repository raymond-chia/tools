//! 逃跑候選與攻擊、治療共同比較效益；沿用一般移動的額度、避障與危險地形規則。
use super::AiTurnPlan;
use super::risk::movement_risk;
use crate::game::Game;
use crate::model::{TerrainGrid, Turn, Unit};
use crate::movement::{
    MovementOption, footprint_cells, footprint_distance, movement_options, placement,
};
use crate::teams::are_allied;
use bevy_ecs::prelude::Entity;

impl Game {
    pub(super) fn flee_turn_plan(&self, actor: Entity, weight: u32) -> Option<(AiTurnPlan, f64)> {
        if weight == 0 {
            return None;
        }
        let unit = self.world.get::<Unit>(actor).expect("逃跑單位應具有 Unit");
        let enemies: Vec<_> = self
            .world
            .iter_entities()
            .filter(|entity| {
                entity
                    .get::<Unit>()
                    .is_some_and(|other| !are_allied(&self.world, &unit.team, &other.team))
            })
            .map(|entity| placement(&self.world, entity.id()))
            .collect();
        let (origin, footprint) = placement(&self.world, actor);
        let nearest_distance = |position| {
            enemies
                .iter()
                .map(|(enemy, size)| footprint_distance(position, footprint, *enemy, *size))
                .min()
        };
        let initial = nearest_distance(origin)?;
        let options = movement_options(&self.world, actor, self.world.resource::<Turn>(), true);
        let grid = self.world.resource::<TerrainGrid>();
        let movement = options
            .into_iter()
            .filter(
                |MovementOption {
                     position: _,
                     path,
                     cost: _,
                 }| {
                    !path
                        .iter()
                        .skip(1)
                        .flat_map(|cell| footprint_cells(*cell, footprint))
                        .any(|cell| grid.cells[(cell.y * grid.width + cell.x) as usize].hazard)
                },
            )
            .max_by(|a, b| {
                let utility = |movement: &MovementOption| {
                    let gain = nearest_distance(movement.position).expect("已確認至少有一名敵人")
                        - initial;
                    let risk = movement_risk(&self.world, actor, &movement.path);
                    // 非致死的藉機攻擊不阻止撤離。
                    f64::from(gain) * f64::from(weight) - risk.death_cost()
                };
                utility(a).total_cmp(&utility(b)).then_with(|| {
                    (b.cost, b.position.y, b.position.x).cmp(&(a.cost, a.position.y, a.position.x))
                })
            })?;
        let gain = nearest_distance(movement.position).expect("已確認至少有一名敵人") - initial;
        if gain <= 0 {
            return None;
        }
        let risk = movement_risk(&self.world, actor, &movement.path);
        Some((
            AiTurnPlan {
                movement,
                action: None,
            },
            f64::from(gain) * f64::from(weight) - risk.death_cost(),
        ))
    }
}
