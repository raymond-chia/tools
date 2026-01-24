use board::types::SceneError;

#[test]
fn show_error_messages() {
    // InvalidDimensions
    let err = SceneError::InvalidDimensions("5-5 格式不支援".to_string());
    println!("InvalidDimensions: {}", err);
}
