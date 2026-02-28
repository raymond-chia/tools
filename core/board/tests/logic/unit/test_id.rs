use board::logic::id_generator::generate_unique_id;
use std::collections::HashSet;

#[test]
fn generates_unique_ids() {
    let mut used_ids = HashSet::new();
    let id1 = generate_unique_id(&mut used_ids);
    let id2 = generate_unique_id(&mut used_ids);

    assert_ne!(id1, id2);
    assert_eq!(used_ids.len(), 2);
    assert!(used_ids.contains(&id1));
    assert!(used_ids.contains(&id2));
}

#[test]
fn avoids_existing_ids() {
    let mut used_ids = HashSet::new();
    let count = 10;
    for _ in 0..count {
        generate_unique_id(&mut used_ids);
    }
    assert_eq!(used_ids.len(), count);

    for _ in 0..count {
        generate_unique_id(&mut used_ids);
    }
    assert_eq!(used_ids.len(), count * 2);
}
