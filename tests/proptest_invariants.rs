use std::collections::HashSet;
use std::io::Cursor;

use proptest::prelude::*;
#[cfg(feature = "headline-parser")]
use proptest::string::string_regex;
use ropey::Rope;
use starsector::*;

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

fn assert_tree_invariants(doc: &Document, arena: &Arena) {
    let root = doc.root;
    assert_eq!(root.level(arena), 0);
    assert!(root.parent(arena).is_none());

    let descendants: Vec<_> = root.descendants(arena).collect();
    let mut seen: HashSet<Section> = HashSet::new();

    for node in descendants.iter().copied() {
        assert!(seen.insert(node));
        if node == root {
            continue;
        }

        let parent = node.parent(arena).expect("non-root must have parent");
        assert!(node.level(arena) > parent.level(arena));

        let mut has_root = false;
        for ancestor in node.ancestors(arena) {
            if ancestor == root {
                has_root = true;
                break;
            }
        }
        assert!(has_root, "every node should reach root through ancestors");
    }

    let emitted = doc.to_rope(arena).to_string();
    let mut reparsed_arena = Arena::default();
    let reparsed = reparsed_arena.parse_str(&emitted);
    assert_eq!(reparsed.to_rope(&reparsed_arena).to_string(), emitted);
}

fn descendants(root: Section, arena: &Arena) -> Vec<Section> {
    root.descendants(arena).collect()
}

fn make_headline_text(level: u16, label: &str) -> String {
    format!("{} {}", "*".repeat(level as usize), label)
}

fn apply_valid_mirrored_edit(
    doc_checked: &Document,
    arena_checked: &mut Arena,
    doc_unchecked: &Document,
    arena_unchecked: &mut Arena,
    op_kind: u8,
    target_index: usize,
    extra_depth: u16,
    step: usize,
) {
    let nodes_checked = descendants(doc_checked.root, arena_checked);
    let nodes_unchecked = descendants(doc_unchecked.root, arena_unchecked);
    assert_eq!(nodes_checked.len(), nodes_unchecked.len());
    if nodes_checked.is_empty() {
        return;
    }

    let mut index = target_index % nodes_checked.len();
    let mut anchor_checked = nodes_checked[index];
    let mut anchor_unchecked = nodes_unchecked[index];

    let op = op_kind % 4;
    if (op == 2 || op == 3) && anchor_checked.parent(arena_checked).is_none() {
        if nodes_checked.len() == 1 {
            return;
        }
        index = 1;
        anchor_checked = nodes_checked[index];
        anchor_unchecked = nodes_unchecked[index];
    }

    let min_level = match op {
        0 | 1 => anchor_checked.level(arena_checked) + 1,
        2 | 3 => {
            anchor_checked
                .parent(arena_checked)
                .expect("insert anchor has parent")
                .level(arena_checked)
                + 1
        }
        _ => unreachable!(),
    };
    let level = min_level + (extra_depth % 3);
    let text = make_headline_text(level, &format!("p{}", step));

    let new_checked = arena_checked.new_section(text.clone().into()).unwrap();
    let new_unchecked = arena_unchecked.new_section(text.into()).unwrap();

    match op {
        0 => {
            anchor_checked
                .checked_append(arena_checked, new_checked)
                .unwrap();
            anchor_unchecked.unchecked_append(arena_unchecked, new_unchecked);
        }
        1 => {
            anchor_checked
                .checked_prepend(arena_checked, new_checked)
                .unwrap();
            anchor_unchecked.unchecked_prepend(arena_unchecked, new_unchecked);
        }
        2 => {
            anchor_checked
                .checked_insert_before(arena_checked, new_checked)
                .unwrap();
            anchor_unchecked.unchecked_insert_before(arena_unchecked, new_unchecked);
        }
        3 => {
            anchor_checked
                .checked_insert_after(arena_checked, new_checked)
                .unwrap();
            anchor_unchecked.unchecked_insert_after(arena_unchecked, new_unchecked);
        }
        _ => unreachable!(),
    }
}

#[cfg(feature = "headline-parser")]
fn expected_set_level_ok(node: Section, arena: &Arena, target_level: u16) -> bool {
    let old_level = node.level(arena);
    if old_level == target_level {
        return true;
    }
    if old_level == 0 || target_level == 0 {
        return false;
    }
    if let Some(parent) = node.parent(arena)
        && parent.level(arena) >= target_level
    {
        return false;
    }
    if node
        .children(arena)
        .any(|child| child.level(arena) <= target_level)
    {
        return false;
    }
    true
}

#[cfg(feature = "headline-parser")]
fn build_tree_from_recipe(recipe: &[(usize, u16)]) -> (Arena, Document) {
    let mut arena = Arena::default();
    let doc = arena.parse_str("* seed\n");

    for (i, (parent_pick, requested_level)) in recipe.iter().copied().enumerate() {
        let nodes = descendants(doc.root, &arena);
        let parent = nodes[parent_pick % nodes.len()];
        let level = requested_level.max(1);
        let text = make_headline_text(level, &format!("lv{}", i));
        let new_node = arena.new_section(text.into()).unwrap();
        parent.append(&mut arena, new_node).unwrap();
    }

    (arena, doc)
}

fn doc_string_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..256).prop_map(|chars| chars.into_iter().collect())
}

fn byte_index_at_char(s: &str, char_index: usize) -> usize {
    if char_index == s.chars().count() {
        s.len()
    } else {
        s.char_indices()
            .nth(char_index)
            .map(|(i, _)| i)
            .expect("char index in bounds")
    }
}

fn needle_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 1..5).prop_map(|chars| chars.into_iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn prop_parse_emit_identity(input in doc_string_strategy()) {
        let mut arena = Arena::default();
        let doc = arena.parse_str(&input);
        prop_assert_eq!(doc.to_rope(&arena).to_string(), input);
    }

    #[test]
    fn prop_entry_points_equivalent(input in doc_string_strategy()) {
        let mut arena_str = Arena::default();
        let doc_str = arena_str.parse_str(&input);
        let text_str = doc_str.to_rope(&arena_str).to_string();
        let empty_root_str = doc_str.empty_root_section;
        let terminal_newline_str = doc_str.terminal_newline;
        let semantics_str = collect_section_semantics(doc_str.root, &arena_str);

        let mut arena_string = Arena::default();
        let doc_string = arena_string.parse_string(input.clone());
        prop_assert_eq!(empty_root_str, doc_string.empty_root_section);
        prop_assert_eq!(terminal_newline_str, doc_string.terminal_newline);
        prop_assert_eq!(&text_str, &doc_string.to_rope(&arena_string).to_string());
        prop_assert_eq!(
            &semantics_str,
            &collect_section_semantics(doc_string.root, &arena_string)
        );

        let mut arena_rope = Arena::default();
        let doc_rope = arena_rope.parse_rope(Rope::from(input.clone()));
        prop_assert_eq!(empty_root_str, doc_rope.empty_root_section);
        prop_assert_eq!(terminal_newline_str, doc_rope.terminal_newline);
        prop_assert_eq!(&text_str, &doc_rope.to_rope(&arena_rope).to_string());
        prop_assert_eq!(
            &semantics_str,
            &collect_section_semantics(doc_rope.root, &arena_rope)
        );

        let mut arena_reader = Arena::default();
        let mut reader = Cursor::new(input.as_bytes());
        let doc_reader = arena_reader.parse_reader(&mut reader).unwrap();
        prop_assert_eq!(empty_root_str, doc_reader.empty_root_section);
        prop_assert_eq!(terminal_newline_str, doc_reader.terminal_newline);
        prop_assert_eq!(&text_str, &doc_reader.to_rope(&arena_reader).to_string());
        prop_assert_eq!(
            &semantics_str,
            &collect_section_semantics(doc_reader.root, &arena_reader)
        );
    }

    #[test]
    fn prop_tree_checked_unchecked_edit_invariants(
        ops in prop::collection::vec((any::<u8>(), any::<usize>(), 0u16..4u16), 0..64)
    ) {
        let mut checked_arena = Arena::default();
        let checked_doc = checked_arena.parse_str("* seed\n");

        let mut unchecked_arena = Arena::default();
        let unchecked_doc = unchecked_arena.parse_str("* seed\n");

        for (step, (op_kind, target_index, extra_depth)) in ops.into_iter().enumerate() {
            apply_valid_mirrored_edit(
                &checked_doc,
                &mut checked_arena,
                &unchecked_doc,
                &mut unchecked_arena,
                op_kind,
                target_index,
                extra_depth,
                step,
            );

            let checked_text = checked_doc.to_rope(&checked_arena).to_string();
            let unchecked_text = unchecked_doc.to_rope(&unchecked_arena).to_string();
            prop_assert_eq!(checked_text, unchecked_text);

            assert_tree_invariants(&checked_doc, &checked_arena);
            assert_tree_invariants(&unchecked_doc, &unchecked_arena);
        }
    }

    #[test]
    fn prop_new_section_contract(input in doc_string_strategy()) {
        let mut parse_arena = Arena::default();
        let parsed = parse_arena.parse_str(&input);
        let has_multiple_root_sections = parsed.root.children(&parse_arena).count() > 1;

        let mut ns_arena = Arena::default();
        let maybe_section = ns_arena.new_section(input.clone().into());

        prop_assert_eq!(maybe_section.is_none(), has_multiple_root_sections);

        if let Some(section) = maybe_section {
            let serialized = section.to_rope(&ns_arena).to_string();
            let mut reparse_arena = Arena::default();
            let reparsed = reparse_arena.new_section(serialized.clone().into());
            prop_assert!(reparsed.is_some());
            prop_assert_eq!(
                reparsed.unwrap().to_rope(&reparse_arena).to_string(),
                serialized
            );
        }
    }

    #[test]
    fn prop_ropeext_byte_char_boundaries(
        input in doc_string_strategy(),
        start_char_raw in any::<usize>(),
        end_char_raw in any::<usize>(),
        needle in needle_strategy(),
        memchr_needle in any::<u8>(),
        memchr_start_raw in any::<usize>(),
    ) {
        let char_len = input.chars().count();
        let start_char = start_char_raw % (char_len + 1);
        let end_char = end_char_raw % (char_len + 1);
        let (a, b) = if start_char <= end_char {
            (start_char, end_char)
        } else {
            (end_char, start_char)
        };
        let start_byte = byte_index_at_char(&input, a);
        let end_byte = byte_index_at_char(&input, b);

        let rope = Rope::from(input.clone());
        let slice = rope.slice(..);

        let expected = &input[start_byte..end_byte];
        prop_assert_eq!(rope.slice_bytes(start_byte..end_byte).to_string(), expected);
        prop_assert_eq!(slice.slice_bytes(start_byte..end_byte).to_string(), expected);

        prop_assert_eq!(rope.find(&needle), input.find(&needle));
        prop_assert_eq!(slice.find(&needle), input.find(&needle));
        prop_assert_eq!(rope.rfind(&needle), input.rfind(&needle));
        prop_assert_eq!(slice.rfind(&needle), input.rfind(&needle));

        let memchr_start = memchr_start_raw.min(input.len());
        let expected_memchr = input
            .as_bytes()
            .iter()
            .enumerate()
            .skip(memchr_start)
            .find(|(_, b)| **b == memchr_needle)
            .map(|(i, _)| i)
            .unwrap_or(input.len());

        prop_assert_eq!(rope.memchr(memchr_needle, memchr_start), expected_memchr);
        prop_assert_eq!(slice.memchr(memchr_needle, memchr_start), expected_memchr);
    }
}

#[cfg(feature = "headline-parser")]
fn safe_tag_strategy() -> impl Strategy<Value = String> {
    string_regex("[A-Za-z0-9_@#%]{1,9}")
        .expect("valid regex")
        .prop_map(|s| s)
}

#[cfg(feature = "headline-parser")]
fn safe_title_strategy() -> impl Strategy<Value = Rope> {
    prop::collection::vec(string_regex("[a-z]{1,8}").expect("valid regex"), 0..5).prop_map(
        |words| {
            if words.is_empty() {
                Rope::from("")
            } else {
                // Prefix avoids accidental TODO/COMMENT/priority parsing.
                Rope::from(format!("t {}", words.join(" ")))
            }
        },
    )
}

#[cfg(feature = "headline-parser")]
fn safe_body_strategy() -> impl Strategy<Value = Rope> {
    prop::collection::vec(
        string_regex("[A-Za-z0-9 _-]{0,24}").expect("valid regex"),
        0..6,
    )
    .prop_map(|lines| {
        if lines.is_empty() {
            Rope::from("")
        } else {
            Rope::from(lines.join("\n"))
        }
    })
}

#[cfg(feature = "headline-parser")]
fn headline_builder_strategy() -> impl Strategy<Value = HeadlineBuilder> {
    (
        1u16..=8u16,
        any::<bool>(),
        prop_oneof![
            Just(None),
            Just(Some(Rope::from("TODO"))),
            Just(Some(Rope::from("DONE"))),
        ],
        prop_oneof![
            Just(None),
            Just(Some('A')),
            Just(Some('B')),
            Just(Some('C')),
        ],
        prop::collection::vec(safe_tag_strategy(), 0..5),
        safe_title_strategy(),
        safe_body_strategy(),
    )
        .prop_map(|(level, commented, keyword, priority, tags, title, body)| {
            let mut builder = HeadlineBuilder::default();
            builder.level(level);
            builder.commented(commented);
            builder.keyword(keyword);
            builder.priority(priority);
            for tag in tags {
                builder.add_tag(&tag);
            }
            builder.title(title);
            builder.body(body);
            builder
        })
}

#[cfg(feature = "headline-parser")]
fn date_strategy() -> impl Strategy<Value = Date> {
    (1970i32..=2100i32, 1u32..=12u32, 1u32..=28u32).prop_map(|(y, m, d)| Date::new(y, m, d))
}

#[cfg(feature = "headline-parser")]
fn time_strategy() -> impl Strategy<Value = Time> {
    (0u32..=23u32, 0u32..=59u32).prop_map(|(h, m)| Time::new(h, m))
}

#[cfg(feature = "headline-parser")]
fn point_strategy() -> impl Strategy<Value = Point> {
    (
        date_strategy(),
        prop_oneof![Just(Activity::Active), Just(Activity::Inactive)],
        prop_oneof![Just(None), time_strategy().prop_map(Some)],
    )
        .prop_map(|(date, active, time)| Point::new(date).with_active(active).with_time(time))
}

#[cfg(feature = "headline-parser")]
fn timestamp_strategy() -> impl Strategy<Value = Timestamp<'static>> {
    let point = point_strategy().prop_map(Timestamp::from);

    let range =
        (point_strategy(), point_strategy()).prop_map(|(a, b)| Timestamp::Range(Range::new(a, b)));

    let time_range = (point_strategy(), time_strategy()).prop_map(|(start, end_time)| {
        let start = if start.time().is_some() {
            start
        } else {
            start.with_time(Some(Time::new(0, 0)))
        };
        Timestamp::TimeRange(TimeRange::new(start, end_time))
    });

    let diary = string_regex("[A-Za-z0-9 _-]{0,32}")
        .expect("valid regex")
        .prop_map(|s| Timestamp::Diary(Diary::new(&s).into_owned()));

    prop_oneof![point, range, time_range, diary]
}

#[cfg(feature = "headline-parser")]
fn headline_node_spec_strategy() -> impl Strategy<
    Value = (
        Option<Rope>,
        Option<char>,
        bool,
        Rope,
        Vec<String>,
        bool,
        bool,
        bool,
        bool,
    ),
> {
    (
        prop_oneof![
            Just(None),
            Just(Some(Rope::from("TODO"))),
            Just(Some(Rope::from("DONE"))),
        ],
        prop_oneof![
            Just(None),
            Just(Some('A')),
            Just(Some('B')),
            Just(Some('C')),
        ],
        any::<bool>(),
        string_regex("[a-z]{1,8}( [a-z]{1,8}){0,2}")
            .expect("valid regex")
            .prop_map(Rope::from),
        prop::collection::vec(string_regex("[a-z0-9]{1,6}").expect("valid regex"), 0..3),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
}

#[cfg(feature = "headline-parser")]
fn build_generated_headline_text(
    level: u16,
    index: usize,
    spec: (
        Option<Rope>,
        Option<char>,
        bool,
        Rope,
        Vec<String>,
        bool,
        bool,
        bool,
        bool,
    ),
) -> String {
    let (
        keyword,
        priority,
        commented,
        title,
        tags,
        has_scheduled,
        has_deadline,
        has_closed,
        has_property_drawer,
    ) = spec;

    let mut text = String::new();
    text.push_str(&"*".repeat(level as usize));
    text.push(' ');

    if let Some(keyword) = keyword {
        text.push_str(&keyword.to_string());
        text.push(' ');
    }
    if let Some(priority) = priority {
        text.push_str(&format!("[#{}] ", priority));
    }
    if commented {
        text.push_str("COMMENT ");
    }
    text.push_str(&title.to_string());
    if !tags.is_empty() {
        text.push_str(" :");
        text.push_str(&tags.join(":"));
        text.push(':');
    }

    let mut planning_parts: Vec<String> = Vec::new();
    if has_scheduled {
        planning_parts.push("SCHEDULED: <2024-01-02 Tue>".to_string());
    }
    if has_deadline {
        planning_parts.push("DEADLINE: <2024-01-03 Wed>".to_string());
    }
    if has_closed {
        planning_parts.push("CLOSED: [2024-01-04 Thu]".to_string());
    }
    if !planning_parts.is_empty() {
        text.push('\n');
        text.push_str(&planning_parts.join(" "));
    }

    if has_property_drawer {
        text.push('\n');
        text.push_str(":PROPERTIES:\n");
        text.push_str(&format!(":P{}: v{}\n", index, index));
        text.push_str(":END:");
    }
    text
}

#[cfg(feature = "headline-parser")]
fn assert_headline_semantics_match_orgize_prop(
    ours: Section,
    theirs: orgize::Headline,
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

#[cfg(feature = "headline-parser")]
proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn prop_headline_builder_roundtrip(builder in headline_builder_strategy()) {
        let headline = builder.headline(None).expect("generated builder should be valid");
        prop_assert_eq!(headline.to_builder(), builder.clone());

        let reparsed = parse_valid_single_headline(headline.to_rope().slice(..), &Context::default());
        prop_assert_eq!(reparsed.to_builder(), builder);
    }

    #[test]
    fn prop_timestamp_roundtrip(timestamp in timestamp_strategy()) {
        let input = timestamp.to_string();
        let (rest, parsed) = Timestamp::parse(&input).expect("generated timestamp must parse");
        prop_assert_eq!(rest, "");

        let emitted = parsed.to_string();
        let (rest2, reparsed) = Timestamp::parse(&emitted).expect("re-emitted timestamp must parse");
        prop_assert_eq!(rest2, "");
        prop_assert_eq!(reparsed, parsed);
    }

    #[test]
    fn prop_differential_vs_orgize(
        seed in headline_node_spec_strategy(),
        ops in prop::collection::vec((any::<usize>(), headline_node_spec_strategy()), 0..20),
    ) {
        let mut arena = Arena::default();
        let doc = arena.parse_str("");

        let seed_text = build_generated_headline_text(1, 0, seed);
        let seed_node = arena.new_section(seed_text.into()).unwrap();
        doc.root.append(&mut arena, seed_node).unwrap();

        for (i, (parent_pick, spec)) in ops.into_iter().enumerate() {
            let nodes = descendants(doc.root, &arena);
            let parent = nodes[parent_pick % nodes.len()];
            let level = parent.level(&arena) + 1;
            let text = build_generated_headline_text(level, i + 1, spec);
            let node = arena.new_section(text.into()).unwrap();
            parent.append(&mut arena, node).unwrap();
        }

        let emitted = doc.to_rope(&arena).to_string();
        let org = orgize::Org::parse(&emitted);

        fn compare(ours: Section, theirs: orgize::Headline, org: &orgize::Org, arena: &Arena) {
            assert_headline_semantics_match_orgize_prop(ours, theirs, org, arena);
            assert_eq!(ours.children(arena).count(), theirs.children(org).count());
            for (o, t) in ours.children(arena).zip(theirs.children(org)) {
                compare(o, t, org, arena);
            }
        }

        assert_eq!(
            doc.root.children(&arena).count(),
            org.document().children(&org).count()
        );
        for (ours, theirs) in doc.root.children(&arena).zip(org.document().children(&org)) {
            compare(ours, theirs, &org, &arena);
        }
    }

    #[test]
    fn prop_timestamp_conversion_laws(timestamp in timestamp_strategy()) {
        // parse/display roundtrip already covered; this test focuses on conversion and trait laws.
        let ts = timestamp.clone();

        match ts.clone() {
            Timestamp::Point(p) => {
                prop_assert_eq!(Point::try_from(ts.clone()).unwrap(), p);
                prop_assert!(Range::try_from(ts.clone()).is_err());
                prop_assert!(TimeRange::try_from(ts.clone()).is_err());
            }
            Timestamp::Range(r) => {
                prop_assert_eq!(Range::try_from(ts.clone()).unwrap(), r);
                if let Ok(tr) = TimeRange::try_from(&r) {
                    let rr: Range = tr.into();
                    prop_assert_eq!(rr, r);
                }
            }
            Timestamp::TimeRange(tr) => {
                prop_assert_eq!(TimeRange::try_from(ts.clone()).unwrap(), tr);
                let r: Range = tr.into();
                prop_assert_eq!(TimeRange::try_from(&r).unwrap(), tr);
            }
            Timestamp::Diary(d) => {
                prop_assert!(Diary::try_from(&ts).is_ok());
                prop_assert_eq!(Diary::try_from(ts.clone()).unwrap().to_borrowed(), d.to_borrowed());
                prop_assert!(Point::try_from(ts.clone()).is_err());
            }
        }

        // TimestampExt consistency checks.
        if let Some(start) = ts.start_point() {
            prop_assert_eq!(ts.start_time(), start.time());
            prop_assert_eq!(ts.active(), start.active());
        } else {
            prop_assert!(matches!(ts, Timestamp::Diary(_)));
        }

        if let Some(end) = ts.end_point() {
            prop_assert_eq!(ts.end_time(), end.time());
        } else {
            prop_assert!(matches!(ts, Timestamp::Point(_) | Timestamp::Diary(_)));
        }

        match ts {
            Timestamp::Point(p) => {
                prop_assert_eq!(Timestamp::Point(p).date(), Some(p.date().unwrap()));
                prop_assert_eq!(Timestamp::Point(p).time(), p.time());
                prop_assert_eq!(Timestamp::Point(p).cookie(), p.cookie());
            }
            Timestamp::Range(r) => {
                let t = Timestamp::Range(r);
                prop_assert_eq!(t.date(), None);
                prop_assert_eq!(t.time(), None);
                prop_assert_eq!(t.cookie(), None);
            }
            Timestamp::TimeRange(tr) => {
                let t = Timestamp::TimeRange(tr);
                prop_assert_eq!(t.date(), tr.date());
                prop_assert_eq!(t.time(), None);
                prop_assert_eq!(t.cookie(), tr.cookie());
            }
            Timestamp::Diary(_) => {
                prop_assert_eq!(ts.date(), None);
                prop_assert_eq!(ts.time(), None);
                prop_assert_eq!(ts.cookie(), None);
            }
        }
    }
}

#[cfg(feature = "headline-parser")]
proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn prop_set_level_structural_safety(
        recipe in prop::collection::vec((any::<usize>(), 1u16..8u16), 0..24),
        node_pick in any::<usize>(),
        target_level in 0u16..12u16,
    ) {
        let (mut arena, doc) = build_tree_from_recipe(&recipe);
        let nodes = descendants(doc.root, &arena);
        let node = nodes[node_pick % nodes.len()];

        let expected_ok = expected_set_level_ok(node, &arena, target_level);
        let before_text = doc.to_rope(&arena).to_string();
        let before_semantics = collect_section_semantics(doc.root, &arena);

        let result = node.set_level(&mut arena, target_level);

        if expected_ok {
            prop_assert!(result.is_ok());
            assert_tree_invariants(&doc, &arena);
        } else {
            prop_assert!(result.is_err());
            prop_assert_eq!(doc.to_rope(&arena).to_string(), before_text);
            prop_assert_eq!(collect_section_semantics(doc.root, &arena), before_semantics);
        }
    }
}
