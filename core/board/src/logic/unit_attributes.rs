//! 單位屬性計算邏輯

use crate::domain::alias::SkillName;
use crate::domain::core_types::{Attribute, BuffType, ContinuousEffect, SkillType};
use crate::ecs_types::components::*;
use crate::error::{Result, UnitError};
use std::collections::HashMap;

pub fn filter_continuous_effect<'a>(
    skill_names: &'a [SkillName],
    buffs: &'a [BuffType],
    skill_map: &'a HashMap<SkillName, SkillType>,
) -> Result<impl Iterator<Item = &'a ContinuousEffect>> {
    let passives = skill_names
        .iter()
        .map(|name| {
            skill_map.get(name).ok_or_else(|| {
                UnitError::SkillNotFound {
                    skill_name: name.clone(),
                }
                .into()
            })
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|skill| match skill {
            SkillType::Passive { effects, .. } => {
                if effects.is_empty() {
                    None
                } else {
                    Some(effects.iter())
                }
            }
            SkillType::Active { .. } | SkillType::Reaction { .. } => None,
        })
        .flatten();

    let from_buffs = buffs.iter().flat_map(|buff| buff.while_active.iter());

    Ok(passives.chain(from_buffs))
}

/// 計算單位屬性
pub fn calculate_attributes<'a>(
    effects: impl Iterator<Item = &'a ContinuousEffect>,
) -> AttributeBundle {
    let mut attributes = CalculatedAttributes::default();

    // 收集所有被動技能效果
    let mut flat_effects = Vec::new();
    let mut scaling_effects = Vec::new();

    for effect in effects {
        collect_continuous_effect(effect, &mut flat_effects, &mut scaling_effects);
    }

    // 第一階段：累加所有固定值
    for (attribute, value) in flat_effects {
        add_attribute_value(&mut attributes, attribute, value);
    }

    // 第二階段：應用所有倍率效果
    for (attribute, multiplier) in scaling_effects {
        let base_value = get_attribute_value(&attributes, attribute);
        let new_value = (base_value * multiplier) / 100;
        set_attribute_value(&mut attributes, attribute, new_value);
    }

    attributes.into()
}

fn collect_continuous_effect(
    effect: &ContinuousEffect,
    flat_effects: &mut Vec<(Attribute, i32)>,
    scaling_effects: &mut Vec<(Attribute, i32)>,
) {
    match effect {
        ContinuousEffect::AttributeFlat { attribute, value } => {
            flat_effects.push((*attribute, *value));
        }
        ContinuousEffect::AttributeScaling {
            target_attribute,
            value_percent,
            ..
        } => {
            scaling_effects.push((*target_attribute, *value_percent));
        }
        ContinuousEffect::NearbyAllyScaling { .. } | ContinuousEffect::HpRatioScaling { .. } => {
            // TODO
        }
        ContinuousEffect::Perception { .. }
        | ContinuousEffect::DamageToMp { .. }
        | ContinuousEffect::EmitLight { .. }
        | ContinuousEffect::Blinded => {
            // 不影響屬性
        }
    }
}

/// 產生 get/set/add_attribute_value 函數的 macro
macro_rules! define_attribute_accessors {
    ($(($field:ident, $variant:ident)),* $(,)?) => {
        /// 計算出的單位屬性
        #[derive(Debug, Default, Clone)]
        struct CalculatedAttributes {
            $(pub $field: i32,)*
        }

        fn get_attribute_value(attributes: &CalculatedAttributes, attribute: Attribute) -> i32 {
            match attribute {
                $(Attribute::$variant => attributes.$field,)*
            }
        }

        fn set_attribute_value(attributes: &mut CalculatedAttributes, attribute: Attribute, value: i32) {
            match attribute {
                $(Attribute::$variant => attributes.$field = value,)*
            }
        }

        fn add_attribute_value(attributes: &mut CalculatedAttributes, attribute: Attribute, value: i32) {
            match attribute {
                $(Attribute::$variant => attributes.$field += value,)*
            }
        }
    };
}

define_attribute_accessors!(
    (hp, Hp),
    (mp, Mp),
    (initiative, Initiative),
    (accuracy, Accuracy),
    (evasion, Evasion),
    (block, Block),
    (block_protection, BlockProtection),
    (physical_attack, PhysicalAttack),
    (magical_attack, MagicalAttack),
    (magical_dc, MagicalDc),
    (fortitude, Fortitude),
    (reflex, Reflex),
    (will, Will),
    (movement_point, MovementPoint),
    (reaction_point, ReactionPoint),
);

impl From<CalculatedAttributes> for AttributeBundle {
    fn from(attributes: CalculatedAttributes) -> Self {
        AttributeBundle {
            max_hp: MaxHp(attributes.hp),
            current_hp: CurrentHp(attributes.hp),
            max_mp: MaxMp(attributes.mp),
            current_mp: CurrentMp(attributes.mp),
            initiative: Initiative(attributes.initiative),
            accuracy: Accuracy(attributes.accuracy),
            evasion: Evasion(attributes.evasion),
            block: Block(attributes.block),
            block_protection: BlockProtection(attributes.block_protection),
            physical_attack: PhysicalAttack(attributes.physical_attack),
            magical_attack: MagicalAttack(attributes.magical_attack),
            magical_dc: MagicalDc(attributes.magical_dc),
            fortitude: Fortitude(attributes.fortitude),
            reflex: Reflex(attributes.reflex),
            will: Will(attributes.will),
            movement_point: MovementPoint(attributes.movement_point),
            reaction_point: ReactionPoint(attributes.reaction_point),
        }
    }
}
