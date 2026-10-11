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

/// 依序為欄位名稱、欄位可放入的裝備種類與目前裝備 ID（空字串代表未裝備）。
pub(crate) type EquipmentSlots<'a> = [(&'static str, EquipmentSlot, &'a str); 4];

/// 單位配裝欄位的唯一清單。
pub(crate) fn equipment_slots(unit: &UnitType) -> EquipmentSlots<'_> {
    [
        ("main_hand", EquipmentSlot::OneHand, &unit.main_hand),
        ("off_hand", EquipmentSlot::OneHand, &unit.off_hand),
        ("armor", EquipmentSlot::Armor, &unit.armor),
        ("accessory", EquipmentSlot::Accessory, &unit.accessory),
    ]
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
            return Err(error::invalid_equipment_definition(&entry.id));
        }
        if result.insert(entry.id.clone(), entry).is_some() {
            return Err(error::duplicate_equipment_id());
        }
    }
    Ok(result)
}

pub(crate) fn resolve(
    unit: &UnitType,
    entries: &HashMap<String, EquipmentDef>,
) -> Result<EquipmentStats, GameError> {
    resolve_slots(unit, equipment_slots(unit), entries)
}

/// 以指定配裝計算單位能力。
pub(crate) fn resolve_slots(
    unit: &UnitType,
    slots: EquipmentSlots<'_>,
    entries: &HashMap<String, EquipmentDef>,
) -> Result<EquipmentStats, GameError> {
    let mut hp = i64::from(unit.hp);
    let mut physical = i64::from(unit.physical_power);
    let mut magical = i64::from(unit.magical_power);
    let mut block = 0i64;
    let mut reduction = 0i64;
    let mut hands = 0;
    for (_, slot, id) in slots {
        if id.is_empty() {
            continue;
        }
        let entry = entries
            .get(id)
            .ok_or_else(|| error::unknown_equipment(&unit.id, id))?;
        if entry.slot != slot
            && !(slot == EquipmentSlot::OneHand && entry.slot == EquipmentSlot::TwoHand)
        {
            return Err(error::equipment_wrong_slot(&unit.id, id));
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
        return Err(error::too_many_hand_equipment(&unit.id));
    }
    let number = |value| i32::try_from(value).map_err(|_| error::numeric_range(&unit.id));
    let [
        (_, _, main_hand),
        (_, _, off_hand),
        (_, _, armor),
        (_, _, accessory),
    ] = slots;
    Ok(EquipmentStats {
        hp: number(hp)?,
        physical_power: number(physical)?,
        magical_power: number(magical)?,
        block: number(block)?,
        block_reduction: number(reduction)?,
        equipment: crate::model::EquipmentView {
            main_hand: main_hand.to_owned(),
            off_hand: off_hand.to_owned(),
            armor: armor.to_owned(),
            accessory: accessory.to_owned(),
        },
    })
}
