# 技能系統資料結構重新設計

重新設計技能系統的資料結構，使其能表達 `ignore-skill-summary.md` 中所有技能需求。

## 設計決策

- 效果條件鏈用巢狀結構，每個節點是「條件 → 效果」，可分支
- 位移統一為 ForcedMove，SwapPosition 和 Trample 因語義獨立仍為獨立效果
- 推的碰撞傷害在 ForcedMove 內部處理
- 反應技能的 source_filter 篩選觸發源，觸發源即為反應效果的目標；效果鏈中的 EffectTarget::Target 都鎖定觸發源
- 地面持續效果 = SpawnObject，contact_effects 定義單位接觸時的效果鏈
- EndCondition 之間為 OR 關係
- Buff 內嵌在技能 TOML，AppliedBuff 記錄施法者、目標、繼承的 DcType
- HP 正值 = 治療，負值 = 傷害；倍率用 i32 百分比（100 = 100%）
- Trample 路徑規則：目的地不能有單位或障礙物物件；移動無視中間單位但不能經過障礙物物件；攻擊路徑上經過的敵方單位。內建 hit-based 檢定，對每個經過的敵人獨立判定
- contact_effects 觸發時機：物件產生時、每回合開始時、單位進入時
- 技能可帶多個 tag，用於 EndCondition 篩選和技能使用條件判斷。
- Buff 的 per_turn_effects 在回合結束時觸發

## 資料結構

```rust
enum SkillType {
    Active {
        name: SkillName,
        tags: Vec<SkillTag>,
        cost: u32,
        target: Target,
        effects: Vec<EffectNode>,
    },
    Reaction {
        name: SkillName,
        tags: Vec<SkillTag>,
        cost: u32,
        triggering_unit: TriggeringSource,
        effects: Vec<EffectNode>,
    },
    Passive {
        name: SkillName,
        tags: Vec<SkillTag>,
        effects: Vec<PassiveEffect>,
    },
}

enum SkillTag {
    AllowedDuringGrabbing,
    UsableAfterDoubleMove,
}

struct Target {
    range: u32,
    selection: TargetSelection,
    selectable_filter: TargetFilter,
    count: u32,
    allow_same_target: bool,
    area: Area,
}

enum TargetSelection { Unit, Ground }

enum Area {
    Single,
    Diamond { radius: u32 },
    Cross { length: u32 },
    Line { length: u32 },
}

struct TriggeringSource {
    source_range: u32,
    source_filter: TargetFilter,
    trigger: ReactionTrigger,
}

enum ReactionTrigger {
    AttackOfOpportunity,
    TakesDamage,
}

enum TargetFilter {
    Any,
    AnyExceptCaster,
    Ally,
    AllyExceptCaster,
    Enemy,
    CasterOnly,
}


enum EffectNode {
    Area { area: Area, filter: TargetFilter, nodes: Vec<EffectNode> },
    Branch {
        condition: EffectCondition,
        on_success: Vec<EffectNode>,
        on_failure: Vec<EffectNode>,
    },
    Leaf { who: EffectTarget, effect: Effect },
}

enum EffectTarget { Caster, Target }

enum EffectCondition {
    HitCheck { accuracy_bonus: i32, crit_bonus: i32 },
    DcCheck { dc_type: DcType, dc_bonus: i32 },
}

enum DcType { Fort, Reflex, Will }

enum Effect {
    HpEffect { value_percent: i32 },
    MpEffect { value: i32 },
    ApplyBuff { buff: BuffType },
    ForcedMove { direction: MoveDirection, distance: u32 },
    AllowRemainingMovement,
    SwapPosition,
    Trample { distance: u32, value_percent: i32, accuracy_bonus: i32, crit_bonus: i32 },
    SpawnObject {
        object_type: TypeName,
        duration: Option<u32>,
        contact_effects: Vec<EffectNode>,
    },
}
enum MoveDirection { AwayFromCaster, TowardCaster, AwayFromTarget, TowardTarget }
enum PerceptionType { Auditory }
enum LightType { Bright, Dim, Darkness }

struct BuffType {
    stackable: bool,
    while_active: Vec<WhileActiveEffect>,
    per_turn_effects: Vec<EffectNode>,
    end_conditions: Vec<EndCondition>,
}

struct AppliedBuff {
    def: BuffType,
    caster: Occupant,
    target: Occupant,
    remaining_duration: Option<u32>,
    inherited_dc: Option<DcType>,
}

enum WhileActiveEffect {
    ModifyAttribute { attribute: Attribute, value: i32 },
    EmitLight { light_type: LightType, range: u32 },
    Blinded,
}

enum Attribute { Accuracy, Evasion, Block, BlockProtection, PhysicalAttack, MagicalAttack, MagicalDc, Fortitude, Reflex, Will, MovementPoint, Initiative, ReactionCount }

enum EndCondition {
    Duration(u32),
    TargetSavesPerTurn,         // DC 種類從施加時繼承
    CasterUsesSkillWithoutTag(SkillTag),
    EitherDies,
    EitherMoves,
    TargetMoves,
}

enum PassiveEffect {
    FlatModifier { attribute: Attribute, value: i32 },
    NearbyAllyScaling { range: u32, attribute: Attribute, per_ally_percent: i32, base_percent: i32 },
    HpRatioScaling { attribute: Attribute, min_bonus_percent: i32, step_percent: u32, bonus_per_step: i32, max_bonus_percent: i32 },
    Perception { perception_type: PerceptionType, range: u32 },
    DamageToMp { ratio_percent: i32 },
}
```

## 實作步驟

1. 定義所有 Rust 資料結構（domain 層，不含 ECS）
2. 實作 TOML 反序列化
3. 實作效果鏈執行引擎（依序執行 + 條件分支）
4. 實作各 Effect 的具體邏輯
5. 實作 Buff 系統（施加、持續、回合結束觸發、結束條件檢查）
6. 實作反應系統
7. 實作被動效果計算

## 範例

### 輔助閃光技能

```rust
effects: [
    Area { area: Diamond { radius: 2 }, filter: Any, nodes: [
        Branch {
            condition: DcCheck { dc_type: Reflex, dc_bonus: 0 },
            on_success: [Leaf { who: Target, effect: ApplyBuff {
                buff: BuffType {
                    stackable: false,
                    while_active: [Blinded],
                    per_turn_effects: [],
                    end_conditions: [Duration(2)],
                },
            }}],
            on_failure: [],
        },
    ]},
    Area { area: Diamond { radius: 1 }, filter: Any, nodes: [
        Leaf { who: Target, effect: ApplyBuff {
            buff: BuffType {
                stackable: false,
                while_active: [EmitLight { light_type: Dim, range: 2 }],
                per_turn_effects: [],
                end_conditions: [Duration(4)],
            },
        }},
        Leaf { who: Target, effect: SpawnObject {
            object_type: "dim_light_zone_2",
            duration: Some(4),
            contact_effects: [],
        }},
    ]},
]
```

### 控場火焰技能

```rust
effects: [
    Leaf { who: Target, effect: SpawnObject {
        object_type: "fire_zone",
        duration: Some(2),
        contact_effects: [Branch {
            condition: DcCheck { dc_type: Reflex, dc_bonus: 10 },
            on_success: [Leaf { who: Target, effect: ApplyBuff {
                buff: BuffType {
                    stackable: true,
                    while_active: [EmitLight { light_type: Dim, range: 2 }],
                    per_turn_effects: [Leaf { who: Target, effect: HpEffect {
                        value_percent: -20,
                    }}],
                    end_conditions: [Duration(2)],
                },
            }}],
            on_failure: [],
        }],
    }},
]
```

### 控場黏滯地面技能

```rust
effects: [
    Area { area: Diamond { radius: 1 }, filter: Any, nodes: [
        Leaf { who: Target, effect: SpawnObject {
            object_type: "sticky_zone",
            duration: Some(3),
            contact_effects: [Branch {
                condition: DcCheck { dc_type: Fort, dc_bonus: 0 },
                on_success: [Leaf { who: Target, effect: ApplyBuff {
                    buff: BuffType {
                        stackable: false,
                        while_active: [
                            ModifyAttribute { attribute: MovementPoint, value: -10 },
                            ModifyAttribute { attribute: Evasion, value: -10 },
                        ],
                        per_turn_effects: [],
                        end_conditions: [Duration(1), TargetMoves],
                    },
                }}],
                on_failure: [],
            }],
        }},
    ]},
]
```

### 輔助回魔技能

```rust
effects: [
    Leaf { who: Caster, effect: MpEffect { value: 8 } },
    Area { area: Diamond { radius: 1 }, filter: AnyExceptCaster, nodes: [
        Leaf { who: Target, effect: MpEffect { value: 3 } },
    ]},
]
```
