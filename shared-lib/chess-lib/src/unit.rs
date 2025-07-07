use crate::*;
use serde::{Deserialize, Serialize};
use skills_lib::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Team {
    pub id: TeamID,
    pub color: (u8, u8, u8), // RGB color
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
