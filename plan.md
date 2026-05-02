# 反應技能設計

## 流程

1. `execute_move` / `execute_skill` 執行後，偵測可反應的單位，存入 resource
2. UI 呼叫 `get_pending_reactions()`，取得所有待決定的反應者清單（含觸發者）
3. 玩家一次決定每個人用哪個技能（或放棄）與執行順序
4. UI 呼叫 `set_reactions(decisions, order)`，將決定存入 resource
5. UI 一直呼叫 `process_reactions()`，每次執行一個反應並回傳結果
6. `process_reactions()` 自動收集新反應

## UI 職責

- 呼叫 `execute_move` / `execute_skill` 後檢查是否有待反應
- 顯示選項（含觸發者資訊），等玩家完成決定後呼叫 `set_reactions`
- 一直呼叫 `process_reactions` 直到收到「需要新決定」或「結束」
