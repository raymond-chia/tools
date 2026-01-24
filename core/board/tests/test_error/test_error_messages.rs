use board::types::{Error, SceneError};

// clear; cargo fmt; cargo.exe test -- --nocapture
#[test]
fn show_error_messages() {
    // InvalidDimensions
    let scene_err = SceneError::InvalidDimensions("5-5 格式不支援".to_string());
    let err: Error = scene_err.into();
    let err = err
        .context("解析行：5")
        .context("處理棋盤配置")
        .context("載入場景檔案");

    let error_str = err.to_string();

    // 驗證原始錯誤訊息
    assert!(error_str.contains("5-5 格式不支援"));

    // 驗證 contexts 存在
    assert!(
        error_str.contains("解析行：5 [core\\board\\tests\\test_error\\test_error_messages.rs")
    );
    assert!(
        error_str.contains("處理棋盤配置 [core\\board\\tests\\test_error\\test_error_messages.rs")
    );
    assert!(
        error_str.contains("載入場景檔案 [core\\board\\tests\\test_error\\test_error_messages.rs")
    );
}
