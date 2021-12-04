use starsector::*;

use itertools::Itertools;
use ropey::Rope;

#[cfg(test)]
use rand::RngCore;

fn make_alphabet() -> Vec<char> {
    " \n\r\t*a:() []#@%_<>-092\\/,.+-dwym饭\"␤' …‍Âè♀├á	｜▶ "
        .chars().chain("\u{000b}\u{0085}\u{00a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}".chars())
        .unique().collect()
}

#[test]
fn test_identity() {
    let mut arena = Arena::default();

    for s in &["", " ", "  ", "* ", "** Hello"] {
        let root = arena.parse_str(s);
        assert_eq!(root.to_rope(&arena).to_string(), *s);
    }
}

#[test]
fn test_identity_nearly_exhaustive_short() {
    let alphabet = " \r\n\t*a饭\u{2028}\u{2029}";
    let mut arena = Arena::default();

    for length in 0..7 {
        for s in (0..length)
            .map(|_| alphabet.chars())
            .multi_cartesian_product()
        {
            let s: String = s.into_iter().collect();
            let root = arena.parse_str(&s);
            assert_eq!(root.to_rope(&arena).to_string(), s);
        }
    }
}

#[test]
fn test_identity_exhaustive_very_short() {
    let alphabet = " \r\n\t*a饭\u{000b}\u{0085}\u{00a0}\u{1680}\u{2001}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}";
    let mut arena = Arena::default();

    for length in 0..5 {
        for s in (0..length)
            .map(|_| alphabet.chars())
            .multi_cartesian_product()
        {
            let s: String = s.into_iter().collect();
            let root = arena.parse_str(&s);
            assert_eq!(root.to_rope(&arena).to_string(), s);
        }
    }
}

#[test]
fn test_identity_fuzz() {
    let alphabet = make_alphabet();
    let mut arena = Arena::default();

    let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(0);

    let mut s = String::default();
    for _ in 0..10000 {
        s.clear();
        let length = (rng.next_u32() % 75) as usize;
        for _ in 0..length {
            s.push(alphabet[(rng.next_u32() as usize) % alphabet.len()])
        }
        let root = arena.parse_str(&s);
        let fix = root.to_rope(&arena).to_string();
        assert_eq!(fix, s);
    }
}

#[test]
fn test_identity_fuzz_slow() {
    let alphabet = make_alphabet();
    let mut arena = Arena::default();

    let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(2);

    let mut s = String::default();
    for _ in 0..10000 {
        s.clear();
        let length = (rng.next_u32() % 1000) as usize;
        for _ in 0..length {
            s.push(alphabet[(rng.next_u32() as usize) % alphabet.len()])
        }
        let root = arena.parse_str(&s);
        let fix = root.to_rope(&arena).to_string();
        assert_eq!(fix, s);
    }
}

#[test]
fn test_identity_fuzz_big() {
    let alphabet = make_alphabet();
    let mut arena = Arena::default();

    let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(10);

    let mut s = String::default();
    for _ in 0..100 {
        s.clear();
        let length = (rng.next_u32() % 1000 + 10000) as usize;
        for _ in 0..length {
            s.push(alphabet[(rng.next_u32() as usize) % alphabet.len()])
        }
        let root = arena.parse_str(&s);
        let fix = root.to_rope(&arena).to_string();
        assert_eq!(fix, s);
    }
}

#[test]
fn test_parse_section() {
    // Some of these may seem surprising. I care about faithful handling of
    // edge cases for Document but not for Section. If you need API
    // stability, or want precise reproduction, use that instead. If you're
    // happy trimming here and removing whitespace there, this is convenient
    // in some cases.
    let mut arena = Arena::default();

    let root = arena.new_section("Hello".into()).unwrap();
    assert_eq!(root.to_rope(&arena).to_string(), "Hello\n");

    let root = arena.new_section(Rope::default()).unwrap();
    assert_eq!(root.to_rope(&arena).to_string(), "");

    let root = arena.new_section(" ".into()).unwrap();
    assert_eq!(root.to_rope(&arena).to_string(), " \n");

    let root = arena.new_section("\n".into()).unwrap();
    assert_eq!(root.to_rope(&arena).to_string(), "");

    let root = arena.new_section(" \n".into()).unwrap();
    assert_eq!(root.to_rope(&arena).to_string(), " \n");

    let root = arena.new_section("\n ".into()).unwrap();
    assert_eq!(root.to_rope(&arena).to_string(), "\n \n");

    let root = arena.new_section("** Hello".into()).unwrap();
    assert_eq!(root.to_rope(&arena).to_string(), "** Hello\n");

    let root = arena.new_section("HI!!!** Hello".into()).unwrap();
    assert_eq!(root.to_rope(&arena).to_string(), "HI!!!** Hello\n");

    // Must have exactly one root node.
    assert!(arena.new_section("* Nope\n* Nope".into()).is_none());

    // Multiple sections are allowed, so long as there is a single root.
    let root = arena.new_section("* Nope\n*** Nope2".into()).unwrap();
    assert_eq!(root.to_rope(&arena).to_string(), "* Nope\n*** Nope2\n");

    let root = arena.new_section("Nope\n* Nope2".into()).unwrap();
    assert_eq!(root.to_rope(&arena).to_string(), "Nope\n* Nope2\n");
}

#[test]
fn test_parsing() {
    let mut arena = Arena::default();

    let doc = arena.parse_str("");
    let root = doc.root;
    assert_eq!(root.text(&arena), "");
    assert_eq!(0, root.level(&arena));
    assert!(root.parent(&arena).is_none());
    assert_eq!(0, root.children(&arena).count());

    let doc = arena.parse_str("* ");
    let root = doc.root;
    assert_eq!(root.text(&arena), "");
    assert_eq!(0, root.level(&arena));
    assert_eq!(1, root.children(&arena).count());
    let child_id = root.children(&arena).next().unwrap();
    assert_eq!(0, child_id.children(&arena).count());
    assert_eq!(child_id.parent(&arena), Some(root));

    for text in &[
        " ",
        "\n",
        " \n",
        "hello",
        "hello\n",
        "hello\nworld",
        "hello\nworld",
    ] {
        let doc = arena.parse_str(text);
        let root = doc.root;
        if text.ends_with("\n") {
            assert_eq!(*root.text(&arena), text[..text.len() - 1]);
        } else {
            assert_eq!(root.text(&arena), *text);
        }
        assert_eq!(0, root.level(&arena));
        assert!(root.parent(&arena).is_none());
        assert_eq!(0, root.children(&arena).count());
    }

    for text in &[
        "* ",
        "* \n",
        "* *\n",
        "* * ",
        "* Hello",
        "* Hello\n",
        "*** Hello!!!",
        "* Has body\n  Body",
        "*** Has bullets\n * B1\n * B2\n",
        "* Comma\n,* Hello!\n",
    ] {
        let doc = arena.parse_str(text);
        let root = doc.root;
        assert!(root.text(&arena).is_empty());
        assert!(doc.empty_root_section);

        assert_eq!(0, root.level(&arena));
        assert_eq!(1, root.children(&arena).count());
        let child_id = root.children(&arena).next().unwrap();
        assert_eq!(0, child_id.children(&arena).count());
        assert_eq!(child_id.parent(&arena), Some(root));

        if text.ends_with("\n") {
            assert_eq!(*child_id.text(&arena), text[..text.len() - 1]);
        } else {
            assert_eq!(child_id.text(&arena), *text);
        }
    }

    let doc = arena.parse_str("* Curved\n* Swords");
    let root = doc.root;
    assert!(root.text(&arena).is_empty());
    assert!(doc.empty_root_section);

    assert_eq!(0, root.level(&arena));
    assert_eq!(2, root.children(&arena).count());
    let mut ci = root.children(&arena);

    let child_id1 = ci.next().unwrap();
    let child_id2 = ci.next().unwrap();

    assert_eq!(0, child_id1.children(&arena).count());
    assert_eq!(child_id1.parent(&arena), Some(root));

    assert_eq!(0, child_id2.children(&arena).count());
    assert_eq!(child_id2.parent(&arena), Some(doc.root));

    assert_eq!(child_id1.text(&arena), "* Curved");
    assert_eq!(child_id2.text(&arena), "* Swords");

    let doc = arena.parse_str("\n* One\n** Two\n** Another\n* One\n*** Three\n     Hello!\n");
    let root = doc.root;
    assert!(root.text(&arena).is_empty());
    assert!(!doc.empty_root_section);

    assert_eq!(0, root.level(&arena));
    assert_eq!(2, root.children(&arena).count());
    let mut ci = root.children(&arena);

    let one_id = ci.next().unwrap();
    let one_id2 = ci.next().unwrap();

    assert_eq!(2, one_id.children(&arena).count());
    assert_eq!(1, one_id2.children(&arena).count());

    let two_id = one_id.children(&arena).next().unwrap();
    let another_id = one_id.children(&arena).skip(1).next().unwrap();
    let three_id = one_id2.children(&arena).next().unwrap();

    assert_eq!(one_id.text(&arena), "* One");
    assert_eq!(one_id.level(&arena), 1);

    assert_eq!(another_id.text(&arena), "** Another");
    assert_eq!(another_id.level(&arena), 2);

    assert_eq!(one_id2.text(&arena), "* One");
    assert_eq!(one_id2.level(&arena), 1);

    assert_eq!(three_id.text(&arena), "*** Three\n     Hello!");
    assert_eq!(three_id.level(&arena), 3);

    assert_eq!(two_id.text(&arena), "** Two");
    assert_eq!(two_id.level(&arena), 2);
}

#[test]
fn test_fuzz_delta() {
    let mut arena = Arena::default();
    let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(30);
    let mut alphabet: Vec<_> = vec![" ", "\n", "a", "饭"]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    for i in 1..5 {
        let mut ss: Vec<_> = vec![" ", "a", "饭"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let mut s = String::default();
        for _ in 0..i {
            for j in &mut ss {
                s.push('*');
                j.push('*');
            }
        }
        alphabet.append(&mut ss);
        s.push(' ');
        alphabet.push(s);
    }

    let mut test_with_size = |min_size: usize, max_size: usize| {
        let mut s = String::default();
        s.clear();
        let length = ((rng.next_u32() as usize) % (max_size - min_size) + min_size) as usize;
        for _ in 0..length {
            s += &alphabet[(rng.next_u32() as usize) % alphabet.len()];
        }
        let doc = arena.parse_str(&s);
        let org = orgize::Org::parse(&s);

        assert_eq!(
            doc.root.children(&arena).count(),
            org.document().children(&org).count()
        );

        // FIXME This only compares titles, and skips sections (including
        // the root).
        fn compare_node(
            ours: &Section,
            theirs: &orgize::Headline,
            org: &orgize::Org,
            arena: &Arena,
        ) {
            let level = ours.level(&arena) as usize;
            assert_eq!(level, theirs.level());
            let mut text = ours.text(&arena).slice(..);
            if level > 0 {
                text = text.slice(level + 1..);
            }
            assert_eq!(
                text.lines()
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_default()
                    .trim(),
                theirs.title(&org).raw.trim()
            );
            assert_eq!(ours.children(&arena).count(), theirs.children(&org).count());

            for (ours, theirs) in ours.children(&arena).zip(theirs.children(&org)) {
                compare_node(&ours, &theirs, &org, &arena);
            }
        }

        for (ours, theirs) in doc.root.children(&arena).zip(org.document().children(&org)) {
            compare_node(&ours, &theirs, &org, &arena);
        }

        doc.root.children(&arena).count();
    };

    for _ in 0..2500 {
        test_with_size(0, 250);
    }

    for _ in 0..100 {
        test_with_size(100000, 120000);
    }
}
