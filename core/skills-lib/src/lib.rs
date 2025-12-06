use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use strum_macros::{Display, EnumIter, EnumString};

pub type Degree = u16;
pub type SkillID = String;

/// 技能資料結構
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Skill {
    #[serde(default = "default_tags")]
    pub tags: BTreeSet<Tag>,
    #[serde(default)]
    pub range: (usize, usize),
    #[serde(default)]
    pub cost: i32,
    #[serde(default)]
    pub accuracy: Option<i32>,
    #[serde(default)]
    pub crit_rate: Option<u16>,
    #[serde(default)]
    pub effects: Vec<Effect>,
}

#[derive(
    Debug,
    Deserialize,
    Serialize,
    Clone,
    EnumString,
    Display,
    EnumIter,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Tag {
    // 主動; 被動
    BasicPassive,
    Passive,
    Active,
    // 範圍
    Single,
    Area,
    // 距離
    Caster,
    Melee,
    Ranged,
    // 物理或者魔法
    Physical,
    Magical,
    // 特性
    Attack,
    Beneficial,
    BodyControl,
    MindControl,
    // 其他
    Heal,
    Fire,
}

#[derive(
    Debug, Deserialize, Serialize, Clone, Default, EnumString, Display, EnumIter, PartialEq,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TargetType {
    Caster,
    Ally,
    AllyExcludeCaster,
    Enemy,
    AnyUnit,
    #[default]
    Any,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default, EnumString, Display, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Shape {
    #[default]
    Point,
    Circle(usize),
    Line(usize),
    Cone(usize, Degree),
}

#[derive(Debug, Deserialize, Serialize, Clone, EnumIter, Display, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
#[strum(serialize_all = "snake_case")]
pub enum Effect {
    Hp {
        target_type: TargetType,
        shape: Shape,
        value: i32,
    },
    Mp {
        target_type: TargetType,
        shape: Shape,
        value: i32,
    },
    MaxHp {
        target_type: TargetType,
        shape: Shape,
        value: i32,
        duration: i32, // -1 代表永久
    },
    MaxMp {
        target_type: TargetType,
        shape: Shape,
        value: i32,
        duration: i32, // -1 代表永久
    },
    Initiative {
        target_type: TargetType,
        shape: Shape,
        value: i32,
        duration: i32, // -1 代表永久
    },
    Evasion {
        target_type: TargetType,
        shape: Shape,
        value: i32,    // 增加的閃避
        duration: i32, // -1 代表永久
    },
    Block {
        target_type: TargetType,
        shape: Shape,
        value: i32,    // 增加的格擋
        duration: i32, // -1 代表永久
    },
    MovePoints {
        target_type: TargetType,
        shape: Shape,
        value: i32,
        duration: i32, // -1 代表永久
    },
    Burn {
        target_type: TargetType,
        shape: Shape,
        duration: i32, // -1 代表永久
    },
    HitAndRun {
        target_type: TargetType,
        shape: Shape,
        duration: i32, // -1 代表永久
    },
    Shove {
        target_type: TargetType,
        shape: Shape,
        distance: usize,
    },
}

impl Default for Skill {
    fn default() -> Self {
        Skill {
            tags: default_tags(),
            range: (0, 0),
            cost: 0,
            accuracy: None,
            crit_rate: None,
            effects: Vec::new(),
        }
    }
}

// macro 產生 match-arm 用來取得某個欄位的參考
// 使用 macro 以減少在多個 getter 中重複列出所有 enum 分支
macro_rules! effect_field_ref {
    ($self:expr, $field:ident) => {
        match $self {
            Effect::Hp { $field, .. }
            | Effect::Mp { $field, .. }
            | Effect::MaxHp { $field, .. }
            | Effect::MaxMp { $field, .. }
            | Effect::Initiative { $field, .. }
            | Effect::Evasion { $field, .. }
            | Effect::Block { $field, .. }
            | Effect::MovePoints { $field, .. }
            | Effect::Burn { $field, .. }
            | Effect::HitAndRun { $field, .. }
            | Effect::Shove { $field, .. } => $field,
        }
    };
}

impl Effect {
    pub fn target_type(&self) -> &TargetType {
        // 透過 macro 取得 target_type 的參考
        effect_field_ref!(self, target_type)
    }

    pub fn is_targeting_unit(&self) -> bool {
        match self.target_type() {
            TargetType::Caster
            | TargetType::Ally
            | TargetType::AllyExcludeCaster
            | TargetType::Enemy
            | TargetType::AnyUnit => true,
            TargetType::Any => false,
        }
    }

    pub fn shape(&self) -> &Shape {
        // 透過 macro 取得 shape 的參考
        effect_field_ref!(self, shape)
    }

    pub fn duration(&self) -> i32 {
        // 對於有 duration 欄位的 variant 回傳其值，其他（立即生效的 variant）回傳 0
        match self {
            Effect::MaxHp { duration, .. }
            | Effect::MaxMp { duration, .. }
            | Effect::Initiative { duration, .. }
            | Effect::Evasion { duration, .. }
            | Effect::Block { duration, .. }
            | Effect::MovePoints { duration, .. }
            | Effect::Burn { duration, .. }
            | Effect::HitAndRun { duration, .. } => *duration,
            Effect::Hp { .. } | Effect::Mp { .. } | Effect::Shove { .. } => 0,
        }
    }
}

fn default_tags() -> BTreeSet<Tag> {
    BTreeSet::from([Tag::Active, Tag::Single, Tag::Melee])
}

#[cfg(test)]
mod tests {
    use super::*;

    // 測試 1：覆蓋所有 TargetType 分支的 target_type() 與 is_targeting_unit()
    #[test]
    fn test_target_type_and_is_targeting_unit_all_targets() {
        // 這些 target 應被視為「指向單位」
        let unit_targets = [
            TargetType::Caster,
            TargetType::Ally,
            TargetType::AllyExcludeCaster,
            TargetType::Enemy,
            TargetType::AnyUnit,
        ];

        for t in &unit_targets {
            let e = Effect::Hp {
                target_type: t.clone(),
                shape: Shape::Point,
                value: 1,
            };
            // target_type() 回傳參考，應等於原始值
            assert_eq!(e.target_type(), t);
            assert!(
                e.is_targeting_unit(),
                "Expected {:?} to be unit-targeting",
                t
            );
        }

        let unit_targets = [TargetType::Any];
        for t in &unit_targets {
            // Any 不應視為指向單位
            let e_any = Effect::Mp {
                target_type: t.clone(),
                shape: Shape::Point,
                value: 0,
            };
            assert_eq!(e_any.target_type(), t);
            assert!(!e_any.is_targeting_unit());
        }
    }

    // 測試 2：shape() 覆蓋 Point / Circle / Line / Cone 分支
    #[test]
    fn test_shape_variants() {
        let s_point = Effect::Mp {
            target_type: Default::default(),
            shape: Shape::Point,
            value: 1,
        };
        assert_eq!(s_point.shape(), &Shape::Point);

        let s_circle = Effect::MaxMp {
            target_type: Default::default(),
            shape: Shape::Circle(3),
            value: 5,
            duration: 2,
        };
        assert_eq!(s_circle.shape(), &Shape::Circle(3));

        let s_line = Effect::Initiative {
            target_type: Default::default(),
            shape: Shape::Line(4),
            value: 2,
            duration: 1,
        };
        assert_eq!(s_line.shape(), &Shape::Line(4));

        let s_cone = Effect::Evasion {
            target_type: Default::default(),
            shape: Shape::Cone(2, 90),
            value: 1,
            duration: 3,
        };
        assert_eq!(s_cone.shape(), &Shape::Cone(2, 90));
    }

    // 測試 3：duration() 覆蓋所有有 duration 的變體，且沒有 duration 的變體回傳 0
    #[test]
    fn test_duration_for_all_variants() {
        // 有 duration 欄位的變體
        assert_eq!(
            Effect::MaxHp {
                target_type: Default::default(),
                shape: Shape::Point,
                value: 1,
                duration: 7
            }
            .duration(),
            7
        );

        assert_eq!(
            Effect::MaxMp {
                target_type: Default::default(),
                shape: Shape::Point,
                value: 1,
                duration: -1
            }
            .duration(),
            -1
        );

        assert_eq!(
            Effect::Initiative {
                target_type: Default::default(),
                shape: Shape::Point,
                value: 1,
                duration: 0
            }
            .duration(),
            0
        );

        assert_eq!(
            Effect::Evasion {
                target_type: Default::default(),
                shape: Shape::Point,
                value: 2,
                duration: 5
            }
            .duration(),
            5
        );

        assert_eq!(
            Effect::Block {
                target_type: Default::default(),
                shape: Shape::Point,
                value: 2,
                duration: 4
            }
            .duration(),
            4
        );

        assert_eq!(
            Effect::MovePoints {
                target_type: Default::default(),
                shape: Shape::Point,
                value: 3,
                duration: 2
            }
            .duration(),
            2
        );

        assert_eq!(
            Effect::Burn {
                target_type: Default::default(),
                shape: Shape::Point,
                duration: 3
            }
            .duration(),
            3
        );

        assert_eq!(
            Effect::HitAndRun {
                target_type: Default::default(),
                shape: Shape::Point,
                duration: -1
            }
            .duration(),
            -1
        );

        // 沒有 duration 欄位的變體應回傳 0
        assert_eq!(
            Effect::Hp {
                target_type: Default::default(),
                shape: Shape::Point,
                value: 1
            }
            .duration(),
            0
        );

        assert_eq!(
            Effect::Mp {
                target_type: Default::default(),
                shape: Shape::Point,
                value: 1
            }
            .duration(),
            0
        );

        assert_eq!(
            Effect::Shove {
                target_type: Default::default(),
                shape: Shape::Point,
                distance: 2
            }
            .duration(),
            0
        );
    }

    // 測試 4：default_tags 的內容檢查（保留原本意義）
    #[test]
    fn test_default_tags_contains_expected() {
        let tags = default_tags();
        assert!(tags.contains(&Tag::Active));
        assert!(tags.contains(&Tag::Single));
        assert!(tags.contains(&Tag::Melee));
    }
}
