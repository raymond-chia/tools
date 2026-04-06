use crate::domain::core_types::{
    Area, Attribute, BuffType, CasterOrTarget, ContinuousEffect, Effect, EffectNode, Scaling,
    SkillType, Target, TargetFilter, TargetSelection,
};
use crate::ecs_types::components::*;
use crate::logic::skill::unit_attributes::{calculate_attributes, filter_continuous_effect};
use std::collections::HashMap;

// 技能名稱常數
const SKILL_ACCURACY: &str = "accuracy";
const SKILL_ACCURACY_2: &str = "accuracy_2";
const SKILL_EVASION: &str = "evasion";
const SKILL_BASIC_ATTACK: &str = "basic_attack";
const SKILL_NONEXISTENT: &str = "nonexistent";

/// 建立被動技能
fn create_passive_skill(name: &str, effects: Vec<ContinuousEffect>) -> SkillType {
    SkillType::Passive {
        name: name.to_string(),
        tags: Vec::new(),
        effects,
    }
}

/// 建立主動技能（用於測試「忽略主動技能」）
fn create_active_skill() -> SkillType {
    SkillType::Active {
        name: SKILL_BASIC_ATTACK.to_string(),
        tags: Vec::new(),
        cost: 0,
        target: Target {
            range: (1, 1),
            selection: TargetSelection::Unit,
            selectable_filter: TargetFilter::Enemy,
            count: 1,
            allow_same_target: false,
            area: Area::Single,
        },
        effects: vec![EffectNode::Leaf {
            who: CasterOrTarget::Target,
            effect: Effect::HpEffect {
                scaling: Scaling {
                    source: CasterOrTarget::Caster,
                    source_attribute: Attribute::PhysicalAttack,
                    value_percent: 100,
                },
            },
        }],
    }
}

/// 建立 AttributeFlat 效果
fn flat(attribute: Attribute, value: i32) -> ContinuousEffect {
    ContinuousEffect::AttributeFlat { attribute, value }
}

/// 建立 AttributeScaling 效果（來源與目標同屬性）
fn scaling(attribute: Attribute, value_percent: i32) -> ContinuousEffect {
    ContinuousEffect::AttributeScaling {
        target_attribute: attribute,
        source: CasterOrTarget::Caster,
        source_attribute: attribute,
        value_percent,
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
                    SKILL_ACCURACY.to_string(),
                    create_passive_skill(SKILL_ACCURACY, vec![flat(Attribute::Accuracy, 10)]),
                );
                map
            },
            vec![SKILL_ACCURACY.to_string()],
            vec![],
            AttributeBundle {
                accuracy: Accuracy(10),
                ..Default::default()
            },
        ),
        (
            "單個被動技能固定值，技能庫有多個技能",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_ACCURACY.to_string(),
                    create_passive_skill(SKILL_ACCURACY, vec![flat(Attribute::Accuracy, 10)]),
                );
                map.insert(
                    SKILL_EVASION.to_string(),
                    create_passive_skill(SKILL_EVASION, vec![flat(Attribute::Evasion, 10)]),
                );
                map
            },
            vec![SKILL_ACCURACY.to_string()],
            vec![],
            AttributeBundle {
                accuracy: Accuracy(10),
                ..Default::default()
            },
        ),
        (
            "多技能多屬性修正",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_ACCURACY.to_string(),
                    create_passive_skill(
                        SKILL_ACCURACY,
                        vec![
                            flat(Attribute::Accuracy, 10),
                            flat(Attribute::Evasion, 20),
                            flat(Attribute::Accuracy, 40),
                        ],
                    ),
                );
                map.insert(
                    SKILL_EVASION.to_string(),
                    create_passive_skill(SKILL_EVASION, vec![flat(Attribute::Evasion, 100)]),
                );
                map
            },
            vec![SKILL_ACCURACY.to_string(), SKILL_EVASION.to_string()],
            vec![],
            AttributeBundle {
                accuracy: Accuracy(50),
                evasion: Evasion(120),
                ..Default::default()
            },
        ),
        (
            "忽略主動技能，只計算被動",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_ACCURACY.to_string(),
                    create_passive_skill(SKILL_ACCURACY, vec![flat(Attribute::Accuracy, 10)]),
                );
                map.insert(SKILL_BASIC_ATTACK.to_string(), create_active_skill());
                map
            },
            vec![SKILL_ACCURACY.to_string(), SKILL_BASIC_ATTACK.to_string()],
            vec![],
            AttributeBundle {
                accuracy: Accuracy(10),
                ..Default::default()
            },
        ),
        (
            "正面 Buff 效果",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_ACCURACY.to_string(),
                    create_passive_skill(SKILL_ACCURACY, vec![flat(Attribute::Accuracy, 10)]),
                );
                map
            },
            vec![SKILL_ACCURACY.to_string()],
            vec![flat(Attribute::Accuracy, 20)],
            AttributeBundle {
                accuracy: Accuracy(30),
                ..Default::default()
            },
        ),
        (
            "正面 Buff 效果 (倍率)",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_ACCURACY.to_string(),
                    create_passive_skill(SKILL_ACCURACY, vec![flat(Attribute::Accuracy, 10)]),
                );
                map
            },
            vec![SKILL_ACCURACY.to_string()],
            vec![scaling(Attribute::Accuracy, 200)],
            AttributeBundle {
                accuracy: Accuracy(20),
                ..Default::default()
            },
        ),
        (
            "負面 Buff 效果",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_ACCURACY.to_string(),
                    create_passive_skill(SKILL_ACCURACY, vec![flat(Attribute::Accuracy, 20)]),
                );
                map
            },
            vec![SKILL_ACCURACY.to_string()],
            vec![flat(Attribute::Accuracy, -15)],
            AttributeBundle {
                accuracy: Accuracy(5),
                ..Default::default()
            },
        ),
        (
            "嚴重負面 Buff 效果",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_ACCURACY.to_string(),
                    create_passive_skill(SKILL_ACCURACY, vec![flat(Attribute::Accuracy, 20)]),
                );
                map
            },
            vec![SKILL_ACCURACY.to_string()],
            vec![flat(Attribute::Accuracy, -25)],
            AttributeBundle {
                accuracy: Accuracy(-5),
                ..Default::default()
            },
        ),
        (
            "單一倍率效果",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_ACCURACY.to_string(),
                    create_passive_skill(SKILL_ACCURACY, vec![flat(Attribute::Accuracy, 10)]),
                );
                map.insert(
                    SKILL_ACCURACY_2.to_string(),
                    create_passive_skill(
                        SKILL_ACCURACY_2,
                        vec![
                            scaling(Attribute::Accuracy, 200),
                            flat(Attribute::Accuracy, 10),
                        ],
                    ),
                );
                map
            },
            vec![SKILL_ACCURACY.to_string(), SKILL_ACCURACY_2.to_string()],
            vec![],
            AttributeBundle {
                accuracy: Accuracy(40),
                ..Default::default()
            },
        ),
        (
            "多個倍率效果",
            {
                let mut map = HashMap::new();
                map.insert(
                    SKILL_ACCURACY.to_string(),
                    create_passive_skill(
                        SKILL_ACCURACY,
                        vec![
                            flat(Attribute::Accuracy, 10),
                            scaling(Attribute::Accuracy, 200),
                        ],
                    ),
                );
                map.insert(
                    SKILL_ACCURACY_2.to_string(),
                    create_passive_skill(
                        SKILL_ACCURACY_2,
                        vec![
                            scaling(Attribute::Accuracy, 200),
                            flat(Attribute::Accuracy, 10),
                        ],
                    ),
                );
                map
            },
            vec![SKILL_ACCURACY.to_string(), SKILL_ACCURACY_2.to_string()],
            vec![flat(Attribute::Accuracy, 5)],
            AttributeBundle {
                accuracy: Accuracy(100),
                ..Default::default()
            },
        ),
    ];

    for (desc, skill_map, skill_names, buffs, expected) in test_data {
        let buffs = buffs
            .into_iter()
            .map(|e| BuffType {
                stackable: false,
                while_active: vec![e],
                per_turn_effects: vec![],
                end_conditions: vec![],
            })
            .collect::<Vec<_>>();
        let result = filter_continuous_effect(&skill_names, &buffs, &skill_map);
        let effects = result.expect(&format!("測試 '{}' 應該成功", desc));
        let attrs = calculate_attributes(effects);
        assert_eq!(
            attrs.accuracy.0, expected.accuracy.0,
            "測試 '{}' - 命中不符",
            desc
        );
        assert_eq!(
            attrs.evasion.0, expected.evasion.0,
            "測試 '{}' - 迴避不符",
            desc
        );
    }
}

#[test]
fn test_calculate_attributes_skill_not_found() {
    let skill_map = HashMap::new();

    let skill_names = vec![SKILL_NONEXISTENT.to_string()];
    let result = filter_continuous_effect(&skill_names, &[], &skill_map);
    assert!(result.is_err());
}
