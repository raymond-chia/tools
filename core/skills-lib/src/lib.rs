use object_lib::ObjectType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use strum_macros::{Display, EnumIter, EnumString};

pub type Degree = u16;
pub type SkillID = String;

/// 攻擊結果
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum AttackResult {
    NoAttack, // 不需要攻擊判定
    Normal,   // 普通命中（1x 傷害）
    Critical, // 爆擊（2x 傷害）
}

/// 防禦結果
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum DefenseResult {
    Hit,             // 命中
    Evaded,          // 閃避
    Blocked,         // 格擋
    GuaranteedHit,   // 完全命中（亂數 > 95）
    GuaranteedEvade, // 完全閃避（亂數 <= 5）
}

/// 豁免結果
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum SaveResult {
    NoSave,  // 不需要豁免
    Success, // 豁免成功
    Failure, // 豁免失敗
}

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
    Default,
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
    #[default]
    Passive,
    Active,
    Basic, // 額外標記，必須與 Passive 一起使用
    // 來源標記
    Character, // 角色本身的技能
    Equipment, // 裝備賦予的技能
    // 距離
    Caster,
    Melee,
    Ranged,
    // 範圍
    Single,
    Area,
    // 物理或者魔法
    Physical,
    Magical,
    // 特性
    Attack,
    Heal,
    Buff,
    Debuff,
    // 物件交互
    Ignite,     // 點燃範圍內的可燃物件
    Extinguish, // 熄滅範圍內的物件
    // 其他
    CanBeReaction,
    Fire,
}

/// 豁免檢定類型
#[derive(
    Debug, Deserialize, Serialize, Default, Clone, EnumString, Display, EnumIter, PartialEq,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SaveType {
    #[default]
    Fortitude, // 強韌：對抗毒素、疾病、物理效果
    Reflex, // 反射：對抗範圍效果、需要閃避的效果
    Will,   // 意志：對抗心靈控制、幻術
}

/// 反應動作觸發條件
#[derive(
    Debug, Deserialize, Serialize, Default, Clone, EnumString, Display, EnumIter, PartialEq,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReactionTrigger {
    #[default]
    OnMove, // 敵人離開相鄰格時（借機攻擊）
    OnAttacked, // 自己被攻擊命中時（反擊）
}

/// 被觸發的技能來源
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TriggeredSkill {
    SkillId { id: SkillID }, // 使用特定技能 ID
    Tag { tag: Tag },        // 使用有此 Tag 的技能
}

impl Default for TriggeredSkill {
    fn default() -> Self {
        TriggeredSkill::Tag {
            tag: Tag::default(),
        }
    }
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
    Accuracy {
        target_type: TargetType,
        shape: Shape,
        value: i32,
        duration: i32, // -1 代表永久
    },
    Evasion {
        target_type: TargetType,
        shape: Shape,
        value: i32,
        duration: i32, // -1 代表永久
    },
    Block {
        target_type: TargetType,
        shape: Shape,
        value: i32,    // 增加的格擋
        duration: i32, // -1 代表永久
    },
    BlockReduction {
        target_type: TargetType,
        shape: Shape,
        value: i32,    // 增加的格擋減傷百分比
        duration: i32, // -1 代表永久
    },
    Flanking {
        target_type: TargetType,
        shape: Shape,
        value: i32,
        duration: i32, // -1 代表永久
    },
    MovePoints {
        target_type: TargetType,
        shape: Shape,
        value: i32,
        duration: i32, // -1 代表永久
    },
    MaxReactions {
        target_type: TargetType,
        shape: Shape,
        value: i32,
        duration: i32, // -1 代表永久
    },
    Reaction {
        target_type: TargetType,
        shape: Shape,
        trigger: ReactionTrigger,        // 觸發條件
        triggered_skill: TriggeredSkill, // 被觸發的技能
        duration: i32,                   // -1 代表永久
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
    Potency {
        target_type: TargetType,
        shape: Shape,
        tag: Tag, // 提升哪種 Tag 技能的效力（如 Fire）
        value: i32,
        duration: i32, // -1 代表永久
    },
    Resistance {
        target_type: TargetType,
        shape: Shape,
        save_type: SaveType,
        value: i32,
        duration: i32, // -1 代表永久
    },
    Burn {
        target_type: TargetType,
        shape: Shape,
        save_type: SaveType,
        duration: i32, // -1 代表永久
    },
    /// 低光視覺：在指定範圍內忽略光照懲罰
    LowLightVision {
        target_type: TargetType,
        shape: Shape,
        range: usize,
        perceive_range: usize,
        duration: i32, // -1 代表永久
    },
    /// 聽覺感知：可透過聽覺定位目標（繞過煙霧，無法繞過牆壁）
    Hearing {
        target_type: TargetType,
        shape: Shape,
        range: usize,
        perceive_range: usize,
        duration: i32, // -1 代表永久
    },
    /// 噪音干擾：範圍內的單位無法被聽覺定位
    Noise {
        target_type: TargetType,
        shape: Shape,
        range: usize,
        duration: i32, // -1 代表永久
    },
    /// 攜帶光源：單位發出光芒
    CarriesLight {
        target_type: TargetType,
        shape: Shape,
        bright_range: usize,
        dim_range: usize,
        duration: i32, // -1 代表永久
    },
    /// 創造物件：在目標位置創造物件
    CreateObject {
        target_type: TargetType,
        shape: Shape,
        object_type: ObjectType,
        duration: i32, // -1 代表永久，> 0 代表臨時
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
            | Effect::Accuracy { $field, .. }
            | Effect::Evasion { $field, .. }
            | Effect::Block { $field, .. }
            | Effect::BlockReduction { $field, .. }
            | Effect::Flanking { $field, .. }
            | Effect::MovePoints { $field, .. }
            | Effect::MaxReactions { $field, .. }
            | Effect::Reaction { $field, .. }
            | Effect::HitAndRun { $field, .. }
            | Effect::Shove { $field, .. }
            | Effect::Potency { $field, .. }
            | Effect::Resistance { $field, .. }
            | Effect::Burn { $field, .. }
            | Effect::LowLightVision { $field, .. }
            | Effect::Hearing { $field, .. }
            | Effect::Noise { $field, .. }
            | Effect::CarriesLight { $field, .. }
            | Effect::CreateObject { $field, .. } => $field,
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

    /// 取得效果的豁免類型（如果需要豁免判定）
    pub fn save_type(&self) -> Option<&SaveType> {
        match self {
            Effect::Burn { save_type, .. } => Some(save_type),
            Effect::Hp { .. }
            | Effect::Mp { .. }
            | Effect::MaxHp { .. }
            | Effect::MaxMp { .. }
            | Effect::Initiative { .. }
            | Effect::Accuracy { .. }
            | Effect::Evasion { .. }
            | Effect::Block { .. }
            | Effect::BlockReduction { .. }
            | Effect::Flanking { .. }
            | Effect::MovePoints { .. }
            | Effect::MaxReactions { .. }
            | Effect::Reaction { .. }
            | Effect::HitAndRun { .. }
            | Effect::Shove { .. }
            | Effect::Potency { .. }
            | Effect::Resistance { .. }
            | Effect::LowLightVision { .. }
            | Effect::Hearing { .. }
            | Effect::Noise { .. }
            | Effect::CarriesLight { .. }
            | Effect::CreateObject { .. } => None,
        }
    }

    pub fn duration(&self) -> i32 {
        // 對於有 duration 欄位的 variant 回傳其值，其他（立即生效的 variant）回傳 0
        match self {
            Effect::MaxHp { duration, .. }
            | Effect::MaxMp { duration, .. }
            | Effect::Initiative { duration, .. }
            | Effect::Accuracy { duration, .. }
            | Effect::Evasion { duration, .. }
            | Effect::Block { duration, .. }
            | Effect::BlockReduction { duration, .. }
            | Effect::Flanking { duration, .. }
            | Effect::MovePoints { duration, .. }
            | Effect::MaxReactions { duration, .. }
            | Effect::Reaction { duration, .. }
            | Effect::HitAndRun { duration, .. }
            | Effect::Potency { duration, .. }
            | Effect::Resistance { duration, .. }
            | Effect::Burn { duration, .. }
            | Effect::LowLightVision { duration, .. }
            | Effect::Hearing { duration, .. }
            | Effect::Noise { duration, .. }
            | Effect::CarriesLight { duration, .. }
            | Effect::CreateObject { duration, .. } => *duration,
            Effect::Hp { .. } | Effect::Mp { .. } | Effect::Shove { .. } => 0,
        }
    }

    /// 減少效果的持續時間（僅對 duration > 0 的效果，永久效果 -1 不減少）
    pub fn decrease_duration(&mut self) {
        match self {
            Effect::MaxHp { duration, .. }
            | Effect::MaxMp { duration, .. }
            | Effect::Initiative { duration, .. }
            | Effect::Accuracy { duration, .. }
            | Effect::Evasion { duration, .. }
            | Effect::Block { duration, .. }
            | Effect::BlockReduction { duration, .. }
            | Effect::Flanking { duration, .. }
            | Effect::MovePoints { duration, .. }
            | Effect::MaxReactions { duration, .. }
            | Effect::Reaction { duration, .. }
            | Effect::HitAndRun { duration, .. }
            | Effect::Potency { duration, .. }
            | Effect::Resistance { duration, .. }
            | Effect::Burn { duration, .. }
            | Effect::LowLightVision { duration, .. }
            | Effect::Hearing { duration, .. }
            | Effect::Noise { duration, .. }
            | Effect::CarriesLight { duration, .. }
            | Effect::CreateObject { duration, .. } => {
                // 只有 duration > 0 時才減少（永久效果 -1 不減少）
                if *duration > 0 {
                    *duration -= 1;
                }
            }
            Effect::Hp { .. } | Effect::Mp { .. } | Effect::Shove { .. } => {
                // 這些效果沒有 duration，不需要處理
            }
        }
    }
}

fn default_tags() -> BTreeSet<Tag> {
    BTreeSet::from([Tag::Passive, Tag::Character, Tag::Caster, Tag::Single])
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
                save_type: SaveType::Reflex,
                duration: 3,
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

    // 測試 6：Resistance 和 Potency 的 duration
    #[test]
    fn test_resistance_and_potency_duration() {
        let resistance = Effect::Resistance {
            target_type: TargetType::Caster,
            shape: Shape::Point,
            save_type: SaveType::Fortitude,
            value: 20,
            duration: -1,
        };
        assert_eq!(resistance.duration(), -1);

        let potency = Effect::Potency {
            target_type: TargetType::Caster,
            shape: Shape::Point,
            tag: Tag::Fire,
            value: 15,
            duration: 5,
        };
        assert_eq!(potency.duration(), 5);
    }

    // 測試 5：Burn 的 save_type 和 duration
    #[test]
    fn test_burn_with_save_type() {
        let burn = Effect::Burn {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            duration: 3,
            save_type: SaveType::Reflex,
        };
        assert_eq!(burn.duration(), 3);
        assert_eq!(burn.target_type(), &TargetType::Enemy);
    }

    // 測試 8：測試 Effect 的 Display trait
    #[test]
    fn test_effect_display() {
        let burn = Effect::Burn {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            duration: 3,
            save_type: SaveType::Reflex,
        };
        assert_eq!(burn.to_string(), "burn");

        let resistance = Effect::Resistance {
            target_type: TargetType::Caster,
            shape: Shape::Point,
            save_type: SaveType::Fortitude,
            value: 10,
            duration: -1,
        };
        assert_eq!(resistance.to_string(), "resistance");
    }
}
