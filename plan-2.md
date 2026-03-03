# 實作回合順序計算與顯示

## 目標與範圍

實作「每輪開始時，所有單位依 INI + 隨機骰決定行動順序」的機制，並在戰鬥模式以左側浮動面板顯示順序。
支援結束回合、延後行動。
所有單位結束後自動進入下一輪。

**做的事：**

- 回合順序計算邏輯（`core/board/src/logic/turn_order.rs`）
- TurnEntry 型別定義（`core/board/src/domain/core_types.rs`）
- TurnOrder ECS Resource（`core/board/src/ecs_types/resources.rs`）
- 回合操作 ECS 函數（`core/board/src/ecs_logic/turn.rs`）
- 在 `battle.rs` 以浮動面板顯示順序
- 結束回合、延後行動功能

**不做的事：**

- 回合效果（buff/debuff TTL 扣除）— 尚無效果系統
- 實際行動執行（移動、技能）
- 單位死亡移除（尚無戰鬥傷害系統，但預留介面）

## 設計決策

| 決策     | 選擇                                                               | 原因                                 |
| -------- | ------------------------------------------------------------------ | ------------------------------------ |
| 骰子範圍 | 1d6                                                                | 策略性與隨機性平衡，常數定義方便調整 |
| 排序方式 | 主排序：INI+1d6（降序）；次排序：INI\*10+1 if player+0.xxx（降序） | 兩欄位排序，幾乎不會同分             |
| 死亡處理 | 直接從順序表移除                                                   | 清單乾淨                             |
| 延後規則 | 只能往後延，點擊清單選擇插入位置                                   | 輪到自己才能延後，自然只能往後       |
| 觸發時機 | 進入戰鬥模式自動擲骰產生第一輪                                     | 自動開始                             |
| 狀態存放 | 全部放 ECS World Resource                                          | `LevelTabUIState` 不存戰鬥狀態       |
| UI 形式  | 浮動面板疊在戰場左側                                               | 不佔用固定佈局空間                   |
| 單位識別 | 用 Occupant（含唯一 ID）                                           | 不依賴位置，單位移動後仍可識別       |
| 參數風格 | calculate_turn_order 用專用輸入結構，不直接傳 UnitBundle           | 明確顯示函數依賴                     |
| 顯示內容 | 只顯示「INI + 骰子 = 總分」，隱藏 tiebreaker                       | 對玩家清晰                           |

## 實作步驟

### 步驟 1：在 `core/board/src/domain/core_types.rs` 定義 TurnEntry

```rust
/// 單位在回合表中的資訊
#[derive(Debug, Clone)]
pub struct TurnEntry {
    pub occupant: Occupant,
    pub initiative: i32,        // 原始 INI
    pub roll: i32,              // 1d6 結果
    pub total: i32,             // INI + roll（主排序，顯示用）
    pub tiebreaker: f64,        // INI*10 + 1 if player + 0.xxx（次排序，隱藏）
    pub has_acted: bool,
}
```

### 步驟 2：在 `core/board/src/ecs_types/resources.rs` 定義 TurnOrder Resource

```rust
/// 回合順序 Resource
#[derive(Debug, Resource, Default)]
pub struct TurnOrder {
    pub round: u32,
    pub entries: Vec<TurnEntry>,
    pub current_index: usize,
}
```

### 步驟 3：在 `core/board/src/logic/turn_order.rs` 新增純邏輯

```rust
const TURN_ORDER_DICE_SIDES: i32 = 6;
const TURN_ORDER_DICE_MIN: i32 = 1;

/// 計算順序的輸入資料
pub struct TurnOrderInput {
    pub occupant: Occupant,
    pub initiative: i32,
    pub is_player: bool,
}

/// 計算一輪的行動順序（純邏輯，不操作 World）
/// rng_int: 產生 1~6 整數
/// rng_float: 產生 0.001~0.999 小數
pub fn calculate_turn_order(
    inputs: &[TurnOrderInput],
    rng_int: &mut impl FnMut() -> i32,
    rng_float: &mut impl FnMut() -> f64,
) -> Vec<TurnEntry>

/// 將當前單位延後到 target_index 位置（只能往後）
pub fn delay_unit(
    entries: &mut Vec<TurnEntry>,
    current_index: usize,
    target_index: usize,
) -> Result<()>

/// 取得下一個未行動的單位索引（從 from 開始搜尋）
pub fn next_active_index(entries: &[TurnEntry], from: usize) -> Option<usize>

/// 檢查本輪是否所有單位都已行動
pub fn is_round_complete(entries: &[TurnEntry]) -> bool

/// 移除指定 Occupant 的單位
pub fn remove_unit(entries: &mut Vec<TurnEntry>, occupant: Occupant) -> Option<TurnEntry>
```

### 步驟 4：在 `core/board/src/ecs_logic/turn.rs` 新增 World 操作

```rust
/// 開始新的一輪（擲骰、排序、存入 TurnOrder Resource）
pub fn start_new_round(world: &mut World) -> Result<()>

/// 結束當前單位的回合，推進到下一個單位；若全部結束則自動開始下一輪
pub fn end_current_turn(world: &mut World) -> Result<()>

/// 延後當前單位到指定位置
pub fn delay_current_unit(world: &mut World, target_index: usize) -> Result<()>

/// 移除死亡單位（從順序表中移除）
pub fn remove_dead_unit(world: &mut World, occupant: Occupant) -> Result<()>

/// 查詢當前回合狀態
pub fn get_turn_order(world: &World) -> Result<&TurnOrder>
```

### 步驟 5：更新 mod.rs

- `core/board/src/logic/mod.rs` 加入 `pub mod turn_order;`
- `core/board/src/ecs_logic/mod.rs` 加入 `pub mod turn;`

### 步驟 6：撰寫測試

在 `core/board/tests/logic/` 新增 `turn/` 目錄：

- 基本排序（INI 不同，確認 INI + 骰子高者先行動。有 INI 高但是 1d6 低所以順序在 INI 低的後面；INI 高且 1d6 不低所以順序在前面）
- 同分排序（INI + 骰子相同時，tiebreaker 決定順序）
- 結束回合後推進到下一個
- 所有單位結束後 `is_round_complete` 回傳 true
- 延後行動：移動到指定位置
- 延後驗證：不能往前延
- 移除單位後索引正確調整

### 步驟 7：修改 `battle.rs` — 浮動面板

進入戰鬥模式時自動呼叫 `start_new_round`。

用 `egui::Window` 在戰場上疊一個半透明浮動面板：

- 標題：「第 N 輪」
- 順序清單：每個條目顯示「名稱 (INI+骰=總分)」
- 當前行動單位高亮
- 已行動單位灰色
- 底部按鈕：「結束回合」、「延後」
- 延後模式：點「延後」後，清單中尚未行動的位置變成可點擊的插入點

### 步驟 8：更新索引文件

更新 `core-index.md` 和 `editor-index.md`。

## 注意事項

- 隨機骰透過注入 rng 函數，測試時傳固定值
- 所有戰鬥狀態（TurnOrder）存在 ECS World Resource，`LevelTabUIState` 不新增任何欄位
- 延後後 current_index 不變（指向延後後的下一個單位）
- 下一輪開始時重新擲骰，重新排序
- TurnEntry 是領域資料型別，放在 core_types.rs，不是 ECS Component
