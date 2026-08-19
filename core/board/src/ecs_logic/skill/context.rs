use crate::domain::alias::ID;
use crate::ecs_logic::get_component;
use crate::ecs_logic::query::{find_entity_by_occupant, get_resource, read_attribute_bundle};
use crate::ecs_logic::turn::get_current_unit;
use crate::ecs_types::components::{AttributeBundle, Occupant, Position, UnitFaction};
use crate::ecs_types::resources::TurnOrder;
use crate::error::{BoardError, Result};
use crate::logic::skill::UnitInfo;
use crate::logic::skill::skill_execution::CombatStats;
use bevy_ecs::prelude::{Entity, World};

pub(super) struct ActiveCasterSnapshot {
    pub entity: Entity,
    pub position: Position,
    pub occupant: Occupant,
    pub faction_id: ID,
    pub attributes: AttributeBundle,
}

impl ActiveCasterSnapshot {
    pub fn read(world: &World) -> Result<Self> {
        let turn_order = get_resource::<TurnOrder>(world, "請先呼叫 start_new_round")?;
        let occupant = get_current_unit(turn_order)?;
        let entity = find_entity_by_occupant(world, occupant)?;
        let entity_ref = world.entity(entity);

        Ok(Self {
            entity,
            position: *get_component!(entity_ref, Position)?,
            occupant: *get_component!(entity_ref, Occupant)?,
            faction_id: get_component!(entity_ref, UnitFaction)?.0,
            attributes: read_attribute_bundle(&entity_ref)?,
        })
    }

    pub fn unit_info(&self, alliance_id: ID) -> UnitInfo {
        UnitInfo {
            occupant: self.occupant,
            faction_id: self.faction_id,
            alliance_id,
        }
    }

    pub fn combat_stats(&self, alliance_id: ID) -> CombatStats {
        CombatStats {
            unit_info: self.unit_info(alliance_id),
            attribute: self.attributes.clone(),
        }
    }

    pub fn unit_id(&self) -> Result<ID> {
        match self.occupant {
            Occupant::Unit(id) => Ok(id),
            Occupant::Object(_) => Err(BoardError::NoActiveUnit.into()),
        }
    }
}
