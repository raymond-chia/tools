//! 錯誤處理系統
//!
//! 自製而非 anyhow 的原因：
//! - 庫開發需要強類型（anyhow 型別擦除）
//! - FFI 邊界需要明確錯誤映射
//! - AI 時代開發速度無差異
//! - 維護成本低

use crate::alias::{Coord, SkillName};
use thiserror::Error as ThisError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// 頂層錯誤，包含原始錯誤和 context 鏈
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    contexts: Vec<String>,
}

/// 錯誤種類
#[derive(Debug, ThisError)]
pub enum ErrorKind {
    #[error(transparent)]
    Board(#[from] BoardError),
    #[error(transparent)]
    Load(#[from] LoadError),
    #[error(transparent)]
    Unit(#[from] UnitError),
}

/// 棋盤錯誤
#[derive(Debug, ThisError)]
pub enum BoardError {
    #[error("位置超出棋盤邊界: ({x}, {y}) 邊界 ({width}, {height})")]
    OutOfBounds {
        x: Coord,
        y: Coord,
        width: Coord,
        height: Coord,
    },
    #[error("佔據者已存在於位置 ({x}, {y})，無法重複插入: {occupant}")]
    OccupantAlreadyExists {
        occupant: String,
        x: Coord,
        y: Coord,
    },
}

/// 格式載入錯誤
#[derive(Debug, ThisError)]
pub enum LoadError {
    #[error("解析失敗: {0}")]
    ParseError(String),
    #[error("{format} 反序列化失敗: {reason}")]
    DeserializeError { format: String, reason: String },
    #[error("{format} 序列化失敗: {reason}")]
    SerializeError { format: String, reason: String },
}

/// 單位相關錯誤
#[derive(Debug, ThisError)]
pub enum UnitError {
    #[error("技能未找到: {skill_name}")]
    SkillNotFound { skill_name: SkillName },
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// 添加錯誤上下文，自動記錄呼叫位置
    #[track_caller]
    pub fn context<C: Into<String>>(mut self, context: C) -> Self {
        let loc = std::panic::Location::caller();
        let msg = format!("{} [{}:{}]", context.into(), loc.file(), loc.line());
        self.contexts.push(msg);
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)?;
        for ctx in &self.contexts {
            write!(f, "\n  {}", ctx)?;
        }
        Ok(())
    }
}

impl<E: Into<ErrorKind>> From<E> for Error {
    fn from(error: E) -> Self {
        Self {
            kind: error.into(),
            contexts: Vec::new(),
        }
    }
}

/// Result 擴展 trait，用於添加錯誤上下文
pub trait Context<T> {
    fn context<C: Into<String>>(self, context: C) -> Result<T>;
}

impl<T> Context<T> for Result<T> {
    fn context<C: Into<String>>(self, context: C) -> Result<T> {
        self.map_err(|e| e.context(context))
    }
}
