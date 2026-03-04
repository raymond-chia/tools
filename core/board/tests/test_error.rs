use board::error::{Error, LoadError};

#[test]
fn backtrace_contains_creation_site() {
    let scene_err = LoadError::ParseError("Invalid symbol".to_string());
    let err: Error = scene_err.into();

    let error_str = err.to_string();

    // 驗證原始錯誤訊息
    assert!(error_str.contains("Invalid symbol"));

    // 驗證包含 fn
    assert!(error_str.contains("backtrace_contains_creation_site"));

    // 驗證 backtrace 包含錯誤建立處的檔案與行號
    assert!(
        error_str.contains("test_error.rs:6"),
        "backtrace 應包含 into() 呼叫位置: {error_str}"
    );
}
