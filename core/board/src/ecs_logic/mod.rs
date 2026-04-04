pub mod deployment;
pub mod loader;
pub mod movement;
pub mod query;
pub mod skill;
pub mod spawner;
pub mod turn;

/// 從 EntityRef 取得 component 的不可變引用，若缺少則回傳 DataError::MissingComponent
macro_rules! get_component {
    ($entity_ref:expr, $component:ty) => {
        $entity_ref
            .get::<$component>()
            .ok_or_else(|| crate::error::DataError::MissingComponent {
                name: stringify!($component).to_string(),
            })
    };
}

/// 從 EntityMut 取得 component 的可變引用，若缺少則回傳 DataError::MissingComponent
macro_rules! get_component_mut {
    ($entity_mut:expr, $component:ty) => {
        $entity_mut.get_mut::<$component>().ok_or_else(|| {
            crate::error::DataError::MissingComponent {
                name: stringify!($component).to_string(),
            }
        })
    };
}

pub(crate) use get_component;
pub(crate) use get_component_mut;
