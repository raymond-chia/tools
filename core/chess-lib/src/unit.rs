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

    /// 計算單位本回合的 initiative 值
    /// - 1D6 隨機
    /// - 技能 initiative 加總（i32）
    /// - 未來可擴充 buff/debuff、裝備等
    pub fn calc_initiative<R: rand::Rng>(rng: &mut R, skills: &BTreeMap<&SkillID, &Skill>) -> i32 {
        let roll = rng.random_range(1..=6);
        let skill_initiative = skills_to_initiative(&skills);
        roll + skill_initiative
    }
}

use inner::*;
mod inner {
    use super::*;

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
}
