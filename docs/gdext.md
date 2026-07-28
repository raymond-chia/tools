# gdext 易錯處

本檔案只記載 gdext（godot-rust）**容易寫錯或不易從編譯錯誤推出**的地方。
一般寫法（`#[derive(GodotClass)]`、`#[godot_api]`、`#[func]`、`#[var]`/`#[export]`、
`new_gd()` vs `new_alloc()`、`cast`/`upcast`、`bind()`/`bind_mut()`）不在此重複，
需要時查 [The godot-rust book](https://godot-rust.github.io/book/)。

---

## 1. `entry_symbol` 是固定值

`.gdextension` 的 `entry_symbol` 固定為 `gdext_rust_init`，**與 crate 名稱無關**，不可自行更名。

```ini
[configuration]
entry_symbol = "gdext_rust_init"
```

## 2. typed signal API 需要 `Base<T>` 欄位

`self.signals()` 來自 `WithUserSignals` trait，要能使用必須同時具備：

- 一個 inherent `#[godot_api]` 區塊（`#[signal]` 宣告必須放這裡，不能放在 virtual trait 區塊）
- 一個 `Base<T>` 欄位

少了 `Base<T>` 欄位時 `self.signals()` 不存在。

## 3. 類別外部取 signal 不要先 `bind_mut()`

只有 `Gd<T>` 時，`Gd` 本身就有 `signals()`，回傳同一組 API 且不涉及借用檢查：

```rust
let monster: Gd<Monster> = ...;
let sig = monster.signals().damage_taken();  // 正確
```

**不要**寫成 `monster.bind_mut().signals()`。

## 4. 對 `to_gd()` 的結果 `bind_mut()` 必定 panic

在類別方法內 `&mut self` 已持有借用，此時對 `to_gd()` 取得的 `Gd<Self>`
呼叫 `bind_mut()` 是 double-borrow，會 panic（gdext 用內部可變性，違反借用規則是 panic 不是 UB）。

## 5. `OnReady<T>` 與 `OnEditor<T>` 的分工

- `OnReady<Gd<T>>`：需要在 `ready()` 期間自動載入節點或資源時用。
  自動模式（`new()`、`from_base_fn()`、`from_node()`、`from_loaded()`）
  會在 `ready()` 前**依欄位宣告順序**自動初始化。
  `#[init(node = "路徑")]` 等同 `OnReady::from_node("路徑")`。
- `OnEditor<T>`：Rust 端無法提供值、必須由編輯器 Inspector 填入時才用。
  單純要載入節點請用 `OnReady`，不要用 `OnEditor`。

## 6. 匯出 enum 需要三個 derive 加 `via`

```rust
#[derive(GodotConvert, Var, Export, Default, Clone)]
#[godot(via = GString)]  // 或 i64；必須指定底層表示
pub enum Planet {
    #[default]
    Earth,
    Mars,
}
```

## 7. 把 Rust 錯誤精準傳給 Godot

**已於 godot 0.5.4 編譯驗證**（`cargo check` 通過，零警告）。
未驗證的部分見本節末。

### 背景

`#[func]` 自 **v0.5.2** 起可回傳 `Result<T, E>`，機制是 `godot::meta::error::ErrorToGodot` trait：

```rust
pub trait ErrorToGodot<T>: Sized
where
    T: ToGodot,
{
    type Mapped: ToGodot;
    fn result_to_godot(result: Result<T, Self>) -> CallOutcome<Self::Mapped>;
}
```

`strat` 模組**只提供一個內建 impl**：`Unexpected`。它的 `Mapped = T`——
出錯時印 Godot error，然後中止函數**或回傳 `T` 的預設值**。
GDScript 收到的是預設值而非 nil，會被靜默當成正常值，故不適合需要分支的場合。

另兩種常見策略（`()` 回 nil、`global::Error` 回引擎錯誤碼）**並非內建**，需自行 impl。
其中 `global::Error` 的 `Mapped` 就是 error enum 本身，沒有位置放成功值，
**只適用 `Result<(), Error>`**，有回傳值的函數一律不能用。

關鍵限制：`CallOutcome` 只有兩個變體，`Return(R)` 與 `CallFailed(String)`。
**`CallFailed` 只能帶字串**，結構化資訊必須改由 `Mapped` 型別承載，不能靠 `CallFailed` 傳遞。

### 本專案採用的做法

自訂錯誤類別 + `Mapped = Variant`（即「Variant 雙路」：成功與失敗都回 `Variant`，
GDScript 用 `is` 做執行期型別判別）。

Variant 裡裝自訂類別而非 Dictionary，因為本專案有回傳 map 的查詢函數
（映射到 Godot 就是 Dictionary），用 `r is Dictionary` 判別會與成功值撞型。

```rust
// 供 GDScript 讀取的錯誤物件。
#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub struct BoardErrorInfo {
    #[var]
    code: i64,       // 對應 ErrorKind 的 variant，GDScript 用來決定行為
    #[var]
    detail: GString, // 只給 debug 看，不保證可解析
}

// newtype：ErrorToGodot 與 board::Error 都不在本 crate，孤兒規則要求本地型別。
pub struct BoardError(pub board::Error);

// 有了這個 From，`#[func]` 內可直接對 board 函數用 `?`，不必逐處 map_err。
impl From<board::Error> for BoardError {
    fn from(error: board::Error) -> Self {
        Self(error)
    }
}

impl<T: ToGodot> ErrorToGodot<T> for BoardError {
    type Mapped = Variant;

    fn result_to_godot(result: Result<T, Self>) -> CallOutcome<Variant> {
        match result {
            Ok(value) => CallOutcome::Return(value.to_variant()),
            Err(BoardError(err)) => {
                let info = BoardErrorInfo {
                    code: error_code(err.kind()),
                    detail: GString::from(&format!("{err}")),
                };
                CallOutcome::Return(Gd::from_object(info).to_variant())
            }
        }
    }
}
```

已驗證的三點：

- `Mapped = Variant` 成立。
- `impl<T: ToGodot>` 全泛型**不與內建 `Unexpected` impl 衝突**。
- **不需要** `where T::Via: Clone` bound（PR #1544 提及某些 impl 需要，此形狀不需要）。

驗證涵蓋的成功值型別：`()`、`i64`、`Dictionary<GString, i64>`。

### 每個 `#[func]` 零樣板

轉換邏輯集中在上面三塊（`From`、`ErrorToGodot`、`error_code`），
新增 `#[func]` 時只需把回傳型別寫成 `Result<T, BoardError>`，內部直接用 `?`：

```rust
#[func]
fn advance_turn(&mut self) -> Result<i64, BoardError> {
    end_current_turn(&mut self.world)?;              // 無 map_err
    let turn_order = get_turn_order(&self.world)?;   // 無 map_err
    Ok(turn_order.entries.len() as i64)
}
```

`error_code` 只在 `board` 新增 error variant 時才需要改。

### code 粒度：variant 級

`ErrorKind` 是兩層結構（6 個大類、43 個 variant）。code 對到 **variant 層級**，
讓 GDScript 能區分 `OutOfRange` 與 `NoLineOfSight` 等相近錯誤並給精準提示。

分段配號：每個大類佔一個百位區間（Load 100、Data 200、Board 300、
Deployment 400、Unit 500、Reaction 600），新增 variant 往區間尾端加，不重排既有值。

映射函數按大類拆成 6 個子函數，對應 `ErrorKind` 的兩層結構——
不併成單一 `match`，避免嵌套模式降低可讀性。

GDScript 側：

```gdscript
var r = board.plan_move(pos)
if r is BoardErrorInfo:
    match r.code:
        ERR_OUT_OF_RANGE: show_hint()
        ERR_NO_LINE_OF_SIGHT: play_blocked()
        _: push_error(r.detail)
else:
    use_result(r)
```

### 尚未驗證：`Gd<BoardErrorInfo>` 是否被提早釋放

[godot-cpp #652](https://github.com/godotengine/godot-cpp/issues/652) 記載 GDExtension 回傳
RefCounted 物件時可能在函數返回後立刻釋放、GDScript 收到 null。
gdext 的 `Gd<T>` 理論上有處理引用計數，但**必須實跑 Godot 從 GDScript 呼叫才能確認**，
`cargo check` 涵蓋不到。

## 8. `GString` 與 `AsArg` 的三個坑

以下皆為 0.5.4 實際編譯遇到，錯誤訊息不一定指向正解。

### 8.1 `GString` 沒有 `From<String>`

只有 `From<&String>` 與 `From<&str>`。所以 `format!` 的結果不能直接 `.into()`：

```rust
detail: format!("{err}").into()        // 編不過：String 不 Into<GString>
detail: GString::from(&format!("{err}"))  // 正確
```

### 8.2 傳給 `AsArg` 參數要用 `&GString`

`Dictionary::set` 等 API 的參數是 `impl AsArg<K>`，而 `AsArg` 只對 `&GString` 實作，
不對 `GString` 實作——gdext 明說是為效能不做隱式轉換：

```rust
dict.set(GString::from("hp"), 10);   // 編不過
dict.set(&GString::from("hp"), 10);  // 正確
```

`StringName`、`NodePath` 同理。

### 8.3 `Dictionary` 自 0.5 起是泛型

裸寫 `Dictionary` 會得到 E0107（expected 2 generic arguments）：

```rust
fn f() -> Dictionary { ... }                  // 編不過
fn f() -> Dictionary<GString, i64> { ... }    // 正確
```

---

## 參考範例（取自 gdext ReadMe，已驗證）

```rust
use godot::classes::{ISprite2D, ProgressBar, Sprite2D};
use godot::prelude::*;

// Declare the Player class inheriting Sprite2D.
#[derive(GodotClass)]
#[class(init, base=Sprite2D)] // Automatic initialization, no manual init() needed.
struct Player {
    // Inheritance via composition: access to Sprite2D methods.
    base: Base<Sprite2D>,

    // #[class(init)] above allows attribute-initialization of fields.
    #[init(val = 100)]
    hitpoints: i32,

    // Access to a child node, auto-initialized when _ready() is called.
    #[init(node = "Ui/HealthBar")] // <- Path to the node in the scene tree.
    health_bar: OnReady<Gd<ProgressBar>>,
}

// Implement Godot's virtual methods via predefined trait.
#[godot_api]
impl ISprite2D for Player {
    // Override the `_ready` method.
    fn ready(&mut self) {
        godot_print!("Player ready!");

        // Health bar is already initialized and straightforward to access.
        self.health_bar.set_max(self.hitpoints as f64);
        self.health_bar.set_value(self.hitpoints as f64);

        // Connect type-safe signal: print whenever the health bar is updated.
        self.health_bar.signals().value_changed().connect(|hp| {
            godot_print!("Health changed to: {hp}");
        });
    }
}

// Implement custom methods that can be called from GDScript.
#[godot_api]
impl Player {
    #[func]
    fn take_damage(&mut self, damage: i32) {
        self.hitpoints -= damage;
        godot_print!("Player hit! HP left: {}", self.hitpoints);

        // Update health bar.
        self.health_bar.set_value(self.hitpoints as f64);

        // Call Node methods on self, via mutable base access.
        if self.hitpoints <= 0 {
            self.base_mut().queue_free();
        }
    }
}
```

## 查證來源

- [Signals](https://godot-rust.github.io/book/register/signals.html)
- [Objects](https://godot-rust.github.io/book/godot-api/objects.html)
- [Properties](https://godot-rust.github.io/book/register/properties.html)
- [Hello World（入口點）](https://godot-rust.github.io/book/intro/hello-world.html)
- [OnReady](https://godot-rust.github.io/docs/gdext/master/godot/obj/struct.OnReady.html)
- [OnEditor](https://docs.rs/godot/latest/godot/obj/struct.OnEditor.html)
- [ErrorToGodot](https://godot-rust.github.io/docs/gdext/master/godot/meta/error/trait.ErrorToGodot.html)
- [CallOutcome](https://godot-rust.github.io/docs/gdext/master/godot/meta/error/enum.CallOutcome.html)
- [error::strat（內建策略清單）](https://godot-rust.github.io/docs/gdext/master/godot/meta/error/strat/index.html)
- [AsArg](https://godot-rust.github.io/docs/gdext/master/godot/meta/trait.AsArg.html)
- [PR #1544（Result 支援，v0.5.2）](https://github.com/godot-rust/gdext/pull/1544)
- [godot-cpp #652（RefCounted 提早釋放）](https://github.com/godotengine/godot-cpp/issues/652)
