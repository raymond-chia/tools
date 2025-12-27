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

## 一、控場類（Control）- 限制敵人行動

### Stun（暈眩）✨ 需要新增

```rust
Stun {
    target_type: TargetType,
    shape: Shape,
    duration: i32,  // 持續回合數
}
```

**效果**：目標跳過下一個（或數個）回合

**戰術用途**：

- 暈眩高優先級敵人（法師、治療者）
- 打亂敵方回合順序
- 為我方爭取準備時間（buff、走位）

**技能範例**：

```rust
// 盾牌猛擊
Skill {
    tags: [Active, Melee, Physical],
    range: (1, 1),
    cost: 8,
    accuracy: Some(70),
    effects: vec![
        Effect::Hp { value: -5, ... },
        Effect::Stun { duration: 1, ... },  // 暈眩 1 回合
    ],
}
```

---

## 二、位移類（Positioning）- 配合地形戰術

### Pull（拉取）⭐ 優先實作

```rust
Pull {
    target_type: TargetType,
    shape: Shape,
    distance: usize,  // 拉近距離
}
```

**效果**：將目標拉向施術者方向

**戰術用途**：

- **配合 Shove 組合技**：Pull 到懸崖邊 → Shove 推下去
- 破壞敵方陣型（拉出後排）
- 救援隊友（拉離危險區域）

**與 Shove 的關係**：

- `Shove`: 推離施術者
- `Pull`: 拉近施術者
- 方向相反，戰術互補

**組合技範例**：

```
佈局：
  . . . E . .    E = 敵人（Boss）
  . . . . C X    C = Cliff, X = Pit
  . M W . . .    M = 法師, W = 戰士

步驟：
  1. 法師施放「奧術拉取」(Pull distance: 3)
     → Boss 從遠處被拉到 C（懸崖邊）
  2. 戰士移動到 Boss 旁邊
  3. 戰士使用「盾擊」(Shove distance: 2)
     → Boss 被推入 X（Pit）
  4. Boss 墜落身亡

結果：
  - 團隊協同（法師 + 戰士）
  - 地形利用（懸崖 + 深淵）
  - 即使高 HP Boss 也能速殺
```

---

### Swap（位置交換）⭐ 優先實作

```rust
Swap {
    target_type: TargetType,
    shape: Shape,
}
```

**效果**：施術者與目標交換位置

**戰術用途**：

- 救援被包圍的隊友（坦克與脆皮交換）
- 突破陣型（與敵方後排交換，直接攻擊）
- 位置調整（靈活應對戰場變化）

**範例**：

```
救援戰術：
  E E E     E = 敵人
  E A E     A = 脆皮射手（被包圍）
  . T .     T = 坦克

  1. 坦克使用「位置交換」與射手交換
  2. 射手脫困，坦克承受傷害
  3. 坦克有高 HP/Block，能撐住
  → 保護隊友的戰術選擇

突襲戰術：
  E E E M   M = 敵方法師（後排）
  E E E .
  . . W .   W = 戰士

  1. 戰士使用「突襲交換」與敵方法師交換
  2. 戰士瞬間出現在敵後排
  3. 直接攻擊法師
  → 打亂敵方陣型
```

---

### Blink（閃現）

```rust
Blink {
    target_type: TargetType,  // 可以是 Any（地面）
    shape: Shape,
    max_range: usize,
}
```

**效果**：傳送到目標位置

**戰術用途**：

- 跨越障礙物（牆壁、深水）
- 快速脫離包圍
- 突襲敵方後排

**與移動的區別**：

- 移動：受地形移動成本影響，需要路徑
- 閃現：無視地形，直接傳送（但可能需要視線）

---

### Charge（衝鋒）

```rust
Charge {
    target_type: TargetType,
    shape: Shape,
    damage: i32,
    max_range: usize,
}
```

**效果**：快速移動到目標旁邊並造成傷害

**戰術用途**：

- 破陣技能（衝入敵陣）
- 追擊殘血敵人
- 節省移動力（移動 + 攻擊一次完成）

---

## 三、團隊協同類（Team Synergy）- 實現角色合作

### Mark（標記）⭐ 優先實作

```rust
Mark {
    target_type: TargetType,
    shape: Shape,
    duration: i32,
    damage_bonus: i32,  // 額外傷害加成
}
```

**效果**：目標被標記，所有隊友攻擊時獲得額外傷害

**戰術用途**：

- **實現「不同角色使用不同技能合作達成戰術」**
- 偵查兵標記 → 輸出職業集火
- 針對高價值目標（Boss、治療者）
- 團隊傷害最大化

**範例**：

```rust
// 遊俠技能：獵人印記
Skill {
    tags: [Active, Ranged, Beneficial],
    range: (1, 6),
    cost: 5,
    accuracy: Some(95),  // 高命中率
    effects: vec![
        Effect::Mark {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            duration: 3,
            damage_bonus: 8,  // 隊友攻擊 +8 傷害
        }
    ],
}
```

**協同戰術**：

```
步驟：
  1. 遊俠使用「獵人印記」標記 Boss
  2. 戰士攻擊：10 傷害 + 8 = 18 傷害
  3. 法師火球：15 傷害 + 8 = 23 傷害
  → 團隊 DPS 大幅提升！
```

---

### Guard（護衛）

```rust
Guard {
    target_type: TargetType,  // 只能是 Ally
    shape: Shape,
    duration: i32,
}
```

**效果**：施術者代替目標承受傷害

**戰術用途**：

- 坦克職責明確化
- 保護脆皮輸出/治療
- 風險承擔（犧牲坦克 HP 換取隊友安全）

**實作細節**：

- 受到護衛的單位被攻擊時，傷害轉移到護衛者
- 護衛者可能會因此陣亡
- 需要計算護衛範圍（同一格？相鄰格？）

---

## 四、資源操作類（Resource Manipulation）- 配合資源管理

### Drain（汲取）

```rust
Drain {
    target_type: TargetType,
    shape: Shape,
    resource_type: ResourceType,  // Hp 或 Mp
    value: i32,
}

pub enum ResourceType {
    Hp,
    Mp,
}
```

**效果**：吸收敵人 HP/MP 轉為己用

**戰術用途**：

- **配合「MP 是一天的限制」設計**
- 連續戰鬥時補充 MP
- 吸血維持戰鬥力
- 針對性削弱敵方法師

**範例**：

```rust
// 吸血鬼技能：生命汲取
Skill {
    tags: [Active, Melee, Magical],
    range: (1, 1),
    cost: 5,
    accuracy: Some(75),
    effects: vec![
        Effect::Drain {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            resource_type: ResourceType::Hp,
            value: 12,  // 吸取 12 HP
        }
    ],
}
```

**資源管理戰術**：

```
情境：連續 3 場戰鬥，法師 MP 不足

戰術：
  1. 法師裝備「魔力汲取」技能
  2. 在雜兵戰使用 Drain(Mp) 吸取敵人 MP
  3. 補充自己的 MP 池
  4. 為 Boss 戰保留資源
  → 跨戰鬥的資源管理策略
```

---

### Steal（偷取）✨ 需要新增

```rust
Steal {
    target_type: TargetType,
    shape: Shape,
    resource_type: ResourceType,
    value: i32,
}
```

**效果**：偷取敵人資源（不轉為己用，純削弱）

**戰術用途**：

- 干擾敵方法師
- 阻止 Boss 放大招
- 針對性削弱

**與 Drain 的區別**：

- `Drain`: 吸取並恢復自己
- `Steal`: 只削弱敵人，不恢復自己（效果更強）

---

## 五、防禦與反制類（Defense & Counter）- 應對高風險情況

### Shield（護盾）

```rust
Shield {
    target_type: TargetType,
    shape: Shape,
    duration: i32,
    absorb_amount: i32,  // 可吸收的傷害量
}
```

**效果**：吸收一定傷害後消失

**戰術用途**：

- 臨時防禦（抵擋已知的強力攻擊）
- 保護脆皮單位
- 配合「完美信息」（知道敵人下一步行動）

**與 Block 的區別**：

- `Block`: 被動防禦，基於機率
- `Shield`: 主動防禦，確定吸收傷害

**實作細節**：

```rust
// 護盾機制
if unit.has_shield {
    let remaining_damage = damage - shield.absorb_amount;
    if remaining_damage > 0 {
        shield.absorb_amount = 0;  // 護盾破碎
        unit.hp -= remaining_damage;
    } else {
        shield.absorb_amount -= damage;
    }
} else {
    // 正常傷害計算
}
```

---

### Counter（反擊）

```rust
Counter {
    target_type: TargetType,
    shape: Shape,
    duration: i32,
    damage: i32,
}
```

**效果**：受到攻擊時自動反擊

**戰術用途**：

- 懲罰近戰敵人
- 將防禦轉化為進攻
- 配合 Guard（護衛）保護隊友時反擊攻擊者

**範例**：

```
戰術：
  1. 坦克使用「反擊姿態」buff
  2. 坦克使用「護衛」保護法師
  3. 敵人攻擊法師（傷害轉移給坦克）
  4. 坦克自動反擊造成傷害
  → 保護隊友同時反擊
```

---

### Reflect（反射）

```rust
Reflect {
    target_type: TargetType,
    shape: Shape,
    duration: i32,
}
```

**效果**：反射下一次攻擊給攻擊者

**戰術用途**：

- 高風險高報酬（時機判斷）
- 針對性克制強力單體技能
- **實現「以技巧克服隨機性」**

**範例**：

```
反射戰術：
  1. 觀察到敵方 Boss 準備放大招（40 傷害）
  2. 法師使用「魔法反射」
  3. Boss 施放大招
  4. 傷害反彈給 Boss 自己（40 傷害）
  → 以技巧克服強敵
```

---

## 六、狀態與協同類（Status & Synergy）- 未來擴充

以下為**暫緩實作**的複雜協同系統，記錄於此供未來參考：

### 元素協同系統

```rust
// 潮濕狀態
Wet {
    target_type: TargetType,
    shape: Shape,
    duration: i32,
}

// 油污狀態
Oiled {
    target_type: TargetType,
    shape: Shape,
    duration: i32,
}

// 冰凍狀態
Frozen {
    target_type: TargetType,
    shape: Shape,
    duration: i32,
}
```

**協同範例**（類似 Divinity: Original Sin 2）：

```
1. 法師 A 使用「造水術」→ 敵人獲得 Wet 狀態
2. 法師 B 使用「冰凍術」→ Wet + 冰凍 = Frozen（無法行動）
3. 或者使用「閃電術」→ Wet + 電擊 = 連鎖傷害

1. 盜賊使用「油瓶」→ 敵人獲得 Oiled 狀態
2. 法師使用「火球術」→ Oiled + 火 = 燃燒（Burn + 額外傷害）
```

**暫緩原因**：

- 需要複雜的狀態互動邏輯
- 需要大量技能配合才有意義
- 實作成本高，先做基礎系統

---

## 實作優先級總結

### 可用既有 Effect 達成（無需新增）✅

- **Root（定身）** → `MovePoints { value: -999 }`
- **Haste（加速）** → `Initiative { value: +20 }`
- **Slow（緩速）** → `MovePoints { value: -10 }` 或 `Initiative { value: -10 }`
- **Blind（致盲）** → `Accuracy { value: -30 }`
- **Inspire（鼓舞）** → 組合多個 Effect（Accuracy, Evasion, Initiative 等）
- **Blessed/Cursed（祝福/詛咒）** → 組合多個 Effect
- **Share MP（分享魔力）** → `cost: 20` + `Mp { value: 20, target_type: Ally }`
- **Share HP（分享生命）** → `Hp { value: -20, target_type: Caster }` + `Hp { value: 20, target_type: Ally }`

---

### 第一優先（立即實作）⭐

**理由**：當前需求或與現有系統高度協同

- **Pull**（拉取）

  - 配合 Shove 實現「拉 + 推」組合技
  - 實作難度：中等（類似 Shove 反向）

- **Swap**（位置交換）

  - 救援隊友、突破陣型
  - 實作難度：簡單（直接交換座標）

- **Mark**（標記）

  - 實現角色協同
  - 實作難度：中等（需要傷害計算時檢查標記）

- **Grab**（抓取）⭐ **新增**
  - 參考：D&D 5e Grapple
  - 效果：限制目標移動，可嘗試掙脫（力量對抗）
  - 戰術：抓住刺客保護後排、阻止 Boss 逃跑、配合集火
  - 實作難度：中等（需要掙脫判定）

---

### 第二優先（豐富控場選擇）

**理由**：基礎控場能力，擴展戰術深度

- **Stun**（暈眩）

  - 基礎控場
  - 實作難度：中等（需要跳過回合邏輯）

- **Spike Growth**（尖刺生長）⭐ **新增**

  - 參考：D&D 5e Spike Growth
  - 效果：創造尖刺地形（移動受傷 + 移動消耗增加）
  - 戰術：阻擋通道、配合 Shove/Pull 推入尖刺區、保護後排
  - 實作難度：中高（需要地形創造系統）

- **Grease**（油膩術）⭐ **新增**
  - 參考：D&D 5e Grease
  - 效果：創造油膩地形（移動進入判定，失敗摔倒 Prone）
  - 元素反應：Grease + 火焰 → 燃燒區域（爆炸傷害）
  - 戰術：控制區域、配合火系法師、讓敵人摔倒
  - 實作難度：中高（需要地形創造 + 元素反應）

---

### 第三優先（資源管理深度）

**理由**：配合「MP 是一天的限制」設計

- **Drain**（汲取）
  - MP 補充機制
  - 實作難度：中等（需要資源轉移邏輯）
- **Shield**（護盾）
  - 防禦選擇
  - 實作難度：中等（需要傷害計算邏輯）

---

### 第四優先（進階戰術）

- **Guard**（護衛）- 傷害轉移
- **Blink**（閃現）- 位置靈活性
- **Counter**（反擊）- 反制近戰
- **Charge**（衝鋒）- 需要路徑計算 + 傷害

---

### 暫緩實作

**理由**：複雜度高,需要更多系統支援

- **Reflect**（反射）- 需要複雜的傷害來源追蹤
- **Steal**（偷取）- 資源轉移邏輯，可等 Drain 實作後再做
- **複雜元素協同**（Wet, Oiled, Frozen 互動）- 需要大量技能配合

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

  - 對立兩側包圍敵人：+20 accuracy
  - 戰術：兩個單位配合包圍

- **光線（Light）**
  - Bright: 正常
  - Dim: -10 accuracy
  - Darkness: -30 accuracy
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

## 實作階段規劃

### 第一階段：位置戰術強化

- 視線系統（LOS）
- 掩體系統
- 夾擊

**目標**：讓位置選擇變得重要

---

### 第二階段：控場技能

- Grab（抓取）
- Pull（拉取）
- Swap（位置交換）
- Stun（暈眩）

**目標**：提供多樣化控場選擇

---

### 第三階段：地形操控

- Spike Growth（尖刺生長）- 地形創造系統
- Grease（油膩術）- 地形創造 + Prone 狀態
- 元素反應（Grease + 火焰）

**目標**：實現「製造地形」戰術

---

### 第四階段：進階系統

- 反應動作（Reactions）
- 戰爭迷霧（Fog of War）

**目標**：增加戰術深度和沉浸感

---

## 技能設計範例（完整）

### Pull + Shove 組合技

```rust
// 法師技能：奧術拉取
pub const SKILL_ARCANE_PULL: &str = r#"
tags = ["active", "ranged", "magical", "single"]
range = [1, 5]
cost = 10
accuracy = 80

[[effects]]
type = "pull"
target_type = "enemy"
shape = "point"
distance = 3
"#;

// 戰士技能：盾擊
pub const SKILL_SHIELD_BASH: &str = r#"
tags = ["active", "melee", "physical", "single"]
range = [1, 1]
cost = 5
accuracy = 90

[[effects]]
type = "shove"
target_type = "enemy"
shape = "point"
distance = 2
"#;
```

**組合戰術**：法師 Pull(3) → 戰士 Shove(2) → 推下懸崖

---

### Mark + 集火戰術

```rust
// 遊俠技能：獵人印記
pub const SKILL_HUNTERS_MARK: &str = r#"
tags = ["active", "ranged", "beneficial", "single"]
range = [1, 6]
cost = 5
accuracy = 95

[[effects]]
type = "mark"
target_type = "enemy"
shape = "point"
duration = 3
damage_bonus = 8
"#;

// 戰士技能：重擊
pub const SKILL_HEAVY_STRIKE: &str = r#"
tags = ["active", "melee", "physical", "single", "attack"]
range = [1, 1]
cost = 5
accuracy = 70

[[effects]]
type = "hp"
target_type = "enemy"
shape = "point"
value = -15
"#;
```

**組合戰術**：遊俠 Mark → 戰士 Heavy Strike（15 + 8 = 23 傷害）

---

### Swap 救援戰術

```rust
// 坦克技能：英勇交換
pub const SKILL_HEROIC_SWAP: &str = r#"
tags = ["active", "ranged", "beneficial", "single"]
range = [1, 4]
cost = 8
accuracy = 100  # 必中（救援技能）

[[effects]]
type = "swap"
target_type = "ally"
shape = "point"
"#;
```

**戰術**：脆皮被包圍 → 坦克 Swap 救援 → 坦克承受傷害

---

## 實作檢查清單

### Pull 效果實作

- [ ] 在 `Effect` enum 中加入 `Pull` 變體
- [ ] 實作 `apply_pull_effect()` 函數
  - [ ] 計算拉取方向（與 Shove 相反）
  - [ ] 計算目標位置
  - [ ] 檢查路徑上的障礙物
  - [ ] 更新單位位置
- [ ] 測試：Pull 到懸崖邊 + Shove 組合技
- [ ] 測試：Pull 被牆壁阻擋

### Swap 效果實作

- [ ] 在 `Effect` enum 中加入 `Swap` 變體
- [ ] 實作 `apply_swap_effect()` 函數
  - [ ] 檢查目標位置是否可通行
  - [ ] 交換兩個單位的座標
  - [ ] 更新 Board 的位置映射
- [ ] 測試：Swap 救援隊友
- [ ] 測試：Swap 敵人（突襲）

### Mark 效果實作

- [ ] 在 `Effect` enum 中加入 `Mark` 變體
- [ ] 在 `Unit` 中加入標記狀態
- [ ] 修改傷害計算邏輯，檢查目標是否被標記
- [ ] 實作標記持續時間衰減
- [ ] 測試：標記 + 多次攻擊的傷害加成

---

## 參考資料

### 設計理念來源

- **Into the Breach**：位置戰術、推擊機制
- **Divinity: Original Sin 2**：元素協同、地形互動
- **XCOM 2**：風險評估、穩健 vs 賭博技能
- **Darkest Dungeon**：標記系統、位置系統
- **火焰之紋章**：地形影響、角色協同

### 相關文檔

- `README-設計機制.md`：核心設計理念與願景
- `CLAUDE.md`：專案架構與實作規範
- `core/skills-lib/src/lib.rs`：當前技能系統實作

---

## 附錄：已實作的效果

### 立即生效類

- `Hp` - HP 變化（傷害/治療）
- `Mp` - MP 變化（消耗/恢復）
- `Shove` - 推擊

### 持續效果類

- `MaxHp` - 最大 HP 調整
- `MaxMp` - 最大 MP 調整
- `Initiative` - 先攻調整
- `Accuracy` - 命中調整
- `Evasion` - 閃避調整
- `Block` - 格擋調整
- `BlockReduction` - 格擋減傷調整
- `MovePoints` - 移動力調整
- `Burn` - 燃燒（DoT）
- `HitAndRun` - 打了就跑

---

## 暫存區域（設計草稿）

### Reaction 系統設計

#### 核心機制

**次數管理**（類似 move_points）：

- `Unit.max_reactions_per_turn: usize` - 從技能計算
- `Unit.reactions_used_this_turn: usize` - 當前已使用次數
- 每回合開始重置為 0

**Effect 類型**：

- `Effect::Reaction` - 提供 Reaction 能力（觸發條件、執行技能）
- `Effect::MaxReactions` - 提供 Reaction 次數上限

**觸發條件（ReactionTrigger）**：

- `OnMove` - 敵人離開相鄰格時（借機攻擊）
- `OnAttacked` - 自己被攻擊命中時（反擊）

**技能來源（ReactionSkillSource）**：

```rust
pub enum ReactionSkillSource {
    SkillId(String),    // 使用特定技能 ID
    Tag(Tag),           // 使用有此 Tag 的技能
}
```

- `SkillId` - 明確指定技能（如特殊的反擊技能）
- `Tag` - 按 Tag 查找技能（如 `BasicAttack`）
  - 如果找到**多個**符合的技能 → UI 讓玩家選擇要用哪個
  - 如果只有一個 → 直接使用

#### UI 互動設計 ⭐ **核心原則**

**所有 Reaction 都必須由玩家確認執行**：

- 無論有 1 個還是多個 Reaction，都要跳出提示
- 遊戲暫停，顯示可用的 Reaction 選項
- 玩家可選擇：
  - ✅ 執行（消耗 Reaction 次數）
  - ❌ 跳過（保留次數給其他時機）
- 多個 Reaction 時，依照玩家點選順序執行

**設計理由**：

- ✅ **玩家主導**：戰術決策權在玩家手中
- ✅ **資源管理**：玩家自行判斷何時使用寶貴的 Reaction 機會
- ✅ **透明度**：清楚知道觸發時機和可用選項
- ✅ **避免意外**：不會因為 AI 自動執行而浪費 Reaction
- ✅ **戰術深度**：有多個技能可選時，玩家根據情況選最適合的
  - 敵人血少 → 選高傷害技能
  - 敵人血多 → 選帶 debuff 的技能

**範例場景 1：單一技能**

```
敵人移動離開相鄰格 → 遊戲暫停
UI 提示：「戰士 A1 可以使用借機攻擊 (斬擊)，是否執行？」
玩家選擇：[執行] / [跳過]
```

**範例場景 2：多個技能可選** ⭐

```
觸發 Reaction (skill_source = Tag::BasicAttack)
→ 找到 3 個符合的技能 → UI 彈出選擇：

┌─────────────────────────────┐
│ 戰士可以使用借機攻擊，選擇：  │
│ [ ] 斬擊 (10 傷害)           │
│ [ ] 刺擊 (8 傷害 + 流血)     │
│ [ ] 橫掃 (6 傷害 AOE)        │
│ [ ] 跳過                     │
└─────────────────────────────┘

玩家根據情況選擇最適合的技能
```

#### 移動與 Reaction 整合設計 ⭐ **核心架構**

**問題**：

- 移動過程中觸發的 reactions（如降低 MovePoints）需要立即執行並影響後續移動
- 目前 `move_unit_along_path` 只收集 reactions，移動完成後才回傳，無法即時生效

**UI 層流程**：

- 遍歷路徑，對每一步：
  1. 檢查 reactions → 玩家選擇是否執行 → 執行 `execute_reaction`
  2. 執行移動 → 檢查是否能繼續 → 若否則停止

**關鍵特點**：

- ✅ Reactions 立即執行，影響後續移動
- ✅ 玩家完全控制（符合 UI 互動設計）
- ✅ 職責分離（chess-lib 不依賴 UI）
- ✅ 單位死亡或移動力不足時停止移動

---

#### 實作待辦

**核心資料結構與邏輯**（已完成 ✅）

- [x] skills-lib: 加入 ReactionTrigger enum (OnMove, OnAttacked)
- [x] skills-lib: 加入 TriggeredSkill enum (SkillId, Tag) - 重命名自 ReactionSkillSource
- [x] skills-lib: 加入 Effect::Reaction, Effect::MaxReactions
- [x] Unit: 加入 max_reactions_per_turn, reactions_used_this_turn (使用 ReactionCount 類型別名)
- [x] unit.rs: 實作 skills_to_max_reactions() 計算函數（含測試）
- [x] action/reaction.rs: 實作 find_reaction_skills() 查找邏輯（含測試）
  - [x] SkillId 查找：檢查單位是否擁有該技能
  - [x] Tag 查找：找到所有符合 tag 的技能
  - [x] 錯誤處理：沒有找到技能時返回 SkillNotFound 錯誤
- [x] action/reaction.rs: 實作 can_trigger_reaction() 次數檢查（含測試）

**整合與執行邏輯**（部分完成 ⏳）

- [x] battle.rs: 回合開始重置 reactions_used_this_turn = 0 (含測試)
- [x] action/reaction.rs: 實作 consume_reaction() 消耗次數（含測試）
  - [x] 檢查是否還有可用次數
  - [x] 消耗一次 reaction 次數
  - [x] 錯誤處理：次數用盡時返回錯誤
- [x] action/reaction.rs: 實作 execute_reaction() 執行 reaction 技能（含測試）
- [ ] action/movement.rs: 實作逐步移動 API ⭐
  - [ ] `check_next_step_reactions()` - 檢查單步觸發的 reactions
  - [ ] `execute_one_step_move()` - 執行單步移動
  - [ ] `can_continue_moving()` - 檢查是否能繼續移動
  - [ ] 重構 `move_unit_along_path` 供 AI 使用（自動執行 reactions）
- [ ] action/skill.rs: 整合 OnAttacked 觸發點
  - [ ] 技能命中目標時檢查 reactions
  - [ ] 回傳 `Vec<PendingReaction>` 供外層處理

**UI 與測試**（待完成 ❌）

- [ ] UI: Reaction 確認介面（單一技能）
- [ ] UI: Reaction 技能選擇介面（多個技能可選）
- [ ] 完整場景測試
  - [ ] 借機攻擊（OnMove）
  - [ ] 反擊（OnAttacked）
  - [ ] 次數限制（max_reactions_per_turn）
  - [ ] 多技能選擇

---

**最後更新**：2025-12-26
**文檔版本**：v1.2

**v1.2 更新內容**：

- ✅ Reaction 系統核心邏輯已完成（資料結構、計算函數、查找邏輯）
- ⏳ 待整合：觸發點、回合管理、UI 互動
