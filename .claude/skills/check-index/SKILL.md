---
name: check-index
description: 檢查並更新索引文件，使其準確反映現有檔案結構與公開函數
disable-model-invocation: false
---

請檢查 `.claude/rules/core-index.md` 和 `.claude/rules/editor-index.md` 是否準確反映現況，並直接修改使其符合。

工作目錄：`$CLAUDE_PROJECT_DIR`（即專案根目錄）

## 步驟 1：確認當前檔案結構

掃描以下目錄的所有 `.rs` 檔案：

- `core/board/src/`
- `editor/src/`

對照索引文件中的「專案結構」區塊，找出：

- 索引中有但實際不存在的檔案
- 實際存在但索引中缺少的檔案

## 步驟 2：確認公開函數簽名

讀取 `core/board/src/` 和 `editor/src/` 下所有 `.rs` 檔案，提取所有 `pub fn`，對照索引文件中的「Function 集」區塊，找出：

- 索引中有但實際不存在的函數
- 實際存在但索引中缺少的函數（包含步驟 1 發現的新增檔案）
- 簽名與實際不符的函數

## 步驟 3：更新索引文件

依照各索引文件開頭的「編輯規則」更新：

- 補上缺少的檔案與函數
- 移除已不存在的項目
- 修正簽名錯誤

更新完成後，簡短報告修改了哪些內容。
