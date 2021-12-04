use std::borrow::Cow;

use rand::RngCore;
use ropey::Rope;

use starsector::*;

#[test]
fn level() {
    HeadlineBuilder::default()
        .level(0)
        .headline(None)
        .unwrap_err();

    HeadlineBuilder::default()
        .level(0)
        .validate_partially(None)
        .unwrap_err();

    HeadlineBuilder::default().level(1).headline(None).unwrap();

    HeadlineBuilder::default()
        .level(1)
        .validate_partially(None)
        .unwrap();

    HeadlineBuilder::default()
        .level(1024)
        .headline(None)
        .unwrap();

    HeadlineBuilder::default()
        .level(1024)
        .validate_partially(None)
        .unwrap();
}

#[test]
fn commented() {
    HeadlineBuilder::default()
        .commented(true)
        .headline(None)
        .unwrap();

    HeadlineBuilder::default()
        .commented(true)
        .validate_partially(None)
        .unwrap();
}

#[test]
fn keyword() {
    for keyword in ["", "TODO", "DONE"] {
        let keyword = if keyword.is_empty() {
            None
        } else {
            Some(keyword.into())
        };

        HeadlineBuilder::default()
            .keyword(keyword.clone())
            .headline(None)
            .unwrap();

        HeadlineBuilder::default()
            .keyword(keyword)
            .validate_partially(None)
            .unwrap();
    }

    let keyword = Some("POTATO".into());

    HeadlineBuilder::default()
        .keyword(keyword.clone())
        .headline(None)
        .unwrap_err();

    HeadlineBuilder::default()
        .keyword(keyword)
        .validate_partially(None)
        .unwrap_err();
}

#[test]
fn priority() {
    HeadlineBuilder::default()
        .priority(None)
        .headline(None)
        .unwrap();

    HeadlineBuilder::default()
        .priority(None)
        .validate_partially(None)
        .unwrap();

    HeadlineBuilder::default()
        .priority(Some('A'))
        .headline(None)
        .unwrap();

    HeadlineBuilder::default()
        .priority(Some('A'))
        .validate_partially(None)
        .unwrap();

    for bad in "a9! ".chars() {
        HeadlineBuilder::default()
            .priority(Some(bad))
            .headline(None)
            .unwrap_err();

        HeadlineBuilder::default()
            .priority(Some(bad))
            .validate_partially(None)
            .unwrap_err();
    }
}

#[test]
fn tags() {
    let invalid = &["+", "-", "@+", "☃", " hi", "hi "];

    for tag in invalid {
        assert_eq!(
            HeadlineBuilder::default().add_tag(*tag).headline(None).ok(),
            None
        );
    }

    let valid = &[
        HeadlineBuilder::default().add_tag("tomatoes").clone(),
        HeadlineBuilder::default().add_tag("#pasta").clone(),
        HeadlineBuilder::default().add_tag("tartine@").clone(),
        HeadlineBuilder::default().add_tag("麻婆豆腐").clone(),
        HeadlineBuilder::default()
            .add_tag("apple:pie")
            .remove_tags(&["pie"])
            .clone(),
        HeadlineBuilder::default().add_tag("abc_123%").clone(),
        HeadlineBuilder::default()
            .set_tags(vec!["a", "a"].iter().map(|s| Cow::Borrowed(&**s)))
            .update_tags(vec!["b", "a", "b"].iter().map(|s| Cow::Borrowed(&**s)))
            .clone(),
        HeadlineBuilder::default()
            .add_tag("abc_123%")
            .clear_tags()
            .clone(),
        HeadlineBuilder::default()
            .add_tag("%")
            .add_tag("abc")
            .clone(),
        HeadlineBuilder::default()
            .add_tag("%")
            .clear_tag("%")
            .clone(),
        HeadlineBuilder::default()
            .set_raw_tags_string("a:b:c:d:e".to_string())
            .add_tag("%")
            .clear_tag("b")
            .clone(),
        HeadlineBuilder::default()
            .set_raw_tags("a:b:c:d:e")
            .add_tag("%")
            .clear_tag("b")
            .clone(),
    ];

    for builder in valid {
        builder.headline(None).unwrap();
        builder.validate_partially(None).unwrap();
    }
}

#[test]
fn body() {
    let valid = &[
        HeadlineBuilder::default().body(Rope::default()).clone(),
        HeadlineBuilder::default().body("*".into()).clone(),
        HeadlineBuilder::default().body(" * ".into()).clone(),
        HeadlineBuilder::default().body("- ".into()).clone(),
        HeadlineBuilder::default().body("- Hello".into()).clone(),
        HeadlineBuilder::default()
            .body(" * TODO Hello".into())
            .clone(),
    ];

    for builder in valid {
        builder.headline(None).unwrap();
        builder.validate_partially(None).unwrap();
    }

    let invalid = &[
        HeadlineBuilder::default().body("* ".into()).clone(),
        HeadlineBuilder::default().body("* \n".into()).clone(),
        HeadlineBuilder::default().body("\n* ".into()).clone(),
        HeadlineBuilder::default().body(" \n* ".into()).clone(),
    ];

    for builder in invalid {
        builder.headline(None).unwrap_err();
        builder.validate_partially(None).unwrap_err();
    }
}

#[test]
fn title() {
    HeadlineBuilder::default()
        .title("Hello world!".into())
        .headline(None)
        .unwrap();

    HeadlineBuilder::default()
        .title(Rope::default())
        .headline(None)
        .unwrap();

    HeadlineBuilder::default()
        .title("* TODO Hello".into())
        .headline(None)
        .unwrap();

    HeadlineBuilder::default()
        .title("[#9] Hello".into())
        .headline(None)
        .unwrap();

    HeadlineBuilder::default()
        .title("[#A] Hello".into())
        .headline(None)
        .unwrap_err();

    HeadlineBuilder::default()
        .title("TODO Hello".into())
        .headline(None)
        .unwrap_err();

    HeadlineBuilder::default()
        .title("Hello :world:".into())
        .headline(None)
        .unwrap_err();

    HeadlineBuilder::default()
        .title("COMMENT Hello".into())
        .headline(None)
        .unwrap_err();

    HeadlineBuilder::default()
        .title("COMMENT".into())
        .headline(None)
        .unwrap_err();
}

#[test]
fn fuzz() {
    let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(30);

    let tags = &["a", "#", "@hello", "50%", "麻婆豆腐", "b", "c:d", "c", "d"];

    let choose_tag = |rng: &mut rand::rngs::StdRng| tags[(rng.next_u32() as usize) % tags.len()];

    let choose_tags = |rng: &mut rand::rngs::StdRng| {
        let mut tags = Vec::new();

        for _ in 0..(rng.next_u32() % 5) {
            tags.push(choose_tag(rng));
        }

        if !tags.is_empty() && rng.next_u32() % 2 == 0 {
            let t = (rng.next_u32() as usize) % tags.len();
            let t = tags[t];
            tags.push(t);
        }

        tags
    };

    let choose_title = |rng: &mut rand::rngs::StdRng| {
        let c = rng.next_u32() % 5;
        if c == 0 {
            ""
        } else if c == 1 {
            "hello"
        } else if c == 2 {
            "- Test"
        } else if c == 3 {
            "* HI"
        } else if c == 4 {
            "* [#A] Hello"
        } else {
            "* TODO Hello"
        }
    };

    let choose_body = |rng: &mut rand::rngs::StdRng| {
        let c = rng.next_u32() % 7;
        if c == 0 {
            ""
        } else if c == 1 {
            "hello"
        } else if c == 2 {
            "- Test\n- That"
        } else if c == 3 {
            "\n"
        } else if c == 4 {
            "\n * HI"
        } else if c == 5 {
            " * [#A] Hello"
        } else if c == 6 {
            " * Hello\n * World"
        } else {
            " * Hello\n * World\n"
        }
    };

    for _ in 0..5000 {
        let mut builder = HeadlineBuilder::default();

        for _ in 0..(rng.next_u32() % 100) {
            let c = rng.next_u32() % 7;
            if c == 0 {
                let c = rng.next_u32() % 10;
                if c == 0 {
                    builder.add_tag(&choose_tag(&mut rng));
                } else if c == 1 {
                    builder.clear_tag(&choose_tag(&mut rng));
                    builder.add_tag(&choose_tag(&mut rng));
                } else if c == 2 {
                    builder.add_tag(&choose_tag(&mut rng));
                    builder.add_tag(&choose_tag(&mut rng));
                } else if c == 3 {
                    builder.clear_tag(&choose_tag(&mut rng));
                    builder.clear_tag(&choose_tag(&mut rng));
                } else if c == 4 {
                    builder.clear_tags();
                } else if c == 5 {
                    builder.update_tags(choose_tags(&mut rng).iter().map(|s| Cow::Borrowed(&**s)));
                } else if c == 6 {
                    builder.remove_tags(&choose_tags(&mut rng));
                } else if c == 7 {
                    builder.set_tags(choose_tags(&mut rng).iter().map(|s| Cow::Borrowed(&**s)));
                } else if c == 8 {
                    builder.set_raw_tags_string(choose_tags(&mut rng).join(":"));
                } else {
                    builder.canonical_tags();
                }
            } else if c == 1 {
                builder.commented(rng.next_u32() % 2 == 0);
            } else if c == 2 {
                let c = rng.next_u32() % 3;
                if c == 0 {
                    builder.priority(None);
                } else if c == 1 {
                    builder.priority(Some('A'));
                } else if c == 2 {
                    builder.priority(Some('B'));
                }
            } else if c == 3 {
                let c = rng.next_u32() % 3;
                if c == 0 {
                    builder.keyword(None);
                } else if c == 1 {
                    builder.keyword(Some("TODO".into()));
                } else if c == 2 {
                    builder.keyword(Some("DONE".into()));
                }
            } else if c == 4 {
                builder.level((rng.next_u32() % 10 + 1) as u16);
            } else if c == 5 {
                builder.title(choose_title(&mut rng).into());
            } else if c == 6 {
                builder.body(choose_body(&mut rng).into());
            }
        }

        builder.headline(None).unwrap();
    }
}
