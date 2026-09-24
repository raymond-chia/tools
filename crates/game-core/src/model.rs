//! 戰鬥資料格式、ECS 元件與輸出資料型別。
use bevy_ecs::prelude::{Component, Resource};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    Player,
    Enemy(String),
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Ongoing,
    Victory,
    Defeat,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RollDegree {
    CriticalFailure,
    Failure,
    Success,
    CriticalSuccess,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttackResult {
    Dodge,
    Block,
    Hit,
}
#[derive(Component)]
pub(crate) struct Id(pub(crate) String);
#[derive(Component, Clone, Copy)]
pub(crate) struct Pos(pub(crate) GridPos);
#[derive(Component, Clone, Copy)]
pub(crate) struct Footprint {
    pub(crate) width: i32,
    pub(crate) height: i32,
}
#[derive(Component)]
pub(crate) struct Hp {
    pub(crate) current: i32,
    pub(crate) maximum: i32,
}
#[derive(Component, Clone)]
pub(crate) struct Unit {
    pub(crate) name: String,
    pub(crate) visual: String,
    pub(crate) team: Team,
    pub(crate) movement: u32,
    pub(crate) initiative: i32,
    pub(crate) dodge: i32,
    pub(crate) block: i32,
    pub(crate) attack: i32,
    pub(crate) damage: i32,
    pub(crate) skills: Vec<String>,
}
#[derive(Component)]
pub(crate) struct Downed;
#[derive(Resource, Clone)]
pub(crate) struct Board {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) costs: Vec<u32>,
    pub(crate) triggers: HashMap<GridPos, String>,
    pub(crate) terrain_types: HashMap<String, TerrainTypeDef>,
}
#[derive(Clone)]
pub(crate) struct TemporaryTerrain {
    pub(crate) kind: String,
    pub(crate) expires_after_round: u32,
}
#[derive(Resource, Default, Clone)]
pub(crate) struct TemporaryTerrains(pub(crate) HashMap<GridPos, TemporaryTerrain>);
#[derive(Resource, Default, Clone)]
pub(crate) struct Encounter {
    pub(crate) participants: HashSet<String>,
    pub(crate) order: Vec<String>,
    pub(crate) cursor: usize,
    pub(crate) round: u32,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Phase {
    Ready,
    Moving,
    AfterMove,
    Ended,
}
#[derive(Resource, Clone)]
pub(crate) struct Turn {
    pub(crate) actor: Option<String>,
    pub(crate) phase: Phase,
    pub(crate) remaining: u32,
    pub(crate) moves: u8,
}
#[derive(Resource)]
pub(crate) struct Random(pub(crate) u64);
#[derive(Resource, Default)]
pub(crate) struct Log(pub(crate) Vec<CombatLogEvent>);
#[derive(Resource)]
pub(crate) struct ResultState(pub(crate) Outcome);
#[derive(Resource)]
pub(crate) struct Skills {
    pub(crate) definitions: HashMap<String, SkillDef>,
    pub(crate) ai_default: String,
}

#[derive(Deserialize)]
pub(crate) struct Definition {
    pub(crate) map: MapDef,
    pub(crate) terrain_types: HashMap<String, TerrainTypeDef>,
    pub(crate) skills: Vec<SkillDef>,
    pub(crate) units: Vec<UnitDef>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct SkillDef {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) ranged: bool,
    pub(crate) attack_bonus: i32,
    pub(crate) damage_bonus: i32,
    pub(crate) min_range: i32,
    pub(crate) max_range: i32,
    pub(crate) duration: Option<u32>,
    pub(crate) heal_amount: Option<i32>,
    pub(crate) terrain: Option<String>,
    #[serde(default)]
    pub(crate) effect: SkillEffect,
    #[serde(default)]
    pub(crate) ai_default: bool,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillEffect {
    #[default]
    Attack,
    Push,
    Mire,
    Heal,
}
#[derive(Deserialize)]
pub(crate) struct MapDef {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) costs: Vec<u32>,
    #[serde(default)]
    pub(crate) triggers: Vec<TriggerDef>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct TriggerDef {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) kind: String,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct TerrainTypeDef {
    pub(crate) name_key: String,
    pub(crate) visual: String,
    pub(crate) passable: bool,
    #[serde(default)]
    pub(crate) ends_movement: bool,
    #[serde(default)]
    pub(crate) damage: i32,
    #[serde(default)]
    pub(crate) movement_cost_bonus: u32,
    #[serde(default)]
    pub(crate) dodge_penalty: i32,
    #[serde(default)]
    pub(crate) block_penalty: i32,
    #[serde(default)]
    pub(crate) forced_entry: ForcedEntry,
    pub(crate) effect_key: String,
    pub(crate) forced_entry_log_key: Option<String>,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ForcedEntry {
    #[default]
    None,
    Blocked,
    Defeat,
}
#[derive(Deserialize)]
pub(crate) struct UnitDef {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) visual: String,
    pub(crate) team: Team,
    pub(crate) x: i32,
    pub(crate) y: i32,
    #[serde(default = "one")]
    pub(crate) width: i32,
    #[serde(default = "one")]
    pub(crate) height: i32,
    pub(crate) hp: i32,
    pub(crate) movement: u32,
    pub(crate) initiative: i32,
    pub(crate) dodge: i32,
    pub(crate) block: i32,
    pub(crate) attack: i32,
    pub(crate) damage: i32,
    #[serde(default)]
    pub(crate) skills: Vec<String>,
}
pub(crate) fn one() -> i32 {
    1
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Start,
    AutoStep,
    Move {
        actor: String,
        x: i32,
        y: i32,
    },
    Skill {
        actor: String,
        target: String,
        x: i32,
        y: i32,
        skill: String,
    },
    CellSkill {
        actor: String,
        x: i32,
        y: i32,
        skill: String,
    },
    EndTurn {
        actor: String,
    },
    Delay {
        actor: String,
        after: String,
    },
}
#[derive(Serialize)]
pub struct Snapshot {
    pub width: i32,
    pub height: i32,
    pub costs: Vec<u32>,
    pub terrain_effects: Vec<TerrainEffectView>,
    pub terrain_cells: Vec<TerrainCellView>,
    pub units: Vec<UnitView>,
    pub reachable: Vec<GridPos>,
    pub second_reachable: Vec<GridPos>,
    pub skill_ranges: Vec<SkillRangeView>,
    pub turn_order: Vec<String>,
    pub turn: TurnView,
    pub round: u32,
    pub outcome: Outcome,
    pub log: Vec<CombatLogEvent>,
    pub movements: Vec<MovementTransition>,
}
#[derive(Clone, Serialize)]
pub struct MovementTransition {
    pub unit_id: String,
    pub path: Vec<GridPos>,
    pub before_log_index: usize,
}
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CombatLogEvent {
    NewRound {
        round: u32,
        initiative_rolls: Vec<InitiativeRollLog>,
    },
    Skill {
        actor: String,
        actor_team: Team,
        skill: String,
        target: String,
        target_team: Team,
        roll: i32,
        die_sides: u32,
        attack_stat_modifier: i32,
        skill_attack_modifier: i32,
        flanking_modifier: i32,
        attack_modifier: i32,
        attack_total: i32,
        dodge_target: i32,
        block_target: i32,
        result: AttackResult,
        critical: bool,
        raw_damage: i32,
        damage_reduction: i32,
        damage: i32,
        remaining_hp: i32,
        max_hp: i32,
        downed: bool,
        pushed: bool,
        push_blocked: bool,
        push_distance: i32,
        collision_damage: i32,
        collision_units: Vec<CollisionUnitLog>,
    },
    TerrainCreated {
        actor: String,
        actor_team: Team,
        skill: String,
        terrain: String,
        terrain_name_key: String,
    },
    Healing {
        actor: String,
        actor_team: Team,
        skill: String,
        target: String,
        target_team: Team,
        healing: i32,
        remaining_hp: i32,
        max_hp: i32,
    },
    StatusApplied {
        target: String,
        target_team: Team,
        status: String,
        status_name_key: String,
    },
    TerrainDamage {
        target: String,
        target_team: Team,
        terrain: String,
        terrain_name_key: String,
        log_key: String,
        damage: i32,
        remaining_hp: i32,
        max_hp: i32,
        downed: bool,
    },
}
#[derive(Clone, Serialize)]
pub struct CollisionUnitLog {
    pub(crate) unit: String,
    pub(crate) team: Team,
    pub(crate) remaining_hp: i32,
    pub(crate) max_hp: i32,
    pub(crate) downed: bool,
}
#[derive(Clone, Serialize)]
pub struct InitiativeRollLog {
    pub unit: String,
    pub team: Team,
    pub roll: i32,
    pub die_sides: u32,
    pub modifier: i32,
    pub total: i32,
}
#[derive(Serialize)]
pub struct SkillRangeView {
    pub id: String,
    pub name_key: String,
    pub details: Vec<DetailView>,
    pub cell_targeted: bool,
    pub enabled: bool,
    pub cells: Vec<GridPos>,
}
#[derive(Serialize)]
pub struct DetailView {
    pub text_key: String,
    pub arguments: Vec<i32>,
}
#[derive(Serialize)]
pub struct MovePreview {
    pub first: Vec<GridPos>,
    pub second: Vec<GridPos>,
    pub interrupted: bool,
    pub total_cost: u32,
}
#[derive(Serialize)]
#[serde(untagged)]
pub enum SkillPreview {
    Attack(AttackPreview),
    Healing(HealingPreview),
}
#[derive(Serialize)]
pub struct HealingPreview {
    pub target: String,
    pub target_hp: i32,
    pub target_max_hp: i32,
    pub target_mana: i32,
    pub healing: i32,
    pub remaining_hp: i32,
    pub missing_hp: i32,
    pub health_segments: HealthSegmentsView,
}
#[derive(Serialize)]
pub struct HealthSegmentsView {
    pub hit: i32,
    pub block: i32,
    pub damage: i32,
    pub missing: i32,
}
#[derive(Serialize)]
pub struct AttackPreview {
    pub target: String,
    pub target_hp: i32,
    pub target_max_hp: i32,
    pub target_mana: i32,
    pub hit_remaining_hp: i32,
    pub block_remaining_hp: i32,
    pub dodge_remaining_hp: i32,
    pub dodge_chance: u32,
    pub block_chance: u32,
    pub hit_chance: u32,
    pub critical_chance: u32,
    pub dodge_damage: i32,
    pub block_damage: i32,
    pub critical_block_damage: i32,
    pub hit_damage: i32,
    pub critical_hit_damage: i32,
    pub health_segments: HealthSegmentsView,
}
#[derive(Serialize)]
pub struct UnitView {
    pub id: String,
    pub name: String,
    pub visual: String,
    pub team: Team,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub large: bool,
    pub occupied_cells: Vec<GridPos>,
    pub health_ratio: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub movement: u32,
    pub initiative: i32,
    pub dodge: i32,
    pub block: i32,
    pub attack: i32,
    pub damage: i32,
    pub downed: bool,
    pub active: bool,
}
#[derive(Serialize)]
pub struct TerrainEffectView {
    pub x: i32,
    pub y: i32,
    pub effect: String,
    pub visual: String,
    pub damage: i32,
    pub remaining_rounds: Option<u32>,
}
#[derive(Serialize)]
pub struct TerrainCellView {
    pub x: i32,
    pub y: i32,
    pub kind: String,
    pub name_key: String,
    pub visual: String,
    pub passable: bool,
    pub base_kind: String,
    pub unit_id: Option<String>,
    pub cost: u32,
    pub effect: String,
    pub damage: i32,
    pub remaining_rounds: Option<u32>,
    pub effect_description: DetailView,
}
#[derive(Serialize)]
pub struct TurnView {
    pub actor: Option<String>,
    pub phase: String,
    pub auto_step: bool,
    pub move_remaining: u32,
    pub can_move: bool,
    pub can_skill: bool,
    pub can_end_turn: bool,
    pub can_delay: bool,
}
