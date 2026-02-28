//! 單位屬性計算邏輯

use crate::domain::alias::SkillName;
use crate::domain::core_types::{Attribute, CalculatedAttributes};
use crate::error::{Result, UnitError};
use crate::loader_schema::{
    AttributeSource, BuffEffect, Mechanic, SkillEffect, SkillType, TargetFilter, TargetMode,
    TriggerEvent, ValueFormula,
};
use std::collections::HashMap;

/// 計算單位屬性
pub fn calculate_attributes(
    skill_names: &[SkillName],
    buffs: &[BuffEffect],
    skill_map: &HashMap<SkillName, SkillType>,
) -> Result<CalculatedAttributes> {
    let mut attributes = CalculatedAttributes::default();

    // 收集所有被動技能效果
    let mut fixed_effects = Vec::new();
    let mut multiplier_effects = Vec::new();

    for skill_name in skill_names {
        let skill = skill_map.get(skill_name).ok_or(UnitError::SkillNotFound {
            skill_name: skill_name.clone(),
        })?;

        if skill.trigger != TriggerEvent::Passive {
            continue;
        }

        for effect in &skill.effects {
            match effect {
                SkillEffect::AttributeModify {
                    formula,
                    attribute,
                    // 確保其他設定沒問題
                    mechanic: Mechanic::Guaranteed,
                    target_mode:
                        TargetMode::SingleTarget {
                            filter: TargetFilter::Caster,
                        },
                    duration: None,
                } => {
                    collect_formula_effect(
                        *attribute,
                        &formula,
                        &mut fixed_effects,
                        &mut multiplier_effects,
                    );
                }
                _ => {}
            }
        }
    }

    // 收集所有 Buff 效果
    for buff in buffs {
        collect_formula_effect(
            buff.attribute,
            &buff.formula,
            &mut fixed_effects,
            &mut multiplier_effects,
        );
    }

    // 第一階段：累加所有 Fixed 值
    for (attribute, value) in fixed_effects {
        add_attribute_value(&mut attributes, attribute, value);
    }

    // 第二階段：應用所有倍率效果
    for (attribute, multiplier) in multiplier_effects {
        let base_value = get_attribute_value(&attributes, attribute);
        let new_value = (base_value * multiplier) / 100;
        set_attribute_value(&mut attributes, attribute, new_value);
    }

    Ok(attributes)
}

fn collect_formula_effect(
    target_attribute: Attribute,
    formula: &ValueFormula,
    fixed_effects: &mut Vec<(Attribute, i32)>,
    multiplier_effects: &mut Vec<(Attribute, i32)>,
) {
    match formula {
        ValueFormula::Fixed { value } => {
            fixed_effects.push((target_attribute, *value));
        }
        ValueFormula::Attribute {
            source: AttributeSource::Caster,
            multiplier,
            ..
        } => {
            // 假設 formula.attribute == buff.attribute
            multiplier_effects.push((target_attribute, *multiplier));
        }
        _ => {}
    }
}

/// 產生 get/set/add_attribute_value 函數的 macro
macro_rules! define_attribute_accessors {
    ($(($field:ident, $variant:ident)),* $(,)?) => {
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
    (hit, Hit),
    (evasion, Evasion),
    (block, Block),
    (block_protection, BlockProtection),
    (physical_attack, PhysicalAttack),
    (magical_attack, MagicalAttack),
    (magical_dc, MagicalDc),
    (fortitude, Fortitude),
    (reflex, Reflex),
    (will, Will),
    (movement, Movement),
    (reaction, Reaction),
);
