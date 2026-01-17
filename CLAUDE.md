# 通用規則

- 在 WSL: use `cargo.exe` instead of `cargo` for all Rust commands
- 使用 match 而不要 let else
- match arm 使用編譯器的 exhaustiveness checking 保護，避免未來忘記添加 match arm
- 撰寫計畫的時候不要添加程式碼，請保持精簡
- 如果多數 function caller 已經知道具體類型，不要只傳遞基本型別再反推
- 請假設每個程式碼檔案都很大
- 確保型別安全，有完整錯誤處理（錯誤訊息包含豐富上下文）
- 禁止 magic numbers/strings
- `use` 語句放在檔案頂部，不要放在 function 裡面
- 不確定時詢問使用者，不要自行決定

# 本專案

- 本專案是戰術回合制 RPG 遊戲（Rust）。
- 如果發現 core 底下的 function 與本檔案紀錄不合的時候，請更新本檔案  
  只列出公開函數（pub fn），不包含 struct/enum 定義。格式：`函數簽名` - 簡短說明

## 專案結構

- **core/chess-lib**: 核心遊戲邏輯（戰鬥系統、移動、技能施放、AI）
- **core/skills-lib**: 技能定義、效果類型
- **core/object-lib**: 物件類型定義
- **core/dialogs-lib**: 對話系統
- **editor**: egui GUI 編輯器（地圖、單位、技能編輯）

## function 集

### core/chess-lib

**棋盤操作** (`board.rs`)

- `Board::from_config(config, unit_templates, skills) -> Result<Board, Error>` - 從配置建立棋盤
- `Board::pos_to_unit(&self, pos: Pos) -> Option<UnitID>` - 位置 → 單位 ID
- `Board::unit_to_pos(&self, unit_id: UnitID) -> Option<Pos>` - 單位 ID→ 位置
- `Board::is_tile_passable(&self, pos: Pos) -> bool` - 檢查格子可通行性
- `Board::can_see_target(&self, (observer_id, from), to, skills) -> Result<bool, Error>` - 視線檢查
- `Board::get_light_level(&self, pos: Pos, skills) -> Result<LightLevel, Error>` - 取得光照等級
- `Board::get_tile(&self, pos: Pos) -> Option<&Tile>` - 取得格子
- `Board::get_tile_mut(&mut self, pos: Pos) -> Option<&mut Tile>` - 取得可變格子
- `Board::width(&self) -> usize` - 棋盤寬度
- `Board::height(&self) -> usize` - 棋盤高度
- `UnitMap::insert(&mut self, unit_id, pos)` - 插入單位
- `UnitMap::remove(&mut self, unit_id) -> Option<Pos>` - 移除單位並返回原位置
- `UnitMap::move_unit(&mut self, unit_id, from, to) -> Result<(), Error>` - 移動單位
- `UnitMap::get_unit(&self, pos) -> Option<UnitID>` - 查詢位置的單位
- `UnitMap::get_pos(&self, unit_id) -> Option<Pos>` - 查詢單位位置
- `ObjectMap::insert(&mut self, object)` - 插入物件
- `ObjectMap::remove(&mut self, object_id) -> Option<Object>` - 移除物件
- `ObjectMap::get(&self, object_id) -> Option<&Object>` - 取得物件
- `ObjectMap::get_objects_at(&self, pos) -> Vec<&Object>` - 查詢位置的所有物件
- `ObjectMap::decrease_object_duration(&mut self, object_id)` - 減少物件持續時間
- `ObjectMap::ignite_objects_at(&mut self, pos) -> usize` - 點燃位置上的可燃物件
- `ObjectMap::extinguish_objects_at(&mut self, pos) -> usize` - 熄滅位置上的物件
- `Object::is_passable(&self) -> bool` - 物件是否可通行
- `Object::blocks_sight_from(&self, from_pos, obj_pos) -> bool` - 是否阻擋視線
- `Object::light_level_at(&self, distance) -> LightLevel` - 物件在距離的光照等級

**ObjectMap 封裝規則**:

- 禁止 `ObjectMap::get_mut()` - 避免外部修改 `affected_positions` 導致索引不同步
- 若需修改物件狀態，在 `ObjectMap` 上新增專用方法（如 `ignite_object`、`extinguish_object`）

**戰鬥管理** (`battle.rs`)

- `Battle::new(turn_order: Vec<TurnEntity>) -> Battle` - 建立戰鬥
- `Battle::get_current_entity(&self) -> Option<&TurnEntity>` - 取得當前回合實體
- `Battle::remove_entity_from_turn_order(&mut self, entity)` - 從回合順序移除實體
- `Battle::insert_object_before_next_turn(&mut self, object_id)` - 在下一回合前插入物件
- `Battle::next_turn(&mut self, board, skill_selection)` - 推進回合
- `remove_expired_status_effects(unit: &mut Unit)` - 移除過期狀態效果
- `process_status_effects_at_turn_end(unit, statistics) -> bool` - 處理回合結束狀態並記錄燃燒傷害
- `remove_object_if_expired(board, object_id) -> bool` - 移除過期物件
- `check_and_remove_dead_unit(board, battle, unit_id, cause) -> Option<UnitID>` - 檢查並移除死亡單位

**單位系統** (`unit.rs`)

- `Unit::from_template(marker, template, skills) -> Result<Unit, Error>` - 從模板建立單位
- `Unit::recalc_from_skills(&mut self, skills) -> Result<(), Error>` - 從技能重新計算屬性
- `calc_initiative(rng, skill_ids, skills) -> Result<i32, Error>` - 計算先攻
- `skills_to_max_hp(skill_ids, skills) -> Result<i32, Error>` - 技能 → 最大 HP
- `skills_to_max_mp(skill_ids, skills) -> Result<i32, Error>` - 技能 → 最大 MP
- `skills_to_move_points(skill_ids, skills) -> Result<MovementCost, Error>` - 技能 → 移動力
- `skills_to_max_reactions(skill_ids, skills) -> Result<ReactionCount, Error>` - 技能 → 最大反應次數
- `skills_to_accuracy(skill_ids, skills) -> Result<i32, Error>` - 技能 → 命中
- `skills_to_evasion(skill_ids, skills) -> Result<i32, Error>` - 技能 → 閃避
- `skills_to_block(skill_ids, skills) -> Result<i32, Error>` - 技能 → 格擋
- `skills_to_block_reduction(skill_ids, skills) -> Result<i32, Error>` - 技能 → 破甲
- `skills_to_flanking(skill_ids, skills) -> Result<i32, Error>` - 技能 → 側翼加成
- `skills_to_hit_and_run(skill_ids, skills) -> Result<bool, Error>` - 技能 → 打帶跑
- `skills_to_potency(skill_ids, skills, tag) -> Result<i32, Error>` - 技能 → 威力
- `skills_to_resistance(skill_ids, skills, save_type) -> Result<i32, Error>` - 技能 → 抗性
- `skills_to_sense(skill_ids, skills, distance) -> Result<bool, Error>` - 技能 → 感知能力
- `effects_to_light_level(effects, distance) -> LightLevel` - 效果 → 光照等級
- `effects_to_sense(effects, distance) -> bool` - 效果 → 感知能力

**技能施放** (`action/skill/mod.rs`, `action/skill/casting.rs`)

- `SkillSelection::select_skill(&mut self, skill_id: Option<SkillID>)` - 選擇技能
- `SkillSelection::execute_action(&self, board, battle, skills, caster, target) -> Result<Vec<String>, Error>` - 執行技能
- `SkillSelection::skill_affect_area(&self, board, skills, caster_pos, to) -> Vec<Pos>` - 技能影響範圍
- `skill_casting_area(board, active_unit_pos, range, skills) -> Vec<Pos>` - 技能施放範圍
- `is_able_to_act(unit) -> Result<(), Error>` - 檢查是否能行動
- `consume_action(unit) -> Result<(), Error>` - 消耗行動力
- `calc_shape_area(board, shape, from, to) -> Vec<Pos>` - 計算形狀範圍
- `is_targeting_valid_target(...) -> Result<bool, Error>` - 檢查目標是否有效
- `is_in_skill_range_manhattan(range, from, to) -> bool` - 檢查是否在技能範圍內
- `calc_direction_manhattan(from, to) -> (isize, isize)` - 計算曼哈頓方向向量
- `clamp_pi(rad: f64) -> f64` - 限制角度在 [-π, π]

**命中與豁免** (`action/skill/hit.rs`, `action/skill/save.rs`)

- `calc_hit_result(board, battle, caster, skills, skill, &[Pos], accuracy) -> Result<Vec<String>, Error>` - 計算命中結果
- `calc_save_result(board, skills, caster_id, target_id, skill, effect) -> Result<SaveResult, Error>` - 計算豁免結果

**移動系統** (`action/movement.rs`)

- `get_adjacent_positions(pos: Pos) -> Vec<Pos>` - 取得相鄰位置
- `movable_area(board, from, skills_map) -> HashMap<Pos, (MovementCost, Pos)>` - 計算可移動範圍
- `reconstruct_path(map, from, to) -> Result<Vec<Pos>, Error>` - 重建路徑
- `move_unit_along_path(board, path, reacted, skills_map) -> Result<MoveResult, Error>` - 沿路徑移動
- `movement_tile_color(board, movable, active_unit_id, path, pos) -> Result<RGBA, Error>` - 移動格子顏色

**反應系統** (`action/reaction.rs`)

- `find_reaction_skills(triggered_skill, unit_skills, all_skills) -> Result<Vec<SkillID>, Error>` - 尋找反應技能
- `is_able_to_react(unit) -> Result<(), Error>` - 檢查是否能反應
- `consume_reaction(unit) -> Result<(), Error>` - 消耗反應次數
- `check_unit_reactions(unit, trigger_type, all_skills) -> Result<Vec<ReactionInfo>, Error>` - 檢查單位反應
- `check_reactions(board, battle, caster, target, skills) -> Result<Vec<PendingReaction>, Error>` - 檢查技能反應
- `check_move_reactions(board, (unit, pos), skills_map) -> Result<Vec<PendingReaction>, Error>` - 檢查移動反應
- `execute_reaction(board, battle, reactor, skills, reaction_skill, target) -> Result<Vec<String>, Error>` - 執行反應

**AI 系統** (`ai.rs`)

- `decide_action(board, skills, config, unit_id) -> Result<ScoredAction, Error>` - AI 決定行動
- `score_actions(board, skills, config, unit_id) -> Result<Vec<ScoredAction>, Error>` - AI 評估所有行動

**路徑尋找** (`action/algo.rs`)

- `dijkstra<T: PathfindingBoard>(graph: &T, start: Pos) -> HashMap<Pos, (MovementCost, Pos)>` - Dijkstra 最短路徑
- `bresenham_line(from, to, len, is_valid) -> Vec<Pos>` - Bresenham 直線

**輔助函數** (`terrain.rs`, `lib.rs`)

- `manhattan_distance(a: Pos, b: Pos) -> usize` - 曼哈頓距離
- `movement_cost(terrain: Terrain) -> MovementCost` - 地形移動成本

### core/skills-lib

**方法**

- `Effect::target_type(&self) -> &TargetType` - 效果目標類型
- `Effect::is_targeting_unit(&self) -> bool` - 是否針對單位
- `Effect::shape(&self) -> &Shape` - 效果形狀
- `Effect::save_type(&self) -> Option<&SaveType>` - 豁免類型
- `Effect::duration(&self) -> i32` - 持續時間
- `Effect::decrease_duration(&mut self)` - 減少持續時間
- `Skill::default() -> Self` - 預設技能

### core/dialogs-lib

**方法**

- `Node::pos(&self) -> Pos` - 節點位置
- `Node::set_pos(&mut self, p: Pos)` - 設定節點位置

### core/object-lib

**方法**

- `ObjectType::is_ignitable(&self) -> bool` - 檢查物件是否可被點燃
- `ObjectType::is_extinguishable(&self) -> bool` - 檢查物件是否可被熄滅
- `ObjectType::try_ignite(&mut self) -> bool` - 嘗試點燃物件
- `ObjectType::try_extinguish(&mut self) -> bool` - 嘗試熄滅物件

## 基本指令

```bash
# 測試
cargo.exe test
```

## 程式碼規範

**語言**: 繁體中文（程式碼、註解、文件）

- **禁止向後相容**
  - 直接刪除未使用的程式碼，不要保留
  - 如果程式碼未使用，完全刪除它

**測試**:

- 不替以下撰寫測試: `ai.rs`、`editor` crate、inner functions
- 只有在副作用難以測試時才修改程式碼邏輯
