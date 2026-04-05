# Core/Board 專案索引

本檔案包含 `core/board` crate 的專案結構和 function 集。
此檔案不設定 paths 限制，讓其他 crate（如 editor）也能參考。

## 編輯規則

### 專案結構

禁止列舉 enum variants 或 struct，只記錄檔案與職責。
職責描述必須是一句話，不帶任何實作細節（不提型別名、欄位名、演算法名）。

例子：

- ❌ 錯誤：component.rs - Position(x, y), Unit { name: String, hp: i32 }
- ❌ 錯誤：core_types.rs - occupant 與位置之間的雙向索引型別
- ✓ 正確：component.rs - ECS Component 定義
- ✓ 正確：core_types.rs - 遊戲核心資料型別定義

### Function 集簽名

保留完整簽名（pub fn、pub struct、pub trait、pub enum）。
不記錄 `impl Trait for Type`（trait 實現）。
移除實現細節、常數值。

例子：

- ❌ 錯誤：`pub fn reachable_positions(...) -> Result<HashMap<Position, ReachableInfo>>` - 使用 BFS 演算法，檢查地形花費，預算為 10
- ✓ 正確：`pub fn reachable_positions(...) -> Result<HashMap<Position, ReachableInfo>>` - 計算預算內可到達的所有位置

## 專案結構

```
core/board/
├── src/
│   ├── error.rs          - 錯誤型別定義
│   ├── loader_schema.rs  - 載入相關資料結構定義
│   ├── domain/           - 遊戲領域模型
│   │   ├── alias.rs      - 類型別名定義
│   │   ├── constants.rs  - 遊戲常數定義
│   │   └── core_types.rs - 遊戲核心資料型別定義
│   ├── ecs_types/        - ECS 型別定義
│   │   ├── components.rs - ECS Component 定義（只存資料，derive Component 和 Debug，禁止 impl）
│   │   └── resources.rs  - ECS World Resource 定義
│   ├── ecs_logic/        - ECS 操作函數（讀寫 World，整合 logic）
│   │   ├── mod.rs        - 模組宣告
│   │   ├── loader.rs     - 遊戲資料載入函數
│   │   ├── spawner.rs    - 關卡生成函數
│   │   ├── deployment.rs - 單位部署函數
│   │   ├── query.rs      - World 查詢函數
│   │   ├── movement.rs   - 單位移動 ECS 操作函數
│   │   ├── turn.rs       - 回合順序 ECS 操作函數
│   │   └── skill.rs      - 技能系統 ECS 操作函數
│   └── logic/            - 核心業務邏輯（純邏輯運算，不依賴 ECS Query）
│       ├── mod.rs        - 邏輯模組定義
│       ├── board.rs      - 棋盤驗證邏輯
│       ├── id_generator.rs - ID 產生邏輯
│       ├── movement.rs   - 移動邏輯
│       ├── turn_order.rs - 回合順序計算邏輯
│       ├── unit_attributes.rs - 單位屬性計算邏輯
│       ├── skill.rs      - 技能效果計算邏輯
│       ├── skill_check.rs - 技能命中與豁免判定邏輯
│       ├── skill_execute.rs - 技能效果執行邏輯
│       ├── skill_reaction.rs - 技能反應收集邏輯
│       └── debug.rs      - 調試工具函數
└── tests/
    ├── test.rs           - 集成測試入口
    ├── test_error.rs     - 錯誤型別測試
    ├── helpers/          - 測試輔助工具
    │   ├── mod.rs        - 測試輔助模組
    │   └── level_builder.rs - 測試用 ASCII 關卡建構工具
    ├── ecs_logic/        - ECS 操作與測試
    │   ├── mod.rs        - 模組宣告
    │   ├── constants.rs  - 測試常數定義
    │   ├── test_loader.rs - 資料載入測試
    │   ├── test_spawner.rs - 關卡生成測試
    │   ├── test_deployment.rs - 單位部署測試
    │   ├── test_movement.rs - 單位移動測試
    │   ├── test_query.rs - World 查詢測試
    │   ├── test_turn.rs  - 回合順序測試
    │   └── test_skill.rs - 技能系統 ECS 操作測試
    └── logic/            - 業務邏輯測試
        ├── mod.rs        - 模組宣告
        ├── board/        - 棋盤與移動測試
        │   ├── mod.rs    - 模組宣告
        │   ├── test_board.rs - 棋盤驗證測試
        │   ├── test_movement.rs - 移動邏輯測試
        │   ├── test_collect_move_reactions.rs - 移動反應收集測試
        │   ├── test_compute_affected_positions.rs - AOE 計算測試
        │   ├── test_select_skill_targets.rs - 技能目標選擇測試
        │   └── test_skill_execute.rs - 技能效果執行測試
        ├── turn/         - 回合順序測試
        │   ├── mod.rs    - 模組宣告
        │   └── test_turn_order.rs - 回合順序計算與管理測試
        └── unit/         - 單位屬性與 ID 測試
            ├── mod.rs    - 模組宣告
            ├── test_attribute.rs - 屬性計算測試
            ├── test_id.rs - ID 生成測試
            └── test_skill_check.rs - 命中與豁免判定測試
```

## Function 集

### logic/board.rs

- `pub fn is_valid_position(board: Board, pos: Position) -> bool` - 驗證位置在棋盤邊界內
- `pub(crate) fn try_position(board: Board, x: i32, y: i32) -> Option<Position>` - 嘗試將整數座標轉換為有效位置

### logic/id_generator.rs

- `pub fn generate_unique_id(used_ids: &mut HashSet<ID>) -> Result<ID>` - 產生不重複的 ID

### logic/movement.rs

- `pub fn step_in_direction(board: Board, pos: Position, direction: Direction) -> Option<Position>` - 計算移動一格後的位置
- `pub fn reachable_positions<F, G>(board: Board, mover: Mover, budget: MovementCost, get_occupant_alliance: F, get_terrain_cost: G) -> Result<HashMap<Position, ReachableInfo>>` - 計算預算內可到達的所有位置
- `pub fn reconstruct_path(reachable: &HashMap<Position, ReachableInfo>, start: Position, target: Position) -> Vec<Position>` - 回溯路徑從起點到目標

### logic/unit_attributes.rs

- `pub fn filter_continuous_effect<'a>(skill_names: &'a [SkillName], buffs: &'a [BuffType], skill_map: &'a HashMap<SkillName, SkillType>) -> Result<impl Iterator<Item = &'a ContinuousEffect>>` - 從技能和狀態中篩選並合併持續性效果
- `pub fn calculate_attributes<'a>(effects: impl Iterator<Item = &'a ContinuousEffect>) -> AttributeBundle` - 計算單位屬性

### logic/turn_order.rs

- `pub fn calculate_turn_order(inputs: &[TurnOrderInput], rng_int: &mut impl FnMut() -> i32, rng_float: &mut impl FnMut() -> f64) -> Vec<TurnEntry>` - 計算一輪的行動順序
- `pub fn delay_unit(entries: &mut Vec<TurnEntry>, target_index: usize) -> Result<()>` - 將單位延後到指定位置（只能往後）
- `pub fn get_active_unit(entries: &[TurnEntry]) -> Option<Occupant>` - 取得下一個未行動的單位
- `pub fn remove_unit(entries: &mut Vec<TurnEntry>, occupant: Occupant) -> Result<TurnEntry>` - 移除指定佔據者的單位
- `pub fn get_active_index(entries: &[TurnEntry]) -> Option<usize>` - 取得下一個未行動的單位索引

### logic/skill.rs

- `pub fn select_skill_targets(caster: &CasterInfo, target_def: &Target, targets: &[Position], units_on_board: &HashMap<Position, UnitInfo>, board_size: Board) -> Result<Vec<Occupant>>` - 驗證並解析技能目標
- `pub fn compute_affected_positions(area: &Area, caster: Position, target: Position, board_size: Board) -> Result<Vec<Position>>` - 計算 AOE 影響的所有位置
- `pub(crate) fn compute_range_positions(caster: Position, range: (Coord, Coord), board_size: Board) -> Vec<Position>` - 計算攻擊距離內的所有位置
- `pub(crate) fn manhattan_distance(a: Position, b: Position) -> Coord` - 計算兩位置的曼哈頓距離
- `pub(crate) fn is_in_filter(caster: &UnitInfo, target: &UnitInfo, filter: &TargetFilter) -> bool` - 判斷目標是否符合技能篩選條件

### logic/skill_check.rs

- `pub fn resolve_hit(attacker_hit: i32, defender_evasion: i32, defender_block: i32, crit_rate: i32, rng_int: &mut impl FnMut() -> i32) -> HitResult` - 解析命中判定結果
- `pub fn resolve_dc(attacker_dc: i32, defender_save: i32, rng_int: &mut impl FnMut() -> i32) -> DcResult` - 解析 DC 豁免判定結果

### logic/skill_execute.rs

- `pub fn resolve_effect_tree(nodes: &[EffectNode], caster: &CombatStats, caster_pos: Position, target_pos: Position, units_on_board: &HashMap<Position, CombatStats>, board_size: Board, rng: &mut impl FnMut() -> i32) -> Vec<EffectEntry>` - 執行效果樹節點並產生效果條目

### logic/skill_reaction.rs

- `pub fn collect_move_reactions(mover: &UnitInfo, path: &[Position], units_on_board: &HashMap<Position, ReactionUnitInfo<'_>>) -> Result<CollectMoveReactionsResult>` - 收集移動路徑上最早觸發反應的所有反應者

### logic/debug.rs

- `pub(crate) fn short_type_name<T: ?Sized>() -> String` - 取得泛型型別的短名稱


### domain/core_types.rs

- `pub fn name(&self) -> &SkillName` (SkillType 方法) - 取得技能名稱

### tests/helpers/level_builder.rs

- `pub fn load_from_ascii(ascii: &str) -> Result<(Board, HashMap<String, Vec<Position>>)>` - 從 ASCII 格式載入棋盤
- `pub fn from_ascii(ascii: &str) -> Self` - 以 ASCII art 初始化關卡建構器
- `pub fn unit(mut self, marker: &str, type_name: &str, faction_id: u32) -> Self` - 設定標記對應的單位類型與陣營
- `pub fn deploy(mut self, marker: &str) -> Self` - 設定標記為部署點
- `pub fn object(mut self, marker: &str, type_name: &str) -> Self` - 設定標記對應的物件類型
- `pub fn max_player_units(mut self, n: usize) -> Self` - 手動設定玩家單位上限
- `pub fn to_unit_map(self) -> Result<(Board, HashMap<String, Vec<Position>>, HashMap<String, Vec<MarkerEntry>>)>` - 解析為棋盤、位置對應及 Marker 條目
- `pub fn to_toml(self) -> Result<String>` - 組裝完整 TOML 字串

### ecs_logic/loader.rs

- `pub fn parse_and_insert_game_data(world: &mut World, units_toml: &str, skills_toml: &str, objects_toml: &str) -> Result<()>` - 反序列化 TOML 並存入 World Resource

### ecs_logic/spawner.rs

- `pub fn spawn_level(world: &mut World, level_toml: &str, level_name: &str) -> Result<()>` - 生成關卡的所有 Entity

### ecs_logic/mod.rs

- `pub(crate) use get_component;` (巨集) - 取得 Component 的便利巨集
- `pub(crate) use get_component_mut;` (巨集) - 取得可變 Component 的便利巨集

### ecs_logic/deployment.rs

- `pub fn deploy_unit(world: &mut World, unit_type_name: &TypeName, position: Position) -> Result<()>` - 部署玩家單位到指定位置
- `pub fn undeploy_unit(world: &mut World, position: Position) -> Result<()>` - 取消指定部署點上的玩家單位部署
- `pub fn remove_deployment_positions(world: &mut World)` - 清除所有部署位置

### ecs_logic/query.rs

- `pub fn get_all_units(world: &mut World) -> Result<HashMap<Position, UnitBundle>>` - 查詢所有單位及其位置
- `pub fn get_all_objects(world: &mut World) -> Result<HashMap<Position, ObjectQueryResult>>` - 查詢所有物件及其位置
- `pub(crate) fn setup_occupant_index(world: &mut World)` - 初始化佔據者索引
- `pub(crate) fn find_entity_by_occupant(world: &World, occupant: Occupant) -> Result<Entity>` - 根據佔據者查找實體
- `pub fn get_resource<'a, T: Resource>(world: &'a World, note: &str) -> Result<&'a T>` - 取得 World Resource（帶錯誤提示）
- `pub(crate) fn get_resource_mut<'a, T: Resource>(world: &'a mut World, note: &str) -> Result<Mut<'a, T>>` - 取得可變 World Resource（帶錯誤提示）

### ecs_logic/movement.rs

- `pub fn get_reachable_positions(world: &mut World, occupant: Occupant) -> Result<HashMap<Position, ReachableInfo>>` - 計算單位可到達的所有位置
- `pub fn execute_move(world: &mut World, target: Position) -> Result<MoveResult>` - 執行當前單位移動到指定位置

### ecs_logic/turn.rs

- `pub fn start_new_round(world: &mut World) -> Result<&TurnOrder>` - 開始新的一輪並回傳
- `pub fn end_current_turn(world: &mut World) -> Result<&TurnOrder>` - 結束當前單位的回合，推進到下一個
- `pub fn can_delay_current_unit(world: &mut World) -> Result<bool>` - 檢查當前單位是否可被延遲
- `pub fn delay_current_unit(world: &mut World, target_index: usize) -> Result<&TurnOrder>` - 延後當前單位到指定位置並回傳
- `pub fn remove_dead_unit(world: &mut World, occupant: Occupant) -> Result<&TurnOrder>` - 移除死亡單位並回傳
- `pub fn get_turn_order(world: &World) -> Result<&TurnOrder>` - 查詢當前回合狀態
- `pub fn end_battle(world: &mut World) -> Result<()>` - 結束戰鬥

### ecs_logic/skill.rs

- `pub fn get_available_skills(world: &mut World) -> Result<Vec<AvailableSkill>>` - 取得當前行動單位的所有主動技能及其可用狀態
- `pub fn get_skill_targetable_positions(world: &mut World, skill_name: &SkillName) -> Result<Vec<Position>>` - 計算指定技能的可攻擊位置
- `pub fn get_skill_affected_positions(world: &mut World, skill_name: &SkillName, target_pos: Position) -> Result<PreviewAffectedPositions>` - 計算指定技能在目標位置的影響範圍預覽

### error.rs

Error 的方法：

- `pub fn kind(&self) -> &ErrorKind` - 取得錯誤種類
