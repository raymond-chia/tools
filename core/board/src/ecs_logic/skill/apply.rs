use crate::domain::alias::ID;
use crate::ecs_logic::query::find_entity_by_occupant;
use crate::ecs_logic::{get_component, get_component_mut};
use crate::ecs_types::components::{
    ContactEffects, CurrentHp, MaxHp, Object, ObjectBundle, ObjectMovementCost, Occupant,
    OccupantTypeName,
};
use crate::error::Result;
use crate::logic::id_generator::generate_unique_id;
use crate::logic::skill::skill_execution::{CheckTarget, EffectEntry, ResolvedEffect};
use bevy_ecs::prelude::World;
use std::collections::HashSet;
use std::sync::Arc;

/// 將效果條目寫入 World（HP 變更、物件生成）。
pub(crate) fn apply_effect_entries(
    world: &mut World,
    entries: &[EffectEntry],
    used_ids: &mut HashSet<ID>,
) -> Result<()> {
    for entry in entries {
        match &entry.effect {
            ResolvedEffect::HpChange { final_amount, .. } => {
                let entity = match entry.target {
                    CheckTarget::Unit(id) => find_entity_by_occupant(world, Occupant::Unit(id))?,
                    CheckTarget::Position(_) => unreachable!("HpChange 不應該有 Position 目標"),
                };
                let mut entity_mut = world.entity_mut(entity);
                let max_hp = get_component!(entity_mut, MaxHp)?.0;
                let mut hp = get_component_mut!(entity_mut, CurrentHp)?;
                hp.0 = (hp.0 + final_amount).min(max_hp);
            }
            ResolvedEffect::SpawnObject { object_type } => {
                let pos = match entry.target {
                    CheckTarget::Position(pos) => pos,
                    CheckTarget::Unit(_) => unreachable!("SpawnObject 不應該有 Unit 目標"),
                };
                let id = generate_unique_id(used_ids)?;
                // TODO 物件的其他屬性（例如 contact_effects）應該從技能效果定義中讀取，而不是寫死
                world.spawn(ObjectBundle {
                    object: Object,
                    position: pos,
                    occupant: Occupant::Object(id),
                    occupant_type_name: OccupantTypeName(object_type.clone()),
                    terrain_movement_cost: ObjectMovementCost(0),
                    contact_effects: ContactEffects(Arc::from([])),
                });
            }
            // TODO 其他效果類型的寫入邏輯
            ResolvedEffect::ApplyBuff(_) | ResolvedEffect::NoEffect => {}
        }
    }
    Ok(())
}
