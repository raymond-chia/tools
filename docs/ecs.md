## SimulationState 改用 ECS

### 背景

目前 `SimulationState` 用 `HashMap<usize, TypeName>` 維護部署狀態，
`get_unit_at_position` 以手動查詢方式取得位置上的單位。
目標是讓 `SimulationState` 持有 `bevy_ecs::world::World`，
把單位、玩家部署和物件都存入 ECS，透過 ECS 取得指定位置的單位與物件。

### 架構決策

#### UI 與 ECS 的互動方式

採用 **ViewModel + Command** 模式，讓 UI 層與 ECS 完全解耦。

資料流方向（單向循環）：

```
使用者操作 → UI 產生 Command → 普通函數處理邏輯 → 更新 ViewModel → UI 讀取顯示
```

**三個核心概念：**

| 概念      | 方向     | 用途       | 說明                                     |
| --------- | -------- | ---------- | ---------------------------------------- |
| Command   | UI → ECS | 玩家操作   | enum，UI 將操作轉成指令丟進 queue        |
| ViewModel | ECS → UI | 持續狀態   | struct，UI 每幀讀取來渲染畫面            |
| Event     | ECS → UI | 一次性通知 | 動畫觸發、音效等（目前不需要，未來再加） |

**原則：**

- UI 層只認識 Command 和 ViewModel，不知道 `bevy_ecs` 的存在
- UI 不直接 query World，不持有 Entity
- ViewModel 的結構由 UI 需求決定，不是 component 的 1:1 映射
- 將來換 UI 框架（如 godot）時，只需重寫 UI 層，Command/ViewModel 定義不變

#### 不使用 bevy Schedule

直接用普通函數操作 `World`，不使用 bevy 的 `Schedule` 或 `System`。

**理由：**

- 回合制遊戲：玩家操作一次才處理一次，沒有「每幀跑多個 system」的需求
- 處理流程是線性的（驗證 → 執行 → 更新 ViewModel），順序由我們控制
- 不需要自動平行化，單執行緒足夠
- Schedule 的好處（自動排序、平行化）對回合制無意義，反而增加複雜度
- 直接操作 World 的寫法和 System 幾乎一樣，差別只在 `world.query::<&T>().iter(world)` vs `Query<&T>` 參數，未來要遷移到 Schedule 很簡單

#### UI 框架與執行緒

egui 和 godot 都在主執行緒執行 Rust 邏輯，不需要多執行緒同步。
UI 在同一執行緒持有 `&mut World`，透過普通函數呼叫處理 command。

### 流程範例

#### 右鍵點選單位顯示資訊

1. 使用者右鍵點擊格子
2. UI 將螢幕座標轉為棋盤座標，push `Command::SelectUnit { position }`
3. 處理函數收到 command，query 該位置的單位，更新 ViewModel 的 `selected_unit`
4. 下一幀 UI 讀取 `view_model.selected_unit`，畫出資訊面板

#### 選單位 → 移動

1. 使用者左鍵點擊己方單位
2. UI push `Command::SelectUnit { position }`
3. 處理函數確認是己方單位，計算可移動範圍，更新 ViewModel：`reachable_tiles`、`mode = AwaitingMoveTarget`
4. UI 讀取 ViewModel，畫出高亮的可移動格子
5. 使用者點擊目的地
6. UI 檢查目的地在 `reachable_tiles` 內（前置過濾），push `Command::MoveUnit { target }`
7. 處理函數驗證合法性（最終驗證），執行移動，更新 ViewModel
8. UI 讀取更新後的 ViewModel，顯示新位置

**驗證職責分配：**

- UI 做簡單前置過濾（如檢查是否在可移動範圍內）→ 為了使用者體驗
- ECS 處理函數做最終驗證 → 為了資料正確性

### Component 設計

新增到 `core/board/src/component.rs`：

```rust
// 共用
pub struct OccupantTypeName(pub TypeName);
pub struct Unit;   // tag：標記此 entity 為單位
pub struct Object; // tag：標記此 entity 為物件

// Unit 專用
pub struct Skills(pub Vec<SkillName>);

// Object 專用
pub struct TerrainMovementCost(pub MovementCost);
pub struct HpModify(pub i32);
pub struct BlocksSight; // tag
pub struct BlocksSound; // tag
```

**Unit entity 掛載的 components：**
`Position` + `Faction` + `OccupantTypeName` + `Skills` + `Unit`（tag）

**Object entity 掛載的 components：**
`Position` + `OccupantTypeName` + `TerrainMovementCost` + `HpModify` + `BlocksSight`（可選 tag）+ `BlocksSound`（可選 tag）+ `Object`（tag）

### Command 設計

UI 發送給 ECS 的操作指令，放在 `core/board/src/command.rs`：

```rust
pub enum GameCommand {
    SelectUnit { position: Position },
    DeselectUnit,
    DeployUnit { position: Position, type_name: TypeName },
    RemoveDeployedUnit { position: Position },
    MoveUnit { target: Position },
    // 之後擴充：AttackUnit, UseSkill, EndTurn ...
}
```

### ViewModel 設計

ECS 產出給 UI 讀取的顯示資料，放在 `core/board/src/view_model.rs`：
ViewModel 由 UI 需求決定欄位，不是 component 的 1:1 映射。
例如 UI 需要 `hp_percentage` 但 ECS 裡不存在，就放在 ViewModel 裡算好。

```rust
pub struct UnitInfoViewModel {
    pub name: TypeName,
    pub faction: Faction,
    pub position: Position,
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
}

pub struct TileViewModel {
    pub position: Position,
    pub units: Vec<UnitInfoViewModel>,
    pub is_reachable: bool,
    pub is_deployment_point: bool,
}

pub struct GameViewModel {
    pub mode: GameMode, // Idle, UnitSelected, AwaitingMoveTarget ...
    pub selected_unit: Option<UnitInfoViewModel>,
    pub tiles: Vec<TileViewModel>,
    pub reachable_tiles: Vec<Position>,
    pub deployment_points: Vec<Position>,
}
```

### World 操作函數設計

放在 `core/board/src/system/` 下，是普通函數（不是 bevy System），但因為直接操作 `&mut World` 和 `&World`，所以歸在 system 層。

1. **處理 Command**：消費 command queue，操作 World（spawn/despawn/修改 component）
2. **產出 ViewModel**：query World，組裝成 ViewModel struct

`logic/` 不碰 bevy，只做純邏輯運算（如移動計算、屬性計算）。
`system/` 負責 World 的讀寫，是 logic 和 World 之間的橋樑。

```
process_commands(&mut World, Vec<GameCommand>) → 修改 World
build_game_view_model(&World) → GameViewModel
```

查詢寫法範例（對比 bevy System，僅供參考）：

| 操作          | 普通函數寫法                      | bevy System 寫法 |
| ------------- | --------------------------------- | ---------------- |
| 查詢          | `world.query::<&T>().iter(world)` | `Query<&T>`      |
| 讀 Resource   | `world.get_resource::<T>()`       | `Res<T>`         |
| 取單一 entity | `world.entity(e).get::<T>()`      | `query.get(e)`   |
| Spawn         | `world.spawn(bundle)`             | `Commands`       |

### SimulationState 改動

檔案：`editor/src/tabs/level_tab.rs`

- 移除 `Clone` derive（`World` 不可 Clone）
- 移除欄位：`deployed_units`、`selected_deployment_point`
- 新增欄位：`world: bevy_ecs::world::World`
- 新增欄位：`view_model: GameViewModel`（每次處理完 command 後更新）
- 新增欄位：`commands: Vec<GameCommand>`（UI 每幀塞入，處理函數消費）
- 新增 `SimulationState::new(...)` 建構 World 和初始 ViewModel
- 新增 `SimulationState::process_commands(...)` 消費 command queue，呼叫普通函數處理邏輯，更新 ViewModel

### UI 層的職責

UI（egui）每幀只做兩件事：

1. **讀取 ViewModel → 繪製畫面**
   - 讀 `view_model.tiles` 畫棋盤
   - 讀 `view_model.selected_unit` 畫側邊資訊面板
   - 讀 `view_model.reachable_tiles` 畫高亮格子

2. **收集使用者輸入 → 產生 Command**
   - 右鍵點格子 → push `GameCommand::SelectUnit { position }`
   - 左鍵點可移動格子 → push `GameCommand::MoveUnit { target }`
   - 點部署按鈕 → push `GameCommand::DeployUnit { ... }`

### LevelTabUIState 移除欄位

檔案：`editor/src/tabs/level_tab.rs`

移除：

- `temp_unit_name: Option<TypeName>`
- `skills_map: HashMap<SkillName, SkillType>`
- `units_map: HashMap<TypeName, UnitType>`
- `deployed_units`、`selected_deployment_point`（移入 World）

所有資料改從 ViewModel 取得。
