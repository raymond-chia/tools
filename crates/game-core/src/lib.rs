//! Godot 無關的權威戰棋核心。
//! 資料型別見 model；錯誤 ID 與描述見 error；載入、回合與快照見 game；移動與空間規則見 movement；技能與戰鬥規則見 skill。

pub mod authoring;
mod error;
mod game;
mod gameplay_config;
mod model;
mod movement;
mod skill;
#[cfg(test)]
mod tests;

#[cfg(test)]
use bevy_ecs::prelude::{Entity, World};
pub use error::GameError;
pub use game::Game;
pub use model::{
    AttackPreview, AttackResult, CollisionUnitLog, CombatLogEvent, Command, DetailView,
    ForcedEntry, GridPos, HealingPreview, HealthSegmentsView, InitiativeRollLog, MovePreview,
    MovementTransition, Outcome, RollDegree, SkillDef, SkillEffect, SkillPreview, SkillRangeView,
    Snapshot, Team, TerrainCellView, TerrainEffectView, TerrainPlacement, TerrainTypeDef, TurnView,
    UnitView,
};
#[cfg(test)]
use model::{Board, Footprint, Hp, Id, Log, Phase, Pos, Skills, TemporaryTerrains, Turn, Unit};
use model::{Definition, MapDef, UnitDef};
pub use skill::degree;
#[cfg(test)]
use skill::{attack_damage, attack_modifier};
#[cfg(test)]
use std::collections::HashMap;
