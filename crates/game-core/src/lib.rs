//! Godot 無關的權威戰棋核心。
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

pub mod authoring;
mod gameplay_config;
#[cfg(test)]
mod tests;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    Player,
    Enemy,
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
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttackStat {
    Melee,
    Ranged,
}
#[derive(Component)]
struct Id(String);
#[derive(Component, Clone, Copy)]
struct Pos(GridPos);
#[derive(Component, Clone, Copy)]
struct Footprint {
    width: i32,
    height: i32,
}
#[derive(Component)]
struct Hp {
    current: i32,
    maximum: i32,
}
#[derive(Component, Clone)]
struct Unit {
    name: String,
    visual: String,
    team: Team,
    group: String,
    movement: u32,
    initiative: i32,
    dodge: i32,
    block: i32,
    melee: i32,
    ranged: i32,
    damage: i32,
    range: i32,
    skills: Vec<String>,
}
#[derive(Component)]
struct Downed;
#[derive(Resource, Clone)]
struct Board {
    width: i32,
    height: i32,
    costs: Vec<u32>,
    triggers: HashMap<GridPos, String>,
    terrain_types: HashMap<String, TerrainTypeDef>,
}
#[derive(Clone)]
struct TemporaryTerrain {
    kind: String,
    expires_after_round: u32,
}
#[derive(Resource, Default, Clone)]
struct TemporaryTerrains(HashMap<GridPos, TemporaryTerrain>);
#[derive(Resource, Default, Clone)]
struct Encounter {
    participants: HashSet<String>,
    order: Vec<String>,
    cursor: usize,
    round: u32,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Ready,
    Moving,
    AfterMove,
    Ended,
}
#[derive(Resource, Clone)]
struct Turn {
    actor: Option<String>,
    phase: Phase,
    remaining: u32,
    moves: u8,
}
#[derive(Resource)]
struct Random(u64);
#[derive(Resource, Default)]
struct Log(Vec<CombatLogEvent>);
#[derive(Resource)]
struct ResultState(Outcome);
#[derive(Resource)]
struct Skills {
    definitions: HashMap<String, SkillDef>,
    ai_default: String,
}

#[derive(Deserialize)]
struct Definition {
    map: MapDef,
    terrain_types: HashMap<String, TerrainTypeDef>,
    skills: Vec<SkillDef>,
    units: Vec<UnitDef>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct SkillDef {
    id: String,
    name: String,
    ranged: bool,
    attack_bonus: i32,
    damage_bonus: i32,
    range: Option<i32>,
    duration: Option<u32>,
    heal_amount: Option<i32>,
    terrain: Option<String>,
    #[serde(default)]
    effect: SkillEffect,
    #[serde(default)]
    ai_default: bool,
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
struct MapDef {
    width: i32,
    height: i32,
    costs: Vec<u32>,
    #[serde(default)]
    triggers: Vec<TriggerDef>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct TriggerDef {
    x: i32,
    y: i32,
    kind: String,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct TerrainTypeDef {
    name_key: String,
    visual: String,
    passable: bool,
    #[serde(default)]
    ends_movement: bool,
    #[serde(default)]
    damage: i32,
    #[serde(default)]
    movement_cost_bonus: u32,
    #[serde(default)]
    dodge_penalty: i32,
    #[serde(default)]
    block_penalty: i32,
    #[serde(default)]
    forced_entry: ForcedEntry,
    effect_key: String,
    forced_entry_log_key: Option<String>,
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
struct UnitDef {
    id: String,
    name: String,
    visual: String,
    team: Team,
    group: String,
    x: i32,
    y: i32,
    #[serde(default = "one")]
    width: i32,
    #[serde(default = "one")]
    height: i32,
    hp: i32,
    movement: u32,
    initiative: i32,
    dodge: i32,
    block: i32,
    melee: i32,
    ranged: i32,
    damage: i32,
    range: i32,
    #[serde(default)]
    skills: Vec<String>,
}
fn one() -> i32 {
    1
}

fn terrain_type<'a>(board: &'a Board, kind: &str) -> &'a TerrainTypeDef {
    board
        .terrain_types
        .get(kind)
        .expect("載入時已驗證所有地形種類")
}

fn terrain_damage(board: &Board, kind: &str) -> i32 {
    terrain_type(board, kind).damage
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Start,
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
        attack_stat: AttackStat,
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
    unit: String,
    team: Team,
    remaining_hp: i32,
    max_hp: i32,
    downed: bool,
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
    pub group: String,
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
    pub melee: i32,
    pub ranged: i32,
    pub damage: i32,
    pub range: i32,
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
    pub move_remaining: u32,
    pub can_move: bool,
    pub can_skill: bool,
    pub can_end_turn: bool,
    pub can_delay: bool,
}

pub struct Game {
    world: World,
    movements: Vec<MovementTransition>,
}
struct MovePlan {
    entity: Entity,
    turn: Turn,
    first_budget: u32,
    second_budget: u32,
    path: Vec<GridPos>,
}
impl Game {
    pub fn from_toml(s: &str) -> Result<Self, String> {
        let d: Definition = toml::from_str(s).map_err(|e| e.to_string())?;
        Self::from_definition(d)
    }
    pub fn from_documents(definitions: &str, map: &str) -> Result<Self, String> {
        let definitions: authoring::Definitions =
            toml::from_str(definitions).map_err(|e| e.to_string())?;
        let map: authoring::Map = toml::from_str(map).map_err(|e| e.to_string())?;
        Self::from_authoring(definitions, map)
    }
    pub fn from_authoring(
        definitions: authoring::Definitions,
        map: authoring::Map,
    ) -> Result<Self, String> {
        Self::from_definition(authoring::into_definition(definitions, map)?)
    }
    fn from_definition(d: Definition) -> Result<Self, String> {
        if d.map.width <= 0
            || d.map.height <= 0
            || d.map
                .width
                .checked_mul(d.map.height)
                .is_none_or(|cells| d.map.costs.len() != cells as usize)
        {
            return Err("map.costs 數量與尺寸不符".into());
        }
        if d.map.costs.contains(&0) {
            return Err("movement cost 必須大於 0".into());
        }
        if d.map.triggers.iter().any(|trigger| {
            trigger.x < 0 || trigger.y < 0 || trigger.x >= d.map.width || trigger.y >= d.map.height
        }) {
            return Err("map trigger 超出地圖範圍".into());
        }
        for required in ["plain", "rough"] {
            if !d.terrain_types.contains_key(required) {
                return Err(format!("缺少必要地形種類 {required}"));
            }
        }
        for trigger in &d.map.triggers {
            if !d.terrain_types.contains_key(&trigger.kind) {
                return Err(format!("找不到地形種類 {}", trigger.kind));
            }
        }
        let mut terrain_positions = HashSet::new();
        for trigger in &d.map.triggers {
            if !terrain_positions.insert((trigger.x, trigger.y)) {
                return Err(format!("地形格子重複 ({}, {})", trigger.x, trigger.y));
            }
        }
        let mut w = World::new();
        let triggers = d
            .map
            .triggers
            .into_iter()
            .map(|t| (GridPos { x: t.x, y: t.y }, t.kind))
            .collect();
        w.insert_resource(Board {
            width: d.map.width,
            height: d.map.height,
            costs: d.map.costs,
            triggers,
            terrain_types: d.terrain_types,
        });
        w.insert_resource(Encounter::default());
        w.insert_resource(TemporaryTerrains::default());
        w.insert_resource(Turn {
            actor: None,
            phase: Phase::Ended,
            remaining: 0,
            moves: 0,
        });
        w.insert_resource(Random(0xc0ffee));
        w.insert_resource(Log::default());
        w.insert_resource(ResultState(Outcome::Ongoing));
        let mut skills = HashMap::new();
        let mut ai_default = None;
        for skill in d.skills {
            if skill.effect == SkillEffect::Heal
                && !matches!(skill.heal_amount, Some(amount) if amount > 0)
            {
                return Err(format!("{} 的 heal_amount 必須大於 0", skill.id));
            }
            if skill.duration == Some(0) {
                return Err(format!("{} 的 duration 必須大於 0", skill.id));
            }
            if skill.effect == SkillEffect::Mire
                && !skill.terrain.as_ref().is_some_and(|terrain| {
                    w.resource::<Board>().terrain_types.contains_key(terrain)
                })
            {
                return Err(format!("{} 必須指定已定義的 terrain", skill.id));
            }
            if skill.ai_default {
                if ai_default.replace(skill.id.clone()).is_some() {
                    return Err("只能有一個 ai_default 技能".into());
                }
            }
            if skills.insert(skill.id.clone(), skill).is_some() {
                return Err("duplicate skill id".into());
            }
        }
        let ai_default = ai_default.ok_or("缺少 ai_default 技能")?;
        w.insert_resource(Skills {
            definitions: skills,
            ai_default,
        });
        let all_skill_ids: Vec<_> = w.resource::<Skills>().definitions.keys().cloned().collect();
        let mut ids = HashSet::new();
        let mut occupied = HashSet::new();
        for u in d.units {
            if !ids.insert(u.id.clone()) {
                return Err(format!("重複 id {}", u.id));
            }
            let f = Footprint {
                width: u.width,
                height: u.height,
            };
            if f.width <= 0 || f.height <= 0 || u.hp <= 0 {
                return Err(format!("{} 的佔用尺寸與 HP 必須大於 0", u.id));
            }
            let p = GridPos { x: u.x, y: u.y };
            if !fits(w.resource::<Board>(), p, f) {
                return Err(format!("{} 超出地圖", u.id));
            }
            if footprint_on_impassable(w.resource::<Board>(), p, f) {
                return Err(format!("{} 不可放置在峭壁或懸崖", u.id));
            }
            if footprint_cells(p, f)
                .iter()
                .any(|cell| !occupied.insert(*cell))
            {
                return Err(format!("{} 與其他單位重疊", u.id));
            }
            if let Some(unknown) = u
                .skills
                .iter()
                .find(|skill| !w.resource::<Skills>().definitions.contains_key(*skill))
            {
                return Err(format!("{} 使用未知技能 {}", u.id, unknown));
            }
            w.spawn((
                Id(u.id),
                Pos(p),
                f,
                Hp {
                    current: u.hp,
                    maximum: u.hp,
                },
                Unit {
                    name: u.name,
                    visual: u.visual,
                    team: u.team,
                    group: u.group,
                    movement: u.movement,
                    initiative: u.initiative,
                    dodge: u.dodge,
                    block: u.block,
                    melee: u.melee,
                    ranged: u.ranged,
                    damage: u.damage,
                    range: u.range,
                    skills: if u.skills.is_empty() {
                        all_skill_ids.clone()
                    } else {
                        u.skills
                    },
                },
            ));
        }
        Ok(Self {
            world: w,
            movements: Vec::new(),
        })
    }
    pub fn command(&mut self, c: Command) -> Result<Snapshot, String> {
        self.movements.clear();
        if self.world.resource::<ResultState>().0 != Outcome::Ongoing {
            return Ok(self.snapshot());
        }
        match c {
            Command::Start => self.start(),
            Command::Move { actor, x, y } => self.move_to(&actor, GridPos { x, y }),
            Command::Skill {
                actor,
                target,
                x,
                y,
                skill,
            } => {
                let definition = self
                    .world
                    .resource::<Skills>()
                    .definitions
                    .get(&skill)
                    .cloned()
                    .ok_or_else(|| format!("unknown skill: {skill}"))?;
                self.use_skill(&actor, &target, GridPos { x, y }, definition)
            }
            Command::CellSkill { actor, x, y, skill } => {
                let definition = self
                    .world
                    .resource::<Skills>()
                    .definitions
                    .get(&skill)
                    .cloned()
                    .ok_or_else(|| format!("unknown skill: {skill}"))?;
                self.use_cell_skill(&actor, GridPos { x, y }, definition)
            }
            Command::EndTurn { actor } => {
                self.ensure(&actor)?;
                self.finish();
                Ok(())
            }
            Command::Delay { actor, after } => self.delay(&actor, &after),
        }?;
        self.enemy_turns()?;
        self.outcome();
        Ok(self.snapshot())
    }
    pub fn set_random_seed(&mut self, seed: u64) {
        self.world.resource_mut::<Random>().0 = seed;
    }
    pub fn preview_move(&self, actor: &str, end: GridPos) -> Result<MovePreview, String> {
        let MovePlan {
            entity: _,
            turn: _,
            first_budget,
            second_budget: _,
            path,
        } = self.move_plan(actor, end)?;
        let interrupted = path
            .last()
            .is_some_and(|position| terrain_ends_movement(&self.world, *position));
        let mut first = Vec::new();
        let mut second = Vec::new();
        let mut spent = 0;
        if first_budget == 0 {
            second.push(path[0]);
        } else {
            first.push(path[0]);
        }
        for position in path.into_iter().skip(1) {
            spent += movement_cost(&self.world, position);
            if spent <= first_budget {
                first.push(position);
            } else {
                if second.is_empty() {
                    second.push(
                        *first
                            .last()
                            .expect("第二段移動開始前，第一段路徑應包含起點"),
                    );
                }
                second.push(position);
            }
        }
        Ok(MovePreview {
            first,
            second,
            interrupted,
            total_cost: spent,
        })
    }
    pub fn preview_skill(
        &self,
        actor: &str,
        target: &str,
        target_cell: GridPos,
        skill_id: &str,
    ) -> Result<SkillPreview, String> {
        self.ensure(actor)?;
        if !can_use_skill(self.world.resource::<Turn>()) {
            return Err("目前不能使用 Skill".into());
        }
        let attacker = self.entity(actor).ok_or("找不到攻擊者")?;
        let target_entity = self.entity(target).ok_or("找不到目標")?;
        let skill = self
            .world
            .resource::<Skills>()
            .definitions
            .get(skill_id)
            .ok_or_else(|| format!("unknown skill: {skill_id}"))?;
        validate_unit_skill_target(&self.world, attacker, target_entity, target_cell, skill)?;
        if skill.effect == SkillEffect::Heal {
            return Ok(SkillPreview::Healing(healing_preview(
                &self.world,
                target_entity,
                skill,
            )));
        }

        let attacker_unit = self
            .world
            .get::<Unit>(attacker)
            .expect("已建立的戰鬥單位應具有 Unit 元件");
        let target_unit = self
            .world
            .get::<Unit>(target_entity)
            .expect("已建立的戰鬥單位應具有 Unit 元件");
        let target_hp = self
            .world
            .get::<Hp>(target_entity)
            .expect("已建立的戰鬥單位應具有 Hp 元件");
        let modifier = attack_modifier(&self.world, attacker, target_entity, skill);
        let dodge_target =
            gameplay_config::BASE_DEFENSE + effective_dodge(&self.world, target_entity);
        let block_target = dodge_target + effective_block(&self.world, target_entity);
        let mut dodge_count = 0;
        let mut block_count = 0;
        let mut hit_count = 0;
        for natural in 1..=gameplay_config::ATTACK_DIE_SIDES as i32 {
            match attack_result(natural, modifier, dodge_target, block_target) {
                AttackResult::Dodge => dodge_count += 1,
                AttackResult::Block => block_count += 1,
                AttackResult::Hit => hit_count += 1,
            }
        }
        let hit_damage = attacker_unit.damage + skill.damage_bonus;
        let block_damage = attack_damage(
            hit_damage,
            AttackResult::Block,
            gameplay_config::BLOCK_DAMAGE_REDUCTION,
            false,
        );
        let hit_remaining_hp = (target_hp.current - hit_damage).max(0);
        let block_remaining_hp = (target_hp.current - block_damage).max(0);
        let block_segment = if block_count > 0 {
            block_remaining_hp - hit_remaining_hp
        } else {
            0
        };
        Ok(SkillPreview::Attack(AttackPreview {
            target: target_unit.name.clone(),
            target_hp: target_hp.current,
            target_max_hp: target_hp.maximum,
            target_mana: gameplay_config::DEFAULT_MANA,
            hit_remaining_hp,
            block_remaining_hp,
            dodge_remaining_hp: target_hp.current,
            dodge_chance: dodge_count * 100 / gameplay_config::ATTACK_DIE_SIDES,
            block_chance: block_count * 100 / gameplay_config::ATTACK_DIE_SIDES,
            hit_chance: hit_count * 100 / gameplay_config::ATTACK_DIE_SIDES,
            critical_chance: 100 / gameplay_config::ATTACK_DIE_SIDES,
            dodge_damage: 0,
            block_damage,
            critical_block_damage: attack_damage(
                hit_damage,
                AttackResult::Block,
                gameplay_config::BLOCK_DAMAGE_REDUCTION,
                true,
            ),
            hit_damage,
            critical_hit_damage: attack_damage(
                hit_damage,
                AttackResult::Hit,
                gameplay_config::BLOCK_DAMAGE_REDUCTION,
                true,
            ),
            health_segments: HealthSegmentsView {
                hit: hit_remaining_hp,
                block: block_segment,
                damage: target_hp.current - hit_remaining_hp - block_segment,
                missing: target_hp.maximum - target_hp.current,
            },
        }))
    }
    fn start(&mut self) -> Result<(), String> {
        if self.world.resource::<Encounter>().round > 0 {
            return Ok(());
        }
        let ids: Vec<_> = self
            .world
            .query::<&Id>()
            .iter(&self.world)
            .map(|i| i.0.clone())
            .collect();
        self.world
            .resource_mut::<Encounter>()
            .participants
            .extend(ids);
        self.roll_round();
        Ok(())
    }
    fn roll_round(&mut self) {
        let next_round = self.world.resource::<Encounter>().round + 1;
        self.world
            .resource_mut::<TemporaryTerrains>()
            .0
            .retain(|_, terrain| terrain.expires_after_round >= next_round);
        let active = self.world.resource::<Encounter>().participants.clone();
        let entries: Vec<_> = self
            .world
            .query::<(&Id, &Unit, Has<Downed>)>()
            .iter(&self.world)
            .filter(|(i, _, d)| active.contains(&i.0) && !*d)
            .map(|(i, f, _)| (i.0.clone(), f.name.clone(), f.team, f.initiative))
            .collect();
        let mut rolled: Vec<_> = entries
            .into_iter()
            .map(|(id, name, team, modifier)| {
                let roll = die(&mut self.world, gameplay_config::INITIATIVE_DIE_SIDES) as i32;
                (roll + modifier, id, name, team, roll, modifier)
            })
            .collect();
        rolled.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        let initiative_rolls = rolled
            .iter()
            .map(
                |(total, _id, unit, team, roll, modifier)| InitiativeRollLog {
                    unit: unit.clone(),
                    team: *team,
                    roll: *roll,
                    die_sides: gameplay_config::INITIATIVE_DIE_SIDES,
                    modifier: *modifier,
                    total: *total,
                },
            )
            .collect();
        {
            let mut e = self.world.resource_mut::<Encounter>();
            e.round += 1;
            e.order = rolled.into_iter().map(|(_, id, _, _, _, _)| id).collect();
            e.cursor = 0;
        }
        let round = self.world.resource::<Encounter>().round;
        self.world
            .resource_mut::<Log>()
            .0
            .push(CombatLogEvent::NewRound {
                round,
                initiative_rolls,
            });
        self.begin()
    }
    fn begin(&mut self) {
        let actor = self
            .world
            .resource::<Encounter>()
            .order
            .get(self.world.resource::<Encounter>().cursor)
            .cloned();
        let remaining = actor
            .as_ref()
            .and_then(|a| self.entity(a))
            .map(|e| {
                self.world
                    .get::<Unit>(e)
                    .expect("已建立的戰鬥單位應具有 Unit 元件")
                    .movement
            })
            .unwrap_or(0);
        *self.world.resource_mut::<Turn>() = Turn {
            actor,
            phase: Phase::Ready,
            remaining,
            moves: 0,
        }
    }
    fn finish(&mut self) {
        self.world.resource_mut::<Turn>().phase = Phase::Ended;
        let end = {
            let mut e = self.world.resource_mut::<Encounter>();
            e.cursor += 1;
            e.cursor >= e.order.len()
        };
        if end { self.roll_round() } else { self.begin() }
    }
    fn delay(&mut self, actor: &str, after: &str) -> Result<(), String> {
        self.ensure(actor)?;
        let turn = self.world.resource::<Turn>();
        if turn.phase != Phase::Ready || turn.moves != 0 {
            return Err("開始行動後不能延後".into());
        }
        let encounter = self.world.resource::<Encounter>();
        let target_index = encounter
            .order
            .iter()
            .position(|unit_id| unit_id == after)
            .ok_or("找不到延後目標")?;
        if target_index <= encounter.cursor {
            return Err("只能延後到尚未行動的單位之後".into());
        }
        let mut encounter = self.world.resource_mut::<Encounter>();
        let current_index = encounter.cursor;
        let delayed_actor = encounter.order.remove(current_index);
        encounter.order.insert(target_index, delayed_actor);
        drop(encounter);
        self.begin();
        Ok(())
    }
    fn ensure(&self, a: &str) -> Result<(), String> {
        if self.world.resource::<Turn>().actor.as_deref() != Some(a) {
            Err("不是該單位的回合".into())
        } else {
            Ok(())
        }
    }
    fn move_to(&mut self, a: &str, end: GridPos) -> Result<(), String> {
        let MovePlan {
            entity: e,
            turn,
            first_budget,
            second_budget,
            path,
        } = self.move_plan(a, end)?;
        let mut spent = 0;
        for p in path.into_iter().skip(1) {
            spent += movement_cost(&self.world, p);
            self.world
                .get_mut::<Pos>(e)
                .expect("已建立的戰鬥單位應具有 Pos 元件")
                .0 = p;
            self.reveal(e);
            if let Some(k) = terrain_at(&self.world, p) {
                if !terrain_ends_movement(&self.world, p) {
                    continue;
                }
                let unit = self
                    .world
                    .get::<Unit>(e)
                    .expect("已建立的戰鬥單位應具有 Unit 元件");
                let target = unit.name.clone();
                let target_team = unit.team;
                let terrain_definition = terrain_type(self.world.resource::<Board>(), &k);
                let terrain_name_key = terrain_definition.name_key.clone();
                let log_key = terrain_definition
                    .forced_entry_log_key
                    .clone()
                    .unwrap_or_else(|| "COMBAT_LOG_TERRAIN_DAMAGE".into());
                let damage = terrain_definition.damage;
                if damage > 0 {
                    let mut hp = self
                        .world
                        .get_mut::<Hp>(e)
                        .expect("已建立的戰鬥單位應具有 Hp 元件");
                    hp.current = (hp.current - damage).max(0);
                    let remaining_hp = hp.current;
                    let max_hp = hp.maximum;
                    let downed = remaining_hp == 0;
                    if downed {
                        self.world.entity_mut(e).insert(Downed);
                        self.world
                            .resource_mut::<Encounter>()
                            .participants
                            .remove(a);
                    }
                    self.world
                        .resource_mut::<Log>()
                        .0
                        .push(CombatLogEvent::TerrainDamage {
                            target,
                            target_team,
                            terrain: k,
                            terrain_name_key,
                            log_key,
                            damage,
                            remaining_hp,
                            max_hp,
                            downed,
                        });
                } else {
                    self.world
                        .resource_mut::<Log>()
                        .0
                        .push(CombatLogEvent::StatusApplied {
                            target,
                            target_team,
                            status: k,
                            status_name_key: terrain_name_key,
                        });
                }
                break;
            }
        }
        let mut t = self.world.resource_mut::<Turn>();
        if turn.moves == 0 && spent <= first_budget {
            t.remaining = first_budget - spent;
        } else if turn.moves == 0 {
            t.moves = 1;
            t.remaining = second_budget - (spent - first_budget);
        } else {
            t.remaining = second_budget - spent;
        }
        if t.remaining == 0 {
            t.moves += 1;
            t.phase = Phase::AfterMove;
        } else {
            t.phase = Phase::Moving;
        }
        Ok(())
    }
    fn move_plan(&self, actor: &str, end: GridPos) -> Result<MovePlan, String> {
        self.ensure(actor)?;
        let entity = self.entity(actor).ok_or("找不到移動單位")?;
        let allowance = self
            .world
            .get::<Unit>(entity)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .movement;
        let turn = self.world.resource::<Turn>().clone();
        if !matches!(turn.phase, Phase::Ready | Phase::Moving | Phase::AfterMove) || turn.moves >= 2
        {
            return Err("目前不能移動".into());
        }
        let start = self
            .world
            .get::<Pos>(entity)
            .expect("已建立的戰鬥單位應具有 Pos 元件")
            .0;
        let footprint = *self
            .world
            .get::<Footprint>(entity)
            .expect("已建立的戰鬥單位應具有 Footprint 元件");
        let first_budget = if turn.moves == 0 { turn.remaining } else { 0 };
        let second_budget = allowance;
        let first_path = if first_budget > 0 {
            find_path(&self.world, entity, start, end, footprint, first_budget)
        } else {
            None
        };
        let mut path = first_path
            .or_else(|| {
                find_path(
                    &self.world,
                    entity,
                    start,
                    end,
                    footprint,
                    first_budget + second_budget,
                )
            })
            .ok_or("目的地不可達")?;
        if let Some(trigger_index) = path
            .iter()
            .skip(1)
            .position(|position| terrain_ends_movement(&self.world, *position))
        {
            path.truncate(trigger_index + 2);
        }
        Ok(MovePlan {
            entity,
            turn,
            first_budget,
            second_budget,
            path,
        })
    }
    fn reveal(&mut self, mover: Entity) {
        let p = self
            .world
            .get::<Pos>(mover)
            .expect("已建立的戰鬥單位應具有 Pos 元件")
            .0;
        let active = self.world.resource::<Encounter>().participants.clone();
        let add: Vec<_> = self
            .world
            .query::<(&Id, &Pos, &Unit)>()
            .iter(&self.world)
            .filter(|(i, q, f)| {
                f.team == Team::Enemy
                    && !active.contains(&i.0)
                    && distance(p, q.0) <= gameplay_config::ENEMY_REVEAL_RANGE
            })
            .map(|(i, _, _)| i.0.clone())
            .collect();
        if !add.is_empty() {
            self.world
                .resource_mut::<Encounter>()
                .participants
                .extend(add);
        }
    }
    fn use_skill(
        &mut self,
        a: &str,
        target: &str,
        target_cell: GridPos,
        skill: SkillDef,
    ) -> Result<(), String> {
        self.ensure(a)?;
        let turn = self.world.resource::<Turn>();
        if !can_use_skill(turn) {
            return Err("目前不能使用 Skill".into());
        }
        let ae = self.entity(a).ok_or("找不到攻擊者")?;
        let te = self.entity(target).ok_or("找不到目標")?;
        validate_unit_skill_target(&self.world, ae, te, target_cell, &skill)?;
        let attacker_unit = self
            .world
            .get::<Unit>(ae)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .clone();
        let target_unit = self
            .world
            .get::<Unit>(te)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .clone();
        if skill.effect == SkillEffect::Heal {
            let HealingPreview {
                target: target_name,
                target_hp: _,
                target_max_hp: max_hp,
                target_mana: _,
                healing,
                remaining_hp,
                missing_hp: _,
                health_segments: _,
            } = healing_preview(&self.world, te, &skill);
            self.world
                .get_mut::<Hp>(te)
                .expect("已建立的戰鬥單位應具有 Hp 元件")
                .current = remaining_hp;
            self.world
                .resource_mut::<Log>()
                .0
                .push(CombatLogEvent::Healing {
                    actor: attacker_unit.name,
                    actor_team: attacker_unit.team,
                    skill: skill.name,
                    target: target_name,
                    target_team: target_unit.team,
                    healing,
                    remaining_hp,
                    max_hp,
                });
            self.finish();
            return Ok(());
        }
        let AttackModifierBreakdown {
            attack_stat,
            attack_stat_modifier,
            skill_attack_modifier,
            flanking_modifier,
            total: modifier,
        } = attack_modifier_breakdown(&self.world, ae, te, &skill);
        let natural = die(&mut self.world, gameplay_config::ATTACK_DIE_SIDES) as i32;
        let target_dodge = effective_dodge(&self.world, te);
        let target_block = effective_block(&self.world, te);
        let degree = degree(
            natural,
            modifier,
            gameplay_config::BASE_DEFENSE + target_dodge,
        );
        let result = attack_result(
            natural,
            modifier,
            gameplay_config::BASE_DEFENSE + target_dodge,
            gameplay_config::BASE_DEFENSE + target_dodge + target_block,
        );
        let base_damage = attacker_unit.damage + skill.damage_bonus;
        let critical = degree == RollDegree::CriticalSuccess;
        let raw_damage = attack_damage(
            base_damage,
            AttackResult::Hit,
            gameplay_config::BLOCK_DAMAGE_REDUCTION,
            critical,
        );
        let damage = attack_damage(
            base_damage,
            result,
            gameplay_config::BLOCK_DAMAGE_REDUCTION,
            critical,
        );
        let damage_reduction = raw_damage - damage;
        let mut downed = false;
        if damage > 0 {
            let mut hp = self
                .world
                .get_mut::<Hp>(te)
                .expect("已建立的戰鬥單位應具有 Hp 元件");
            hp.current = (hp.current - damage).max(0);
            if hp.current == 0 {
                downed = true;
                self.world.entity_mut(te).insert(Downed);
                self.world
                    .resource_mut::<Encounter>()
                    .participants
                    .remove(target);
            }
        }
        let mut pushed = false;
        let mut push_blocked = false;
        let mut collision_damage = 0;
        let mut collision_units = Vec::new();
        if skill.effect == SkillEffect::Push && result == AttackResult::Hit && !downed {
            let direction = push_direction(&self.world, ae, target_cell);
            let current = self
                .world
                .get::<Pos>(te)
                .expect("已建立的戰鬥單位應具有 Pos 元件")
                .0;
            let footprint = *self
                .world
                .get::<Footprint>(te)
                .expect("已建立的戰鬥單位應具有 Footprint 元件");
            let destination = GridPos {
                x: current.x + direction.x * gameplay_config::PUSH_DISTANCE,
                y: current.y + direction.y * gameplay_config::PUSH_DISTANCE,
            };
            let terrain_allows_push = fits(self.world.resource::<Board>(), destination, footprint)
                && !footprint_blocks_forced_entry(
                    self.world.resource::<Board>(),
                    destination,
                    footprint,
                );
            let blocking_units: Vec<Entity> = if terrain_allows_push {
                self.world
                    .iter_entities()
                    .filter(|entity| {
                        entity.id() != te
                            && entity.get::<Downed>().is_none()
                            && entity
                                .get::<Pos>()
                                .zip(entity.get::<Footprint>())
                                .is_some_and(|(position, other_footprint)| {
                                    overlap(destination, footprint, position.0, *other_footprint)
                                })
                    })
                    .map(|entity| entity.id())
                    .collect()
            } else {
                Vec::new()
            };
            let can_push = terrain_allows_push && blocking_units.is_empty();
            if can_push {
                self.world
                    .get_mut::<Pos>(te)
                    .expect("已建立的戰鬥單位應具有 Pos 元件")
                    .0 = destination;
                pushed = true;
            } else {
                push_blocked = true;
                collision_damage = gameplay_config::COLLISION_DAMAGE;
                let mut hp = self
                    .world
                    .get_mut::<Hp>(te)
                    .expect("已建立的戰鬥單位應具有 Hp 元件");
                hp.current = (hp.current - collision_damage).max(0);
                if hp.current == 0 {
                    downed = true;
                    self.world.entity_mut(te).insert(Downed);
                    self.world
                        .resource_mut::<Encounter>()
                        .participants
                        .remove(target);
                }
                for blocking_entity in blocking_units {
                    let unit = self
                        .world
                        .get::<Unit>(blocking_entity)
                        .expect("佔用格子的戰鬥單位應具有 Unit 元件")
                        .clone();
                    let id = self
                        .world
                        .get::<Id>(blocking_entity)
                        .expect("佔用格子的戰鬥單位應具有 Id 元件")
                        .0
                        .clone();
                    let mut hp = self
                        .world
                        .get_mut::<Hp>(blocking_entity)
                        .expect("佔用格子的戰鬥單位應具有 Hp 元件");
                    hp.current = (hp.current - collision_damage).max(0);
                    let remaining_hp = hp.current;
                    let max_hp = hp.maximum;
                    let blocker_downed = remaining_hp == 0;
                    if blocker_downed {
                        self.world.entity_mut(blocking_entity).insert(Downed);
                        self.world
                            .resource_mut::<Encounter>()
                            .participants
                            .remove(&id);
                    }
                    collision_units.push(CollisionUnitLog {
                        unit: unit.name,
                        team: unit.team,
                        remaining_hp,
                        max_hp,
                        downed: blocker_downed,
                    });
                }
            }
        }
        let hp = self
            .world
            .get::<Hp>(te)
            .expect("已建立的戰鬥單位應具有 Hp 元件");
        let remaining_hp = hp.current;
        let max_hp = hp.maximum;
        self.world
            .resource_mut::<Log>()
            .0
            .push(CombatLogEvent::Skill {
                actor: attacker_unit.name,
                actor_team: attacker_unit.team,
                skill: skill.name,
                target: target_unit.name,
                target_team: target_unit.team,
                roll: natural,
                die_sides: gameplay_config::ATTACK_DIE_SIDES,
                attack_stat,
                attack_stat_modifier,
                skill_attack_modifier,
                flanking_modifier,
                attack_modifier: modifier,
                attack_total: natural + modifier,
                dodge_target: gameplay_config::BASE_DEFENSE + target_dodge,
                block_target: gameplay_config::BASE_DEFENSE + target_dodge + target_block,
                result,
                critical,
                raw_damage,
                damage_reduction,
                damage,
                remaining_hp,
                max_hp,
                downed,
                pushed,
                push_blocked,
                push_distance: if pushed {
                    gameplay_config::PUSH_DISTANCE
                } else {
                    0
                },
                collision_damage,
                collision_units,
            });
        if pushed {
            self.apply_pushed_terrain(te, target);
        }
        self.finish();
        Ok(())
    }
    fn apply_pushed_terrain(&mut self, entity: Entity, id: &str) {
        let position = self
            .world
            .get::<Pos>(entity)
            .expect("已建立的戰鬥單位應具有 Pos 元件")
            .0;
        let terrain = terrain_at(&self.world, position);
        let terrain = match terrain {
            Some(value) => value,
            None => return,
        };
        let terrain_definition = terrain_type(self.world.resource::<Board>(), &terrain);
        let terrain_name_key = terrain_definition.name_key.clone();
        let log_key = terrain_definition
            .forced_entry_log_key
            .clone()
            .unwrap_or_else(|| "COMBAT_LOG_TERRAIN_DAMAGE".into());
        let damage = if terrain_definition.forced_entry == ForcedEntry::Defeat {
            self.world
                .get::<Hp>(entity)
                .expect("被推動的單位應具有 Hp")
                .current
        } else {
            terrain_definition.damage
        };
        if damage == 0 {
            return;
        }
        let unit = self
            .world
            .get::<Unit>(entity)
            .expect("已建立的戰鬥單位應具有 Unit 元件");
        let target = unit.name.clone();
        let target_team = unit.team;
        let mut hp = self
            .world
            .get_mut::<Hp>(entity)
            .expect("已建立的戰鬥單位應具有 Hp 元件");
        hp.current = (hp.current - damage).max(0);
        let remaining_hp = hp.current;
        let max_hp = hp.maximum;
        let downed = remaining_hp == 0;
        if downed {
            self.world.entity_mut(entity).insert(Downed);
            self.world
                .resource_mut::<Encounter>()
                .participants
                .remove(id);
        }
        self.world
            .resource_mut::<Log>()
            .0
            .push(CombatLogEvent::TerrainDamage {
                target,
                target_team,
                terrain,
                terrain_name_key,
                log_key,
                damage,
                remaining_hp,
                max_hp,
                downed,
            });
    }
    fn use_cell_skill(
        &mut self,
        actor: &str,
        position: GridPos,
        skill: SkillDef,
    ) -> Result<(), String> {
        self.ensure(actor)?;
        if !can_use_skill(self.world.resource::<Turn>()) {
            return Err("現在不能使用技能".into());
        }
        let entity = self.entity(actor).ok_or("找不到行動角色")?;
        let unit = self
            .world
            .get::<Unit>(entity)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .clone();
        if !unit.skills.contains(&skill.id) || skill.effect != SkillEffect::Mire {
            return Err("此角色不能使用這個技能".into());
        }
        let board = self.world.resource::<Board>();
        if !fits(
            board,
            position,
            Footprint {
                width: 1,
                height: 1,
            },
        ) {
            return Err("目標格超出地圖".into());
        }
        let origin = self
            .world
            .get::<Pos>(entity)
            .expect("已建立的戰鬥單位應具有 Pos 元件")
            .0;
        if distance(origin, position) > skill.range.unwrap_or(gameplay_config::DEFAULT_MIRE_RANGE) {
            return Err("目標超出技能範圍".into());
        }
        let current_round = self.world.resource::<Encounter>().round;
        let duration = skill
            .duration
            .unwrap_or(gameplay_config::DEFAULT_MIRE_DURATION);
        let terrain = skill
            .terrain
            .as_ref()
            .expect("載入時已驗證地形技能具有 terrain")
            .clone();
        self.world.resource_mut::<TemporaryTerrains>().0.insert(
            position,
            TemporaryTerrain {
                kind: terrain.clone(),
                expires_after_round: current_round + duration - 1,
            },
        );
        let terrain_name_key = terrain_type(self.world.resource::<Board>(), &terrain)
            .name_key
            .clone();
        self.world
            .resource_mut::<Log>()
            .0
            .push(CombatLogEvent::TerrainCreated {
                actor: unit.name,
                actor_team: unit.team,
                skill: skill.name,
                terrain,
                terrain_name_key,
            });
        self.finish();
        Ok(())
    }
    fn enemy_turns(&mut self) -> Result<(), String> {
        loop {
            let a = match self.world.resource::<Turn>().actor.clone() {
                Some(v) => v,
                None => return Ok(()),
            };
            let e = self.entity(&a).ok_or("先攻單位不存在")?;
            if self.world.get::<Downed>(e).is_some() {
                self.finish();
                continue;
            }
            if self
                .world
                .get::<Unit>(e)
                .expect("已建立的戰鬥單位應具有 Unit 元件")
                .team
                == Team::Player
            {
                return Ok(());
            }
            let t = match self.closest(e) {
                Some(v) => v,
                None => {
                    self.finish();
                    continue;
                }
            };
            if entity_distance(&self.world, e, t) > gameplay_config::DEFAULT_MELEE_RANGE {
                let goal = self
                    .world
                    .get::<Pos>(t)
                    .expect("已建立的戰鬥單位應具有 Pos 元件")
                    .0;
                let start = self
                    .world
                    .get::<Pos>(e)
                    .expect("已建立的戰鬥單位應具有 Pos 元件")
                    .0;
                let fp = *self
                    .world
                    .get::<Footprint>(e)
                    .expect("已建立的戰鬥單位應具有 Footprint 元件");
                let b = self
                    .world
                    .get::<Unit>(e)
                    .expect("已建立的戰鬥單位應具有 Unit 元件")
                    .movement;
                if let Some(path) = toward(&self.world, e, start, goal, fp, b) {
                    if let [_, .., last] = path.as_slice() {
                        self.movements.push(MovementTransition {
                            unit_id: a.clone(),
                            path: path.clone(),
                            before_log_index: self.world.resource::<Log>().0.len(),
                        });
                        self.world
                            .get_mut::<Pos>(e)
                            .expect("已建立的戰鬥單位應具有 Pos 元件")
                            .0 = *last
                    }
                }
            }
            if entity_distance(&self.world, e, t) <= gameplay_config::DEFAULT_MELEE_RANGE {
                let id = self
                    .world
                    .get::<Id>(t)
                    .expect("已建立的戰鬥單位應具有 Id 元件")
                    .0
                    .clone();
                let skills = self.world.resource::<Skills>();
                let skill = skills
                    .definitions
                    .get(&skills.ai_default)
                    .cloned()
                    .expect("載入時已驗證 ai_default 技能");
                let target_cell = closest_occupied_cell(&self.world, e, t);
                self.use_skill(&a, &id, target_cell, skill)?
            } else {
                self.finish()
            }
            self.outcome();
            if self.world.resource::<ResultState>().0 != Outcome::Ongoing {
                return Ok(());
            }
        }
    }
    fn entity(&self, id: &str) -> Option<Entity> {
        self.world
            .iter_entities()
            .find(|e| e.get::<Id>().is_some_and(|i| i.0 == id))
            .map(|e| e.id())
    }
    fn closest(&self, e: Entity) -> Option<Entity> {
        let p = self.world.get::<Pos>(e)?.0;
        self.world
            .iter_entities()
            .filter(|q| {
                q.get::<Unit>().is_some_and(|f| f.team == Team::Player)
                    && q.get::<Downed>().is_none()
            })
            .min_by_key(|q| {
                distance(
                    p,
                    q.get::<Pos>().expect("已建立的戰鬥單位應具有 Pos 元件").0,
                )
            })
            .map(|q| q.id())
    }
    fn outcome(&mut self) {
        let mut p = false;
        let mut e = false;
        for q in self.world.iter_entities() {
            if let Some(f) = q.get::<Unit>().filter(|_| q.get::<Downed>().is_none()) {
                match f.team {
                    Team::Player => p = true,
                    Team::Enemy => e = true,
                }
            }
        }
        self.world.resource_mut::<ResultState>().0 = if !p {
            Outcome::Defeat
        } else if !e {
            Outcome::Victory
        } else {
            Outcome::Ongoing
        }
    }
    pub fn snapshot(&mut self) -> Snapshot {
        let b = self.world.resource::<Board>().clone();
        let temporary_terrains = self.world.resource::<TemporaryTerrains>().clone();
        let enc = self.world.resource::<Encounter>().clone();
        let turn = self.world.resource::<Turn>().clone();
        let mut units: Vec<_> = self
            .world
            .iter_entities()
            .filter(|entity| entity.get::<Downed>().is_none())
            .filter_map(|e| {
                Some((
                    e.id(),
                    e.get::<Id>()?,
                    e.get::<Pos>()?,
                    e.get::<Footprint>()?,
                    e.get::<Hp>()?,
                    e.get::<Unit>()?,
                    e.get::<Downed>().is_some(),
                ))
            })
            .map(|(entity, i, p, fp, h, f, d)| UnitView {
                id: i.0.clone(),
                name: f.name.clone(),
                visual: f.visual.clone(),
                team: f.team,
                group: f.group.clone(),
                x: p.0.x,
                y: p.0.y,
                width: fp.width,
                height: fp.height,
                large: fp.width > 1 || fp.height > 1,
                occupied_cells: footprint_cells(p.0, *fp),
                health_ratio: h.current as f32 / h.maximum as f32,
                hp: h.current,
                max_hp: h.maximum,
                movement: f.movement,
                initiative: f.initiative,
                dodge: effective_dodge(&self.world, entity),
                block: effective_block(&self.world, entity),
                melee: f.melee,
                ranged: f.ranged,
                damage: f.damage,
                range: f.range,
                downed: d,
                active: enc.participants.contains(&i.0),
            })
            .collect();
        units.sort_by(|a, b| a.id.cmp(&b.id));
        let movement_ranges = turn
            .actor
            .as_ref()
            .and_then(|a| self.entity(a))
            .map(|entity| movement_ranges(&self.world, entity, &turn));
        let (reachable, second_reachable) = movement_ranges.unwrap_or_default();
        let skill_ranges = turn
            .actor
            .as_ref()
            .and_then(|actor| self.entity(actor))
            .map(|entity| skill_ranges(&self.world, entity))
            .unwrap_or_default();
        let can_skill = can_use_skill(&turn);
        let turn_order: Vec<_> = enc
            .order
            .iter()
            .skip(enc.cursor + 1)
            .filter(|unit_id| {
                self.entity(unit_id)
                    .is_some_and(|entity| self.world.get::<Downed>(entity).is_none())
            })
            .cloned()
            .collect();
        let can_delay = turn.phase == Phase::Ready
            && turn.moves == 0
            && turn.actor.is_some()
            && !turn_order.is_empty();
        let terrain_cells = (0..b.height)
            .flat_map(|y| {
                let board = &b;
                let temporary = &temporary_terrains;
                let world = &self.world;
                let units = &units;
                (0..b.width).map(move |x| {
                    let position = GridPos { x, y };
                    let base_cost = board.costs[(y * board.width + x) as usize];
                    let temporary_terrain = temporary.0.get(&position);
                    let effect = temporary_terrain
                        .map(|terrain| terrain.kind.clone())
                        .or_else(|| board.triggers.get(&position).cloned())
                        .unwrap_or_default();
                    let cost = movement_cost(world, position);
                    let kind = if !effect.is_empty() {
                        effect.as_str()
                    } else if base_cost > 1 {
                        "rough"
                    } else {
                        "plain"
                    };
                    let definition = terrain_type(board, kind);
                    let damage = definition.damage;
                    let remaining_rounds = temporary_terrain
                        .map(|terrain| terrain.expires_after_round.saturating_sub(enc.round) + 1);
                    let effect_arguments = if remaining_rounds.is_some()
                        && (definition.dodge_penalty > 0
                            || definition.block_penalty > 0
                            || definition.movement_cost_bonus > 0)
                    {
                        vec![
                            definition.dodge_penalty.max(definition.block_penalty),
                            definition.movement_cost_bonus as i32,
                            remaining_rounds.unwrap_or(0) as i32,
                        ]
                    } else if damage > 0 {
                        vec![damage]
                    } else {
                        Vec::new()
                    };
                    let effect_description = detail(&definition.effect_key, &effect_arguments);
                    TerrainCellView {
                        x,
                        y,
                        kind: kind.to_string(),
                        name_key: definition.name_key.clone(),
                        visual: definition.visual.clone(),
                        passable: definition.passable,
                        base_kind: if base_cost > 1 { "rough" } else { "plain" }.to_string(),
                        unit_id: units
                            .iter()
                            .find(|unit| unit.occupied_cells.contains(&position))
                            .map(|unit| unit.id.clone()),
                        cost,
                        damage,
                        remaining_rounds,
                        effect_description,
                        effect,
                    }
                })
            })
            .collect();
        Snapshot {
            width: b.width,
            height: b.height,
            costs: b.costs.clone(),
            terrain_effects: {
                let mut effects: Vec<_> = b
                    .triggers
                    .clone()
                    .into_iter()
                    .filter(|(position, _)| !temporary_terrains.0.contains_key(position))
                    .map(|(position, effect)| TerrainEffectView {
                        x: position.x,
                        y: position.y,
                        damage: terrain_damage(&b, &effect),
                        visual: terrain_type(&b, &effect).visual.clone(),
                        effect,
                        remaining_rounds: None,
                    })
                    .collect();
                effects.extend(temporary_terrains.0.into_iter().map(|(position, terrain)| {
                    TerrainEffectView {
                        x: position.x,
                        y: position.y,
                        damage: terrain_damage(&b, &terrain.kind),
                        visual: terrain_type(&b, &terrain.kind).visual.clone(),
                        effect: terrain.kind,
                        remaining_rounds: Some(
                            terrain.expires_after_round.saturating_sub(enc.round) + 1,
                        ),
                    }
                }));
                effects.sort_by_key(|effect| (effect.y, effect.x));
                effects
            },
            terrain_cells,
            units,
            reachable,
            second_reachable,
            skill_ranges,
            turn_order,
            turn: TurnView {
                can_end_turn: turn.actor.is_some(),
                actor: turn.actor,
                phase: format!("{:?}", turn.phase).to_lowercase(),
                move_remaining: turn.remaining,
                can_move: matches!(turn.phase, Phase::Ready | Phase::Moving | Phase::AfterMove)
                    && turn.moves < 2,
                can_skill,
                can_delay,
            },
            round: enc.round,
            outcome: self.world.resource::<ResultState>().0,
            log: self.world.resource::<Log>().0.iter().cloned().collect(),
            movements: self.movements.clone(),
        }
    }
}
fn healing_preview(world: &World, target: Entity, skill: &SkillDef) -> HealingPreview {
    let hp = world
        .get::<Hp>(target)
        .expect("已建立的戰鬥單位應具有 Hp 元件");
    let remaining_hp = (hp.current
        + skill
            .heal_amount
            .expect("載入時已驗證治療技能具有正值 heal_amount"))
    .min(hp.maximum);
    HealingPreview {
        target: world
            .get::<Unit>(target)
            .expect("已建立的戰鬥單位應具有 Unit 元件")
            .name
            .clone(),
        target_hp: hp.current,
        target_max_hp: hp.maximum,
        target_mana: gameplay_config::DEFAULT_MANA,
        healing: remaining_hp - hp.current,
        remaining_hp,
        missing_hp: hp.maximum - remaining_hp,
        health_segments: HealthSegmentsView {
            hit: hp.current,
            block: 0,
            damage: 0,
            missing: hp.maximum - remaining_hp,
        },
    }
}

fn validate_unit_skill_target(
    world: &World,
    attacker: Entity,
    target: Entity,
    target_cell: GridPos,
    skill: &SkillDef,
) -> Result<(), String> {
    if world.get::<Downed>(target).is_some() {
        return Err("目標已倒下".into());
    }
    let target_position = world
        .get::<Pos>(target)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let target_footprint = *world
        .get::<Footprint>(target)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    if !overlap(
        target_cell,
        Footprint {
            width: 1,
            height: 1,
        },
        target_position,
        target_footprint,
    ) {
        return Err("所選格不屬於目標".into());
    }
    let attacker_unit = world
        .get::<Unit>(attacker)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    let target_unit = world
        .get::<Unit>(target)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    if !attacker_unit.skills.contains(&skill.id) || skill.effect == SkillEffect::Mire {
        return Err("此角色不能使用這個技能".into());
    }
    if skill.effect == SkillEffect::Heal {
        if attacker_unit.team != target_unit.team {
            return Err("只能治療自己或友軍".into());
        }
    } else if attacker_unit.team == target_unit.team {
        return Err("不能攻擊友軍".into());
    }
    let range = skill.range.unwrap_or(if skill.ranged {
        attacker_unit.range
    } else {
        gameplay_config::DEFAULT_MELEE_RANGE
    });
    let attacker_position = world
        .get::<Pos>(attacker)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let attacker_footprint = *world
        .get::<Footprint>(attacker)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    if footprint_distance(
        attacker_position,
        attacker_footprint,
        target_cell,
        Footprint {
            width: 1,
            height: 1,
        },
    ) > range
    {
        return Err("目標超出射程".into());
    }
    Ok(())
}

fn attack_result(
    natural: i32,
    modifier: i32,
    dodge_target: i32,
    block_target: i32,
) -> AttackResult {
    let roll_degree = degree(natural, modifier, dodge_target);
    if matches!(
        roll_degree,
        RollDegree::Failure | RollDegree::CriticalFailure
    ) {
        AttackResult::Dodge
    } else if natural + modifier < block_target {
        AttackResult::Block
    } else {
        AttackResult::Hit
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TargetSide {
    Left,
    Right,
    Above,
    Below,
}

struct AttackModifierBreakdown {
    attack_stat: AttackStat,
    attack_stat_modifier: i32,
    skill_attack_modifier: i32,
    flanking_modifier: i32,
    total: i32,
}

fn attack_modifier(world: &World, attacker: Entity, target: Entity, skill: &SkillDef) -> i32 {
    attack_modifier_breakdown(world, attacker, target, skill).total
}

fn attack_modifier_breakdown(
    world: &World,
    attacker: Entity,
    target: Entity,
    skill: &SkillDef,
) -> AttackModifierBreakdown {
    let unit = world
        .get::<Unit>(attacker)
        .expect("可發動攻擊的單位應具有 Unit");
    let (attack_stat, attack_stat_modifier) = if skill.ranged {
        (AttackStat::Ranged, unit.ranged)
    } else {
        (AttackStat::Melee, unit.melee)
    };
    let skill_attack_modifier = skill.attack_bonus;
    let flanking_modifier = flanking_bonus(world, attacker, target, skill);
    AttackModifierBreakdown {
        attack_stat,
        attack_stat_modifier,
        skill_attack_modifier,
        flanking_modifier,
        total: attack_stat_modifier + skill_attack_modifier + flanking_modifier,
    }
}

fn flanking_bonus(world: &World, attacker: Entity, target: Entity, skill: &SkillDef) -> i32 {
    if skill.ranged {
        return 0;
    }
    let attack_range = skill.range.unwrap_or(gameplay_config::DEFAULT_MELEE_RANGE);
    let attacker_side = match target_side_within_range(world, attacker, target, attack_range) {
        Some(side) => side,
        None => return 0,
    };
    let attacker_team = world
        .get::<Unit>(attacker)
        .expect("可發動攻擊的單位應具有 Unit")
        .team;
    let skills = world.resource::<Skills>();
    let has_supporter = world.iter_entities().any(|entity| {
        if entity.id() == attacker || entity.id() == target || entity.get::<Downed>().is_some() {
            return false;
        }
        let unit = match entity.get::<Unit>() {
            Some(unit) => unit,
            None => return false,
        };
        if unit.team != attacker_team {
            return false;
        }
        let support_range = unit
            .skills
            .iter()
            .filter_map(|skill_id| skills.definitions.get(skill_id))
            .filter(|support_skill| {
                !support_skill.ranged
                    && matches!(
                        support_skill.effect,
                        SkillEffect::Attack | SkillEffect::Push
                    )
            })
            .map(|support_skill| {
                support_skill
                    .range
                    .unwrap_or(gameplay_config::DEFAULT_MELEE_RANGE)
            })
            .max();
        support_range.is_some_and(|range| {
            target_side_within_range(world, entity.id(), target, range)
                .is_some_and(|side| sides_are_opposite(attacker_side, side))
        })
    });
    if has_supporter {
        gameplay_config::FLANKING_ATTACK_BONUS
    } else {
        0
    }
}

fn target_side_within_range(
    world: &World,
    unit: Entity,
    target: Entity,
    range: i32,
) -> Option<TargetSide> {
    let unit_position = world.get::<Pos>(unit).expect("參與包夾的單位應具有 Pos").0;
    let unit_footprint = *world
        .get::<Footprint>(unit)
        .expect("參與包夾的單位應具有 Footprint");
    let target_position = world.get::<Pos>(target).expect("包夾目標應具有 Pos").0;
    let target_footprint = *world
        .get::<Footprint>(target)
        .expect("包夾目標應具有 Footprint");
    if footprint_distance(
        unit_position,
        unit_footprint,
        target_position,
        target_footprint,
    ) > range
    {
        return None;
    }

    let unit_right = unit_position.x + unit_footprint.width - 1;
    let unit_bottom = unit_position.y + unit_footprint.height - 1;
    let target_right = target_position.x + target_footprint.width - 1;
    let target_bottom = target_position.y + target_footprint.height - 1;
    let rows_overlap = unit_position.y <= target_bottom && unit_bottom >= target_position.y;
    let columns_overlap = unit_position.x <= target_right && unit_right >= target_position.x;
    if rows_overlap && unit_right < target_position.x {
        Some(TargetSide::Left)
    } else if rows_overlap && unit_position.x > target_right {
        Some(TargetSide::Right)
    } else if columns_overlap && unit_bottom < target_position.y {
        Some(TargetSide::Above)
    } else if columns_overlap && unit_position.y > target_bottom {
        Some(TargetSide::Below)
    } else {
        None
    }
}

fn sides_are_opposite(first: TargetSide, second: TargetSide) -> bool {
    matches!(
        (first, second),
        (TargetSide::Left, TargetSide::Right)
            | (TargetSide::Right, TargetSide::Left)
            | (TargetSide::Above, TargetSide::Below)
            | (TargetSide::Below, TargetSide::Above)
    )
}

fn attack_damage(
    base_damage: i32,
    result: AttackResult,
    block_damage_reduction: i32,
    critical: bool,
) -> i32 {
    let result_damage = match result {
        AttackResult::Dodge => 0,
        AttackResult::Block => (base_damage - block_damage_reduction).max(0),
        AttackResult::Hit => base_damage,
    };
    result_damage * if critical { 2 } else { 1 }
}

fn can_use_skill(turn: &Turn) -> bool {
    matches!(turn.phase, Phase::Ready | Phase::Moving) && turn.moves == 0
        || matches!(turn.phase, Phase::AfterMove) && turn.moves == 1
}
pub fn degree(n: i32, m: i32, t: i32) -> RollDegree {
    if n == 1 {
        RollDegree::CriticalFailure
    } else if n == gameplay_config::ATTACK_DIE_SIDES as i32 {
        RollDegree::CriticalSuccess
    } else if n + m >= t {
        RollDegree::Success
    } else {
        RollDegree::Failure
    }
}
fn die(w: &mut World, s: u32) -> u32 {
    let mut r = w.resource_mut::<Random>();
    r.0 = r.0.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((r.0 >> 32) as u32 % s) + 1
}
fn terrain_at(w: &World, position: GridPos) -> Option<String> {
    w.resource::<TemporaryTerrains>()
        .0
        .get(&position)
        .map(|terrain| terrain.kind.clone())
        .or_else(|| w.resource::<Board>().triggers.get(&position).cloned())
}

fn movement_cost(w: &World, position: GridPos) -> u32 {
    let board = w.resource::<Board>();
    let base = board.costs[(position.y * board.width + position.x) as usize];
    base + terrain_at(w, position)
        .map(|kind| terrain_type(board, &kind).movement_cost_bonus)
        .unwrap_or(0)
}

fn terrain_ends_movement(w: &World, position: GridPos) -> bool {
    terrain_at(w, position)
        .is_some_and(|kind| terrain_type(w.resource::<Board>(), &kind).ends_movement)
}

fn effective_dodge(w: &World, entity: Entity) -> i32 {
    let unit = w
        .get::<Unit>(entity)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    let position = w
        .get::<Pos>(entity)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let footprint = *w
        .get::<Footprint>(entity)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let penalty = footprint_terrain_penalty(
        w.resource::<Board>(),
        w.resource::<TemporaryTerrains>(),
        position,
        footprint,
        |terrain| terrain.dodge_penalty,
    );
    (unit.dodge - penalty).max(0)
}

fn effective_block(w: &World, entity: Entity) -> i32 {
    let unit = w
        .get::<Unit>(entity)
        .expect("已建立的戰鬥單位應具有 Unit 元件");
    let position = w
        .get::<Pos>(entity)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let footprint = *w
        .get::<Footprint>(entity)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let penalty = footprint_terrain_penalty(
        w.resource::<Board>(),
        w.resource::<TemporaryTerrains>(),
        position,
        footprint,
        |terrain| terrain.block_penalty,
    );
    (unit.block - penalty).max(0)
}

fn footprint_terrain_penalty(
    board: &Board,
    temporary: &TemporaryTerrains,
    position: GridPos,
    footprint: Footprint,
    penalty: impl Fn(&TerrainTypeDef) -> i32,
) -> i32 {
    (position.y..position.y + footprint.height)
        .flat_map(|y| (position.x..position.x + footprint.width).map(move |x| GridPos { x, y }))
        .filter_map(|cell| {
            temporary
                .0
                .get(&cell)
                .map(|terrain| terrain.kind.as_str())
                .or_else(|| board.triggers.get(&cell).map(String::as_str))
        })
        .map(|kind| penalty(terrain_type(board, kind)))
        .max()
        .unwrap_or(0)
}

fn footprint_on_impassable(board: &Board, position: GridPos, footprint: Footprint) -> bool {
    (position.y..position.y + footprint.height).any(|y| {
        (position.x..position.x + footprint.width).any(|x| {
            board
                .triggers
                .get(&GridPos { x, y })
                .is_some_and(|kind| !terrain_type(board, kind).passable)
        })
    })
}

fn footprint_blocks_forced_entry(board: &Board, position: GridPos, footprint: Footprint) -> bool {
    (position.y..position.y + footprint.height).any(|y| {
        (position.x..position.x + footprint.width).any(|x| {
            board
                .triggers
                .get(&GridPos { x, y })
                .is_some_and(|kind| terrain_type(board, kind).forced_entry == ForcedEntry::Blocked)
        })
    })
}

fn push_direction(w: &World, attacker: Entity, target_cell: GridPos) -> GridPos {
    let attacker_position = w
        .get::<Pos>(attacker)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let attacker_footprint = *w
        .get::<Footprint>(attacker)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    if attacker_position.x + attacker_footprint.width <= target_cell.x {
        GridPos { x: 1, y: 0 }
    } else if target_cell.x < attacker_position.x {
        GridPos { x: -1, y: 0 }
    } else if attacker_position.y + attacker_footprint.height <= target_cell.y {
        GridPos { x: 0, y: 1 }
    } else {
        GridPos { x: 0, y: -1 }
    }
}

fn closest_occupied_cell(w: &World, attacker: Entity, target: Entity) -> GridPos {
    let attacker_position = w
        .get::<Pos>(attacker)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let attacker_footprint = *w
        .get::<Footprint>(attacker)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let target_position = w
        .get::<Pos>(target)
        .expect("已建立的戰鬥單位應具有 Pos 元件")
        .0;
    let target_footprint = *w
        .get::<Footprint>(target)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    footprint_cells(target_position, target_footprint)
        .into_iter()
        .min_by_key(|cell| {
            footprint_distance(
                attacker_position,
                attacker_footprint,
                *cell,
                Footprint {
                    width: 1,
                    height: 1,
                },
            )
        })
        .expect("載入時已驗證單位佔用尺寸為正值，應至少佔用一格")
}
fn footprint_cells(position: GridPos, footprint: Footprint) -> Vec<GridPos> {
    (position.y..position.y + footprint.height)
        .flat_map(|y| (position.x..position.x + footprint.width).map(move |x| GridPos { x, y }))
        .collect()
}
fn fits(b: &Board, p: GridPos, f: Footprint) -> bool {
    p.x >= 0 && p.y >= 0 && p.x + f.width <= b.width && p.y + f.height <= b.height
}
fn distance(a: GridPos, b: GridPos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
fn overlap(a: GridPos, af: Footprint, b: GridPos, bf: Footprint) -> bool {
    a.x < b.x + bf.width && a.x + af.width > b.x && a.y < b.y + bf.height && a.y + af.height > b.y
}
fn occupied(w: &World, ignore: Entity, p: GridPos, f: Footprint) -> bool {
    w.iter_entities().any(|e| {
        e.id() != ignore
            && e.get::<Downed>().is_none()
            && e.get::<Pos>()
                .zip(e.get::<Footprint>())
                .is_some_and(|(q, g)| overlap(p, f, q.0, *g))
    })
}
fn entity_distance(w: &World, a: Entity, b: Entity) -> i32 {
    let ap = w.get::<Pos>(a).expect("已建立的戰鬥單位應具有 Pos 元件").0;
    let af = *w
        .get::<Footprint>(a)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let bp = w.get::<Pos>(b).expect("已建立的戰鬥單位應具有 Pos 元件").0;
    let bf = *w
        .get::<Footprint>(b)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    footprint_distance(ap, af, bp, bf)
}
fn footprint_distance(a: GridPos, af: Footprint, b: GridPos, bf: Footprint) -> i32 {
    let dx = (b.x - (a.x + af.width - 1))
        .max(a.x - (b.x + bf.width - 1))
        .max(0);
    let dy = (b.y - (a.y + af.height - 1))
        .max(a.y - (b.y + bf.height - 1))
        .max(0);
    dx + dy
}
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct PathState {
    danger: u32,
    position: GridPos,
}

#[derive(Eq)]
struct Node {
    movement: u32,
    state: PathState,
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.state
            .danger
            .cmp(&self.state.danger)
            .then_with(|| o.movement.cmp(&self.movement))
            .then_with(|| self.state.position.x.cmp(&o.state.position.x))
            .then_with(|| self.state.position.y.cmp(&o.state.position.y))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.movement == o.movement && self.state == o.state
    }
}

struct Paths {
    best: HashMap<GridPos, PathState>,
    previous: HashMap<PathState, PathState>,
}
fn paths(w: &World, e: Entity, start: GridPos, f: Footprint, budget: u32) -> Paths {
    let board = w.resource::<Board>();
    let start_state = PathState {
        danger: 0,
        position: start,
    };
    let mut dist = HashMap::from([(start_state, 0)]);
    let mut labels = HashMap::from([(start, vec![(0, 0)])]);
    let mut best = HashMap::new();
    let mut previous = HashMap::new();
    let mut heap = BinaryHeap::from([Node {
        movement: 0,
        state: start_state,
    }]);
    while let Some(Node { movement, state }) = heap.pop() {
        if movement
            > *dist
                .get(&state)
                .expect("加入搜尋佇列的狀態應已有移動成本紀錄")
        {
            continue;
        }
        best.entry(state.position).or_insert(state);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = GridPos {
                x: state.position.x + dx,
                y: state.position.y + dy,
            };
            if !fits(board, n, f) || occupied(w, e, n, f) || footprint_on_impassable(board, n, f) {
                continue;
            }
            let next_state = PathState {
                danger: state.danger + u32::from(terrain_ends_movement(w, n)),
                position: n,
            };
            let next_movement = movement + movement_cost(w, n);
            let dominated = labels.get(&n).is_some_and(|known| {
                known.iter().any(|(danger, movement)| {
                    *danger <= next_state.danger && *movement <= next_movement
                })
            });
            if next_movement <= budget && !dominated {
                dist.insert(next_state, next_movement);
                labels
                    .entry(n)
                    .or_default()
                    .push((next_state.danger, next_movement));
                previous.insert(next_state, state);
                heap.push(Node {
                    movement: next_movement,
                    state: next_state,
                })
            }
        }
    }
    Paths { best, previous }
}
fn find_path(
    w: &World,
    e: Entity,
    s: GridPos,
    end: GridPos,
    f: Footprint,
    b: u32,
) -> Option<Vec<GridPos>> {
    let Paths { best, previous } = paths(w, e, s, f, b);
    let mut state = *best.get(&end)?;
    let mut out = vec![state.position];
    while state.position != s {
        state = *previous
            .get(&state)
            .expect("非起點的最佳尋路狀態必須有前一個狀態");
        out.push(state.position)
    }
    out.reverse();
    Some(out)
}
fn toward(
    w: &World,
    e: Entity,
    s: GridPos,
    target: GridPos,
    f: Footprint,
    b: u32,
) -> Option<Vec<GridPos>> {
    let board = w.resource::<Board>();
    let search_budget = (0..board.height)
        .flat_map(|y| (0..board.width).map(move |x| GridPos { x, y }))
        .fold(0_u32, |total, cell| {
            total.saturating_add(movement_cost(w, cell))
        });
    let Paths { best, previous } = paths(w, e, s, f, search_budget);
    let end = *best
        .keys()
        .min_by_key(|p| (distance(**p, target), p.y, p.x))?;
    let mut state = *best.get(&end).expect("選出的終點應有尋路狀態");
    let mut route = vec![state.position];
    while state.position != s {
        state = *previous
            .get(&state)
            .expect("非起點的最佳尋路狀態必須有前一個狀態");
        route.push(state.position);
    }
    route.reverse();
    let mut spent = 0;
    let mut steps = 1;
    for cell in route.iter().skip(1) {
        let cost = movement_cost(w, *cell);
        if cost > b.saturating_sub(spent) {
            break;
        }
        spent += cost;
        steps += 1;
        if terrain_ends_movement(w, *cell) {
            break;
        }
    }
    route.truncate(steps);
    Some(route)
}
fn reach(w: &World, e: Entity, b: u32) -> Vec<GridPos> {
    let p = w.get::<Pos>(e).expect("已建立的戰鬥單位應具有 Pos 元件").0;
    let f = *w
        .get::<Footprint>(e)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let mut v: Vec<_> = paths(w, e, p, f, b).best.into_keys().collect();
    v.sort_by_key(|p| (p.y, p.x));
    v
}
fn movement_ranges(w: &World, e: Entity, turn: &Turn) -> (Vec<GridPos>, Vec<GridPos>) {
    if turn.moves >= 2 {
        return (Vec::new(), Vec::new());
    }
    let allowance = w
        .get::<Unit>(e)
        .expect("已建立的戰鬥單位應具有 Unit 元件")
        .movement;
    let first_budget = if turn.moves == 0 { turn.remaining } else { 0 };
    let reachable = if first_budget > 0 {
        reach(w, e, first_budget)
    } else {
        Vec::new()
    };
    let first_cells: HashSet<_> = reachable.iter().copied().collect();
    let second_reachable = reach(w, e, first_budget + allowance)
        .into_iter()
        .filter(|cell| !first_cells.contains(cell))
        .collect();
    (reachable, second_reachable)
}
fn skill_ranges(w: &World, e: Entity) -> Vec<SkillRangeView> {
    let board = w.resource::<Board>();
    let position = w.get::<Pos>(e).expect("已建立的戰鬥單位應具有 Pos 元件").0;
    let footprint = *w
        .get::<Footprint>(e)
        .expect("已建立的戰鬥單位應具有 Footprint 元件");
    let unit = w.get::<Unit>(e).expect("已建立的戰鬥單位應具有 Unit 元件");
    let skills = w.resource::<Skills>();
    let ranges: Vec<_> = unit
        .skills
        .iter()
        .filter_map(|skill_id| skills.definitions.get(skill_id))
        .map(|skill| {
            let range = skill.range.unwrap_or(if skill.ranged {
                unit.range
            } else {
                gameplay_config::DEFAULT_MELEE_RANGE
            });
            let mut cells = Vec::new();
            for y in 0..board.height {
                for x in 0..board.width {
                    let cell = GridPos { x, y };
                    let cell_distance = footprint_distance(
                        position,
                        footprint,
                        cell,
                        Footprint {
                            width: 1,
                            height: 1,
                        },
                    );
                    if cell_distance <= range
                        && (matches!(skill.effect, SkillEffect::Mire | SkillEffect::Heal)
                            || cell_distance > 0)
                    {
                        cells.push(cell);
                    }
                }
            }
            SkillRangeView {
                id: skill.id.clone(),
                name_key: format!("SKILL_{}_NAME", skill.id.to_ascii_uppercase()),
                details: skill_details(skill, range, board),
                cell_targeted: skill.effect == SkillEffect::Mire,
                enabled: can_use_skill(w.resource::<Turn>()),
                cells,
            }
        })
        .collect();
    ranges
}

fn skill_details(skill: &SkillDef, range: i32, board: &Board) -> Vec<DetailView> {
    let target_key = if skill.effect == SkillEffect::Mire {
        "SKILL_TARGET_CELL"
    } else if skill.effect == SkillEffect::Heal {
        "SKILL_TARGET_ALLY"
    } else {
        "SKILL_TARGET_ENEMY"
    };
    let type_key = if skill.ranged {
        "SKILL_TYPE_RANGED"
    } else {
        "SKILL_TYPE_MELEE"
    };
    let mut details = vec![
        detail(target_key, &[]),
        detail(type_key, &[]),
        detail("SKILL_RANGE", &[range]),
    ];
    if matches!(skill.effect, SkillEffect::Attack | SkillEffect::Push) {
        details.push(detail("SKILL_ATTACK_BONUS", &[skill.attack_bonus]));
        details.push(detail("SKILL_DAMAGE_BONUS", &[skill.damage_bonus]));
    }
    match skill.effect {
        SkillEffect::Attack => {}
        SkillEffect::Push => details.push(detail(
            "SKILL_EFFECT_PUSH",
            &[gameplay_config::PUSH_DISTANCE],
        )),
        SkillEffect::Mire => {
            let terrain = skill
                .terrain
                .as_ref()
                .expect("載入時已驗證地形技能具有 terrain");
            details.push(detail(
                "SKILL_EFFECT_MIRE",
                &[
                    terrain_type(board, terrain).movement_cost_bonus as i32,
                    skill
                        .duration
                        .unwrap_or(gameplay_config::DEFAULT_MIRE_DURATION)
                        as i32,
                ],
            ))
        }
        SkillEffect::Heal => details.push(detail(
            "SKILL_EFFECT_HEAL",
            &[skill
                .heal_amount
                .expect("載入時已驗證治療技能具有正值 heal_amount")],
        )),
    }
    details
}

fn detail(text_key: &str, values: &[i32]) -> DetailView {
    DetailView {
        text_key: text_key.into(),
        arguments: values.to_vec(),
    }
}
