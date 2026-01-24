# 通用規則

- **語言**: 繁體中文（程式碼、註解、文件），禁止簡體中文
- 使用 match 而不要 let else
- match arm 使用編譯器的 exhaustiveness checking 保護，避免未來忘記添加 match arm
- 撰寫計畫的時候不要添加程式碼，請保持精簡
- 如果多數 function caller 已經知道具體類型，不要只傳遞基本型別再反推
- 確保型別安全，有完整錯誤處理（錯誤訊息包含豐富上下文）
- 禁止 magic numbers/strings
- `use` 語句放在檔案頂部，不要放在 function 裡面
- 不確定時詢問使用者，不要自行決定

# 本專案

- 本專案是戰術回合制 RPG 遊戲（Rust）。
- 禁止向後相容
- 使用 test driven (TDD)

## 基本指令

```bash
# 測試
cargo test
```

## **Struct/Enum 設計**

- 所有 struct/enum 都必須 derive Debug
- 補齊必要的 derive
- **不要預先設計**：只補現在用到的 derive、方法、錯誤類型
- 有需要時再添加，不要預測未來需求

## **測試**:

- 不替以下撰寫測試: `editor` crate、inner functions
- 只有在副作用難以測試時才修改程式碼邏輯

## 專案索引

- 如果發現 core 底下的檔案結構跟本檔案紀錄不合的時候，請更新本檔案 `專案結構`
- 如果發現 core 底下的 function 與本檔案紀錄不合的時候，請更新本檔案 `function 集`  
  只列出公開函數（pub fn），不包含 struct/enum 定義  
  格式：`函數簽名` - 簡短說明

### 專案結構

```
core/
└── board/          戰棋板邏輯與數據結構
    ├── logic/      (暫未實現)
    └── types/
        ├── error.rs     - 錯誤
        └── position.rs  - Pos (位置座標)
```

### function 集

**core/board:**

- `Pos::new(x: usize, y: usize) -> Pos` - 建立新的位置座標
