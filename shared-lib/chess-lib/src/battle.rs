use crate::{
    PLAYER_TEAM,
    battlefield::{Battlefield, Cell, Pos},
};
use strum_macros::{Display, EnumString};

pub type UnitID = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleState {
    NotStarted,
    InProgress,
    Finished,
}

pub struct Battle {
    pub state: BattleState,       // 戰鬥狀態
    pub round: usize,             // 當前輪數
    pub unit_orders: Vec<UnitID>, // 本輪所有單位行動順序
    pub active_unit_id: UnitID,   // 目前行動單位的 UnitId
    pub moved_distance: usize,    // 本回合已移動距離
}

impl Default for Battle {
    fn default() -> Self {
        Battle {
            state: BattleState::NotStarted,
            round: 1,
            unit_orders: Vec::new(),
            active_unit_id: String::new(),
            moved_distance: 0,
        }
    }
}

impl Battle {
    /// 開始戰鬥
    pub fn start(self, unit_orders: Vec<UnitID>) -> Result<Self, String> {
        if unit_orders.is_empty() {
            return Err("unit_orders is empty when starting the battle".to_string());
        }
        let active_unit_id = unit_orders[0].clone();
        return Ok(Self {
            state: BattleState::InProgress,
            round: 1,
            unit_orders,
            active_unit_id,
            moved_distance: 0,
        });
    }

    /// 結束目前單位回合，進入下一單位
    pub fn end_turn(&mut self) {
        // 取得下一個 UnitId
        if let Some(idx) = self
            .unit_orders
            .iter()
            .position(|id| id == &self.active_unit_id)
        {
            let next_idx = idx + 1;
            if next_idx < self.unit_orders.len() {
                self.active_unit_id = self.unit_orders[next_idx].clone();
                self.moved_distance = 0;
                return;
            }
            self.next_round();
            return;
        }
    }

    /// 進入下一輪
    pub fn next_round(&mut self) {
        self.round += 1;
        self.active_unit_id = self.unit_orders[0].clone();
        self.moved_distance = 0;
    }
}

#[derive(Debug, EnumString, Display)]
#[strum(serialize_all = "snake_case")]
pub enum ValidResult {
    FirstMovement,
    SecondMovement,

    PickMinion,
}

#[derive(Debug, EnumString, Display)]
#[strum(serialize_all = "snake_case")]
pub enum InvalidResult {
    NotActiveUnit,
    InvalidNextPick,
    NotPickingMinion,
    TargetOccupied,
    MovementExceedsRange,
}

impl Battle {
    /// 判斷單位是否可以從 from 移動到 to
    /// - move_range: 單位移動力
    /// - moved_distance: 本回合已移動距離
    pub fn can_unit_move(
        battlefield: &Battlefield,
        from: Pos,
        to: Pos,
        move_range: usize,
        moved_distance: usize,
    ) -> Result<ValidResult, InvalidResult> {
        if !battlefield.is_valid_position(to) {
            return Err(InvalidResult::InvalidNextPick);
        }
        // 目標格不能有單位
        if Self::get_cell(battlefield, to).unit_id.is_some() {
            return Err(InvalidResult::TargetOccupied);
        }
        // 距離限制（曼哈頓距離）
        let dist =
            (from.x as isize - to.x as isize).abs() + (from.y as isize - to.y as isize).abs();
        let dist = dist as usize;
        // 判斷本次移動屬於哪一倍移動力
        if moved_distance + dist > move_range * 2 {
            return Err(InvalidResult::MovementExceedsRange);
        }
        if moved_distance + dist <= move_range {
            return Ok(ValidResult::FirstMovement);
        }
        return Ok(ValidResult::SecondMovement);
    }

    /// 通用單位選取與移動互動邏輯
    pub fn click_battlefield(
        &mut self,
        battlefield: &mut Battlefield,
        last_pick: Option<Pos>,
        next_pick: Pos,
        move_range: usize,
    ) -> Result<ValidResult, InvalidResult> {
        if !battlefield.is_valid_position(next_pick) {
            return Err(InvalidResult::InvalidNextPick);
        }
        // 已選取己方單位，點擊目標格
        if let Some(last_pick) = last_pick {
            let last_picked_unit_id = &Self::get_cell(battlefield, last_pick).unit_id;
            if let Some(last_picked_unit_id) = last_picked_unit_id {
                let last_picked_unit = battlefield.unit_id_to_unit.get(last_picked_unit_id);
                if let Some(last_picked_unit) = last_picked_unit {
                    if &last_picked_unit.team_id == PLAYER_TEAM {
                        return self.order_minion(battlefield, last_pick, next_pick, move_range);
                    }
                }
            }
        }
        // 尚未選取己方單位
        Self::is_picking_minion(battlefield, next_pick)?;
        return Ok(ValidResult::PickMinion);
    }
}

// private methods for Battle
impl Battle {
    fn get_cell(battlefield: &Battlefield, pos: Pos) -> &Cell {
        &battlefield.grid[pos.y][pos.x]
    }

    fn get_cell_mut(battlefield: &mut Battlefield, pos: Pos) -> &mut Cell {
        &mut battlefield.grid[pos.y][pos.x]
    }

    /// 執行單位移動，會更新 moved_distance
    fn move_unit_on_field(
        &mut self,
        battlefield: &mut Battlefield,
        from: Pos,
        to: Pos,
        move_range: usize,
    ) -> Result<ValidResult, InvalidResult> {
        if Some(&self.active_unit_id) != Self::get_cell(battlefield, from).unit_id.as_ref() {
            return Err(InvalidResult::NotActiveUnit);
        }
        let move_type =
            Self::can_unit_move(battlefield, from, to, move_range, self.moved_distance)?;
        let unit_id = Self::get_cell_mut(battlefield, from).unit_id.take();
        Self::get_cell_mut(battlefield, to).unit_id = unit_id;
        let dist =
            (from.x as isize - to.x as isize).abs() + (from.y as isize - to.y as isize).abs();
        self.moved_distance += dist as usize;
        return Ok(move_type);
    }

    fn is_picking_minion(battlefield: &Battlefield, pos: Pos) -> Result<(), InvalidResult> {
        let cell = Self::get_cell(battlefield, pos);
        let Some(unit_id) = &cell.unit_id else {
            return Err(InvalidResult::NotPickingMinion);
        };
        let unit = battlefield.unit_id_to_unit.get(unit_id);
        let Some(unit) = unit else {
            return Err(InvalidResult::NotPickingMinion);
        };
        if unit.team_id != PLAYER_TEAM {
            return Err(InvalidResult::NotPickingMinion);
        }
        return Ok(());
    }

    fn order_minion(
        &mut self,
        battlefield: &mut Battlefield,
        from: Pos,
        to: Pos,
        move_range: usize,
    ) -> Result<ValidResult, InvalidResult> {
        return self.move_unit_on_field(battlefield, from, to, move_range);
    }
}
