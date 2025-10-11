//! unit.rs：
//! - 定義單位（Unit）、單位模板（UnitTemplate）等資料結構，僅負責靜態資料與屬性，不含戰鬥邏輯。
//! - 所有單位屬性衍生值（如先攻 initiative、移動力等）之計算，應實作於 unit.rs 內部方法或輔助函式。
//! - 不負責戰鬥流程與判定（如命中、閃避、格擋、傷害計算等）。
use crate::*;
use serde::{Deserialize, Serialize};
use skills_lib::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Team {
    pub id: TeamID,
    pub color: RGB,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnitTemplate {
    pub name: UnitTemplateType,
    pub skills: BTreeSet<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UnitMarker {
    pub id: UnitID,
    pub unit_template_type: UnitTemplateType,
    pub team: TeamID,
    pub pos: Pos,
}

#[derive(Debug)]
pub struct Unit {
    pub id: UnitID,
    pub unit_template_type: UnitTemplateType,
    pub team: TeamID,
    pub moved: MovementCost,
    pub move_points: MovementCost,
    pub has_cast_skill_this_turn: bool,
    pub hp: i32,
    pub max_hp: i32,
    pub skills: BTreeSet<String>,
}

impl Default for UnitTemplate {
    fn default() -> Self {
        Self {
            name: String::new(),
            skills: BTreeSet::new(),
        }
    }
}

impl Unit {
    pub fn from_template(
        marker: &UnitMarker,
        template: &UnitTemplate,
        skills: &BTreeMap<SkillID, Skill>,
    ) -> Result<Self, Error> {
        let func = "Unit::from_template";

        let skills: Result<_, _> = template
            .skills
            .iter()
            .map(|id| {
                skills
                    .get(id)
                    .map(|s| (id, s))
                    .ok_or_else(|| Error::SkillNotFound {
                        func,
                        skill_id: id.clone(),
                    })
            })
            .collect();
        let skills = skills?;
        let max_hp = skills_to_max_hp(&skills);
        Ok(Unit {
            id: marker.id,
            unit_template_type: marker.unit_template_type.clone(),
            team: marker.team.clone(),
            moved: 0,
            move_points: skills_to_move_points(&skills),
            has_cast_skill_this_turn: false,
            hp: max_hp,
            max_hp,
            skills: template.skills.clone(),
        })
    }
}

/// 計算單位本回合的 initiative 值
/// - 1D6 隨機
/// - 技能 initiative 加總（i32）
/// - 未來可擴充 buff/debuff、裝備等
pub fn calc_initiative<R: rand::Rng>(rng: &mut R, skills: &BTreeMap<&SkillID, &Skill>) -> i32 {
    let roll = rng.random_range(1..=6);
    let skill_initiative = skills_to_initiative(&skills);
    roll + skill_initiative
}

pub fn skills_to_max_hp(skills: &BTreeMap<&SkillID, &Skill>) -> i32 {
    skills
        .iter()
        .flat_map(|(_, skill)| &skill.effects)
        .filter_map(|effect| {
            if let Effect::MaxHp { value, .. } = effect {
                Some(*value)
            } else {
                None
            }
        })
        .sum()
}

/// 計算單位 initiative 技能等級總和
/// 尋找所有 effect 為 Effect::Initiative 的技能，並加總其 value
pub fn skills_to_initiative(skills: &BTreeMap<&SkillID, &Skill>) -> i32 {
    skills
        .iter()
        .flat_map(|(_, skill)| &skill.effects)
        .filter_map(|effect| {
            if let Effect::Initiative { value, .. } = effect {
                Some(*value)
            } else {
                None
            }
        })
        .sum()
}

/// 計算單位 evasion 技能等級總和
/// 尋找所有 effect 為 Effect::Evasion 的技能，並加總其 value
pub fn skills_to_evasion(skills: &BTreeMap<&SkillID, &Skill>) -> i32 {
    skills
        .iter()
        .flat_map(|(_, skill)| &skill.effects)
        .filter_map(|effect| {
            if let Effect::Evasion { value, .. } = effect {
                Some(*value)
            } else {
                None
            }
        })
        .sum()
}

/// 計算單位 block 技能等級總和
/// 尋找所有 effect 為 Effect::Block 的技能，並加總其 value
pub fn skills_to_block(skills: &BTreeMap<&SkillID, &Skill>) -> i32 {
    skills
        .iter()
        .flat_map(|(_, skill)| &skill.effects)
        .filter_map(|effect| {
            if let Effect::Block { value, .. } = effect {
                Some(*value)
            } else {
                None
            }
        })
        .sum()
}

pub fn skills_to_move_points(skills: &BTreeMap<&SkillID, &Skill>) -> MovementCost {
    let points: i32 = skills
        .iter()
        .flat_map(|(_, skill)| &skill.effects)
        .filter_map(|effect| {
            if let Effect::MovePoints { value, .. } = effect {
                Some(*value)
            } else {
                None
            }
        })
        .sum();
    if points < 0 {
        0
    } else {
        points as MovementCost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use std::collections::HashMap;

    #[test]
    fn test_deserialize_unit() {
        let data = include_str!("../tests/unit.json");
        let v: serde_json::Value = serde_json::from_str(data).unwrap();
        // 從 skill_sprint.json 載入 sprint 技能
        let sprint_data = include_str!("../tests/skill_sprint.json");
        let sprint_skill: Skill = serde_json::from_str(sprint_data).unwrap();
        let max_hp_data = include_str!("../tests/skill_max_hp.json");
        let max_hp_skill: Skill = serde_json::from_str(max_hp_data).unwrap();

        // 測試 Team
        let team: Team = serde_json::from_value(v["Team"].clone()).unwrap();
        assert_eq!(team.id, "t1");
        assert_eq!(team.color, (255, 0, 0));

        // 測試 UnitTemplate
        let template: UnitTemplate = serde_json::from_value(v["UnitTemplate"].clone()).unwrap();
        assert_eq!(template.name, "knight");
        assert_eq!(template.skills.len(), 2);
        assert!(template.skills.contains("sprint"));
        assert!(template.skills.contains("slash"));

        // 測試 UnitMarker
        let marker: UnitMarker = serde_json::from_value(v["UnitMarker"].clone()).unwrap();
        assert_eq!(marker.id, 42);
        assert_eq!(marker.unit_template_type, "knight");
        assert_eq!(marker.team, "t1");
        assert_eq!(marker.pos, Pos { x: 0, y: 0 });

        // 測試 Unit::from_template
        let skills_map = BTreeMap::from([
            ("sprint".to_string(), sprint_skill),
            ("max_hp".to_string(), max_hp_skill),
        ]);

        fn with_skills(mut template: UnitTemplate, skills: &[&str]) -> UnitTemplate {
            template.skills = skills.iter().map(|s| s.to_string()).collect();
            template
        }
        let test_data = [
            (vec![], HashMap::from([("move_points", 0), ("max_hp", 0)])),
            (
                vec!["sprint"],
                HashMap::from([("move_points", 30), ("max_hp", 0)]),
            ),
            (
                vec!["max_hp"],
                HashMap::from([("move_points", 0), ("max_hp", 10)]),
            ),
            (
                vec!["sprint", "max_hp"],
                HashMap::from([("move_points", 30), ("max_hp", 10)]),
            ),
        ];

        for (skills, expect) in test_data {
            let template = with_skills(template.clone(), &skills);
            let unit = Unit::from_template(&marker, &template, &skills_map).unwrap();
            assert_eq!(unit.id, marker.id);
            assert_eq!(unit.unit_template_type, marker.unit_template_type);
            assert_eq!(unit.team, marker.team);
            assert_eq!(unit.moved, 0);
            assert_eq!(unit.move_points, expect["move_points"] as usize);
            assert_eq!(unit.hp, expect["max_hp"]);
            assert_eq!(unit.max_hp, expect["max_hp"]);
            assert_eq!(unit.skills.len(), skills.len());
            for skill in skills {
                assert!(unit.skills.contains(skill));
            }
        }
    }

    #[test]
    fn test_unit_template_default() {
        let default = UnitTemplate::default();
        assert_eq!(default.name, "");
        assert!(default.skills.is_empty());
    }

    #[test]
    fn test_unit_struct_fields_extreme() {
        let unit = Unit {
            id: 999,
            unit_template_type: "超級戰士".to_string(),
            team: "t99".to_string(),
            moved: usize::MAX,
            move_points: usize::MAX,
            has_cast_skill_this_turn: true,
            hp: i32::MIN,
            max_hp: i32::MAX,
            skills: ["超級技能".to_string()].iter().cloned().collect(),
        };
        assert_eq!(unit.id, 999);
        assert_eq!(unit.unit_template_type, "超級戰士");
        assert_eq!(unit.team, "t99");
        assert_eq!(unit.moved, usize::MAX);
        assert_eq!(unit.move_points, usize::MAX);
        assert!(unit.has_cast_skill_this_turn);
        assert_eq!(unit.hp, i32::MIN);
        assert_eq!(unit.max_hp, i32::MAX);
        assert!(unit.skills.contains("超級技能"));
    }

    #[test]
    fn test_unit_from_template_skill_not_found() {
        let marker = UnitMarker {
            id: 1,
            unit_template_type: "knight".to_string(),
            team: "t1".to_string(),
            pos: Pos { x: 0, y: 0 },
        };
        let template = UnitTemplate {
            name: "knight".to_string(),
            skills: ["not_exist_skill".to_string()].iter().cloned().collect(),
        };
        let skills_map = BTreeMap::new();
        let result = Unit::from_template(&marker, &template, &skills_map);
        match result {
            Err(Error::SkillNotFound { skill_id, .. }) => assert_eq!(skill_id, "not_exist_skill"),
            _ => panic!("Should return Error::SkillNotFound"),
        }
    }

    #[test]
    fn test_skills_to_max_hp() {
        let mut skills = BTreeMap::new();
        // 無技能
        assert_eq!(skills_to_max_hp(&skills), 0);

        // 一個 MaxHp 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::MaxHp {
            value: 10,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(&key_a, &skill1);
        assert_eq!(skills_to_max_hp(&skills), 10);

        // 多個 MaxHp 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::MaxHp {
            value: 20,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(&key_b, &skill2);
        assert_eq!(skills_to_max_hp(&skills), 30);

        // 非 MaxHp 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::Initiative {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(&key_c, &skill3);
        assert_eq!(skills_to_max_hp(&skills), 30);
    }

    #[test]
    fn test_calc_initiative_with_and_without_skill() {
        let mut rng = rand::rng();
        let mut skills = BTreeMap::new();
        // 無技能
        let result = calc_initiative(&mut rng, &skills);
        assert!(result >= 1 && result <= 6);

        // 有 initiative 技能
        let mut skill = Skill::default();
        skill.effects = vec![Effect::Initiative {
            value: 3,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key = "test".to_string();
        skills.insert(&key, &skill);
        let result = calc_initiative(&mut rng, &skills);
        assert!(result >= 4 && result <= 9);

        // 有 initiative 技能
        let mut skill = Skill::default();
        skill.effects = vec![
            Effect::Initiative {
                value: 3,
                target_type: Default::default(),
                shape: Default::default(),
                duration: 0,
            },
            Effect::Initiative {
                value: 2,
                target_type: Default::default(),
                shape: Default::default(),
                duration: 0,
            },
        ];
        let key = "test".to_string();
        skills.insert(&key, &skill);
        let result = calc_initiative(&mut rng, &skills);
        assert!(result >= 6 && result <= 11);
    }

    #[test]
    fn test_skills_to_initiative() {
        let mut skills = BTreeMap::new();
        // 無技能
        assert_eq!(skills_to_initiative(&skills), 0);

        // 一個 initiative 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::Initiative {
            value: 2,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(&key_a, &skill1);
        assert_eq!(skills_to_initiative(&skills), 2);

        // 多個 initiative 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::Initiative {
            value: 3,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(&key_b, &skill2);
        assert_eq!(skills_to_initiative(&skills), 5);

        // 非 initiative 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MaxHp {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(&key_c, &skill3);
        assert_eq!(skills_to_initiative(&skills), 5);
    }

    #[test]
    fn test_skills_to_evasion() {
        let mut skills = BTreeMap::new();
        // 無技能
        assert_eq!(skills_to_evasion(&skills), 0);

        // 一個 evasion 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::Evasion {
            value: 2,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(&key_a, &skill1);
        assert_eq!(skills_to_evasion(&skills), 2);

        // 多個 evasion 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::Evasion {
            value: 3,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(&key_b, &skill2);
        assert_eq!(skills_to_evasion(&skills), 5);

        // 非 evasion 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MaxHp {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(&key_c, &skill3);
        assert_eq!(skills_to_evasion(&skills), 5);
    }

    #[test]
    fn test_skills_to_block() {
        let mut skills = BTreeMap::new();
        // 無技能
        assert_eq!(skills_to_block(&skills), 0);

        // 一個 block 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::Block {
            value: 2,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(&key_a, &skill1);
        assert_eq!(skills_to_block(&skills), 2);

        // 多個 block 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::Block {
            value: 3,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(&key_b, &skill2);
        assert_eq!(skills_to_block(&skills), 5);

        // 非 block 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MaxHp {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(&key_c, &skill3);
        assert_eq!(skills_to_block(&skills), 5);
    }

    #[test]
    fn test_skills_to_move_points_negative() {
        let mut skills = BTreeMap::new();
        // 負數 move_points，應回傳 0
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::MovePoints {
            value: -10,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(&key_a, &skill1);
        assert_eq!(skills_to_move_points(&skills), 0);

        // 正負混合，總和為負，仍回傳 0
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::MovePoints {
            value: 5,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(&key_b, &skill2);
        assert_eq!(skills_to_move_points(&skills), 0);

        // 正負混合，總和為正
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MovePoints {
            value: 20,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(&key_c, &skill3);
        assert_eq!(skills_to_move_points(&skills), 15);
    }
}
