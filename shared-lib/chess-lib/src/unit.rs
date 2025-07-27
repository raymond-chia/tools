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
    ) -> Result<Self, String> {
        let skills: Result<_, _> = template
            .skills
            .iter()
            .map(|id| {
                skills
                    .get(id)
                    .map(|s| (id, s))
                    .ok_or_else(|| format!("Skill {id} not found"))
            })
            .collect();
        let skills = skills?;
        Ok(Unit {
            id: marker.id,
            unit_template_type: marker.unit_template_type.clone(),
            team: marker.team.clone(),
            moved: 0,
            move_points: skills_to_move_points(&skills),
            skills: template.skills.clone(),
        })
    }
}

fn skills_to_move_points(skills: &BTreeMap<&SkillID, &Skill>) -> MovementCost {
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

    #[test]
    fn test_deserialize_unit() {
        let data = include_str!("../tests/unit.json");
        let v: serde_json::Value = serde_json::from_str(data).unwrap();
        // 從 skill_sprint.json 載入 sprint 技能
        let sprint_data = include_str!("../tests/skill_sprint.json");
        let sprint_skill: Skill = serde_json::from_str(sprint_data).unwrap();
        let slash_data = include_str!("../tests/skill_slash.json");
        let slash_skill: Skill = serde_json::from_str(slash_data).unwrap();

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
            ("slash".to_string(), slash_skill),
        ]);

        let unit = Unit::from_template(&marker, &template, &skills_map).unwrap();
        assert_eq!(unit.id, marker.id);
        assert_eq!(unit.unit_template_type, marker.unit_template_type);
        assert_eq!(unit.team, marker.team);
        assert_eq!(unit.moved, 0);
        assert_eq!(unit.move_points, 30);
        assert_eq!(unit.skills.len(), 2);
        assert!(unit.skills.contains("sprint"));
        assert!(unit.skills.contains("slash"));
    }

    #[test]
    fn test_skills_to_move_points() {
        let sprint_data = include_str!("../tests/skill_sprint.json");
        let sprint_skill: Skill = serde_json::from_str(sprint_data).unwrap();
        let slash_data = include_str!("../tests/skill_slash.json");
        let slash_skill: Skill = serde_json::from_str(slash_data).unwrap();
        let sprint = "sprint".to_string();
        let slash = "slash".to_string();

        let skills_map = BTreeMap::from([(&sprint, &sprint_skill), (&slash, &slash_skill)]);

        let move_points = super::skills_to_move_points(&skills_map);
        assert_eq!(move_points, 30);
    }
}
