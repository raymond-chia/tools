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
        let mut all = movement_options(&self.world, actor, turn, true);
        let mut loot_action = None;
        if can_use_skill(&self.world) {
            if let Some((skill, target_cell)) = loot {
                let first = movement_options(&self.world, actor, turn, false);
                if let Some(movement) = first.into_iter().find(|plan| plan.position == goal) {
                    all.retain(|plan| plan.position != goal);
                    all.push(movement);
                    loot_action = Some(AiAction {
                        skill,
                        target_cell,
                        attack_target: None,
                    });
                }
            }
        }
        // 沿貨物路徑比較前進收益與挨打風險；完成目標另有加分。
        let index = all
            .iter()
            .enumerate()
            .filter_map(|(index, movement)| {
                route
                    .iter()
                    .position(|cell| *cell == movement.position)
                    .map(|progress| {
                        let risk = super::risk::movement_risk(&self.world, actor, &movement.path);
                        let completion = if movement.position == goal {
                            100.0
                        } else {
                            0.0
                        };
                        (
                            index,
                            (progress as f64 * 10.0 + completion) * (1.0 - risk.death_chance)
                                - risk.cost(),
                        )
                    })
            })
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
            .expect("貨物路徑起點應包含在可用移動選項中");
        let movement = all.swap_remove(index);
        let action = if movement.position == goal {
            loot_action
        } else {
            None
        };
        Some(AiTurnPlan { movement, action })
    }
}
