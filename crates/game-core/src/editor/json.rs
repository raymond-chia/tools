//! Godot editor JSON 的數字轉換集中於輸入邊界，欄位型別仍由共用 Rust 型別決定。
use serde::de::DeserializeOwned;
use serde_json::Value;

pub(super) fn from_str<T: DeserializeOwned>(text: &str) -> Result<T, serde_json::Error> {
    let mut value: Value = serde_json::from_str(text)?;
    normalize_numbers(&mut value);
    // 保留型別錯誤在轉換後 JSON 中的行列資訊。
    serde_json::from_str(&value.to_string())
}

pub(super) fn from_value<T: DeserializeOwned>(mut value: Value) -> Result<T, serde_json::Error> {
    normalize_numbers(&mut value);
    serde_json::from_value(value)
}

fn normalize_numbers(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                normalize_numbers(value);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                normalize_numbers(value);
            }
        }
        Value::Number(number) if number.is_f64() => {
            if let Some(value) = number.as_f64() {
                if value.fract() == 0.0 {
                    // 透過 JSON Number 解析，避免浮點轉整數的截斷或飽和轉型。
                    // 非整數與超出整數範圍的值仍交由目標欄位型別拒絕；浮點欄位也接受整數表示。
                    if let Ok(integer) = format!("{value:.0}").parse() {
                        *number = integer;
                    }
                }
            }
        }
        _ => {}
    }
}
