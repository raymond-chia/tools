//! 編輯器資料之間引用關係的共用操作。

use std::collections::HashSet;
use std::hash::Hash;

/// 是否含有不在有效名稱集合中的引用。
pub(crate) fn has_invalid<'a, T>(
    mut references: impl Iterator<Item = &'a T>,
    valid_names: &HashSet<T>,
) -> bool
where
    T: Eq + Hash + 'a,
{
    references.any(|name| !valid_names.contains(name))
}

/// 保留所有有效引用。
pub(crate) fn retain_valid<T>(references: &mut Vec<T>, valid_names: &HashSet<T>)
where
    T: Eq + Hash,
{
    references.retain(|name| valid_names.contains(name));
}

/// 清除失效的選填引用。
pub(crate) fn clear_invalid_option<T>(reference: &mut Option<T>, valid_names: &HashSet<T>)
where
    T: Eq + Hash,
{
    if reference
        .as_ref()
        .is_some_and(|name| !valid_names.contains(name))
    {
        *reference = None;
    }
}
