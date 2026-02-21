use starsector::*;

use rand::RngCore;

fn assert_semantically_equivalent(a: &Headline, b: &Headline) {
    assert_eq!(a.level(), b.level());
    assert_eq!(a.priority(), b.priority());
    assert_eq!(a.commented(), b.commented());
    assert_eq!(
        a.keyword().map(|k| k.to_string()),
        b.keyword().map(|k| k.to_string())
    );
    assert_eq!(a.title().to_string(), b.title().to_string());
    assert_eq!(
        a.tags().map(|t| t.to_string()).collect::<Vec<_>>(),
        b.tags().map(|t| t.to_string()).collect::<Vec<_>>()
    );
    assert_eq!(a.planning(), b.planning());
    assert_eq!(a.body().to_string(), b.body().to_string());
}

fn assert_reparse_converges(mut emitted: String) {
    for _ in 0..16 {
        let reparsed = parse_valid_single_headline(
            ropey::Rope::from(emitted.clone()).slice(..),
            &Context::default(),
        );
        let next = reparsed.to_rope().to_string();
        if next == emitted {
            return;
        }
        emitted = next;
    }
    panic!("headline parse/emit did not converge within bound");
}

// Test that we can parse gibberish without crashing, and that parsing it,
// writing out a headline, then parsing that, is identical to the initial
// parse.
#[test]
fn parse_fuzz() {
    let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(36);

    let mut alphabet: Vec<String> = " \n\t@:#[]*A9饭".chars().map(|c| c.to_string()).collect();

    alphabet.extend(
        [
            "\n**",
            "\n** ",
            "\n*****",
            "\n\n",
            "TODO ",
            "COMMENT ",
            "[#B] ",
            " :hello: ",
            " :a:b:",
        ]
        .iter()
        .map(|x| x.to_string()),
    );

    let mut arena = Arena::default();

    for _ in 0..25000 {
        let mut s = String::default();

        for _ in 0..(rng.next_u32() % 100) {
            let c = (rng.next_u32() as usize) % alphabet.len();
            s += &alphabet[c];
        }

        let doc = arena.parse_str(&s);
        if let Some(headline) = doc.root.headline(&arena, None) {
            let emitted = headline.to_builder().headline(None).unwrap();
            assert_semantically_equivalent(&headline, &emitted);

            let reparsed =
                parse_valid_single_headline(emitted.to_rope().slice(..), &Context::default());
            assert_semantically_equivalent(&headline, &reparsed);
            assert_semantically_equivalent(&emitted, &reparsed);
        }
    }
}

#[test]
fn parse_fuzz_regression_crash_d514a0daf082b676aca72cae0b4e0b57f92ee1dc() {
    let s = "\n\n* ::: ::\n\n* ::: ::::uuuu:\u{000c}\u{000c}";
    let mut arena = Arena::default();
    let doc = arena.parse_str(s);

    for section in doc.root.descendants(&arena) {
        if let Some(headline) = section.headline(&arena, None) {
            assert_reparse_converges(headline.to_rope().to_string());
        }
    }
}

#[test]
fn parse_fuzz_regression_crash_ad7a749fab092a5051dfcb509e3223035ad83866() {
    let s = "\u{000c}:: ::\n* :::1:uu:\u{000c}:: ::\n* :::1:uu:\u{000c}:* *\r\r\r/";
    let mut arena = Arena::default();
    let doc = arena.parse_str(s);

    for section in doc.root.descendants(&arena) {
        if let Some(headline) = section.headline(&arena, None) {
            assert_reparse_converges(headline.to_rope().to_string());
        }
    }
}
