# 戰棋單位移動功能

## 目標與範圍

### 目標

實作單位移動的核心功能：計算可達範圍（雙色顯示）、路徑回溯、執行移動。

### 範圍內

- 計算可達範圍，區分 1×MOV 和 2×MOV 兩種顏色
- 路徑回溯函數（從 `ReachableInfo` 重建完整路徑，供 hover 顯示路徑與預估消耗）
- 執行移動（驗證合法性 + 更新 Entity 的 Position component）
- 將 `HpModify` 重新命名為 `TerrainEffect`
- 停留驗證：不可停在其他單位格子上

### 範圍外（預留空間但不實作）

- 藉機攻擊（敵方主動移動離開相鄰格時觸發）
- 沿途地形效果觸發（經過 TerrainEffect 時，在 ecs_logic 內處理）

## 設計決策

### 行動模式與移動力

根據 GDD，每回合的移動預算為 `2 × MOV`。單位可以多次移動，系統追蹤本回合已消耗的移動力。

- 移動後若已消耗 `<= MOV`，仍可使用技能
- 移動後若已消耗 `> MOV`，不可使用技能（只能繼續移動）

UI 同時顯示兩個範圍：

- 從當前位置出發，消耗後總計 `<= MOV` 的範圍：顏色 A（移動後仍可用技能）
- 消耗後總計 `> MOV` 的範圍：顏色 B（移動後不可用技能）

`execute_move` 從 World 查詢單位的 `Movement` 屬性和 `MovementUsed`，計算剩餘預算 `2 × MOV - 已消耗`，作為 `reachable_positions` 的 budget。

### 移動力追蹤

新增 Component `MovementUsed(MovementCost)`，追蹤單位本回合已消耗的移動力。

- `execute_move` 成功後更新 `MovementUsed`
- 回合結束時（`end_current_turn`）重置為 0

### 位置查詢

不引入 `OccupantMap` 作為持久化 World Resource。改用查詢時建構的方式：提供函數從 ECS Query 建構臨時 HashMap，供 `reachable_positions` 的回呼使用。

理由：

- 單位數量約數十到上百，建構 HashMap 成本為微秒等級
- 回合制中每回合最多呼叫一兩次，建構頻率極低
- 不需要維護同步，不可能忘記更新

### 碰撞與停留規則

- 可穿越友軍，不可穿越敵軍或阻止通過的物件
- **不可停留在其他單位的格子上**（穿越 ≠ 停留）
- 現有 `reachable_positions` 已正確處理：結果過濾掉有 occupant 的格子

### 重新命名

- `HpModify` → `TerrainEffect`（未來可擴展為不只修改 HP 的地形效果）

### execute_move 內部驗證

`execute_move` 內部呼叫 `reachable_positions` 驗證目標位置合法性，不信任呼叫端，fail fast。

### MoveResult

```rust
pub struct MoveResult {
    pub path: Vec<Position>,   // 移動路徑（不含起點）
    pub cost: MovementCost,    // 實際消耗的移動力
}
```

未來擴展時可加入 `terrain_effects`、`opportunity_attacks` 等欄位。

## 實作步驟

### 1. 重新命名 HpModify → TerrainEffect

- 修改 `components.rs` 中的 struct 名稱
- 更新所有引用處（spawner、query、ObjectBundle、loader_schema）

### 2. logic/movement.rs — 新增 reconstruct_path

```rust
pub fn reconstruct_path(
    target: Position,
    reachable: &HashMap<Position, ReachableInfo>,
    start: Position,
) -> Vec<Position>
```

- 從 target 沿 `prev` 回溯到 start，反轉後回傳
- 不含起點，包含終點
- 測試：驗證路徑正確性、邊界情況（target == start 旁邊一格）

### 3. ecs_logic/movement.rs — 新增 get_reachable_positions

```rust
pub fn get_reachable_positions(
    world: &mut World,
    occupant: Occupant,
) -> Result<ReachableResult>
```

```rust
pub struct ReachableResult {
    pub reachable: HashMap<Position, ReachableInfo>,
    pub mov: MovementCost,            // 單位的 MOV 屬性
    pub movement_used: MovementCost,  // 本回合已消耗的移動力
}
```

UI 分色邏輯：對每個位置，`info.cost + movement_used <= mov` 為顏色 A，否則為顏色 B。

流程（遵守 World 操作集中原則）：

1. **讀取階段**：從 World 查詢 Board、單位的 Position/Faction/Movement/MovementUsed、所有單位位置與陣營、所有物件的 terrain_cost
2. **邏輯階段**：計算 `budget = 2 × MOV - movement_used`，呼叫 `reachable_positions`
3. **回傳**：組裝 `ReachableResult`

### 4. ecs_logic/movement.rs — 新增 execute_move

```rust
pub fn execute_move(
    world: &mut World,
    occupant: Occupant,
    target: Position,
) -> Result<MoveResult>
```

流程（遵守 World 操作集中原則）：

1. **讀取階段**：從 World 查詢 Board、移動單位的 Position/Faction/Movement/MovementUsed、所有單位位置與陣營、所有物件的 terrain_cost
2. **邏輯階段**：
   - 計算 `budget = 2 × MOV - movement_used`
   - 呼叫 `reachable_positions` 驗證 target 可達
   - 呼叫 `reconstruct_path` 取得路徑
   - 組裝 MoveResult
3. **寫入階段**：更新該 Entity 的 Position component，更新 MovementUsed

### 5. ecs_logic/query.rs — 新增位置查詢輔助函數

提供建構 occupant 位置 HashMap 的函數，供 `execute_move`、`get_reachable_positions` 和 editor 使用。

### 6. 測試

詳見下方測試計畫。

## 測試計畫

### A. `reconstruct_path` 單元測試（logic 層，加在 `tests/logic/board/test_movement.rs`）

1. **相鄰一格** — target 在 start 旁邊，回傳 `[target]`
2. **直線路徑** — 多格直線，驗證完整路徑順序
3. **L 型路徑** — 需要轉彎，驗證回溯正確
4. **繞過牆壁的路徑** — 有障礙時路徑正確繞行

### B. `execute_move` 整合測試（新建 `tests/ecs_logic/test_movement.rs`）

5. **合法移動** — Position 更新，MoveResult 的 path/cost 正確，MovementUsed 更新
6. **目標超出預算** — 回傳錯誤
7. **目標是友軍格子** — 被拒絕（不可停留）
8. **目標是敵軍格子** — 被拒絕
9. **穿過友軍到達目標** — 成功
10. **必須穿過敵人才能到達** — 被拒絕
11. **穿過高消耗物件阻擋** — 被拒絕（IMPASSABLE）
12. **穿過低消耗物件** — 成功但消耗較高

## 注意事項

- 沿途效果觸發的 hook 點在 `execute_move` 內部的路徑迭代中，未來在此處加入邏輯
- 藉機攻擊的 hook 點也在路徑迭代中（檢查離開的格子是否有相鄰敵人）
