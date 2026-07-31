//! Error 到 Godot 的橋接。
//!
//! 做法見 docs/gdext.md 第 7 節：`#[func]` 回傳 `Result<T, BoardError>`，
//! 透過 ErrorToGodot 轉成 `BoardErrorInfo`（帶 code 與 detail）供 GDScript 判別。
//! code 對到 ErrorKind 的 variant 層級，讓 GDScript 能區分相近錯誤。

use board::error::{
    BoardError as BoardErrorKind, DataError, DeploymentError, Error, ErrorKind, LoadError,
    ReactionError, UnitError,
};
use godot::meta::error::{CallOutcome, ErrorToGodot};
use godot::prelude::*;

// 分段配號：每個大類佔一個百位區間，新增 variant 往區間尾端加，不重排既有值。
const CODE_LOAD_PARSE: i64 = 100;
const CODE_LOAD_DESERIALIZE: i64 = 101;
const CODE_LOAD_SERIALIZE: i64 = 102;

const CODE_DATA_INTERNAL: i64 = 200;
const CODE_DATA_MISSING_RESOURCE: i64 = 201;
const CODE_DATA_RESOURCE_ALREADY_EXISTS: i64 = 202;
const CODE_DATA_MISSING_COMPONENT: i64 = 203;
const CODE_DATA_INVALID_COMPONENT: i64 = 204;
const CODE_DATA_ID_GENERATION_FAILED: i64 = 205;
const CODE_DATA_UNIT_TYPE_NOT_FOUND: i64 = 206;
const CODE_DATA_OBJECT_TYPE_NOT_FOUND: i64 = 207;

const CODE_BOARD_OUT_OF_BOARD: i64 = 300;
const CODE_BOARD_UNREACHABLE: i64 = 301;
const CODE_BOARD_NO_ACTIVE_UNIT: i64 = 302;
const CODE_BOARD_OCCUPANT_NOT_FOUND: i64 = 303;
const CODE_BOARD_INVALID_DELAY: i64 = 304;
const CODE_BOARD_INVALID_SKILL_TARGET: i64 = 305;
const CODE_BOARD_WRONG_TARGET_COUNT: i64 = 306;
const CODE_BOARD_OUT_OF_RANGE: i64 = 307;
const CODE_BOARD_NO_LINE_OF_SIGHT: i64 = 308;
const CODE_BOARD_TARGET_FILTER_MISMATCH: i64 = 309;
const CODE_BOARD_NO_UNIT_AT_TARGET: i64 = 310;
const CODE_BOARD_DUPLICATE_TARGET: i64 = 311;
const CODE_BOARD_TARGET_COUNT_FULL: i64 = 312;

const CODE_DEPLOYMENT_POSITION_NOT_DEPLOYABLE: i64 = 400;
const CODE_DEPLOYMENT_MAX_PLAYER_UNITS_REACHED: i64 = 401;
const CODE_DEPLOYMENT_NOTHING_TO_UNDEPLOY: i64 = 402;

const CODE_UNIT_SKILL_NOT_FOUND: i64 = 500;
const CODE_UNIT_INSUFFICIENT_ACTION_POINT: i64 = 501;
const CODE_UNIT_INSUFFICIENT_MP: i64 = 502;
const CODE_UNIT_INSUFFICIENT_REACTION_POINT: i64 = 503;
const CODE_UNIT_EMPTY_SKILL_EFFECTS: i64 = 504;

const CODE_REACTION_NO_PENDING_REACTIONS: i64 = 600;
const CODE_REACTION_REACTOR_NOT_FOUND: i64 = 601;

/// 供 GDScript 讀取的錯誤物件。
#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub struct BoardErrorInfo {
    /// 對應 ErrorKind 的 variant，GDScript 用來決定行為。
    #[var]
    code: i64,
    /// 完整錯誤字串（含 backtrace），只給 debug 看。
    #[var]
    detail: GString,
}

/// newtype：ErrorToGodot 與 Error 都不在本 crate，孤兒規則要求本地型別。
pub struct BoardError(pub Error);

// 有了這個 From，`#[func]` 內可直接對 board 函數用 `?`，不必逐處 map_err。
impl From<Error> for BoardError {
    fn from(error: Error) -> Self {
        Self(error)
    }
}

impl<T: ToGodot> ErrorToGodot<T> for BoardError {
    type Mapped = Variant;

    fn result_to_godot(result: Result<T, Self>) -> CallOutcome<Variant> {
        match result {
            Ok(value) => CallOutcome::Return(value.to_variant()),
            Err(BoardError(err)) => {
                let info = BoardErrorInfo {
                    code: error_code(err.kind()),
                    detail: GString::from(&format!("{err}")),
                };
                CallOutcome::Return(Gd::from_object(info).to_variant())
            }
        }
    }
}

// 按大類拆成 6 個子函數，對應 ErrorKind 的兩層結構，避免嵌套模式降低可讀性。
fn error_code(kind: &ErrorKind) -> i64 {
    match kind {
        ErrorKind::Load(err) => load_code(err),
        ErrorKind::Data(err) => data_code(err),
        ErrorKind::Board(err) => board_code(err),
        ErrorKind::Deployment(err) => deployment_code(err),
        ErrorKind::Unit(err) => unit_code(err),
        ErrorKind::Reaction(err) => reaction_code(err),
    }
}

fn load_code(err: &LoadError) -> i64 {
    match err {
        LoadError::ParseError(_) => CODE_LOAD_PARSE,
        LoadError::DeserializeError { .. } => CODE_LOAD_DESERIALIZE,
        LoadError::SerializeError { .. } => CODE_LOAD_SERIALIZE,
    }
}

fn data_code(err: &DataError) -> i64 {
    match err {
        DataError::InternalError { .. } => CODE_DATA_INTERNAL,
        DataError::MissingResource { .. } => CODE_DATA_MISSING_RESOURCE,
        DataError::ResourceAlreadyExists { .. } => CODE_DATA_RESOURCE_ALREADY_EXISTS,
        DataError::MissingComponent { .. } => CODE_DATA_MISSING_COMPONENT,
        DataError::InvalidComponent { .. } => CODE_DATA_INVALID_COMPONENT,
        DataError::IDGenerationFailed => CODE_DATA_ID_GENERATION_FAILED,
        DataError::UnitTypeNotFound { .. } => CODE_DATA_UNIT_TYPE_NOT_FOUND,
        DataError::ObjectTypeNotFound { .. } => CODE_DATA_OBJECT_TYPE_NOT_FOUND,
    }
}

fn board_code(err: &BoardErrorKind) -> i64 {
    match err {
        BoardErrorKind::OutOfBoard { .. } => CODE_BOARD_OUT_OF_BOARD,
        BoardErrorKind::Unreachable { .. } => CODE_BOARD_UNREACHABLE,
        BoardErrorKind::NoActiveUnit => CODE_BOARD_NO_ACTIVE_UNIT,
        BoardErrorKind::OccupantNotFound { .. } => CODE_BOARD_OCCUPANT_NOT_FOUND,
        BoardErrorKind::InvalidDelay { .. } => CODE_BOARD_INVALID_DELAY,
        BoardErrorKind::InvalidSkillTarget { .. } => CODE_BOARD_INVALID_SKILL_TARGET,
        BoardErrorKind::WrongTargetCount { .. } => CODE_BOARD_WRONG_TARGET_COUNT,
        BoardErrorKind::OutOfRange { .. } => CODE_BOARD_OUT_OF_RANGE,
        BoardErrorKind::NoLineOfSight { .. } => CODE_BOARD_NO_LINE_OF_SIGHT,
        BoardErrorKind::TargetFilterMismatch { .. } => CODE_BOARD_TARGET_FILTER_MISMATCH,
        BoardErrorKind::NoUnitAtTarget { .. } => CODE_BOARD_NO_UNIT_AT_TARGET,
        BoardErrorKind::DuplicateTarget { .. } => CODE_BOARD_DUPLICATE_TARGET,
        BoardErrorKind::TargetCountFull { .. } => CODE_BOARD_TARGET_COUNT_FULL,
    }
}

fn deployment_code(err: &DeploymentError) -> i64 {
    match err {
        DeploymentError::PositionNotDeployable { .. } => CODE_DEPLOYMENT_POSITION_NOT_DEPLOYABLE,
        DeploymentError::MaxPlayerUnitsReached { .. } => CODE_DEPLOYMENT_MAX_PLAYER_UNITS_REACHED,
        DeploymentError::NothingToUndeploy { .. } => CODE_DEPLOYMENT_NOTHING_TO_UNDEPLOY,
    }
}

fn unit_code(err: &UnitError) -> i64 {
    match err {
        UnitError::SkillNotFound { .. } => CODE_UNIT_SKILL_NOT_FOUND,
        UnitError::InsufficientActionPoint { .. } => CODE_UNIT_INSUFFICIENT_ACTION_POINT,
        UnitError::InsufficientMp { .. } => CODE_UNIT_INSUFFICIENT_MP,
        UnitError::InsufficientReactionPoint { .. } => CODE_UNIT_INSUFFICIENT_REACTION_POINT,
        UnitError::EmptySkillEffects { .. } => CODE_UNIT_EMPTY_SKILL_EFFECTS,
    }
}

fn reaction_code(err: &ReactionError) -> i64 {
    match err {
        ReactionError::NoPendingReactions => CODE_REACTION_NO_PENDING_REACTIONS,
        ReactionError::ReactorNotFound { .. } => CODE_REACTION_REACTOR_NOT_FOUND,
    }
}
