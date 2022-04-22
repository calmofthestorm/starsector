use std::io::Result;

use ropey::Rope;

use starsector::*;

fn main() -> Result<()> {
    // Arena is physical storage for one or more org documents.
    let mut arena = Arena::default();

    let doc = arena.parse_str("");
    println!(
        "All documents have a root section. This one has level {} and text \"{}\"",
        doc.root.level(&arena),
        doc.root.text(&arena)
    );

    // Create some loose headlines.
    let child1 = arena.new_section("* Hello".into()).unwrap();
    let grandchild1 = arena.new_section("* Sub Headline".into()).unwrap();
    let child2 = arena.new_section("* World".into()).unwrap();
    let grandchild2 = arena
        .new_section("******* Deep Sub Headline".into())
        .unwrap();

    // Attempt to attach fails because `grandchild1` level is <= `child1` level.
    child1.checked_append(&mut arena, grandchild1).unwrap_err();

    // This succeeds because `append` will adjust the level if needed.
    child1.append(&mut arena, grandchild1).unwrap();

    // This also suceeds because skipping levels is permitted.
    child2.checked_append(&mut arena, grandchild2).unwrap();

    // Attach the loose headlines to the document.
    doc.root.append(&mut arena, child1).unwrap();
    doc.root.append(&mut arena, child2).unwrap();

    println!("The document is:\n{}\n\n-------\n\n", doc.to_rope(&arena));

    // Appending a section will detach it from its existing parent, if any. If
    // this is undesired, you can create a new section with the same content.
    grandchild1.checked_append(&mut arena, grandchild2).unwrap();

    let duplicate_child2 = arena.clone_section(child2);
    grandchild2.prepend(&mut arena, duplicate_child2).unwrap();

    let duplicate_child1 = arena.clone_section(child1);
    child1.insert_after(&mut arena, duplicate_child1).unwrap();

    // You can change root text too, but not to contain headlines.
    doc.root
        .set_raw(&mut arena, "- This is\n- body text".into())
        .unwrap();

    println!("The document is:\n{}\n\n-------\n\n", doc.to_rope(&arena));

    // Sections may be modified, but must remain valid in the tree -- that is,
    // they must be a headline (or the root of the document) as they were
    // before, at an appropriate level.
    assert_eq!(child1.level(&arena), 1);
    child1.set_level(&mut arena, 0).unwrap_err();
    child1.set_level(&mut arena, 1).unwrap();

    // Fails because grandchild1 has level 2.
    child1.set_level(&mut arena, 2).unwrap_err();
    grandchild1.set_level(&mut arena, 3).unwrap();
    child1.set_level(&mut arena, 2).unwrap();

    child1
        .set_raw(&mut arena, "Not a headline".into())
        .unwrap_err();
    child1
        .set_raw(&mut arena, "* More than\n** one headline".into())
        .unwrap_err();
    child1
        .set_raw(&mut arena, "* More than\n* one headline".into())
        .unwrap_err();

    child1
        .set_raw(
            &mut arena,
            "* Reduces level and\nchanges all text\n- list\n- ok".into(),
        )
        .unwrap();

    println!("The document is:\n{}\n\n-------\n\n", doc.to_rope(&arena));

    #[cfg(feature = "headline-parser")]
    {
        // With headline parser feature, keywords, tags, priority, comment, and
        // title all become supported.
        //
        // Many headline functions require a Context, which specifies the
        // keywords so that the headline can be parsed unambiguously.
        //
        // You can specify None for the context to use the default keywords TODO
        // and DONE.
        let headline = child1.parse_headline(&arena, /*context=*/ None);
        let headline = headline.unwrap();
        let mut headline_builder = headline.to_builder();
        headline_builder
            .keyword(Some("TODO".into()))
            .clear_tag("bar")
            .add_tag("qux")
            .priority(Some('C'))
            .add_tag("foo")
            .commented(true)
            .add_tag("foo")
            .clear_tag("qux")
            .add_tag("bar");
        headline_builder.body(Rope::default());
        let headline = headline_builder.headline(/*context=*/ None).unwrap();
        let tags: Vec<_> = headline.tags().collect();
        assert_eq!(tags.len(), 2);
        assert!(headline.has_tag("foo"));
        assert!(headline.has_tag("bar"));
        assert!(!headline.has_tag("qux"));

        // The headline borrows from the arena, so in order to use it mutably,
        // we need to copy.
        let headline = headline.to_owned();

        child1.set_headline(&mut arena, &headline).unwrap();

        println!("The document is:\n{}\n\n-------\n\n", doc.to_rope(&arena));

        // You can also create a new node with a headline, though it still must
        // correspond to a single headline -- no subtrees.

        // No subtrees.
        HeadlineBuilder::default()
            .title("My Title\n* Hello".into())
            .headline(/*context=*/ None)
            .unwrap_err();
        HeadlineBuilder::default()
            .title("My Title".into())
            .body("* Foo\n* Bar".into())
            .headline(/*context=*/ None)
            .unwrap_err();

        // Allowed, but note that the * is not part of the level.
        assert_eq!(
            Rope::from("* * Hello"),
            HeadlineBuilder::default()
                .title("* Hello".into())
                .headline(/*context=*/ None)
                .unwrap()
                .to_rope()
        );

        // There are convenience accessors that bypass the headline, if desired.
        assert_eq!(
            grandchild2.keyword(&arena, /*context=*/ None).unwrap(),
            None
        );
        grandchild2
            .set_keyword(&mut arena, Some("TODO".into()), /*context=*/ None)
            .unwrap();
        assert_eq!(
            grandchild2.keyword(&arena, /*context=*/ None).unwrap(),
            Some("TODO".into())
        );

        println!("The document is:\n{}\n\n-------\n\n", doc.to_rope(&arena));
    }

    #[cfg(feature = "orgize-integration")]
    {
        doc.root.remove_children(&mut arena);

        // If Orgize support is enabled, properties and planning
        // (DEADLINE/SCHEDULE) become available. We don't wrap the entire orgize
        // API, and in particular, any other elements inside the body itself.
        let section = arena
            .new_section(
                "* TODO do stuff\nDEADLINE: <2020-07-09 Thu>\n:PROPERTIES:\n:ID: myid123\n:END:"
                    .into(),
            )
            .unwrap();
        doc.root.prepend(&mut arena, section).unwrap();

        let mut headline_builder = HeadlineBuilder::default();
        headline_builder
            .title("Other stuff".into())
            .keyword(Some("TODO".into()))
            .property("FOO", "BAR")
            .unwrap()
            .level(1)
            .scheduled(Some(Point::new(Date::new(2109, 11, 11)).into()));

        let section2 = arena
            .new_section(
                headline_builder
                    .headline(/*context=*/ None)
                    .unwrap()
                    .to_rope(),
            )
            .unwrap();
        doc.root.prepend(&mut arena, section2).unwrap();

        // Parse the headline we generated.
        let section1 = doc.root.children(&arena).next().unwrap();
        eprintln!(
            "Builder created node property FOO is {}",
            section1
                .get_property(&arena, "FOO", /*context=*/ None)
                .unwrap()
                .unwrap()
        );
        eprintln!(
            "Builder created node scheduled isj {:?}",
            section1.scheduled(&arena, /*context=*/ None).unwrap()
        );
        eprintln!(
            "Builder created node closed is: {:?}",
            section1.closed(&arena, /*context=*/ None).unwrap()
        );

        // Now repeat for the one we parsed from text.
        let section2 = doc.root.children(&arena).last().unwrap();
        eprintln!(
            "Starsector created node property ID is {}",
            section2.get_id(&arena, /*context=*/ None).unwrap().unwrap()
        );
        eprintln!(
            "Starsector created node scheduled is: {:?}",
            section2.scheduled(&arena, /*context=*/ None).unwrap()
        );
        eprintln!(
            "Starsector created node deadline is: {:?}",
            section2.deadline(&arena, /*context=*/ None).unwrap()
        );

        println!("The document is:\n{}\n\n-------\n\n", doc.to_rope(&arena));

        // Convenience accessors exist for these as well.
        section1
            .set_property(&mut arena, "FOO", "CORGE", /*context=*/ None)
            .unwrap();
        section2
            .set_scheduled(
                &mut arena,
                Some(Point::new(Date::new(1971, 11, 11)).into()),
                /*context=*/ None,
            )
            .unwrap();
        section1.generate_id(&mut arena, /*context=*/ None).unwrap();
        section2
            .add_tag(&mut arena, "mytag", /*context=*/ None)
            .unwrap();

        println!("The document is:\n{}\n\n-------\n\n", doc.to_rope(&arena));
    }

    #[cfg(not(feature = "headline-parser"))]
    println!("Enable headline parser for headline examples.");

    Ok(())
}
