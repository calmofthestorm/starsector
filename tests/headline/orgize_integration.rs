use std::borrow::Cow;

use indexmap::IndexMap;
use starsector::*;

fn section_with_properties(arena: &mut Arena) -> Section {
    let doc = arena.parse_str("* TODO Headline\n:PROPERTIES:\n:FOO: bar\n:END:\nBody");
    doc.root.children(arena).next().unwrap()
}

#[test]
fn orgize_section_property_api_roundtrip() {
    let mut arena = Arena::default();
    let section = section_with_properties(&mut arena);

    assert!(section.has_property(&arena, "FOO", None).unwrap());
    assert_eq!(
        section.get_property(&arena, "FOO", None).unwrap(),
        Some(Cow::Borrowed("bar"))
    );
    assert_eq!(section.get_id(&arena, None).unwrap(), None);

    let props = section.properties(&arena, None).unwrap();
    assert_eq!(props.get("FOO"), Some(&Cow::Borrowed("bar")));

    section
        .set_property(&mut arena, "ID", "fixed-id", None)
        .unwrap();
    assert_eq!(
        section.get_id(&arena, None).unwrap(),
        Some(Cow::Borrowed("fixed-id"))
    );

    section.clear_property(&mut arena, "FOO", None).unwrap();
    assert!(!section.has_property(&arena, "FOO", None).unwrap());

    let mut new_props: IndexMap<Cow<'static, str>, Cow<'static, str>> = IndexMap::new();
    new_props.insert(Cow::Borrowed("ALPHA"), Cow::Borrowed("1"));
    new_props.insert(Cow::Borrowed("BETA"), Cow::Borrowed("2"));
    section.set_properties(&mut arena, new_props, None).unwrap();

    let props = section.properties(&arena, None).unwrap();
    assert_eq!(props.get("ALPHA"), Some(&Cow::Borrowed("1")));
    assert_eq!(props.get("BETA"), Some(&Cow::Borrowed("2")));

    let generated = section.generate_id(&mut arena, None).unwrap();
    assert!(!generated.is_empty());
    assert_eq!(
        section.get_id(&arena, None).unwrap(),
        Some(generated.clone())
    );
    assert_eq!(section.generate_id(&mut arena, None).unwrap(), generated);
}

#[test]
fn orgize_headline_value_and_builder_api_roundtrip() {
    let mut arena = Arena::default();
    let section = section_with_properties(&mut arena);
    let headline = section.headline(&arena, None).unwrap();

    assert_eq!(
        headline.get_property("FOO").unwrap(),
        Some(ropey::Rope::from("bar"))
    );
    let props = headline.properties().unwrap();
    assert_eq!(props.get("FOO"), Some(&Cow::Borrowed("bar")));

    let mut builder = headline.to_builder();
    builder.property("ID", "builder-id").unwrap();
    builder.clear_property("FOO").unwrap();

    let mut replacement_props: IndexMap<Cow<'static, str>, Cow<'static, str>> = IndexMap::new();
    replacement_props.insert(Cow::Borrowed("X"), Cow::Borrowed("x"));
    builder.properties(replacement_props).unwrap();

    let built_id = builder.generate_id().unwrap();
    assert!(!built_id.is_empty());

    let rebuilt = builder.headline(None).unwrap();
    section.set_headline(&mut arena, &rebuilt).unwrap();

    let props = section.properties(&arena, None).unwrap();
    assert_eq!(props.get("X"), Some(&Cow::Borrowed("x")));
    assert_eq!(section.get_id(&arena, None).unwrap(), Some(built_id));
}

#[test]
fn orgize_section_property_api_errors_on_non_headline() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("root-only");
    let root = doc.root;

    assert!(matches!(
        root.has_property(&arena, "ID", None),
        Err(HeadlineError::InvalidHeadlineError)
    ));
    assert!(matches!(
        root.get_property(&arena, "ID", None),
        Err(HeadlineError::InvalidHeadlineError)
    ));
    assert!(matches!(
        root.get_id(&arena, None),
        Err(HeadlineError::InvalidHeadlineError)
    ));
    assert!(matches!(
        root.properties(&arena, None),
        Err(HeadlineError::InvalidHeadlineError)
    ));
    assert!(matches!(
        root.set_property(&mut arena, "ID", "x", None),
        Err(HeadlineError::InvalidHeadlineError)
    ));
    assert!(matches!(
        root.clear_property(&mut arena, "ID", None),
        Err(HeadlineError::InvalidHeadlineError)
    ));
    assert!(matches!(
        root.set_properties(&mut arena, IndexMap::new(), None),
        Err(HeadlineError::InvalidHeadlineError)
    ));
    assert!(matches!(
        root.generate_id(&mut arena, None),
        Err(HeadlineError::InvalidHeadlineError)
    ));
}

#[test]
fn orgize_property_mutation_corner_cases() {
    let mut arena = Arena::default();
    let section = section_with_properties(&mut arena);

    // Clearing a missing property is a no-op.
    section.clear_property(&mut arena, "MISSING", None).unwrap();
    assert_eq!(
        section.get_property(&arena, "FOO", None).unwrap(),
        Some(Cow::Borrowed("bar"))
    );

    // Overwrite an existing property value.
    section
        .set_property(&mut arena, "FOO", "baz", None)
        .unwrap();
    assert_eq!(
        section.get_property(&arena, "FOO", None).unwrap(),
        Some(Cow::Borrowed("baz"))
    );

    // Replace entire map with empty set.
    section
        .set_properties(&mut arena, IndexMap::new(), None)
        .unwrap();
    assert!(section.properties(&arena, None).unwrap().is_empty());

    // Add properties back in non-sorted insertion order.
    let mut props: IndexMap<Cow<'static, str>, Cow<'static, str>> = IndexMap::new();
    props.insert(Cow::Borrowed("ZETA"), Cow::Borrowed("26"));
    props.insert(Cow::Borrowed("ALPHA"), Cow::Borrowed("1"));
    section.set_properties(&mut arena, props, None).unwrap();
    assert_eq!(
        section.get_property(&arena, "ALPHA", None).unwrap(),
        Some(Cow::Borrowed("1"))
    );
    assert_eq!(
        section.get_property(&arena, "ZETA", None).unwrap(),
        Some(Cow::Borrowed("26"))
    );
}

#[test]
fn orgize_builder_property_corner_cases() {
    let mut arena = Arena::default();
    let section = section_with_properties(&mut arena);
    let headline = section.headline(&arena, None).unwrap();
    let mut builder = headline.to_builder();

    // Missing clear should not fail.
    builder.clear_property("MISSING").unwrap();

    // Overwrite existing property and add a new one.
    builder.property("FOO", "new").unwrap();
    builder.property("BAR", "two").unwrap();

    let rebuilt = builder.headline(None).unwrap();
    section.set_headline(&mut arena, &rebuilt).unwrap();

    assert_eq!(
        section.get_property(&arena, "FOO", None).unwrap(),
        Some(Cow::Borrowed("new"))
    );
    assert_eq!(
        section.get_property(&arena, "BAR", None).unwrap(),
        Some(Cow::Borrowed("two"))
    );
}
