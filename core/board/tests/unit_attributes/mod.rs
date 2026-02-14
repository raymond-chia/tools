use board::loader_schema::{
    Attribute, AttributeSource, BuffEffect, Mechanic, SkillEffect, SkillType, TargetFilter,
    TargetMode, TriggerEvent, ValueFormula,
};
use board::logic::unit_attributes::{CalculatedAttributes, calculate_attributes};
use std::collections::HashMap;

// 技能名稱常數
const SKILL_STRENGTH: &str = "strength";
const SKILL_CONSTITUTION: &str = "constitution";
const SKILL_PASSIVE: &str = "passive";
const SKILL_ACTIVE: &str = "active";
const SKILL_BASE_ATTACK: &str = "base_attack";
const SKILL_BONUS_ATTACK: &str = "bonus_attack";
const SKILL_NONEXISTENT: &str = "nonexistent";

/// 建立測試用的技能
fn create_test_skill(name: &str, trigger: TriggerEvent, effects: Vec<SkillEffect>) -> SkillType {
    SkillType {
        name: name.to_string(),
        trigger,
        effects,
        ..Default::default()
    }
}

/// 建立 AttributeModify 效果（永久）
/// 參考 editor\src\tabs\skill_tab.rs
fn attribute_modify_permanent(attribute: Attribute, formula: ValueFormula) -> SkillEffect {
    SkillEffect::AttributeModify {
        mechanic: Mechanic::Guaranteed,
        target_mode: TargetMode::SingleTarget {
            filter: TargetFilter::Caster,
        },
        formula,
        attribute,
        duration: None,
    }
}

#[test]
fn test_calculate_attributes() {
    // 測試資料結構：(說明, 技能庫, 技能列表, 臨時效果, 預期結果)
    let test_data = vec![
        (
            "單個被動技能固定值",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_STRENGTH.to_string(),
                    create_test_skill(
                        SKILL_STRENGTH,
                        TriggerEvent::Passive,
                        vec![attribute_modify_permanent(
                            Attribute::PhysicalAttack,
                            ValueFormula::Fixed { value: 10 },
                        )],
                    ),
                );
                map
            },
            vec![SKILL_STRENGTH.to_string()],
            vec![],
            CalculatedAttributes {
                physical_attack: 10,
                ..Default::default()
            },
        ),
        (
            "多技能多屬性修正",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_STRENGTH.to_string(),
                    create_test_skill(
                        SKILL_STRENGTH,
                        TriggerEvent::Passive,
                        vec![
                            attribute_modify_permanent(
                                Attribute::PhysicalAttack,
                                ValueFormula::Fixed { value: 10 },
                            ),
                            attribute_modify_permanent(
                                Attribute::Hp,
                                ValueFormula::Fixed { value: 20 },
                            ),
                            attribute_modify_permanent(
                                Attribute::PhysicalAttack,
                                ValueFormula::Fixed { value: 40 },
                            ),
                        ],
                    ),
                );
                map.insert(
                    SKILL_CONSTITUTION.to_string(),
                    create_test_skill(
                        SKILL_CONSTITUTION,
                        TriggerEvent::Passive,
                        vec![attribute_modify_permanent(
                            Attribute::Hp,
                            ValueFormula::Fixed { value: 100 },
                        )],
                    ),
                );
                map
            },
            vec![SKILL_STRENGTH.to_string(), SKILL_CONSTITUTION.to_string()],
            vec![],
            CalculatedAttributes {
                physical_attack: 50,
                hp: 120,
                ..Default::default()
            },
        ),
        (
            "忽略主動技能，只計算被動",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_PASSIVE.to_string(),
                    create_test_skill(
                        SKILL_PASSIVE,
                        TriggerEvent::Passive,
                        vec![attribute_modify_permanent(
                            Attribute::Hit,
                            ValueFormula::Fixed { value: 10 },
                        )],
                    ),
                );
                map.insert(
                    SKILL_ACTIVE.to_string(),
                    create_test_skill(
                        SKILL_ACTIVE,
                        TriggerEvent::Active,
                        vec![attribute_modify_permanent(
                            Attribute::Hit,
                            ValueFormula::Fixed { value: 20 },
                        )],
                    ),
                );
                map
            },
            vec![SKILL_PASSIVE.to_string(), SKILL_ACTIVE.to_string()],
            vec![],
            CalculatedAttributes {
                hit: 10,
                ..Default::default()
            },
        ),
        (
            "正面效果",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_PASSIVE.to_string(),
                    create_test_skill(
                        SKILL_PASSIVE,
                        TriggerEvent::Passive,
                        vec![attribute_modify_permanent(
                            Attribute::Evasion,
                            ValueFormula::Fixed { value: 10 },
                        )],
                    ),
                );
                map
            },
            vec![SKILL_PASSIVE.to_string()],
            vec![BuffEffect {
                attribute: Attribute::Evasion,
                formula: ValueFormula::Fixed { value: 20 },
                duration: 10,
            }],
            CalculatedAttributes {
                evasion: 30,
                ..Default::default()
            },
        ),
        (
            "正面效果 (倍率)",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_PASSIVE.to_string(),
                    create_test_skill(
                        SKILL_PASSIVE,
                        TriggerEvent::Passive,
                        vec![attribute_modify_permanent(
                            Attribute::Evasion,
                            ValueFormula::Fixed { value: 10 },
                        )],
                    ),
                );
                map
            },
            vec![SKILL_PASSIVE.to_string()],
            vec![BuffEffect {
                attribute: Attribute::Evasion,
                formula: ValueFormula::Attribute {
                    source: AttributeSource::Caster,
                    attribute: Attribute::Evasion,
                    multiplier: 200,
                },
                duration: 10,
            }],
            CalculatedAttributes {
                evasion: 20,
                ..Default::default()
            },
        ),
        (
            "負面效果",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_PASSIVE.to_string(),
                    create_test_skill(
                        SKILL_PASSIVE,
                        TriggerEvent::Passive,
                        vec![attribute_modify_permanent(
                            Attribute::Evasion,
                            ValueFormula::Fixed { value: 20 },
                        )],
                    ),
                );
                map
            },
            vec![SKILL_PASSIVE.to_string()],
            vec![BuffEffect {
                attribute: Attribute::Evasion,
                formula: ValueFormula::Fixed { value: -15 },
                duration: 10,
            }],
            CalculatedAttributes {
                evasion: 5,
                ..Default::default()
            },
        ),
        (
            "嚴重負面效果",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_PASSIVE.to_string(),
                    create_test_skill(
                        SKILL_PASSIVE,
                        TriggerEvent::Passive,
                        vec![attribute_modify_permanent(
                            Attribute::Evasion,
                            ValueFormula::Fixed { value: 20 },
                        )],
                    ),
                );
                map
            },
            vec![SKILL_PASSIVE.to_string()],
            vec![BuffEffect {
                attribute: Attribute::Evasion,
                formula: ValueFormula::Fixed { value: -25 },
                duration: 10,
            }],
            CalculatedAttributes {
                evasion: -5,
                ..Default::default()
            },
        ),
        (
            "單一倍率效果",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_BASE_ATTACK.to_string(),
                    create_test_skill(
                        SKILL_BASE_ATTACK,
                        TriggerEvent::Passive,
                        vec![attribute_modify_permanent(
                            Attribute::PhysicalAttack,
                            ValueFormula::Fixed { value: 10 },
                        )],
                    ),
                );
                map.insert(
                    SKILL_BONUS_ATTACK.to_string(),
                    create_test_skill(
                        SKILL_BONUS_ATTACK,
                        TriggerEvent::Passive,
                        vec![
                            attribute_modify_permanent(
                                Attribute::PhysicalAttack,
                                ValueFormula::Attribute {
                                    source: AttributeSource::Caster,
                                    attribute: Attribute::PhysicalAttack,
                                    multiplier: 200,
                                },
                            ),
                            attribute_modify_permanent(
                                Attribute::PhysicalAttack,
                                ValueFormula::Fixed { value: 10 },
                            ),
                        ],
                    ),
                );
                map
            },
            vec![
                SKILL_BASE_ATTACK.to_string(),
                SKILL_BONUS_ATTACK.to_string(),
            ],
            vec![],
            CalculatedAttributes {
                physical_attack: 40,
                ..Default::default()
            },
        ),
        (
            "多個倍率效果",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_BASE_ATTACK.to_string(),
                    create_test_skill(
                        SKILL_BASE_ATTACK,
                        TriggerEvent::Passive,
                        vec![
                            attribute_modify_permanent(
                                Attribute::PhysicalAttack,
                                ValueFormula::Fixed { value: 10 },
                            ),
                            attribute_modify_permanent(
                                Attribute::PhysicalAttack,
                                ValueFormula::Attribute {
                                    source: AttributeSource::Caster,
                                    attribute: Attribute::PhysicalAttack,
                                    multiplier: 200,
                                },
                            ),
                        ],
                    ),
                );
                map.insert(
                    SKILL_BONUS_ATTACK.to_string(),
                    create_test_skill(
                        SKILL_BONUS_ATTACK,
                        TriggerEvent::Passive,
                        vec![
                            attribute_modify_permanent(
                                Attribute::PhysicalAttack,
                                ValueFormula::Attribute {
                                    source: AttributeSource::Caster,
                                    attribute: Attribute::PhysicalAttack,
                                    multiplier: 200,
                                },
                            ),
                            attribute_modify_permanent(
                                Attribute::PhysicalAttack,
                                ValueFormula::Fixed { value: 10 },
                            ),
                        ],
                    ),
                );
                map
            },
            vec![
                SKILL_BASE_ATTACK.to_string(),
                SKILL_BONUS_ATTACK.to_string(),
            ],
            vec![BuffEffect {
                attribute: Attribute::PhysicalAttack,
                formula: ValueFormula::Fixed { value: 5 },
                duration: 10,
            }],
            CalculatedAttributes {
                physical_attack: 100,
                ..Default::default()
            },
        ),
    ];

    for (desc, skill_map, skill_names, buffs, expected) in test_data {
        let result = calculate_attributes(&skill_names, &buffs, &skill_map);
        assert!(result.is_ok(), "測試 '{}' 應該成功", desc);

        let attrs = result.unwrap();
        assert_eq!(attrs.hp, expected.hp, "測試 '{}' - HP 不符", desc);
        assert_eq!(
            attrs.physical_attack, expected.physical_attack,
            "測試 '{}' - 物理攻擊不符",
            desc
        );
        assert_eq!(attrs.hit, expected.hit, "測試 '{}' - 命中不符", desc);
        assert_eq!(
            attrs.evasion, expected.evasion,
            "測試 '{}' - 迴避不符",
            desc
        );
    }
}

#[test]
fn test_calculate_attributes_skill_not_found() {
    let skill_map = HashMap::new();

    let result = calculate_attributes(&[SKILL_NONEXISTENT.to_string()], &[], &skill_map);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("技能未找到"));
}
