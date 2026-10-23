//! 集中隊伍關係判定，供技能與 AI 共用。
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
