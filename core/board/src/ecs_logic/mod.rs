pub mod deployment;
pub mod loader;
pub mod query;
pub mod spawner;

/// 從 EntityRef 取得 component，若缺少則回傳 DataError::MissingComponent
macro_rules! get_component {
    ($entity_ref:expr, $component:ty) => {
        $entity_ref
            .get::<$component>()
            .ok_or_else(|| crate::error::DataError::MissingComponent {
                component_name: stringify!($component).to_string(),
            })?
            .clone()
    };
}

pub(super) use get_component;
