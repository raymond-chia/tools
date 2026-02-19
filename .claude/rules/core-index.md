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
│   ├── lib.rs            - Crate 根節點，重新導出所有 pub mod
│   ├── alias.rs          - 類型別名（Coord, ID, MovementCost）
│   ├── component.rs      - ECS Component 定義
│   ├── constants.rs      - 遊戲常數
│   ├── core_types.rs      - 基本資料類型定義（未來使用）
│   ├── error.rs          - 錯誤型別定義
│   ├── loader.rs         - 棋盤載入（ASCII 解析）
│   ├── loader_schema.rs  - Loader 相關資料結構定義
│   ├── schema.rs         - 遊戲資料結構定義（Occupant 等）
│   ├── logic/            - 核心業務邏輯（非 ECS System）
│   │   ├── mod.rs        - logic 模組定義
│   │   ├── board.rs      - 棋盤驗證
│   │   ├── movement.rs   - 移動邏輯
│   │   └── unit_attributes.rs - 單位屬性計算邏輯
│   └── system/           - ECS System（目前未實作）
│       ├── mod.rs        - system 模組定義
│       └── position_query.rs - 位置到實體的索引維護
└── tests/                - 整合測試
    ├── board/
    │   ├── mod.rs        - board 測試模組定義
    │   ├── test_board.rs
    │   └── test_movement.rs
    ├── unit_attributes/
    │   └── mod.rs        - 單位屬性計算測試
    ├── test.rs
    └── test_error.rs
```

### alias.rs

- 存放類型別名（type alias）

### component.rs

- 存放 ECS Component：只存資料，不含業務邏輯
- 必須 derive `Component` 和 `Debug`
- 禁止出現 impl

### constants.rs

- 集中存放遊戲常數定義

### core_types.rs

- 遊戲核心資料型別定義

### error.rs

- 集中定義所有錯誤類型
- 確保錯誤訊息包含豐富上下文

### loader.rs

- 棋盤載入邏輯（如 ASCII 解析）

### loader_schema.rs

- 載入相關的資料結構定義（遊戲內容、屬性、技能效果系統）

### schema.rs

- 遊戲資料結構定義（enum、struct）
- 與 ECS 無關，支援各層使用

### logic/

- 存放核心業務邏輯函數（純邏輯運算，不依賴 ECS Query）
- 可以依賴 component.rs 和 core_types.rs 的類型
- 函數參數類型放在同一個檔案中（與函數一起）

### system/

- ECS System 模組的定義入口
- 目前未實作，邏輯優先放在 logic/ 中

## Function 集

### logic/board.rs

- `pub fn is_valid_position(board: Board, pos: Position) -> bool` - 驗證位置是否在棋盤邊界內

### logic/movement.rs

- `pub fn step_in_direction(board: Board, pos: Position, direction: Direction) -> Option<Position>` - 計算從位置往方向移動一格
- `pub fn reachable_positions<F, G>(board: Board, mover: Mover, budget: MovementCost, get_occupant_faction: F, get_terrain_cost: G) -> Result<HashMap<Position, ReachableInfo>>` - 計算預算內可到達的所有位置

### loader.rs

- `pub fn load_from_ascii(ascii: &str) -> Result<(Board, Vec<Position>, HashMap<String, Vec<Position>>)>` - 從 ASCII 格式載入棋盤

### logic/unit_attributes.rs

- `pub fn calculate_attributes(skill_names: &[SkillName], buffs: &[BuffEffect], skill_map: &HashMap<SkillName, SkillType>) -> Result<CalculatedAttributes>` - 計算單位屬性

### core_types.rs

OccupantMap 的方法：

- `pub fn get_occupants_at(&self, pos: Position) -> &[Occupant]` - 查詢指定位置的所有佔據者
- `pub fn get_position_of(&self, occupant: Occupant) -> Option<Position>` - 查詢指定佔據者的位置
- `pub fn insert(&mut self, pos: Position, occupant: Occupant) -> Result<()>` - 插入佔據者到指定位置
- `pub fn remove(&mut self, occupant: Occupant)` - 移除指定的佔據者
