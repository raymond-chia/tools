//! 錯誤處理系統
//!
//! 自製而非 anyhow 的原因：
//! - 庫開發需要強類型（anyhow 型別擦除）
//! - FFI 邊界需要明確錯誤映射
//! - AI 時代開發速度無差異
//! - 維護成本低

use crate::domain::alias::{Coord, SkillName, TypeName};
use crate::ecs_types::components::Occupant;
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
use thiserror::Error as ThisError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// 頂層錯誤，包含原始錯誤和 backtrace
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    backtrace: Backtrace,
}

/// 錯誤種類
#[derive(Debug, ThisError)]
pub enum ErrorKind {
    #[error(transparent)]
    Load(#[from] LoadError),
    #[error(transparent)]
    Data(#[from] DataError),
    #[error(transparent)]
    Board(#[from] BoardError),
    #[error(transparent)]
    Deployment(#[from] DeploymentError),
    #[error(transparent)]
    Unit(#[from] UnitError),
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

/// 遊戲資料存取錯誤
#[derive(Debug, ThisError)]
pub enum DataError {
    #[error("找不到 {name} resource\nNOTE: {note}")]
    MissingResource { name: String, note: String },
    #[error("{name} resource 已存在\nNOTE: {note}")]
    ResourceAlreadyExists { name: String, note: String },
    #[error("Entity 缺少必要的 component: {name}")]
    MissingComponent { name: String },
    #[error("Entity component 值無效: {name}\nNOTE: {note}")]
    InvalidComponent { name: String, note: String },
    #[error("找不到單位類型: {type_name}")]
    UnitTypeNotFound { type_name: TypeName },
    #[error("找不到物件類型: {type_name}")]
    ObjectTypeNotFound { type_name: TypeName },
}

/// 棋盤錯誤
#[derive(Debug, ThisError)]
pub enum BoardError {
    #[error("位置超出棋盤邊界: ({x}, {y}) 邊界 ({width}, {height})")]
    OutOfBoard {
        x: Coord,
        y: Coord,
        width: Coord,
        height: Coord,
    },
    #[error("位置 ({x}, {y}) 不可到達")]
    Unreachable { x: Coord, y: Coord },
    // turn
    #[error("沒有未行動的單位")]
    NoActiveUnit,
    #[error("無效的延後目標：當前位置 {current}，不能延後到 {target}")]
    InvalidDelayTarget { current: usize, target: usize },
    #[error("佔據者不存在於回合表中: {occupant:?}")]
    OccupantNotFound { occupant: Occupant },
}

/// 部署相關錯誤
#[derive(Debug, ThisError)]
pub enum DeploymentError {
    #[error("位置 ({x}, {y}) 不在合法部署區域內")]
    PositionNotDeployable { x: Coord, y: Coord },
    #[error("已達玩家單位上限: {max}")]
    MaxPlayerUnitsReached { max: usize },
    #[error("位置 ({x}, {y}) 沒有已部署的玩家單位可以取消")]
    NothingToUndeploy { x: Coord, y: Coord },
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
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n{}", self.kind, self.backtrace)
    }
}

impl<E: Into<ErrorKind>> From<E> for Error {
    fn from(error: E) -> Self {
        Self {
            kind: error.into(),
            backtrace: Backtrace::force_capture(),
        }
    }
}
