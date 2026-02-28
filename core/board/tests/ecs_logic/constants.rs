pub const SKILL_WARRIOR: &str = "warrior-passive";
pub const SKILL_MELEE: &str = "melee-attack";
pub const UNIT_TYPE_WARRIOR: &str = "warrior";
pub const UNIT_TYPE_MAGE: &str = "mage";
pub const OBJECT_TYPE_WALL: &str = "wall";
pub const OBJECT_TYPE_PIT: &str = "pit";

/// 最小技能 TOML：包含一個被動技能（給 warrior 用）和一個主動技能
pub const SKILLS_TOML: &str = r#"
[[skills]]
name = "warrior-passive"
mp_change = 0
min_range = 0
max_range = 0
tags = []
allows_movement_after = false

[skills.trigger]
type = "Passive"

[[skills.effects]]
type = "AttributeModify"
attribute = "Hp"

[skills.effects.mechanic]
type = "Guaranteed"

[skills.effects.target_mode]
type = "SingleTarget"
filter = "Caster"

[skills.effects.formula]
type = "Fixed"
value = 100

[[skills.effects]]
type = "AttributeModify"
attribute = "Movement"

[skills.effects.mechanic]
type = "Guaranteed"

[skills.effects.target_mode]
type = "SingleTarget"
filter = "Caster"

[skills.effects.formula]
type = "Fixed"
value = 50

[[skills]]
name = "mage-passive"
mp_change = 0
min_range = 0
max_range = 0
tags = []
allows_movement_after = false

[skills.trigger]
type = "Passive"

[[skills.effects]]
type = "AttributeModify"
attribute = "Hp"

[skills.effects.mechanic]
type = "Guaranteed"

[skills.effects.target_mode]
type = "SingleTarget"
filter = "Caster"

[skills.effects.formula]
type = "Fixed"
value = 80

[[skills.effects]]
type = "AttributeModify"
attribute = "Movement"

[skills.effects.mechanic]
type = "Guaranteed"

[skills.effects.target_mode]
type = "SingleTarget"
filter = "Caster"

[skills.effects.formula]
type = "Fixed"
value = 50

[[skills]]
name = "melee-attack"
mp_change = 0
min_range = 1
max_range = 1
tags = []
allows_movement_after = false

[skills.trigger]
type = "Active"

[[skills.effects]]
type = "HpModify"
style = "Physical"

[skills.effects.mechanic]
type = "HitBased"
hit_bonus = 80
crit_rate = 5

[skills.effects.target_mode]
type = "SingleTarget"
filter = "Enemy"

[skills.effects.formula]
type = "Fixed"
value = 10
"#;

/// 最小單位 TOML：包含一個 warrior 單位類型
pub const UNITS_TOML: &str = r#"
[[units]]
name = "warrior"
skills = ["warrior-passive", "melee-attack"]

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
hp_modify = 0

[[objects]]
name = "pit"
movement_cost = 0
blocks_sight = false
blocks_sound = false
hp_modify = -10000
"#;
