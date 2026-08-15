---
paths:
  - "godot/**/*"
  - "godot_bind/**/*"
---

## Rust GDExtension 整合規則

- `godot_bind` 應負責包裝 `ecs_logic` 的功能。
- 包裝 `ecs_logic` 函式時，函式名稱應盡量與被包裝的函式保持一致；參數與回傳型別可依 Godot API 需求調整。

## 本地化 `.translation` 檔

不用管 `.translation` 二進位檔（如 `ui.en.translation`、`ui.zh_TW.translation`）。
它們由 Godot 編輯器 import `ui.csv` 時自動產生，由用戶自行在編輯器觸發重產。
修改本地化只改 `data/localization/ui.csv`，不要嘗試手動產生或編輯 `.translation`。
