# 勝利與失敗判定

## 目標

關卡結束判定，支援多種勝利與失敗方式（如消滅指定陣營），並保留擴充規則的可能性。

## 核心設計

判定維度為 faction（非 alliance）。勝敗都用「某 faction 是否全滅」這一原語表達，「保護盟友」（某友軍 faction 全滅則敗）也能自然表達，不需要「我方/敵方」概念。

資料型別（放 `domain`）：DNF 結構（析取範式），條件組合固定為兩層——上層 OR、內層 AND，key 內建：

```rust
enum EndLevelCondition {
    EliminateFaction(ID),
}

// 一個結局分支：key 為分支識別（多語系 key，導向不同後續劇情），Vec 內所有條件皆成立（AND）此分支才成立
// 保序 Vec：多分支同時成立時回傳第一個，穩定且測試好寫
type OutcomeBranches = Vec<(String, Vec<EndLevelCondition>)>;

enum LevelOutcome {
    Undetermined,     // 尚未判定（關卡初始化即為此，作為 Default）
    Victory(String),  // 觸發分支的多語系 key
    Defeat(String),   // 觸發分支的多語系 key
}
```

`LevelOutcome` 直接定義在 `ecs_types/resources.rs` 並 `derive(Resource)`，不再另包 newtype——它唯一用途就是當 Resource 存進 world，沒有 domain 層使用者，包一層無價值。

- `EliminateFaction(ID)`：指定 faction 全滅則此條件成立。
- 各結局分支之間是 OR，分支內 `Vec` 是 AND。例：`[("eliminate_all", [EliminateFaction(1), EliminateFaction(2)])]` = 同時消滅 faction 1 與 2。
- key 內建於結構：觸發哪一分支，該分支的 String key 就是結局的多語系 key，來源明確（設計者在關卡命名），不需另外決定 key 從哪來。
- `LevelOutcome`：表示「哪一結局」，帶觸發分支的多語系 key（core 只持有 key，翻譯交前端查文字表），作為判定回傳值。

勝負規則配置不另立型別：勝利與失敗規則（各一份 `OutcomeBranches`）直接放進 `LevelType`（`loader_schema.rs`），關卡配置本就是它的職責，不再多包一層。

**組合語意（重要，勿再誤解）**：一個 `Vec<EndLevelCondition>` 是**一個結局分支的達成條件**，分支內所有 `EndLevelCondition` 皆成立才算此分支成立（**分支內 AND**）。`OutcomeBranches`（`Vec<(String, Vec<EndLevelCondition>)>`）是**多個結局分支**，任一分支成立即整份規則成立（**分支間 OR**，各分支導向不同後續劇情）。key 是各分支的識別（多語系命名），屬配置語意。

**不設 `logic` 純邏輯層**：單一 `EliminateFaction(id)` 判定只是 `!alive_factions.contains(&id)` 一行，太薄不值得獨立成層。分支內 AND、分支間 OR、勝敗優先序、單一葉子判定**全部放 `ecs_logic` world 入口**。組合邏輯的正確性**用整合測試覆蓋**（透過 world 建關卡驗證結局），不做純邏輯 TDD。

**world 入口（放 `ecs_logic`）**：負責 world 查詢、單一葉子判定、分支內 AND、分支間 OR、勝敗語意與優先序、key 對應、寫入結果。流程：撈存活 faction 集合 → 依 **defeat 優先於 victory**，各自遍歷該份 `OutcomeBranches` 的每一分支、對分支內每個 `EndLevelCondition` 就地判定（`EliminateFaction(id)` 即 `!alive.contains(&id)`）並取分支內 AND，取**第一個成立分支**的 String key（分支間 OR、保序，多分支同時成立取定義順序第一個）→ defeat 成立包成 `Defeat(key)`；否則 victory 成立包成 `Victory(key)`；皆不成立為 `None` → 寫入結果 Resource。存活 faction 只撈一次，兩份規則共用。

接點：在 `resolve_deaths`（`ecs_logic/turn.rs`）死亡結算之後呼叫上述 world 入口，把結果寫入 Resource 供前端查詢。`end_battle` 職責不變（清理 TurnOrder），與關卡結局判定分離。

TOML：勝利與失敗規則放在 `LevelType` 內（`[level]` 下）。用 serde 預設 tag derive `Deserialize`（editor 產生結構，不需人手寫）。需改 `loader_schema` 與 `spawner`。

縮減範圍：只做 `EliminateFaction`（KISS）。條件組合限 DNF（OR of ANDs），無法表達巢狀如 `A AND (B OR C)`——對關卡勝負為合理縮減。`SurviveTurns`、`ReachPosition` 等未來葉子條件預留不實作。本次不動前端。

## Checklist

- [ ] `domain`：定義 `EndLevelCondition`、`OutcomeBranches`（`LevelOutcome` 不放此處）
- [ ] `ecs_logic`：實作 world 入口（撈存活 faction 集合 → 依 defeat 優先遍歷各分支，分支內 `EliminateFaction` 就地判定取 AND、分支間 OR 取第一個成立分支 key → 包成 `LevelOutcome` → 寫入結果 Resource）
- [ ] 整合測試：透過 world 建關卡驗證結局（分支內 AND、分支間 OR、defeat 優先於 victory、皆不成立為 `None`）
- [ ] `ecs_types/resources`：定義 `LevelOutcome`（derive `Resource`）與存勝敗規則的 Resource
- [ ] `loader_schema` + `spawner`：TOML 載入勝利/失敗規則存入 World（實作後跑一次真實 TOML 載入驗證）
- [ ] `ecs_logic/turn.rs`：`resolve_deaths` 後接 `resolve_level_outcome`，寫入結果 Resource
- [ ] `ecs_logic/query`：新增查詢結果 Resource 的公開函數供前端使用
- [ ] 更新 `.claude/rules/core-index.md`

# Backup

- [ ] 沒觸發藉機攻擊就繼續移動
- [ ] 技能豁免類型
- [ ] 預覽拆解閃避細項
- [ ] 預覽拆解格擋細項
- [ ] log 拆解命中、閃避、格擋細項
- [ ] 移動反悔
