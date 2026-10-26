//! 回合的初始化、行動權限、移動額度與階段切換；Turn 僅承載資料。
use crate::error::{self, GameError};
use crate::game::Game;
use crate::model::{BattleMode, Encounter, Exploration, PendingStrike, Phase, Turn, Unit};
use bevy_ecs::prelude::{Entity, World};

pub(crate) fn new_turn(actor: Option<i64>, movement: u32) -> Turn {
    Turn {
        actor,
        phase: Phase::Ready,
        movement_remaining: movement,
        movement_segments_used: 0,
    }
}

pub(crate) fn initial_turn() -> Turn {
    let mut turn = new_turn(None, 0);
    turn.phase = Phase::Ended;
    turn
}

/// 本回合是否還能移動；每回合最多兩段移動。
pub(crate) fn can_move(turn: &Turn) -> bool {
    let Turn {
        actor: _,
        phase,
        movement_remaining: _,
        movement_segments_used,
    } = turn;
    matches!(phase, Phase::Ready | Phase::Moving | Phase::AfterMove) && *movement_segments_used < 2
}

/// 第一段尚未用完時保留第二段完整額度；進入第二段後只使用剩餘額度。
pub(crate) fn movement_budgets(turn: &Turn, allowance: u32) -> (u32, u32) {
    let Turn {
        actor: _,
        phase,
        movement_remaining,
        movement_segments_used,
    } = turn;
    if *movement_segments_used == 0 {
        (*movement_remaining, allowance)
    } else if *phase == Phase::AfterMove {
        (0, allowance)
    } else {
        (0, *movement_remaining)
    }
}

pub(crate) fn can_use_skill(world: &World) -> bool {
    let Turn {
        actor: _,
        phase,
        movement_remaining: _,
        movement_segments_used,
    } = world.resource::<Turn>();
    matches!(phase, Phase::PendingStrike)
        || world.resource::<Exploration>().mode == BattleMode::Exploring
            && matches!(phase, Phase::Ready | Phase::Moving | Phase::AfterMove)
        || matches!(phase, Phase::Ready | Phase::Moving) && *movement_segments_used == 0
        || matches!(phase, Phase::AfterMove) && *movement_segments_used == 1
}

/// 探索逐格扣除移動成本；本次額度不足時，自動開始同一人的下一回合。
pub(crate) fn spend_exploration_step(
    turn: &mut Turn,
    allowance: u32,
    cost: u32,
) -> Result<(), GameError> {
    let (first, second) = movement_budgets(turn, allowance);
    if !can_move(turn) || cost > first + second {
        *turn = new_turn(turn.actor, allowance);
    }
    let (first, second) = movement_budgets(turn, allowance);
    if cost > first + second {
        return Err(error::unreachable_destination());
    }
    spend_movement(turn, first, second, cost);
    Ok(())
}

/// 尚未開始行動（未移動、未使用技能）才能延後。
pub(crate) fn can_delay(turn: &Turn) -> bool {
    let Turn {
        actor: _,
        phase,
        movement_remaining: _,
        movement_segments_used,
    } = turn;
    *phase == Phase::Ready && *movement_segments_used == 0
}

/// 扣除已驗證路徑的實際成本，並決定下一個行動階段。
pub(crate) fn spend_movement(turn: &mut Turn, first_budget: u32, second_budget: u32, spent: u32) {
    let Turn {
        actor: _,
        phase,
        movement_remaining,
        movement_segments_used: segments_used,
    } = turn;
    if *segments_used == 0 && spent <= first_budget {
        *movement_remaining = first_budget - spent;
    } else if *segments_used == 0 {
        *segments_used = 1;
        *movement_remaining = second_budget - (spent - first_budget);
    } else {
        *movement_remaining = second_budget - spent;
    }
    if *movement_remaining == 0 {
        *segments_used += 1;
        *phase = Phase::AfterMove;
    } else {
        *phase = Phase::Moving;
    }
}

impl Game {
    pub(crate) fn begin_turn(&mut self) {
        let Encounter {
            participants: _,
            order,
            cursor,
            round: _,
        } = self.world.resource::<Encounter>();
        let actor = order.get(*cursor).copied();
        if let Some(entity) = actor.and_then(|id| self.entity(id)) {
            self.world.entity_mut(entity).remove::<PendingStrike>();
        }
        let remaining = actor
            .and_then(|a| self.entity(a))
            .map(|e| {
                self.world
                    .get::<Unit>(e)
                    .expect("已建立的戰鬥單位應具有 Unit 元件")
                    .movement
            })
            .unwrap_or(0);
        *self.world.resource_mut::<Turn>() = new_turn(actor, remaining);
    }
    pub(crate) fn finish_turn(&mut self) {
        if let Some(entity) = self
            .world
            .resource::<Turn>()
            .actor
            .and_then(|actor| self.entity(actor))
        {
            self.world.entity_mut(entity).remove::<PendingStrike>();
        }
        self.world.resource_mut::<Turn>().phase = Phase::Ended;
    }
    pub(crate) fn delay(&mut self, actor: i64, after: i64) -> Result<(), GameError> {
        self.ensure_turn(actor)?;
        let turn = self.world.resource::<Turn>();
        if !can_delay(turn) {
            return Err(error::delay_after_action());
        }
        let Encounter {
            participants: _,
            order,
            cursor,
            round: _,
        } = self.world.resource::<Encounter>();
        let target_index = order
            .iter()
            .position(|unit_id| *unit_id == after)
            .ok_or(error::missing_delay_target())?;
        if target_index <= *cursor {
            return Err(error::invalid_delay_target());
        }
        {
            let Encounter {
                participants: _,
                order,
                cursor,
                round: _,
            } = &mut *self.world.resource_mut::<Encounter>();
            let delayed_actor = order.remove(*cursor);
            order.insert(target_index, delayed_actor);
        }
        self.begin_turn();
        Ok(())
    }
    pub(crate) fn ensure_turn(&self, a: i64) -> Result<(), GameError> {
        if self.world.resource::<Turn>().actor != Some(a) {
            Err(error::wrong_turn())
        } else {
            Ok(())
        }
    }
    pub(crate) fn await_strike(&mut self, entity: Entity, skill: String) {
        self.world
            .entity_mut(entity)
            .insert(PendingStrike { skill });
        self.world.resource_mut::<Turn>().phase = Phase::PendingStrike;
    }

    pub(crate) fn remove_turn_actor(&mut self) {
        self.finish_turn();
        self.world.resource_mut::<Turn>().actor = None;
    }
}
