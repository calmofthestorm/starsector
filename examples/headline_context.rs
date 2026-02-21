use std::borrow::Cow;

use ropey::Rope;
use starsector::{Arena, Context, Date, Point, parse_valid_single_headline};

fn main() {
    let context = Context::from_keywords(&["TODO", "WAIT", "DONE"]);

    let raw = Rope::from("* WAIT [#B] ship docs :ops:ops:rust:\nSCHEDULED: <2026-03-14 Sat>\nBody");
    println!("Initial headline org:\n{}", raw);
    let parsed = parse_valid_single_headline(raw.slice(..), &context);
    assert_eq!(parsed.keyword().map(|k| k.to_string()), Some("WAIT".into()));

    let mut builder = parsed.to_builder();
    builder
        .canonical_tags()
        .set_raw_tags_string("ops:build".to_string())
        .set_raw_tags("ops:build:release")
        .set_tags([Cow::Borrowed("ops"), Cow::Borrowed("rust")].into_iter())
        .update_tags([Cow::Borrowed("infra")].into_iter())
        .remove_tags(&["ops"])
        .add_tag("docs")
        .clear_tag("infra")
        .priority(Some('A'))
        .commented(true)
        .title("release notes".into())
        .keyword(Some("TODO".into()))
        .scheduled(Some(Point::new(Date::new(2026, 4, 2)).into()))
        .deadline(None)
        .closed(None)
        .body("Body updated".into());

    assert!(builder.has_tag("docs"));
    builder
        .validate_partially(Some(&context))
        .expect("builder should validate");

    let headline = builder
        .headline(Some(&context))
        .expect("headline build failed");
    let rendered = builder.to_rope(Some(&context)).expect("render failed");
    assert!(
        rendered
            .to_string()
            .starts_with("* TODO [#A] COMMENT release notes")
    );

    let mut arena = Arena::default();
    let doc = arena.parse_str("* WAIT original\n");
    let section = doc.root.children(&arena).next().expect("missing section");

    section
        .set_headline(&mut arena, &headline)
        .expect("set_headline failed");

    section
        .set_raw_tags(&mut arena, "prod:canary", Some(&context))
        .expect("set_raw_tags failed");
    section
        .set_tags(
            &mut arena,
            [Cow::Borrowed("prod"), Cow::Borrowed("green")].into_iter(),
            Some(&context),
        )
        .expect("set_tags failed");
    section
        .update_tags(
            &mut arena,
            [Cow::Borrowed("rollout")].into_iter(),
            Some(&context),
        )
        .expect("update_tags failed");
    section
        .remove_tags(&mut arena, &["green"], Some(&context))
        .expect("remove_tags failed");
    section
        .clear_tags(&mut arena, Some(&context))
        .expect("clear_tags failed");
    section
        .set_title(&mut arena, "final title".into(), Some(&context))
        .expect("set_title failed");
    section
        .set_commented(&mut arena, false, Some(&context))
        .expect("set_commented failed");
    section
        .set_body(&mut arena, "single line body".into(), Some(&context))
        .expect("set_body failed");

    let reparsed = section
        .headline(&arena, Some(&context))
        .expect("headline parse failed");
    assert_eq!(reparsed.title().to_string(), "final title");

    println!("Final edited org:\n{}", doc.to_rope(&arena));
    println!("Headline context and editing APIs exercised successfully.");
}
