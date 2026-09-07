use bevy_ecs::prelude::World;
use board::domain::alias::ID;
use board::domain::core_types::{LevelOutcome, OutcomeBranches};
use board::ecs_logic::loader::GameDataToml;
use board::ecs_logic::query::{ObjectQueryResult, get_all_objects, get_all_units, get_resource};
use board::ecs_types::components::{
    ActionState, ObjectBundle, Occupant, Position, UnitBundle, attribute_fields,
};
use board::ecs_types::resources::{Board, EndConditionConfig, LevelConfig, TurnOrder};
use board::error::{
    BoardError, DataError, DeploymentError, Error, ErrorKind, LoadError, ReactionError, UnitError,
};
use board::logic::skill::skill_execution::{CheckResult, CheckTarget, EffectEntry, ResolvedEffect};
use godot::prelude::*;
use godot_bind_wrapper_macros::godot_result;
use std::collections::HashMap;

/// FFI 邊界專用錯誤碼：core 的 Coord 為 usize，無法表達負座標，
/// 因此不屬於 board::error::ErrorKind，於此處自成一類。
const ERROR_CODE_INVALID_POSITION: &str = "InvalidPosition";

enum GodotBindError {
    Core(Error),
    Ffi {
        error_code: &'static str,
        message: String,
    },
}

impl From<Error> for GodotBindError {
    fn from(error: Error) -> Self {
        Self::Core(error)
    }
}

type GodotBindResult<T> = std::result::Result<T, GodotBindError>;

fn into_godot_result(
    function_name: &str,
    result: GodotBindResult<Dictionary<Variant, Variant>>,
) -> Dictionary<Variant, Variant> {
    match result {
        Ok(mut result) => {
            result.set("ok", true);
            result
        }
        Err(GodotBindError::Core(error)) => {
            error_dictionary(error_code(&error), format!("{function_name}: {error}"))
        }
        Err(GodotBindError::Ffi {
            error_code,
            message,
        }) => error_dictionary(error_code, format!("{function_name}: {message}")),
    }
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct GameSession {
    base: Base<RefCounted>,
    world: World,
}

#[godot_api]
impl IRefCounted for GameSession {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            world: World::new(),
        }
    }
}

// `godot_result` 會把每個 `#[func]` 的原始函式本體改寫成
// `into_godot_result(function_name, (|| original_body)())`。因此本體其實位於一個
// 回傳 `GodotBindResult<Dictionary<Variant, Variant>>` 的 closure 內，可以使用 `Ok`
// 建立成功值，也可以用 `?` 從該 closure 提前回傳錯誤；外層方法仍回傳 Godot 所需的 Dictionary。
#[godot_result(with = into_godot_result)]
#[godot_api]
impl GameSession {
    #[func]
    pub fn parse_and_insert_game_data(
        &mut self,
        units: GString,
        skills: GString,
        equipments: GString,
        objects: GString,
    ) -> Dictionary<Variant, Variant> {
        board::ecs_logic::loader::parse_and_insert_game_data(
            &mut self.world,
            GameDataToml {
                units: &units.to_string(),
                skills: &skills.to_string(),
                equipments: &equipments.to_string(),
                objects: &objects.to_string(),
            },
        )?;
        Ok(Dictionary::new())
    }

    #[func]
    pub fn spawn_level(
        &mut self,
        level_name: GString,
        level_toml: GString,
    ) -> Dictionary<Variant, Variant> {
        board::ecs_logic::spawner::spawn_level(
            &mut self.world,
            &level_toml.to_string(),
            &level_name.to_string(),
        )?;
        let result = level_state_dictionary(&mut self.world)?;
        Ok(result)
    }

    #[func]
    pub fn get_level_state(&mut self) -> Dictionary<Variant, Variant> {
        let result = level_state_dictionary(&mut self.world)?;
        Ok(result)
    }

    #[func]
    pub fn get_available_skills(&mut self) -> Dictionary<Variant, Variant> {
        let skills = board::ecs_logic::skill::get_available_skills(&mut self.world)?
            .into_iter()
            .map(available_skill_dictionary)
            .collect::<Array<Dictionary<Variant, Variant>>>();
        let mut result = Dictionary::new();
        result.set("skills", &skills);
        Ok(result)
    }

    #[func]
    pub fn get_remaining_turn_units(&mut self) -> Dictionary<Variant, Variant> {
        let config = get_resource::<LevelConfig>(&self.world, "get_remaining_turn_units")?.clone();
        let turn_order = board::ecs_logic::turn::get_turn_order(&self.world)?.clone();
        let mut units: HashMap<Occupant, UnitBundle> = get_all_units(&mut self.world)?
            .into_values()
            .map(|unit| (unit.occupant, unit))
            .collect();
        let turn_units = turn_order.entries[turn_order.current_index..]
            .iter()
            .map(|entry| {
                let unit = units.remove(&entry.occupant).ok_or_else(|| {
                    Error::from(BoardError::OccupantNotFound {
                        occupant: entry.occupant,
                    })
                })?;
                unit_bundle_dictionary(unit, &config)
            })
            .collect::<board::error::Result<Array<Dictionary<Variant, Variant>>>>()?;
        let mut result = Dictionary::new();
        result.set("units", &turn_units);
        Ok(result)
    }

    #[func]
    pub fn can_delay_current_unit(&mut self) -> Dictionary<Variant, Variant> {
        let can_delay = board::ecs_logic::turn::can_delay_current_unit(&mut self.world)?;
        let mut result = Dictionary::new();
        result.set("can_delay", can_delay);
        Ok(result)
    }

    #[func]
    pub fn get_current_unit_reachable_positions(&mut self) -> Dictionary<Variant, Variant> {
        let turn_order = board::ecs_logic::turn::get_turn_order(&self.world)?;
        let occupant = board::ecs_logic::turn::get_current_unit(turn_order)?;
        let unit = get_all_units(&mut self.world)?
            .into_values()
            .find(|unit| unit.occupant == occupant)
            .ok_or_else(|| Error::from(BoardError::OccupantNotFound { occupant }))?;
        let UnitBundle {
            unit: _,
            position: _,
            occupant: _,
            occupant_type_name: _,
            unit_faction: _,
            skills: _,
            equipped_items: _,
            attributes,
            action_state,
        } = unit;
        let movement_range =
            movement_range_metadata(attributes.movement_point.0 as i64, action_state);
        let moves = board::ecs_logic::movement::get_reachable_positions(&mut self.world, occupant)?
            .into_iter()
            .map(|(position, info)| reachable_position_dictionary(position, info))
            .collect::<Array<Dictionary<Variant, Variant>>>();
        let mut result = Dictionary::new();
        result.set("moves", &moves);
        result.set("normal_range_cost", movement_range.normal_range_cost);
        result.set("movement_cost_used", movement_range.movement_cost_used);
        Ok(result)
    }

    #[func]
    pub fn start_battle(&mut self) -> Dictionary<Variant, Variant> {
        board::ecs_logic::turn::start_new_round(&mut self.world)?;
        Ok(Dictionary::new())
    }

    #[func]
    pub fn get_skill_targetable_positions(
        &mut self,
        skill_name: GString,
    ) -> Dictionary<Variant, Variant> {
        let skill_name = skill_name.to_string();
        let positions =
            board::ecs_logic::skill::get_skill_targetable_positions(&mut self.world, &skill_name)?
                .into_iter();
        let positions = positions_array(positions);
        let mut result = Dictionary::new();
        result.set("positions", &positions);
        Ok(result)
    }

    #[func]
    pub fn get_skill_affected_positions(
        &mut self,
        skill_name: GString,
        target_position: Vector2i,
    ) -> Dictionary<Variant, Variant> {
        let target_position = position_from_vector(target_position, "目標位置")?;
        let skill_name = skill_name.to_string();
        let board::ecs_logic::skill::PreviewAffectedPositions {
            all_positions,
            filtered_positions: _,
        } = board::ecs_logic::skill::get_skill_affected_positions(
            &mut self.world,
            &skill_name,
            target_position,
        )?;
        let positions = positions_array(all_positions);
        let mut result = Dictionary::new();
        result.set("positions", &positions);
        Ok(result)
    }

    #[func]
    pub fn execute_skill(
        &mut self,
        skill_name: GString,
        target_positions: Array<Vector2i>,
    ) -> Dictionary<Variant, Variant> {
        let positions = target_positions
            .iter_shared()
            .map(|target| position_from_vector(target, "技能目標座標"))
            .collect::<GodotBindResult<Vec<_>>>()?;
        let skill_name = skill_name.to_string();
        let config = get_resource::<LevelConfig>(&self.world, "execute_skill")?.clone();
        let entries =
            board::ecs_logic::skill::execute_skill(&mut self.world, &skill_name, &positions)?;
        board::ecs_logic::battle_log::append_skill_log(&mut self.world, &entries)?;
        // 死者 despawn 後即無法從 World 查得，因此直接採用 core 回傳的移除清單，
        // 不在此以前後快照差集反推。
        let removed_unit_ids = board::ecs_logic::turn::resolve_deaths(&mut self.world)?
            .into_iter()
            .map(|id| id as i64)
            .collect::<Array<i64>>();
        let outcome = board::ecs_logic::level_outcome::resolve_level_outcome(&mut self.world)?;
        let units = collect_unit_dictionaries(&mut self.world, &config)?;
        let objects = collect_object_dictionaries(&mut self.world)?;
        let entries = entries
            .into_iter()
            .map(effect_entry_dictionary)
            .collect::<Array<Dictionary<Variant, Variant>>>();
        let mut result = Dictionary::new();
        result.set("entries", &entries);
        result.set("units", &units);
        result.set("removed_unit_ids", &removed_unit_ids);
        result.set("objects", &objects);
        result.set("outcome", &outcome_dictionary(outcome));
        Ok(result)
    }

    #[func]
    pub fn move_current_unit(&mut self, target: Vector2i) -> Dictionary<Variant, Variant> {
        let target = position_from_vector(target, "移動目標座標")?;
        board::ecs_logic::movement::plan_move(&mut self.world, target)?;
        let advance_result = board::ecs_logic::movement::advance_move(&mut self.world)?;
        let path_walked = match advance_result {
            board::ecs_logic::movement::AdvanceMoveResult::Completed {
                path_walked,
                cost: _,
            }
            | board::ecs_logic::movement::AdvanceMoveResult::Interrupted {
                path_walked,
                cost: _,
            } => path_walked,
        };
        let mut result = action_result_dictionary(&mut self.world)?;
        result.set("path_walked", &positions_array(path_walked));
        Ok(result)
    }

    #[func]
    pub fn end_current_turn(&mut self) -> Dictionary<Variant, Variant> {
        board::ecs_logic::turn::end_current_turn(&mut self.world)?;
        Ok(action_result_dictionary(&mut self.world)?)
    }

    #[func]
    pub fn delay_current_unit(&mut self, target_index: i64) -> Dictionary<Variant, Variant> {
        let target_index = usize::try_from(target_index).map_err(|_| GodotBindError::Ffi {
            error_code: "InvalidDelay",
            message: format!("延後目標索引不可為負數：{target_index}"),
        })?;
        let TurnOrder {
            round: _,
            entries: _,
            current_index,
        } = board::ecs_logic::turn::get_turn_order(&self.world)?;
        let absolute_target_index = *current_index + target_index;
        board::ecs_logic::turn::delay_current_unit(&mut self.world, absolute_target_index)?;
        Ok(action_result_dictionary(&mut self.world)?)
    }
}

/// 取出 Unit 的 occupant ID，持有 Object 時視為資料錯誤
fn occupant_unit_id(occupant: Occupant) -> board::error::Result<ID> {
    match occupant {
        Occupant::Unit(id) => Ok(id),
        Occupant::Object(id) => Err(DataError::InvalidComponent {
            name: "Occupant".to_string(),
            note: format!("UnitBundle 必須持有 Unit occupant，實際持有 Object({id})"),
        }
        .into()),
    }
}

/// 取出 Object 的 occupant ID，持有 Unit 時視為資料錯誤
fn occupant_object_id(occupant: Occupant) -> board::error::Result<ID> {
    match occupant {
        Occupant::Object(id) => Ok(id),
        Occupant::Unit(id) => Err(DataError::InvalidComponent {
            name: "Occupant".to_string(),
            note: format!("ObjectBundle 必須持有 Object occupant，實際持有 Unit({id})"),
        }
        .into()),
    }
}

fn position_from_vector(position: Vector2i, position_name: &str) -> GodotBindResult<Position> {
    if position.x < 0 || position.y < 0 {
        return Err(GodotBindError::Ffi {
            error_code: ERROR_CODE_INVALID_POSITION,
            message: format!(
                "{position_name}不可為負數：({}, {})",
                position.x, position.y
            ),
        });
    }
    Ok(Position {
        x: position.x as usize,
        y: position.y as usize,
    })
}

fn position_to_vector(position: Position) -> Vector2i {
    Vector2i::new(position.x as i32, position.y as i32)
}

fn positions_array(positions: impl IntoIterator<Item = Position>) -> Array<Vector2i> {
    positions.into_iter().map(position_to_vector).collect()
}

fn available_skill_dictionary(
    skill: board::ecs_logic::skill::AvailableSkill,
) -> Dictionary<Variant, Variant> {
    let board::ecs_logic::skill::AvailableSkill { name, cost, usable } = skill;
    let mut value = Dictionary::new();
    value.set("name", name);
    value.set("cost", cost as i64);
    value.set("usable", usable);
    value
}

fn reachable_position_dictionary(
    position: Position,
    info: board::logic::movement::ReachableInfo,
) -> Dictionary<Variant, Variant> {
    let board::logic::movement::ReachableInfo {
        cost,
        prev,
        passthrough_only,
    } = info;
    let mut value = Dictionary::new();
    value.set("position", position_to_vector(position));
    value.set("cost", cost as i64);
    value.set("previous", position_to_vector(prev));
    value.set("passthrough_only", passthrough_only);
    value
}

fn collect_unit_dictionaries(
    world: &mut World,
    config: &LevelConfig,
) -> board::error::Result<Array<Dictionary<Variant, Variant>>> {
    get_all_units(world)?
        .into_values()
        .map(|unit| unit_bundle_dictionary(unit, config))
        .collect()
}

fn unit_bundle_dictionary(
    unit: UnitBundle,
    config: &LevelConfig,
) -> board::error::Result<Dictionary<Variant, Variant>> {
    let UnitBundle {
        unit: _,
        position,
        occupant,
        occupant_type_name,
        unit_faction,
        skills: _,
        equipped_items: _,
        attributes,
        action_state,
    } = unit;
    let id = occupant_unit_id(occupant)?;
    let faction = match config.factions.get(&unit_faction.0) {
        Some(faction) => faction,
        None => {
            return Err(DataError::InvalidComponent {
                name: "UnitFaction".to_string(),
                note: format!(
                    "單位 {id} 的 faction_id {} 不存在於關卡的 factions 設定中",
                    unit_faction.0
                ),
            }
            .into());
        }
    };
    let (action_state, movement_cost_used) = match action_state {
        ActionState::Moved { cost } => ("moved", cost as i64),
        // Done 不保留移動花費。此處固定填 0 是刻意決定，非待辦：
        // 前端不讀此欄，不需改成 Option 或省略鍵。
        ActionState::Done => ("done", 0),
    };

    let mut faction_color: Array<i64> = Array::new();
    for channel in faction.color {
        faction_color.push(channel as i64);
    }
    // 屬性放進獨立的 sub-dictionary：`attribute_fields` 的欄位名是 core 的對外契約，
    // 平鋪到同一層會讓新增屬性有機會靜默覆蓋 id/name/x/y 等既有鍵。
    let mut attribute_values: Dictionary<Variant, Variant> = Dictionary::new();
    for (name, attribute) in attribute_fields(&attributes) {
        attribute_values.set(name, attribute as i64);
    }

    let mut value = Dictionary::new();
    value.set("id", id as i64);
    value.set("name", occupant_type_name.0);
    value.set("faction_id", unit_faction.0 as i64);
    value.set("faction_name", faction.name.clone());
    value.set("faction_color", &faction_color);
    value.set("x", position.x as i64);
    value.set("y", position.y as i64);
    value.set("attributes", &attribute_values);
    value.set("action_state", action_state);
    value.set("movement_cost_used", movement_cost_used);
    Ok(value)
}

fn level_state_dictionary(world: &mut World) -> board::error::Result<Dictionary<Variant, Variant>> {
    let board = *get_resource::<Board>(world, "level_state_dictionary")?;
    let config = get_resource::<LevelConfig>(world, "level_state_dictionary")?.clone();
    let (victory_conditions, defeat_conditions) = {
        let outcomes = get_resource::<EndConditionConfig>(world, "level_state_dictionary")?;
        (
            outcome_keys(&outcomes.victory),
            outcome_keys(&outcomes.defeat),
        )
    };
    let units = collect_unit_dictionaries(world, &config)?;
    let objects = collect_object_dictionaries(world)?;

    let mut result = Dictionary::new();
    result.set("name", config.name);
    result.set("board_width", board.width as i64);
    result.set("board_height", board.height as i64);
    result.set("victory_conditions", &victory_conditions);
    result.set("defeat_conditions", &defeat_conditions);
    result.set("units", &units);
    result.set("objects", &objects);
    Ok(result)
}

fn action_result_dictionary(
    world: &mut World,
) -> board::error::Result<Dictionary<Variant, Variant>> {
    let config = get_resource::<LevelConfig>(world, "action_result_dictionary")?.clone();
    let units = collect_unit_dictionaries(world, &config)?;
    let objects = collect_object_dictionaries(world)?;
    let outcome = board::ecs_logic::level_outcome::resolve_level_outcome(world)?;
    let removed_unit_ids = Array::<i64>::new();
    let mut result = Dictionary::new();
    result.set("units", &units);
    result.set("objects", &objects);
    result.set("removed_unit_ids", &removed_unit_ids);
    result.set("outcome", &outcome_dictionary(outcome));
    Ok(result)
}

fn collect_object_dictionaries(
    world: &mut World,
) -> board::error::Result<Array<Dictionary<Variant, Variant>>> {
    get_all_objects(world)?
        .into_values()
        .map(object_query_dictionary)
        .collect()
}

fn object_query_dictionary(
    object: ObjectQueryResult,
) -> board::error::Result<Dictionary<Variant, Variant>> {
    let ObjectQueryResult {
        bundle,
        blocks_sight: _,
        blocks_sound: _,
        hazardous: _,
    } = object;
    let ObjectBundle {
        object: _,
        position,
        occupant,
        occupant_type_name,
        terrain_movement_cost: _,
        contact_effects: _,
    } = bundle;
    let id = occupant_object_id(occupant)?;

    let mut value = Dictionary::new();
    value.set("id", id as i64);
    value.set("name", occupant_type_name.0);
    value.set("x", position.x as i64);
    value.set("y", position.y as i64);
    Ok(value)
}

fn outcome_dictionary(outcome: LevelOutcome) -> Dictionary<Variant, Variant> {
    let mut value = Dictionary::new();
    match outcome {
        LevelOutcome::Undetermined => value.set("status", "undetermined"),
        LevelOutcome::Victory(key) => {
            value.set("status", "victory");
            value.set("key", key);
        }
        LevelOutcome::Defeat(key) => {
            value.set("status", "defeat");
            value.set("key", key);
        }
    }
    value
}

struct MovementRangeMetadata {
    normal_range_cost: i64,
    movement_cost_used: i64,
}

fn movement_range_metadata(
    movement_point: i64,
    action_state: ActionState,
) -> MovementRangeMetadata {
    let movement_cost_used = match action_state {
        ActionState::Moved { cost } => cost as i64,
        ActionState::Done => movement_point * 2,
    };
    MovementRangeMetadata {
        normal_range_cost: movement_point,
        movement_cost_used,
    }
}

fn effect_entry_dictionary(entry: EffectEntry) -> Dictionary<Variant, Variant> {
    let EffectEntry {
        caster,
        skill_name,
        target,
        check,
        check_detail: _,
        effect,
    } = entry;
    let mut value = Dictionary::new();
    value.set("caster_id", caster as i64);
    value.set("skill_name", skill_name);
    match target {
        CheckTarget::Unit(id) => {
            value.set("target_type", "unit");
            value.set("target_id", id as i64);
        }
        CheckTarget::Position(position) => {
            value.set("target_type", "position");
            value.set("target_x", position.x as i64);
            value.set("target_y", position.y as i64);
        }
    }
    let (check_type, critical) = match check {
        CheckResult::Auto => ("auto", false),
        CheckResult::Hit { crit } => ("hit", crit),
        CheckResult::Block { crit } => ("block", crit),
        CheckResult::Evade => ("evade", false),
        CheckResult::Resisted => ("resisted", false),
        CheckResult::Affected => ("affected", false),
    };
    value.set("check", check_type);
    value.set("critical", critical);
    match effect {
        ResolvedEffect::NoEffect => {
            value.set("effect_type", "none");
        }
        ResolvedEffect::HpChange {
            raw_amount,
            final_amount,
        } => {
            value.set("effect_type", "hp_change");
            value.set("raw_amount", raw_amount as i64);
            value.set("final_amount", final_amount as i64);
        }
        ResolvedEffect::SpawnObject { object_type } => {
            value.set("effect_type", "spawn_object");
            value.set("object_type", object_type);
        }
        ResolvedEffect::ApplyBuff(buff_name) => {
            value.set("effect_type", "apply_buff");
            value.set("buff_name", buff_name);
        }
    }
    value
}

fn outcome_keys(branches: &OutcomeBranches) -> Array<GString> {
    branches
        .iter()
        .map(|(key, _)| GString::from(key.as_str()))
        .collect()
}

fn error_dictionary(error_code: &str, message: String) -> Dictionary<Variant, Variant> {
    let mut result = Dictionary::new();
    result.set("ok", false);
    result.set("error_code", error_code);
    result.set("error", message);
    result
}

/// 把 core 的錯誤映射成 FFI 錯誤碼字串
///
/// 這些字串是 godot 前端讀的 FFI 契約，屬於 adapter 的職責：core 不該知道
/// adapter 要輸出什麼，因此刻意不上推到 `board::error`。
///
/// match 刻意保持 exhaustive 且不寫 `_` 萬用臂：core 新增 error variant 時
/// 此處編譯失敗，即為「請補上對應錯誤碼」的提醒。
fn error_code(error: &Error) -> &'static str {
    match error.kind() {
        ErrorKind::Load(value) => match value {
            LoadError::ParseError(_) => "ParseError",
            LoadError::DeserializeError { .. } => "DeserializeError",
            LoadError::SerializeError { .. } => "SerializeError",
        },
        ErrorKind::Data(value) => match value {
            DataError::InternalError { .. } => "InternalError",
            DataError::MissingResource { .. } => "MissingResource",
            DataError::ResourceAlreadyExists { .. } => "ResourceAlreadyExists",
            DataError::MissingComponent { .. } => "MissingComponent",
            DataError::InvalidComponent { .. } => "InvalidComponent",
            DataError::IDGenerationFailed => "IDGenerationFailed",
            DataError::UnitTypeNotFound { .. } => "UnitTypeNotFound",
            DataError::EquipmentTypeNotFound { .. } => "EquipmentTypeNotFound",
            DataError::ObjectTypeNotFound { .. } => "ObjectTypeNotFound",
        },
        ErrorKind::Board(value) => match value {
            BoardError::OutOfBoard { .. } => "OutOfBoard",
            BoardError::Unreachable { .. } => "Unreachable",
            BoardError::NoActiveUnit => "NoActiveUnit",
            BoardError::OccupantNotFound { .. } => "OccupantNotFound",
            BoardError::InvalidDelay { .. } => "InvalidDelay",
            BoardError::InvalidSkillTarget { .. } => "InvalidSkillTarget",
            BoardError::WrongTargetCount { .. } => "WrongTargetCount",
            BoardError::OutOfRange { .. } => "OutOfRange",
            BoardError::NoLineOfSight { .. } => "NoLineOfSight",
            BoardError::TargetFilterMismatch { .. } => "TargetFilterMismatch",
            BoardError::NoUnitAtTarget { .. } => "NoUnitAtTarget",
            BoardError::DuplicateTarget { .. } => "DuplicateTarget",
            BoardError::TargetCountFull { .. } => "TargetCountFull",
        },
        ErrorKind::Deployment(value) => match value {
            DeploymentError::PositionNotDeployable { .. } => "PositionNotDeployable",
            DeploymentError::MaxPlayerUnitsReached { .. } => "MaxPlayerUnitsReached",
            DeploymentError::NothingToUndeploy { .. } => "NothingToUndeploy",
        },
        ErrorKind::Unit(value) => match value {
            UnitError::SkillNotFound { .. } => "SkillNotFound",
            UnitError::InsufficientActionPoint { .. } => "InsufficientActionPoint",
            UnitError::InsufficientMp { .. } => "InsufficientMp",
            UnitError::InsufficientReactionPoint { .. } => "InsufficientReactionPoint",
            UnitError::EmptySkillEffects { .. } => "EmptySkillEffects",
        },
        ErrorKind::Reaction(value) => match value {
            ReactionError::NoPendingReactions => "NoPendingReactions",
            ReactionError::ReactorNotFound { .. } => "ReactorNotFound",
        },
    }
}
