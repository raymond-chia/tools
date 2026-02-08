# Claude Code 代辦檢查清單

- [ ] **語言**: 繁體中文（註解），禁止簡體中文
  - ⚠️ **所有回應禁止簡體中文**
- [ ] **Single-responsibility principle**：每個函數和型別只負責一項職責
- [ ] **變數命名**：使用有辨識度的名稱，避免相似的縮寫（例如同一作用域內不要同時使用 `attr` 和 `attribute`）
- [ ] **錯誤訊息包含豐富上下文**：包含操作、預期值、實際值等
- [ ] **禁止 `unwrap()`**：使用 `expect()` 並提供清晰的錯誤訊息
- [ ] **禁止 magic numbers/strings**：使用常數替代
- [ ] **使用 match 而不要 let else**
- [ ] **match arm 有編譯器 exhaustiveness 保護**：避免遺漏 arm，依賴編譯器檢查
- [ ] **Fail fast**：在函數開頭進行所有驗證和檢查，不在執行中間檢查
- [ ] **只補現在用到的內容**：不預先 derive、不添加暫時不用的方法或錯誤類型
- [ ] **優先 derive，不手動實現**：除非 trait 無法被 derive，否則使用 derive（Default、Clone、Debug、Copy 等）
- [ ] `use` 語句放在檔案頂部，不要放在 function 裡面
- [ ] 使用 **functional programming**。請勿使用 oop
- [ ] 每當 Claude Code 提出設計建議時，先明確引用 CLAUDE.md 的 **代辦檢查清單** 來論證為什麼這個設計符合規則

## **技術判斷原則**

- [ ] **不盲目同意**
  - 對用戶建議必須驗證（測試、推導、實驗），不直接認可
  - 對我自己的設計判斷也要同樣嚴謹
  - 區分「理論上可能」與「確實有效」

# 本專案

本專案是戰術回合制 RPG 遊戲（Rust）

- [ ] **禁止向後相容**
- [ ] **優先使用 alias**：使用 `core/board/src/alias.rs` 中的類型別名（`Coord`、`MovementCost`、`ID`、`TypeName`、`SkillName`）提升語義清晰度
- [ ] **檢查設計機制**：寫完後檢查是否有違反 D:\Mega\prog\rust\tools\README-設計機制.md。如果只是尚未實作完畢，只要提示尚未實作就好。如果違反請警告使用者

## 檢查指令

- [ ] **檢查指令：只用 `cargo check`**
  - ⚠️ 禁止用 `cargo run` 或 `cargo build`
  - ⚠️ 禁止用 `cd` 改變目錄後再執行
  - ✅ 正確：`cargo check` 或 `cargo check -p <crate_name>`
