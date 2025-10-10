use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use strum_macros::{Display, EnumIter, EnumString};

pub type DEGREE = u16;
pub type SkillID = String;

/// 技能資料結構
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Skill {
    #[serde(default = "default_tags")]
    pub tags: BTreeSet<Tag>,
    #[serde(default)]
    pub range: (usize, usize),
    #[serde(default)]
    pub cost: u16,
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
    MaxHp {
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

impl Effect {
    pub fn target_type(&self) -> &TargetType {
        match self {
            Effect::Hp { target_type, .. } => target_type,
            Effect::MaxHp { target_type, .. } => target_type,
            Effect::Initiative { target_type, .. } => target_type,
            Effect::Evasion { target_type, .. } => target_type,
            Effect::Block { target_type, .. } => target_type,
            Effect::MovePoints { target_type, .. } => target_type,
            Effect::Burn { target_type, .. } => target_type,
            Effect::HitAndRun { target_type, .. } => target_type,
        }
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
        match self {
            Effect::Hp { shape, .. } => shape,
            Effect::MaxHp { shape, .. } => shape,
            Effect::Initiative { shape, .. } => shape,
            Effect::Evasion { shape, .. } => shape,
            Effect::Block { shape, .. } => shape,
            Effect::MovePoints { shape, .. } => shape,
            Effect::Burn { shape, .. } => shape,
            Effect::HitAndRun { shape, .. } => shape,
        }
    }
}

fn default_tags() -> BTreeSet<Tag> {
    BTreeSet::from([Tag::Active, Tag::Single, Tag::Melee])
}
