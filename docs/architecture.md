# 架構設計

## UI 與 Core 的互動流程

```
UI (Godot / egui)
    │
    │  呼叫 Rust API（具體方式待定）
    ▼
Rust API 層（Bridge）
    │
    │  呼叫內部函數，操作 bevy_ecs World
    ▼
Core 邏輯（多個函數協作，修改 World 並組裝回傳值）
    │
    │  回傳結果（Snapshot 或 ViewModel，待定）
    ▼
UI 收到結果，推導或直接渲染
```

- UI 不直接接觸 `World`，透過 Bridge 層操作
- 一次 API 呼叫對應一個入口函數，該函數內部決定呼叫順序
- 不使用 bevy Schedule，邏輯流程由 Rust 程式碼自行控制

## 設計決策

### 只用 bevy_ecs，不用 bevy 引擎

bevy 引擎本身變動頻繁且功能不如 Godot 完整（bevy 官方也建議正式專案考慮 Godot）。  
但 ECS 的資料導向架構有利於維護，因此只取用 bevy_ecs 作為資料層，遊戲前端由 Godot 負責。

### 不使用 bevy Schedule

回合制戰棋不需要每幀更新。手動控制函數呼叫順序更直觀，也更容易除錯。

### 不使用 Event 模式回傳

Event 模式要求前端維護自身狀態並根據事件更新，增加前端複雜度。回傳結果（Snapshot 或 ViewModel）讓前端邏輯更單純。

### 回傳格式待定（Snapshot vs ViewModel）

- Snapshot：回傳遊戲事實，前端自行推導 UI 狀態
- ViewModel：Core 算好 UI 該呈現什麼，前端直接渲染
- 可混合使用，非二選一
- 延後決定，待實際需求明確後再確定
