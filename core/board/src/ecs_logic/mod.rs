pub mod deployment;
pub mod loader;
pub mod movement;
pub mod query;
pub mod spawner;
pub mod turn;

/// 從 EntityRef 取得 component，若缺少則回傳 DataError::MissingComponent
macro_rules! clone_component {
    ($entity_ref:expr, $component:ty) => {
        $entity_ref
            .get::<$component>()
            .ok_or_else(|| crate::error::DataError::MissingComponent {
                name: stringify!($component).to_string(),
            })?
            .clone()
    };
}

pub(super) use clone_component;
