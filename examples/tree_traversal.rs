use starsector::Arena;

fn main() {
    let mut arena = Arena::default();
    let text = "root\n* A\nA body\n** A1\n*** A1a\n** A2\n* B\n";
    println!("Parsed org:\n{text}");
    let doc = arena.parse_str(text);

    let root = doc.root;
    let a = root.children(&arena).next().expect("missing A");
    let b = root.children(&arena).last().expect("missing B");

    assert_eq!(a.parent(&arena), Some(root));
    assert_eq!(b.parent(&arena), Some(root));

    let a_children: Vec<_> = a.children(&arena).collect();
    assert_eq!(a_children.len(), 2);

    let a_desc: Vec<_> = a.descendants(&arena).collect();
    assert!(a_desc.len() >= 3);

    let a1a = a_desc
        .iter()
        .copied()
        .find(|s| s.level(&arena) == 3)
        .expect("missing A1a");

    let a1a_anc: Vec<_> = a1a.ancestors(&arena).collect();
    assert!(a1a_anc.contains(&a));
    assert!(a1a_anc.contains(&root));

    let b_prev: Vec<_> = b.preceding_siblings(&arena).collect();
    assert!(b_prev.contains(&a));

    let a_next: Vec<_> = a.following_siblings(&arena).collect();
    assert!(a_next.contains(&b));

    let rev_children: Vec<_> = root.reverse_children(&arena).collect();
    assert_eq!(rev_children[0], b);
    assert_eq!(rev_children[1], a);

    let out = doc.to_rope(&arena).to_string();
    let b_offset = doc
        .text_offset_of_child(&arena, b)
        .expect("offset for B missing");
    println!("Offset of B relative to document root: {b_offset}");
    assert!(out.contains("* B"));

    println!("Traversal target org:\n{}", out);
    println!("Traversal APIs returned expected structure and offsets.");
}
