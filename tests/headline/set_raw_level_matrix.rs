use ropey::Rope;
use starsector::*;

fn headline_with_level(level: u16, title: &str) -> Headline {
    let mut b = HeadlineBuilder::default();
    b.level(level);
    b.title(Rope::from(title));
    b.headline(None).unwrap()
}

#[test]
fn set_level_edge_matrix() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("* P\n** M\n*** C\n* S\n");
    let root = doc.root;
    let p = root.children(&arena).next().unwrap();
    let m = p.children(&arena).next().unwrap();
    let c = m.children(&arena).next().unwrap();

    // Root can no-op set to 0, but cannot become a headline.
    assert!(root.set_level(&mut arena, 0).is_ok());
    assert!(matches!(
        root.set_level(&mut arena, 1),
        Err(StructureError::LevelError)
    ));

    // Node with both parent and child can only keep existing level here.
    assert!(matches!(
        m.set_level(&mut arena, 1),
        Err(StructureError::LevelError)
    ));
    assert!(m.set_level(&mut arena, 2).is_ok());
    assert!(matches!(
        m.set_level(&mut arena, 3),
        Err(StructureError::LevelError)
    ));
    assert!(matches!(
        m.set_level(&mut arena, 4),
        Err(StructureError::LevelError)
    ));

    // Leaf can move higher, but not to root or parent level.
    assert!(matches!(
        c.set_level(&mut arena, 0),
        Err(StructureError::LevelError)
    ));
    assert!(matches!(
        c.set_level(&mut arena, 2),
        Err(StructureError::LevelError)
    ));
    assert!(c.set_level(&mut arena, 4).is_ok());
    assert_eq!(c.level(&arena), 4);
}

#[test]
fn set_raw_edge_matrix() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("* P\n** M\n*** C\n");
    let root = doc.root;
    let p = root.children(&arena).next().unwrap();
    let m = p.children(&arena).next().unwrap();
    let c = m.children(&arena).next().unwrap();

    // Valid change that increases level and keeps structure valid.
    c.set_raw(&mut arena, Rope::from("**** New child")).unwrap();
    assert_eq!(c.level(&arena), 4);
    assert_eq!(c.text(&arena), "**** New child");

    // Invalid body: parses as multiple root-level sections.
    assert!(matches!(
        c.set_raw(&mut arena, Rope::from("* A\n* B")),
        Err(HeadlineError::InvalidBodyError)
    ));

    // Invalid body: includes a child headline in same section text.
    assert!(matches!(
        c.set_raw(&mut arena, Rope::from("*** Parent\n**** Child")),
        Err(HeadlineError::InvalidBodyError)
    ));

    // Invalid level changes for current parent relation.
    assert!(matches!(
        c.set_raw(&mut arena, Rope::from("** Too shallow")),
        Err(HeadlineError::InvalidLevelError)
    ));
    assert!(matches!(
        c.set_raw(&mut arena, Rope::from("plain root text")),
        Err(HeadlineError::InvalidLevelError)
    ));

    // Root can set non-headline raw text, but cannot become headline via set_raw.
    root.set_raw(&mut arena, Rope::from("root text")).unwrap();
    assert_eq!(root.level(&arena), 0);
    assert_eq!(root.text(&arena), "root text");
    assert!(matches!(
        root.set_raw(&mut arena, Rope::from("* Not allowed")),
        Err(HeadlineError::InvalidLevelError)
    ));
}

#[test]
fn set_headline_edge_matrix() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("* P\n** M\n*** C\n");
    let root = doc.root;
    let p = root.children(&arena).next().unwrap();
    let m = p.children(&arena).next().unwrap();
    let c = m.children(&arena).next().unwrap();

    // Leaf accepts legal level update through set_headline.
    let h4 = headline_with_level(4, "C4");
    c.set_headline(&mut arena, &h4).unwrap();
    assert_eq!(c.level(&arena), 4);
    assert_eq!(c.text(&arena), "**** C4");

    // Invalid: cannot move to parent level.
    let h2 = headline_with_level(2, "C2");
    assert!(matches!(
        c.set_headline(&mut arena, &h2),
        Err(HeadlineError::InvalidLevelError)
    ));

    // Node with child cannot move to or above child level.
    let hm3 = headline_with_level(4, "M4");
    assert!(matches!(
        m.set_headline(&mut arena, &hm3),
        Err(HeadlineError::InvalidLevelError)
    ));

    // Root cannot become headline via set_headline.
    let hr = headline_with_level(1, "Root");
    assert!(matches!(
        root.set_headline(&mut arena, &hr),
        Err(HeadlineError::InvalidLevelError)
    ));
}
