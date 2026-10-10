//! Godot 無關的權威戰棋核心。
//! 資料型別見 model；錯誤 ID 與描述見 error；載入、回合與快照見 game；移動與空間規則見 movement；技能與戰鬥規則見 skill。

mod ai;
pub mod authoring;
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
mod terrain;
#[cfg(test)]
mod tests;

#[cfg(test)]
use bevy_ecs::prelude::{Entity, World};
pub use error::GameError;
pub use game::Game;
pub use model::{
    AttackPreview, AttackResult, BattleMode, CollisionUnitLog, CombatLogEvent, Command,
    EquipmentView, GridPos, HealingPreview, HealthSegmentsView, InitiativeRollLog, MovePreview,
    MovementTransition, Outcome, PassiveSkillView, Phase, PowerSource, RollDegree, SkillDef,
    SkillDetailEffect, SkillDetailsView, SkillEffect, SkillPreview, SkillRangeView,
    SkillTargetKind, Snapshot, Team, TerrainCellView, TerrainDescriptionView, TerrainEffectView,
    TerrainEntryRule, TerrainLayer, TerrainPlacement, TerrainTypeDef, TurnView, UnitView,
};
#[cfg(test)]
use model::{Board, Footprint, Hp, Id, Log, Pos, Skills, TemporaryTerrains, Turn, Unit};
pub use skill::degree;
#[cfg(test)]
use skill::{attack_damage, attack_modifier};
#[cfg(test)]
use std::collections::HashMap;
