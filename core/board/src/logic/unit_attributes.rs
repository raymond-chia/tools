//! 單位屬性計算邏輯

use crate::alias::SkillName;
use crate::error::{Result, UnitError};
use crate::loader_schema::{
    Attribute, AttributeSource, BuffEffect, Mechanic, SkillEffect, SkillType, TargetFilter,
    TargetMode, TriggerEvent, ValueFormula,
};
use std::collections::HashMap;

/// 計算出的單位屬性（所有 15 個屬性）
#[derive(Debug, Default)]
pub struct CalculatedAttributes {
    pub hp: i32,
    pub mp: i32,
    pub initiative: i32,
    pub hit: i32,
    pub evasion: i32,
    pub block: i32,
    pub block_protection: i32,
    pub physical_attack: i32,
    pub magical_attack: i32,
    pub magical_dc: i32,
    pub fortitude: i32,
    pub reflex: i32,
    pub will: i32,
    pub movement: i32,
    pub opportunity_attacks: i32,
}

/// 計算單位屬性
///
/// # 計算流程
/// 1. 第一階段：累加所有 Fixed 值
///    - 被動技能 (TriggerEvent::Passive) 的 ValueFormula::Fixed
///    - Buff 的 ValueFormula::Fixed
/// 2. 第二階段：應用所有倍率效果（基於第一階段的結果）
///    - 被動技能的 ValueFormula::Attribute 倍率（替換）
///    - Buff 的 ValueFormula::Attribute 倍率（替換）
///
/// # 參數
/// - `skill_names`: 單位習得的技能列表
/// - `buffs`: 臨時效果列表（buff/debuff）
/// - `skill_map`: 技能名稱到技能類型定義的 HashMap
///
/// # 返回值
/// - Ok(CalculatedAttributes): 計算出的屬性
/// - Err(UnitError::SkillNotFound): 技能未找到
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
                    formula: ValueFormula::Fixed { value },
                    attribute,
                    // 確保其他設定沒問題
                    mechanic: Mechanic::Guaranteed,
                    target_mode:
                        TargetMode::SingleTarget {
                            filter: TargetFilter::Caster,
                        },
                    duration: None,
                } => {
                    fixed_effects.push((attribute, *value));
                }
                SkillEffect::AttributeModify {
                    formula:
                        ValueFormula::Attribute {
                            source: AttributeSource::Caster,
                            multiplier,
                            ..
                        },
                    attribute,
                    // 確保其他設定沒問題
                    mechanic: Mechanic::Guaranteed,
                    target_mode:
                        TargetMode::SingleTarget {
                            filter: TargetFilter::Caster,
                        },
                    duration: None,
                } => {
                    // 假設 formula.attribute == attribute
                    multiplier_effects.push((attribute, *multiplier));
                }
                _ => {}
            }
        }
    }

    // 收集所有 Buff 效果
    for buff in buffs {
        match &buff.formula {
            ValueFormula::Fixed { value } => {
                fixed_effects.push((&buff.attribute, *value));
            }
            ValueFormula::Attribute {
                source: AttributeSource::Caster,
                multiplier,
                ..
            } => {
                // 假設 formula.attribute == buff.attribute
                multiplier_effects.push((&buff.attribute, *multiplier));
            }
            _ => {}
        }
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

/// 從 CalculatedAttributes 中讀取指定屬性的值
fn get_attribute_value(attributes: &CalculatedAttributes, attribute: &Attribute) -> i32 {
    match attribute {
        Attribute::Hp => attributes.hp,
        Attribute::Mp => attributes.mp,
        Attribute::Initiative => attributes.initiative,
        Attribute::Hit => attributes.hit,
        Attribute::Evasion => attributes.evasion,
        Attribute::Block => attributes.block,
        Attribute::BlockProtection => attributes.block_protection,
        Attribute::PhysicalAttack => attributes.physical_attack,
        Attribute::MagicalAttack => attributes.magical_attack,
        Attribute::MagicalDc => attributes.magical_dc,
        Attribute::Fortitude => attributes.fortitude,
        Attribute::Reflex => attributes.reflex,
        Attribute::Will => attributes.will,
        Attribute::Movement => attributes.movement,
        Attribute::OpportunityAttacks => attributes.opportunity_attacks,
    }
}

/// 設定 CalculatedAttributes 中的指定屬性值
fn set_attribute_value(attributes: &mut CalculatedAttributes, attribute: &Attribute, value: i32) {
    match attribute {
        Attribute::Hp => attributes.hp = value,
        Attribute::Mp => attributes.mp = value,
        Attribute::Initiative => attributes.initiative = value,
        Attribute::Hit => attributes.hit = value,
        Attribute::Evasion => attributes.evasion = value,
        Attribute::Block => attributes.block = value,
        Attribute::BlockProtection => attributes.block_protection = value,
        Attribute::PhysicalAttack => attributes.physical_attack = value,
        Attribute::MagicalAttack => attributes.magical_attack = value,
        Attribute::MagicalDc => attributes.magical_dc = value,
        Attribute::Fortitude => attributes.fortitude = value,
        Attribute::Reflex => attributes.reflex = value,
        Attribute::Will => attributes.will = value,
        Attribute::Movement => attributes.movement = value,
        Attribute::OpportunityAttacks => attributes.opportunity_attacks = value,
    }
}

/// 修改 CalculatedAttributes 中的指定屬性
fn add_attribute_value(attributes: &mut CalculatedAttributes, attribute: &Attribute, value: i32) {
    match attribute {
        Attribute::Hp => attributes.hp += value,
        Attribute::Mp => attributes.mp += value,
        Attribute::Initiative => attributes.initiative += value,
        Attribute::Hit => attributes.hit += value,
        Attribute::Evasion => attributes.evasion += value,
        Attribute::Block => attributes.block += value,
        Attribute::BlockProtection => attributes.block_protection += value,
        Attribute::PhysicalAttack => attributes.physical_attack += value,
        Attribute::MagicalAttack => attributes.magical_attack += value,
        Attribute::MagicalDc => attributes.magical_dc += value,
        Attribute::Fortitude => attributes.fortitude += value,
        Attribute::Reflex => attributes.reflex += value,
        Attribute::Will => attributes.will += value,
        Attribute::Movement => attributes.movement += value,
        Attribute::OpportunityAttacks => attributes.opportunity_attacks += value,
    }
}
