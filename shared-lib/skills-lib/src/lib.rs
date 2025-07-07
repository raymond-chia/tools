use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use strum_macros::{Display, EnumIter, EnumString};

pub type DEGREE = u16;
pub type SkillID = String;

/// 技能資料結構
#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq)]
pub struct Skill {
    #[serde(default)]
    pub tags: BTreeSet<Tag>,
    #[serde(default)]
    pub range: (usize, usize),
    #[serde(default)]
    pub cost: u16,
    #[serde(default)]
    pub hit_rate: Option<u16>,
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
    Passive,
    Active,
    // 範圍
    Single,
    Area,
    // 距離
    Caster,
    Melee,
    Ranged,
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
    #[default]
    Any,
    AnyExcludeCaster,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default, EnumString, Display, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Shape {
    #[default]
    Point,
    Circle(usize),
    Rectangle(usize, usize),
    Line(usize),
    Cone(usize, DEGREE),
}

#[derive(Debug, Deserialize, Serialize, Clone, EnumIter, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Effect {
    Hp {
        target_type: TargetType,
        shape: Shape,
        value: i32,
    },
    Burn {
        target_type: TargetType,
        shape: Shape,
        duration: u16,
    },
    MovePoints {
        target_type: TargetType,
        shape: Shape,
        value: i32,
        duration: i32, // -1 代表永久
    },
}
