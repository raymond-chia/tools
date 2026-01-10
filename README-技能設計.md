# 技能效果設計文檔

本文件列出**功能導向**的 skill effect 建議，配合遊戲設計理念「戰術規劃 + 地形利用 + 資源管理」。

## 設計原則

1. **功能導向優先**：技能效果應提供戰術選擇，而非單純數值變化
2. **支援團隊協同**：實現「角色使用不同技能合作達成戰術」
3. **配合地形系統**：與現有的 Terrain/Object/Shove 系統產生協同
4. **深化資源管理**：配合「MP 是一天的限制」設計
5. **層次化實作**：優先實作與現有系統協同度高的效果
6. **優先使用既有 Effect**：如果既有 Effect 可以達成相同功能，就不新增
7. **⚠️ 符合現實或 DND/PF**：只採用符合現實邏輯**或者**出現在 DND/Pathfinder 中的效果
   - ❌ 避免純遊戲化機制（如 Taunt「強制攻擊目標」）
   - ✅ 參考 DND 5e, Pathfinder 2e 的法術和能力
   - ✅ 或者有合理的現實解釋（如位置控制、物理阻擋）

## 既有 Effect 總覽

**已實作的立即效果**：

- `Hp` - HP 變化（傷害/治療）
- `Mp` - MP 變化（消耗/恢復）
- `Shove` - 推擊

**已定義但未實作的持續效果**：

- `MaxHp`, `MaxMp` - 最大值調整
- `Initiative` - 先攻調整
- `Accuracy` - 命中調整
- `Evasion` - 閃避調整
- `Block` - 格擋調整
- `BlockReduction` - 格擋減傷調整
- `MovePoints` - 移動力調整
- `Burn` - 燃燒（DoT）
- `HitAndRun` - 打了就跑

---

## 核心機制系統（支援技能設計）

### 位置機制（Location Mechanics）

**重要性**：符合「戰術規劃 + 地形利用」設計理念

- **視線（Line of Sight）**

  - 牆壁、樹木阻擋視線
  - 無視線無法遠程攻擊
  - 戰術：躲在牆後避免被攻擊

- **掩體（Cover）**

  - Half Cover: +20 evasion
  - Full Cover: +40 evasion
  - 戰術：利用地形物件提升防禦

- **高地優勢（High Ground）** ⏸️ **暫緩實作**

  - 實作方式：使用 Object（如 HighGround），而非每格高度屬性
  - 高地攻擊低地：+15 accuracy
  - 低地攻擊高地：-10 accuracy
  - 戰術：佔領高地遠程輸出

- **夾擊（Flanking）**

  - 對立兩側包圍敵人：+10 accuracy
  - 戰術：兩個單位配合包圍

- **光線（Light）**
  - Bright: 正常
  - Dim
  - Darkness
  - 戰術：夜戰需要光源/給潛行職優勢

---

### 位移機制（Movement & Terrain Manipulation）

**重要性**：實現「製造地形 + 利用地形 + 元素反應」

- **製造地形（已規劃）**

  - Spike Growth: 尖刺區域（傷害 + 減速）
  - Grease: 油膩區域（摔倒判定）
  - 未來：Ice（滑動）、Fire（燃燒區）

- **利用地形（已實作）**

  - Pit + Shove = 即死
  - 未來：Grease + 火球 = 燃燒爆炸

- **元素反應（簡化版）**
  - Grease + 火焰傷害 → Fire 地形 + 爆炸
  - 未來：淺水 + 冰凍 → Ice 地形（滑動）
  - 暫緩複雜互動（Wet + 電擊等）

---

### 反應動作（Reactions）

**重要性**：增加戰術深度，參考 D&D 5e/Pathfinder 2e

- **借機攻擊（Opportunity Attack）**：敵人離開相鄰格時攻擊
- **反制施法（Counterspell）**：打斷敵人施法
- **保護（Protection）**：隊友被攻擊時跳到前面承受
- **反射（Shield Spell）**：受到攻擊時提升防禦

**實作難度**：高（需要事件系統）
**優先級**：第四階段

---

### 壓力/士氣機制（Morale System）⏸️ **暫緩實作**

**重要性**：增加沉浸感，參考 Darkest Dungeon

- **士氣值**（0-100）影響戰鬥表現
  - 低士氣（0-20）：恐懼，可能逃跑，accuracy -30
  - 高士氣（80-100）：鼓舞，accuracy +10
- **士氣變化事件**
  - 隊友陣亡 -20
  - 擊殺敵人 +10
  - Boss 陣亡（敵方）-30
  - 受爆擊 -5

**實作難度**：中等
**優先級**：暫緩（先專注核心位置戰術）

---

### 戰爭迷霧（Fog of War）

**重要性**：支援偵查、埋伏戰術

- **已探索 vs 可見**
  - 可見區域：顯示所有資訊
  - 已探索：只顯示地形
  - 未探索：黑色
- **視野計算**
  - 基於單位位置 + 視線
  - 偵查單位視野更廣
- **戰術應用**
  - 需要偵查探索地圖
  - 埋伏戰術
  - 視野控制

**實作難度**：中高（需要渲染系統配合）
**優先級**：第四階段

---

## TODO

## 創造地形系統設計（2026-01）

### 設計目標

實現「創造地形」技能效果，包括：

- **Grease**（油脂）：增加移動成本、絆倒效果
- **Pillar**（石柱）：永久障礙物，阻擋移動和視線
- **Smoke**（煙霧）：臨時視線阻擋

### 核心設計決策

#### 1. 統一 Object 系統

- **移除** `Tile.object` 欄位
- **Object 枚舉** → 改名為 **ObjectType**（保留原有變體：Tree, Wall, Cliff, Pit, Tent, Torch, Campfire）
- **新增 Object 結構**：
  - `id`: ObjectID
  - `affected_positions`: Vec<Pos>（支持多格，單格物體也是 vec![pos]）
  - `object_type`: ObjectType
  - `duration`: i32（-1 = 永久，> 0 = 臨時）
  - `creator_team`: TeamID（None = 地圖預設的中立物件）
- **BoardConfig 改動**：
  - 新增 `objects: BTreeMap<ObjectID, Object>`
- **Board 改動**：
  - 新增 `objects: HashMap<ObjectID, Object>`
  - 新增 `pos_to_object: HashMap<Pos, Vec<ObjectID>>`（支持多個物體疊加）

#### 2. 臨時物體進入 turn_order

- **永久物體**（duration = -1）：不進入 turn_order
- **臨時物體**（duration > 0）：進入 turn_order，輪到時減少 duration，到 0 時自動消失
- **TurnEntity**：枚舉 `Unit(UnitID)` 或 `Object(ObjectID)`
- **Battle.turn_order** 類型改為 `Vec<TurnEntity>`

#### 3. Tag 機制處理環境互動

- **新增 Tag**：
  - `Tag::Ignite`：點燃範圍內的 Torch/Campfire
  - `Tag::Extinguish`：熄滅範圍內的 Torch/Campfire
- **自動觸發**：技能施放後檢查 tag，自動點燃/熄滅（不需要額外視線檢查）
- **火焰技能建議**：同時擁有 `Tag::Fire`（傷害類型）+ `Tag::Ignite`（環境互動）

### 新增物體類型

#### ObjectType::Wall（石牆）

- **永久**（duration = -1）
- **阻擋移動**：blocks_movement() = true
- **阻擋視線**：blocks_sight() = true
- **創造方式**：Effect::CreateObject

#### ObjectType::Grease（油脂）

- **臨時**（duration > 0，建議 3-5 回合）
- **移動成本**：movement_cost_modifier() = +5
- **絆倒效果**：
  - **觸發**：單位在 grease 上的時候, 無論是進入還是回合開始就在. 每個回合只檢定一次
  - **判定**：豁免判定（Reflex）
  - **失敗效果**：
    - 失去剩餘移動力（`unit.moved` 往上到 `unit.move_points` 或者 `unit.move_points * 2` (如果 `unit.moved` 本來就超過 `unit.move_points`)）
    - 降低命中、閃避（添加 Tripped status_effect，持續 1 回合）

#### ObjectType::Smoke（煙霧）

- **臨時**（duration > 0，建議 2-3 回合）
- **阻擋視線**：blocks_sight() = true
- **不阻擋移動**：blocks_movement() = false

### 新增 Effect 和 Tag

#### Effect::CreateObject

- **參數**：
  - `target_type`: TargetType
  - `shape`: Shape
  - `object_type`: ObjectType
  - `duration`: i32（-1 = 永久，> 0 = 臨時）
- **功能**：創造物體（永久或臨時）
- **支持多格**：根據 shape 計算 affected_positions

#### Tag::Ignite / Extinguish

- **Ignite**：點燃 Torch/Campfire（目前只處理這兩種，未來可擴展到可燃物）
- **Extinguish**：熄滅 Torch/Campfire
- **自動觸發**：技能施放後自動檢查 tag 並應用效果

### ObjectType 行為方法

所有 ObjectType 實作以下方法：

- `blocks_movement()` - 是否阻擋移動
- `blocks_sight()` - 是否阻擋視線（Cliff 需要特殊處理方向性）
- `movement_cost_modifier()` - 移動成本修正
- `light_level_at(distance)` - 光照等級計算

### 實作步驟（6 個階段）

1. **Phase 1: 基礎結構重構** (已完成)

   - 定義 ObjectType、Object、Board.objects
   - 遷移所有 tile.object 代碼
   - 實作 ObjectType 行為方法
   - 更新序列化格式

2. **Phase 2: TurnEntity 和回合系統** (已完成)

   - 定義 TurnEntity 枚舉
   - 修改 Battle.turn_order 類型
   - 實作 process_object_turn()（減少 duration）
   - 實作 remove_entity_from_turn_order()（統一移除邏輯）

3. **Phase 3: 創造物系統**

   - 新增 Effect::CreateObject
   - 實作創造邏輯（生成 Object、插入 turn_order）
   - Object 消失時的清理

4. **Phase 4: Tag 點燃/熄滅機制**

   - 新增 Tag::Ignite、Tag::Extinguish
   - 實作 ignite_objects_at()、extinguish_objects_at()
   - 在技能施放時自動觸發

5. **Phase 5: 遊戲邏輯整合**

   - 移動成本計算（檢查 pos_to_object）
   - 可通行性檢查
   - 視線檢查
   - 絆倒判定（grease）

6. **Phase 6: 測試**
   - 基礎測試、創造物測試、duration 測試
   - 點燃/熄滅測試、疊加測試

### 技能範例

**創造石柱**（永久障礙）：

- tags: [Active]
- range: (1, 5)
- cost: -5
- effects: [CreateObject { target_type: Ground, shape: Point, object_type: Wall, duration: -1 }]

**油脂術**（Grease，臨時地形）：

- tags: [Active]
- range: (1, 10)
- cost: -10
- effects: [CreateObject { target_type: Ground, shape: Circle(radius=1), object_type: Grease, duration: 3 }]

**煙霧彈**（臨時視線阻擋）：

- tags: [Active]
- range: (1, 10)
- cost: -5
- effects: [CreateObject { target_type: Ground, shape: Point, object_type: Smoke, duration: 2 }]

**火球術**（傷害 + 點燃）：

- tags: [Active, Fire, Ignite]
- range: (1, 10)
- cost: -15
- accuracy: 70
- crit_rate: 10
- effects: [Hp { target_type: Enemy, shape: Circle(radius=1), value: -30 }]
- 自動觸發：點燃範圍內的 Torch/Campfire
