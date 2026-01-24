#[derive(Debug, Clone, thiserror::Error)]
pub enum SceneError {
    #[error("無效的棋盤維度: {0}")]
    InvalidDimensions(String),

    #[error("未知符號: '{0}'")]
    InvalidSymbol(char),

    #[error("解析失敗: {0}")]
    ParseError(String),
}
