//! 隊伍作者操作集中處理：建立派系、修改單位所屬隊伍與同盟批次改色。
use super::EditorError;
use crate::authoring::{Definitions, Map, UnitPlacement};
use crate::teams::{ENEMY_COLOR, allied_in, fixed_color, valid_color};
use crate::{Game, Team, TeamDefinition};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum TeamEdit {
    SetColor { team: Team, color: String },
    SetUnitTeam { unit_id: i64, team: Team },
    PlaceUnit { unit: UnitPlacement },
}

// 新放置的隊伍由作者操作建立設定，正式載入不補齊缺漏資料。
fn add_team(map: &mut Map, team: &Team) {
    if map.teams.iter().any(|definition| definition.team == *team) {
        return;
    }
    let color = fixed_color(&map.alliances, team)
        .unwrap_or(ENEMY_COLOR)
        .to_owned();
    map.teams.push(TeamDefinition {
        team: team.clone(),
        color,
    });
}

pub fn edit_team_from_json(
    definitions: &str,
    map: &str,
    command: &str,
) -> Result<String, EditorError> {
    let definitions: Definitions = super::json::from_str(definitions)
        .map_err(|e| EditorError::conversion("definitions_json_parse", e.to_string()))?;
    let mut map: Map = super::json::from_str(map)
        .map_err(|e| EditorError::conversion("map_json_parse", e.to_string()))?;
    let command: TeamEdit = super::json::from_str(command)
        .map_err(|e| EditorError::operation("invalid_command", &e.to_string(), Vec::new()))?;
    let changed = match command {
        TeamEdit::SetColor { team, color } => {
            if !valid_color(&color) {
                return Err(crate::error::invalid_team_settings("隊伍顏色必須為 #RRGGBB").into());
            }
            if !map.teams.iter().any(|definition| definition.team == team) {
                return Err(EditorError::operation("not_found", "team", Vec::new()));
            }
            if fixed_color(&map.alliances, &team).is_some() {
                return Err(crate::error::invalid_team_settings("玩家與友軍顏色不可修改").into());
            }
            // 同盟群組可能重疊；擴展至所有相連的敵方隊伍，避免只改一半。
            let mut affected = vec![team];
            loop {
                let mut added = false;
                for definition in &map.teams {
                    if !affected.contains(&definition.team)
                        && fixed_color(&map.alliances, &definition.team).is_none()
                        && affected
                            .iter()
                            .any(|team| allied_in(&map.alliances, team, &definition.team))
                    {
                        affected.push(definition.team.clone());
                        added = true;
                    }
                }
                if !added {
                    break;
                }
            }
            let color = color.to_ascii_lowercase();
            let mut changed = false;
            for definition in &mut map.teams {
                if affected.contains(&definition.team) && definition.color != color {
                    definition.color = color.clone();
                    changed = true;
                }
            }
            changed
        }
        TeamEdit::SetUnitTeam { unit_id, team } => {
            let unit = map
                .units
                .iter_mut()
                .find(|unit| unit.id == unit_id)
                .ok_or_else(|| {
                    EditorError::operation("not_found", &unit_id.to_string(), Vec::new())
                })?;
            let changed = unit.team != team;
            unit.team = team.clone();
            if matches!(team, Team::Enemy(_)) && unit.min_difficulty.is_none() {
                unit.min_difficulty = Some(crate::authoring::Difficulty::Easy);
            }
            add_team(&mut map, &team);
            changed
        }
        TeamEdit::PlaceUnit { unit } => {
            add_team(&mut map, &unit.team);
            map.units.push(unit);
            true
        }
    };
    let mut game =
        Game::from_authoring(definitions, map.clone(), crate::authoring::Difficulty::Hard)?;
    Ok(
        serde_json::json!({"changed": changed, "map": map, "snapshot": game.take_snapshot(None)})
            .to_string(),
    )
}
