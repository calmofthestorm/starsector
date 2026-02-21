use std::panic::{AssertUnwindSafe, catch_unwind};

use starsector::*;

fn child_texts(node: Section, arena: &Arena) -> Vec<String> {
    node.children(arena)
        .map(|s| s.text(arena).to_string())
        .collect()
}

#[test]
fn tree_iterator_ordering_contract() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("* A\n** A1\n*** A1a\n** A2\n* B\n");
    let root = doc.root;
    let a = root.children(&arena).next().unwrap();
    let b = root.children(&arena).nth(1).unwrap();
    let a1 = a.children(&arena).next().unwrap();
    let a1a = a1.children(&arena).next().unwrap();

    assert_eq!(child_texts(root, &arena), vec!["* A", "* B"]);
    assert_eq!(
        a.reverse_children(&arena)
            .map(|s| s.text(&arena).to_string())
            .collect::<Vec<_>>(),
        vec!["** A2", "** A1"]
    );
    assert_eq!(
        a.descendants(&arena)
            .map(|s| s.text(&arena).to_string())
            .collect::<Vec<_>>(),
        vec!["* A", "** A1", "*** A1a", "** A2"]
    );
    assert_eq!(
        a1a.ancestors(&arena)
            .map(|s| s.text(&arena).to_string())
            .collect::<Vec<_>>(),
        vec!["*** A1a", "** A1", "* A", ""]
    );

    // Indextree semantics include self for sibling iterators.
    assert_eq!(
        a.following_siblings(&arena)
            .map(|s| s.text(&arena).to_string())
            .collect::<Vec<_>>(),
        vec!["* A", "* B"]
    );
    assert_eq!(
        b.preceding_siblings(&arena)
            .map(|s| s.text(&arena).to_string())
            .collect::<Vec<_>>(),
        vec!["* B", "* A"]
    );
}

#[test]
fn tree_checked_vs_unchecked_contracts() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("* P\n");
    let root = doc.root;
    let parent = root.children(&arena).next().unwrap();

    // checked_* should return level errors for invalid nodes.
    let bad_child = arena.new_section("* bad child".into()).unwrap();
    assert!(matches!(
        parent.checked_append(&mut arena, bad_child),
        Err(StructureError::LevelError)
    ));
    assert!(matches!(
        parent.checked_prepend(&mut arena, bad_child),
        Err(StructureError::LevelError)
    ));

    // Sibling of a level-1 headline must have level > 0. Root-level section
    // is invalid.
    let bad_sibling = arena.new_section("plain root text".into()).unwrap();
    assert!(matches!(
        parent.checked_insert_before(&mut arena, bad_sibling),
        Err(StructureError::LevelError)
    ));
    assert!(matches!(
        parent.checked_insert_after(&mut arena, bad_sibling),
        Err(StructureError::LevelError)
    ));

    // unchecked_* should panic in the same invalid cases.
    let mut arena = Arena::default();
    let doc = arena.parse_str("* P\n");
    let root = doc.root;
    let parent = root.children(&arena).next().unwrap();

    let bad_child = arena.new_section("* bad child".into()).unwrap();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            parent.unchecked_append(&mut arena, bad_child)
        }))
        .is_err()
    );

    let bad_child = arena.new_section("* bad child".into()).unwrap();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            parent.unchecked_prepend(&mut arena, bad_child)
        }))
        .is_err()
    );

    let bad_sibling = arena.new_section("plain root text".into()).unwrap();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            parent.unchecked_insert_before(&mut arena, bad_sibling)
        }))
        .is_err()
    );

    let bad_sibling = arena.new_section("plain root text".into()).unwrap();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            parent.unchecked_insert_after(&mut arena, bad_sibling)
        }))
        .is_err()
    );
}

#[test]
fn tree_clone_subtree_is_structurally_independent() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("* A\n** A1\n*** A1a\n** A2\n* B\n");
    let root = doc.root;
    let a = root.children(&arena).next().unwrap();
    let b = root.children(&arena).nth(1).unwrap();

    let cloned = a.clone_subtree(&mut arena);
    assert_ne!(cloned, a);
    assert_eq!(
        cloned.to_rope(&arena).to_string(),
        a.to_rope(&arena).to_string()
    );
    assert_eq!(child_texts(cloned, &arena), vec!["** A1", "** A2"]);
    assert!(cloned.parent(&arena).is_none());

    // Remove original subtree from the document. The clone must remain intact.
    a.remove_subtree(&mut arena);
    assert_eq!(child_texts(root, &arena), vec!["* B"]);
    assert_eq!(b.parent(&arena), Some(root));

    assert_eq!(
        cloned.to_rope(&arena).to_string(),
        "* A\n** A1\n*** A1a\n** A2\n"
    );
    assert_eq!(child_texts(cloned, &arena), vec!["** A1", "** A2"]);
}

#[test]
fn tree_unchecked_matches_checked_on_valid_inputs() {
    let base = "* Parent\n** Existing\n* Anchor\n";

    let mut checked_arena = Arena::default();
    let checked_doc = checked_arena.parse_str(base);
    let checked_root = checked_doc.root;
    let checked_parent = checked_root.children(&checked_arena).next().unwrap();
    let checked_anchor = checked_root.children(&checked_arena).nth(1).unwrap();

    let child_append_checked = checked_arena
        .new_section("*** child append".into())
        .unwrap();
    let child_prepend_checked = checked_arena
        .new_section("*** child prepend".into())
        .unwrap();
    let sibling_before_checked = checked_arena.new_section("* before".into()).unwrap();
    let sibling_after_checked = checked_arena.new_section("* after".into()).unwrap();

    checked_parent
        .checked_append(&mut checked_arena, child_append_checked)
        .unwrap();
    checked_parent
        .checked_prepend(&mut checked_arena, child_prepend_checked)
        .unwrap();
    checked_anchor
        .checked_insert_before(&mut checked_arena, sibling_before_checked)
        .unwrap();
    checked_anchor
        .checked_insert_after(&mut checked_arena, sibling_after_checked)
        .unwrap();

    let mut unchecked_arena = Arena::default();
    let unchecked_doc = unchecked_arena.parse_str(base);
    let unchecked_root = unchecked_doc.root;
    let unchecked_parent = unchecked_root.children(&unchecked_arena).next().unwrap();
    let unchecked_anchor = unchecked_root.children(&unchecked_arena).nth(1).unwrap();

    let child_append_unchecked = unchecked_arena
        .new_section("*** child append".into())
        .unwrap();
    let child_prepend_unchecked = unchecked_arena
        .new_section("*** child prepend".into())
        .unwrap();
    let sibling_before_unchecked = unchecked_arena.new_section("* before".into()).unwrap();
    let sibling_after_unchecked = unchecked_arena.new_section("* after".into()).unwrap();

    unchecked_parent.unchecked_append(&mut unchecked_arena, child_append_unchecked);
    unchecked_parent.unchecked_prepend(&mut unchecked_arena, child_prepend_unchecked);
    unchecked_anchor.unchecked_insert_before(&mut unchecked_arena, sibling_before_unchecked);
    unchecked_anchor.unchecked_insert_after(&mut unchecked_arena, sibling_after_unchecked);

    assert_eq!(
        checked_doc.to_rope(&checked_arena).to_string(),
        unchecked_doc.to_rope(&unchecked_arena).to_string()
    );
    assert_eq!(
        checked_root
            .descendants(&checked_arena)
            .map(|s| (s.level(&checked_arena), s.text(&checked_arena).to_string()))
            .collect::<Vec<_>>(),
        unchecked_root
            .descendants(&unchecked_arena)
            .map(|s| (
                s.level(&unchecked_arena),
                s.text(&unchecked_arena).to_string()
            ))
            .collect::<Vec<_>>()
    );
}
