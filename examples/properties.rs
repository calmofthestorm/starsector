use std::borrow::Cow;

use indexmap::IndexMap;
use starsector::{Arena, Context, HeadlineBuilder};

fn main() {
    let context = Context::from_keywords(&["TODO", "DONE"]);
    let mut arena = Arena::default();
    let initial = "* TODO task\n:PROPERTIES:\n:FOO: one\n:END:\n";
    println!("Initial org:\n{initial}");
    let doc = arena.parse_str(initial);
    let section = doc.root.children(&arena).next().expect("missing section");

    assert!(
        section
            .has_property(&arena, "FOO", Some(&context))
            .expect("has_property failed")
    );
    assert_eq!(
        section
            .get_property(&arena, "FOO", Some(&context))
            .expect("get_property failed"),
        Some(Cow::Borrowed("one"))
    );

    let all_props = section
        .properties(&arena, Some(&context))
        .expect("properties failed");
    assert_eq!(all_props.get("FOO").map(|s| s.as_ref()), Some("one"));

    section
        .set_property(&mut arena, "BAR", "two", Some(&context))
        .expect("set_property failed");
    section
        .clear_property(&mut arena, "FOO", Some(&context))
        .expect("clear_property failed");

    let mut replacement = IndexMap::new();
    replacement.insert(Cow::Borrowed("OWNER"), Cow::Borrowed("alex"));
    replacement.insert(Cow::Borrowed("STATE"), Cow::Borrowed("active"));
    section
        .set_properties(&mut arena, replacement, Some(&context))
        .expect("set_properties failed");

    let id = section
        .generate_id(&mut arena, Some(&context))
        .expect("generate_id failed");
    assert!(!id.is_empty());

    let mut builder = HeadlineBuilder::default();
    builder
        .level(1)
        .title("builder node".into())
        .keyword(Some("TODO".into()))
        .property("A", "1")
        .expect("builder property failed")
        .clear_property("A")
        .expect("builder clear_property failed")
        .property("B", "2")
        .expect("builder property failed");

    let mut bprops = IndexMap::new();
    bprops.insert(Cow::Borrowed("X"), Cow::Borrowed("10"));
    bprops.insert(Cow::Borrowed("Y"), Cow::Borrowed("20"));
    builder
        .properties(bprops)
        .expect("builder set properties failed");

    let builder_id = builder.generate_id().expect("builder generate_id failed");
    assert!(!builder_id.is_empty());

    let built = builder
        .headline(Some(&context))
        .expect("builder headline failed");
    let built_props = built.properties().expect("headline properties failed");
    assert!(built_props.contains_key("X"));

    println!("Final edited org:\n{}", doc.to_rope(&arena));
    println!("Built headline org:\n{}", built.to_rope());
    println!("Properties API exercised on Section and HeadlineBuilder.");
}
