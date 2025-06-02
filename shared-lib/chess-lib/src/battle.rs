use crate::battlefield::{Battlefield, Pos};

pub const PLAYER_TEAM: &str = "player";
pub type UnitId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleState {
    NotStarted,
    InProgress,
    Finished,
}

pub struct Battle {
    pub round: usize,              // 當前輪數
    pub action_queue: Vec<UnitId>, // 本輪所有單位行動順序
    pub active_unit_id: String,    // 目前行動單位的 UnitId
    pub moved_distance: u32,       // 本回合已移動距離
    pub state: BattleState,        // 戰鬥狀態
}

impl Battle {
    pub fn new() -> Self {
        Battle {
            round: 1,
            action_queue: Vec::new(),
            active_unit_id: String::new(),
            moved_distance: 0,
            state: BattleState::NotStarted,
        }
    }

    /// 開始戰鬥
    pub fn start(&mut self, unit_order: Vec<UnitId>) -> Result<(), String> {
        if unit_order.is_empty() {
            return Err("action_queue is empty when starting the battle".to_string());
        }
        self.state = BattleState::InProgress;
        self.round = 1;
        self.action_queue = unit_order;
        self.active_unit_id = self.action_queue[0].clone();
        self.moved_distance = 0;
        Ok(())
    }

    /// 結束目前單位回合，進入下一單位
    pub fn end_turn(&mut self) {
        // 這裡需要根據 action_queue 取得下一個 UnitId
        if let Some(pos) = self
            .action_queue
            .iter()
            .position(|id| id == &self.active_unit_id)
        {
            let next_pos = pos + 1;
            if next_pos < self.action_queue.len() {
                self.active_unit_id = self.action_queue[next_pos].clone();
            } else {
                self.next_round();
                return;
            }
        }
        self.moved_distance = 0;
    }

    /// 進入下一輪
    pub fn next_round(&mut self) {
        self.round += 1;
        self.active_unit_id = self.action_queue[0].clone();
        self.moved_distance = 0;
    }
}

impl Battle {
    /// 判斷單位是否可以從 from 移動到 to
    /// - move_range: 單位移動力
    /// - moved_distance: 本回合已移動距離
    /// 回傳 Some(1) 表示本次移動消耗 1 倍移動力，Some(2) 表示消耗 2 倍移動力，None 表示不可移動
    pub fn can_unit_move(
        &self,
        battlefield: &Battlefield,
        from: Pos,
        to: Pos,
        move_range: usize,
    ) -> Option<u8> {
        if !battlefield.is_valid_position(&from) || !battlefield.is_valid_position(&to) {
            return None;
        }
        // 目標格不能有單位
        if battlefield.grid[to.y][to.x].unit_id.is_some() {
            return None;
        }
        // 距離限制（曼哈頓距離）
        let dist =
            (from.x as isize - to.x as isize).abs() + (from.y as isize - to.y as isize).abs();
        let dist = dist as usize;
        // 取得目前行動單位的已移動距離
        let moved = self.moved_distance as usize;
        // 判斷本次移動屬於哪一倍移動力
        if moved + dist > move_range * 2 {
            return None;
        }
        if moved + dist <= move_range {
            Some(1)
        } else {
            Some(2)
        }
    }

    /// 執行單位移動（只處理格子，不動 unit_id_to_team），會更新 moved_distance
    /// 回傳 Some(1) 或 Some(2) 表示移動成功且消耗的移動力倍數，None 表示失敗
    pub fn move_unit_on_field(
        &mut self,
        battlefield: &mut Battlefield,
        from: Pos,
        to: Pos,
        move_range: usize,
    ) -> Option<u8> {
        if Some(&self.active_unit_id) != battlefield.grid[from.y][from.x].unit_id.as_ref() {
            return None;
        }
        let move_type = self.can_unit_move(battlefield, from, to, move_range)?;
        battlefield.grid[from.y][from.x].unit_id = None;
        battlefield.grid[to.y][to.x].unit_id = Some(self.active_unit_id.clone());
        let dist =
            (from.x as isize - to.x as isize).abs() + (from.y as isize - to.y as isize).abs();
        self.moved_distance += dist as u32;
        Some(move_type)
    }

    /// 通用單位選取與移動互動邏輯
    /// 回傳 status_msg: String（包含選取、移動、錯誤等訊息）
    pub fn unit_select_and_move_interaction(
        &mut self,
        battlefield: &mut Battlefield,
        selected_unit: Option<Pos>,
        pos: Pos,
        move_range: usize,
    ) -> String {
        let cell = &battlefield.grid[pos.y][pos.x];
        // 1. 尚未選取單位，且點到己方單位
        let Some(from_pos) = selected_unit else {
            if let Some(unit_id) = &cell.unit_id {
                if let Some(unit) = battlefield.unit_id_to_team.get(unit_id) {
                    if &unit.team_id == PLAYER_TEAM {
                        return format!("選取單位：{}", unit_id);
                    }
                }
            }
            return "請選擇己方單位".to_string();
        };
        // 2. 已選取單位，點擊目標格
        if let Some(mt) = self.can_unit_move(battlefield, from_pos, pos, move_range) {
            let moved = self.move_unit_on_field(battlefield, from_pos, pos, move_range);
            if moved.is_some() {
                if mt == 2 {
                    return "本次移動消耗了兩倍移動力".to_string();
                } else {
                    return "移動成功".to_string();
                }
            } else {
                return "沒有移動力".to_string();
            }
        }
        // 點到其他單位則重新選取
        if let Some(unit_id2) = &cell.unit_id {
            if let Some(unit2) = battlefield.unit_id_to_team.get(unit_id2) {
                if &unit2.team_id == PLAYER_TEAM {
                    return format!("選取單位：{}", unit_id2);
                }
            }
        }
        return "請選擇己方單位".to_string();
    }
}
