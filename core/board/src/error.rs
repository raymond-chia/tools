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
    #[error(transparent)]
    Reaction(#[from] ReactionError),
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
    #[error("ID 生成失敗")]
    IDGenerationFailed,
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
    #[error("佔據者不存在於回合表中: {occupant:?}")]
    OccupantNotFound { occupant: Occupant },
    #[error("單位 {occupant:?} 無法延後: {reason}")]
    InvalidDelay { occupant: Occupant, reason: String },
    // skill
    #[error(
        "非法的技能施放位置:\n\t範圍 {shape}\n\t施放者 ({caster_x}, {caster_y})\n\t目標 ({target_x}, {target_y})"
    )]
    InvalidSkillTarget {
        shape: String,
        caster_x: Coord,
        caster_y: Coord,
        target_x: Coord,
        target_y: Coord,
    },
    #[error("目標數量錯誤: 預期 {expected} 個，實際 {actual} 個")]
    WrongTargetCount { expected: usize, actual: usize },
    #[error("目標超出射程: 距離 {distance}，射程 {min_range}-{max_range}")]
    OutOfRange {
        distance: usize,
        min_range: Coord,
        max_range: Coord,
    },
    #[error("目標位置 ({x}, {y}) 無視線")]
    NoLineOfSight { x: Coord, y: Coord },
    #[error("目標位置 ({x}, {y}) 不符合篩選條件: {filter}")]
    TargetFilterMismatch { x: Coord, y: Coord, filter: String },
    #[error("目標位置 ({x}, {y}) 無單位")]
    NoUnitAtTarget { x: Coord, y: Coord },
    #[error("不允許重複選擇目標位置，所有目標：{targets:?}")]
    DuplicateTarget { targets: Vec<(Coord, Coord)> },
    #[error("已達技能目標數量上限: {max}")]
    TargetCountFull { max: usize },
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

/// 反應系統錯誤
#[derive(Debug, ThisError)]
pub enum ReactionError {
    #[error("沒有待處理的反應，請先呼叫 execute_move 或 set_reactions")]
    NoPendingReactions,
    #[error("佔據者 {occupant:?} 不在待反應清單中")]
    ReactorNotFound { occupant: Occupant },
}

/// 單位相關錯誤
#[derive(Debug, ThisError)]
pub enum UnitError {
    #[error("技能未找到: {skill_name}")]
    SkillNotFound { skill_name: SkillName },
    #[error("行動點不足: 已消耗 {used}，上限 {max}")]
    InsufficientActionPoint { used: i32, max: i32 },
    #[error("MP 不足: 需要 {cost}，目前 {current}")]
    InsufficientMp { cost: u32, current: i32 },
    #[error("技能 '{skill_name}' 必須至少有一個 effect")]
    EmptySkillEffects { skill_name: SkillName },
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
