//! 裝備定義、配裝驗證與能力加總；只在作者輸入邊界解析。
use crate::authoring::{EquipmentDef, EquipmentSlot, UnitType};
use crate::{GameError, error};
use std::collections::HashMap;

#[derive(Clone)]
pub(crate) struct EquipmentStats {
    pub(crate) hp: i32,
    pub(crate) physical_power: i32,
    pub(crate) magical_power: i32,
    pub(crate) block: i32,
    pub(crate) block_reduction: i32,
    pub(crate) equipment: crate::model::EquipmentView,
}

pub(crate) fn definitions(
    entries: Vec<EquipmentDef>,
) -> Result<HashMap<String, EquipmentDef>, GameError> {
    let mut result = HashMap::new();
    for entry in entries {
        if entry.id.trim().is_empty()
            || [
                entry.hp,
                entry.physical_power,
                entry.magical_power,
                entry.block,
                entry.block_reduction,
            ]
            .iter()
            .any(|value| *value < 0)
        {
            return Err(error::invalid_equipment(format!(
                "裝備 {} 的 ID 不可空白，能力加值不可為負數",
                entry.id
            )));
        }
        if result.insert(entry.id.clone(), entry).is_some() {
            return Err(error::invalid_equipment("裝備 ID 重複".to_owned()));
        }
    }
    Ok(result)
}

pub(crate) fn resolve(
    unit: &UnitType,
    entries: &HashMap<String, EquipmentDef>,
) -> Result<EquipmentStats, GameError> {
    let mut hp = i64::from(unit.hp);
    let mut physical = i64::from(unit.physical_power);
    let mut magical = i64::from(unit.magical_power);
    let mut block = 0i64;
    let mut reduction = 0i64;
    let mut hands = 0;
    let equipment = crate::model::EquipmentView {
        main_hand: unit.main_hand.clone(),
        off_hand: unit.off_hand.clone(),
        armor: unit.armor.clone(),
        accessory: unit.accessory.clone(),
    };
    for (id, slot) in [
        (&unit.main_hand, EquipmentSlot::OneHand),
        (&unit.off_hand, EquipmentSlot::OneHand),
        (&unit.armor, EquipmentSlot::Armor),
        (&unit.accessory, EquipmentSlot::Accessory),
    ] {
        if id.is_empty() {
            continue;
        }
        let entry = entries.get(id).ok_or_else(|| {
            error::invalid_equipment(format!("單位 {} 引用不存在的裝備 {id}", unit.id))
        })?;
        if entry.slot != slot
            && !(slot == EquipmentSlot::OneHand && entry.slot == EquipmentSlot::TwoHand)
        {
            return Err(error::invalid_equipment(format!(
                "單位 {} 的裝備 {id} 放在錯誤欄位",
                unit.id
            )));
        }
        hands += match entry.slot {
            EquipmentSlot::OneHand => 1,
            EquipmentSlot::TwoHand => 2,
            _ => 0,
        };
        hp += i64::from(entry.hp);
        physical += i64::from(entry.physical_power);
        magical += i64::from(entry.magical_power);
        block += i64::from(entry.block);
        reduction += i64::from(entry.block_reduction);
    }
    if hands > 2 {
        return Err(error::invalid_equipment(format!(
            "單位 {} 的手持裝備超過兩隻手",
            unit.id
        )));
    }
    let number = |value| i32::try_from(value).map_err(|_| error::numeric_range(&unit.id));
    Ok(EquipmentStats {
        hp: number(hp)?,
        physical_power: number(physical)?,
        magical_power: number(magical)?,
        block: number(block)?,
        block_reduction: number(reduction)?,
        equipment,
    })
}
