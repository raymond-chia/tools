# 單位死亡處理 + 戰鬥 Log 重構

## 目標與範圍

兩件相關的事：

1. **單位死亡處理**：目前 core 完全沒有死亡判定。`apply_effect_entries` 只做 `hp.0 += final_amount`，HP 歸零或負數的單位仍存活在 World、TurnOrder、棋盤上。需要在 core 提供死亡判定與移除，由 editor 編排呼叫。
2. **戰鬥 Log 重構**：目前 log 直接重用 `EffectEntry`（給未來動畫用的低階結算資料），且 log 由 editor UI 持有（`Vec<EffectEntry>`）、散落三處 `extend`。需要拆出一個**獨立、人類可讀的 log 型別**，由 core 產生並持有。需要在 EffectEntry 註解未來用於動畫。

### 不做（本次縮減範圍）

- **不碰 `EffectEntry`**：保留給未來動畫，本次完全不動其定義。
- **不做動畫**：目前還沒處理到動畫。log 與動畫資料職責分開正是本次重構的理由。
- **log 不採階層結構**：採扁平（一事件一筆，學 Owlcat 的可讀做法）。
- **log 事件先只覆蓋三種**：技能、反應、死亡。傷害/治療不另譯成 log 事件（玩家從技能/反應那幾筆自己看出數值與因果）。

---

## 設計決策（已確認）

### D1. 死亡處理：規則在 core，編排由 UI

core 提供獨立的死亡處理函數；editor 在現有的反應/技能 loop 裡編排呼叫。

客觀比較後選此方向，主要理由：

- **避免膨脹**：`execute_skill`（skill.rs:394-551）與 `process_reactions` 已是讀取/驗證/純邏輯/寫入四段式長函數，再塞死亡掃描+log+剔除 pending 會嚴重膨脹，違反 single responsibility。
- **貼合現有架構**：editor 本就在編排 `process_reactions`、`force_advance_move`，多編排一個 `resolve_deaths` 風格一致、最好讀。
- **規則仍在 core**：判死、移除、剔除 pending 的規則本體都在 core 函數內，UI 只決定「何時呼叫」。這不是把規則拆給 UI，與現狀一致。
- **取捨**：UI 可能漏呼叫 → 漏處理死亡。但這與現狀「UI 可能漏呼叫 process_reactions」同類，editor 本就承擔編排責任，可接受。

（記錄：兩種方案都能達成正確行為（死者不出現在反應面板），差別純粹在程式碼組織。機制可行性已查證——見 D3。）

### D2. 死亡處理函數的職責

獨立函數（暫名 `resolve_deaths`）：

1. 掃描全場 HP≤0 的單位，收集成死者清單
2. 為每個死者產生死亡 log 事件（只記身分名稱快照）
3. **批次移除**：全部從 TurnOrder 移除 → 全部 despawn → **最後只判一次**是否全員行動完畢、要不要開新一輪
4. **死當前單位 → 新當前單位跑回合開始流程**：若死掉的是當前行動單位（或批次死光後開新一輪），遞補上來的新當前單位必須跑「回合開始流程」（清其過期 buff）。死非當前單位（且未換輪）時當前單位不變，不重跑。
5. 從 `ReactionState.pending` 剔除已死的反應者

對「無 `ReactionState`」（如 `execute_skill` 後）須安全處理（沒有 pending 可剔除就只做移除）。

**回合開始／結束流程抽成獨立函數**：原 `end_current_turn` 把推進後的副作用依作用對象拆成兩段——對「下一個單位」清過期 buff（回合開始）、對「剛結束者」重置 ActionState 與反應點數（回合結束）。把這兩段各抽成具名函數（`begin_unit_turn`、`end_unit_turn`），由 `end_current_turn` 與 `resolve_deaths` 共用，作為「回合開始／結束」的單一入口。避免兩處邏輯各自演化失同步，也讓 `resolve_deaths` 能正確複用回合開始流程。

- 取捨：`begin_unit_turn` 只含「清過期 buff」，不含 ActionState／反應點數重置（後者語意是剛結束者的善後，不屬於回合開始）。死當前單位被頂上來的新單位若本輪尚未行動，其 ActionState／反應點數本就是 spawn 初始滿值，不需重置。此假設依賴 `get_active_index` 只回傳未行動者。

**刪除 `remove_dead_unit`**：經確認該函數無正當用途（零呼叫者、無「逃跑/放逐」等單個離場需求）。其「單個移除 + 內建開新一輪」語意與批次移除 AOE 多死會打架（單個移除第一個死者後可能誤判全員行動完畢而提前開新一輪）。故把其邏輯吸收進 `resolve_deaths` 的批次移除後，刪掉 `remove_dead_unit`。`turn_order::remove_unit`（純邏輯，單個從 entries 移除）可保留供批次迴圈使用。

### D3. UI 編排時機

editor 在現有 loop 中，於以下時機呼叫 `resolve_deaths`：

- `execute_skill` 之後（主動技能單目標、多目標確認兩處）
- `process_reactions` 回傳 `Executed` 之後

查證依據：`process_reactions` 回傳 `Executed` 時 `ReactionState` 仍存在（reaction.rs:238-246：有 new_pending 則寫回 pending、否則若 decided 也空才 remove_resource）。因此 `Executed` 後 `pending` 還在，UI 此時呼叫 `resolve_deaths` 可安全剔除死者，下一輪反應面板就不會出現死者。

### D4. Log 由 core 持有，存成 Resource

依「未來移植 Godot 輕鬆」判準：log 是 game state 的一部分，存成 core Resource。editor 與未來 Godot 都只讀取渲染，不再各自累積。（暫定方向，若實作中發現造成維護困擾再議。）

### D5. Log 採扁平結構

一筆 log = 一個事件。技能對多目標 → 散成多筆判定結果。不做階層群組。

### D6. Log 事件涵蓋：技能、反應、死亡

新獨立 log 型別需能表達這三種事件，照**發生順序** append。

### D7. Log 事件寫入名稱快照（type name），不依賴事後反查

log 事件在產生當下就把單位的 `OccupantTypeName` 寫進去。死者被 despawn 後 log 仍能顯示「X 死亡」；editor 不再需要掃 `unit_map` 反查（現況 `find_unit_name_by_id` 在死者離場後查不到，會顯示「單位#id」）。Godot 移植時 log 自帶顯示字串。

- 死亡 log 只記**身分（名稱快照）**，不記死亡位置、不記致死來源。

### D8. Log 生命週期：`spawn_level` 建立，整場保留（修訂）

~~core 在 `start_new_round` 時清空 log Resource。~~

**修訂**：log Resource 改在 `spawn_level` 初始化，之後整場戰鬥持有同一份，`start_new_round` 不再 insert 或清空。跨場殘留由「每場重新 `spawn_level` 重建 World」化解。階段 2／3 取用 log 時以 `get_resource::<BattleLog>`（無則報錯）fail fast，不再容忍「尚未建立」。

### D9. editor 改讀 core 提供的 log

移除 editor 的 `battle_log: Vec<EffectEntry>` 與三處 `extend`，改為每幀讀取 core 的 log Resource 並由 core 提供的查詢函數取得渲染資料。

---

## 實作步驟

> **順序即實作順序**：core 部分遵守 TDD（CLAUDE.md：先寫失敗測試 → 實作通過）。下列步驟已把測試穿插在對應實作之前。editor 部分依 `rules/editor.md`「禁止替 editor crate 撰寫測試」，只能編譯 + 手動戰鬥驗證。

### 階段 1：core log 型別與 Resource

- [x] 定義獨立的扁平 log 事件型別（涵蓋技能、反應、死亡；含名稱快照）
- [x] 新增 log Resource（持有 log 事件序列）
- [x] 新增查詢 log Resource 的 pub 函數供前端讀取
- [x] log Resource 在 `spawn_level` 建立、整場保留（依 D8 修訂）
  - ~~（測試）`start_new_round` 清空 log Resource 的測試~~（D8 修訂後作廢：log 不在 `start_new_round` 清空）
  - ~~`start_new_round` 清空 log Resource~~（D8 修訂後作廢：改在 `spawn_level` 建立，`start_new_round` 不 insert 也不清空；跨場殘留由每場重新 `spawn_level` 重建 World 化解）

### 階段 2：core 死亡處理（resolve_deaths）

- [x]（測試）死亡判定：HP≤0 後 `resolve_deaths` 移除、TurnOrder 同步更新；AOE 多死批次移除只開一次新一輪。原 `remove_dead_unit` 兩個測試（移除、找不到報錯）的驗證行為併入此處
- [x]（測試）死者從 `ReactionState.pending` 剔除（反應流程中打死目標 → pending 不含死者）
- [x]（測試）死當前單位 → 新當前單位跑回合開始（過期 buff 被清）；死非當前單位 → 當前不變、不跑回合開始（過期 buff 保留）
- [x] 抽出 `begin_unit_turn`／`end_unit_turn` 兩個獨立函數，`end_current_turn` 改為呼叫它們（行為不變）
- [x] 新增 `resolve_deaths`：掃 HP≤0 → 產生死亡 log 事件（只記身分名稱快照）→ **批次移除**（全部從 TurnOrder 移除 → 全部 despawn → 只判一次開新一輪）→ 死當前單位則新當前單位跑 `begin_unit_turn` → 從 `ReactionState.pending` 剔除死者
- [x] `resolve_deaths` 對「無 `ReactionState`」情況安全處理
- [x] **刪除 `remove_dead_unit`**（邏輯吸收進 `resolve_deaths`；保留 `turn_order::remove_unit` 供批次迴圈使用）
  - 刪除 `test_turn.rs` 的 import 與兩個測試 `test_remove_dead_unit_removes_from_turn_order`、`test_remove_dead_unit_fails_when_occupant_not_found`
  - 更新 `get_current_unit` doc comment（移除對 `remove_dead_unit` 維護 `current_index` 的描述）
- [x] 檢視 `end_battle` 的「TODO 有用到嗎?」現況：仍被 `start_new_round`／測試用於開第二場戰鬥，無 bug；log 改由 `spawn_level` 持有後 `end_battle` 不需動 log。本次不變更，TODO 註解留待後續決定。

### 階段 3：core log 產生

- [ ]（測試）log 產生：技能/反應/死亡三種事件、名稱快照正確
- [ ] `execute_skill` 產生技能 log 事件並 append（含名稱快照）
- [ ] `process_reactions` 產生反應 log 事件並 append（含名稱快照）
- [ ] `resolve_deaths` 產生死亡 log 事件並 append（已含於階段 2）

### 階段 4：editor 改讀 core log + 編排死亡（不寫測試，手動驗證）

- [ ] 移除 `LevelTabUIState.battle_log: Vec<EffectEntry>`
- [ ] 移除三處 `battle_log.extend`（主動技能單目標、多目標確認、反應）
- [ ] 移除 deployment 的 `battle_log.clear`（改由 core `start_new_round` 負責）
- [ ] `execute_skill` 兩處呼叫後編排 `resolve_deaths`
- [ ] `process_reactions` 回傳 `Executed` 後編排 `resolve_deaths`
- [ ] 改寫 log 渲染為新 log 型別，並處理綁在舊 `EffectEntry` 的輔助函數：
  - `render_battle_log`（battle.rs:773，參數 `&[EffectEntry]`）改吃 core 新 log 型別
  - `render_effect_entry`（battle.rs:792）改寫，或以新型別的渲染取代
  - `format_check_target`（battle.rs:815）、`format_check`（battle.rs:825）、`format_effect`（battle.rs:864）：依新 log 型別是否仍需這些明細決定刪除或改寫
  - `find_unit_name_by_id`（battle.rs:873）：新 log 自帶名稱快照後可刪除（不再事後反查）
- [ ] 確認切換 `RightPanelView::Log` 的時機仍正確
- [ ] 編譯 + 手動戰鬥驗證（施放/反應/打死單位，確認 log 顯示與死亡移除正確）

### 階段 5：文檔

- [ ] 更新 `rules/core-index.md`（新增 `resolve_deaths`、log Resource、log 查詢函數；移除 `remove_dead_unit`）
- [ ] 更新 `rules/editor-index.md`（log 渲染函數簽名變動、刪除的輔助函數）
- [ ] 檢查是否違反 `README-設計機制.md`

---

## 注意事項（邊緣情況、已知限制、待實作查證）

- **【邊界】pending 剔除後變空但 Resource 仍在**：若死者被剔除後 `pending` 空、且 `decided` 也空，`ReactionState` 要到下一次 `process_reactions` 才會 `remove_resource`，中間有「pending 空但 Resource 還在」的狀態。需確認 `get_pending_reactions`（回傳空 Vec）與 editor 反應面板處理空 pending 不會卡住。實作時要測。
- **多單位同時死亡**：一次 effect 可能 AOE 同時打死多個單位。`resolve_deaths` 須批次處理——掃描全場收集所有 HP≤0 者，全部移除後**只判一次**是否開新一輪（不可逐個移除各判一次，否則第一個死者移除後可能誤判全員行動完畢而提前開新一輪）。每個死者各 append 一筆死亡 log。
- **死者參與 TurnOrder 與「開新一輪」誤觸**：批次移除後若全員行動完畢需開新一輪——需確認在反應流程中途移除不會誤觸開新一輪（反應發生在某單位回合內）。原 `remove_dead_unit` 的開新一輪邏輯吸收進 `resolve_deaths` 時要重新確認此時機正確。
- **死當前單位後新當前單位的回合開始流程**：死掉當前行動單位後，遞補上來的新當前單位必須跑回合開始流程（清過期 buff），否則該單位「被當前」卻沒清掉該過期的 buff。`end_current_turn` 與 `resolve_deaths` 共用抽出的 `begin_unit_turn`。判斷條件：當前 occupant 改變、或批次死光後開新一輪時才跑；死非當前單位（且未換輪）不重跑。換輪時即使新當前與原當前是同一 occupant，仍屬新一輪回合開始，須跑。
- **log Resource 生命週期**：依 D8 修訂，log 在 `spawn_level` 建立、整場保留，`start_new_round` 不清空；跨場殘留由每場重新 `spawn_level` 重建 World 化解。`end_battle` 不動 log。
- **HP 死亡門檻**：採 HP ≤ 0 為死亡。確認無「HP=0 仍存活」之類設計例外（查 `README-設計機制.md`）。
- **物件（Object）是否會死亡**：本次聚焦單位（Unit）死亡。Object 受傷/摧毀是否同機制處理，尚未納入範圍，待確認。
