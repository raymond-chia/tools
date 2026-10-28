//! 撿貨與撤離的優先規劃；路徑、技能驗證和實際結算仍使用核心共用流程。
use super::{AiAction, AiTurnPlan};
use crate::game::Game;
use crate::model::{
    ActiveSkillDef, ActiveSkillEffect, ActiveSkills, Cargo, CargoState, GridPos, Id, TerrainGrid,
    Turn, Unit,
};
use crate::movement::{
    MovementOption, PathPriority, full_board_paths, movement_options, occupied_cells, placement,
    shortest_route,
};
use crate::skill::validate_loot_from_position;
use crate::teams::{are_allied, is_player};
use crate::turn::can_use_skill;
use bevy_ecs::prelude::Entity;

struct CargoRoute {
    movement: MovementOption,
    loot: Option<(ActiveSkillDef, GridPos)>,
}

impl Game {
    fn cargo_route(
        &self,
        actor: Entity,
        active_skills: &[ActiveSkillDef],
        target_cargo: Option<GridPos>,
    ) -> Option<CargoRoute> {
        let Cargo { items, exits } = self.world.get_resource::<Cargo>()?;
        let id = self.world.get::<Id>(actor).expect("AI 單位應具有 Id").0;
        let carrying = items.contains(&CargoState::Carried(id));
        let unit = self.world.get::<Unit>(actor).expect("AI 單位應具有 Unit");
        let loot_skills: Vec<_> = active_skills
            .iter()
            .filter(|skill| matches!(skill.effect, ActiveSkillEffect::Loot))
            .collect();
        if !carrying && (is_player(&unit.team) || loot_skills.is_empty() || target_cargo.is_none())
        {
            return None;
        }
        let (start, footprint) = placement(&self.world, actor);
        let grid = self.world.resource::<TerrainGrid>();
        let search = full_board_paths(
            grid,
            &occupied_cells(&self.world, actor),
            start,
            footprint,
            PathPriority::MovementCost,
        );
        if carrying {
            return shortest_route(grid, &search, start, footprint, |position| {
                exits.contains(&position)
            })
            .map(|movement| CargoRoute {
                movement,
                loot: None,
            });
        }
        let target = target_cargo?;
        let mut candidates = Vec::new();
        for skill in loot_skills {
            if let Some(movement) = shortest_route(grid, &search, start, footprint, |position| {
                validate_loot_from_position(&self.world, actor, position, target, skill).is_ok()
            }) {
                candidates.push(CargoRoute {
                    movement,
                    loot: Some((skill.clone(), target)),
                });
            }
        }
        candidates.into_iter().min_by_key(
            |CargoRoute {
                 movement:
                     MovementOption {
                         position,
                         path: _,
                         cost,
                     },
                 loot,
             }| {
                let (skill, target) = loot.as_ref().expect("撿貨路徑應包含技能與目標");
                (
                    *cost,
                    target.y,
                    target.x,
                    skill.id.clone(),
                    position.y,
                    position.x,
                )
            },
        )
    }

    // 依實際路程分配貨物；每份貨物與每名取貨者都只分配一次，同距離以 ID 固定順序。
    fn assigned_cargo_route(&self, actor: Entity) -> Option<CargoRoute> {
        let Cargo { items, exits: _ } = self.world.get_resource::<Cargo>()?;
        let actor_unit = self.world.get::<Unit>(actor).expect("AI 單位應具有 Unit");
        let ActiveSkills { definitions } = self.world.resource::<ActiveSkills>();
        let mut candidates = Vec::new();
        for entity in self.world.iter_entities() {
            if let (Some(unit), Some(Id(id))) = (entity.get::<Unit>(), entity.get::<Id>()) {
                if is_player(&unit.team)
                    || !are_allied(&self.world, &actor_unit.team, &unit.team)
                    || items.contains(&CargoState::Carried(*id))
                {
                    continue;
                }
                let skills: Vec<_> = unit
                    .active_skills
                    .iter()
                    .map(|id| definitions.get(id).expect("作者資料已驗證技能引用").clone())
                    .collect();
                for item in items {
                    if let CargoState::Ground(target) = item {
                        if let Some(route) = self.cargo_route(entity.id(), &skills, Some(*target)) {
                            candidates.push((*id, entity.id(), *target, route));
                        }
                    }
                }
            }
        }
        candidates.sort_by_key(
            |(
                id,
                _,
                target,
                CargoRoute {
                    movement:
                        MovementOption {
                            position: _,
                            path: _,
                            cost,
                        },
                    loot: _,
                },
            )| (*cost, *id, target.y, target.x),
        );
        let mut assigned_units = Vec::new();
        let mut assigned_cargo = Vec::new();
        for (id, entity, target, route) in candidates {
            if assigned_units.contains(&id) || assigned_cargo.contains(&target) {
                continue;
            }
            if entity == actor {
                return Some(route);
            }
            assigned_units.push(id);
            assigned_cargo.push(target);
        }
        None
    }

    pub(super) fn ai_cargo_turn_plan(
        &self,
        actor: Entity,
        active_skills: &[ActiveSkillDef],
    ) -> Option<AiTurnPlan> {
        let CargoRoute {
            movement:
                MovementOption {
                    position: goal,
                    path: route,
                    cost: _,
                },
            loot,
        } = self
            .cargo_route(actor, active_skills, None)
            .or_else(|| self.assigned_cargo_route(actor))?;
        let turn = self.world.resource::<Turn>();
        let mut first = movement_options(&self.world, actor, turn, false);
        if can_use_skill(&self.world) {
            if let Some((skill, target_cell)) = loot.as_ref() {
                if let Some(index) = first.iter().position(|plan| plan.position == goal) {
                    return Some(AiTurnPlan {
                        movement: first.swap_remove(index),
                        action: Some(AiAction {
                            skill: skill.clone(),
                            target_cell: *target_cell,
                            attack_target: None,
                        }),
                    });
                }
            }
        }
        let mut all = movement_options(&self.world, actor, turn, true);
        // 本回合能離場時先離場，不為攻擊延後撤離。
        if loot.is_none() {
            if let Some(index) = all.iter().position(|plan| plan.position == goal) {
                return Some(AiTurnPlan {
                    movement: all.swap_remove(index),
                    action: None,
                });
            }
        }
        // 危險地形可能截斷路徑，僅走到本回合實際可到達的最遠路徑前綴。
        let index = route
            .iter()
            .rev()
            .find_map(|cell| all.iter().position(|plan| plan.position == *cell))
            .expect("貨物路徑起點應包含在可用移動選項中");
        Some(AiTurnPlan {
            movement: all.swap_remove(index),
            action: None,
        })
    }
}
