//! 集中隊伍關係判定；敵人一律以「是否與對方同盟」判斷，不以陣營種類判斷。
use crate::authoring::Alliance;
use crate::model::{Alliances, Team};
use bevy_ecs::prelude::World;

pub(crate) const PLAYER_COLOR: &str = "#63a9ff";
pub(crate) const ALLY_COLOR: &str = "#68d98b";
pub(crate) const ENEMY_COLOR: &str = "#ff6868";

/// 作者文件與執行期共用同盟判定。
pub(crate) fn allied_in(alliances: &[Alliance], first: &Team, second: &Team) -> bool {
    first == second
        || alliances
            .iter()
            .any(|alliance| alliance.teams.contains(first) && alliance.teams.contains(second))
}

pub(crate) fn fixed_color(alliances: &[Alliance], team: &Team) -> Option<&'static str> {
    if *team == Team::Player {
        Some(PLAYER_COLOR)
    } else if allied_in(alliances, team, &Team::Player) {
        Some(ALLY_COLOR)
    } else {
        None
    }
}

pub(crate) fn valid_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}
pub(crate) fn are_allied(world: &World, first: &Team, second: &Team) -> bool {
    allied_in(&world.resource::<Alliances>().0, first, second)
}

/// 不與玩家同盟的隊伍即為敵人。
pub(crate) fn is_hostile_to_player(world: &World, team: &Team) -> bool {
    !are_allied(world, team, &Team::Player)
}

/// 是否由玩家操作；同盟陣營仍由 AI 操作。
pub(crate) fn is_player(team: &Team) -> bool {
    *team == Team::Player
}
