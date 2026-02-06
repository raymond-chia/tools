use board::error::{Error, LoadError};

// clear; cargo fmt; cargo test -- --nocapture
#[test]
fn show_error_messages() {
    let scene_err = LoadError::ParseError("Invalid symbol".to_string());
    let err: Error = scene_err.into();
    let err = err
        .context("解析行：5")
        .context("處理棋盤配置")
        .context("載入場景檔案");

    let error_str = err.to_string();

    // 驗證原始錯誤訊息
    assert!(error_str.contains("Invalid symbol"));

    // 驗證 contexts 存在
    assert!(error_str.contains("解析行：5 [core\\board\\tests\\test_error.rs:9]"));
    assert!(error_str.contains("處理棋盤配置 [core\\board\\tests\\test_error.rs:10]"));
    assert!(error_str.contains("載入場景檔案 [core\\board\\tests\\test_error.rs:11]"));
}
