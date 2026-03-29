# 實作 skill_tab.rs 技能編輯表單

## 目標與範圍

為 `editor/src/tabs/skill_tab.rs` 的 `render_form` 實作完整的技能編輯表單，涵蓋 `SkillType` 的所有三個 variant（Active / Reaction / Passive）及其所有巢狀型別。

## 設計決策

1. **Variant 切換**：切換 Active / Reaction / Passive 時清空所有欄位，使用 `Default` 重建
2. **簡單 Vec**（`Vec<SkillTag>`、`Vec<EndCondition>` 等）：平鋪顯示，每項旁邊放刪除按鈕，底部放新增按鈕，不支援排序
3. **複雜 Vec**（`Vec<EffectNode>`、`Vec<ContinuousEffect>`）：用 `CollapsingHeader` 包每個項目，附拖曳 handle + 刪除按鈕，底部用 variant 按鈕新增（如 Area / Branch / Leaf 三個按鈕）
4. **Enum 下拉**：所有簡單 enum（Attribute、DcType、CasterOrTarget、TargetFilter 等）都有 `EnumIter`，統一用 `ComboBox` 迭代產生下拉選項

## 型別結構概覽

```
SkillType
├── Active  { name, tags: Vec<SkillTag>, cost, target: Target, effects: Vec<EffectNode> }
├── Reaction { name, tags: Vec<SkillTag>, cost, triggering_unit: TriggeringSource, effects: Vec<EffectNode> }
└── Passive { name, tags: Vec<SkillTag>, effects: Vec<ContinuousEffect> }

EffectNode（遞迴）
├── Area { area: Area, filter: TargetFilter, nodes: Vec<EffectNode> }
├── Branch { who: CasterOrTarget, condition: EffectCondition, on_success: Vec<EffectNode>, on_failure: Vec<EffectNode> }
└── Leaf { who: CasterOrTarget, effect: Effect }

Effect
├── HpEffect { scaling: Scaling }
├── MpEffect { value }
├── ApplyBuff { buff: BuffType }
├── ForcedMove { direction: MoveDirection, distance }
├── AllowRemainingMovement
├── SwapPosition
├── Trample { distance, scaling: Scaling }
└── SpawnObject { object_type, duration, contact_effects: Vec<EffectNode> }

BuffType { stackable, while_active: Vec<ContinuousEffect>, per_turn_effects: Vec<EffectNode>, end_conditions: Vec<EndCondition> }
```

## 實作步驟

### 步驟 1：enum 下拉輔助函數

建立通用的 enum ComboBox 渲染函數，利用 `EnumIter + Default + Debug + Clone + PartialEq`：
- `fn enum_combo_box<E>(ui, label, value: &mut E, id_salt)` — 適用於所有簡單 enum

### 步驟 2：SkillType variant 選擇 + 共用欄位

- 三個按鈕或 radio 切換 Active / Reaction / Passive，切換時重建為 `Default`
- 渲染共用欄位：name（text edit）、tags（簡單 Vec）、cost（DragValue，Passive 沒有）

### 步驟 3：Target 結構表單（Active 專用）

- range: (Coord, Coord) — 兩個 DragValue
- selection: TargetSelection — enum 下拉
- selectable_filter: TargetFilter — enum 下拉
- count: usize — DragValue
- allow_same_target: bool — checkbox
- area: Area — enum 下拉 + 根據 variant 顯示額外欄位（radius / length）

### 步驟 4：TriggeringSource 結構表單（Reaction 專用）

- source_range: (Coord, Coord) — 兩個 DragValue
- source_filter: TargetFilter — enum 下拉
- trigger: ReactionTrigger — enum 下拉

### 步驟 5：ContinuousEffect 列表（Passive 專用 + BuffType 內共用）

- CollapsingHeader + 拖曳 handle + 刪除按鈕
- 底部用 variant 按鈕新增（AttributeFlat / AttributeScaling / NearbyAllyScaling / HpRatioScaling / Perception / DamageToMp / EmitLight / Blinded）
- 每個 variant 展開對應欄位

### 步驟 6：EffectNode 列表（Active / Reaction 的 effects）

- CollapsingHeader + 拖曳 handle + 刪除按鈕
- 底部三個按鈕：Area / Branch / Leaf
- Area：area 下拉 + filter 下拉 + 遞迴 Vec<EffectNode>
- Branch：who 下拉 + condition（EffectCondition）+ on_success/on_failure 各自遞迴 Vec<EffectNode>
- Leaf：who 下拉 + Effect 表單

### 步驟 7：Effect 表單

- variant 下拉或按鈕選擇
- 各 variant 欄位：Scaling 結構、DragValue、BuffType 子表單、MoveDirection 下拉等

### 步驟 8：BuffType 子表單

- stackable: checkbox
- while_active: Vec<ContinuousEffect>（複用步驟 5）
- per_turn_effects: Vec<EffectNode>（複用步驟 6）
- end_conditions: Vec<EndCondition>（簡單 Vec）

### 步驟 9：EndCondition 列表

- variant 下拉 + 對應欄位（Duration 需要 u32 輸入等）
- 簡單 Vec 操作（新增/刪除，無排序）

### 步驟 10：EffectCondition 表單

- HitCheck：accuracy_bonus + crit_bonus（兩個 DragValue）
- DcCheck：dc_type 下拉 + dc_bonus DragValue

## 注意事項

- EffectNode 是遞迴結構，渲染函數需要接受深度參數或用 unique id_salt 避免 egui ID 衝突
- CollapsingHeader 的 id 必須唯一（用 index + 深度 + 路徑組合）
- `SkillType` 的 `name()` / `set_name()` 是方法不是欄位，render_form 中直接操作即可
- 所有 enum 都已 derive `EnumIter + Default`，可統一處理
- 拖曳排序可複用 `crate::utils::dnd::render_dnd_handle`
