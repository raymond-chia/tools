pub const SKILL_WARRIOR: &str = "warrior-passive";
pub const SKILL_MELEE: &str = "melee-attack";
pub const SKILL_WARRIOR_REACTION: &str = "warrior-reaction";
pub const SKILL_WARRIOR_ACTIVE_2: &str = "warrior-active-2";
pub const SKILL_WARRIOR_ACTIVE_4: &str = "warrior-active-4";
pub const UNIT_TYPE_WARRIOR: &str = "warrior";
pub const UNIT_TYPE_MAGE: &str = "mage";
pub const OBJECT_TYPE_WALL: &str = "wall";
pub const OBJECT_TYPE_PIT: &str = "pit";
pub const OBJECT_TYPE_SWAMP: &str = "swamp";

pub const SKILLS_TOML: &str = r#"
[[skills]]

[skills.Passive]
name = "warrior-passive"
tags = []

[[skills.Passive.effects]]

[skills.Passive.effects.AttributeFlat]
attribute = "Hp"
value = 100

[[skills.Passive.effects]]

[skills.Passive.effects.AttributeFlat]
attribute = "MovementPoint"
value = 50

[[skills]]

[skills.Passive]
name = "mage-passive"
tags = []

[[skills.Passive.effects]]

[skills.Passive.effects.AttributeFlat]
attribute = "Hp"
value = 80

[[skills.Passive.effects]]

[skills.Passive.effects.AttributeFlat]
attribute = "MovementPoint"
value = 50

[[skills]]

[skills.Active]
name = "melee-attack"
tags = []
cost = 0

[skills.Active.target]
range = [1, 1]
selection = "Unit"
selectable_filter = "Enemy"
count = 1
allow_same_target = false
area = "Single"

[[skills.Active.effects]]

[skills.Active.effects.Leaf]
who = "Target"

[skills.Active.effects.Leaf.effect.HpEffect.scaling]
source = "Caster"
source_attribute = "PhysicalAttack"
value_percent = 100

[[skills]]

[skills.Active]
name = "warrior-active-2"
tags = []
cost = 2

[skills.Active.target]
range = [1, 2]
selection = "Unit"
selectable_filter = "Enemy"
count = 1
allow_same_target = false
area = "Single"

[[skills.Active.effects]]

[skills.Active.effects.Leaf]
who = "Target"

[skills.Active.effects.Leaf.effect.HpEffect.scaling]
source = "Caster"
source_attribute = "PhysicalAttack"
value_percent = 120

[[skills]]

[skills.Active]
name = "warrior-active-4"
tags = []
cost = 4

[skills.Active.target]
range = [1, 1]
selection = "Unit"
selectable_filter = "Enemy"
count = 2
allow_same_target = false
area = "Single"

[[skills.Active.effects]]

[skills.Active.effects.Leaf]
who = "Target"

[skills.Active.effects.Leaf.effect.HpEffect.scaling]
source = "Caster"
source_attribute = "PhysicalAttack"
value_percent = 60

[[skills]]

[skills.Reaction]
name = "warrior-reaction"
tags = []
cost = 1

[skills.Reaction.triggering_unit]
source_range = [1, 1]
source_filter = "Enemy"
trigger = "AttackOfOpportunity"

[[skills.Reaction.effects]]

[skills.Reaction.effects.Leaf]
who = "Target"

[skills.Reaction.effects.Leaf.effect.HpEffect.scaling]
source = "Caster"
source_attribute = "PhysicalAttack"
value_percent = 100
"#;

/// 最小單位 TOML：包含一個 warrior 單位類型
pub const UNITS_TOML: &str = r#"
[[units]]
name = "warrior"
skills = ["warrior-passive", "melee-attack", "warrior-active-2", "warrior-active-4", "warrior-reaction"]

[[units]]
name = "mage"
skills = ["mage-passive", "melee-attack"]
"#;

/// 最小物件 TOML：包含一個 wall 物件類型
pub const OBJECTS_TOML: &str = r#"
[[objects]]
name = "wall"
movement_cost = 10000
blocks_sight = true
blocks_sound = true

[[objects]]
name = "pit"
movement_cost = 0
blocks_sight = false
blocks_sound = false

[[objects]]
name = "swamp"
movement_cost = 10
blocks_sight = false
blocks_sound = false
"#;
