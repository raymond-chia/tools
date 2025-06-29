use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Serialize)]
pub struct Team {
    pub id: TeamID,
    pub color: (u8, u8, u8), // RGB color
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnitTemplate {
    pub name: String,
    pub move_points: usize,
    pub skills: BTreeSet<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UnitMarker {
    pub id: UnitID,
    pub unit_template_type: UnitTemplateType,
    pub team: TeamID,
    pub pos: Pos,
}

#[derive(Debug)]
pub struct Unit {
    pub id: UnitID,
    pub team: TeamID,
    pub moved: MovementCost,
    pub move_points: MovementCost,
    pub skills: BTreeSet<String>,
}

impl Default for UnitTemplate {
    fn default() -> Self {
        Self {
            name: String::new(),
            move_points: 30,
            skills: BTreeSet::new(),
        }
    }
}

impl Unit {
    pub fn from_template(marker: &UnitMarker, template: &UnitTemplate) -> Self {
        Unit {
            id: marker.id,
            team: marker.team.clone(),
            moved: 0,
            move_points: template.move_points,
            skills: template.skills.clone(),
        }
    }
}
