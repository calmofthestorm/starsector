use std::borrow::Cow;

use ropey::Rope;
use starsector::*;

fn ts(text: &str) -> Timestamp<'static> {
    Timestamp::parse(text).unwrap().1.into_owned()
}

#[test]
fn section_convenience_getters_work() {
    let mut arena = Arena::default();
    let doc = arena.parse_str(
        "* TODO [#A] Title :x:y:\nSCHEDULED: <2024-01-02 Tue> DEADLINE: <2024-01-03 Wed> CLOSED: [2024-01-04 Thu]\nBody line",
    );
    let section = doc.root.children(&arena).next().unwrap();

    assert_eq!(section.priority(&arena, None).unwrap(), Some('A'));
    assert_eq!(section.raw_tags(&arena, None).unwrap(), "x:y");
    assert_eq!(section.tags(&arena, None).unwrap(), vec!["x", "y"]);
    assert!(section.has_tag("x", &arena, None).unwrap());
    assert!(!section.has_tag("z", &arena, None).unwrap());
    assert_eq!(
        section.keyword(&arena, None).unwrap(),
        Some(Cow::Borrowed("TODO"))
    );
    assert_eq!(section.title(&arena, None).unwrap(), "Title");
    assert!(!section.commented(&arena, None).unwrap());
    assert_eq!(section.body(&arena, None).unwrap(), "Body line");

    let planning = section.planning(&arena, None).unwrap();
    assert_eq!(planning.scheduled, Some(ts("<2024-01-02 Tue>")));
    assert_eq!(planning.deadline, Some(ts("<2024-01-03 Wed>")));
    assert_eq!(planning.closed, Some(ts("[2024-01-04 Thu]")));
    assert_eq!(
        section.scheduled(&arena, None).unwrap(),
        Some(ts("<2024-01-02 Tue>"))
    );
    assert_eq!(
        section.deadline(&arena, None).unwrap(),
        Some(ts("<2024-01-03 Wed>"))
    );
    assert_eq!(
        section.closed(&arena, None).unwrap(),
        Some(ts("[2024-01-04 Thu]"))
    );
}

#[test]
fn section_convenience_mutators_work() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("* TODO Seed\nBody");
    let section = doc.root.children(&arena).next().unwrap();

    section.set_raw_tags(&mut arena, "raw:tags", None).unwrap();
    assert_eq!(section.raw_tags(&arena, None).unwrap(), "raw:tags");

    section
        .set_tags(
            &mut arena,
            vec![Cow::Borrowed("one"), Cow::Borrowed("two")].into_iter(),
            None,
        )
        .unwrap();
    assert_eq!(section.tags(&arena, None).unwrap(), vec!["one", "two"]);

    section
        .update_tags(
            &mut arena,
            vec![Cow::Borrowed("two"), Cow::Borrowed("three")].into_iter(),
            None,
        )
        .unwrap();
    assert_eq!(
        section.tags(&arena, None).unwrap(),
        vec!["one", "two", "three"]
    );

    section.remove_tags(&mut arena, &["two"], None).unwrap();
    assert_eq!(section.tags(&arena, None).unwrap(), vec!["one", "three"]);

    section.add_tag(&mut arena, "four", None).unwrap();
    assert!(section.has_tag("four", &arena, None).unwrap());
    section.clear_tag(&mut arena, "one", None).unwrap();
    assert_eq!(section.tags(&arena, None).unwrap(), vec!["three", "four"]);
    section.clear_tags(&mut arena, None).unwrap();
    assert!(section.tags(&arena, None).unwrap().is_empty());

    section
        .set_keyword(&mut arena, Some("DONE".into()), None)
        .unwrap();
    assert_eq!(
        section.keyword(&arena, None).unwrap(),
        Some(Cow::Borrowed("DONE"))
    );

    section
        .set_title(&mut arena, Rope::from("New title"), None)
        .unwrap();
    assert_eq!(section.title(&arena, None).unwrap(), "New title");

    section.set_commented(&mut arena, true, None).unwrap();
    assert!(section.commented(&arena, None).unwrap());

    let planning = Planning {
        scheduled: Some(ts("<2025-02-01 Sat>")),
        deadline: Some(ts("<2025-02-02 Sun>")),
        ..Planning::default()
    };
    section.set_planning(&mut arena, planning, None).unwrap();
    assert_eq!(
        section.scheduled(&arena, None).unwrap(),
        Some(ts("<2025-02-01 Sat>"))
    );
    assert_eq!(
        section.deadline(&arena, None).unwrap(),
        Some(ts("<2025-02-02 Sun>"))
    );

    section
        .set_scheduled(&mut arena, Some(ts("<2025-02-03 Mon>")), None)
        .unwrap();
    section
        .set_deadline(&mut arena, Some(ts("<2025-02-04 Tue>")), None)
        .unwrap();
    section
        .set_closed(&mut arena, Some(ts("[2025-02-05 Wed]")), None)
        .unwrap();
    assert_eq!(
        section.scheduled(&arena, None).unwrap(),
        Some(ts("<2025-02-03 Mon>"))
    );
    assert_eq!(
        section.deadline(&arena, None).unwrap(),
        Some(ts("<2025-02-04 Tue>"))
    );
    assert_eq!(
        section.closed(&arena, None).unwrap(),
        Some(ts("[2025-02-05 Wed]"))
    );

    section
        .set_body(&mut arena, Rope::from("final body"), None)
        .unwrap();
    assert_eq!(section.body(&arena, None).unwrap(), "final body");
}

#[test]
fn section_convenience_errors_for_invalid_headline_or_input() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("root text");
    let root = doc.root;

    assert!(matches!(
        root.priority(&arena, None),
        Err(HeadlineError::InvalidHeadlineError)
    ));
    assert!(matches!(
        root.set_title(&mut arena, Rope::from("x"), None),
        Err(HeadlineError::InvalidHeadlineError)
    ));
    assert!(matches!(
        root.set_keyword(&mut arena, Some(Rope::from("TODO")), None),
        Err(HeadlineError::InvalidHeadlineError)
    ));

    let doc = arena.parse_str("* TODO ok\nBody");
    let section = doc.root.children(&arena).next().unwrap();
    assert!(matches!(
        section.set_keyword(&mut arena, Some(Rope::from("NOT_A_KEYWORD")), None),
        Err(HeadlineError::InvalidKeywordError)
    ));
    assert!(matches!(
        section.set_body(&mut arena, Rope::from("\n* child headline"), None),
        Err(HeadlineError::InvalidBodyError)
    ));
}
