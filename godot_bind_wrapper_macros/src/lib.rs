use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemImpl, Path, Token, parse_macro_input, parse_quote};

// 保存 `with = handler` 解析出的轉換函式路徑。
struct GodotResultArgs {
    handler: Path,
}

// 定義 attribute 參數的解析規則。
impl Parse for GodotResultArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // 讀取等號左側的參數名稱。
        let name: Ident = input.parse()?;
        // 只接受名稱 `with`。
        if name != "with" {
            return Err(syn::Error::new(name.span(), "預期使用 `with = handler`"));
        }
        // 讀取 `with` 後方的等號。
        input.parse::<Token![=]>()?;
        // 讀取等號右側的 handler 路徑。
        let handler = input.parse()?;
        // 拒絕 handler 後方的多餘參數。
        if !input.is_empty() {
            return Err(input.error("with handler 後方不應有其他參數"));
        }
        // 回傳解析完成的參數。
        Ok(Self { handler })
    }
}

/// 將 Godot API impl 內各 `#[func]` 的 `Result` 本體轉成固定的 Dictionary 回傳格式。
///
/// 此 attribute 必須放在 `#[godot_api]` 上方，並以 `with` 指定轉換函式。
// 將此函式宣告為 attribute procedural macro。
#[proc_macro_attribute]
pub fn godot_result(attribute: TokenStream, item: TokenStream) -> TokenStream {
    // 取得使用端指定的 Result 轉換函式。
    let GodotResultArgs { handler } = parse_macro_input!(attribute as GodotResultArgs);
    // 將被標記的 impl 解析成可修改的語法樹。
    let mut implementation = parse_macro_input!(item as ItemImpl);
    // 記錄實際包裝的方法數量。
    let mut wrapped_count = 0;

    // 逐一檢查 impl 內的項目。
    for item in &mut implementation.items {
        // 只處理方法，略過常數與其他項目。
        let function = match item {
            syn::ImplItem::Fn(function) => function,
            _ => continue,
        };
        // 略過沒有標記 `#[func]` 的 Rust 方法。
        if !function
            .attrs
            .iter()
            .any(|attribute| attribute.path().is_ident("func"))
        {
            continue;
        }

        // 複製方法名稱，供錯誤訊息前綴使用。
        let function_name = function.sig.ident.clone();
        // 複製方法原本的函式內容。
        let body = function.block.clone();
        // 將原始本體放進立即執行的無參數 closure，再把執行結果交給 handler。
        // 這層 closure 會形成局部的 Result 回傳邊界：原始本體中的 `?` 會從 closure
        // 提前回傳 Err，`Ok(...)` 則是 closure 的成功值，不會受外層方法宣告回傳
        // Dictionary 影響；`(|| #body)()` 尾端的 `()` 代表建立後立刻執行 closure。
        function.block = parse_quote!({
            #handler(stringify!(#function_name), (|| #body)())
        });
        // 累計已完成包裝的方法。
        wrapped_count += 1;
    }

    // 沒有找到 `#[func]` 時提示 attribute 順序可能錯誤。
    if wrapped_count == 0 {
        return syn::Error::new_spanned(
            implementation,
            "godot_result 找不到 #[func]；請確認它位於 #[godot_api] 上方",
        )
        .into_compile_error()
        .into();
    }

    // 將修改後的 impl 轉回編譯器需要的 TokenStream。
    quote!(#implementation).into()
}
