use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Deserialize, Serialize, EnumString, Display, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Tag {
    // 主動; 被動
    Active,
    Passive,
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
    Magic,
    Heal,
    Fire,
}

#[derive(Debug, Clone, Deserialize, Serialize, EnumString, Display, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TargetType {
    Caster,
    Ally,
    AllyExcludeCaster,
    Enemy,
    Any,
    AnyExcludeCaster,
}

#[derive(Debug, Clone, Deserialize, Serialize, EnumString, Display, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Shape {
    Point,
    Circle(usize),
    Rectangle(usize, usize),
    Line(usize),
    Cone(usize, f32),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
}

/// 技能資料結構
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Skill {
    #[serde(default)]
    pub tags: Vec<Tag>,
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

/// 實作 Skill 的預設值
impl Default for Skill {
    fn default() -> Self {
        Self {
            tags: vec![],
            range: (0, 0),
            cost: 0,
            hit_rate: None,
            crit_rate: None,
            effects: vec![],
        }
    }
}
