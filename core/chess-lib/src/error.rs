// 棋盤邏輯錯誤型別，攜帶 function name 與 context，支援來源錯誤巢狀
use crate::*;
use skills_lib::*;
use thiserror::Error;

/// 棋盤核心錯誤型別
#[derive(Debug, Error)]
pub enum Error {
    #[error("`{func}`: 參數錯誤")]
    InvalidParameter { func: &'static str },

    #[error("`{func}`: 缺少單位模板 {template_type}")]
    MissingUnitTemplate {
        func: &'static str,
        template_type: UnitTemplateType,
    },

    #[error("`{func}`: 技能 {skill_id} 不存在")]
    SkillNotFound {
        func: &'static str,
        skill_id: SkillID,
    },

    #[error("`{func}`: 單位 {unit_id} 不在 {pos:?}")]
    UnitNotAtPos {
        func: &'static str,
        unit_id: UnitID,
        pos: Pos,
    },

    #[error("`{func}`: 無法找到行動中的單位")]
    NoActingUnit { func: &'static str, unit_id: UnitID },

    #[error("`{func}`: 位置 {pos:?} 已被佔用")]
    PosOccupied { func: &'static str, pos: Pos },

    #[error("`{func}`: 位置 {pos:?} 無單位")]
    NoUnitAtPos { func: &'static str, pos: Pos },

    #[error("`{func}`: 位置 {pos:?} 不存在")]
    NoTileAtPos { func: &'static str, pos: Pos },

    #[error("`{func}`: 位置 {pos:?} 有敵方單位")]
    HostileUnitAtPos { func: &'static str, pos: Pos },

    #[error("`{func}`: 位置 {pos:?} 有友方單位")]
    AlliedUnitAtPos { func: &'static str, pos: Pos },

    #[error("`{func}`: 行動點數不足")]
    NotEnoughPoints { func: &'static str },

    #[error("`{func}`: 目標 {pos:?} 不可到達")]
    NotReachable { func: &'static str, pos: Pos },

    #[error("`{func}`: 未選擇技能")]
    NoSkillSelected { func: &'static str },

    #[error("`{func}`: 技能 {skill_id} 設定錯誤")]
    InvalidSkill {
        func: &'static str,
        skill_id: SkillID,
    },

    #[error("`{func}`: 技能 {skill_id} 無法作用於 {pos:?}，目標格必須有單位")]
    SkillTargetNoUnit {
        func: &'static str,
        skill_id: SkillID,
        pos: Pos,
    },

    #[error("`{func}`: 技能 {skill_id} 無法作用於 {pos:?}")]
    SkillAffectEmpty {
        func: &'static str,
        skill_id: SkillID,
        pos: Pos,
    },

    #[error("`{func}`: 包裝: {source}")]
    Wrap {
        func: &'static str,
        #[source]
        source: Box<Error>,
    },
}

pub fn root_error(err: &Error) -> &Error {
    let mut err = err;
    while let Error::Wrap { source, .. } = err {
        err = source.as_ref();
    }
    err
}
