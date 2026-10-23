//! Godot 無關的權威戰棋核心。
//! 資料型別見 model；錯誤 ID 與描述見 error；載入、回合與快照見 game；移動與空間規則見 movement；技能與戰鬥規則見 skill。

mod ai;
pub mod authoring;
mod cargo;
#[cfg(feature = "editor")]
pub mod editor;
mod equipment;
mod error;
mod game;
mod gameplay_config;
mod model;
mod movement;
mod sight;
mod skill;
mod teams;
mod terrain;
#[cfg(test)]
extern crate self as game_core;
#[cfg(test)]
mod tests;

pub use error::GameError;
pub use game::Game;
pub use model::{
    AttackPreview, AttackResult, BattleMode, CargoState, CargoView, CollisionUnitLog,
    CombatLogEvent, Command, EquipmentView, GridPos, HealingPreview, HealthSegmentsView,
    InitiativeRollLog, MovePreview, MovementTransition, Outcome, PassiveSkillView, Phase,
    PowerSource, PresentationEvent, RollDegree, SkillDef, SkillDetailEffect, SkillDetailsView,
    SkillEffect, SkillPreview, SkillRangeView, SkillTargetKind, Snapshot, Team, TerrainCellView,
    TerrainDescriptionView, TerrainEffectView, TerrainEntryRule, TerrainLayer, TerrainPlacement,
    TerrainTypeDef, TurnView, UnitView,
};
pub use skill::degree;
