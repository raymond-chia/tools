use board::error::{Error, LoadError, create_backtrace};
use std::backtrace::BacktraceStatus;

#[test]
fn create_backtrace_respects_debug_mode() {
    let test_data = [(false, None), (true, Some(BacktraceStatus::Captured))];

    for (debug_mode, expected_status) in test_data {
        let actual_status = create_backtrace(debug_mode).map(|backtrace| backtrace.status());
        assert_eq!(actual_status, expected_status);
    }
}

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
        error_str.contains("test_error.rs:17"),
        "backtrace 應包含 into() 呼叫位置: {error_str}"
    );
}
