use bevy_ecs::prelude::World;
use board::ecs_logic::query::{get_all_objects, get_all_units, get_resource};
use board::ecs_types::resources::{Board, EndConditionConfig, LevelConfig};
use board::error::{
    BoardError, DataError, DeploymentError, Error, ErrorKind, LoadError, ReactionError, UnitError,
};
use godot::prelude::*;

macro_rules! godot_result {
    ($message:expr, $body:block) => {{
        let result: board::error::Result<Dictionary<Variant, Variant>> = (|| $body)();
        match result {
            Ok(mut result) => {
                result.set("ok", true);
                result
            }
            Err(error) => error_dictionary(error_code(&error), format!("{}: {}", $message, error)),
        }
    }};
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

#[godot_api]
impl GameSession {
    #[func]
    pub fn parse_and_insert_game_data(
        &mut self,
        units: GString,
        skills: GString,
        objects: GString,
    ) -> Dictionary<Variant, Variant> {
        godot_result!("parse_and_insert_game_data", {
            board::ecs_logic::loader::parse_and_insert_game_data(
                &mut self.world,
                &units.to_string(),
                &skills.to_string(),
                &objects.to_string(),
            )?;
            Ok(Dictionary::new())
        })
    }

    #[func]
    pub fn spawn_level(
        &mut self,
        level_name: GString,
        level_toml: GString,
    ) -> Dictionary<Variant, Variant> {
        godot_result!("spawn_level", {
            board::ecs_logic::spawner::spawn_level(
                &mut self.world,
                &level_toml.to_string(),
                &level_name.to_string(),
            )?;
            let board = *get_resource::<Board>(&self.world, "spawn_level")?;
            let config = get_resource::<LevelConfig>(&self.world, "spawn_level")?.clone();
            let mut result = Dictionary::new();
            result.set("name", config.name);
            result.set("board_width", board.width as i64);
            result.set("board_height", board.height as i64);
            let outcomes = get_resource::<EndConditionConfig>(&self.world, "spawn_level")?;
            let victory_conditions = outcome_keys(&outcomes.victory);
            let defeat_conditions = outcome_keys(&outcomes.defeat);
            result.set("victory_conditions", &victory_conditions);
            result.set("defeat_conditions", &defeat_conditions);
            let units = get_all_units(&mut self.world)?
                .into_values()
                .map(|unit| {
                    let mut value = Dictionary::new();
                    value.set("name", unit.occupant_type_name.0);
                    value.set("faction_id", unit.unit_faction.0 as i64);
                    value.set(
                        "faction_name",
                        config
                            .factions
                            .get(&unit.unit_faction.0)
                            .map_or("", |faction| faction.name.as_str()),
                    );
                    value.set("x", unit.position.x as i64);
                    value.set("y", unit.position.y as i64);
                    value.set("max_hp", unit.attributes.max_hp.0 as i64);
                    value.set("current_hp", unit.attributes.current_hp.0 as i64);
                    value.set("max_mp", unit.attributes.max_mp.0 as i64);
                    value.set("current_mp", unit.attributes.current_mp.0 as i64);
                    value.set("initiative", unit.attributes.initiative.0 as i64);
                    value.set("physical_attack", unit.attributes.physical_attack.0 as i64);
                    value.set("magical_attack", unit.attributes.magical_attack.0 as i64);
                    value.set(
                        "physical_accuracy",
                        unit.attributes.physical_accuracy.0 as i64,
                    );
                    value.set(
                        "magical_accuracy",
                        unit.attributes.magical_accuracy.0 as i64,
                    );
                    value.set("fortitude", unit.attributes.fortitude.0 as i64);
                    value.set("agility", unit.attributes.agility.0 as i64);
                    value.set("block", unit.attributes.block.0 as i64);
                    value.set(
                        "block_protection",
                        unit.attributes.block_protection.0 as i64,
                    );
                    value.set("will", unit.attributes.will.0 as i64);
                    value.set("movement_point", unit.attributes.movement_point.0 as i64);
                    value.set(
                        "max_reaction_point",
                        unit.attributes.max_reaction_point.0 as i64,
                    );
                    value.set("reaction_point", unit.attributes.reaction_point.0 as i64);
                    value.set(
                        "flanking_accuracy_bonus",
                        unit.attributes.flanking_accuracy_bonus.0 as i64,
                    );
                    value
                })
                .collect::<Array<Dictionary<Variant, Variant>>>();
            result.set("units", &units);
            let objects = get_all_objects(&mut self.world)?
                .into_values()
                .map(|object| {
                    let mut value = Dictionary::new();
                    value.set("name", object.bundle.occupant_type_name.0);
                    value.set("x", object.bundle.position.x as i64);
                    value.set("y", object.bundle.position.y as i64);
                    value
                })
                .collect::<Array<Dictionary<Variant, Variant>>>();
            result.set("objects", &objects);
            Ok(result)
        })
    }
}

fn outcome_keys(branches: &board::domain::core_types::OutcomeBranches) -> Array<GString> {
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
