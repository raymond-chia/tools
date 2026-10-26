//! 撿貨與撤離的優先規劃；路徑、技能驗證和實際結算仍使用核心共用流程。
use super::{ActiveSkillPlan, AiAction, AiTurnPlan};
use crate::authoring::AiProfile;
use crate::game::Game;
use crate::model::{
    ActiveSkillDef, ActiveSkillEffect, ActiveSkillTargetKind, Cargo, CargoState, GridPos, Id,
    TerrainGrid, Turn, Unit,
};
use crate::movement::{
    MovementOption, PathPriority, full_board_paths, movement_options, occupied_cells, placement,
    shortest_route,
};
use crate::skill::target_kind;
use crate::skill::validate_loot_from_position;
use crate::teams::is_player;
use crate::turn::can_use_skill;
use bevy_ecs::prelude::Entity;

struct CargoRoute {
    movement: MovementOption,
    loot: Option<(ActiveSkillDef, GridPos)>,
}

impl Game {
    fn cargo_route(&self, actor: Entity, active_skills: &[ActiveSkillDef]) -> Option<CargoRoute> {
        let Cargo { items, exits } = self.world.get_resource::<Cargo>()?;
        let id = self.world.get::<Id>(actor).expect("AI 單位應具有 Id").0;
        let carrying = items.contains(&CargoState::Carried(id));
        let unit = self.world.get::<Unit>(actor).expect("AI 單位應具有 Unit");
        let loot_skills: Vec<_> = active_skills
            .iter()
            .filter(|skill| matches!(skill.effect, ActiveSkillEffect::Loot))
            .collect();
        if !carrying
            && (is_player(&unit.team)
                || loot_skills.is_empty()
                || !items
                    .iter()
                    .any(|item| matches!(item, CargoState::Ground(_))))
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
        let mut candidates = Vec::new();
        for item in items {
            if let CargoState::Ground(target) = item {
                for skill in &loot_skills {
                    if let Some(movement) =
                        shortest_route(grid, &search, start, footprint, |position| {
                            validate_loot_from_position(
                                &self.world,
                                actor,
                                position,
                                *target,
                                skill,
                            )
                            .is_ok()
                        })
                    {
                        candidates.push(CargoRoute {
                            movement,
                            loot: Some(((*skill).clone(), *target)),
                        });
                    }
                }
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

    pub(super) fn ai_cargo_turn_plan(
        &self,
        actor: Entity,
        profile: &AiProfile,
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
        } = self.cargo_route(actor, active_skills)?;
        let turn = self.world.resource::<Turn>();
        let mut first = movement_options(&self.world, actor, turn, false);
        if can_use_skill(turn) {
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
        // 只在通往目標的路上攻擊，不留在原地或偏離貨物／出口追敵。
        first.retain(|plan| route.iter().skip(1).any(|cell| *cell == plan.position));
        let attacks: Vec<_> = active_skills
            .iter()
            .filter(|skill| target_kind(&skill.effect) == ActiveSkillTargetKind::Enemy)
            .cloned()
            .collect();
        if let Some(ActiveSkillPlan {
            position_index,
            skill,
            target_cell,
            target_id,
            utility: _,
        }) = self.ai_skill_plan(actor, profile, &attacks, &first, None)
        {
            return Some(AiTurnPlan {
                movement: first.swap_remove(position_index),
                action: Some(AiAction {
                    skill,
                    target_cell,
                    attack_target: Some(target_id),
                }),
            });
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
