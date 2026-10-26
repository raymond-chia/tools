//! 集中隊伍關係判定；敵人一律以「是否與對方同盟」判斷，不以陣營種類判斷。
use crate::model::{Alliances, Team};
use bevy_ecs::prelude::World;

pub(crate) fn are_allied(world: &World, first: &Team, second: &Team) -> bool {
    first == second
        || world
            .resource::<Alliances>()
            .0
            .iter()
            .any(|alliance| alliance.teams.contains(first) && alliance.teams.contains(second))
}

/// 不與玩家同盟的隊伍即為敵人。
pub(crate) fn is_hostile_to_player(world: &World, team: &Team) -> bool {
    !are_allied(world, team, &Team::Player)
}

/// 是否由玩家操作；同盟陣營仍由 AI 操作。
pub(crate) fn is_player(team: &Team) -> bool {
    *team == Team::Player
}
