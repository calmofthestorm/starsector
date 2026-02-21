use starsector::Arena;

fn main() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("");
    let root = doc.root;

    let a = arena
        .new_section("* A".into())
        .expect("new section A failed");
    let b = arena
        .new_section("* B".into())
        .expect("new section B failed");
    let c = arena
        .new_section("* C".into())
        .expect("new section C failed");

    root.checked_append(&mut arena, a).expect("append A failed");
    a.checked_insert_after(&mut arena, b)
        .expect("insert B after A failed");
    b.checked_insert_after(&mut arena, c)
        .expect("insert C after B failed");

    let x = arena
        .new_section("* X".into())
        .expect("new section X failed");
    b.checked_insert_before(&mut arena, x)
        .expect("insert X before B failed");

    let y = arena
        .new_section("* Y".into())
        .expect("new section Y failed");
    root.checked_prepend(&mut arena, y)
        .expect("prepend Y failed");

    let child = arena
        .new_section("** Child".into())
        .expect("new child failed");
    a.unchecked_append(&mut arena, child);

    let c1 = arena.new_section("** C1".into()).expect("new C1 failed");
    c.unchecked_prepend(&mut arena, c1);

    let x1 = arena.new_section("* X1".into()).expect("new X1 failed");
    x.unchecked_insert_after(&mut arena, x1);

    let x0 = arena.new_section("* X0".into()).expect("new X0 failed");
    x.unchecked_insert_before(&mut arena, x0);

    let z = arena.new_section("* Z".into()).expect("new Z failed");
    c.insert_before(&mut arena, z)
        .expect("insert_before failed");

    let clone_of_a = a.clone_subtree(&mut arena);
    root.append(&mut arena, clone_of_a)
        .expect("append clone failed");

    x0.remove_subtree(&mut arena);
    x.replace_with_children(&mut arena);

    c.remove_children(&mut arena);

    let out = doc.to_rope(&arena).to_string();
    assert!(out.contains("* A"));
    assert!(out.contains("* B"));
    assert!(out.contains("* C"));
    assert!(out.contains("* Z"));
    assert!(out.contains("* Y"));
    assert!(out.matches("* A").count() >= 2);

    println!("Final edited org:\n{out}");
    println!("Structural editing APIs completed without violating tree invariants.");
}
