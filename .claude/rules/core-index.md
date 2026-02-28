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

保留完整簽名（pub fn、trait 方法），移除實現細節, 常數值, enum, struct

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
│   │   ├── components.rs - ECS Component 定義
│   │   └── resources.rs  - ECS World Resource 定義
│   ├── ecs_logic/        - ECS 操作函數
│   │   ├── mod.rs        - 模組宣告
│   │   ├── loader.rs     - 遊戲資料載入函數
│   │   ├── spawner.rs    - 關卡生成函數
│   │   ├── deployment.rs - 單位部署函數
│   │   └── query.rs      - World 查詢函數
│   └── logic/            - 核心業務邏輯
│       ├── mod.rs        - 邏輯模組定義
│       ├── board.rs      - 棋盤驗證邏輯
│       ├── id_generator.rs - ID 產生邏輯
│       ├── movement.rs   - 移動邏輯
│       ├── unit_attributes.rs - 單位屬性計算邏輯
│       └── debug.rs      - 調試工具函數
└── tests/
    ├── test.rs           - 集成測試入口
    ├── test_error.rs     - 錯誤型別測試
    ├── test_helpers/     - 測試輔助工具
    │   ├── mod.rs        - 測試輔助模組
    │   └── level_builder.rs - 測試用 ASCII 關卡建構工具
    ├── ecs_logic/        - 關卡生成與部署測試
    │   ├── mod.rs        - 模組宣告
    │   ├── constants.rs  - ECS 常數測試
    │   ├── loader.rs     - 資料載入測試
    │   ├── spawner.rs    - 關卡生成測試
    │   ├── deployment.rs - 單位部署測試
    │   └── query.rs      - World 查詢測試
    └── logic/            - 業務邏輯測試
        ├── mod.rs        - 模組宣告
        ├── board/        - 棋盤與移動測試
        │   ├── mod.rs    - 模組宣告
        │   ├── test_board.rs - 棋盤驗證測試
        │   └── test_movement.rs - 移動邏輯測試
        └── unit/         - 單位屬性與 ID 測試
            ├── mod.rs    - 模組宣告
            ├── test_attribute.rs - 屬性計算測試
            └── test_id.rs - ID 生成測試
```

### error.rs

- 集中定義所有錯誤類型
- 確保錯誤訊息包含豐富上下文

### loader_schema.rs

- 載入相關資料結構定義

### domain/

遊戲領域模型（與 ECS 無關）

#### alias.rs

- 類型別名定義

#### constants.rs

- 遊戲常數定義

#### core_types.rs

- 遊戲核心資料型別定義

### ecs_types/

ECS 框架相關的型別定義

#### components.rs

- ECS Component 定義（只存資料，不含業務邏輯）
- 必須 derive `Component` 和 `Debug`
- 禁止出現 impl

#### resources.rs

- ECS World Resource 型別定義

### logic/

核心業務邏輯函數（純邏輯運算，不依賴 ECS Query）

- 可以依賴 domain/ 的類型
- 函數參數類型放在同一個檔案中

### ecs_logic/

直接操作 World 的函數（讀寫 World）

- 負責整合 logic/ 和 World
- 提供完整的遊戲操作 API

## Function 集

### logic/board.rs

- `pub fn is_valid_position(board: Board, pos: Position) -> bool` - 驗證位置在棋盤邊界內

### logic/id_generator.rs

- `pub fn generate_unique_id(used_ids: &mut HashSet<ID>) -> ID` - 產生不重複的 ID

### logic/movement.rs

- `pub fn step_in_direction(board: Board, pos: Position, direction: Direction) -> Option<Position>` - 計算移動一格後的位置
- `pub fn reachable_positions<F, G>(board: Board, mover: Mover, budget: MovementCost, get_occupant_faction: F, get_terrain_cost: G) -> Result<HashMap<Position, ReachableInfo>>` - 計算預算內可到達的所有位置

### logic/unit_attributes.rs

- `pub fn calculate_attributes(skill_names: &[SkillName], buffs: &[BuffEffect], skill_map: &HashMap<SkillName, SkillType>) -> Result<CalculatedAttributes>` - 計算單位屬性

### logic/debug.rs

- `pub fn short_type_name<T: ?Sized>() -> String` - 取得泛型型別的短名稱

### tests/test_helpers/level_builder.rs

- `pub fn load_from_ascii(ascii: &str) -> Result<(Board, HashMap<String, Vec<Position>>)>` - 從 ASCII 格式載入棋盤（僅供測試使用）
- `LevelBuilder::from_ascii(ascii: &str) -> Self` - 以 ASCII art 初始化關卡建構器
- `.unit(marker: &str, type_name: &str, faction_id: u32) -> Self` - 設定標記對應的單位類型與陣營
- `.deploy(marker: &str) -> Self` - 設定標記為部署點
- `.object(marker: &str, type_name: &str) -> Self` - 設定標記對應的物件類型
- `.max_player_units(n: usize) -> Self` - 手動設定玩家單位上限
- `.to_toml(self) -> Result<String>` - 組裝完整 TOML 字串

### ecs_logic/loader.rs

- `pub fn parse_and_insert_game_data(world: &mut World, units_toml: &str, skills_toml: &str, objects_toml: &str) -> Result<()>` - 反序列化 TOML 並存入 World Resource

### ecs_logic/spawner.rs

- `pub fn spawn_level(world: &mut World, level_toml: &str, level_name: &str) -> Result<()>` - 生成關卡的所有 Entity

### ecs_logic/deployment.rs

- `pub fn deploy_unit(world: &mut World, unit_type_name: &TypeName, position: Position) -> Result<()>` - 部署玩家單位到指定位置
- `pub fn undeploy_unit(world: &mut World, position: Position) -> Result<()>` - 取消指定部署點上的玩家單位部署

### ecs_logic/query.rs

- `pub fn get_board(world: &World) -> Result<Board>` - 取得棋盤尺寸
- `pub fn get_deployment_config(world: &World) -> Result<DeploymentConfig>` - 取得部署設定
- `pub fn get_level_config(world: &World) -> Result<LevelConfig>` - 取得關卡設定
- `pub fn get_all_units(world: &mut World) -> Result<HashMap<Position, UnitBundle>>` - 查詢所有單位
- `pub fn get_all_objects(world: &mut World) -> Result<HashMap<Position, ObjectBundle>>` - 查詢所有物件

### domain/core_types.rs

OccupantMap 的方法：

- `pub fn get_occupants_at(&self, pos: Position) -> &[Occupant]` - 查詢指定位置的所有佔據者
- `pub fn get_position_of(&self, occupant: Occupant) -> Option<Position>` - 查詢指定佔據者的位置
- `pub fn insert(&mut self, pos: Position, occupant: Occupant) -> Result<()>` - 插入佔據者到指定位置
- `pub fn remove(&mut self, occupant: Occupant)` - 移除指定的佔據者

### error.rs

Error 的方法：

- `pub fn kind(&self) -> &ErrorKind` - 取得錯誤種類
- `pub fn context<C: Into<String>>(mut self, context: C) -> Self` - 新增錯誤上下文資訊
