use board::types::{BoardError, Error};

// clear; cargo fmt; cargo.exe test -- --nocapture
#[test]
fn show_error_messages() {
    // InvalidDimensions
    let scene_err = BoardError::InvalidDimensions(5, 0);
    let err: Error = scene_err.into();
    let err = err
        .context("解析行：5")
        .context("處理棋盤配置")
        .context("載入場景檔案");

    let error_str = err.to_string();

    // 驗證原始錯誤訊息
    assert!(error_str.contains("width=5"));
    assert!(error_str.contains("height=0"));

    // 驗證 contexts 存在
    assert!(error_str.contains("解析行：5 [core\\board\\tests\\test_error.rs"));
    assert!(error_str.contains("處理棋盤配置 [core\\board\\tests\\test_error.rs"));
    assert!(error_str.contains("載入場景檔案 [core\\board\\tests\\test_error.rs"));
}
