# 使用技能機制

## 目標與範圍

在 core/board 實作「使用主動技能」的完整流程，從查詢可用技能到執行判定。效果暫時只寫 log，不實際套用（不扣 HP、不施加 Buff）。不處理反應技能。

## 設計決策

### 屬性儲存：應改為單一 struct component

- 所有場景都是整體存取，拆開只增加查詢簽名複雜度
- 這是重構，不在這次範圍

### 技能儲存：`Skills(Vec<SkillName>)` 單一 component

- 所有場景都是遍歷全部技能，沒有「只查某個技能」的獨立需求
- 數十個技能 × 幾十個單位，遍歷成本可忽略
- 拆成獨立 entity 增加關聯管理複雜度，換來的 query 過濾能力在這個規模用不到
- 維持現狀，不需修改

### AP 追蹤：`ActionState` enum component

```rust
enum ActionState {
    Moved { cost: MovementCost },
    Done,
}
```

初始值 `Moved { cost: 0 }`。

| 比較方案                          | 優點                                   | 缺點                                       |
| --------------------------------- | -------------------------------------- | ------------------------------------------ |
| `Moved/Done` enum                 | 一個欄位管所有狀態，不可能出現矛盾狀態 | 移動消耗綁在 enum 裡，其他系統要從 enum 取 |
| `RemainingAp(u8)`                 | 最簡單                                 | 無法表達「技能只能用一次」，語義丟失       |
| `MovementUsed + SkillUsed` 兩欄位 | 各自語義清晰                           | 隱含約束沒有型別保護，可能出現矛盾狀態     |

選擇 enum 方案，因為規則本身就是狀態機。

### 查詢可選範圍：由 ecs_logic 提供，不讓 UI 自行計算

- UI 不應持有 World 查詢邏輯
- 拆成兩個函數：可選格子 + AOE 預覽

## 玩家使用技能流程

### 前提

- `start_new_round` 已呼叫，有 active unit
- 單位的 `ActionState` 為 `Moved { .. }`（未 Done）

### 步驟

1. **查詢可用技能** — `get_available_skills(world) -> Vec<AvailableSkill>`
   - 從 TurnOrder 取得當前行動單位
   - 查詢 `Skills` component → 對應 `GameData.skill_map` 取得 Active 技能
   - MP 不足的技能也回傳，標記 `usable: false`（供 UI 顯示灰色）
   - 只回傳 Active 技能

2. **查詢可選目標格子** — `get_skill_targetable_positions(world, skill_name) -> Vec<Position>`
   - 根據技能的 `Target.range` + 施放者位置，計算射程內格子
   - 根據 `Target.selection`（Unit/Ground）和 `Target.selectable_filter` 過濾
   - 回傳可選格子集合（供 UI 高亮）

3. **預覽 AOE 影響範圍** — `preview_skill_affected_positions(world, skill_name, target_pos) -> Vec<Position>`
   - 玩家 hover 某格時呼叫
   - 根據技能的 `Area` 計算影響格子（內部呼叫 `compute_affected_positions`）

4. **執行技能** — `execute_skill(world, skill_name, targets: Vec<Position>) -> SkillResult`
   - 驗證 `ActionState` 不是 `Done`
   - 驗證 MP 足夠 → 扣除 MP
   - 呼叫 `select_skill_targets` 驗證並解析目標
   - 對每個目標：根據 `EffectNode` 遍歷效果樹，遇到判定（Hit/DC）執行檢定
   - 效果暫時只寫 log
   - 設定 `ActionState` 為 `Done`
   - 回傳 `SkillResult`（每個目標的判定結果）

## 實作步驟

1. 新增 `ActionState` enum component
   - 加入 `UnitBundle`
   - 修改 `spawner` 初始化為 `Moved { cost: 0 }`
2. 修改 `execute_move` 改用 `ActionState` 取代 `MovementUsed`
3. 修改 `end_current_turn` 重置 `ActionState` 為 `Moved { cost: 0 }`
4. 在 `ecs_logic/` 新增 `skill.rs`，實作：
   - `get_available_skills`
   - `get_skill_targetable_positions`
   - `preview_skill_affected_positions`
   - `execute_skill`
5. 在 `logic/skill.rs` 補充需要的純邏輯函數（如計算射程內格子）
6. 撰寫測試

## 注意事項

- `MovementUsed` component 被 `ActionState` 取代，需移除並更新所有引用處
- `can_delay_current_unit` 目前檢查 `MovementUsed`，需改為檢查 `ActionState` 是否為 `Moved { cost: 0 }`
- 效果執行（HpEffect、ApplyBuff、ForcedMove 等）留待下一次實作
- 反應技能的執行留待下一次實作
