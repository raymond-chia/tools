//! 國際化 (i18n) 系統
//!
//! 提供基於 TOML 的多語言支援

use std::collections::HashMap;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

/// 全局翻譯存儲
static TRANSLATIONS: OnceLock<RwLock<TranslationStore>> = OnceLock::new();

/// 翻譯存儲結構
pub struct TranslationStore {
    /// 當前選擇的語言
    current_locale: String,
    /// 所有語言的翻譯表：locale -> (key -> value)
    translations: HashMap<String, HashMap<String, String>>,
}

/// 初始化 i18n 系統
///
/// # 參數
/// - `locales_dir`: TOML 檔案所在目錄（如 `tools/locales/`）
/// - `default_locale`: 預設語言（如 `"en"` 或 `"zh-TW"`）
///
/// # 範例
/// ```no_run
/// use std::path::PathBuf;
/// use core::i18n;
///
/// let locales_dir = PathBuf::from("tools/locales");
/// i18n::init(&locales_dir, "zh-TW").expect("Failed to initialize i18n");
/// ```
pub fn init(locales_dir: &Path, default_locale: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = TranslationStore {
        current_locale: default_locale.to_string(),
        translations: HashMap::new(),
    };

    // 讀取目錄中的所有 .toml 檔案
    if locales_dir.exists() && locales_dir.is_dir() {
        for entry in std::fs::read_dir(locales_dir)? {
            let entry = entry?;
            let path = entry.path();

            // 只處理 .toml 檔案
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let locale_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("Invalid file name")?
                    .to_string();

                let content = std::fs::read_to_string(&path)?;
                store.load_locale(&locale_name, &content)?;
            }
        }
    }

    TRANSLATIONS
        .set(RwLock::new(store))
        .map_err(|_| "i18n system already initialized")?;

    Ok(())
}

/// 設定當前語言
///
/// # 範例
/// ```no_run
/// use core::i18n;
///
/// i18n::set_locale("en");      // 切換到英文
/// i18n::set_locale("zh-TW");   // 切換到繁體中文
/// ```
pub fn set_locale(locale: &str) {
    if let Some(store) = TRANSLATIONS.get() {
        if let Ok(mut store) = store.write() {
            store.current_locale = locale.to_string();
        }
    }
}

/// 取得當前語言
///
/// # 範例
/// ```no_run
/// use core::i18n;
///
/// let current = i18n::current_locale();
/// println!("Current locale: {}", current);
/// ```
pub fn current_locale() -> String {
    TRANSLATIONS
        .get()
        .and_then(|store| store.read().ok())
        .map(|store| store.current_locale.clone())
        .unwrap_or_else(|| "en".to_string())
}

/// 翻譯函數：根據鍵名查詢翻譯
///
/// # 後備機制
/// 1. 嘗試在當前語言中查找鍵
/// 2. 如果找不到，後備到英文
/// 3. 如果仍然找不到，返回鍵本身
///
/// # 範例
/// ```no_run
/// use core::i18n;
///
/// let name = i18n::t("abilities.Strength");
/// println!("Ability name: {}", name);
/// ```
pub fn t(key: &str) -> String {
    TRANSLATIONS
        .get()
        .and_then(|store| store.read().ok())
        .and_then(|store| {
            // 先嘗試當前語言
            store
                .translations
                .get(&store.current_locale)
                .and_then(|locale_map| locale_map.get(key))
                .or_else(|| {
                    // 後備到英文
                    store
                        .translations
                        .get("en")
                        .and_then(|locale_map| locale_map.get(key))
                })
                .cloned()
        })
        .unwrap_or_else(|| key.to_string()) // 最終後備：返回鍵本身
}

impl TranslationStore {
    /// 載入單一語言的 TOML 檔案
    fn load_locale(&mut self, locale: &str, toml_content: &str) -> Result<(), toml::de::Error> {
        let parsed: toml::Value = toml::from_str(toml_content)?;
        let mut flat_map = HashMap::new();

        // 扁平化 TOML（將嵌套表格轉換為點記法鍵）
        flatten_toml("", &parsed, &mut flat_map);

        self.translations.insert(locale.to_string(), flat_map);
        Ok(())
    }
}

/// 扁平化 TOML 為點記法鍵值對
///
/// # 範例
/// ```toml
/// [abilities]
/// Strength = "力量"
/// ```
///
/// 轉換為：`"abilities.Strength" => "力量"`
fn flatten_toml(prefix: &str, value: &toml::Value, result: &mut HashMap<String, String>) {
    match value {
        toml::Value::Table(table) => {
            for (k, v) in table {
                let new_prefix = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_toml(&new_prefix, v, result);
            }
        }
        toml::Value::String(s) => {
            result.insert(prefix.to_string(), s.clone());
        }
        _ => {} // 忽略其他類型（整數、布林等）
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_flatten_toml() {
        let toml_str = r#"
            [abilities]
            Strength = "力量"
            Dexterity = "敏捷"
        "#;

        let parsed: toml::Value = toml::from_str(toml_str).unwrap();
        let mut result = HashMap::new();
        flatten_toml("", &parsed, &mut result);

        assert_eq!(result.get("abilities.Strength"), Some(&"力量".to_string()));
        assert_eq!(result.get("abilities.Dexterity"), Some(&"敏捷".to_string()));
    }
}
