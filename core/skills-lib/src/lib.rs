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
            Effect::Hp { $field, .. } => $field,
            Effect::Mp { $field, .. } => $field,
            Effect::MaxHp { $field, .. } => $field,
            Effect::MaxMp { $field, .. } => $field,
            Effect::Initiative { $field, .. } => $field,
            Effect::Evasion { $field, .. } => $field,
            Effect::Block { $field, .. } => $field,
            Effect::MovePoints { $field, .. } => $field,
            Effect::Burn { $field, .. } => $field,
            Effect::HitAndRun { $field, .. } => $field,
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
            Effect::Hp { .. } | Effect::Mp { .. } => 0,
        }
    }
}

fn default_tags() -> BTreeSet<Tag> {
    BTreeSet::from([Tag::Active, Tag::Single, Tag::Melee])
}
