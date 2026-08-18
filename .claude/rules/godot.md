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

## UI Theme

- 共用 Theme 位於 `assets/themes/game_ui.tres`。
- 一般 UI 文字使用 Theme 的 `default_font_size`；不要在場景節點新增字級覆寫。
- 標題 `Label` 必須設定 `theme_type_variation = &"Title"`。`Title` 的字級統一在 `game_ui.tres` 管理，不要在個別場景設定 `theme_override_font_sizes/font_size`。

## UI 滾輪輸入

- 覆蓋戰場的互動 UI 根容器應設定 `mouse_filter = 0` 與 `mouse_force_pass_scroll_events = false`，避免滾輪事件傳到戰場的 `_unhandled_input()`。
