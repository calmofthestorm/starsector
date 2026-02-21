use starsector::*;

use itertools::Itertools;
use ropey::Rope;
use std::io::Cursor;

#[cfg(test)]
use rand::RngCore;

fn make_alphabet() -> Vec<char> {
    " \n\r\t*a:() []#@%_<>-092\\/,.+-dwym饭\"␤' …‍Âè♀├á	｜▶ "
        .chars().chain("\u{000b}\u{0085}\u{00a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}".chars())
        .unique().collect()
}

fn split_headline_and_body(level: u16, text: &str) -> (String, String) {
    if level == 0 {
        return (String::new(), text.to_string());
    }

    match text.find('\n') {
        Some(i) => (text[..i].to_string(), text[i + 1..].to_string()),
        None => (text.to_string(), String::new()),
    }
}

fn collect_section_semantics(root: Section, arena: &Arena) -> Vec<(u16, String, String, String)> {
    root.descendants(arena)
        .map(|section| {
            let level = section.level(arena);
            let text = section.text(arena).to_string();
            let (headline, body) = split_headline_and_body(level, &text);
            (level, text, headline, body)
        })
        .collect()
}

#[cfg(feature = "headline-parser")]
fn assert_headline_semantics_match_orgize(
    ours: &Section,
    theirs: &orgize::Headline,
    org: &orgize::Org,
    arena: &Arena,
) {
    let ours_headline = ours.headline(arena, None).expect("headline section");
    let title = theirs.title(org);

    assert_eq!(ours_headline.level() as usize, title.level);
    assert_eq!(ours_headline.priority(), title.priority);
    assert_eq!(
        ours_headline.keyword().map(|k| k.to_string()),
        title.keyword.as_ref().map(|k| k.to_string())
    );
    assert_eq!(
        ours_headline
            .tags()
            .map(|t| t.to_string())
            .collect::<Vec<_>>(),
        title.tags.iter().map(|t| t.to_string()).collect::<Vec<_>>()
    );

    let ours_title = ours_headline.title().to_string();
    let ours_raw = if ours_headline.commented() {
        if ours_title.is_empty() {
            "COMMENT".to_string()
        } else {
            format!("COMMENT {}", ours_title)
        }
    } else {
        ours_title
    };
    assert_eq!(ours_raw.trim(), title.raw.trim());

    let ours_planning = ours_headline.planning();
    assert_eq!(
        ours_planning.scheduled.is_some(),
        title.scheduled().is_some()
    );
    assert_eq!(ours_planning.deadline.is_some(), title.deadline().is_some());
    assert_eq!(ours_planning.closed.is_some(), title.closed().is_some());

    #[cfg(feature = "orgize-integration")]
    {
        let ours_props = ours_headline.properties().unwrap();
        let theirs_props = title
            .properties
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<Vec<_>>();
        let ours_props = ours_props
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<Vec<_>>();
        assert_eq!(ours_props, theirs_props);
    }
}

fn child_texts(node: Section, arena: &Arena) -> Vec<String> {
    node.children(arena)
        .map(|s| s.text(arena).to_string())
        .collect()
}

fn assert_parse_entry_points_equivalent(input: &str) {
    let mut arena_str = Arena::default();
    let doc_str = arena_str.parse_str(input);
    let text_str = doc_str.to_rope(&arena_str).to_string();
    let empty_root_str = doc_str.empty_root_section;
    let terminal_newline_str = doc_str.terminal_newline;
    let semantics_str = collect_section_semantics(doc_str.root, &arena_str);

    let mut arena_string = Arena::default();
    let doc_string = arena_string.parse_string(input.to_string());
    let text_string = doc_string.to_rope(&arena_string).to_string();
    let semantics_string = collect_section_semantics(doc_string.root, &arena_string);
    assert_eq!(empty_root_str, doc_string.empty_root_section);
    assert_eq!(terminal_newline_str, doc_string.terminal_newline);
    assert_eq!(text_str, text_string);
    assert_eq!(semantics_str, semantics_string);

    let mut arena_rope = Arena::default();
    let doc_rope = arena_rope.parse_rope(Rope::from(input));
    let text_rope = doc_rope.to_rope(&arena_rope).to_string();
    let semantics_rope = collect_section_semantics(doc_rope.root, &arena_rope);
    assert_eq!(empty_root_str, doc_rope.empty_root_section);
    assert_eq!(terminal_newline_str, doc_rope.terminal_newline);
    assert_eq!(text_str, text_rope);
    assert_eq!(semantics_str, semantics_rope);

    let mut arena_reader = Arena::default();
    let mut reader = Cursor::new(input.as_bytes());
    let doc_reader = arena_reader.parse_reader(&mut reader).unwrap();
    let text_reader = doc_reader.to_rope(&arena_reader).to_string();
    let semantics_reader = collect_section_semantics(doc_reader.root, &arena_reader);
    assert_eq!(empty_root_str, doc_reader.empty_root_section);
    assert_eq!(terminal_newline_str, doc_reader.terminal_newline);
    assert_eq!(text_str, text_reader);
    assert_eq!(semantics_str, semantics_reader);
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
fn test_parse_entry_points_equivalence() {
    for input in [
        "",
        "\n",
        " ",
        "hello",
        "hello\nworld",
        "* H1",
        "* H1\nBody line\n** H2\n- bullet",
        "\n* One\n** Two\n** Another\n* Three\n",
        "饭\n* 标题\n  正文\n",
        "*** Stars\n* Space\n\nTrailing\n",
    ] {
        assert_parse_entry_points_equivalent(input);
    }
}

#[test]
fn test_parse_reader_invalid_utf8_errors() {
    let mut arena = Arena::default();
    let mut reader = Cursor::new(vec![0xff, 0xfe, 0x00]);
    assert!(arena.parse_reader(&mut reader).is_err());
}

#[test]
fn test_tree_traversal_iterators_and_offsets() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("* A\n** A1\n*** A1a\n** A2\n* B\n");

    let root = doc.root;
    let a = root.children(&arena).next().unwrap();
    let b = root.children(&arena).nth(1).unwrap();
    let a1 = a.children(&arena).next().unwrap();
    let a2 = a.children(&arena).nth(1).unwrap();
    let a1a = a1.children(&arena).next().unwrap();

    assert_eq!(child_texts(root, &arena), vec!["* A", "* B"]);
    assert_eq!(child_texts(a, &arena), vec!["** A1", "** A2"]);
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

    for node in [a, b, a1, a2, a1a] {
        let offset = doc.text_offset_of_child(&arena, node).unwrap();
        let (at_node, at_offset) = doc.at(&arena, offset).unwrap();
        assert_eq!(at_node, node);
        assert!(at_offset <= node.level(&arena) as usize);
    }

    let section_offset = a.text_offset_of_child(&arena, a1a).unwrap();
    let (at_node, at_offset) = a.at(&arena, section_offset).unwrap();
    assert_eq!(at_node, a1a);
    assert!(at_offset < a1a.text(&arena).len_chars());
    assert!(a.text_offset_of_child(&arena, b).is_none());
}

#[test]
fn test_tree_edit_and_remove_apis() {
    let mut arena = Arena::default();
    let doc = arena.parse_str("* Parent\n");
    let root = doc.root;
    let parent = root.children(&arena).next().unwrap();

    let append_child = arena.new_section("append child".into()).unwrap();
    parent.append(&mut arena, append_child).unwrap();
    assert_eq!(append_child.level(&arena), 2);
    assert_eq!(append_child.text(&arena), "** append child");

    let prepend_child = arena.new_section("prepend child".into()).unwrap();
    parent.prepend(&mut arena, prepend_child).unwrap();
    assert_eq!(prepend_child.level(&arena), 2);
    assert_eq!(
        child_texts(parent, &arena),
        vec!["** prepend child", "** append child"]
    );

    let before = arena.new_section("before".into()).unwrap();
    parent.insert_before(&mut arena, before).unwrap();
    assert_eq!(before.level(&arena), 1);
    assert_eq!(before.text(&arena), "* before");

    let after = arena.new_section("after".into()).unwrap();
    parent.insert_after(&mut arena, after).unwrap();
    assert_eq!(after.level(&arena), 1);
    assert_eq!(after.text(&arena), "* after");
    assert_eq!(
        child_texts(root, &arena),
        vec!["* before", "* Parent", "* after"]
    );

    let bad_child = arena.new_section("* Bad child".into()).unwrap();
    assert!(matches!(
        parent.checked_append(&mut arena, bad_child),
        Err(StructureError::LevelError)
    ));
    assert!(matches!(
        parent.checked_prepend(&mut arena, bad_child),
        Err(StructureError::LevelError)
    ));

    let bad_sibling = arena.new_section("bad sibling".into()).unwrap();
    assert!(matches!(
        parent.checked_insert_before(&mut arena, bad_sibling),
        Err(StructureError::LevelError)
    ));
    assert!(matches!(
        parent.checked_insert_after(&mut arena, bad_sibling),
        Err(StructureError::LevelError)
    ));

    let root_sibling = arena.new_section("* root sibling".into()).unwrap();
    assert!(matches!(
        root.insert_after(&mut arena, root_sibling),
        Err(StructureError::LevelError)
    ));
    assert!(matches!(
        root.insert_before(&mut arena, root_sibling),
        Err(StructureError::LevelError)
    ));

    let mut remove_arena = Arena::default();
    let remove_doc = remove_arena.parse_str("* A\n** X\n*** X1\n** Y\n* B\n");
    let remove_root = remove_doc.root;
    let a = remove_root.children(&remove_arena).next().unwrap();
    let b = remove_root.children(&remove_arena).nth(1).unwrap();
    let x = a.children(&remove_arena).next().unwrap();
    let y = a.children(&remove_arena).nth(1).unwrap();

    x.remove_subtree(&mut remove_arena);
    assert_eq!(child_texts(a, &remove_arena), vec!["** Y"]);
    assert_eq!(x.to_rope(&remove_arena).to_string(), "** X\n*** X1\n");

    a.replace_with_children(&mut remove_arena);
    assert_eq!(child_texts(remove_root, &remove_arena), vec!["** Y", "* B"]);
    assert_eq!(y.parent(&remove_arena), Some(remove_root));
    assert_eq!(b.parent(&remove_arena), Some(remove_root));

    let c = remove_arena.new_section("* C".into()).unwrap();
    remove_root.append(&mut remove_arena, c).unwrap();
    let c1 = remove_arena.new_section("c1".into()).unwrap();
    let c2 = remove_arena.new_section("c2".into()).unwrap();
    c.append(&mut remove_arena, c1).unwrap();
    c.append(&mut remove_arena, c2).unwrap();
    assert_eq!(c.children(&remove_arena).count(), 2);
    c.remove_children(&mut remove_arena);
    assert_eq!(c.children(&remove_arena).count(), 0);
    assert!(c1.parent(&remove_arena).is_none());
    assert!(c2.parent(&remove_arena).is_none());
}

#[test]
fn test_identity_nearly_exhaustive_short() {
    let alphabet = " \r\n\t*a饭\u{2028}\u{2029}";
    let mut arena = Arena::default();
    let max_length = if cfg!(debug_assertions) { 5 } else { 7 };

    for length in 0..max_length {
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
    let max_length = if cfg!(debug_assertions) { 3 } else { 5 };

    for length in 0..max_length {
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
        if let Some(stripped) = text.strip_suffix('\n') {
            assert_eq!(root.text(&arena), stripped);
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

        if let Some(stripped) = text.strip_suffix('\n') {
            assert_eq!(child_id.text(&arena), stripped);
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
    let another_id = one_id.children(&arena).nth(1).unwrap();
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
        let length = (rng.next_u32() as usize) % (max_size - min_size) + min_size;
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
            let level = ours.level(arena) as usize;
            assert_eq!(level, theirs.level());
            let mut text = ours.text(arena).slice(..);
            if level > 0 {
                text = text.slice(level + 1..);
            }
            assert_eq!(
                text.lines()
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_default()
                    .trim(),
                theirs.title(org).raw.trim()
            );
            assert_eq!(ours.children(arena).count(), theirs.children(org).count());

            #[cfg(feature = "headline-parser")]
            assert_headline_semantics_match_orgize(ours, theirs, org, arena);

            for (ours, theirs) in ours.children(arena).zip(theirs.children(org)) {
                compare_node(&ours, &theirs, org, arena);
            }
        }

        for (ours, theirs) in doc.root.children(&arena).zip(org.document().children(&org)) {
            compare_node(&ours, &theirs, &org, &arena);
        }

        let roundtrip = doc.to_rope(&arena).to_string();
        let reparsed = arena.parse_str(&roundtrip);
        assert_eq!(
            doc.to_rope(&arena).to_string(),
            reparsed.to_rope(&arena).to_string()
        );

        let semantics_1 = collect_section_semantics(doc.root, &arena);
        let semantics_2 = collect_section_semantics(reparsed.root, &arena);
        assert_eq!(semantics_1.len(), semantics_2.len());
        for (left, right) in semantics_1.iter().zip(semantics_2.iter()) {
            assert_eq!(left.0, right.0);
            assert_eq!(left.1, right.1);
            assert_eq!(left.2, right.2);
            assert_eq!(left.3, right.3);
        }
    };

    let short_fuzz_iters = if cfg!(debug_assertions) { 120 } else { 2500 };
    let long_fuzz_iters = if cfg!(debug_assertions) { 4 } else { 100 };

    for _ in 0..short_fuzz_iters {
        test_with_size(0, 250);
    }

    for _ in 0..long_fuzz_iters {
        test_with_size(100000, 120000);
    }
}
