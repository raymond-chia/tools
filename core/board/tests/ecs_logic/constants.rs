pub const SKILL_WARRIOR: &str = "warrior-passive";
pub const SKILL_RUBY_BURST: &str = "ruby-burst";
pub const SKILL_MELEE: &str = "melee-attack";
pub const SKILL_WARRIOR_ACTIVE_2: &str = "warrior-active-2";
pub const SKILL_WARRIOR_ACTIVE_4: &str = "warrior-active-4";
pub const SKILL_DIAMOND_AOE: &str = "diamond-aoe-1";
pub const SKILL_SUMMON_WALL_AOE: &str = "summon-wall-aoe";
pub const SKILL_WARRIOR_REACTION: &str = "warrior-reaction";
pub const SKILL_WARRIOR_REACTION_2: &str = "warrior-reaction-2";
pub const SKILL_WARRIOR_COUNTER: &str = "warrior-counter";
pub const SKILL_IRON_SLASH: &str = "iron-slash";
pub const UNIT_TYPE_WARRIOR: &str = "warrior";
pub const UNIT_TYPE_WARRIOR_B: &str = "warrior-b";
pub const UNIT_TYPE_WARRIOR_COUNTER_ONLY: &str = "warrior-counter-only";
pub const UNIT_TYPE_MAGE: &str = "mage";
pub const UNIT_TYPE_SWORD_USER: &str = "sword-user";
pub const UNIT_TYPE_DUAL_WIELDER: &str = "dual-wielder";
pub const UNIT_TYPE_KNIGHT: &str = "knight";
pub const OBJECT_TYPE_WALL: &str = "wall";
pub const OBJECT_TYPE_SPIKE: &str = "spike";
pub const OBJECT_TYPE_SWAMP: &str = "swamp";
pub const OBJECT_TYPE_FOG: &str = "fog";
pub const EQUIPMENT_IRON_SWORD: &str = "iron-sword";
pub const EQUIPMENT_STEEL_SWORD: &str = "steel-sword";
pub const EQUIPMENT_WOODEN_BOW: &str = "wooden-bow";
pub const EQUIPMENT_WOODEN_SHIELD: &str = "wooden-shield";
pub const EQUIPMENT_GREAT_SWORD: &str = "great-sword";
pub const EQUIPMENT_LEATHER_ARMOR: &str = "leather-armor";
pub const EQUIPMENT_GIANT_ARMOR: &str = "giant-armor";
pub const EQUIPMENT_RUBY_RING: &str = "ruby-ring";
pub const EQUIPMENT_ECHO_CHARM: &str = "echo-charm";

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

[[skills.Passive.effects]]

[skills.Passive.effects.AttributeFlat]
attribute = "PhysicalAttack"
value = 10

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
value_percent = -100

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
value_percent = -120

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
value_percent = -60

[[skills]]

[skills.Reaction]
name = "warrior-reaction"
tags = []
cost = 0

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
value_percent = -100

[[skills]]

[skills.Active]
name = "diamond-aoe-1"
tags = []
cost = 0

[skills.Active.target]
range = [1, 2]
selection = "Ground"
selectable_filter = "AllyExceptCaster"
count = 1
allow_same_target = false

[skills.Active.target.area.Diamond]
radius = 1

[[skills.Active.effects]]

[skills.Active.effects.Leaf]
who = "Target"

[skills.Active.effects.Leaf.effect.HpEffect.scaling]
source = "Caster"
source_attribute = "PhysicalAttack"
value_percent = 50

[[skills]]

[skills.Active]
name = "summon-wall-aoe"
tags = []
cost = 0

[skills.Active.target]
range = [0, 2]
selection = "Ground"
selectable_filter = "Any"
count = 1
allow_same_target = false

[skills.Active.target.area.Diamond]
radius = 1

[[skills.Active.effects]]

[skills.Active.effects.Area]
filter = "Any"

[skills.Active.effects.Area.area.Diamond]
radius = 1

[[skills.Active.effects.Area.nodes]]

[skills.Active.effects.Area.nodes.Leaf]
who = "Target"

[skills.Active.effects.Area.nodes.Leaf.effect.SpawnObject]
object_type = "wall"
contact_effects = []

[[skills]]

[skills.Reaction]
name = "warrior-reaction-2"
tags = []
cost = 0

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
value_percent = -50

[[skills]]

[skills.Reaction]
name = "warrior-counter"
tags = []
cost = 0

[skills.Reaction.triggering_unit]
source_range = [1, 1]
source_filter = "Enemy"
trigger = "TakesDamage"

[[skills.Reaction.effects]]

[skills.Reaction.effects.Leaf]
who = "Target"

[skills.Reaction.effects.Leaf.effect.HpEffect.scaling]
source = "Caster"
source_attribute = "PhysicalAttack"
value_percent = -100

[[skills]]

[skills.Passive]
name = "iron-slash"
tags = []

[[skills.Passive.effects]]

[skills.Passive.effects.AttributeFlat]
attribute = "PhysicalAttack"
value = 5

[[skills]]

[skills.Passive]
name = "steel-slash"
tags = []

[[skills.Passive.effects]]

[skills.Passive.effects.AttributeFlat]
attribute = "PhysicalAttack"
value = 10

[[skills]]

[skills.Passive]
name = "leather-armor-passive"
tags = []

[[skills.Passive.effects]]

[skills.Passive.effects.AttributeFlat]
attribute = "Hp"
value = 20

[[skills]]

[skills.Passive]
name = "giant-armor-passive"
tags = []

[[skills.Passive.effects]]

[skills.Passive.effects.AttributeFlat]
attribute = "Hp"
value = 50

[[skills]]

[skills.Passive]
name = "ruby-burst"
tags = []

effects = []
"#;

/// 最小單位 TOML：包含一個 warrior 單位類型
pub const UNITS_TOML: &str = r#"
[[units]]
name = "warrior"
skills = ["warrior-passive", "melee-attack", "warrior-active-2", "warrior-active-4", "warrior-reaction", "warrior-counter"]
off_hand_permission = "None"

[units.equipment]
main_hand = "iron-sword"
armor = "leather-armor"
first_accessory = "ruby-ring"

[[units]]
name = "warrior-b"
skills = ["warrior-passive", "melee-attack", "warrior-reaction", "warrior-reaction-2"]
off_hand_permission = "None"

[units.equipment]

[[units]]
name = "mage"
skills = ["mage-passive", "melee-attack", "diamond-aoe-1", "summon-wall-aoe"]
off_hand_permission = "None"

[units.equipment]

[[units]]
name = "warrior-counter-only"
skills = ["warrior-passive", "warrior-counter"]
off_hand_permission = "None"

[units.equipment]
"#;

/// 副手裝備規則測試用的單位資料。
pub const OFF_HAND_UNITS_TOML: &str = r#"
[[units]]
name = "sword-user"
skills = []
off_hand_permission = "None"

[units.equipment]
main_hand = "iron-sword"

[[units]]
name = "dual-wielder"
skills = []
off_hand_permission = "Weapon"

[units.equipment]
main_hand = "iron-sword"
off_hand = "steel-sword"

[[units]]
name = "knight"
skills = []
off_hand_permission = "Shield"

[units.equipment]
main_hand = "iron-sword"
off_hand = "wooden-shield"

[[units]]
name = "great-sword-user"
skills = []
off_hand_permission = "None"

[units.equipment]
main_hand = "great-sword"
"#;

pub const EQUIPMENTS_TOML: &str = r#"
[[equipments]]
name = "iron-sword"
typ = "Weapon"
granted_skills = ["iron-slash"]

[[equipments]]
name = "steel-sword"
typ = "Weapon"
granted_skills = ["steel-slash"]

[[equipments]]
name = "wooden-bow"
typ = "Weapon"
granted_skills = []

[[equipments]]
name = "wooden-shield"
typ = "Shield"
granted_skills = []

[[equipments]]
name = "great-sword"
typ = "TwoHandedWeapon"
granted_skills = []

[[equipments]]
name = "leather-armor"
typ = "Armor"
granted_skills = ["leather-armor-passive"]

[[equipments]]
name = "giant-armor"
typ = "Armor"
granted_skills = ["giant-armor-passive"]

[[equipments]]
name = "ruby-ring"
typ = "Accessory"
granted_skills = ["ruby-burst"]

[[equipments]]
name = "echo-charm"
typ = "Accessory"
granted_skills = ["warrior-passive"]
"#;

/// 最小物件 TOML：包含一個 wall 物件類型
pub const OBJECTS_TOML: &str = r#"
[[objects]]
name = "wall"
movement_cost = 10000
blocks_sight = true
blocks_sound = true
hazardous = false

[[objects]]
name = "spike"
movement_cost = 0
blocks_sight = false
blocks_sound = false
hazardous = true

[[objects]]
name = "swamp"
movement_cost = 10
blocks_sight = false
blocks_sound = false
hazardous = true

[[objects]]
name = "fog"
movement_cost = 0
blocks_sight = true
blocks_sound = false
hazardous = false
"#;
