# Core/Board 專案索引

本檔案包含 `core/board` crate 的專案結構和 function 集。
此檔案不設定 paths 限制，讓其他 crate（如 editor）也能參考。

## 維護原則

**專案結構**
- 紀錄檔案存在與主要職責，不列舉具體的 enum variants 或 struct fields
- 如果有新增/移除檔案才更新

**Function 集和 Trait**
- 保留 **簽名**：pub fn、trait 方法（API 相對穩定，幫助理解「怎麼用」）
- 移除 **實現細節**（如算法說明、檢查邏輯），這些容易變
- 移除會頻繁變動的具體值（如常數值、enum variants）

## 專案結構

⚠️ **編輯前檢查清單**（見上方「維護原則」）
- [ ] 只記錄檔案存在與主要職責？
- [ ] 沒有列舉 enum variants 或 struct fields？
- [ ] 新增/移除檔案時才編輯？

```
core/board/
├── src/
│   ├── alias.rs          - 類型別名（Coord, ID, MovementCost）
│   ├── component.rs      - ECS Component 定義
│   ├── constants.rs      - 遊戲常數（BASIC_MOVEMENT_COST, IMPASSABLE_MOVEMENT_COST）
│   ├── error.rs          - 錯誤型別定義
│   ├── loader.rs         - 棋盤載入（ASCII 解析）
│   ├── loader_schema.rs  - Loader 相關資料結構定義（遊戲內容、屬性、技能效果系統）
│   ├── primitive.rs      - 基本資料類型定義（未來使用）
│   ├── logic/            - 核心業務邏輯（非 ECS System）
│   │   ├── board.rs      - 棋盤驗證
│   │   └── movement.rs   - 移動邏輯
│   └── system/           - ECS System（目前未實作）
└── tests/                - 測試
    ├── board/
    │   ├── test_board.rs
    │   └── test_movement.rs
    ├── test.rs
    └── test_error.rs
```

## Function 集

⚠️ **編輯前檢查清單**（見上方「維護原則」）
- [ ] 保留了完整的函數簽名？
- [ ] 移除了實現細節（算法說明、檢查邏輯）？
- [ ] 沒有列舉 enum variants 或常數值？

### logic/board.rs
- `pub fn is_valid_position(board: Board, pos: Position) -> bool` - 驗證位置是否在棋盤邊界內

### logic/movement.rs
- `pub fn step_in_direction(board: Board, pos: Position, direction: Direction) -> Option<Position>` - 計算從位置往方向移動一格
- `pub fn reachable_positions<F, G>(board: Board, mover: Mover, budget: MovementCost, get_occupant_faction: F, get_terrain_cost: G) -> Result<HashMap<Position, ReachableInfo>>` - 計算預算內可到達的所有位置

### loader.rs
- `pub fn load_from_ascii(ascii: &str) -> Result<(Board, Vec<Position>, HashMap<String, Vec<Position>>)>` - 從 ASCII 格式載入棋盤
