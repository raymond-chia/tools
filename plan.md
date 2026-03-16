# 移動與反應交替處理

## 目標與範圍

設計移動過程中觸發反應技能的機制。移動逐格進行，每一步可能觸發周圍單位的反應技能（如藉機攻擊）。

### 範圍內

- 移動路徑掃描，找到第一個觸發反應的步驟
- 收集該步驟所有符合條件的反應者
- 返回反應列表給外部處理

### 範圍外

- 反應技能的實際結算（外部處理）
- 反應結算後的後續移動（玩家自行決定再次呼叫移動）
- AI 的反應決策邏輯

## 設計決策

### 觸發條件

- `from` 在反應者監視範圍內且 `from != to`，就觸發
- 被推不算主動移動，不觸發 `OnAdjacentUnitMove`
- 同一次移動中，同一反應者的範圍被經過多次，可觸發多次（受反應次數限制）

### 觸發後行為

- 觸發反應後，移動者停在觸發的那一格（`to`）
- 已走過的格子照常消耗移動力
- 外部結算所有反應後，移動者停下，由玩家自行決定是否用剩餘移動力繼續移動

### 反應者選擇

- 由反應者所屬玩家決定觸發哪些反應、以什麼順序觸發
- 所有選定的反應全部觸發完畢後，才判斷移動者是否死亡

### 反應次數

- `Reaction` component 記錄每回合可用反應次數（通常為 1）
- 每次觸發反應消耗 1 次
- 反應次數足夠時，同一單位可對同一次移動觸發多次

### 連鎖

- 反應可以觸發反應（例如反應攻擊觸發「被攻擊時反擊」）
- 但被推移動不算主動移動，不觸發 `OnAdjacentUnitMove`

## 實作步驟

### 1. 修改 `OnAdjacentUnitMove` 加入 range

`loader_schema.rs` 中 `OnAdjacentUnitMove` 加入 `range: Coord`，目前藉機攻擊設定 `range: 1`。

### 2. 新增 `logic/skill.rs` 的 `collect_move_reactions` 函數

純邏輯函數，只收集 `OnAdjacentUnitMove` 類型的反應。輸入路徑與場上單位資訊，輸出第一個觸發點的反應列表。

```
fn collect_move_reactions(
    mover: &UnitInfo,
    path: &[Position],
    units_on_board: &HashMap<Position, ReactionUnitInfo>,
) -> CollectMoveReactionsResult
```

掃描邏輯：

- 遍歷路徑的每一步 `from → to`（`from != to`）
- 對 `from` 周圍所有單位，檢查是否有 `OnAdjacentUnitMove` 技能且 `range` 涵蓋 `from`
- 篩選 `unit_filter` 匹配、反應次數 > 0、非移動者自身
- 找到第一個有反應的步驟就停止

返回值：

- 無反應：完整路徑的結果
- 有反應：截斷路徑的終點（`to`）、反應者列表（含可觸發的技能）

### 3. 修改 `ecs_logic/movement.rs` 的 `execute_move`

- 計算路徑後，呼叫 `collect_move_reactions`
- 根據結果決定實際移動終點（可能是路徑中途）
- `MoveResult` 新增反應列表欄位

### 4. 測試

- [x] 無反應者：正常移動完成
- [x] 有反應者在路徑中途：移動停在 `to`，返回反應列表
- [x] 多個反應者同一步觸發：全部收集
- [x] 反應次數為 0：不觸發
- [x] 路徑經過同一反應者兩次、反應次數足夠：觸發兩次（在第一次就停下）
- [x] `unit_filter` 不匹配：不觸發
- [x] 移動者自身不觸發自己的反應
- [x] range > 1 — 反應者 range=2，距離 2 也能觸發

## 注意事項

- `collect_move_reactions` 是純邏輯函數，放在 `logic/skill.rs`，不依賴 ECS
