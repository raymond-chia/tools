//! 調試相關的工具函數

use std::any::type_name;

/// 取得泛型型別的短名稱（移除模組路徑）
pub fn short_type_name<T: ?Sized>() -> String {
    let full_name = type_name::<T>();
    full_name
        .split("::")
        .last()
        .unwrap_or(full_name)
        .to_string()
}
