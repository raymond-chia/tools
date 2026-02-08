//! Loader 相關的資料結構定義

/// TODO
// 有狀態變化才要重新計算血量最大值
use crate::alias::{Coord, MovementCost, SkillName, TypeName};
use crate::component::Position;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

// ============================================================================
// 屬性系統 (Attribute System)
// ============================================================================

/// 角色屬性類型（根據 README-設計機制.md:72）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, EnumIter)]
pub enum Attribute {
    Hp,
    Mp,
    Initiative,
    Hit,
    Evasion,
    Block,
    BlockProtection,
    PhysicalAttack,
    MagicalAttack,
    MagicalDc,
    Fortitude,
    Reflex,
    Will,
    Movement,
    OpportunityAttacks,
}

// ============================================================================
// 基礎列舉 (Base Enums)
// ============================================================================

/// 屬性來源（用於 ValueFormula）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttributeSource {
    /// 施放者的屬性
    Caster,
    /// 目標的屬性
    Target,
}

/// 傷害類型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttackStyle {
    /// 物理傷害
    Physical,
    /// 魔法傷害
    Magical,
}

/// 檢定類型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SaveType {
    /// 強韌
    Fortitude,
    /// 反射
    Reflex,
    /// 意志
    Will,
}

// ============================================================================
// 技能系統 (Skill System)
// ============================================================================

/// 數值計算公式（用於傷害、屬性修正等）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ValueFormula {
    /// 固定數值
    Fixed { value: i32 },
    /// 屬性倍率計算
    Attribute {
        source: AttributeSource,
        attribute: Attribute,
        multiplier: f32,
    },
}

/// 判定機制（如何判定成功）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Mechanic {
    /// 命中機制（命中 + 1d100 vs 閃避 + 格擋 + 1d100）
    HitBased { hit_bonus: i32, crit_rate: i32 },
    /// DC 檢定機制（DC + 1d100 vs 強韌/反射/意志 + 1d100）
    DcBased { dc: i32, save_type: SaveType },
    /// 必中/必定生效（無判定）
    Guaranteed,
}

/// 目標過濾條件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TargetFilter {
    /// 所有單位
    All,
    /// 所有單位（不含施放者）
    AllExcludingCaster,
    /// 只針對敵人
    Enemy,
    /// 只針對友軍（含施放者）
    Ally,
    /// 只針對友軍（不含施放者）
    AllyExcludingCaster,
    /// 只針對施放者自己
    Caster,
}

/// AOE 形狀
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AoeShape {
    /// 菱形（曼哈頓距離）
    Diamond { radius: Coord },
    /// 十字形
    Cross { length: Coord },
    /// 直線
    Line { length: Coord },
    /// 長方形
    Rectangle { width: Coord, height: Coord },
}

/// 目標模式（影響誰）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TargetMode {
    /// 單一目標
    SingleTarget { filter: TargetFilter },
    /// 多個獨立目標
    MultiTarget {
        count: u32,
        allow_duplicate: bool,
        filter: TargetFilter,
    },
    /// 範圍
    Area {
        aoe_shape: AoeShape,
        targets_unit: bool, // true=以單位為中心，false=以地面為中心
        filter: TargetFilter,
    },
}

/// 觸發事件類型
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerEvent {
    /// 主動技能（需要手動施放）
    #[default]
    Active,

    /// 永久生效（屬性修正）
    Passive,

    /// 擁有者的回合結束時
    TurnEnd,

    /// 被攻擊時（任何回合都可能觸發）
    OnBeingAttacked { attacker_filter: TargetFilter },

    /// 相鄰單位移動時（固定 1 格距離，任何回合都可能觸發）
    OnAdjacentUnitMove { unit_filter: TargetFilter },
}

/// 技能效果類型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SkillEffect {
    /// HP 修正效果（負數 = 傷害，正數 = 治療）
    HpModify {
        mechanic: Mechanic,
        target_mode: TargetMode,
        formula: ValueFormula,
        style: AttackStyle,
    },
    /// 屬性修正效果（buff/debuff/被動技能）
    AttributeModify {
        mechanic: Mechanic,
        target_mode: TargetMode,
        formula: ValueFormula,
        attribute: Attribute,
        duration: Option<i32>, // None=永久，Some(n)=n 回合
    },
    /// 位移效果
    Push {
        mechanic: Mechanic,
        target_mode: TargetMode,
        distance: Coord,
    },
}

/// 技能類型定義
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillType {
    pub name: SkillName,
    pub mp_change: i32, // MP 消耗（0 表示無消耗，負數表示消耗）
    pub min_range: Coord,
    pub max_range: Coord,
    pub tags: Vec<String>,
    pub allows_movement_after: bool,
    pub effects: Vec<SkillEffect>,
    pub trigger: TriggerEvent,
}

// ============================================================================
// 單位系統 (Unit System)
// ============================================================================

/// 單位類型定義
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnitType {
    pub name: String,
    pub skills: Vec<SkillName>,
}

// ============================================================================
// 物件系統 (Object System)
// ============================================================================

/// 物件類型定義
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectType {
    pub name: String,
    pub movement_cost: MovementCost,
    pub blocks_sight: bool,
    pub blocks_sound: bool,
    pub hp_modify: i32, // 負數：傷害，正數：補血
}

// ============================================================================
// 關卡系統 (Level System)
// ============================================================================

/// 單位配置（關卡中的單位放置）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnitPlacement {
    pub position: Position,
    pub unit_type_name: TypeName,
}

/// 物件配置（關卡中的物件放置）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectPlacement {
    pub position: Position,
    pub object_type_name: TypeName,
}

/// 關卡類型定義
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelType {
    pub name: String,
    pub board_width: Coord,
    pub board_height: Coord,
    pub max_player_units: u32,
    pub player_placement_positions: Vec<Position>,
    pub enemy_units: Vec<UnitPlacement>,
    pub object_placements: Vec<ObjectPlacement>,
}
